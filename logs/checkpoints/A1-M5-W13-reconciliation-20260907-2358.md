# A1 · M5-W13 Reconciliation（START DOCS ONLY · 2026-09-07 23:55 CST）

> Lane: **A1** (M5 横切 · docs only)
> Repo: /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
> Branch: master
> Wave: M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch
> 派发: `PARALLEL_COMMAND_BOARD.md` L228+（A0 2026-09-07 23:55 CST 派发）
> 任务: **START DOCS ONLY** · reconcile W12 as accepted after A0 push + mark W13 active + plugin 卡范围收敛为 stage-I manifest 生命周期

---

## 0. Wave 派发原文

> **Current NEXT: M5-W13 plugin runtime Stage-I manifest lifecycle**; **W12 is accepted locally and pending A0 commit/push in this batch**; W13 opens only A9 plugin manifest lifecycle product code (install from local manifest/resource metadata, enable/disable/list/get/key registry as safe local state), while plugin invoke/command execution, network download/listener, dynamic code execution, MCP full runtime, Agent/Skill execution, daemon, and model call remain LOCKED; only A0 pushes
>
> — `PARALLEL_COMMAND_BOARD.md` L228+ · A0 added 2026-09-07 23:55 CST

**W12 在本 wave 已被 A0 拣入**（`3c3f460`）= HEAD（origin/master）；W12 = PASS（`logs/checkpoints/A0-M5-W12-accept-W13-dispatch-20260907-2355.md`），W13 = W12 拣入后 A0 即时派发。

---

## 1. 任务范围

| Lane | W13 任务 | 责任 |
|---|---|---|
| **A1（本卡）** | **START DOCS ONLY** | reconcile W12 PUSHED + mark W13 active + plugin 卡范围收敛 + A7 W13 GRAPH NON-REGRESSION 守护范围 + A19 plugin UI 仍 SUPPORT DOCS ONLY 标注 |
| A2 | START BOUNDARY REVIEW ONLY | 审 W13 plugin `plugin.rs` 不 import `crate::bridge` + 不引 `tokio`/`reqwest`/`ureq`/`std::process::Command::new`/`std::net`/后台 worker |
| A3 | START MCP REVIEW ONLY | 审 W13 plugin 6 命令**未**经 MCP stdio 暴露为可执行工具；MCP 仍 dry-run/read-only introspection |
| A4 | START PRIVACY REVIEW ONLY | 审 W13 plugin 输出 `PluginManifestView`/`PluginStateView`/`PluginKeyEntryView` 不含 secret/raw signature/stdout/stderr/路径含环境变量；错误稳定码零 secret echo |
| A5 | START AGENT/SKILL REVIEW ONLY | 确认 Agent/Skill 执行仍锁（PENDING=6）、plugin stage-I 不触 agent/skill runtime |
| A6 | START UI REVIEW ONLY | 审 workspace UI 在 W13 plugin 变更后 Agent/Skill 面板执行控件仍 disabled + deterministic；plugin UI 6 项**仍 LOCKED**（A19 未派发） |
| A7 | **START GRAPH NON-REGRESSION DOCS ONLY** | 必保 W12 graph 集成锚点不回归（8 锚点 = `mod graph`@`main.rs:9` + `GraphState`@`main.rs:1347` + 快照载入@`main.rs:1328-1335` + 3 graph 命令注册@`main.rs:1474-1476` + `bridge.rs:6605/6624/6641` + `domain.rs` `GraphNode*`/`GraphEdge*`/`GraphProps`/View@L2042-2115 + 7 容量常量@`domain.rs` L2188-2200 + ACL 3 行@`default-commands.toml` L127-129 末条 `list_artifact_images`@L130） |
| A8 | START GRAPH UI REVIEW ONLY | 审 workspace UI 在 W13 plugin 变更后图谱面板仍正确消费 3 只读命令 + AbortController/debounce + 确定性 empty/error/loading；不破 W12 graph UI 113/113 断言 |
| **A9** | **START PRODUCT CODE NARROW** | 实施 stage-I 6 命令：`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry` + 本地 manifest 解析 + 静态资源元数据装载 + 本地状态机 + ACL 6 行插末条 `list_artifact_images` 前 + `bridge.ts` / `types.ts` 镜像 + `check-plugin-policy.py` 新增 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 守门码（ACTIVE 6 → 7）|
| A10 | START SECURITY REVIEW | 批量审 W13 plugin bridge/UI；block on invoke / cancel / storage_* 5 stub 任何 runtime execution 路径落地 + secret/raw signature 暴露 + 动态加载 / 网络 / 监听 / 后台 worker / build metric 回归 |
| A11 | START VERIFICATION | W13 验证矩阵：cargo test plugin + 整包 cargo test + `check-plugin-policy.py` --self-test/default（ACTIVE 7）+ graph policy / MCP policy / Agent-Skill lock / graph UI / agent-skill UI 仍 PASS + npm build + pre-merge + build metrics ≤ 23% + warnings 不变 |
| **A19** | **SUPPORT DOCS ONLY** | plugin UI 6 项 = 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览，**全部依赖** W13 stage-I 真实后端 6 命令落地；A0 W12/W13 期间未派发 A19 plugin UI 产品代码；**plugin UI 6 项 0% ACTIVE = DEBT-04 W14+ 仍挂账** |

