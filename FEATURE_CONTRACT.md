# Feature Contract — Clinic Card Management System

**Version:** 5.2 (approved) · **Date:** 2026-06-26

---

## 1. Purpose

Single-PC, offline desktop application for a small clinic to manage basic patient
information. Physical patient folders are filed by sequential card number (not
alphabetized, by design). When a patient loses their card, staff struggle to find
the folder. **The app's core job is the reverse lookup:** search by name or phone →
get the card number → pull the physical folder.

## 2. Constraints

- One Windows PC. No server, no network. Fully offline.
- No recurring cost, no paid services.
- Small data volume.
- Patient data is sensitive → login + encryption required.

## 3. Platform & Stack

- **GUI desktop app** — Tauri, Windows installer + desktop icon, offline. Not browser-based.
- **Database:** SQLite encrypted at rest via **SQLCipher**.
- React + TypeScript · maintained Ethiopian-calendar library (**used for DOB only**).
- **English only** — no multi-language.

---

## 4. Patient Record

| Field | Required | Rules |
|---|---|---|
| **Card number** | Auto | `first/sub` (e.g. `2/3`). System-assigned, unique, read-only. See §5. |
| **First name** | Yes | Text. |
| **Father's name** | Yes | Text. |
| **Grandfather's name** | Yes | Text. |
| **Age** *or* **DOB** | One of the two | **DOB** entered via an **Ethiopian calendar picker** (month grid with weekday columns, month-name header, year input, clickable days). Age derived from DOB. **Age** stored with the date recorded, so displayed age **auto-increases each year** (computed live — §4.1). |
| **Sex** | Yes | Male / Female only. |
| **Phone** | **Yes** | 10 digits, must start `09` or `07`. Non-unique (families share numbers) but used for duplicate detection. |
| **Address** | No | Single free-text box. |
| **City** | No | Text. |
| **Registered timestamp** | Auto | **System date & time** (Gregorian, from the PC clock) — *not* EC. |

**4.1 Auto-incrementing age** — age is not bumped yearly by a job. We store the age
**plus the date it was given**, and the app **computes the current age on the fly**
(recorded age + whole years elapsed). Always correct, no maintenance. DOB-based ages
compute from the EC DOB the same way.

---

## 5. Card Numbering

- **Two regimes (CLIENT REQUIREMENT — not a bug, do not "normalize"):**
  - Cards **1 … 6045** are **plain sequential**, no sub: `1, 2, 3, … 6045`. These are
    the pre-existing paper folders; their numbers are frozen.
  - From **6046 onwards** `sub` cycles 0→8 before `first` increments:
    `6046, 6046/1 … 6046/8 → 6047, 6047/1 … 6047/8 → 6048 …`.
  - The `6045/6046` boundary is where the clinic's paper filing switched schemes. It is
    intentional and lives in code as `CARD_PLAIN_MAX = 6045`. Do not remove the plain-range
    branch or make numbering uniform.
- **Auto-assigned** — staff never type it.
- Deleted numbers are **not reused** (stays aligned with the physical drawer).
- Excel import **keeps existing card numbers** when the file provides them, and **auto-assigns**
  in row order when the card-number column is left unmapped or a cell is blank; the sequence
  continues from the highest card either way.

---

## 6. Core Features

- **6.1 Home / patient list:** on login, the home screen shows **all active patients**
  (up to 5 000) in card-number order with a live count. Staff can filter by **Sex** and
  **City** (City dropdown is populated from the real cities in the database). Typing in
  the search box narrows the list to name parts, phone, or card number. The card number
  is shown prominently on every row to support the reverse-lookup use-case.
- **6.2 Register:** validated form; card number auto-assigned. **Duplicate warning** if
  **either** (first+father+grandfather names match) **or** (phone matches) an existing
  patient — matches shown, staff may still proceed.
- **6.3 Edit:** any field except card number. Field order in the form: names, sex, phone,
  age/DOB, **city**, address.
- **6.6 Settings screen:** accessible to all users. Contains "Change password" (all roles) and
  "Users" section (Admin only — add, reset, remove users).
