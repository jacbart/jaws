//! Handler for `jaws connect` — enroll with a remote jaws server.
//! Handler for `jaws disconnect` — remove a server connection.

use crate::client::ClientPaths;
use crate::config::{Config, ServerConnection};
use crate::server::pki;
use crate::server::service::proto;
use proto::jaws_service_client::JawsServiceClient;

/// Handle the `jaws connect` command.
pub async fn handle_connect(
    config: &Config,
    url: &str,
    token: &str,
    name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine client name (defaults to hostname)
    let client_name = hostname::get()
        .ok()
        .and_then(|h| h.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "jaws-client".to_string());

    eprintln!("Connecting to {}...", url);

    // Generate a CSR (key pair stays local)
    let (csr_pem, client_key_pem) = pki::generate_csr(&client_name)?;

    // Connect to the server for enrollment (no mTLS yet)
    let channel = crate::client::connection::connect_for_enrollment(url).await?;
    let mut grpc_client = JawsServiceClient::new(channel);

    // Send enrollment request
    let response = grpc_client
        .enroll(proto::EnrollRequest {
            token: token.to_string(),
            client_name: client_name.clone(),
            csr_pem,
        })
        .await
        .map_err(|e| format!("Enrollment failed: {}", e))?;

    let resp = response.into_inner();
    let server_name = name.unwrap_or(resp.server_name.clone());

    eprintln!(
        "Enrolled with server '{}' as '{}'",
        resp.server_name, client_name
    );

    // Use the jaws config directory for client certs
    let jaws_config_dir = Config::default_config_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let client_paths = ClientPaths::new(&jaws_config_dir, &server_name);

    // Save the CA cert, signed client cert, and our private key
    client_paths.save(&resp.ca_cert_pem, &resp.client_cert_pem, &client_key_pem)?;

    eprintln!("  Certificates saved to: {}", client_paths.dir.display());

    // Build a server connection config entry
    let server_conn = ServerConnection {
        name: server_name.clone(),
        url: url.to_string(),
        ca_cert: Some(client_paths.ca_cert.to_string_lossy().to_string()),
        client_cert: Some(client_paths.client_cert.to_string_lossy().to_string()),
        client_key: Some(client_paths.client_key.to_string_lossy().to_string()),
    };

    // Try to update the config file
    if let Some(config_path) = Config::find_existing_config() {
        let mut config = config.clone();
        config.add_server(server_conn);
        config.save(&config_path)?;
        eprintln!("  Server connection saved to: {}", config_path.display());
    } else {
        eprintln!("  No config file found. Add this to your jaws.kdl:");
        eprintln!();
        eprintln!("  server \"{}\" url=\"{}\" {{", server_name, url);
        eprintln!("      ca-cert \"{}\"", client_paths.ca_cert.display());
        eprintln!(
            "      client-cert \"{}\"",
            client_paths.client_cert.display()
        );
        eprintln!("      client-key \"{}\"", client_paths.client_key.display());
        eprintln!("  }}");
    }

    eprintln!();
    eprintln!(
        "Connected! Remote providers will appear as '{}/PROVIDER_ID'.",
        server_name
    );
    eprintln!("Run 'jaws sync' to discover remote secrets.");

    Ok(())
}

/// Handle the `jaws disconnect` command.
pub async fn handle_disconnect(
    config: &Config,
    server_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if this server exists in config
    let server_exists = config.servers.iter().any(|s| s.name == server_name);
    if !server_exists {
        return Err(format!(
            "Server '{}' not found in configuration. Known servers: {}",
            server_name,
            if config.servers.is_empty() {
                "(none)".to_string()
            } else {
                config
                    .servers
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )
        .into());
    }

    // Remove from config
    let config_path = Config::find_existing_config().ok_or("No config file found")?;
    let mut config = config.clone();
    config.remove_server(server_name);
    config.save(&config_path)?;

    // Try to remove client cert directory
    let jaws_config_dir = Config::default_config_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let client_paths = ClientPaths::new(&jaws_config_dir, server_name);
    if client_paths.dir.exists() {
        std::fs::remove_dir_all(&client_paths.dir)?;
        eprintln!(
            "  Removed client certificates from: {}",
            client_paths.dir.display()
        );
    }

    println!("Disconnected from server '{}'.", server_name);
    Ok(())
}
