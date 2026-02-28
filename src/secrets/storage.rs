//! File storage operations for secrets.
//!
//! Secrets are stored as flat files with names in the format:
//! `{sanitized_display_name}_{hash}_{version}` where hash is the first 16 characters of SHA256(api_ref).

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Length of the hash prefix used for filenames (16 hex chars = 64 bits).
const HASH_LENGTH: usize = 16;

/// Compute the hash of an API reference for use in filenames.
/// Returns the first 16 characters of the hex-encoded SHA256 hash.
pub fn hash_api_ref(api_ref: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_ref.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)[..HASH_LENGTH].to_string()
}

/// Sanitize a display name for use in filenames.
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

/// Generate a filename for a secret with the given display name, hash and version.
pub fn secret_filename(display_name: &str, hash: &str, version: i32) -> String {
    let sanitized = sanitize_filename(display_name);
    format!("{}_{}_{}", sanitized, hash, version)
}

/// Parse a filename to extract display name, hash, and version.
/// Expected format: `{display_name}_{hash}_{version}`
/// Returns None if the filename doesn't match the expected format.
pub fn parse_filename(filename: &str) -> Option<(String, String, i32)> {
    // Split from the right to get version first
    let parts: Vec<&str> = filename.rsplitn(2, '_').collect();
    if parts.len() != 2 {
        return None;
    }
    let version: i32 = parts[0].parse().ok()?;
    let rest = parts[1]; // display_name_hash

    // Split again to get hash (last 16 chars before the version separator)
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

/// Save a secret to the filesystem.
/// Returns (filename, content_hash).
pub fn save_secret_file(
    secrets_path: &Path,
    display_name: &str,
    hash: &str,
    version: i32,
    content: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    // Ensure directory exists
    fs::create_dir_all(secrets_path)?;

    let filename = secret_filename(display_name, hash, version);
    let file_path = secrets_path.join(&filename);

    let mut file = File::create(&file_path)?;
    file.write_all(content.as_bytes())?;

    // Restrict permissions to owner-only (0600) since this contains secret data
    crate::utils::restrict_file_permissions(&file_path)?;

    let content_hash = compute_content_hash(content);

    Ok((filename, content_hash))
}

/// Load a secret from the filesystem.
pub fn load_secret_file(
    secrets_path: &Path,
    filename: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let file_path = secrets_path.join(filename);
    let content = fs::read_to_string(&file_path)?;
    Ok(content)
}

/// Get the full path to a secret file.
pub fn get_secret_path(secrets_path: &Path, filename: &str) -> PathBuf {
    secrets_path.join(filename)
}

/// Compute SHA256 hash of content (for detecting changes).
pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Check if a secret file exists.
pub fn secret_file_exists(secrets_path: &Path, filename: &str) -> bool {
    secrets_path.join(filename).exists()
}

/// Delete a secret file.
pub fn delete_secret_file(
    secrets_path: &Path,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = secrets_path.join(filename);
    if file_path.exists() {
        fs::remove_file(&file_path)?;
    }
    Ok(())
}

/// List all secret files in the secrets directory.
/// Returns a list of (display_name, hash, version, filename) tuples.
pub fn list_secret_files(
    secrets_path: &Path,
) -> Result<Vec<(String, String, i32, String)>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    if !secrets_path.exists() {
        return Ok(results);
    }

    for entry in fs::read_dir(secrets_path)? {
        let entry = entry?;
        let path = entry.path();

        // Skip directories and the database file
        if path.is_dir() {
            continue;
        }

        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Skip the database file
            if filename == "jaws.db" || filename.ends_with("-journal") || filename.ends_with("-wal")
            {
                continue;
            }

            if let Some((display_name, hash, version)) = parse_filename(filename) {
                results.push((display_name, hash, version, filename.to_string()));
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_api_ref() {
        let hash = hash_api_ref("op://vault123/item456/field789");
        assert_eq!(hash.len(), HASH_LENGTH);
        // Same input should produce same hash
        assert_eq!(hash, hash_api_ref("op://vault123/item456/field789"));
        // Different input should produce different hash
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
    fn test_secret_filename() {
        let filename = secret_filename("my_secret", "a1b2c3d4e5f6g7h8", 1);
        assert_eq!(filename, "my_secret_a1b2c3d4e5f6g7h8_1");

        let filename = secret_filename("path/to/secret", "a1b2c3d4e5f6g7h8", 42);
        assert_eq!(filename, "path_to_secret_a1b2c3d4e5f6g7h8_42");
    }

    #[test]
    fn test_parse_filename() {
        assert_eq!(
            parse_filename("my_secret_a1b2c3d4e5f6g7h8_1"),
            Some(("my_secret".to_string(), "a1b2c3d4e5f6g7h8".to_string(), 1))
        );
        assert_eq!(
            parse_filename("path_to_secret_a1b2c3d4e5f6g7h8_42"),
            Some((
                "path_to_secret".to_string(),
                "a1b2c3d4e5f6g7h8".to_string(),
                42
            ))
        );
        // Invalid formats
        assert_eq!(parse_filename("invalid"), None);
        assert_eq!(parse_filename("name_short_1"), None); // hash too short
        assert_eq!(parse_filename("name_a1b2c3d4e5f6g7h8_notanumber"), None);
    }
}
