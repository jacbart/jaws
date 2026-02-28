//! Export and import command handlers - archiving secrets.

use std::fs;

use crate::archive::{
    export_secrets, format_size, import_secrets, prompt_passphrase, prompt_passphrase_with_confirm,
    DecryptionMethod, EncryptionMethod,
};
use crate::config::Config;

/// Handle the export command - archive and encrypt secrets
pub fn handle_export(
    config: &Config,
    ssh_key: Option<std::path::PathBuf>,
    output: Option<std::path::PathBuf>,
    delete: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let secrets_path = config.secrets_path();
    let output_path = output.unwrap_or_else(|| std::path::PathBuf::from("./jaws.barrel"));

    // Validate secrets directory exists
    if !secrets_path.exists() {
        return Err(format!(
            "Secrets directory not found: {}\nNothing to export.",
            secrets_path.display()
        )
        .into());
    }

    // Determine encryption method
    let encryption = if let Some(pubkey_path) = ssh_key {
        if !pubkey_path.exists() {
            return Err(format!("SSH public key not found: {}", pubkey_path.display()).into());
        }
        println!("Encrypting with SSH key: {}", pubkey_path.display());
        EncryptionMethod::SshPublicKey(pubkey_path)
    } else {
        // Default: passphrase
        let passphrase = prompt_passphrase_with_confirm("Enter passphrase")?;
        EncryptionMethod::Passphrase(passphrase)
    };

    // Create the archive
    let size = export_secrets(&secrets_path, &output_path, encryption)?;

    println!(
        "Exported {} to {} ({})",
        secrets_path.display(),
        output_path.display(),
        format_size(size)
    );

    // Delete original if requested
    if delete {
        fs::remove_dir_all(&secrets_path)?;
        println!("Deleted {}", secrets_path.display());
    }

    Ok(())
}

/// Handle the import command - decrypt and extract archive
pub fn handle_import(
    config: &Config,
    archive_path: &std::path::Path,
    ssh_key: Option<std::path::PathBuf>,
    delete: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let secrets_path = config.secrets_path();

    // Validate archive exists
    if !archive_path.exists() {
        return Err(format!("Archive not found: {}", archive_path.display()).into());
    }

    // Warn if secrets directory already exists
    if secrets_path.exists() {
        eprintln!(
            "Warning: {} already exists and will be overwritten",
            secrets_path.display()
        );
    }

    // Determine decryption method
    let decryption = if let Some(privkey_path) = ssh_key {
        if !privkey_path.exists() {
            return Err(format!("SSH private key not found: {}", privkey_path.display()).into());
        }
        println!("Decrypting with SSH key: {}", privkey_path.display());
        DecryptionMethod::SshPrivateKey(privkey_path)
    } else {
        // Default: passphrase
        let passphrase = prompt_passphrase("Enter passphrase")?;
        DecryptionMethod::Passphrase(passphrase)
    };

    // Import the archive
    import_secrets(archive_path, &secrets_path, decryption)?;

    println!("Imported to {}", secrets_path.display());

    // Delete archive if requested
    if delete {
        fs::remove_file(archive_path)?;
        println!("Deleted {}", archive_path.display());
    }

    Ok(())
}
