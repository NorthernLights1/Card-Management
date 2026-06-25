import { useEffect, useState } from "react";
import "./App.css";
import type { Patient, UserInfo } from "./lib/api";
import { changePassword, isInitialized, logout } from "./lib/api";
import { SetupScreen } from "./components/SetupScreen";
import { LoginScreen } from "./components/LoginScreen";
import { SearchScreen } from "./components/SearchScreen";
import { PatientForm } from "./components/PatientForm";
import { DeletedScreen } from "./components/DeletedScreen";
import { SettingsScreen } from "./components/SettingsScreen";
import { PrintCard } from "./components/PrintCard";

type Phase = "loading" | "setup" | "login" | "app";
type View =
  | { name: "search" }
  | { name: "register" }
  | { name: "edit"; patient: Patient }
  | { name: "deleted" }
  | { name: "settings" };

export default function App() {
  const [phase, setPhase] = useState<Phase>("loading");
  const [user, setUser] = useState<UserInfo | null>(null);
  const [view, setView] = useState<View>({ name: "search" });
  const [printing, setPrinting] = useState<Patient | null>(null);
  const [changingPassword, setChangingPassword] = useState(false);

  useEffect(() => {
    isInitialized()
      .then((ready) => setPhase(ready ? "login" : "setup"))
      .catch(() => setPhase("login"));
  }, []);

  useEffect(() => {
    if (!printing) return;
    const after = () => setPrinting(null);
    window.addEventListener("afterprint", after);
    window.print();
    return () => window.removeEventListener("afterprint", after);
  }, [printing]);

  const enterApp = (u: UserInfo) => {
    setUser(u);
    setView({ name: "search" });
    setPhase("app");
  };

  const signOut = async () => {
    await logout();
    setUser(null);
    setPhase("login");
  };

  if (phase === "loading") {
    return <div className="auth-wrap">Loading…</div>;
  }
  if (phase === "setup") {
    return <SetupScreen onReady={enterApp} />;
  }
  if (phase === "login") {
    return <LoginScreen onAuthed={enterApp} />;
  }

  return (
    <>
      <header className="app-header">
        <span className="brand">Clinic Card Management</span>
        <nav>
          {user?.role === "Admin" && view.name !== "deleted" && (
            <button className="ghost" onClick={() => setView({ name: "deleted" })}>
              Deleted patients
            </button>
          )}
          {user?.role === "Admin" && view.name !== "settings" && (
            <button className="ghost" onClick={() => setView({ name: "settings" })}>
              Backups
            </button>
          )}
          <span className="who">
            {user?.username} · {user?.role}
          </span>
          <button className="ghost" onClick={() => setChangingPassword(true)}>
            Change password
          </button>
          <button className="ghost" onClick={signOut}>
            Sign out
          </button>
        </nav>
      </header>
      <main className="app-main">
        {view.name === "search" && (
          <SearchScreen
            onRegister={() => setView({ name: "register" })}
            onEdit={(patient) => setView({ name: "edit", patient })}
            onPrint={(patient) => setPrinting(patient)}
          />
        )}
        {view.name === "deleted" && (
          <DeletedScreen onBack={() => setView({ name: "search" })} />
        )}
        {view.name === "settings" && (
          <SettingsScreen
            user={user!}
            onBack={() => setView({ name: "search" })}
            onRestored={() => {
              setUser(null);
              setPhase("login");
            }}
          />
        )}
        {view.name === "register" && (
          <PatientForm
            patient={null}
            onSaved={() => setView({ name: "search" })}
            onCancel={() => setView({ name: "search" })}
          />
        )}
        {view.name === "edit" && (
          <PatientForm
            patient={view.patient}
            onSaved={() => setView({ name: "search" })}
            onCancel={() => setView({ name: "search" })}
          />
        )}
      </main>
      {printing && <PrintCard patient={printing} />}
      {changingPassword && (
        <ChangePasswordModal onClose={() => setChangingPassword(false)} />
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
    if (newPassword !== confirm) {
      setError("New passwords do not match.");
      return;
    }
    if (newPassword.length < 4) {
      setError("New password must be at least 4 characters.");
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await changePassword(oldPassword, newPassword);
      setDone(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
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
              <input
                type="password"
                value={oldPassword}
                onChange={(e) => setOldPassword(e.target.value)}
                autoFocus
              />
            </div>
            <div className="field">
              <label>New password</label>
              <input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
              />
            </div>
            <div className="field">
              <label>Confirm new password</label>
              <input
                type="password"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
              />
            </div>
            <div className="form-actions">
              <button className="ghost" onClick={onClose} disabled={busy}>
                Cancel
              </button>
              <button
                className="primary"
                onClick={submit}
                disabled={busy || !oldPassword || !newPassword || !confirm}
              >
                {busy ? "Saving…" : "Change password"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
