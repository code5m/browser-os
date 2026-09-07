# A1 · M5-W11 reconciliation checkpoint（2026-09-07 18:30 CST · Lane A1 · 文档对账 + W10 PUSHED/W11 ACTIVE 标注 · 不 push）

> **STATUS**：DRAFT（待 A0 W11 拣入期合并定）
> **LANE**：A1（docs-only reconciliation）
> **WAVE**：M5-W11 · **MCP Stdio Dry-Run Hardening Dispatch**（`PARALLEL_COMMAND_BOARD.md` L176-228，Added 2026-09-07 18:30 CST by A0）
> **BASE**：`5226aad`（origin/master HEAD · A0 W10 拣入完成 · 2 commit = `ba78092` + `5226aad` = A3 W10 MCP stdio-prep feature-gated skeleton + A0 W10 整包合并拣入）
> **HEAD**：工作树（A1 范围 9 文件改动 + 1 new checkpoint + 1 new patch；其他 lane 工作树 = 由各 lane 负责，A1 不动）
> **A1 W11 任务**（`PARALLEL_COMMAND_BOARD.md` L192 A1 行）：*START DOCS ONLY* · *Reconcile W10 as accepted after A0 push and mark W11 active. Update M5 cards so W10 stdio-prep is recorded as feature-gated PASS and W11 remains dry-run only.* scope = `PARALLEL_COMMAND_BOARD.md` + 3 主文档 + `logs/checkpoints/M5-20260906/*.md` + `logs/checkpoints/A1-M5-W11-*.md`；Must Deliver = **One reconciliation checkpoint; no product code**.
> **整包交付结束**：本文件 + `logs/checkpoints/Lane-A1-M5-W11-reconciliation-20260907-1830.patch` 整包；不 push；A0 W11 拣入期合并策略同 W10（A1 W10 整包已 A0 拣入 → A1 W11 整包合并拣入避免 A0 分两次消）。

---

## 0. 一句话

按 `PARALLEL_COMMAND_BOARD.md` L176-228（**M5-W11 MCP Stdio Dry-Run Hardening Dispatch**，Added 2026-09-07 18:30 CST by A0）的 A1 行指令 *"Reconcile W10 as accepted after A0 push and mark W11 active. Update M5 cards so W10 stdio-prep is recorded as feature-gated PASS and W11 remains dry-run only"*，A1 在 W11 仅做文档对账与 W10→W11 状态迁移标注：**(a)** 工作树 + origin/master 同步 + 拣入现状检视（HEAD = `5226aad` 含 A0 W10 拣入完成，2 commit = `ba78092` + `5226aad`；本地工作树干净，无其他 lane 漂移）；**(b)** `M5-0-overview.md` 标题链追加 *W10 PUSHED + W11 active* + 头部时间戳链追加 *W10 拣入（`ba78092` + `5226aad` 两 commit 回填）+ W11 active* 两行 + L20 基准链追加 *W10 拣入* 行 + 末尾新增 `[W10 reconciliation]` 段（W10 拣入事实回填 + A3 W10 feature-gated stdio-prep 落地证据 + 10 lane W10 实施期合规 + 残留债挂账）+ 新增 `[W11 active]` 段（11 lane 角色 + W11 runtime surface 锁定状态表 10 行 + W11 硬停止遵守记录 8 条 + W11 复检必跑 5 项）；**(c)** `M5-9/10/11/12` 四张子卡头部 W10 ACTIVE 状态行后追加 *W10 PUSHED* 行（标 `ba78092` + `5226aad`）+ *W11 ACTIVE* 状态行（A3 W11 = MCP STDRY narrow product code 干运行硬化 / A7 W11 = GRAPH DOCS ONLY W12 plan / A9 W11 = PLUGIN DOCS/POLICY ONLY W12/W13 plan / A6 W11 = UI SMALL no-backend / runtime LOCKED）；**(d)** `M5-13` 头部加 W10 PUSHED + W11 ACTIVE 状态行 + 末尾 `[W10 reconciliation]` 段（W10 拣入验证事实回填）+ 末尾 `[W11 verification scope]` 段（**W11 验证矩阵 6 FAC**：FAC-2.W11 new A3 MCP stdio dry-run bounded hardening + FAC-13.W11 carried build metrics 22% + FAC-10/11.W11 carried plugin manifest/commands + FAC-7/8.W11 carried graph model/store + FAC-4/5/6.W11 carried agent/skill runtime/commands/UI + FAC-14.W11 new A11 W11 verification matrix + W11 hard stops 验证必跑 8 条）；**(e)** `M5-14` 标题追加 *W10 reconciliation + W11 active* + 顶部加 *W10 PUSHED* 行（`ba78092` + `5226aad` 两 commit 回填 + M5 final debt ledger 53 条 + W10 实测 7/7 mcp_server feature + 9/9 PENDING 0 policy + build metrics 22% 阈值复检待 A11 W11）+ *W11 ACTIVE* 行（M5 final debt ledger 维持 53 条 + W11 增量预期 = 2：FAC-2.W11 A3 bounded stdio dry-run narrow·待验 + DEBT-04 carried / A11 W11 verification delta 必跑）+ 末尾新增 `[W11 active]` 段（runtime surface 锁定债务标注 + W11 增量债 2 条 = DEBT-04 carried + A3 W11 bounded stdio dry-run new·窄）；**(f)** 三份主文档（`AI-模型切换与接手清单.md` + `详细设计与实施计划.md` + `后续需求TODO.md`）L1 加 A0 18:30 W11 派发行 + A1 18:30 W11 update 行（与 A0 18:30 W10 PASS 拣入行 + A1 16:00 W10 update 行并列）；**(g)** 写本 checkpoint + patch；不写产品代码；不重写 §1~§11 决策史；不移动 `NEXT`；不提交 / 不 push。

