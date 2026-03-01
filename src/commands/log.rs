//! Log command handlers - viewing operation history and per-secret version history.

use crate::config::Config;
use crate::db::SecretRepository;
use crate::utils::parse_secret_ref;

use super::snapshot::is_dirty;

/// Handle the log command.
///
/// - If `secret_name` is provided, show version history for that specific secret.
/// - Otherwise, show the global operation log.
pub fn handle_log(
    config: &Config,
    repo: &SecretRepository,
    secret_name: Option<String>,
    limit: Option<usize>,
    provider: Option<String>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(name) = secret_name {
        show_secret_versions(config, repo, &name, verbose, limit)
    } else {
        show_operation_log(repo, limit, provider)
    }
}

/// Show the global operation log across all secrets.
fn show_operation_log(
    repo: &SecretRepository,
    limit: Option<usize>,
    provider: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use chrono_humanize::HumanTime;

    let operations = repo.list_operations(limit, provider.as_deref())?;

    if operations.is_empty() {
        println!("No operations recorded yet.");
        return Ok(());
    }

    println!("Operation log:");
    for op in operations {
        let age = HumanTime::from(op.created_at);
        let details = op.details.as_deref().unwrap_or("");
        println!(
            "  {} | {:8} | {:12} | {} {}",
            age, op.operation_type, op.provider_id, op.secret_name, details
        );
    }

    Ok(())
}

/// Show version history for a specific secret.
fn show_secret_versions(
    config: &Config,
    repo: &SecretRepository,
    name: &str,
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

    // Find matching secrets
    let selected_secrets: Vec<_> =
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
        };

    if selected_secrets.is_empty() {
        println!("No matching secrets found for '{}'", name);
        return Ok(());
    }

    // Show history for each matching secret
    for (secret, latest_download) in selected_secrets {
        let downloads = repo.list_downloads(secret.id)?;

        if downloads.is_empty() {
            println!("{}: No download history", secret.display_name);
            continue;
        }

        println!("\n{}://{}", secret.provider_id, secret.display_name);
        println!(
            "{}",
            "-".repeat((secret.provider_id.len() + secret.display_name.len() + 3).min(60))
        );

        // Check for uncommitted changes
        if is_dirty(config, &latest_download) {
            println!("  (uncommitted changes)");
        }

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
