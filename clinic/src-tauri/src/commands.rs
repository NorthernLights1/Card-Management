//! Tauri command layer: session state + the API the UI calls via `invoke`.
//!
//! The master key and DB connection live here in process memory only while a
//! user is logged in. The frontend never sees the key or touches SQL.

use crate::audit;
use crate::auth::{AuthStore, Role, Session};
use crate::backup;
use crate::import;
use crate::patient::{self, Patient, PatientInput};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    data_dir: PathBuf,
    active: Mutex<Option<Active>>,
}

struct Active {
    session: Session,
    conn: Connection,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            active: Mutex::new(None),
        }
    }
    fn auth_path(&self) -> PathBuf {
        self.data_dir.join("auth.json")
    }
    fn db_path(&self) -> PathBuf {
        self.data_dir.join("clinic.db")
    }
    /// Best-effort backup after every change; never blocks the save.
    fn run_backups(&self, conn: &Connection) {
        backup::backup_all(conn, &self.auth_path(), &self.data_dir);
    }
    fn audit(&self, user: &str, role: &str, action: &str, target: &str) {
        audit::log(&self.data_dir, user, role, action, target);
    }
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::Admin => "Admin",
        Role::Staff => "Staff",
    }
}

#[derive(Clone, serde::Serialize)]
pub struct UserInfo {
    pub username: String,
    pub role: Role,
}

fn open_session(state: &State<AppState>, session: Session) -> Result<(), String> {
    let conn = crate::db::open(&state.db_path(), &session.master_key).map_err(|e| e.to_string())?;
    *state.active.lock().unwrap() = Some(Active { session, conn });
    Ok(())
}

// --- auth / session ---------------------------------------------------------

#[tauri::command]
pub fn is_initialized(state: State<AppState>) -> bool {
    AuthStore::exists(&state.auth_path())
}

#[tauri::command]
pub fn initialize_admin(
    state: State<AppState>,
    username: String,
    password: String,
) -> Result<UserInfo, String> {
    std::fs::create_dir_all(&state.data_dir).map_err(|e| e.to_string())?;
    let (_, master_key) = AuthStore::initialize(&state.auth_path(), &username, &password)?;
    let session = Session {
        username: username.clone(),
        role: Role::Admin,
        master_key,
    };
    open_session(&state, session)?;
    state.audit(&username, "Admin", "SETUP_ADMIN", "");
    Ok(UserInfo {
        username,
        role: Role::Admin,
    })
}

#[tauri::command]
pub fn login(
    state: State<AppState>,
    username: String,
    password: String,
) -> Result<UserInfo, String> {
    let store = AuthStore::load(&state.auth_path())?;
    let session = match store.login(&username, &password) {
        Ok(s) => s,
        Err(e) => {
            state.audit(&username, "-", "LOGIN_FAILED", "");
            return Err(e);
        }
    };
    let info = UserInfo {
        username: session.username.clone(),
        role: session.role,
    };
    let role = session.role;
    open_session(&state, session)?;
    state.audit(&info.username, role_str(role), "LOGIN", "");
    Ok(info)
}

#[tauri::command]
pub fn logout(state: State<AppState>) {
    let mut guard = state.active.lock().unwrap();
    if let Some(active) = guard.as_ref() {
        audit::log(
            &state.data_dir,
            &active.session.username,
            role_str(active.session.role),
            "LOGOUT",
            "",
        );
    }
    *guard = None;
}

#[tauri::command]
pub fn current_user(state: State<AppState>) -> Option<UserInfo> {
    state.active.lock().unwrap().as_ref().map(|a| UserInfo {
        username: a.session.username.clone(),
        role: a.session.role,
    })
}

// --- user management (Admin only) ------------------------------------------

/// Returns (acting username, master key) if the current user is an Admin.
fn require_admin(state: &State<AppState>) -> Result<(String, [u8; 32]), String> {
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    if active.session.role != Role::Admin {
        return Err("This action requires an Admin".into());
    }
    Ok((active.session.username.clone(), active.session.master_key))
}

#[tauri::command]
pub fn add_user(
    state: State<AppState>,
    username: String,
    password: String,
    role: Role,
) -> Result<(), String> {
    let (actor, master) = require_admin(&state)?;
    let mut store = AuthStore::load(&state.auth_path())?;
    store.add_user(&master, &username, &password, role)?;
    state.audit(&actor, "Admin", "USER_ADD", &username);
    Ok(())
}

