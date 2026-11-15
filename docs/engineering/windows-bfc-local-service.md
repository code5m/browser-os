# Windows BFC local service: pinned dependency and manual foreground start

BrowserOS BFC reads user-visible Bilibili favorites DOM and sends the snapshot to a separate
loopback-only service at `127.0.0.1:4198`. Windows BrowserOS alone **cannot save a snapshot**
unless the separate service is running. This is a release dependency, not an online Bilibili API.

## Pinned source (review before distribution)

- Repository: https://github.com/code5m/bfc-workbench
- Commit pinned for Windows CI: `cb113c219a90df06db6ceeec10d1a9be46ad3694`
- Runtime: Node.js 20+, PowerShell on Windows 11 x64.
- Service root (example): `$env:LOCALAPPDATA\BrowserOS\bfc-workbench`
- User data: `$env:LOCALAPPDATA\BrowserOS\Bilibili-Favorites-Review`.
  This directory is **outside the Git repository**, must never be committed or deleted on upgrade.

## Manual installation

```powershell
New-Item -ItemType Directory -Path (Join-Path $env:LOCALAPPDATA 'BrowserOS') -Force
git clone https://github.com/code5m/bfc-workbench.git "$env:LOCALAPPDATA\BrowserOS\bfc-workbench"
git -C "$env:LOCALAPPDATA\BrowserOS\bfc-workbench" checkout cb113c219a90df06db6ceeec10d1a9be46ad3694
pwsh -File scripts/windows/start-bfc-local.ps1
```

The script intentionally does **not** download arbitrary code, obtain Bilibili credentials,
register a scheduled startup task or run in the background. It checks for a local Node.js
runtime and runs the pinned service in the foreground until Ctrl+C.

## Verification

```powershell
Invoke-RestMethod http://127.0.0.1:4198/api/health
Invoke-RestMethod http://127.0.0.1:4198/api/capabilities
```

Require health `ok:true`, `localOnly:true`, `bilibiliNetworkRequests:0`,
and capabilities `bilibili.readSnapshotOnly:true`.
BrowserOS verifies health/capabilities, then proves the service identity with a per-user token challenge before requesting `/api/browseros/session` or sending any snapshot. The token file is `.browseros-service-identity.json` in the data directory; Windows ACL is rebuilt for the current SID and SYSTEM on every service launch; reparse points are rejected and existing child-file ACLs are reset to inherit from the protected root. Linux mode is 0600. Import also requires origin, CSRF and `X-BrowserOS-Service-Token`;
the localhost service is **not** a system-wide privileged service.

## Failure and lifecycle

- Service unavailable → show `BFC_LOCAL_SERVICE_UNAVAILABLE`, do not claim import.
- Port 4198 occupied by different app → stop, do not send personal snapshots to unknown server;
  the current integration verifies the per-user service identity and HMAC challenge before transmitting any snapshot body.
- No WebView2 logged-in DOM, HTTP 412, unsafe partial, receipt mismatch, timeout → preserve
  the existing partial / unknown / rejected semantics without retrying blindly.
- This document does not claim an interactive Windows GUI, installer or a real Bilibili
  login session has passed. Those require actual Windows desktop verification.

Related: [V2 plan](../../docs/design/browseros-windows-linux-support-v2-20261114.md)
and [integration status](../../docs/engineering/reviews/2026-11-14-windows-linux-onepass-integration-status.md).
