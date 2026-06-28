//! Hardware-locked licensing and 14-day trial management.
//!
//! Device ID  : motherboard serial (wmic) → fallback to Windows MachineGuid
//! License key: SHA256(device_id) → first 20 hex chars → XXXXX-XXXXX-XXXXX-XXXXX
//! Trial      : start date written to HKCU\Software\ClinicApp\ on first run,
//!              protected by an integrity hash so the date cannot be edited solo.

use chrono::{Local, NaiveDate};
use sha2::{Digest, Sha256};

#[cfg(windows)]
use winreg::{enums::*, RegKey};

const REG_PATH: &str = "Software\\ClinicApp";
const TRIAL_DAYS: i64 = 14;

/// Serial values treated as absent (OEM default / blank).
const GENERIC_SERIALS: &[&str] = &[
    "",
    "to be filled by o.e.m.",
    "default string",
    "none",
    "not applicable",
    "n/a",
    "unknown",
];

#[derive(serde::Serialize, Debug, Clone)]
#[serde(tag = "status")]
pub enum LicenseStatus {
    Licensed,
    Trial { days_remaining: i64 },
    Expired,
}

// ── Crypto helpers ────────────────────────────────────────────────────────────

fn sha256_hex(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    format!("{:x}", h.finalize())
}

/// SHA256(device_id) → first 20 hex chars → XXXXX-XXXXX-XXXXX-XXXXX
pub fn make_key(device_id: &str) -> String {
    let hex = sha256_hex(device_id);
    let raw: String = hex.chars().take(20).collect::<String>().to_uppercase();
    raw.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && i % 5 == 0 {
                vec!['-', c]
            } else {
                vec![c]
            }
        })
        .collect()
}

fn normalize_key(k: &str) -> String {
    k.trim().to_uppercase().replace('-', "")
}

// ── Device ID ────────────────────────────────────────────────────────────────

#[cfg(windows)]
fn get_mobo_serial() -> Option<String> {
    let out = std::process::Command::new("wmic")
        .args(["baseboard", "get", "serialnumber", "/value"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if let Some(serial) = line.strip_prefix("SerialNumber=") {
            let s = serial.trim();
            if !GENERIC_SERIALS.contains(&s.to_lowercase().as_str()) {
                return Some(s.to_string());
            }
        }
    }
    None
}

#[cfg(windows)]
fn get_machine_guid() -> Option<String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey("SOFTWARE\\Microsoft\\Cryptography")
        .ok()?;
    key.get_value("MachineGuid").ok()
}

/// The stable device identifier used for key generation and validation.
pub fn get_device_id() -> String {
    #[cfg(windows)]
    {
        get_mobo_serial()
            .or_else(get_machine_guid)
            .unwrap_or_else(|| "UNKNOWN".to_string())
    }
    #[cfg(not(windows))]
    {
        "UNKNOWN".to_string()
    }
}

// ── Registry helpers (Windows only) ──────────────────────────────────────────

#[cfg(windows)]
fn reg_app() -> std::io::Result<RegKey> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.create_subkey(REG_PATH).map(|(k, _)| k)
}

#[cfg(windows)]
fn read_reg_value(name: &str) -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(REG_PATH).ok()?;
    key.get_value(name).ok()
}

#[cfg(windows)]
fn write_reg_value(name: &str, value: &str) -> Result<(), String> {
    reg_app()
        .and_then(|k| k.set_value(name, &value.to_string()))
        .map_err(|e| e.to_string())
}

// ── License activation ────────────────────────────────────────────────────────

/// Validate the key against this device and persist it in the registry.
pub fn activate(key: &str) -> Result<(), String> {
    let device_id = get_device_id();
    let expected = make_key(&device_id);
    if normalize_key(key) != normalize_key(&expected) {
        return Err("Invalid license key for this device.".to_string());
    }
    #[cfg(windows)]
    write_reg_value("license", key.trim())?;
    Ok(())
}

// ── Trial management ──────────────────────────────────────────────────────────

fn trial_integrity(device_id: &str, start_date: &str, last_seen_date: &str) -> String {
    sha256_hex(&format!("{device_id}|{start_date}|{last_seen_date}"))
}

#[cfg(windows)]
fn legacy_trial_integrity(device_id: &str, date_str: &str) -> String {
    sha256_hex(&format!("{device_id}|{date_str}"))
}

#[cfg(windows)]
struct TrialRecord {
    start_date: String,
    last_seen_date: String,
}

