//! Database schema and initialization.

use rusqlite::{Connection, Result};
use std::path::Path;

const SCHEMA_VERSION: i32 = 1;

/// Initialize the database at the given path, creating tables if needed.
pub fn init_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;

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

fn get_schema_version(conn: &Connection) -> Result<i32> {
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

fn set_schema_version(conn: &Connection, version: i32) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO schema_version (id, version) VALUES (1, ?)",
        [version],
    )?;
    Ok(())
}

fn create_tables(conn: &Connection) -> Result<()> {
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
            remote_updated_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(provider_id, api_ref)
        );

        -- Downloaded versions (history)
        CREATE TABLE downloads (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            secret_id INTEGER NOT NULL REFERENCES secrets(id) ON DELETE CASCADE,
            version INTEGER NOT NULL,
            filename TEXT NOT NULL,
            downloaded_at TEXT NOT NULL DEFAULT (datetime('now')),
            file_hash TEXT,
            UNIQUE(secret_id, version)
        );

        -- Indexes for fast lookups
        CREATE INDEX idx_secrets_hash ON secrets(hash);
        CREATE INDEX idx_secrets_provider ON secrets(provider_id);
        CREATE INDEX idx_downloads_secret ON downloads(secret_id);
        "#,
    )?;
    Ok(())
}

fn migrate(conn: &Connection, from_version: i32, to_version: i32) -> Result<()> {
    // Future migrations go here
    // For now, we only have version 1
    for version in from_version..to_version {
        match version {
            0 => {
                // Initial schema - handled by create_tables
                create_tables(conn)?;
            }
            _ => {
                // Unknown version, skip
            }
        }
    }
    set_schema_version(conn, to_version)?;
    Ok(())
}