---

## 1. W11 一行 prompt

A1 W11 = 文档对账（docs-only reconciliation）：reconcile W10 as PUSHED + mark W11 active + 标注 W11 runtime surfaces 锁定状态（10 行锁定状态表）。

---

## 2. A1 W11 工作树（A1 lane 范围 9 文件改动 + 1 new checkpoint + 1 new patch）

### 2.1 9 文件改动（unstaged · A1 docs-only）

| # | 路径 | 变更类型 | 内容 |
|---|------|----------|------|
| 1 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | M | 标题链追加 *W10 reconciliation + W10 PUSHED + W11 active* + 头部时间戳链 +2 行（W10 拣入 + W11 active）+ L20 基准链追加 *W10 拣入* 行 + 末尾追加 `[W10 reconciliation]` 段 + `[W11 active]` 段（runtime-lock 状态表 10 行）|
| 2 | `logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md` | M | L12 头部 *W10 ACTIVE* 后追加 *W10 PUSHED* 状态行（A8 W10 PUSHED `5226aad`）+ *W11 ACTIVE* 状态行（A8 W11 = GRAPH UI SMALL no-backend / A7 W11 = GRAPH DOCS ONLY W12 plan / graph live-query command LOCKED）|
| 3 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | M | L12 头部 *W10 ACTIVE* 后追加 *W10 PUSHED* 状态行（A9 W10 PUSHED `5226aad`）+ *W11 ACTIVE* 状态行（A9 W11 = PLUGIN DOCS/POLICY ONLY W12/W13 plan / plugin install/enable/delete/download runtime LOCKED）|
| 4 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | M | L12 头部 *W10 ACTIVE* 后追加 *W10 PUSHED* 状态行（A9 W10 PUSHED `5226aad`）+ *W11 ACTIVE* 状态行（A9 W11 = PLUGIN DOCS/POLICY ONLY W12/W13 plan / commands_isolation runtime LOCKED）|
| 5 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | M | L12 头部 *W10 ACTIVE* 后追加 *W10 PUSHED* 状态行（A9 W10 PUSHED `5226aad`）+ *W11 ACTIVE* 状态行（A19 W11 仍 SUPPORT DOCS ONLY / plugin UI runtime LOCKED）|
| 6 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | M | 头部 *W10 ACTIVE* 后追加 *W10 PUSHED* + *W11 ACTIVE* 状态行（implicit via [W11 verification scope] 段）+ 末尾追加 `[W10 reconciliation]` 段 + 末尾追加 `[W11 verification scope]` 段（W11 验证矩阵 6 FAC + W11 hard stops 验证必跑 8 条）|
| 7 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | M | 标题追加 *W10 reconciliation + W10 PUSHED + W11 active* + 顶部加 *W10 PUSHED* 行 + *W11 ACTIVE* 行 + 末尾插入 `[W11 active]` 段（runtime-lock 债务标注 + W11 增量债 2 条）|
| 8 | `AI-模型切换与接手清单.md` | M | L1 后追加 A0 18:30 W11 派发行 + A1 18:30 W11 update 行（与 A0 18:30 W10 PASS 拣入行 + A1 16:00 W10 update 行并列）|
| 9 | `详细设计与实施计划.md` | M | L1 后追加 A0 18:30 W11 派发行 + A1 18:30 W11 update 行 |
| 10 | `后续需求TODO.md` | M | L1 后追加 A0 18:30 W11 派发行 + A1 18:30 W11 update 行 |

