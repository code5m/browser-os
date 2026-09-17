# Phase 0 — Agent D：Gate Wiring / GATE INVENTORY

日期：2026-09-17
原则：只有 READY（可靠、无已知误报、自测通过、证据充分）的 Checker 才能接线。
严格禁止：checker 本身还有误报 → 先接 pre-merge → 再慢慢修。

## GATE INVENTORY

| Checker | Exists | Self-test | Reliable | Wired | Default result (current repo) | Blocking |
|---|---|---|---|---|---|---|
| check-architecture.mjs | ✓ | — | ✓ | npm run check + pre-merge | 视仓库 | blocking |
| check-ui.mjs | ✓ | ✓ | ✓ | npm run check + pre-merge | **FAIL**（FilePanel fixed overlay，产品债） | blocking |
| check-native.mjs | ✓ | — | ✓ | npm run check + pre-merge | 视仓库 | blocking |
| check-browser-runtime.mjs | ✓ | — | ✓ | npm run check + pre-merge | 视仓库 | blocking |
| check-task-boundary.mjs | ✓ | — | ✓ | npm run check + pre-merge | 视仓库 | blocking |
| doctor.mjs | ✓ | — | ✓ | pre-merge | 视仓库 | blocking |
| check-native-webview-overlay.mjs | ✓ | — | ✓ | pre-merge (L372) | 视仓库 | blocking |
| check-ui-thread-blocking.py | ✓ | ✓ | ✓ | pre-merge (L183) | 视仓库 | blocking |
| check-grid-close-logic.mjs | ✓ | ✓ | ✓ | **本 Phase 0 新接** npm run check + pre-merge | PASS（修复后） | blocking |
| check-command-set-consistency.py | ✓ | ✓ | ✓ | **未接 blocking gate** | FAIL（move_path ACL 缺口，真实缺陷，待例外流程） | informational / pending |
| check-graph-policy.py | ✓ | ✓ | ✓ | pre-merge（run_pre_merge + run_self_test） | 视仓库 | blocking |
| 其余 ~40 个 checker | ✓ | 部分 | 部分 | 各自策略 | 视实现 | 各自 |

## 分类（本 Phase 0 关注项）

- READY_AND_GATED：check-grid-close-logic.mjs（本阶段修复 + 自测后接线）。
- READY_NOT_GATED：check-command-set-consistency.py（可靠，但当前报出真实 `move_path` ACL 缺口；
  按 §例外规则，该缺口须走「证据→Reviewer→Chief→独立 commit」流程修复 ACL，修复后再将其纳入
  blocking gate。现阶段仅作为可运行审计工具 `--report` 使用，不阻塞合并）。
- IMPLEMENTED_UNRELIABLE→已修复：
  - check-command-set-consistency.py 旧版只取第一个 generate_handler! 块 + 漏掉泛型 invoke → 大量假阳性
    "stale ACL"；现修复，无假阳性。
  - check-grid-close-logic.mjs 旧版 G1#4 查 `schedulePosition()`（源码已改用 `relocate()`）、G2 公式冻结错误
    语义 `!gridOpen && mainView==='browser'`（CURRENT 实为 `mainView==='browser'`）→ 旧版对正确源码 RED；
    现修复，对正确源码 GREEN。
- REJECTED：无。
- PLANNED：将 move_path ACL 缺口修复后，把 check-command-set-consistency.py 接入 blocking gate。

## Gate Graph（npm run check）

```
npm run check
  → check-architecture.mjs   (blocking)
  → check-ui.mjs             (blocking)   ← 当前 FAIL（FilePanel 产品债）
  → check-native.mjs         (blocking)
  → check-browser-runtime.mjs(blocking)
  → check-task-boundary.mjs  (blocking)
  → check-grid-close-logic.mjs (blocking, 本 Phase 0 新增, 现 PASS)
```

pre-merge.sh Phase 03 loop：
`check-architecture check-ui check-native check-browser-runtime check-task-boundary check-grid-close doctor`
（check-grid-close 为本 Phase 0 新增；check-command-set-consistency.py 未加入 blocking loop。）

## 接线变更（本 Phase 0）

1. `package.json` scripts.check 追加 `&& node scripts/check-grid-close-logic.mjs`。
2. `pre-merge.sh` Phase 03 loop 追加 `check-grid-close`。

均仅接入已验证可靠且当前仓库 PASS 的 checker，未将仍报真实缺陷的 IPC checker 接入阻塞门。
