//! Database model structs representing table rows.

use chrono::{DateTime, Utc};

/// A provider configured in jaws.kdl
#[derive(Debug, Clone)]
pub struct DbProvider {
    pub id: String,
    pub kind: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub remote_updated_at: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

/// A downloaded version of a secret
#[derive(Debug, Clone)]
pub struct DbDownload {
    #[allow(dead_code)]
    pub id: i64,
    #[allow(dead_code)]
    pub secret_id: i64,
    pub version: i32,
    pub filename: String,
    #[allow(dead_code)]
    pub downloaded_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub file_hash: Option<String>,
}

/// Input for creating/updating a secret (without auto-generated fields)
#[derive(Debug, Clone)]
pub struct SecretInput {
    pub provider_id: String,
    pub api_ref: String,
    pub display_name: String,
    pub hash: String,
    pub remote_updated_at: Option<DateTime<Utc>>,
}
