//! Encrypted SQLite (SQLCipher) connection + schema.
//!
//! The database is opened with a 32-byte raw master key (supplied by the auth
//! layer in Phase 2). All patient data lives here and is encrypted at rest.

use rusqlite::Connection;
use std::path::Path;

/// Open (or create) an encrypted database file and run migrations.
pub fn open(path: &Path, key: &[u8; 32]) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    apply_key(&conn, key)?;
    verify_key(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

/// In-memory encrypted database — used by tests.
pub fn open_in_memory(key: &[u8; 32]) -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    apply_key(&conn, key)?;
    migrate(&conn)?;
    Ok(conn)
}

/// Set the SQLCipher key as a raw hex key (no KDF — the key is already a random
/// master key, wrapped per-user at the auth layer).
fn apply_key(conn: &Connection, key: &[u8; 32]) -> rusqlite::Result<()> {
    let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    conn.execute_batch(&format!("PRAGMA key = \"x'{hex}'\";"))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(())
}

/// Touch the DB so a wrong key fails immediately (SQLCipher errors on first read).
fn verify_key(conn: &Connection) -> rusqlite::Result<()> {
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))?;
    Ok(())
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS patients (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            card_first       INTEGER NOT NULL,
            card_sub         INTEGER NOT NULL,
            first_name       TEXT NOT NULL,
            father_name      TEXT NOT NULL,
            grandfather_name TEXT NOT NULL,
            sex              TEXT NOT NULL CHECK (sex IN ('Male','Female')),
            phone            TEXT NOT NULL,
            dob_year         INTEGER,
            dob_month        INTEGER,
            dob_day          INTEGER,
            age_recorded     INTEGER,
            age_recorded_on  TEXT,
            address          TEXT,
            city             TEXT,
            registered_at    TEXT NOT NULL,
            created_by       TEXT NOT NULL,
            updated_at       TEXT,
            updated_by       TEXT,
            deleted_at       TEXT,
            deleted_by       TEXT,
            UNIQUE (card_first, card_sub)
        );

        CREATE INDEX IF NOT EXISTS idx_patients_names
            ON patients (first_name, father_name, grandfather_name);
        CREATE INDEX IF NOT EXISTS idx_patients_phone ON patients (phone);

        -- Single-row counter for the next card number to issue. Seeded to 1/0.
        CREATE TABLE IF NOT EXISTS card_seq (
            id         INTEGER PRIMARY KEY CHECK (id = 1),
            next_first INTEGER NOT NULL,
            next_sub   INTEGER NOT NULL
        );
        INSERT OR IGNORE INTO card_seq (id, next_first, next_sub) VALUES (1, 1, 0);
        "#,
    )
}
