#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/clinic/src-tauri"

command -v cargo >/dev/null 2>&1 || {
  echo "Missing dependency: cargo"
  exit 1
}

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

echo "Running Rust tests in $(pwd) with CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS"
cargo test --locked "$@"
