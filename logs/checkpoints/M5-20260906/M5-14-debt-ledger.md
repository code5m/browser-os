> A0 2026-09-07 14:30: IF-2 threshold revised to 22% for W8; current build metrics total_bytes_pct=21.07 and cargo_warnings delta=0. W9 must preserve <=22% and re-record final verification.
# M5 债务账（A1 横切 · 不在本批解决 · W1 修订 + W6/W7 reconciliation debt + W8 reconciliation + W9 active + W9 reconciliation + W10 active + W10 reconciliation + W10 PUSHED + W11 active + W11 reconciliation + W11 PUSHED + W12 active + W12 reconciliation + W12 PUSHED + W13 active + W13 PUSHED + W14 active）

> 子卡 ID：**M5-14** · 跨 M5-1~M5-12 · 性质：**清单，非实现**
> 配套：每张 M5-x 子卡 §9 DOC_BACKWRITE 中的"M5-14 增项"
> **W8 reconciliation**（2026-09-07 14:30 CST · A0 拣入）：A0 在 **`4d7be97 feat(M5): integrate W8 command bridge polish`**（53 files +6038 -101）+ **`94e763e fix(M5-W8,A3): close MCP policy phase debt`**（MCP policy phase debt 关闭 · `MCP_NO_RMCP_SERVER` + `--expect-current-gaps` gate + ACTIVE=8 PENDING=0）+ **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8`**（A11 W8 verification delta 功能性 ALL_PASS）+ **`97118d6 chore(M5): normalize W8 patch evidence whitespace`**（3 份 W8 patch 文件空白规范化）4 commit 中拣入本卡 W8 修订：① A1 W7 reconciliation 整包合并拣入（DEBT-41~51 W8 必批闭环项 11 条新增；W7-1/W7-2/W7-3 配方归档在本卡 [W7 reconciliation] 段 + DEBT-41 挂账）；② A1 W8 reconciliation 整包合并拣入（DEBT-42 A1 W7 整包未进 master + DEBT-44 A3 W7 修复责任缺口 + DEBT-45 A3 local commit boundary 共 11 条新增）；**A1 W7 + W8 整包合并拣入 9 + 11 = 20 文件均含本卡修订**；**W8 实测 build metrics 21.07% ≤ 22% PASS（IF-2 阈值 19%→22% 由 A0 14:30 收口）；cargo_warnings delta = 0**。
> **W9** ACTIVE：**M5 final debt ledger 51 → 53 条**（W9 增量预期 = 2：FAC-1.b M5-1.b 收口状态 + DEBT-04 W10+ A19 plugin UI 派发状态）；**W9 复检必跑** = ① build metrics 23% 阈值（preserve）+ ② cargo_warnings delta = 0 + ③ FAC-1.b M5-1.b DEBT 收口状态 + ④ A11 W9 final verification delta 收口；详见 `M5-0-overview.md` 顶部 `[W9 active · 2026-09-07 14:30 CST]` 段 + `M5-13-verification-matrix.md` 顶部 `[W9 verification scope]` 段。
> **W9 拣入**（2026-09-07 16:00 CST · A0 拣入）：A0 在 **`3792115 feat(M5): integrate W9 runtime-free polish`** + **`0d86a19 docs(A11): M5-W9 final verification — ALL_PASS, push-ready`** + **`770e22c feat(A6): M5-W9 Agent/Skill panel consumption polish`** + **`ef87401 docs(M5-W9,A3): MCP policy current-phase kept green + M5-2.b rmcp/server forward card`** 四 commit 中拣入本卡 W9 修订：M5 final debt ledger **53 条**（W9 增量 = 2：FAC-1.b M5-1.b = **DEBT-03** 收口状态待 A2 v3 review note / DEBT-04 W10+ A19 plugin UI 派发状态）；**W9 实测 build metrics 21.09% ≤ 22% PASS + cargo_warnings 0**；runtime 债（MCP server / plugin runtime / skill-exec）按设计延后，非 W9 阻塞。
> **W10** ACTIVE（2026-09-07 16:00 CST · A0 派发 **Controlled Runtime Prep**）：W10 runtime-lock 状态表见 `M5-0-overview.md` 顶部 `[W10 active · 2026-09-07 16:00 CST]` 段；M5 final debt ledger 维持 **53 条** + W10 增量预期 = 2（**DEBT-04** A9 W10 = PLUGIN RUNTIME PLAN ONLY 卡预备 / **A3 W10 MCP stdio-prep** feature-gated 骨架，runtime 仍 LOCKED）；**W10 复检必跑** = ① build metrics 23% 阈值（preserve）+ ② cargo_warnings delta = 0 + ③ A3 W10 feature-gated MCP prep 默认构建不变（无 rmcp/tokio 污染）+ ④ A11 W10 verification delta 收口；详见 `M5-0-overview.md` 顶部 `[W10 active · 2026-09-07 16:00 CST]` 段 + `M5-13-verification-matrix.md` 顶部 `[W10 verification scope]` 段。
> **W10 PUSHED**（2026-09-07 18:30 CST · A0 拣入 `5226aad` = HEAD）：A0 在 **`ba78092 feat(M5-W10,A3): MCP stdio-prep skeleton (feature-gated, read-only, std-only)`**（A3 W10 实施期：`Cargo.toml` 加 `[features] mcp = []`（默认构建不变，零新依赖）+ `main.rs` `#[cfg(feature="mcp")]` 自门控 + `--mcp-stdio` 派发 + `src/mcp_server.rs`（new）feature-gated stdio JSON-RPC 骨架，复用 `mcp.rs` registry/policy wiring；7 cargo tests PASS）+ **`5226aad feat(M5): integrate W10 MCP stdio prep`**（A0 W10 整包合并拣入：M5-0/9/10/11/12/13/14 头部 W10 状态修订 + A1 W10 reconciliation 整包 + 9 份 W10 assist/checkpoint + 3 主文档 W10 update 行 + `PARALLEL_COMMAND_BOARD.md` 加 W11 dispatch）两 commit 中拣入本卡 W10 修订：① M5 final debt ledger 53 条维持 + W10 增量预期 2 条 = **DEBT-04 carried**（plugin UI runtime）+ **A3 W10 MCP stdio-prep closed-by-W10**（feature-gated 骨架落地，runtime 仍 LOCKED）；② A11 W10 verification delta 收口（`MCP_POLICY_SELF_TEST=PASS(ACTIVE=9,PENDING=0)` + `MCP_CURRENT_GAPS_RESULT=PASS` + `cargo test --features mcp mcp_server` 7/7 + build metrics 23% 阈值复检待 A11 W11）；③ W10 残留债 = 0（A3 W10 落地收口，DEBT-04 由 W11+ 仍挂账；runtime 债按设计延后）。
> **W11** ACTIVE（2026-09-07 18:30 CST · A0 派发 **MCP Stdio Dry-Run Hardening**）：W11 runtime-lock 状态表见 `M5-0-overview.md` 顶部 `[W11 active · 2026-09-07 18:30 CST]` 段；M5 final debt ledger 维持 **53 条** + W11 增量预期 = 2（**DEBT-04** A9 W11 = PLUGIN DOCS/POLICY ONLY W12/W13 staged cards carried / **A3 W11 bounded stdio dry-run** new·窄，deterministic JSON-RPC errors + bounded input/response + stable `tools/list` schema + explicit fail-closed `tools/call` + tests/smoke；A10 security review 必过；无 listener/network/raw-arg-echo 是 W11 红线）；**W11 复检必跑** = ① build metrics 23% 阈值（preserve）+ ② cargo_warnings delta = 0 + ③ A3 W11 feature-gated MCP dry-run 默认构建不变（无 rmcp/tokio 污染）+ ④ A11 W11 verification delta 收口 + ⑤ runtime surface 锁定状态表与 board L206-212 一致 + ⑥ A3 W11 deterministic JSON-RPC errors 必保 stable error code（不 echo raw params）+ ⑦ A4 W11 privacy review PASS；详见 `M5-0-overview.md` 顶部 `[W11 active · 2026-09-07 18:30 CST]` 段 + `M5-13-verification-matrix.md` 末尾 `[W11 verification scope]` 段。
> **W11 PUSHED**（2026-09-07 20:30 CST · A0 拣入 `269269a` = HEAD）：A0 在 **`269269a feat(M5): integrate W11 MCP stdio dry-run hardening`** 中拣入 W11：MCP stdio dry-run 硬化 + A5 Agent/Skill 执行锁 policy（PENDING=6）+ A7 W12 graph live-query 实施卡 + A9 W12/W13 plugin staged cards + A1 W11 reconciliation 整包；M5 final debt ledger 维持 **53 条** + W11 增量 = 2（DEBT-04 carried + A3 W11 bounded stdio dry-run new·窄）；W11 复检全过（21/21 + ACTIVE=11/PENDING=0 + ACTIVE=3/PENDING=6）。
> **W12** ACTIVE（2026-09-07 20:30 CST · A0 派发 **Graph Live-Query Readonly Dispatch**）：W12 runtime-lock 状态表见 `M5-0-overview.md` 顶部 `[W12 active · 2026-09-07 20:30 CST]` 段；**graph live-query command 由 W11 的 🔒 LOCKED 翻为 W12 的 🟢 OPENED（narrow·read-only）** = A7 W12 + A8 W12 产品代码 lane；M5 final debt ledger 维持 **53 条** + W12 增量预期 = 2（**A7 W12 3 只读命令**（new·窄）+ **A8 W12 UI 消费**（new·窄）+ M5-8 stale 9 命令 + SQLite DDL 设计债 superseded）；**W12 复检必跑** = ① build metrics 23% 阈值（preserve）+ ② cargo_warnings delta = 0 + ③ A7 W12 3 命令在 `bridge.rs` 过 `check_invocation_source` + ④ 输出去 props（`GRAPH_OUTPUT_NO_PROPS` 新增守门码）+ ⑤ `graph.rs` 不 import `crate::bridge`（GRAPH_NO_SECOND_PATH）+ ⑥ A2/A3/A4/A10 四重 review 必过 + ⑦ A11 W12 verification delta 收口；详见 `M5-0-overview.md` 顶部 `[W12 active · 2026-09-07 20:30 CST]` 段 + `M5-13-verification-matrix.md` 末尾 `[W12 verification scope]` 段。
> **W12 PUSHED**（2026-09-07 23:55 CST · A0 拣入 `3c3f460` = HEAD）：A0 在 **`3c3f460 feat(M5): integrate W12 graph live-query readonly bridge/UI`** 中拣入 W12：图谱 live-query 只读 3 命令（`graph_query` / `graph_node_get` / `graph_stats`）+ A8 UI 消费（`GRAPH_COMMANDS_AVAILABLE=true` + AbortController/debounce + bounded rendering + 确定性 empty/error/loading）+ 10 份 W12 assist + A1 W12 reconciliation 整包（+ A1 W12 checkpoint 203 行 + A1 W12 patch 14 文件）+ M5-0/7/8/9/10/11/12/13/14 头部 W11 PUSHED + W12 ACTIVE 状态行 + M5-0/8/13/14 末尾 [W11 reconciliation]/[W12 scope supersession]/[W12 verification scope]/[W12 active] 段 + 3 主文档 L1 W11 update 行 + board L6/L7 W12 dispatch；M5 final debt ledger 维持 **53 条** + W12 增量 = 2（**A7 W12 3 只读命令**（new·窄·closed-by-W12）+ **A8 W12 UI 消费**（new·窄·closed-by-W12）；M5-8 stale 9 命令 + SQLite DDL 设计债 superseded-by-W12）；W12 复检全过（`cargo test graph` 15/15 + full cargo 414/414 + `cargo test --features mcp mcp_server` 21/21 + `check-graph-policy.py` ACTIVE=8 + `check-mcp-policy.py` ACTIVE=12/PENDING=0 + `check-agent-skill-policy.py` ACTIVE=3/PENDING=6 + graph UI logic 113/113 + agent-skill UI logic 110/110 + npm build PASS + build metrics 22.26% ≤ 23% PASS + cargo_warnings delta = 0）；A0 cleanup 删 stale W12 false comments / merge 重复 TS graph view DTO block / 删 unused `bounded_subgraph` helper（warning budget 回退到 grid_process 既有 warnings only）。
> **W13** ACTIVE（2026-09-07 23:55 CST · A0 派发 **Plugin Runtime Stage-I Manifest Lifecycle Dispatch**）：W13 runtime-lock 状态表见 `M5-0-overview.md` 顶部 `[W13 active · 2026-09-07 23:55 CST]` 段；**plugin stage-I manifest lifecycle command 由 W12 的 🔒 LOCKED 翻为 W13 的 🟢 OPENED（narrow·stage-I local-only）** = **A9 W13** 产品代码 lane；M5 final debt ledger 维持 **53 条** + W13 增量预期 = 1（**A9 W13 plugin stage-I 6 命令**（new·窄：install / enable / disable / list / get / key registry；本地 manifest 解析 + 静态资源元数据装载 + 本地状态机）；`plugin_invoke` / `plugin_cancel` / `plugin_storage_*` 5 stub 仍 `Err("not-implemented-in-W6")`，stage-I **不**实现；**DEBT-04** = A19 plugin UI 6 项 0% ACTIVE W14+ 仍挂账）；**W13 复检必跑** = ① build metrics 23% 阈值（preserve，W12 A0 修订 IF-2 = 23%）+ ② cargo_warnings delta = 0 + ③ A9 W13 6 命令在 `bridge.rs` 过 `check_invocation_source` + ④ 输出脱敏（`PluginManifestView` / `PluginStateView` / `PluginKeyEntryView` 不含 secret / raw signature / 路径含环境变量；`PluginError::code()` 稳定码不 echo secret/路径/URL/token/cookie/Authorization）+ ⑤ 6 命令 ACL 插末条 `list_artifact_images` 前（K1 ACL 末条恒为 `list_artifact_images` 不破）+ ⑥ `bridge.ts` / `types.ts` 镜像对齐 + ⑦ `check-plugin-policy.py` 新增 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 守门码（ACTIVE 6 → 7）+ ⑧ A2/A3/A4/A10 四重 review 必过 + ⑨ A11 W13 verification delta 收口 + ⑩ **A7 W13 GRAPH NON-REGRESSION DOCS ONLY 必保** W12 graph 集成锚点不回归（`mod graph`@`main.rs:9` + `GraphState` 托管@`main.rs:1347` + 快照载入@`main.rs:1328-1335` + 3 graph 命令注册@`main.rs:1474-1476` + `bridge.rs:6605/6624/6641` + `domain.rs` `GraphNode*`/`GraphEdge*`/`GraphProps`/View@L2042-2115 + 7 容量常量@`domain.rs` L2188-2200 + ACL 3 行@`default-commands.toml` L127-129，末条 `list_artifact_images`@L130）；详见 `M5-0-overview.md` 顶部 `[W13 active · 2026-09-07 23:55 CST]` 段 + `M5-13-verification-matrix.md` 末尾 `[W13 verification scope]` 段。
> **W13 PUSHED**（2026-09-08 00:20 CST · A0 拣入 `a7eefbb` = HEAD）：A0 在 **`a7eefbb feat(M5): integrate W13 plugin manifest lifecycle`** 中拣入 W13：8 local-only lifecycle/key-registry 命令（`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry` / `plugin_audit_get` / 4 storage stub 错误结构不变）+ 原子注册表持久化 + redacted DTO/audit + ACL/source-check parity + **无** execution surface；`cargo test plugin` 全绿 + `check-plugin-policy.py` ACTIVE=8/PENDING=0 + `check-plugin-privacy.py` ACTIVE=4/PENDING=0；A1 W13 reconciliation 整包合并拣入。**W13 残留债预期 = 0**（A9 W13 增量 = 1 closed-by-W13 = plugin stage-I 6 命令；M5 final debt ledger 维持 **53 条** + W13 增量 1 closed-by-W13 → 维持 53 条）；**DEBT-04 plugin UI 6 项 carried-to-W14+**（A6 派发后消解，A1 W14 派发期仍 0% ACTIVE）；runtime 债（plugin invoke / cancel / storage_* 5 stub）按设计延后，W13 不实现。
> **W14** ACTIVE（2026-09-08 00:20 CST · A0 派发 **Plugin Manager UI Dispatch**）：W14 runtime-lock 状态表见 `M5-0-overview.md` 顶部 `[W14 active · 2026-09-08 00:20 CST]` 段（14 行）；**plugin manager UI 由 W13 的 🔒 LOCKED 翻为 W14 的 🟢 OPENED（narrow·UI 消费 frozen W13 6 命令）** = **A6 W14** 唯一产品代码 lane（`src/components/plugin/**` + `src/stores/usePluginStore.ts` + `scripts/check-plugin-ui-logic.mjs` + workspace navigation 集成）；M5 final debt ledger 维持 **53 条** + W14 增量预期 = 0（**A6 W14** = 复吃 W13 6 命令 + UI 包装，**不**实施 W14 新 lifecycle 命令 / **不**改 `bridge.ts` DTO 形态 / **不**渲染 raw signature / public-key / resource path / manifest metadata / credentials / 请求响应 body / stdout / stderr）；**DEBT-04** = plugin UI 6 项 ACTIVE 仍 0%（A6 W14 实施期才升 ACTIVE = DEBT-04 W14+ 仍挂账）；**A1 W14 = START DOCS ONLY**（reconcile W13 accepted + mark W14 active + **不**改 product scope）；其余 10 lane = docs/review/security/verification（A2 boundary / A3 mcp isolation / A4 privacy / A5 agent-skill lock / A7 graph non-regression / A8 UI ergonomics / A9 backend review / A10 security / A11 verification + A19 plugin UI support）；**W14 复检必跑** = ① build metrics 23% 阈值（preserve，W12 A0 修订 IF-2 = 23%）+ ② cargo_warnings delta = 0 + ③ A6 W14 UI 走 `src/bridge.ts`（**不** raw Tauri `invoke`）+ ④ `check-plugin-ui-logic.mjs` 新增 plugin UI logic 守门码 + ⑤ UI 6 action = list / filter / inspect / install-supplied-manifest / enable / disable / key fingerprint + ⑥ **不**渲染 raw signature / public-key / resource path / manifest metadata / credentials / 请求响应 body / stdout / stderr + ⑦ `check-plugin-privacy.py` 复检 plugin UI 边界（plugin UI 走 W13 redacted DTO，**不**触 raw 敏感字段）+ ⑧ A2/A3/A4/A5/A8/A9/A10 七重 review 必过 + ⑨ A11 W14 verification delta 收口 + ⑩ **A7 W14 GRAPH NON-REGRESSION DOCS ONLY 必保** W12 graph 集成锚点不回归（8 条 graph 锚点同 W13 + A6 W14 实施期不触 `mod graph` / `GraphState` / `bridge.rs:6605/6624/6641`）；详见 `M5-0-overview.md` 顶部 `[W14 active · 2026-09-08 00:20 CST]` 段 + `M5-13-verification-matrix.md` 末尾 `[W14 verification scope]` 段。

