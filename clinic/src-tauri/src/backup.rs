//! Backups and restore.
//!
//! A backup *set* is two files: the encrypted DB (`clinic_live.db`) plus
//! `auth.json` (the wrapped master keys). Both are needed to restore — the DB is
//! encrypted with a random master key that only auth.json can unlock.
//!
//! - PC-local: `<data_dir>/backups/` — `clinic_live.db` every save + last 5 daily
//!   snapshots. Always on.
//! - USB: same set under `<usb_root>/ClinicBackup/`, only when a stick whose
//!   volume serial matches the configured one (and carries the marker file) is
//!   present. Unknown sticks are ignored.

use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

const MARKER_FILE: &str = ".clinic_backup";
const USB_SUBDIR: &str = "ClinicBackup";
const KEEP_DAILY: usize = 5;

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct BackupConfig {
    usb_serial: Option<u32>,
}

fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("backup_config.json")
}

fn load_config(data_dir: &Path) -> BackupConfig {
    fs::read(config_path(data_dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_config(data_dir: &Path, cfg: &BackupConfig) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(cfg).map_err(e2s)?;
    fs::write(config_path(data_dir), json).map_err(e2s)
}

// --- writing backups --------------------------------------------------------

/// Run PC-local backups, and USB backups when the configured stick is present.
/// Best-effort: a save must never be blocked by a backup failure.
pub fn backup_all(conn: &Connection, auth_path: &Path, data_dir: &Path) {
    let audit = data_dir.join("audit.log");
    let local = data_dir.join("backups");
    let _ = write_set(conn, auth_path, &local);
    copy_if_exists(&audit, &local.join("audit.log"));
    if let Some(root) = detect_usb(data_dir) {
        let usb = root.join(USB_SUBDIR);
        let _ = write_set(conn, auth_path, &usb);
        copy_if_exists(&audit, &usb.join("audit.log"));
    }
}

fn copy_if_exists(from: &Path, to: &Path) {
    if from.exists() {
        let _ = fs::copy(from, to);
    }
}

fn write_set(conn: &Connection, auth_path: &Path, dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(e2s)?;
    let live = dir.join("clinic_live.db");
    vacuum_to(conn, &live)?;
    if auth_path.exists() {
        fs::copy(auth_path, dir.join("auth.json")).map_err(e2s)?;
    }
    let today = chrono::Local::now()
        .format("clinic_%Y-%m-%d.db")
        .to_string();
    let snapshot = dir.join(&today);
    if !snapshot.exists() {
        fs::copy(&live, &snapshot).map_err(e2s)?;
    }
    prune_daily(dir, KEEP_DAILY)?;
    Ok(())
}

/// Make a consistent encrypted copy of the open DB. VACUUM INTO requires a fresh
/// target, so we write to a temp file then swap it into place.
fn vacuum_to(conn: &Connection, final_path: &Path) -> Result<(), String> {
    let tmp = final_path.with_extension("tmp");
    let _ = fs::remove_file(&tmp);
    conn.execute("VACUUM INTO ?1", params![tmp.to_string_lossy()])
        .map_err(e2s)?;
    if final_path.exists() {
        fs::remove_file(final_path).map_err(e2s)?;
    }
    fs::rename(&tmp, final_path).map_err(e2s)?;
    Ok(())
}

/// Keep the newest `keep` daily snapshots (clinic_YYYY-MM-DD.db); delete older.
fn prune_daily(dir: &Path, keep: usize) -> Result<(), String> {
    let mut snapshots: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(e2s)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| is_daily_snapshot(p))
        .collect();
    snapshots.sort(); // date-sortable filenames -> oldest first
    if snapshots.len() > keep {
        for old in &snapshots[..snapshots.len() - keep] {
            let _ = fs::remove_file(old);
        }
    }
    Ok(())
}

fn is_daily_snapshot(p: &Path) -> bool {
    let name = match p.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };
    // clinic_YYYY-MM-DD.db
    let mid = name
        .strip_prefix("clinic_")
        .and_then(|s| s.strip_suffix(".db"));
    match mid {
        Some(d) => d.len() == 10 && d.as_bytes()[4] == b'-' && d.as_bytes()[7] == b'-',
        None => false,
    }
}

// --- USB detection / setup --------------------------------------------------

/// Designate the stick at `drive` (e.g. "E:\\") as the backup target: record its
/// volume serial and drop a marker file so only this stick is ever written to.
pub fn set_usb(data_dir: &Path, drive: &str) -> Result<(), String> {
    let serial = volume_serial(drive).ok_or("Could not read that drive")?;
    let marker = Path::new(drive).join(MARKER_FILE);
    fs::write(&marker, b"clinic backup target\n")
        .map_err(|_| "Drive is not writable".to_string())?;
    save_config(
        data_dir,
        &BackupConfig {
            usb_serial: Some(serial),
        },
    )
}

/// The root path of the configured USB if it is currently connected (serial
/// matches AND the marker file is present), else None.
pub fn detect_usb(data_dir: &Path) -> Option<PathBuf> {
    let serial = load_config(data_dir).usb_serial?;
    for drive in drive_roots() {
        if volume_serial(&drive) == Some(serial) && Path::new(&drive).join(MARKER_FILE).exists() {
            return Some(PathBuf::from(drive));
        }
    }
    None
}

#[derive(serde::Serialize)]
pub struct UsbStatus {
    pub configured: bool,
    pub connected: bool,
    pub drive: Option<String>,
}

