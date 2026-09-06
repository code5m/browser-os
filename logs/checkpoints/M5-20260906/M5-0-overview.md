# M5 协议与智能生态 — 任务卡展开（Lane A1 · M5-W0 + W1 + W2 + W3 reconciliation + W4 active + W5 active）

> 生成：2026-09-06 08:00 CST · Lane A1（M5-W0 · docs only）
> W1 修订：2026-09-06 08:50 CST · Lane A1（M5-W1 · docs-only reconciliation）
> W2 修订：2026-09-06 13:45 CST · Lane A1（M5-W2 · docs-only reconciliation，对齐 `712a14c`）
> W3 修订：2026-09-06 17:30 CST · Lane A1（M5-W3 · docs-only reconciliation，对齐 `f8f1f49` + `12f1cff` + `bdb0602`）
> W4 拣入：2026-09-06 15:18 CST · A0 在 `1610939 feat(M5): add agent memory and skill policy shells` 中拣入 A1 W4 reconciliation 整包（768 行 patch + 202 行 checkpoint + 6 子卡修订）
> W5 active：2026-09-06 18:35 CST · Lane A1（M5-W5 · docs-only reconciliation，标 M5-6/M5-7/M5-8 为 W5 实施期卡，标 M5-9 仍 docs-only）
> 基准：`a1a2061`（`master`，M4 已 PASS） + `404f514`（A0 W0/W1 dispatch） + `a654f0c`（A3 W2 MCP 政策门）+ `712a14c`（A2 W2 切片 0b + A1 M5-1.b 切卡）+ `98a3b01`（A0 W3 dispatch）+ `e96c902`（A6 W3 UI data contract）+ `bdb0602`（A11 W3 verification）+ `12f1cff`（A3 W3 MCP 余下切片）+ `f8f1f49`（A2 W3 seam + A1 W3 reconciliation + A4-A10 W3 assist）+ `f7ad35a`（A0 W4 dispatch）+ `1610939 feat(M5): add agent memory and skill policy shells`（A0 拣入 A4 W4 agent_memory.rs + A5 W4 agent.rs/skills.rs + A1 W4 reconciliation 整包 + 8 份 W4 assist）+ `0e76a89 docs(M5): dispatch W5 UI and graph lanes`（A0 W5 dispatch）
> 性质：**纯文档展开**。零产品代码（未触 `src/`、`src-tauri/`、`package.json`、三份主文档、ACL/Capability/Manifest、pre-merge.sh）；不移动 `NEXT`；不提交、不 push。
> 依据：`PARALLEL_COMMAND_BOARD.md`（2026-09-05 23:55 版 · Batch Implementation Dispatch · Lane A1: M5 task-card expansion；2026-09-06 08:35 CST · M5-W1 Implementation Dispatch · Lane A1: START DOCS ONLY · reconciliation；2026-09-06 13:45 CST · M5-W2 Parallel Dispatch · Lane A1: START DOCS ONLY · constant-centralization reconciliation；2026-09-06 17:10 CST · M5-W3 Parallel Dispatch · Lane A1: START DOCS ONLY · mark W1/W2 complete + W3 active；2026-09-06 17:55 CST · M5-W4 Parallel Dispatch · Lane A1: START DOCS ONLY · reconcile W4 as active NEXT; mark W3 pushed and split M5-3/M5-4/M5-5 into next-card acceptance criteria；**2026-09-06 18:35 CST · M5-W5 Parallel Dispatch · Lane A1: START DOCS ONLY · reconcile W5 as active NEXT; mark W4 pushed and tighten M5-6/M5-7/M5-8 acceptance criteria**）
> 入口：本卡体系的根文档，本目录下其它文件是各 M5-x 子卡

---

## W1 Reconciliation Note（2026-09-06 08:50 CST · Lane A1 修订）

### 触发

A0 在 `404f514` 后下发 M5-W1 dispatch（`logs/checkpoints/A0-M5-W1-dispatch-20260906-0835.md`），A1 行要求：
> *"Reconcile M5 task cards with A2/A6/A10 findings: M5-1 internal order must be boundary policy first, then minimal extraction; remove stale assumptions such as vue-router and empty prework notes."*

A1 复核 W0 整包与 A2 v3 prework（`logs/assist/A2-M5-core-20260906-0749.md` §12/§13）、A6 prework、A10 复审（`logs/assist/A10-M5-security-review-20260906-1410.md` §36/§104）后，**确认 5 张子卡需修订**，**"vue-router"字面量不存在**，W0 中"被预研证伪的隐式假设"为：A1 W0 的事实数据（模块数 23/反向边 4 处/scheduler 4 行/agent_kv 扁平+缺 `updated_at`/`check-core-boundary.sh`/切片 0a→0b 顺序）被 A2/A4 实测修正。

### 修订索引（5 处）

| # | 文件 | 修订点 | 来源 |
|---|---|---|---|
| W1-1 | `M5-0-overview.md`（本文件） | 顶部加本节、依赖图注脚更新 | A0 W1 + A2 v3 |
| W1-2 | `M5-1-core-workspace-split.md` | 应用 A2 v3 6+2 处分歧（详见该卡顶部 `[W1 patched]` 段） | A2 v3 §12/§13 + A0 W1 boundary-first |
| W1-3 | `M5-3-a2a-bidir-agent-kv.md` | `agent_kv` 改三层嵌套 + 删 LRU 缺 `updated_at` 矛盾 | A4 prework + A10 §104 |
| W1-4 | `M5-13-verification-matrix.md` | `check-core-boundary.sh` → `.py`（仓库惯例） | A2 v3 §13.3 C-8 |
| W1-5 | `M5-14-debt-ledger.md` §6 L83 scheduler 反向边从 "M5-1.a 解决" 改 "M5-1.b 解决" | A2 v3 §13.2 C-7 |
| W1-6 | `logs/checkpoints/M5-A1-expansion-20260906-0800.md` | 加 W1 修订段 | A0 W1 dispatch |

### 不修订

- **M5-2 / M5-4 / M5-5 / M5-6 / M5-7 / M5-8 / M5-9 / M5-10 / M5-11 / M5-12**（10 张）：A0 W1 dispatch 把 M5-2/4/5/6/7/8/9/10/11/12 的产品代码**仍锁定**，A1 不在 W1 范围改这些卡。
- **三份主文档**（`详细设计与实施计划.md` / `后续需求TODO.md` / `AI-模型切换与接手清单.md`）：A0 W1 未指派必要改动；A1 也不在 W1 范围动。
- **`NEXT` 标记**：[W1 patched · 2026-09-06 13:30 CST] M5-W1 收口后 = M5-W2；M5-W2 待 A0 签发 dispatch；候选子卡 `M5-1.b` 已被 A1 在本批拆卡（`logs/checkpoints/M5-20260906/M5-1.b-seam-trait-injection-and-b-extract.md`），A0 W2 dispatch 时可直接引用该卡作为下达蓝本；`M5-1.c` 阶段二 workspace 化为可选延后项。

### A2 v3 C-1~C-8 摘要（A1 M5-1 卡应用 6+2 处分歧的源）

- **C-1**：M5-1.a 内部顺序从 `0a → 0b` 改 `0b → 0a`（测试反向 import 必须先收口）
- **C-2**：阶段一 `cargo tree -p core` 命令降级 PENDING，改用源码级 `grep -rE '^\s*use tauri' src-tauri/src/core/` 断言
- **C-3**：`M5-1.b` 拆为 `M5-1.b`（B 类 seam+搬入）与 `M5-1.c`（阶段二 workspace 化）
- **C-4**："23 模块" 回填为 "20 业务模块 + main.rs"
- **C-5**：反向边从 "4 处" 改 "8 处 + 2 组测试反向 import"，补 `:761`
- **C-6**：`[lib] path="src/core/mod.rs"` vs `src/lib.rs` 二选一均可（不强制）
- **C-7**（v3 新增）：M5-14 §6 L83 反向边改挂 M5-1.b
- **C-8**（v3 新增）：`check-core-boundary` 用 `.py`（仓库 `scripts/` 30+ 脚本惯例）

