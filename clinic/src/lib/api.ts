// Typed wrappers around the Rust Tauri commands. The UI only ever calls these —
// it never touches SQL or the encryption key.
import { invoke } from "@tauri-apps/api/core";
import type { EcDate } from "./ethiopian";

export type Role = "Admin" | "Staff";
export type Sex = "Male" | "Female";

export type UserInfo = { username: string; role: Role };

export type PatientInput = {
  first_name: string;
  father_name: string;
  grandfather_name: string;
  sex: Sex;
  phone: string;
  dob: EcDate | null;
  age: number | null;
  address: string | null;
  city: string | null;
};

export type Patient = {
  id: number;
  card_number: string;
  first_name: string;
  father_name: string;
  grandfather_name: string;
  sex: Sex;
  phone: string;
  dob: EcDate | null;
  age_recorded: number | null;
  age_recorded_on: string | null;
  address: string | null;
  city: string | null;
  registered_at: string;
};

// --- auth / session ---
export const isInitialized = () => invoke<boolean>("is_initialized");

export const initializeAdmin = (username: string, password: string) =>
  invoke<UserInfo>("initialize_admin", { username, password });

export const login = (username: string, password: string) =>
  invoke<UserInfo>("login", { username, password });

export const logout = () => invoke<void>("logout");

export const currentUser = () => invoke<UserInfo | null>("current_user");

// --- user management (Admin only) ---
export const addUser = (username: string, password: string, role: Role) =>
  invoke<void>("add_user", { username, password, role });

export const removeUser = (username: string) =>
  invoke<void>("remove_user", { username });

export const listUsers = () => invoke<UserInfo[]>("list_users");

export const changePassword = (oldPassword: string, newPassword: string) =>
  invoke<void>("change_password", { oldPassword, newPassword });

export const resetUserPassword = (username: string, newPassword: string) =>
  invoke<void>("reset_user_password", { username, newPassword });

// --- patients ---
export const registerPatient = (input: PatientInput) =>
  invoke<Patient>("register_patient", { input });

export const updatePatient = (id: number, input: PatientInput) =>
  invoke<void>("update_patient", { id, input });

export const deletePatient = (id: number) =>
  invoke<void>("delete_patient", { id });

export const listPatients = () => invoke<Patient[]>("list_patients");

export const searchPatients = (query: string) =>
  invoke<Patient[]>("search_patients", { query });

export const getPatient = (id: number) =>
  invoke<Patient | null>("get_patient", { id });

export const checkDuplicates = (input: PatientInput) =>
  invoke<Patient[]>("check_duplicates", { input });

// --- backups (Admin only, except status) ---
export type UsbStatus = { configured: boolean; connected: boolean; drive: string | null };
export type DriveInfo = { drive: string; label: string };

export const usbStatus = () => invoke<UsbStatus>("usb_status");
export const listRemovableDrives = () => invoke<DriveInfo[]>("list_removable_drives");
export const setUsbBackup = (drive: string) => invoke<void>("set_usb_backup", { drive });
export const exportBackup = (destDir: string) => invoke<void>("export_backup", { destDir });
export const restorePreview = (folder: string, username: string, password: string) =>
  invoke<number>("restore_preview", { folder, username, password });
export const restoreApply = (folder: string, username: string, password: string) =>
  invoke<void>("restore_apply", { folder, username, password });

export const readAuditLog = () => invoke<string>("read_audit_log");

// --- import / export (Admin only) ---
export type ImportPreview = { headers: string[]; sample: string[][]; total_rows: number };
export type ImportMapping = {
  card_number: number;
  first_name: number;
  father_name: number;
  grandfather_name: number;
  sex: number;
  phone: number;
  age: number | null;
  city: number | null;
  address: number | null;
};
export type ImportReport = { imported: number; skipped: { row: number; reason: string }[] };

export const importPreview = (path: string) => invoke<ImportPreview>("import_preview", { path });
export const importApply = (path: string, mapping: ImportMapping) =>
  invoke<ImportReport>("import_apply", { path, mapping });
export const exportPatientCsv = (destPath: string) =>
  invoke<number>("export_patient_csv", { destPath });

// --- deleted patients (Admin only) ---
export const listDeletedPatients = () =>
  invoke<Patient[]>("list_deleted_patients");

export const restorePatient = (id: number) =>
  invoke<void>("restore_patient", { id });

export const purgePatient = (id: number) =>
  invoke<void>("purge_patient", { id });

// --- reports (Admin only) ---
export type CityCount = { city: string; count: number };
export type PatientStats = {
  total: number;
  registered_this_month: number;
  registered_this_year: number;
  male: number;
  female: number;
  cities: CityCount[];
};

export const getPatientStats = () => invoke<PatientStats>("get_patient_stats");

// --- license / trial ---
export type LicenseStatus =
  | { status: "Licensed" }
  | { status: "Trial"; days_remaining: number }
  | { status: "Expired" };

export const getDeviceId = () => invoke<string>("get_device_id");
export const getLicenseStatus = () => invoke<LicenseStatus>("get_license_status");
export const activateLicense = (key: string) => invoke<void>("activate_license", { key });
