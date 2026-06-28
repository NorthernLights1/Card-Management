# Clinic Card Management

Offline Windows desktop app for a small clinic. Physical patient folders are filed by
sequential card number. The app's core job is the **reverse lookup**: search by name or
phone → get the card number → pull the physical folder.

## Stack

| Layer | Tech |
|-------|------|
| Desktop shell | Tauri v2 (Windows, offline, single-PC) |
| Frontend | React 18 + TypeScript + Vite |
| Backend | Rust (Tauri commands in `src-tauri/src/`) |
| Database | SQLite encrypted at rest via **SQLCipher** |
| Auth | Argon2id key-derivation + AES-256-GCM envelope encryption |
| Calendar | Ethiopian calendar picker (DOB only) |

## Quick start

```powershell
cd clinic
npm install
npm run tauri dev     # dev mode (Rust + React hot-reload)
npm run tauri build   # production installer → src-tauri/target/release/bundle/
```

**Build prerequisites:** Rust (stable), Node 18+, Perl + NASM (required by
SQLCipher's vendored OpenSSL), Visual Studio C++ build tools.

## Project layout

```
clinic/
├── src/                        # React/TypeScript frontend
│   ├── App.tsx                 # Root — license gate + app shell
│   ├── App.css                 # All styles (single flat file)
│   ├── lib/
│   │   ├── api.ts              # Typed wrappers for every Tauri command
│   │   └── ethiopian.ts        # Ethiopian calendar conversion
│   └── components/
│       ├── LicenseScreen.tsx   # Shown when trial expired
│       ├── SearchScreen.tsx    # Home: all patients + live search
│       ├── PatientForm.tsx     # Register / edit form
│       ├── DeletedScreen.tsx   # Soft-deleted patients (Admin)
│       ├── SettingsScreen.tsx  # Passwords, users, Device ID
│       ├── BackupsScreen.tsx   # Backup/restore/import (Admin)
│       └── ReportsScreen.tsx   # Stats dashboard (Admin)
├── src-tauri/src/              # Rust backend
│   ├── commands.rs             # Every Tauri command (single source of truth for IPC)
│   ├── lib.rs                  # Plugin registration + AppState wiring
│   ├── db.rs                   # SQLCipher open + schema migrations
│   ├── auth.rs                 # Multi-user auth + envelope encryption
│   ├── patient.rs              # Patient CRUD, card numbering, search
│   ├── backup.rs               # PC-local + USB backup tiers
│   ├── import.rs               # Excel/CSV import
│   ├── audit.rs                # External plaintext audit log
│   └── license.rs              # Hardware-locked license + 14-day trial
└── src-tauri/Cargo.toml        # Rust deps (rusqlite/sqlcipher, sha2, winreg…)
```

## Key design decisions (quick reference)

- **Card format** `first/sub` — sub runs 0–8 then first increments. Starts `1/0`.
  Deleted numbers never reused (stays aligned with physical drawer).
- **Encryption** — one random 32-byte master key encrypts the DB. The master key is
  wrapped (AES-256-GCM) under each user's Argon2id-derived key and stored in `auth.json`.
  Any user's password unlocks the same data.
- **Dates** — DOB in Ethiopian calendar. All system timestamps (registered, audit) in
  system clock time (Gregorian).
- **Age** — stored with the date it was given; displayed age auto-increments each year
  without a cron job.
- **Soft delete only** — records are flagged hidden, never erased. Card numbers never reused.
- **Roles** — Admin and Staff (displayed as "Reception"). Stored as `"Staff"` in `auth.json`.
- **Audit log** — external plaintext `.txt` file, one line per action, append-only,
  logs identifiers only (card number, username) — no PHI.
- **License** — hardware-locked to motherboard serial (fallback: Windows MachineGuid).
  Key = SHA256(device_id) first 20 hex chars formatted as `XXXXX-XXXXX-XXXXX-XXXXX`.
  14-day trial stored in Windows registry (`HKCU\Software\ClinicApp\`), survives reinstall.

See [`FEATURE_CONTRACT.md`](FEATURE_CONTRACT.md) for the full product spec and
[`ai-context/`](ai-context/) for architecture notes aimed at AI assistants.
