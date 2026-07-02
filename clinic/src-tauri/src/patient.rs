//! Patient model, validation, card numbering, and data access.
//!
//! All functions take a `&Connection` (or `&mut` for writes) so they can run
//! against the encrypted DB in production and an in-memory DB in tests.

use rusqlite::{params, Connection, OptionalExtension, Row};

const CARD_SUB_MAX: i64 = 8;
// ╔══════════════════════════════════════════════════════════════════════════╗
// ║  ⛔ DO NOT "FIX" THIS. THE TWO-REGIME NUMBERING IS INTENTIONAL. ⛔        ║
// ║                                                                          ║
// ║  This is a CLIENT REQUIREMENT, not a bug, not an inconsistency, not an    ║
// ║  off-by-one. It is the single most-reverted thing in this repo — every   ║
// ║  reviewer who "simplified" it introduced a real bug. If it looks wrong,  ║
// ║  YOU ARE MISREADING IT.                                                   ║
// ║                                                                          ║
// ║  Required sequence (locked by tests, see below):                         ║
// ║    1, 2, 3, … 6045          → plain sequential, sub ALWAYS 0, no slash    ║
// ║    6046, 6046/1 … 6046/8    → from 6046 sub cycles 0→8 …                  ║
// ║    6047, 6047/1 … 6047/8    → … THEN first increments and sub resets      ║
// ║    6048, …                                                               ║
// ║                                                                          ║
// ║  The 6045/6046 boundary is where the clinic's physical paper filing      ║
// ║  switched schemes — a historical fact about the client's folders.        ║
// ║                                                                          ║
// ║  Do NOT: make numbering uniform, remove the `first <= CARD_PLAIN_MAX`     ║
// ║  branch, change 6045, or change the `first/sub` slash format.            ║
// ║  Tests that FAIL if you break this (the tests are right, your change is   ║
// ║  wrong): card_numbers_plain_sequential_below_6046,                       ║
// ║  card_numbers_sub_cycle_at_and_above_6046. See FEATURE_CONTRACT.md §5     ║
// ║  and AGENTS.md.                                                           ║
// ╚══════════════════════════════════════════════════════════════════════════╝
const CARD_PLAIN_MAX: i64 = 6045;
const LIST_PAGE_LIMIT_MAX: i64 = 500;

// ⛔ INTENTIONAL two-regime advance — see the banner above. Do NOT collapse the
// branches or drop the `first <= CARD_PLAIN_MAX` guard. This is not a bug.
fn advance_card(first: i64, sub: i64) -> (i64, i64) {
    if first <= CARD_PLAIN_MAX {
        // plain range (1..=6045): increment first, sub stays 0 — NO slash numbers here
        (first + 1, 0)
    } else if sub < CARD_SUB_MAX {
        // sub-cycling range (6046+): hold first, advance sub 0→8 (6046/1 … 6046/8)
        (first, sub + 1)
    } else {
        // sub exhausted at 8: roll to next first, reset sub (6046/8 → 6047)
        (first + 1, 0)
    }
}

fn normalize_next_card(first: i64, sub: i64) -> (i64, i64) {
    if first <= CARD_PLAIN_MAX && sub > 0 {
        (first + 1, 0)
    } else {
        (first, sub)
    }
}

fn next_after_existing_card(first: i64, sub: i64) -> (i64, i64) {
    let (next_first, next_sub) = advance_card(first, sub);
    normalize_next_card(next_first, next_sub)
}

/// Ethiopian-calendar date (the only date the user enters; system timestamps are
/// stored separately in system time).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EcDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// Incoming patient data from the UI.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PatientInput {
    pub first_name: String,
    pub father_name: String,
    pub grandfather_name: String,
    pub sex: String,
    pub phone: String,
    pub dob: Option<EcDate>,
    pub age: Option<i64>,
    pub address: Option<String>,
    pub city: Option<String>,
}

/// A stored patient as returned to the UI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Patient {
    pub id: i64,
    pub card_number: String,
    pub first_name: String,
    pub father_name: String,
    pub grandfather_name: String,
    pub sex: String,
    pub phone: String,
    pub dob: Option<EcDate>,
    pub age_recorded: Option<i64>,
    pub age_recorded_on: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub registered_at: String,
}

// --- validation -------------------------------------------------------------

/// Validate at the trust boundary. Returns a user-facing message on failure.
pub fn validate(input: &PatientInput) -> Result<(), String> {
    require(&input.first_name, "First name is required")?;
    require(&input.father_name, "Father's name is required")?;
    require(&input.grandfather_name, "Grandfather's name is required")?;

    if input.sex != "Male" && input.sex != "Female" {
        return Err("Sex must be Male or Female".into());
    }
    if !is_valid_phone(&input.phone) {
        return Err("Phone must be 10 digits starting with 09 or 07".into());
    }
    match (&input.dob, input.age) {
        (None, None) => Err("Either date of birth or age is required".into()),
        (Some(dob), _) => validate_ec(dob), // DOB present → it wins, age ignored
        (None, Some(age)) => {
            if (0..=200).contains(&age) {
                Ok(())
            } else {
                Err("Age looks invalid".into())
            }
        }
    }
}

fn require(value: &str, msg: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(msg.into())
    } else {
        Ok(())
    }
}

fn is_valid_phone(p: &str) -> bool {
    p.len() == 10
        && p.bytes().all(|b| b.is_ascii_digit())
        && (p.starts_with("09") || p.starts_with("07"))
}

