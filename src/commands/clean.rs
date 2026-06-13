//! Clean command handlers - clearing local cache.

use std::fs;
use std::io::{self, Write};

use crate::config::Config;
use crate::db::SecretRepository;

/// Handle the clean command - clear local cache and secrets
pub fn handle_clean(
    config: &Config,
    repo: &SecretRepository,
    force: bool,
    dry_run: bool,
    keep_local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let secrets_path = config.secrets_path();

    // Gather information about what exists
    let all_secrets = repo.list_all_secrets(None)?;
    let jaws_secrets: Vec<_> = all_secrets
        .iter()
        .filter(|s| s.provider_id == "jaws")
        .collect();
    let remote_secrets: Vec<_> = all_secrets
        .iter()
        .filter(|s| s.provider_id != "jaws")
        .collect();

    // Get all downloaded files info
    let downloaded = repo.list_all_downloaded_secrets().unwrap_or_default();
    let jaws_downloads: Vec<_> = downloaded
        .iter()
        .filter(|(s, _)| s.provider_id == "jaws")
        .collect();
    let remote_downloads: Vec<_> = downloaded
        .iter()
        .filter(|(s, _)| s.provider_id != "jaws")
        .collect();

    // Count working files + version archives.
    let file_count = count_secret_files(&secrets_path);

    // Nothing to clean?
    if all_secrets.is_empty() && file_count == 0 {
        println!("Nothing to clean. Secrets directory is already empty.");
        return Ok(());
    }

    // Display summary
    println!("Secrets directory: {}", secrets_path.display());
    println!();

    if keep_local {
        println!("Mode: --keep-local (preserving local jaws secrets)");
        println!();
        println!("Will delete:");
        println!(
            "  - {} remote provider secret(s) from database",
            remote_secrets.len()
        );
        println!("  - {} remote secret file(s)", remote_downloads.len());
        println!();
        println!("Will keep:");
        println!("  - {} local jaws secret(s)", jaws_secrets.len());
        println!("  - {} local jaws file(s)", jaws_downloads.len());
    } else {
        println!("Will delete:");
        println!(
            "  - {} secret(s) from database ({} local jaws, {} remote)",
            all_secrets.len(),
            jaws_secrets.len(),
            remote_secrets.len()
        );
        println!("  - {} secret file(s)", file_count);
        println!("  - Database file (jaws.db)");

        // Warn about jaws secrets
        if !jaws_secrets.is_empty() {
            println!();
            println!(
                "WARNING: {} local jaws secret(s) will be PERMANENTLY deleted:",
                jaws_secrets.len()
            );
            for secret in &jaws_secrets {
                println!("  - jaws://{}", secret.display_name);
            }
            println!();
            println!(
                "These secrets are stored locally and cannot be recovered from any remote provider!"
            );
        }
    }

    // Dry run - stop here
    if dry_run {
        println!();
        println!("(dry run - no changes made)");
        return Ok(());
    }

    // Confirm if jaws secrets exist and not forcing
    if !jaws_secrets.is_empty() && !keep_local && !force {
        println!();
        print!("Type 'yes' to confirm deletion: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() != "yes" {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Perform cleanup
    if keep_local {
        // Only delete remote provider secrets
        let mut deleted_files = 0;

        // Delete working file + every archive for every remote secret
        for (secret, _download) in &remote_downloads {
            let working = crate::secrets::storage::working_file_path(
                &secrets_path,
                &secret.provider_id,
                &secret.display_name,
            );
            if working.exists() {
                let _ = fs::remove_file(&working);
                deleted_files += 1;
            }
            let downloads = repo.list_downloads(secret.id).unwrap_or_default();
            deleted_files += downloads.len();
            let _ = crate::secrets::storage::delete_all_archives(
                &secrets_path,
                &secret.provider_id,
                &secret.display_name,
            );
        }

        // Delete remote secrets from database (by each non-jaws provider)
        let providers: std::collections::HashSet<_> = remote_secrets
            .iter()
            .map(|s| s.provider_id.as_str())
            .collect();
        let mut deleted_secrets = 0;
        for provider_id in providers {
            deleted_secrets += repo.delete_secrets_by_provider(provider_id)?;
        }

        println!();
        println!(
            "Deleted {} remote secret(s) and {} file(s).",
            deleted_secrets, deleted_files
        );
        println!("Kept {} local jaws secret(s).", jaws_secrets.len());
    } else {
        // Full cleanup - delete everything

        let mut deleted_files = 0;
        for dir in [
            secrets_path.join(crate::secrets::storage::WORKING_DIR),
            secrets_path.join(crate::secrets::storage::VERSIONS_DIR),
        ] {
            if dir.exists() {
                deleted_files += count_recursive(&dir);
                let _ = fs::remove_dir_all(&dir);
            }
        }
        // Also sweep any stray legacy files at the root.
        if secrets_path.exists() {
            for entry in fs::read_dir(&secrets_path)? {
                let entry = entry?;
                let path = entry.path();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if name.ends_with(".db")
                    || name.ends_with("-journal")
                    || name.ends_with("-wal")
                    || name.ends_with("-shm")
                {
                    continue;
                }
                if path.is_file() {
                    let _ = fs::remove_file(&path);
                    deleted_files += 1;
                }
            }
        }

        // Clear database
        let deleted_secrets = repo.delete_all_secrets()?;

        println!();
        println!(
            "Deleted {} secret(s) and {} file(s).",
            deleted_secrets, deleted_files
        );
    }

    Ok(())
}

fn count_secret_files(secrets_path: &std::path::Path) -> usize {
    let mut total = 0;
    for dir in [
        secrets_path.join(crate::secrets::storage::WORKING_DIR),
        secrets_path.join(crate::secrets::storage::VERSIONS_DIR),
    ] {
        total += count_recursive(&dir);
    }
    // Include any stray legacy files at the root.
    if let Ok(entries) = fs::read_dir(secrets_path) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().is_file()
                && !name.ends_with(".db")
                && !name.ends_with("-journal")
                && !name.ends_with("-wal")
                && !name.ends_with("-shm")
            {
                total += 1;
            }
        }
    }
    total
}

fn count_recursive(dir: &std::path::Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                total += count_recursive(&path);
            } else {
                total += 1;
            }
        }
    }
    total
}
