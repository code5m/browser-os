# Native 58 — installed GUI side-effect audit (2026-11-09)

## Proof requirements

- Exactly 58 names from `native-permission-debt-baseline.json`; same names are checked statically against the JavaScript driver and the Rust handler. No new production IPC command is added.
- **Installed, exact-SHA Tauri Release** only: use the built `.deb` installed `/usr/bin/mvp-browser-os` after package digest verification. Do not substitute a mocked `invoke`, browser-based Vue preview, or a Rust unit test.
- Run in a fresh **Xvfb + DBus + temporary HOME/XDG** profile. A local `xdg-settings` recorder prevents changing the user's actual default browser. Clipboard access is bound to that isolated X11 display. The HTTP fixture binds only loopback and uses synthetic, non-sensitive content.
- `report_title` and `report_resources` execute in a real remote-origin child WebView; verify emitted events and a separate, correctly classified denial of remote `clipboard_write`.
- The remaining 56 legacy IPC calls execute through the installed trusted `main` WebView. Record operation results, readbacks, scope and one-shot consent rejection, and native postconditions for sessions, resource buffers, windows, tabs, snippets and bookmarks.
- An error, skipped step, missing native postcondition or missing external evidence is **FAIL**, never a synthetic PASS. The full JSON contains every command status and the evidence folder contains server events, source hash, application log and a live screenshot.

## Run

```bash
# After building and verifying the same source/package, in an isolated display:
dbus-run-session -- xvfb-run -a python3 scripts/native-58-gui-regression.py \
  --binary /usr/bin/mvp-browser-os --evidence-dir /tmp/browseros-native-58-evidence
```

GitHub `native-regression` uses this same script after the package cold-start and M0 GUI regression, uploading `/tmp/browseros-native-58-evidence/*` to the existing GUI artifact. Required statuses are `58/58 PASS`, `nativePostconditionsStatus=PASS` and external `remote_origin_allow_and_deny=PASS`, `child_webview_eval_side_effect=PASS`, `isolated_xdg_settings=PASS`.

**Status:** Driver and CI wiring created. The legacy 58-item *pending side-effect review* ledger must not be cleared until the above installed-release artifacts exist for the exact merge candidate SHA and have been inspected. Static-source and Rust unit passes alone do not close this gate.