**W13 = 仅 A9 可写产品代码**（A9 plugin stage-I 6 命令 + 本地状态机）；其余 10 lane 全部 review / docs / verification。

---

## 2. 工作树起点（reconcile W12 as accepted after A0 push）

### 2.1 HEAD 与 W12 拣入事实

**HEAD = `3c3f460 feat(M5): integrate W12 graph live-query readonly bridge/UI`**（origin/master；`git log 269269a..3c3f460` 单 commit）。

**A0 W12 验收**（`logs/checkpoints/A0-M5-W12-accept-W13-dispatch-20260907-2355.md`，STATUS=**PASS**）：

- 图谱 live-query 只读桥 + UI 消费已落地：3 只读命令 `graph_query` / `graph_node_get` / `graph_stats` + A8 UI 消费（`GRAPH_COMMANDS_AVAILABLE=true` + AbortController/debounce + bounded rendering + 确定性 empty/error/loading + 无 props/secret 渲染）
- `check-graph-policy.py` **ACTIVE=8 PASS**（含 `GRAPH_OUTPUT_NO_PROPS` 守门码）
- 验证全绿：`cargo test graph` 15/15 + full cargo 414/414 + `cargo test --features mcp mcp_server` 21/21 + `check-mcp-policy.py` ACTIVE=12/PENDING=0 + `check-agent-skill-policy.py` ACTIVE=3/PENDING=6 + graph UI logic 113/113 + agent-skill UI logic 110/110 + npm build PASS + build metrics `total_bytes_pct=22.26` ≤ **23%**（A0 W12 IF-2 阈值 22% → 23% 修订）+ cargo_warnings delta = 0
- A0 cleanup：删 stale W12 false comments / merge 重复 TS graph view DTO block / 删 unused `bounded_subgraph` helper（warning budget 回退到 grid_process 既有 warnings only）

### 2.2 工作树 W12 后起点 = 干净

A0 W12 拣入 = `3c3f460` 单 commit；A1 W12 reconciliation 整包（`logs/checkpoints/A1-M5-W12-reconciliation-20260907-2030.md` 203 行 + `logs/checkpoints/Lane-A1-M5-W12-reconciliation-20260907-2030.patch` 14 文件）已合并拣入 `3c3f460` git stat；M5-0/7/8/9/10/11/12/13/14 头部 W11 PUSHED + W12 ACTIVE 状态行 + 末尾 [W12 scope supersession]/[W12 verification scope]/[W12 active] 段 + 3 主文档 L1 W11 update 行 + board L6/L7 W12 dispatch 均已合并入 master。

**A1 W13 工作树起点** = 干净 master（`3c3f460`）+ 9 lane 在 W13 实施期未提交改动（**A1 不依赖其他 lane 的实施产物**；A1 W13 仅消费 A0 拣入 W12 的事实回填 + A0 W13 派发指令 + A0 验收 W12 后的 `3c3f460` commit；A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 各自的 W13 assist / checkpoint 落地与否不影响 A1 文档对账）。