fn validate_ec(d: &EcDate) -> Result<(), String> {
    if !(1..=3000).contains(&d.year) {
        return Err("Date of birth year is out of range".into());
    }
    if !(1..=13).contains(&d.month) {
        return Err("Ethiopian month must be 1-13".into());
    }
    // ponytail: months 1-12 have 30 days; month 13 (Pagumē) has 5-6.
    // Allow up to 6 rather than computing the leap day.
    let max_day = if d.month == 13 { 6 } else { 30 };
    if d.day < 1 || d.day > max_day {
        return Err(format!("Day must be 1-{max_day} for that month"));
    }
    Ok(())
}

// --- card numbering ---------------------------------------------------------

/// Read and advance the next card number. Never reuses a number. Caller must run
/// this inside the same transaction as the patient insert.
fn assign_next_card(conn: &Connection) -> rusqlite::Result<(i64, i64)> {
    let (first, sub): (i64, i64) = conn.query_row(
        "SELECT next_first, next_sub FROM card_seq WHERE id = 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let (first, sub) = normalize_next_card(first, sub);
    let (next_first, next_sub) = advance_card(first, sub);
    conn.execute(
        "UPDATE card_seq SET next_first = ?1, next_sub = ?2 WHERE id = 1",
        params![next_first, next_sub],
    )?;
    Ok((first, sub))
}

// ⛔ INTENTIONAL display format. `sub == 0` → bare number ("6045"); otherwise
// "first/sub" ("6046/1"). Do NOT change the slash format or always-show-sub — this
// is the exact string the clinic writes on physical folders. See banner above.
fn card_number(first: i64, sub: i64) -> String {
    if sub == 0 {
        format!("{first}")
    } else {
        format!("{first}/{sub}")
    }
}

// --- writes -----------------------------------------------------------------

/// Insert a new patient, auto-assigning the next card number.
pub fn create(conn: &mut Connection, input: &PatientInput, actor: &str) -> Result<Patient, String> {
    validate(input)?;
    let now = now_string();
    let tx = conn.transaction().map_err(e2s)?;
    let (card_first, card_sub) = assign_next_card(&tx).map_err(e2s)?;

    let (dy, dm, dd) = match &input.dob {
        Some(d) => (Some(d.year), Some(d.month), Some(d.day)),
        None => (None, None, None),
    };
    // Age is stored only when DOB is unknown, alongside the date it was recorded
    // so the displayed age can increment over time.
    let (age, age_on) = match (&input.dob, input.age) {
        (None, Some(a)) => (Some(a), Some(now.clone())),
        _ => (None, None),
    };

    tx.execute(
        "INSERT INTO patients
            (card_first, card_sub, first_name, father_name, grandfather_name, sex, phone,
             dob_year, dob_month, dob_day, age_recorded, age_recorded_on,
             address, city, registered_at, created_by)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            card_first,
            card_sub,
            input.first_name.trim(),
            input.father_name.trim(),
            input.grandfather_name.trim(),
            input.sex,
            input.phone,
            dy,
            dm,
            dd,
            age,
            age_on,
            opt_trim(&input.address),
            opt_trim(&input.city),
            now,
            actor,
        ],
    )
    .map_err(e2s)?;
    let id = tx.last_insert_rowid();
    tx.commit().map_err(e2s)?;

    get_by_id(conn, id)?.ok_or_else(|| "Patient vanished after insert".into())
}

/// Update an existing patient. The card number is immutable and never touched.
pub fn update(conn: &Connection, id: i64, input: &PatientInput, actor: &str) -> Result<(), String> {
    validate(input)?;
    let now = now_string();
    let (dy, dm, dd) = match &input.dob {
        Some(d) => (Some(d.year), Some(d.month), Some(d.day)),
        None => (None, None, None),
    };
    let (age, age_on) = match (&input.dob, input.age) {
        (None, Some(a)) => (Some(a), Some(now.clone())),
        _ => (None, None),
    };
    let rows = conn
        .execute(
            "UPDATE patients SET
                first_name=?1, father_name=?2, grandfather_name=?3, sex=?4, phone=?5,
                dob_year=?6, dob_month=?7, dob_day=?8, age_recorded=?9, age_recorded_on=?10,
                address=?11, city=?12, updated_at=?13, updated_by=?14
             WHERE id=?15 AND deleted_at IS NULL",
            params![
                input.first_name.trim(),
                input.father_name.trim(),
                input.grandfather_name.trim(),
                input.sex,
                input.phone,
                dy,
                dm,
                dd,
                age,
                age_on,
                opt_trim(&input.address),
                opt_trim(&input.city),
                now,
                actor,
                id,
            ],
        )
        .map_err(e2s)?;
    if rows == 0 {
        return Err("Patient not found".into());
    }
    Ok(())
}

/// Soft-delete: hide the record but keep it (recoverable by an Admin).
pub fn soft_delete(conn: &Connection, id: i64, actor: &str) -> Result<(), String> {
    let rows = conn
        .execute(
            "UPDATE patients SET deleted_at=?1, deleted_by=?2
             WHERE id=?3 AND deleted_at IS NULL",
            params![now_string(), actor, id],
        )
        .map_err(e2s)?;
    if rows == 0 {
        return Err("Patient not found".into());
    }
    Ok(())
}

/// Restore a soft-deleted patient (Admin action).
pub fn restore(conn: &Connection, id: i64) -> Result<(), String> {
    let rows = conn
        .execute(
            "UPDATE patients SET deleted_at=NULL, deleted_by=NULL
             WHERE id=?1 AND deleted_at IS NOT NULL",
            params![id],
        )
        .map_err(e2s)?;
    if rows == 0 {
        return Err("Deleted patient not found".into());
    }
    Ok(())
}

