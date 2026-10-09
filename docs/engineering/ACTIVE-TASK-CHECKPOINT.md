# ACTIVE TASK CHECKPOINT — BrowserOS security + Rust semantic closure

Status: ACTIVE

## Recovery source
- Repository: code5m/browser-os
- Branch: feature/native-semantic-security-debt-closure
- Master at task start: dc36d159d60467a7a760c37e6ec19f7b362db138
- Last validated feature SHA before this repair: 524c9e4ef77fbd75273d10265f6083abe21d5f8b
- Stable tag must remain: capability-platform-v4-stable -> a6ff92674db98ffad964b784167fdb8c9a98f3cc

## Goal
1. Remove historical production npm advisory allowlist with real compatible security upgrades.
2. Extend Rust semantics with source-derived events / active IPC observations while explicitly keeping structural Rust Serde <-> TypeScript equivalence UNVERIFIED where no executable schema proof exists.
3. Preserve all existing build/security/GUI gates; no threshold or baseline relaxation.

## Completed before this checkpoint
- Supply Chain strict production npm audit is green on the security-upgraded lockfile.
- package-lock security upgrades observed: Vue family 3.5.43, nanoid 3.3.20, PostCSS 8.5.29, source-map-js 1.2.2.
- Rust runtime semantic checker and negative fixtures are integrated.
- Engineering Governance / UI Safety / Hot-Plug / Supply Chain passed at 524c9e4e.
- Full Validation passed npm checks, Rust fmt/check/tests, packaged GUI cold start, Full Tauri GUI regression and diff whitespace.

## Current blockers at 524c9e4e
- pre-merge build metrics: rounded total_bytes_pct=25.2 but actual value is just over the frozen 25.2% limit.
- three historical domain checkers reject the legitimate package-lock security remediation because they byte-hash the whole lockfile:
  - check-image-preview-policy.py
  - check-script-domain-policy.py
  - check-command-domain-policy.py

## Repair in progress
- Replace lockfile byte hash protection with root direct-dependency consistency; package.json direct dependency declarations remain frozen.
- Supply Chain Assurance remains responsible for transitive advisory/SBOM verification.
- Then minimize secure Vue resolution if needed to recover build budget; do not raise 25.2% limit or modify build-metrics baseline.

## Completion conditions
- feature: all mandatory workflows PASS and PRE_MERGE_RESULT=ALL_PASS
- PR merged without human review if repository protection permits
- master: all mandatory workflows PASS
- stable v4 tag unchanged
- checkpoint updated to FINAL/ARCHIVED with evidence links