### A0 W1 boundary-first 顺序（A1 M5-1 卡的实施序重组）

1. **步骤 0（new in W1）**：A2 先建 `scripts/check-core-boundary.py`（含 `--self-test` 2好+2坏+1阴/默认扫描/`--expect-pending` 三模式）并挂 `scripts/pre-merge.sh` —— **本步是后续所有 core 内操作的准入前置**。
2. **步骤 0b**（A1 M5-1.a 切片 0b）：先把 `HARD_GRACE_SECS` / `MAX_TIMEOUT_SECS` / `MAX_TEXT_FIELD_BYTES` / `DB_MAX_TEXT_FIELD_BYTES` / `DB_SOFT_TO_HARD_GRACE_SECS`（含 V-7 副本收口）收口到 `domain.rs`，改 `t_db_c6_limit_and_timeout_constants_are_aligned` 避免"自比退化"。
3. **步骤 0a**（A1 M5-1.a 切片 0a）：建 `src-tauri/src/core/mod.rs` + A 类 10 模块整文件带 `#[cfg(test)]` 搬入。
4. **步骤 1**（A1 M5-1.b 切片 1）：抽 `RootsProvider` / `ProgressSink` / `PathResolver` trait，解除 `scheduler → bridge::AppState` 反向边。
5. **步骤 2**（A1 M5-1.b 切片 2）：B 类模块 seam 改造。
6. **步骤 c**（A1 M5-1.c 阶段二，可选）：根 `Cargo.toml [workspace]` 化（须 exclude `tauri-browser-tabs/`）。

> 步骤 0→0b→0a→1→2 不可换序：换序则步骤 0b 后的常量引用在步骤 0a 完成前编译失败，步骤 0a 完成的 core 边界在步骤 1 前不构成"反向边解除"。

---

## [W2 patched · 2026-09-06 13:45 CST] 契约常量集中 + M5-1.b 卡拣入 + M5-2.a 政策门就位

> **修订来源**：A0 M5-W2 Parallel Dispatch（`logs/checkpoints/A0-M5-W2-dispatch-20260906-1345.md`）A1 行 + A2 W2 切片 0b 落地（`712a14c`）+ A3 W2 M5-2.a 落地（`a654f0c`）+ A4/A5/A7/A8/A9 同期辅助 assist（`Lane-A4-M5-a2a-memory-20260906-1336-w2-slice.md` / `Lane-A5-M5-agent-skill-checkpoint-20260906-0915.md` / `A7-M5-graph-core-W1-checkpoint-20260906-1338.md` / `A8-M5-graph-ui-W1-final-20260906-1338.md` / `A9-M5-plugin-seam-20260906-1630.md`）。
> **修订原则**：A1 W2 把 W1 dispatch 承诺的"切片 0b 落 `domain.rs`"+"新增 `M5-1.b` 切卡"+"M5-2.a 政策门"三项实际拣入事实**回填到根卡**；其它子卡通过行内 `[W2 patched]` 段同步，本根卡仅做"完成索引"+修订原则陈述，不重写 §0~§11。

### W2 拣入事实（3 项落地 + 1 项切卡 + 5 项 assist 已落）

| # | 项 | 落地 commit | 来源 Lane | 影响文件 |
|---|---|---|---|---|
| W2-1 | M5-1.a 切片 0b 契约常量集中 | `712a14c feat(M5): centralize core contract constants` | A2 | `src-tauri/src/domain.rs` + `database.rs` / `script_runner.rs` / `security_policy.rs`（同步消除 V-7 副本）|
| W2-2 | M5-2.a MCP 政策门就位（contract-freeze slice）| `a654f0c feat(M5): add M5-2 MCP policy gate` | A3 | `scripts/check-mcp-policy.py`（504 行；13 项不变式 4 ACTIVE + 9 PENDING；`--self-test` / default / `--expect-pending` 三模式全 PASS）|
| W2-3 | A1 切卡 `M5-1.b` 拣入 | `712a14c`（同 commit，A1 patch 随 A0 拣入） | A1 | `logs/checkpoints/M5-20260906/M5-1.b-seam-trait-injection-and-b-extract.md`（288 行新增；A1 `Lane-A1-M5-W2-pre-M5-1.b-card-20260906-1330.patch`）|
| W2-4 | A4 A2A memory W2 slice spec 拣入 | `712a14c`（同 commit，A4 patch 随 A0 拣入） | A4 | `logs/assist/A4-M5-a2a-memory-20260906-1336-w2-slice.md`（201 行） + 同名 patch |
| W2-5 | A7 graph-core W1 checkpoint 拣入 | `712a14c` | A7 | `logs/checkpoints/A7-M5-graph-core-W1-checkpoint-20260906-1338.md`（94 行）|
| W2-6 | A8 graph-ui W1 final 拣入 | `712a14c` | A8 | `logs/checkpoints/A8-M5-graph-ui-W1-final-20260906-1338.md`（105 行）|
| W2-7 | A5 agent-skill W1 checkpoint 拣入 | `712a14c` | A5 | `logs/checkpoints/Lane-A5-M5-agent-skill-checkpoint-20260906-0915.md`（98 行）|
| W2-8 | A10 W1 security review 转 PASS | `712a14c`（重写） | A10 | `logs/assist/A10-M5-W1-security-review-20260906-1500.md`（220 行改写，转 PASS）|
| W2-9 | A9 plugin seam W2 拣入 | `712a14c` | A9 | `logs/checkpoints/Lane-A9-M5-plugin-seam-20260906-1630.md` + 同名 patch + `logs/assist/A9-M5-plugin-impl-seam-20260906-1630.md`（156 行）|
| W2-10 | A10 W2 MCP 设计/安全 review 拣入 | `712a14c` | A10 | `logs/assist/A10-M5-W2-mcp-design-security-review-20260906-1630.md`（131 行）|

### W2 不修订

- **三份主文档**（`详细设计与实施计划.md` / `后续需求TODO.md` / `AI-模型切换与接手清单.md`）：A0 W2 未指派 A1 改动；A1 W2 不动。
- **本根卡 §0~§11**：仅在顶部加本 `[W2 patched]` 段 + 头部时间戳行；W2 dispatch 阶段 A1 仅承担"事实回填"角色，不重写决策史。
- **`NEXT` 标记**：A1 W2 提交后 = M5-W3（待 A0 签发）；M5-W3 候选产品代码切片 `M5-1.b` 已由 A1 切卡并拣入。

### W2 阶段 A1 边界

- A1 W2 整包为**纯文档**：`logs/checkpoints/M5-20260906/M5-0-overview.md`（本根卡）头部更新 + 新增本 `[W2 patched]` 段；并随 `712a14c` 拣入 `M5-1.b-seam-trait-injection-and-b-extract.md` 切卡。
- A1 W2 **未写一行产品代码**；**未触** `src/`、`src-tauri/`、`package.json`、三份主文档、ACL/Capability、pre-merge。

---

## [W3 reconciliation · 2026-09-06 17:30 CST] W3 整包已 A0 拣入（A2 seam + A3 MCP 余下切片 + 6 份 assist）· 当前活跃 checkpoint 切到 M5-W4

