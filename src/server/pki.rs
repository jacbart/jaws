//! PKI module - Certificate Authority management, certificate generation and storage.
//!
//! On first run, `jaws serve` generates a self-signed CA certificate and key using `rcgen`.
//! Client certificates are signed by this CA during enrollment. Server certificates
//! are also signed by the CA so clients can verify the server's identity.

use std::fs;
use std::path::{Path, PathBuf};

use rand::Rng;
use rcgen::{
    BasicConstraints, CertificateParams, CertificateSigningRequestParams, DnType, DnValue,
    ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose, SerialNumber,
};
use time::OffsetDateTime;

use crate::error::JawsError;
use crate::utils::restrict_file_permissions;

/// Default validity period for the CA certificate (10 years).
const CA_VALIDITY_DAYS: i64 = 3650;
/// Default validity period for server certificates (1 year).
const SERVER_CERT_VALIDITY_DAYS: i64 = 365;
/// Default validity period for client certificates (1 year).
const CLIENT_CERT_VALIDITY_DAYS: i64 = 365;

/// Paths to the PKI files for a jaws server instance.
#[derive(Debug, Clone)]
pub struct PkiPaths {
    /// Directory containing all PKI material.
    pub dir: PathBuf,
    /// CA certificate (PEM).
    pub ca_cert: PathBuf,
    /// CA private key (PEM).
    pub ca_key: PathBuf,
    /// Server certificate (PEM).
    pub server_cert: PathBuf,
    /// Server private key (PEM).
    pub server_key: PathBuf,
}

impl PkiPaths {
    /// Compute the standard PKI paths under the given secrets directory.
    pub fn new(secrets_path: &Path) -> Self {
        let dir = secrets_path.join("server");
        Self {
            ca_cert: dir.join("ca.pem"),
            ca_key: dir.join("ca-key.pem"),
            server_cert: dir.join("server.pem"),
            server_key: dir.join("server-key.pem"),
            dir,
        }
    }

    /// Whether the CA has already been initialized.
    pub fn ca_exists(&self) -> bool {
        self.ca_cert.exists() && self.ca_key.exists()
    }

    /// Whether the server certificate has already been generated.
    pub fn server_cert_exists(&self) -> bool {
        self.server_cert.exists() && self.server_key.exists()
    }
}

/// Generate a random 16-byte serial number for certificates.
fn random_serial() -> SerialNumber {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill(&mut bytes);
    // Ensure the high bit is clear (serial numbers are positive integers)
    bytes[0] &= 0x7f;
    SerialNumber::from_slice(&bytes)
}

/// Reconstruct a CA certificate from its PEM representation.
/// This is needed because rcgen doesn't have `Certificate::from_pem` —
/// the workaround is to parse the params and re-self-sign with the same key.
fn load_ca(
    ca_cert_pem: &str,
    ca_key_pem: &str,
) -> Result<(rcgen::Certificate, KeyPair), JawsError> {
    let ca_key = KeyPair::from_pem(ca_key_pem)
        .map_err(|e| JawsError::encryption(format!("Failed to parse CA key: {}", e)))?;
    let ca_params = CertificateParams::from_ca_cert_pem(ca_cert_pem)
        .map_err(|e| JawsError::encryption(format!("Failed to parse CA cert: {}", e)))?;
    let ca_cert = ca_params
        .self_signed(&ca_key)
        .map_err(|e| JawsError::encryption(format!("Failed to reconstruct CA cert: {}", e)))?;
    Ok((ca_cert, ca_key))
}

