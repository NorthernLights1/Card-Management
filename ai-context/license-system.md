# License System

Implemented in `clinic/src-tauri/src/license.rs` (branch `feature/license-trial`,
merged into `main` via PR after commit `7d6fee4`).

## How it works

### Device ID

1. Run `wmic baseboard get serialnumber /value` — parse the `SerialNumber=` line.
2. If the result is blank or a generic OEM placeholder (list in `GENERIC_SERIALS`),
   fall back to `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid`.
3. If both fail: return `"UNKNOWN"` (edge case; key generation still works but is
   less unique).

### Key generation

```
SHA256(device_id) → first 20 hex chars → uppercase → split into 4 groups of 5
→ "XXXXX-XXXXX-XXXXX-XXXXX"
```

No HMAC secret — the algorithm is public. Acceptable because piracy risk is very low
(single clinic, single PC). To generate a key for a client:

```python
python -c "import hashlib; s='<DEVICE_ID>'; h=hashlib.sha256(s.encode()).hexdigest()[:20].upper(); print('-'.join(h[i:i+5] for i in range(0,20,5)))"
```

### Activation

`activate(key)` in `license.rs`:
1. Get current device ID.
2. Compute expected key via `make_key(device_id)`.
3. Compare (normalized — strip dashes, uppercase) — error if mismatch.
4. Write the key to `HKCU\Software\ClinicApp\license` in the Windows registry.

### 14-day trial

Registry key: `HKCU\Software\ClinicApp\trial`
Value format: `"YYYY-MM-DD|SHA256(device_id|YYYY-MM-DD)"`

- **First run:** no `trial` value → write today's date + integrity hash → return
  `Trial { days_remaining: 14 }`.
- **Subsequent runs:** read the stored date, verify integrity hash (detects manual
  date editing), compute `elapsed = today - stored_date`.
  - `elapsed < 14` → `Trial { days_remaining: 14 - elapsed }`
  - `elapsed >= 14` → `Expired`
- The registry key lives in `HKCU`, which Tauri's uninstaller does **not** clear,
  so the trial clock survives uninstall/reinstall.

### App startup gate (App.tsx)

```
getLicenseStatus()  →  licenseStatus state set
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
          Licensed        Trial         Expired
              │             │             │
         isInitialized()  isInitialized() │
              │             │         <LicenseScreen>
         normal boot    normal boot   (blocks login)
         (no banner)    (amber banner
                         in header)
```

`LicenseScreen` shows the device ID (read-only + copy button) and a key input field.
On successful activation it calls `getLicenseStatus()` again to refresh, which then
allows `isInitialized()` to run and the normal boot flow to proceed.

### In-trial activation (Settings screen)

During the Trial phase, `SettingsScreen` receives `licenseStatus` and
`onLicenseActivated` props from `App.tsx`. It renders an **Activate License** card
with a key input field. On success it calls `onLicenseActivated()`, which calls
`getLicenseStatus().then(setLicenseStatus)` in `App.tsx` — the trial banner
disappears immediately without requiring a restart.

```
User (during trial) → Settings → Activate License card
  → types key → clicks Activate
  → activateLicense(key) [Tauri command]
  → onLicenseActivated() → getLicenseStatus() → licenseStatus = Licensed
  → trial banner gone, Activate card gone
```

The `LicenseScreen` (shown only on expiry) still exists as a hard gate for users
who didn't activate proactively.

## Registry layout

```
HKCU\Software\ClinicApp\
    license  REG_SZ  "XXXXX-XXXXX-XXXXX-XXXXX"   (absent if not activated)
    trial    REG_SZ  "YYYY-MM-DD|YYYY-MM-DD|<sha256hex>"  (3-part format with last-seen)
```

## Where device ID appears in the UI

- **Settings screen** → "Device ID" section (visible to all roles, with copy button).
- **Settings screen** → "Activate License" section (visible only during Trial).
- **LicenseScreen** → shown when expired, same copy button.

## Workaround (manual registry write)

If the UI is inaccessible, activate directly via PowerShell:
```powershell
Set-ItemProperty -Path "HKCU:\Software\ClinicApp" -Name "license" -Value "XXXXX-XXXXX-XXXXX-XXXXX"
```
