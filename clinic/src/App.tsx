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
    </>
  );
}
