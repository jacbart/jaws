//! Default command handler - when no subcommand is provided.

use std::process::Command;

use clap::CommandFactory;

use crate::cli::Cli;
use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::get_secret_path;

use super::snapshot::{print_snapshot_summary, snapshot_secrets};

/// Handle the default command (no subcommand) - show picker for downloaded secrets to edit
pub async fn handle_default_command(
    config: &Config,
    repo: &SecretRepository,
) -> Result<(), Box<dyn std::error::Error>> {
    let downloaded = repo.list_all_downloaded_secrets()?;

    if downloaded.is_empty() {
        // Show help if no secrets downloaded
        Cli::command().print_help()?;
        println!(); // Add newline after help
        return Ok(());
    }

    // Build list of display names for the picker
    use ff::{TuiConfig, create_items_channel, run_tui_with_config};

    let (tx, rx) = create_items_channel();

    // Send all downloaded secrets to the picker
    for (secret, _download) in &downloaded {
        let display = format!("{} | {}", secret.provider_id, secret.display_name);
        if tx.send(display).await.is_err() {
            break;
        }
    }
    drop(tx);

    let mut tui_config = TuiConfig::fullscreen();
    tui_config.show_help_text = false;

    let selected = run_tui_with_config(rx, true, tui_config)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    if selected.is_empty() {
        return Ok(());
    }

    // Collect all selected file paths and secret IDs
    let mut files_to_open: Vec<String> = Vec::new();
    let mut selected_secret_ids: Vec<i64> = Vec::new();

    for (_, selected_display) in &selected {
        for (secret, download) in &downloaded {
            let display = format!("{} | {}", secret.provider_id, secret.display_name);
            if &display == selected_display {
                let file_path = get_secret_path(&config.secrets_path(), &download.filename);
                files_to_open.push(file_path.to_string_lossy().to_string());
                selected_secret_ids.push(secret.id);
                break;
            }
        }
    }

    if !files_to_open.is_empty() {
        // Open editor
        let _ = Command::new(config.editor())
            .args(&files_to_open)
            .status()
            .expect("failed to launch editor");

        // After editor closes, check for modifications and auto-snapshot
        let results = snapshot_secrets(config, repo, &selected_secret_ids)?;

        // Print summary of saved versions
        print_snapshot_summary(&results);
    }

    Ok(())
}
