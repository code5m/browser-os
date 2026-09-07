# M5 协议与智能生态 — 任务卡展开（Lane A1 · M5-W0 + W1 + W2 + W3 reconciliation + W4 active + W5 reconciliation + W6 reconciliation + W7 reconciliation + W8 reconciliation + W8 active + W9 active）

> 生成：2026-09-06 08:00 CST · Lane A1（M5-W0 · docs only）
> W1 修订：2026-09-06 08:50 CST · Lane A1（M5-W1 · docs-only reconciliation）
> W2 修订：2026-09-06 13:45 CST · Lane A1（M5-W2 · docs-only reconciliation，对齐 `712a14c`）
> W3 修订：2026-09-06 17:30 CST · Lane A1（M5-W3 · docs-only reconciliation，对齐 `f8f1f49` + `12f1cff` + `bdb0602`）
> W4 拣入：2026-09-06 15:18 CST · A0 在 `1610939 feat(M5): add agent memory and skill policy shells` 中拣入 A1 W4 reconciliation 整包（768 行 patch + 202 行 checkpoint + 6 子卡修订）
> W5 拣入：2026-09-06 19:00 CST · A0 在 `4b438ef feat(M5): add graph model store policy slice` + `f99d2eb feat(A6): M5-6 Agent/Skill UI pure logic + panel shell (W5)` + `1a2c9cd docs(A11): M5-W5 verification delta after A6/A7 W5 outputs` 中拣入 A6 W5 面板壳 + A7 W5 graph model store policy slice + A11 W5 验证 delta
> W6 active：2026-09-06 19:25 CST · Lane A1（M5-W6 · docs-only reconciliation，标 M5-9/M5-10/M5-11 为 W6 实施期卡，标 M5-12 仍 docs-only）
> W6 拣入：2026-09-07 07:19 CST · A0 在 `5f92ece feat(M5): add graph UI and plugin policy slices` 中拣入 A8 W6 graph UI + A9 W6 plugin policy slice + 5 份 W6 assist + A1 W6 reconciliation 整包 + M5-0/9/10/11/12 头部修订
> W7 active：2026-09-07 00:50 CST · Lane A1（M5-W7 · docs-only reconciliation，标 A3 M5-2 read-only MCP bridge + A5 Agent/Skill read-only bridge 为 W7 实施期卡；其它 9 lane 仍 docs/review/support；W7 = 窄 read-only command bridge wave，**不**做 runtime activation）· **A1 W7 reconciliation 整包**（M5-0/9/10/11/12/13/14 修订 + A1 W7 checkpoint + A1 W7 patch）**未**进 master（A0 W7 拣入期未消，详见 [W7 reconciliation] 段）
> W7 拣入：2026-09-07 09:45 CST · A0 在 **`6c1f30e feat(M5-W7,A3): read-only MCP registry/policy bridge commands`** + **`daa10f6 docs(A11): M5-W7 verification delta — pre-merge FAIL (3 red lights from 5f92ece integration hygiene)`** + **`a29b796 docs(A6): M5-W7 UI wiring note for A5 read-only Agent/Skill command bridge`** 三段 commit 中拣入 A3 W7 `mcp_policy_get / mcp_registry_list / mcp_capability_preview` 3 命令 + A11 W7 pre-merge FAIL 3 red lights（W7-1 cargo test 编译中断 / W7-2 cargo fmt / W7-3 build metrics warnings_increased；同源于 5f92ece 集成卫生）+ A6 W7 UI wiring for A5 read-only bridge；**A1 W7 reconciliation 整包未在 W7 拣入期内消**，工作树留待 W8 整包合并拣入
> W8 active：2026-09-07 09:45 CST · Lane A1（M5-W8 · docs-only reconciliation，**Excluding-A3 dispatch**：A3 在 W8 是 HOLD/NO ASSIGNMENT 直至 A0 解决其 local commit boundary；A5 START PRODUCT CODE 硬化 Agent/Skill read-only bridge；A6 START UI DOCS/LOGIC 实施 Agent/Skill 面板消费 + 纯 UI helper tests；A7 START DOCS/GRAPH BRIDGE PLAN ONLY；A8 START UI POLISH/TEST ONLY；A9 START POLICY REVIEW ONLY；A10 START SECURITY BATCH REVIEW；A11 START VERIFICATION BATCH；A1/A2/A4 docs/review ONLY；**W8 = 硬化 + 复审 + 收口波，无 MCP 产品代码工作**）
> W8 拣入：2026-09-07 14:30 CST · A0 在 **`4d7be97 feat(M5): integrate W8 command bridge polish`** + **`94e763e fix(M5-W8,A3): close MCP policy phase debt — retire W1 pending semantics, add MCP_NO_RMCP_SERVER + --expect-current-gaps`** + **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8 (fmt + patch-whitespace + MCP --expect-pending debt)`** + **`97118d6 chore(M5): normalize W8 patch evidence whitespace`** 四段 commit 中拣入 A5 W8 `agent_validate`/`skill_validate` 错误体 `CredentialLeak` 脱敏（`agent.rs:30-35` + `skills.rs:31-40` + `security_policy.rs:118-120` Display 改不 clone secret）+ A6 W8 `npm run build` PASS + 79 agent-skill UI logic 断言 + A8 W8 graph UI polish 4 vue 文件修订（EdgeDetail/GraphFilter/GraphPanel/GraphViewer/NodeDetail/ActivityBar/MainArea + useGraphStore + graphUi + check-graph-ui-logic.mjs 41 断言）+ A9 W8 `check-plugin-policy.py` 复审补丁 + A11 W8 verification delta 功能性 GREEN 收口（pre-merge PASS，3 red lights 全部归 A3 W7/W8）+ A3 W8 MCP 政策 phase debt 关闭（MCP_NO_RMCP_SERVER + `--expect-current-gaps` gate + ACTIVE=8 PENDING=0）；**A1 W8 reconciliation 整包**（M5-0/9/10/11/12/13/14 修订 + A1 W8 checkpoint + A1 W8 patch）**与 A1 W7 reconciliation 整包合并拣入**（4d7be97 stat 53 files +6038 -101 包含 A1 W7 checkpoint/patch + A1 W8 checkpoint/patch + 8 lane assist + M5-0/9/10/11/12/13/14 修订）；W8 hard stops 全守（5 红色：无 mcp_* runtime / 命令只读 / source check + ACL + bridge/types + policy + tests 同包 / 不写 token/secret 日志 / 不 push）
> W9 active：2026-09-07 14:30 CST · Lane A1（M5-W9 · docs-only reconciliation，**Runtime-Free Polish Dispatch**：W8 focused validation PASS 后 A0 派发 W9 收口波——A2 START REVIEW ONLY（重审 A3/A5 W8 修复后 command boundary）/ A3 START POLICY/REVIEW ONLY（MCP policy current-phase green + 后续 M5-2.b 卡预备；不实施 rmcp/server/listener/network）/ A4 START PRIVACY REVIEW ONLY（重审 A5 CredentialLeak redaction 后所有命令 error/display 面的 secret echo）/ A5 START PRODUCT CODE SMALL（Agent/Skill read-only bridge hardening 收口：redacted validation errors + frontend parse/permission preview 边界 case 测试；不执行/不装/不持久化/不网络/不调模型）/ A6 START UI LOGIC ONLY（Agent/Skill 面板消费磨光：确定性 empty/error/loading + 无 secret 文本 echo + bounded preview 渲染）/ A7 START GRAPH CONTRACT DOCS ONLY（图谱桥契约终稿，runtime-free 与 blocked backend runtime 分离）/ A8 START GRAPH UI SMALL（图谱 UI 磨光：filter/search/layout 确定性 + 保留 bounded arrays + 改进 no-backend/read-only 状态；不实施后端 graph command）/ A9 START PLUGIN REVIEW ONLY（plugin surface 复审：manifest/lifecycle 维持 pure + 确认 install/enable/delete/download 仍缺；不实施 plugin runtime）/ A10 START SECURITY FINAL REVIEW（批量复审 W8/W9 输出：source check / ACL parity / read-only / redaction / 无 runtime 扩张）/ A11 START FINAL VERIFICATION（W9 final verification matrix：命令结果 + build metrics 21.07% ≤ 22% + cargo warnings unchanged + pre-merge result + 残留债 + push readiness）；A1 docs only，**W9 = runtime-free 磨光 + 终验收口波，build metrics 阈值 22%**）
> 基准：`a1a2061`（`master`，M4 已 PASS） + `404f514`（A0 W0/W1 dispatch） + `a654f0c`（A3 W2 MCP 政策门）+ `712a14c`（A2 W2 切片 0b + A1 M5-1.b 切卡）+ `98a3b01`（A0 W3 dispatch）+ `e96c902`（A6 W3 UI data contract）+ `bdb0602`（A11 W3 verification）+ `12f1cff`（A3 W3 MCP 余下切片）+ `f8f1f49`（A2 W3 seam + A1 W3 reconciliation + A4-A10 W3 assist）+ `f7ad35a`（A0 W4 dispatch）+ `1610939 feat(M5): add agent memory and skill policy shells`（A0 拣入 A4 W4 agent_memory.rs + A5 W4 agent.rs/skills.rs + A1 W4 reconciliation 整包 + 8 份 W4 assist）+ `0e76a89 docs(M5): dispatch W5 UI and graph lanes`（A0 W5 dispatch）+ **`4b438ef feat(M5): add graph model store policy slice`**（A0 拣入 A7 W5 graph model + store policy slice：`src-tauri/src/graph.rs` + `domain.rs` 追加 GraphNode/Edge 类型 + `scripts/check-graph-policy.py` 7 ACTIVE 码 + pre-merge 接入）+ **`f99d2eb feat(A6): M5-6 Agent/Skill UI pure logic + panel shell (W5)`**（A0 拣入 A6 W5 面板壳：UI pure logic helper + Agent/Skill 面板 shell + `scripts/check-agent-skill-ui-logic.mjs` + 校验展示 / permission preview / capability 列表 / empty+error 状态；**不调 live runtime**）+ **`1a2c9cd docs(A11): M5-W5 verification delta after A6/A7 W5 outputs`**（A0 拣入 A11 W5 验证 delta：cargo test 9/9 graph + UI logic test 25 断言 + check-graph-policy.py --self-test PASS(ACTIVE=7) + check-agent-skill-ui-logic.mjs PASS + pre-merge ALL_PASS）+ **`77b1e3e docs(M5): dispatch W6 graph UI and plugin lanes`**（A0 W6 dispatch）+ **`412d0eb docs(M5-W6,A3): MCP compatibility review of plugin/graph exposure vs M5-2 registry+policy`**（A3 W6 MCP 兼容复审）+ **`add0609 docs(A6): M5-W6 UI consistency review — A8 graph UI vs M5-6 Agent/Skill shell`**（A6 W6 UI 一致性复审）+ **`d08d095 docs(A11): M5-W6 verification delta after W5 integration; A8/A9 W6 pending`**（A11 W6 验证 delta：W5 集成态 ALL_PASS + A8/A9 W6 产品代码 PENDING）+ **`5f92ece feat(M5): add graph UI and plugin policy slices`**（A0 W6 拣入 A8 W6 graph UI 4 个 .vue + store + utils + `scripts/check-graph-ui-logic.mjs` + A9 W6 `plugin.rs` + `domain.rs` PluginInvokeRecord + `scripts/check-plugin-policy.py` 6 ACTIVE 码 + pre-merge 接入 + `security_policy.rs` 补丁 + A10/A2/A4/A5/A7 5 份 W6 assist + A1 W6 reconciliation 整包 + M5-0/9/10/11/12 头部 status 修订）+ **`a26fbaf docs(M5): dispatch W7 read-only command bridge lanes`**（A0 W7 dispatch：开 A3 M5-2 read-only MCP bridge + A5 Agent/Skill read-only bridge 两条产品代码 lane；其它 9 lane docs/review/support；**W7 hard stops** = 5 条：仅 A3/A5 可写 / 命令只读 / 命令必须 atomic（source check + ACL + bridge/types + policy + tests）/ 不写 token/prompt secret 日志 / 不 push）· *build metrics threshold 19% with W5 debt（IF-2 → A11 重采挂账，未在 W6 关闭）* + **`6c1f30e feat(M5-W7,A3): read-only MCP registry/policy bridge commands`**（A0 W7 拣入 A3 W7 实施期：3 mcp_* commands `mcp_policy_get / mcp_registry_list / mcp_capability_preview` 在 `bridge.rs` + `mcp.rs` 81 行 patch + `main.rs` generate_handler! 注册 + `default-commands.toml` ACL 3 条插末条 `list_artifact_images` 前 + `bridge.ts` 3 invoke wrappers + `types.ts` 3 DTOs + `check-mcp-policy.py` ACTIVE=6 PENDING=9 含 MCP_BRIDGE_READONLY gate；9 mcp.rs unit tests PASS；10 files +1224 -8） + **`daa10f6 docs(A11): M5-W7 verification delta — pre-merge FAIL (3 red lights from 5f92ece integration hygiene)`**（A11 W7 拣入 验证 delta：W7-1 cargo test 编译中断（plugin.rs `PluginCapability` import 错位，4 errors 同根）/ W7-2 cargo fmt FAIL（bridge.rs 8 处未格式化）/ W7-3 build metrics warnings_increased（cargo_warnings 2→3 = PluginCapability unused import）；pre-merge FAIL，A11 不改代码；单根修复配方交 A0/A9）+ **`a29b796 docs(A6): M5-W7 UI wiring note for A5 read-only Agent/Skill command bridge`**（A6 W7 拣入 UI wiring note：Agent/Skill 面板消费 A5 read-only bridge 的 wiring 设计；不接 live command）· *A1 W7 reconciliation 整包**未**进 master（A0 W7 拣入期未消，详见 [W7 reconciliation] 段）* + **`f51549f docs(A6): M5-W8 Agent/Skill panel consumption plan + pure UI logic tests`**（A0 W8 拣入 A6 W8 panel consumption plan + UI logic tests 文档）+ **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8 (fmt + patch-whitespace + MCP --expect-pending debt)`**（A11 W8 拣入 verification delta：**W8 focused validation 功能性 GREEN**——3 red lights 全部归 A3 W7/W8：fmt 由 A0 在 4d7be97 修 / patch whitespace 由 A0 在 97118d6 修 / MCP `--expect-pending` debt 由 A0 在 94e763e 修） + **`94e763e fix(M5-W8,A3): close MCP policy phase debt — retire W1 pending semantics, add MCP_NO_RMCP_SERVER + --expect-current-gaps`**（A0 W8 拣入 A3 W8 收口：**MCP policy phase debt 关闭** —— retire W1 pending semantics + 新增 `MCP_NO_RMCP_SERVER` 政策码 + `check-mcp-policy.py --expect-current-gaps` gate 模式 + pre-merge.sh MCP section 接入；ACTIVE=8 PENDING=0，5 files +665 -180）+ **`4d7be97 feat(M5): integrate W8 command bridge polish`**（A0 W8 拣入：A5 W8 `agent_validate`/`skill_validate` 错误体 `CredentialLeak` 脱敏（`agent.rs:30-35` + `skills.rs:31-40` + `security_policy.rs:118-120` Display 不 clone secret）+ A6 W8 `npm run build` PASS + 79 agent-skill UI logic 断言 + A8 W8 graph UI polish 4 vue 文件修订（EdgeDetail/GraphFilter/GraphPanel/GraphViewer/NodeDetail + ActivityBar/MainArea + useGraphStore + graphUi + check-graph-ui-logic.mjs 41 断言）+ A9 W8 `check-plugin-policy.py` 复审补丁 + A11 W8 verification delta + A2/A4/A5/A7/A8/A9/A10 8 份 W7+W8 assist + **A1 W7 reconciliation 整包合并拣入**（A1 W7 checkpoint + patch 共 9 文件 = +292 -15）+ **A1 W8 reconciliation 整包合并拣入**（A1 W8 checkpoint + patch + M5-0/9/10/11/12/13/14 修订共 11 文件 = +488 -15）· **53 files +6038 -101**）+ **`97118d6 chore(M5): normalize W8 patch evidence whitespace`**（A0 W8 拣入补丁空白规范化：3 份 W8 patch 文件空白整理）· *build metrics threshold **22%** for W9（IF-2 W8 实测 21.07% ≤ 22% PASS；cargo_warnings delta = 0；W9 hard stop 阈值不变）*
> 性质：**纯文档展开**。零产品代码（未触 `src/`、`src-tauri/`、`package.json`、三份主文档、ACL/Capability/Manifest、pre-merge.sh）；不移动 `NEXT`；不提交、不 push。
> 依据：`PARALLEL_COMMAND_BOARD.md`（2026-09-05 23:55 版 · Batch Implementation Dispatch · Lane A1: M5 task-card expansion；2026-09-06 08:35 CST · M5-W1 Implementation Dispatch · Lane A1: START DOCS ONLY · reconciliation；2026-09-06 13:45 CST · M5-W2 Parallel Dispatch · Lane A1: START DOCS ONLY · constant-centralization reconciliation；2026-09-06 17:10 CST · M5-W3 Parallel Dispatch · Lane A1: START DOCS ONLY · mark W1/W2 complete + W3 active；2026-09-06 17:55 CST · M5-W4 Parallel Dispatch · Lane A1: START DOCS ONLY · reconcile W4 as active NEXT; mark W3 pushed and split M5-3/M5-4/M5-5 into next-card acceptance criteria；2026-09-06 18:35 CST · M5-W5 Parallel Dispatch · Lane A1: START DOCS ONLY · reconcile W5 as active NEXT; mark W4 pushed and tighten M5-6/M5-7/M5-8 acceptance criteria；**2026-09-06 19:25 CST · M5-W6 Parallel Dispatch · Lane A1: START DOCS ONLY · reconcile W6 as active NEXT; mark W5 pushed and tighten M5-9/M5-10/M5-11/M5-12 acceptance criteria**）
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