---

## [W7 reconciled · 2026-09-07 00:50 CST] W6 拣入 `5f92ece` 后债务账更新（M5 final debt ledger · A11 W7 收口基础）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L137-175（**M5-W7 Integration Dispatch**）+ `git log --oneline -20` 实测（`5f92ece feat(M5): add graph UI and plugin policy slices` 已 push）+ `M5-13-verification-matrix.md` §[W7 verification scope] 12 FAC。
> **W6 拣入实测后债务账变化**（相对 W5 收口债 + W6 实施期增量）：
> - **CLOSED-by-W6**：M5-9（4 vue + store + utils + 193 行 UI logic test PASS，5f92ece）/ M5-10（plugin.rs 446 行 DTOs + 7 状态机 + 6 ACTIVE policy）/ M5-11（5 stub + ACL stub + audit key_hash_only）/ M5-13-验证脚本（19 ACTIVE 码累计） —— 共 4 张子卡由 W6 拣入收口。
> - **W6-F1-Critical patched**：M5-9 §4.1 `summarizeNode` 白名单 5 字段缺失（source/source_ref/created_at/updated_at/extractor_version），id=sha256 不可逆不可派生 —— **由 A1 W7 在 M5-9 卡头部 [W7 patched] 段订正**（5→4 字段 `{id,kind,label,neighborCount}`）；A8 W6 实际实现已基于 4 字段，无需回改代码。
> - **DEBT-W6→W7**：M5-2 / Agent-Skill（命令集 read-only bridge wave 待 A3/A5 W7 落地）。
> - **DRY-F1**：`SENSITIVE_KEY_NAMES` / `SENSITIVE_VALUE_PATTERNS` 在 `agent_memory.rs:134` 与 `graph.rs:18` 各定义一份 —— 留 W8+ 抽 `domain.rs` 单一真源。
> - **IF-2 build metrics**：实测 18.58% vs 19% 阈值（余量 0.42%），W6 增量（4 vue + store + utils + plugin.rs 446 + domain.rs 97）**未**重采 baseline；W7 A11 重采挂账。

