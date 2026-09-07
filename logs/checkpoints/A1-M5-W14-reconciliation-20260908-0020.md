# A1 M5-W14 reconciliation checkpoint（START DOCS ONLY · 2026-09-08 00:20 CST）

> Lane A1（M5-W14 · docs-only reconciliation）
> 派发来源：`PARALLEL_COMMAND_BOARD.md` L1163+（M5-W14 Plugin Manager UI Dispatch，Added 2026-09-08 00:20 CST by A0）
> A0 W13 acceptance 来源：`logs/checkpoints/A0-M5-W13-accept-W14-dispatch-20260908-0020.md`
> 整包交付：本 checkpoint + `logs/checkpoints/Lane-A1-M5-W14-reconciliation-20260908-0020.patch`（11 文件改动）
> A1 W14 立场 = **START DOCS ONLY · 零产品代码 · 不 push**

---

## §1 任务接收

**W14 派发** = Plugin Manager UI Dispatch（board L1163+）；**W14 = 仅 1 lane UI 产品代码 open**（**A6 W14 = START PRODUCT CODE NARROW**），其余 10 lane = docs/review/security/verification（A1/A2/A3/A4/A5/A7/A8/A9/A10/A11 + A19 plugin UI 仍 SUPPORT DOCS ONLY）。

**A1 W14 任务边界（START DOCS ONLY · 冻结）**：
1. reconcile W13 as accepted after A0 push（`a7eefbb` = HEAD）
2. mark W14 active + 更新 M5-12/13/14 status + 末尾段
3. **不**改 product scope（plugin UI 6 项 ACTIVE 仍 0%，A6 W14 实施期才升 ACTIVE = DEBT-04 W14+ 仍挂账）
4. **不**碰产品代码（`src/components/plugin/**` / `src/stores/usePluginStore.ts` / `scripts/check-plugin-ui-logic.mjs` / `src/bridge.ts` / `src/types.ts`）
5. **不** push

---

## §2 整包改动清单（11 文件）

| # | 文件 | 改动 | 行数 |
|---|------|------|------|
| 1 | `PARALLEL_COMMAND_BOARD.md` | L6 mainline chain 更新 `3c3f460` → `a7eefbb`（W13 PUSHED · 集成） | 1 替换 |
| 2 | `详细设计与实施计划.md` | L1 加 A1 W14 update 行 | 1 新增 |
| 3 | `AI-模型切换与接手清单.md` | L1 加 A1 W14 update 行 | 1 新增 |
| 4 | `后续需求TODO.md` | L1 加 A1 W14 update 行 | 1 新增 |
| 5 | `logs/checkpoints/A1-M5-W14-reconciliation-20260908-0020.md` | **本 checkpoint（new）** | 165 |
| 6 | `logs/checkpoints/Lane-A1-M5-W14-reconciliation-20260908-0020.patch` | **本 checkpoint 配套 patch（new）** | 313 |
| 7 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | L1 title + 末尾 [W13 PUSHED · 2026-09-08 00:20 CST] 段 + [W14 active · 2026-09-08 00:20 CST] 段（含 runtime-lock 14 行状态表） | 68 +/2 - |
| 8 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | 头 L17-19 增 W13 PUSHED + W14 ACTIVE status 行 | 2 新增 |
| 9 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | 头 L17-19 增 W13 PUSHED + W14 ACTIVE status 行 | 2 新增 |
| 10 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | 头 L18-20 增 W13 PUSHED + W14 ACTIVE status 行 + 末尾 [W14 active · 2026-09-08 00:20 CST] 段 | 35 +/0 - |
| 11 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | L1 title 加 W14 verification scope 关键词 + 末尾 [W14 verification scope · 2026-09-08 00:20 CST] 段（11 项必跑验证矩阵） | 32 +/2 - |
| 12 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | L1 title + 头 status 行 W13 PUSHED + W14 ACTIVE + 末尾 [W14 active · 2026-09-08 00:20 CST] 段（runtime surface 14 行表 + DEBT-04 仍挂账转 A6 W14） | 44 +/2 - |

> 备注：上表 12 行（含本 checkpoint + patch）= **A1 W14 整包 11 个用户可见改动 + 本 checkpoint 文件 + 本 patch 文件 = 11 实际改动文件 + 2 个交付文件**；A1 W13 reconciliation 整包是 14 文件改动（W12 board L7 + 3 主文档 + M5-0/10/11/12/13/14 + A1 W13 checkpoint + A1 W13 patch = 11 实际 + 2 交付），A1 W14 整包对齐 W12/W13 同形（**11 实际改动文件** = board + 3 主文档 + A1 W14 checkpoint + A1 W14 patch + M5-0/10/11/12/13/14）。

