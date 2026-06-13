//! Database schema and initialization.

use rusqlite::Connection;
use std::path::Path;

use crate::error::JawsError;

const SCHEMA_VERSION: i32 = 6;

/// Initialize the database at the given path, creating tables if needed.
pub fn init_db(path: &Path) -> Result<Connection, JawsError> {
    let conn = Connection::open(path)?;

    // Restrict database file permissions to owner-only (contains encrypted credentials)
    let _ = crate::utils::restrict_file_permissions(path);

    // Enable foreign keys
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    // Check schema version
    let version = get_schema_version(&conn)?;

    if version == 0 {
        // Fresh database, create all tables
        create_tables(&conn)?;
        set_schema_version(&conn, SCHEMA_VERSION)?;
    } else if version < SCHEMA_VERSION {
        // Run migrations
        migrate(&conn, version, SCHEMA_VERSION)?;
    }

    Ok(conn)
}

fn get_schema_version(conn: &Connection) -> rusqlite::Result<i32> {
    // Check if schema_version table exists
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version')",
        [],
        |row| row.get(0),
    )?;

    if !exists {
        return Ok(0);
    }

    conn.query_row("SELECT version FROM schema_version", [], |row| row.get(0))
}

fn set_schema_version(conn: &Connection, version: i32) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO schema_version (id, version) VALUES (1, ?)",
        [version],
    )?;
    Ok(())
}

fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        -- Schema version tracking
        CREATE TABLE schema_version (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            version INTEGER NOT NULL
        );

        -- Providers configured in jaws.kdl
        CREATE TABLE providers (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            last_sync_at TEXT,
            config_json TEXT
        );

        -- All known secrets (remote + local)
        CREATE TABLE secrets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
            api_ref TEXT NOT NULL,
            display_name TEXT NOT NULL,
            hash TEXT NOT NULL,
            description TEXT,
            remote_updated_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(provider_id, api_ref)
        );

        -- Downloaded versions (history). `filename` is the relative path under
        -- secrets_path for this version's immutable archive
        -- (e.g. ".versions/aws-prod/db-password/v3"). `pushed_at` is non-NULL once
        -- the row's content has been synced to the remote provider; NULL means
        -- the local edit is still pending push. For the local "jaws" provider,
        -- `pushed_at` is stamped equal to `downloaded_at` (no remote exists).
        CREATE TABLE downloads (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            secret_id INTEGER NOT NULL REFERENCES secrets(id) ON DELETE CASCADE,
            version INTEGER NOT NULL,
            filename TEXT NOT NULL,
            downloaded_at TEXT NOT NULL DEFAULT (datetime('now')),
            file_hash TEXT,
            pushed_at TEXT,
            UNIQUE(secret_id, version)
        );

        -- Indexes for fast lookups
        CREATE INDEX idx_secrets_hash ON secrets(hash);
        CREATE INDEX idx_secrets_provider ON secrets(provider_id);
        CREATE INDEX idx_secrets_provider_name ON secrets(provider_id, display_name);
        CREATE INDEX idx_downloads_secret ON downloads(secret_id);
        CREATE INDEX idx_downloads_unpushed ON downloads(pushed_at) WHERE pushed_at IS NULL;

        -- Operation log for tracking all secret operations
        CREATE TABLE operations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            operation_type TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            secret_name TEXT NOT NULL,
            details TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX idx_operations_created ON operations(created_at DESC);
        CREATE INDEX idx_operations_provider ON operations(provider_id);

        -- Encrypted credentials for provider authentication tokens
        CREATE TABLE credentials (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id TEXT NOT NULL,
            credential_key TEXT NOT NULL,
            encrypted_value BLOB NOT NULL,
            encryption_method TEXT NOT NULL,
            ssh_pubkey_fingerprint TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(provider_id, credential_key)
        );

        CREATE INDEX idx_credentials_provider ON credentials(provider_id);

        -- Enrolled clients (server mode)
        CREATE TABLE clients (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            cert_fingerprint TEXT NOT NULL UNIQUE,
            cert_pem TEXT NOT NULL,
            issued_at TEXT NOT NULL DEFAULT (datetime('now')),
            revoked INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX idx_clients_name ON clients(name);
        CREATE INDEX idx_clients_fingerprint ON clients(cert_fingerprint);

        -- Enrollment tokens (server mode)
        CREATE TABLE enrollment_tokens (
            token TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            used INTEGER NOT NULL DEFAULT 0,
            used_by_client_id TEXT
        );
        "#,
    )?;
    Ok(())
}

