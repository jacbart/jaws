//! History command handlers - viewing secret version history.

use crate::config::Config;
use crate::db::SecretRepository;
use crate::utils::parse_secret_ref;

/// Handle the history command - show version history for downloaded secrets
pub async fn handle_history(
    _config: &Config,
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

        println!("\n{}", secret.display_name);
        println!("{}", "-".repeat(secret.display_name.len().min(60)));

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
