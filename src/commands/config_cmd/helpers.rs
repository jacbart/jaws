//! Shared helpers for interactive config setup.

use std::io::{self, Write};

use crate::config::Config;
use crate::credentials::{prompt_encryption_method, store_encrypted_credential};
use crate::db::{SecretRepository, init_db};

/// Credentials pending storage, collected during interactive config setup.
pub(super) struct PendingCredential {
    pub provider_id: String,
    pub credential_key: String,
    pub plaintext_value: String,
}

/// Read a line of input with a default value shown in brackets.
pub(super) fn prompt(message: &str, default: &str) -> String {
    print!("{} [{}]: ", message, default);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    if input.is_empty() {
        default.to_string()
    } else {
        input.to_string()
    }
}

/// y/N confirmation prompt.
pub(super) fn confirm(message: &str) -> bool {
    print!("{} [y/N]: ", message);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Encrypt and store any pending credentials collected during provider setup.
pub(super) fn store_pending_credentials(
    config: &Config,
    pending_credentials: &[PendingCredential],
) -> Result<(), Box<dyn std::error::Error>> {
    if pending_credentials.is_empty() {
        return Ok(());
    }

    println!();
    println!(
        "Encrypting {} credential(s) for secure storage...",
        pending_credentials.len()
    );

    let (method, method_tag, ssh_fingerprint) = prompt_encryption_method()?;

    let secrets_path = config.secrets_path();
    std::fs::create_dir_all(&secrets_path)?;
    let conn = init_db(&config.db_path())?;
    let repo = SecretRepository::new(conn);

    let use_keychain = config.keychain_cache();
    for cred in pending_credentials {
        match store_encrypted_credential(
            &repo,
            &cred.provider_id,
            &cred.credential_key,
            &cred.plaintext_value,
            &method,
            &method_tag,
            ssh_fingerprint.as_deref(),
            use_keychain,
            &secrets_path,
        ) {
            Ok(()) => {
                println!(
                    "  Stored encrypted {} for provider '{}'",
                    cred.credential_key, cred.provider_id
                );
            }
            Err(e) => {
                eprintln!(
                    "  Failed to store {} for '{}': {}",
                    cred.credential_key, cred.provider_id, e
                );
            }
        }
    }
    println!("Credential storage complete.");
    Ok(())
}