- **6.7 Backups screen:** Admin only. Contains USB backup, export/restore, import from
  Excel/CSV, export patient list, and activity log.
- **6.8 Reports screen:** Admin only. Overview tier: total patients, registered this month,
  registered this year. Demographics tier: male/female counts with visual bar, top-10 cities
  by patient count.
- **6.4 Delete:** **soft delete** only — record hidden, recoverable, never erased (see §10).
- **6.5 Print card:** ~~removed~~ — clinic uses pre-printed cards that do not feed into a printer.

---

## 7. Dates & Time

- **DOB:** Ethiopian calendar (the only place EC is used).
- **All system timestamps** (registered, created/modified, audit): **system clock date
  & time**, not EC.

---

## 8. Login, Users & Encryption

Two-tier permission model:

| Capability | Reception | Admin |
|---|---|---|
| Search, register, edit | ✅ | ✅ |
| Soft-delete a patient | ✅ | ✅ |
| Change own password (via Settings) | ✅ | ✅ |
| View / restore deleted patients | ✅ | ✅ |
| Permanently purge deleted patients | — | ✅ |
| Excel import | — | ✅ |
| Backup config, export, import/restore (Backups screen) | — | ✅ |
| Add / remove users, reset passwords (Settings screen) | — | ✅ |
| View reports | — | ✅ |
| View audit log | — | ✅ |

- Each user has their **own username + password**. Login required to open the app.
- **Encryption:** one random master key encrypts the database (SQLCipher). The master
  key is stored **wrapped under each user's password** — any user's password unlocks
  the same data; adding a user wraps the key under their password, removing a user
  deletes their wrapped copy. Data is never re-encrypted.
- **First user is an Admin.** Setup prompts for a **second Admin**.
- **Audit trail:** every register / edit / delete / restore / purge records **which user**
  did it and when (system clock) — in the DB and in the external log (§15).
- ✅ **Any user's forgotten password is survivable** — any Admin can **reset any other
  user's password** (no old password required; the Admin's session master key is used to
  re-wrap). Staff who forget their password just ask an Admin. Data is only unrecoverable
  if **every** user loses their password simultaneously.
- ✅ **Admin password reset is audited** — every reset writes `PASSWORD_RESET | target`
  to the audit log.
- ⚠️ **Keep at least two Admins** — only Admins manage users; if the sole Admin forgets
  their password, no one can add/fix users (data still opens for others, but user
  management is stuck). Two Admins removes this risk entirely.

---

## 9. Backup & Restore

Three tiers — newest data always safe locally, off-machine copy on USB, off-site copy later.

- **9.1 PC-local (always on):** `clinic_live.db` overwritten on **every save** + last
  **5 daily** snapshots, in a backup folder on the PC's own disk. Works with or without USB.
- **9.2 Designated USB:** set up **once** (recognized by **volume serial number** +
  marker file `.clinic_backup`); auto-recognized afterward on any drive letter; mirrors
  live + 5 daily when present. **Unknown USBs are ignored** (never writes patient data
  to an unrecognized stick). If the backup USB is absent: **friendly, non-blocking
  message** — *"USB backup paused — your backup USB isn't connected. Data is still saved
  and backed up on this PC."* — auto-resumes when reconnected.
- **9.3 Export:** write the encrypted DB file to any chosen folder/USB.
- **9.4 Import/Restore:** select a backup file → **preview record count + confirm** →
  replaces current data; **snapshots current DB first** so a wrong restore is reversible.
  Requires the **same password** the backup was made with.
- **9.5 Google Drive cloud backup:** deferred, **built last** — daily copy into the
  owner's own free Google Drive Desktop folder. No code until local backup is done and
  approved.

All backups/exports are copies of the **already-encrypted** DB → encrypted automatically.

---

## 10. Soft Delete & Recovery

- "Delete" **flags the record hidden**, it doesn't erase it — removed from searches/lists
  but preserved underneath.
- Staff can **soft-delete** (reversible). Only **Admin** can **view, restore, or
  permanently purge** deleted records, via a **"Deleted patients"** screen.
