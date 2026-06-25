import type { Patient } from "./api";
import { ageFromEcDob, ageFromRecorded, formatEcDate } from "./ethiopian";

export function fullName(p: Patient): string {
  return `${p.first_name} ${p.father_name} ${p.grandfather_name}`;
}

export function displayAge(p: Patient): string {
  if (p.dob) return `${ageFromEcDob(p.dob)} yrs`;
  if (p.age_recorded != null && p.age_recorded_on) {
    return `${ageFromRecorded(p.age_recorded, p.age_recorded_on)} yrs`;
  }
  if (p.age_recorded != null) return `${p.age_recorded} yrs`;
  return "—";
}

export function displayDob(p: Patient): string {
  return p.dob ? formatEcDate(p.dob) : "DOB unknown";
}