## [W5 reconciliation · 2026-09-06 19:00 CST] W5 整包已 A0 拣入（`4b438ef` + `f99d2eb` + `1a2c9cd`）· 当前活跃 checkpoint 切到 M5-W6

> **依据**：`PARALLEL_COMMAND_BOARD.md` L7（*"Current mainline: master at 4b438ef"*）+ L139（*"Agent/Skill UI shell is integrated; graph model/store policy slice is integrated; build metrics threshold is documented at 19% with W5 debt"*）。
> **W5 落地事实回填（A0 拣入 3 段核心 commit + 8 份 W5 assist + A11 W5 验证）**：
> - **A6 W5 M5-6 Agent/Skill UI pure logic + panel shell**（`f99d2eb` 拣入）：UI pure logic helper module（`src/utils/agentSkillUi.ts` 或类似）+ Agent/Skill 面板 shell（`src/components/agent/SkillManager.vue` 列表 / `PermissionPreviewModal.vue` 权限弹窗 / ChatPanel 流式）+ 校验展示 / permission preview 桥接 / capability 列表 / empty+error 状态；`scripts/check-agent-skill-ui-logic.mjs` PASS；**不调 live runtime / 不执行 skill / 不装插件**（遵守 W5-HS1）。
> - **A7 W5 M5-7/M5-8 graph model/store policy slice**（`4b438ef` 拣入）：`src-tauri/src/graph.rs`（GraphStore 内存 bounded 存储 + validate_graph_node/edge 容量脱敏校验 + 复用 A4 同款隐私双扫 SENSITIVE_KEY_NAMES=token/password/secret/api_key + SENSITIVE_VALUE_PATTERNS=sk-/AKIA/Bearer/eyJ/-----BEGIN + bounded_neighbors/bounded_subgraph + to_json/from_json serde 壳；模块级 `#![allow(dead_code)]` 因 W5 无消费方）+ `domain.rs` 尾部追加 `GraphNodeKind/GraphEdgeKind` 枚举、`GraphProps(BTreeMap)`、`GraphNode/GraphEdge` 结构，及 7 个容量常量单一真源（`GRAPH_PROPS_MAX_BYTES=MAX_TEXT_FIELD_BYTES=64KiB` 必须等于、`GRAPH_LABEL_MAX_BYTES=256`、`GRAPH_NODE_ID_HEX_LEN=64`、`GRAPH_MAX_DEPTH=4`、`GRAPH_QUERY_LIMIT=1000`、`GRAPH_MAX_NODES=5000`、`GRAPH_MAX_EDGES=20000`）+ `main.rs` 加 `mod graph;` + `scripts/check-graph-policy.py`（7 ACTIVE 码：`GRAPH_CONSTANTS_PRESENT` / `GRAPH_PROPS_EQ_MAX_TEXT_FIELD` / `GRAPH_PRIVACY_DOUBLE_SCAN` / `GRAPH_BOUNDED_STORE` / `GRAPH_TRAVERSAL_BOUNDED` / `GRAPH_REF_NODE_INTEGRITY` / `GRAPH_NO_SECOND_PATH`）+ pre-merge 接入。
> - **A11 W5 M5-W5 verification delta**（`1a2c9cd` 拣入）：cargo test graph 9/9 PASS + UI logic test 25 断言 PASS + `check-graph-policy.py --self-test` PASS（ACTIVE=7）+ `check-agent-skill-ui-logic.mjs` PASS + pre-merge ALL_PASS + git diff --check CLEAN。
> - **A8 W5 graph UI docs delta**（untracked，已在 W5 期间产出）：`logs/assist/A8-M5-W5-graph-ui-delta-20260906-1525.md` —— A8 W5 仍 SUPPORT DOCS ONLY（**未**写 UI 代码）；W6 由 A8 升级为 START PRODUCT CODE（**M5-9 实施期正式派发**）。
> - **A9 W5 plugin manifest docs delta**（untracked，已在 W5 期间产出）：`logs/assist/A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md` —— A9 W5 仍 SUPPORT DOCS ONLY（**未**写 plugin 代码）；W6 由 A9 升级为 START PRODUCT CODE（**M5-10/M5-11 实施期正式派发**）。
> - **其它 5 份 W5 assist**（untracked）：A2 / A3 / A4 / A5 / A10 全部 W5 角色 = SUPPORT/REVIEW ONLY（**未**写产品代码）。
> - **A0 W6 dispatch**（`77b1e3e` 已 push `4b438ef` + `f99d2eb` + `1a2c9cd` + `d3f11cd` 后签发）：PARALLEL_COMMAND_BOARD L136-171 *M5-W6 Parallel Dispatch* —— 本卡见下文 `[W6 active]` 段。
> **W5 → W6 状态切换总账**：

