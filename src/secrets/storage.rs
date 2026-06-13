//! File storage operations for secrets.
//!
//! Layout under `secrets_path`:
//!   - `jaws.db`                                  — SQLite metadata
//!   - `secrets/{provider_id}/{name}`             — user-editable working copy (always = latest)
//!   - `.versions/{provider_id}/{name}/v{N}`      — immutable per-version archive
//!
//! Working file and the latest version archive always have identical contents for the
//! current version; older archives preserve prior versions verbatim.

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::JawsError;

/// Length of the hash prefix used for the `secrets.hash` DB column (16 hex chars = 64 bits).
const HASH_LENGTH: usize = 16;

/// Subdirectory holding user-editable working copies.
pub const WORKING_DIR: &str = "secrets";

/// Subdirectory holding immutable per-version archives.
pub const VERSIONS_DIR: &str = ".versions";

/// Compute the hash of an API reference for use in the `secrets.hash` column.
/// Returns the first 16 characters of the hex-encoded SHA256 hash.
pub fn hash_api_ref(api_ref: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_ref.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)[..HASH_LENGTH].to_string()
}

/// Sanitize a display name for use as a filesystem path segment.
/// Replaces path separators, spaces, and special characters with underscores.
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ' ' => '_',
            ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' => c,
            _ => '_',
        })
        .collect()
}

/// Compute SHA256 hash of secret content (for change detection).
pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Absolute path to the working-copy file for a given secret.
pub fn working_file_path(secrets_path: &Path, provider_id: &str, name: &str) -> PathBuf {
    secrets_path
        .join(WORKING_DIR)
        .join(sanitize_filename(provider_id))
        .join(sanitize_filename(name))
}

/// Absolute path to a specific version archive.
pub fn version_archive_path(
    secrets_path: &Path,
    provider_id: &str,
    name: &str,
    version: i32,
) -> PathBuf {
    secrets_path
        .join(VERSIONS_DIR)
        .join(sanitize_filename(provider_id))
        .join(sanitize_filename(name))
        .join(format!("v{}", version))
}

/// Relative path (under `secrets_path`) for a version archive — what we store in
/// `downloads.filename`.
pub fn archive_relpath(provider_id: &str, name: &str, version: i32) -> String {
    format!(
        "{}/{}/{}/v{}",
        VERSIONS_DIR,
        sanitize_filename(provider_id),
        sanitize_filename(name),
        version
    )
}

/// Relative path (under `secrets_path`) for the working-copy file.
pub fn working_relpath(provider_id: &str, name: &str) -> String {
    format!(
        "{}/{}/{}",
        WORKING_DIR,
        sanitize_filename(provider_id),
        sanitize_filename(name)
    )
}

/// Resolve a relative path stored in `downloads.filename` to an absolute path.
pub fn get_secret_path(secrets_path: &Path, relpath: &str) -> PathBuf {
    secrets_path.join(relpath)
}

// ---------------------------------------------------------------------------
// I/O
// ---------------------------------------------------------------------------

/// Write a new version: stores `content` to BOTH the version archive (immutable)
/// and the working-copy file (user-editable). Both are restricted to mode 0600.
/// Returns (archive_relpath, content_hash).
pub fn write_secret_version(
    secrets_path: &Path,
    provider_id: &str,
    name: &str,
    version: i32,
    content: &str,
) -> Result<(String, String), JawsError> {
    let archive = version_archive_path(secrets_path, provider_id, name, version);
    let working = working_file_path(secrets_path, provider_id, name);

    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = working.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut f = File::create(&archive)?;
    f.write_all(content.as_bytes())?;
    crate::utils::restrict_file_permissions(&archive)?;

    let mut f = File::create(&working)?;
    f.write_all(content.as_bytes())?;
    crate::utils::restrict_file_permissions(&working)?;

    let content_hash = compute_content_hash(content);
    Ok((archive_relpath(provider_id, name, version), content_hash))
}

/// Read content from an arbitrary relative path stored in `downloads.filename`.
/// Use for reading a specific version archive.
pub fn load_secret_file(secrets_path: &Path, relpath: &str) -> Result<String, JawsError> {
    let path = secrets_path.join(relpath);
    let content = fs::read_to_string(&path)?;
    Ok(content)
}

/// Read the user-editable working file for a given secret.
pub fn read_working_file(
    secrets_path: &Path,
    provider_id: &str,
    name: &str,
) -> Result<String, JawsError> {
    let path = working_file_path(secrets_path, provider_id, name);
    let content = fs::read_to_string(&path)?;
    Ok(content)
}

/// True iff the working file exists for the given secret.
pub fn working_file_exists(secrets_path: &Path, provider_id: &str, name: &str) -> bool {
    working_file_path(secrets_path, provider_id, name).exists()
}

