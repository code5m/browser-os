#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# collect-diagnostics.sh — READ-ONLY runtime diagnostics (Recovery Layer / Agent C)
#
# Purpose: when the app misbehaves (crash / hang / blank / native failure),
#          gather enough evidence to locate and judge impact BEFORE changing
#          anything (DIAGNOSTICS BEFORE ROLLBACK).
#
# READ-ONLY GUARANTEE:
#   - never writes to any tracked file or app data
#   - never kills, restarts or signals any process
#   - the only write target is: diagnostics/<timestamp>/
#
# PRIVACY GUARANTEE (redaction applied to every dynamic text we emit):
#   - NEVER reads keyring entries (git tokens / db passwords / site passwords)
#   - NEVER reads cookies, sessions content, or user files/notes
#   - NEVER copies app data CONTENT; only names, sizes and mtimes
#   - log tails are redacted (token/secret/password/Bearer/sk-/AKIA patterns)
#
# Usage:
#   ./scripts/collect-diagnostics.sh [label]
#   ./scripts/collect-diagnostics.sh --list
#
# Output: diagnostics/<timestamp>/{system.txt,process.txt,app.txt,git.txt}
# ---------------------------------------------------------------------------
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

LABEL="${1:-diag}"
if [[ "$LABEL" == "-h" || "$LABEL" == "--help" ]]; then
  sed -n '2,24p' "${BASH_SOURCE[0]}"
  exit 0
fi
if [[ "$LABEL" == "--list" ]]; then
  if [[ -d "$ROOT/diagnostics" ]]; then
    ls -1t "$ROOT/diagnostics" | head -20
  else
    echo "no diagnostics yet (diagnostics/ absent)"
  fi
  exit 0
fi

TS="$(date +%Y%m%d-%H%M%S)"
OUT="$ROOT/diagnostics/$TS"
mkdir -p "$OUT"

APP=mvp-browser-os
APP_IDENT="com.jizhijiandan.mvp"
INSTALLED_BIN="/usr/bin/$APP"
DATA_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/$APP_IDENT"
LOG_DIR="$DATA_ROOT/logs"

# Redact anything that looks like a credential before it reaches a report file.
redact() {
  sed -E \
    -e 's/([?&](token|access_token|api_key|apikey|key|secret|password|passwd|pwd|auth|sig|signature)=)[^&"'"'"' ]*/\1<REDACTED>/Ig' \
    -e 's/(Bearer[[:space:]]+)[A-Za-z0-9._-]+/\1<REDACTED>/Ig' \
    -e 's/sk-[A-Za-z0-9_-]{6,}/<REDACTED>/g' \
    -e 's/AKIA[0-9A-Z]{8,}/<REDACTED>/g' \
    -e 's/(-----BEGIN [A-Z ]*PRIVATE KEY-----).*/\1<REDACTED>/g'
}

banner() {
  echo "=============================================================="
  echo " MVP Browser OS — DIAGNOSTICS ($1)"
  echo " read-only · no process signalled · no app data content copied"
  echo " redacted: no keyring / cookie / user-file content included"
  echo "=============================================================="
  echo "label      : $LABEL"
  echo "created_at : $(date -Is)"
  echo "host       : $(hostname 2>/dev/null || echo N/A)"
  echo
}

in_git_repo() { git rev-parse --is-inside-work-tree >/dev/null 2>&1; }
gv() { if in_git_repo; then "$@" 2>/dev/null || echo N/A; else echo N/A; fi; }