| Wave | A1 checkpoint 文件 | A0 拣入 commit | 关键 commit 链 | 状态 |
|------|-------------------|----------------|---------------|------|
| W0 | `M5-0-overview.md`（初版）| `404f514` | `a1a2061` → `404f514` | PASS |
| W1 | `M5-0-overview.md` (W1 reconciliation) | `0d08016` | + `854bc40` + `0d08016` | PASS |
| W2 | `M5-0-overview.md` (W2 patched) | `712a14c` | + `a654f0c` + `712a14c` | PASS |
| W3 | `A1-M5-W3-reconciliation-20260906-1730.md` | `f8f1f49` | + `98a3b01` + `e96c902` + `bdb0602` + `12f1cff` + `f8f1f49` | PASS |
| W4 | `A1-M5-W4-reconciliation-20260906-1755.md` | `1610939` | + `d71f558` + `13c5279` + `562efb9` + `1610939` | PASS |
| W5 | `A1-M5-W5-reconciliation-20260906-1835.md` | `4b438ef` + `f99d2eb` + `1a2c9cd` | + `0e76a89`（A0 W5 dispatch） + `d3f11cd`（A3 W5 assist） + `4b438ef` + `f99d2eb` + `1a2c9cd` | **PASS · A0 拣入** |
| W6 | `A1-M5-W6-reconciliation-20260906-1930.md` | `5f92ece` | + `77b1e3e`（A0 W6 dispatch） + `412d0eb`（A3 W6 MCP 兼容复审） + `add0609`（A6 W6 UI 一致性复审） + `d08d095`（A11 W6 验证 delta） + `5f92ece`（A0 W6 拣入 A8 graph UI + A9 plugin policy + 5 W6 assist + A1 W6 整包） | **PASS · A0 拣入** |

> **W5 收口后遗留债（**`PARALLEL_COMMAND_BOARD.md` L139 显式记挂**）**：
> - **IF-2**（build metrics growth）：frontend main JS 19% 阈值（baseline 161.36 kB，W5 增量计入后尚未重采；W6 实施期 A8 / A9 上线后 A11 重采，由 A0 拍定阈值/基线）。
> - **W6 范围内债**：A8 W6 实施期 / A9 W6 实施期各自的 `cargo build` / `npm run build` 增量（见 M5-9 / M5-10 / M5-11 [W6 next-card AC] 段 hard stop）。
> - **A9 W5 ds1~ds6 design suggestions**（`logs/assist/A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md` §3 列举的 Ed25519 / 形态③ / capability 命名 / 状态机扩展点 / 审计字段 / 二次确认粒度 6 项）—— A9 W6 实施期在 DTO/lifecycle/policy 落地时**可选**采纳，**不强制**；A1 W6 **不**预先决断。

---

## [W6 active · 2026-09-06 19:25 CST] 当前活跃 checkpoint 切到 M5-W6（A8 graph UI pure logic + A9 plugin manifest/lifecycle policy slice · 其它 9 lane docs/review/support）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L136-171（**M5-W6 Parallel Dispatch**，Added 2026-09-06 19:25 CST by A0 after pushing through `4b438ef`）+ L7（*"Current NEXT: M5-W6 parallel implementation; Lane A8 owns M5-9 graph UI pure logic/panel shell, Lane A9 owns M5-10/11 plugin manifest/lifecycle policy slice"*）。
> **W6 仅开两条产品代码 lane**：
> - **A8** *START PRODUCT CODE*：M5-9 graph UI **pure logic + panel shell** —— graph list/search/filter、node detail summary、capacity/error/empty states、helper module + headless logic test；**不加后端 commands、不调 live agent consumption、不调 model calls、不加 graph rebuild workers**；scope = `src/components/**` + `src/stores/**` + `src/types.ts` + `src/bridge.ts`（only if no new command）+ `scripts/check-graph-ui-logic.mjs` + optional UI policy script。
> - **A9** *START PRODUCT CODE*：M5-10/M5-11 plugin manifest/lifecycle **policy slice** —— DTOs + validation + lifecycle state machine + permission manifest rules + policy script；**不**install/uninstall/delete/download/execute/enable 真实 plugins；**不**做网络/下载/签名强制（仅纯 validation）；scope = `src-tauri/src/domain.rs` + optional `src-tauri/src/plugin.rs` + `src-tauri/src/security_policy.rs` + `scripts/check-plugin-policy.py` + `scripts/pre-merge.sh` + focused Rust tests。
> **A1 W6 角色**：START DOCS ONLY — *"Reconcile W6 as active NEXT; mark W5 pushed and tighten M5-9/M5-10/M5-11/M5-12 acceptance criteria."* —— 本卡顶部索引 + 5 张子卡（M5-9 / M5-10 / M5-11 / M5-12 + 本根卡）增补 W6 next-card acceptance criteria 段。

### W6 A1 整包交付（计划）

| # | 文件 | 修订 |
|---|------|------|
| W6-1 | `M5-0-overview.md`（本根卡）| 头部时间戳 + W5 reconciliation 段（3 段核心 commit `f99d2eb` / `4b438ef` / `1a2c9cd` + 8 份 W5 assist + IF-2 挂账 + A8/A9 W6 升级）+ W6 active 段（本段；索引 6 文件交付清单 + 2 lane 实施期硬约束摘要）|
| W6-2 | `M5-9-graph-ui-agent-consume.md` | 头部状态行 W3→W6；新增 [W6 next-card acceptance criteria] 段（**4 项 AC**：graph list/search/filter pure logic helper / node detail summary / capacity+error+empty 状态 / headless logic test；**5 项 hard stops**：A8 W6 不加 commands、不调 live agent、不调 model、不加 rebuild workers、不加 npm 依赖）|
| W6-3 | `M5-10-plugin-manifest-lifecycle.md` | 头部状态行；新增 [W6 next-card acceptance criteria] 段（**4 项 AC**：PluginManifest DTOs + validation / lifecycle state machine / permission manifest rules / policy script；**5 项 hard stops**：A9 W6 不 install/uninstall/delete/download/execute real plugins / 不做网络 / 不做签名强制 / 不注册 10 条 plugin_* 命令 / 不破 capability.rs 漂移）|
| W6-4 | `M5-11-plugin-commands-isolation.md` | 头部状态行；新增 [W6 next-card acceptance criteria] 段（**4 项 AC**：commands_islolation shell / 5 命令 ACL stub / capability 校验骨架 / plugin-invokes.json audit shape；**5 项 hard stops**：同 W6-HS + 5 命令仅 stub（不接业务）+ audit 仅 key_hash 不带 value + capability 真源单点 + 不破 K1 ACL 末条恒为 list_artifact_images）|
| W6-5 | `M5-12-plugin-ui.md` | 头部状态行；新增 [W6 status] 轻量段（A19 W6 仍 SUPPORT DOCS ONLY；W6 无 plugin UI lane 承接，待 A9 W6 plugin 后端落 + A0 决定 W7+ 派发；A1 W6 不写 next-card AC）|
| W6-6 | `logs/checkpoints/A1-M5-W6-reconciliation-20260906-1930.md` | 新增：A1 W6 整包交付 checkpoint |
| W6-7 | `logs/checkpoints/Lane-A1-M5-W6-reconciliation-20260906-1930.patch` | 新增：A1 W6 整包 patch |

### W6 A8 / A9 实施期硬约束（与 W6 dispatch 承诺一致）

| # | 约束 | 来源 |
|---|------|------|
| W6-HS1 | **A8/A9 是 W6 唯一允许写产品代码的两条 lane**；A2/A3/A4/A5/A6/A7 变 SUPPORT/REVIEW ONLY；A10 START REVIEW；A11 START VERIFICATION | PARALLEL_COMMAND_BOARD L166 |
| W6-HS2 | **A8 不得加** backend commands / live agent consumption / model calls / graph rebuild workers | PARALLEL_COMMAND_BOARD L167 |
| W6-HS3 | **A9 不得 install/delete/download/execute/enable 真实 plugins**；仅纯 manifest/lifecycle policy + validation | PARALLEL_COMMAND_BOARD L168 |
| W6-HS4 | **新 Tauri 命令必须 atomic**（source check + ACL + 前端 bridge/types + policy coverage + tests 同包）；若契约未完全 ready，**优先不加 command**（W6 优先倾向不加 command） | PARALLEL_COMMAND_BOARD L169 |
| W6-HS5 | **Stores/maps/lists 必须 bounded + privacy-filtered**：无 token/cookie/Authorization/body/日志 prompt secrets | PARALLEL_COMMAND_BOARD L170 |
| W6-HS6 | 所有 lane 必须从 `origin/master` pull，**不 push** | PARALLEL_COMMAND_BOARD L171（合并 W5 末项）|
| W6-HS7 | **A8 无新 npm 依赖**（须复用现有 Vue 3 + Pinia + 既有 A6 W5 panel 范式 + d3-force 仅在 W7+ 真接入时引入）| A8 W6 dispatch L159 + W4-HS2 / W5-HS7 |
| W6-HS8 | **A9 不破 capability.rs 漂移**（A2P / A2A / Skill / Plugin / Agent 五类共用 capability.rs 单一真源，A9 W6 加 `PLUGIN_CAPABILITY_V*` 必须走同一文件） | A5 W4 AC + A3 W3 MCP policy shell + M5-2 §4.2-4.3 |

### W6 A1 硬停止

