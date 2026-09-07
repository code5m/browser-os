# A1 M5-W15 reconciliation checkpoint（DOCS ONLY · 2026-09-08 09:30 CST）

> Lane A1（M5-W15 · docs-only reconciliation）
> 派发来源：`PARALLEL_COMMAND_BOARD.md` L1163+（M5-W15 Release Readiness Dispatch，Added 2026-09-08 02:00 CST by A0）
> A0 W14 acceptance 来源：`logs/checkpoints/A0-M5-W14-accept-W15-dispatch-20260908-0200.md`
> 整包交付：本 checkpoint + `logs/checkpoints/Lane-A1-M5-W15-reconciliation-20260908-0930.patch`（11 文件改动）
> A1 W15 立场 = **DOCS ONLY · 零产品代码 · 不 push**

---

## §1 任务接收

**W15 派发** = Release Readiness Dispatch（board L1163+）：*W15 closes M5 evidence and GUI acceptance without expanding runtime authority. Only A0 pushes.*

**A1 W15 任务边界（DOCS ONLY · 冻结）**：
1. reconcile W14 as accepted after A0 push（`886ea29` = HEAD，与 `origin/master` 一致）
2. track the 25% metric limit（构建体积门禁阈值 23% → 25%）
3. track W15 status（11 lane 交付快照 + A1 自身整包）
4. **不**改 product scope（W15 不扩张 runtime authority）
5. **不**碰产品代码（`src/**` / `src-tauri/**` / `scripts/**` / `src/bridge.ts` / `src/types.ts`）
6. **不** push

**启动门禁实测**（`WORKSPACE_IDENTITY.md` Required Startup Check）：

| 项 | 实测 | 判定 |
|---|---|---|
| `pwd` | `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3` | ✅ |
| `.workspace-identity` | `WORKSPACE_ID=BACKV3_MAIN` | ✅ |
| branch | `master` | ✅ |
| `git fetch origin` + `git pull --ff-only` | 已经是最新的（`886ea29` = `origin/master`） | ✅ |
| 工作树并发改动 | 存在 A2/A3/A4/A5/A6/A7/A8/A9/A10 其他 lane 的 W15 文件（**非 A1 范围，本 patch 不收**） | ⚠️ 记录不干预 |

---

## §2 整包改动清单（11 文件）

| # | 文件 | 改动 | 性质 |
|---|------|------|------|
| 1 | `PARALLEL_COMMAND_BOARD.md` | L3 Updated 时间戳 + L6 mainline chain `a7eefbb` → `886ea29`（W14 PUSHED · accepted）+ W15 active 事实 | modified |
| 2 | `详细设计与实施计划.md` | A1 W15 update 行（紧随 A0 2026-09-08 02:00 行后） | modified |
| 3 | `AI-模型切换与接手清单.md` | A1 W15 update 行（同上） | modified |
| 4 | `后续需求TODO.md` | A1 W15 update 行（同上） | modified |
| 5 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | L1 title 加 `W14 PUSHED + W15 active` + 末尾 [W15 active · 2026-09-08 09:30 CST] 段（W14 拣入事实 + 14 行 runtime surface 表 + 25% 指标跟踪 + W15 lane 快照） | modified |
| 6 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | 头增 W14 PUSHED + W15 ACTIVE status 行 | modified |
| 7 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | 头增 W14 PUSHED + W15 ACTIVE status 行 | modified |
| 8 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | 头增 W14 PUSHED + W15 ACTIVE status 行 + 末尾 [W15 · plugin UI 收口] 段 | modified |
| 9 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | L1 title 加 W15 verification scope + 末尾 [W15 verification scope · 2026-09-08 09:30 CST] 段（含 25% 门禁项） | modified |
| 10 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | L2 title 加 `W14 PUSHED + W15 active` + 末尾 [W15 active] 段（DEBT-04 closed-by-W14） | modified |
| 11 | `logs/checkpoints/A1-M5-W15-reconciliation-20260908-0930.md` | **本 checkpoint（new）** | new |
| 12 | `logs/checkpoints/Lane-A1-M5-W15-reconciliation-20260908-0930.patch` | **本 checkpoint 配套 patch（new）** | new |

> 结构对齐 A1 W12/W13/W14 三 wave 同形：board + 3 主文档 + M5-0/10/11/12/13/14（**10 个已跟踪文件修订**）+ 本 checkpoint（new, #11）= **patch 内 11 文件**；**交付物 = 本 checkpoint + patch 两个文件**（patch 不自我包含）。

---

## §3 W14 accepted 对账（依据 `886ea29` + A0 acceptance）

