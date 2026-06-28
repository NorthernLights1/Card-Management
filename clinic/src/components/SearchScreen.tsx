import { useEffect, useMemo, useState } from "react";
import type { Patient, Sex } from "../lib/api";
import { deletePatient, listPatients, searchPatients } from "../lib/api";
import { displayAge, displayDob, fullName } from "../lib/patientView";

type Props = {
  onRegister: () => void;
  onEdit: (patient: Patient) => void;
};

export function SearchScreen({ onRegister, onEdit }: Props) {
  const [allPatients, setAllPatients] = useState<Patient[]>([]);
  const [searchResults, setSearchResults] = useState<Patient[] | null>(null);
  const [query, setQuery] = useState("");
  const [sexFilter, setSexFilter] = useState<"" | Sex>("");
  const [cityFilter, setCityFilter] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    listPatients()
      .then(setAllPatients)
      .catch((e) => setError(String(e)));
  }, [reloadKey]);

  useEffect(() => {
    const q = query.trim();
    if (q === "") {
      setSearchResults(null);
      return;
    }
    let active = true;
    const id = setTimeout(() => {
      searchPatients(q)
        .then((rows) => { if (active) setSearchResults(rows); })
        .catch((e) => { if (active) setError(String(e)); });
    }, 200);
    return () => { active = false; clearTimeout(id); };
  }, [query]);

  const cities = useMemo(
    () => [...new Set(allPatients.map((p) => p.city).filter(Boolean) as string[])].sort(),
    [allPatients],
  );

  const displayed = (searchResults ?? allPatients).filter((p) => {
    if (sexFilter && p.sex !== sexFilter) return false;
    if (cityFilter && p.city !== cityFilter) return false;
    return true;
  });

  const remove = async (p: Patient) => {
    if (!window.confirm(`Delete ${fullName(p)} (card ${p.card_number})? This can be restored by an Admin.`)) return;
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
        <select value={sexFilter} onChange={(e) => setSexFilter(e.target.value as "" | Sex)}>
          <option value="">All sexes</option>
          <option value="Male">Male</option>
          <option value="Female">Female</option>
        </select>
        <select value={cityFilter} onChange={(e) => setCityFilter(e.target.value)}>
          <option value="">All cities</option>
          {cities.map((c) => (
            <option key={c} value={c}>{c}</option>
          ))}
        </select>
        <div style={{ flex: 1 }} />
        <button className="primary" onClick={onRegister}>
          + Register patient
        </button>
      </div>
      <div className="search">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search by name, phone, or card number…"
          autoFocus
        />
      </div>

      {error && <div className="banner error">{error}</div>}

      <div className="patient-count muted" style={{ padding: "4px 0 8px", fontSize: 13 }}>
        {displayed.length === allPatients.length
          ? `${allPatients.length} patient${allPatients.length === 1 ? "" : "s"}`
          : `${displayed.length} of ${allPatients.length} patient${allPatients.length === 1 ? "" : "s"}`}
      </div>

      {displayed.length === 0 && allPatients.length === 0 ? (
        <div className="empty">No patients registered yet. Register the first one.</div>
      ) : displayed.length === 0 ? (
        <div className="empty">No patients match the current filters.</div>
      ) : (
        displayed.map((p) => (
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
