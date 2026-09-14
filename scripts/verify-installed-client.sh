#!/usr/bin/env bash
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="mvp-browser-os"
PACKAGE_NAME="mvp-browser-os_0.1.0_amd64.deb"
DEB_PATH="$ROOT/src-tauri/target/release/bundle/deb/$PACKAGE_NAME"
EXTRACT_DIR="${TMPDIR:-/tmp}/mvp-browser-os-verify-deb"
LOG_FILE="${TMPDIR:-/tmp}/mvp-browser-os-verify-startup.log"
EXTRACTED_BIN="$EXTRACT_DIR/usr/bin/$APP_NAME"
INSTALLED_BIN="/usr/bin/$APP_NAME"
BUILD=0
INSTALL=0
START_CLIENT=1
FAILURES=0
WARNINGS=0
OWN_PID=""

usage() {
  cat <<'EOF'
Usage: bash scripts/verify-installed-client.sh [--build] [--install] [--no-start]

Checks the real packaged Tauri client, not only the Vite browser page.

Options:
  --build     Build the deb package before checking it.
  --install   Build, stop existing app instances, install the deb, then verify it.
  --no-start  Skip the GUI cold-start check. Useful on headless machines.
EOF
}

ok() { printf '[OK] %s\n' "$*"; }
warn() { WARNINGS=$((WARNINGS + 1)); printf '[WARN] %s\n' "$*"; }
fail() { FAILURES=$((FAILURES + 1)); printf '[FAIL] %s\n' "$*"; }
hint() { printf '       -> %s\n' "$*"; }

cleanup() {
  if [[ -n "$OWN_PID" ]] && kill -0 "$OWN_PID" 2>/dev/null; then
    kill "$OWN_PID" 2>/dev/null || true
    wait "$OWN_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

while [[ $# -gt 0 ]]; do
  case "$1" in
    --build) BUILD=1 ;;
    --install) BUILD=1; INSTALL=1 ;;
    --no-start) START_CLIENT=0 ;;
    -h|--help) usage; exit 0 ;;
    *) fail "Unknown option: $1"; usage; exit 2 ;;
  esac
  shift
done

printf '== mvp-browser-os installed client verification ==\n'
printf 'Project: %s\n\n' "$ROOT"

if [[ ! -f "$ROOT/package.json" ]] || ! grep -q '"name": "mvp-browser-os"' "$ROOT/package.json"; then
  fail "This does not look like the mvp-browser-os frontend package."
  hint "Run this script from /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3, not from a different project."
else
  ok "package.json identifies mvp-browser-os."
fi

if [[ ! -f "$ROOT/src-tauri/Cargo.toml" ]] || ! grep -q '^name = "mvp-browser-os"' "$ROOT/src-tauri/Cargo.toml"; then
  fail "src-tauri/Cargo.toml does not identify the Tauri client package."
  hint "Check whether AGENTS.md or the working directory points to the wrong repository."
else
  ok "src-tauri/Cargo.toml identifies the Tauri client."
fi

if [[ -f "$ROOT/AGENTS.md" ]]; then
  if grep -q '后端 Java 支付中台\|phantom-payment' "$ROOT/AGENTS.md"; then
    fail "AGENTS.md still contains Java payment-center instructions in this Tauri client project."
    hint "Treat that block as polluted scope and fix AGENTS.md before making project decisions."
  elif grep -q 'Tauri\|Vue\|mvp-browser-os' "$ROOT/AGENTS.md"; then
    ok "AGENTS.md contains current client-project guidance."
  else
    warn "AGENTS.md exists but does not clearly mention this Tauri/Vue client."
    hint "Add a project identity section so future agents do not apply unrelated rules."
  fi
else
  warn "AGENTS.md is missing."
  hint "Add project-scoped startup and install verification rules."
fi

if [[ -f "$ROOT/scripts/check-workspace-startup.mjs" ]]; then
  if (cd "$ROOT" && node scripts/check-workspace-startup.mjs >/tmp/mvp-browser-os-workspace-startup.out 2>&1); then
    ok "Workspace store startup regression passed."
  else
    fail "Workspace store startup regression failed."
    sed 's/^/       /' /tmp/mvp-browser-os-workspace-startup.out >&2
    hint "Fix frontend runtime initialization before building; TypeScript build can miss TDZ/order bugs."
  fi
else
  fail "scripts/check-workspace-startup.mjs is missing."
  hint "Add a Pinia store construction test so blank-screen startup regressions are caught."
fi

if [[ -f "$ROOT/scripts/check-image-preview-lazy-loading.mjs" ]]; then
  if (cd "$ROOT" && node scripts/check-image-preview-lazy-loading.mjs >/tmp/mvp-browser-os-image-preview.out 2>&1); then
    ok "Image preview lazy-loading regression passed."
  else
    fail "Image preview lazy-loading regression failed."
    sed 's/^/       /' /tmp/mvp-browser-os-image-preview.out >&2
    hint "Directory preview must not batch-read images. Keep image loading visible-area driven with bounded concurrency."
  fi