# ------------------------------- system.txt --------------------------------
{
  banner "system.txt"
  echo "--- OS / KERNEL ---"
  echo "kernel    : $(uname -srm 2>/dev/null)"
  echo "os        : $(. /etc/os-release 2>/dev/null && echo "${PRETTY_NAME:-N/A}" || echo N/A)"
  echo "arch      : $(uname -m 2>/dev/null)"
  echo "session   : type=${XDG_SESSION_TYPE:-N/A} desktop=${XDG_CURRENT_DESKTOP:-N/A}"
  echo "display   : DISPLAY=${DISPLAY:-unset} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-unset}"

  echo
  echo "--- CPU / MEMORY ---"
  echo "nproc     : $(nproc 2>/dev/null || echo N/A)"
  lscpu 2>/dev/null | grep -m1 'Model name' | sed 's/^/cpu       : /'
  echo "loadavg   : $(cat /proc/loadavg 2>/dev/null || echo N/A)"
  free -h 2>/dev/null | sed 's/^/mem       : /'

  echo
  echo "--- DISK ---"
  df -h / 2>/dev/null | tail -1 | sed 's/^/rootfs    : /'
  df -h "$HOME" 2>/dev/null | tail -1 | sed 's/^/homefs    : /'
  echo "project_fs: $(df -h "$ROOT" 2>/dev/null | tail -1)"

  echo
  echo "--- NATIVE DEPENDENCIES ---"
  echo "webkit2gtk-4.1 : $(pkg-config --modversion webkit2gtk-4.1 2>/dev/null || echo N/A)"
  echo "webkit2gtk-4.0 : $(pkg-config --modversion webkit2gtk-4.0 2>/dev/null || echo N/A)"
  echo "libsoup-3.0    : $(pkg-config --modversion libsoup-3.0 2>/dev/null || echo N/A)"
  echo "gtk+-3.0       : $(pkg-config --modversion gtk+-3.0 2>/dev/null || echo N/A)"
  echo "installed_pkg  : $(dpkg-query -W -f='${Package} ${Version} ${Status}' libwebkit2gtk-4.1-0 2>/dev/null || echo N/A)"
} >"$OUT/system.txt"

# ------------------------------ process.txt --------------------------------
{
  banner "process.txt"
  echo "--- APP PROCESSES ---"
  if pgrep -f "$APP" >/dev/null 2>&1; then
    pgrep -af "$APP" | redact | sed 's/^/  /'
  else
    echo "  (none running)"
  fi

  echo
  echo "--- WEBVIEW CHILD PROCESSES ---"
  echo "WebKitWebProcess count    : $(pgrep -cf 'WebKitWebProcess' 2>/dev/null || echo 0)"
  echo "WebKitNetworkProcess count: $(pgrep -cf 'WebKitNetworkProcess' 2>/dev/null || echo 0)"
  ps -eo pid,ppid,rss,stat,comm 2>/dev/null | grep -i 'webkit' | head -20 | sed 's/^/  /'

  echo
  echo "--- PROCESS OVERVIEW ---"
  echo "total processes : $(ps -e 2>/dev/null | wc -l | tr -d ' ')"
  echo "zombies         : $(ps -eo stat 2>/dev/null | grep -c '^Z' || echo 0)"
  echo
  echo "top by CPU:"
  ps -eo pid,pcpu,pmem,rss,comm --sort=-pcpu 2>/dev/null | head -10 | redact | sed 's/^/  /'
  echo
  echo "top by RSS:"
  ps -eo pid,pcpu,pmem,rss,comm --sort=-rss 2>/dev/null | head -10 | redact | sed 's/^/  /'
} >"$OUT/process.txt"

