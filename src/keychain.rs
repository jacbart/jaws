//! OS keychain integration for caching decrypted credentials.
//!
//! This module provides a TTL-based cache layer backed by the operating system's
//! native credential store (macOS Keychain, Linux kernel keyutils, etc.).
//!
//! The keychain is used as a **cache only** -- the age-encrypted credentials in
//! SQLite remain the source of truth. After the first successful decryption the
//! plaintext is stored in the keychain so that subsequent `jaws` invocations can
//! retrieve it without prompting the user, until the TTL expires.
//!
//! ## Namespace isolation
//!
//! Keychain entries are scoped by the **canonicalized secrets path**, so multiple
//! `jaws.kdl` configs on the same machine (with different `secrets_path` values)
//! will never collide, even if they share provider IDs.
//!
//! Entry key format: `{canonical_secrets_path}:{provider_id}/{credential_key}`
//!
//! ## Error handling
//!
//! All keychain errors are treated as non-fatal: if the keychain is unavailable
//! (e.g. headless SSH session, locked keychain) jaws silently falls back to
//! prompting.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::db::SecretRepository;

/// Service name used for all jaws keychain entries.
const SERVICE: &str = "jaws";

/// A cached credential value with a timestamp for TTL enforcement.
#[derive(Serialize, Deserialize)]
struct CachedValue {
    value: String,
    stored_at: i64, // Unix timestamp (seconds)
}

/// Canonicalize a secrets path for use as a keychain namespace.
///
/// Uses `std::fs::canonicalize` to resolve symlinks and relative paths.
/// Falls back to the string representation of the original path if
/// canonicalization fails (e.g. the directory doesn't exist yet).
fn canonical_prefix(secrets_path: &Path) -> String {
    std::fs::canonicalize(secrets_path)
        .unwrap_or_else(|_| secrets_path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

/// Build the keyring "user" field from secrets path, provider, and credential key.
///
/// Format: `{canonical_secrets_path}:{provider_id}/{credential_key}`
fn entry_user(secrets_path: &Path, provider_id: &str, credential_key: &str) -> String {
    let prefix = canonical_prefix(secrets_path);
    format!("{}:{}/{}", prefix, provider_id, credential_key)
}

/// Check whether the OS keychain backend is functional.
///
/// Performs a real round-trip probe: writes a test value, reads it back, and
/// deletes it. Returns `false` if any step fails, which catches cases where
/// the `keyring` crate falls back to a non-functional mock store (e.g. when
/// no platform-native backend is available).
pub fn keychain_available() -> bool {
    let entry = match keyring::Entry::new(SERVICE, "__jaws_probe__") {
        Ok(e) => e,
        Err(_) => return false,
    };

    let probe_value = "__jaws_probe_value__";

    if entry.set_password(probe_value).is_err() {
        return false;
    }

    let ok = entry
        .get_password()
        .map(|v| v == probe_value)
        .unwrap_or(false);

    // Always clean up the probe entry
    let _ = entry.delete_credential();

    ok
}

/// Store a decrypted credential value in the OS keychain.
///
/// Silently returns on any error (keychain locked, not available, etc.).
pub fn keychain_store(
    secrets_path: &Path,
    provider_id: &str,
    credential_key: &str,
    plaintext: &str,
) {
    let user = entry_user(secrets_path, provider_id, credential_key);
    let cached = CachedValue {
        value: plaintext.to_string(),
        stored_at: chrono::Utc::now().timestamp(),
    };
    let json = match serde_json::to_string(&cached) {
        Ok(j) => j,
        Err(_) => return,
    };
    if let Ok(entry) = keyring::Entry::new(SERVICE, &user) {
        let _ = entry.set_password(&json);
    }
}

/// Retrieve a cached credential from the OS keychain.
///
/// Returns `None` if the entry is missing, expired (older than `ttl_secs`),
/// or any keychain error occurs.
pub fn keychain_retrieve(
    secrets_path: &Path,
    provider_id: &str,
    credential_key: &str,
    ttl_secs: u64,
) -> Option<String> {
    let user = entry_user(secrets_path, provider_id, credential_key);
    let entry = keyring::Entry::new(SERVICE, &user).ok()?;
    let json = entry.get_password().ok()?;
    let cached: CachedValue = serde_json::from_str(&json).ok()?;

    // Check TTL
    let now = chrono::Utc::now().timestamp();
    let age = now - cached.stored_at;
    if age < 0 || age as u64 > ttl_secs {
        // Expired -- delete the stale entry silently
        let _ = entry.delete_credential();
        return None;
    }

    Some(cached.value)
}

/// Delete a single keychain entry for a provider credential.
///
/// Silently ignores errors (entry not found, keychain locked, etc.).
pub fn keychain_delete(secrets_path: &Path, provider_id: &str, credential_key: &str) {
    let user = entry_user(secrets_path, provider_id, credential_key);
    if let Ok(entry) = keyring::Entry::new(SERVICE, &user) {
        let _ = entry.delete_credential();
    }
}

/// Clear all jaws keychain entries for credentials known to the database.
///
/// Iterates over all providers and their stored credentials, removing each
/// corresponding keychain entry. Returns the number of entries cleared.
pub fn keychain_clear_all(secrets_path: &Path, repo: &SecretRepository) -> usize {
    let mut cleared = 0;

    match repo.get_all_stored_credential_keys() {
        Ok(keys) => {
            for (provider_id, credential_key) in keys {
                let user = entry_user(secrets_path, &provider_id, &credential_key);
                if let Ok(entry) = keyring::Entry::new(SERVICE, &user) {
                    if entry.delete_credential().is_ok() {
                        cleared += 1;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: could not read credential keys from database: {}",
                e
            );
        }
    }

    cleared
}
