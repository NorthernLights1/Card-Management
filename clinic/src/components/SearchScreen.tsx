import { useEffect, useState } from "react";
import type { Patient } from "../lib/api";
import { deletePatient, searchPatients } from "../lib/api";
import { displayAge, displayDob, fullName } from "../lib/patientView";

type Props = {
  onRegister: () => void;
  onEdit: (patient: Patient) => void;
  onPrint: (patient: Patient) => void;
};

export function SearchScreen({ onRegister, onEdit, onPrint }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Patient[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    const q = query.trim();
    if (q === "") {
      setResults([]);
      return;
    }
    let active = true;
    const id = setTimeout(() => {
      searchPatients(q)
        .then((rows) => active && setResults(rows))
        .catch((e) => active && setError(String(e)));
    }, 200);
    return () => {
      active = false;
      clearTimeout(id);
    };
  }, [query, reloadKey]);

  const remove = async (p: Patient) => {
    if (!window.confirm(`Delete ${fullName(p)} (card ${p.card_number})? This can be restored by an Admin.`)) {
      return;
    }
    try {
      await deletePatient(p.id);
      setReloadKey((k) => k + 1);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <>
      <div className="toolbar">
        <div className="search">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search by name, phone, or card number…"
            autoFocus
          />
        </div>
        <button className="primary" onClick={onRegister}>
          + Register patient
        </button>
      </div>

      {error && <div className="banner error">{error}</div>}

      {query.trim() === "" ? (
        <div className="empty">
          Search a patient's name, phone, or card number to find their card.
        </div>
      ) : results.length === 0 ? (
        <div className="empty">No matching patients.</div>
      ) : (
        results.map((p) => (
          <div className="result-row" key={p.id}>
            <span className="card-badge">{p.card_number}</span>
            <div>
              <div className="result-name">{fullName(p)}</div>
              <div className="result-meta">
                {p.sex} · {displayAge(p)} · {displayDob(p)} · {p.phone}
                {p.city ? ` · ${p.city}` : ""}
              </div>
            </div>
            <div className="result-actions">
              <button onClick={() => onPrint(p)}>Print card</button>
              <button onClick={() => onEdit(p)}>Edit</button>
              <button className="danger" onClick={() => remove(p)}>
                Delete
              </button>
            </div>
          </div>
        ))
      )}
    </>
  );
}
