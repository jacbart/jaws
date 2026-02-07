//! List command handlers - listing secrets.

use crate::db::SecretRepository;

/// Handle the list command - print all known secrets
pub fn handle_list(
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

        for (secret, _download) in filtered {
            println!("{}://{}", secret.provider_id, secret.display_name);
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

        for secret in secrets {
            // Print in PROVIDER://NAME format
            println!("{}://{}", secret.provider_id, secret.display_name);
        }
    }

    Ok(())
}
