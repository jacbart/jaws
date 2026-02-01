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

    // Count files in secrets directory (excluding db files)
    let file_count = if secrets_path.exists() {
        fs::read_dir(&secrets_path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        let name = e.file_name();
                        let name_str = name.to_string_lossy();
                        !name_str.ends_with(".db")
                            && !name_str.ends_with("-journal")
                            && !name_str.ends_with("-wal")
                            && !name_str.ends_with("-shm")
                    })
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };

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

        // Delete files for remote secrets
        for (secret, download) in &remote_downloads {
            let file_path = secrets_path.join(&download.filename);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    eprintln!("Warning: Failed to delete {}: {}", file_path.display(), e);
                } else {
                    deleted_files += 1;
                }
            }
            // Also delete any older versions
            for dl in repo.list_downloads(secret.id).unwrap_or_default() {
                let old_path = secrets_path.join(&dl.filename);
                if old_path.exists() && old_path != file_path {
                    let _ = fs::remove_file(&old_path);
                }
            }
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

        // Delete all secret files (not the db yet)
        let mut deleted_files = 0;
        if secrets_path.exists() {
            for entry in fs::read_dir(&secrets_path)? {
                let entry = entry?;
                let path = entry.path();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Skip database files for now
                if name.ends_with(".db")
                    || name.ends_with("-journal")
                    || name.ends_with("-wal")
                    || name.ends_with("-shm")
                {
                    continue;
                }

                if path.is_file() {
                    if let Err(e) = fs::remove_file(&path) {
                        eprintln!("Warning: Failed to delete {}: {}", path.display(), e);
                    } else {
                        deleted_files += 1;
                    }
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
