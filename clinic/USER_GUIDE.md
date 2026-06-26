# Clinic Card Management — User Guide

A single-PC, offline desktop app for managing patient cards. All data is stored
**encrypted** on this computer.

---

## First run

1. Launch **Clinic Card Management** from the desktop/Start-menu icon.
2. Create the **first Admin** account (username + password).
3. You'll be asked to add a **second Admin** — please do this. See the warning below.

## ⚠️ Most important things to know

- **Admins can reset any user's password.** If a Staff member forgets their password,
  any Admin can reset it from **Backups → Users → Reset password** — no old password
  required.
- **If every Admin loses their password, user management is blocked.** The database
  can still be opened by Staff, but no one can add or fix Admin accounts. Two Admins
  removes this risk entirely.
- **If every single user loses their password, data is unrecoverable.** The database is
  fully encrypted. Write passwords down and keep them somewhere safe.
- **This is one computer.** If the PC fails, your only copies are the backups.
  Set up the USB backup (below), and keep the stick somewhere safe.

---

## Daily use

- **Home screen:** when you log in, all patients are listed in card-number order with a
  count at the top. Use the **Sex** and **City** dropdowns to filter. Type in the search
  box to narrow by name, phone, or card number.
- **Find a patient:** the big number on the left of each row is their **card number** —
  use it to pull the paper file.
- **Register a patient:** click **+ Register patient**, fill the form. The app assigns
  the next card number automatically. If a name or phone already exists, you'll see a
  duplicate warning — continue only if it's truly a different person.
- **Edit / Delete:** on any row. Deleting only **hides** the record — anyone can
  restore it from **Deleted patients**. Only Admins can permanently purge.

### Ages and dates
- When you choose **"Date of birth (Ethiopian calendar)"**, a calendar grid opens
  showing the month by name with weekday columns (Su–Sa). Use the **←** / **→**
  buttons to navigate months, type directly in the **year field**, then click the day.
- If the DOB is unknown, choose **"Age only"** and enter the age in years.
- Ages shown in the app update automatically as years pass — no manual adjustment needed.

---

## Roles

| Action | Reception | Admin |
|---|---|---|
| Search, register, edit, delete (hide) | ✅ | ✅ |
| Change own password (Settings) | ✅ | ✅ |
| View / restore deleted patients | ✅ | ✅ |
| Permanently purge deleted patients | — | ✅ |
| Reset any user's password (Settings) | — | ✅ |
| Manage users (Settings) | — | ✅ |
| Backups, import/export (Backups) | — | ✅ |
| View reports (Reports) | — | ✅ |
| View the activity log (Backups) | — | ✅ |

---

## Settings (header → **Settings**)

Available to all users:
- **Change your own password:** click **Change password…** and enter your current and new password.

Admin-only:
- **Add a user:** click **+ Add user**, enter a username, password, and role (Reception or Admin).
- **Reset a user's password:** click **Reset password** next to any user. The user can log in with the new password immediately.
- **Remove a user:** click **Remove** next to any user (you cannot remove yourself). The last Admin cannot be removed.

## Reports (Admin → **Reports**)

- **Overview:** total active patients, registered this month, registered this year.
- **Demographics:** male/female breakdown with a visual bar; top cities ranked by patient count.

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