/// Permanently remove a soft-deleted patient (Admin action). The card number is
/// still never reused, so the physical drawer stays aligned.
pub fn purge(conn: &Connection, id: i64) -> Result<(), String> {
    let rows = conn
        .execute(
            "DELETE FROM patients WHERE id=?1 AND deleted_at IS NOT NULL",
            params![id],
        )
        .map_err(e2s)?;
    if rows == 0 {
        return Err("Deleted patient not found".into());
    }
    Ok(())
}

/// List soft-deleted patients (Admin view), most recently deleted first.
pub fn list_deleted(conn: &Connection) -> Result<Vec<Patient>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLS} FROM patients WHERE deleted_at IS NOT NULL
             ORDER BY deleted_at DESC LIMIT 500"
        ))
        .map_err(e2s)?;
    let rows = stmt.query_map([], row_to_patient).map_err(e2s)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(e2s)
}

/// List all active patients ordered by card number (home screen view).
pub fn list_all(conn: &Connection) -> Result<Vec<Patient>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLS} FROM patients WHERE deleted_at IS NULL ORDER BY card_first, card_sub"
        ))
        .map_err(e2s)?;
    let rows = stmt.query_map([], row_to_patient).map_err(e2s)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(e2s)
}

/// One page of active patients plus the total active count.
pub fn list_page(
    conn: &Connection,
    offset: i64,
    limit: i64,
) -> Result<(Vec<Patient>, i64), String> {
    if offset < 0 {
        return Err("Offset must be non-negative".into());
    }
    if limit <= 0 {
        return Err("Limit must be greater than zero".into());
    }
    let limit = limit.min(LIST_PAGE_LIMIT_MAX);

    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .map_err(e2s)?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLS} FROM patients WHERE deleted_at IS NULL \
             ORDER BY card_first, card_sub LIMIT ?1 OFFSET ?2"
        ))
        .map_err(e2s)?;
    let rows = stmt
        .query_map(params![limit, offset], row_to_patient)
        .map_err(e2s)?;
    let patients = rows.collect::<rusqlite::Result<Vec<_>>>().map_err(e2s)?;
    Ok((patients, total))
}

// --- reads ------------------------------------------------------------------

/// Card number for a patient id (regardless of deleted state) — for the audit log.
pub fn card_of(conn: &Connection, id: i64) -> String {
    conn.query_row(
        "SELECT CASE WHEN card_sub = 0 THEN CAST(card_first AS TEXT) \
         ELSE card_first || '/' || card_sub END FROM patients WHERE id = ?1",
        params![id],
        |r| r.get::<_, String>(0),
    )
    .unwrap_or_else(|_| format!("id:{id}"))
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Patient>, String> {
    conn.query_row(
        &format!("SELECT {COLS} FROM patients WHERE id = ?1 AND deleted_at IS NULL"),
        params![id],
        row_to_patient,
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(e2s(other)),
    })
}

/// Reverse lookup: search active patients by name parts, phone, or card number.
pub fn search(conn: &Connection, query: &str) -> Result<Vec<Patient>, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    if q.contains('/') {
        if let Ok((card_first, card_sub)) = parse_card(q) {
            let exact = conn
                .query_row(
                    &format!(
                        "SELECT {COLS} FROM patients
                         WHERE deleted_at IS NULL AND card_first = ?1 AND card_sub = ?2"
                    ),
                    params![card_first, card_sub],
                    row_to_patient,
                )
                .optional()
                .map_err(e2s)?;
            if let Some(patient) = exact {
                return Ok(vec![patient]);
            }
        }
    }
    if let Ok((card_first, 0)) = parse_card(q) {
        let like = format!("%{q}%");
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {COLS} FROM patients
                 WHERE deleted_at IS NULL AND (
                     first_name LIKE ?1 OR father_name LIKE ?1 OR grandfather_name LIKE ?1
                     OR phone LIKE ?1
                     OR (card_first = ?2 AND card_sub = 0)
                 )
                 ORDER BY CASE WHEN card_first = ?2 AND card_sub = 0 THEN 0 ELSE 1 END,
                          card_first, card_sub
                 LIMIT 100"
            ))
            .map_err(e2s)?;
        let rows = stmt
            .query_map(params![like, card_first], row_to_patient)
            .map_err(e2s)?;
        let patients = rows.collect::<rusqlite::Result<Vec<_>>>().map_err(e2s)?;
        if patients.iter().any(|p| p.card_number == q) {
            return Ok(patients);
        }
    }
    let like = format!("%{q}%");
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLS} FROM patients
             WHERE deleted_at IS NULL AND (
                 first_name LIKE ?1 OR father_name LIKE ?1 OR grandfather_name LIKE ?1
                 OR phone LIKE ?1
                 OR (card_first || '/' || card_sub) LIKE ?1
             )
             ORDER BY card_first, card_sub
             LIMIT 100"
        ))
        .map_err(e2s)?;
    let rows = stmt.query_map(params![like], row_to_patient).map_err(e2s)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(e2s)
}

/// Possible duplicates: same full name OR same phone (families share phones, so
/// this is a warning signal, not a hard block).
pub fn find_duplicates(conn: &Connection, input: &PatientInput) -> Result<Vec<Patient>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLS} FROM patients
             WHERE deleted_at IS NULL AND (
                 (lower(first_name)=lower(?1) AND lower(father_name)=lower(?2)
                  AND lower(grandfather_name)=lower(?3))
                 OR phone = ?4
             )
             ORDER BY card_first, card_sub
             LIMIT 50"
        ))
        .map_err(e2s)?;
    let rows = stmt
        .query_map(
            params![
                input.first_name.trim(),
                input.father_name.trim(),
                input.grandfather_name.trim(),
                input.phone,
            ],
            row_to_patient,
        )
        .map_err(e2s)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(e2s)
}

