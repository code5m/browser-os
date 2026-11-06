# BrowserOS freeze and address-bar navigation audit · 2026-11-05

## Evidence collected (developer workstation, installed build)

- **2026-11-05 17:37:30 +08:00:** kernel logged `global_oom` and killed a **ChatGPT** process (PID 7874), not BrowserOS. This occurred **before** the installed BrowserOS run below. It proves environment memory pressure, **not** that this specific BrowserOS process deadlocked or crashed.
- Installed binary: `/usr/bin/mvp-browser-os` (timestamp 17:43 +08:00).
- **2026-11-05 17:47:49 +08:00:** BrowserOS session `session-20261105-174749-main-pid300390.log` began. There is no `crash.log`; BrowserOS main and WebKit child processes were still alive at capture. A GTK thread seen in `poll_schedule_timeout` at sampling time is **not proof of a deadlock**.
- Its session log contains **66 native `navigationFinished` callbacks** for `tab-2`, including repeated **`about:blank`** and **`about:srcdoc`**, and **66 unfiltered `tab-navigated` forwards**. The same session reports some `Load request cancelled` and TLS handshake failures. Do not treat cancellation as a fatal renderer crash.
- Source trace: `tauri-browser-tabs/.../commands.rs` emitted all `on_navigation` callbacks, `main.rs` forwarded every URL, `App.vue` bound `onTabNavigated` to `useBrowserStore.setNavigated`, and `ActivityBar.vue` displayed `browser.url`. An iframe could therefore overwrite a tab's visible main-frame URL. Additionally, `ActivityBar.vue` intentionally skipped state updates while focused but did not reconcile the latest URL when focus ended.

## Root-cause confidence

| Finding | Confidence |
| --- | --- |
| Linux system ran out of memory and killed a ChatGPT process at 17:37 | CONFIRMED |
| The *currently installed BrowserOS process* was killed by that OOM | FALSE — current process started 17:47 |
| iframe navigation events corrupt the main tab URL / address bar | CONFIRMED in log and source, true GUI acceptance still required |
| Main GTK event loop was demonstrably deadlocked at sample time | NOT ESTABLISHED |
| Synchronous Grid reposition/shutdown on GTK event callbacks can block | CODE PATH CONFIRMED, latency and reproducibility not measured |
| BrowserOS had comprehensive hang diagnostics | FALSE — existing crashlog records stderr/panic/fatal signals, not nonfatal UI unresponsiveness |

## Changes in the one integration branch

1. Linux native browser plugin now observes **WebKitGTK top-level WebView `notify::uri`**, not iframe-level `on_navigation`; no adjustments to locked GTK move/resize/size_allocate/hide behavior. Other platforms retain their previous navigation path.
2. Address input refreshes its actual URL upon blur. Browser store ignores stale closed-tab and empty navigation events.
3. Trusted native navigation forwarding uses numeric sequence IDs and logs metadata (tab label, URL length), not full URLs. Main Vue Shell records matching receipt and 5-second pulse, plus timer lag.
4. Central Tauri bridge records only **slow/failed IPC command name + elapsed time + status**, rate-limited; no arguments, responses or exception messages. Use these to differentiate a blocked command from a silent renderer.
5. A dedicated **out-of-band GTK responsiveness worker** queues **at most one** main-thread callback. If no callback reaches GTK for >=5 seconds, it writes a bounded, private `hang-...jsonl` with thread wait channels, RSS/Swap, WebKit child PID metadata and Linux memory pressure. A recovery event is written when GTK resumes.
6. `scripts/collect-browseros-hang.sh` is independently executable from a second terminal even if the BrowserOS window is frozen. It captures process/thread state, kernel OOM/hung/GPU signals, redacted tail of 3 recent session logs and any watchdog snapshots into a local-only tarball; it **does not kill or attach to the running GUI**. No WebKit cookies, profile data, HTML, clipboard, prompts, SQL, or raw browsing history are intentionally collected.

## How to reproduce and capture

Without closing the stuck app, run from a separate terminal:

```bash
cd '/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3'
bash scripts/collect-browseros-hang.sh
```

The archive appears under `~/.local/share/com.jizhijiandan.mvp/diagnostics/support-*.tar.gz`. Review its contents before sharing. Existing app session logs are under `~/.local/share/com.jizhijiandan.mvp/logs/session-*.log`. The *new* hang JSONL will appear only after installing and running a binary containing this integration branch.

## Verification requirements and limitations

- `cargo fmt --check`, `cargo check --locked`, the focused watchdog unit test, `npm run check:module-contracts` and full `npm run check` should pass after changes.
- **Real installed Linux GUI regression is mandatory** for main frame redirects, iframe insertion, `history.pushState/replaceState`, Back/Forward, popups, tab switching, native WebView focus and resize. A static check or Rust typecheck cannot establish that WebKit GTK `notify::uri` captures every SPA navigation.
- Reproduce a controlled **GTK event loop stall** in a test-only client; verify `gtk_unresponsive_suspected` is emitted while UI is stuck and `gtk_responsive_again` after release, and watchdog does not enqueue unbounded callbacks.
- Reproduce controlled shell JS loop lag and a long/failed Tauri IPC; verify no URL/query/user content is written to diagnostics, no secrets are copied to the support archive.
- Existing **P0 Core+Shared 1.5 MiB approved budget** and image-preview policy remain blocking in the integration branch; do not reset the historical baseline, expand a protected remote ACL without a security review, or merge prematurely.
- This change is a **diagnostic and URL-synchronization fix**. Full Grid relocation/shutdown async refactoring and exhaustive end-to-end installed GUI acceptance remain open; no `master PASS` or `GUI PASS` is claimed.