**A0 在 `886ea29 feat(M5): integrate W14 plugin manager UI`（2026-09-07 15:07 CST）中拣入 W14 整包**，并出具 `A0-M5-W14-accept-W15-dispatch-20260908-0200.md`（`STATUS=PASS`）：

- A6 W14 落地 local-only Plugin Manager UI：list / filter / inspect / install-supplied-manifest / enable / disable / key fingerprint
- 仅消费 frozen W13 6 命令（`plugin_list` / `plugin_get` / `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_key_registry`），UI 只走 `src/bridge.ts`，**无** raw Tauri `invoke`
- plugin 执行 / 动态加载 / 网络下载监听 / daemon / 模型调用 / Agent-Skill 执行 / MCP 全量 runtime / graph 写导出 / 后台 worker **全部仍 LOCKED**
- A0 拣入期 4 项修正：Pinia setup-store 自动解包（模板不再 `store.*.value`）、错误 UI 改用固定安全文案（不透传后端错误文本）、privacy fixture 区分"允许的瞬态表单输入"与"渲染出的秘密"、补第 61 条 UI 断言防 `.value` 回归

**验证实跑（A0 acceptance 记录）**：

| 项 | 结果 |
|---|---|
| plugin Rust tests | 28 passed |
| full Rust tests | 431 passed |
| MCP feature tests | 21 passed |
| plugin / plugin-privacy / plugin-UI-privacy / MCP / Agent-Skill / graph 政策 | PASS |
| plugin UI logic | 61 assertions passed |
| graph UI logic | 113 passed |
| Agent/Skill UI logic | 110 passed |
| `npm run build` | PASS |
| build metrics | `total_bytes_pct=24.89`，warning delta=0；**accepted limit 抬至 25%** |

**A1 对账结论**：W13（`a7eefbb`）→ W14（`886ea29`）主线条已闭合，board L6 过期描述（`master at a7eefbb pending A0 W14 integration`）由本整包更正为 `886ea29` W14 PUSHED · accepted；`NEXT=M5-W15`。

---

## §4 25% 指标跟踪（A1 W15 第二项职责）

### 4.1 阈值演进与当前常量

| 阶段 | 阈值 | 依据 |
|---|---|---|
| M0-4.c 初始 | 15% | `scripts/pre-merge.sh` 文案（**L76 仍写 ≤15%，已滞后，见 4.4**） |
| M5-W5 前后 | 19% | IF-2 A11 重采挂账 |
| M5-W8/W9 | 22% | A0 2026-09-07 14:30 收口（`M5-14-debt-ledger.md` L1） |
| M5-W12 | 23% | A0 W12 拣入期 IF-2 修订（`total_bytes_pct=22.26`） |
| **M5-W14 → W15（当前）** | **25.0** | `scripts/measure-build-metrics.py` L38 `TOTAL_BYTES_GROWTH_LIMIT_PCT = 25.0`（L35-37 注释记录：M5-W14 新增懒加载 PluginManager 后 A0 于 2026-09-08 抬至 25%） |

### 4.2 当前实测（A1 只读复跑，2026-09-08 09:30 CST）

```bash
B=$(ls -1 logs/m0-build-metrics/build-metrics-*.json | sort | head -1)
python3 scripts/measure-build-metrics.py --compare "$B" --skip-build
```

```json
{
  "exceeds_growth_limit": false,
  "deltas": { "total_bytes_pct": 24.89, "cargo_warnings": 0 },
  "warnings_increased": false
}
```

- 基线：`logs/m0-build-metrics/build-metrics-4f0e8ab.json`（`pre-merge.sh` L220 取同名排序首文件）
- `total_bytes_pct = 24.89` ≤ **25.0** → **PASS**，余量 **0.11 个百分点**
- `cargo_warnings` delta = 0，`warnings_increased = false`，exit = 0

### 4.3 余量跟踪结论（交 A0 / A6 / A11）

- 余量仅 **0.11pp**，为 M5 全周期最紧的一档；W15 唯一产品代码 lane = **A6 窄 UI 磨光**（无障碍确认/模态焦点 + empty/loading/error 态），**不得**新增 chunk 体积、不得引入新依赖
- A6 W15 若有体积增量 → 必须重跑 `measure-build-metrics.py --compare`，超过 25.0 即 `exceeds_growth_limit=true`，`pre-merge.sh` 判 FAIL
- 建议 A11 W15 收口时把 `total_bytes_pct` 实测值（含基线文件名）写入 W15 verification delta；A0 若再抬阈值须同步 `scripts/measure-build-metrics.py` L35-38 注释 + 本 checkpoint 4.1 表