// --- import / export --------------------------------------------------------

/// One row to import; card number is preserved from the source file.
pub struct ImportItem {
    pub row_index: usize,
    pub card_number: String,
    pub input: PatientInput,
}

#[derive(serde::Serialize)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: Vec<SkippedRow>,
}

#[derive(serde::Serialize)]
pub struct SkippedRow {
    pub row: usize,
    pub reason: String,
}

/// Import rows, preserving source card numbers. Invalid rows are skipped and
/// reported; valid rows reseed the automatic sequence so later registrations
/// continue after the highest imported or existing card.
pub fn import_rows(conn: &mut Connection, items: &[ImportItem]) -> Result<ImportReport, String> {
    let now = now_string();
    let tx = conn.transaction().map_err(e2s)?;
    normalize_card_seq(&tx).map_err(e2s)?;
    let mut imported = 0usize;
    let mut skipped: Vec<SkippedRow> = Vec::new();
    // Two-phase so a provided card number always wins over an auto-assigned one,
    // regardless of row order. Phase 1 inserts every row that carries a card number
    // (reporting invalid/duplicate rows in row order) and defers the blank rows.
    // Phase 2 auto-assigns the deferred rows, skipping any card already taken by an
    // existing patient or a preserved number from phase 1. A single row can no
    // longer collide-and-abort the whole import.
    let mut deferred: Vec<&ImportItem> = Vec::new();
    for item in items {
        if let Err(reason) = validate(&item.input) {
            skipped.push(SkippedRow {
                row: item.row_index,
                reason,
            });
            continue;
        }
        // Blank/unmapped card number → auto-assign in phase 2. A provided number is
        // preserved as-is (the clinic's frozen paper numbers).
        if item.card_number.trim().is_empty() {
            deferred.push(item);
            continue;
        }
        let (cf, cs) = match parse_card(&item.card_number) {
            Ok(card) => card,
            Err(reason) => {
                skipped.push(SkippedRow {
                    row: item.row_index,
                    reason,
                });
                continue;
            }
        };
        if card_exists(&tx, cf, cs).map_err(e2s)? {
            skipped.push(SkippedRow {
                row: item.row_index,
                reason: format!("card number '{}' already exists", item.card_number.trim()),
            });
            continue;
        }
        insert_with_card(&tx, cf, cs, &item.input, &now).map_err(e2s)?;
        imported += 1;
    }
    // Phase 2: auto-assign deferred (blank) rows in row order. The skip loop steps
    // over any card already taken; it terminates because the sequence advances
    // monotonically and the set of existing cards is finite.
    for item in deferred {
        let (cf, cs) = loop {
            let (cf, cs) = assign_next_card(&tx).map_err(e2s)?;
            if !card_exists(&tx, cf, cs).map_err(e2s)? {
                break (cf, cs);
            }
        };
        insert_with_card(&tx, cf, cs, &item.input, &now).map_err(e2s)?;
        imported += 1;
    }
    reseed_card_seq(&tx).map_err(e2s)?;
    tx.commit().map_err(e2s)?;
    Ok(ImportReport { imported, skipped })
}

fn normalize_card_seq(conn: &Connection) -> rusqlite::Result<()> {
    let (first, sub): (i64, i64) = conn.query_row(
        "SELECT next_first, next_sub FROM card_seq WHERE id = 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let (next_first, next_sub) = normalize_next_card(first, sub);
    if (next_first, next_sub) != (first, sub) {
        conn.execute(
            "UPDATE card_seq SET next_first = ?1, next_sub = ?2 WHERE id = 1",
            params![next_first, next_sub],
        )?;
    }
    Ok(())
}

fn card_exists(conn: &Connection, first: i64, sub: i64) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM patients WHERE card_first = ?1 AND card_sub = ?2)",
        params![first, sub],
        |r| r.get(0),
    )
}

fn reseed_card_seq(conn: &Connection) -> rusqlite::Result<()> {
    let max_card: Option<(i64, i64)> = conn
        .query_row(
            "SELECT card_first, card_sub FROM patients ORDER BY card_first DESC, card_sub DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((max_first, max_sub)) = max_card else {
        return Ok(());
    };
    let candidate = next_after_existing_card(max_first, max_sub);
    let current: (i64, i64) = conn.query_row(
        "SELECT next_first, next_sub FROM card_seq WHERE id = 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let current = normalize_next_card(current.0, current.1);
    if candidate > current {
        conn.execute(
            "UPDATE card_seq SET next_first = ?1, next_sub = ?2 WHERE id = 1",
            params![candidate.0, candidate.1],
        )?;
    }
    Ok(())
}

fn parse_card(s: &str) -> Result<(i64, i64), String> {
    let s = s.trim();
    let (first, sub) = if let Some((a, b)) = s.split_once('/') {
        let f: i64 = a
            .trim()
            .parse()
            .map_err(|_| format!("invalid card number '{s}'"))?;
        let u: i64 = b
            .trim()
            .parse()
            .map_err(|_| format!("invalid card number '{s}'"))?;
        (f, u)
    } else {
        let f: i64 = s
            .parse()
            .map_err(|_| format!("invalid card number '{s}'"))?;
        (f, 0)
    };
    if first < 1 || !(0..=CARD_SUB_MAX).contains(&sub) {
        return Err(format!("card number out of range '{s}'"));
    }
    // Invariant (see banner at CARD_PLAIN_MAX): cards 1..=6045 are plain sequential
    // with no sub. A below-boundary slash card like "5/1" is not a valid card number
    // in this system, so reject it here rather than let an import preserve it.
    if first <= CARD_PLAIN_MAX && sub > 0 {
        return Err(format!(
            "card number '{s}' is invalid: cards up to {CARD_PLAIN_MAX} have no sub-number"
        ));
    }
    Ok((first, sub))
}

fn insert_with_card(
    tx: &Connection,
    cf: i64,
    cs: i64,
    input: &PatientInput,
    now: &str,
) -> rusqlite::Result<()> {
    let (dy, dm, dd) = match &input.dob {
        Some(d) => (Some(d.year), Some(d.month), Some(d.day)),
        None => (None, None, None),
    };
    let (age, age_on) = match (&input.dob, input.age) {
        (None, Some(a)) => (Some(a), Some(now.to_string())),
        _ => (None, None),
    };
    tx.execute(
        "INSERT INTO patients
            (card_first, card_sub, first_name, father_name, grandfather_name, sex, phone,
             dob_year, dob_month, dob_day, age_recorded, age_recorded_on,
             address, city, registered_at, created_by)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,'import')",
        params![
            cf,
            cs,
            input.first_name.trim(),
            input.father_name.trim(),
            input.grandfather_name.trim(),
            input.sex,
            input.phone,
            dy,
            dm,
            dd,
            age,
            age_on,
            opt_trim(&input.address),
            opt_trim(&input.city),
            now,
        ],
    )?;
    Ok(())
}

