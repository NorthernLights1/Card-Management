// Ethiopian <-> Gregorian calendar conversion and live age computation.
//
// EC<->GC conversion is a fixed, well-documented algorithm via Julian Day Number.
// The epoch constant below is calibrated against known Ethiopian New Year dates
// (see ethiopian.test.ts) rather than trusted blindly — that's the one place an
// off-by-one would hide, so it's pinned by tests.

export type EcDate = { year: number; month: number; day: number };

// JDN of 1 Meskerem, year 1 (Amete Mihret). Calibrated by tests.
const ETHIOPIC_EPOCH = 1724221;

export const ETHIOPIAN_MONTHS = [
  "Meskerem", "Tikimt", "Hidar", "Tahsas", "Tir", "Yekatit",
  "Megabit", "Miazia", "Ginbot", "Sene", "Hamle", "Nehase", "Pagume",
] as const;

function ethiopicToJdn(year: number, month: number, day: number): number {
  return ETHIOPIC_EPOCH + 365 * (year - 1) + Math.floor(year / 4) + 30 * (month - 1) + (day - 1);
}

function jdnToEthiopic(jdn: number): EcDate {
  const n = jdn - ETHIOPIC_EPOCH; // 0-based days since 1 Meskerem year 1
  let year = Math.floor((4 * n + 3) / 1461) + 1;
  let yearStart = 365 * (year - 1) + Math.floor(year / 4);
  if (n < yearStart) {
    year -= 1;
    yearStart = 365 * (year - 1) + Math.floor(year / 4);
  }
  const dayOfYear = n - yearStart;
  return {
    year,
    month: Math.floor(dayOfYear / 30) + 1,
    day: (dayOfYear % 30) + 1,
  };
}

function gregorianToJdn(year: number, month: number, day: number): number {
  const a = Math.floor((14 - month) / 12);
  const y = year + 4800 - a;
  const m = month + 12 * a - 3;
  return (
    day +
    Math.floor((153 * m + 2) / 5) +
    365 * y +
    Math.floor(y / 4) -
    Math.floor(y / 100) +
    Math.floor(y / 400) -
    32045
  );
}

function jdnToGregorian(jdn: number): { year: number; month: number; day: number } {
  const a = jdn + 32044;
  const b = Math.floor((4 * a + 3) / 146097);
  const c = a - Math.floor((146097 * b) / 4);
  const d = Math.floor((4 * c + 3) / 1461);
  const e = c - Math.floor((1461 * d) / 4);
  const m = Math.floor((5 * e + 2) / 153);
  return {
    day: e - Math.floor((153 * m + 2) / 5) + 1,
    month: m + 3 - 12 * Math.floor(m / 10),
    year: 100 * b + d - 4800 + Math.floor(m / 10),
  };
}

/** Ethiopian leap year: the year before a Gregorian leap year (year % 4 === 3). */
export function isEthiopianLeapYear(year: number): boolean {
  return year % 4 === 3;
}

/** Days in an Ethiopian month: 30 for months 1-12, 5 or 6 for Pagume (month 13). */
export function ethiopianMonthLength(year: number, month: number): number {
  if (month < 13) return 30;
  return isEthiopianLeapYear(year) ? 6 : 5;
}

export function ecToGregorian(dob: EcDate): { year: number; month: number; day: number } {
  return jdnToGregorian(ethiopicToJdn(dob.year, dob.month, dob.day));
}

export function gregorianToEc(year: number, month: number, day: number): EcDate {
  return jdnToEthiopic(gregorianToJdn(year, month, day));
}

/** Today in the Ethiopian calendar, from the system clock. */
export function todayEc(now: Date = new Date()): EcDate {
  return gregorianToEc(now.getFullYear(), now.getMonth() + 1, now.getDate());
}

function ecToDate(dob: EcDate): Date {
  const g = ecToGregorian(dob);
  return new Date(g.year, g.month - 1, g.day);
}

function wholeYearsBetween(from: Date, to: Date): number {
  let years = to.getFullYear() - from.getFullYear();
  const monthDiff = to.getMonth() - from.getMonth();
  if (monthDiff < 0 || (monthDiff === 0 && to.getDate() < from.getDate())) {
    years -= 1;
  }
  return Math.max(0, years);
}

/** Age from an Ethiopian DOB, as of `now`. */
export function ageFromEcDob(dob: EcDate, now: Date = new Date()): number {
  return wholeYearsBetween(ecToDate(dob), now);
}

/**
 * Live age for a patient who was registered with an age (DOB unknown): the
 * recorded age plus whole years elapsed since it was recorded. Always correct,
 * no yearly maintenance.
 */
export function ageFromRecorded(
  ageRecorded: number,
  recordedOn: string,
  now: Date = new Date(),
): number {
  const recorded = parseSystemDate(recordedOn);
  if (!recorded) return ageRecorded;
  return ageRecorded + wholeYearsBetween(recorded, now);
}

function parseSystemDate(value: string): Date | null {
  // Stored as "YYYY-MM-DD HH:MM:SS" (system clock).
  const m = value.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (!m) return null;
  return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
}

export function formatEcDate(dob: EcDate): string {
  const name = ETHIOPIAN_MONTHS[dob.month - 1] ?? `M${dob.month}`;
  return `${dob.day} ${name} ${dob.year}`;
}