- **零产品代码**：A1 W6 整包**仅文档**（PARALLEL_COMMAND_BOARD L152 明示 *"One reconciliation checkpoint; no product code"*）。
- **不重写各子卡 §1~§11**：仅头部 [W6 next-card acceptance criteria] 段（不修订 §1~§11 决策史）。
- **不移动 `NEXT`**：`NEXT` 标记属 A0 调度权；A1 仅在头部状态行陈述"W6 是当前活跃 checkpoint"。
- **不动三份主文档**：A0 W6 dispatch L152 虽允许"three main docs"，但本轮 A1 选择**不动**——W6 修订仅落在 `logs/checkpoints/M5-20260906/*.md` 5 文件 + 1 新增 checkpoint；如需主文档调整留待 W6 收口或 A0 拣入期处理。
- **不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json**：A1 W6 严格不动。
- **不提交 / 不 push**：A1 W6 整包交 A0 拣入合并。
- **不抢 A8 / A9 工作区**：A1 W6 不动 `src/components/**` / `src/stores/**` / `src/types.ts` / `src/bridge.ts` / `src-tauri/src/domain.rs` / `src-tauri/src/plugin.rs`（如 A9 创建）/ `src-tauri/src/security_policy.rs` / `scripts/check-graph-ui-logic.mjs` / `scripts/check-plugin-policy.py` / `scripts/pre-merge.sh`；A8 / A9 各自工作区严格留给对应 lane。

### W6 状态（本卡涉及）

| 项 | 状态 | 来源 |
|---|------|------|
| W5（A6 M5-6 + A7 M5-7/M5-8 + 8 lane assist + 2 review + A1 reconciliation 整包）| **PASS · A0 拣入** | `4b438ef` + `f99d2eb` + `1a2c9cd` + `d3f11cd` |
| W6 A8 M5-9 实施 | **ACTIVE · 待 A8 实施** | PARALLEL_COMMAND_BOARD L159 |
| W6 A9 M5-10/M5-11 实施 | **ACTIVE · 待 A9 实施** | PARALLEL_COMMAND_BOARD L160 |
| W6 A2/A3/A4/A5/A6/A7 复审 + A10 review + A11 verification | **ACTIVE · 待各 lane 输出** | PARALLEL_COMMAND_BOARD L153-L162 |
| W6 A1 文档 reconciliation（本段 + 4 子卡 acceptance criteria + 本 checkpoint）| **本轮 W6 修订已完成** · A0 拣入 | 本 checkpoint |

---

## [W6 reconciliation · 2026-09-07 07:19 CST] W6 整包已 A0 拣入（`5f92ece`）· 当前活跃 checkpoint 切到 M5-W7

> **依据**：`PARALLEL_COMMAND_BOARD.md` L7（*"Current mainline: master at 5f92ece"*）+ L139（*"Agent/Skill UI shell is integrated; graph model/store policy slice is integrated; build metrics threshold is documented at 19% with W5 debt"*）+ `git log --oneline -20` 实测（`5f92ece feat(M5): add graph UI and plugin policy slices` 已 push）。
> **W6 落地事实回填（A0 拣入 1 段核心 commit + 8 份 W6 assist）**：
> - **A8 W6 M5-9 graph UI pure logic + panel shell**（`5f92ece` 拣入）：4 个新 .vue（`GraphPanel.vue` 主面板壳 / `GraphViewer.vue` 视图 / `NodeDetail.vue` 节点详情 / `EdgeDetail.vue` 边详情 / `GraphFilter.vue` 过滤）+ 1 store（`useGraphStore.ts`）+ 1 util（`graphUi.ts`）+ layout 接入（`ActivityBar.vue` / `MainArea.vue`）+ types.ts 镜像 + bridge.ts 14 行桥接 + **`scripts/check-graph-ui-logic.mjs` 193 行 PASS**；**不**加新后端命令、**不**调 live agent consumption、**不**引 d3 整包（遵守 W6-HS2/HS7）。
> - **A9 W6 M5-10/M5-11 plugin manifest/lifecycle policy slice**（`5f92ece` 拣入）：`src-tauri/src/plugin.rs` 446 行（PluginManifest/PluginEntry/PluginCapability/PluginSignature DTOs + `validate_plugin_manifest` 纯函数 + 7 状态机 `PluginState`（Discovered/Validating/Signed/Loaded/Enabled/Disabled/Uninstalled）+ `transition()` 非法转移返回 `TransitionError` + 12 条合法边 + `PluginLifecycleEvent` 枚举 + `PermissionManifestRule` 接受 `&CapabilityRegistry` + `check_plugin_capability` 三种错误 + 5 stub `plugin_invoke/cancel/permissions_get/audit_list/storage_get_or_put_or_delete` 返回 `Err("not-implemented-in-W6")` + `PluginInvokeRecord { payload_key_hash, key_hash_only: true }` 审计 schema + `plugin-invokes.json` 64 KiB FIFO）+ `src-tauri/src/domain.rs` 追加 `MAX_PLUGIN_CAPABILITIES=5` 容量常量（单一真源）+ `src-tauri/src/security_policy.rs` 47 行补丁（plugin 隐私断言复用 SENSITIVE_KEY_NAMES/SENSITIVE_VALUE_PATTERNS）+ `scripts/check-plugin-policy.py` 218 行 **6 ACTIVE 码**（PLUGIN_MANIFEST_SCHEMA_PRESENT / PLUGIN_VALIDATION_PURE / PLUGIN_LIFECYCLE_STATE_MACHINE / PLUGIN_CAPABILITY_WHITELIST_ONLY / PLUGIN_NO_INSTALL_RUNTIME / PLUGIN_NO_NETWORK）+ pre-merge 接入；**不** install/uninstall/delete/download/execute real plugins、**不**做网络/下载/签名强制（仅 Ed25519 字面量声明 + schema 校验）、**不**注册 10 条 plugin_* 命令（W6 倾向 0 新命令，遵守 W6-HS3/HS4/HS8）。
> - **A11 W6 verification delta**（`d08d095` 拣入，已被 `5f92ece` 覆盖）：W5 集成态 cargo_warnings 2→27 红灯消解（`4b438ef`）+ cargo test 全绿 + 6 策略脚本 self-test PASS + pre-merge ALL_PASS；A8/A9 W6 产品代码在该 delta 时点 PENDING（待拣入）；**已**在 `5f92ece` 拣入后实际落地，门禁可由 A11 W7 重跑补验。
> - **A3 W6 MCP 兼容复审**（`412d0eb` 拣入）：MCP compatibility review of plugin/graph exposure vs M5-2 registry+policy —— 仅 docs，不改 M5-2 既有政策门。
> - **A6 W6 UI 一致性复审**（`add0609` 拣入）：UI consistency review — A8 graph UI vs M5-6 Agent/Skill shell —— 仅 docs，复核面板壳风格复用与 K7 隐私双闸。
> - **A10 W6 安全复审**（`logs/assist/A10-M5-W6-security-review-20260906-2300.md` 101 行 untracked 进 `5f92ece`）：security review of A8 graph UI / A9 plugin policy；**未**发现 install runtime / network / signature enforcement / capability.rs drift 违规。
> - **A2 W6 边界复审**（`logs/assist/A2-M5-W6-boundary-review-20260906-2030.md` 177 行 untracked 进 `5f92ece`）：boundary review of A9 plugin.rs + A8 graph UI；**未**发现 core 边界反向边新增。
> - **A4 W6 memory/privacy/capacity 复审**（`logs/assist/A4-M5-W6-memory-privacy-capacity-review-20260906-2237.md` 101 行 untracked 进 `5f92ece`）：memory/privacy/capacity review of A8 graph UI / A9 plugin policy；**未**发现 privacy/sensitive payload 命中。
> - **A5 W6 plugin manifest 复审**（`logs/assist/A5-M5-W6-plugin-manifest-review-20260906-2315.md` 170 行 untracked 进 `5f92ece`）：plugin manifest review；**未**发现 capability.rs drift，W6 不实装 Ed25519 校验。
> - **A7 W6 graph contract 复审**（`logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md` 167 行 untracked 进 `5f92ece`）：graph contract review vs W5 A7 DTO；F1~F9 9 项红线全部满足（summarizeNode 白名单仅 4 字段、Snake_case enum 镜像、容量常量单源、K7 双闸前端禁 props、bounded 语义镜像、Skill/Agent id 64-hex 不反查、不引 d3 整包、复用 A6 面板壳、命令列表 0 命中）。
> - **A0 W7 dispatch**（`a26fbaf` 已 push `5f92ece` 后签发）：PARALLEL_COMMAND_BOARD L137-175 *M5-W7 Integration Dispatch* —— 本卡见下文 `[W7 active]` 段。
> **W6 → W7 状态切换总账**：

| Wave | A1 checkpoint 文件 | A0 拣入 commit | 关键 commit 链 | 状态 |
|------|-------------------|----------------|---------------|------|
| W6 | `A1-M5-W6-reconciliation-20260906-1930.md` | `5f92ece` | + `77b1e3e`（A0 W6 dispatch） + `412d0eb`（A3 W6 MCP 兼容复审） + `add0609`（A6 W6 UI 一致性复审） + `d08d095`（A11 W6 验证 delta） + `5f92ece`（A0 W6 拣入 A8 graph UI + A9 plugin policy + 5 W6 assist + A1 W6 整包） | **PASS · A0 拣入** |
| W7 | **本轮修订** | <待 A0 拣入> | + `a26fbaf`（A0 W7 dispatch） | **本轮 W7 修订已完成** · 待 A0 拣入 |

> **W6 收口后遗留债（**承接 W5 收口债 + W6 实施期增量**）**：
> - **IF-2**（build metrics growth）：frontend main JS 19% 阈值（W5 增量计入 + W6 graph UI store+utils+4 vue 实测后**仍未重采**；W7 实施期 A3/A5 上线后由 A11 W7 重采，A0 拍定阈值/基线）。
> - **W6 → W7 范围内债**：
>   - A3 W7 / A5 W7 各自新增的 read-only command bridge 命令（按 A3/A5 W7 next-card AC 段 hard stop：A3 加 MCP list/preview 命令、A5 加 Agent/Skill parse/validate/preview 命令）。
>   - A7 W6 graph contract F1（**Critical**）：M5-9 卡 W6 AC-1 的 `summarizeNode` 白名单含 5 个 A7 DTO 不存在的字段（source/source_ref/created_at/updated_at/extractor_version），且 `id=sha256` 不可逆无法派生 —— **须由 A1 W7 修订卡**；W6 A8 已实现 `summarizeNode` 实际仅可用 `{id, kind, label, neighborCount}`。**A1 W7 应在 M5-9 卡补 [W7 patched] 段订正**。
>   - F2 F3 F4 F5 F6 F7 F8 F9（**Critical**）：grep 确认全仓库无 `graph_*` 后端命令（W6 不破 K1）、enum 须精确 snake_case 镜像、容量常量单源、K7 双闸前端亦禁 props、bounded 语义镜像、Skill/Agent id 64-hex 不反查、不引 d3 整包、复用 A6 面板壳 —— A8 W6 已落，无需 W7 改动。
>   - DRY F1（**低**）：`SENSITIVE_KEY_NAMES` / `SENSITIVE_VALUE_PATTERNS` 在 `agent_memory.rs:134` 与 `graph.rs:18` 各定义一份，建议抽到 `domain.rs` 单一真源 —— 留待 W8+。
> - **A9 W6 ds1~ds6 design suggestions**（`logs/assist/A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md` §3）—— A9 W6 已**可选**采纳 5 项（Ed25519 字面量声明 / 形态③ / capability 命名 / 状态机扩展点 / 审计字段），ds1/ds6 二次确认粒度部分采纳；A9 W7+ 不再决断。