- Every delete/restore/purge is stamped with the user who did it.
- Soft-deleted card numbers are **not reused**, keeping numbering aligned with the
  physical drawer.

---

## 11. Data Import — existing data / Excel

- Occasional one-time migration from **Excel (.xlsx) or CSV**.
- **Admin-only**, tucked away in an Admin/Settings area, not on the main screen.
- Column-mapping step + per-row validation (names, sex, phone). Bad rows **reported and
  skipped**, not silently dropped.
- **Card number is optional in the mapping.** Mapped → existing numbers are preserved
  (including `6046/1`-style sub-cards). Unmapped or blank cell → the number is auto-assigned
  in row order. Auto-sequence continues from the highest card.

---

## 12. Patient-List Export

- Human-readable **CSV/Excel** export of the patient list (separate from the encrypted
  DB backup), for the clinic's own use.

---

## 13. Out of Scope

Appointments, billing, prescriptions, clinical notes · any networking / multi-PC /
remote access · analytics beyond the patient list · in-app cloud restore UI · any
language other than English.

---

## 14. Assumptions & Risks

- **Single PC = single point of failure** → mitigated by PC-local + USB backup (Google
  later as off-site).
- **All users lose passwords = data loss** (encryption has no backdoor; survivable as
  long as one password is known).
- **EC date conversion** depends on a third-party library's correctness.
- **USB sticks can fail** → Google backup is the off-site complement later.

---

## 15. External Audit Log

- A plain **`.txt` file, external to the database**, appended **one line per activity** —
  readable without the app, and intact even if the DB has trouble.
- **Format per line:** `system-timestamp | username | role | action | target`
  (e.g. `2026-06-25 14:32:10 | meron | Staff | REGISTER | card 12/3`).
- **Logged actions:** login (success + **failed**), logout, register, edit, soft-delete,
  restore, purge, user add/remove, role change, Excel import, export, backup/restore,
  USB connect/disconnect.
- **Append-only**, never edited by the app. Lives in the app data folder and is
  **mirrored to the USB/PC backup** like the DB.
- 🔒 **Privacy:** because this file is **plaintext and unencrypted** (the point is it's
  readable outside the app), it logs only **identifiers** — username, action, and
  **card number** — **not** patient names, phone, or other personal data. The encrypted
  DB stays the only place full patient info lives, so the audit log can't leak PHI if the
  file is copied.

---

## Decision Log (summary of choices made)

- Card numbering: **1–6045 plain sequential**, then **6046+** cycles `first/sub` with
  sub 0–8. Auto-assigned, never reused. The 6045→6046 boundary is a client requirement.
- Dates: DOB in Ethiopian calendar; all system timestamps in system clock time.
- Age: stored with record date, computed live, auto-increments.
- Patient form field order: first/father/grandfather names, sex, phone, age/DOB, city,
  address. City placed after DOB so related location fields are grouped together.
- Address: single free-text box + separate City field.
- Phone: mandatory, non-unique, used for duplicate detection.
- Language: English only.
- Users: multiple, per-user passwords, two-tier (Admin / Staff).
- Encryption: SQLCipher at rest, master key wrapped per user password.
- Admin password reset: uses the session master key (already in memory) to re-wrap for
  the target user — no old password needed.
- Home screen: all-patients view (not search-first) with live count and Sex/City filters.
  City filter dynamically populated from database values. Search narrows the same list.
- Role display label: internal value remains "Staff" (stored in auth.json); displayed as
  "Reception" in all UI surfaces.
- Settings screen: user management (add/reset/remove) + change-own-password, separated from
  the Backups screen. Settings accessible to all roles; Users section Admin-only.
- Reports screen: Admin-only. Tier 1 (counts) + Tier 2 (demographics). Tier 3 (visit
  patterns) deferred until date-of-visit feature is built.
- Backup: PC-local (live + 5 daily) + designated USB (serial-recognized); export/import;
  Google Drive last.
- Audit: in-DB trail + external append-only `.txt` log (identifiers only).
