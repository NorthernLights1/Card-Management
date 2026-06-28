import { useEffect, useState } from "react";
import { activateLicense, getDeviceId } from "../lib/api";

type Props = {
  onActivated: () => void;
};

export function LicenseScreen({ onActivated }: Props) {
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [key, setKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    getDeviceId()
      .then(setDeviceId)
      .catch(() => setDeviceId("Could not read device ID"));
  }, []);

  const copyDeviceId = () => {
    if (!deviceId) return;
    navigator.clipboard.writeText(deviceId).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  const doActivate = async () => {
    setError(null);
    setBusy(true);
    try {
      await activateLicense(key.trim());
      onActivated();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="auth-wrap">
      <div className="auth-card" style={{ maxWidth: 440 }}>
        <h1 style={{ fontSize: 20, marginBottom: 4 }}>License Required</h1>
        <p className="sub">Your 14-day trial has ended. Enter your license key to continue.</p>

        <div className="field">
          <label>Your Device ID</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              readOnly
              value={deviceId ?? "Reading…"}
              style={{ fontFamily: "monospace", fontSize: 13, color: "var(--ink-soft)" }}
            />
            <button onClick={copyDeviceId} disabled={!deviceId} style={{ flexShrink: 0 }}>
              {copied ? "Copied!" : "Copy"}
            </button>
          </div>
          <p className="muted" style={{ margin: "4px 0 0", fontSize: 12 }}>
            Share this with your provider to receive your license key.
          </p>
        </div>

        {error && <div className="banner error">{error}</div>}

        <div className="field">
          <label>License key</label>
          <input
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder="XXXXX-XXXXX-XXXXX-XXXXX"
            autoFocus
            onKeyDown={(e) => e.key === "Enter" && key.trim() && doActivate()}
          />
        </div>

        <button
          className="primary"
          onClick={doActivate}
          disabled={busy || !key.trim()}
          style={{ width: "100%" }}
        >
          {busy ? "Activating…" : "Activate"}
        </button>
      </div>
    </div>
  );
}