pub fn usb_status(data_dir: &Path) -> UsbStatus {
    let configured = load_config(data_dir).usb_serial.is_some();
    match detect_usb(data_dir) {
        Some(p) => UsbStatus {
            configured,
            connected: true,
            drive: Some(p.to_string_lossy().to_string()),
        },
        None => UsbStatus {
            configured,
            connected: false,
            drive: None,
        },
    }
}

#[derive(serde::Serialize)]
pub struct DriveInfo {
    pub drive: String,
    pub label: String,
}

/// Drives the user can pick as a backup target (removable + fixed, excluding the
/// system drive's typical C:). Kept simple: list everything writable except C:.
pub fn removable_drives() -> Vec<DriveInfo> {
    drive_roots()
        .into_iter()
        .filter(|d| !d.starts_with('C') && !d.starts_with('c'))
        .map(|d| DriveInfo {
            label: volume_label(&d).unwrap_or_default(),
            drive: d,
        })
        .collect()
}

fn drive_roots() -> Vec<String> {
    ('A'..='Z')
        .map(|c| format!("{c}:\\"))
        .filter(|r| Path::new(r).exists())
        .collect()
}

// --- export / restore -------------------------------------------------------

/// Write the full backup set (DB + auth.json) into a chosen folder.
pub fn export_to(conn: &Connection, auth_path: &Path, dest_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dest_dir).map_err(e2s)?;
    vacuum_to(conn, &dest_dir.join("clinic_live.db"))?;
    if auth_path.exists() {
        fs::copy(auth_path, dest_dir.join("auth.json")).map_err(e2s)?;
    }
    Ok(())
}

/// Verify a backup folder and count its patients (preview before restoring).
pub fn restore_preview(folder: &Path, username: &str, password: &str) -> Result<i64, String> {
    let auth_path = folder.join("auth.json");
    let db_path = folder.join("clinic_live.db");
    if !auth_path.exists() || !db_path.exists() {
        return Err("That folder doesn't contain a clinic backup".into());
    }
    let store = crate::auth::AuthStore::load(&auth_path)?;
    let session = store.login(username, password)?;
    let conn = crate::db::open(&db_path, &session.master_key).map_err(e2s)?;
    conn.query_row(
        "SELECT count(*) FROM patients WHERE deleted_at IS NULL",
        [],
        |r| r.get::<_, i64>(0),
    )
    .map_err(e2s)
}

/// Replace the live data with a backup, after snapshotting the current data so a
/// wrong restore is itself reversible.
pub fn restore_apply(
    folder: &Path,
    data_dir: &Path,
    db_path: &Path,
    auth_path: &Path,
    username: &str,
    password: &str,
) -> Result<(), String> {
    // Validate first.
    restore_preview(folder, username, password)?;

    // Snapshot current state.
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let snap_dir = data_dir.join(format!("pre_restore_{stamp}"));
    fs::create_dir_all(&snap_dir).map_err(e2s)?;
    if db_path.exists() {
        fs::copy(db_path, snap_dir.join("clinic.db")).map_err(e2s)?;
    }
    if auth_path.exists() {
        fs::copy(auth_path, snap_dir.join("auth.json")).map_err(e2s)?;
    }

    // Swap in the backup.
    fs::copy(folder.join("clinic_live.db"), db_path).map_err(e2s)?;
    fs::copy(folder.join("auth.json"), auth_path).map_err(e2s)?;
    Ok(())
}

// --- windows volume info ----------------------------------------------------

#[cfg(windows)]
fn volume_serial(root: &str) -> Option<u32> {
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
    let wide: Vec<u16> = format!("{}\\", root.trim_end_matches('\\'))
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut serial: u32 = 0;
    let ok = unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            0,
            &mut serial,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };
    if ok != 0 {
        Some(serial)
    } else {
        None
    }
}

#[cfg(windows)]
fn volume_label(root: &str) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
    let wide: Vec<u16> = format!("{}\\", root.trim_end_matches('\\'))
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut buf = [0u16; 256];
    let ok = unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };
    if ok == 0 {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

#[cfg(not(windows))]
fn volume_serial(_root: &str) -> Option<u32> {
    None
}
#[cfg(not(windows))]
fn volume_label(_root: &str) -> Option<String> {
    None
}

fn e2s<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: [u8; 32] = [9u8; 32];

    #[test]
    fn write_set_creates_db_and_daily_snapshot() {
        let conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let base = std::env::temp_dir().join(format!("clinic_bkp_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let auth = base.join("auth.json"); // lives beside, not inside, the backup dir
        let dir = base.join("backups");
        fs::write(&auth, b"{}").unwrap();

        write_set(&conn, &auth, &dir).unwrap();
        assert!(dir.join("clinic_live.db").exists());
        assert!(dir.join("auth.json").exists());
        let snaps: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| is_daily_snapshot(p))
            .collect();
        assert_eq!(snaps.len(), 1);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn prune_keeps_only_newest_five() {
        let dir = std::env::temp_dir().join(format!("clinic_prune_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for d in [
            "2026-01-01",
            "2026-01-02",
            "2026-01-03",
            "2026-01-04",
            "2026-01-05",
            "2026-01-06",
            "2026-01-07",
        ] {
            fs::write(dir.join(format!("clinic_{d}.db")), b"x").unwrap();
        }
        prune_daily(&dir, 5).unwrap();
        let remaining: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| is_daily_snapshot(p))
            .collect();
        assert_eq!(remaining.len(), 5);
        // oldest two removed
        assert!(!dir.join("clinic_2026-01-01.db").exists());
        assert!(!dir.join("clinic_2026-01-02.db").exists());
        assert!(dir.join("clinic_2026-01-07.db").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