#[tauri::command]
pub fn remove_user(state: State<AppState>, username: String) -> Result<(), String> {
    let (actor, _) = require_admin(&state)?;
    let mut store = AuthStore::load(&state.auth_path())?;
    store.remove_user(&username)?;
    state.audit(&actor, "Admin", "USER_REMOVE", &username);
    Ok(())
}

#[tauri::command]
pub fn list_users(state: State<AppState>) -> Result<Vec<UserInfo>, String> {
    require_admin(&state)?;
    let store = AuthStore::load(&state.auth_path())?;
    Ok(store
        .list_users()
        .into_iter()
        .map(|(username, role)| UserInfo { username, role })
        .collect())
}

#[tauri::command]
pub fn change_password(
    state: State<AppState>,
    old_password: String,
    new_password: String,
) -> Result<(), String> {
    let (username, role) = {
        let guard = state.active.lock().unwrap();
        let active = guard.as_ref().ok_or("Not logged in")?;
        (active.session.username.clone(), active.session.role)
    };
    let mut store = AuthStore::load(&state.auth_path())?;
    store.change_password(&username, &old_password, &new_password)?;
    state.audit(&username, role_str(role), "PASSWORD_CHANGE", &username);
    Ok(())
}

// --- patients ---------------------------------------------------------------

#[tauri::command]
pub fn register_patient(state: State<AppState>, input: PatientInput) -> Result<Patient, String> {
    let mut guard = state.active.lock().unwrap();
    let active = guard.as_mut().ok_or("Not logged in")?;
    let actor = active.session.username.clone();
    let role = active.session.role;
    let patient = patient::create(&mut active.conn, &input, &actor)?;
    state.audit(&actor, role_str(role), "REGISTER", &patient.card_number);
    state.run_backups(&active.conn);
    Ok(patient)
}

#[tauri::command]
pub fn update_patient(
    state: State<AppState>,
    id: i64,
    input: PatientInput,
) -> Result<(), String> {
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    let actor = active.session.username.clone();
    let role = active.session.role;
    patient::update(&active.conn, id, &input, &actor)?;
    state.audit(&actor, role_str(role), "EDIT", &patient::card_of(&active.conn, id));
    state.run_backups(&active.conn);
    Ok(())
}

#[tauri::command]
pub fn delete_patient(state: State<AppState>, id: i64) -> Result<(), String> {
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    let actor = active.session.username.clone();
    let role = active.session.role;
    patient::soft_delete(&active.conn, id, &actor)?;
    state.audit(&actor, role_str(role), "DELETE", &patient::card_of(&active.conn, id));
    state.run_backups(&active.conn);
    Ok(())
}

#[tauri::command]
pub fn search_patients(state: State<AppState>, query: String) -> Result<Vec<Patient>, String> {
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    patient::search(&active.conn, &query)
}

#[tauri::command]
pub fn get_patient(state: State<AppState>, id: i64) -> Result<Option<Patient>, String> {
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    patient::get_by_id(&active.conn, id)
}

#[tauri::command]
pub fn check_duplicates(
    state: State<AppState>,
    input: PatientInput,
) -> Result<Vec<Patient>, String> {
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    patient::find_duplicates(&active.conn, &input)
}

// --- deleted patients (Admin only) -----------------------------------------

#[tauri::command]
pub fn list_deleted_patients(state: State<AppState>) -> Result<Vec<Patient>, String> {
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    if active.session.role != Role::Admin {
        return Err("This action requires an Admin".into());
    }
    patient::list_deleted(&active.conn)
}

#[tauri::command]
pub fn restore_patient(state: State<AppState>, id: i64) -> Result<(), String> {
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    if active.session.role != Role::Admin {
        return Err("This action requires an Admin".into());
    }
    let actor = active.session.username.clone();
    patient::restore(&active.conn, id)?;
    state.audit(&actor, "Admin", "RESTORE_PATIENT", &patient::card_of(&active.conn, id));
    state.run_backups(&active.conn);
    Ok(())
}