# -------------------------------- app.txt ----------------------------------
{
  banner "app.txt"
  echo "--- INSTALLED BINARY ---"
  if [[ -f "$INSTALLED_BIN" ]]; then
    echo "path      : $INSTALLED_BIN"
    echo "size      : $(stat -c%s "$INSTALLED_BIN" 2>/dev/null) bytes"
    echo "mtime     : $(stat -c%y "$INSTALLED_BIN" 2>/dev/null)"
    echo "sha256    : $(sha256sum "$INSTALLED_BIN" 2>/dev/null | awk '{print $1}')"
  else
    echo "path      : $INSTALLED_BIN (ABSENT — app not installed)"
  fi
  echo "dpkg      : $(dpkg-query -W -f='${Package} ${Version} ${Status}' "$APP" 2>/dev/null || echo N/A)"

  echo
  echo "--- DESKTOP ENTRY / ICON ---"
  for f in "/usr/share/applications/$APP.desktop" "/usr/share/icons/hicolor/512x512/apps/$APP.png"; do
    if [[ -f "$f" ]]; then echo "present   : $f ($(stat -c%s "$f" 2>/dev/null) bytes, $(stat -c%y "$f" 2>/dev/null))"
    else echo "absent    : $f"; fi
  done

  echo
  echo "--- NATIVE LINKAGE ---"
  if [[ -f "$INSTALLED_BIN" ]]; then
    ldd "$INSTALLED_BIN" 2>/dev/null | grep -iE 'webkit|gtk|gstreamer|soup' | head -15 | sed 's/^/  /'
  fi

  echo
  echo "--- APP DATA (metadata only; NO file content is read) ---"
  echo "data_root : $DATA_ROOT"
  if [[ -d "$DATA_ROOT" ]]; then
    echo "data_size : $(du -sh "$DATA_ROOT" 2>/dev/null | cut -f1)"
    echo "top-level entries (name + size + mtime):"
    ls -la "$DATA_ROOT" 2>/dev/null | tail -n +2 | awk '{print "  "$5"  "$6" "$7"  "$9}'
    if [[ -d "$DATA_ROOT/$APP" ]]; then
      echo "business data dir ($DATA_ROOT/$APP):"
      ls -la "$DATA_ROOT/$APP" 2>/dev/null | tail -n +2 | awk '{print "  "$5"  "$9}'
    fi
  else
    echo "data_root : ABSENT"
  fi
  echo "NOTE      : file contents (sessions, workspace artifacts, repos) are deliberately NOT copied."

  echo
  echo "--- LOGS (names + redacted tail only) ---"
  echo "log_dir   : $LOG_DIR"
  if [[ -d "$LOG_DIR" ]]; then
    ls -la "$LOG_DIR" 2>/dev/null | tail -n +2 | awk '{print "  "$5"  "$6" "$7"  "$9}'
    NEWEST="$(ls -t "$LOG_DIR" 2>/dev/null | head -1)"
    if [[ -n "${NEWEST:-}" && -f "$LOG_DIR/$NEWEST" ]]; then
      echo
      echo "tail of newest log: $NEWEST (last 40 lines, redacted)"
      tail -n 40 "$LOG_DIR/$NEWEST" 2>/dev/null | redact | sed 's/^/  /'
    fi
    if [[ -f "$LOG_DIR/crash.log" ]]; then
      echo
      echo "tail of crash.log (last 40 lines, redacted)"
      tail -n 40 "$LOG_DIR/crash.log" 2>/dev/null | redact | sed 's/^/  /'
    fi
  else
    echo "log_dir   : ABSENT"
  fi
} >"$OUT/app.txt"

# -------------------------------- git.txt ----------------------------------
{
  banner "git.txt"
  echo "branch       : $(gv git rev-parse --abbrev-ref HEAD)"
  echo "HEAD         : $(gv git rev-parse HEAD)"
  echo "HEAD_short   : $(gv git rev-parse --short HEAD)"
  echo "HEAD_subject : $(gv git log -1 --format=%s)"
  echo "describe     : $(gv git describe --tags)"
  echo "latest_tag   : $(gv git describe --tags --abbrev=0)"
  echo "dirty_files  : $(gv git status --porcelain | wc -l | tr -d ' ')"
  echo
  echo "recent commits:"
  if in_git_repo; then git log --oneline -10 | sed 's/^/  /' || true; fi
  echo
  echo "dirty files:"
  if in_git_repo; then git status --porcelain | sed 's/^/  /' || true; fi
  echo
  echo "NOTE: a dirty worktree means the running binary may not match the source;"
  echo "      rebuild (npm run build / tauri build) before concluding a code bug."
} >"$OUT/git.txt"

echo "DIAGNOSTICS_CREATED"
echo "path: $OUT"
echo "timestamp: $(date -Is)"
echo "files: system.txt process.txt app.txt git.txt"
echo "redaction: applied (no keyring / cookie / user-file content collected)"