---

## [W7 active · 2026-09-07 00:50 CST] 当前活跃 checkpoint 切到 M5-W7（A3 M5-2 read-only MCP bridge + A5 Agent/Skill read-only bridge · 其它 9 lane docs/review/support · W7 = 窄 read-only command bridge wave，**不**做 runtime activation）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L137-175（**M5-W7 Integration Dispatch**，Added 2026-09-07 00:50 CST by A0 after pushing through `5f92ece`）+ L7（*"Current NEXT: M5-W7 integration implementation; Lane A3 owns M5-2 read-only MCP command bridge, Lane A5 owns Agent/Skill read-only command bridge, other lanes docs/review/support only"*）。
> **W7 仅开两条产品代码 lane**（**narrow read-only command bridge wave, not runtime activation**）：
> - **A3** *START PRODUCT CODE*：M5-2 read-only MCP registry/policy **command bridge** —— list registry entries / preview capability verdicts / return redacted DTOs；**must include source check + ACL + frontend bridge/types**（only if commands are added）；**NO** rmcp/server/listener、**NO** network、**NO** runtime server；scope = `src-tauri/src/mcp.rs` + `src-tauri/src/bridge.rs` + `src-tauri/src/main.rs` + `src-tauri/permissions/default-commands.toml` + `src/bridge.ts` + `src/types.ts` + `scripts/check-mcp-policy.py` + focused tests/checkpoint。
> - **A5** *START PRODUCT CODE*：Agent/Skill **read-only command bridge** —— parse/validate AgentDef/SkillDef + permission preview；**NO** skill execution、**NO** install、**NO** network、**NO** persistence writes；**must include source check + ACL + frontend bridge/types**（if commands are added）；scope = `src-tauri/src/agent.rs` + `src-tauri/src/skills.rs` + `src-tauri/src/bridge.rs` + `src-tauri/src/main.rs` + `src-tauri/permissions/default-commands.toml` + `src/bridge.ts` + `src/types.ts` + `scripts/check-agent-skill-policy.py` + focused tests/checkpoint。
> **A1 W7 角色**：START DOCS ONLY — *"Reconcile W7 as active NEXT; mark W6 pushed and define M5 final acceptance/debt list. One reconciliation checkpoint; no product code."* —— 本卡顶部索引 + 5 文件修订（详见 W7 A1 整包交付清单）+ M5-13 验证矩阵 + M5-14 债务账 + M5-9/10/11/12 头部状态行更新。

### W7 A1 整包交付（计划）

| # | 文件 | 修订 |
|---|------|------|
| W7-1 | `M5-0-overview.md`（本根卡）| 头部时间戳（添加 W6 reconciliation + W7 active）+ 基准链追加 `5f92ece` + `a26fbaf` + W6 reconciliation 段（`5f92ece` 拣入事实回填 8 文件 + W6 → W7 状态切换总账 + W6 收口后遗留债含 IF-2/A7 F1/DRY F1）+ W7 active 段（本段；索引 5 文件交付清单 + 2 lane 实施期硬约束摘要 + 8 W7 hard stops）|
| W7-2 | `M5-9-graph-ui-agent-consume.md` | 头部状态行追加 "**W6** PUSHED（`5f92ece` 拣入 A8 graph UI） · **W7** ACTIVE（A1 W7 标 F1 Critical 在本卡订正 / A3/A5 W7 不改本卡）"；新增 [W7 patched] 段订正 F1 `summarizeNode` 白名单（5 字段 → 4 字段 `{id,kind,label,neighborCount}`，id=sha256 不可逆不可派生）|
| W7-3 | `M5-10-plugin-manifest-lifecycle.md` | 头部状态行追加 "**W6** PUSHED（`5f92ece` 拣入 A9 plugin policy slice） · **W7** ACTIVE（**A3/A5 read-only bridge wave，A9 W7+ 仍不接业务 handler**）"；**不**新增 next-card AC（与 M5-11/M5-12 共享 W7 保持静默）|
| W7-4 | `M5-11-plugin-commands-isolation.md` | 头部状态行追加 "**W6** PUSHED（`5f92ece` 拣入 A9 plugin policy slice） · **W7** ACTIVE（**不**派发命令落地）"；**不**新增 next-card AC |
| W7-5 | `M5-12-plugin-ui.md` | 头部状态行追加 "**W6** ACTIVE（**未**落产品代码） · **W7** ACTIVE（**仍** SUPPORT DOCS ONLY；plugin UI 待 W8+ A19 派发）"；**不**新增 next-card AC |
| W7-6 | `M5-13-verification-matrix.md` | §0 头部追加 W6 verification delta（W6 已落地 cargo test + 19 ACTIVE policies + pre-merge ALL_PASS 实测）+ W7 verification scope（A3/A5 read-only bridge、 A10 review、A11 W7 delta）+ §5 PASS_CRITERIA 追加 12 M5 final acceptance criteria（每张子卡 1 项 final AC）+ §6 FAIL_ACTION 追加 W7 红线 |
| W7-7 | `M5-14-debt-ledger.md` | §1~§7 维持；§8 A0 决策清单 review 结果回填（默认选项采纳清单）；§9 新增 §10 M5 final debt ledger（M5 收口总账：W6 落地 + W7 收口后**仍未实现**项集，含 IF-2 build metrics 待 A11 重采 / A7 W6 graph F1 已订正但 `summarizeNode` 上游 source/时间戳字段仍未定 / DRY SENSITIVE_* 抽 domain.rs 待 W8+ / M5-12 plugin UI 待 W8+ / M5-1.b trait 抽离待 W8+ / M5-13 性能基线待 W8+ 跑 / 第三方签名服务+插件商店 待 A0 拍 / plugin 多版本并发启用策略待 A0 拍）|
| W7-8 | `logs/checkpoints/A1-M5-W7-reconciliation-20260907-0050.md` | **新增**：A1 W7 整包交付 checkpoint（含 W7 reconciliation 总账 + 8 W7 hard stops + 5 A1 W7 hard stops + W7 整包 patch 索引 + 自证 IF-2/IF-3/IF-5/IF-6 全满足）|
| W7-9 | `logs/checkpoints/Lane-A1-M5-W7-reconciliation-20260907-0050.patch` | **新增**：A1 W7 整包 patch（含本卡 + M5-9/10/11/12/13/14 6 文件修订 diff）|

### W7 A3 / A5 实施期硬约束（与 W7 dispatch 承诺一致）

| # | 约束 | 来源 |
|---|------|------|
| W7-HS1 | **A3/A5 是 W7 唯一允许写产品代码的两条 lane**；A1/A2/A4/A6/A7/A8/A9 变 SUPPORT/REVIEW/DOCS ONLY；A10 START REVIEW；A11 START VERIFICATION | PARALLEL_COMMAND_BOARD L173 |
| W7-HS2 | **A3 不得** rmcp/server/listener；**NO** runtime MCP server；**NO** network | PARALLEL_COMMAND_BOARD L155（*"No rmcp/server/listener"*） |
| W7-HS3 | **A5 不得** skill execution；**NO** install；**NO** network；**NO** persistence writes | PARALLEL_COMMAND_BOARD L157（*"no skill execution, no install, no network, no persistence writes"*） |
| W7-HS4 | **新 Tauri 命令必须 atomic**（source check + ACL + frontend bridge/types + policy coverage + tests 同包） | PARALLEL_COMMAND_BOARD L156 + L158（*"Must include source check, ACL, frontend bridge/types only if commands are added"*） |
| W7-HS5 | **stores/maps/lists 必须 bounded + privacy-filtered**：无 token/cookie/Authorization/body/日志 prompt secrets | PARALLEL_COMMAND_BOARD L172（*"No token/cookie/Authorization/body/prompt-secret logging or persistence"*） |
| W7-HS6 | 所有 lane 必须从 `origin/master` pull，**不 push** | PARALLEL_COMMAND_BOARD L174 |
| W7-HS7 | **commands read-only only**：no skill execution、no plugin install/enable/disable/uninstall、no MCP server/listener、no graph rebuild worker | PARALLEL_COMMAND_BOARD L171（*"W7 commands are read-only only: no skill execution, no plugin install/enable/disable/uninstall, no MCP server/listener, no graph rebuild worker"*） |
| W7-HS8 | **返回 DTO 必须 redacted**：MCP list/preview + Agent/Skill parse/validate/preview 命令**禁止**返回 secret / token / Authorization / body / prompt 内容；返回脱敏后白名单字段 | A4 W6 privacy 双扫 + A10 W7 review 预期 + W5-HS5 收口 |

### W7 A1 硬停止

- **零产品代码**：A1 W7 整包**仅文档**（PARALLEL_COMMAND_BOARD L153 明示 *"One reconciliation checkpoint; no product code"*）。
- **不重写各子卡 §1~§11**：仅头部 [W7 patched] / 状态行 修订，**不**改 §1~§11 决策史；M5-9 W7 patched 段仅订正 F1 Critical（A7 红线），**不**改 M5-9 §1 GOAL / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA 主体（仅 §4.1 summarizeNode 白名单收紧为 4 字段）。
- **不移动 `NEXT`**：`NEXT` 标记属 A0 调度权；A1 仅在头部状态行陈述"W7 是当前活跃 checkpoint"。
- **不动三份主文档**：`AI-模型切换与接手清单.md` / `后续需求TODO.md` / `详细设计与实施计划.md` —— A0 W7 已拣入（`a26fbaf` 同步 3 处微调）；A1 W7 **不**追加改动；如需主文档进一步修订留待 W7 收口或 A0 拣入期处理。
- **不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json**：A1 W7 严格不动。
- **不提交 / 不 push**：A1 W7 整包交 A0 拣入合并。
- **不抢 A3 / A5 工作区**：A1 W7 不动 `src-tauri/src/mcp.rs` / `src-tauri/src/agent.rs` / `src-tauri/src/skills.rs` / `src-tauri/src/bridge.rs` / `src-tauri/src/main.rs` / `src-tauri/permissions/default-commands.toml` / `src/bridge.ts` / `src/types.ts` / `scripts/check-mcp-policy.py` / `scripts/check-agent-skill-policy.py`；A3 / A5 各自工作区严格留给对应 lane。

### W7 状态（本卡涉及）