---

## §3 W13 拣入事实回填（依据 `a7eefbb`）

**A0 在 `a7eefbb feat(M5): integrate W13 plugin manifest lifecycle` 中拣入 W13 整包**：
- 8 local-only lifecycle/key-registry 命令（`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry` / `plugin_audit_get` / 4 storage stub 错误结构不变）
- 原子注册表持久化 + redacted DTO/audit + ACL/source-check parity
- **无** execution surface（plugin_invoke / cancel / storage_* 5 stub 仍 `Err("not-implemented-in-W6")`）
- 31 files changed（含 4 共享文件修订 + 前端镜像 + 门禁脚本 + 文档）

**验证实跑（a7eefbb 拣入后 W13 batch 验证）**：
- `cargo test plugin` 全绿（411/411 default + 21/0 feature mcp_server + 15/0 graph）
- `check-plugin-policy.py --self-test` PASS（ACTIVE=8/PENDING=0 + `--expect-pending` PASS）
- `check-plugin-privacy.py --self-test` PASS（ACTIVE=4/PENDING=0）
- `check-graph-policy.py` ACTIVE=8
- `check-mcp-policy.py` ACTIVE=12/PENDING=0
- graph UI logic 113/113 + agent-skill UI logic 110/110
- `npm run build` PASS
- `python3 scripts/pre-merge.sh` ALL_PASS
- `python3 scripts/measure-build-metrics.py` `total_bytes_pct ≤ 23%`（W12 A0 修订 IF-2 = 23%）
- cargo_warnings delta = 0

**W13 残留债 = 0**：A9 W13 增量 = 1 closed-by-W13 = plugin stage-I 6 命令；M5 final debt ledger 维持 **53 条** + W13 增量 1 closed-by-W13 → 维持 53 条；DEBT-04 plugin UI 6 项 carried-to-W14+（A6 派发后消解，A1 W14 派发期仍 0% ACTIVE）。

---

## §4 W14 派发边界（依据 board L1163+）

**W14 = Plugin Manager UI Dispatch**（Added 2026-09-08 00:20 CST by A0）；**W14 核心 = 仅 1 lane UI 产品代码 open**：

### 4.1 A6 W14 唯一产品代码 lane 范围（冻结）

- 消费 **frozen W13 6 命令**（`plugin_list` / `plugin_get` / `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_key_registry`）
- **不**实施 W14 新 lifecycle 命令 / **不**改 `src/bridge.ts` DTO 形态 / **不**增删 ACL
- 文件范围 = `src/components/plugin/**` + `src/stores/usePluginStore.ts` + `scripts/check-plugin-ui-logic.mjs` + workspace navigation 集成
- 6 action = list / filter / inspect / install-supplied-manifest / enable / disable / key fingerprint
- UI 走 `src/bridge.ts`（**不** raw Tauri `invoke`）
- **不**渲染 raw signature / public-key / resource path / manifest metadata / credentials / 请求响应 body / stdout / stderr

### 4.2 其余 10 lane 全 review / docs / verification

| Lane | W14 角色 | 必保 |
|------|---------|------|
| A1 | START DOCS ONLY（本卡） | 11 文件整包收口 + 不改 product scope + 不 push |
| A2 | START REVIEW ONLY | boundary verdict = W14 plugin UI 边界 = 消费 frozen W13 6 命令，不越界 |
| A3 | START MCP REVIEW ONLY | isolation verdict = W14 plugin UI 走 bridge.ts 不引入 mcp_* runtime |
| A4 | START PRIVACY REVIEW ONLY | privacy verdict = W14 plugin UI 走 W13 redacted DTO 不触 raw 敏感字段 |
| A5 | START AGENT/SKILL REVIEW ONLY | lock verdict = W14 plugin UI 不触发 Agent/Skill runtime |
| A7 | SUPPORT DOCS ONLY | graph non-regression = W12 8 条 graph 集成锚点 + A6 W14 不触 graph |
| A8 | SUPPORT UI REVIEW ONLY | UI ergonomics + graph/workspace non-regression |
| A9 | SUPPORT BACKEND REVIEW ONLY | A6 W14 是否吃 frozen W13 contract + 缺哪些 read-only 字段 |
| A10 | START SECURITY REVIEW | raw-invoke bypass / confirmation bypass / source-ACL drift / sensitive-render regression 四项不破 |
| A11 | START VERIFICATION | W14 matrix 全跑（`M5-13-verification-matrix.md` 末尾 [W14 verification scope] 段 11 项必跑） |
| A19 | SUPPORT DOCS ONLY | plugin UI 仍 0% ACTIVE 派发期；A6 W14 实施完成 = DEBT-04 closed-by-W14 |

