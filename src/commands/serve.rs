//! Handler for `jaws serve` — starts the gRPC secret sharing server.

use std::net::SocketAddr;
use std::path::PathBuf;

use crate::config::Config;
use crate::db::{DbProvider, SecretRepository};
use crate::secrets::detect_providers;
use crate::server::{self, ServerConfig};

/// Handle the `jaws serve` command.
///
/// Provider detection is done internally so that sub-modes like
/// `--generate-token`, `--list-clients`, and `--revoke` can complete
/// instantly without waiting for cloud provider initialization.
pub async fn handle_serve(
    config: &Config,
    repo: &SecretRepository,
    bind: &str,
    name: Option<String>,
    generate_token: bool,
    ca_cert: Option<PathBuf>,
    ca_key: Option<PathBuf>,
    server_cert: Option<PathBuf>,
    server_key: Option<PathBuf>,
    revoke: Option<String>,
    list_clients: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Handle --generate-token (quick exit, no providers needed)
    if generate_token {
        server::generate_and_print_token(config, repo)?;
        return Ok(());
    }

    // Handle --list-clients (quick exit, no providers needed)
    if list_clients {
        return handle_list_clients(config, repo);
    }

    // Handle --revoke (quick exit, no providers needed)
    if let Some(client_name) = revoke {
        return handle_revoke_client(config, repo, &client_name);
    }

    // ── Full server start: detect providers, then bind ────────────────

    // Parse bind address
    let bind_addr: SocketAddr = bind
        .parse()
        .map_err(|e| format!("Invalid bind address '{}': {}", bind, e))?;

    // Determine server name
    let server_name = name.unwrap_or_else(|| {
        hostname::get()
            .ok()
            .and_then(|h| h.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "jaws-server".to_string())
    });

    eprintln!("Initializing providers...");
    let providers = detect_providers(config, Some(repo)).await?;

    // Register providers in the database
    for provider in &providers {
        repo.upsert_provider(&DbProvider {
            id: provider.id().to_string(),
            kind: provider.kind().to_string(),
            last_sync_at: None,
            config_json: None,
        })?;
    }
    eprintln!("  {} provider(s) available", providers.len());

    let server_config = ServerConfig {
        bind_addr,
        server_name,
        ca_cert_path: ca_cert,
        ca_key_path: ca_key,
        server_cert_path: server_cert,
        server_key_path: server_key,
    };

    // Start the server (blocks until Ctrl+C)
    server::run_server(config, server_config, providers, repo.clone()).await?;

    Ok(())
}

fn handle_list_clients(
    config: &Config,
    repo: &SecretRepository,
) -> Result<(), Box<dyn std::error::Error>> {
    let pki_paths = crate::server::pki::PkiPaths::new(&config.secrets_path());
    if !pki_paths.ca_exists() {
        return Err("Server has not been initialized yet. Run 'jaws serve' first.".into());
    }

    let ca_cert_pem = std::fs::read_to_string(&pki_paths.ca_cert)?;
    let ca_key_pem = std::fs::read_to_string(&pki_paths.ca_key)?;
    let enrollment =
        crate::server::enrollment::EnrollmentManager::new(ca_cert_pem, ca_key_pem, repo.clone());

    let clients = enrollment.list_clients()?;
    if clients.is_empty() {
        println!("No enrolled clients.");
    } else {
        println!("Enrolled clients:\n");
        for (name, fingerprint, revoked) in clients {
            let status = if revoked { " (REVOKED)" } else { "" };
            println!("  {}{}", name, status);
            println!(
                "    fingerprint: {}...",
                &fingerprint[..16.min(fingerprint.len())]
            );
        }
    }

    Ok(())
}

fn handle_revoke_client(
    config: &Config,
    repo: &SecretRepository,
    client_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pki_paths = crate::server::pki::PkiPaths::new(&config.secrets_path());
    if !pki_paths.ca_exists() {
        return Err("Server has not been initialized yet. Run 'jaws serve' first.".into());
    }

    let ca_cert_pem = std::fs::read_to_string(&pki_paths.ca_cert)?;
    let ca_key_pem = std::fs::read_to_string(&pki_paths.ca_key)?;
    let enrollment =
        crate::server::enrollment::EnrollmentManager::new(ca_cert_pem, ca_key_pem, repo.clone());

    if enrollment.revoke_client(client_name)? {
        println!("Client '{}' has been revoked.", client_name);
    } else {
        println!("Client '{}' not found.", client_name);
    }

    Ok(())
}
