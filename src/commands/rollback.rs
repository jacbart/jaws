//! Rollback command handlers - restoring secrets to previous versions.

use std::fs;
use std::process::Command;

use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::{get_secret_path, save_secret_file, Provider};

/// Handle the rollback command - rollback a secret to a previous version
pub async fn handle_rollback(
    config: &Config,
    repo: &SecretRepository,
    secret_name: Option<String>,
    version: Option<i32>,
    edit: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get all downloaded secrets
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        println!("No secrets downloaded. Use 'jaws pull' first.");
        return Ok(());
    }

    // If secret_name provided, filter; otherwise show picker
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

    let (secret, latest_download) = selected_secret;

    // Get all versions for this secret
    let all_downloads = repo.list_downloads(secret.id)?;

    if all_downloads.len() <= 1 {
        println!(
            "Only one version exists for '{}'. Nothing to rollback to.",
            secret.display_name
        );
        return Ok(());
    }

    // Select version to restore
    let target_download = if let Some(v) = version {
        repo.get_download_by_version(secret.id, v)?
            .ok_or_else(|| format!("Version {} not found for '{}'", v, secret.display_name))?
    } else {
        // Show picker for version selection
        use chrono_humanize::HumanTime;
        use ff::{create_items_channel, run_tui_with_config, TuiConfig};

        let (tx, rx) = create_items_channel();

        // Skip the current (latest) version - we want to restore to something else
        for download in all_downloads.iter().skip(1) {
            let age = HumanTime::from(download.downloaded_at);
            let display = format!(
                "v{} - {} - {}",
                download.version,
                age,
                download
                    .file_hash
                    .as_deref()
                    .map(|h| &h[..8])
                    .unwrap_or("?")
            );
            if tx.send(display).await.is_err() {
                break;
            }
        }
        drop(tx);

        if all_downloads.len() <= 1 {
            println!("Only one version exists. Nothing to rollback to.");
            return Ok(());
        }

        let mut tui_config = TuiConfig::with_height(10.min(all_downloads.len() as u16 + 2));
        tui_config.show_help_text = false;

        let selected = run_tui_with_config(rx, false, tui_config)
            .await
            .map_err(|e| e as Box<dyn std::error::Error>)?;

        if selected.is_empty() {
            return Ok(());
        }

        // Parse version from selection "v{version} - ..."
        let selected_str = &selected[0];
        let version_str = selected_str
            .strip_prefix("v")
            .and_then(|s| s.split(" - ").next())
            .ok_or("Failed to parse version")?;
        let selected_version: i32 = version_str.parse()?;

        repo.get_download_by_version(secret.id, selected_version)?
            .ok_or("Selected version not found")?
    };

    // Read the old version's content
    let old_file_path = get_secret_path(&config.secrets_path(), &target_download.filename);
    if !old_file_path.exists() {
        return Err(format!(
            "Version {} file not found at: {}\nThe file may have been deleted.",
            target_download.version,
            old_file_path.display()
        )
        .into());
    }

    let content = fs::read_to_string(&old_file_path)?;

    // Create a new version with this content (next version number after latest)
    let new_version = latest_download.version + 1;
    let (new_filename, content_hash) = save_secret_file(
        &config.secrets_path(),
        &secret.display_name,
        &secret.hash,
        new_version,
        &content,
    )?;

    // Record the new download
    repo.create_download(secret.id, &new_filename, &content_hash)?;

    println!(
        "Rolled back '{}' from v{} -> v{} (new current)",
        secret.display_name, target_download.version, new_version
    );

    // Log the operation
    repo.log_operation(
        "rollback",
        &secret.provider_id,
        &secret.display_name,
        Some(&format!(
            "{{\"from_version\": {}, \"to_version\": {}}}",
            target_download.version, new_version
        )),
    )?;

    // Open in editor if requested
    if edit {
        let file_path = get_secret_path(&config.secrets_path(), &new_filename);
        let _ = Command::new(config.editor())
            .arg(file_path.to_string_lossy().to_string())
            .status()
            .expect("failed to launch editor");
    }

    Ok(())
}

/// Handle the remote rollback command - rollback on provider
pub async fn handle_remote_rollback(
    providers: &[Provider],
    secret_name: Option<String>,
    version_id: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected = if let Some(name) = secret_name {
        vec![(providers[0].id().to_string(), name)]
    } else {
        crate::secrets::select_from_all_providers(providers).await?
    };

    for (provider_id, secret_ref) in selected {
        let provider = providers
            .iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| format!("Provider {} not found", provider_id))?;

        match provider.rollback(&secret_ref, version_id.as_deref()).await {
            Ok(result) => {
                if let Some(vid) = &version_id {
                    println!(
                        "{} [{}] rolled back to version {} -> {}",
                        secret_ref, provider_id, vid, result
                    );
                } else {
                    println!(
                        "{} [{}] rolled back to previous version -> {}",
                        secret_ref, provider_id, result
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Error rolling back {} in {}: {}",
                    secret_ref, provider_id, e
                );
            }
        }
    }

    Ok(())
}
