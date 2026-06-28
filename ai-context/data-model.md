# Data Model

## Database: `clinic.db` (SQLCipher, AES-256)

### `patients` table

```sql
CREATE TABLE patients (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    card_first       INTEGER NOT NULL,   -- e.g. 2  (for card "2/3")
    card_sub         INTEGER NOT NULL,   -- e.g. 3  (0–8 range)
    first_name       TEXT NOT NULL,
    father_name      TEXT NOT NULL,
    grandfather_name TEXT NOT NULL,
    sex              TEXT NOT NULL CHECK (sex IN ('Male','Female')),
    phone            TEXT NOT NULL,      -- 10 digits, must start 09 or 07
    dob_year         INTEGER,            -- Ethiopian calendar
    dob_month        INTEGER,
    dob_day          INTEGER,
    age_recorded     INTEGER,            -- age at time of registration
    age_recorded_on  TEXT,               -- ISO date when age was recorded
    address          TEXT,
    city             TEXT,
    registered_at    TEXT NOT NULL,      -- system clock, ISO datetime
    created_by       TEXT NOT NULL,
    updated_at       TEXT,
    updated_by       TEXT,
    deleted_at       TEXT,               -- NULL = active, non-NULL = soft-deleted
    deleted_by       TEXT,
    UNIQUE (card_first, card_sub)
);
```

### `card_seq` table

Single-row counter for the next card to issue.

```sql
CREATE TABLE card_seq (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    next_first INTEGER NOT NULL,
    next_sub   INTEGER NOT NULL
);
-- Seeded to (1, 0) → first card issued is 1/0
```

## Card numbering

Format: `{first}/{sub}`. Sub runs `0 → 8`; when sub would exceed 8, first increments
and sub resets to 0. Sequence: `1/0, 1/1, … 1/8, 2/0, … 9/8, 10/0, …`

- Auto-assigned on register — staff never type it.
- Deleted numbers are **not reused** (physical drawer stays in sync).
- Excel import preserves existing card numbers; `card_seq` is advanced past the highest
  imported number so new registrations don't collide.

## Age computation

Age is **not stored as a current value**. Two fields are written at registration:
- `age_recorded` — the age at the time of registration (or null if DOB was given)
- `age_recorded_on` — the system date at registration

The frontend computes the current age: `age_recorded + whole years elapsed since age_recorded_on`.
DOB-based ages compute from the Ethiopian DOB the same way.

## Auth: `auth.json`

```json
{
  "version": 1,
  "users": [
    {
      "username": "admin",
      "role": "Admin",
      "salt": "<base64, 16 bytes>",
      "nonce": "<base64, 12 bytes>",
      "wrapped_key": "<base64, 48 bytes — 32-byte master key sealed with AES-256-GCM>",
      "created_at": "2026-06-25T10:00:00"
    }
  ]
}
```

**Envelope encryption:** one random 32-byte master key is generated at first run.
For each user: Argon2id(password, salt) → 32-byte KEK → AES-256-GCM seal of master key.
Any user's password can unwrap the same master key. Adding a user wraps the key under
their password; removing a user deletes their record. The data is never re-encrypted.

## Roles

Two roles, stored as `"Admin"` or `"Staff"` in `auth.json`. Displayed in the UI as
"Admin" / "Reception". The internal string `"Staff"` must not change (it's serialized).

| Capability | Staff | Admin |
|-----------|-------|-------|
| Search, register, edit, soft-delete | ✅ | ✅ |
| Change own password | ✅ | ✅ |
| View / restore deleted patients | ✅ | ✅ |
| Purge deleted patients | — | ✅ |
| Import (Excel/CSV), export, backup/restore | — | ✅ |
| Add / remove users, reset passwords | — | ✅ |
| View reports, audit log | — | ✅ |