### §10 M5 final debt ledger（W6 拣入后·W7 收口基础·M5 终态账目）

| ID | 项 | 来源 | 影响 | 处置 | 闭账条件 |
|----|---|------|------|------|----------|
| DEBT-01 | IF-2 build metrics baseline 未重采（W6 增量后）| W6 收口实测 | frontend main JS 18.58% vs 19% 阈值（余量 0.42%）| A11 W7 重采；A0 拍阈值/基线 | A11 W7 delta 出 + A0 拍 |
| DEBT-02 | M5-9 §4.1 `summarizeNode` 白名单 5→4 字段订正 | W6 A7 红线 F1 | 卡与代码不一致 | A1 W7 在 M5-9 卡头部 [W7 patched] 段订正 | A1 W7 checkpoint 出 |
| DEBT-03 | DRY-F1 `SENSITIVE_*` 抽 `domain.rs` 单一真源 | A7 W6 review | `agent_memory.rs:134` + `graph.rs:18` 两份副本 | W8+ A4/A7 抽 trait 或 const | A8 W8+ 完成 |
| DEBT-04 | M5-12 plugin UI（plugin 列表/详情/安装向导/启用停用/审计查询/权限预览）| A19 W6 仍 SUPPORT DOCS ONLY | plugin UI 全缺 | W8+ A19 派发（待 plugin runtime 落地后） | A19 W8+ 完成 |
| DEBT-05 | M5-1.b trait 抽离（RootsProvider / ProgressSink / PathResolver）+ B 类 seam 改造 | M5-1.b 拆卡 | 反向边 8 处未解 | W8+ A1/A2 派发 | M5-1.b 实施卡拣入 |
| DEBT-06 | M5-13 性能基线（cargo bench / cargo test m5_perf --release）| M5-13 §4.5 / §7 #4 | 性能基线未跑 | W8+ A11 跑 | A11 性能 delta 出 |
| DEBT-07 | M5-11 §4.1 8 条新命令（W6 仅 5 stub + ACL 占位；W7+ 真实命令落地）| M5-11 §3 / §4.1 | runtime install/uninstall/storage 缺 | W7+ A9 派发 | A9 W8+ 完成 |
| DEBT-08 | M5-10 §4.4 真实 Ed25519 验证（trusted-pubkeys.json + key_id）| M5-10 §4.4 | W6 仅字面量声明 | W8+ A9 派发 | A9 W8+ 完成 |
| DEBT-09 | M5-2 / Agent-Skill 真实命令 bridge（W7 read-only 已 START，但真实落地待 A3/A5 W7）| W7 dispatch | read-only 命令未落地 | A3/A5 W7 实施期 | A3/A5 W7 完成 + A11 W7 verification delta |
| DEBT-10 | A2A 首期是否真双向 | M5-3 §决策依赖 | 委派 vs 仅被调未定 | A0 拍 | A0 决策 + 实施期落地 |
| DEBT-11 | LLM 用量配额/费用统计 | M5-3/4 | 首期是否做计费/限额未定 | A0 拍 | A0 决策 |
| DEBT-12 | mcp-calls.json 500 上限是否够 | M5-2 | 高频 Agent 调用可能刷爆 | 实施期观察 | A11 观察后调整 |
| DEBT-13 | Skill 升级迁移（v1 → v2）数据保留 | M5-4 | 首期是否做兼容性迁移 | 实施期评估 | A5 W8+ 评估 |
| DEBT-14 | Agent 流式断线重连 | M5-4 | 弱网场景体验 | 实施期评估 | A5 W8+ 评估 |
| DEBT-15 | 审计汇总 UI（skill-runs.json / agent-runs.json 500 上限 UI 浏览）| M5-5 | 审计 UI 缺 | 实施期评估 | A6 W8+ 评估 |
| DEBT-16 | LLM Key 轮换策略 | M5-3/4 | 多 Key 切换 | A0 拍 | A0 决策 |
| DEBT-17 | 外部 CLI Agent（codex/claude）实测 | M5-4 | `ExternalCli` 走 script_runner 是否真能复用 | A19 实施期验证 | A19 W8+ 实测 |
| DEBT-18 | `source=ai` 智能抽取 | M5-7 | 首期仅留 trait，不实现 | A0 拍 | A0 决策 + 实施期落地 |
| DEBT-19 | 相似度算法与阈值 | M5-7 | 首期仅同 hash 精确匹配 | 实施期评估 | A7 W8+ 评估 |
| DEBT-20 | 二进制文档解析（pdf/docx）| M5-7 | 防漏洞，首期不做 | A0 拍 | A0 决策 |
| DEBT-21 | `graph_export("graphml")` | M5-8 | 首期返回"暂不支持" | 实施期评估 | A7 W8+ 评估 |
| DEBT-22 | 路径查找（KSP/最短路）| M5-8 | 首期不开 | 实施期评估 | A7 W8+ 评估 |
| DEBT-23 | `props` JSON 宽容版本 | M5-7/8 | 首期严格 schema | 实施期评估 | A7 W8+ 评估 |
| DEBT-24 | 浏览器访问历史接入 | M5-7 | 防爬虫语义，首期不接 | A0 拍 | A0 决策 |
| DEBT-25 | 大图（> 10k 节点）渲染 | M5-9 | D3 力导向性能 | 实施期评估（WebGL 化？）| A8 W8+ 评估 |
| DEBT-26 | 形态④ 第三方签名服务（如 Sigstore）| M5-10 | 首期仅本地 trusted-pubkeys.json | A0 拍 | A0 决策 |
| DEBT-27 | 插件商店 | M5-10/12 | 中心化分发 | A0 拍 | A0 决策 |
| DEBT-28 | 插件多版本并存 | M5-10 | `<id>/<version>/` 已设计；并发启用策略未拍 | A0 拍 | A0 决策 |
| DEBT-29 | 插件自动更新 | M5-10 | 首期手动 | A0 拍 | A0 决策 |
| DEBT-30 | 公钥托管（远程）| M5-10 | 首期本地 | A0 拍 | A0 决策 |
| DEBT-31 | withGlobalTauri 全局化对 src/stores/useSystemStore.ts:84 的影响 | M5-1 评估期 | 唯一引用点需迁移或封装 | M5-1.a 实施期评估 | A2 W8+ 评估 |
| DEBT-32 | tauri-browser-tabs/ 已是另一 workspace | M5-1 切片期 | 根 `Cargo.toml [workspace]` 嵌套冲突 | 阶段二（M5-1.c）排除该路径 | M5-1.c W8+ 派发 |
| DEBT-33 | D23 终端 GUI 实点 | M4-1 | M5-4/5 复跑 chat 时需在终端实点 | M5-4/5 实施期补 | A5 W8+ 实点 |
| DEBT-34 | D24 吞吐基线未重采 | M4-1 | M5-4 流式回传基准未定 | M5-13 性能基线一并采 | A11 W8+ 跑（与 DEBT-06 同）|
| DEBT-35 | D25 on_channel_dead 未 wait | M4-1 | M5-2 `mcp-server-shutdown` 必须 wait；M5-3 `a2a-shutdown` 必须 wait | M5-2/3 实施期补 | A3/A4 W7+ 补 |
| DEBT-36 | D26 历史未按字符封顶 | M4-1 | M5-13 验证矩阵反向用例补"按字符封顶" | M5-13 实施期补 | A11 W8+ 补 |
| DEBT-37 | 基线文件双份（`4f0e8ab` + `6f4e554`）| W6 测出 | pre-merge 以 `sort|head -1` 取最旧，行为正确 | A0 归档旧基线 | A0 W8+ 拍 |
| DEBT-38 | check-agent-skill-ui-logic.mjs 未接入 pre-merge | W6 测出 | 测试本身 PASS 但未接门禁 | A11 W7+ 接入 | A11 W8+ 接入 |
| DEBT-39 | agent-skill 策略缺 --expect-pending 模式 | W6 测出 | 与其它策略不一致 | A11 W8+ 补 | A11 W8+ 补 |
| DEBT-40 | MCP/agent-memory/graph --expect-pending FAIL by design | W5+W6 测出 | 翻转 PENDING→ACTIVE 模式 | 后续 wave 翻转 | A11 W8+ 翻转 |

