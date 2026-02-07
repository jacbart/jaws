//! Auto-snapshot functionality for tracking local secret changes.
//!
//! This module provides utilities to automatically detect and version
//! changes made to secret files, ensuring all edits are tracked in history.

use std::fs;

use crate::config::Config;
use crate::db::{DbDownload, DbSecret, SecretRepository};
use crate::secrets::storage::{compute_content_hash, get_secret_path, save_secret_file};

/// Result of a snapshot check - indicates if a file was modified and versioned
#[derive(Debug)]
pub struct SnapshotResult {
    pub secret_name: String,
    pub provider_id: String,
    pub new_version: Option<i32>,
    pub was_modified: bool,
}

/// Check if a secret file has been modified and create a new version if so.
///
/// Returns `SnapshotResult` with the new version number if a snapshot was created,
/// or `None` if the file was unchanged.
pub fn check_and_snapshot(
    config: &Config,
    repo: &SecretRepository,
    secret: &DbSecret,
    download: &DbDownload,
) -> Result<SnapshotResult, Box<dyn std::error::Error>> {
    let file_path = get_secret_path(&config.secrets_path(), &download.filename);

    // Check if file exists
    if !file_path.exists() {
        return Ok(SnapshotResult {
            secret_name: secret.display_name.clone(),
            provider_id: secret.provider_id.clone(),
            new_version: None,
            was_modified: false,
        });
    }

    // Read current content and compute hash
    let content = fs::read_to_string(&file_path)?;
    let current_hash = compute_content_hash(&content);

    // Compare with stored hash
    let stored_hash = download.file_hash.as_deref().unwrap_or("");
    if current_hash == stored_hash {
        // No changes
        return Ok(SnapshotResult {
            secret_name: secret.display_name.clone(),
            provider_id: secret.provider_id.clone(),
            new_version: None,
            was_modified: false,
        });
    }

    // File was modified - create new version
    let new_version = download.version + 1;

    let (filename, content_hash) = save_secret_file(
        &config.secrets_path(),
        &secret.display_name,
        &secret.hash,
        new_version,
        &content,
    )?;

    // Record in database
    repo.create_download(secret.id, &filename, &content_hash)?;

    // Log the operation
    repo.log_operation(
        "auto_save",
        &secret.provider_id,
        &secret.display_name,
        Some(&format!("{{\"version\": {}}}", new_version)),
    )?;

    // Prune old versions if max_versions is configured
    if let Some(max) = config.max_versions() {
        prune_old_versions(config, repo, secret.id, max)?;
    }

    Ok(SnapshotResult {
        secret_name: secret.display_name.clone(),
        provider_id: secret.provider_id.clone(),
        new_version: Some(new_version),
        was_modified: true,
    })
}

/// Check if a secret file has uncommitted changes (is "dirty").
pub fn is_dirty(config: &Config, download: &DbDownload) -> bool {
    let file_path = get_secret_path(&config.secrets_path(), &download.filename);

    if !file_path.exists() {
        return false;
    }

    if let Ok(content) = fs::read_to_string(&file_path) {
        let current_hash = compute_content_hash(&content);
        let stored_hash = download.file_hash.as_deref().unwrap_or("");
        current_hash != stored_hash
    } else {
        false
    }
}

/// Snapshot all modified secrets.
///
/// Returns a list of secrets that were snapshotted with their new version numbers.
pub fn snapshot_all_modified(
    config: &Config,
    repo: &SecretRepository,
) -> Result<Vec<SnapshotResult>, Box<dyn std::error::Error>> {
    let downloaded = repo.list_all_downloaded_secrets()?;
    let mut results = Vec::new();

    for (secret, download) in downloaded {
        let result = check_and_snapshot(config, repo, &secret, &download)?;
        if result.new_version.is_some() {
            results.push(result);
        }
    }

    Ok(results)
}

/// Snapshot specific secrets by their IDs.
///
/// Returns a list of secrets that were snapshotted with their new version numbers.
pub fn snapshot_secrets(
    config: &Config,
    repo: &SecretRepository,
    secret_ids: &[i64],
) -> Result<Vec<SnapshotResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    for &secret_id in secret_ids {
        if let Some(secret) = repo.get_secret_by_id(secret_id)? {
            if let Some(download) = repo.get_latest_download(secret_id)? {
                let result = check_and_snapshot(config, repo, &secret, &download)?;
                if result.new_version.is_some() {
                    results.push(result);
                }
            }
        }
    }

    Ok(results)
}

/// Prune old versions, keeping only the most recent `max_versions`.
///
/// This deletes both the database records and the files on disk.
fn prune_old_versions(
    config: &Config,
    repo: &SecretRepository,
    secret_id: i64,
    max_versions: u32,
) -> Result<usize, Box<dyn std::error::Error>> {
    if max_versions == 0 {
        return Ok(0);
    }

    let downloads = repo.list_downloads(secret_id)?;

    if downloads.len() <= max_versions as usize {
        return Ok(0);
    }

    // Downloads are sorted by version DESC, so skip the first max_versions
    let to_delete: Vec<_> = downloads.into_iter().skip(max_versions as usize).collect();
    let delete_count = to_delete.len();

    for download in to_delete {
        // Delete the file
        let file_path = get_secret_path(&config.secrets_path(), &download.filename);
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }

        // Delete the database record
        repo.delete_download(download.id)?;
    }

    Ok(delete_count)
}

/// Print a summary of snapshot results.
pub fn print_snapshot_summary(results: &[SnapshotResult]) {
    if results.is_empty() {
        return;
    }

    if results.len() == 1 {
        let r = &results[0];
        println!(
            "Saved: {}://{} (v{})",
            r.provider_id,
            r.secret_name,
            r.new_version.unwrap_or(0)
        );
    } else {
        println!("Saved {} version(s):", results.len());
        for r in results {
            println!(
                "  {}://{} (v{})",
                r.provider_id,
                r.secret_name,
                r.new_version.unwrap_or(0)
            );
        }
    }
}