### 2.3 A1 W12 reconciliation 整包已被 A0 合并拣入

`3c3f460` git stat 含 `A1-M5-W12-reconciliation-20260907-2030.md` 203 行 + `Lane-A1-M5-W12-reconciliation-20260907-2030.patch` 14 文件 = + M5-0/7/8/9/10/11/12/13/14 头部 W11 PUSHED + W12 ACTIVE 状态行 + M5-0/8/13/14 末尾 [W11 reconciliation]/[W12 scope supersession]/[W12 verification scope]/[W12 active] 段 + 3 主文档 L1 W11 update 行 + board L6/L7 W12 dispatch。

---

## 3. W12 runtime surface 锁定状态表（A1 在 M5-0/13/14 标注）

| Runtime Surface | W11 → W12 状态 | W12 拣入后状态 |
|---|---|---|
| **Graph live-query command（3 只读命令）** | 🟢 **OPENED**（W12, narrow, **read-only**） | 🟢 **维持**（W12 graph 集成锚点由 A7 W13 GRAPH NON-REGRESSION DOCS ONLY 守护） |
| Graph build / index / write / export | 🔒 **LOCKED** | 🔒 **维持 LOCKED**（W13 Hard Stop） |
| Graph UI consumption | 🟢 **OPENED** | 🟢 **维持**（A8 W13 GRAPH UI REVIEW ONLY 守护） |
| **Plugin stage-I manifest lifecycle command（6 命令）** | 🔒 **LOCKED** | 🟢 **OPENED**（W13, narrow, **stage-I local-only**）：`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry`；**无** invoke / cancel / storage_* / 网络下载 / 监听 / 动态加载 |
| Plugin invoke / command execution / dynamic code loading | 🔒 **LOCKED** | 🔒 **维持 LOCKED**（W13 Hard Stop；5 stub `plugin_invoke/cancel/permissions_get/audit_list/storage_*` 仍 `Err("not-implemented-in-W6")`） |
| Plugin UI（列表 / 安装向导 / 启用停用 / 审计查询 / 权限预览） | 🔒 **LOCKED** | 🔒 **维持 LOCKED**（A19 仍 SUPPORT DOCS ONLY；DEBT-04 W14+ 仍挂账） |
| MCP full runtime / rmcp server | 🔒 **LOCKED** | 🔒 **维持 LOCKED**（W13 Hard Stop） |
| Skill/Agent execution | 🔒 **LOCKED** | 🔒 **维持 LOCKED**（W13 Hard Stop；PENDING=6 执行锁） |
| Model call | 🔒 **LOCKED** | 🔒 **维持 LOCKED**（W13 Hard Stop） |
| Background daemon / network listener | 🔒 **LOCKED** | 🔒 **维持 LOCKED**（W13 Hard Stop） |
| DB / script / hidden execution path | 🔒 **LOCKED** | 🔒 **维持 LOCKED**（W13 Hard Stop） |
| Build metrics 阈值 23% | 🟢 **维持**（W12 A0 IF-2 修订 22% → 23%） | 🟢 **维持**（W13 Hard Stop） |
| cargo_warnings delta | 🟢 **= 0** | 🟢 **= 0**（W13 Hard Stop） |
| Push | 🔒 **仅 A0** | 🔒 **仅 A0**（W13 Hard Stop） |

**W12 graph 范围冻结 = 恰好 3 条只读命令 + UI 消费**（详见 `M5-0-overview.md` 顶部 `[W12 active]` 段 + `M5-8-graph-store-query.md` `[W12 scope supersession]` 段）。

**W13 plugin 范围冻结 = 恰好 6 条 stage-I manifest 生命周期命令**（详见本卡 + `M5-0-overview.md` 顶部 `[W13 active]` 段 + `M5-10-plugin-manifest-lifecycle.md` `[W13 stage-I scope]` 段 + `M5-11-plugin-commands-isolation.md` `[W13 stage-I isolation]` 段 + `M5-12-plugin-ui.md` `[W13 plugin UI still locked]` 段）。

---

