//! Server module for `jaws serve`.
//!
//! Provides a gRPC server with mTLS authentication that exposes the server's
//! configured secret providers to remote jaws clients.

pub mod auth;
pub mod enrollment;
pub mod pki;
pub mod service;

use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;

use tonic::transport::{Identity, Server, ServerTlsConfig};

use crate::config::Config;
use crate::db::SecretRepository;
use crate::error::JawsError;
use crate::secrets::Provider;

use self::enrollment::EnrollmentManager;
use self::pki::PkiPaths;
use self::service::JawsServiceImpl;
use self::service::proto::jaws_service_server::JawsServiceServer;

/// Configuration for the jaws server.
pub struct ServerConfig {
    /// Address to bind the server to.
    pub bind_addr: SocketAddr,
    /// Server name (used as provider prefix on clients).
    pub server_name: String,
    /// Path to CA cert (if using external PKI).
    pub ca_cert_path: Option<std::path::PathBuf>,
    /// Path to CA key (if using external PKI).
    pub ca_key_path: Option<std::path::PathBuf>,
    /// Path to server cert (if using external PKI).
    pub server_cert_path: Option<std::path::PathBuf>,
    /// Path to server key (if using external PKI).
    pub server_key_path: Option<std::path::PathBuf>,
}

/// Start the jaws gRPC server with mTLS.
pub async fn run_server(
    config: &Config,
    server_config: ServerConfig,
    providers: Vec<Provider>,
    repo: SecretRepository,
) -> Result<(), JawsError> {
    let bind_addr = server_config.bind_addr;

    // Determine SANs for the server certificate
    let san_entries = compute_san_entries(&bind_addr);

    // Load or generate PKI material
    let (ca_cert_pem, ca_key_pem, server_cert_pem, server_key_pem) =
        load_or_generate_pki(config, &server_config, &san_entries)?;

    // Set up the enrollment manager
    let enrollment = Arc::new(EnrollmentManager::new(
        ca_cert_pem.clone(),
        ca_key_pem,
        repo.clone(),
    ));

    // Generate and display an enrollment token
    let token = enrollment.generate_token()?;
    eprintln!();
    eprintln!("=== Enrollment Token ===");
    eprintln!("  {}", token);
    eprintln!();
    eprintln!("  Clients can connect with:");
    eprintln!(
        "    jaws connect https://{}:{} --token {}",
        if bind_addr.ip().is_unspecified() {
            "SERVER_IP".to_string()
        } else {
            bind_addr.ip().to_string()
        },
        bind_addr.port(),
        token
    );
    eprintln!();
    eprintln!("  Token expires in 15 minutes. Generate a new one with:");
    eprintln!("    jaws serve --generate-token");
    eprintln!("========================");
    eprintln!();

    // Build the gRPC service
    let service = JawsServiceImpl::new(
        providers,
        server_config.server_name.clone(),
        enrollment,
        repo,
    );

    // Configure TLS with optional client auth.
    // Client auth is optional so that unauthenticated clients can call the
    // Enroll RPC to obtain a certificate.  All other RPCs enforce a valid
    // client certificate at the application layer (see service.rs).
    let tls_config = ServerTlsConfig::new()
        .identity(Identity::from_pem(&server_cert_pem, &server_key_pem))
        .client_ca_root(tonic::transport::Certificate::from_pem(&ca_cert_pem))
        .client_auth_optional(true);

    eprintln!(
        "jaws server '{}' listening on {} (mTLS enabled)",
        server_config.server_name, bind_addr
    );
    eprintln!("Press Ctrl+C to stop.");
    eprintln!();

    // Start the server
    Server::builder()
        .tls_config(tls_config)
        .map_err(|e| JawsError::Other(format!("Failed to configure TLS: {}", e)))?
        .add_service(JawsServiceServer::new(service))
        .serve(bind_addr)
        .await
        .map_err(|e| JawsError::Other(format!("Server error: {}", e)))?;

    Ok(())
}

/// Load PKI material from disk or generate new ones.
fn load_or_generate_pki(
    config: &Config,
    server_config: &ServerConfig,
    san_entries: &[String],
) -> Result<(String, String, String, String), JawsError> {
    // If external certs are provided, use them
    if let (Some(ca_cert_path), Some(ca_key_path), Some(server_cert_path), Some(server_key_path)) = (
        &server_config.ca_cert_path,
        &server_config.ca_key_path,
        &server_config.server_cert_path,
        &server_config.server_key_path,
    ) {
        let ca_cert = fs::read_to_string(ca_cert_path).map_err(|e| JawsError::Io(e))?;
        let ca_key = fs::read_to_string(ca_key_path).map_err(|e| JawsError::Io(e))?;
        let server_cert = fs::read_to_string(server_cert_path).map_err(|e| JawsError::Io(e))?;
        let server_key = fs::read_to_string(server_key_path).map_err(|e| JawsError::Io(e))?;
        return Ok((ca_cert, ca_key, server_cert, server_key));
    }

    // Use built-in PKI
    let pki_paths = pki::init_server_pki(&config.secrets_path(), san_entries)?;

    let ca_cert = fs::read_to_string(&pki_paths.ca_cert)?;
    let ca_key = fs::read_to_string(&pki_paths.ca_key)?;
    let server_cert = fs::read_to_string(&pki_paths.server_cert)?;
    let server_key = fs::read_to_string(&pki_paths.server_key)?;

    Ok((ca_cert, ca_key, server_cert, server_key))
}

/// Compute SAN entries for the server certificate based on the bind address.
fn compute_san_entries(bind_addr: &SocketAddr) -> Vec<String> {
    let mut sans = vec!["localhost".to_string()];

    let ip = bind_addr.ip();
    if ip.is_unspecified() {
        // 0.0.0.0 — add common local IPs
        sans.push("127.0.0.1".to_string());
        // Try to get the hostname
        if let Ok(hostname) = hostname::get() {
            if let Some(h) = hostname.to_str() {
                if !sans.contains(&h.to_string()) {
                    sans.push(h.to_string());
                }
            }
        }
    } else {
        sans.push(ip.to_string());
    }

    sans
}

/// Generate and print a new enrollment token (for --generate-token flag).
pub fn generate_and_print_token(config: &Config, repo: &SecretRepository) -> Result<(), JawsError> {
    let pki_paths = PkiPaths::new(&config.secrets_path());
    if !pki_paths.ca_exists() {
        return Err(JawsError::config(
            "Server has not been initialized yet. Run 'jaws serve' first to generate the CA.",
        ));
    }

    let ca_cert_pem = fs::read_to_string(&pki_paths.ca_cert)?;
    let ca_key_pem = fs::read_to_string(&pki_paths.ca_key)?;

    let enrollment = EnrollmentManager::new(ca_cert_pem, ca_key_pem, repo.clone());
    let token = enrollment.generate_token()?;

    println!("{}", token);
    eprintln!("Token generated (valid for 15 minutes)");

    Ok(())
}