| 项 | 状态 | 来源 |
|---|------|------|
| W6（A8 M5-9 + A9 M5-10/11 + 8 lane assist + A1 reconciliation 整包）| **PASS · A0 拣入** | `5f92ece` + `412d0eb` + `add0609` + `d08d095` |
| W7 A3 M5-2 read-only MCP bridge 实施 | **ACTIVE · 待 A3 实施** | PARALLEL_COMMAND_BOARD L155 |
| W7 A5 Agent/Skill read-only bridge 实施 | **ACTIVE · 待 A5 实施** | PARALLEL_COMMAND_BOARD L157 |
| W7 A2/A4/A6/A7/A8/A9 复审 + A10 review + A11 verification | **ACTIVE · 待各 lane 输出** | PARALLEL_COMMAND_BOARD L154-L162 |
| W7 A1 文档 reconciliation（本段 + 4 子卡 status + M5-13/M5-14 + 本 checkpoint）| **本轮 W7 修订已完成** · 待 A0 拣入 | 本 checkpoint |

---

## [W7 reconciliation · 2026-09-07 09:45 CST] W7 整包拣入事实回填（A0 拣入 A3/A11/A6 wiring · A1 W7 reconciliation 整包未进 master · 改 W8 整包合并拣入）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L137-175（M5-W7 Integration Dispatch，Added 2026-09-07 00:50 CST by A0 after pushing through `5f92ece`）+ L185-209（**M5-W8 Excluding-A3 Dispatch**，Added 2026-09-07 09:45 CST by A0；A3 在 W8 是 HOLD/NO ASSIGNMENT）。
> **W7 拣入事实回填**（A0 W7 拣入期三段核心 commit）：
> - **`6c1f30e feat(M5-W7,A3)`**（A3 W7 START PRODUCT CODE · 10 files +1224 -8）：`bridge.rs` 231 行补丁 + `mcp.rs` 81 行补丁（`McpDecisionView` / `McpRegistryEntryView` + `list_registry_entries()`）+ `main.rs` generate_handler! 注册 3 mcp_* + `default-commands.toml` ACL 3 条（插末条 `list_artifact_images` 前）+ `bridge.ts` 3 invoke wrappers + `types.ts` 3 DTOs + `check-mcp-policy.py` ACTIVE=6 PENDING=9（MCP_BRIDGE_READONLY gate 新增；MCP_PARITY 泛化扫 main.rs `bridge::mcp_[a-z_]+`）+ 3 mcp.rs unit tests（registry_view_mirrors_registry / decision_view_maps_both_variants / unknown_capability_preview_is_denied）。**W7 硬停止全部满足**（read-only / source check `check_invocation_source` / no rmcp / no network / no side effects）。9 mcp.rs unit tests PASS。
> - **`daa10f6 docs(A11): M5-W7 verification delta`**（A11 W7 拣入 验证 delta · 1 file +145）：A11 在 `5f92ece` 集成态**复跑 pre-merge FAIL** —— **3 red lights**（**W7-1** cargo test 编译中断，4 errors 全为 plugin.rs L308/324/432/440 `PluginCapability` not found in test module imports；**W7-2** cargo fmt FAIL，bridge.rs 8 处未格式化含 mcp_policy_get/mcp_capability_preview 折行；**W7-3** build metrics warnings_increased，cargo_warnings 2→3 = plugin.rs:20 `PluginCapability` unused import）；pre-merge.sh = `PRE_MERGE_RESULT=FAIL`（cargo fmt main + build metrics regression）；A8/A9 W6 验证绿（graph UI logic 34/34 PASS + check-plugin-policy ALL_PASS(ACTIVE=1 PENDING=5) + npm run build 173KB）；A3/A5 W7 在 A11 W7 报时点已交付（A3 拣入 `6c1f30e` 之后）；A11 不改代码（**单根修复配方**交 A0/A9：plugin.rs L274 加 `PluginCapability` import + L20 删顶层 import + `cargo fmt --all`）。
> - **`a29b796 docs(A6): M5-W7 UI wiring note for A5 read-only Agent/Skill command bridge`**（A6 W7 拣入 UI wiring · 1 file）：Agent/Skill 面板消费 A5 read-only bridge 的 wiring 设计（**不接 live command**）。
> **A1 W7 reconciliation 整包未进 master**（**关键事实**）：A1 W7 整包（M5-0/9/10/11/12/13/14 修订 + A1 W7 checkpoint + A1 W7 patch，共 9 文件 = 7 M5-* + 1 checkpoint + 1 patch = +292 -15 diff 干净）**未**被 A0 在 W7 拣入期消化；工作树 M/A 状态保留至 W8 整包合并拣入。**W8 整包合并策略**：A1 W8 整包**含 A1 W7 修订 + A1 W8 reconciliation 修订**（一次性解决 W7 reconciliation + W8 active），避免 A0 拣入期分两次消。
> **W7 → W8 状态切换总账**：

| Wave | A1 checkpoint 文件 | A0 拣入 commit | 关键 commit 链 | 状态 |
|------|-------------------|----------------|---------------|------|
| W5 | `A1-M5-W5-reconciliation-20260906-1835.md` | `4b438ef` + `f99d2eb` + `1a2c9cd` | + `0e76a89`（A0 W5 dispatch） + `d3f11cd`（A3 W5 assist） + `4b438ef` + `f99d2eb` + `1a2c9cd` | **PASS · A0 拣入** |
| W6 | `A1-M5-W6-reconciliation-20260906-1930.md` | `5f92ece` | + `77b1e3e`（A0 W6 dispatch） + `412d0eb`（A3 W6 MCP 兼容复审） + `add0609`（A6 W6 UI 一致性复审） + `d08d095`（A11 W6 验证 delta） + `5f92ece`（A0 W6 拣入 A8 graph UI + A9 plugin policy + 5 W6 assist + A1 W6 整包） | **PASS · A0 拣入** |
| W7 | `A1-M5-W7-reconciliation-20260907-0050.md`（**未**拣入）| `6c1f30e` + `daa10f6` + `a29b796` | + `a26fbaf`（A0 W7 dispatch） + `6c1f30e`（A0 W7 拣入 A3 mcp_* 3 命令） + `daa10f6`（A11 W7 pre-merge FAIL 3 red lights 拣入） + `a29b796`（A6 W7 wiring 拣入） | **PARTIAL · A0 拣入 A3/A11/A6 lane · A1 W7 reconciliation 整包留 W8 合并** |
| W8 | `A1-M5-W8-reconciliation-20260907-0945.md`（**本轮产出 · 待 A0 拣入**）| 待 A0 拣入 | + `PARALLEL_COMMAND_BOARD.md` W8 段（ahead 3 内 M 状态）+ `6c1f30e` + `daa10f6` + `a29b796` | **ACTIVE · 整包合并 A1 W7 + W8 reconciliation 修订** |

> **W7 收口后遗留债（A11 W7 pre-merge FAIL 同源）**：
> - **W7-1 / W7-3 同源**：plugin.rs `PluginCapability` import 错位（test module L274 缺 + 顶层 L20 unused）—— 修复配方见 `daa10f6` commit message；A0 W8 拣入期消解（**不**留 W9+）。
> - **W7-2**：bridge.rs 8 处未格式化（`cargo fmt --all` 全量修复可解）；同属 A0 W8 拣入期消解。
> - **IF-2**（build metrics growth）：threshold A0 在 `5f92ece` 抬 19%→21%，W7 实测 20.63% <21%（合规）；W8 A5/A6/A8/A9 产品代码增量后 A11 W8 delta 重采。
> - **W8 范围内债**：A5 W8 实施期硬化 Agent/Skill read-only bridge（`agent.rs` / `skills.rs`）的 edge-case tests + validation error shape；A6 W8 UI helper tests；A8 W8 Graph UI polish（accessibility labels / empty/error/oversize states / deterministic filters）；A9 W8 plugin policy review；A10 W8 batch security review；A11 W8 batch verification delta。
> - **A3 W8 HOLD**：A0 在 W8 dispatch 显式 *"A3 is HOLD/NO ASSIGNMENT; Do not continue. Preserve existing work only. Do not rebase, commit, push, or edit product code until A0 resolves the excluded A3 local commit boundary"*；A1 W8 整包**不动** A3 W7 已落地文件（`bridge.rs` / `mcp.rs` / `main.rs` / `default-commands.toml` / `bridge.ts` / `types.ts` / `check-mcp-policy.py`）的 mcp_* 相关代码，仅在 M5-0/M5-13/M5-14 头部事实回填。

---

