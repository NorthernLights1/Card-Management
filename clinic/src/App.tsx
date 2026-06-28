import { useEffect, useState } from "react";
import "./App.css";
import type { LicenseStatus, Patient, UserInfo } from "./lib/api";
import { getLicenseStatus, isInitialized, logout } from "./lib/api";
import { SetupScreen } from "./components/SetupScreen";
import { LoginScreen } from "./components/LoginScreen";
import { LicenseScreen } from "./components/LicenseScreen";
import { SearchScreen } from "./components/SearchScreen";
import { PatientForm } from "./components/PatientForm";
import { DeletedScreen } from "./components/DeletedScreen";
import { SettingsScreen } from "./components/SettingsScreen";
import { BackupsScreen } from "./components/BackupsScreen";
import { ReportsScreen } from "./components/ReportsScreen";

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
  const [licenseStatus, setLicenseStatus] = useState<LicenseStatus | null>(null);

  // Step 1: check license on mount
  useEffect(() => {
    getLicenseStatus()
      .then(setLicenseStatus)
      .catch(() => setLicenseStatus({ status: "Expired" }));
  }, []);

  // Step 2: proceed with normal init once license is known (and not expired)
  useEffect(() => {
    if (licenseStatus === null || licenseStatus.status === "Expired") return;
    isInitialized()
      .then((ready) => setPhase(ready ? "login" : "setup"))
      .catch(() => setPhase("login"));
  }, [licenseStatus]);

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

  if (licenseStatus?.status === "Expired") {
    return <LicenseScreen onActivated={() => getLicenseStatus().then(setLicenseStatus)} />;
  }
  if (phase === "loading") return <div className="auth-wrap">Loading…</div>;
  if (phase === "setup") return <SetupScreen onReady={enterApp} />;
  if (phase === "login") return <LoginScreen onAuthed={enterApp} />;

  const isAdmin = user?.role === "Admin";

  return (
    <>
      {licenseStatus?.status === "Trial" && (
        <div className="trial-banner">
          Trial — {licenseStatus.days_remaining} day{licenseStatus.days_remaining === 1 ? "" : "s"} remaining.
          Contact your provider for a license key.
        </div>
      )}
      <header className="app-header">
        <span className="brand">Clinic Card Management</span>
        <nav>
          {view.name !== "search" && view.name !== "register" && view.name !== "edit" && (
            <button className="ghost" onClick={() => setView({ name: "search" })}>Patients</button>
          )}
          {view.name !== "deleted" && (
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
          />
        )}
        {view.name === "deleted" && (
          <DeletedScreen onBack={() => setView({ name: "search" })} isAdmin={isAdmin} />
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
    </>
  );
}
