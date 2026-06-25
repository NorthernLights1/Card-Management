//! External plain-text audit log.
//!
//! Append-only `audit.log` next to the database. Readable without the app and
//! intact even if the DB has trouble. Logs **identifiers only** (username,
//! action, card number) — never patient names or phone, since this file is
//! unencrypted on purpose.

use std::io::Write;
use std::path::Path;

/// `timestamp | user | role | action | target`. Best-effort; never fails a save.
pub fn log(data_dir: &Path, user: &str, role: &str, action: &str, target: &str) {
    let line = format!(
        "{} | {} | {} | {} | {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        user,
        role,
        action,
        target,
    );
    let path = data_dir.join("audit.log");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
}
