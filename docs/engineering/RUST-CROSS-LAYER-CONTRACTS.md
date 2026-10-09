# Rust Cross-Layer Contracts

> Status: ACTIVE / BLOCKING for command coverage, invoke payload keys, and listener cleanup.
> Rust source remains the authority. Generated TypeScript and JSON are derived outputs, not handwritten truth.

## Why

BrowserOS has a Rust/Tauri backend and a TypeScript/Vue frontend. A Rust command can compile successfully while the frontend silently sends a wrong argument key or keeps an event listener alive. This gate makes those cross-layer contracts observable and machine-checkable.

## Authorities

- Command registration: `src-tauri/src/main.rs` `generate_handler!`
- Command signatures and Serde DTOs: `src-tauri/src/**/*.rs`
- Native semantic owner/resource/permission: `docs/architecture/native-boundary/native-commands.yaml`
- Resource lifecycle ownership: `docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md`
- Frontend observations: literal `invoke()` / `listen()` calls under `src/`

No generated file is an authority.

## Generated outputs

Before `dev`, `build`, and `check`:

```bash
npm run generate:rust-contracts
```

regenerates:

- `src/generated/native-contracts.ts` — consumable TypeScript command/event contract types.
- `artifacts/native-contracts/generated-crosslayer-contracts.json` — machine report with command, event, lifecycle, and impact data.

Both are generated/ignored outputs.

## Blocking rules

`npm run check:rust-contracts` fails when:

1. a registered Tauri command cannot be parsed into a command contract;
2. active command coverage differs from the registered handler set;
3. a literal frontend `invoke()` object uses argument keys that do not match Tauri's wire contract;
4. an observed frontend `listen()` has no verified returned cleanup/unlisten path;
5. generated output differs from a fresh regeneration in the same run.

The normal `npm run check` includes this gate.

## Honest partial boundaries

The generator intentionally emits `unknown` and records `UNVERIFIED/PARTIAL` when structural equivalence cannot be proven safely. Current examples include:

- same short Rust type name defined in more than one module;
- externally/internally tagged Serde enums carrying variant payloads that are not yet normalized by the generator;
- Tauri `Channel<T>`, which is IPC transport semantics rather than a plain JSON DTO;
- TypeScript aliases whose structural equivalence to a Rust DTO has not been compiled/proven.

These are not counted as PASS.

## Event governance

The report joins:

`Rust literal emit -> event name -> Rust payload observation -> TS listener type -> cleanup status`

A listener must expose a cleanup/unlisten route. Events with complex payloads remain PARTIAL until their payload structure is machine-proven.

## Resource lifecycle graph

The generator does not copy lifecycle truth. It projects the existing Native Physical Boundary Matrix into the generated report, including all 26 AppState fields and their unique owner/classification/sink, then joins command resource semantics from `native-commands.yaml`.

## Capability impact analysis

Before changing a Rust command, run:

```bash
npm run analyze:native-impact -- --command <command_name>
```

Example:

```bash
npm run analyze:native-impact -- --command db_query
```

The output shows the Rust signature, owner/resource/permission, bridge wrapper, observed frontend files, and affected frontend Capabilities. This is an impact hint derived from real call sites; dynamic calls that static extraction cannot see remain explicitly outside the proof boundary.

## Known proof boundary

This system upgrades BrowserOS from observation-only semantics to generated cross-layer contracts, but it does not claim universal Rust/Serde <-> TypeScript structural equivalence. Unsupported edges stay visible as debt instead of being coerced into green status.
