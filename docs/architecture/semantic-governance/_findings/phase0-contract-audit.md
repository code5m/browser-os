# Phase 0 — Agent F：Contract Auditor

日期：2026-09-17
读源：00-EXECUTIVE-SUMMARY / 02-STATE-SOURCES / 08-DUPLICATE-SEMANTICS / 10-TARGET-SEMANTIC-CONTRACTS
/ 11-CHECKER-DESIGN / 12-MIGRATION-PLAN / 13-ACCEPTANCE-MATRIX / 14-DECISIONS
/ 18-SECOND-REVIEW-SYNTHESIS / 19-SECOND-REVIEW-EVIDENCE + A–E 结果。

## 审计目标

验证 Phase 0 最终没有：
1. 把错误规则固化（bake wrong rule）
2. 把 Target 当 Current（Target-as-Current）
3. 把未接线 Checker 写成 Active
4. 把失败 Checker 写成 PASS

## 逐项核对

### (1) 错误规则固化 —— 否

- Grid checker G2：修复后断言 `isBrowserVisible = mainView === "browser"`，与 CURRENT 源码
  （useBrowserStore.ts:149-151）一致；并显式守卫「不得重新引入 gridOpen 旧耦合」（DUP-004 指出的
  stale 公式已纠正，不再冻结错误语义）。
- Grid checker G1#4：由 `schedulePosition()`（源码已无）改为 `relocate()`（closeGridAll 实际调用，
  useBrowserStore.ts:509）。不再把不存在的符号当不变量。
- IPC checker：去掉「第一个 generate_handler! 块」的片面假设，改为全部块；去掉「只认普通 invoke」
  的盲区，改为同时认泛型 `invoke<Type>()`。无错误规则被固化。

### (2) Target-as-Current —— 否

- Grid checker 编码的是 CURRENT（`mainView==='browser'`），未编码 DUP-002/004 建议的 TARGET
  `gridVisible = gridOpen && mainView==='grid'`。Phase 1 意图 API（exitGrid / gridVisible）未设计、未引入。
- IPC checker 编码三源真实交集，未假设未来命令集。

### (3) 未接线 Checker 写成 Active —— 否

- `check-command-set-consistency.py` 在 GATE INVENTORY 中明确标为 READY_NOT_GATED，未谎称为 Active。
- `check-grid-close-logic.mjs` 真实接入 npm run check + pre-merge（已改 package.json / pre-merge.sh），
  属真实接线，非文档声称。

### (4) 失败 Checker 写成 PASS —— 否

- check-ui.mjs 因 FilePanel 真实产品债 FAIL，审计报告如实记录 REPOSITORY_POLICY_GATE = FAIL，
  未写作 ALL PASS（见 §17/§18）。
- check-command-set-consistency.py 因真实 `move_path` ACL 缺口 FAIL，如实报告，
  未用 allow-list 洗绿（move_path 未加入 KNOWN，缺口作为真实缺陷记录并路由到例外流程）。
- 两个 checker 的自测均 PASS（COMMAND_SET_SELF_TEST: ALL_PASS / GRID_CLOSE_SELF_TEST: ALL_PASS），
  但自测通过 ≠ 仓库策略 PASS，二者区分清晰。

## 结论

Phase 0 没有把错误规则固化、没有把 Target 当 Current、没有把未接线 Checker 写成 Active、
没有把失败 Checker 写成 PASS。契约审计通过。