/// Delete the working file for a given secret (no-op if missing).
pub fn delete_working_file(
    secrets_path: &Path,
    provider_id: &str,
    name: &str,
) -> Result<(), JawsError> {
    let path = working_file_path(secrets_path, provider_id, name);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Delete every version archive for a given secret, plus the archive directory if empty.
pub fn delete_all_archives(
    secrets_path: &Path,
    provider_id: &str,
    name: &str,
) -> Result<(), JawsError> {
    let dir = secrets_path
        .join(VERSIONS_DIR)
        .join(sanitize_filename(provider_id))
        .join(sanitize_filename(name));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Scan / discovery
// ---------------------------------------------------------------------------

/// A working file discovered by `scan_working_dir`.
#[derive(Debug, Clone)]
pub struct WorkingFile {
    pub provider_id: String,
    pub display_name: String,
    pub path: PathBuf,
}

/// Walk the `secrets/{provider_id}/{name}` tree and return every working file found.
pub fn scan_working_dir(secrets_path: &Path) -> Result<Vec<WorkingFile>, JawsError> {
    let root = secrets_path.join(WORKING_DIR);
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for provider_entry in fs::read_dir(&root)? {
        let provider_entry = provider_entry?;
        if !provider_entry.file_type()?.is_dir() {
            continue;
        }
        let provider_id = provider_entry.file_name().to_string_lossy().to_string();
        for name_entry in fs::read_dir(provider_entry.path())? {
            let name_entry = name_entry?;
            if !name_entry.file_type()?.is_file() {
                continue;
            }
            let display_name = name_entry.file_name().to_string_lossy().to_string();
            out.push(WorkingFile {
                provider_id: provider_id.clone(),
                display_name,
                path: name_entry.path(),
            });
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Legacy filename parsing (used only during one-shot v6 migration)
// ---------------------------------------------------------------------------

/// Parse a legacy `{display_name}_{hash}_{version}` filename. Returns `None` if
/// the filename doesn't match. Used solely by the schema-v6 migration to detect
/// pre-v6 files lying at the root of `secrets_path`.
pub fn parse_legacy_filename(filename: &str) -> Option<(String, String, i32)> {
    let parts: Vec<&str> = filename.rsplitn(2, '_').collect();
    if parts.len() != 2 {
        return None;
    }
    let version: i32 = parts[0].parse().ok()?;
    let rest = parts[1];
    let rest_parts: Vec<&str> = rest.rsplitn(2, '_').collect();
    if rest_parts.len() != 2 {
        return None;
    }
    let hash = rest_parts[0].to_string();
    let display_name = rest_parts[1].to_string();
    if hash.len() != HASH_LENGTH {
        return None;
    }
    Some((display_name, hash, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_api_ref() {
        let hash = hash_api_ref("op://vault123/item456/field789");
        assert_eq!(hash.len(), HASH_LENGTH);
        assert_eq!(hash, hash_api_ref("op://vault123/item456/field789"));
        assert_ne!(hash, hash_api_ref("op://other/item/field"));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("simple"), "simple");
        assert_eq!(sanitize_filename("with spaces"), "with_spaces");
        assert_eq!(sanitize_filename("path/to/secret"), "path_to_secret");
        assert_eq!(
            sanitize_filename("Service Account/item/field"),
            "Service_Account_item_field"
        );
        assert_eq!(
            sanitize_filename("special:*?\"<>|chars"),
            "special_______chars"
        );
    }

    #[test]
    fn test_working_and_archive_paths() {
        let root = Path::new("/tmp/jaws-test");
        assert_eq!(
            working_file_path(root, "aws-prod", "db-password"),
            PathBuf::from("/tmp/jaws-test/secrets/aws-prod/db-password")
        );
        assert_eq!(
            version_archive_path(root, "aws-prod", "db-password", 3),
            PathBuf::from("/tmp/jaws-test/.versions/aws-prod/db-password/v3")
        );
        assert_eq!(
            archive_relpath("aws-prod", "db-password", 3),
            ".versions/aws-prod/db-password/v3"
        );
        assert_eq!(
            working_relpath("aws-prod", "db-password"),
            "secrets/aws-prod/db-password"
        );
    }

    #[test]
    fn test_write_and_read_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("jaws-storage-{}", uuid::Uuid::new_v4()));
        let (relpath, hash) =
            write_secret_version(&tmp, "jaws", "demo", 1, "hello-world").expect("write");
        assert_eq!(relpath, ".versions/jaws/demo/v1");
        assert_eq!(hash, compute_content_hash("hello-world"));
        assert_eq!(load_secret_file(&tmp, &relpath).expect("read"), "hello-world");
        assert_eq!(
            read_working_file(&tmp, "jaws", "demo").expect("read working"),
            "hello-world"
        );
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_parse_legacy_filename() {
        assert_eq!(
            parse_legacy_filename("my_secret_a1b2c3d4e5f6g7h8_1"),
            Some(("my_secret".to_string(), "a1b2c3d4e5f6g7h8".to_string(), 1))
        );
        assert_eq!(parse_legacy_filename("invalid"), None);
        assert_eq!(parse_legacy_filename("name_short_1"), None);
        assert_eq!(parse_legacy_filename("name_a1b2c3d4e5f6g7h8_notanumber"), None);
    }
}
