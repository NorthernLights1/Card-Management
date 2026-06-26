import { useEffect, useState } from "react";
import "./App.css";
import type { Patient, UserInfo } from "./lib/api";
import { isInitialized, logout } from "./lib/api";
import { SetupScreen } from "./components/SetupScreen";
import { LoginScreen } from "./components/LoginScreen";
import { SearchScreen } from "./components/SearchScreen";
import { PatientForm } from "./components/PatientForm";
import { DeletedScreen } from "./components/DeletedScreen";
import { SettingsScreen } from "./components/SettingsScreen";
import { BackupsScreen } from "./components/BackupsScreen";
import { ReportsScreen } from "./components/ReportsScreen";
import { PrintCard } from "./components/PrintCard";

type Phase = "loading" | "setup" | "login" | "app";
type View =
  | { name: "search" }
  | { name: "register" }
  | { name: "edit"; patient: Patient }
  | { name: "deleted" }
  | { name: "settings" }
  | { name: "backups" }
  | { name: "reports" };

export default function App() {
  const [phase, setPhase] = useState<Phase>("loading");
  const [user, setUser] = useState<UserInfo | null>(null);
  const [view, setView] = useState<View>({ name: "search" });
  const [printing, setPrinting] = useState<Patient | null>(null);

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

  if (phase === "loading") return <div className="auth-wrap">Loading…</div>;
  if (phase === "setup") return <SetupScreen onReady={enterApp} />;
  if (phase === "login") return <LoginScreen onAuthed={enterApp} />;

  const isAdmin = user?.role === "Admin";

  return (
    <>
      <header className="app-header">
        <span className="brand">Clinic Card Management</span>
        <nav>
          {isAdmin && view.name !== "deleted" && (
            <button className="ghost" onClick={() => setView({ name: "deleted" })}>Deleted patients</button>
          )}
          {isAdmin && view.name !== "reports" && (
            <button className="ghost" onClick={() => setView({ name: "reports" })}>Reports</button>
          )}
          {isAdmin && view.name !== "backups" && (
            <button className="ghost" onClick={() => setView({ name: "backups" })}>Backups</button>
          )}
          {view.name !== "settings" && (
            <button className="ghost" onClick={() => setView({ name: "settings" })}>Settings</button>
          )}
          <span className="who">
            {user?.username} · {user?.role === "Admin" ? "Admin" : "Reception"}
          </span>
          <button className="ghost" onClick={signOut}>Sign out</button>
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
          <SettingsScreen user={user!} onBack={() => setView({ name: "search" })} />
        )}
        {view.name === "backups" && (
          <BackupsScreen
            onBack={() => setView({ name: "search" })}
            onRestored={() => { setUser(null); setPhase("login"); }}
          />
        )}
        {view.name === "reports" && (
          <ReportsScreen onBack={() => setView({ name: "search" })} />
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
    </>
  );
}