else
  warn "scripts/check-image-preview-lazy-loading.mjs is missing."
  hint "Add a regression so large image directories cannot reintroduce eager data URL loading."
fi

if [[ "$BUILD" -eq 1 ]]; then
  printf '\n== Building deb package ==\n'
  if (cd "$ROOT" && npm run tauri -- build --bundles deb); then
    ok "Deb package build completed."
  else
    fail "Deb package build failed."
    hint "Fix the build first; do not validate an old package after a failed build."
  fi
fi

stop_existing_instances() {
  local process_list
  process_list="$(pgrep -a -x "$APP_NAME" || true)"
  if [[ -z "$process_list" ]]; then
    ok "No existing $APP_NAME process needs to be stopped."
    return 0
  fi
  printf '%s\n' "$process_list" | while read -r pid rest; do
    [[ -z "$pid" ]] && continue
    local exe
    exe="$(readlink "/proc/$pid/exe" 2>/dev/null || printf '<unreadable>')"
    if [[ "$exe" == "$INSTALLED_BIN" || "$exe" == "$ROOT/src-tauri/target/release/$APP_NAME" || "$rest" == *"$APP_NAME"* ]]; then
      printf '[INFO] Stopping existing instance pid=%s exe=%s\n' "$pid" "$exe"
      kill "$pid" 2>/dev/null || true
    else
      warn "Found $APP_NAME-like process but skipped it because its executable is unexpected: pid=$pid exe=$exe"
    fi
  done
  for _ in $(seq 1 20); do
    [[ -z "$(pgrep -a -x "$APP_NAME" || true)" ]] && break
    sleep 0.2
  done
  if [[ -n "$(pgrep -a -x "$APP_NAME" || true)" ]]; then
    fail "Existing $APP_NAME process is still running after stop request."
    hint "Close it from the UI or run: pkill -x $APP_NAME"
    return 1
  fi
  ok "Existing $APP_NAME instances stopped."
}

if [[ "$INSTALL" -eq 1 ]]; then
  printf '\n== Install deb package ==\n'
  if [[ ! -f "$DEB_PATH" ]]; then
    fail "Cannot install because deb package is missing: $DEB_PATH"
  elif [[ "$FAILURES" -gt 0 ]]; then
    fail "Install skipped because earlier checks already failed."
  else
    stop_existing_instances
    if [[ "$FAILURES" -eq 0 ]]; then
      if sudo apt install -y "$DEB_PATH"; then
        ok "Deb package installed."
      else
        fail "Deb package install failed."
        hint "If sudo asked for a password, run the same command in a terminal: sudo apt install -y '$DEB_PATH'"
      fi
    fi
  fi
fi

if [[ ! -f "$DEB_PATH" ]]; then
  fail "Deb package not found: $DEB_PATH"
  hint "Run: npm run tauri -- build --bundles deb"
else
  ok "Deb package exists: $DEB_PATH"
fi

if [[ -f "$DEB_PATH" ]]; then
  rm -rf "$EXTRACT_DIR"
  mkdir -p "$EXTRACT_DIR"
  if dpkg-deb -x "$DEB_PATH" "$EXTRACT_DIR"; then
    ok "Deb package extracted for inspection."
  else
    fail "Cannot extract deb package."
    hint "The package is corrupt or dpkg-deb is unavailable. Rebuild the deb."
  fi
fi

if [[ -x "$EXTRACTED_BIN" ]]; then
  ok "Package contains executable: $EXTRACTED_BIN"
else
  fail "Package does not contain executable usr/bin/$APP_NAME."
  hint "Check src-tauri/tauri.conf.json productName/identifier and bundler output."
fi

if [[ -x "$INSTALLED_BIN" && -x "$EXTRACTED_BIN" ]]; then
  INSTALLED_SHA="$(sha256sum "$INSTALLED_BIN" | awk '{print $1}')"
  EXTRACTED_SHA="$(sha256sum "$EXTRACTED_BIN" | awk '{print $1}')"
  if [[ "$INSTALLED_SHA" == "$EXTRACTED_SHA" ]]; then
    ok "Installed /usr/bin binary matches the deb package."
  else
    if [[ "$INSTALL" -eq 1 ]]; then
      fail "Installed /usr/bin binary still does not match the deb package after install."
    else
      fail "Installed /usr/bin binary does not match the deb package."
      hint "Install the new package: sudo apt install -y '$DEB_PATH'"
    fi
    hint "The apt '_apt sandbox' warning for a local deb is usually not an install failure; compare hashes after install."
  fi
