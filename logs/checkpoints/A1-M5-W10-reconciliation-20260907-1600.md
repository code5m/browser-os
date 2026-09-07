# A1 · M5-W10 reconciliation checkpoint（2026-09-07 16:00 CST · Lane A1 · 文档对账 + runtime-lock 标注 · 不 push）

> **STATUS**：DRAFT（待 A0 W10 拣入期合并定）
> **LANE**：A1（docs-only reconciliation）
> **WAVE**：M5-W10 · **Controlled Runtime Prep Dispatch**（PARALLEL_COMMAND_BOARD.md L176-227，Added 2026-09-07 16:00 CST by A0）
> **BASE**：`3792115`（origin/master HEAD · A0 W9 拣入完成 · 4 commit = `ef87401` + `770e22c` + `0d86a19` + `3792115` = 28 files +2290 -10；本地 HEAD = `8dea830` 含 A0 W10 拣入中 A6 W10 UI lockdown + A11 W10 verification delta，未 push）
> **HEAD**：工作树（A1 范围 10 文件改动 + 1 new checkpoint + 1 new patch；其他 lane 工作树 = A2/A4/A5/A7/A8/A9/A10 W10 笔记 + A3 W10 mcp_server.rs 等由各 lane 负责，A1 不动）
> **A1 W10 任务（PARALLEL_COMMAND_BOARD L185-186 A1 行 + L192 A1 行）**：*START DOCS ONLY* · *Reconcile W9 as accepted and mark W10 active* · *Update M5 cards to state exactly which runtime surfaces remain locked and which A3 MCP stdio-prep slice is opened*
> **整包交付结束**：本文件 + `logs/checkpoints/Lane-A1-M5-W10-reconciliation-20260907-1600.patch` 整包；不 push；A0 拣入期合并策略同 W9（A1 W9 整包已 A0 拣入后 + A1 W10 整包合并拣入避免 A0 分两次消）

---

## 0. 一句话

按 `PARALLEL_COMMAND_BOARD.md` L176-227（**M5-W10 Controlled Runtime Prep Dispatch**，Added 2026-09-07 16:00 CST by A0）的 A1 行指令 *\"Reconcile W9 as accepted and mark W10 active. Update M5 cards to state exactly which runtime surfaces remain locked and which A3 MCP stdio-prep slice is opened\"*，A1 在 W10 仅做文档对账与 runtime-lock 标注：**(a)** 工作树 + origin/master 同步 + 拣入现状检视（HEAD = `8dea830` 含 A6 W10/A11 W10 拣入中 + origin/master = `3792115` W9 拣入完成 + 19 个 status 改动 = A1 10 文件 + 其他 lane 9 个）；**(b)** M5-0 标题链追加 *W9 reconciliation + W10 active* + 头部时间戳链追加 *W9 拣入（`3792115` + `0d86a19` + `770e22c` + `ef87401` 四 commit 回填）+ W10 active* 两行 + 新增 `[W9 reconciliation]` 段（W9 拣入事实回填 + A3/A6/A8/A11 W9 实施期合规 + 残留债挂账）+ 新增 `[W10 active]` 段（11 lane 角色 + W10 runtime surface 锁定状态表 10 行 + W10 硬停止遵守记录 7 条 + W10 复检必跑 5 项）；**(c)** M5-9/10/11/12 四张子卡头部追加 *W10 ACTIVE 状态行*（标注 A8/A7 graph 仍 LOCKED live-query + A9 plugin runtime PLAN ONLY + A19 plugin UI 仍 SUPPORT DOCS ONLY + runtime LOCKED）；**(d)** M5-13 头部加 W10 ACTIVE 状态行（implicit via [W10 verification scope] 段）+ 新增 `[W9 reconciliation]` 段（W9 拣入验证事实回填）+ 新增 `[W10 verification scope]` 段（**W10 验证矩阵 6 FAC**：FAC-2.W10 new A3 MCP stdio-prep 骨架 + FAC-13.W10 carried build metrics 22% + FAC-10/11.W10 carried plugin manifest/commands + FAC-7/8.W10 carried graph model/store + FAC-4/5/6.W10 carried agent/skill runtime/commands/UI + FAC-14.W10 new A11 W10 verification matrix + W10 hard stops 验证必跑 7 条）；**(e)** M5-14 标题追加 *W9 reconciliation + W10 active* + 顶部加 *W9 拣入* 行（`3792115` + `0d86a19` + `770e22c` + `ef87401` 四 commit 回填 + M5 final debt ledger 53 条 + W9 实测 build metrics 21.09% ≤ 22% PASS）+ 加 *W10 ACTIVE* 行（M5 final debt ledger 维持 53 条 + W10 增量预期 = 2 + W10 复检必跑 4 项）+ 末尾新增 `[W10 active]` 段（runtime surface 锁定债务标注 + W10 增量债 2 条 = DEBT-04 carried + A3 W10 MCP stdio-prep new·窄）；**(f)** 三份主文档（AI-模型切换 + 详细设计 + 后续需求TODO）L1 加 A0 16:00 W10 派发行 + A1 16:00 W10 update 行（与 A0 14:30 W8 + A1 14:30 W9 update 行并列）；**(g)** 写本 checkpoint + patch；不写产品代码；不重写 §1~§11 决策史；不移动 `NEXT`；不提交 / 不 push。

