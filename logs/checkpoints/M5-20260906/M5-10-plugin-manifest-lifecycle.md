# M5-10 插件 manifest 与生命周期（签名/load/unload）

> 子卡 ID：**M5-10** · 需求 #15（插件系统）· `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A18**
> 父卡：`详细设计与实施计划.md` L574（`M5-10 插件 manifest 与生命周期`）
> 主预研：`logs/assist/M5-15.a-prework-20260902-1055.md` · `logs/assist/A9-M5-plugin-form-feasibility-20260906-0700.md`（**默认形态③声明式**）
> 配套：`M5-11-plugin-commands-isolation.md`（命令与隔离）· `M5-12-plugin-ui.md`（管理 UI）
>
> **W3** BLOCKED（待 A5 W4 契约 + A3 W3 MCP policy shell）· **W4** ACTIVE（A9 W4 仍 SUPPORT DOCS ONLY）· **W5** ACTIVE（A9 W5 仍 SUPPORT DOCS ONLY）· **W6** PUSHED · **A9 升级为 START PRODUCT CODE** · `5f92ece` 拣入（M5-10 manifest DTOs + validation + 7 状态机 + 12 合法边 + permission manifest rules + `scripts/check-plugin-policy.py` 6 ACTIVE 码；详见本卡顶部 `[W6 next-card acceptance criteria]` 段；M5-11 同步拣入见其卡顶部 [W6] 段；`5f92ece` git stat 实测含 `src-tauri/src/plugin.rs` 446 行 + `domain.rs` 追加 `MAX_PLUGIN_CAPABILITIES=5` + `security_policy.rs` 47 行补丁）· **W7** RECONCILIATION（A3 W7 mcp_* 3 命令 + A11 W7 pre-merge FAIL 3 red lights + A6 W7 wiring 在 `6c1f30e` / `daa10f6` / `a29b796` 已拣入 master；A9 M5-10/11 在 W7 仍无新命令落地，5 stub `plugin_invoke/cancel/permissions_get/audit_list/storage_get_or_put_or_delete` 维持 `Err("not-implemented-in-W6")`；**A11 W7 pre-merge FAIL 同源于 `5f92ece` 集成卫生** —— plugin.rs L274 test module 缺 `PluginCapability` import + L20 顶层 `PluginCapability` unused + bridge.rs 8 处未格式化 + cargo_warnings 2→3，单根修复配方见 `daa10f6` commit message，A0 W8 拣入期消解；A1 W7 整包本卡修订未进 master，留 W8 整包合并拣入；详见 `M5-0-overview.md` 顶部 `[W7 reconciliation]` 段 + `M5-13-verification-matrix.md` 顶部 `[W7 verification scope]` 段）· **W8** ACTIVE（**A9 W8 = POLICY REVIEW ONLY** —— Plugin surface W8 review：无 install/enable/delete/download/runtime command 漏入 / lifecycle 维持 pure / capability verdict text 维持 bounded/redacted；**不**写 plugin runtime / install / delete / download；**无** runtime product code；可扩 plugin policy docs/tests 仅在具体 failure 时；A3 W8 = HOLD/NO ASSIGNMENT；详见 `M5-0-overview.md` 顶部 `[W8 active]` 段 + `M5-13-verification-matrix.md` 顶部 `[W8 verification scope]` 段）
> **W8 reconciliation**（2026-09-07 14:30 CST · A0 拣入）：A0 在 **`4d7be97 feat(M5): integrate W8 command bridge polish`**（53 files +6038 -101）+ **`94e763e fix(M5-W8,A3): close MCP policy phase debt`**（MCP policy phase debt 关闭 · `MCP_NO_RMCP_SERVER` + `--expect-current-gaps` gate + ACTIVE=8 PENDING=0）+ **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8`**（A11 W8 verification delta 功能性 ALL_PASS）+ **`97118d6 chore(M5): normalize W8 patch evidence whitespace`**（3 份 W8 patch 文件空白规范化）4 commit 中拣入本卡 W8 修订：① A9 W8 `check-plugin-policy.py` 复审补丁（policy-only hunk，无 plugin runtime / install / delete / download）落地；② A1 W7 reconciliation 整包合并拣入（DEBT-32 secret-echo 残留 + A11 W7 pre-merge FAIL 3 red lights 配方归档在本卡 [W7 reconciliation] 段）；③ A1 W8 reconciliation 整包合并拣入（本卡头部 + [W8 active] 段 + 顶部 history 链更新）；**A1 W7 + W8 整包合并拣入 9 + 11 = 20 文件均含本卡修订**。
> **W9** ACTIVE（**A9 W9 = PLUGIN REVIEW ONLY** —— plugin surface 复审：manifest/lifecycle 维持 pure + 确认 install/enable/delete/download 仍缺（DEBT-12 不在 W9 消）+ capability verdict text 维持 bounded/redacted；**不**实施 plugin runtime；A19 W9 仍 SUPPORT DOCS ONLY —— plugin UI 待 W10+ A19 派发，DEBT-04 收口推 W10；A3 W9 = POLICY/REVIEW ONLY MCP policy current-phase green + 后续 M5-2.b 卡预备，不实施 rmcp server/listener/network；详见 `M5-0-overview.md` 顶部 `[W9 active · 2026-09-07 14:30 CST]` 段 + `M5-13-verification-matrix.md` 顶部 `[W9 verification scope]` 段）
> **W10** ACTIVE（**A9 W10 = PLUGIN RUNTIME PLAN ONLY**（plugin runtime dispatch 卡预备：install/enable/delete/list 命令序列 + signature failure modes + storage limits + audit redaction + UI dependencies；**不**实施 plugin runtime）；**plugin install/enable/delete/download runtime 仍 LOCKED**（W10 Hard Stop L207）；详见 `M5-0-overview.md` 顶部 `[W10 active · 2026-09-07 16:00 CST]` 段 runtime-lock 状态表）
> **W10 PUSHED**（2026-09-07 18:30 CST · A0 拣入 `5226aad` = HEAD）：A0 在 **`ba78092 feat(M5-W10,A3): MCP stdio-prep skeleton (feature-gated, read-only, std-only)`**（A3 W10 实施期）+ **`5226aad feat(M5): integrate W10 MCP stdio prep`**（A0 W10 整包合并拣入）两 commit 中拣入 W10：① A9 W10 plugin runtime plan only 落地（manifest/lifecycle 维持 pure + install/enable/delete/download 仍缺 + capability verdict text 维持 bounded/redacted）；② A1 W10 reconciliation 整包合并拣入；③ A11 W10 verification delta 收口（`check-plugin-policy.py --self-test` PASS(ACTIVE=6) + plugin manifest/lifecycle validation 维持 pure）；**M5-10 关联债 DEBT-04（plugin UI runtime）= W11+ 仍挂账**。
> **W11** ACTIVE（**A9 W11 = PLUGIN DOCS/POLICY ONLY**（W12/W13 plugin staged install/enable/delete/download cards 预备；**不**实施 plugin runtime）；**plugin install/enable/delete/download runtime 仍 LOCKED**（W11 Hard Stop L207）；详见 `M5-0-overview.md` 顶部 `[W11 active · 2026-09-07 18:30 CST]` 段 runtime-lock 状态表 10 行）
> **W11 PUSHED**（2026-09-07 20:30 CST · A0 拣入 `269269a` = HEAD）：A0 在 **`269269a feat(M5): integrate W11 MCP stdio dry-run hardening`** 中拣入 W11；MCP stdio dry-run 硬化 + A5 Agent/Skill 执行锁 policy（PENDING=6）；`cargo test --features mcp mcp_server` 21/21 + `check-mcp-policy.py` ACTIVE=11/PENDING=0；A1 W11 reconciliation 整包已合并拣入；**plugin runtime 仍 LOCKED**（W12 Hard Stop L209）。
> **W12** ACTIVE（**A9 W12 = PLUGIN DOCS ONLY**（W12/W13 plugin staged cards 细化仅在 W11 反馈改变 blocker 时；**不**实施 plugin runtime）；**plugin install/enable/delete/download runtime 仍 LOCKED**（W12 Hard Stop L209）；详见 `M5-0-overview.md` 顶部 `[W12 active · 2026-09-07 20:30 CST]` 段 runtime-lock 状态表 11 行）
> **W12 PUSHED**（2026-09-07 23:55 CST · A0 拣入 `3c3f460` = HEAD）：A0 在 **`3c3f460 feat(M5): integrate W12 graph live-query readonly bridge/UI`** 中拣入 W12：图谱 live-query 只读 3 命令 + A8 UI 消费；`cargo test graph` 15/15 + full cargo 414/414 + `check-graph-policy.py` ACTIVE=8 + `check-mcp-policy.py` ACTIVE=12/PENDING=0 + graph UI logic 113/113 + npm build PASS + build metrics 22.26% ≤ 23% + cargo_warnings delta = 0；A1 W12 reconciliation 整包合并拣入（含本卡头部 status 修订 + 顶部 history 链 + 14 文件 = +A1 W12 checkpoint 203 行 + A1 W12 patch）；**plugin install/enable/delete/download runtime 仍 LOCKED**（W13 Hard Stop）。
> **W13** ACTIVE（**A9 W13 = START PRODUCT CODE NARROW**（**plugin stage-I manifest 生命周期**：`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry` 6 命令；本地 manifest 解析 / 静态资源元数据装载 / install / enable / disable / list / get / key registry 安全本地状态机；**不**实施 runtime invoke / 不暴露执行边 / 不引入网络下载或监听 / 不动态加载代码 / 不暴露文件 / 数据库 / 脚本 / Agent / Skill 副作用；**复用** W6 `5f92ece` `plugin.rs` 5 stub `plugin_invoke/cancel/permissions_get/audit_list/storage_*` 维持 `Err("not-implemented-in-W6")`，stage-I **不**实现 invoke / cancel / storage_* 5 stub；ACL 加 6 条插末条 `list_artifact_images` 前；`bridge.ts` / `types.ts` 镜像对齐；`check-plugin-policy.py` 新增 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 政策码（ACTIVE 6 → 7））；**plugin invoke / command execution / dynamic code loading / network download/listener / MCP full runtime / Agent/Skill execution / daemon / model call / graph build-write-export 仍 LOCKED**（W13 Hard Stop）；详见 `M5-0-overview.md` 顶部 `[W13 active · 2026-09-07 23:55 CST]` 段 runtime-lock 状态表 13 行；**W13 plugin 范围冻结 = 恰好 6 条 stage-I manifest 生命周期命令**（详见 [W13 stage-I scope] 段）
> **W13 PUSHED**（2026-09-08 00:20 CST · A0 拣入 `a7eefbb` = HEAD）：A0 在 **`a7eefbb feat(M5): integrate W13 plugin manifest lifecycle`** 中拣入 W13：8 local-only lifecycle/key-registry 命令（`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry` / `plugin_audit_get` / `plugin_storage_*` 4 stub 错误结构不变）+ 原子注册表持久化 + redacted DTO/audit + ACL/source-check parity + **无** execution surface；`cargo test plugin` 全绿 + `check-plugin-policy.py` ACTIVE=8/PENDING=0 + `check-plugin-privacy.py` ACTIVE=4/PENDING=0；A1 W13 reconciliation 整包合并拣入；**plugin UI runtime 仍 LOCKED**（W14 Hard Stop）+ **A19 仍 SUPPORT DOCS ONLY**（plugin UI 6 项仍 0% ACTIVE，DEBT-04 W14+ 仍挂账）。
> **W14** ACTIVE（**A6 W14 = START PRODUCT CODE NARROW（plugin manager UI 唯一产品代码 lane）** —— 消费 frozen W13 6 命令（`plugin_list` / `plugin_get` / `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_key_registry`）+ `usePluginStore` + `check-plugin-ui-logic.mjs` + workspace navigation 集成；6 action = list / filter / inspect / install-supplied-manifest / enable / disable / key fingerprint；**不**实施 W14 新 lifecycle 命令 / **不**碰 `src/bridge.ts` DTO 形态 / **不** raw Tauri `invoke` / **不**渲染 raw signature / public-key / resource path / manifest metadata / credentials / 请求响应 body / stdout / stderr；**plugin invoke / 命令执行 / 动态加载 / 网络下载监听 / daemon / 模型调用 / Agent-Skill 执行 / MCP 全量 / graph 写导出 / 后台 worker 仍 LOCKED**（W14 Hard Stop）；W14 命令名 + DTO 冻结 = 复吃 W13 6 命令，不增删 lifecycle；**A1 W14 = START DOCS ONLY**（reconcile W13 accepted + mark W14 active + **不**改 product scope）；A19 仍 SUPPORT（plugin UI 6 项 ACTIVE 仍 0%，待 A6 W14 实施完成 = DEBT-04 W14+ 仍挂账）；详见 `M5-0-overview.md` 顶部 `[W14 active · 2026-09-08 00:20 CST]` 段 runtime-lock 状态表 14 行 + `M5-12-plugin-ui.md` 末尾 [W14 active] 段）
> **W14 PUSHED**（2026-09-08 09:30 CST 对账 · A0 拣入 `886ea29` = HEAD）：A0 在 **`886ea29 feat(M5): integrate W14 plugin manager UI`** 中拣入 A6 W14 plugin manager UI 并接受（`STATUS=PASS`）：消费 frozen W13 6 命令（`plugin_list` / `plugin_get` / `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_key_registry`）+ `src/components/plugin/**` + `src/stores/usePluginStore.ts` + `scripts/check-plugin-ui-logic.mjs` + workspace navigation 集成；6 action 全覆盖 + plugin UI logic **61/61** + plugin Rust 28/28 + full Rust 431/431 + MCP feature 21/21 + `npm run build` PASS + build metrics `total_bytes_pct=24.89` ≤ **25%**（A0 W14 拣入期阈值 23% → 25% 上调）+ cargo_warnings delta = 0；A0 修正 Pinia 自动解包与 raw error 透传；**plugin invoke / 命令执行 / 动态加载 / 网络下载监听 / daemon / 模型调用 / Agent-Skill 执行 / MCP 全量 / graph 写导出 / 后台 worker 仍 LOCKED**（W15 Hard Stop）
> **W15** ACTIVE（**A6 W15 = NARROW UI POLISH ONLY**（accessible confirmation / modal focus + empty/loading/error 态；**不**新增命令 / runtime surface / 体积增量）+ **其余 10 lane = review / docs / verification**（A1 DOCS ONLY / A2 BOUNDARY / A3 MCP ISOLATION / A4 PRIVACY+STABLE-ERROR / A5 AGENT-SKILL LOCK / A7 GRAPH NON-REGRESSION / A8 GUI ACCEPTANCE / A9 FROZEN CONTRACT（**no backend changes**）/ A10 SECURITY / A11 VERIFICATION）；**W15 = release readiness + GUI 验收，不扩张运行时权限**；本卡 W15 **无** manifest / lifecycle 命令改动、**无** DTO / ACL 改动、**无**后端改动；详见 `M5-0-overview.md` 末尾 `[W15 active · 2026-09-08 09:30 CST]` 段 runtime surface 锁定状态表 14 行 + `M5-13-verification-matrix.md` 末尾 [W15 verification scope] 段）

---

## [W6 next-card acceptance criteria · 2026-09-06 19:25 CST] A9 M5-10 W6 实施期 acceptance criteria（manifest DTOs + validation + lifecycle state machine + permission manifest rules + policy script · 不 install/uninstall/delete/download/execute real plugins / 不做网络 / 不做签名强制 / 不注册 10 条 plugin_* 命令 / 不破 capability.rs 漂移）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L160（**A9 M5-W6** *"Implement M5-10/M5-11 plugin manifest/lifecycle policy slice: DTOs, validation, lifecycle state machine, permission manifest rules, policy script. No install/uninstall file mutation runtime, no downloaded plugins, no signature enforcement beyond pure validation unless fully local."*）+ L164-170 硬约束 + A9 prework（A9-M5-plugin-system-prework-20260906-1100.md + A9-M5-plugin-impl-seam-20260906-1630.md + A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md）+ A5 W4 capability 真源（A5-W4 `src-tauri/src/agent.rs` `preview_*` + AclLevel + MCP_CAPABILITY_V1）。
> **消费依赖（已落地，A9 W6 可直接接入）**：
> - **A5 W4 AgentDef/SkillDef + permission preview 已落**（`1610939` 拣入 `src-tauri/src/agent.rs` + `skills.rs`）—— A9 W6 PluginManifest schema / PluginCapability 命名 / permission manifest rules **复用** 同一 capability.rs 真源。
> - **A3 W3 MCP command-registry + global policy shell 已落**（`12f1cff` 拣入）—— A9 W6 lifecycle state machine 与 A3 W3 MCP registry 复用同款"白名单+审计"基元。
> - **A2 W2 constants centralized**（`712a14c`）—— A9 W6 PluginManifest 容量 / 路径 / 字节上限**复用** `MAX_TEXT_FIELD_BYTES` / 路径 policy 同一真源，**禁止**在 `plugin.rs` 写容量字面量。
> - **A4 W4 agent_memory KV privacy 双扫**（`1610939` `agent_memory.rs`）—— A9 W6 plugin 资源 metadata **复用** 同一套 `SENSITIVE_KEY_NAMES` + `SENSITIVE_VALUE_PATTERNS` 隐私断言（plugin 不存 secret / token / DSN）。
> - **A7 W5 graph 7 容量常量**（`4b438ef`）—— A9 W6 plugin metadata 容量**不**与 graph 耦合，但 PluginManifest 字段长度上限**复用** `MAX_TEXT_FIELD_BYTES=64KiB` 同一真源。
> **A1 W6 角色**：A1 W6 **不**改 §1~§11 决策史；仅在头部加本 `[W6 next-card acceptance criteria]` 段，**明确 A9 W6 实施期 4 项 AC + 5 项 hard stops**，供 A9 / A10 / A11 / A0 验收。

### W6 A9 M5-10 实施期 acceptance criteria（4 项）

| AC | 描述 | 验收证据 |
|----|------|----------|
| AC-1 **PluginManifest DTOs + validation 冻结** | `src-tauri/src/domain.rs` 追加：① `PluginManifest { id, version, display_name, description, min_app_version, entry: PluginEntry, capabilities: Vec<PluginCapability>, hash, signature: PluginSignature, metadata: serde_json::Value }` ② `PluginEntry { entry_url, icon }` ③ `PluginCapability { capability, reason }`（**capability 字段名复用** capability.rs 既有 `MCP_CAPABILITY_V1` 命名空间，**不**重定义）④ `PluginSignature { algorithm, key_id, value, signed_at }`（**算法仅声明 "Ed25519" 字面量**；**不**实装验证逻辑，validation 仅做 schema 字段长度/必填/版本正则 + 资源 hash 字节长度 64-hex sha256）⑤ `validate_plugin_manifest(m: &PluginManifest) -> ValidationResult` 纯函数（**不**调 fs / **不**调 crypto / **不**调 network）—— 容量上限：`display_name ≤ 64` / `description ≤ 1024` / `min_app_version` 必为 semver / `metadata` JSON 序列化字节 ≤ `MAX_TEXT_FIELD_BYTES/2`（**复用** A2 W2 常量）| `cargo test --manifest-path src-tauri/Cargo.toml plugin` PASS + `python3 scripts/check-plugin-policy.py --self-test` PASS + A10 抽查 |
| AC-2 **lifecycle state machine 状态机冻结** | `src-tauri/src/plugin.rs`（或 `src-tauri/src/plugin_lifecycle.rs`，A9 W6 决定）追加：① `PluginState` 枚举（`Discovered` / `Validating` / `Signed` / `Loaded` / `Enabled` / `Disabled` / `Uninstalled`）② `PluginLifecycleEvent` 枚举（`OnDiscovered` / `OnValidationOk` / `OnValidationFail` / `OnSigned` / `OnSignedFail` / `OnLoad` / `OnEnable` / `OnDisable` / `OnUninstall`）③ `transition(state, event) -> Result<PluginState, TransitionError>` 纯函数（**不**写文件 / **不**调网络 / **不**调真实 crypto）④ 状态转移图覆盖 7 态 + 至少 12 条边（详见 A9 W5 提案 §3 状态机扩展点）+ 至少 3 个非法转移（`Discovered → Enabled` / `Loaded → Uninstalled` 等）返回 `TransitionError` | 单测 + `python3 scripts/check-plugin-policy.py` PASS + 状态转移图文档（A9 W6 出 `logs/assist/A9-M5-W6-plugin-lifecycle-state-machine-20260906-XXXX.md` 备查）|
| AC-3 **permission manifest rules + capability 真源单点** | ① `PermissionManifestRule` 纯函数（输入 `Vec<PluginCapability>` + `&CapabilityRegistry`（= capability.rs 既有白名单）→ 输出 `PermissionVerdict { allowed: Vec<PluginCapability>, denied: Vec<(PluginCapability, DenyReason)> }`）② 拒绝原因枚举：`CapabilityNotInWhitelist` / `CapabilityEmpty` / `ReasonEmpty`（capability 字段名必填 reason 字段，**禁止**空 reason 通过）③ 同一 capability **不**允许在 plugin metadata 出现 2 次（去重）④ 同一 plugin 最多 5 个 capability（首期上限，**复用** `MAX_PLUGIN_CAPABILITIES=5` 单一真源常量）| 单测 + A10 抽查 + A5 W4 capability.rs 0 drift |
| AC-4 **policy script + pre-merge wire** | `scripts/check-plugin-policy.py` 新增：① 至少 6 ACTIVE 码：`PLUGIN_MANIFEST_SCHEMA_PRESENT` / `PLUGIN_VALIDATION_PURE` / `PLUGIN_LIFECYCLE_STATE_MACHINE` / `PLUGIN_CAPABILITY_WHITELIST_ONLY` / `PLUGIN_NO_INSTALL_RUNTIME` / `PLUGIN_NO_NETWORK` ② 3 模式（`--self-test` 至少 4 好 + 4 坏样本；`--default` 扫描当前仓库；`--expect-pending` 报 PENDING）③ `scripts/pre-merge.sh` 接入 `check-plugin-policy.py`（如 A6 W5 接入 `check-graph-policy.py` 同款位置）④ 无 secrets / token / DSN 写日志（参考 A4 W4 隐私双扫） | `python3 scripts/check-plugin-policy.py --self-test` PASS（ACTIVE=6） + `python3 scripts/check-plugin-policy.py` PASS + `python3 scripts/check-plugin-policy.py --expect-pending` PASS（如有 PENDING） + `bash scripts/pre-merge.sh` ALL_PASS |

### W6 A9 M5-10 实施期 hard stops（5 项）

| HS | 约束 | 来源 |
|----|------|------|
| W6-HS1 | **不 install/uninstall/delete/download/execute/enable 真实 plugins** —— W6 仅纯 manifest/lifecycle policy + validation（**不**写 `plugins_dir()` 文件 IO / **不**写 install 真实解包 / **不**写 delete 真删文件 / **不**做网络下载）| PARALLEL_COMMAND_BOARD L160 + L168 |
| W6-HS2 | **不做网络 / 不做下载 / 不做签名强制**（仅 schema 字段校验 + algorithm 字面量 `"Ed25519"` 声明；**不**实装 Ed25519 验证 / **不**连 NTP / **不**拉远端 pubkey / **不**写 trusted-pubkeys.json 持久化）—— 持久化 / 网络 / 加密 / 远端校验在 W7+ 派发 | PARALLEL_COMMAND_BOARD L160 + L168 |
| W6-HS3 | **不注册 10 条 plugin_* 命令**（**不**改 `src-tauri/bridge.rs` / `main.rs` / `default-commands.toml` / `src/bridge.ts` / `src/types.ts` 任何 plugin_* 条目；W6 优先倾向 **0 新命令**）—— 10 条命令在 W7+ 由 A9 实施期承接 M5-11 §3 WRITE 列表时引入 | PARALLEL_COMMAND_BOARD L160 + L169（*"prefer no command in W6"*）|
| W6-HS4 | **不破 capability.rs 漂移**（A2P / A2A / Skill / Plugin / Agent 五类共用 capability.rs 单一真源；A9 W6 加 `PLUGIN_CAPABILITY_V*` 必须**走同一文件**，**禁止**在 `plugin.rs` 内嵌 capability 字面量；`PermissionManifestRule` 接受 `&CapabilityRegistry` 参数）| A5 W4 + A3 W3 + M5-2 §4.2-4.3 + M5-10 §4.1 形态③ |
| W6-HS5 | **不破 K1（ACL 末条恒为 `list_artifact_images`）+ K3 + K5** —— 任何 plugin_* 命令插入必须插在 `list_artifact_images` **之前**（W6 期间不插，但 W7+ 接入时严守）| M5-10 §5 FORBID + 全局 K1 |

### W6 验证清单（供 A11 收口）

- `cargo test --manifest-path src-tauri/Cargo.toml plugin` PASS（如 A9 W6 创建 plugin module）
- `cargo test --manifest-path src-tauri/Cargo.toml domain` PASS
- `cargo test --manifest-path src-tauri/Cargo.toml` 全绿（无新增 warning > 0；W6 受 A0 warning 门禁约束）
- `python3 scripts/check-plugin-policy.py --self-test` PASS（ACTIVE=6）
- `python3 scripts/check-plugin-policy.py` PASS
- `python3 scripts/check-plugin-policy.py --expect-pending` PASS（如有 PENDING）
- `bash scripts/pre-merge.sh` ALL_PASS
- `git diff --check` CLEAN
- A11 比对 `src-tauri/src/domain.rs` 与 A5 W4 capability.rs / A3 W3 MCP registry 0 drift
- `grep -nE 'plugin_install|plugin_uninstall|plugin_list|plugin_get|plugin_enable|plugin_disable|plugin_keys_add' src-tauri/src/bridge.rs src-tauri/src/main.rs src-tauri/permissions/default-commands.toml src/bridge.ts src/types.ts` W6 期间 0 命中（10 条命令 W7+ 才插）
- `grep -nE 'tauri::Manager|std::fs::write|reqwest|ureq' src-tauri/src/plugin.rs src-tauri/src/plugin_lifecycle.rs` 0 命中（无真实 IO / 无网络）
- `grep -nE 'ed25519|ring::signature|signature::Signer' src-tauri/src/plugin.rs` 仅命中 `algorithm = "Ed25519"` 字面量声明（不命中真验证代码）
- **A10 复审 PASS**（无 install runtime / 无网络 / 无签名强制 / 无 10 条命令 / capability 真源单点 / 隐私断言 / 状态机非法转移覆盖）
- **A11 verification delta** 产出 `logs/checkpoints/M5-A11-W6-*.md`

### W6 A1 不修订范围（本卡）

- **§1 GOAL / §2 READ / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION / §9 DOC_BACKWRITE / §10 COMMIT / §11 FORBID 遵守记录**：A1 W6 **不动**（决策史保持 W0 原文；W6 AC 在本顶部段单列；M5-10 §4.1 形态决策与 §4.4 签名验证在 W6 **仅冻结 schema，不实装**）。
- **三份主文档 / ACL / Capability / pre-merge.sh / scripts/**：A1 W6 不动（policy 脚本由 A9 W6 落地）。
- **`NEXT` 标记**：A0 调度权；A1 不改字面值。

---

## 0. 编号与锚定

- 批次任务号 `M5-10`；需求号 #15；WBS L574 一致。
- 依赖：A0 拍形态决策（**默认形态③声明式**）+ M5-2 ✅（capability.rs 共用）
- **关键决策交 A0 拍**：① 形态（**默认形态③**——Tauri v2 不支持按 webview 关全局，**形态②放弃**）② 签名方案（**默认 Ed25519 + 本地受信任 keys 目录**）

---

## 1. GOAL

冻结 `PluginManifest` schema（id / version / entry / capabilities / hash / signature / min_app_version）；实现插件加载/卸载生命周期（manifest 解析 → 签名校验 → capability 校验 → 加载到 `plugins_dir()` → 注册到 `capability.rs` → 启用 / 停用 / 卸载）；**默认形态③**——插件是"声明式资源包"（图标 + 入口 URL + 权限声明），**不**是独立 webview，**不**是独立 stdio 进程。

---

## 2. READ

1. `logs/assist/M5-15.a-prework-20260902-1055.md`（**全读**，manifest/签名/形态）
2. `logs/assist/A9-M5-plugin-form-feasibility-20260906-0700.md`（**全读**，三形态可行性）
3. `M5-2-rmcp-mcp-policy.md` §4.2-4.3（capability.rs 共用）
4. `M5-4-agent-skill-runtime.md` §4.2（`SkillExec` 禁内联，K6）
5. `src-tauri/src/workspace.rs`（`plugins_dir()`）
6. `src-tauri/src/keyring_store.rs`（签名 key 存放）
7. `src-tauri/src/domain.rs`（契约根，**全读**）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src-tauri/src/plugin_manifest.rs` | **新增** | Manifest 解析 + 校验 + 签名验证 |
| `src-tauri/src/plugin_lifecycle.rs` | **新增** | load / unload / enable / disable 状态机 |
| `src-tauri/src/plugin_signature.rs` | **新增** | Ed25519 签名 + 验证 + 本地受信任 keys |
| `src-tauri/src/plugins/` | **新增目录** | `plugins_dir()` 默认 `~/.local/share/.../plugins/` |
| `src-tauri/src/keys/trusted-pubkeys.json` | 持久化 | 受信任公钥列表（受 keyring 保护访问） |
| `src-tauri/src/domain.rs` | 新增类型 | `PluginManifest` / `PluginState` / `PluginCapability` / `PluginSignature` |
| `src-tauri/src/capability.rs` | 扩展 | 加 `PLUGIN_CAPABILITY_V1` 列表（首期空，按需逐个评估） |
| `src-tauri/src/bridge.rs` | 修改 | `plugin_install` / `plugin_uninstall` / `plugin_list` / `plugin_get` / `plugin_enable` / `plugin_disable` / `plugin_keys_add/list/remove`（**全进 ACL**） |
| `src-tauri/permissions/default-commands.toml` | 修改 | 插 `plugin_*` 共 10 条于 `list_artifact_images` 之前 |
| `src/types.ts` | 新增 | TS 镜像 `PluginManifest` |

---

## 4. 关键契约

### 4.1 形态决策（默认形态③ 声明式）

| 形态 | 内容 | 决策 |
|---|---|---|
| ① 独立 stdio 进程 | 插件 = 独立可执行 + stdio 通信 | **否决**（与"无第二执行路径"冲突） |
| ② 独立 webview（host 进程） | 插件 = webview，访问 `window.__TAURI__` | **否决**（`withGlobalTauri` 全局未按 webview 关闭，跨插件泄露） |
| **③ 声明式资源包**（默认） | 插件 = 图标 + 入口 URL + 权限声明 + JS 钩子 | **采用** |

**形态③插件的本质**：
- 入口 URL 走既有 webview 框架（不开新 webview）
- JS 钩子通过受控 `bridge.ts` 子集调用（不允许裸 `invoke`）
- 资源文件（图标/文案）走 `workspace/` 同款隔离

### 4.2 `PluginManifest` schema

```rust
pub struct PluginManifest {
    pub id: String,                  // 反向域名规范
    pub version: String,             // semver
    pub display_name: String,
    pub description: String,
    pub min_app_version: String,     // semver，过期拒绝
    pub entry: PluginEntry,          // 形态③
    pub capabilities: Vec<PluginCapability>,  // 引用 capability.rs
    pub hash: String,                // 资源包 sha256
    pub signature: PluginSignature,  // Ed25519
    pub metadata: serde_json::Value,
}
pub struct PluginEntry {
    pub entry_url: String,           // 形态③ 入口 URL（与既有 webview 同源）
    pub icon: String,                // 图标相对路径
}
pub struct PluginCapability {
    pub capability: String,          // 引用 capability.rs 常量
    pub reason: String,              // 为什么需要此 capability
}
pub struct PluginSignature {
    pub algorithm: String,           // "Ed25519"
    pub key_id: String,              // 引用 trusted-pubkeys.json 的 key
    pub value: String,               // base64 签名
    pub signed_at: DateTime<Utc>,
}
```

### 4.3 生命周期状态机

```
Discovered → Validating → Signed(OK|Failed) → Loaded → Enabled → Disabled
                                                    ↓
                                                Uninstalled
```

- `Discovered`：扫描 `plugins_dir()` 找到新 manifest
- `Validating`：schema 校验 + `min_app_version` 校验
- `Signed`：签名验证（trusted-pubkeys.json 必须在 key）
  - **Failed**：拒绝加载 + 写 `audit.json` + 通知 UI
- `Loaded`：解压到 `plugins_dir()/<id>/<version>/` + 校验 hash
- `Enabled`：注册到 `capability.rs` + UI 可见
- `Disabled`：保留资源但移除 capability（可重新启用）
- `Uninstalled`：删 `plugins_dir()/<id>/`

### 4.4 签名验证

- 签名算法：**Ed25519**（默认）
- 公钥存 `~/.local/share/.../keys/trusted-pubkeys.json`（keyring 保护读写）
- 首次安装要求用户提供 pubkey（或开发者自带）
- 升级时必须验证 `hash` + `signature` 同时通过
- `manifest_hash` 变更 → 已启用插件自动 `Disabled`，等用户重新确认

### 4.5 二次确认（与 MCP/Skill 范式一致）

- 任何 `plugin_install` / `plugin_enable` → 走两段式确认闸门
- 必须显式列 `capabilities`（**逐项**）+ 来源 + 风险

---

## 5. FORBID

- **不**采用形态①/形态②（A9 feasibility 已决）
- **不**让插件接触 `window.__TAURI__` 全局（K1 + 形态③）
- **不**让 `PluginManifest.hash` 与 `signature` 校验任一失败时仍加载
- **不**让 `capability.rs` 漂移（A2P / A2A / Skill / Plugin / Agent 共用）
- **不**让 `trusted-pubkeys.json` 被未授权修改
- **不**让 `manifest_hash` 变更不触发重新确认
- **不**破 K1（ACL 末条恒为 `list_artifact_images`）/K3/K5
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 现状复核
ls src-tauri/src/plugins src-tauri/src/keys 2>&1
grep -nE "PluginManifest|plugin_lifecycle" src-tauri/src | head

# B. ACL 末条
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -5

# C. 反向用例
# N1: 形态② 插件（webview 访问 window.__TAURI__）→ 阻断
# N2: 签名验证失败 → 拒绝加载 + 审计
# N3: min_app_version 过期 → 拒绝加载
# N4: trusted-pubkeys.json 不含 key_id → 拒绝
# N5: 资源包 hash 与 manifest 不一致 → 拒绝
# N6: manifest_hash 变更 → 插件自动 Disabled
# N7: 插件安装不弹闸门 → 阻断

# D. 性能基线
# 10 插件安装 < 5s
# 100 插件列表 < 100ms

# E. 编译与基线
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# F. 门禁
bash scripts/pre-merge.sh
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | `PluginManifest` / `PluginState` 落 `domain.rs` | `grep` 命中 |
| 2 | 形态③ 是唯一形态；形态①/② 实现不存在 | `grep` 0 命中 |
| 3 | 签名验证失败拒绝加载 | 单测 N2 |
| 4 | `min_app_version` 过期拒绝 | 单测 N3 |
| 5 | `trusted-pubkeys.json` 缺失 key_id 拒绝 | 单测 N4 |
| 6 | 资源包 hash 校验失败拒绝 | 单测 N5 |
| 7 | `manifest_hash` 变更触发 Disabled | 单测 N6 |
| 8 | 安装必弹闸门 | 单测 N7 |
| 9 | ACL 末条仍为 `list_artifact_images` | 命令 B |
| 10 | `cargo test` 全绿 | 命令 E |
| 11 | `pre-merge.sh` ALL_PASS | 命令 F |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| 形态①/② 出现 | **红线失守**：与 K1/形态决策冲突 |
| 签名校验被绕过 | **红线失守**：阻断 |
| `trusted-pubkeys.json` 越权改 | 立即回滚 + 审计 + 提示用户检查 |
| `manifest_hash` 变更不触发确认 | 阻断（用户数据安全） |
| 闸门被绕过 | 阻断 |
| `cargo clippy` warning > 13 + 本卡新增 | 按基线清零再合入 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L574 `[ ]` → `[x]`
2. `后续需求TODO.md` §15 状态 `PARTIAL`（留 `M5-11/12`）
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-11`
4. `logs/checkpoints/M5-10.a-2026MMDD-HHMM.md`
5. `M5-14-debt-ledger.md` 增项：第三方签名服务（是否首期做） / 插件商店

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A18 实施填
- **NEXT**：M5-11（命令与隔离），同 A18 拆卡

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
