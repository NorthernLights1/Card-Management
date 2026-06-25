import { useState } from "react";
import type { UserInfo } from "../lib/api";
import { addUser, initializeAdmin } from "../lib/api";

type Props = { onReady: (user: UserInfo) => void };

// First-run: create the first Admin, then prompt for a second Admin (recommended,
// so a forgotten password can never lock everyone out of user management).
export function SetupScreen({ onReady }: Props) {
  const [step, setStep] = useState<1 | 2>(1);
  const [admin, setAdmin] = useState<UserInfo | null>(null);

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const createFirst = async () => {
    setError(null);
    if (password.length < 4) return setError("Password must be at least 4 characters.");
    if (password !== confirm) return setError("Passwords do not match.");
    setBusy(true);
    try {
      const user = await initializeAdmin(username.trim(), password);
      setAdmin(user);
      setUsername("");
      setPassword("");
      setConfirm("");
      setStep(2);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const addSecond = async () => {
    setError(null);
    if (password.length < 4) return setError("Password must be at least 4 characters.");
    if (password !== confirm) return setError("Passwords do not match.");
    setBusy(true);
    try {
      await addUser(username.trim(), password, "Admin");
      onReady(admin!);
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  return (
    <div className="auth-wrap">
      <div className="auth-card">
        {step === 1 ? (
          <>
            <h1>Set up the clinic</h1>
            <p className="sub">Create the first Admin account. Keep this password safe — there is no way to recover the data without it.</p>
            {error && <div className="banner error">{error}</div>}
            <div className="field">
              <label>Admin username</label>
              <input value={username} onChange={(e) => setUsername(e.target.value)} autoFocus />
            </div>
            <div className="field">
              <label>Password</label>
              <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} />
            </div>
            <div className="field">
              <label>Confirm password</label>
              <input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
            </div>
            <button className="primary" style={{ width: "100%" }} onClick={createFirst} disabled={busy || !username.trim()}>
              {busy ? "Creating…" : "Create Admin"}
            </button>
          </>
        ) : (
          <>
            <h1>Add a second Admin</h1>
            <p className="sub">Strongly recommended. If the only Admin forgets their password, no one can manage users. Skip only if you understand the risk.</p>
            {error && <div className="banner error">{error}</div>}
            <div className="field">
              <label>Second Admin username</label>
              <input value={username} onChange={(e) => setUsername(e.target.value)} autoFocus />
            </div>
            <div className="field">
              <label>Password</label>
              <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} />
            </div>
            <div className="field">
              <label>Confirm password</label>
              <input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <button className="ghost" onClick={() => onReady(admin!)} disabled={busy}>
                Skip for now
              </button>
              <button className="primary" style={{ flex: 1 }} onClick={addSecond} disabled={busy || !username.trim()}>
                {busy ? "Adding…" : "Add Admin & continue"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
