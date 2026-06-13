//! gRPC client connection management with mTLS.
//!
//! Provides two connection modes:
//! - `connect()` / `connect_from_pem()`: Full mTLS with CA cert + client cert
//! - `connect_for_enrollment()`: TLS with no server verification and no client
//!   cert, for the initial enrollment handshake (TOFU model).

use std::fs;
use std::path::Path;
use std::sync::Arc;

use tokio::net::TcpStream;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};

use crate::error::JawsError;

/// Extract the hostname (or IP) from a URL string for TLS domain verification.
fn extract_domain(url: &str) -> String {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = without_scheme
        .split(':')
        .next()
        .unwrap_or(without_scheme)
        .split('/')
        .next()
        .unwrap_or(without_scheme);
    host.to_string()
}

/// Extract host:port from a URL, defaulting to port 443.
fn extract_host_port(url: &str) -> (String, u16) {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    // Strip path
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    // Split host and port
    if let Some((host, port_str)) = authority.rsplit_once(':') {
        let port = port_str.parse::<u16>().unwrap_or(443);
        (host.to_string(), port)
    } else {
        (authority.to_string(), 443)
    }
}

/// Ensure a URL has the https:// scheme prefix (required by tonic/gRPC).
fn normalize_url(url: &str) -> String {
    if url.starts_with("https://") || url.starts_with("http://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

/// Establish a mTLS gRPC channel to a remote jaws server.
pub async fn connect(
    url: &str,
    ca_cert_path: &Path,
    client_cert_path: &Path,
    client_key_path: &Path,
) -> Result<Channel, JawsError> {
    let ca_cert_pem = fs::read_to_string(ca_cert_path)?;
    let client_cert_pem = fs::read_to_string(client_cert_path)?;
    let client_key_pem = fs::read_to_string(client_key_path)?;

    connect_from_pem(url, &ca_cert_pem, &client_cert_pem, &client_key_pem).await
}

/// Establish a mTLS gRPC channel using PEM strings directly.
pub async fn connect_from_pem(
    url: &str,
    ca_cert_pem: &str,
    client_cert_pem: &str,
    client_key_pem: &str,
) -> Result<Channel, JawsError> {
    let url = normalize_url(url);
    let domain = extract_domain(&url);

    let tls_config = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(ca_cert_pem))
        .identity(Identity::from_pem(client_cert_pem, client_key_pem))
        .domain_name(domain);

    let endpoint = Channel::from_shared(url.clone())
        .map_err(|e| JawsError::Other(format!("Invalid server URL '{}': {}", url, e)))?
        .tls_config(tls_config)
        .map_err(|e| JawsError::Other(format!("TLS configuration failed: {}", e)))?;

    let channel = endpoint
        .connect()
        .await
        .map_err(|e| JawsError::Other(format!("Failed to connect to server: {}", e)))?;

    Ok(channel)
}

/// Connect to a server for enrollment (no client cert, no CA cert).
///
/// During enrollment the client has neither a client certificate nor the
/// server's CA certificate.  We handle TLS ourselves using `tokio-rustls`
/// with a permissive certificate verifier that captures the server's
/// certificate fingerprint, then hand the already-encrypted stream to tonic.
///
/// Returns the gRPC channel and the SHA-256 fingerprint of the server's
/// end-entity certificate so the caller can verify it out-of-band.
pub async fn connect_for_enrollment(url: &str) -> Result<(Channel, Option<String>), JawsError> {
    let url = normalize_url(url);
    let (host, port) = extract_host_port(&url);
    let host_for_tls = host.clone();

    // Shared storage for the captured server certificate fingerprint
    let captured_fp = Arc::new(std::sync::Mutex::new(None));

    // Build a rustls ClientConfig that accepts any server certificate but
    // records its fingerprint for out-of-band verification.
    let verifier = Arc::new(CapturingCertVerifier {
        captured: Arc::clone(&captured_fp),
    });

    let mut tls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    tls_config.alpn_protocols = vec![b"h2".to_vec()];
    let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(tls_config));

    // Build a custom connector that does TCP + TLS, returning an
    // already-encrypted stream.  The Endpoint uses http:// so tonic
    // does not try to add its own TLS layer.
    let connector = tower::service_fn(move |_uri: http::Uri| {
        let host = host_for_tls.clone();
        let tls = tls_connector.clone();
        let port = port;
        async move {
            // TCP connect
            let tcp = TcpStream::connect(format!("{}:{}", host, port))
                .await
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        format!("TCP connect to {}:{} failed: {}", host, port, e),
                    )
                })?;

            // TLS handshake — build an owned ServerName
            let server_name =
                rustls::pki_types::ServerName::try_from(host.clone()).unwrap_or_else(|_| {
                    rustls::pki_types::ServerName::IpAddress(
                        host.parse()
                            .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST))
                            .into(),
                    )
                });
            let tls_stream = tls.connect(server_name, tcp).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("TLS handshake with {}:{} failed: {}", host, port, e),
                )
            })?;

            Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(tls_stream))
        }
    });

    // Use http:// so tonic's Connector wrapper does NOT try to add
    // another TLS layer on top of our already-encrypted stream.
    let fake_http_url = format!("http://{}:{}", host, port);
    let endpoint = Endpoint::from_shared(fake_http_url)
        .map_err(|e| JawsError::Other(format!("Invalid URL: {}", e)))?;

    let channel: Channel = endpoint
        .connect_with_connector(connector)
        .await
        .map_err(|e| {
            JawsError::Other(format!(
                "Failed to connect to server at {}:{} — {}",
                host, port, e
            ))
        })?;

    // Extract the captured fingerprint now that the handshake is complete
    let server_fingerprint = captured_fp.lock().ok().and_then(|g| g.clone());

    Ok((channel, server_fingerprint))
}

// ── Capturing cert verifier (enrollment only) ─────────────────────────

/// A certificate verifier that accepts any server certificate but records
/// the SHA-256 fingerprint of the end-entity certificate.
///
/// Used ONLY during the enrollment handshake so the user can verify the
/// server's identity out-of-band before trusting the returned CA certificate.
#[derive(Debug)]
struct CapturingCertVerifier {
    captured: Arc<std::sync::Mutex<Option<String>>>,
}

impl rustls::client::danger::ServerCertVerifier for CapturingCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(end_entity.as_ref());
        let fp = hex::encode(hash);
        if let Ok(mut guard) = self.captured.lock() {
            *guard = Some(fp);
        }
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}