/// Write all active patients to a CSV file. Returns the row count.
pub fn export_csv(conn: &Connection, path: &std::path::Path) -> Result<usize, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLS} FROM patients WHERE deleted_at IS NULL ORDER BY card_first, card_sub"
        ))
        .map_err(e2s)?;
    let rows = stmt.query_map([], row_to_patient).map_err(e2s)?;
    let mut out = String::from(
        "card_number,first_name,father_name,grandfather_name,sex,phone,dob_ec,age,city,address,registered_at\n",
    );
    let mut count = 0usize;
    for r in rows {
        let p = r.map_err(e2s)?;
        let dob = p
            .dob
            .map(|d| format!("{}-{:02}-{:02}", d.year, d.month, d.day))
            .unwrap_or_default();
        let age = p.age_recorded.map(|a| a.to_string()).unwrap_or_default();
        let fields = [
            p.card_number,
            p.first_name,
            p.father_name,
            p.grandfather_name,
            p.sex,
            p.phone,
            dob,
            age,
            p.city.unwrap_or_default(),
            p.address.unwrap_or_default(),
            p.registered_at,
        ];
        let line: Vec<String> = fields.iter().map(|f| csv_field(f)).collect();
        out.push_str(&line.join(","));
        out.push('\n');
        count += 1;
    }
    std::fs::write(path, out).map_err(e2s)?;
    Ok(count)
}

fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// All active patients (used by the CSV export command; search caps at 100).
pub fn all_active(conn: &Connection) -> Result<Vec<Patient>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLS} FROM patients WHERE deleted_at IS NULL ORDER BY card_first, card_sub"
        ))
        .map_err(e2s)?;
    let rows = stmt.query_map([], row_to_patient).map_err(e2s)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(e2s)
}

// --- row mapping ------------------------------------------------------------

const COLS: &str = "id, card_first, card_sub, first_name, father_name, grandfather_name, sex, \
     phone, dob_year, dob_month, dob_day, age_recorded, age_recorded_on, address, city, \
     registered_at";

fn row_to_patient(r: &Row) -> rusqlite::Result<Patient> {
    let card_first: i64 = r.get(1)?;
    let card_sub: i64 = r.get(2)?;
    let dob_year: Option<i32> = r.get(8)?;
    let dob_month: Option<u32> = r.get(9)?;
    let dob_day: Option<u32> = r.get(10)?;
    let dob = match (dob_year, dob_month, dob_day) {
        (Some(year), Some(month), Some(day)) => Some(EcDate { year, month, day }),
        _ => None,
    };
    Ok(Patient {
        id: r.get(0)?,
        card_number: card_number(card_first, card_sub),
        first_name: r.get(3)?,
        father_name: r.get(4)?,
        grandfather_name: r.get(5)?,
        sex: r.get(6)?,
        phone: r.get(7)?,
        dob,
        age_recorded: r.get(11)?,
        age_recorded_on: r.get(12)?,
        address: r.get(13)?,
        city: r.get(14)?,
        registered_at: r.get(15)?,
    })
}

// --- helpers ----------------------------------------------------------------

