# Tauri IPC Commands

All commands are defined in `clinic/src-tauri/src/commands.rs` and registered in
`clinic/src-tauri/src/lib.rs`. TypeScript wrappers live in `clinic/src/lib/api.ts`.

Commands that require a logged-in session access `State<AppState>` and return
`Err("Not logged in")` if no session is active. License commands have no session
requirement and can be called before login.

## Auth / session

| Command | Rust fn | Notes |
|---------|---------|-------|
| `is_initialized` | `is_initialized` | Returns `bool` — false on first run |
| `initialize_admin` | `initialize_admin(username, password)` | First-run only; creates admin + DB |
| `login` | `login(username, password)` | Opens encrypted DB, stores session |
| `logout` | `logout` | Drops session and DB connection |
| `current_user` | `current_user` | Returns `UserInfo | null` |

## User management (Admin only)

| Command | Rust fn | Notes |
|---------|---------|-------|
| `add_user` | `add_user(username, password, role)` | role: `"Admin"` or `"Staff"` |
| `remove_user` | `remove_user(username)` | Cannot remove self |
| `list_users` | `list_users` | Returns `UserInfo[]` |
| `reset_user_password` | `reset_user_password(username, new_password)` | Admin re-wraps master key for target |
| `change_password` | `change_password(old_password, new_password)` | Self-service, any role |

## Patients

| Command | Rust fn | Notes |
|---------|---------|-------|
| `register_patient` | `register_patient(input)` | Returns `Patient` with assigned card number |
| `update_patient` | `update_patient(id, input)` | Returns `()` |
| `delete_patient` | `delete_patient(id)` | Soft delete (sets deleted_at) |
| `list_patients` | `list_patients` | All active patients, card-number order |
| `search_patients` | `search_patients(query)` | Names, phone, card number (FTS-lite) |
| `get_patient` | `get_patient(id)` | Returns `Patient | null` |
| `check_duplicates` | `check_duplicates(input)` | Returns matching active patients |
| `list_deleted_patients` | `list_deleted_patients` | Soft-deleted records |
| `restore_patient` | `restore_patient(id)` | Clears deleted_at |
| `purge_patient` | `purge_patient(id)` | Hard delete, Admin only |

## Backups (Admin only except `usb_status`)

| Command | Rust fn | Notes |
|---------|---------|-------|
| `usb_status` | `usb_status` | Returns `{configured, connected, drive}` |
| `list_removable_drives` | `list_removable_drives` | Returns `DriveInfo[]` |
| `set_usb_backup` | `set_usb_backup(drive)` | Writes marker file, seeds first backup |
| `export_backup` | `export_backup(dest_dir)` | Copies encrypted DB to path |
| `restore_preview` | `restore_preview(folder, username, password)` | Returns record count |
| `restore_apply` | `restore_apply(folder, username, password)` | Closes DB, overwrites files |

## Import / export (Admin only)

| Command | Rust fn | Notes |
|---------|---------|-------|
| `import_preview` | `import_preview(path)` | Returns `{headers, sample, total_rows}` |
| `import_apply` | `import_apply(path, mapping)` | Column mapping required; returns report |
| `export_patient_csv` | `export_patient_csv(dest_path)` | Returns row count |

## Reports (Admin only)

| Command | Rust fn | Notes |
|---------|---------|-------|
| `read_audit_log` | `read_audit_log` | Last 300 lines of `audit.log` as string |
| `get_patient_stats` | `get_patient_stats` | Returns `PatientStats` (counts + city breakdown) |

## License / trial (no session required)

| Command | Rust fn | Notes |
|---------|---------|-------|
| `get_device_id` | `get_device_id` | Motherboard serial or MachineGuid |
| `get_license_status` | `get_license_status` | Returns `LicenseStatus` discriminated union |
| `activate_license` | `activate_license(key)` | Validates and persists key to registry |

`LicenseStatus` shape:
```typescript
| { status: "Licensed" }
| { status: "Trial"; days_remaining: number }
| { status: "Expired" }
```