> **W7 收口后建议下批（A0 视角）**：
> 1. **必批 W7**（最小收口）：A3 W7 + A5 W7 + A10 W7 + A11 W7 整包交付（4 lane）—— 闭环 DEBT-09 + DEBT-35。
> 2. **必批 W8**（中批收口）：A1/A2 W8（M5-1.b trait + DRY-F1 抽）+ A9 W8（M5-11 真实命令）+ A19 W8（M5-12 plugin UI）+ A11 W8（M5-13 性能基线 + D23~D26 补）—— 闭环 DEBT-03 / DEBT-04 / DEBT-05 / DEBT-06 / DEBT-07 / DEBT-08 / DEBT-33 / DEBT-34 / DEBT-36 / DEBT-38 / DEBT-39。
> 3. **可选 W9+**（决策依赖）：A0 拍 DEBT-10~DEBT-32 中待 A0 决策项 + A0 拍 DEBT-26~DEBT-30 第三方/商店/自动更新/多版本。

---

## [W7 reconciliation · 2026-09-07 09:45 CST] W7 拣入实测后债务账更新（A3 mcp_* 3 命令归位 / A11 pre-merge FAIL 3 red lights 挂账 / A1 W7 整包未进 master / DRY-F1 W7 残留扩大）

> **依据**：`6c1f30e`（A3 W7 拣入）+ `daa10f6`（A11 W7 pre-merge FAIL 拣入）+ `a29b796`（A6 W7 wiring 拣入）+ A0 W7 dispatch `a26fbaf`。
> **W7 reconciliation 债务账变化**（**A3 mcp_* 3 命令归位**）：

| # | DEBT | W7 reconciliation | 状态 | closure |
|---|------|------------------|------|---------|
| DEBT-01 | IF-2 build metrics baseline 未重采 | W7 实测 **20.63% < 21% 阈值**（A0 `5f92ece` 抬阈值 19%→21% 后）| **THRESHOLD-REVISED · W8 delta 重采** | A11 W8 delta 复跑 metrics after A5/A6/A8/A9 实施期增量 |
| DEBT-02 | A7 W6 review F1（`summarizeNode` 5 字段不在 DTO + id=sha256 不可逆）| A1 W7 [W7 patched] 段订正 8→4 字段 `{id,kind,label,neighborCount}` + W6-HS5 8→4+`props_size` + §4.2 fixture 8 字段 W8+ 取决 | **W7 F1 PATCHED** | A8 W8 UI POLISH 同步；fixture W8+ 在 graph_query 落地时同步 4 字段 |
| **DEBT-41**（**新增**）| A11 W7 pre-merge FAIL **3 red lights**（W7-1 cargo test 4 errors / W7-2 cargo fmt 8 处 / W7-3 warnings_increased 2→3）| A0 拣入 `daa10f6` 时未修 | **A0 W8 拣入期消解**（plugin.rs L274 import + L20 删 import + `cargo fmt --all`）| A0 W8 拣入期执行修复并 `cargo test` + `cargo fmt --check` 复跑 |
| **DEBT-42**（**新增**）| A1 W7 reconciliation 整包**未进 master**（M5-0/9/10/11/12/13/14 修订 + A1 W7 checkpoint + A1 W7 patch，共 9 文件 = +292 -15）| A0 W7 拣入期未消 | **A0 W8 拣入期必先消**（含 A1 W8 整包合并拣入）| A0 W8 拣入 A1 W8 整包（M5-0/9/10/11/12/13/14 修订 + A1 W8 checkpoint + A1 W8 patch）|
| **DEBT-43**（**新增**）| DRY-F1 残留扩大（W7：A4 `agent_memory.rs:134` / A7 `graph.rs:18` SENSITIVE_* 双份；`useGraphStore.ts:24-25` / `graphUi.ts:26-29` 图谱容量常量三处拷贝）| A4 W7 review 已标 DRY-F1 扩大 | **A0 W8 必批 A1/A2 抽**（前端常量统一从 `src/types.ts` 引入 + 后端 SENSITIVE_* 抽 `domain.rs` 单源）| W8 A2 review note + A2/A1 W8 抽 `domain.rs` 单源 + `useGraphStore.ts` / `graphUi.ts` import `src/types.ts` |
| **DEBT-44**（**新增**）| A3 W7 拣入**无** W7-1/W7-2/W7-3 修复责任（A3 仅管 MCP 文件，plugin.rs 属 A9）；A9 W8 POLICY REVIEW ONLY 模式**不写** plugin runtime 修复，故 W7-1/W7-3 须 A0 W8 拣入期**直接动手** | 责任缺口 | **A0 W8 拣入期必先消** | A0 W8 拣入期执行 `cargo fmt --all` + `cargo test` 复跑 + 删 plugin.rs L20 unused import |

> **W7 → W8 状态切换债务账**：

| 阶段 | 拣入 commit | DEBT 变化 |
|------|------------|---------|
| W6 收口 | `5f92ece` | 4 张子卡 CLOSED-by-W6（M5-3/4/5/6/7/8/9/10/11 = 9 张子卡） + DEBT-01 IF-2 抬阈值 + DEBT-02 F1 patched + DEBT-W6→W7 |
| W7 拣入 | `6c1f30e` + `daa10f6` + `a29b796` | **DEBT-41 W7-1/W7-2/W7-3**（A11 pre-merge FAIL）+ **DEBT-42 A1 W7 整包未进 master** + **DEBT-43 DRY-F1 扩大** + **DEBT-44 A3 W7 修复责任缺口** + A3 mcp_* 3 命令归位 FAC-2 |
| **W8 active**（**A1 W8 reconciliation 整包**）| 待 A0 拣入 | 收口 DEBT-41 / DEBT-42 / DEBT-44（**A0 拣入期必消**）；推 DEBT-43（A2 W8 review note + A1/A2 W8 抽单源）；续 A1/A2/A4/A5/A6/A7/A8/A9/A10/A11 W8 实施期债 |

> **W8 实施期新增债**（**W8 = 硬化 + 复审 + 收口波，无 MCP 产品代码工作**）：

| Lane | W8 范围 | 预计新增债 | closure |
|------|--------|-----------|---------|
| A1 | DOCS reconciliation | 无新增债（仅 reconciliation 整包）| W8 整包拣入即闭环 |
| A2 | REVIEW ONLY（非 A3 boundary）| DEBT-43 DRY-F1 抽单源（提案 + review note）| A2 W8 review note 必填 |
| A3 | **HOLD/NO ASSIGNMENT** | DEBT-45（**新增**）A3 local commit boundary 未消（A0 在 W8 dispatch 显式 *"A0 resolves the excluded A3 local commit boundary"*）| A0 在 W8 拣入期消解；A11 W8 delta 必标 A3 W8 HOLD 期间 mcp_* 相关产品代码改动数 = 0 |
| A4 | REVIEW ONLY（Agent/Skill + graph + plugin 隐私/记忆复审）| DEBT-43 DRY-F1 扩大已挂账（A4 W7 review 标） | A4 W8 review note 必填 |
| A5 | PRODUCT CODE（Agent/Skill read-only bridge 硬化）| 预计 DEBT-46（**新增**）：edge-case tests 缺口（W7 9 mcp.rs unit tests 类似深度）；validation error shape 改动 | A5 W8 实施期 0 债提交 |
| A6 | UI DOCS/LOGIC（Agent/Skill 面板消费 + 纯 UI helper tests）| 预计 DEBT-47（**新增**）：UI helper tests 缺口（与 A5 read-only bridge 协调）| A6 W8 实施期 0 债提交 |
| A7 | DOCS/GRAPH BRIDGE PLAN ONLY | 预计 DEBT-48（**新增**）：graph query command contract 提案（W8 不实施后端） | A7 W8 必填 1 graph bridge card |
| A8 | UI POLISH/TEST ONLY | 预计 DEBT-49（**新增**）：accessibility labels / empty/error/oversize states / deterministic filters / search / 无 unbounded arrays 5 项 polish | A8 W8 实施期 0 债提交 |
| A9 | POLICY REVIEW ONLY | 无新增债（仅 review note 或 policy patch）| A9 W8 0 债提交 |
| A10 | SECURITY BATCH REVIEW | 预计 DEBT-50（**新增**）：A5/A6/A8/A9 W8 输出的批量安全复审 verdict 缺口 | A10 W8 必填 1 security verdict（**不** per-file drip）|
| A11 | VERIFICATION BATCH | 预计 DEBT-51（**新增**）：W8 verification matrix（excluding A3）+ 12 FAC 实测 + 残留债 列表 | A11 W8 必填 1 final verification delta |

> **W8 必批闭环**（A0 拣入期）：

