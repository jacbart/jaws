//! List command handlers - listing secrets.

use crate::config::Config;
use crate::db::SecretRepository;

use super::snapshot::is_dirty;

/// Handle the list command - print all known secrets
pub fn handle_list(
    config: &Config,
    repo: &SecretRepository,
    provider: Option<String>,
    local_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if local_only {
        // Show only downloaded secrets (local copies)
        let downloaded = repo.list_all_downloaded_secrets()?;

        if downloaded.is_empty() {
            eprintln!("No local secrets found. Use 'jaws pull' to download secrets.");
            return Ok(());
        }

        let filtered: Vec<_> = if let Some(ref p) = provider {
            downloaded
                .into_iter()
                .filter(|(s, _)| &s.provider_id == p)
                .collect()
        } else {
            downloaded
        };

        for (secret, download) in filtered {
            let modified = if is_dirty(config, &download) {
                " (modified)"
            } else {
                ""
            };
            println!(
                "{}://{}{}",
                secret.provider_id, secret.display_name, modified
            );
        }
    } else {
        // Show all known secrets (from sync)
        let secrets = repo.list_all_secrets(provider.as_deref())?;

        if secrets.is_empty() {
            eprintln!(
                "No secrets found. Run 'jaws sync' first to discover secrets from providers."
            );
            return Ok(());
        }

        // Get downloaded secrets to check for modifications
        let downloaded = repo.list_all_downloaded_secrets()?;

        for secret in secrets {
            // Check if this secret is downloaded and modified
            let modified = downloaded
                .iter()
                .find(|(s, _)| s.id == secret.id)
                .map(|(_, d)| {
                    if is_dirty(config, d) {
                        " (modified)"
                    } else {
                        ""
                    }
                })
                .unwrap_or("");

            println!(
                "{}://{}{}",
                secret.provider_id, secret.display_name, modified
            );
        }
    }

    Ok(())
}