### 4.3 W14 Hard Stops（10 条）

1. 仅 master / pull --ff-only / 仅 A0 push
2. **No** plugin_invoke / 代码执行 / 动态加载 / 网络下载监听 / daemon / 模型调用 / Agent-Skill 执行 / MCP 全量 runtime / graph 写导出 / 后台 worker
3. UI 只能调 `src/bridge.ts`，**不得** raw Tauri `invoke`
4. **不得** display/persist raw signature / public-key material / resource path / manifest metadata / credentials / 请求响应 body / stdout / stderr
5. 现有命令名 + DTO 冻结，**不得** W14 加 lifecycle 命令

---

## §5 A1 W14 = START DOCS ONLY 立场

### 5.1 必做（已在本 checkpoint 完成）

- ✅ reconcile W13 as accepted after A0 push（`a7eefbb`）—— §3
- ✅ mark W14 active —— 3 主文档 L1 + board L6 + M5-0/10/11/12/13/14 头 status 行
- ✅ 更新 M5-12/13/14 status —— M5-12 头增 W13 PUSHED + W14 ACTIVE + 末尾 [W14 active] 段；M5-13 L1 title + 末尾 [W14 verification scope] 段；M5-14 L1 title + 头 status 行 + 末尾 [W14 active] 段
- ✅ 更新 M5-0 history chain + 末尾段 —— M5-0 L1 title + 末尾 [W13 PUSHED] + [W14 active] 段
- ✅ 3 主文档 L1 A1 W14 update 行（与 W12/W13 同形 · 紧随 A1 W13 update 行后）

### 5.2 必不做（A1 零产品代码 / 零 push）

- ❌ 不写 `src/components/plugin/**` 任何文件
- ❌ 不写 `src/stores/usePluginStore.ts`
- ❌ 不写 `scripts/check-plugin-ui-logic.mjs`
- ❌ 不碰 `src/bridge.ts` / `src/types.ts`（DTO 冻结）
- ❌ 不实施 W14 新 lifecycle 命令
- ❌ 不增删 ACL（K1 ACL 末条恒为 `list_artifact_images` 不破）
- ❌ 不修改 product scope（plugin UI 6 项 ACTIVE 仍 0%）
- ❌ 不 push

### 5.3 验证矩阵（`M5-13-verification-matrix.md` 末尾 [W14 verification scope] 段 11 项必跑）

1. `cargo test plugin` 全绿
2. `cargo test graph` 15/15 PASS（A7 W14 GRAPH NON-REGRESSION）
3. full cargo 414/414 PASS
4. `mcp feature test` 21/21 PASS（A3 W14 MCP PLUGIN ISOLATION）
5. `cargo fmt --check` 干净
6. `check-plugin-policy.py` ACTIVE=8/PENDING=0
7. `check-plugin-privacy.py` ACTIVE=4/PENDING=0
8. `check-plugin-ui-logic.mjs` ≥9/9 PASS（A6 W14 实施期新建）
9. `check-graph-ui-logic.mjs` 113/113 PASS
10. `check-agent-skill-ui-logic.mjs` 110/110 PASS
11. `npm run build` PASS + `pre-merge.sh` ALL_PASS + `measure-build-metrics.py` ≤ 23% + cargo_warnings delta = 0

> 备注：A1 W14 派发期不实施任何验证（验证 = A11 实施期 W14 verification delta 收口）；A1 W14 仅交付 11 文件整包文档收口 + 不 push。

---

## §6 债务账变化（`M5-14-debt-ledger.md` §10）

- **W14 增量预期 = 0**（A6 W14 = 复吃 W13 6 命令 + UI 包装，**不**实施 W14 新 lifecycle 命令 / **不**改 DTO / **不**渲染 raw 敏感字段 → 无新债增量）
- **M5 final debt ledger 维持 53 条**（W14 拣入后维持 53 条 + A6 W14 实施期收口 DEBT-04 = 6/6 ACTIVE）
- **DEBT-04 plugin UI 6 项 ACTIVE = 0% → 6/6**（A6 W14 实施期收口；**A1 W14 派发期仍 0%** = 派发期不算 ACTIVE）
- **新增债 0**（W14 不实施 invoke / cancel / storage_* / 网络 / 动态加载 / daemon / 模型 / Agent-Skill / MCP / graph 写 / 后台 worker）

---

## §7 A1 W14 = A1 W13 整包同形（W12 整包简化形）

