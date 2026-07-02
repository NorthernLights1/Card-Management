import { useEffect, useState } from "react";
import type { LicenseStatus, Role, UserInfo } from "../lib/api";
import {
  activateLicense,
  addUser,
  changePassword,
  getDeviceId,
  listUsers,
  removeUser,
  resetUserPassword,
} from "../lib/api";

type Props = {
  user: UserInfo;
  onBack: () => void;
  licenseStatus: LicenseStatus;
  onLicenseActivated: () => void;
};

export function SettingsScreen({ user, onBack, licenseStatus, onLicenseActivated }: Props) {
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [licenseKey, setLicenseKey] = useState("");
  const [licenseBusy, setLicenseBusy] = useState(false);
  const [licenseError, setLicenseError] = useState<string | null>(null);
  const [users, setUsers] = useState<UserInfo[] | null>(null);
  const [showAddUser, setShowAddUser] = useState(false);
  const [addForm, setAddForm] = useState({ username: "", password: "", role: "Staff" as Role });
  const [resetTarget, setResetTarget] = useState<string | null>(null);
  const [showChangePassword, setShowChangePassword] = useState(false);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [deviceIdCopied, setDeviceIdCopied] = useState(false);

  const loadUsers = () => listUsers().then(setUsers).catch((e) => setError(String(e)));
  useEffect(() => { loadUsers(); }, []);
  useEffect(() => { getDeviceId().then(setDeviceId).catch(() => setDeviceId("Unavailable")); }, []);

  const doAddUser = async () => {
    setError(null);
    const name = addForm.username.trim();
    try {
      await addUser(name, addForm.password, addForm.role);
      setAddForm({ username: "", password: "", role: "Staff" });
      setShowAddUser(false);
      setMessage(`User "${name}" added.`);
      loadUsers();
    } catch (e) { setError(String(e)); }
  };

  const doRemoveUser = async (username: string) => {
    if (!window.confirm(`Remove user "${username}"? They will no longer be able to log in.`)) return;
    setError(null);
    try {
      await removeUser(username);
      setMessage(`User "${username}" removed.`);
      loadUsers();
    } catch (e) { setError(String(e)); }
  };

  const doActivateLicense = async () => {
    setLicenseBusy(true);
    setLicenseError(null);
    try {
      await activateLicense(licenseKey.trim());
      onLicenseActivated();
    } catch (e) {
      setLicenseError(String(e));
    } finally {
      setLicenseBusy(false);
    }
  };

  return (
    <>
      <div className="toolbar">
        <button className="ghost" onClick={onBack}>← Back</button>
        <h1 className="page" style={{ margin: 0 }}>Settings</h1>
      </div>

      {error && <div className="banner error">{error}</div>}
      {message && <div className="banner warn">{message}</div>}

      <div className="form-card" style={{ marginBottom: 16 }}>
        <div className="section-title">Password</div>
        <p className="muted">Change your own login password.</p>
        <button onClick={() => setShowChangePassword(true)}>Change password…</button>
      </div>

      {user.role === "Admin" && (
        <div className="form-card">
          <div className="section-title">Users</div>
          {users === null ? (
            <p className="muted">Loading…</p>
          ) : (
            users.map((u) => (
              <div className="result-row" key={u.username} style={{ gridTemplateColumns: "1fr auto", marginBottom: 4 }}>
                <div>
                  <span className="result-name">{u.username}</span>
                  {u.username === user.username && (
                    <span className="muted" style={{ marginLeft: 6 }}>(you)</span>
                  )}
                  <div className="result-meta">{u.role === "Admin" ? "Admin" : "Reception"}</div>
                </div>
                <div style={{ display: "flex", gap: 6 }}>
                  <button className="ghost" onClick={() => setResetTarget(u.username)}>Reset password</button>
                  {u.username !== user.username && (
                    <button className="danger" onClick={() => doRemoveUser(u.username)}>Remove</button>
                  )}
                </div>
              </div>
            ))
          )}
          {!showAddUser ? (
            <button style={{ marginTop: 10 }} onClick={() => setShowAddUser(true)}>+ Add user</button>
          ) : (
            <div style={{ marginTop: 12, padding: 12, background: "var(--surface-2)", borderRadius: 8 }}>
              <div className="field">
                <label>Username</label>
                <input value={addForm.username} onChange={(e) => setAddForm({ ...addForm, username: e.target.value })} autoFocus />
              </div>
              <div className="field">
                <label>Password</label>
                <input type="password" value={addForm.password} onChange={(e) => setAddForm({ ...addForm, password: e.target.value })} />
              </div>
              <div className="field">
                <label>Role</label>
                <select value={addForm.role} onChange={(e) => setAddForm({ ...addForm, role: e.target.value as Role })}>
                  <option value="Staff">Reception</option>
                  <option value="Admin">Admin</option>
                </select>
              </div>
              <div className="form-actions">
                <button className="ghost" onClick={() => { setShowAddUser(false); setAddForm({ username: "", password: "", role: "Staff" }); }}>
                  Cancel
                </button>
                <button className="primary" onClick={doAddUser} disabled={!addForm.username.trim() || !addForm.password}>
                  Add user
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {licenseStatus.status === "Trial" && (
        <div className="form-card" style={{ marginTop: 16 }}>
          <div className="section-title">Activate License</div>
          <p className="muted">Enter your license key to activate before your trial expires.</p>
          {licenseError && <div className="banner error">{licenseError}</div>}
          <div style={{ display: "flex", gap: 8 }}>
            <input
              value={licenseKey}
              onChange={(e) => setLicenseKey(e.target.value)}
              placeholder="XXXXX-XXXXX-XXXXX-XXXXX"
              style={{ fontFamily: "monospace", fontSize: 13 }}
              onKeyDown={(e) => { if (e.key === "Enter" && licenseKey.trim()) doActivateLicense(); }}
            />
            <button
              className="primary"
              onClick={doActivateLicense}
              disabled={licenseBusy || !licenseKey.trim()}
              style={{ flexShrink: 0 }}
            >
              {licenseBusy ? "Activating…" : "Activate"}
            </button>
          </div>
        </div>
      )}

      <div className="form-card" style={{ marginTop: 16 }}>
        <div className="section-title">Device ID</div>
        <p className="muted">Your unique device identifier. Share this with your provider to get a license key.</p>
        <div style={{ display: "flex", gap: 8 }}>
          <input
            readOnly
            value={deviceId ?? "Reading…"}
            style={{ fontFamily: "monospace", fontSize: 13, color: "var(--ink-soft)" }}
          />
          <button
            onClick={() => {
              if (!deviceId) return;
              navigator.clipboard.writeText(deviceId).then(() => {
                setDeviceIdCopied(true);
                setTimeout(() => setDeviceIdCopied(false), 2000);
              });
            }}
            disabled={!deviceId}
            style={{ flexShrink: 0 }}
          >
            {deviceIdCopied ? "Copied!" : "Copy"}
          </button>
        </div>
      </div>

      {showChangePassword && (
        <ChangePasswordModal onClose={() => setShowChangePassword(false)} />
      )}

      {resetTarget && (
        <ResetPasswordModal
          username={resetTarget}
          onClose={() => setResetTarget(null)}
          onDone={(msg) => { setResetTarget(null); setMessage(msg); }}
        />
      )}
    </>
  );
}

function ChangePasswordModal({ onClose }: { onClose: () => void }) {
  const [oldPassword, setOldPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);

  const submit = async () => {
    if (newPassword !== confirm) { setError("New passwords do not match."); return; }
    if (newPassword.length < 4) { setError("New password must be at least 4 characters."); return; }
    setError(null);
    setBusy(true);
    try { await changePassword(oldPassword, newPassword); setDone(true); }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true">
      <div className="modal">
        <h3>Change password</h3>
        {error && <div className="banner error">{error}</div>}
        {done ? (
          <>
            <div className="banner warn">Password changed successfully.</div>
            <div className="form-actions">
              <button className="primary" onClick={onClose}>Close</button>
            </div>
          </>
        ) : (
          <>
            <div className="field">
              <label>Current password</label>
              <input type="password" value={oldPassword} onChange={(e) => setOldPassword(e.target.value)} autoFocus />
            </div>
            <div className="field">
              <label>New password</label>
              <input type="password" value={newPassword} onChange={(e) => setNewPassword(e.target.value)} />
            </div>
            <div className="field">
              <label>Confirm new password</label>
              <input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
            </div>
            <div className="form-actions">
              <button className="ghost" onClick={onClose} disabled={busy}>Cancel</button>
              <button className="primary" onClick={submit} disabled={busy || !oldPassword || !newPassword || !confirm}>
                {busy ? "Saving…" : "Change password"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function ResetPasswordModal({ username, onClose, onDone }: { username: string; onClose: () => void; onDone: (msg: string) => void }) {
  const [newPassword, setNewPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    if (newPassword !== confirm) { setError("Passwords do not match."); return; }
    if (newPassword.length < 4) { setError("Password must be at least 4 characters."); return; }
    setError(null);
    setBusy(true);
    try { await resetUserPassword(username, newPassword); onDone(`Password for "${username}" has been reset.`); }
    catch (e) { setError(String(e)); setBusy(false); }
  };

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true">
      <div className="modal">
        <h3>Reset password — {username}</h3>
        {error && <div className="banner error">{error}</div>}
        <div className="field">
          <label>New password</label>
          <input type="password" value={newPassword} onChange={(e) => setNewPassword(e.target.value)} autoFocus />
        </div>
        <div className="field">
          <label>Confirm new password</label>
          <input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
        </div>
        <div className="form-actions">
          <button className="ghost" onClick={onClose} disabled={busy}>Cancel</button>
          <button className="primary" onClick={submit} disabled={busy || !newPassword || !confirm}>
            {busy ? "Saving…" : "Reset password"}
          </button>
        </div>
      </div>
    </div>
  );
}