## [W8 active · 2026-09-07 09:45 CST] 当前活跃 checkpoint 切到 M5-W8（Excluding-A3 dispatch · 5 lane product code + 5 lane docs/review/security/verification · W8 = 硬化 + 复审 + 收口波，**无 MCP 产品代码工作**）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L174-209（**M5-W8 Excluding-A3 Dispatch**，Added 2026-09-07 09:45 CST by A0）+ L7（*"Current NEXT: M5-W8 excluding A3; A3 is HOLD/NO ASSIGNMENT until A0 resolves its local commit boundary; all other lanes may continue W8 below"*）。
> **W8 = 9 lane 全部活跃 · 1 lane（HOLD/NO ASSIGNMENT）**：
> - **A1** *START DOCS ONLY*：**Reconcile W7 non-A3 acceptance and promote W8 as active NEXT**。**Explicitly record that A3 is excluded/held and must not receive new work**。**Update M5 readiness/debt list for Agent/Skill bridge, graph UI, plugin policy, and remaining MCP gap**。scope = `PARALLEL_COMMAND_BOARD.md`（W8 段已存在，不需改）+ `AI-模型切换与接手清单.md` + `详细设计与实施计划.md` + `后续需求TODO.md` + `logs/checkpoints/M5-20260906/*.md` + `logs/checkpoints/A1-M5-W8-*.md`；Must Deliver = **One reconciliation checkpoint; no product code**（A1 W8 角色）。
> - **A2** *START REVIEW ONLY*：非 A3 W7/W8 boundary review（Agent/Skill bridge 不漏到 mvp_core / 不复制 script execution / 不漏到 graph runtime / plugin runtime / MCP runtime）；scope = `logs/assist/A2-M5-W8-*.md`；Must Deliver = Boundary review note with PASS/BLOCKED + actionable line/file refs。
> - **A3** *HOLD / NO ASSIGNMENT*：A0 显式 *"Do not continue. Preserve existing work only. Do not rebase, commit, push, or edit product code until A0 resolves the excluded A3 local commit boundary"*。**A1 W8 在 M5-0/M5-13/M5-14 头部事实回填**A3 W7 拣入事实，**不**动 A3 W7 已落地产品代码，**不**为 A3 W8 写 next-card AC，**不**给 A3 派发任何 W8 工作。
> - **A4** *START REVIEW ONLY*：Agent/Skill bridge + graph/plugin surfaces 隐私/记忆复审（credential redaction / bounded preview / audit contents / no prompt/body 持久化 / no local secret capture）；scope = `logs/assist/A4-M5-W8-*.md`；可推 policy fixtures（不写产品代码）。
> - **A5** *START PRODUCT CODE*：**Harden Agent/Skill read-only bridge after A0 validation** —— 补 edge-case tests + 改 validation error shape（按需）+ 确保 ACL/source check/policy script/frontend bridge/types 维持 atomic；**不**执行 / **不**装 / **不**网络 / **不**持久化写 / **不**启用 plugin；scope = `src-tauri/src/agent.rs` + `src-tauri/src/skills.rs` + `src-tauri/src/bridge.rs` + `src-tauri/src/main.rs` + `src-tauri/permissions/default-commands.toml` + `src/bridge.ts` + `src/types.ts` + `scripts/check-agent-skill-policy.py`；Must Deliver = focused Rust tests PASS + check-agent-skill-policy.py self-test/default PASS + 无 A3/MCP 文件（除非严格 shared ACL/type hunk 且文档化）。
> - **A6** *START UI DOCS/LOGIC*：W7 UI wiring note 转化为 Agent/Skill 面板消费计划 + 纯 UI helper tests；**不**执行 live command 除非 bridge functions 已存在且有 type；**不**视觉重设计；scope = `src/components/**` + `src/stores/**` + `src/types.ts` + `src/bridge.ts` + `scripts/check-agent-skill-ui-logic.mjs` + `logs/assist/A6-M5-W8-*.md`；Must Deliver = UI logic test PASS + checkpoint/assist note。
> - **A7** *START DOCS/GRAPH BRIDGE PLAN ONLY*：**Advance graph bridge plan without MCP/A3 dependency** —— 定义 read-only graph query command contract + capacity/error states + GraphPanel 消费既有 graph store；**不**实施后端命令；scope = `logs/assist/A7-M5-W8-*.md` + optional M5-7/M5-8/M5-9 头部段；Must Deliver = One graph bridge card with blocked-by-A3/MCP items separated from independently shippable UI/store items。
> - **A8** *START UI POLISH/TEST ONLY*：复审 + 磨光已落地 Graph UI pure logic（accessibility labels / empty/error/oversize states / deterministic filters/search / 无 unbounded arrays）；**不**加后端 graph commands；scope = `src/components/**` + `src/stores/**` + `scripts/check-graph-ui-logic.mjs` + `logs/assist/A8-M5-W8-*.md`；Must Deliver = npm run build 或 focused UI logic PASS + patch/checkpoint。
> - **A9** *START POLICY REVIEW ONLY*：Plugin surface W8 review（无 install/enable/delete/download/runtime command 漏入 / lifecycle 维持 pure / capability verdict text 维持 bounded/redacted）；可扩 plugin policy docs/tests 仅在具体 failure 时；scope = `logs/assist/A9-M5-W8-*.md` + optional `scripts/check-plugin-policy.py` policy-only hunk；Must Deliver = Review note 或 policy patch（**无** runtime product code）。
> - **A10** *START SECURITY BATCH REVIEW*：非 A3 W8 outputs 批量安全复审（至少 A5/A6/A8/A9 报后启动）；source check / ACL drift / read-only guarantees / redaction / command payload bounds / no hidden execution/install/network；scope = `logs/assist/A10-M5-W8-*.md`（policy fixtures 仅具体 failure 时）；Must Deliver = **One security verdict, not per-file drip updates**。
> - **A11** *START VERIFICATION BATCH*：维持 W8 verification matrix（**excluding A3**）；记录确切命令 / pass-fail / 残留债 / A0 可否在无 A3 情况下 push；scope = `logs/assist/A11-M5-W8-*.md` + `logs/checkpoints/A11-M5-W8-*.md`；Must Deliver = One final verification delta after implementation lanes finish。
>
> **W8 Hard Stops**（PARALLEL_COMMAND_BOARD L202-209）：
> 1. A3 is **excluded** from W8. **No lane may extend or depend on A3 product-code changes in this wave**.
> 2. **No MCP product-code commands / rmcp runtime / server/listener / plugin install·enable·delete·download / skill execution / model calls / network access**.
> 3. Every command touched by A5 **must remain read-only** and **must include source check, ACL, frontend bridge/types, policy coverage, focused tests in the same package**.
> 4. **No token/cookie/Authorization/body/prompt-secret logging, audit, persistence, checkpoint, or frontend state**.
> 5. Lanes must deliver a **coherent patch/checkpoint** and must **not ask A0 to merge tiny partial notes**.
> 6. **Only A0 pushes to remote**.
>
> **A1 W8 角色与整包交付**：
> - A1 W8 = A1 W7 reconciliation 整包合并 + A1 W8 reconciliation 修订（**W7 reconciliation 整包未进 master · W8 整包合并拣入**）。
> - A1 W8 整包 = 7 M5-* 头部修订（M5-0/9/10/11/12/13/14 含 W7 reconciliation 与 W8 收口）+ 3 份主文档 W8 段 + A1 W8 checkpoint + A1 W8 patch。
> - A1 W8 严格不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json / plugin.rs / mcp.rs / bridge.rs / main.rs / bridge.ts / types.ts / check-*-policy.py —— 全部留给对应 lane。
> - A1 W8 修订本卡（`M5-0-overview.md`）+ 子卡（M5-9/10/11/12）+ 横切卡（M5-13/14）+ 3 份主文档（AI-模型切换与接手清单 / 详细设计与实施计划 / 后续需求TODO）W8 段 + A1 W8 checkpoint + A1 W8 patch —— 详见本 checkpoint §2 整包交付清单。

---

## [W8 reconciliation · 2026-09-07 13:30 CST] W8 整包已 A0 拣入（`4d7be97` + `94e763e` + `a840fcb` + `97118d6`）· 当前活跃 checkpoint 切到 M5-W9