## 4. W13 硬停止遵守记录（board L228+ Hard Stops）

- ① ✅ W13 plugin 范围**stage-I 6 命令 + 本地状态机**：`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry`；**无** invoke / cancel / storage_* 5 stub 任何 runtime execution 路径落地（5 stub 维持 `Err("not-implemented-in-W6")`）
- ② ✅ plugin 输出 DTO **脱敏**：`PluginManifestView` / `PluginStateView` / `PluginKeyEntryView` 不含 secret / raw signature / stdout / stderr / 路径含环境变量；`PluginError::code()` 稳定码不 echo secret / 路径 / URL / token / cookie / Authorization
- ③ ✅ 6 命令必须同包过 `check_invocation_source` + ACL（6 行插末条 `list_artifact_images` 前）+ `main.rs` handler + `bridge.ts` + `types.ts` + `check-plugin-policy.py`（含 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 守门码）+ tests
- ④ ✅ MCP full runtime / plugin invoke·command execution·cancel·storage_* / Agent/Skill execution / network listener / daemon / model call / hidden script·db execution / graph build-write-export **全部 LOCKED**
- ⑤ ✅ build metrics 阈值 **23%** 维持（W12 A0 IF-2 修订）；cargo warnings 不增加
- ⑥ ✅ 仅 A0 push（本卡不 push，工作树留待 A0 拣入）

---

## 5. A7 W13 GRAPH NON-REGRESSION 守护（必保 W12 graph 集成锚点不回归）

W13 A9 plugin 实施只触碰共享文件（`src-tauri/src/domain.rs` / `src-tauri/src/bridge.rs` / `src-tauri/src/main.rs` / `src-tauri/permissions/default-commands.toml` / `src/types.ts` / `src/bridge.ts`）；A7 W13 必保 8 锚点完整：

| 锚点 | 位置 | A7 必保 |
|---|---|---|
| 1 | `src-tauri/src/main.rs:9` `mod graph;` | `mod graph;` 仍存在（plugin domain 实施不删） |
| 2 | `src-tauri/src/main.rs:1347` `GraphState` 托管 state | 仍托管 state，**不**被 plugin state 覆盖 |
| 3 | `src-tauri/src/main.rs:1328-1335` graph store 快照载入 | 仍快照载入既有 store |
| 4 | `src-tauri/src/main.rs:1474-1476` 3 graph 命令注册 | 仍注册 `graph_query` / `graph_node_get` / `graph_stats` |
| 5 | `src-tauri/src/bridge.rs:6605/6624/6641` 3 graph invoke handler | 仍存在；**不**被 plugin handler 覆盖；ACL `check_invocation_source` 仍过 |
| 6 | `src-tauri/src/domain.rs` L2042-2115 `GraphNode*` / `GraphEdge*` / `GraphProps` / `GraphNodeView` / `GraphEdgeView` | DTO 完整；plugin domain 追加类型**不**重名 / **不**改 graph 类型签名 |
| 7 | `src-tauri/src/domain.rs` L2188-2200 7 graph 容量常量（`GRAPH_PROPS_MAX_BYTES=MAX_TEXT_FIELD_BYTES=64KiB` 必须等于 / `GRAPH_LABEL_MAX_BYTES=256` / `GRAPH_NODE_ID_HEX_LEN=64` / `GRAPH_MAX_DEPTH=4` / `GRAPH_QUERY_LIMIT=1000` / `GRAPH_MAX_NODES=5000` / `GRAPH_MAX_EDGES=20000`） | 常量值不变；plugin 实施**不**改 graph 常量 |
| 8 | `src-tauri/permissions/default-commands.toml` L127-129 ACL 3 graph 行（末条 `list_artifact_images`@L130） | 3 graph ACL 行仍存在，**不**被 plugin ACL 6 行覆盖；K1 ACL 末条恒为 `list_artifact_images` |

A7 W13 必跑 GRAPH NON-REGRESSION 检查（实施期 `logs/assist/A7-M5-W13-graph-nonregression-20260907-2358.md` 落地 + patch 仅含本说明）：8 锚点任一漂移即 block W13 plugin 实施；A7 W13 patch `Lane-A7-M5-W13-graph-nonregression-20260907-2358.patch` 已被 A0 拣入。