/// Generate a new Certificate Authority key pair and self-signed certificate.
/// Returns (ca_cert_pem, ca_key_pem).
pub fn generate_ca() -> Result<(String, String), JawsError> {
    let key_pair = KeyPair::generate()
        .map_err(|e| JawsError::encryption(format!("Failed to generate CA key pair: {}", e)))?;

    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, DnValue::Utf8String("jaws CA".into()));
    params
        .distinguished_name
        .push(DnType::OrganizationName, DnValue::Utf8String("jaws".into()));
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    params.not_before = OffsetDateTime::now_utc();
    params.not_after = OffsetDateTime::now_utc() + time::Duration::days(CA_VALIDITY_DAYS);
    params.serial_number = Some(random_serial());

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| JawsError::encryption(format!("Failed to self-sign CA certificate: {}", e)))?;

    Ok((cert.pem(), key_pair.serialize_pem()))
}

/// Generate a server certificate signed by the CA.
/// `san_entries` should include the bind address hostname(s) and/or IPs.
/// Returns (server_cert_pem, server_key_pem).
pub fn generate_server_cert(
    ca_cert_pem: &str,
    ca_key_pem: &str,
    san_entries: &[String],
) -> Result<(String, String), JawsError> {
    let (ca_cert, ca_key) = load_ca(ca_cert_pem, ca_key_pem)?;

    let server_key = KeyPair::generate()
        .map_err(|e| JawsError::encryption(format!("Failed to generate server key pair: {}", e)))?;

    let mut params = CertificateParams::new(san_entries.to_vec()).map_err(|e| {
        JawsError::encryption(format!("Failed to create server cert params: {}", e))
    })?;
    params.distinguished_name.push(
        DnType::CommonName,
        DnValue::Utf8String("jaws server".into()),
    );
    params
        .distinguished_name
        .push(DnType::OrganizationName, DnValue::Utf8String("jaws".into()));
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.not_before = OffsetDateTime::now_utc();
    params.not_after = OffsetDateTime::now_utc() + time::Duration::days(SERVER_CERT_VALIDITY_DAYS);
    params.serial_number = Some(random_serial());

    let server_cert = params
        .signed_by(&server_key, &ca_cert, &ca_key)
        .map_err(|e| JawsError::encryption(format!("Failed to sign server certificate: {}", e)))?;

    Ok((server_cert.pem(), server_key.serialize_pem()))
}

/// Sign a CSR (Certificate Signing Request) with the CA, producing a client certificate.
/// The CSR provides the client's public key; the CA signs it.
/// Returns the signed client certificate PEM.
pub fn sign_csr(
    ca_cert_pem: &str,
    ca_key_pem: &str,
    csr_pem: &str,
    client_name: &str,
) -> Result<String, JawsError> {
    let (ca_cert, ca_key) = load_ca(ca_cert_pem, ca_key_pem)?;

    // Parse the CSR
    let csr = CertificateSigningRequestParams::from_pem(csr_pem)
        .map_err(|e| JawsError::encryption(format!("Failed to parse CSR: {}", e)))?;

    // Override the params with our desired certificate properties
    let mut params = csr.params;
    params
        .distinguished_name
        .push(DnType::CommonName, DnValue::Utf8String(client_name.into()));
    params.distinguished_name.push(
        DnType::OrganizationName,
        DnValue::Utf8String("jaws-client".into()),
    );
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.not_before = OffsetDateTime::now_utc();
    params.not_after = OffsetDateTime::now_utc() + time::Duration::days(CLIENT_CERT_VALIDITY_DAYS);
    params.serial_number = Some(random_serial());

    // Sign with the CSR's public key (signed_by on CertificateSigningRequestParams
    // takes the public key from the CSR)
    let csr_with_params = CertificateSigningRequestParams {
        params,
        public_key: csr.public_key,
    };

    let client_cert = csr_with_params.signed_by(&ca_cert, &ca_key).map_err(|e| {
        JawsError::encryption(format!("Failed to sign client cert from CSR: {}", e))
    })?;

    Ok(client_cert.pem())
}

