import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { ImportMapping, ImportPreview, ImportReport } from "../lib/api";
import { importApply, importPreview } from "../lib/api";

type Props = { onClose: () => void };

type ImportFieldKey = keyof ImportMapping;
type FieldDef = { key: ImportFieldKey; label: string; required: boolean; syn: string[] };

const FIELDS: FieldDef[] = [
  { key: "card_number", label: "Card number", required: true, syn: ["card", "cardnumber"] },
  { key: "first_name", label: "First name", required: true, syn: ["first", "fname", "name"] },
  { key: "father_name", label: "Father's name", required: true, syn: ["father"] },
  { key: "grandfather_name", label: "Grandfather's name", required: true, syn: ["grandfather"] },
  { key: "sex", label: "Sex", required: true, syn: ["sex", "gender"] },
  { key: "phone", label: "Phone", required: true, syn: ["phone", "mobile", "tel"] },
  { key: "age", label: "Age", required: false, syn: ["age"] },
  { key: "city", label: "City", required: false, syn: ["city", "town"] },
  { key: "address", label: "Address", required: false, syn: ["address", "addr"] },
];

const norm = (s: string) => s.toLowerCase().replace(/[^a-z]/g, "");

function guessMapping(headers: string[]): Record<string, number> {
  const map: Record<string, number> = {};
  for (const f of FIELDS) {
    const idx = headers.findIndex((h) => f.syn.some((s) => norm(h).includes(s)));
    map[f.key] = idx; // -1 if not found
  }
  return map;
}

export function ImportScreen({ onClose }: Props) {
  const [path, setPath] = useState<string | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [mapping, setMapping] = useState<Record<string, number>>({});
  const [report, setReport] = useState<ImportReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const pickFile = async () => {
    setError(null);
    const file = await open({
      multiple: false,
      filters: [{ name: "Spreadsheet", extensions: ["xlsx", "xls", "csv"] }],
    });
    if (typeof file !== "string") return;
    setBusy(true);
    try {
      const pv = await importPreview(file);
      setPath(file);
      setPreview(pv);
      setMapping(guessMapping(pv.headers));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const missingRequired = preview
    ? FIELDS.filter((f) => f.required && (mapping[f.key] ?? -1) < 0).map((f) => f.label)
    : [];

  const apply = async () => {
    if (!path) return;
    setError(null);
    setBusy(true);
    try {
      const m: ImportMapping = {
        card_number: mapping.card_number,
        first_name: mapping.first_name,
        father_name: mapping.father_name,
        grandfather_name: mapping.grandfather_name,
        sex: mapping.sex,
        phone: mapping.phone,
        age: mapping.age >= 0 ? mapping.age : null,
        city: mapping.city >= 0 ? mapping.city : null,
        address: mapping.address >= 0 ? mapping.address : null,
      };
      setReport(await importApply(path, m));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true">
      <div className="modal" style={{ maxWidth: 640 }}>
        <h3>Import patients from a spreadsheet</h3>
        {error && <div className="banner error">{error}</div>}

        {!preview ? (
          <>
            <p className="muted">
              Choose an Excel (.xlsx) or CSV file. The first row must be column headers. Card numbers
              are imported from the mapped column. Ages are imported; dates of birth can be added later by editing.
            </p>
            <div className="form-actions">
              <button className="ghost" onClick={onClose}>Cancel</button>
              <button className="primary" onClick={pickFile} disabled={busy}>
                {busy ? "Reading…" : "Choose file…"}
              </button>
            </div>
          </>
        ) : report ? (
          <>
            <div className="banner warn">
              Imported <strong>{report.imported}</strong> patient{report.imported === 1 ? "" : "s"}.
              {report.skipped.length > 0 && <> Skipped <strong>{report.skipped.length}</strong>.</>}
            </div>
            {report.skipped.length > 0 && (
              <pre style={{ maxHeight: 220, overflow: "auto", fontSize: 12, background: "var(--surface-2)", border: "1px solid var(--line)", borderRadius: 8, padding: 12 }}>
                {report.skipped.map((s) => `Row ${s.row}: ${s.reason}`).join("\n")}
              </pre>
            )}
            <div className="form-actions">
              <button className="primary" onClick={onClose}>Done</button>
            </div>
          </>
        ) : (
          <>
            <p className="muted">
              {preview.total_rows} data row{preview.total_rows === 1 ? "" : "s"}. Match your columns to the
              fields below.
            </p>
            <div className="form-grid">
              {FIELDS.map((f) => (
                <div key={f.key}>
                  <label>
                    {f.label}
                    {f.required ? " *" : " (optional)"}
                  </label>
                  <select
                    value={mapping[f.key] ?? -1}
                    onChange={(e) => setMapping({ ...mapping, [f.key]: Number(e.target.value) })}
                  >
                    {!f.required && <option value={-1}>— none —</option>}
                    {preview.headers.map((h, i) => (
                      <option key={i} value={i}>
                        {h || `Column ${i + 1}`}
                      </option>
                    ))}
                  </select>
                </div>
              ))}
            </div>
            {missingRequired.length > 0 && (
              <div className="banner warn" style={{ marginTop: 12 }}>
                Please choose a column for: {missingRequired.join(", ")}.
              </div>
            )}
            <div className="form-actions">
              <button className="ghost" onClick={onClose}>Cancel</button>
              <button
                className="primary"
                onClick={apply}
                disabled={busy || missingRequired.length > 0}
              >
                {busy ? "Importing…" : "Import"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