#[tauri::command]
pub fn purge_patient(state: State<AppState>, id: i64) -> Result<(), String> {
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    if active.session.role != Role::Admin {
        return Err("This action requires an Admin".into());
    }
    let actor = active.session.username.clone();
    let card = patient::card_of(&active.conn, id); // capture before the row is gone
    patient::purge(&active.conn, id)?;
    state.audit(&actor, "Admin", "PURGE_PATIENT", &card);
    state.run_backups(&active.conn);
    Ok(())
}

// --- backups (Admin only, except read-only status) -------------------------

#[tauri::command]
pub fn usb_status(state: State<AppState>) -> backup::UsbStatus {
    backup::usb_status(&state.data_dir)
}

#[tauri::command]
pub fn list_removable_drives(state: State<AppState>) -> Result<Vec<backup::DriveInfo>, String> {
    require_admin(&state)?;
    Ok(backup::removable_drives())
}

#[tauri::command]
pub fn set_usb_backup(state: State<AppState>, drive: String) -> Result<(), String> {
    let (actor, _) = require_admin(&state)?;
    backup::set_usb(&state.data_dir, &drive)?;
    state.audit(&actor, "Admin", "USB_SETUP", &drive);
    // Immediately seed the stick with a full backup set.
    let guard = state.active.lock().unwrap();
    if let Some(active) = guard.as_ref() {
        state.run_backups(&active.conn);
    }
    Ok(())
}

#[tauri::command]
pub fn export_backup(state: State<AppState>, dest_dir: String) -> Result<(), String> {
    let (actor, _) = require_admin(&state)?;
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    backup::export_to(&active.conn, &state.auth_path(), Path::new(&dest_dir))?;
    state.audit(&actor, "Admin", "EXPORT", &dest_dir);
    Ok(())
}

#[tauri::command]
pub fn restore_preview(
    state: State<AppState>,
    folder: String,
    username: String,
    password: String,
) -> Result<i64, String> {
    require_admin(&state)?;
    backup::restore_preview(Path::new(&folder), &username, &password)
}

#[tauri::command]
pub fn restore_apply(
    state: State<AppState>,
    folder: String,
    username: String,
    password: String,
) -> Result<(), String> {
    let (actor, _) = require_admin(&state)?;
    // Validate the backup before we touch anything.
    backup::restore_preview(Path::new(&folder), &username, &password)?;
    // Close our open DB handle first, or Windows won't let us overwrite the file.
    *state.active.lock().unwrap() = None;
    backup::restore_apply(
        Path::new(&folder),
        &state.data_dir,
        &state.db_path(),
        &state.auth_path(),
        &username,
        &password,
    )?;
    state.audit(&actor, "Admin", "RESTORE", &folder);
    Ok(())
}

// --- import / export (Admin only) ------------------------------------------

#[tauri::command]
pub fn import_preview(state: State<AppState>, path: String) -> Result<import::ImportPreview, String> {
    require_admin(&state)?;
    import::preview(Path::new(&path))
}

#[tauri::command]
pub fn import_apply(
    state: State<AppState>,
    path: String,
    mapping: import::Mapping,
) -> Result<patient::ImportReport, String> {
    let (actor, _) = require_admin(&state)?;
    let items = import::build_items(Path::new(&path), &mapping)?;
    let mut guard = state.active.lock().unwrap();
    let active = guard.as_mut().ok_or("Not logged in")?;
    let report = patient::import_rows(&mut active.conn, &items)?;
    state.audit(
        &actor,
        "Admin",
        "IMPORT",
        &format!("{} imported, {} skipped", report.imported, report.skipped.len()),
    );
    state.run_backups(&active.conn);
    Ok(report)
}

#[tauri::command]
pub fn export_patient_csv(state: State<AppState>, dest_path: String) -> Result<usize, String> {
    let (actor, _) = require_admin(&state)?;
    let guard = state.active.lock().unwrap();
    let active = guard.as_ref().ok_or("Not logged in")?;
    let count = patient::export_csv(&active.conn, Path::new(&dest_path))?;
    state.audit(&actor, "Admin", "EXPORT_CSV", &dest_path);
    Ok(count)
}

#[tauri::command]
pub fn read_audit_log(state: State<AppState>) -> Result<String, String> {
    require_admin(&state)?;
    let path = state.data_dir.join("audit.log");
    if !path.exists() {
        return Ok(String::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(300); // most recent 300 entries
    Ok(lines[start..].join("\n"))
}