| 债 | 责任 | W8 拣入期消解路径 |
|---|------|------------------|
| DEBT-41 W7-1/W7-2/W7-3 | A0 | `cargo fmt --all` + plugin.rs L274 import + L20 删 import + `cargo test` 复跑 |
| DEBT-42 A1 W7 整包未进 master | A0 | A0 W8 拣入 A1 W8 整包（合并 A1 W7 + A1 W8 reconciliation）|
| DEBT-44 A3 W7 修复责任缺口 | A0 | 同 DEBT-41 |
| DEBT-45 A3 local commit boundary | A0 | A0 在 W8 dispatch 显式 *"resolves the excluded A3 local commit boundary"* |
| DEBT-43 DRY-F1 | A2 + A1 | A2 W8 review note + A2/A1 W8 抽 `domain.rs` 单源 + `useGraphStore.ts` / `graphUi.ts` import `src/types.ts` |
| DEBT-46/47/49/50/51 | A5/A6/A8/A10/A11 | 各 lane W8 实施期 0 债提交 / 1 final note |

> **W8 闭环条件**（**A0 推 master 前必达**）：
> 1. A0 修复 DEBT-41 / DEBT-44（cargo fmt + cargo test + L20 unused import）。
> 2. A0 拣入 A1 W8 整包（含 A1 W7 reconciliation 合并 + A1 W8 reconciliation 修订）。
> 3. A0 解决 A3 local commit boundary（DEBT-45）。
> 4. A2 W8 review note 必填（DEBT-43）。
> 5. A11 W8 delta 必填 12 FAC + 残留债 + A3 W8 HOLD 期间 mcp_* 文件改动数 = 0 + W7-1/W7-2/W7-3 修复状态确认。

---

## [W1 patched · 2026-09-06 08:50 CST] §6 反向边挂卡修正（C-7/V-7 补项）

> **修订来源**：A0 M5-W1 dispatch（`logs/checkpoints/A0-M5-W1-dispatch-20260906-0835.md`）A1 行 reconcile + A2 v3 prework（`logs/assist/A2-M5-core-20260906-0749.md` §13.2 C-7 "scheduler 反向边错挂在 M5-1.a"+ §13.4 V-7 "core 内常量影子副本"）。
> **修订原则**：A1 W1 接受 A2 v3 全部建议（C-7 + V-7 + C-5 补 `scheduler.rs:761` + `scripts.rs` 15 处测试反向 import 纳入 M5-1.b 切片 2）。修订**仅 §6 整段**与本顶部 `[W1 patched]` 段，**不重写** §1~§5/§7/§8 决策史。

### 修订影响

- §6 4 行原"处置：M5-1.a 解决"全改"**M5-1.b 解决**"（C-7）
- §6 补 3 行（W1 新增）：`scheduler.rs:761`（C-5 漏项）/ `scripts.rs` 15 处测试反向 import（测试反向）/ V-7 `database.rs` 常量副本（步骤 0b 收口）
- **关键不变量**：`M5-1.a` 只解决"测试反向 import 收口 + V-7 副本收口"，**`scheduler → bridge::AppState` 反向边 8 处 + 测试反向 2 组属 `M5-1.b`**（A1 卡 §9 与 M5-14 §6 必须严格一致）

---

## 0. 用途

把 M5 范围内**不实现**或**待 A0 决策**的项目集中成账，供：
- A0 拍板时一次性 review
- 后续 M5-W1/R 批次拣选
- 验收时核对"未实现"项是否有充分理由

---

## 1. 形态与基础设施（横切）

| 项 | 来源 | 说明 | 处置 |
|---|---|---|---|
| `withGlobalTauri` 全局暴露 | M0-0b + 详细设计 §7 | Tauri v2 不支持按 webview 关闭 `window.__TAURI__` | **形态②插件放弃**，改形态③声明式（M5-10 已决） |
| `withGlobalTauri` 全局化对 `src/stores/useSystemStore.ts:84` 的影响 | M5-1 评估期 | 唯一引用点需迁移或封装 | M5-1.a 实施期评估 |
| `tauri-browser-tabs/` 已是另一 workspace | M5-1 切片期 | 根 `Cargo.toml [workspace]` 嵌套冲突 | 阶段二（M5-1.b）排除该路径 |

---

## 2. MCP / A2A（#7）

| 项 | 来源 | 说明 | 决策依赖 |
|---|---|---|---|
| A2A 首期是否真双向 | M5-3 | 仅"被调"or"既可被调又可委派" | **A0 拍** |
| LLM 用量配额/费用统计 | M5-3/4 | 首期是否做计费/限额 | **A0 拍** |
| `mcp-calls.json` 500 上限是否够 | M5-2 | 高频 Agent 调用可能刷爆 | 实施期观察 |

---

## 3. Agent/Skill（#12）

| 项 | 来源 | 说明 | 决策依赖 |
|---|---|---|---|
| Skill 升级迁移（v1 → v2）数据保留 | M5-4 | 首期是否做兼容性迁移 | 实施期评估 |
| Agent 流式断线重连 | M5-4 | 弱网场景体验 | 实施期评估 |
| 审计汇总 UI | M5-5 | `skill-runs.json` / `agent-runs.json` 500 上限是否提供 UI 浏览 | 实施期评估 |
| LLM Key 轮换策略 | M5-3/4 | 多 Key 切换 | A0 拍 |
| 外部 CLI Agent（codex/claude）实测 | M5-4 | `ExternalCli` 走 script_runner 是否真能复用 | A19 实施期验证 |

---

## 4. Graph（#13）

| 项 | 来源 | 说明 | 决策依赖 |
|---|---|---|---|
| `source=ai` 智能抽取 | M5-7 | 首期仅留 trait，不实现 | A0 拍（成本/价值） |
| 相似度算法与阈值 | M5-7 | 首期仅同 hash 精确匹配 | 实施期评估 |
| 二进制文档解析（pdf/docx） | M5-7 | 防漏洞，首期不做 | A0 拍（需求强度） |
| `graph_export("graphml")` | M5-8 | 首期返回"暂不支持" | 实施期评估（社区需要） |
| 路径查找（KSP/最短路） | M5-8 | 首期不开 | 实施期评估 |
| `props` JSON 宽容版本 | M5-7/8 | 首期严格 schema；宽容版本做迁移期 | 实施期评估 |
| 浏览器访问历史接入 | M5-7 | 避免爬虫语义，首期不接 | A0 拍 |
| 大图（> 10k 节点）渲染 | M5-9 | D3 力导向性能 | 实施期评估（WebGL 化？） |

---

## 5. Plugin（#15）

| 项 | 来源 | 说明 | 决策依赖 |
|---|---|---|---|
| 形态① 独立 stdio | M5-10 | 与"无第二执行路径"冲突，已否决 | 决定（已） |
| 形态② 独立 webview | M5-10 | 与 `withGlobalTauri` 全局冲突，已否决 | 决定（已） |
| 形态④ 第三方签名服务（如 Sigstore） | M5-10 | 首期仅本地 trusted-pubkeys.json | A0 拍 |
| 插件商店 | M5-10/12 | 中心化分发 | A0 拍 |
| 插件多版本并存 | M5-10 | `<id>/<version>/` 已设计；并发启用策略未拍 | A0 拍 |
| 插件自动更新 | M5-10 | 首期手动 | A0 拍 |
| 公钥托管（远程） | M5-10 | 首期本地 | A0 拍 |

---

## 6. 反向边 / 架构债（沿用 A2 prework §4 + v3 §13.2 C-7 修订）

| 项 | 来源 | 说明 | 处置 |
|---|---|---|---|
| `domain.rs:1525-1526` 测试反向 import | M5-1 | 切片 0b 必收口 | M5-1.a 解决 |
| `bridge::allowed_roots(app)` | M5-1 | 4 处反向边（L26/28/259/491/588/617/622/742 scheduler.rs） | **[W1 patched] M5-1.b 解决**（抽 RootsProvider trait）—— 原挂 `M5-1.a` 错（C-7）；切片 1 才真正抽 trait |
| `tauri::Emitter` 在 script_runner 直用 | M5-1 | 切片 1 抽 ProgressSink trait | **[W1 patched] M5-1.b 解决** |
| `app.path().data_dir()` 散落 | M5-1 | 切片 1 抽 PathResolver trait | **[W1 patched] M5-1.b 解决** |
| `scheduler.rs:761` 漏反向边 | M5-1 | [W1 patched] C-5 补：同模式 `&AppState` | M5-1.b 解决 |
| `scripts.rs` 15 处测试反向 import | M5-1 | [W1 patched] 测试反向 `crate::workspace::*_at/_in` | M5-1.b 解决 |
| `database.rs:45/52` 常量影子副本 | M5-1 | [W1 patched] V-7 `DB_MAX_TEXT_FIELD_BYTES` / `DB_SOFT_TO_HARD_GRACE_SECS` 副本，值同但 `u32`/`u64` 类型不同 | M5-1.a 解决（步骤 0b 收口 + `as u64` 转换） |

---

## 7. M4 挂账跨 M5 处置（沿用 M4 终态备忘 D23~D26）

| 项 | 来自 | M5 关联 | 处置 |
|---|---|---|---|
| D23 终端 GUI 实点 | M4-1 | M5-4/5 复跑 chat 时需在终端实点 | M5-4/5 实施期补 |
| D24 吞吐基线未重采 | M4-1 | M5-4 流式回传基准未定 | M5-13 性能基线一并采 |
| D25 `on_channel_dead` 未 wait | M4-1 | M5-2 `mcp-server-shutdown` 必须 wait；M5-3 `a2a-shutdown` 必须 wait | M5-2/3 实施期补 |
| D26 历史未按字符封顶 | M4-1 | M5-13 验证矩阵反向用例补"按字符封顶" | M5-13 实施期补 |

---

## 8. A0 决策清单（一次过 review）