fn migrate(conn: &Connection, from_version: i32, to_version: i32) -> rusqlite::Result<()> {
    for version in from_version..to_version {
        match version {
            0 => {
                // Initial schema - handled by create_tables
                create_tables(conn)?;
            }
            1 => {
                // Migration from v1 to v2:
                // - Add description column to secrets
                // - Add operations table
                conn.execute("ALTER TABLE secrets ADD COLUMN description TEXT", [])?;
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS operations (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        operation_type TEXT NOT NULL,
                        provider_id TEXT NOT NULL,
                        secret_name TEXT NOT NULL,
                        details TEXT,
                        created_at TEXT NOT NULL DEFAULT (datetime('now'))
                    );

                    CREATE INDEX IF NOT EXISTS idx_operations_created ON operations(created_at DESC);
                    CREATE INDEX IF NOT EXISTS idx_operations_provider ON operations(provider_id);
                    "#,
                )?;
            }
            2 => {
                // Migration from v2 to v3:
                // - Add credentials table for encrypted provider auth tokens
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS credentials (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        provider_id TEXT NOT NULL,
                        credential_key TEXT NOT NULL,
                        encrypted_value BLOB NOT NULL,
                        encryption_method TEXT NOT NULL,
                        ssh_pubkey_fingerprint TEXT,
                        created_at TEXT NOT NULL DEFAULT (datetime('now')),
                        updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                        UNIQUE(provider_id, credential_key)
                    );

                    CREATE INDEX IF NOT EXISTS idx_credentials_provider ON credentials(provider_id);
                    "#,
                )?;
            }
            3 => {
                // Migration from v3 to v4:
                // - Add clients table for enrolled remote clients (server mode)
                // - Add enrollment_tokens table for one-time enrollment tokens
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS clients (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL,
                        cert_fingerprint TEXT NOT NULL UNIQUE,
                        cert_pem TEXT NOT NULL,
                        issued_at TEXT NOT NULL DEFAULT (datetime('now')),
                        revoked INTEGER NOT NULL DEFAULT 0
                    );

                    CREATE INDEX IF NOT EXISTS idx_clients_name ON clients(name);
                    CREATE INDEX IF NOT EXISTS idx_clients_fingerprint ON clients(cert_fingerprint);

                    CREATE TABLE IF NOT EXISTS enrollment_tokens (
                        token TEXT PRIMARY KEY,
                        created_at TEXT NOT NULL,
                        expires_at TEXT NOT NULL,
                        used INTEGER NOT NULL DEFAULT 0,
                        used_by_client_id TEXT
                    );
                    "#,
                )?;
            }
            4 => {
                // Migration from v4 to v5:
                // - Add filename column to downloads (was missing in early schemas)
                conn.execute(
                    "ALTER TABLE downloads ADD COLUMN filename TEXT NOT NULL DEFAULT ''",
                    [],
                )?;
            }
            5 => {
                // Migration from v5 to v6:
                // - Add downloads.pushed_at: distinguishes "saved locally" from
                //   "uploaded to remote".  For migration, treat every existing
                //   row as already pushed (it was uploaded under the old model
                //   that never had a split between save and push).
                // - Add idx_secrets_provider_name and idx_downloads_unpushed.
                // - Rewrite downloads.filename from the legacy
                //   `{name}_{hash}_{version}` form to the new relative-archive
                //   path `.versions/{provider}/{name}/v{N}`. The actual file
                //   moves on disk are handled by `secrets::migration` after
                //   `init_db` returns.
                conn.execute("ALTER TABLE downloads ADD COLUMN pushed_at TEXT", [])?;
                conn.execute(
                    "UPDATE downloads SET pushed_at = downloaded_at WHERE pushed_at IS NULL",
                    [],
                )?;
                conn.execute_batch(
                    r#"
                    CREATE INDEX IF NOT EXISTS idx_secrets_provider_name
                        ON secrets(provider_id, display_name);
                    CREATE INDEX IF NOT EXISTS idx_downloads_unpushed
                        ON downloads(pushed_at) WHERE pushed_at IS NULL;
                    "#,
                )?;
                rewrite_filenames_to_v6(conn)?;
            }
            _ => {
                // Unknown version, skip
            }
        }
    }
    set_schema_version(conn, to_version)?;
    Ok(())
}

/// Rewrite every `downloads.filename` from the legacy `{name}_{hash}_{version}`
/// form (a bare filename) to the new `.versions/{provider}/{name}/v{N}` relative
/// archive path. Used by the v5→v6 migration. Idempotent.
fn rewrite_filenames_to_v6(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        r#"
        SELECT d.id, d.version, s.provider_id, s.display_name
        FROM downloads d
        JOIN secrets s ON s.id = d.secret_id
        "#,
    )?;
    let rows: Vec<(i64, i32, String, String)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);

    for (id, version, provider_id, display_name) in rows {
        let relpath = format!(
            "{}/{}/{}/v{}",
            crate::secrets::storage::VERSIONS_DIR,
            crate::secrets::storage::sanitize_filename(&provider_id),
            crate::secrets::storage::sanitize_filename(&display_name),
            version
        );
        conn.execute(
            "UPDATE downloads SET filename = ?1 WHERE id = ?2",
            rusqlite::params![relpath, id],
        )?;
    }
    Ok(())
}
