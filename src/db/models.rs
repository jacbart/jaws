//! Database model structs representing table rows.

use chrono::{DateTime, Utc};

/// A provider configured in jaws.hcl
#[derive(Debug, Clone)]
pub struct DbProvider {
    pub id: String,
    pub kind: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub config_json: Option<String>,
}

/// A known secret (may or may not be downloaded locally)
#[derive(Debug, Clone)]
pub struct DbSecret {
    pub id: i64,
    pub provider_id: String,
    pub api_ref: String,
    pub display_name: String,
    pub hash: String,
    pub description: Option<String>,
    pub remote_updated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// A downloaded version of a secret
#[derive(Debug, Clone)]
pub struct DbDownload {
    pub id: i64,
    pub secret_id: i64,
    pub version: i32,
    /// Relative path under `secrets_path` to this version's archive
    /// (e.g. `.versions/aws-prod/db-password/v3`).
    pub filename: String,
    pub downloaded_at: DateTime<Utc>,
    pub file_hash: Option<String>,
    /// Non-NULL once this row's content has been synced to the remote provider.
    /// NULL means the local edit is pending push. For the local "jaws" provider
    /// this is stamped equal to `downloaded_at` (no remote exists).
    pub pushed_at: Option<DateTime<Utc>>,
}

/// Input for creating/updating a secret (without auto-generated fields)
#[derive(Debug, Clone)]
pub struct SecretInput {
    pub provider_id: String,
    pub api_ref: String,
    pub display_name: String,
    pub hash: String,
    pub description: Option<String>,
    pub remote_updated_at: Option<DateTime<Utc>>,
}

/// An operation log entry
#[derive(Debug, Clone)]
pub struct DbOperation {
    pub id: i64,
    pub operation_type: String,
    pub provider_id: String,
    pub secret_name: String,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// An encrypted credential stored for a provider.
/// Used to persist authentication tokens (e.g., 1Password service account token,
/// Bitwarden access token, AWS long-lived keys) encrypted with age.
#[derive(Debug, Clone)]
pub struct StoredCredential {
    pub id: i64,
    pub provider_id: String,
    /// Key identifying this credential, e.g. "token", "access_key_id", "secret_access_key"
    pub credential_key: String,
    /// The age-encrypted credential value
    pub encrypted_value: Vec<u8>,
    /// Encryption method used: "passphrase" or "ssh"
    pub encryption_method: String,
    /// For SSH encryption, the path/fingerprint of the public key used
    pub ssh_pubkey_fingerprint: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