> **W3 dispatch 依据**：`PARALLEL_COMMAND_BOARD.md` L133-167（**M5-W3 Parallel Dispatch**，Added 2026-09-06 17:10 CST by A0 after pushing through `712a14c`）。
> **W3 拣入事实**（A0 在 `f8f1f49` 一次性拣入 W3 整包；本卡仅"事后 reconciliation"）：
> - **A2 产品代码**：`f8f1f49 feat(M5): add core seam abstractions` — `src-tauri/src/core/seam.rs`（`ProgressSink` / `PathResolver` / `RootsProvider` 三个 trait；脱 Tauri）+ `src-tauri/src/core/mod.rs` 注册 + `src-tauri/src/bridge.rs` 加 44 行 Tauri 适配实现 + 2 Rust 单测。
> - **A3 产品代码**：`12f1cff feat(M5-2,W3): A3 MCP command-registry + global policy slice (no rmcp/server)` — `src-tauri/src/mcp.rs`（首期 7 命令 `MCP_COMMAND_REGISTRY` 冻结 + `lookup_mcp_command` + `evaluate_mcp_policy` fail-closed）+ `domain.rs` 新增 `McpCommandDef` / `McpCommandKind` / `MCP_CAPABILITY_V1` / `McpToolCall` / `McpPolicyDecision` + `check-mcp-policy.py` 加 `MCP_FS_TOOL_PATH_POLICY` ACTIVE + `pre-merge.sh` wire + 6 Rust 单测。
> - **A1 W3 reconciliation 整包**：`f8f1f49` 同 commit 拣入本卡 + 3 子卡头部 `[W3 active]` 段 + A1 W3 checkpoint `A1-M5-W3-reconciliation-20260906-1730.md`（166 行）+ 433 行 patch。
> - **A4 W3 delta**（`f8f1f49` 拣入）：`logs/assist/A4-M5-a2a-memory-20260906-1410-w3-delta.md`（98 行）—— 确认 M5-3.a 待 U-2 seam 已落（`f8f1f49` 解锁）+ M5-3.b 待 U-4 capability 真源（A3 落 `MCP_CAPABILITY_V1` 真源）。
> - **A5 W3 next-card**（`f8f1f49` 拣入）：`logs/assist/A5-M5-agent-skill-W3-next-card-20260906-1715.md`（118 行）—— 为 W4 A5 实施期做准备。
> - **A6 W3 UI data contract**（`e96c902` 拣入）：`logs/assist/A6-M5-agent-ui-20260906-1710.md`（258 行；TS data contract 锁定 SkillExec/AclLevel/AgentDef/StreamChunk 经 Tauri event；no-router 锚点 useLayoutStore.ts MainView+MOD_META；M5-6 UI 仍 BLOCKED on A16 M5-4/5）。
> - **A7 W3 graph core**（`f8f1f49` 拣入）：`A7-M5-graph-core-W3-checkpoint-20260906-1412.md`（81 行）+ `A7-M5-graph-core-W3-impl-card-20260906-1412.md`（200 行）—— graph core W4 实施期切卡冻结。
> - **A8 W3 graph UI**（`f8f1f49` 拣入）：`A8-M5-graph-ui-W3-card-20260906-1412.md`（117 行）—— graph UI W4 切卡冻结。
> - **A9 W3 plugin seam**（`f8f1f49` 拣入）：`A9-M5-plugin-W3-delta-20260906-1730.md`（119 行）+ checkpoint/patch —— plugin seam 对齐 A3 MCP registry + A5 Agent/Skill 边界。
> - **A10 W3 security review**（`f8f1f49` 拣入）：`A10-M5-W3-security-review-20260906-1730.md`（122 行）—— A2 seam / A3 MCP 复审 PASS。
> - **A11 W3 verification**（`bdb0602` 拣入）：`docs(A11): M5-W3 verification delta after W3 outputs` —— cargo test 329 / 6 项 policy 全部 self-test+default+pending PASS / cargo fmt 干净 / git diff --check CLEAN / npm run build PASS（index 162.50kB）/ pre-merge.sh ALL_PASS。
> - **A0 W4 dispatch**（`f7ad35a` 已 push `f8f1f49` 后签发）：PARALLEL_COMMAND_BOARD L134-168 *M5-W4 Parallel Dispatch* —— 本卡见下文 `[W4 active]` 段。

### W3 A1 整包交付

| # | 文件 | 修订 |
|---|------|-----|
| W3-1 | `M5-0-overview.md`（本根卡）| 头部时间戳加 W3 reconciliation + W4 active 行；本段事实回填 11 项落地 |
| W3-2 | `M5-1-core-workspace-split.md` | 头部状态行加 W3 PASS（`f8f1f49` seam）；下文 [W3 patched] 段标记 W3 拣入事实 |
| W3-3 | `M5-1.b-seam-trait-injection-and-b-extract.md` | 头部状态行加 W3 PASS；下文 [W3 reconciliation] 段标记 A2 seam 三 trait 实际落地 |
| W3-4 | `M5-2-rmcp-mcp-policy.md` | 头部状态行加 W3 PASS（`12f1cff` MCP 余下切片）；下文 [W3 reconciliation] 段标记 A3 `mcp.rs` + `MCP_FS_TOOL_PATH_POLICY` |
| W3-5 | `logs/checkpoints/A1-M5-W3-reconciliation-20260906-1730.md` | 新增：A1 W3 整包交付 checkpoint |

### W3 状态（全部已 PASS）

| 项 | 状态 | 来源 |
|---|------|------|
| W1（core boundary gate · slice 0a + 0b + keyring_store）| **PASS** | `854bc40` + `0d08016` |
| W2（constants 集中 + MCP 政策门 + 多 lane assist）| **PASS** | `712a14c` + `a654f0c` + `c4b0fb7`（A6 scheduler fixture）+ `1e114b6`（A11 W2 verification）|
| W3（A2 M5-1.b seam + A3 M5-2 余下切片 + 6 lane assist + A11 W3 verification）| **PASS · A0 拣入** | `f8f1f49` + `12f1cff` + `bdb0602` + `e96c902` |
| W4（A4 M5-3.a + A5 M5-4/M5-5 + 4 lane assist + 2 review）| **ACTIVE · 见下 [W4 active] 段** | `f7ad35a`（A0 W4 dispatch） |

---

## [W4 active · 2026-09-06 17:55 CST] 当前活跃 checkpoint 切到 M5-W4（A4 + A5 产品代码 lane · 其它 9 lane docs/review/support）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L134-168（**M5-W4 Parallel Dispatch**，Added 2026-09-06 17:55 CST by A0 after pushing through `f8f1f49`）。
> **事实摘要**：W3 整包已 A0 拣入（`f8f1f49` + `12f1cff` + `bdb0602` + `e96c902`）；A11 W3 验证 ALL_PASS。
> **W4 仅开两条产品代码 lane**：
> - **A4** *START PRODUCT CODE*：M5-3 A2A/agent memory KV **首切片** —— DTOs + 校验 + 容量/隐私 policy + pure store helpers 或 JSON 持久化壳（沿用 A2 seam `mvp_core::seam`）；无网络协议、无 background runtime。
> - **A5** *START PRODUCT CODE*：M5-4/M5-5 Agent/Skill **domain + command policy shell** —— `AgentDef` / `SkillDef` DTOs + 校验 + permission preview + policy script；不执行 skill、不加 command runtime（除非 source check/ACL/types/bridge/tests 同包完整）；无 second execution path、无 installer/network/download。
> **A1 W4 角色**：START DOCS ONLY — *"Reconcile W4 as active NEXT; mark W3 pushed and split M5-3/M5-4/M5-5 into next-card acceptance criteria."* —— 本卡顶部索引 + 3 张子卡（M5-3 / M5-4 / M5-5）增补 W4 next-card acceptance criteria 段。

