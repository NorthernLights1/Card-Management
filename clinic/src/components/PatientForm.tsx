import { useState } from "react";
import type { EcDate } from "../lib/ethiopian";
import type { Patient, PatientInput, Sex } from "../lib/api";
import { checkDuplicates, registerPatient, updatePatient } from "../lib/api";
import { fullName, displayAge } from "../lib/patientView";
import { EcDateInput } from "./EcDateInput";

type Props = {
  patient: Patient | null; // null = register, otherwise edit
  onSaved: () => void;
  onCancel: () => void;
};

type AgeMode = "dob" | "age";

export function PatientForm({ patient, onSaved, onCancel }: Props) {
  const isEdit = patient !== null;
  const [first, setFirst] = useState(patient?.first_name ?? "");
  const [father, setFather] = useState(patient?.father_name ?? "");
  const [grand, setGrand] = useState(patient?.grandfather_name ?? "");
  const [sex, setSex] = useState<Sex | "">(patient?.sex ?? "");
  const [phone, setPhone] = useState(patient?.phone ?? "");
  const [address, setAddress] = useState(patient?.address ?? "");
  const [city, setCity] = useState(patient?.city ?? "");
  const [ageMode, setAgeMode] = useState<AgeMode>(patient?.dob ? "dob" : "age");
  const [dob, setDob] = useState<EcDate | null>(patient?.dob ?? null);
  const [age, setAge] = useState<string>(
    patient?.age_recorded != null ? String(patient.age_recorded) : "",
  );

  const [error, setError] = useState<string | null>(null);
  const [dupes, setDupes] = useState<Patient[] | null>(null);
  const [busy, setBusy] = useState(false);

  const buildInput = (): PatientInput => ({
    first_name: first,
    father_name: father,
    grandfather_name: grand,
    sex: (sex || "Male") as Sex,
    phone,
    dob: ageMode === "dob" ? dob : null,
    age: ageMode === "age" && age !== "" ? Number(age) : null,
    address: address.trim() === "" ? null : address,
    city: city.trim() === "" ? null : city,
  });

  const save = async (skipDupCheck: boolean) => {
    setError(null);
    if (sex === "") {
      setError("Please select Male or Female.");
      return;
    }
    const input = buildInput();
    setBusy(true);
    try {
      if (isEdit) {
        await updatePatient(patient!.id, input);
        onSaved();
        return;
      }
      if (!skipDupCheck) {
        const found = await checkDuplicates(input);
        if (found.length > 0) {
          setDupes(found);
          setBusy(false);
          return;
        }
      }
      await registerPatient(input);
      onSaved();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  return (
    <div className="form-card">
      <h1 className="page">{isEdit ? `Edit patient · ${patient!.card_number}` : "Register new patient"}</h1>

      {error && <div className="banner error">{error}</div>}

      <div className="form-grid">
        <div>
          <label>First name</label>
          <input value={first} onChange={(e) => setFirst(e.target.value)} autoFocus />
        </div>
        <div>
          <label>Father's name</label>
          <input value={father} onChange={(e) => setFather(e.target.value)} />
        </div>
        <div>
          <label>Grandfather's name</label>
          <input value={grand} onChange={(e) => setGrand(e.target.value)} />
        </div>
        <div>
          <label>Sex</label>
          <select value={sex} onChange={(e) => setSex(e.target.value as Sex)}>
            <option value="">Select…</option>
            <option value="Male">Male</option>
            <option value="Female">Female</option>
          </select>
        </div>
        <div>
          <label>Phone</label>
          <input
            value={phone}
            onChange={(e) => setPhone(e.target.value.replace(/\D/g, "").slice(0, 10))}
            placeholder="09… or 07…"
            inputMode="numeric"
          />
        </div>
        <div>
          <label>City</label>
          <input value={city} onChange={(e) => setCity(e.target.value)} />
        </div>

        <div className="full">
          <label>Age or date of birth</label>
          <div className="radio-row">
            <label>
              <input
                type="radio"
                checked={ageMode === "age"}
                onChange={() => setAgeMode("age")}
              />
              Age only (DOB unknown)
            </label>
            <label>
              <input
                type="radio"
                checked={ageMode === "dob"}
                onChange={() => setAgeMode("dob")}
              />
              Known date of birth
            </label>
          </div>
          {ageMode === "age" ? (
            <input
              type="number"
              min={0}
              max={200}
              value={age}
              onChange={(e) => setAge(e.target.value)}
              placeholder="Age in years"
              style={{ maxWidth: 180 }}
            />
          ) : (
            <EcDateInput value={dob} onChange={setDob} />
          )}
        </div>

        <div className="full">
          <label>Address</label>
          <input value={address} onChange={(e) => setAddress(e.target.value)} />
        </div>
      </div>

      <div className="form-actions">
        <button className="ghost" onClick={onCancel} disabled={busy}>
          Cancel
        </button>
        <button className="primary" onClick={() => save(false)} disabled={busy}>
          {busy ? "Saving…" : isEdit ? "Save changes" : "Register patient"}
        </button>
      </div>

      {dupes && (
        <div className="modal-backdrop" role="dialog" aria-modal="true">
          <div className="modal">
            <h3>Possible duplicate</h3>
            <p className="muted">
              {dupes.length} existing patient{dupes.length > 1 ? "s" : ""} match this name or phone.
              Register anyway only if this is a different person.
            </p>
            <div style={{ margin: "12px 0" }}>
              {dupes.map((d) => (
                <div className="result-row" key={d.id} style={{ gridTemplateColumns: "auto 1fr" }}>
                  <span className="card-badge">{d.card_number}</span>
                  <div>
                    <div className="result-name">{fullName(d)}</div>
                    <div className="result-meta">
                      {d.sex} · {displayAge(d)} · {d.phone}
                    </div>
                  </div>
                </div>
              ))}
            </div>
            <div className="form-actions">
              <button className="ghost" onClick={() => setDupes(null)}>
                Go back
              </button>
              <button
                className="primary"
                onClick={() => {
                  setDupes(null);
                  void save(true);
                }}
              >
                Register anyway
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
