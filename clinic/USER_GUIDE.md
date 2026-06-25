# Clinic Card Management — User Guide

A single-PC, offline desktop app for managing patient cards. All data is stored
**encrypted** on this computer.

---

## First run

1. Launch **Clinic Card Management** from the desktop/Start-menu icon.
2. Create the **first Admin** account (username + password).
3. You'll be asked to add a **second Admin** — please do this. See the warning below.

## ⚠️ Most important things to know

- **There is no password recovery.** The patient database is encrypted with the
  passwords. If **every** user forgets their password, the data **cannot** be
  recovered by anyone. Write the passwords down and keep them somewhere safe.
- **Keep at least two Admins.** Only Admins can manage users. If your only Admin
  forgets their password, no one can add or fix accounts. Two Admins removes this risk.
- **This is one computer.** If the PC fails, your only copies are the backups.
  Set up the USB backup (below), and keep the stick somewhere safe.

---

## Daily use

- **Find a patient:** type their name, phone, or card number in the search box.
  The big number on the left is their **card number** — use it to pull the paper file.
- **Register a patient:** click **+ Register patient**, fill the form. The app assigns
  the next card number automatically. If a name or phone already exists, you'll see a
  duplicate warning — continue only if it's truly a different person.
- **Edit / Delete:** on a search result. Deleting only **hides** the record — an Admin
  can restore it from **Deleted patients**.
- **Print a card:** click **Print card** on a search result.

### Ages and dates
- Enter a **date of birth** in the **Ethiopian calendar**, or just an **age** if the
  DOB is unknown. Ages shown in the app update automatically as years pass.

---

## Roles

| Action | Staff | Admin |
|---|---|---|
| Search, register, edit, print, delete (hide) | ✅ | ✅ |
| Restore / permanently delete patients | — | ✅ |
| Manage users, backups, import/export | — | ✅ |
| View the activity log | — | ✅ |

---

## Backups (Admin → **Backups**)

- **On this PC (automatic):** every change is saved to a backup folder on the PC,
  with the last 5 daily snapshots. Always on, nothing to do.
- **USB (recommended):** click **Set up USB drive**, plug in a stick, and choose it.
  After that, every change is also mirrored to that specific stick. If it's unplugged
  you'll see "USB backup paused" — it resumes automatically when you plug it back in.
- **Export backup…** saves a full copy (database + accounts) to any folder/USB.
- **Restore from backup…** replaces all current data with a backup. You'll need the
  password the backup was made with, and you'll be signed out afterward. The current
  data is snapshotted first, so a wrong restore can be undone.

## Import existing records (Admin → Backups → **Import from Excel/CSV…**)

- Choose an `.xlsx` or `.csv` file. The first row must be column headers, and each row
  must include its existing **card number**.
- Match your columns to the fields, then **Import**. Bad rows are reported and skipped.
- Ages are imported; dates of birth can be added later by editing a patient.

## Export the patient list

- Admin → Backups → **Export patient list (CSV)…** — opens in Excel.

---

## The activity log

A plain-text `audit.log` records who did what and when (logins, register/edit/delete,
user changes, backups). It contains only card numbers and usernames — **no** patient
names or phone numbers. Admins can view it under **Backups → Activity log**.

---

## Where the data lives

In the app's data folder on this PC (`%APPDATA%\com.clinic.cardmanagement`):
`clinic.db` (encrypted), `auth.json` (accounts), `audit.log`, and `backups\`.
A backup set is **both** `clinic_live.db` and `auth.json` — you need both to restore.
