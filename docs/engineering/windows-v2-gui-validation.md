# BrowserOS Windows V2: real desktop & BFC login acceptance (pending)

Status: **GUI_PENDING / BFC_REAL_PENDING**. Windows GitHub Actions can validate Rust,
NSIS and localhost service with synthetic data but cannot establish a real
signed-in Bilibili user's persistent login or verify UI screenshots personally.

## Acceptance steps on a Windows 11 x64 desktop

1. Record Windows build, WebView2 Evergreen Runtime version, Rust/Tauri version,
   BrowserOS commit and NSIS installer checksum.
2. Build/install the **release NSIS** package as the current user; keep the
   BrowserOS data directories when upgrading.
3. Normally close only any existing BrowserOS process; verify desktop shortcut
   points to the installed binary and executable SHA-256 matches the new package.
4. Run the local PowerShell script
   scripts/windows/verify-installed-browseros.ps1 with a full -ExePath to the
   installed executable, and optionally -CaptureScreenshot. Inspect the real
   window rendering, dimensions, child WebView2 navigation and input focus.
   The script returns WINDOW_SMOKE_PASS_GUI_PENDING, **never GUI_PASS**.
5. Start the BFC local service from the pinned source with
   scripts/windows/start-bfc-local.ps1 in a separate foreground PowerShell
   session. Check /api/health localOnly true, no Bilibili requests, /api/session
   CSRF token and readSnapshotOnly true.
6. Manually log in to Bilibili using its normal UI in an external browser child
   WebView2. Do not export Cookie, use remote debugging/CDP or fake login state.
7. Begin BFC readback in BrowserOS. Confirm actual active HTTPS Bilibili tab,
   MID matching the visible login and a definitive DOM readback. Check initial
   page, multiple pages, incomplete captures and any access-limited response.
8. Import through the local BFC service. Compare scanId, pages, cards, unique
   BVID count and snapshot evidence with the local receipt. A timeout after
   send must stay outcome_unknown: do not blindly retry.
9. Close BrowserOS normally, cold launch from the installed desktop shortcut,
   and verify the real external browser's Bilibili login state persists.
   The trusted main UI remains separate from external page session data.
10. Run Ubuntu 22.04/24.04 X11 installed-client GUI regression independently;
    neither platform inherits GUI_PASS from the other.

## Evidence boundaries and security

- Cargo check is CODE, release NSIS package build is PACKAGE.
- Node service on 127.0.0.1 and synthetic BFC import is BFC_FIXTURE only.
- Windows hosted actions window/process smoke cannot establish real login.
- Do not publish real Bilibili account screenshots via public CI artifacts.
- A failing WebView2 callback, window lifecycle, TLS, HTTP 412 or service
  version mismatch must remain PENDING/BLOCKED, never fabricated as PASS.
- Do not delete existing user data to make an upgrade or login test succeed.
- If no Windows GUI is available, log GUI_PENDING and BFC_REAL_PENDING with
  concrete pending verification steps; do not imply later work is automated.

## Related documents

- docs/design/browseros-windows-linux-support-v2-20261114.md
- docs/engineering/windows-bfc-local-service.md
- docs/engineering/reviews/2026-11-14-windows-linux-onepass-integration-status.md
