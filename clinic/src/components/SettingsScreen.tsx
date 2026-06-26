import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { DriveInfo, Role, UsbStatus, UserInfo } from "../lib/api";
import {
  addUser,
  exportBackup,
  exportPatientCsv,
  listRemovableDrives,
  listUsers,
  readAuditLog,
  removeUser,
  resetUserPassword,
  restoreApply,
  restorePreview,
  setUsbBackup,
  usbStatus,
} from "../lib/api";
import { ImportScreen } from "./ImportScreen";

type Props = {
  user: UserInfo;
  onBack: () => void;
  onRestored: () => void;
};

export function SettingsScreen({ user, onBack, onRestored }: Props) {
  const [usb, setUsb] = useState<UsbStatus | null>(null);
  const [drives, setDrives] = useState<DriveInfo[] | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [restore, setRestore] = useState<RestoreState | null>(null);
  const [auditLog, setAuditLog] = useState<string | null>(null);
  const [showImport, setShowImport] = useState(false);
  const [users, setUsers] = useState<UserInfo[] | null>(null);
  const [showAddUser, setShowAddUser] = useState(false);
  const [addForm, setAddForm] = useState({ username: "", password: "", role: "Staff" as Role });
  const [resetTarget, setResetTarget] = useState<string | null>(null);

  const loadUsb = () => usbStatus().then(setUsb).catch((e) => setError(String(e)));
  const loadUsers = () => listUsers().then(setUsers).catch((e) => setError(String(e)));
  useEffect(() => {
    loadUsb();
    loadUsers();
  }, []);

  const doAddUser = async () => {
    setError(null);
    const name = addForm.username.trim();
    try {
      await addUser(name, addForm.password, addForm.role);
      setAddForm({ username: "", password: "", role: "Staff" });
      setShowAddUser(false);
      setMessage(`User "${name}" added.`);
      loadUsers();
    } catch (e) {
      setError(String(e));
    }
  };

  const doRemoveUser = async (username: string) => {
    if (!window.confirm(`Remove user "${username}"? They will no longer be able to log in.`)) return;
    setError(null);
    try {
      await removeUser(username);
      setMessage(`User "${username}" removed.`);
      loadUsers();
    } catch (e) {
      setError(String(e));
    }
  };

  const chooseDrives = async () => {
    setError(null);
    try {
      setDrives(await listRemovableDrives());
    } catch (e) {
      setError(String(e));
    }
  };

  const pickDrive = async (drive: string) => {
    try {
      await setUsbBackup(drive);
      setDrives(null);
      setMessage(`USB backup set to ${drive}. A backup was written.`);
      loadUsb();
    } catch (e) {
      setError(String(e));
    }
  };

  const doExport = async () => {
    setError(null);
    setMessage(null);
    const dir = await open({ directory: true, title: "Choose a folder for the backup" });
    if (typeof dir !== "string") return;
    try {
      await exportBackup(dir);
      setMessage(`Backup exported to ${dir}`);
    } catch (e) {
      setError(String(e));
    }
  };

  const exportCsv = async () => {
    setError(null);
    setMessage(null);
    const dest = await save({
      defaultPath: "patients.csv",
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (typeof dest !== "string") return;
    try {
      const n = await exportPatientCsv(dest);
      setMessage(`Exported ${n} patient${n === 1 ? "" : "s"} to ${dest}`);
    } catch (e) {
      setError(String(e));
    }
  };

  const startRestore = async () => {
    setError(null);
    setMessage(null);
    const folder = await open({ directory: true, title: "Choose a backup folder to restore" });
    if (typeof folder !== "string") return;
    setRestore({ folder, username: "", password: "", count: null, busy: false });
  };

  return (
    <>
      <div className="toolbar">
        <button className="ghost" onClick={onBack}>
          ← Back to search
        </button>
        <h1 className="page" style={{ margin: 0 }}>
          Backups
        </h1>
      </div>

      {error && <div className="banner error">{error}</div>}
      {message && <div className="banner warn">{message}</div>}

      <div className="form-card" style={{ marginBottom: 16 }}>
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
                <div className="result-meta">{u.role}</div>
              </div>
              <div style={{ display: "flex", gap: 6 }}>
                <button className="ghost" onClick={() => setResetTarget(u.username)}>
                  Reset password
                </button>
                {u.username !== user.username && (
                  <button className="danger" onClick={() => doRemoveUser(u.username)}>
                    Remove
                  </button>
                )}
              </div>
            </div>
          ))
        )}
        {!showAddUser ? (
          <button style={{ marginTop: 10 }} onClick={() => setShowAddUser(true)}>
            + Add user
          </button>
        ) : (
          <div style={{ marginTop: 12, padding: 12, background: "var(--surface-2)", borderRadius: 8 }}>
            <div className="field">
              <label>Username</label>
              <input
                value={addForm.username}
                onChange={(e) => setAddForm({ ...addForm, username: e.target.value })}
                autoFocus
              />
            </div>
            <div className="field">
              <label>Password</label>
              <input
                type="password"
                value={addForm.password}
                onChange={(e) => setAddForm({ ...addForm, password: e.target.value })}
              />
            </div>
            <div className="field">
              <label>Role</label>
              <select
                value={addForm.role}
                onChange={(e) => setAddForm({ ...addForm, role: e.target.value as Role })}
              >
                <option value="Staff">Staff</option>
                <option value="Admin">Admin</option>
              </select>
            </div>
            <div className="form-actions">
              <button
                className="ghost"
                onClick={() => {
                  setShowAddUser(false);
                  setAddForm({ username: "", password: "", role: "Staff" });
                }}
              >
                Cancel
              </button>
              <button
                className="primary"
                onClick={doAddUser}
                disabled={!addForm.username.trim() || !addForm.password}
              >
                Add user
              </button>
            </div>
          </div>
        )}
      </div>

      <div className="form-card" style={{ marginBottom: 16 }}>
        <div className="section-title">USB backup</div>
        {usb === null ? (
          <p className="muted">Checking…</p>
        ) : usb.configured ? (
          <p>
            {usb.connected ? (
              <>✅ Connected — backing up to <strong>{usb.drive}</strong> on every save.</>
            ) : (
              <>⚠️ USB backup paused — the backup stick isn't connected. Data is still saved and backed up on this PC. It resumes automatically when you reconnect it.</>
            )}
          </p>
        ) : (
          <p className="muted">No USB backup set up yet. Choose a stick to mirror backups onto.</p>
        )}
        <div style={{ display: "flex", gap: 10, marginTop: 8 }}>
          <button onClick={chooseDrives}>{usb?.configured ? "Change USB drive" : "Set up USB drive"}</button>
          <button className="ghost" onClick={loadUsb}>Refresh</button>
        </div>

        {drives && (
          <div style={{ marginTop: 14 }}>
            {drives.length === 0 ? (
              <p className="muted">No drives found. Plug in a USB stick and try again.</p>
            ) : (
              drives.map((d) => (
                <div className="result-row" key={d.drive} style={{ gridTemplateColumns: "1fr auto" }}>
                  <div>
                    <div className="result-name">{d.drive}</div>
                    <div className="result-meta">{d.label || "Removable drive"}</div>
                  </div>
                  <button className="primary" onClick={() => pickDrive(d.drive)}>
                    Use this drive
                  </button>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      <div className="form-card" style={{ marginBottom: 16 }}>
        <div className="section-title">Export</div>
        <p className="muted">Save a full encrypted backup (database + accounts) to any folder or USB.</p>
        <button onClick={doExport}>Export backup…</button>
      </div>

      <div className="form-card" style={{ marginBottom: 16 }}>
        <div className="section-title">Patient data</div>
        <p className="muted">Import existing records from a spreadsheet, or export the current list.</p>
        <div style={{ display: "flex", gap: 10 }}>
          <button onClick={() => setShowImport(true)}>Import from Excel/CSV…</button>
          <button onClick={exportCsv}>Export patient list (CSV)…</button>
        </div>
      </div>

      <div className="form-card">
        <div className="section-title">Restore</div>
        <p className="muted">
          Replace all current data with a backup. The current data is snapshotted first. You'll need
          the password the backup was made with, and you'll be signed out afterward.
        </p>
        <button className="danger" onClick={startRestore}>
          Restore from backup…
        </button>
      </div>

      <div className="form-card" style={{ marginTop: 16 }}>
        <div className="section-title">Activity log</div>
        <p className="muted">Who did what, and when. Identifiers only — no patient details.</p>
        <button
          onClick={() => readAuditLog().then(setAuditLog).catch((e) => setError(String(e)))}
        >
          {auditLog === null ? "View activity log" : "Refresh"}
        </button>
        {auditLog !== null && (
          <pre
            style={{
              marginTop: 12,
              maxHeight: 320,
              overflow: "auto",
              background: "var(--surface-2)",
              border: "1px solid var(--line)",
              borderRadius: 8,
              padding: 12,
              fontSize: 12,
              whiteSpace: "pre-wrap",
            }}
          >
            {auditLog.trim() === "" ? "No activity recorded yet." : auditLog}
          </pre>
        )}
      </div>

      {restore && (
        <RestoreModal
          state={restore}
          setState={setRestore}
          onDone={onRestored}
        />
      )}

      {showImport && <ImportScreen onClose={() => setShowImport(false)} />}

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

type RestoreState = {
  folder: string;
  username: string;
  password: string;
  count: number | null;
  busy: boolean;
};

function RestoreModal({
  state,
  setState,
  onDone,
}: {
  state: RestoreState;
  setState: (s: RestoreState | null) => void;
  onDone: () => void;
}) {
  const [error, setError] = useState<string | null>(null);

  const preview = async () => {
    setError(null);
    setState({ ...state, busy: true });
    try {
      const count = await restorePreview(state.folder, state.username.trim(), state.password);
      setState({ ...state, count, busy: false });
    } catch (e) {
      setError(String(e));
      setState({ ...state, busy: false });
    }
  };

  const apply = async () => {
    setError(null);
    setState({ ...state, busy: true });
    try {
      await restoreApply(state.folder, state.username.trim(), state.password);
      onDone();
    } catch (e) {
      setError(String(e));
      setState({ ...state, busy: false });
    }
  };

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true">
      <div className="modal">
        <h3>Restore backup</h3>
        <p className="muted" style={{ wordBreak: "break-all" }}>{state.folder}</p>
        {error && <div className="banner error">{error}</div>}

        {state.count === null ? (
          <>
            <div className="field">
              <label>Username (from the backup)</label>
              <input
                value={state.username}
                onChange={(e) => setState({ ...state, username: e.target.value })}
                autoFocus
              />
            </div>
            <div className="field">
              <label>Password</label>
              <input
                type="password"
                value={state.password}
                onChange={(e) => setState({ ...state, password: e.target.value })}
              />
            </div>
            <div className="form-actions">
              <button className="ghost" onClick={() => setState(null)} disabled={state.busy}>
                Cancel
              </button>
              <button className="primary" onClick={preview} disabled={state.busy || !state.username.trim()}>
                {state.busy ? "Checking…" : "Check backup"}
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="banner warn">
              This backup has <strong>{state.count}</strong> patient{state.count === 1 ? "" : "s"}.
              Restoring will replace all current data. This cannot be undone (except from the
              automatic pre-restore snapshot).
            </div>
            <div className="form-actions">
              <button className="ghost" onClick={() => setState(null)} disabled={state.busy}>
                Cancel
              </button>
              <button className="danger" onClick={apply} disabled={state.busy}>
                {state.busy ? "Restoring…" : "Replace all data"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function ResetPasswordModal({
  username,
  onClose,
  onDone,
}: {
  username: string;
  onClose: () => void;
  onDone: (msg: string) => void;
}) {
  const [newPassword, setNewPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    if (newPassword !== confirm) { setError("Passwords do not match."); return; }
    if (newPassword.length < 4) { setError("Password must be at least 4 characters."); return; }
    setError(null);
    setBusy(true);
    try {
      await resetUserPassword(username, newPassword);
      onDone(`Password for "${username}" has been reset.`);
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
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
          <button
            className="primary"
            onClick={submit}
            disabled={busy || !newPassword || !confirm}
          >
            {busy ? "Saving…" : "Reset password"}
          </button>
        </div>
      </div>
    </div>
  );
}