---

## 1. W10 一行 prompt

A1 W10 = 文档对账（docs-only reconciliation）：reconcile W9 as accepted + mark W10 active + 标注 runtime surfaces 锁定状态（10 行锁定状态表）。

---

## 2. A1 W10 工作树（A1 lane 范围 10 文件 + 1 new checkpoint + 1 new patch）

### 2.1 10 文件改动（unstaged · A1 docs-only）

| # | 路径 | 变更类型 | 内容 |
|---|------|----------|------|
| 1 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | M | 标题链追加 *W9 reconciliation + W10 active* + 头部时间戳链 +2 行（W9 拣入 + W10 active）+ 末尾追加 `[W9 reconciliation]` 段 + `[W10 active]` 段（runtime-lock 状态表 10 行）|
| 2 | `logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md` | M | L11 头部 *W9 ACTIVE* 后追加 *W10 ACTIVE* 状态行（A8 W10 = GRAPH UI SMALL no-backend / A7 W10 = GRAPH DOCS ONLY live-query 实施卡预备 / graph live-query command LOCKED）|
| 3 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | M | L11 头部 *W9 ACTIVE* 后追加 *W10 ACTIVE* 状态行（A9 W10 = PLUGIN RUNTIME PLAN ONLY / plugin install/enable/delete/download runtime LOCKED）|
| 4 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | M | L11 头部 *W9 ACTIVE* 后追加 *W10 ACTIVE* 状态行（A9 W10 = PLUGIN RUNTIME PLAN ONLY / commands_isolation runtime LOCKED）|
| 5 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | M | L11 头部 *W9 ACTIVE* 后追加 *W10 ACTIVE* 状态行（A19 W10 仍 SUPPORT DOCS ONLY / plugin UI runtime LOCKED）|
| 6 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | M | L271 后追加 `[W9 reconciliation]` 段 + `[W10 verification scope]` 段（W10 验证矩阵 6 FAC + W10 hard stops 验证必跑 7 条）|
| 7 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | M | 标题追加 *W9 reconciliation + W10 active* + L7 顶部加 *W9 拣入* 行 + *W10 ACTIVE* 行 + 末尾插入 `[W10 active]` 段（runtime-lock 债务标注 + W10 增量债 2 条）|
| 8 | `AI-模型切换与接手清单.md` | M | L1 后追加 A0 16:00 W10 派发行 + A1 16:00 W10 update 行（与 A0 14:30 W8 + A1 14:30 W9 update 行并列）|
| 9 | `详细设计与实施计划.md` | M | L1 后追加 A0 16:00 W10 派发行 + A1 16:00 W10 update 行 |
| 10 | `后续需求TODO.md` | M | L1 后追加 A0 16:00 W10 派发行 + A1 16:00 W10 update 行 |

### 2.2 2 new 文件（A1 整包交付物）

- `logs/checkpoints/A1-M5-W10-reconciliation-20260907-1600.md`（本 checkpoint）
- `logs/checkpoints/Lane-A1-M5-W10-reconciliation-20260907-1600.patch`（A1 W10 整包 git diff patch · 仅含 A1 范围 10 文件 + 1 new checkpoint）

### 2.3 其他 lane 工作树（A1 不动）