### 2.2 2 new 文件（A1 整包交付物）

- `logs/checkpoints/A1-M5-W11-reconciliation-20260907-1830.md`（本 checkpoint）
- `logs/checkpoints/Lane-A1-M5-W11-reconciliation-20260907-1830.patch`（A1 W11 整包 git diff patch · 仅含 A1 范围 9 文件 + 1 new checkpoint）

### 2.3 其他 lane 工作树（A1 不动）

- **A3 W11**：`src-tauri/src/mcp_server.rs` 硬化（待 A3 实施期；A1 静观，待 A0 拣入）
- **A6 W11**：`src/components/workspace/AgentChatPanel.vue` + `AgentManagerPanel.vue` + `SkillManagerPanel.vue` + `scripts/check-agent-skill-ui-logic.mjs`（A6 W11 UI small，no execution buttons）
- **A7 W11**：`logs/assist/A7-M5-W11-graph-w12-card-*.md`（A7 W11 graph W12 plan，DOCS ONLY）
- **A8 W11**：`scripts/check-graph-ui-logic.mjs` + `src/components/graph/GraphViewer.vue` + `src/stores/useGraphStore.ts` + `src/utils/graphUi.ts`（A8 W11 graph UI small）
- **A9 W11**：`logs/assist/A9-M5-W11-plugin-staged-*.md`（A9 W11 plugin W12/W13 staged cards）
- **A2 W11**：`logs/assist/A2-M5-W11-boundary-review-*.md`（A2 W11 boundary review）
- **A4 W11**：`logs/assist/A4-M5-W11-privacy-review-*.md`（A4 W11 privacy review）
- **A5 W11**：`logs/assist/A5-M5-W11-agent-skill-readiness-*.md` + `scripts/check-agent-skill-policy.py`（A5 W11 policy only）
- **A10 W11**：`logs/assist/A10-M5-W11-security-review-*.md`（A10 W11 security review）
- **A11 W11**：`logs/checkpoints/A11-M5-W11-verification-*.md` + `logs/assist/A11-M5-W11-*.md`（A11 W11 verification delta）

---

## 3. A1 W11 修订依据

### 3.1 `PARALLEL_COMMAND_BOARD.md` L176-228（M5-W11 MCP Stdio Dry-Run Hardening Dispatch，Added 2026-09-07 18:30 CST by A0）

