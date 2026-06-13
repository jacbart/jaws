//! Auto-snapshot helpers — surface unsynced edits to the working file as new
//! versions in the local DB + `.versions/` archive.
//!
//! Under the v6 folder-first layout, the **working file** at
//! `secrets/{provider_id}/{display_name}` is the source of truth for the
//! current value. These helpers detect divergence between that working file's
//! hash and the latest `downloads.file_hash`, and record a new version when
//! they differ.

use chrono::Utc;

use crate::config::Config;
use crate::db::{DbDownload, DbSecret, SecretRepository};
use crate::secrets::storage::{
    archive_relpath, compute_content_hash, read_working_file, version_archive_path,
    working_file_exists, write_secret_version,
};

/// Result of a snapshot check.
#[derive(Debug)]
pub struct SnapshotResult {
    pub secret_name: String,
    pub provider_id: String,
    pub new_version: Option<i32>,
    pub was_modified: bool,
}

/// If the working file has been edited since `download` was created, write a
/// new version and return its number.
pub fn check_and_snapshot(
    config: &Config,
    repo: &SecretRepository,
    secret: &DbSecret,
    download: &DbDownload,
) -> Result<SnapshotResult, Box<dyn std::error::Error>> {
    if !working_file_exists(&config.secrets_path(), &secret.provider_id, &secret.display_name) {
        return Ok(SnapshotResult {
            secret_name: secret.display_name.clone(),
            provider_id: secret.provider_id.clone(),
            new_version: None,
            was_modified: false,
        });
    }

    let content = read_working_file(
        &config.secrets_path(),
        &secret.provider_id,
        &secret.display_name,
    )?;
    let current_hash = compute_content_hash(&content);
    let stored_hash = download.file_hash.as_deref().unwrap_or("");
    if current_hash == stored_hash {
        return Ok(SnapshotResult {
            secret_name: secret.display_name.clone(),
            provider_id: secret.provider_id.clone(),
            new_version: None,
            was_modified: false,
        });
    }

    let new_version = download.version + 1;
    let (relpath, content_hash) = write_secret_version(
        &config.secrets_path(),
        &secret.provider_id,
        &secret.display_name,
        new_version,
        &content,
    )?;

    // Local jaws has no remote; remote providers are picked up by `jaws push`.
    let pushed_at = if secret.provider_id == "jaws" {
        Some(Utc::now())
    } else {
        None
    };
    repo.create_download(secret.id, &relpath, &content_hash, pushed_at)?;

    repo.log_operation(
        "auto_save",
        &secret.provider_id,
        &secret.display_name,
        Some(&format!("{{\"version\": {}}}", new_version)),
    )?;

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

/// True iff the working file's hash differs from the latest recorded hash.
pub fn is_dirty(config: &Config, secret: &DbSecret, download: &DbDownload) -> bool {
    if !working_file_exists(&config.secrets_path(), &secret.provider_id, &secret.display_name) {
        return false;
    }
    match read_working_file(
        &config.secrets_path(),
        &secret.provider_id,
        &secret.display_name,
    ) {
        Ok(content) => {
            let current_hash = compute_content_hash(&content);
            let stored_hash = download.file_hash.as_deref().unwrap_or("");
            current_hash != stored_hash
        }
        Err(_) => false,
    }
}

/// Snapshot every modified working file.
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
pub fn snapshot_secrets(
    config: &Config,
    repo: &SecretRepository,
    secret_ids: &[i64],
) -> Result<Vec<SnapshotResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    for &secret_id in secret_ids {
        if let Some(secret) = repo.get_secret_by_id(secret_id)?
            && let Some(download) = repo.get_latest_download(secret_id)?
        {
            let result = check_and_snapshot(config, repo, &secret, &download)?;
            if result.new_version.is_some() {
                results.push(result);
            }
        }
    }

    Ok(results)
}

/// Prune `.versions/{provider}/{name}/v{N}` archives older than `max_versions`,
/// deleting both the archive file and the DB row.
pub(crate) fn prune_old_versions(
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

    let secret = repo
        .get_secret_by_id(secret_id)?
        .ok_or("Secret missing while pruning")?;
    let to_delete: Vec<_> = downloads.into_iter().skip(max_versions as usize).collect();
    let delete_count = to_delete.len();

    for download in to_delete {
        let archive = version_archive_path(
            &config.secrets_path(),
            &secret.provider_id,
            &secret.display_name,
            download.version,
        );
        if archive.exists() {
            let _ = std::fs::remove_file(&archive);
        }
        // Fallback path lookup for rows that still carry an old relpath form.
        let expected = archive_relpath(&secret.provider_id, &secret.display_name, download.version);
        if download.filename != expected {
            let alt = config.secrets_path().join(&download.filename);
            if alt.exists() {
                let _ = std::fs::remove_file(&alt);
            }
        }
        repo.delete_download(download.id)?;
    }

    Ok(delete_count)
}

/// Print a one-line summary per snapshotted secret.
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
