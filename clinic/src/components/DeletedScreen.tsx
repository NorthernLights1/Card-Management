import { useEffect, useState } from "react";
import type { Patient } from "../lib/api";
import { listDeletedPatients, purgePatient, restorePatient } from "../lib/api";
import { displayAge, fullName } from "../lib/patientView";

type Props = { onBack: () => void; isAdmin: boolean };

export function DeletedScreen({ onBack, isAdmin }: Props) {
  const [rows, setRows] = useState<Patient[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = () => {
    listDeletedPatients()
      .then(setRows)
      .catch((e) => setError(String(e)));
  };

  useEffect(load, []);

  const restore = async (p: Patient) => {
    try {
      await restorePatient(p.id);
      load();
    } catch (e) {
      setError(String(e));
    }
  };

  const purge = async (p: Patient) => {
    if (!window.confirm(`Permanently delete ${fullName(p)} (card ${p.card_number})? This cannot be undone.`)) {
      return;
    }
    try {
      await purgePatient(p.id);
      load();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <>
      <div className="toolbar">
        <button className="ghost" onClick={onBack}>
          ← Back to search
        </button>
        <h1 className="page" style={{ margin: 0 }}>
          Deleted patients
        </h1>
      </div>

      {error && <div className="banner error">{error}</div>}

      {rows.length === 0 ? (
        <div className="empty">No deleted patients.</div>
      ) : (
        rows.map((p) => (
          <div className="result-row" key={p.id}>
            <span className="card-badge">{p.card_number}</span>
            <div>
              <div className="result-name">{fullName(p)}</div>
              <div className="result-meta">
                {p.sex} · {displayAge(p)} · {p.phone}
              </div>
            </div>
            <div className="result-actions">
              <button className="primary" onClick={() => restore(p)}>
                Restore
              </button>
              {isAdmin && (
                <button className="danger" onClick={() => purge(p)}>
                  Delete forever
                </button>
              )}
            </div>
          </div>
        ))
      )}
    </>
  );
}