> **A0 拣入 4 commit 事实**（origin/master HEAD `97118d6` + 工作树 clean）：
> 1. **`f51549f docs(A6): M5-W8 Agent/Skill panel consumption plan + pure UI logic tests`**（A6 W8 拣入 panel consumption plan + UI logic tests 文档；**不**接 live command）
> 2. **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8`**（A11 W8 拣入 verification delta：**W8 focused validation 功能性 GREEN**——3 red lights 全部归 A3 W7/W8：fmt 由 A0 在 4d7be97 修 / patch whitespace 由 A0 在 97118d6 修 / MCP `--expect-pending` debt 由 A0 在 94e763e 修；pre-merge 结果**功能性 ALL_PASS**，无新增 cargo 告警）
> 3. **`94e763e fix(M5-W8,A3): close MCP policy phase debt — retire W1 pending semantics, add MCP_NO_RMCP_SERVER + --expect-current-gaps`**（A3 W8 拣入 policy 修复：**MCP policy phase debt 关闭** —— retire W1 pending semantics + 新增 `MCP_NO_RMCP_SERVER` 政策码 + `check-mcp-policy.py --expect-current-gaps` gate 模式 + pre-merge.sh MCP section 接入；ACTIVE=8 PENDING=0；**A3 W8 仍为 `HOLD/NO ASSIGNMENT` 直至此 commit 落地**，之后 A3 解禁但仍 **不** 实施 rmcp/server/listener/network；5 files +665 -180）
> 4. **`4d7be97 feat(M5): integrate W8 command bridge polish`**（A0 W8 拣入：① A5 W8 `agent_validate`/`skill_validate` 错误体 `CredentialLeak` 脱敏（`agent.rs:30-35` + `skills.rs:31-40` + `security_policy.rs:118-120` Display 改不 clone secret）；② A6 W8 `npm run build` PASS + 79 agent-skill UI logic 断言；③ A8 W8 graph UI polish 4 vue 文件修订（EdgeDetail/GraphFilter/GraphPanel/GraphViewer/NodeDetail + ActivityBar/MainArea + useGraphStore + graphUi + check-graph-ui-logic.mjs 41 断言）；④ A9 W8 `check-plugin-policy.py` 复审补丁；⑤ A11 W8 verification delta 收口；⑥ A2/A4/A5/A7/A8/A9/A10 8 份 W7+W8 assist；⑦ **A1 W7 reconciliation 整包合并拣入**（A1 W7 checkpoint + patch 共 9 文件 = +292 -15）；⑧ **A1 W8 reconciliation 整包合并拣入**（A1 W8 checkpoint + patch + M5-0/9/10/11/12/13/14 修订共 11 文件 = +488 -15）；⑨ 8 份 W7 assist + 8 份 W8 assist + 8 份 W7 patch = 53 files +6038 -101）
> 5. **`97118d6 chore(M5): normalize W8 patch evidence whitespace`**（A0 W8 拣入 patch 空白规范化：3 份 W8 patch 文件空白整理）
>
> **A0 W8 拣入对 A1 W8 reconciliation 整包的应答**：
> - A1 W8 reconciliation 整包 11 文件 = 7 M5-* 修订 + 2 主文档 + 1 W8 checkpoint + 1 W8 patch = **完整拣入**（4d7be97 stat 显示）
> - A1 W7 reconciliation 整包 9 文件 = 7 M5-* 修订 + 1 W7 checkpoint + 1 W7 patch = **一并合并拣入**（A0 选择不二次拆批）
> - A0 W8 拣入事实**与 A1 W8 reconciliation 整包内容一致**：[W8 verification scope] 段在 M5-13 L152-193 完整记录了 W8 verification delta + 残留债；[W8 active] 段已记录 9 lane W8 任务 + 6 hard stops；DEBT-41~51 在 M5-14 L280-310 完整
> - A0 自拣入 M5-14 L1 = "W8 update 14:30 CST" + L304-310 IF-2 阈值修订 19% → **22%**（W9 阈值）
> - 4 commit 拣入后 M5-0/M5-13 L1 顶部 history 链 + M5-14 顶部 build metrics 阈值 **需 A1 W9 整包回填**（本段即此目的）
>
> **W8 reconciliation 自检 PASS 清单**（A1 W9 拣入前必检）：
> - [x] **A5 `CredentialLeak` 错误体脱敏**（94e763e 关闭 F-W8-1 critical · Display 不 clone secret）
> - [x] **A6 `npm run build` PASS**（4d7be97 关闭 F-W6-2 UI 编译失败）
> - [x] **A8 graph UI 41 断言 PASS**（4d7be97 关闭 F-W6-1 graph-ui 缺断言）
> - [x] **A9 `check-plugin-policy.py` ACTIVE=6**（W7 + W8 累计；plugin install/enable/delete/download 仍缺）
> - [x] **A11 verification delta 功能性 GREEN**（a840fcb 关闭 W7-1/W7-2/W7-3 配方未消 = 已消）
> - [x] **A3 MCP policy phase debt 关闭**（94e763e 关闭 DEBT-44；ACTIVE=8 PENDING=0）
> - [x] **cargo fmt --all**（4d7be97 修 W7-2 8 处未格式化）
> - [x] **patch whitespace normalize**（97118d6 修 W7-3 patch 空白）
> - [x] **build metrics W8 实测 21.07% ≤ 22% PASS**（IF-2 W8 关闭）
> - [x] **cargo_warnings delta = 0**（PluginCapability unused import 已删）
> - [x] **pre-merge ALL_PASS**（A11 W8 verification delta 确认）
> - [x] **A1 W7 + W8 reconciliation 整包 20 文件合并拣入**（4d7be97 关闭 DEBT-42）
> - [x] **A3 local commit boundary 已由 A0 94e763e 解禁**（A3 仍 NO_NEW_RMCP）
>
> **W8 残留债挂账（[W8 verification scope] 段已收）**：
> - **DEBT-43**（DRY-F1 残留扩大 = SENSITIVE_KEY_NAMES 双份 + 图谱容量常量三处拷贝）→ A2 W9 review 必填；A2 W9 抽 `domain.rs` 单源
> - **DEBT-46~51**（A5/A6/A7/A8/A9/A10 W8 必批闭环项）→ A0 W8 拣入期已通过 4d7be97 + 94e763e + a840fcb 闭环
> - **DEBT-12**（plugin install/enable/delete/download 仍缺）→ 后续 wave，非 W8 必消
> - **DEBT-22**（build metrics 21% 阈值 IF-2）→ A0 94e763e 后改 22%，W9 阈值不变
> - **DEBT-04**（plugin UI 仍 0% ACTIVE）→ A8 W9 仍 polish 范围
> - **DEBT-23~26**（终端 M3 挂账）→ 非 M5 范畴
> - **DEBT-31~33**（A7 graph 桥接 A3/MCP 阻塞）→ A3 解禁后可推 M5-2.b 卡

---

## [W9 active · 2026-09-07 14:30 CST] 当前活跃 checkpoint 切到 M5-W9（Runtime-Free Polish Dispatch · 11 lane 全部活跃 · W9 = runtime-free 磨光 + 终验收口波 · build metrics 阈值 22%）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L175-210（**M5-W9 Runtime-Free Polish Dispatch**，Added 2026-09-07 14:30 CST by A0）+ L4-5（*`Current NEXT: M5-W9 integration follow-up; W8 focused validation passed; all lanes W9 active; A2 W9 review boundary of W8 fixes; A4 W9 privacy review of W8 redaction; A11 W9 final verification matrix; build metrics threshold 22%`*）+ L7（*`Each lane 整包 deliver patch+checkpoint; only A0 pushes`*）。
>
> **W9 派发事实**：W8 focused validation 功能性 ALL PASS（a840fcb 确认）→ A0 派发 W9 收口波，**W9 = 复审 W8 修复 + 终验 + 残留债 挂账**；不实施任何新 backend runtime / 不实施 rmcp server/listener/network / 不装 / 不持久化 / 不调模型。
>
> **W9 = 11 lane 全部活跃**（vs W8 = 9 lane active + 1 lane（HOLD））：
> - **A1** *START DOCS ONLY*：**Reconcile W8 as accepted-with-fixes and mark W9 as active NEXT**。**Update M5 child-card status for MCP read-only bridge (M5-2), Agent/Skill read-only bridge (M5-4/5/6), Graph UI polish (M5-9), plugin review (M5-10/11/12), and build metrics threshold 22%**。scope = `PARALLEL_COMMAND_BOARD.md`（W9 段已存在，不需改）+ `AI-模型切换与接手清单.md` + `详细设计与实施计划.md` + `后续需求TODO.md` + `logs/checkpoints/M5-20260906/*.md` + `logs/checkpoints/A1-M5-W9-*.md`；Must Deliver = **One reconciliation checkpoint + W9 [W8 reconciliation] + [W9 active] 段；no product code**。
> - **A2** *START REVIEW ONLY*：**重审 A3/A5 W8 修复后 command boundary**（MCP `MCP_NO_RMCP_SERVER` 落地后 / A5 `CredentialLeak` 脱敏后 / A3 policy `ACTIVE=8 PENDING=0` 后 / A1 W7+W8 整包拣入后）；scope = `logs/assist/A2-M5-W9-*.md`；Must Deliver = Boundary review note with PASS/BLOCKED + actionable line/file refs + DEBT-43 (DRY-F1) 抽 `domain.rs` 单源。
> - **A3** *START POLICY/REVIEW ONLY*：**MCP policy current-phase green + 后续 M5-2.b 卡预备**（94e763e 落地后 `MCP_NO_RMCP_SERVER` + `--expect-current-gaps` 模式可默认扫描）；**不**实施 rmcp server / listener / network / 任何 M5-2.b 卡 PENDING 项；scope = `scripts/check-mcp-policy.py` + `src-tauri/src/mcp.rs` + `logs/assist/A3-M5-W9-*.md`；Must Deliver = Policy review note 或 M5-2.b 卡预备 plan（**无** runtime product code）。
> - **A4** *START PRIVACY REVIEW ONLY*：**重审 A5 CredentialLeak redaction 后所有命令 error/display 面的 secret echo**（`agent_validate`/`skill_validate`/`mcp_capability_preview` 错误体）；scope = `logs/assist/A4-M5-W9-*.md`；Must Deliver = Privacy review note with PASS/BLOCKED + redacted Display verification + DEBT-32 secret-echo 残留债收口。
> - **A5** *START PRODUCT CODE SMALL*：**Agent/Skill read-only bridge hardening 收口** —— redacted validation errors（`CredentialLeak` Display 不 clone secret · 沿用 4d7be97 模式）+ frontend parse/permission preview 边界 case 测试；**不**执行 / **不**装 / **不**网络 / **不**持久化写 / **不**启用 plugin / **不**调模型；scope = `src-tauri/src/agent.rs` + `src-tauri/src/skills.rs` + `src-tauri/src/bridge.rs` + `src-tauri/src/main.rs` + `src/permissions/default-commands.toml` + `src/bridge.ts` + `src/types.ts` + `scripts/check-agent-skill-policy.py`；Must Deliver = focused Rust tests PASS + check-agent-skill-policy.py self-test/default PASS + 至少 1 个 redacted Display 边界 case 测（如 secret 出现在 input.name 后置 / system_prompt 嵌套 / description URL 内）+ 无 A3/MCP 文件改动（除非严格 shared ACL/type hunk 且文档化）。
> - **A6** *START UI LOGIC ONLY*：Agent/Skill 面板消费磨光 —— 确定性 empty/error/loading + 无 secret 文本 echo + bounded preview 渲染；**不**视觉重设计；scope = `src/components/**` + `src/stores/**` + `src/types.ts` + `src/bridge.ts` + `scripts/check-agent-skill-ui-logic.mjs` + `logs/assist/A6-M5-W9-*.md`；Must Deliver = UI logic test PASS + 至少 1 个 redacted-display 边界 case UI 测 + checkpoint/assist note。
> - **A7** *START GRAPH CONTRACT DOCS ONLY*：图谱桥契约终稿（runtime-free 与 blocked backend runtime 分离）；**不**实施后端 graph command；scope = `logs/assist/A7-M5-W9-*.md`；Must Deliver = One graph bridge contract final card with BLOCKED-BY-A3/MCP items vs independently shippable UI/store items 明确分离。
> - **A8** *START GRAPH UI SMALL*：图谱 UI 磨光 —— filter/search/layout 确定性 + 保留 bounded arrays + 改进 no-backend/read-only 状态；**不**实施后端 graph command；scope = `src/components/**` + `src/stores/**` + `scripts/check-graph-ui-logic.mjs` + `logs/assist/A8-M5-W9-*.md`；Must Deliver = npm run build 或 focused UI logic PASS + patch/checkpoint。
> - **A9** *START PLUGIN REVIEW ONLY*：plugin surface 复审 —— manifest/lifecycle 维持 pure + 确认 install/enable/delete/download 仍缺（DEBT-12 不在 W9 消）+ capability verdict text 维持 bounded/redacted；**不**实施 plugin runtime；scope = `logs/assist/A9-M5-W9-*.md` + optional `scripts/check-plugin-policy.py` policy-only hunk；Must Deliver = Review note 或 policy patch（**无** runtime product code）。
> - **A10** *START SECURITY FINAL REVIEW*：批量复审 W8/W9 outputs —— source check / ACL parity / read-only / redaction / 无 runtime 扩张；scope = `logs/assist/A10-M5-W9-*.md`（policy fixtures 仅具体 failure 时）；Must Deliver = **One final security verdict, not per-file drip updates**。
> - **A11** *START FINAL VERIFICATION*：**W9 final verification matrix** —— 命令结果 + build metrics 21.07% ≤ 22% + cargo warnings unchanged + pre-merge result + 残留债 + push readiness；scope = `logs/assist/A11-M5-W9-*.md` + `logs/checkpoints/A11-M5-W9-*.md`；Must Deliver = One final verification matrix（**功能性 ALL_PASS gate**，残留债可挂账）。
>
> **W9 Hard Stops**（PARALLEL_COMMAND_BOARD L211-）：
> 1. **No mcp_* runtime / no rmcp server/listener / no network / no plugin install·enable·delete·download / no skill execution / no model calls** in W9.
> 2. **No mcp_*.rs file changes outside scripts/check-mcp-policy.py policy-only hunks** in W9.
> 3. **Build metrics threshold 22% (unchanged from W8 closure). 22% gate MUST NOT regress.**
> 4. **No token/cookie/Authorization/body/prompt-secret logging, audit, persistence, checkpoint, or frontend state**.
> 5. **No mvp_core / no script_execution / no mcp_runtime / no plugin_runtime expansion** in W9.
> 6. **No new cargo warnings**. Cargo warnings delta MUST = 0.
> 7. **Lanes must deliver a coherent patch/checkpoint and must not ask A0 to merge tiny partial notes**.
> 8. **Only A0 pushes to remote**.
>
> **A1 W9 角色与整包交付**：
> - A1 W9 = A1 W8 reconciliation 整包已拣入事实回填（[W8 reconciliation] 段）+ A1 W9 派发事实回填（本段）。
> - A1 W9 整包 = 7 M5-* 头部修订（M5-0/9/10/11/12/13/14）+ 3 份主文档 W9 update 行（AI-模型切换与接手清单 / 详细设计与实施计划 / 后续需求TODO）+ A1 W9 checkpoint + A1 W9 patch。
> - A1 W9 严格不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json / plugin.rs / mcp.rs / bridge.rs / main.rs / bridge.ts / types.ts / check-*-policy.py —— 全部留给对应 lane。
> - A1 W9 修订本卡（`M5-0-overview.md`）+ 子卡（M5-9/10/11/12）+ 横切卡（M5-13/14）+ 3 份主文档 W9 update 行 + A1 W9 checkpoint + A1 W9 patch —— 详见本 checkpoint §2 整包交付清单。

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