fn now_string() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn opt_trim(v: &Option<String>) -> Option<String> {
    v.as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn e2s<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: [u8; 32] = [7u8; 32];

    fn input(first: &str, father: &str, grand: &str, phone: &str) -> PatientInput {
        PatientInput {
            first_name: first.into(),
            father_name: father.into(),
            grandfather_name: grand.into(),
            sex: "Male".into(),
            phone: phone.into(),
            dob: None,
            age: Some(30),
            address: None,
            city: None,
        }
    }

    fn set_card_seq(conn: &rusqlite::Connection, first: i64, sub: i64) {
        conn.execute(
            "UPDATE card_seq SET next_first = ?1, next_sub = ?2 WHERE id = 1",
            params![first, sub],
        )
        .unwrap();
    }

    #[test]
    fn card_numbers_plain_sequential_below_6046() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let mut numbers = Vec::new();
        for i in 0..5 {
            let p = create(
                &mut conn,
                &input("A", "B", &format!("C{i}"), "0911111111"),
                "t",
            )
            .unwrap();
            numbers.push(p.card_number);
        }
        assert_eq!(numbers, vec!["1", "2", "3", "4", "5"]);
    }

    #[test]
    fn card_numbers_sub_cycle_at_and_above_6046() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        set_card_seq(&conn, 6045, 0);
        let mut numbers = Vec::new();
        for i in 0..12 {
            let phone = format!("091111{:04}", i);
            let p = create(&mut conn, &input("A", "B", &format!("C{i}"), &phone), "t").unwrap();
            numbers.push(p.card_number);
        }
        assert_eq!(
            numbers,
            vec![
                "6045", "6046", "6046/1", "6046/2", "6046/3", "6046/4", "6046/5", "6046/6",
                "6046/7", "6046/8", "6047", "6047/1"
            ]
        );
    }

    #[test]
    fn legacy_slash_card_seq_below_plain_limit_is_normalized() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        set_card_seq(&conn, 10, 5);

        let p = create(&mut conn, &input("A", "B", "C", "0911111111"), "t").unwrap();
        assert_eq!(p.card_number, "11");

        let next = create(&mut conn, &input("D", "E", "F", "0911111112"), "t").unwrap();
        assert_eq!(next.card_number, "12");
    }

    #[test]
    fn deleted_card_numbers_are_not_reused() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let first = create(&mut conn, &input("A", "B", "C", "0911111111"), "t").unwrap();
        assert_eq!(first.card_number, "1");
        soft_delete(&conn, first.id, "t").unwrap();
        let second = create(&mut conn, &input("D", "E", "F", "0911111112"), "t").unwrap();
        assert_eq!(second.card_number, "2"); // not reusing 1
    }

    #[test]
    fn phone_validation_accepts_09_and_07_ten_digits() {
        let mut ok = input("A", "B", "C", "0911111111");
        assert!(validate(&ok).is_ok());
        ok.phone = "0711111111".into();
        assert!(validate(&ok).is_ok());
    }

    #[test]
    fn phone_validation_rejects_bad_numbers() {
        for bad in [
            "091111111",
            "08911111111",
            "0911a11111",
            "1911111111",
            "09111111110",
        ] {
            let p = input("A", "B", "C", bad);
            assert!(validate(&p).is_err(), "should reject {bad}");
        }
    }

    #[test]
    fn requires_age_or_dob() {
        let mut p = input("A", "B", "C", "0911111111");
        p.age = None;
        p.dob = None;
        assert!(validate(&p).is_err());
        p.dob = Some(EcDate {
            year: 2000,
            month: 4,
            day: 15,
        });
        assert!(validate(&p).is_ok());
    }

    #[test]
    fn rejects_invalid_ethiopian_dates() {
        let mut p = input("A", "B", "C", "0911111111");
        p.age = None;
        p.dob = Some(EcDate {
            year: 2000,
            month: 14,
            day: 1,
        });
        assert!(validate(&p).is_err());
        p.dob = Some(EcDate {
            year: 2000,
            month: 13,
            day: 7,
        });
        assert!(validate(&p).is_err());
        p.dob = Some(EcDate {
            year: 2000,
            month: 13,
            day: 6,
        });
        assert!(validate(&p).is_ok());
    }

    #[test]
    fn duplicate_detection_matches_name_or_phone() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        create(
            &mut conn,
            &input("Abel", "Kebede", "Tadesse", "0911111111"),
            "t",
        )
        .unwrap();

        // same full name, different phone
        let same_name = input("abel", "kebede", "tadesse", "0922222222");
        assert_eq!(find_duplicates(&conn, &same_name).unwrap().len(), 1);

        // different name, same phone (shared family phone)
        let same_phone = input("Sara", "Kebede", "Tadesse", "0911111111");
        assert_eq!(find_duplicates(&conn, &same_phone).unwrap().len(), 1);

        // unrelated
        let neither = input("Yonas", "Girma", "Bekele", "0933333333");
        assert_eq!(find_duplicates(&conn, &neither).unwrap().len(), 0);
    }

    #[test]
    fn search_finds_by_name_phone_and_card_number() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let p = create(
            &mut conn,
            &input("Meron", "Alemu", "Tesfaye", "0911223344"),
            "t",
        )
        .unwrap();
        assert_eq!(search(&conn, "Meron").unwrap().len(), 1);
        assert_eq!(search(&conn, "1122").unwrap().len(), 1);
        assert_eq!(search(&conn, &p.card_number).unwrap().len(), 1);
        assert_eq!(search(&conn, "Nonexistent").unwrap().len(), 0);
    }

    #[test]
    fn search_bare_numeric_card_number_does_not_skip_phone_matches() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let first = create(&mut conn, &input("One", "A", "B", "0911111111"), "t").unwrap();
        let second = create(&mut conn, &input("Two", "A", "B", "0911111112"), "t").unwrap();

        assert_eq!(first.card_number, "1");
        let results = search(&conn, "1").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(
            results.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![first.id, second.id]
        );
    }

    #[test]
    fn search_bare_numeric_visible_card_does_not_match_sub_cards() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let items = vec![
            ImportItem {
                row_index: 1,
                card_number: "6046".into(),
                input: input("Exact", "A", "B", "0911111111"),
            },
            ImportItem {
                row_index: 2,
                card_number: "6046/1".into(),
                input: input("Sub", "A", "B", "0922222222"),
            },
        ];
        assert_eq!(import_rows(&mut conn, &items).unwrap().imported, 2);

        let matches = search(&conn, "6046").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].card_number, "6046");
    }

    #[test]
    fn malformed_slash_search_falls_back_to_like_search() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let p = create(&mut conn, &input("A/B", "A", "B", "0911111111"), "t").unwrap();

        let matches = search(&conn, "A/B").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, p.id);
        assert!(search(&conn, "6046/x").unwrap().is_empty());
    }

    #[test]
    fn list_page_rejects_negative_offset() {
        let conn = crate::db::open_in_memory(&TEST_KEY).unwrap();

        assert!(list_page(&conn, -1, 10).is_err());
    }

    #[test]
    fn list_page_rejects_non_positive_limit() {
        let conn = crate::db::open_in_memory(&TEST_KEY).unwrap();

        assert!(list_page(&conn, 0, 0).is_err());
        assert!(list_page(&conn, 0, -1).is_err());
    }

    #[test]
    fn list_page_caps_over_limit_values() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        for i in 0..501 {
            let phone = format!("091{i:07}");
            create(
                &mut conn,
                &input("Paged", "A", &format!("B{i}"), &phone),
                "t",
            )
            .unwrap();
        }

        let (patients, total) = list_page(&conn, 0, LIST_PAGE_LIMIT_MAX + 1).unwrap();

        assert_eq!(total, 501);
        assert_eq!(patients.len(), LIST_PAGE_LIMIT_MAX as usize);
    }

    #[test]
    fn soft_deleted_patients_are_hidden_from_search_and_get() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let p = create(&mut conn, &input("Hidden", "X", "Y", "0911223344"), "t").unwrap();
        soft_delete(&conn, p.id, "t").unwrap();
        assert!(get_by_id(&conn, p.id).unwrap().is_none());
        assert_eq!(search(&conn, "Hidden").unwrap().len(), 0);
    }

    #[test]
    fn restore_brings_a_deleted_patient_back() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let p = create(&mut conn, &input("Back", "X", "Y", "0911223344"), "t").unwrap();
        soft_delete(&conn, p.id, "t").unwrap();
        assert_eq!(list_deleted(&conn).unwrap().len(), 1);
        restore(&conn, p.id).unwrap();
        assert!(get_by_id(&conn, p.id).unwrap().is_some());
        assert_eq!(list_deleted(&conn).unwrap().len(), 0);
    }

    #[test]
    fn purge_only_removes_deleted_and_never_reuses_number() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let p = create(&mut conn, &input("Gone", "X", "Y", "0911223344"), "t").unwrap();
        assert_eq!(p.card_number, "1");
        // cannot purge an active patient
        assert!(purge(&conn, p.id).is_err());
        soft_delete(&conn, p.id, "t").unwrap();
        purge(&conn, p.id).unwrap();
        assert_eq!(list_deleted(&conn).unwrap().len(), 0);
        // next patient still gets 2, not the purged 1
        let next = create(&mut conn, &input("New", "X", "Y", "0911223345"), "t").unwrap();
        assert_eq!(next.card_number, "2");
    }

    #[test]
    fn import_preserves_cards_skips_invalid_rows_and_reseeds_sequence() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let items = vec![
            ImportItem {
                row_index: 1,
                card_number: "42".into(),
                input: input("Abel", "K", "T", "0911111111"),
            },
            ImportItem {
                row_index: 2,
                card_number: "6046/3".into(),
                input: input("Sara", "K", "T", "0922222222"),
            },
            ImportItem {
                row_index: 3,
                card_number: "43".into(),
                input: input("Bad", "K", "T", "123"),
            }, // bad phone — skipped
            ImportItem {
                row_index: 4,
                card_number: "bad-card".into(),
                input: input("BadCard", "K", "T", "0933333333"),
            },
        ];
        let report = import_rows(&mut conn, &items).unwrap();
        assert_eq!(report.imported, 2);
        assert_eq!(report.skipped.len(), 2);
        assert_eq!(report.skipped[0].row, 3);
        assert_eq!(report.skipped[1].row, 4);
        assert_eq!(search(&conn, "Abel").unwrap()[0].card_number, "42");
        assert_eq!(search(&conn, "Sara").unwrap()[0].card_number, "6046/3");
        // sequence continues after the highest imported card
        let next = create(&mut conn, &input("New", "X", "Y", "0911111119"), "t").unwrap();
        assert_eq!(next.card_number, "6046/4");
    }

    #[test]
    fn import_auto_assigns_blank_card_numbers_across_the_6046_boundary() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        set_card_seq(&conn, 6045, 0);
        let items: Vec<ImportItem> = (0..4)
            .map(|i| ImportItem {
                row_index: i + 1,
                card_number: "  ".into(), // blank → auto-assign
                input: input("Auto", "K", "T", &format!("091100{:04}", i)),
            })
            .collect();

        let report = import_rows(&mut conn, &items).unwrap();

        assert_eq!(report.imported, 4);
        assert!(report.skipped.is_empty());
        let cards: Vec<String> = search(&conn, "Auto")
            .unwrap()
            .into_iter()
            .map(|p| p.card_number)
            .collect();
        assert_eq!(cards, vec!["6045", "6046", "6046/1", "6046/2"]);
    }

    #[test]
    fn import_mixes_preserved_and_auto_assigned_card_numbers() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let items = vec![
            ImportItem {
                row_index: 1,
                card_number: "100".into(), // preserved
                input: input("Kept", "K", "T", "0911111111"),
            },
            ImportItem {
                row_index: 2,
                card_number: "".into(), // auto → 1
                input: input("Auto", "K", "T", "0922222222"),
            },
        ];

        let report = import_rows(&mut conn, &items).unwrap();

        assert_eq!(report.imported, 2);
        assert_eq!(search(&conn, "Kept").unwrap()[0].card_number, "100");
        assert_eq!(search(&conn, "Auto").unwrap()[0].card_number, "1");
        // sequence continues past the highest card (100)
        let next = create(&mut conn, &input("New", "X", "Y", "0933333333"), "t").unwrap();
        assert_eq!(next.card_number, "101");
    }

    #[test]
    fn import_provided_low_card_before_blank_does_not_abort() {
        // Scenario 1: a provided card equal to what the sequence would auto-assign
        // appears BEFORE a blank row. The blank row must skip the taken card, not
        // collide and abort the whole import.
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let items = vec![
            ImportItem {
                row_index: 1,
                card_number: "1".into(), // preserved, equals the next auto value
                input: input("Kept", "K", "T", "0911111111"),
            },
            ImportItem {
                row_index: 2,
                card_number: "".into(), // auto → must become 2, not collide with 1
                input: input("Auto", "K", "T", "0922222222"),
            },
        ];

        let report = import_rows(&mut conn, &items).unwrap();

        assert_eq!(report.imported, 2);
        assert!(report.skipped.is_empty());
        assert_eq!(search(&conn, "Kept").unwrap()[0].card_number, "1");
        assert_eq!(search(&conn, "Auto").unwrap()[0].card_number, "2");
    }

    #[test]
    fn import_blank_before_provided_preserves_the_provided_card() {
        // Scenario 2: a blank row appears BEFORE a row that provides the card the
        // blank would otherwise consume. The provided number must be preserved and
        // the blank row auto-assigned to a different card.
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let items = vec![
            ImportItem {
                row_index: 1,
                card_number: "".into(), // auto — must NOT steal card 1
                input: input("Auto", "K", "T", "0911111111"),
            },
            ImportItem {
                row_index: 2,
                card_number: "1".into(), // preserved — must survive
                input: input("Kept", "K", "T", "0922222222"),
            },
        ];

        let report = import_rows(&mut conn, &items).unwrap();

        assert_eq!(report.imported, 2);
        assert!(report.skipped.is_empty());
        assert_eq!(search(&conn, "Kept").unwrap()[0].card_number, "1");
        assert_eq!(search(&conn, "Auto").unwrap()[0].card_number, "2");
    }

    #[test]
    fn parse_card_accepts_bare_integer_as_sub_zero() {
        assert_eq!(parse_card("6045").unwrap(), (6045, 0));
        assert_eq!(parse_card(" 100 ").unwrap(), (100, 0));
    }

    #[test]
    fn parse_card_rejects_below_boundary_slash_cards() {
        // 1..=6045 are plain; a sub below the boundary is not a valid card.
        assert!(parse_card("5/1").is_err());
        assert!(parse_card("6045/1").is_err());
        // The boundary itself and above still allow subs.
        assert_eq!(parse_card("6046/1").unwrap(), (6046, 1));
    }

    #[test]
    fn import_skips_below_boundary_slash_card_and_keeps_valid_rows() {
        let mut conn = crate::db::open_in_memory(&TEST_KEY).unwrap();
        let items = vec![
            ImportItem {
                row_index: 1,
                card_number: "5/1".into(), // invalid: below-boundary slash → skipped
                input: input("BadSub", "K", "T", "0911111111"),
            },
            ImportItem {
                row_index: 2,
                card_number: "6046/1".into(), // valid above-boundary sub-card
                input: input("Good", "K", "T", "0922222222"),
            },
        ];

        let report = import_rows(&mut conn, &items).unwrap();

        assert_eq!(report.imported, 1);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].row, 1);
        assert!(report.skipped[0].reason.contains("sub-number"));
        assert_eq!(search(&conn, "Good").unwrap()[0].card_number, "6046/1");
        assert!(search(&conn, "BadSub").unwrap().is_empty());
    }

    #[test]
    fn card_number_omits_sub_when_zero() {
        assert_eq!(card_number(6045, 0), "6045");
        assert_eq!(card_number(6046, 1), "6046/1");
        assert_eq!(card_number(6046, 8), "6046/8");
    }
}