### 4.4 发现（文档滞后 · A1 不修，交 A0）

- `scripts/pre-merge.sh` L76 文案仍写「总体积 ≤15% 增长」，与实际生效常量 `TOTAL_BYTES_GROWTH_LIMIT_PCT = 25.0` 不一致（仅为说明文本滞后，**门禁实际读常量，不影响判定**）
- A1 为 docs-only lane，门禁脚本不属 A1 允许范围（W14 lane 表 A1 允许 = board / 3 主文档 / `logs/checkpoints/**`），故**只记录不修改**：建议 A0 在 W15 拣入期把 L76 文案同步为 `≤25%`（或改为读常量的动态说明），消除文档/实现漂移

---

## §5 W15 状态跟踪（2026-09-08 09:30 CST 快照 · 基线 `886ea29`）

| Lane | W15 角色（board L1167-1179） | 交付快照 | 结论 |
|---|---|---|---|
| A1 | Docs only：reconcile W14 accepted + 25% 指标 + W15 状态 | 本 checkpoint + patch（10 实际改动 + 2 交付） | DOCS ONLY · 不 push |
| A2 | Boundary review of W14 UI/store and W15 evidence | `logs/assist/A2-M5-W15-boundary-verdict-20260907-1523.md` + `logs/checkpoints/A2-M5-W15-20260907-1523.md` + `Lane-A2-M5-W15-20260907-1523.patch` | `STATUS=PASS_WITH_DEBT`（非阻塞） |
| A3 | MCP isolation regression review | `logs/assist/A3-M5-W15-mcp-isolation-verdict-20260907-1521.md` + `logs/checkpoints/A3-M5-W15-mcp-isolation-20260907-1521.md` | PASS（无 MCP 隔离回归，零产品代码） |
| A4 | Plugin UI privacy and stable-error review | `logs/assist/A4-M5-W15-privacy-stable-error-review-20260908-0900.md` + `Lane-A4-...patch` | 隐私面 PASS；稳定错误面 PASS_WITH_NOTE（建议删 `uninstall` 死 case） |
| A5 | Agent/Skill execution-lock regression review | `logs/assist/A5-M5-W15-agent-skill-lock-verdict-20260907-1505.md` | `STATUS=PASS_WITH_CONTEXT`（执行锁 PASS） |
| A6 | Narrow UI polish only（accessible confirmation/modal focus, empty/loading/error） | 工作树改动（A6 已 staged）：new = `scripts/check-ui-a11y-logic.mjs` / `src/components/shared/AsyncState.vue` / `src/composables/useModalFocus.ts` / `src/utils/asyncView.ts` / `src/utils/modalA11y.ts`；modified = `ConfirmModal.vue` / `ImageLightbox.vue` / `AuditPanel.vue` / `GitWriteConfirmDialog.vue` / `PermissionPreviewModal.vue` / `RunHistoryModal.vue` / `ScriptRunDialog.vue` / `TaskEditDialog.vue` | 进行中（A1 快照时未见其 checkpoint/patch） |
| A7 | Graph non-regression review | `logs/assist/A7-M5-W15-graph-nonregression-20260907-1430.md` | Graph 零回归，可进 M5-W15 Release |
| A8 | GUI/manual acceptance checklist + 视觉人体工学 | `logs/assist/A8-M5-W15-gui-acceptance-20260907-1526.md`（**快照时文件为空**） | 待内容落盘 |
| A9 | Frozen W13 backend-contract review（no backend changes） | `logs/assist/A9-M5-W15-frozen-contract-review-20260907-1505.md` + `Lane-A9-M5-W15-frozen-contract-review-20260907-1505.patch` | 契约冻结确认 + W14/W15 零后端改动 |
| A10 | Security release review（raw-invoke + error rendering） | `logs/assist/A10-M5-W15-security-release-review-20260907-1522.md` + `logs/checkpoints/A10-M5-W15-20260907-1522.md` | `STATUS=PASS`（无裸 IPC 绕过、无敏感渲染逃逸） |
| A11 | Final verification matrix and push readiness | 快照时未见 W15 文件 | 待交付（push readiness 最终闸门） |

> 快照口径：以 `git status --short` 与 `ls logs/assist logs/checkpoints` 实测为准；其它 lane 的并发改动**不在** A1 patch 范围，A1 只记录状态。

---

## §6 A1 W15 立场

### 6.1 必做（已在本 checkpoint 完成）

