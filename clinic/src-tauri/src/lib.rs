mod audit;
mod auth;
mod backup;
mod commands;
mod db;
mod import;
mod patient;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            app.manage(commands::AppState::new(data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::is_initialized,
            commands::initialize_admin,
            commands::login,
            commands::logout,
            commands::current_user,
            commands::add_user,
            commands::remove_user,
            commands::list_users,
            commands::change_password,
            commands::register_patient,
            commands::update_patient,
            commands::delete_patient,
            commands::search_patients,
            commands::get_patient,
            commands::check_duplicates,
            commands::list_deleted_patients,
            commands::restore_patient,
            commands::purge_patient,
            commands::usb_status,
            commands::list_removable_drives,
            commands::set_usb_backup,
            commands::export_backup,
            commands::restore_preview,
            commands::restore_apply,
            commands::read_audit_log,
            commands::import_preview,
            commands::import_apply,
            commands::export_patient_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
