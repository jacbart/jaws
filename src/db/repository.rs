//! Repository for database CRUD operations.

use super::models::{DbDownload, DbOperation, DbProvider, DbSecret, SecretInput, StoredCredential};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::{Arc, Mutex};

/// Repository for managing secrets in the database.
pub struct SecretRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SecretRepository {
    /// Create a new repository with the given connection.
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    // ========================================================================
    // Provider operations
    // ========================================================================

    /// Insert or update a provider.
    pub fn upsert_provider(&self, provider: &DbProvider) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            r#"
            INSERT INTO providers (id, kind, last_sync_at, config_json)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                last_sync_at = excluded.last_sync_at,
                config_json = excluded.config_json
            "#,
            params![
                provider.id,
                provider.kind,
                provider.last_sync_at.map(|dt| dt.to_rfc3339()),
                provider.config_json,
            ],
        )?;
        Ok(())
    }

    /// Get a provider by ID.
    pub fn get_provider(&self, id: &str) -> Result<Option<DbProvider>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                "SELECT id, kind, last_sync_at, config_json FROM providers WHERE id = ?",
                [id],
                |row| {
                    Ok(DbProvider {
                        id: row.get(0)?,
                        kind: row.get(1)?,
                        last_sync_at: row
                            .get::<_, Option<String>>(2)?
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc)),
                        config_json: row.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(result)
    }

    /// Update the last sync time for a provider to now.
    pub fn update_provider_sync_time(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE providers SET last_sync_at = ? WHERE id = ?",
            params![now, id],
        )?;
        Ok(())
    }

    // ========================================================================
    // Secret operations
    // ========================================================================

    /// Insert or update a secret. Returns the secret ID.
    pub fn upsert_secret(&self, secret: &SecretInput) -> Result<i64, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            r#"
            INSERT INTO secrets (provider_id, api_ref, display_name, hash, description, remote_updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(provider_id, api_ref) DO UPDATE SET
                display_name = excluded.display_name,
                hash = excluded.hash,
                description = COALESCE(excluded.description, secrets.description),
                remote_updated_at = excluded.remote_updated_at
            "#,
            params![
                secret.provider_id,
                secret.api_ref,
                secret.display_name,
                secret.hash,
                secret.description,
                secret.remote_updated_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;

        // Get the ID (either newly inserted or existing)
        let id: i64 = conn.query_row(
            "SELECT id FROM secrets WHERE provider_id = ? AND api_ref = ?",
            params![secret.provider_id, secret.api_ref],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    /// Get a secret by its hash.
    pub fn get_secret_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<DbSecret>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                r#"
                SELECT id, provider_id, api_ref, display_name, hash, description, remote_updated_at, created_at
                FROM secrets WHERE hash = ?
                "#,
                [hash],
                Self::row_to_secret,
            )
            .optional()?;
        Ok(result)
    }

    /// Get a secret by provider ID and API reference.
    pub fn get_secret_by_api_ref(
        &self,
        provider_id: &str,
        api_ref: &str,
    ) -> Result<Option<DbSecret>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                r#"
                SELECT id, provider_id, api_ref, display_name, hash, description, remote_updated_at, created_at
                FROM secrets WHERE provider_id = ? AND api_ref = ?
                "#,
                params![provider_id, api_ref],
                Self::row_to_secret,
            )
            .optional()?;
        Ok(result)
    }

    /// List all secrets for a provider.
    pub fn list_secrets_by_provider(
        &self,
        provider_id: &str,
    ) -> Result<Vec<DbSecret>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, provider_id, api_ref, display_name, hash, description, remote_updated_at, created_at
            FROM secrets WHERE provider_id = ?
            ORDER BY display_name
            "#,
        )?;

        let secrets = stmt
            .query_map([provider_id], Self::row_to_secret)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(secrets)
    }

    /// Find a secret by provider ID and display_name.
    pub fn find_secret_by_provider_and_name(
        &self,
        provider_id: &str,
        display_name: &str,
    ) -> Result<Option<DbSecret>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                r#"
                SELECT id, provider_id, api_ref, display_name, hash, description, remote_updated_at, created_at
                FROM secrets WHERE provider_id = ? AND display_name = ?
                "#,
                params![provider_id, display_name],
                Self::row_to_secret,
            )
            .optional()?;
        Ok(result)
    }

    /// List all known secrets, optionally filtered by provider.
    /// Returns all secrets from the database regardless of download status.
    pub fn list_all_secrets(
        &self,
        provider_id: Option<&str>,
    ) -> Result<Vec<DbSecret>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let secrets = if let Some(provider) = provider_id {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, provider_id, api_ref, display_name, hash, description, remote_updated_at, created_at
                FROM secrets WHERE provider_id = ?
                ORDER BY provider_id, display_name
                "#,
            )?;
            stmt.query_map([provider], Self::row_to_secret)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, provider_id, api_ref, display_name, hash, description, remote_updated_at, created_at
                FROM secrets
                ORDER BY provider_id, display_name
                "#,
            )?;
            stmt.query_map([], Self::row_to_secret)?
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(secrets)
    }

    /// List all downloaded secrets with their latest download info.
    pub fn list_all_downloaded_secrets(
        &self,
    ) -> Result<Vec<(DbSecret, DbDownload)>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                s.id, s.provider_id, s.api_ref, s.display_name, s.hash, s.description, s.remote_updated_at, s.created_at,
                d.id, d.secret_id, d.version, d.filename, d.downloaded_at, d.file_hash
            FROM secrets s
            INNER JOIN downloads d ON s.id = d.secret_id
            WHERE d.version = (SELECT MAX(version) FROM downloads WHERE secret_id = s.id)
            ORDER BY s.display_name
            "#,
        )?;

        let results = stmt
            .query_map([], |row| {
                let secret = DbSecret {
                    id: row.get(0)?,
                    provider_id: row.get(1)?,
                    api_ref: row.get(2)?,
                    display_name: row.get(3)?,
                    hash: row.get(4)?,
                    description: row.get(5)?,
                    remote_updated_at: row
                        .get::<_, Option<String>>(6)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    created_at: row
                        .get::<_, String>(7)
                        .ok()
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now),
                };
                let download = DbDownload {
                    id: row.get(8)?,
                    secret_id: row.get(9)?,
                    version: row.get(10)?,
                    filename: row.get(11)?,
                    downloaded_at: row
                        .get::<_, String>(12)
                        .ok()
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now),
                    file_hash: row.get(13)?,
                };
                Ok((secret, download))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Delete all secrets and their downloads for a specific provider.
    /// Returns the number of secrets deleted.
    pub fn delete_secrets_by_provider(
        &self,
        provider_id: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // First delete all downloads for secrets of this provider
        conn.execute(
            r#"
            DELETE FROM downloads WHERE secret_id IN (
                SELECT id FROM secrets WHERE provider_id = ?
            )
            "#,
            [provider_id],
        )?;

        // Then delete the secrets themselves
        let deleted = conn.execute("DELETE FROM secrets WHERE provider_id = ?", [provider_id])?;

        Ok(deleted)
    }

    /// Delete all secrets and downloads (full reset).
    /// Returns the number of secrets deleted.
    pub fn delete_all_secrets(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Delete all downloads first (foreign key constraint)
        conn.execute("DELETE FROM downloads", [])?;

        // Delete all secrets
        let deleted = conn.execute("DELETE FROM secrets", [])?;

        // Delete all operations
        conn.execute("DELETE FROM operations", [])?;

        Ok(deleted)
    }

    // ========================================================================
    // Download operations
    // ========================================================================

    /// Create a new download record. Returns the created download.
    pub fn create_download(
        &self,
        secret_id: i64,
        filename: &str,
        file_hash: &str,
    ) -> Result<DbDownload, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // Get next version number
        let next_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM downloads WHERE secret_id = ?",
                [secret_id],
                |row| row.get(0),
            )
            .unwrap_or(1);

        let now = Utc::now();
        conn.execute(
            r#"
            INSERT INTO downloads (secret_id, version, filename, downloaded_at, file_hash)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                secret_id,
                next_version,
                filename,
                now.to_rfc3339(),
                file_hash,
            ],
        )?;

        let id = conn.last_insert_rowid();

        Ok(DbDownload {
            id,
            secret_id,
            version: next_version,
            filename: filename.to_string(),
            downloaded_at: now,
            file_hash: Some(file_hash.to_string()),
        })
    }

    /// Get the latest download for a secret.
    pub fn get_latest_download(
        &self,
        secret_id: i64,
    ) -> Result<Option<DbDownload>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                r#"
                SELECT id, secret_id, version, filename, downloaded_at, file_hash
                FROM downloads 
                WHERE secret_id = ?
                ORDER BY version DESC
                LIMIT 1
                "#,
                [secret_id],
                Self::row_to_download,
            )
            .optional()?;
        Ok(result)
    }

    /// List all downloads for a secret.
    pub fn list_downloads(
        &self,
        secret_id: i64,
    ) -> Result<Vec<DbDownload>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, secret_id, version, filename, downloaded_at, file_hash
            FROM downloads WHERE secret_id = ?
            ORDER BY version DESC
            "#,
        )?;

        let downloads = stmt
            .query_map([secret_id], Self::row_to_download)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(downloads)
    }

    /// Get a specific download by version number.
    pub fn get_download_by_version(
        &self,
        secret_id: i64,
        version: i32,
    ) -> Result<Option<DbDownload>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                r#"
                SELECT id, secret_id, version, filename, downloaded_at, file_hash
                FROM downloads 
                WHERE secret_id = ? AND version = ?
                "#,
                params![secret_id, version],
                Self::row_to_download,
            )
            .optional()?;
        Ok(result)
    }

    /// Delete a specific download record by ID.
    pub fn delete_download(&self, download_id: i64) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM downloads WHERE id = ?", [download_id])?;
        Ok(())
    }

    /// Get a secret by its ID.
    pub fn get_secret_by_id(
        &self,
        id: i64,
    ) -> Result<Option<DbSecret>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                r#"
                SELECT id, provider_id, api_ref, display_name, hash, description, remote_updated_at, created_at
                FROM secrets WHERE id = ?
                "#,
                [id],
                Self::row_to_secret,
            )
            .optional()?;
        Ok(result)
    }

    /// Delete a secret and all its download records.
    pub fn delete_secret(&self, id: i64) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        // Downloads are deleted via CASCADE (foreign key constraint)
        conn.execute("DELETE FROM secrets WHERE id = ?", [id])?;
        Ok(())
    }

    // ========================================================================
    // Operation log
    // ========================================================================

    /// Log an operation for auditing/history purposes.
    pub fn log_operation(
        &self,
        operation_type: &str,
        provider_id: &str,
        secret_name: &str,
        details: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            r#"
            INSERT INTO operations (operation_type, provider_id, secret_name, details)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![operation_type, provider_id, secret_name, details],
        )?;
        Ok(())
    }

    /// List operations, optionally filtered by provider and limited.
    pub fn list_operations(
        &self,
        limit: Option<usize>,
        provider_filter: Option<&str>,
    ) -> Result<Vec<DbOperation>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let query = if provider_filter.is_some() {
            format!(
                r#"
                SELECT id, operation_type, provider_id, secret_name, details, created_at
                FROM operations
                WHERE provider_id = ?1
                ORDER BY created_at DESC
                LIMIT {}
                "#,
                limit.unwrap_or(100)
            )
        } else {
            format!(
                r#"
                SELECT id, operation_type, provider_id, secret_name, details, created_at
                FROM operations
                ORDER BY created_at DESC
                LIMIT {}
                "#,
                limit.unwrap_or(100)
            )
        };

        let mut stmt = conn.prepare(&query)?;

        let operations = if let Some(provider) = provider_filter {
            stmt.query_map([provider], Self::row_to_operation)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], Self::row_to_operation)?
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(operations)
    }

    // ========================================================================
    // Credential operations (encrypted provider auth tokens)
    // ========================================================================

    /// Store or update an encrypted credential for a provider.
    pub fn store_credential(
        &self,
        provider_id: &str,
        credential_key: &str,
        encrypted_value: &[u8],
        encryption_method: &str,
        ssh_pubkey_fingerprint: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO credentials (provider_id, credential_key, encrypted_value, encryption_method, ssh_pubkey_fingerprint, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
            ON CONFLICT(provider_id, credential_key) DO UPDATE SET
                encrypted_value = excluded.encrypted_value,
                encryption_method = excluded.encryption_method,
                ssh_pubkey_fingerprint = excluded.ssh_pubkey_fingerprint,
                updated_at = excluded.updated_at
            "#,
            params![
                provider_id,
                credential_key,
                encrypted_value,
                encryption_method,
                ssh_pubkey_fingerprint,
                now,
            ],
        )?;
        Ok(())
    }

    /// Get all stored credentials for a provider.
    pub fn get_credentials(
        &self,
        provider_id: &str,
    ) -> Result<Vec<StoredCredential>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, provider_id, credential_key, encrypted_value, encryption_method, ssh_pubkey_fingerprint, created_at, updated_at
            FROM credentials WHERE provider_id = ?
            ORDER BY credential_key
            "#,
        )?;

        let creds = stmt
            .query_map([provider_id], |row| {
                Ok(StoredCredential {
                    id: row.get(0)?,
                    provider_id: row.get(1)?,
                    credential_key: row.get(2)?,
                    encrypted_value: row.get(3)?,
                    encryption_method: row.get(4)?,
                    ssh_pubkey_fingerprint: row.get(5)?,
                    created_at: row
                        .get::<_, String>(6)
                        .ok()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now),
                    updated_at: row
                        .get::<_, String>(7)
                        .ok()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(creds)
    }

    /// Get all (provider_id, credential_key) pairs across all providers.
    ///
    /// Used by keychain cache clearing to enumerate entries that may exist
    /// in the OS credential store.
    pub fn get_all_stored_credential_keys(
        &self,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT provider_id, credential_key FROM credentials ORDER BY provider_id, credential_key",
        )?;
        let keys = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(keys)
    }

    /// Delete all stored credentials for a provider.
    pub fn delete_credentials(
        &self,
        provider_id: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let deleted = conn.execute(
            "DELETE FROM credentials WHERE provider_id = ?",
            [provider_id],
        )?;
        Ok(deleted)
    }

    // ========================================================================
    // Helper functions
    // ========================================================================

    fn row_to_secret(row: &rusqlite::Row) -> rusqlite::Result<DbSecret> {
        Ok(DbSecret {
            id: row.get(0)?,
            provider_id: row.get(1)?,
            api_ref: row.get(2)?,
            display_name: row.get(3)?,
            hash: row.get(4)?,
            description: row.get(5)?,
            remote_updated_at: row
                .get::<_, Option<String>>(6)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            created_at: row
                .get::<_, String>(7)
                .ok()
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
        })
    }

    fn row_to_download(row: &rusqlite::Row) -> rusqlite::Result<DbDownload> {
        Ok(DbDownload {
            id: row.get(0)?,
            secret_id: row.get(1)?,
            version: row.get(2)?,
            filename: row.get(3)?,
            downloaded_at: row
                .get::<_, String>(4)
                .ok()
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            file_hash: row.get(5)?,
        })
    }

    fn row_to_operation(row: &rusqlite::Row) -> rusqlite::Result<DbOperation> {
        Ok(DbOperation {
            id: row.get(0)?,
            operation_type: row.get(1)?,
            provider_id: row.get(2)?,
            secret_name: row.get(3)?,
            details: row.get(4)?,
            created_at: row
                .get::<_, String>(5)
                .ok()
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
        })
    }
}

impl Clone for SecretRepository {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}
