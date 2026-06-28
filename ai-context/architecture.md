# Architecture

## Overview

Single-process Tauri v2 desktop app. The Rust backend owns all state; the React
frontend is a thin UI layer that calls Rust via `invoke()`. There is no HTTP server,
no network, no external services at runtime.

```
React UI  ──invoke()──▶  Rust command layer (commands.rs)
                              │
                    ┌─────────┼─────────────────┐
                    ▼         ▼                  ▼
                 auth.rs   patient.rs         license.rs
                              │
                           db.rs (SQLCipher)
```

## Startup sequence

```
1. App.tsx mounts
2. getLicenseStatus() called (no session required — registry only)
3a. Expired → <LicenseScreen> blocks everything
3b. Trial / Licensed → isInitialized() called
4a. false → <SetupScreen> (first-run admin creation)
4b. true  → <LoginScreen>
5.  Login succeeds → Rust opens the encrypted DB, stores Connection in AppState
6.  App shell renders, all commands now available
```

## Session state (Rust side)

`AppState` lives in `commands.rs` and is managed by Tauri as a `State<AppState>`:

```rust
pub struct AppState {
    data_dir: PathBuf,          // %APPDATA%\com.clinic.app\  (set by Tauri)
    active: Mutex<Option<Active>>,
}
struct Active {
    session: Session,           // username, role, master key (32 bytes, in-memory only)
    conn: Connection,           // open SQLCipher connection
}
```

The master key exists only in process memory while a user is logged in. It is zeroed
when `logout` is called (the `Option` is set to `None`, dropping the `Active`).

## Data files (all in %APPDATA%\com.clinic.app\)

| File | Contents |
|------|----------|
| `clinic.db` | SQLCipher-encrypted patient database |
| `auth.json` | User records: username, role, salt, nonce, wrapped master key (base64) |
| `audit.log` | Append-only plaintext audit trail |
| `backup.json` | USB backup config (volume serial + marker path) |
| `backup/` | PC-local backup snapshots (live + 5 daily) |

## Frontend → Backend contract

All IPC goes through `clinic/src/lib/api.ts`. Every exported function is a typed
wrapper around `invoke()`. The UI never touches SQL or the encryption key.

Command names are snake_case strings matching the `#[tauri::command]` function name in
`commands.rs`. Parameters are passed as a JSON object matching the Rust function's
parameter names exactly (camelCase on TS side maps to snake_case on Rust via Tauri's
automatic conversion).

## Backup tiers

1. **PC-local** — after every save: `clinic_live.db` overwritten + up to 5 daily
   snapshots in `backup/`. Always on, no config.
2. **USB** — recognized by volume serial number + a `.clinic_backup` marker file written
   at setup time. Auto-mirrors on every save when connected. Unknown sticks ignored.
3. **Export** — admin copies the encrypted DB to any chosen path.
4. **Restore** — preview (record count + password check) then apply. Snapshots current
   DB before overwriting so a wrong restore is undoable.

## Audit log format

```
2026-06-25 14:32:10 | meron | Staff | REGISTER | card 12/3
```

Logged events: `LOGIN`, `LOGIN_FAILED`, `LOGOUT`, `REGISTER`, `EDIT`, `DELETE`,
`RESTORE_PATIENT`, `PURGE_PATIENT`, `USER_ADD`, `USER_REMOVE`, `PASSWORD_RESET`,
`PASSWORD_CHANGE`, `SETUP_ADMIN`, `IMPORT`, `EXPORT`, `EXPORT_CSV`, `USB_SETUP`,
`RESTORE`.

The log intentionally records only card numbers (not patient names/phones) so the
plaintext file cannot leak PHI if copied.
