# Developer Workflows

## Prerequisites (Windows)

- **Rust** (stable) — `rustup toolchain install stable`
- **Node 18+** — `nvm install 18` or direct installer
- **Perl** — required by SQLCipher's vendored OpenSSL (Strawberry Perl works)
- **NASM** — assembler for OpenSSL, must be on PATH
- **Visual Studio C++ build tools** (MSVC)
- **Tauri CLI** — installed as a devDependency, run via `npm run tauri`

## Common commands

```powershell
cd clinic

# Install JS deps
npm install

# Dev mode (Rust + React both hot-reload)
npm run tauri dev

# Production build → installer at src-tauri/target/release/bundle/msi/ or /nsis/
npm run tauri build

# TypeScript check only (no Rust)
npm run build          # Vite build (also type-checks)

# Rust check only (fast, no link)
cd src-tauri && cargo check

# Rust tests
cd src-tauri && cargo test

# Rust type/lint
cd src-tauri && cargo clippy
```

## Branches

| Branch | Purpose |
|--------|---------|
| `main` | Stable, releasable |
| `testing-branch` | Active development |
| `feature/license-trial` | License system (merged via PR; kept for reference) |

## Adding a new Tauri command

1. Write the logic in the appropriate `src-tauri/src/*.rs` module.
2. Add a `#[tauri::command] pub fn my_command(...)` in `commands.rs`.
3. Register it in `lib.rs` inside `.invoke_handler(tauri::generate_handler![..., commands::my_command])`.
4. Add a typed wrapper in `src/lib/api.ts`.

## Adding a new frontend screen

1. Create `src/components/MyScreen.tsx`.
2. Add a variant to the `View` union type in `App.tsx`.
3. Add a nav button and render branch in `App.tsx`.
4. No routing library — the app uses a manual view state pattern.

## Generating a license key for a client

```python
python -c "import hashlib; s='<DEVICE_ID_FROM_SETTINGS>'; h=hashlib.sha256(s.encode()).hexdigest()[:20].upper(); print('-'.join(h[i:i+5] for i in range(0,20,5)))"
```

The client reads their Device ID from Settings → Device ID section (or from the
LicenseScreen if their trial has expired) and sends it to you.

## Offline installer

The repo includes an offline installer setup (commit `be0f94d`). See that commit for
details on bundling the installer without requiring an internet connection at the
install site.

## Keybindings / entry points

- `clinic/src/main.tsx` — React entry
- `clinic/src-tauri/src/main.rs` — Rust binary entry (just calls `lib.rs::run()`)
- `clinic/src-tauri/src/lib.rs` — Tauri app setup, plugin registration, AppState
