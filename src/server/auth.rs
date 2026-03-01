//! mTLS authentication - extract client identity from TLS peer certificates.

use tonic::Request;
use tonic::transport::server::TcpConnectInfo;
use tonic::transport::server::TlsConnectInfo;

/// Client identity extracted from a mTLS connection.
#[derive(Debug, Clone)]
pub struct ClientIdentity {
    /// Common Name from the client certificate.
    pub name: String,
    /// SHA-256 fingerprint of the client certificate (hex).
    pub cert_fingerprint: String,
}

/// Extract the client identity from a gRPC request's TLS peer certificates.
/// Returns None if the request doesn't contain TLS connection info or peer certs.
pub fn extract_client_identity<T>(request: &Request<T>) -> Option<ClientIdentity> {
    let tls_info = request
        .extensions()
        .get::<TlsConnectInfo<TcpConnectInfo>>()?;

    let certs = tls_info.peer_certs()?;
    let first_cert = certs.first()?;
    let cert_der = first_cert.as_ref();

    // Parse the DER-encoded certificate
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der).ok()?;

    // Extract CN
    let cn = cert
        .subject()
        .iter()
        .flat_map(|rdn| rdn.iter())
        .find(|attr| attr.attr_type() == &x509_parser::oid_registry::OID_X509_COMMON_NAME)
        .and_then(|attr| attr.as_str().ok())
        .map(|s| s.to_string())?;

    // Compute fingerprint
    use sha2::{Digest, Sha256};
    let fingerprint = hex::encode(Sha256::digest(cert_der));

    Some(ClientIdentity {
        name: cn,
        cert_fingerprint: fingerprint,
    })
}