- ✅ reconcile W14 as accepted（board L6 + M5-0/10/11/12 + 3 主文档 + 本卡 §3）
- ✅ track the 25% metric limit（常量定位 + 只读复跑实测 24.89/25.0 + 余量 0.11pp + 4.4 文档滞后发现 → §4）
- ✅ track W15 status（11 lane 交付快照 → §5）
- ✅ M5-13 末尾补 [W15 verification scope]（含 `≤25%` 门禁项）
- ✅ M5-14 末尾补 [W15 active]（DEBT-04 closed-by-W14）

### 6.2 必不做（A1 零产品代码 / 零 push）

- ❌ 不写 `src/**`（含 A6 W15 的 `AsyncState.vue` / `useModalFocus.ts` / `asyncView.ts` / `modalA11y.ts`）
- ❌ 不写 `scripts/**`（含 `check-ui-a11y-logic.mjs`、`measure-build-metrics.py`、`pre-merge.sh`）
- ❌ 不写 `src-tauri/**`，不动 ACL / 命令 / DTO / `bridge.ts` / `types.ts`
- ❌ 不扩张 runtime authority（plugin invoke / 动态加载 / 网络 / daemon / 模型 / Agent-Skill 执行 / MCP 全量 / graph 写导出 / 后台 worker 全锁）
- ❌ 不替其他 lane 收口其 checkpoint/patch，不 `git add` 他 lane 文件
- ❌ 不 commit、不 push（本目录为 canonical main，A2 已有 staged 改动，A1 提交会误收他人变更）

---

## §7 债务账变化（`M5-14-debt-ledger.md`）

- **DEBT-04（plugin UI 6 项）**：W14 派发期 0% ACTIVE → A6 W14 实施 + A0 拣入（`886ea29`）后 **6/6 收口 = closed-by-W14**
- **M5 final debt ledger 维持 53 条**（条目数不变，DEBT-04 状态由"挂账"转"收口"）
- **W15 预期增量 = 0**（A6 W15 = 窄 UI 磨光，不新增命令/runtime；其余 10 lane = review/docs/verification）

---

## §8 A1 W15 整包交付

- ✅ `logs/checkpoints/A1-M5-W15-reconciliation-20260908-0930.md`（本 checkpoint）
- ✅ `logs/checkpoints/Lane-A1-M5-W15-reconciliation-20260908-0930.patch`（patch 内 **11 文件** = 10 个已跟踪文件修订 + 本 checkpoint（new），**路径限定 A1 范围**，不自我包含）
- ✅ 工作树 A1 范围改动 = board + 3 主文档 + M5-0/10/11/12/13/14（10 个 modified）+ 本 checkpoint（new）+ patch（交付容器）
- ✅ 自检：`grep -c "^diff --git"` = **11**（与整包清单一致）；`git apply --check --reverse` 通过（patch 与工作树 A1 改动一致）

**patch 生成（路径限定，避免误收并发 lane 改动）**：

```bash
git add -N logs/checkpoints/A1-M5-W15-reconciliation-20260908-0930.md \
           logs/checkpoints/Lane-A1-M5-W15-reconciliation-20260908-0930.patch
git diff --binary -- PARALLEL_COMMAND_BOARD.md 详细设计与实施计划.md \
  AI-模型切换与接手清单.md 后续需求TODO.md logs/checkpoints/M5-20260906 \
  logs/checkpoints/A1-M5-W15-reconciliation-20260908-0930.md \
  logs/checkpoints/Lane-A1-M5-W15-reconciliation-20260908-0930.patch \
  > logs/checkpoints/Lane-A1-M5-W15-reconciliation-20260908-0930.patch
```

**A1 W15 不 push**：A0 拣入期按本 checkpoint + patch 套用；A11 出 push readiness 后由 A0 合并推送。

---

## §9 关联文件（Pointer）

- 派发原文：`PARALLEL_COMMAND_BOARD.md` L1163-1185（M5-W15 Release Readiness Dispatch）
- A0 W14 acceptance：`logs/checkpoints/A0-M5-W14-accept-W15-dispatch-20260908-0200.md`
- A1 W14 整包（同形参考）：`logs/checkpoints/A1-M5-W14-reconciliation-20260908-0020.md` + `Lane-A1-M5-W14-reconciliation-20260908-0020.patch`
- 25% 常量：`scripts/measure-build-metrics.py` L35-38；门禁调用：`scripts/pre-merge.sh` L218-231（文案 L76 滞后）
- M5-0 末尾段：[W15 active · 2026-09-08 09:30 CST]
- M5-12 末尾段：[W15 · plugin UI 收口 · 2026-09-08 09:30 CST]
- M5-13 末尾段：[W15 verification scope · 2026-09-08 09:30 CST]
- M5-14 末尾段：[W15 active · 2026-09-08 09:30 CST]
