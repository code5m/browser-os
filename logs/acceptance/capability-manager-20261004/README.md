# Capability Manager release GUI acceptance — 2026-10-04

## Scope and truth boundary

- Source commit under test: `2487f56` plus the final dependency-boundary cleanup in the working tree.
- Release package: `src-tauri/target/release/bundle/deb/mvp-browser-os_0.1.0_amd64.deb`.
- Package SHA-256: `6f7b239c909184f35d1beecf986e2e2e560b535fcac75af506c44d5f0a0df28f`.
- Release GUI executable: the exact binary extracted from that deb by `verify-installed-client.sh`.
- The GUI run used an isolated XDG data/config/cache directory, so it did not mutate the user's installed-client profile.
- This is valid release-package GUI evidence. It is **not** presented as installed `/usr/bin` evidence: the installed binary still has a different hash and replacement requires interactive sudo authentication.

## Result

The generated release package rendered a real `1854x1048` desktop window and passed the Capability Manager lifecycle acceptance for `bookmark`:

1. `ACTIVE → SUSPENDED → ACTIVE`.
2. `ACTIVE → SUSPENDED → DISABLED → ACTIVE`.
3. Three consecutive lifecycle rounds ended in `ACTIVE` with exactly one contribution entry.
4. A disabled state survived frontend refresh; the bookmark entry remained absent.
5. A disabled state survived full process shutdown and release-binary restart; the bookmark entry remained absent.
6. Re-enabling after restart restored exactly one entry.
7. Capability Manager remained visible throughout, and `当前视图不可用` did not appear.
8. Startup logs reported `bootstrap activated=true error=none`; no panic or runtime error was observed.

The test process was stopped after capture so it cannot intercept a later desktop-icon launch.

## Evidence

| Evidence | What it proves |
|---|---|
| [01-home-release.png](01-home-release.png) | The release package paints a real non-white/non-black application window. |
| [02-manager-active.png](02-manager-active.png) | Capability Manager is available and bookmark is active. |
| [03-manager-suspended.png](03-manager-suspended.png) | Pause reaches `SUSPENDED` and exposes the valid resume/disable actions. |
| [04-manager-disabled-after-refresh.png](04-manager-disabled-after-refresh.png) | Disabled state and contribution absence survive frontend refresh. |
| [05-manager-disabled-after-restart.png](05-manager-disabled-after-restart.png) | Disabled state and contribution absence survive process restart. |
| [06-manager-enabled-final.png](06-manager-enabled-final.png) | Enable restores the contribution exactly once after restart. |

## Installed-client blocker

`npm run verify:client` completed the release build, deb extraction, desktop-entry inspection, geometry check, and extracted-release cold start. Its only failure was the expected installed-binary mismatch:

- generated deb binary: `3698eb98e9480c863f58b44f4cae5acdf128ab8ead9b841d6d3d57cdf4ca3c22`
- installed `/usr/bin/mvp-browser-os`: `96ebcbd03cf7355113aa2fc500605c7ba9df77298eed1f26f6aa2b8fdbc2b6e4`
- desktop entry: `Exec=mvp-browser-os`

Replacing `/usr/bin/mvp-browser-os` is a system installation action and requires the user to enter the sudo password directly. Until that is done and the installed entry is cold-started, the truthful installed-client status remains `GUI_PENDING`.
