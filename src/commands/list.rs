//! List command handlers - listing secrets.

use crate::db::SecretRepository;

/// Handle the list command - print all known secrets
pub fn handle_list(
    repo: &SecretRepository,
    provider: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let secrets = repo.list_all_secrets(provider.as_deref())?;

    if secrets.is_empty() {
        eprintln!("No secrets found. Run 'jaws sync' first to discover secrets from providers.");
        return Ok(());
    }

    for secret in secrets {
        // Print in PROVIDER://NAME format
        println!("{}://{}", secret.provider_id, secret.display_name);
    }

    Ok(())
}