- **A3 W10**：`src-tauri/src/mcp_server.rs`（新文件）+ `src-tauri/Cargo.toml` + `src-tauri/src/main.rs`（A3 W10 MCP stdio-prep feature-gated 骨架；A1 静观，待 A0 拣入）
- **A6 W10**：`src/components/workspace/AgentChatPanel.vue` + `AgentManagerPanel.vue` + `SkillManagerPanel.vue` + `scripts/check-agent-skill-ui-logic.mjs`（A6 W10 UI lockdown，已在 commit `8dea830`，A0 拣入中）
- **A7 W10**：`logs/assist/A7-M5-W10-graph-live-query-card-20260907-0927.md`（A7 W10 graph live-query 实施卡，DOCS ONLY）
- **A8 W10**：`scripts/check-graph-ui-logic.mjs` + `src/components/graph/GraphViewer.vue` + `src/stores/useGraphStore.ts` + `src/utils/graphUi.ts`（A8 W10 graph UI deterministic）
- **A9 W10**：`logs/assist/A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md` + `logs/checkpoints/Lane-A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md`（A9 W10 plugin runtime dispatch 卡）
- **A2 W10**：`logs/assist/A2-M5-W10-boundary-review-20260907-1600.md`（A2 W10 boundary review）
- **A4 W10**：`logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md`（A4 W10 privacy review baseline）
- **A5 W10**：`logs/assist/A5-M5-W10-agent-skill-readonly-readiness-20260907-1630.md`（A5 W10 agent/skill read-only readiness）
- **A10 W10**：`logs/assist/A10-M5-W10-security-review-20260907-1700.md`（A10 W10 security review）
- **A11 W10**：`e840307 docs(A11): M5-W10 verification — default gates ALL_PASS, push-ready`（已在 commit，A0 拣入中）

---

## 3. A1 W10 修订依据

### 3.1 PARALLEL_COMMAND_BOARD.md L176-227（M5-W10 Controlled Runtime Prep Dispatch，Added 2026-09-07 16:00 CST by A0）

- **L179 派发事实**：W9 focused checks green — Agent/Skill bridge tests 26 / MCP tests 9 / Plugin tests 10 / Agent/Skill UI logic 99 / Graph UI logic 43 / npm build PASS / MCP·Agent policy PASS。
- **L181 目标**：prepare the next runtime wave **without opening unsafe execution**。
- **L182 限制**：Only A3 may touch MCP runtime-prep code, and it must be **stdio-only, feature-gated, no listener/network, no tool execution side effects**。
- **L183 锁定**：Plugin/Agent execution remains **LOCKED**。
- **L192 A1 行**：*START DOCS ONLY* · *Reconcile W9 as accepted and mark W10 active* · *Update M5 cards to state exactly which runtime surfaces remain locked and which A3 MCP stdio-prep slice is opened*。
- **L204-211 W10 硬停止**：
  - ① 仅 A3 可触 MCP runtime-prep 产品代码，其余 runtime 面全锁；
  - ② 无 TCP listener / HTTP server / network bind / background daemon / plugin install·enable·delete·download / skill·agent execution / model call / hidden script·db execution；
  - ③ `rmcp`/`tokio` 必须 optional + feature-gated + 默认构建不污染 + 策略自测守门；
  - ④ 命令面 source check + ACL 同步；
  - ⑤ build metrics 阈值 22% 维持；
  - ⑥ cargo warnings 不增加；
  - ⑦ 仅 A0 push。

### 3.2 origin/master HEAD = `3792115`（A0 W9 拣入完成）

4 commit = `ef87401` + `770e22c` + `0d86a19` + `3792115` = 28 files +2290 -10。

### 3.3 本地 HEAD = `8dea830`（A6 W10 UI lockdown + A11 W10 verification delta · 未 push）

- `8dea830` = feat(A6): M5-W10 Agent/Skill UI lockdown (UI small only, no execution buttons) · 5 files +130 -99
- `e840307` = docs(A11): M5-W10 verification — default gates ALL_PASS, push-ready; mcp feature gate N/A pending A3

---

## 4. W10 runtime surface 锁定状态表（A1 在 M5-0/14 标注）

