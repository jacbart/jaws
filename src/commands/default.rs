//! Default command handler - when no subcommand is provided.

use std::process::Command;

use clap::CommandFactory;

use crate::cli::Cli;
use crate::config::Config;
use crate::db::SecretRepository;
use crate::secrets::get_secret_path;

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

    // Collect all selected file paths
    let mut files_to_open: Vec<String> = Vec::new();
    for selected_display in &selected {
        for (secret, download) in &downloaded {
            let display = format!("{} | {}", secret.provider_id, secret.display_name);
            if &display == selected_display {
                let file_path = get_secret_path(&config.secrets_path(), &download.filename);
                files_to_open.push(file_path.to_string_lossy().to_string());
                break;
            }
        }
    }

    if !files_to_open.is_empty() {
        let _ = Command::new(config.editor())
            .args(&files_to_open)
            .status()
            .expect("failed to launch editor");
    }

    Ok(())
}
