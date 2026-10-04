# Installed client GUI acceptance — 2026-10-05

## Result

`GUI_PASS` for the installed Debian client.

- dpkg status: `install ok installed`, version `0.1.0`, architecture `amd64`.
- installed entry: `/usr/bin/mvp-browser-os`.
- installed binary SHA-256: `24048434eb2f26eb166733582a2047a8e4a8f4bec153a01b6d3c6ebb1f828b48`.
- the installed binary exactly matches the executable extracted from the final deb.
- desktop entry: `Exec=mvp-browser-os`, `Icon=mvp-browser-os`, `StartupWMClass=mvp-browser-os`.
- `verify-installed-client.sh`: PASSED, 0 failures, 0 warnings.
- cold-start window: `1854x1048`, `tauri://localhost`, Vue mounted, first-paint root present, `bootstrap activated=true error=none`.

The installed `/usr/bin` executable was launched with an isolated XDG profile and passed:

1. `ACTIVE → SUSPENDED → ACTIVE`.
2. `ACTIVE → SUSPENDED → DISABLED → ACTIVE`.
3. Disabled state and missing bookmark contribution survived frontend refresh.
4. Disabled state and missing bookmark contribution survived full process shutdown and restart.
5. Enabling after restart restored exactly one bookmark entry.
6. Capability Manager remained available and did not show `当前视图不可用`.
7. No runtime error or panic was observed.

The test process was stopped after evidence capture.

## Evidence

| Evidence | Assertion |
|---|---|
| [01-home-installed.png](01-home-installed.png) | Real installed client paints a non-white/non-black application window. |
| [02-manager-suspended.png](02-manager-suspended.png) | Installed Manager reaches `SUSPENDED` and exposes resume/disable. |
| [03-manager-disabled-after-refresh.png](03-manager-disabled-after-refresh.png) | Disabled state and contribution absence survive refresh. |
| [04-manager-disabled-after-restart.png](04-manager-disabled-after-restart.png) | Disabled state and contribution absence survive process restart. |
| [05-manager-enabled-final.png](05-manager-enabled-final.png) | Re-enable restores exactly one contribution and leaves the final state active. |