> 这些是 M5-W0 内需 A0 拍的关键决策，避免 M5-W1 阻塞

1. **M5-1**：A0 签发 `M5-1.a`（切片 0a+0b）还是 `M5-1.b`（含切片 1/2，B 类搬入）？
2. **M5-1**：阶段二提升为根 `Cargo.toml [workspace]` 是否本批做？
3. **M5-2**：MCP 是否真仅 stdio，不开 TCP？（默认是；与 A1 预研一致）
4. **M5-3**：A2A 首期是否真双向？
5. **M5-3**：LLM 用量配额首期是否做？
6. **M5-4**：ExternalCli 走 script_runner 是 A1 推断；A0 是否认可？
7. **M5-7**：`source=ai` 智能抽取首期是否留 trait（默认留）？
8. **M5-7**：`props` JSON 容错策略（默认严格）？
9. **M5-8**：首期只读 + 抽写 vs 支持"任意边写入"（默认前者）？
10. **M5-8**：`graph_export("graphml")` 首期是否做（默认不做）？
11. **M5-10**：形态决策 = 形态③声明式（默认）；A0 确认？
12. **M5-10**：签名方案 = Ed25519 + 本地受信任 keys（默认）；A0 确认？
13. **M5-10**：插件多版本并存并发启用策略？
14. **M5-10**：插件自动更新首期是否做（默认手动）？
15. **M5-10/15**：第三方签名服务 / 插件商店首期是否做？

---

## 9. M5-W1 签发建议（A1 视角）

> **不签字**。仅给 A0 决策时参考。

- 若 A0 接受本卡 + 12 张 M5-x 子卡 → 签发 `M5-1.a` 实施卡（A13），其余 11 张待 A1.a 完成后按 §5 合并顺序滚动签发
- 若 A0 调整形态（如改回形态①/②）→ 阻断 M5-10/11；先回 §1 重定
- 若 A0 调整 capability.rs 位置 → 阻断 M5-1/2/4/10；先回 M5-1 切片 0a 决位置
- 若 A0 引入新约束（如 LLM Key 必走远端 KMS）→ 改 M5-3/4 后再签发

---

## 10. 回写 / 维护

- 本账由 A1 创建
- 后续 M5-W1/R 批次的 A0 决策落地后，本账对应行改 `决定（已）` 或删行
- 新增债务项 = 各 M5-x 子卡 §9 DOC_BACKWRITE 的"M5-14 增项"
- 维护人：A11（沿用 A7/A11 角色）

---

---

## [W10 active · 2026-09-07 16:00 CST] W10 受控运行时预备波 — runtime surface 锁定状态（M5 final debt ledger 维持 53 条 + W10 增量 = 2）

> **W10 派发事实**（board L179 / L176-227）：W9 focused checks green（Agent/Skill 26 / MCP 9 / Plugin 10 bridge tests + Agent/Skill 99 / Graph 43 UI logic + npm build PASS + MCP/Agent policy PASS）；W10 = prepare next runtime wave **without opening unsafe execution**；Only A3 may touch MCP runtime-prep code（stdio-only, feature-gated, no listener/network, no tool execution side effects）；Plugin/Agent execution remains **LOCKED**.

> **W10 runtime surface 锁定债务标注**（承接 [W10 active] 段 runtime-lock 状态表）：

| Runtime Surface | W10 状态 | 责任 Lane / 债务项 |
|---|---|---|
| MCP server / rmcp runtime | 🔒 LOCKED（不激活 server/listener/network） | W10 Hard Stop L206；A3 W10 = 仅 stdio-prep 骨架 |
| A3 MCP stdio-prep slice（OPENED·窄） | 🟢 OPENED | A3 W10（feature-gated `mcp` Cargo feature/bin；复用 mcp.rs；无 listener/网络/rmcp tool 副作用/file/db/script/plugin 执行；rmcp/tokio optional + required-features gated） |
| Plugin install/enable/delete/download | 🔒 LOCKED | W10 Hard Stop L207；A9 W10 = PLUGIN RUNTIME PLAN ONLY（不实施 runtime）；**DEBT-04** |
| Skill/Agent execution | 🔒 LOCKED | W10 Hard Stop L207；A5 W10 = TEST/POLICY ONLY（read-only bridge 仍锁执行） |
| Model call | 🔒 LOCKED | W10 Hard Stop L207 |
| Background daemon | 🔒 LOCKED | W10 Hard Stop L207 |
| Graph live-query command | 🔒 LOCKED（BLOCKED backend runtime） | A7 W10 = GRAPH DOCS ONLY（live-query 实施卡预备） |
| Build metrics 阈值 23% | 🟢 维持 | W10 Hard Stop L210；IF-2 |
| cargo_warnings delta | 🟢 = 0 | W10 Hard Stop L210 |
| Push | 🔒 仅 A0 | W10 Hard Stop L211 |