---

## 6. W13 十一 lane 角色（board L228+）

| Lane | Status | W13 任务要点 |
|---|---|---|
| **A1**（本卡） | START DOCS ONLY | reconcile W12 PUSHED + mark W13 active + plugin 卡范围收敛为 stage-I 6 命令 + A7 GRAPH NON-REGRESSION 守护范围 + A19 plugin UI 仍 SUPPORT DOCS ONLY |
| A2 | START BOUNDARY REVIEW ONLY | 审 W13 plugin `plugin.rs` 不 import `crate::bridge` + 不引 `tokio`/`reqwest`/`ureq`/`std::process::Command::new`/`std::net`/后台 worker；plugin.rs 维持 5 stub `Err("not-implemented-in-W6")` |
| A3 | START MCP REVIEW ONLY | 审 W13 plugin 6 命令**未**经 MCP stdio 暴露为可执行工具；MCP 仍 dry-run/read-only introspection |
| A4 | START PRIVACY REVIEW ONLY | 审 W13 plugin 输出 `PluginManifestView`/`PluginStateView`/`PluginKeyEntryView` 不含 secret/raw signature/stdout/stderr/路径含环境变量；错误稳定码零 secret echo |
| A5 | START AGENT/SKILL REVIEW ONLY | 确认 Agent/Skill 执行仍锁（PENDING=6）、plugin stage-I 不触 agent/skill runtime |
| A6 | START UI REVIEW ONLY | 审 workspace UI 在 W13 plugin 变更后 Agent/Skill 面板执行控件仍 disabled + deterministic；plugin UI 6 项**仍 LOCKED**（A19 未派发） |
| **A7** | **START GRAPH NON-REGRESSION DOCS ONLY** | 必保 W12 graph 集成锚点不回归（8 锚点清单）|
| A8 | START GRAPH UI REVIEW ONLY | 审 workspace UI 在 W13 plugin 变更后图谱面板仍正确消费 3 只读命令 + AbortController/debounce + 确定性 empty/error/loading；不破 W12 graph UI 113/113 断言 |
| **A9** | **START PRODUCT CODE NARROW** | 实施 6 命令 + 本地 manifest 解析 + 静态资源元数据装载 + 本地状态机 + ACL 6 行插末条 `list_artifact_images` 前 + `bridge.ts` / `types.ts` 镜像 + `check-plugin-policy.py` 新增 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 守门码（ACTIVE 6 → 7）|
| A10 | START SECURITY REVIEW | 批量审 W13 plugin bridge/UI；block on invoke / cancel / storage_* 5 stub 任何 runtime execution 路径落地 + secret/raw signature 暴露 + 动态加载 / 网络 / 监听 / 后台 worker / build metric 回归 |
| A11 | START VERIFICATION | W13 验证矩阵 |
| **A19** | **SUPPORT DOCS ONLY** | plugin UI 6 项**仍 LOCKED**（A0 W12/W13 期间未派发 A19 plugin UI 产品代码） |

**W13 = 仅 A9 可写产品代码**（A9 plugin stage-I manifest 生命周期 6 命令 + 本地状态机）；其余 10 lane 全 review / docs / verification。

---

## 7. W13 验证矩阵（A1 在 M5-13 `[W13 verification scope]` 段标注）

