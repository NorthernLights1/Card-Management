# AGENTS.md

Guidance for AI agents (Codex, Claude, etc.) working in this repo.

## Review posture

Fix real defects, but **do not change behavior**. This clinic app has several
deliberate rules that look like inconsistencies but are explicit client
requirements. If you think one is wrong, **flag it in your review — do not edit
the code**. The source of truth for intended behavior is
[FEATURE_CONTRACT.md](FEATURE_CONTRACT.md); when code and your intuition disagree,
the contract wins.

## Intentional behaviors — DO NOT "fix" these

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