| Wave | 整包文件数 | 实际改动文件 | A1 立场 |
|------|----------|-------------|---------|
| W12 | 13 | M5-0/7/8/9/10/11/12/13/14 + 3 主文档 + board = 11 实际 + 2 交付 | START DOCS ONLY |
| W13 | 14 | M5-0/10/11/12/13/14 + 3 主文档 + board = 9 实际 + 2 交付 + M5-13 复检段 | START DOCS ONLY |
| W14 | 11 | M5-0/10/11/12/13/14 + 3 主文档 + board = 8 实际 + 2 交付 + M5-13 复检段 | START DOCS ONLY |

> 备注：W12/W13/W14 三 wave 整包结构同形（board + 3 主文档 + A1 checkpoint + A1 patch + M5-0 + M5-12/13/14 status + 末尾段）；W14 与 W13 差异 = M5-0/12/13/14 末尾 [W14 active]/[W14 verification scope] 段为新增；M5-10/11 头 status 行 W14 与 W13 同形（仅 W14 ACTIVE status 行新增）。

---

## §8 A1 W14 整包交付（已就绪）

- ✅ `logs/checkpoints/A1-M5-W14-reconciliation-20260908-0020.md`（本 checkpoint · 165 行）
- ✅ `logs/checkpoints/Lane-A1-M5-W14-reconciliation-20260908-0020.patch`（11 文件改动 · 313 行）
- ✅ 工作树 A1 范围改动：11 文件 modified（含本 checkpoint + patch = 11 实际 + 2 交付）

**A1 W14 整包改动行数**：modified files = 10 个（M5-0/10/11/12/13/14 + 3 主文档 + board）+ 1 new（本 checkpoint）+ 1 new（patch）= **12 文件 / 11 实际改动 + 2 交付**（A1 W14 任务边界 = 11 实际改动 + 2 交付文件 = 13 总文件；与 W12/W13 整包同形）。

**A1 W14 不 push**：本工作树改动仅 A1 范围 = 11 实际 + 2 交付（其他 lane 并发改动 = A2/A4/A7/A8/A9/A10 W14 自身职责范围 = 不是我加的，不归 A1 W14 patch 管）；A0 拣入期由 A0 套用本 checkpoint + patch。

---

## §9 W14 实施期预期（A6 + A11 + A7 + A10）

- A6 W14 实施期：消费 frozen W13 6 命令 + 6 action 全 ACTIVE + `check-plugin-ui-logic.mjs` ≥9/9 PASS
- A11 W14 实施期：W14 verification delta 收口（11 项必跑全 PASS）
- A7 W14 实施期：graph non-regression PASS（W12 8 条 graph 集成锚点 + A6 W14 不触 graph）
- A10 W14 实施期：security review PASS（raw-invoke bypass / confirmation bypass / source-ACL drift / sensitive-render regression 四项不破）
- A0 W14 拣入期：拣入 A6/A7/A8/A9/A10/A11 W14 整包 + A1 W14 reconciliation 整包 → DEBT-04 closed-by-W14

**A1 W14 派发期立场声明**：本 wave 仅交付 11 文件整包文档收口 + 零产品代码 + 不 push；A6/A7/A8/A9/A10/A11 实施期收口后由 A0 拣入，A1 W15+ 派发接收时再 reconcile W14 as accepted。

---

## §10 关联文件（Pointer）

- 派发原文：`PARALLEL_COMMAND_BOARD.md` L1163-1189（11 lane 表 + W14 dispatch）
- A0 W13 acceptance：`logs/checkpoints/A0-M5-W13-accept-W14-dispatch-20260908-0020.md`
- A1 W14 patch：`logs/checkpoints/Lane-A1-M5-W14-reconciliation-20260908-0020.patch`（11 文件改动）
- M5-0 末尾段：[W14 active · 2026-09-08 00:20 CST] 段（runtime surface 14 行表 + W14 hard stops）
- M5-12 末尾段：[W14 active · 2026-09-08 00:20 CST] 段（plugin manager UI 派发期立场）
- M5-13 末尾段：[W14 verification scope · 2026-09-08 00:20 CST] 段（11 项必跑验证矩阵）
- M5-14 末尾段：[W14 active · 2026-09-08 00:20 CST] 段（runtime surface 14 行表 + DEBT-04 仍挂账转 A6 W14）
- A1 W13 整包：`logs/checkpoints/A1-M5-W13-reconciliation-20260907-2358.md` + `logs/checkpoints/Lane-A1-M5-W13-reconciliation-20260907-2358.patch`（参考同形结构）
- A1 W12 整包：`logs/checkpoints/A1-M5-W12-reconciliation-20260907-2030.md` + `logs/checkpoints/Lane-A1-M5-W12-reconciliation-20260907-2030.patch`（参考同形结构）