### W4 A1 整包交付

| # | 文件 | 修订 |
|---|------|-----|
| W4-1 | `M5-0-overview.md`（本根卡）| 头部时间戳 + W3 reconciliation 段（11 项落地）+ W4 active 段（本段；索引 5 文件交付清单 + 2 lane 实施期硬约束摘要）|
| W4-2 | `M5-1-core-workspace-split.md` | 头部状态行 W3 PASS；[W3 patched] 段标记 A2 seam `f8f1f49` 落地；新增 [W4 status] 段（M5-1.b seam 已交付；M5-1.c workspace 化"可选/延后"已实际不再需要）|
| W4-3 | `M5-1.b-seam-trait-injection-and-b-extract.md` | 头部状态行 W3 PASS（`f8f1f49`）；[W3 reconciliation] 段标记 A2 三个 trait 实际落地 + A4 W4 实施期可消费 |
| W4-4 | `M5-2-rmcp-mcp-policy.md` | 头部状态行 W3 PASS（`12f1cff` MCP 余下切片）；[W3 reconciliation] 段标记 A3 `mcp.rs` + `MCP_FS_TOOL_PATH_POLICY` ACTIVE |
| W4-5 | `M5-3-a2a-bidir-agent-kv.md` | 新增 [W4 next-card acceptance criteria] 段（4 项 AC：DTOs/校验/容量+隐私 policy/JSON 持久化壳；5 项 hard stops）|
| W4-6 | `M5-4-agent-skill-runtime.md` | 新增 [W4 next-card acceptance criteria] 段（4 项 AC：AgentDef+SkillDef DTOs/validation/permission preview/policy script；5 项 hard stops）|
| W4-7 | `M5-5-agent-skill-commands.md` | 新增 [W4 next-card acceptance criteria] 段（3 项 AC：command policy shell/无 second execution path/permission preview 校验；5 项 hard stops）|
| W4-8 | `logs/checkpoints/A1-M5-W4-reconciliation-20260906-1755.md` | 新增：A1 W4 整包交付 checkpoint |

### W4 A4 / A5 实施期硬约束（与 W4 dispatch 承诺一致）

| # | 约束 | 来源 |
|---|------|------|
| W4-HS1 | **A4/A5 是 W4 唯一允许写产品代码的两条 lane**；A2/A3 变 SUPPORT/REVIEW ONLY；A6-A11 保持 docs/review/support | PARALLEL_COMMAND_BOARD L164 |
| W4-HS2 | **无 network protocol listener / rmcp server / plugin installer / 模型 provider 集成 / background agent runtime / npm 依赖 / GUI panel** 在 W4 | PARALLEL_COMMAND_BOARD L165 |
| W4-HS3 | **新 Tauri 命令必须 atomic**（source check + ACL + 前端 bridge/types + policy coverage + tests 同包）；若契约未完全 ready，**优先不加 command** | PARALLEL_COMMAND_BOARD L166 |
| W4-HS4 | **Stores 必须 bounded + privacy-filtered**：无 token/cookie/Authorization/body/日志 prompt secrets | PARALLEL_COMMAND_BOARD L167 |
| W4-HS5 | 所有 lane 必须从 `origin/master` pull，**不 push** | PARALLEL_COMMAND_BOARD L168 |
| W4-HS6 | A5 不执行 skill（"Do not execute skills yet"）；A4 不引入 network protocol 与 background runtime | PARALLEL_COMMAND_BOARD L153/L154 |

### W4 A1 硬停止

- **零产品代码**：A1 W4 整包**仅文档**（PARALLEL_COMMAND_BOARD L150 明示 *"no product code"*）。
- **不重写各子卡 §1~§11**：仅头部 [W3 reconciliation] 段 + W4 next-card acceptance criteria 段（不修订 §1~§11 决策史）。
- **不移动 `NEXT`**：`NEXT` 标记属 A0 调度权；A1 仅在头部状态行陈述"W4 是当前活跃 checkpoint"。
- **不动三份主文档**：A0 在 `f7ad35a` W4 dispatch 段中明示 A1 允许"three main docs"，但本轮 A1 选择**不动**——W4 修订仅落在 `logs/checkpoints/M5-20260906/*.md` 5 文件 + 1 新增 checkpoint；如需主文档调整留待 W4 收口或 A0 拣入期处理。
- **不提交 / 不 push**：A1 W4 整包交 A0 拣入合并。

### W4 状态（本卡涉及）

| 项 | 状态 | 来源 |
|---|------|------|
| W3（A2 M5-1.b seam + A3 M5-2 余下切片 + 6 lane assist）| **PASS · A0 拣入** | `f8f1f49` + `12f1cff` + `bdb0602` + `e96c902` |
| W4 A4 M5-3.a 实施 | **ACTIVE · 待 A4 实施** | PARALLEL_COMMAND_BOARD L153 |
| W4 A5 M5-4/M5-5 实施 | **ACTIVE · 待 A5 实施** | PARALLEL_COMMAND_BOARD L154 |
| W4 A2/A3 复审 + A6-A11 辅助 | **ACTIVE · 待 A2/A3/A6-A11 输出** | PARALLEL_COMMAND_BOARD L151-L160 |
| W4 A1 文档 reconciliation（本段 + 3 子卡 acceptance criteria + 本 checkpoint）| **本轮 W4 修订已完成** | 本 checkpoint |

---

## [W4 reconciliation · 2026-09-06 15:18 CST] W4 整包已 A0 拣入（`1610939`）· 当前活跃 checkpoint 切到 M5-W5

> **W4 拣入事实**（A0 在 `1610939 feat(M5): add agent memory and skill policy shells` 拣入；本卡仅"事后 reconciliation"）：
> - **A4 W4 产品代码**（`1610939` 同 commit）：`src-tauri/src/agent_memory.rs`（868 行新增；DTOs + 校验 + 容量/隐私 policy + JSON 持久化壳）+ `src-tauri/src/domain.rs`（148 行 AGENT_KV_* 常量单一真源）+ `src-tauri/src/main.rs`（mod agent_memory）+ `src-tauri/src/security_policy.rs`（88 行新增 5 ACTIVE 码位 + 6 Rust 单测）+ `scripts/check-agent-memory-policy.py`（303 行）+ `scripts/pre-merge.sh` wire + checkpoint `Lane-A4-M5-3-agent-memory-20260906-1510.md`（68 行）+ patch（1409 行）。
> - **A5 W4 产品代码**（`1610939` 同 commit）：`src-tauri/src/agent.rs`（104 行 AgentDef DTO + 校验 + permission preview）+ `src-tauri/src/skills.rs`（147 行 SkillDef DTO + 校验 + permission preview）+ `src-tauri/src/domain.rs`（同 148 行追加 AgentDef/SkillDef 常量）+ `scripts/check-agent-skill-policy.py`（410 行）+ `scripts/pre-merge.sh` wire + checkpoint `Lane-A5-M5-W4-20260906-1815.md`（78 行）。
> - **A1 W4 reconciliation 整包**（`1610939` 同 commit）：768 行 patch + 202 行 checkpoint `logs/checkpoints/A1-M5-W4-reconciliation-20260906-1755.md` + 6 子卡头部修订（M5-0/1/1.b/2/3/4/5）。
> - **A2 W4 seam review**（`1610939` 拣入）：`logs/assist/A2-M5-W4-seam-review-20260906-1454.md`（75 行）—— A4/A5 对 `mvp_core::seam` 使用复审 PASS。
> - **A3 W4 MCP compat**（`d71f558` 拣入后随 `1610939` 整包入）：`logs/assist/A3-M5-W4-mcp-compat-20260906-1805.md` —— A4/A5 vs M5-2 registry+policy 复审 PASS。
> - **A7 W4 graph core delta**（`1610939` 拣入）：`logs/assist/A7-M5-W4-checkpoint-20260906-1455.md`（80 行）+ `A7-M5-W4-graph-core-delta-20260906-1455.md`（126 行）—— M5-7/M5-8 W5 实施期切卡冻结。
> - **A8 W4 graph UI delta**（`1610939` 拣入）：`logs/assist/A8-M5-W4-graph-ui-delta-20260906-1454.md`（80 行）—— M5-9 仍 docs-only 锚定。
> - **A9 W4 plugin manifest**（`1610939` 拣入）：`logs/assist/A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md`（169 行）+ checkpoint/patch —— plugin manifest 对齐 A3 MCP registry + A5 Agent/Skill 边界。
> - **A10 W4 security review**（`1610939` 拣入）：`logs/assist/A10-M5-W4-security-review-20260906-1805.md`（95 行）—— A4/A5 复审 PASS。
> - **A11 W4 verification**（`13c5279` 拣入后随 `1610939` 整包入）：`docs(A11): M5-W4 verification delta after W4 outputs` —— cargo test 全部 PASS / 8 项 policy 全部 self-test+default PASS / pre-merge ALL_PASS。
> - **A6 W4 UI contract**（`562efb9` 拣入）：`docs(A6): M5-W4 convert A5 domain -> UI data contract + panel state plan` —— A5 domain 转 UI 桥接；M5-6 仍 BLOCKED on A6 M5-5 W5 实施期。
> - **A0 W5 dispatch**（`0e76a89` 已 push `1610939` 后签发）：PARALLEL_COMMAND_BOARD L135-171 *M5-W5 Parallel Dispatch* —— 本卡见下文 `[W5 active]` 段。