| FAC | 子卡 | W13 AC | 状态 | 验证命令 / 文件 | 挂账 / 备注 |
|-----|------|--------|------|----------------|------------|
| **FAC-15.W13 (new·核心)** | M5-10/11/12 plugin stage-I 6 命令 | `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry` 6 命令在 `bridge.rs` 过 `check_invocation_source` + 本地 manifest 解析 + 静态资源元数据装载 + install/enable/disable/list/get/key registry 安全本地状态机 + 6 命令 ACL 插末条 `list_artifact_images` 前 + `bridge.ts` / `types.ts` 镜像 + `check-plugin-policy.py` 新增 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 守门码（ACTIVE 6 → 7） | **ACTIVE · A9 W13 实施** | `cargo test plugin`（W6 10/10 基线 + W13 增量 6 命令）PASS；`check-plugin-policy.py --self-test/default` PASS（W6 ACTIVE=6 + W13 新增 `PLUGIN_LIFECYCLE_LOCAL_ONLY`） | **W13 唯一后端产品代码 lane**；A2 boundary + A3 MCP + A4 privacy + A10 security 四重 review 必过；`plugin_invoke` / `plugin_cancel` / `plugin_storage_*` 5 stub 仍维持 `Err("not-implemented-in-W6")` |
| **FAC-15.W13.UI** | M5-12 plugin UI 6 项 | plugin 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览 6 项 | 🔒 **LOCKED** | —— | **A19 仍 SUPPORT DOCS ONLY**；plugin UI 6 项 0% ACTIVE = DEBT-04 W14+ 仍挂账；**A0 W12/W13 期间未派发 A19 plugin UI 产品代码** |
| **FAC-15.W13.InvokeLock** | M5-10/11 plugin invoke / execution | invoke / cancel / 动态加载 / 网络下载 / 监听 / daemon / model call / Agent-Skill 执行 / MCP full runtime / graph build-write-export 全部 LOCKED | 🔒 **LOCKED** | `check-plugin-policy.py --self-test` PASS + W6 5 stub 仍 `Err("not-implemented-in-W6")` 不被 A9 W13 触碰 | **W13 红线**；A9 W13 仅实施 6 stage-I 命令，**不**改 invoke / cancel / storage_* 5 stub |
| **FAC-7/8.W12 (regression)** | M5-7/8/9 graph 3 只读命令 | W12 graph 集成锚点完整（8 锚点） | 🟢 **维持** | `cargo test graph` 15/15 + `check-graph-policy.py` ACTIVE=8 | **A7 W13 GRAPH NON-REGRESSION DOCS ONLY 守护**；A9 W13 实施触碰共享文件须保留 8 锚点 |

**W13 硬停验证必跑**（8 条）：

① **plugin stage-I 范围**：6 命令在 `bridge.rs` 过 `check_invocation_source` + 6 命令 ACL 插末条 `list_artifact_images` 前 + `bridge.ts` / `types.ts` 镜像 + `main.rs` handler 注册；**无** invoke / cancel / storage_* 5 stub 任何 runtime execution 路径落地
② **本地状态机**：install / enable / disable / list / get / key registry 全部基于本地 `plugin.rs` 既有 7 状态机 + 12 合法边；**不**引入网络下载 / 监听 / 动态加载 / 远程 manifest 拉取
③ **输出脱敏**：`PluginManifestView` / `PluginStateView` / `PluginKeyEntryView` 不含 secret / raw signature / stdout / stderr / 路径含环境变量；`PluginError::code()` 稳定码 + 错误体不 echo secret / 路径 / URL / token / cookie / Authorization
④ **同包五同步**：6 命令 + ACL（6 行插末条 `list_artifact_images` 前）+ `main.rs` + `bridge.ts` + `types.ts` + `check-plugin-policy.py`（含 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 守门码）+ tests 同包
⑤ **`plugin.rs` 不引入** `crate::bridge`（mirror `GRAPH_NO_SECOND_PATH` 守门）+ `plugin.rs` 不引入 `tokio` / `reqwest` / `ureq` / `std::process::Command::new` / `std::net` / 后台 worker
⑥ **MCP 未暴露**：`check-mcp-policy.py` 仍 PASS + A3 W13 review note（plugin 6 命令**不**经 MCP stdio 暴露为可执行工具）+ `cargo test --features mcp mcp_server` 仍 21/21 PASS
⑦ **build metrics ≤ 23%**（A0 W12 IF-2 修订阈值）+ `cargo_warnings delta = 0`
⑧ **仅 A0 push**（A1 W13 整包不 push，工作树留待 A0 拣入）

---

## 8. A1 W13 整包交付物

### 8.1 整包改动文件清单（14 文件 = 与 W12 patch 同形 14）

