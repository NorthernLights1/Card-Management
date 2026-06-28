import { useEffect, useState } from "react";
import type { PatientStats } from "../lib/api";
import { getPatientStats } from "../lib/api";

type Props = { onBack: () => void };

export function ReportsScreen({ onBack }: Props) {
  const [stats, setStats] = useState<PatientStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getPatientStats().then(setStats).catch((e) => setError(String(e)));
  }, []);

  const reload = () => {
    setStats(null);
    setError(null);
    getPatientStats().then(setStats).catch((e) => setError(String(e)));
  };

  return (
    <>
      <div className="toolbar">
        <button className="ghost" onClick={onBack}>← Back</button>
        <h1 className="page" style={{ margin: 0 }}>Reports</h1>
        <button className="ghost" onClick={reload}>Refresh</button>
      </div>

      {error && <div className="banner error">{error}</div>}

      {stats === null && !error && <p className="muted" style={{ padding: "0 20px" }}>Loading…</p>}

      {stats && (
        <>
          <div className="form-card" style={{ marginBottom: 16 }}>
            <div className="section-title">Overview</div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12, marginTop: 8 }}>
              <StatBox label="Total patients" value={stats.total} />
              <StatBox label="Registered this month" value={stats.registered_this_month} />
              <StatBox label="Registered this year" value={stats.registered_this_year} />
            </div>
          </div>

          <div className="form-card" style={{ marginBottom: 16 }}>
            <div className="section-title">Demographics — Sex</div>
            <SexBar male={stats.male} female={stats.female} total={stats.total} />
          </div>

          <div className="form-card">
            <div className="section-title">Demographics — Top cities</div>
            {stats.cities.length === 0 ? (
              <p className="muted">No city data recorded yet.</p>
            ) : (
              <table style={{ width: "100%", borderCollapse: "collapse", marginTop: 8 }}>
                <thead>
                  <tr>
                    <th style={thStyle}>City</th>
                    <th style={{ ...thStyle, textAlign: "right" }}>Patients</th>
                    <th style={{ ...thStyle, textAlign: "right" }}>Share</th>
                  </tr>
                </thead>
                <tbody>
                  {stats.cities.map((c) => (
                    <tr key={c.city}>
                      <td style={tdStyle}>{c.city}</td>
                      <td style={{ ...tdStyle, textAlign: "right" }}>{c.count}</td>
                      <td style={{ ...tdStyle, textAlign: "right" }}>
                        {stats.total > 0 ? `${Math.round((c.count / stats.total) * 100)}%` : "—"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </>
      )}
    </>
  );
}

function StatBox({ label, value }: { label: string; value: number }) {
  return (
    <div style={{ background: "var(--surface-2)", borderRadius: 8, padding: "14px 16px", textAlign: "center" }}>
      <div style={{ fontSize: 28, fontWeight: 700, color: "var(--ink)" }}>{value}</div>
      <div style={{ fontSize: 12, color: "var(--ink-soft)", marginTop: 4 }}>{label}</div>
    </div>
  );
}

function SexBar({ male, female, total }: { male: number; female: number; total: number }) {
  const malePct = total > 0 ? Math.round((male / total) * 100) : 0;
  const femalePct = total > 0 ? Math.round((female / total) * 100) : 0;

  return (
    <div style={{ marginTop: 8 }}>
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6, fontSize: 13 }}>
        <span>Male — {male} ({malePct}%)</span>
        <span>Female — {female} ({femalePct}%)</span>
      </div>
      <div style={{ height: 16, background: "var(--surface-2)", borderRadius: 8, overflow: "hidden", display: "flex" }}>
        {malePct > 0 && (
          <div style={{ width: `${malePct}%`, background: "var(--accent)", transition: "width 0.3s" }} />
        )}
        {femalePct > 0 && (
          <div style={{ width: `${femalePct}%`, background: "var(--ink-soft)", transition: "width 0.3s" }} />
        )}
      </div>
      {total === 0 && <p className="muted" style={{ marginTop: 8 }}>No patients registered yet.</p>}
    </div>
  );
}

const thStyle: React.CSSProperties = {
  textAlign: "left",
  fontSize: 11,
  fontWeight: 600,
  color: "var(--ink-soft)",
  padding: "4px 8px",
  borderBottom: "1px solid var(--line)",
};

const tdStyle: React.CSSProperties = {
  fontSize: 13,
  padding: "6px 8px",
  borderBottom: "1px solid var(--line)",
};