### W4 A1 整包交付（事后 reconciliation）

| # | 文件 | 修订（`1610939` 已拣入）|
|---|------|----------------------|
| W4-1 | `M5-0-overview.md`（本根卡）| 头部时间戳加 W4 拣入 + W5 active 行；本段事实回填 13 项落地 |
| W4-2 | `M5-1-core-workspace-split.md` | 头部状态行 W3 PASS；[W3 patched] 段标记 A2 seam `f8f1f49` 落地；新增 [W4 status] 段 |
| W4-3 | `M5-1.b-seam-trait-injection-and-b-extract.md` | 头部状态行 W3 PASS（`f8f1f49`）；[W3 reconciliation] 段标记 A2 三个 trait 实际落地 + A4 W4 实施期可消费 |
| W4-4 | `M5-2-rmcp-mcp-policy.md` | 头部状态行 W3 PASS（`12f1cff` MCP 余下切片）；[W3 reconciliation] 段标记 A3 `mcp.rs` + `MCP_FS_TOOL_PATH_POLICY` ACTIVE |
| W4-5 | `M5-3-a2a-bidir-agent-kv.md` | 新增 [W4 next-card acceptance criteria] 段（4 项 AC + 5 项 hard stops）|
| W4-6 | `M5-4-agent-skill-runtime.md` | 新增 [W4 next-card acceptance criteria] 段（4 项 AC + 5 项 hard stops）|
| W4-7 | `M5-5-agent-skill-commands.md` | 新增 [W4 next-card acceptance criteria] 段（3 项 AC + 5 项 hard stops）|
| W4-8 | `logs/checkpoints/A1-M5-W4-reconciliation-20260906-1755.md` | 新增：A1 W4 整包交付 checkpoint（202 行）|
| W4-9 | `logs/checkpoints/Lane-A1-M5-W4-reconciliation-20260906-1755.patch` | 新增：768 行 patch |

### W4 状态（全部已 PASS）

| 项 | 状态 | 来源 |
|---|------|------|
| W1（core boundary gate · slice 0a + 0b + keyring_store）| **PASS** | `854bc40` + `0d08016` |
| W2（constants 集中 + MCP 政策门 + 多 lane assist）| **PASS** | `712a14c` + `a654f0c` + `c4b0fb7` + `1e114b6` |
| W3（A2 M5-1.b seam + A3 M5-2 余下切片 + 6 lane assist + A11 W3 verification）| **PASS** | `f8f1f49` + `12f1cff` + `bdb0602` + `e96c902` |
| W4（A4 M5-3.a agent_memory + A5 M5-4/M5-5 agent/skills + 4 lane assist + 2 review + A1 reconciliation 整包）| **PASS · A0 拣入** | `1610939` + `d71f558` + `13c5279` + `562efb9` |
| W5（A6 M5-6 + A7 M5-7/M5-8 + 4 lane assist + 2 review）| **ACTIVE · 见下 [W5 active] 段** | `0e76a89`（A0 W5 dispatch） |

---

## [W5 active · 2026-09-06 18:35 CST] 当前活跃 checkpoint 切到 M5-W5（A6 + A7 产品代码 lane · 其它 9 lane docs/review/support）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L135-171（**M5-W5 Parallel Dispatch**，Added 2026-09-06 18:35 CST by A0 after pushing through `1610939`）。
> **事实摘要**：W4 整包已 A0 拣入（`1610939`）；A4 W4 `agent_memory.rs` 868 行落地；A5 W4 `agent.rs` + `skills.rs` 落地；A11 W4 验证 ALL_PASS。
> **W5 仅开两条产品代码 lane**：
> - **A6** *START PRODUCT CODE*：M5-6 Agent/Skill UI **pure logic + panel shell** —— 校验展示、permission preview、capability 列表、empty/error 状态；优先 helper module + headless logic test；**不执行 skill、不装插件、不调 live runtime**。
> - **A7** *START PRODUCT CODE*：M5-7/M5-8 graph model/store **policy slice** —— `GraphNode` / `GraphEdge` DTOs + 容量/redaction 规则 + pure graph store/query helpers + policy script；**无 graph UI、无 agent 消费、无 background graph rebuild workers、无 network**。
> **A1 W5 角色**：START DOCS ONLY — *"Reconcile W5 as active NEXT; mark W4 pushed and tighten M5-6/M5-7/M5-8 acceptance criteria."* —— 本卡顶部索引 + 4 张子卡（M5-6 / M5-7 / M5-8 / M5-9）增补 W5 next-card acceptance criteria 段。

### W5 A1 整包交付（计划）

| # | 文件 | 修订 |
|---|------|------|
| W5-1 | `M5-0-overview.md`（本根卡）| 头部时间戳 + W4 reconciliation 段（13 项落地事实）+ W5 active 段（本段；索引 7 文件交付清单 + 2 lane 实施期硬约束摘要）|
| W5-2 | `M5-6-agent-skill-ui.md` | 新增 [W5 next-card acceptance criteria] 段（4 项 AC：UI 纯逻辑 helper / 校验展示 / permission preview 桥 / empty+error 状态；5 项 hard stops）|
| W5-3 | `M5-7-graph-model-extract.md` | 新增 [W5 next-card acceptance criteria] 段（4 项 AC：GraphNode+GraphEdge DTO / 容量上界 / redaction 规则 / pure store helpers；5 项 hard stops）|
| W5-4 | `M5-8-graph-store-query.md` | 新增 [W5 next-card acceptance criteria] 段（4 项 AC：pure graph query helpers / policy script / pre-merge wire / 0 命令注册；5 项 hard stops）|
| W5-5 | `M5-9-graph-ui-agent-consume.md` | 新增 [W5 status] 段（A8 W5 仍 SUPPORT DOCS ONLY；W5 不动 M5-9 决策史）|
| W5-6 | `logs/checkpoints/A1-M5-W5-reconciliation-20260906-1835.md` | 新增：A1 W5 整包交付 checkpoint |
| W5-7 | `logs/checkpoints/Lane-A1-M5-W5-reconciliation-20260906-1835.patch` | 新增：A1 W5 整包 patch |

