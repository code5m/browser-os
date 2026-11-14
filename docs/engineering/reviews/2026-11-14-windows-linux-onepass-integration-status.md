# Windows / Linux V2 single-branch integration ledger (2026-11-14)

- Integration branch: feature/browseros-windows-linux-v2-onepass-20261114
- Base: fix/bfc-readback-workbench-hardening-20261114 @ 908a506bd1855ab97b4e0446a4c8321bc77ce853
- Merge policy: **one integrated branch; do not repeatedly merge individual XP tasks into master**.
- Status: **PARTIAL_IMPLEMENTATION**. Not R1/R2 release acceptance; no real Windows GUI or signed-in Bilibili acceptance evidence.
- Source of design: docs/design/browseros-windows-linux-support-v2-20261114.md
- Task backlog: docs/engineering/reviews/2026-11-14-windows-linux-implementation-backlog-v2.md

## Implemented and committed (see branch history)

| Workstream | Production change | Verified evidence | Unverified / not supported |
| --- | --- | --- | --- |
| XP-007 Windows JS result | WebView2/Tauri eval_with_callback added to the native tab manager with JSON-string normalization, one-shot callback and 512 KiB result cap; eliminated previous GRID_PLATFORM_UNSUPPORTED from the Windows eval path | Windows Rust cargo check reached NSIS packaging in run 43587288672 for older branch snapshot; static JS contract passed | No independent real Windows native WebView2 callback/DOM readback test, timeout and page process failure acceptance |
| XP-006 Windows Profile | Stable app-private WebView2 User Data Folder for external browser tabs, separate from trusted main UI | Static source contract passed | Actual Bilibili login remains signed in after cold restart: **NOT TESTED**; Windows ACL/environment options require GUI confirmation |
| XP-008 navigation | Tauri on_page_load Started/Finished instead of falsely emitting NavigationFinished on request callback | Windows Cargo code type-checked | TLS/HTTP 412/redirect/process crash semantics on real Windows still incomplete |
| XP-011R1 file safety | Fail-closed non-Linux write_file_cas and move_path native commands; removed unsafe path-write fallback; unique temp and file fsync for session JSON writes | Static source contract and Rust build for older snapshot | Windows handle-anchored CAS/move, Windows crash/lock/reparse/ACL regression and full user data migration pending |
| XP-004 platform compile isolation | Isolated Unix Grid, LSP sandbox, MCP context, crash logging and Linux GTK/Freedesktop code behind platform cfg; Windows fail-closed Grid/LSP/MCP; nonUnix terminal signal fix and code permission checks | First Windows cargo check progressed to release NSIS step after Windows-only fixes; the latest source requires its own CI result | **Windows Grid/Job Object/Named Pipe, Agent sandbox, ConPTY/AI/full DB functionality NOT implemented** |
| Windows credential store | Added keyring Windows-native feature and updated Cargo.lock to avoid mock credentials | Cargo config and static source contract checked | Windows Credential Manager real secure save/load/delete after upgrade pending |
| XP-018A packaging | Tauri Windows NSIS current-user config, WebView2 Evergreen bootstrapper; experimental Windows build workflow | Windows job entered release NSIS stage in run 43587288672 | NSIS artifact hash, install/upgrade/desktop GUI release evidence not yet established |
| Windows BFC local service | Pinned code5m/bfc-workbench service at 8ded529a2fcaf97a9249e2174a09273d5ee1179c; fixed NTFS-oriented test; PowerShell foreground launcher, isolated user app-data folder, CSRF and loopback import | GitHub Windows runner npm tests, /api/health, /api/capabilities, /api/session and synthetic /api/readback/import receipt + persisted snapshot **PASS**; launcher smoke **PASS** | Not bundled into NSIS, requires locally installed Node >= 20 and pinned service source; no real signed-in Bilibili readback |
| XP-019 Linux regression | Added Ubuntu 22.04 native Rust CI and corrected Tauri frontendDist prerequisite by building Vue frontend first | Earlier Linux run isolated missing dist to CI setup; newer Linux Cargo check underway | Installed DEB GUI, Ubuntu 24.04, AppImage and Linux BFC real-site regression **NOT TESTED** |

## Concrete CI evidence (results apply only to recorded commit/run)

- Windows native Cargo check entered NSIS release build:
  https://github.com/code5m/browser-os/actions/runs/43587288672
- Windows BFC local loopback, synthetic import and actual launcher smoke:
  https://github.com/code5m/browser-os/actions/runs/43587526840
- Linux/Windows independent experimental builds after fixes:
  https://github.com/code5m/browser-os/actions/runs/43587563037
- Original Windows-only compiler error reproduction:
  https://github.com/code5m/browser-os/actions/runs/43586726338

CI results change as commits are added; read the HEAD-specific jobs before making any PASS claim.

## Windows real GUI acceptance (NOT RUN)

See docs/engineering/windows-v2-gui-validation.md and
scripts/windows/verify-installed-browseros.ps1. The script must run on an
interactive Windows 11 desktop against the installed release binary, and its
WINDOW_SMOKE_PASS_GUI_PENDING result is still **not GUI_PASS**. The Bilibili
user must authenticate through the site's own UI; never collect/export Cookie,
activate CDP, or put personal account screenshots in GitHub Actions artifacts.

## R1/R2 remaining blockers

- R1: Actual installed Windows WebView2 tab geometry, native JavaScript
  execution callback against live DOM, Profile cold restart with Bilibili
  login; local BFC readback + true import receipt on real account; complete
  native filesystem/persistence secure operations that R1 exposes.
- R1: Installer verification and supportability of local Node BFC backend
  (currently a separate manual dependency, not NSIS-bundled).
- R2: Windows handle-anchored SecureFileSystem, Job Object + Named Pipe,
  ConPTY/PowerShell terminal, Grid, external DB, Agent/LSP isolation,
  full Windows credential/OS integration, update/rollback; Linux AppImage
  and real GUI regression. Stubs are **unavailable**, not feature parity.
- Read-only BFC does not grant permission to create, move, delete or reorder
  original Bilibili favorites; writeback is a separate product decision.

## Release/merge rule

Only merge to master **once**, after review of the integrated diff and OS-specific
CODE/PACKAGE/GUI/BFC_REAL evidence. Do not bypass or disable existing safety,
governance or supply-chain checks; do not invent tests, screenshots or real logins.
