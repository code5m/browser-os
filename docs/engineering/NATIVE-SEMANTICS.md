# Rust Core Semantic Closure

BrowserOS treats Rust/Tauri semantics as a first-class support system.

## Authorities — no duplicate truth

- Rust registration: `src-tauri/src/main.rs` `generate_handler!`
- Rust definitions: `src-tauri/src/**/*.rs` `#[tauri::command]`
- Owner/resource/permission/side-effect: `docs/architecture/native-boundary/native-commands.yaml`
- AppState ownership: `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md`
- Frontend IPC closure: `src/bridge.ts` + `scripts/check-command-set-consistency.py`
- Capability ownership/dependencies: existing Capability registry and manifests

`npm run check:native-semantics` composes those existing authorities. It fails if the native command inventory is not closed, semantic metadata is incomplete, ownership is UNKNOWN, or the governed command/AppState counts drift without an explicit governance update.

`node scripts/check-native-semantic-closure.mjs --report artifacts/native-semantic-report.json` creates the machine-readable audit artifact used by CI. This report is derived evidence, never a new permission or ownership source.

Current governed baseline at this change: **149 registered native commands / 26 AppState fields**. These numbers are drift tripwires, not targets; legitimate Rust changes update code, authoritative registries and governance together.

## Runtime Event / IPC cross-layer observation

`npm run check:rust-runtime-semantics` and `--report artifacts/rust-runtime-semantics.json` scan Rust files for literal event emission sites, identify literal event listeners in the TypeScript bridge, and reconcile literal bridge IPC names with registered Rust handlers. Negative fixtures prove unknown literal IPC calls are detected.

This is **an observed semantic graph, not a claim of full cross-language type equivalence**: dynamic event names and TS consumers outside the bridge may not be visible to this projection, and the Rust Serde DTO / TypeScript structural schema correspondence remains explicitly UNVERIFIED where no executable structural test exists. Existing native inventory, lifecycle, state ownership, and cross-layer checks remain blocking sources of truth.