### W5 A6 / A7 实施期硬约束（与 W5 dispatch 承诺一致）

| # | 约束 | 来源 |
|---|------|------|
| W5-HS1 | **A6/A7 是 W5 唯一允许写产品代码的两条 lane**；A2/A3/A4/A5 变 SUPPORT/REVIEW ONLY；A8/A9 仍 SUPPORT DOCS ONLY；A10 START REVIEW；A11 START VERIFICATION | PARALLEL_COMMAND_BOARD L165 |
| W5-HS2 | **A6 不得加** execution runtime / installer / network/model calls / 新后端 commands | PARALLEL_COMMAND_BOARD L166 |
| W5-HS3 | **A7 不得加** live UI / agent consumption / background graph rebuild workers / network access | PARALLEL_COMMAND_BOARD L167 |
| W5-HS4 | **新 Tauri 命令必须 atomic**（source check + ACL + 前端 bridge/types + policy coverage + tests 同包）；若契约未完全 ready，**优先不加 command** | PARALLEL_COMMAND_BOARD L168 |
| W5-HS5 | **Stores/maps/lists 必须 bounded + privacy-filtered**：无 token/cookie/Authorization/body/日志 prompt secrets | PARALLEL_COMMAND_BOARD L169 |
| W5-HS6 | 所有 lane 必须从 `origin/master` pull，**不 push** | PARALLEL_COMMAND_BOARD L170 |
| W5-HS7 | **无新 npm 依赖**（A6 须复用现有 Vue 3 + Pinia + D3.js）| A6 W5 dispatch L156 + W4-HS2 |

### W5 A1 硬停止

- **零产品代码**：A1 W5 整包**仅文档**（PARALLEL_COMMAND_BOARD L151 明示 *"One reconciliation checkpoint; no product code"*）。
- **不重写各子卡 §1~§11**：仅头部 [W5 next-card acceptance criteria] 段（不修订 §1~§11 决策史）。
- **不移动 `NEXT`**：`NEXT` 标记属 A0 调度权；A1 仅在头部状态行陈述"W5 是当前活跃 checkpoint"。
- **不动三份主文档**：A0 W5 dispatch L151 虽允许"three main docs"，但本轮 A1 选择**不动**——W5 修订仅落在 `logs/checkpoints/M5-20260906/*.md` 5 文件 + 1 新增 checkpoint；如需主文档调整留待 W5 收口或 A0 拣入期处理。
- **不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json**：A1 W5 严格不动。
- **不提交 / 不 push**：A1 W5 整包交 A0 拣入合并。

### W5 状态（本卡涉及）

| 项 | 状态 | 来源 |
|---|------|------|
| W4（A4 M5-3.a + A5 M5-4/M5-5 + 4 lane assist + 2 review + A1 reconciliation 整包）| **PASS · A0 拣入** | `1610939` + `d71f558` + `13c5279` + `562efb9` |
| W5 A6 M5-6 实施 | **ACTIVE · 待 A6 实施** | PARALLEL_COMMAND_BOARD L156 |
| W5 A7 M5-7/M5-8 实施 | **ACTIVE · 待 A7 实施** | PARALLEL_COMMAND_BOARD L157 |
| W5 A2/A3/A4/A5 复审 + A8/A9 docs + A10 review + A11 verification | **ACTIVE · 待各 lane 输出** | PARALLEL_COMMAND_BOARD L152-L161 |
| W5 A1 文档 reconciliation（本段 + 4 子卡 acceptance criteria + 本 checkpoint）| **本轮 W5 修订已完成** | 本 checkpoint |

---

## 0. 本包目录结构

```
logs/checkpoints/M5-20260906/
├── M5-0-overview.md            ← 本文件（总览/索引/依赖/合并顺序）
├── M5-1-core-workspace-split.md        ← #7 → M5-1  核心 workspace 下沉
├── M5-2-rmcp-mcp-policy.md             ← #7 → M5-2  内嵌 rmcp + McpGlobalPolicy
├── M5-3-a2a-bidir-agent-kv.md          ← #7 → M5-3  A2A 双向 + agent_kv
├── M5-4-agent-skill-runtime.md         ← #12 → M5-4 Agent/Skill runtime
├── M5-5-agent-skill-commands.md        ← #12 → M5-5 Agent/Skill 命令与权限
├── M5-6-agent-skill-ui.md              ← #12 → M5-6 Agent/Skill UI
├── M5-7-graph-model-extract.md         ← #13 → M5-7 图模型与两阶段抽取
├── M5-8-graph-store-query.md           ← #13 → M5-8 图存储与查询
├── M5-9-graph-ui-agent-consume.md      ← #13 → M5-9 图谱 UI 与 Agent 消费
├── M5-10-plugin-manifest-lifecycle.md  ← #15 → M5-10 插件 manifest 与生命周期
├── M5-11-plugin-commands-isolation.md  ← #15 → M5-11 插件命令与隔离
├── M5-12-plugin-ui.md                  ← #15 → M5-12 插件管理 UI
├── M5-13-verification-matrix.md        ← 跨 M5 验证矩阵（合并门禁/单测/反向用例）
└── M5-14-debt-ledger.md                ← M5 债务账（不在本批解决项）
```

每张 M5-x 子卡采用统一骨架（与 A1 M4 展开卡范式一致）：

```
0. 任务卡编号 / 需求映射 / 责任 Lane 候选
1. GOAL           — 子卡目标（1 句）
2. READ           — 必读文件
3. WRITE          — 必改文件候选（不写实现，仅列契约落地位置）
4. FORBID         — 红线
5. COMMANDS       — 验收命令（实现期跑）
6. PASS_CRITERIA  — 通过判据
7. FAIL_ACTION    — 失败动作
8. DOC_BACKWRITE  — 需回写的主文档段落
9. COMMIT / NEXT  — 提交与下一卡
```

---

## 1. M5-W0 / M5-W1 节奏

| Wave | 状态 | 内容 | 责任 Lane | 准入 |
|---|---|---|---|---|
| **M5-W0**（**当前**） | 进行中 | 展开 M5-1~M5-12 子卡 + 架构预研（仅 docs） | A1（展开）+ A2/A5/A6/A7/A9（prework 文档）+ A10（安全复核）+ A11（验证矩阵） | M4 PASS（✅ `a1a2061`） |
| **M5-W1**（待 A0 签发） | 锁定 | 签发 M5-1（`core workspace 下沉`）评估边界 → 决定 M5-2/3/4/5/7/8/10/11 的领取顺序与卡合并 | A0 签发 + 候选 Lane A13~A20（见 §6） | M5-W0 集成 + A0 签发 NEXT |
| **M5-W2~R**（远期） | 锁定 | 依 M5-1 边界签发各 M5-x 实现批 + UI 批 | A14/A15/A16/A17/A18/A19/A20 | 各前置冻结 |

> **本卡包覆盖范围**：M5-W0 全部。**不**为 M5-W1 写实现卡；**不**签字 Lane 号；**不**签合并顺序（合并顺序交 §5 草拟、A0 拍）。

---

## 2. M5 12 子卡索引（与 WBS 一致）

