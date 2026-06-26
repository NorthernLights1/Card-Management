# Plan: Clinic Card Management System

**Source contract:** FEATURE_CONTRACT.md (v5.0, approved)
**Build model:** Opus 4.8 (full project, extended thinking)
**Complexity:** Large

## Summary
Offline Windows GUI desktop app (Tauri) with an encrypted SQLite/SQLCipher store,
multi-user login (Admin/Staff), patient register/search/edit/soft-delete, auto card
numbering, Ethiopian-calendar DOB, mandatory phone + duplicate detection, PC-local + USB
backups, export/import-restore, external `.txt` audit log, Excel import, and CSV/Excel list
export. Google Drive backup deferred.

## Patterns to Mirror
Greenfield repo — no existing code or conventions. This build establishes them.

## Key Architecture Decisions
1. Data layer in Rust (Tauri commands); React never touches SQL or the encryption key.
2. SQLCipher via `rusqlite` with `bundled-sqlcipher` feature.
3. Multi-user = key wrapping: one random master key opens the DB; per user, a KEK from
   Argon2id wraps the master key (AES-256-GCM). Stored in a separate unencrypted `auth.json`
   (usernames/roles/salts/wrapped-key only — no patient data).
4. Backups use SQLite backup API / `VACUUM INTO`, not raw file copy of an open DB.
5. EC dates + live age = pure TS functions at presentation; DB stores entered EC y/m/d +
   recorded age + record date.
6. Native over libraries: print card = print-view + `window.print()`.

## Dependencies
- Rust: `rusqlite` (bundled-sqlcipher), `argon2`, `aes-gcm`, `rand`, `serde`/`serde_json`,
  `calamine` (Phase 7), `tauri` v2, `windows`/winapi (volume serial).
- npm: `react`, `typescript`, `vite`, EC-calendar package, `xlsx` (SheetJS).

## Phases
- **Phase 0 — Scaffold (Small):** Tauri v2 + React + TS + Vite; wire rusqlite bundled-sqlcipher; window opens; smoke command.
- **Phase 1 — Encrypted DB, schema, patient data layer (Medium):** schema + migrations; `patients`, `card_seq`, `meta`; commands for assign-next-card, CRUD, search; server-side validation; duplicate check. Tests for card sequence, no-reuse, phone, duplicates.
- **Phase 2 — Auth, roles, key wrapping (Medium, security-critical):** `auth.json`; first-run admin + second admin; login derive KEK → unwrap master key → open DB; user management; role gating in Rust. Tests for unwrap/revoke/role-reject.
- **Phase 3 — Core UI (Medium):** login; search landing (card number prominent); register (validation + duplicate modal); edit; EC DOB + live age; soft delete.
- **Phase 4 — Deleted-patients admin & print card (Small):** Admin view/restore/purge; print-card view + window.print().
- **Phase 5 — Backups (Large):** every-save `clinic_live.db`; daily snapshots keep 5; USB serial-recognized mirror + marker file + friendly status; export; import/restore with pre-restore snapshot.
- **Phase 6 — External audit log (Small):** append-only `.txt` (`timestamp | user | role | action | target`); hook all actions; identifiers only; mirror to backup.
- **Phase 7 — Excel import & list export (Medium):** Admin import (parse → map → validate → insert keeping card numbers, reseed seq → report skipped); CSV/Excel list export.
- **Phase 8 — Packaging & docs (Small):** Windows installer + icon; first-run UX; help docs (password-loss warning, keep two admins, USB setup).
- **Phase 9 — Google Drive backup: DEFERRED, not built now.**

## Risks
| Risk | Likelihood | Mitigation |
|---|---|---|
| SQLCipher build friction on Windows | Medium | Resolve in Phase 0 before dependents; bundled feature avoids system libs |
| Crypto key-wrapping error | Medium / high impact | Vetted crates; Phase 2 tests; no hand-rolled crypto |
| EC date library correctness | Medium | Maintained package; unit-test known dates |
| USB serial detection edge cases | Low | Match serial + marker; PC-local always works |
| All admins lose password | Low / total loss | Enforce 2-admin setup + warning |

## Acceptance
- [x] All phases 0–8 complete (9 deferred)
- [x] `cargo test` and validation steps pass per phase
- [x] Encryption, auth, backups verified by tests, not just UI
- [x] First demo milestone: through Phase 3 (encrypted multi-user register + search)

---

## Post-v1 Improvements (2026-06-26)

Changes made after Phase 8 delivery in response to client review:

### UX / Layout
- Patient registration form: **City** field moved to after the Age/DOB block and before
  Address, so location fields are grouped together.

### Home screen
- Replaced the search-first empty state with an **all-patients view**: loads all active
  patients on login (up to 5 000), shows a live count, supports **Sex** and **City**
  filter dropdowns (City list populated dynamically from the database), and search still
  narrows the same list. New Rust command: `list_patients` / `patient::list_all`.

### Admin password reset
- Admins can now **reset any user's password** without knowing the old one. The session
  master key (already in memory after login) is used to re-wrap the target user's record.
  New: `AuthStore::reset_user_password`, `reset_user_password` Tauri command, "Reset
  password" button in the Users section of SettingsScreen, `ResetPasswordModal` component.
  Audited as `PASSWORD_RESET` in the audit log.

### Ethiopian calendar DOB picker (2026-06-26)
- `EcDateInput` component rewritten as a proper calendar grid: month-name header
  with ← / → navigation, year number input, Su–Sa weekday columns, clickable day
  cells, selected day highlighted in accent color. Navigation only updates the view;
  a value is only committed when the user clicks a day.
- Radio label updated from "Known date of birth" → "Date of birth (Ethiopian calendar)".
- `ecMonthStartWeekday(year, month)` helper added to `ethiopian.ts` to compute the
  weekday offset for each month start.

| File | Change |
|---|---|
| `src-tauri/src/ethiopian.ts` | Added `ecMonthStartWeekday` export |
| `src/components/EcDateInput.tsx` | Rewritten as calendar grid picker |
| `src/components/PatientForm.tsx` | Updated radio label to name Ethiopian calendar |
| `src/App.css` | Replaced `.ec-date` with `.ec-calendar` styles |

### Files changed (v5.1)
| File | Change |
|---|---|
| `src-tauri/src/auth.rs` | Added `reset_user_password` method |
| `src-tauri/src/patient.rs` | Added `list_all` function |
| `src-tauri/src/commands.rs` | Added `reset_user_password` and `list_patients` commands |
| `src-tauri/src/lib.rs` | Registered both new commands |
| `src/lib/api.ts` | Added `resetUserPassword` and `listPatients` wrappers |
| `src/components/PatientForm.tsx` | Moved City field after DOB block |
| `src/components/SearchScreen.tsx` | Rewrote as all-patients view with filters |
| `src/components/SettingsScreen.tsx` | Added reset password button and `ResetPasswordModal` |
| `FEATURE_CONTRACT.md` | Updated §6, §8 roles table, §8 password text, Decision Log |
| `clinic/USER_GUIDE.md` | Updated warnings, daily use, roles table, added Users section |
