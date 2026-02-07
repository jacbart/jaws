//! History command handlers - viewing secret version history (local and remote).

use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::Provider;
use crate::utils::parse_secret_ref;

/// Handle the unified history command - show version history (local or remote)
pub async fn handle_history(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    secret_name: Option<String>,
    verbose: bool,
    limit: Option<usize>,
    remote: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if remote {
        handle_remote_history(config, providers, secret_name).await
    } else {
        handle_local_history(repo, secret_name, verbose, limit).await
    }
}

/// Handle local history - show version history for downloaded secrets
async fn handle_local_history(
    repo: &SecretRepository,
    secret_name: Option<String>,
    verbose: bool,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    use chrono_humanize::HumanTime;

    // Get all downloaded secrets
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        println!("No secrets downloaded. Use 'jaws pull' first.");
        return Ok(());
    }

    // If secret_name provided, filter; otherwise show picker
    let selected_secrets: Vec<_> = if let Some(name) = &secret_name {
        // Check if it's a specific reference (PROVIDER://NAME)
        if let Ok((provider, specific_name)) = parse_secret_ref(name, None) {
            downloaded
                .into_iter()
                .filter(|(s, _)| s.provider_id == provider && s.display_name == specific_name)
                .collect()
        } else {
            // Fuzzy search by name
            downloaded
                .into_iter()
                .filter(|(s, _)| {
                    s.display_name.to_lowercase().contains(&name.to_lowercase())
                        || s.hash.starts_with(name)
                })
                .collect()
        }
    } else {
        // Show picker for selecting a secret
        use ff::{TuiConfig, create_items_channel, run_tui_with_config};

        let (tx, rx) = create_items_channel();

        for (secret, _download) in &downloaded {
            let display = format!("{} | {}", secret.provider_id, secret.display_name);
            if tx.send(display).await.is_err() {
                break;
            }
        }
        drop(tx);

        let mut tui_config = TuiConfig::fullscreen();
        tui_config.show_help_text = false;

        let selected = run_tui_with_config(rx, false, tui_config) // single select
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        if selected.is_empty() {
            return Ok(());
        }

        // Find the selected secret
        downloaded
            .into_iter()
            .filter(|(s, _)| {
                let display = format!("{} | {}", s.provider_id, s.display_name);
                selected.contains(&display)
            })
            .collect()
    };

    if selected_secrets.is_empty() {
        println!(
            "No matching secrets found{}",
            secret_name
                .map(|n| format!(" for '{}'", n))
                .unwrap_or_default()
        );
        return Ok(());
    }

    // Show history for each selected secret
    for (secret, _latest_download) in selected_secrets {
        let downloads = repo.list_downloads(secret.id)?;

        if downloads.is_empty() {
            println!("{}: No download history", secret.display_name);
            continue;
        }

        println!("\n{}://{}", secret.provider_id, secret.display_name);
        println!("{}", "-".repeat((secret.provider_id.len() + secret.display_name.len() + 3).min(60)));

        let versions_to_show: Vec<_> = if let Some(n) = limit {
            downloads.into_iter().take(n).collect()
        } else {
            downloads
        };

        for (i, download) in versions_to_show.iter().enumerate() {
            let age = HumanTime::from(download.downloaded_at);
            let current_marker = if i == 0 { " (current)" } else { "" };

            if verbose {
                println!(
                    "  v{}: {} | {} | {}{}",
                    download.version,
                    download.downloaded_at.format("%Y-%m-%d %H:%M:%S"),
                    download.file_hash.as_deref().unwrap_or("no hash"),
                    download.filename,
                    current_marker
                );
            } else {
                println!("  v{}: {}{}", download.version, age, current_marker);
            }
        }
    }

    Ok(())
}

/// Handle remote history - show version history from the provider
async fn handle_remote_history(
    config: &Config,
    providers: &[Provider],
    secret_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected = if let Some(name) = secret_name {
        let (provider_id, secret) = parse_secret_ref(&name, config.default_provider().as_deref())
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        vec![(provider_id, secret)]
    } else {
        crate::secrets::select_from_all_providers(providers).await?
    };

    if selected.is_empty() {
        return Ok(());
    }

    for (provider_id, secret_ref) in selected {
        let provider = providers
            .iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

        println!("\n{}://{}", provider_id, secret_ref);
        println!("{}", "-".repeat((provider_id.len() + secret_ref.len() + 3).min(60)));

        match provider.kind() {
            "jaws" => {
                println!("  'jaws' is a local-only provider. Use 'jaws history' without --remote.");
            }
            "aws" => {
                // AWS Secrets Manager supports versioning
                println!("  Remote version history for AWS Secrets Manager is available via the AWS Console.");
                println!("  Use 'jaws rollback --remote' to restore a previous version.");
                println!();
                println!("  Tip: AWS maintains AWSCURRENT and AWSPREVIOUS version labels.");
                println!("  When you update a secret, the previous value becomes AWSPREVIOUS.");
            }
            "onepassword" => {
                println!("  1Password item history is available via the 1Password app or web interface.");
                println!("  The op CLI and service accounts have limited version history support.");
            }
            "bitwarden" => {
                println!("  Bitwarden Secrets Manager does not currently support version history.");
                println!("  Use local history ('jaws history' without --remote) to track your changes.");
            }
            _ => {
                println!("  Remote version history is not yet implemented for this provider.");
            }
        }
    }

    println!();
    println!("Hint: Use 'jaws history' (without --remote) to view local version history.");

    Ok(())
}