/// Generate a CSR (Certificate Signing Request) for a client.
/// Returns (csr_pem, private_key_pem).
pub fn generate_csr(client_name: &str) -> Result<(String, String), JawsError> {
    let key_pair = KeyPair::generate()
        .map_err(|e| JawsError::encryption(format!("Failed to generate client key: {}", e)))?;

    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, DnValue::Utf8String(client_name.into()));
    params.distinguished_name.push(
        DnType::OrganizationName,
        DnValue::Utf8String("jaws-client".into()),
    );

    let csr = params
        .serialize_request(&key_pair)
        .map_err(|e| JawsError::encryption(format!("Failed to create CSR: {}", e)))?;

    let csr_pem = csr
        .pem()
        .map_err(|e| JawsError::encryption(format!("Failed to PEM-encode CSR: {}", e)))?;

    Ok((csr_pem, key_pair.serialize_pem()))
}

/// Initialize PKI for a server: generate CA if needed, then generate server cert.
/// `san_entries` should include hostnames/IPs the server listens on.
/// Returns the PkiPaths.
pub fn init_server_pki(secrets_path: &Path, san_entries: &[String]) -> Result<PkiPaths, JawsError> {
    let paths = PkiPaths::new(secrets_path);

    // Create PKI directory
    fs::create_dir_all(&paths.dir)?;

    // Generate CA if it doesn't exist
    if !paths.ca_exists() {
        eprintln!("Generating new Certificate Authority...");
        let (ca_cert_pem, ca_key_pem) = generate_ca()?;
        fs::write(&paths.ca_cert, &ca_cert_pem)?;
        fs::write(&paths.ca_key, &ca_key_pem)?;
        restrict_file_permissions(&paths.ca_key)?;
        restrict_file_permissions(&paths.ca_cert)?;
        eprintln!("  CA certificate: {}", paths.ca_cert.display());
    }

    // Generate server cert if it doesn't exist
    if !paths.server_cert_exists() {
        eprintln!("Generating server certificate...");
        let ca_cert_pem = fs::read_to_string(&paths.ca_cert)?;
        let ca_key_pem = fs::read_to_string(&paths.ca_key)?;
        let (server_cert_pem, server_key_pem) =
            generate_server_cert(&ca_cert_pem, &ca_key_pem, san_entries)?;
        fs::write(&paths.server_cert, &server_cert_pem)?;
        fs::write(&paths.server_key, &server_key_pem)?;
        restrict_file_permissions(&paths.server_key)?;
        restrict_file_permissions(&paths.server_cert)?;
        eprintln!("  Server certificate: {}", paths.server_cert.display());
        eprintln!("  SANs: {:?}", san_entries);
    }

    Ok(paths)
}

/// Extract the Common Name (CN) from a PEM-encoded certificate.
pub fn extract_cn_from_cert_pem(cert_pem: &str) -> Result<String, JawsError> {
    let pem = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes())
        .map_err(|e| JawsError::encryption(format!("Failed to parse PEM: {}", e)))?;
    let cert = pem
        .1
        .parse_x509()
        .map_err(|e| JawsError::encryption(format!("Failed to parse X.509: {}", e)))?;

    for rdn in cert.subject().iter() {
        for attr in rdn.iter() {
            if attr.attr_type() == &x509_parser::oid_registry::OID_X509_COMMON_NAME {
                return attr
                    .as_str()
                    .map(|s| s.to_string())
                    .map_err(|e| JawsError::encryption(format!("Failed to read CN: {}", e)));
            }
        }
    }

    Err(JawsError::encryption("No CN found in certificate"))
}

/// Compute a SHA-256 fingerprint of a PEM-encoded certificate (hex-encoded).
pub fn cert_fingerprint(cert_pem: &str) -> Result<String, JawsError> {
    use sha2::{Digest, Sha256};
    let pem = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes())
        .map_err(|e| JawsError::encryption(format!("Failed to parse PEM: {}", e)))?;
    let der = pem.1.contents;
    let hash = Sha256::digest(&der);
    Ok(hex::encode(hash))
}
