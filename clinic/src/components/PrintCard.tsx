import type { Patient } from "../lib/api";
import { displayAge, displayDob, fullName } from "../lib/patientView";

// Rendered hidden on screen; only visible when printing (see .print-card in App.css).
export function PrintCard({ patient }: { patient: Patient }) {
  return (
    <div className="print-card">
      <div className="pc-title">Clinic Patient Card</div>
      <div className="pc-number">{patient.card_number}</div>
      <div className="pc-name">{fullName(patient)}</div>
      <div className="pc-line">
        {patient.sex} · {displayAge(patient)} · {displayDob(patient)}
      </div>
      <div className="pc-line">Phone: {patient.phone}</div>
      {patient.city && <div className="pc-line">City: {patient.city}</div>}
    </div>
  );
}
