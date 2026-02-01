//! Delete command handlers - deleting local secrets.

use std::fs;
use std::io::{self, Write};

use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::{get_secret_path, Provider};

/// Handle the local delete command - delete local secret files and DB records
pub async fn handle_delete(
    config: &Config,
    repo: &SecretRepository,
    secret_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get all downloaded secrets
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        println!("No local secrets to delete.");
        return Ok(());
    }

    // Select secret to delete
    let selected_secret = if let Some(name) = &secret_name {
        let matches: Vec<_> = downloaded
            .iter()
            .filter(|(s, _)| {
                s.display_name.to_lowercase().contains(&name.to_lowercase())
                    || s.hash.starts_with(name)
            })
            .collect();

        if matches.is_empty() {
            return Err(format!("No secret found matching '{}'", name).into());
        } else if matches.len() > 1 {
            return Err(format!(
                "Multiple secrets match '{}'. Be more specific.",
                name
            )
            .into());
        }
        matches[0].clone()
    } else {
        // Show picker for selecting a secret
        use ff::{create_items_channel, run_tui_with_config, TuiConfig};

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

        let selected = run_tui_with_config(rx, false, tui_config)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        if selected.is_empty() {
            return Ok(());
        }

        downloaded
            .into_iter()
            .find(|(s, _)| {
                let display = format!("{} | {}", s.provider_id, s.display_name);
                selected.contains(&display)
            })
            .ok_or("Secret not found")?
    };

    let (secret, _latest_download) = selected_secret;

    // Get all versions for this secret
    let all_downloads = repo.list_downloads(secret.id)?;
    let version_count = all_downloads.len();

    // Prompt for confirmation
    print!(
        "Delete '{}' and {} local version(s)? [y/N]: ",
        secret.display_name, version_count
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input != "y" && input != "yes" {
        println!("Cancelled.");
        return Ok(());
    }

    // Delete all version files
    let mut deleted_files = 0;
    for download in &all_downloads {
        let file_path = get_secret_path(&config.secrets_path(), &download.filename);
        if file_path.exists() {
            fs::remove_file(&file_path)?;
            deleted_files += 1;
        }
    }

    // Delete DB records (downloads are deleted via CASCADE when secret is deleted)
    repo.delete_secret(secret.id)?;

    println!(
        "Deleted '{}' ({} file(s), {} version record(s))",
        secret.display_name, deleted_files, version_count
    );

    Ok(())
}

/// Handle the remote delete command - delete from provider
pub async fn handle_remote_delete(
    providers: &[Provider],
    secret_name: Option<String>,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected = if let Some(name) = secret_name {
        // Find which provider might have this secret
        vec![(providers[0].id().to_string(), name)]
    } else {
        crate::secrets::select_from_all_providers(providers).await?
    };

    for (provider_id, secret_ref) in selected {
        let provider = providers
            .iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| format!("Provider {} not found", provider_id))?;

        match provider.delete(&secret_ref, force).await {
            Ok(()) => {
                if force {
                    println!("{} [{}] deleted (force)", secret_ref, provider_id);
                } else {
                    println!(
                        "{} [{}] deleted (recovery period: 7-30 days)",
                        secret_ref, provider_id
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Error deleting {} from {}: {}",
                    secret_ref, provider_id, e
                );
            }
        }
    }

    Ok(())
}