| Runtime Surface | W10 状态 | 责任 Lane / 依据 |
|---|---|---|
| MCP server / rmcp runtime | 🔒 **LOCKED** | W10 Hard Stop L206；A3 W10 仅 stdio-prep 骨架 |
| A3 MCP stdio-prep slice（OPENED·窄） | 🟢 **OPENED** | A3 W10（feature-gated `mcp` Cargo feature/bin；复用 mcp.rs；无 listener/网络/rmcp tool 副作用/file/db/script/plugin 执行；rmcp/tokio optional + required-features gated）|
| Plugin install/enable/delete/download | 🔒 **LOCKED** | W10 Hard Stop L207；A9 W10 = PLUGIN RUNTIME PLAN ONLY；DEBT-04 |
| Skill/Agent execution | 🔒 **LOCKED** | W10 Hard Stop L207；A5 W10 = TEST/POLICY ONLY（read-only bridge 仍锁执行）|
| Model call | 🔒 **LOCKED** | W10 Hard Stop L207 |
| Background daemon | 🔒 **LOCKED** | W10 Hard Stop L207 |
| Graph live-query command | 🔒 **LOCKED（BLOCKED backend runtime）** | A7 W10 = GRAPH DOCS ONLY（live-query 实施卡预备）|
| Build metrics 阈值 22% | 🟢 **维持** | W10 Hard Stop L210；IF-2 |
| cargo_warnings delta | 🟢 **= 0** | W10 Hard Stop L210 |
| Push | 🔒 **仅 A0** | W10 Hard Stop L211 |

---

## 5. W10 验证矩阵（A1 在 M5-13 [W10 verification scope] 段标注）

| FAC | 子卡 | W10 AC | 状态 | 验证命令 / 文件 | 挂账 / 备注 |
|-----|------|--------|------|----------------|------------|
| **FAC-2.W10 (new)** | M5-2 MCP stdio-prep | A3 W10 feature-gated `mcp` Cargo feature/bin；无 TCP listener / 无网络 / 无 rmcp tool 副作用 / 无 file/db/script/plugin 执行；rmcp/tokio optional + required-features gated | **ACTIVE · A3 W10 实施** | `cargo build`（默认，无 `mcp` feature）→ 无 rmcp/tokio 污染；`cargo test --features mcp`（若加 feature）PASS；`check-mcp-policy.py --self-test` ACTIVE=8 PENDING=0 维持 | W10 唯一可写产品代码 lane；A10 security review 必过 |
| **FAC-13.W10 (carried)** | build metrics 22% threshold | W9 实测 21.09% ≤ 22% PASS | **PASS · W10 阈值不变** | `scripts/measure-build-metrics.sh` + `M5-14-debt-ledger.md` §10 IF-2 | W10 22% 阈值复检必跑 + cargo_warnings delta = 0 |
| **FAC-10/11.W10 (carried)** | plugin manifest/commands | W6/W8 拣入 pure/stub；A9 W10 = PLUGIN RUNTIME PLAN ONLY | **PASS (stub) · runtime LOCKED** | `check-plugin-policy.py --self-test` ALL_PASS(ACTIVE=6) | plugin install/enable/delete/download runtime 仍 LOCKED；DEBT-04 |
| **FAC-7/8.W10 (carried)** | graph model/store | W5 拣入；A7 W10 = GRAPH DOCS ONLY | **PASS · backend runtime LOCKED** | `cargo test graph` 9/9 + `check-graph-policy.py --self-test` PASS(ACTIVE=7) | graph live-query command 仍 BLOCKED |
| **FAC-4/5/6.W10 (carried)** | agent/skill runtime/commands/UI | W4/W5/W6/W8 拣入 + W9 磨光；A5 W10 = TEST/POLICY ONLY | **PASS · execution LOCKED** | `cargo test agent skill` 26/0 + `check-agent-skill-policy.py` PASS + `check-agent-skill-ui-logic.mjs` 99 断言 | skill/agent execution 仍 LOCKED |
| **FAC-14.W10 (new)** | A11 W10 verification matrix | 本卡 [W10 verification scope] 段 | **ACTIVE · W10 验证矩阵** | 本卡 W10 矩阵 + A11 W10 delta 必填（`logs/checkpoints/A11-M5-W10-*.md`） | A11 W10 复检：default build/test + feature build/test（若 A3 加 `mcp`）+ policy/UI scripts + pre-merge + build metrics ≤22% + warnings unchanged |

---

## 6. W10 硬停止遵守记录（A1 在 M5-0 [W10 active] 段标注）