- **L179 派发事实**：W10 A3 stdio-prep skeleton is feature-gated (`mcp`), std-only, no rmcp/tokio dependency, no TCP/network listener, and `cargo test --features mcp mcp_server` passes **7/7**. W10 policy gates pass with `MCP_POLICY_SELF_TEST=PASS(ACTIVE=9,PENDING=0)` and `MCP_CURRENT_GAPS_RESULT=PASS`. A6/A8 UI logic checks and npm build pass in W10.
- **L180 目标**：harden the MCP stdio shell through deterministic dry-run behavior and review evidence only. This is still **not full runtime activation**: no file/db/script/plugin execution, no network listener, no daemon, no model call.
- **L192 A1 行**：*START DOCS ONLY* · *Reconcile W10 as accepted after A0 push and mark W11 active. Update M5 cards so W10 stdio-prep is recorded as feature-gated PASS and W11 remains dry-run only.*
- **L204-211 W11 硬停止**（8 条，比 W10 多了"无 raw argument/query/token/cookie/Authorization echo in stdio responses/logs/audit/checkpoints/UI state"）：
  - ① W11 仍 dry-run only：不真实执行 file/db/script/plugin/agent/skill/model；
  - ② 无 TCP listener / HTTP server / network bind / background daemon / plugin install·enable·delete·download / model call / hidden script·db execution；
  - ③ 无 raw argument/query/token/cookie/Authorization echo in stdio responses / logs / audit / checkpoints / UI state；
  - ④ 任何依赖添加必须 optional + feature-gated；默认构建行为必须不变；
  - ⑤ 新增或变更命令面必须保持 source check + ACL + bridge/types parity + policy self-tests 同步；
  - ⑥ build metrics 阈值 22% 维持；
  - ⑦ cargo warnings 不增加；
  - ⑧ 仅 A0 push。

### 3.2 origin/master HEAD = `5226aad`（A0 W10 拣入完成）

- 2 commit = `ba78092` + `5226aad`：
  - **`ba78092 feat(M5-W10,A3): MCP stdio-prep skeleton (feature-gated, read-only, std-only)`**（A3 W10 实施期：`Cargo.toml` 加 `[features] mcp = []`（默认构建不变，零新依赖）+ `main.rs` `#[cfg(feature="mcp")]` 自门控 + `--mcp-stdio` 派发 + `src/mcp_server.rs`（new）feature-gated stdio JSON-RPC 骨架，复用 `mcp.rs` registry/policy wiring；7 cargo tests PASS）
  - **`5226aad feat(M5): integrate W10 MCP stdio prep`**（A0 W10 整包合并拣入：M5-0/9/10/11/12/13/14 头部 W10 状态修订 + A1 W10 reconciliation 整包 + A2/A4/A5/A6/A7/A8/A9/A10/A11 9 份 W10 assist/checkpoint + 3 主文档 W10 update 行 + `PARALLEL_COMMAND_BOARD.md` 加 W11 dispatch）

### 3.3 本地 HEAD = `5226aad`（A0 W10 拣入完成 · 未 push）

- 本地工作树干净，无未提交改动。
- 任何未跟踪文件（`logs/checkpoints/A0-M5-W10-accept-W11-dispatch-20260907-1830.md` 等）由 A0 单独留待拣入；A1 不动。

---

## 4. W11 runtime surface 锁定状态表（A1 在 M5-0/14 标注）

| Runtime Surface | W11 状态 | 责任 Lane / 依据 |
|---|---|---|
| MCP server / rmcp runtime | 🔒 **LOCKED** | W11 Hard Stop L206；A3 W11 仅 stdio dry-run/list-call hardening |
| A3 MCP stdio dry-run/list-call hardening（OPENED·窄） | 🟢 **OPENED** | A3 W11（deterministic JSON-RPC errors + bounded input/response size + stable `tools/list` schema + explicit fail-closed `tools/call` for all unbound capabilities + tests/smoke for invalid JSON/unknown tool/large params；无真实 file/db/script/plugin 执行；无 listener/network/rmcp/tokio）|
| Plugin install/enable/delete/download | 🔒 **LOCKED** | W11 Hard Stop L207；A9 W11 = PLUGIN DOCS/POLICY ONLY W12/W13 plan；DEBT-04 |
| Skill/Agent execution | 🔒 **LOCKED** | W11 Hard Stop L206；A5 W11 = AGENT/SKILL POLICY ONLY（execution 仍锁）|
| Model call | 🔒 **LOCKED** | W11 Hard Stop L206 |
| Background daemon | 🔒 **LOCKED** | W11 Hard Stop L207 |
| Graph live-query command | 🔒 **LOCKED（BLOCKED backend runtime）** | A7 W11 = GRAPH DOCS ONLY（W12 plan；不实施命令）|
| Build metrics 阈值 22% | 🟢 **维持** | W11 Hard Stop L211 |
| cargo_warnings delta | 🟢 **= 0** | W11 Hard Stop L211 |
| Push | 🔒 **仅 A0** | W11 Hard Stop L212 |

