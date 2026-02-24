//! Rollback command handlers - restoring secrets to previous versions (local or remote).

use std::fs;
use std::process::Command;

use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::{Provider, get_secret_path, save_secret_file, storage::compute_content_hash};
use crate::utils::parse_secret_ref;

use super::snapshot::{check_and_snapshot, is_dirty};

/// Handle the unified rollback command - can rollback locally or on remote provider
pub async fn handle_rollback(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    secret_name: Option<String>,
    version: Option<i32>,
    edit: bool,
    remote: bool,
    version_id: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if remote {
        handle_remote_rollback(config, providers, secret_name, version_id).await
    } else {
        handle_local_rollback(config, repo, secret_name, version, edit).await
    }
}

/// Handle local rollback - restore a secret to a previous local version
async fn handle_local_rollback(
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
        let matches: Vec<_> = if let Ok((provider, specific_name)) = parse_secret_ref(name, None) {
            downloaded
                .iter()
                .filter(|(s, _)| s.provider_id == provider && s.display_name == specific_name)
                .cloned()
                .collect()
        } else {
            downloaded
                .iter()
                .filter(|(s, _)| {
                    s.display_name.to_lowercase().contains(&name.to_lowercase())
                        || s.hash.starts_with(name)
                })
                .cloned()
                .collect()
        };

        if matches.is_empty() {
            return Err(format!("No secret found matching '{}'", name).into());
        } else if matches.len() > 1 {
            return Err(format!("Multiple secrets match '{}'. Be more specific.", name).into());
        }
        matches[0].clone()
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
                selected.iter().any(|(_, sel)| sel == &display)
            })
            .ok_or("Secret not found")?
    };

    let (secret, latest_download) = selected_secret;

    // Check for uncommitted changes and auto-snapshot before rollback
    if is_dirty(config, &latest_download) {
        match check_and_snapshot(config, repo, &secret, &latest_download) {
            Ok(result) => {
                if let Some(v) = result.new_version {
                    println!(
                        "Saved local changes: {}://{} (v{})",
                        secret.provider_id, secret.display_name, v
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not save local changes for {}: {}",
                    secret.display_name, e
                );
            }
        }
    }

    // Re-fetch downloads after potential snapshot
    let all_downloads = repo.list_downloads(secret.id)?;
    let latest_download = all_downloads.first().cloned().ok_or("No downloads found")?;

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
        use ff::{TuiConfig, create_items_channel, run_tui_with_config};

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
        let (_, selected_str) = &selected[0];
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
    let target_content_hash = compute_content_hash(&content);

    // Compare with current version's hash - skip if identical
    if let Some(current_hash) = &latest_download.file_hash {
        if current_hash == &target_content_hash {
            println!(
                "No changes - content identical to v{}.",
                target_download.version
            );
            return Ok(());
        }
    }

    // Content differs - create a new version with this content
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
        "rollback_local",
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

/// Handle remote rollback - rollback on the provider
async fn handle_remote_rollback(
    config: &Config,
    providers: &[Provider],
    secret_name: Option<String>,
    version_id: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected = if let Some(name) = secret_name {
        // Parse the secret reference to identify provider
        let (provider_id, secret) = parse_secret_ref(&name, config.default_provider().as_deref())
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

        vec![(provider_id, secret)]
    } else {
        crate::secrets::select_from_all_providers(providers).await?
    };

    for (provider_id, secret_ref) in selected {
        let provider = providers
            .iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| format!("Provider {} not found", provider_id))?;

        if provider_id == "jaws" {
            println!(
                "Note: 'jaws' is a local-only provider. Use 'jaws rollback' without --remote for local rollback."
            );
            continue;
        }

        match provider.rollback(&secret_ref, version_id.as_deref()).await {
            Ok(result) => {
                if let Some(vid) = &version_id {
                    println!(
                        "{}://{} rolled back to version {} -> {}",
                        provider_id, secret_ref, vid, result
                    );
                } else {
                    println!(
                        "{}://{} rolled back to previous version -> {}",
                        provider_id, secret_ref, result
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