> **W10 增量债（M5 final debt ledger 维持 53 条 + 2 新增预期）**：
> - **DEBT-04**（carried）：plugin UI runtime 仍 LOCKED；A9 W10 = PLUGIN RUNTIME PLAN ONLY 卡预备（install/enable/delete/list 命令序列 + signature failure modes + storage limits + audit redaction + UI dependencies）；A19 仍 SUPPORT DOCS ONLY；收口推 W10+.
> - **A3 W10 MCP stdio-prep**（new·窄）：feature-gated `mcp` Cargo feature/bin 或等价编译隔离骨架；registry/policy wiring 复用现有 mcp.rs；无 TCP listener / 无网络 / 无 rmcp tool 副作用 / 无 file/db/script/plugin 执行；rmcp/tokio optional + required-features gated，默认构建不变；A10 security review 必过（block on any listener/network/default-dependency pollution/side-effecting tool）.
> - **FAC-1.b M5-1.b = DEBT-03**（carried）：待 A2 v3 review note 收口（W9 未消，W10 仍挂账）.
> - **runtime 债（MCP server / plugin runtime / skill-exec）**：按设计延后到后续 runtime wave（W10 硬停止禁运行时），非 W10 阻塞.

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src/`、`src-tauri/`、`package.json`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push

---

## [W11 active · 2026-09-07 18:30 CST] A1 W11 reconciliation 立场 · runtime surface 锁定债务标注

> **依据**：`PARALLEL_COMMAND_BOARD.md` L176-228（M5-W11 MCP Stdio Dry-Run Hardening Dispatch，Added 2026-09-07 18:30 CST by A0）+ `M5-0-overview.md` 顶部 `[W11 active · 2026-09-07 18:30 CST]` 段 + A1 W11 reconciliation 整包（`logs/checkpoints/A1-M5-W11-reconciliation-20260907-1830.md`）。

**M5 final debt ledger · W11 增量预期 = 2**：

- **DEBT-04**（carried · M5-10/11/12 关联）：plugin UI runtime LOCKED；A9 W11 = PLUGIN DOCS/POLICY ONLY W12/W13 staged cards（不实施 runtime）；A19 W11 仍 SUPPORT DOCS ONLY（plugin UI 6 项待 W12+ A19 派发）。**收口推 W12+**。
- **A3 W11 bounded stdio dry-run**（new·窄）：deterministic JSON-RPC errors + bounded input/response + stable `tools/list` schema + explicit fail-closed `tools/call` for all unbound capabilities + tests/smoke for invalid JSON/unknown tool/large params；A10 security review 必过；**无** listener/network/raw-arg-echo 是 W11 红线。**预期挂账** = W11 拣入期若 A3 实施期发现 stable error code 模板不完整 / bounded size 阈值漂移 / fail-closed 不闭合，则升级为 DEBT-05 独立项；若 A3 实施期闭合，则 A11 W11 verification delta 关闭本增量预期。

**runtime 债（MCP server / plugin runtime / skill-exec）**：

| Runtime Surface | W11 状态 | 收口计划 | 挂账 ID |
|---|---|---|---|
| MCP server / rmcp runtime | 🔒 **LOCKED** | 按设计延后到后续 runtime wave（W11 硬停止禁运行时） | runtime-债（无 ID · 设计债） |
| Plugin install/enable/delete/download | 🔒 **LOCKED** | W12+ plugin staged cards | DEBT-04（plugin UI runtime） |
| Skill/Agent execution | 🔒 **LOCKED** | 按设计延后到后续 runtime wave | runtime-债（无 ID · 设计债） |
| Model call | 🔒 **LOCKED** | 按设计延后到后续 runtime wave | runtime-债（无 ID · 设计债） |
| Background daemon | 🔒 **LOCKED** | 按设计延后到后续 runtime wave | runtime-债（无 ID · 设计债） |
| Graph live-query command | 🔒 **LOCKED（BLOCKED backend runtime）** | A7 W11 = GRAPH DOCS ONLY W12 plan | DEBT-19（GRAPH_OUTPUT_NO_PROPS · W11 增量预期） |
| Build metrics 阈值 23% | 🟢 **维持** | W11 复检必跑 | IF-2（A11 重采挂账） |
| cargo_warnings delta | 🟢 **= 0** | W11 复检必跑 | （无债 · 守门项） |
| Push | 🔒 **仅 A0** | W11 拣入期 | （无债 · 守门项） |

**W11 必跑项（守门）**：

- ① A3 W11 实施期闭合增量预期 = 2（DEBT-04 carried + A3 W11 bounded stdio dry-run new·窄）= A11 W11 verification delta 必过
- ② A4 W11 privacy review PASS（W11 dry-run responses 的 URL / userinfo / query redaction 完整性 + tool result URL / capability reasons / serialized error 三处脱敏双闸）
- ③ A10 W11 security review PASS（focused on A3 stdio dry-run hardening · block on any listener/network/default-dependency pollution/side-effecting tool/raw-arg-echo）
- ④ A11 W11 verification delta 收口（default build/test + feature build/test `mcp_server` + policy scripts + UI scripts + pre-merge + build metrics ≤ 22% + warnings unchanged + W11 dry-run 行为确定性 + 残留债 + push readiness）
- ⑤ runtime surface 锁定状态表与 board L206-212 一致（A1 W11 已在本卡 + M5-0 标注 10 行锁定表）

**A1 W11 reconciliation 立场债挂账**：

- 无 A1 W11 自身债（A1 docs-only，不动产品代码）
- A1 W11 文档修订必须等到 A0 W11 拣入期合并（A1 W10 整包已 A0 拣入 → A1 W11 整包合并拣入避免 A0 分两次消）
- A3 W11 mcp_server.rs 实施债由 A10 security review 挂账（A1 静观）
- A4 W11 privacy review 关注 W11 dry-run responses 的 URL/userinfo/query redaction 完整性（若 raw params 可 echo 即 block）

---

## [W12 active · 2026-09-07 20:30 CST] A1 W12 reconciliation 立场 · runtime 债挂账

> **依据**：`PARALLEL_COMMAND_BOARD.md` L176-227（M5-W12 Graph Live-Query Readonly Dispatch）+ `M5-0-overview.md` 顶部 `[W12 active · 2026-09-07 20:30 CST]` 段 runtime-lock 状态表 11 行 + A1 W12 reconciliation 整包（`logs/checkpoints/A1-M5-W12-reconciliation-20260907-2030.md`）。

**M5 final debt ledger · W12 增量预期 = 2**：

- **A7 W12 3 只读命令**（new·窄）：bounded depth/limit + 稳定错误码 + `GraphState` 载入 + ACL/bridge/types 同包；A2/A3/A4/A10 四重 review 必过；无写/列/导出/构建/后台 worker 是 W12 红线。**预期挂账** = W12 拣入期若 A7 实施期发现 bounded 阈值漂移 / 输出去 props 不闭合 / `graph.rs` 误 import `crate::bridge`，则升级为独立 DEBT；若 A7 实施期闭合，则 A11 W12 verification delta 关闭本增量预期。
- **A8 W12 UI 消费**（new·窄）：`GRAPH_COMMANDS_AVAILABLE` 翻 `true` + AbortController/debounce + bounded rendering + 确定性 empty/error/loading；不渲染 props/secret。**预期挂账** = W12 拣入期若 A8 实施期发现 unbounded 渲染回潮 / 错误态非确定性，则升级为独立 DEBT。
- **M5-8 stale 设计债**（new·文档）：9 命令 + SQLite DDL + 5 个 `source*` 字段 vs 真实落地（3 只读 + 内存 GraphStore）—— **本包已加 `[W12 scope supersession]` 段标注**，避免后续 lane 误按 stale 设计实施。

**runtime 债（MCP server / plugin runtime / skill-exec / graph write & persistence）**：

| Runtime Surface | W12 状态 | 收口计划 | 挂账 ID |
|---|---|---|---|
| MCP server / rmcp runtime | 🔒 **LOCKED** | 按设计延后到后续 runtime wave | runtime-债（无 ID · 设计债） |
| Plugin install/enable/delete/download | 🔒 **LOCKED** | W12+ plugin staged cards | DEBT-04（plugin UI runtime） |
| Skill/Agent execution | 🔒 **LOCKED** | 按设计延后到后续 runtime wave | runtime-债（无 ID · 设计债） |
| Model call | 🔒 **LOCKED** | 按设计延后到后续 runtime wave | runtime-债（无 ID · 设计债） |
| Background daemon | 🔒 **LOCKED** | 按设计延后到后续 runtime wave | runtime-债（无 ID · 设计债） |
| Graph live-query command | 🟢 **OPENED（narrow·read-only）** | A7 W12 + A8 W12 产品代码 lane | W12 增量预期（A7/A8） |
| Graph build / index / write / export / SQLite / extractor | 🔒 **LOCKED** | 后续独立 wave | runtime-债（无 ID · 设计债） |
| Build metrics 阈值 23% | 🟢 **维持** | W12 复检必跑 | IF-2（A11 重采挂账） |
| cargo_warnings delta | 🟢 **= 0** | W12 复检必跑 | （无债 · 守门项） |
| Push | 🔒 **仅 A0** | W12 拣入期 | （无债 · 守门项） |

**W12 必跑项（守门）**：

- ① A7 W12 实施期闭合增量预期 = 2（A7 3 只读命令 + A8 UI 消费）= A11 W12 verification delta 必过
- ② A2/A3/A4/A10 W12 四重 review PASS（focused on A7 3 命令：boundary / MCP 未暴露 / privacy / security）
- ③ A11 W12 verification delta 收口（default build/test + feature build/test + policy scripts + UI scripts + pre-merge + build metrics ≤ 22% + warnings unchanged + 3 只读命令验证 + 残留债 + push readiness）
- ④ runtime surface 锁定状态表与 board L204-211 一致（A1 W12 已在本卡 + M5-0 标注 11 行锁定表）
- ⑤ M5-8 stale 设计债已 superseded，后续 lane 不得按 9 命令 SQLite 设计实施

**A1 W12 reconciliation 立场债挂账**：

- 无 A1 W12 自身债（A1 docs-only，不动产品代码）
- A1 W12 文档修订必须等到 A0 W12 拣入期合并（A1 W11 整包已 A0 拣入 → A1 W12 整包合并拣入避免 A0 分两次消）
- A7 W12 graph.rs/bridge.rs 实施债由 A2/A3/A4/A10 security review 挂账（A1 静观）
- A4 W12 privacy review 关注 3 命令输出去 props 完整性（若 props 可 echo 即 block）


## [W12 build metrics threshold · 2026-09-07 23:55 CST]

IF-2 threshold revised to 23% for W12 graph live-query readonly bridge/UI; current build metrics `total_bytes_pct=22.26`, `cargo_warnings delta=0`. W13 must preserve <=23% and avoid new cargo warnings.

---

## [W12 reconciliation · 2026-09-07 23:55 CST] W12 拣入 `3c3f460` 后债务账更新（M5 final debt ledger · 53 条 → 53 条 + W12 增量 = 2 closed）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L176-227（M5-W12 Graph Live-Query Readonly Dispatch，Added 2026-09-07 20:30 CST by A0）+ `M5-0-overview.md` 顶部 `[W12 reconciliation · 2026-09-07 23:55 CST]` 段 + A1 W12 reconciliation 整包（`logs/checkpoints/A1-M5-W12-reconciliation-20260907-2030.md`）。

**HEAD = `3c3f460 feat(M5): integrate W12 graph live-query readonly bridge/UI`**（origin/master；`git log 269269a..3c3f460` 单 commit）。

**A0 W12 验收**（`logs/checkpoints/A0-M5-W12-accept-W13-dispatch-20260907-2355.md`，STATUS=**PASS**）：

- 图谱 live-query 只读桥 + UI 消费已落地：3 只读命令 + A8 UI 消费
- 验证全绿：`cargo test graph` 15/15 + full cargo 414/414 + `cargo test --features mcp mcp_server` 21/21 + `check-graph-policy.py` ACTIVE=8 + `check-mcp-policy.py` ACTIVE=12/PENDING=0 + `check-agent-skill-policy.py` ACTIVE=3/PENDING=6 + graph UI 113/113 + agent-skill UI 110/110 + npm build PASS + build metrics 22.26% ≤ 23% + cargo_warnings delta = 0

**W12 债务账更新**：

| 类别 | W12 增量 | closed-by-W12 / carried / superseded | 备注 |
|---|---|---|---|
| **A7 W12 graph live-query 3 只读命令** | **+1 new·窄** | **closed-by-W12** | `graph_query` / `graph_node_get` / `graph_stats` + ACL 3 行 + `bridge.ts`/`types.ts` 镜像 + `GRAPH_OUTPUT_NO_PROPS` 守门码（ACTIVE=8） |
| **A8 W12 graph UI 消费** | **+1 new·窄** | **closed-by-W12** | `GRAPH_COMMANDS_AVAILABLE=true` + AbortController/debounce + bounded rendering + 确定性 empty/error/loading + graph UI logic 113/113 PASS |
| **M5-8 stale 9 命令** | superseded | **superseded-by-W12** | 旧 9 命令 + SQLite DDL 设计已 superseded；M5-8 顶部 [W12 scope supersession] 段 |
| **DEBT-04 plugin UI runtime** | 维持挂账 | **carried-to-W14+** | A19 仍 SUPPORT DOCS ONLY；plugin UI 6 项仍 0% ACTIVE = W14+ 仍挂账 |

**W12 残留债 = 0**（A7 + A8 增量 = 2 全部 closed-by-W12；M5-8 stale superseded-by-W12）；M5 final debt ledger 维持 **53 条**（W11 → W12 53 条不变）；DEBT-04 carried-to-W14+。

**A0 cleanup**（W12 拣入期处理）：① 删 stale W12 false comments；② merge 重复 TS graph view DTO block；③ 删 unused `bounded_subgraph` helper（warning budget 回退到 grid_process 既有 warnings only）。

---

## [W13 active · 2026-09-07 23:55 CST] A1 W13 reconciliation 立场 · runtime 债挂账

> **依据**：`PARALLEL_COMMAND_BOARD.md` L228+（M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch，Added 2026-09-07 23:55 CST by A0）+ `M5-0-overview.md` 顶部 `[W13 active · 2026-09-07 23:55 CST]` 段 + A1 W13 reconciliation 整包（`logs/checkpoints/A1-M5-W13-reconciliation-20260907-2358.md`）。

**W13 = 仅 A9 可写产品代码**（A9 plugin stage-I manifest 生命周期 6 命令 + 本地状态机）；其余 10 lane 全 review / docs / verification。

**W13 增量预期 = 1 + DEBT-04 carried**：

| 类别 | W13 增量 | 关闭路径 | 备注 |
|---|---|---|---|
| **A9 W13 plugin stage-I 6 命令** | **+1 new·窄** | **closed-by-W13** | `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry` 6 命令 + 6 ACL 行 + `bridge.ts`/`types.ts` 镜像 + `PLUGIN_LIFECYCLE_LOCAL_ONLY` 守门码（ACTIVE 6 → 7）；本地 manifest 解析 + 静态资源元数据装载 + 本地状态机；**不**实施 invoke / cancel / storage_* 5 stub |
| **DEBT-04 plugin UI runtime（6 项）** | 维持挂账 | **carried-to-W14+** | A19 仍 SUPPORT DOCS ONLY；plugin UI 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览 6 项仍 0% ACTIVE = W14+ 仍挂账；A0 W12/W13 期间未派发 A19 plugin UI 产品代码 |

**W13 必保红线条目**（不增债 / 不破既有债）：

- K1 ACL 末条恒为 `list_artifact_images` —— A9 W13 新增 6 行插末条前，不破 K1
- `plugin.rs` 不 import `crate::bridge`（mirror `GRAPH_NO_SECOND_PATH` 守门）
- `plugin.rs` 不引入 `tokio` / `reqwest` / `ureq` / `std::process::Command::new` / `std::net` / 后台 worker
- `check-plugin-policy.py --self-test` PASS + W6 5 stub 仍 `Err("not-implemented-in-W6")` 不被 A9 W13 触碰
- 5 stub `plugin_invoke/cancel/permissions_get/audit_list/storage_*` stage-I **不**实现（runtime execution 仍 LOCKED）
- A7 W13 GRAPH NON-REGRESSION DOCS ONLY 必保 W12 graph 集成锚点不回归（`mod graph`@`main.rs:9` + `GraphState` 托管@`main.rs:1347` + 快照载入@`main.rs:1328-1335` + 3 graph 命令注册@`main.rs:1474-1476` + `bridge.rs:6605/6624/6641` + `domain.rs` `GraphNode*`/`GraphEdge*`/`GraphProps`/View@L2042-2115 + 7 容量常量@`domain.rs` L2188-2200 + ACL 3 行@`default-commands.toml` L127-129，末条 `list_artifact_images`@L130）
- build metrics ≤ 23% 维持（IF-2 W12 A0 修订）
- cargo_warnings delta = 0
- 仅 A0 push

**W13 残留债预期 = 0**（A9 W13 增量 = 1 closed-by-W13；M5 final debt ledger 维持 **53 条** + W13 增量 1 closed-by-W13 → 维持 53 条）；**DEBT-04 plugin UI 6 项 carried-to-W14+**（A19 派发后消解）；runtime 债（plugin invoke / cancel / storage_* 5 stub）按设计延后，W13 不实现。

---

## [W14 active · 2026-09-08 00:20 CST] A1 W14 reconciliation 立场 · runtime surface 锁定状态表（14 行）+ DEBT-04 仍挂账转 A6 W14

**W14 派发** = Plugin Manager UI Dispatch（board L1163+）；**W14 = 仅 1 lane UI 产品代码 open**（**A6 W14 = START PRODUCT CODE NARROW**），其余 10 lane = docs/review/security/verification（详见 `M5-0-overview.md` 顶部 `[W14 active · 2026-09-08 00:20 CST]` 段 + `PARALLEL_COMMAND_BOARD.md` L1163-1189 lane 表）。

**W14 runtime surface 锁定状态表（14 行 · 沿用 W13 13 行 + 新增 W14 plugin UI 1 行）**：

| 维度 | W13 status | W14 status | 备注 |
|------|-----------|-----------|------|
| 1. `plugin_invoke` / `plugin_cancel` / `plugin_storage_*` 5 stub | 🔒 LOCKED | 🔒 LOCKED | 5 stub 错误结构不变 `Err("not-implemented-in-W6")`；W14 不实现 invoke / cancel / storage_* runtime；plugin UI 不触发 |
| 2. `plugin_install` / `plugin_enable` / `plugin_disable` stage-I lifecycle | 🟢 OPENED（narrow·local-only） | 🟢 OPENED（narrow·local-only） | A9 W13 已落地 6 命令；W14 UI 消费，不重写 |
| 3. `plugin_list` / `plugin_get` / `plugin_key_registry` | 🟢 OPENED（read-only） | 🟢 OPENED（read-only） | A9 W13 已落地；W14 UI 消费 |
| 4. `plugin_audit_get` | 🟢 OPENED（read-only） | 🟢 OPENED（read-only） | A9 W13 已落地；W14 UI 可选消费 |
| 5. network download / listener | 🔒 LOCKED | 🔒 LOCKED | W14 plugin UI 不引入网络下载；manifest 来源 = 后端命令输出（不重新下载） |
| 6. dynamic code loading | 🔒 LOCKED | 🔒 LOCKED | W14 plugin UI 不引入动态加载 |
| 7. daemon / model call | 🔒 LOCKED | 🔒 LOCKED | W14 plugin UI 不引入 daemon / 模型调用 |
| 8. Agent/Skill execution | 🔒 LOCKED | 🔒 LOCKED | W14 plugin UI 不触发 Agent/Skill runtime |
| 9. MCP full runtime | 🔒 LOCKED | 🔒 LOCKED | W14 plugin UI 不引入 mcp_* 边界 |
| 10. graph build-write-export | 🔒 LOCKED | 🔒 LOCKED | W14 plugin UI 不引入 graph 写导出 |
| 11. background worker | 🔒 LOCKED | 🔒 LOCKED | W14 plugin UI 不引入后台 worker |
| 12. plugin UI 6 action（list / filter / inspect / install-supplied-manifest / enable / disable / key fingerprint） | 🔒 LOCKED（A19 SUPPORT） | 🟢 OPENED（A6 START PRODUCT CODE NARROW） | W14 唯一产品代码 lane open；A6 W14 实施期完成 6 action |
| 13. raw Tauri `invoke`（绕过 bridge.ts） | 🔒 LOCKED | 🔒 LOCKED | W14 plugin UI **必须**走 `src/bridge.ts`；`check-plugin-ui-logic.mjs` 新增 `PLUGIN_UI_NO_RAW_INVOKE` / `PLUGIN_UI_BRIDGE_ONLY` 守门码 |
| 14. raw signature / public-key / resource path / manifest metadata / credentials / 请求响应 body / stdout / stderr 渲染 | 🔒 LOCKED | 🔒 LOCKED | W14 plugin UI **不**渲染 raw 敏感字段；`check-plugin-ui-logic.mjs` 新增 6 守门码（`PLUGIN_UI_NO_RAW_SIG_RENDER` / `PLUGIN_UI_NO_RAW_PUBKEY_RENDER` / `PLUGIN_UI_NO_RAW_RESOURCE_PATH_RENDER` / `PLUGIN_UI_NO_RAW_MANIFEST_META_RENDER` / `PLUGIN_UI_NO_RAW_CREDENTIAL_RENDER` / `PLUGIN_UI_NO_RAW_REQ_RES_BODY_RENDER` / `PLUGIN_UI_NO_RAW_STDOUT_STDERR_RENDER`） |

**W14 债务账变化（相对 W13 拣入后）**：
- **W14 增量预期 = 0**（A6 W14 = 复吃 W13 6 命令 + UI 包装，**不**实施 W14 新 lifecycle 命令 / **不**改 DTO / **不**渲染 raw 敏感字段 → 无新债增量）
- **M5 final debt ledger 维持 53 条**（W14 拣入后维持 53 条 + A6 W14 实施期收口 DEBT-04 = 6/6 ACTIVE）
- **DEBT-04 plugin UI 6 项 ACTIVE = 0% → 6/6**（A6 W14 实施期收口；**A1 W14 派发期仍 0%** = 派发期不算 ACTIVE）
- **新增债 0**（W14 不实施 invoke / cancel / storage_* / 网络 / 动态加载 / daemon / 模型 / Agent-Skill / MCP / graph 写 / 后台 worker）

**A1 W14 派发期立场 = START DOCS ONLY**：
- reconcile W13 as accepted after A0 push（`a7eefbb`）
- mark W14 active + 更新 3 主文档 L1 + 本卡头 + 末尾段 + `M5-12-plugin-ui.md` 末尾 [W14 active] 段 + `M5-13-verification-matrix.md` 末尾 [W14 verification scope] 段 + `M5-0-overview.md` 末尾 [W14 active] 段
- **不**改 product scope（plugin UI 6 项 ACTIVE 仍 0%，A6 W14 实施期才升 ACTIVE）
- **不**碰产品代码（`src/components/plugin/**` / `usePluginStore.ts` / `check-plugin-ui-logic.mjs` / `bridge.ts` / `types.ts`）
- **不** push

**A6 W14 实施期收口**：A6 W14 实施完成 = 6 action 全 ACTIVE + `check-plugin-ui-logic.mjs` ≥9/9 PASS + A11 W14 verification delta 功能性 ALL_PASS + A7 W14 graph non-regression PASS + A10 W14 security review PASS → DEBT-04 closed-by-W14，M5 final debt ledger 维持 53 条（plugin UI 6 项由"挂账"转"收口"）。