---

## 5. W11 验证矩阵（A1 在 M5-13 [W11 verification scope] 段标注）

| FAC | 子卡 | W11 AC | 状态 | 验证命令 / 文件 | 挂账 / 备注 |
|-----|------|--------|------|----------------|------------|
| **FAC-2.W11 (new)** | M5-2 MCP stdio dry-run hardening | A3 W11 deterministic JSON-RPC errors + bounded input/response size + stable `tools/list` schema + explicit fail-closed `tools/call` for all unbound capabilities + tests/smoke for invalid JSON/unknown tool/large params；无真实 file/db/script/plugin 执行 | **ACTIVE · A3 W11 实施** | `cargo test --features mcp mcp_server`（W10 7/7 基线 + W11 增量）PASS；`check-mcp-policy.py --self-test` ACTIVE ≥ 9 维持 + W11 新增 MCP_STDIO_DRY_RUN_BOUNDED / MCP_NO_LISTENER / MCP_NO_NETWORK / MCP_NO_RAW_ARG_ECHO 等守门码 | W11 唯一可写产品代码 lane；A10 security review 必过；无 listener/network/raw-arg-echo 是 W11 红线 |
| **FAC-13.W11 (carried)** | build metrics 22% threshold | W10 实测 ≤ 22% PASS | **PASS · W11 阈值不变** | `scripts/measure-build-metrics.sh` + `M5-14-debt-ledger.md` §10 IF-2 | W11 22% 阈值复检必跑 + cargo_warnings delta = 0 |
| **FAC-10/11.W11 (carried)** | plugin manifest/commands | W6/W8/W10 拣入 pure/stub；A9 W11 = PLUGIN DOCS/POLICY ONLY W12/W13 staged cards | **PASS (stub) · runtime LOCKED** | `check-plugin-policy.py --self-test` ALL_PASS(ACTIVE=6) | plugin install/enable/delete/download runtime 仍 LOCKED（W11 Hard Stop L207）；DEBT-04 |
| **FAC-7/8.W11 (carried)** | graph model/store | W5 拣入；A7 W11 = GRAPH DOCS ONLY W12 plan | **PASS · backend runtime LOCKED** | `cargo test graph` 9/9 + `check-graph-policy.py --self-test` PASS(ACTIVE=7) | graph live-query command 仍 BLOCKED（不实施） |
| **FAC-4/5/6.W11 (carried)** | agent/skill runtime/commands/UI | W4/W5/W6/W8/W9/W10 拣入 + W10 磨光；A5 W11 = AGENT/SKILL POLICY ONLY | **PASS · execution LOCKED** | `cargo test agent skill` 26/0 + `check-agent-skill-policy.py` PASS + `check-agent-skill-ui-logic.mjs` 99 断言 | skill/agent execution 仍 LOCKED（W11 Hard Stop L206） |
| **FAC-14.W11 (new)** | A11 W11 verification matrix | 本卡 [W11 verification scope] 段 | **ACTIVE · W11 验证矩阵** | 本卡 W11 矩阵 + A11 W11 delta 必填（`logs/checkpoints/A11-M5-W11-*.md`） | A11 W11 复检：default build/test + feature build/test + policy/UI scripts + pre-merge + build metrics ≤22% + warnings unchanged + W11 dry-run 行为确定性 |

---

## 6. W11 硬停止遵守记录（A1 在 M5-0 [W11 active] 段标注）