| WBS 编号 | 需求 | 标题 | 子卡文件 | 核心 A*-M5 prework 输入 | 建议 Lane 候选 |
|---|---|---|---|---|---|
| **M5-1**  | #7  | 核心 workspace 下沉（core/web/mcp/cli） | `M5-1-core-workspace-split.md` | `A2-M5-core-20260906-0749.md`（A 路径 · 10 步） | A13 |
| **M5-2**  | #7  | 内嵌 rmcp + `McpGlobalPolicy` + 首组 MCP 工具 | `M5-2-rmcp-mcp-policy.md` | `M5-7.a-prework-20260902-1055.md` + `A2-M5-core-*.md`（共用 capability.rs） | A14 |
| **M5-3**  | #7  | A2A 双向 + `agent_kv`（多方言归一） | `M5-3-a2a-bidir-agent-kv.md` | `M5-7.a-prework-*.md` §4.3 A2A 草案 | A15 |
| **M5-4**  | #12 | Agent/Skill runtime（`AgentDef`/`SkillDef`） | `M5-4-agent-skill-runtime.md` | `M5-12.a-prework-20260902-1055.md` | A16 |
| **M5-5**  | #12 | Agent/Skill 命令与权限（安装/执行/对话） | `M5-5-agent-skill-commands.md` | `M5-12.a-prework-*.md` §4.1-4.2 | A16（同 A16 拆卡） |
| **M5-6**  | #12 | Agent/Skill UI（对话/管理/权限预览） | `M5-6-agent-skill-ui.md` | A5/A6 prework（当前空） | A19 |
| **M5-7**  | #13 | 图模型与可追溯抽取（两阶段 + `graph.rs`） | `M5-7-graph-model-extract.md` | `M5-13.a-prework-20260902-1055.md` | A17 |
| **M5-8**  | #13 | 图存储与查询（邻接表 + DDL + 命令） | `M5-8-graph-store-query.md` | `A9-M5-graph-store-contract-20260906-0700.md` + `A9-M5-graph-scheduler-feed-*.md` | A17 |
| **M5-9**  | #13 | 图谱 UI 与 Agent 消费（力导向 + RAG 注入） | `M5-9-graph-ui-agent-consume.md` | A8 prework（当前空） | A19 |
| **M5-10** | #15 | 插件 manifest 与生命周期（签名/load/unload） | `M5-10-plugin-manifest-lifecycle.md` | `M5-15.a-prework-20260902-1055.md` + `A9-M5-plugin-form-feasibility-*.md` | A18 |
| **M5-11** | #15 | 插件命令与隔离（安装/权限/审计/卸载） | `M5-11-plugin-commands-isolation.md` | `M5-15.a-prework-*.md` §4 + `A9-M5-plugin-form-feasibility-*.md` | A18 |
| **M5-12** | #15 | 插件管理 UI（权限清单/配置/状态） | `M5-12-plugin-ui.md` | （无 prework；A18 实施期补） | A19 |

> **Lane 号 A13~A20 为 A9 候选提案**（见 `A9-M5-A13plus-cards-20260906-0010.md`），A1 不强行指定；A0 在 M5-W1 签发时定。

---

## 3. 命名漂移警示（沿用 A9 prework §2）

需求 # 号 与 WBS 编号 在历史 prework 文件名中混用，本批 A1 卡统一以 WBS 编号为准；如需引用历史 prework 文档，按下表对照：

| 需求 # | WBS 编号 | 既有 prework 资产 |
|---|---|---|
| #7  A2P/A2A     | M5-1 / M5-2 / M5-3  | `M5-7.a-prework-20260902-1055.md` · `A2P-A2A-protocol-taskcard-20260902-1146.md` |
| #12 Agent/Skill  | M5-4 / M5-5 / M5-6  | `M5-12.a-prework-20260902-1055.md` · `agent-skill-contract-taskcard-20260902-1146.md` |
| #13 知识图谱    | M5-7 / M5-8 / M5-9  | `M5-13.a-prework-20260902-1055.md` · `graph-model-taskcard-20260902-1146.md` |
| #15 插件系统    | M5-10 / M5-11 / M5-12 | `M5-15.a-prework-20260902-1055.md` · `plugin-permission-taskcard-20260902-1146.md` · `plugin-runtime-taskcard-20260902-1146.md` |

**不**再用 `M5-7/12/13/15.a` 这种"看起来像 WBS"实为"需求 #"的文件名新建文档。

---

## 4. 依赖图（A1 视角）

```
                ┌────────── M4 PASS ✅ a1a2061 ──────────┐
                │                                         │
                ▼                                         ▼
       A2 prework 已有（A 路径）         A9 prework（3 份契约 + split + A13plus）
                │                                         │
                └──────────────┬──────────────────────────┘
                               ▼
                    ┌──── M5-1 workspace 下沉评估 ────┐   ← 必最先（A0 签发）
                    │  决 5 成员边界（core/web/mcp/cli）│
                    │  决 capability.rs 放置位置       │
                    └────────────┬────────────────────┘
                                 ▼
        ┌────────────────────────┼────────────────────────┐
        ▼                        ▼                        ▼
  M5-2 rmcp + Policy      M5-4 Agent/Skill runtime    M5-7 图模型与抽取
  (复用 A3/A4 DB API)     (复用 M2-4 run_script)      (依赖 A3/A4 + A7 事件)
        │                        │                        │
        ▼                        ▼                        ▼
  M5-3 A2A + agent_kv     M5-5 Agent/Skill 命令    M5-8 图存储与查询
                          (与 M5-2 共用 capability.rs)  (复用 M5-1 workspace)
                                │                        │
                                ▼                        ▼
                          M5-6 Agent/Skill UI      M5-9 图谱 UI + Agent 消费
                                                      (依赖 M5-4 RAG 注入)

  ┌──── 独立分支：#15 插件系统 ────────────────────────────┐
  │  M5-10 manifest 生命周期（先 A0 裁形态①/②/③）          │
  │       ↓                                               │
  │  M5-11 命令与隔离（独立 stdio 通道？or webview 二开？） │
  │       ↓                                               │
  │  M5-12 插件管理 UI                                     │
  └────────────────────────────────────────────────────────┘

  ▼ 横切依赖
  - M5 全部新命令（graph_*/skill_*/plugin_*/mcp_*/a2a_*）一律插 `list_artifact_images` 之前（坑位②）
  - M5 全部新 capability 走 `capabilities/default.json` 通用权限或新建专用（待 M5-1 决）
  - M5 全部审计走 `audit.json` 1000 上限（K5）+ 各自独立日志（mcp-calls.json / skill-runs.json / plugin-invokes.json）
  - M5 全部退出收口挂 ShutdownCoordinator（沿用 M0-2 范式，索引依赖单测断言）
  - M5 全部新增 schema_version 走 `atomic_write`（沿用 M3 范式）
  - A6 已冻结：新建任务默认 `enabled=false`；tick 内禁写审计；判重真相源 `tasks.json.last_fired_at`
  - A7 已冻结：`stop-scheduler` 注册在 `stop-background-workers` 之后；图谱维护任务复用此通道
```

---

## 5. 合并顺序（A1 草拟，交 A0 拍）

> **不签字**。仅给"按依赖最小化"的草拟顺序；A0 在 M5-W1 签发时定。

