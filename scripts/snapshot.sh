#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# snapshot.sh — READ-ONLY project state snapshot (Recovery Layer / Agent B)
#
# Purpose: capture "what exactly was the project like at this moment" so that
#          any later failure can be located, compared and rolled back.
#
# READ-ONLY GUARANTEE:
#   - never writes to any tracked file
#   - never mutates git state (no add/commit/checkout/stash/reset/clean)
#   - never reads credentials: no keyring, no cookies, no user data files
#   - the only write target is the output artifact: .snapshots/<file>.txt
#
# Usage:
#   ./scripts/snapshot.sh [label]      e.g. ./scripts/snapshot.sh before-refactor
#   ./scripts/snapshot.sh --list       list recent snapshots
#   ./scripts/snapshot.sh --help
#
# Output: .snapshots/<date>-<label>.txt  (local artifact; untracked by design)
# ---------------------------------------------------------------------------
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

LABEL="${1:-manual}"

if [[ "$LABEL" == "-h" || "$LABEL" == "--help" ]]; then
  sed -n '2,20p' "${BASH_SOURCE[0]}"
  exit 0
fi

if [[ "$LABEL" == "--list" ]]; then
  if [[ -d "$ROOT/.snapshots" ]]; then
    ls -1t "$ROOT/.snapshots" | head -20
  else
    echo "no snapshots yet (.snapshots/ absent)"
  fi
  exit 0
fi

# --- invariant: refuse to run with a label that would escape the output dir ---
case "$LABEL" in
  */*) echo "ERROR: label must not contain '/'"; exit 2 ;;
esac

DATE="$(date +%F)"
TIME="$(date +%H%M%S)"
OUT_DIR="$ROOT/.snapshots"
mkdir -p "$OUT_DIR"
OUT="$OUT_DIR/${DATE}-${LABEL}.txt"
[[ -f "$OUT" ]] && OUT="$OUT_DIR/${DATE}-${LABEL}-${TIME}.txt"

in_git_repo() { git rev-parse --is-inside-work-tree >/dev/null 2>&1; }
gv() { # git value with N/A fallback
  if in_git_repo; then "$@" 2>/dev/null || echo "N/A"; else echo "N/A"; fi
}

{
  echo "=============================================================="
  echo " MVP Browser OS — PROJECT SNAPSHOT (read-only)"
  echo " read-only: no tracked file and no git state was modified"
  echo " no credentials are collected (no keyring / cookies / user data)"
  echo "=============================================================="
  echo "project      : $ROOT"
  echo "label        : $LABEL"
  echo "created_at   : $(date -Is)"
  echo "host         : $(hostname 2>/dev/null || echo N/A)"

  echo
  echo "--- GIT ---"
  echo "branch        : $(gv git rev-parse --abbrev-ref HEAD)"
  echo "HEAD          : $(gv git rev-parse HEAD)"
  echo "HEAD_short    : $(gv git rev-parse --short HEAD)"
  echo "HEAD_subject  : $(gv git log -1 --format=%s)"
  echo "describe      : $(gv git describe --tags)"
  echo "latest_tag    : $(gv git describe --tags --abbrev=0)"
  echo "dirty_files   : $(gv git status --porcelain | wc -l | tr -d ' ')"
  echo "stash_entries : $(gv git stash list | wc -l | tr -d ' ')"
  echo "tags_total    : $(gv git tag | wc -l | tr -d ' ')"
  echo "ahead_origin_master : $(gv git rev-list --count origin/master..HEAD)"
  echo
  echo "dirty file list (porcelain):"
  if in_git_repo; then git status --porcelain | sed 's/^/  /' || true; fi
  echo
  echo "recent tags (newest first):"
  if in_git_repo; then git tag --sort=-creatordate | head -10 | sed 's/^/  /' || true; fi

  echo
  echo "--- TOOLCHAIN ---"
  echo "node   : $(node -v 2>/dev/null || echo N/A)"
  echo "npm    : $(npm -v 2>/dev/null || echo N/A)"
  echo "rustc  : $(rustc -V 2>/dev/null || echo N/A)"
  echo "cargo  : $(cargo -V 2>/dev/null || echo N/A)"
  echo "shell  : $BASH_VERSION"

  echo
  echo "--- PROJECT VERSION ---"
  echo "package      : $(node -e 'const p=require("./package.json");process.stdout.write(p.name+" "+p.version)' 2>/dev/null || echo N/A)"
  echo "tauri        : $(node -e 'const c=require("./src-tauri/tauri.conf.json");process.stdout.write(c.productName+" "+c.version+" "+c.identifier)' 2>/dev/null || echo N/A)"
  echo "NOTE         : deb package version comes from src-tauri/tauri.conf.json, NOT from git v* tags (two independent version systems)"

  echo
  echo "--- CONFIG CHECKSUMS (sha256) ---"
  for f in \
    package.json \
    src-tauri/tauri.conf.json \
    src-tauri/Cargo.toml \
    src-tauri/capabilities/default.json \
    src-tauri/capabilities/browser-remote.json \
    src-tauri/permissions/default-commands.toml \
    src-tauri/permissions/remote-collect.toml \
    scripts/pre-merge.sh \
    scripts/check-view-intent.mjs \
    scripts/check-grid-close-logic.mjs \
    scripts/runtime-phase1-browser-grid.mjs ; do
    if [[ -f "$f" ]]; then
      echo "$(sha256sum "$f" | awk '{print $1}')  $f"
    else
      echo "MISSING                                                          $f"
    fi
  done

  echo
  echo "--- SOURCE SHAPE ---"
  echo "src_files      : $(find src -type f 2>/dev/null | wc -l | tr -d ' ')"
  echo "rust_modules   : $(find src-tauri/src -name '*.rs' 2>/dev/null | wc -l | tr -d ' ')"
  echo "rust_lines     : $(find src-tauri/src -name '*.rs' -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')"
  echo "checker_scripts: $(ls scripts 2>/dev/null | grep -c '^check-')"

  echo
  echo "--- BUILD ARTIFACTS ---"
  if [[ -d dist ]]; then
    echo "dist_bytes     : $(du -sb dist 2>/dev/null | cut -f1)"
    echo "dist_entries   : $(find dist -type f 2>/dev/null | wc -l | tr -d ' ')"
  else
    echo "dist           : absent (npm run build not run)"
  fi
  for m in scripts/build-metrics-*.json; do
    [[ -f "$m" ]] && echo "metrics        : $m"
  done

  echo
  echo "--- DISK ---"
  df -h "$ROOT" 2>/dev/null | tail -1 | sed 's/^/project_fs  : /'
  echo "git_dir_bytes  : $(du -sh .git 2>/dev/null | cut -f1)"
  echo "target_bytes   : $(du -sh src-tauri/target 2>/dev/null | cut -f1 || echo N/A)"
} >"$OUT"

echo "SNAPSHOT_CREATED"
echo "path: $OUT"
echo "timestamp: $(date -Is)"
echo "HEAD: $(gv git rev-parse HEAD)"
echo "branch: $(gv git rev-parse --abbrev-ref HEAD)"