- ① ✅ W11 仍 dry-run only（不真实执行 file/db/script/plugin/agent/skill/model）；A3 W11 仅做 stdio dry-run/list-call hardening（不实施真实执行）。
- ② ✅ 无 TCP listener / HTTP server / network bind / background daemon / plugin install·enable·delete·download / model call / hidden script·db execution（lock 状态表 10 行已标注）。
- ③ ✅ 无 raw argument/query/token/cookie/Authorization echo in stdio responses / logs / audit / checkpoints / UI state（A4 W11 privacy review 必过；A3 W11 必保 stable JSON-RPC error 模板，无原始参数回显）。
- ④ ✅ 任何依赖添加必须 optional + feature-gated + 默认构建不污染 + 策略自测守门（A3 W11 与 W10 同口径，default `cargo build` 不引入 `rmcp`/`tokio`）。
- ⑤ ✅ 命令面 source check + ACL 同步（若 A3 W11 新增命令必 bridge/types/policy/tests 同包）。
- ⑥ ✅ build metrics 阈值 22% 维持（W10 实测 ≤ 22% PASS；W11 复检必跑）。
- ⑦ ✅ cargo warnings 不增加（A1 W11 无产品代码，warnings delta = 0）。
- ⑧ ✅ 仅 A0 push（A1 不 push，工作树留待 A0 拣入）。

---

## 7. W11 复检必跑（A1 立场，留待 A0 拣入时审视）

- ① build metrics 22% 阈值（preserve）· W10 实测 ≤ 22% PASS
- ② cargo_warnings delta = 0 · A1 W11 无产品代码，warnings 必不变
- ③ A3 W11 feature-gated MCP dry-run 默认构建不变（无 rmcp/tokio 污染）· A3 W11 实施期必守
- ④ A11 W11 verification delta 收口 · 待 A11 W11 出 delta
- ⑤ runtime surface 锁定状态表与 board L206-212 一致 · A1 W11 已在 M5-0/14 标注 10 行锁定表

---

## 8. M5 final debt ledger（W11 维持 53 条 + W11 增量预期 = 2）

- **DEBT-03**（carried）：FAC-1.b M5-1.b 收口状态待 A2 v3 review note · W10 未消 · W11 仍挂账
- **DEBT-04**（carried）：plugin UI runtime LOCKED；A9 W11 = PLUGIN DOCS/POLICY ONLY W12/W13 staged cards；A19 仍 SUPPORT DOCS ONLY；收口推 W12+
- **A3 W11 bounded stdio dry-run**（new·窄）：deterministic JSON-RPC errors + bounded input/response + stable `tools/list` schema + explicit fail-closed `tools/call` + tests/smoke；A10 security review 必过；无 listener/network/raw-arg-echo 是 W11 红线
- **runtime 债（MCP server / plugin runtime / skill-exec）**：按设计延后到后续 runtime wave（W11 硬停止禁运行时），非 W11 阻塞

---

## 9. A1 W11 残留债挂账

- 无 A1 W11 自身债（A1 docs-only，不动产品代码）
- A1 W11 文档修订必须等到 A0 W11 拣入期合并（A1 W10 整包已 A0 拣入 → A1 W11 整包合并拣入）
- A3 W11 mcp_server.rs 实施债由 A10 security review 挂账（A1 静观）
- A4 W11 privacy review 关注 W11 dry-run responses 的 URL/userinfo/query redaction 完整性（若 raw params 可 echo 即 block）

---

## 10. FORBID 遵守记录

- 本卡为 A1 M5-W11 文档展开，**未写任何产品代码**
- 未触 `src/`、`src-tauri/`、`package.json`、ACL/Capability
- 未触 `permissions/default-commands.toml`、`capabilities/default.json`
- 未移动 `NEXT`（仍 `M5-W11`，A0 拣入期管理）
- 未提交、未 push（本卡包为 A0 待拣入的待选文档）
- 9 文件改动全在 A1 范围内，未触其他 lane 工作树（A3/A6/A7/A8/A9 W11 等）
- 3 份主文档 L1 修订与 A0 18:30 W10 PASS 拣入行 + A1 16:00 W10 update 行并列，未触决策史
- M5-0/13/14 末尾新增段不影响 §1~§11 决策史
- 标题链 + 头部时间戳链追加使用现有引号/箭头/破折号格式（中文/英文标点全保留）
- 严格遵守 `PARALLEL_COMMAND_BOARD.md` L7 规则：*`Each lane 整包 deliver patch+checkpoint; only A0 pushes`*
