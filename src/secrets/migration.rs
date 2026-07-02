//! One-shot migration of legacy `{name}_{hash}_{version}` files in the
//! `secrets_path` root to the new layout:
//!
//! - `secrets/{provider_id}/{name}` — user-editable working copy (latest)
//! - `.versions/{provider_id}/{name}/v{N}` — per-version archive
//!
//! Run after schema v6 applies. Idempotent: subsequent runs find no legacy
//! files and short-circuit.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::db::SecretRepository;
use crate::debug_eprintln;
use crate::error::JawsError;
use crate::secrets::storage::{
    archive_relpath, parse_legacy_filename, sanitize_filename, version_archive_path,
    working_file_path, VERSIONS_DIR, WORKING_DIR,
};

/// Walk `secrets_path` root, find any legacy `{name}_{hash}_{version}` files,
/// move them into the v6 layout, and update `downloads.filename` to match.
///
/// Returns the number of files relocated.
pub fn migrate_legacy_layout(
    secrets_path: &Path,
    repo: &SecretRepository,
) -> Result<usize, JawsError> {
    if !secrets_path.exists() {
        return Ok(0);
    }

    // Group legacy files by (display_name, hash) — the same secret may have
    // multiple versions on disk.
    #[derive(Default)]
    struct Bucket {
        files: Vec<(i32, std::path::PathBuf)>,
    }
    let mut groups: HashMap<(String, String), Bucket> = HashMap::new();

    for entry in fs::read_dir(secrets_path)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // Skip db sidecars and anything that doesn't look legacy.
        if filename == "jaws.db"
            || filename.ends_with("-journal")
            || filename.ends_with("-wal")
            || filename.ends_with("-shm")
        {
            continue;
        }
        let Some((display_name, hash, version)) = parse_legacy_filename(filename) else {
            continue;
        };
        groups
            .entry((display_name, hash))
            .or_default()
            .files
            .push((version, path));
    }

    if groups.is_empty() {
        return Ok(0);
    }

    let mut moved = 0usize;
    for ((display_name, hash), bucket) in groups {
        // Find which provider this hash belongs to. If none matches we leave
        // the file where it is — better safe than to nuke unrelated content.
        let Some(secret) = repo.get_secret_by_hash(&hash)? else {
            debug_eprintln!(
                "  jaws migrate: skipping orphaned legacy file group {} (hash {}) — no matching secrets row",
                display_name, &hash[..8.min(hash.len())]
            );
            continue;
        };
        let provider_id = secret.provider_id.clone();

        // Sort versions ascending so the highest moves last to be the working copy.
        let mut versions = bucket.files;
        versions.sort_by_key(|(v, _)| *v);
        let max_version = versions.last().map(|(v, _)| *v).unwrap_or(1);

        // Ensure target dirs exist.
        let working_dir = secrets_path
            .join(WORKING_DIR)
            .join(sanitize_filename(&provider_id));
        let archive_dir = secrets_path
            .join(VERSIONS_DIR)
            .join(sanitize_filename(&provider_id))
            .join(sanitize_filename(&display_name));
        fs::create_dir_all(&working_dir)?;
        fs::create_dir_all(&archive_dir)?;

        for (version, src) in versions.iter() {
            // Copy each version to its archive slot (copy → remove so we still
            // succeed across filesystems that disallow direct rename).
            let archive_dst = version_archive_path(secrets_path, &provider_id, &display_name, *version);
            if !archive_dst.exists() {
                fs::copy(src, &archive_dst).map_err(JawsError::from)?;
                let _ = crate::utils::restrict_file_permissions(&archive_dst);
            }
        }
        // Working file = the max version's content.
        if let Some((_, src)) = versions.iter().find(|(v, _)| *v == max_version) {
            let working_dst = working_file_path(secrets_path, &provider_id, &display_name);
            fs::copy(src, &working_dst).map_err(JawsError::from)?;
            let _ = crate::utils::restrict_file_permissions(&working_dst);
        }

        // Delete the legacy files now that copies are in place.
        for (_, src) in versions.iter() {
            let _ = fs::remove_file(src);
            moved += 1;
        }

        // downloads.filename has already been rewritten by the SQL migration
        // (see schema::rewrite_filenames_to_v6) — sanity-check that the path
        // it expects now exists.
        let downloads = repo.list_downloads(secret.id)?;
        for d in downloads {
            let expected = archive_relpath(&provider_id, &display_name, d.version);
            if d.filename != expected {
                // SQL migration should have set this, but be defensive.
                debug_eprintln!(
                    "  jaws migrate: NOTE downloads row v{} for {}://{} has filename '{}' but expected '{}'",
                    d.version, provider_id, display_name, d.filename, expected
                );
            }
        }
    }

    if moved > 0 {
        debug_eprintln!(
            "jaws: migrated {} legacy secret file(s) into the new secrets/ + .versions/ layout",
            moved
        );
    }
    Ok(moved)
}