// ── Stats / Reports ──────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct CityCount {
    pub city: String,
    pub count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct PatientStats {
    pub total: i64,
    pub registered_this_month: i64,
    pub registered_this_year: i64,
    pub male: i64,
    pub female: i64,
    pub cities: Vec<CityCount>,
}

pub fn get_stats(conn: &Connection) -> rusqlite::Result<PatientStats> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;

    let registered_this_month: i64 = conn.query_row(
        "SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL \
         AND strftime('%Y-%m', registered_at) = strftime('%Y-%m', 'now')",
        [],
        |r| r.get(0),
    )?;

    let registered_this_year: i64 = conn.query_row(
        "SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL \
         AND strftime('%Y', registered_at) = strftime('%Y', 'now')",
        [],
        |r| r.get(0),
    )?;

    let male: i64 = conn.query_row(
        "SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL AND sex = 'Male'",
        [],
        |r| r.get(0),
    )?;

    let female: i64 = conn.query_row(
        "SELECT COUNT(*) FROM patients WHERE deleted_at IS NULL AND sex = 'Female'",
        [],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT COALESCE(NULLIF(TRIM(city), ''), 'Unknown') as city, COUNT(*) as cnt \
         FROM patients WHERE deleted_at IS NULL \
         GROUP BY city ORDER BY cnt DESC LIMIT 10",
    )?;
    let cities = stmt
        .query_map([], |r| {
            Ok(CityCount {
                city: r.get(0)?,
                count: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(PatientStats {
        total,
        registered_this_month,
        registered_this_year,
        male,
        female,
        cities,
    })
}
