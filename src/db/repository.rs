//! Repository for database CRUD operations.

use super::models::{DbDownload, DbProvider, DbSecret, SecretInput};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
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
            INSERT INTO secrets (provider_id, api_ref, display_name, hash, remote_updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(provider_id, api_ref) DO UPDATE SET
                display_name = excluded.display_name,
                hash = excluded.hash,
                remote_updated_at = excluded.remote_updated_at
            "#,
            params![
                secret.provider_id,
                secret.api_ref,
                secret.display_name,
                secret.hash,
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
    #[allow(dead_code)]
    pub fn get_secret_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<DbSecret>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                r#"
                SELECT id, provider_id, api_ref, display_name, hash, remote_updated_at, created_at
                FROM secrets WHERE hash = ?
                "#,
                [hash],
                |row| Self::row_to_secret(row),
            )
            .optional()?;
        Ok(result)
    }

    /// Get a secret by provider ID and API reference.
    #[allow(dead_code)]
    pub fn get_secret_by_api_ref(
        &self,
        provider_id: &str,
        api_ref: &str,
    ) -> Result<Option<DbSecret>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                r#"
                SELECT id, provider_id, api_ref, display_name, hash, remote_updated_at, created_at
                FROM secrets WHERE provider_id = ? AND api_ref = ?
                "#,
                params![provider_id, api_ref],
                |row| Self::row_to_secret(row),
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
            SELECT id, provider_id, api_ref, display_name, hash, remote_updated_at, created_at
            FROM secrets WHERE provider_id = ?
            ORDER BY display_name
            "#,
        )?;

        let secrets = stmt
            .query_map([provider_id], |row| Self::row_to_secret(row))?
            .collect::<Result<Vec<_>, _>>()?;

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
                s.id, s.provider_id, s.api_ref, s.display_name, s.hash, s.remote_updated_at, s.created_at,
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
                    remote_updated_at: row
                        .get::<_, Option<String>>(5)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    created_at: row
                        .get::<_, String>(6)
                        .ok()
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now),
                };
                let download = DbDownload {
                    id: row.get(7)?,
                    secret_id: row.get(8)?,
                    version: row.get(9)?,
                    filename: row.get(10)?,
                    downloaded_at: row
                        .get::<_, String>(11)
                        .ok()
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now),
                    file_hash: row.get(12)?,
                };
                Ok((secret, download))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
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
                |row| Self::row_to_download(row),
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
            .query_map([secret_id], |row| Self::row_to_download(row))?
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
                |row| Self::row_to_download(row),
            )
            .optional()?;
        Ok(result)
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
                SELECT id, provider_id, api_ref, display_name, hash, remote_updated_at, created_at
                FROM secrets WHERE id = ?
                "#,
                [id],
                |row| Self::row_to_secret(row),
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
    // Helper functions
    // ========================================================================

    fn row_to_secret(row: &rusqlite::Row) -> rusqlite::Result<DbSecret> {
        Ok(DbSecret {
            id: row.get(0)?,
            provider_id: row.get(1)?,
            api_ref: row.get(2)?,
            display_name: row.get(3)?,
            hash: row.get(4)?,
            remote_updated_at: row
                .get::<_, Option<String>>(5)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            created_at: row
                .get::<_, String>(6)
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
}

impl Clone for SecretRepository {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}