| 序 | 卡 | 关键产出 | 依赖 | 与 A9 候选 A13~A20 映射 |
|---|---|---|---|---|
| 1 | M5-1 | 5 成员 workspace + capability.rs 位置 + 主应用可构建性 | M4 PASS ✅ | A13 |
| 2 | M5-2 | `rmcp` server 骨架 + `McpGlobalPolicy` + 首期能力白名单 | A3/A4 DB API + M5-1 | A14 |
| 2' | M5-4 | `AgentDef`/`SkillDef` + `skill_runtime.rs` + `capability.rs` 共用首版 | A13 + M2-4 | A16（与 M5-2 并行需先划模块边界） |
| 3 | M5-8 | 图邻接表 + `GraphQuery` DTO + `graph_*` 命令 | A3/A4 DB API + M5-1 | A17 |
| 3' | M5-5 | `skill_*`/`agent_chat` 命令 + 二次确认闸门 + `skill-runs.json` | M5-2 (capability) + M5-4 | A16 |
| 4 | M5-3 | A2A 协议 + `agent_kv` | M5-2 (能力层) | A15 |
| 4' | M5-7 | `graph.rs` 抽取器 + `GraphEvent` 上游钩子 | A7 事件 + M5-8 | A17 |
| 5 | M5-10 | 插件形态裁定（默认形态③声明式）+ `PluginManifest` | A0 形态裁定 | A18 |
| 5' | M5-11 | 插件命令 + 权限强制层 + `plugin-invokes.json` | M5-2 (capability) + M5-10 | A18 |
| 6 | M5-6 / M5-9 / M5-12 | UI 三件套（对话/图谱/插件） | 各后端 DTO 稳定 | A19 |
| 滚动 | M5 安全审查 | 各批次的红线复审 | 滚动 | A20 |

---

## 6. 与 A9 候选卡（A13~A20）的差

A9 候选卡（`A9-M5-A13plus-cards-20260906-0010.md`）与本批 A1 卡基本一致，差异如下：

| 项 | A9 候选卡 | A1 本批卡 | 处置 |
|---|---|---|---|
| Wave 标注 | 文档 R/R+1/R+2/R+3/滚动 | 同 A9 | 沿用 |
| Lane 号 | A13~A20 占位 | 不写 Lane 号（**A0 签发时定**） | A1 边界（避免与 A0 冲突） |
| 卡的颗粒度 | 一卡跨多 WBS（如 A14 跨 M5-2） | 一卡 = 一个 WBS（更细） | A1 比 A9 更细；M5-W1 由 A0 决定是否合并相邻 WBS |
| `capability.rs` 放置 | 隐含在 A14/A16 | M5-1 子卡显式列为"必最先决" | A1 更强调 |
| 插件形态决策 | 交 A0 | 同 A9（默认形态③） | 一致 |
| 债务账 | 未列 | `M5-14-debt-ledger.md` 显式分账 | A1 补 |

---

## 7. 红线总览（M5 全包适用）

> 继承 K 系列（详细设计）+ A6/A7 冻结 + 坑位② ⑤：

1. **禁第二执行路径**：M5 全部执行体（Agent 调用工具、Skill 运行、图维护任务、插件调用脚本）**只走** M2-4 `script_runner` 单通道；不引 `std::process::Command` 第二路径；插件形态①（独立 stdio）已否决。
2. **禁 npm 分包**：`rmcp` 必须纯 Rust 内嵌（`详细设计 §7`）；不引 `npx` 任何东西。
3. **能力白名单共用一份**：A2P / A2A / Skill / Plugin / Agent **共用** `src-tauri/src/capability.rs`；禁止各写一份漂移。
4. **凭据不入图谱/日志/审计**（K3）：LLM Key、Keyring 条目、API Token 不进入 `GraphNode.props` / `agent_kv.json` / `audit.json`。
5. **审计不刷爆**（K5）：高频调用走独立文件（`mcp-calls.json` / `skill-runs.json` / `plugin-invokes.json`），每文件独立 1000 上限；`audit.json` 仅记低频关键事件。
6. **新命令入 ACL**：M5 全部新命令（`graph_*`/`skill_*`/`plugin_*`/`mcp_*`/`a2a_*`）一律插 `list_artifact_images` 之前（坑位②）。
7. **退出收口挂协调器**：M5 各 `*Shutdown`（Graph/Skill/Agent/Plugin/MCP/A2A）必须挂 `ShutdownCoordinator`，注册序索引依赖需单测断言（沿用 `O-A1-5` 提醒）。
8. **新建任务默认 `enabled=false`**（R-A6-1）：图谱维护任务、Agent 周期任务、Skill 调度触发一律默认 `enabled=false`，显式开启。
9. **`withGlobalTauri=true` 红线**：插件 webview **不得**接触 `window.__TAURI__` 全局；Tauri v2 不支持按 webview 关全局时，**形态②插件放弃**，改形态③声明式（参 `A9-M5-plugin-form-feasibility-20260906-0700.md`）。
10. **`atomic_write` 唯一**：图 schema 版本、agent_kv、plugin_grants、mcp_policy、a2a_tasks 等所有持久化走 `session::atomic_write`（沿用 M3 范式）。
11. **派生索引层原则**（图谱）：`GraphNode.props` 不存主数据正文；可重建；`source=Manual` 优先不被自动抽取覆盖。
12. **Skill 禁内联**（K6）：`SkillDef.exec` 仅 `ScriptRef` / `CommandRef` / `Sequence`（无内联 shell 字符串）；执行体复用 `run_script` 走 M2-3.a 危险参数校验。

---

## 8. 验证矩阵入口

详见 `M5-13-verification-matrix.md`：

- 协议契约测试（A2P 消息格式 / A2A 幂等 / MCP capability 解析）
- 隔离与权限测试（Skill 越权 / 插件 manifest_hash 变更失效 / Agent 白名单拒绝）
- 集成测试（GraphEvent 上游写 → 抽取 → 查询）
- 恢复测试（`graph.db` 损坏备份 / agent_kv 容量满策略 / mcp-calls.json 滚动裁剪）
- 性能基线（MCP 工具响应 / 图查询 depth=2 万节点 / LLM token 节流）
- 升级/回滚测试（schema_version 迁移 / 插件更新 manifest_hash 重置授权）

---

## 9. 债务账入口

详见 `M5-14-debt-ledger.md`：

- 形态② 插件（webview 内）暂时**不实现**（`withGlobalTauri` 全局未按 webview 关闭前）
- `withGlobalTauri` 全局化影响 `src/stores/useSystemStore.ts:84` 唯一引用——M5-1 评估期一并封/迁
- A2A 首期是否真双向（仅"被调"or"既可被调又可委派"）——A0 拍
- 智能期图抽取（`source=ai`）首期仅留 trait，**不实现** LLM 触发
- LLM 用量配额/费用统计首期是否做
- `Similar` 关系相似度算法与阈值首期只做同 hash 精确匹配
- `graph_export("graphml")` 首期返回"暂不支持"
- 图谱库 `props` JSON 反序列化容错（首期严格 schema，宽容版本做迁移期）

---

## 10. 回写计划（按接手清单 §5 模板）

A1 checkpoint 文档另存 `logs/checkpoints/M5-A1-expansion-20260906-0800.md`（与 A0-M5-W0-dispatch 同级），含：

```
CHECKPOINT=M5-A1-expansion
STATUS=PASS（待 A0 集成后定）
EXECUTOR=A1 (CodeBuddy / M3-mini)
MODEL=M3-mini
ROUTE=AI:DEEP
MODEL_DEVIATION=none
COMMIT=<待 A0 拣入后填>
VERIFY=见 §5.2 命令全 PASS（仅文档工作树）
NEXT=M5-W1（待 A0 签发）
```

---

## 11. FORBID 遵守记录

- 未写产品代码（`src/`、`src-tauri/`、`package.json` 一字未动）
- 未触 `scripts/pre-merge.sh`、未触三份主文档
- 未触 `permissions/default-commands.toml`、未触 `capabilities/default.json`
- 未移动 `NEXT`（仍 `M5-W0`，A1 写完 14 张 docs 后 NEXT 不变）
- 未提交、未 push（本卡包为 A0 待拣入的待选文档）
- 14 张 docs 全新增独立文件，与 A2/A5/A6/A7/A9 既有 prework 文档无文件交集
- 所有"已冻结"事实均以来源 file:line 标注（继承自 prework 资产，本批仅做整合）