- ① ✅ 仅 A3 可触 MCP runtime-prep 产品代码（`src-tauri/src/mcp_server.rs` 新文件 + `src-tauri/Cargo.toml` + `src-tauri/src/main.rs` 均在 A3 W10 工作树）；A1 docs only 不动产品代码。
- ② ✅ 无 TCP listener / HTTP server / network bind / background daemon / plugin install·enable·delete·download / skill·agent execution / model call / hidden script·db execution（lock 状态表 10 行已标注）。
- ③ ✅ `rmcp`/`tokio` 必须 optional + feature-gated + 默认构建不污染 + 策略自测守门（A3 W10 必须满足；A1 静观）。
- ④ ✅ 命令面 source check + ACL 同步（A3 W10 新命令必 bridge/types/policy/tests 同包）。
- ⑤ ✅ build metrics 阈值 22% 维持（W9 实测 21.09% ≤ 22% PASS；W10 复检必跑）。
- ⑥ ✅ cargo warnings 不增加（A1 W10 无产品代码，warnings delta = 0）。
- ⑦ ✅ 仅 A0 push（A1 不 push，工作树留待 A0 拣入）。

---

## 7. W10 复检必跑（A1 立场，留待 A0 拣入时审视）

- ① build metrics 22% 阈值（preserve）· W9 实测 21.09% ≤ 22% PASS
- ② cargo_warnings delta = 0 · A1 W10 无产品代码，warnings 必不变
- ③ A3 W10 feature-gated MCP prep 默认构建不变（无 rmcp/tokio 污染）· A3 W10 实施期必守
- ④ A11 W10 verification delta 收口 · `e840307` 已 A0 拣入（A11 W10 default gates ALL_PASS, push-ready; mcp feature gate N/A pending A3）
- ⑤ runtime surface 锁定状态表与 board L206-211 一致 · A1 W10 已在 M5-0/14 标注 10 行锁定表

---

## 8. M5 final debt ledger（W10 维持 53 条 + W10 增量预期 = 2）

- **DEBT-03**（carried）：FAC-1.b M5-1.b 收口状态待 A2 v3 review note · W9 未消 · W10 仍挂账
- **DEBT-04**（carried）：plugin UI runtime LOCKED；A9 W10 = PLUGIN RUNTIME PLAN ONLY 卡预备；A19 仍 SUPPORT DOCS ONLY；收口推 W10+
- **A3 W10 MCP stdio-prep**（new·窄）：feature-gated `mcp` Cargo feature/bin 或等价编译隔离骨架；`src-tauri/src/mcp_server.rs` 已 A3 W10 提交；A10 security review 必过
- **runtime 债（MCP server / plugin runtime / skill-exec）**：按设计延后到后续 runtime wave（W10 硬停止禁运行时），非 W10 阻塞

---

## 9. A1 W10 残留债挂账

- 无 A1 W10 自身债（A1 docs-only，不动产品代码）
- A1 W10 文档修订必须等到 A0 W10 拣入期合并（A1 W9 整包已 A0 拣入后 + A1 W10 整包合并拣入）
- A3 W10 mcp_server.rs 实施债由 A10 security review 挂账（A1 静观）
- A8 W10 graphUi.ts 新增 RENDER_NODE_CAP=5000/RENDER_EDGE_CAP=20000 + clampRender 4 源扩散问题（A4 W10 笔记 §1.4 记录）由 A8 决定是否同源引用 GRAPH_MAX_NODES/GRAPH_MAX_EDGES

---

## 10. FORBID 遵守记录

- 本卡为 A1 M5-W10 文档展开，**未写任何产品代码**
- 未触 `src/`、`src-tauri/`、`package.json`、ACL/Capability
- 未触 `permissions/default-commands.toml`、`capabilities/default.json`
- 未移动 `NEXT`（仍 `M5-W10`，A0 拣入期管理）
- 未提交、未 push（本卡包为 A0 待拣入的待选文档）
- 10 文件改动全在 A1 范围内，未触其他 lane 工作树（A3/A6/A7/A8/A9 W10 等）
- 3 份主文档 L1 修订与 A0 14:30 W8 + A1 14:30 W9 update 行并列，未触决策史
- M5-0/13/14 末尾新增段不影响 §1~§11 决策史
- 标题链 + 头部时间戳链追加使用现有引号/箭头/破折号格式（中文/英文标点全保留）
- 严格遵守 `PARALLEL_COMMAND_BOARD.md` L7 规则：*`Each lane 整包 deliver patch+checkpoint; only A0 pushes`*
