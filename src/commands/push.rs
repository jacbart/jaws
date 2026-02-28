//! Push command handlers - uploading secrets to providers.

use std::fs;
use std::process::Command;

use crate::config::Config;
use crate::db::{DbDownload, DbSecret, SecretRepository};
use crate::secrets::{Provider, get_secret_path};

use crate::utils::parse_secret_ref;

use super::snapshot::{check_and_snapshot, is_dirty, print_snapshot_summary};

/// Handle the push command - push local secrets to providers
pub async fn handle_push(
    config: &Config,
    repo: &SecretRepository,
    providers: &[Provider],
    secret_name: Option<String>,
    edit: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get all downloaded secrets
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        return Err("No secrets downloaded. Use 'jaws pull' first.".into());
    }

    // Filter by name if provided, otherwise show TUI with modified secrets
    let secrets_to_push: Vec<(DbSecret, DbDownload)> = if let Some(name) = &secret_name {
        if let Ok((provider, specific_name)) = parse_secret_ref(name, None) {
            downloaded
                .into_iter()
                .filter(|(s, _)| s.provider_id == provider && s.display_name == specific_name)
                .collect()
        } else {
            downloaded
                .into_iter()
                .filter(|(s, _)| s.display_name.contains(name) || s.hash.starts_with(name))
                .collect()
        }
    } else {
        // No secret specified - show TUI with modified (dirty) secrets
        select_dirty_secrets(config, &downloaded).await?
    };

    if secrets_to_push.is_empty() {
        if secret_name.is_some() {
            return Err(format!("No matching secrets found for '{}'", secret_name.unwrap()).into());
        } else {
            println!("No modified secrets to push.");
            return Ok(());
        }
    }

    // Collect file paths for editing
    let files: Vec<String> = secrets_to_push
        .iter()
        .map(|(_, d)| {
            get_secret_path(&config.secrets_path(), &d.filename)
                .to_string_lossy()
                .to_string()
        })
        .collect();

    // Open in editor if requested
    if edit && !files.is_empty() {
        Command::new(config.editor())
            .args(&files)
            .status()
            .map_err(|e| {
                format!(
                    "Failed to launch editor '{}': {}. Set a valid editor with 'jaws config set editor <path>'.",
                    config.editor(), e
                )
            })?;
    }

    // Auto-snapshot any modified secrets before pushing
    let mut snapshot_results = Vec::new();
    for (secret, download) in &secrets_to_push {
        let result = check_and_snapshot(config, repo, secret, download)?;
        if result.new_version.is_some() {
            snapshot_results.push(result);
        }
    }

    // Print snapshot summary if any were saved
    if !snapshot_results.is_empty() {
        print_snapshot_summary(&snapshot_results);
        println!();
    }

    // Re-fetch the latest downloads after snapshotting
    let secrets_to_push: Vec<(DbSecret, DbDownload)> = {
        let mut result = Vec::new();
        for (secret, _) in &secrets_to_push {
            if let Some(download) = repo.get_latest_download(secret.id)? {
                result.push((secret.clone(), download));
            }
        }
        result
    };

    // Push each secret
    let mut pushed_count = 0;
    let mut error_count = 0;

    for (secret, download) in secrets_to_push {
        let file_path = get_secret_path(&config.secrets_path(), &download.filename);

        if !file_path.exists() {
            eprintln!("Error: File not found: {}", file_path.display());
            error_count += 1;
            continue;
        }

        let content = fs::read_to_string(&file_path)?;

        // For jaws (local) secrets, show a message about future remote push capability
        if secret.provider_id == "jaws" {
            println!(
                "{} [jaws] -> Updated locally (v{})",
                secret.display_name, download.version
            );
            pushed_count += 1;
            continue;
        }

        // Find the provider
        let provider = providers
            .iter()
            .find(|p| p.id() == secret.provider_id)
            .ok_or_else(|| format!("Provider {} not found", secret.provider_id))?;

        match provider.update(&secret.api_ref, &content).await {
            Ok(result) => {
                println!(
                    "{} [{}] -> {}",
                    secret.display_name, secret.provider_id, result
                );
                pushed_count += 1;

                // Log the operation
                let _ = repo.log_operation("push", &secret.provider_id, &secret.display_name, None);
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("ResourceNotFoundException")
                    || error_msg.contains("not found")
                {
                    match provider.create(&secret.display_name, &content, None).await {
                        Ok(result) => {
                            println!(
                                "{} [{}] -> {} (created)",
                                secret.display_name, secret.provider_id, result
                            );
                            pushed_count += 1;

                            // Log the operation
                            let _ = repo.log_operation(
                                "push_create",
                                &secret.provider_id,
                                &secret.display_name,
                                None,
                            );
                        }
                        Err(create_err) => {
                            eprintln!(
                                "Error creating {} in {}: {}",
                                secret.display_name, secret.provider_id, create_err
                            );
                            error_count += 1;
                        }
                    }
                } else {
                    eprintln!(
                        "Error updating {} in {}: {}",
                        secret.display_name, secret.provider_id, e
                    );
                    error_count += 1;
                }
            }
        }
    }

    if pushed_count > 0 || error_count > 0 {
        println!(
            "\nPush complete: {} succeeded, {} failed",
            pushed_count, error_count
        );
    }

    Ok(())
}

/// Find secrets that have been modified locally (dirty)
fn find_dirty_secrets(
    config: &Config,
    downloaded: &[(DbSecret, DbDownload)],
) -> Vec<(DbSecret, DbDownload, bool)> {
    let mut results = Vec::new();

    for (secret, download) in downloaded {
        let dirty = is_dirty(config, download);
        results.push((secret.clone(), download.clone(), dirty));
    }

    results
}

/// Show TUI selector for dirty (modified) secrets
async fn select_dirty_secrets(
    config: &Config,
    downloaded: &[(DbSecret, DbDownload)],
) -> Result<Vec<(DbSecret, DbDownload)>, Box<dyn std::error::Error>> {
    use ff::{TuiConfig, create_items_channel, run_tui_with_config};

    // Find all dirty secrets
    let dirty_secrets: Vec<_> = find_dirty_secrets(config, downloaded)
        .into_iter()
        .filter(|(_, _, is_dirty)| *is_dirty)
        .map(|(s, d, _)| (s, d))
        .collect();

    if dirty_secrets.is_empty() {
        return Ok(Vec::new());
    }

    let (tx, rx) = create_items_channel();

    for (secret, _download) in &dirty_secrets {
        let display = format!(
            "{} | {} (modified)",
            secret.provider_id, secret.display_name
        );
        if tx.send(display).await.is_err() {
            break;
        }
    }
    drop(tx);

    let mut tui_config = TuiConfig::fullscreen();
    tui_config.show_help_text = false;

    // Enable multi-select for batch push
    let selected = run_tui_with_config(rx, true, tui_config)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    if selected.is_empty() {
        return Ok(Vec::new());
    }

    // Filter dirty secrets to only those selected
    let result = dirty_secrets
        .into_iter()
        .filter(|(s, _)| {
            let display = format!("{} | {} (modified)", s.provider_id, s.display_name);
            selected.iter().any(|(_, sel)| sel == &display)
        })
        .collect();

    Ok(result)
}
