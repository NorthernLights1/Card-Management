# AGENTS.md

Guidance for AI agents (Codex, Claude, etc.) working in this repo.

## Review posture

Fix real defects, but **do not change behavior**. This clinic app has several
deliberate rules that look like inconsistencies but are explicit client
requirements. If you think one is wrong, **flag it in your review — do not edit
the code**. The source of truth for intended behavior is
[FEATURE_CONTRACT.md](FEATURE_CONTRACT.md); when code and your intuition disagree,
the contract wins.

## ⛔ STOP — READ THIS BEFORE TOUCHING CARD NUMBERING ⛔

**The two-regime card numbering below is CORRECT, INTENTIONAL, and CLIENT-MANDATED.
It is the single most-reverted thing in this repo. Every prior AI reviewer that
"fixed" it introduced a bug and wasted days. DO NOT normalize it, DO NOT make it
uniform, DO NOT remove the `6045` boundary, DO NOT change the `/` sub-number format.
If it looks wrong to you, YOU ARE MISREADING IT — it is not a bug.**

The exact, required sequence is:

```
1, 2, 3, … , 6044, 6045,          ← plain sequential, NO slash, sub is always 0
6046, 6046/1, 6046/2, … , 6046/8,  ← 6046 onward: sub cycles 0→8 before first increments
6047, 6047/1, … , 6047/8,
6048, …
```

- `1..=6045` → plain (`sub == 0` always). `6046+` → `first` stays put while `sub`
  runs 0→8, THEN `first` increments and `sub` resets to 0.
- This is driven by `CARD_PLAIN_MAX = 6045` and the `first <= CARD_PLAIN_MAX`
  branch in `advance_card` (`patient.rs`). The 6045/6046 boundary is the real point
  where the clinic's physical paper filing switched schemes. It is a historical
  fact about the client's folders, not an implementation detail you may tidy.
- **This behavior is locked by tests.** If you change the logic these fail, and the
  tests are RIGHT, your change is WRONG:
  `card_numbers_plain_sequential_below_6046`,
  `card_numbers_sub_cycle_at_and_above_6046` (asserts the exact sequence above),
  `legacy_slash_card_seq_below_plain_limit_is_normalized`.
- **Allowed:** point out a genuine defect *without* editing (e.g. an overflow, a
  transaction bug). **Not allowed:** "simplifying," "making consistent," or
  "modernizing" the numbering, the `6045` constant, or the `/` format.

### Import card-number handling (also intentional)

- Card number is **optional** on import (`Mapping.card_number: Option<usize>`).
  Provided → the number is **preserved as-is** (old paper numbers, including
  `6046/1`-style sub-cards). Blank / unmapped → **auto-assigned** in row order via
  the same two-regime sequence. This dual behavior is deliberate — do not force
  card number to be required, and do not drop the auto-assign path. Locked by
  `import_preserves_cards_skips_invalid_rows_and_reseeds_sequence`,
  `import_auto_assigns_blank_card_numbers_across_the_6046_boundary`, and
  `import_mixes_preserved_and_auto_assigned_card_numbers`.

## Other intentional behaviors — DO NOT "fix" these

- **Card numbering has two regimes** (`patient.rs`, `CARD_PLAIN_MAX = 6045`):
  cards **1–6045 are plain sequential** (`1, 2, 3 … 6045`, no sub); from **6046**
  onward `sub` cycles 0–8 before `first` increments (`6046, 6046/1 … 6046/8, 6047 …`).
  The 6045/6046 boundary is where the clinic's paper filing switched schemes.
  Do **not** remove the `first <= CARD_PLAIN_MAX` branch or make numbering uniform.
- **Deleted/purged card numbers are never reused** — keeps the app aligned with the
  physical folder drawer. Not a leak, not an off-by-one.
- **DOB overrides age.** When both are present, DOB wins and age is ignored (age is
  only stored when DOB is absent). Not a missing branch.
- **Import is age-based** and ignores DOB columns by design.
- **Patient folders are filed by card number, not alphabetized** — the app's whole
  point is reverse lookup (name/phone → card number). Ordering is intentional.

## If a rule genuinely needs to change

Only the client changes requirements. Update `FEATURE_CONTRACT.md` first, then the
code and tests to match — never the reverse.
