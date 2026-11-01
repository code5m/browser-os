#!/usr/bin/env bash
# Standalone verification; invoke on an authorized developer machine only.
# Does not build, install or run BrowserOS.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$ROOT/agent-foundation/Cargo.toml"
cd "$ROOT"

echo "[CHECK] independent Rust fmt"
cargo fmt --manifest-path "$MANIFEST" --all -- --check
echo "[CHECK] independent Rust compile"
cargo check --manifest-path "$MANIFEST" --all-targets
echo "[CHECK] independent Rust tests"
cargo test --manifest-path "$MANIFEST" --all-targets
echo "[CHECK] independent Rust lints"
cargo clippy --manifest-path "$MANIFEST" --all-targets -- -D warnings

if git rev-parse --verify master >/dev/null 2>&1; then
  BASE="$(git merge-base HEAD master)"
  echo "[CHECK] file isolation from master: $BASE"
  while IFS= read -r changed; do
    case "$changed" in
      agent-foundation/*) ;;
      *) echo "[FAIL] Non-foundation file changed: $changed" >&2; exit 1 ;;
    esac
  done < <(git diff --name-only "$BASE" HEAD)
else
  echo "[WARN] Cannot check branch isolation: local master not found"
fi
echo "[PASS] standalone crate checks only; no GUI acceptance implied"