elif [[ -x "$EXTRACTED_BIN" ]]; then
  warn "Installed binary is not present at $INSTALLED_BIN."
  hint "Install it with: sudo apt install -y '$DEB_PATH'"
fi

DESKTOP_FILE="/usr/share/applications/$APP_NAME.desktop"
if [[ -f "$DESKTOP_FILE" ]]; then
  EXEC_LINE="$(grep -E '^Exec=' "$DESKTOP_FILE" | tail -n 1 || true)"
  ICON_LINE="$(grep -E '^Icon=' "$DESKTOP_FILE" | tail -n 1 || true)"
  WM_LINE="$(grep -E '^StartupWMClass=' "$DESKTOP_FILE" | tail -n 1 || true)"
  if [[ "$EXEC_LINE" == "Exec=$APP_NAME" || "$EXEC_LINE" == "Exec=/usr/bin/$APP_NAME" ]]; then
    ok "Desktop Exec points to $APP_NAME."
  else
    fail "Desktop Exec is suspicious: ${EXEC_LINE:-<missing>}"
    hint "Reinstall the deb and refresh desktop database: sudo update-desktop-database /usr/share/applications"
  fi
  [[ -n "$ICON_LINE" ]] && ok "Desktop icon is set: $ICON_LINE" || warn "Desktop Icon is missing."
  [[ -n "$WM_LINE" ]] && ok "StartupWMClass is set: $WM_LINE" || warn "StartupWMClass is missing."
else
  warn "Desktop file is not installed: $DESKTOP_FILE"
  hint "Install the package, then verify the launcher again."
fi

printf '\n== Existing process check ==\n'
PROCESS_LIST="$(pgrep -a -x "$APP_NAME" || true)"
if [[ -n "$PROCESS_LIST" ]]; then
  warn "Existing $APP_NAME process found; Tauri single-instance can route a new launch to this old process."
  printf '%s\n' "$PROCESS_LIST" | while read -r pid rest; do
    [[ -z "$pid" ]] && continue
    EXE="$(readlink "/proc/$pid/exe" 2>/dev/null || printf '<unreadable>')"
    printf '       pid=%s exe=%s cmd=%s\n' "$pid" "$EXE" "$rest"
  done
  hint "Close the app from the UI, or run: kill <pid>"
  hint "Then rerun this script so the cold-start check is not hijacked by an old instance."
else
  ok "No existing $APP_NAME process is running."
fi

if [[ "$START_CLIENT" -eq 1 ]]; then
  printf '\n== Cold-start check from extracted package ==\n'
  if [[ -n "$PROCESS_LIST" ]]; then
    fail "Cold-start check skipped because an existing single-instance process is running."
    hint "Stop the listed process, install the new deb if needed, then rerun: bash scripts/verify-installed-client.sh"
  elif [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
    warn "No DISPLAY/WAYLAND_DISPLAY is available; GUI cold-start check skipped."
    hint "Run this on the desktop session, or use --no-start in CI/headless shells."
  elif [[ ! -x "$EXTRACTED_BIN" ]]; then
    fail "Cannot cold-start because extracted executable is missing."
  else
    : > "$LOG_FILE"
    WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND="${GDK_BACKEND:-x11}" GTK_USE_PORTAL=0 "$EXTRACTED_BIN" >"$LOG_FILE" 2>&1 &
    OWN_PID=$!
    ok "Started extracted package pid=$OWN_PID."
    FOUND=0
    for _ in $(seq 1 24); do
      if grep -q '\[FE\].*vue mounted' "$LOG_FILE" && grep -q 'first-paint probe.*root=ok' "$LOG_FILE"; then
        FOUND=1
        break
      fi
      if ! kill -0 "$OWN_PID" 2>/dev/null; then
        break
      fi
      sleep 0.5
    done
    if [[ "$FOUND" -eq 1 ]]; then
      ok "Extracted package reached Vue mount and first-paint probe."
      grep -E '\[main\] main window url=|\[FE\].*vue mounted|first-paint probe|\[M0\] m0_ready' "$LOG_FILE" | tail -n 8 | sed 's/^/       /'
    else
      fail "Extracted package did not report Vue mount + first-paint."
      sed -n '1,160p' "$LOG_FILE" | sed 's/^/       /'
      hint "If logs show ReferenceError/TypeError, fix frontend startup before packaging."
      hint "If logs show an old dev URL or no tauri://localhost, inspect tauri config and single-instance routing."
      hint "If screenshots are black on Wayland, first verify the screenshot tool; black scrot output alone is not proof of blank UI."
    fi
  fi
fi

printf '\n== Result ==\n'
if [[ "$FAILURES" -gt 0 ]]; then
  printf 'FAILED: %s failure(s), %s warning(s).\n' "$FAILURES" "$WARNINGS"
  exit 1
fi
printf 'PASSED: 0 failure(s), %s warning(s).\n' "$WARNINGS"
exit 0
