# 14 — RELEASE CHECKLIST（Capability Platform vNext Hardening）

状态：**CODE PASS 达成**；停在 `READY_FOR_FINAL_HUMAN_ACCEPTANCE = YES`，等待人工验收。

## A. 已满足（自动证据可复跑）

- [x] `npm run check` → EXIT=0（含新接入的 native 边界 + contract drift 门禁）
- [x] `npm run build` → PASS
- [x] `check-capability-platform` → 29 PASS / 0 FAIL
- [x] `check-capability-boundaries` → PASS（fail=0）
- [x] `check-capability-composition` → 33/33；`check-composition-profiles` → 11/11
- [x] `check-capability-registry` → PASS（fail=0）
- [x] `check-capability-contract-drift` → **fail=0**（已接入 `npm run check`）
- [x] `check-native-capability-boundaries` → PASS（148/148 owned，UNKNOWN=0；自检 ALL_PASS）
- [x] `check-semantic-registry` / `closure` / `sensitive-side-effects` → PASS / 27-27 / fail=0
- [x] `runtime-resource-absence` → 12/12（framework grid-child=0、PTY=0）
- [x] `check-ui-boundaries` → PASS（fail=0, vacuous=0）；home 三件套 PASS
- [x] `git diff --check` 干净；`git fsck --full` 无损坏
- [x] CARGO = **NOT_REQUIRED**（本轮未改 Rust：`git diff --name-only HEAD` 中 `.rs`/`src-tauri/` = 0）
- [x] SECOND_TRUTHS = 0；INTERNAL_CROSS_CAPABILITY_IMPORTS = 0；ILLEGAL_CYCLES = 0
- [x] BLOCKING_DEBT = 0；NEW_REGRESSION = 0
- [x] 未 push、未 merge master、未改系统安装、未改用户数据

## B. 待人工（**未完成，不得跳过**）

- [ ] `FINAL-HUMAN-ACCEPTANCE.md` H01–H10（既有文档）+ H11–H15（本轮补齐）逐项执行
- [ ] 人工目视（HUMAN_VISUAL）通过后才可考虑 **Human PASS**

## C. Tag 纪律

- [x] `capability-platform-vnext-hardening-code-pass`（annotated，已创建于 `8e33fde`）
- [x] **未移动** `capability-platform-vnext-code-pass`
- [x] **未创建** `capability-platform-vnext-pass`（Human Acceptance 未执行）
- [ ] Human Acceptance 通过后（人工决定）才允许创建 Human PASS tag

## D. 禁止项

- 禁止 push / merge master / force / 移动历史 tag
- 禁止为清 BLOCKING 而降低 checker、扩大 allowlist、无据改 baseline
- 禁止把 DECLARED 资源结论写成 MEASURED，禁止冒充 HUMAN_VISUAL PASS