| # | 文件 | 改动内容 |
|---|---|---|
| 1 | `PARALLEL_COMMAND_BOARD.md` L6 | mainline 链加 W12 拣入 `3c3f460` = HEAD；4-commit chain (W10 `ba78092`/`5226aad` + W11 `269269a` + W12 `3c3f460`)；W13 dispatch 已存在 L7 |
| 2 | `详细设计与实施计划.md` L1 | 加 A1 W13 update 行 |
| 3 | `AI-模型切换与接手清单.md` L1 | 加 A1 W13 update 行 |
| 4 | `后续需求TODO.md` L1 | 加 A1 W13 update 行 |
| 5 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | L1 标题 +W12 reconciliation +W12 PUSHED +W13 active + 顶部 L24 history 链加 W12 拣入 + W13 active + 末尾 [W12 reconciliation] + [W13 active] 段 |
| 6 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | 头部 status 加 W12 PUSHED + W13 ACTIVE 2 行 |
| 7 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | 头部 status 加 W12 PUSHED + W13 ACTIVE 2 行 |
| 8 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | 头部 status 加 W12 PUSHED + W13 ACTIVE 2 行（A19 仍 SUPPORT DOCS ONLY 强调） |
| 9 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | L1 标题 + 头部 status 加 W12 PUSHED 1 行 + 末尾 [W13 verification scope] 段 |
| 10 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | L1 标题 + 头部 status 加 W12 PUSHED + W13 ACTIVE 2 行 + 末尾 [W12 reconciliation] + [W13 active] 段 |

（注：M5-7/8/9 头部 L1 已在 W12 patch 中加 W11 PUSHED + W12 ACTIVE 行，W13 不再加（plugin 范围与 graph 范围分离），保持 W12 状态稳定。）

### 8.2 整包交付物清单

- `logs/checkpoints/A1-M5-W13-reconciliation-20260907-2358.md`（**本卡** · A1 W13 文档对账立场 + 11 lane 角色 + 8 锚点 + 8 硬停验证 + 14 文件改动清单）
- `logs/checkpoints/Lane-A1-M5-W13-reconciliation-20260907-2358.patch`（**A1 W13 整包 git diff patch**，14 文件 = 4 文件树 top-level + 6 张 M5-0/10/11/12/13/14 M5 子卡）

### 8.3 A1 W13 零产品代码

A1 W13 = **START DOCS ONLY**：**未触** `src-tauri/`、`src/`、`package.json`、3 主文档正文、ACL/Capability/Manifest、pre-merge.sh；**不**移动 `NEXT`（`NEXT` 仍 `M5-W14`，由 A0 W13 拣入期派发）；**不**提交、**不** push。

---

## 9. 后续 A0 拣入期建议

A0 W13 拣入期建议步骤（仅参考，不强制）：

1. **A0 W13 拣入接收** = `git pull` 当前 master + 检查 A1 W13 整包 14 文件 + 跑 `git diff --check`（A1 提交前已自检 whitespace 干净）
2. **A11 W13 verification delta** = 收口 cargo test plugin / 全 cargo / `check-plugin-policy.py` ACTIVE 7 / graph policy / MCP policy / Agent-Skill lock / graph UI / agent-skill UI / npm build / pre-merge / build metrics ≤ 23% / cargo_warnings delta = 0
3. **A0 拣入 W13 commit message 模板**：`feat(M5): integrate W13 plugin stage-I manifest lifecycle`（参照 W10/W11/W12 拣入命名规范）
4. **A0 W14 派发 = A19 plugin UI 6 项 + A9 plugin stage-II（视情况）+ A2/A3/A4/A5/A6/A7/A8/A10/A11 复审 W13 收口**

---

## 10. W13 = DOCS_ONLY（与 W12 同形；A1 仍 hold 整包待 A0 拣入）

**W13 整包状态** = A1 持有（`logs/checkpoints/A1-M5-W13-reconciliation-20260907-2358.md` + `logs/checkpoints/Lane-A1-M5-W13-reconciliation-20260907-2358.patch`），A0 拣入期消解。

**A1 W13 零产品代码 = A1 W12 整包交付模式的延续**（W12 整包 14 文件 / 203 行 / 13 文件改动；W13 整包 14 文件 / 与 W12 同形 14）。

A1 W13 = **START DOCS ONLY DONE**。