#[cfg(windows)]
fn read_trial(device_id: &str) -> Result<Option<TrialRecord>, String> {
    let Some(val) = read_reg_value("trial") else {
        return Ok(None);
    };
    let parts: Vec<&str> = val.split('|').collect();
    match parts.as_slice() {
        [start_date, last_seen_date, stored_hash]
            if *stored_hash == trial_integrity(device_id, start_date, last_seen_date) =>
        {
            Ok(Some(TrialRecord {
                start_date: (*start_date).to_string(),
                last_seen_date: (*last_seen_date).to_string(),
            }))
        }
        [start_date, stored_hash]
            if *stored_hash == legacy_trial_integrity(device_id, start_date) =>
        {
            Ok(Some(TrialRecord {
                start_date: (*start_date).to_string(),
                last_seen_date: (*start_date).to_string(),
            }))
        }
        _ => Err("Trial integrity check failed.".to_string()),
    }
}

#[cfg(windows)]
fn write_trial(start_date: &str, last_seen_date: &str, device_id: &str) -> Result<(), String> {
    let integrity = trial_integrity(device_id, start_date, last_seen_date);
    write_reg_value("trial", &format!("{start_date}|{last_seen_date}|{integrity}"))
}

fn trial_status_for_elapsed_days(elapsed: i64) -> LicenseStatus {
    if elapsed < 0 {
        LicenseStatus::Expired
    } else if elapsed < TRIAL_DAYS {
        LicenseStatus::Trial {
            days_remaining: TRIAL_DAYS - elapsed,
        }
    } else {
        LicenseStatus::Expired
    }
}

fn trial_status_for_dates(
    start_date: NaiveDate,
    last_seen_date: NaiveDate,
    today: NaiveDate,
) -> LicenseStatus {
    if today < last_seen_date {
        return LicenseStatus::Expired;
    }
    trial_status_for_elapsed_days((today - start_date).num_days())
}

// ── Status check ──────────────────────────────────────────────────────────────

/// Called on every app launch before the login screen is shown.
pub fn check_status() -> Result<LicenseStatus, String> {
    let device_id = get_device_id();

    // 1. Valid stored license key?
    #[cfg(windows)]
    if let Some(stored) = read_reg_value("license") {
        if normalize_key(&stored) == normalize_key(&make_key(&device_id)) {
            return Ok(LicenseStatus::Licensed);
        }
    }

    // 2. Trial
    let today = Local::now().date_naive();
    let today_str = today.format("%Y-%m-%d").to_string();

    #[cfg(windows)]
    {
        match read_trial(&device_id) {
            None => {
                // First run — start the trial clock
                write_trial(&today_str, &today_str, &device_id)?;
                Ok(LicenseStatus::Trial {
                    days_remaining: TRIAL_DAYS,
                })
            }
            Some(record) => {
                let start = NaiveDate::parse_from_str(&record.start_date, "%Y-%m-%d")
                    .map_err(|e| e.to_string())?;
                let last_seen = NaiveDate::parse_from_str(&record.last_seen_date, "%Y-%m-%d")
                    .map_err(|e| e.to_string())?;
                if today < last_seen {
                    return Ok(LicenseStatus::Expired);
                }
                write_trial(&record.start_date, &today_str, &device_id)?;
                Ok(trial_status_for_dates(start, last_seen, today))
            }
            Err(_) => {
                // Integrity check — detect registry date tampering
                Ok(LicenseStatus::Expired)
            }
        }
    }

    #[cfg(not(windows))]
    Ok(LicenseStatus::Trial {
        days_remaining: TRIAL_DAYS,
    })
}

#[cfg(test)]
mod tests {
    use super::{trial_status_for_dates, trial_status_for_elapsed_days, LicenseStatus};
    use chrono::NaiveDate;

    fn trial_days_remaining(status: LicenseStatus) -> Option<i64> {
        match status {
            LicenseStatus::Trial { days_remaining } => Some(days_remaining),
            LicenseStatus::Licensed | LicenseStatus::Expired => None,
        }
    }

    #[test]
    fn trial_starts_with_full_days_remaining() {
        assert_eq!(trial_days_remaining(trial_status_for_elapsed_days(0)), Some(14));
    }

    #[test]
    fn trial_allows_last_valid_day() {
        assert_eq!(trial_days_remaining(trial_status_for_elapsed_days(13)), Some(1));
    }

    #[test]
    fn trial_expires_at_fourteen_elapsed_days() {
        assert!(matches!(trial_status_for_elapsed_days(14), LicenseStatus::Expired));
    }

    #[test]
    fn trial_expires_when_clock_moves_before_start_date() {
        assert!(matches!(trial_status_for_elapsed_days(-1), LicenseStatus::Expired));
    }

    #[test]
    fn trial_expires_when_clock_rolls_back_after_later_valid_day() {
        let start = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let last_seen = NaiveDate::from_ymd_opt(2026, 6, 10).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 6, 2).unwrap();

        assert!(matches!(
            trial_status_for_dates(start, last_seen, today),
            LicenseStatus::Expired
        ));
    }
}
