# ACTIVE TASK CHECKPOINT — Rust Cross-Layer Contract Final Closure

Status: ACTIVE

## Repository
- Repository: code5m/browser-os
- Branch: feature/rust-crosslayer-contract-final-closure
- Base master: 48b092c396f5e4dfa783e649636c20c34a813006
- Stable tag must remain: capability-platform-v4-stable -> a6ff92674db98ffad964b784167fdb8c9a98f3cc

## One-pass goals
1. Rust -> TypeScript generated command/data contracts from real Rust/Serde sources.
2. Tauri Command argument/return drift tests.
3. Event name/payload/listener contract governance.
4. Resource ownership/lifecycle semantic graph linked to the existing Native Physical Boundary authority.
5. Capability impact analysis for native command changes.

## Design decision
Start with a repository-local code-derived generator and checker, with no new runtime dependency. It must derive from Rust + existing machine registries, emit generated TypeScript/JSON, and mark unsupported dynamic/custom serde edges PARTIAL/UNVERIFIED rather than guessing.

## Completion rules
- No second handwritten semantic truth source.
- No weakening supply-chain, build metrics, GUI, native boundary, or pre-merge gates.
- Positive/negative fixtures required.
- Feature + PR + master mandatory workflows green.
- Stable tag unchanged.
