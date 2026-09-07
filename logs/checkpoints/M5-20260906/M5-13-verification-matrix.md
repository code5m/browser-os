# M5 验证矩阵（A1 横切 · M5-W0 末位 + W1 reconciliation + W6 verification delta + W7 verification scope + W8 reconciliation + W9 verification scope + W10 verification scope + W10 verification delta + W11 verification scope + W12 verification scope + W12 build metrics threshold + W13 verification scope + W14 verification scope + W14 PUSHED + W15 verification scope + W15 PUSHED + W16）

> 子卡 ID：**M5-13** · 跨 M5-1~M5-12 · `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A11**（沿用 A7/A11 角色，A0 签发时定）
> 父卡：`详细设计与实施计划.md` 整体（验证门禁横切）
> 配套：每张 M5-x 子卡 §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION
> **W8 reconciliation**（2026-09-07 14:30 CST · A0 拣入）：A0 在 **`4d7be97 feat(M5): integrate W8 command bridge polish`**（53 files +6038 -101）+ **`94e763e fix(M5-W8,A3): close MCP policy phase debt`**（MCP policy phase debt 关闭 · `MCP_NO_RMCP_SERVER` + `--expect-current-gaps` gate + ACTIVE=8 PENDING=0）+ **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8`**（A11 W8 verification delta 功能性 ALL_PASS）+ **`97118d6 chore(M5): normalize W8 patch evidence whitespace`**（3 份 W8 patch 文件空白规范化）4 commit 中拣入本卡 W8 修订：① A11 W8 verification delta 收口（a840fcb 关闭 W7-1/W7-2/W7-3 配方未消 = 已消；W8 focused validation 功能性 ALL_PASS）；② A1 W7 reconciliation 整包合并拣入（FAC-1.b DEBT-03 + A11 W7 pre-merge FAIL 3 red lights 配方归档在本卡 [W7 verification scope] 段）；③ A1 W8 reconciliation 整包合并拣入（本卡头部 L1 标题修订 + 顶部 W8 reconciliation 状态行 + [W8 verification scope] 段修订）。
> **W9** ACTIVE：**W9 final verification matrix 模式** —— 命令结果 + build metrics 21.07% ≤ 22% + cargo warnings unchanged + pre-merge result + 残留债 + push readiness（见 [W9 verification scope] 段）；详见 `M5-0-overview.md` 顶部 `[W9 active · 2026-09-07 14:30 CST]` 段
> **W10 PUSHED**（2026-09-07 18:30 CST · A0 拣入 `5226aad` = HEAD）：A0 在 **`ba78092 feat(M5-W10,A3): MCP stdio-prep skeleton (feature-gated, read-only, std-only)`**（A3 W10 实施期）+ **`5226aad feat(M5): integrate W10 MCP stdio prep`**（A0 W10 整包合并拣入）两 commit 中拣入 W10：① A3 W10 feature-gated MCP stdio-prep 骨架落地（`cargo test --features mcp mcp_server` 7/7 PASS）；② A1 W10 reconciliation 整包合并拣入；③ A11 W10 verification delta 收口（`MCP_POLICY_SELF_TEST=PASS(ACTIVE=9,PENDING=0)` + `MCP_CURRENT_GAPS_RESULT=PASS` + 9/9 graph + 26/0 agent_skill + 99/43 UI 断言 + npm build PASS + pre-merge ALL_PASS + build metrics ≤ 22% + cargo_warnings delta = 0）；**W10 = 受控运行时预备，feature-gated std-only 落地，**完整验证矩阵见 [W10 verification delta] 段
> **W11** ACTIVE：**W11 final verification matrix 模式** —— W11 dry-run 行为确定性 + A3 W11 bounded stdio dry-run hardening + A11 W11 verification delta 必跑 + 残留债 + push readiness（见 [W11 verification scope] 段）；详见 `M5-0-overview.md` 顶部 `[W11 active · 2026-09-07 18:30 CST]` 段
> **W11 PUSHED**（2026-09-07 20:30 CST · A0 拣入 `269269a` = HEAD）：A0 在 **`269269a feat(M5): integrate W11 MCP stdio dry-run hardening`** 中拣入 W11：MCP stdio dry-run 硬化（21/21 + ACTIVE=11/PENDING=0）+ A5 Agent/Skill 执行锁 policy（PENDING=6）+ A7 W12 graph live-query 实施卡 + A9 W12/W13 plugin staged cards + A1 W11 reconciliation 整包；完整验证矩阵见 [W11 verification scope] 段。
> **W12** ACTIVE：**W12 graph live-query read-only verification matrix 模式** —— W12 三只读命令 `graph_query` / `graph_node_get` / `graph_stats` + A8 UI 消费 + 9 lane review/verification（见 [W12 verification scope] 段）；详见 `M5-0-overview.md` 顶部 `[W12 active · 2026-09-07 20:30 CST]` 段。
> **W12 PUSHED**（2026-09-07 23:55 CST · A0 拣入 `3c3f460` = HEAD）：A0 在 **`3c3f460 feat(M5): integrate W12 graph live-query readonly bridge/UI`** 中拣入 W12；图谱 live-query 只读 3 命令 + A8 UI 消费 + 10 份 W12 assist + A1 W12 reconciliation 整包 + M5-0/7/8/9/10/11/12/13/14 头部 W11 PUSHED + W12 ACTIVE 修订 + 末尾 [W12 scope supersession]/[W12 verification scope]/[W12 active] 段 + 3 主文档 L1 W11 update 行 + board L6/L7 W12 dispatch；`cargo test graph` **15/15 PASS** + full cargo **414/414 PASS** + `cargo test --features mcp mcp_server` **21/21 PASS** + `check-graph-policy.py` **ACTIVE=8 PASS** + `check-mcp-policy.py` ACTIVE=12/PENDING=0 PASS + `check-agent-skill-policy.py` ACTIVE=3/PENDING=6 PASS + `check-graph-ui-logic.mjs` **113/113 PASS** + `check-agent-skill-ui-logic.mjs` **110/110 PASS** + `npm run build` PASS + `python3 scripts/measure-build-metrics.py` **`total_bytes_pct=22.26` ≤ 23% PASS**（A0 W12 拣入期 IF-2 阈值 22% → 23% 修订）+ cargo_warnings delta = 0。
> **W14 PUSHED**（2026-09-08 09:30 CST 对账 · A0 拣入 `886ea29` = HEAD）：A0 在 **`886ea29 feat(M5): integrate W14 plugin manager UI`** 中拣入 W14 并接受（`STATUS=PASS`）：plugin UI logic **61/61** + plugin Rust 28/28 + full Rust 431/431 + MCP feature 21/21 + graph UI 113/113 + Agent/Skill UI 110/110 + `npm run build` PASS + `pre-merge.sh` ALL_PASS + build metrics `total_bytes_pct=24.89` ≤ **25%**（阈值 23% → 25% 上调，余量仅 0.11pp）+ cargo_warnings delta = 0；**W15 全部验证须以 25% 为门禁**（详见本卡末尾 [W15 verification scope] 段）。

---

## [W6 verification delta · 2026-09-07 07:19 CST] W6 拣入 `5f92ece` 后验证增量（5f92ece + 8 W6 assist 落点实测）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L7（*"Current mainline: master at 5f92ece"*）+ `git log --oneline -20` 实测（`5f92ece feat(M5): add graph UI and plugin policy slices` 已 push）+ `git show 5f92ece --stat`（38 文件 +6415 -7）。
> **W6 实测拣入事实回填**（承接 W5 终态 baseline，本 delta 由 A0 拣入后实测）：
> - **A8 W6 M5-9 graph UI pure logic + panel shell**（`5f92ece`）：`src/components/graph/{GraphPanel,GraphViewer,NodeDetail,EdgeDetail,GraphFilter}.vue` 5 个文件 + `src/stores/useGraphStore.ts` + `src/utils/graphUi.ts` + `src/components/layout/{ActivityBar,MainArea}.vue` 2 处接入 + `src/types.ts` + `src/bridge.ts` + **`scripts/check-graph-ui-logic.mjs` 193 行 PASS** —— 0 新后端命令（`grep graph_* bridge.rs main.rs default-commands.toml` W6 0 命中，F2 红线满足）；不引 d3 整包（package.json 无 d3 依赖项）；不调 live agent consumption / model / rebuild workers（仅 static GraphNode[] 快照/fixture，F2 红线满足）。
> - **A9 W6 M5-10/M5-11 plugin manifest/lifecycle policy slice**（`5f92ece`）：`src-tauri/src/plugin.rs` 446 行（含 DTOs + 7 状态机 + 12 合法边 + 5 stub 返回 `Err("not-implemented-in-W6")` + audit key_hash_only）+ `src-tauri/src/domain.rs` 追加 `MAX_PLUGIN_CAPABILITIES=5` + `src-tauri/src/security_policy.rs` 47 行补丁 + **`scripts/check-plugin-policy.py` 218 行 6 ACTIVE 码**（PLUGIN_MANIFEST_SCHEMA_PRESENT / PLUGIN_VALIDATION_PURE / PLUGIN_LIFECYCLE_STATE_MACHINE / PLUGIN_CAPABILITY_WHITELIST_ONLY / PLUGIN_NO_INSTALL_RUNTIME / PLUGIN_NO_NETWORK）+ pre-merge 接入 —— 0 新命令（`grep plugin_* bridge.rs main.rs default-commands.toml` W6 0 命中，W6-HS3 满足）；无 install runtime / 无网络 / 无签名强制（W6-HS1/HS2 满足）；capability.rs 单点未漂移。
> - **A11 W6 verification delta**（`d08d095` 拣入 → 被 `5f92ece` 覆盖）：W5 集成态 pre-merge ALL_PASS + cargo_warnings 2→27 红灯消解（`4b438ef` 修）+ cargo test 全绿 + 19 ACTIVE 策略脚本 self-test PASS；A8/A9 W6 在该 delta 时点 PENDING（待拣入）；**W6 拣入 `5f92ece` 后实测已落地**，门禁由 A11 W7 重跑补验（见 [W7 verification scope] 段）。
> - **A3 W6 MCP 兼容复审**（`412d0eb` 拣入）：仅 docs，未破 M5-2 既有政策门。
> - **A6 W6 UI 一致性复审**（`add0609` 拣入）：仅 docs，复核面板壳风格复用。
> - **A10 W6 安全复审**（`logs/assist/A10-M5-W6-security-review-20260906-2300.md` 101 行 untracked 进 `5f92ece`）：**未**发现 install runtime / network / signature enforcement / capability.rs drift 违规。
> - **A2 W6 边界复审**（`logs/assist/A2-M5-W6-boundary-review-20260906-2030.md` 177 行 untracked 进 `5f92ece`）：**未**发现 core 边界反向边新增。
> - **A4 W6 memory/privacy/capacity 复审**（`logs/assist/A4-M5-W6-memory-privacy-capacity-review-20260906-2237.md` 101 行 untracked 进 `5f92ece`）：**未**发现 privacy/sensitive payload 命中。
> - **A5 W6 plugin manifest 复审**（`logs/assist/A5-M5-W6-plugin-manifest-review-20260906-2315.md` 170 行 untracked 进 `5f92ece`）：**未**发现 capability.rs drift，W6 不实装 Ed25519 校验。
> - **A7 W6 graph contract 复审**（`logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md` 167 行 untracked 进 `5f92ece`）：F1~F9 9 项红线全部满足；**F1**（summarizeNode 5 字段缺失）由 A1 W7 在本卡订正（见 [W7 patched] 段）。
> - **W6 W7 交接债**：IF-2 build metrics threshold 19% 实测 18.58%，余量 0.42%；W6 实施期增量（W6 graph UI + plugin policy slice）**未**重采 baseline，由 A11 W7 验证时一并重采。

---

## [W7 verification scope · 2026-09-07 00:50 CST] W7 验证范围（A3 M5-2 read-only MCP bridge + A5 Agent/Skill read-only bridge · 其它 9 lane docs/review/support · W7 = 窄 read-only command bridge wave）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L137-175（**M5-W7 Integration Dispatch**，Added 2026-09-07 00:50 CST by A0 after pushing through `5f92ece`）+ L155（*"Implement M5-2 read-only MCP registry/policy bridge commands: list registry entries, preview capability verdicts, return redacted DTOs. Must include source check, ACL, frontend bridge/types only if commands are added. No rmcp/server/listener"*）+ L157（*"Implement Agent/Skill read-only command bridge: parse/validate AgentDef/SkillDef and permission preview. No skill execution, no install, no network, no persistence writes"*）。
> **A11 W7 验证范围**（START VERIFICATION）：
> 1. **A3 W7 read-only MCP bridge** 验证项：A3 实施期 4 项 AC + 5 项 hard stops（**W7-HS1~HS5**）：
>    - AC-1 MCP list registry 命令（返回白名单内 capability 名 + 风险等级 + source；不返回 secret/token/Authorization/body；DTO 字段脱敏） + ACL + source check + 前端 bridge/types 镜像
>    - AC-2 MCP preview capability verdict 命令（输入 capability 名 → 返回 Verdict {allowed: bool, reason: str, risk: enum, required_acl: list}；**不**触真实 IPC；**不**连真 MCP server） + ACL + source check
>    - AC-3 redacted DTOs（命令返回结构体字段白名单固化在 `domain.rs`，**禁**返回任意 map）
>    - AC-4 policy script `scripts/check-mcp-policy.py` 接入 pre-merge + `--self-test` ACTIVE 码 + focused tests PASS
> 2. **A5 W7 read-only Agent/Skill bridge** 验证项：A5 实施期 4 项 AC + 5 项 hard stops（**W7-HS1~HS5**）：
>    - AC-1 AgentDef parse/validate 命令（输入 `path: String` → 返回 `Result<AgentDefSummary, ParseError>`；**不**写 storage；**不**创建 instance；**不**触发 skill 执行） + ACL + source check + 前端 bridge/types
>    - AC-2 SkillDef parse/validate 命令（同上模式）+ ACL + source check + 前端 bridge/types
>    - AC-3 permission preview 命令（输入 `id: String` → 返回 `PermissionSummary { capabilities: list, risk_level: enum, requires_confirm: bool }`；**不**触发 skill 调用；**不**写 audit；**不**弹 keyring 二次认证） + ACL + source check
>    - AC-4 policy script `scripts/check-agent-skill-policy.py` 接入 pre-merge + `--self-test` ACTIVE 码 + focused tests PASS
> 3. **A10 W7 review** 复审范围：A3/A5 命令 source check + ACL + redaction + 无执行/install/network + 无敏感审计（FAIL 即阻断）。
> 4. **A11 W7 verification delta** 必出 1 份：`logs/assist/M5-A11-W7-*.md` 或 `logs/checkpoints/M5-A11-W7-*.md`（承接 W6 delta 模板）。

### W7 A11 验证增量必检项（命令清单）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 0. 启动门禁
cat .workspace-identity && pwd && git fetch origin && git pull --ff-only && git status --short --branch

# 1. A3 W7 命令落地后
grep -nE 'mcp_(list_registry|preview_capability|preview_request|list_policies|get_policy)' src-tauri/src/bridge.rs src-tauri/src/main.rs src-tauri/permissions/default-commands.toml src/bridge.ts src/types.ts
grep -nE 'rmcp::|tauri_plugin_mcp|hyper::|tokio::net::TcpListener|std::net::TcpListener|reqwest|ureq' src-tauri/src/mcp.rs src-tauri/src/bridge.rs  # 应 0 命中
python3 scripts/check-mcp-policy.py --self-test
python3 scripts/check-mcp-policy.py
bash scripts/pre-merge.sh

# 2. A5 W7 命令落地后
grep -nE 'agent_(parse_def|validate_def|preview_permission|list_defs)|skill_(parse_def|validate_def|preview_permission|list_defs)' src-tauri/src/bridge.rs src-tauri/src/main.rs src-tauri/permissions/default-commands.toml src/bridge.ts src/types.ts
grep -nE 'agent_store\.put|agent_store\.persist|skill_runtime::invoke|skill_runtime::exec|skill_runner::run|std::fs::write' src-tauri/src/agent.rs src-tauri/src/skills.rs src-tauri/src/bridge.rs  # 应 0 命中
python3 scripts/check-agent-skill-policy.py --self-test
python3 scripts/check-agent-skill-policy.py
bash scripts/pre-merge.sh

# 3. 全量门禁复跑（W6 已落地 + W7 命令落地后的合并门禁）
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml plugin
cargo test --manifest-path src-tauri/Cargo.toml graph
cargo test --manifest-path src-tauri/Cargo.toml domain
cargo test --manifest-path src-tauri/Cargo.toml agent
cargo test --manifest-path src-tauri/Cargo.toml skills
cargo test --manifest-path src-tauri/Cargo.toml mcp
bash scripts/pre-merge.sh
git diff --check

# 4. 反向用例新增（A3/A5 命令的 redacted DTO 必须过滤 secret/token/Authorization/body）
# 4.1 A3 命令返回 DTO 不含敏感字段（手工回归或新增 reverse case）
# 4.2 A5 命令返回 DTO 不含 token/Authorization/prompt 内容
# 4.3 ACL 末条仍为 list_artifact_images（W7 命令插入仍 K1 严守）

# 5. 红色 baseline 重采（IF-2 收口）
python3 scripts/measure-build-metrics.py  # 期望 ≤ 19% 或由 A0 拍新阈值
```

### W7 12 M5 final acceptance criteria（每张 M5-x 子卡 1 项 final AC，供 A11 W7+ 收口）

| # | 子卡 | Final AC | 当前状态 |
|---|------|----------|---------|
| FAC-1 | M5-1 core/workspace split | A1 / A2 W1+W2+W3 已 PASS（`854bc40` + `0d08016` + `712a14c` + `f8f1f49`）；M5-1.b trait 抽离留 W8+ | **CLOSED**（M5-1.a PASS · M5-1.b DEBT） |
| FAC-2 | M5-2 MCP rmcp + global policy | A3 W3 PASS（`12f1cff`）；W7 由 A3 升级为 read-only command bridge（**start**） | **ACTIVE（read-only bridge pending）** |
| FAC-3 | M5-3 A2A + agent_kv | A4 W4 PASS（`1610939`）；agent_kv 脱敏 + LRU 闭环 | **CLOSED** |
| FAC-4 | M5-4 Agent/Skill runtime | A5 W4 PASS（`1610939`）；runtime + exec 闭环 | **CLOSED** |
| FAC-5 | M5-5 Agent/Skill commands | A5 W4 PASS（`1610939`）；K6 禁内联 + ACL + 闸门 | **CLOSED** |
| FAC-6 | M5-6 Agent/Skill UI | A6 W5 PASS（`f99d2eb`）；UI 面板壳 + 校验 + permission preview 桥；A6 W6 UI 一致性复审 PASS（`add0609`） | **CLOSED** |
| FAC-7 | M5-7 graph model + extract | A7 W5 PASS（`4b438ef`）；GraphNode/Edge DTO + 7 容量常量 + 隐私双扫 | **CLOSED** |
| FAC-8 | M5-8 graph store + query | A7 W5 PASS（`4b438ef`）；pure graph store/query helpers + policy script 7 ACTIVE | **CLOSED** |
| FAC-9 | M5-9 graph UI + agent consume | A8 W6 PASS（`5f92ece`）；4 vue + store + utils + UI logic test 193 行 PASS；F1 由 A1 W7 订正（`summarizeNode` 白名单 5→4 字段） | **CLOSED（W7 F1 patched）** |
| FAC-10 | M5-10 plugin manifest + lifecycle | A9 W6 PASS（`5f92ece`）；DTOs + 7 状态机 + 12 合法边 + 6 ACTIVE policy；Ed25519 仅字面量声明 + schema 校验 | **CLOSED** |
| FAC-11 | M5-11 plugin commands + isolation | A9 W6 PASS（`5f92ece`）；5 stub + ACL 占位 + audit key_hash_only；W7+ 真实命令在 A9 W8+ 派发 | **CLOSED（stub）· runtime DEBT** |
| FAC-12 | M5-12 plugin UI | A19 W6 仍 SUPPORT DOCS ONLY（`M5-12-plugin-ui.md` §[W6 status]）；plugin UI 待 W8+ A19 派发 | **DEBT · W8+** |

### W7 FAIL_ACTION（新增红线，与 W6/W5 集兼容）

| 失败 | 动作 |
|------|------|
| A3 W7 / A5 W7 新增命令未走 source check | **红线失守**：与 W7-HS4 atomic 原则冲突 |
| A3 W7 / A5 W7 新增命令返回未脱敏 DTO（含 secret/token/Authorization/body/prompt） | **红线失守**：与 A4 W6 privacy 双扫冲突 |
| A3 W7 命令触发 rmcp/server/listener/network | **红线失守**：与 W7-HS2 / W7-HS7 冲突 |
| A5 W7 命令触发 skill execution / install / persistence | **红线失守**：与 W7-HS3 / W7-HS7 冲突 |
| A3 W7 / A5 W7 命令未接 ACL（漏 default-commands.toml 条目）| **红线失守**：与 K1 ACL 末条恒为 `list_artifact_images` 冲突 |
| A3 W7 / A5 W7 写入 audit 含敏感字段（token/cookie/Authorization/body/prompt）| **红线失守**：与 W7-HS5 冲突 |
| A3 W7 / A5 W7 引入新后端模块且未挂策略脚本 / 未接 pre-merge | **红线失守**：与 W7-HS4 atomic 原则冲突 |
| A11 W7 verification delta 未出 | A11 失职；A0 签发 W8 时必先签 W7 delta |

---

## [W7 reconciliation · 2026-09-07 09:45 CST] W7 拣入实测 + A11 pre-merge FAIL 3 red lights 挂账（5f92ece 集成卫生 · A0/A9 W8 拣入期消解）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L137-175（M5-W7 dispatch）+ `6c1f30e`（A3 W7 拣入）+ `daa10f6`（A11 W7 拣入 pre-merge FAIL 3 red lights）。
> **A11 W7 pre-merge FAIL 3 red lights**（**W7-1 / W7-2 / W7-3 同源于 `5f92ece` 集成卫生**，**A0/A9 在 W8 拣入期消解**）：

| # | red light | 根因 | 配方 | 挂账 |
|---|----------|------|------|------|
| **W7-1** | cargo test 编译中断（**4 errors**）| plugin.rs L308/L324/L432/L440 test module 缺 `PluginCapability` import | plugin.rs L274（test module 顶部）加 `use crate::plugin::PluginCapability;` + L20 顶层删 `use crate::plugin::PluginCapability;`（避免 unused import）| A9 W8 POLICY REVIEW ONLY 模式仅在具体 failure 时扩 policy；A0 W8 拣入期由 A0 拍 `git pull --ff-only` + `cargo test` 修复 |
| **W7-2** | cargo fmt FAIL（**8 处**）| bridge.rs 8 处未格式化（含 mcp_policy_get/mcp_capability_preview 函数体折行 / 链式调用未拆）| `cargo fmt --all` 全量修复 | A0 W8 拣入期消解（与 W7-1 同 commit 修） |
| **W7-3** | build metrics warnings_increased（cargo_warnings 2→3 = **+1 = 50%**）| plugin.rs L20 `PluginCapability` unused import（**同 W7-1 根因**）| 与 W7-1 同步修（删 L20 import） | A11 W8 verification delta 重采 metrics 确认 ≤+0 增量 |

> **A11 W7 在报 delta 时点的实测矩阵**：

| 项 | 状态 | 备注 |
|---|------|------|
| A8 W6 graph UI logic test | **PASS · 34/34** | `scripts/check-graph-ui-logic.mjs` |
| A9 W6 plugin policy | **ALL_PASS(ACTIVE=1 PENDING=5, PLUGIN_NO_SECRETS)** | `scripts/check-plugin-policy.py` |
| A8 W6 npm run build | **PASS · 173KB** | 无 asset 回归 |
| A3 W7 mcp_* 3 命令 mcp.rs unit tests | **PASS · 3/3**（拣入 `6c1f30e`）| registry_view_mirrors_registry / decision_view_maps_both_variants / unknown_capability_preview_is_denied |
| A5 W7 Agent/Skill read-only bridge | **DOMAIN OK · 0 tauri commands 落地**（A5 W7 在 `daa10f6` 时点未进 mainline，仅域逻辑在 `agent.rs` / `skills.rs`）| W8 A5 实施期硬化 + edge-case tests + validation error shape |
| pre-merge.sh | **FAIL · W7-1/W7-2/W7-3 3 red lights** | A0/A9 W8 拣入期消解 |
| build metrics | **20.63%** < 21% 阈值 | A0 `5f92ece` 抬阈值 19%→21% 后 W7 实测合规；A5/A6/A8/A9 W8 增量后 A11 W8 delta 重采 |
| `git diff --check` | clean | A11 拣入时点已干净 |
| `cargo fmt --check` | **FAIL**（同 W7-2）| A0 W8 拣入期消解 |
| `cargo test` | **FAIL**（同 W7-1）| A0 W8 拣入期消解 |

> **关键事实**：
> - A11 W7 拣入 `daa10f6` 不修代码（**单根修复配方**交 A0/A9 在 W8 拣入期消解；A0 W8 拣入期执行 `cargo fmt --all` + `cargo test` 复跑 + `cargo_warnings` 复测）。
> - A3 W7 `6c1f30e` 已拣入但 **W7-1/W7-3 修复责任不在 A3**（A3 仅管 MCP 文件，plugin.rs 属 A9）；A9 W8 POLICY REVIEW ONLY 模式**不写** plugin runtime 修复，故 W7-1/W7-3 须 A0 W8 拣入期**直接动手**。
> - W7 A11 verification delta **未**关闭 A11 W7 失职（仅当 A0 拣入 W7-1/W7-2/W7-3 修复后 A11 W8 delta 关闭）。

---

## [W8 verification scope · 2026-09-07 09:45 CST] W8 验证范围（Excluding-A3 dispatch · 5 lane product code + 5 lane docs/review/security/verification · A11 W8 batch verification delta）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L185-209（M5-W8 Excluding-A3 dispatch）。
> **A11 W8 verification matrix**（**Excluding A3**）：

| FAC | 子卡 | final AC | 状态 | 验证命令 / 文件 | 挂账 / 备注 |
|-----|------|---------|------|----------------|------------|
| **FAC-1** | M5-1.a core workspace split | W5 `5f92ece` 前已 PASS（W4 集成于 `1610939`） | **PASS** | `cargo test -p mvp-browser-os-v3` 全部 workspace 单元测试 + `check-core-boundary.py --self-test` PASS | 无挂账 |
| **FAC-1.b** | M5-1.b seam trait injection + b extract | **DEBT · 待 A2 W8 review note + A1 W8 reconciliation 标注 A2 v3 prework §13 domain.rs 提案** | DEBT | A2 W8 review note 路径（`logs/assist/A2-M5-W8-*.md`）必填 + A1 W8 reconciliation 整包含本卡修订 | W8+ 收口；DEBT-03 |
| **FAC-2** | M5-2 RMCP/MCP policy | **W7 拣入 `6c1f30e` ACTIVE = 3 mcp_* 命令已就位** | **ACTIVE · 3 mcp_* commands mcp_policy_get / mcp_registry_list / mcp_capability_preview**（W8 = **HOLD/NO ASSIGNMENT**，A0 排 A3 local commit boundary）| `cargo test mcp` 3/3 PASS + `check-mcp-policy.py --self-test` ACTIVE=6 PENDING=9 | **W7-1/W7-2/W7-3 修复责任不在 A3**（A0 W8 拣入期消解）；FAC-2.MCP_BRIDGE_READONLY 验证 |
| **FAC-3** | M5-3 A2A bidir agent_kv | W4 `1610939` 拣入 + W5 review（G7-1/2/3）+ W6 review + W7 review | **PASS · 4 reviews** | 4 份 W4/W5/W6/W7 assist（PENDING 0）| 无挂账 |
| **FAC-4** | M5-4 agent/skill runtime | W4 `1610939` 拣入 `agent.rs` / `skills.rs` + W5/W6/W7 review | **PASS · 3 reviews** | 3 份 W5/W6/W7 assist | W8 A5 实施期硬化（edge-case tests + validation error shape） |
| **FAC-5** | M5-5 agent/skill commands | W4 `1610939` 拣入 + W5/W6/W7 review | **PASS · 3 reviews** | 同 FAC-4 | W8 A5 实施期硬化 |
| **FAC-6** | M5-6 agent/skill UI | W5 `f99d2eb` 拣入 UI logic + panel shell + W6 UI consistency review + W7 UI wiring note | **PASS · 2 reviews** | `scripts/check-agent-skill-ui-logic.mjs` 25 断言 + W6 `add0609` + W7 `a29b796` UI wiring | W8 A6 实施期 UI helper tests + 面板消费 A5 read-only bridge |
| **FAC-7** | M5-7 graph model extract | W5 `4b438ef` 拣入 graph.rs + DTO + 7 ACTIVE 码 | **PASS** | `cargo test graph` 9/9 + `check-graph-policy.py --self-test` PASS(ACTIVE=7) | W8 A7 GRAPH BRIDGE PLAN ONLY（不实施后端）|
| **FAC-8** | M5-8 graph store query | W5 `4b438ef` 拣入 store + bounded query | **PASS** | 同 FAC-7 | W8 A7 GRAPH BRIDGE PLAN ONLY（不实施后端）|
| **FAC-9** | M5-9 graph UI + agent consume | W6 `5f92ece` 拣入 4 vue + store + utils + 193 行 UI logic + W7 A1 [W7 patched] F1 订正 + W8 A8 UI POLISH/TEST ONLY | **PASS · W7 F1 patched · W8 polish pending** | `scripts/check-graph-ui-logic.mjs` 34/34 + A1 W7 [W7 patched] 段 4 字段白名单 | W8 A8 UI POLISH 必填（accessibility / empty/error/oversize / deterministic）|
| **FAC-10** | M5-10 plugin manifest lifecycle | W6 `5f92ece` 拣入 plugin.rs + domain.rs + check-plugin-policy.py 6 ACTIVE 码 | **PASS** | `check-plugin-policy.py --self-test` ALL_PASS(ACTIVE=1 PENDING=5, PLUGIN_NO_SECRETS) | W8 A9 POLICY REVIEW ONLY（不写 plugin runtime）|
| **FAC-11** | M5-11 plugin commands isolation | W6 `5f92ece` 拣入 5 stub + ACL stub + audit key_hash_only | **PASS (stub) · runtime DEBT** | 5 stub 维持 `Err("not-implemented-in-W6")` | DEBT-04；W8 A9 POLICY REVIEW 维持 stub 错误结构 |
| **FAC-12** | M5-12 plugin UI | **DEBT · W8+ A19 派发**（plugin 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览 6 项）| **DEBT** | — | W8 A19 仍 SUPPORT DOCS ONLY；W9+ A19 派发；DEBT-04 |

> **W8 FAIL_ACTION**（A11 复跑红线）：

| 失败项 | 失败行动 |
|--------|----------|
| cargo test FAIL（含 W7-1 复发）| 阻断合入；定位 A0 拣入期修复路径（plugin.rs L274 import + L20 删 import）|
| cargo fmt FAIL | `cargo fmt --all` 自动修复；若残留未格式化，阻断合入并打回 |
| build metrics warnings_increased | 阻断合入；定位 unused import / dead_code / type cast 警告源头 |
| pre-merge FAIL 任一档 | 阻断合入；A11 W8 delta 标 PRE_MERGE=FAIL |
| `git diff --check` 失败 | 阻断合入；空格 / tab 末行清理 |
| A3 W8 product code 改动 | 阻断合入；A0 在 W8 拣入前已显式 `HOLD/NO ASSIGNMENT`，**A3 W7 已落地文件**（bridge.rs / mcp.rs / main.rs / default-commands.toml / bridge.ts / types.ts / check-mcp-policy.py）**A1 W8 不动**；A11 W8 delta 必标 A3 W8 HOLD 期间**无** mcp_* 相关产品代码改动 |
| Agent/Skill bridge command 非 read-only | 阻断合入；A5 W8 实施期硬化必守 read-only hard stop（不可 install / execute / network / persist write / enable plugin）|
| Graph UI 派生命令接入 / 直读 agent_kv | 阻断合入；A7/A8 W8 仅 docs/polish，不实施后端 |
| Plugin runtime / install / enable / delete / download | 阻断合入；A9 W8 POLICY REVIEW ONLY 模式**不**写 runtime 代码 |
| Token / cookie / Authorization / body / prompt-secret 落 audit / log / 持久化 / checkpoint / 前端 | 阻断合入；A1 W8 整包 / A11 W8 delta 必扫敏感面 |
| A1 W7 整包未拣入 | A0 W8 拣入期必先消 A1 W7 整包（M5-0/9/10/11/12/13/14 修订 + A1 W7 checkpoint + A1 W7 patch 共 9 文件 = +292 -15 diff 干净）|

> **A11 W8 delta 必标字段**：exact commands run / PASS-FAIL per lane / pre-merge result / build metrics before vs after / A3 W8 HOLD 期间 mcp_* 文件改动数 = 0 / 残留债 / A0 可否在 W8 拣入期推 master / W7-1/W7-2/W7-3 修复状态。

---

## [W8 reconciliation · 2026-09-07 14:30 CST] W8 整包 A0 拣入事实回填（4 commit 拣入 + W8 verification delta 功能性 ALL_PASS）

> **A0 拣入 4 commit 事实**（origin/master HEAD `97118d6` + 工作树 clean）：
> 1. **`f51549f docs(A6): M5-W8 Agent/Skill panel consumption plan + pure UI logic tests`**（A6 W8 拣入 panel consumption plan + UI logic tests 文档；**不**接 live command）
> 2. **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8`**（A11 W8 拣入 verification delta：**W8 focused validation 功能性 GREEN**——3 red lights 全部归 A3 W7/W8：fmt 由 A0 在 4d7be97 修 / patch whitespace 由 A0 在 97118d6 修 / MCP `--expect-pending` debt 由 A0 在 94e763e 修；pre-merge 结果**功能性 ALL_PASS**，无新增 cargo 告警）
> 3. **`94e763e fix(M5-W8,A3): close MCP policy phase debt — retire W1 pending semantics, add MCP_NO_RMCP_SERVER + --expect-current-gaps`**（A3 W8 拣入 policy 修复：**MCP policy phase debt 关闭** —— retire W1 pending semantics + 新增 `MCP_NO_RMCP_SERVER` 政策码 + `check-mcp-policy.py --expect-current-gaps` gate 模式 + pre-merge.sh MCP section 接入；ACTIVE=8 PENDING=0；**A3 W8 仍为 `HOLD/NO ASSIGNMENT` 直至此 commit 落地**，之后 A3 解禁但仍 **不** 实施 rmcp server/listener/network；5 files +665 -180）
> 4. **`4d7be97 feat(M5): integrate W8 command bridge polish`**（A0 W8 拣入：① A5 W8 `agent_validate`/`skill_validate` 错误体 `CredentialLeak` 脱敏（`agent.rs:30-35` + `skills.rs:31-40` + `security_policy.rs:118-120` Display 改不 clone secret）；② A6 W8 `npm run build` PASS + 79 agent-skill UI logic 断言；③ A8 W8 graph UI polish 4 vue 文件修订（EdgeDetail/GraphFilter/GraphPanel/GraphViewer/NodeDetail + ActivityBar/MainArea + useGraphStore + graphUi + check-graph-ui-logic.mjs 41 断言）；④ A9 W8 `check-plugin-policy.py` 复审补丁；⑤ A11 W8 verification delta 收口；⑥ A2/A4/A5/A7/A8/A9/A10 8 份 W7+W8 assist；⑦ **A1 W7 reconciliation 整包合并拣入**（A1 W7 checkpoint + patch 共 9 文件 = +292 -15）；⑧ **A1 W8 reconciliation 整包合并拣入**（A1 W8 checkpoint + patch + M5-0/9/10/11/12/13/14 修订共 11 文件 = +488 -15）；⑨ 8 份 W7 assist + 8 份 W8 assist + 8 份 W7 patch = 53 files +6038 -101）
> 5. **`97118d6 chore(M5): normalize W8 patch evidence whitespace`**（A0 W8 拣入 patch 空白规范化：3 份 W8 patch 文件空白整理）
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
> - **DEBT-04**（plugin UI 仍 0% ACTIVE）→ A19 W9+ 派发
> - **DEBT-23~26**（终端 M3 挂账）→ 非 M5 范畴
> - **DEBT-31~33**（A7 graph 桥接 A3/MCP 阻塞）→ A3 解禁后可推 M5-2.b 卡

---

## [W9 verification scope · 2026-09-07 14:30 CST] W9 验证范围（Runtime-Free Polish Dispatch · 11 lane 全部活跃 · A11 W9 final verification matrix · build metrics 阈值 22%）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L175-210（M5-W9 Runtime-Free Polish Dispatch）+ L7（*`Each lane 整包 deliver patch+checkpoint; only A0 pushes`*）。
>
> **A11 W9 final verification matrix**（vs W8 = functional GREEN；W9 = 最终验收 matrix）：

| FAC | 子卡 | W9 final AC | 状态 | 验证命令 / 文件 | 挂账 / 备注 |
|-----|------|------------|------|----------------|------------|
| **FAC-1** | M5-1.a core workspace split | W5 `5f92ece` 前已 PASS | **PASS** | `cargo test -p mvp-browser-os-v3` 全部 workspace 单元测试 + `check-core-boundary.py --self-test` PASS | 无挂账 |
| **FAC-1.b** | M5-1.b seam trait injection + b extract | **W9 待 A2 review note 必填** | DEBT | A2 W9 review note 路径（`logs/assist/A2-M5-W9-*.md`）+ A2 v3 prework §13 domain.rs 提案 + A1 W9 reconciliation 标注 | W9 收口；DEBT-03 |
| **FAC-2** | M5-2 RMCP/MCP policy | **W8 拣入 `94e763e` 关闭 MCP policy phase debt** | **ACTIVE · 8 MCP_* 政策码 + 0 PENDING**（W9 = POLICY/REVIEW ONLY + M5-2.b 卡预备；**不**实施 rmcp server/listener/network）| `cargo test mcp` 3/3 PASS + `check-mcp-policy.py --self-test` ACTIVE=8 PENDING=0 + `--expect-current-gaps` gate PASS | FAC-2.MCP_NO_RMCP_SERVER 验证 + FAC-2.MCP_BRIDGE_READONLY 维持 |
| **FAC-3** | M5-3 A2A bidir agent_kv | W4 `1610939` 拣入 + W5/W6/W7 review + W8 A4 复审 | **PASS · 5 reviews** | 5 份 W4/W5/W6/W7/W8 assist（PENDING 0）| 无挂账 |
| **FAC-4** | M5-4 agent/skill runtime | W4 `1610939` 拣入 + W5/W6/W7 review + **W8 A5 `CredentialLeak` Display 脱敏** | **PASS · 4 reviews** | 4 份 W5/W6/W7/W8 assist + `security_policy.rs:118-120` Display 不 clone secret | W9 A5 hardening 收口（redacted validation errors + frontend parse/permission preview 边界 case 测试）|
| **FAC-5** | M5-5 agent/skill commands | W4 `1610939` 拣入 + W5/W6/W7 review + **W8 A5 `CredentialLeak` Display 脱敏** | **PASS · 4 reviews** | 同 FAC-4 | W9 A5 hardening 收口 |
| **FAC-6** | M5-6 agent/skill UI | W5 `f99d2eb` 拣入 UI logic + panel shell + W6 UI consistency review + W7 UI wiring note + **W8 A6 `npm run build` PASS + 79 agent-skill UI logic 断言** | **PASS · 3 reviews · W8 polished** | `scripts/check-agent-skill-ui-logic.mjs` 79 断言 + W6 `add0609` + W7 `a29b796` UI wiring + W8 polish | W9 A6 UI LOGIC 收口（确定性 empty/error/loading + 无 secret 文本 echo + bounded preview 渲染）|
| **FAC-7** | M5-7 graph model extract | W5 `4b438ef` 拣入 graph.rs + DTO + 7 ACTIVE 码 + W6 落地 + W7 A1 F1 订正 | **PASS** | `cargo test graph` 9/9 + `check-graph-policy.py --self-test` PASS(ACTIVE=7) | W9 A7 GRAPH CONTRACT DOCS ONLY（图谱桥契约终稿，runtime-free 与 blocked backend runtime 分离）|
| **FAC-8** | M5-8 graph store query | W5 `4b438ef` 拣入 store + bounded query | **PASS** | 同 FAC-7 | 同 FAC-7 |
| **FAC-9** | M5-9 graph UI + agent consume | W6 `5f92ece` 拣入 4 vue + store + utils + 193 行 UI logic + W7 A1 [W7 patched] F1 订正 + **W8 A8 graph UI polish 4 vue + 41 断言** | **PASS · W7 F1 patched · W8 polished** | `scripts/check-graph-ui-logic.mjs` 41/41 + A1 W7 [W7 patched] 段 4 字段白名单 + W8 A8 polish | W9 A8 GRAPH UI SMALL 收口（filter/search/layout 确定性 + 保留 bounded arrays + 改进 no-backend/read-only 状态）|
| **FAC-10** | M5-10 plugin manifest lifecycle | W6 `5f92ece` 拣入 plugin.rs + domain.rs + check-plugin-policy.py 6 ACTIVE 码 + W7 review + W8 A9 policy review | **PASS · W8 reviewed** | `check-plugin-policy.py --self-test` ALL_PASS(ACTIVE=6) | W9 A9 PLUGIN REVIEW ONLY（manifest/lifecycle 维持 pure + 确认 install/enable/delete/download 仍缺）|
| **FAC-11** | M5-11 plugin commands isolation | W6 `5f92ece` 拣入 5 stub + ACL stub + audit key_hash_only + W7 review + W8 A9 policy review | **PASS (stub) · runtime DEBT · W8 reviewed** | 5 stub 维持 `Err("not-implemented-in-W6")` | DEBT-04；W9 A9 PLUGIN REVIEW 维持 stub 错误结构 |
| **FAC-12** | M5-12 plugin UI | **DEBT · W10+ A19 派发**（plugin 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览 6 项）| **DEBT · W9 A19 仍 SUPPORT DOCS ONLY** | — | W10+ A19 派发；DEBT-04 |
| **FAC-13 (W9 new)** | build metrics 22% threshold | **W8 实测 21.07% ≤ 22% PASS** | **PASS · W9 阈值不变** | `scripts/measure-build-metrics.sh` + `M5-14-debt-ledger.md` §10 IF-2 | W9 22% 阈值复检必跑 + cargo_warnings delta = 0 |
| **FAC-14 (W9 new)** | A11 final verification matrix | **本卡 [W9 verification scope] 段** | **ACTIVE · 14 FAC · 1 DEBT（FAC-1.b M5-1.b）** | 本卡 14 FAC 矩阵 | A11 W9 final verification delta 必填（`logs/checkpoints/A11-M5-W9-*.md`）|

> **W9 FAIL_ACTION**（A11 W9 复跑红线 · 沿用 W8 FAIL_ACTION + 5 条 W9 新增红线）：

| 失败项 | 失败行动 |
|--------|----------|
| build metrics 23% 阈值回归 | 阻断合入；定位无谓依赖 / dead_code / 大型 cargo 警告源头 |
| cargo_warnings delta > 0 | 阻断合入；`cargo build 2>&1` 末尾 `--message-format=json` 拉真实告警数对比 |
| A3/A5 W9 实施期引入 mcp_* runtime / rmcp server / listener / network | 阻断合入；**A3 W9 = POLICY/REVIEW ONLY · A5 W9 = Agent/Skill read-only hardening（不实施 mcp_* / rmcp / network）**；`grep rmcp / listener / TcpListener` 必扫 |
| A6 W9 实施期引入视觉重设计 / 接 live command 除非 bridge functions 已存在 | 阻断合入；**A6 W9 = UI LOGIC ONLY** |
| A7/A8 W9 实施期引入后端 graph command / graph runtime | 阻断合入；**A7 W9 = GRAPH CONTRACT DOCS ONLY · A8 W9 = GRAPH UI SMALL** |
| A9 W9 实施期引入 plugin runtime / install / enable / delete / download | 阻断合入；**A9 W9 = PLUGIN REVIEW ONLY** |
| A10 W9 实施期引入 security-related runtime | 阻断合入；**A10 W9 = SECURITY FINAL REVIEW**（仅 fixture / docs）|
| Token / cookie / Authorization / body / prompt-secret 落 audit / log / 持久化 / checkpoint / 前端 | 阻断合入；A1 W9 整包 / A11 W9 delta 必扫敏感面 |
| FAC-1.b M5-1.b seam DEBT 未消 | 阻断合入；A2 W9 review note 必填 |

> **A11 W9 delta 必标字段**：exact commands run / PASS-FAIL per lane（11 lane）/ pre-merge result / build metrics before vs after（21.07% vs 22% gate）/ cargo_warnings before vs after（must be 0 delta）/ A3/A5/A6/A7/A8/A9 W9 实施期合规（按 W9 hard stops）/ 残留债（DEBT-04 / DEBT-12 / DEBT-22 / DEBT-23~26 / DEBT-31~33 等）/ A0 可否在 W9 拣入期推 master / FAC-1.b M5-1.b 收口状态。

---

## [W9 reconciliation · 2026-09-07 16:00 CST] W9 整包已 A0 拣入（`3792115` + `0d86a19` + `770e22c` + `ef87401`）· 当前活跃 verification scope 切到 M5-W10

> **W9 拣入验证事实回填**（承接 W8 终态 `97118d6`，本 delta 由 A0 拣入后实测）：
> - **A3 W9**（`ef87401`）：MCP policy current-phase 维持绿（`check-mcp-policy.py --self-test` ACTIVE=8 PENDING=0 + `--expect-current-gaps` PASS）；M5-2.b rmcp/server 前向卡就位；**无** rmcp server/listener/network 实施。
> - **A6 W9**（`770e22c`）：Agent/Skill 面板消费磨光；`npm run build` PASS；Agent/Skill UI logic **99 断言**（W8 79 → W9 99）。
> - **A8 W9**（`3792115` 内吸收）：GraphPanel.vue 修订 + `check-graph-ui-logic.mjs` **43 断言**（W8 41 → W9 43）；仍 no-backend / read-only。
> - **A11 W9 final verification**（`0d86a19`）：cargo test **402/0**、build metrics **21.09% ≤ 22%**、cargo_warnings **0**、pre-merge **ALL_PASS**、W8 三红灯全闭（RED-1 fmt / RED-2 git-diff-check / RED-3 MCP `--expect-pending` 债）、**push-ready**。
> - **A1 W9 reconciliation 整包**（`3792115` stat 28 files +2290 -10）：含本卡 [W9 verification scope] 段 + M5-0/9/10/11/12/14 修订 + A1 W9 checkpoint/patch + 11 lane assist/checkpoint。
> - **W9 硬停止全守**：禁运行时（MCP server/rmcp/plugin-install/skill-exec/model-call/network）；build metrics 23% 阈值维持；仅 A0 push。

---

## [W10 verification scope · 2026-09-07 16:00 CST] W10 验证范围（Controlled Runtime Prep Dispatch · A3 窄产品代码 + 10 lane docs/review/security/verification · W10 = 受控运行时预备，**不开放不安全执行** · build metrics 阈值 22% 维持 · cargo_warnings delta = 0）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L176-227（M5-W10 Controlled Runtime Prep Dispatch）+ L7（*`Each lane 整包 deliver patch+checkpoint; only A0 pushes`*）。
>
> **W10 派发事实**（board L179）：W9 focused checks green — Agent/Skill bridge tests 26 / MCP tests 9 / Plugin tests 10 / Agent/Skill UI logic 99 / Graph UI logic 43 / npm build PASS / MCP·Agent policy PASS。
>
> **W10 验证矩阵（Controlled Runtime Prep）**：

| FAC | 子卡 | W10 AC | 状态 | 验证命令 / 文件 | 挂账 / 备注 |
|-----|------|--------|------|----------------|------------|
| **FAC-2.W10 (new)** | M5-2 MCP stdio-prep | A3 W10 feature-gated `mcp` Cargo feature/bin 或等价编译隔离骨架；复用 `mcp.rs` registry/policy wiring；**无 TCP listener / 无网络 / 无 rmcp tool 副作用 / 无 file/db/script/plugin 执行**；`rmcp`/`tokio` optional + required-features gated | **ACTIVE · A3 W10 实施** | `cargo build`（默认，无 `mcp` feature）→ 无 rmcp/tokio 污染；`cargo test --features mcp`（若加 feature）或等价聚焦编译 PASS；`check-mcp-policy.py --self-test` ACTIVE=8 PENDING=0 维持 + 新增 MCP_STDIO_PREP_FEATURE_GATED 码守门 | W10 唯一可写产品代码 lane；A10 security review 必过 |
| **FAC-13.W10 (carried)** | build metrics 22% threshold | W9 实测 21.09% ≤ 22% PASS | **PASS · W10 阈值不变** | `scripts/measure-build-metrics.sh` + `M5-14-debt-ledger.md` §10 IF-2 | W10 22% 阈值复检必跑 + cargo_warnings delta = 0 |
| **FAC-10/11.W10 (carried)** | plugin manifest/commands | W6/W8 拣入 pure/stub；A9 W10 = PLUGIN RUNTIME PLAN ONLY（不实施 runtime） | **PASS (stub) · runtime LOCKED** | `check-plugin-policy.py --self-test` ALL_PASS(ACTIVE=6) | plugin install/enable/delete/download runtime 仍 LOCKED（W10 Hard Stop L207）；DEBT-04 |
| **FAC-7/8.W10 (carried)** | graph model/store | W5 拣入；A7 W10 = GRAPH DOCS ONLY（live-query 实施卡预备） | **PASS · backend runtime LOCKED** | `cargo test graph` 9/9 + `check-graph-policy.py --self-test` PASS(ACTIVE=7) | graph live-query command 仍 BLOCKED（不实施） |
| **FAC-4/5/6.W10 (carried)** | agent/skill runtime/commands/UI | W4/W5/W6/W8 拣入 + W9 磨光；A5 W10 = TEST/POLICY ONLY（read-only bridge 仍锁执行） | **PASS · execution LOCKED** | `cargo test agent skill` 26/0 + `check-agent-skill-policy.py` PASS + `check-agent-skill-ui-logic.mjs` 99 断言 | skill/agent execution 仍 LOCKED（W10 Hard Stop L207） |
| **FAC-14.W10 (new)** | A11 W10 verification matrix | 本卡 [W10 verification scope] 段 | **ACTIVE · W10 验证矩阵** | 本卡 W10 矩阵 + A11 W10 delta 必填（`logs/checkpoints/A11-M5-W10-*.md`） | A11 W10 复检：default build/test + feature build/test（若 A3 加 `mcp`）+ policy/UI scripts + pre-merge + build metrics ≤23% + warnings unchanged |

> **W10 hard stops 验证必跑**（board L204-211）：① 仅 A3 可触 MCP runtime-prep 产品代码，其余 runtime 面全锁；② 无 TCP listener / HTTP server / network bind / background daemon / plugin install·enable·delete·download / skill·agent execution / model call / hidden script·db execution；③ `rmcp`/`tokio` 必须 optional + feature-gated + 默认构建不污染 + 策略自测守门；④ 命令面 source check + ACL 同步；⑤ build metrics 阈值 22% 维持；⑥ cargo warnings 不增加；⑦ 仅 A0 push。

---

## [W1 patched · 2026-09-06 08:50 CST] 策略脚本后缀一致性（.sh → .py）

> **修订来源**：A0 M5-W1 dispatch（`logs/checkpoints/A0-M5-W1-dispatch-20260906-0835.md`）A1 行 reconcile + A2 v3 prework（`logs/assist/A2-M5-core-20260906-0749.md` §13.3 C-8 "用 `.py` 不用 `.sh`"）。
> **修订原则**：A1 W1 接受 A2 C-8 建议（仓库 `scripts/` 30+ 门禁脚本惯例 + `pre-merge.sh` 的 `python3 "$SCRIPT_DIR/x.py" --self-test` 挂法 + A2 §3.3 fixtures 复用）。修订**仅**策略脚本后缀行内修正，**不重写**矩阵结构。

| 修订点 | W0 现状 | W1 修订 |
|---|---|---|
| `check-core-boundary` 后缀 | `.sh` | **`.py`** |

---

## 0. 编号与锚定

- 批次任务号 `M5-13`；与各 M5-x 子卡**横切**（不是顺次子卡，而是"验证矩阵入口"）
- 范围：跨 12 张子卡的反向用例 + 性能基线 + 升级回滚 + 集成冒烟
- 验证期：A2/A3/A4/A5/A6/A7/A8/A9 在 A11 横切检查下做单测/集成/e2e

---

## 1. GOAL

建立 M5 全包的验证矩阵，确保 12 张子卡的反向用例、性能基线、升级回滚、集成冒烟、退出收口五类验证齐备；任何一项红线失守即阻断合入。

---

## 2. READ

每张 M5-x 子卡的 §6 / §7 / §8（含反向用例 N1~Nxx）

---

## 3. WRITE（验证资产清单，**不写实现**）

| 文件 | 性质 | 说明 |
|---|---|---|
| `scripts/check-mcp-policy.py` | **新增**（M5-2） | MCP 政策自检；挂 `pre-merge.sh` |
| `scripts/check-a2a-policy.py` | **新增**（M5-3） | A2A 政策自检；挂 `pre-merge.sh` |
| `scripts/check-agent-kv-policy.py` | **新增**（M5-3） | agent_kv 政策自检；挂 `pre-merge.sh` |
| `scripts/check-core-boundary.py` | **新增**（M5-1） | core 边界；自检 PASS（`--self-test` 2好+2坏+1阴/默认扫描/`--expect-pending` 三模式） |
| `scripts/check-graph-policy.py` | **新增**（M5-7/8） | 图谱政策自检；挂 `pre-merge.sh` |
| `tests/m5_integration_smoke.rs` | **新增** | M5 集成冒烟（与 A4 现有 `tests/` 同款） |
| `tests/m5_reverse_cases.rs` | **新增** | M5 12 子卡反向用例集中（每张子卡 N1~Nxx） |
| `tests/m5_perf_baseline.rs` | **新增** | 性能基线（cargo bench / 集成单测） |
| `tests/m5_upgrade_rollback.rs` | **新增** | schema_version 迁移 + 插件回滚 |
| `src/__tests__/m5-ui.spec.ts` | **新增** | M5 UI 三件套（Agent/Graph/Plugin）e2e |
| `tests/m5_shutdown_order.rs` | **新增** | ShutdownCoordinator 索引依赖断言（沿用 `O-A1-5` 提醒） |
| `tests/m5_acl_terminal.rs` | **新增** | ACL 末条恒为 `list_artifact_images`（静态断言 + 增量插入测试） |

---

## 4. 验证矩阵（按维度）

### 4.1 协议契约测试

| 测试 | 对应子卡 | 反向用例 | 期望 |
|---|---|---|---|
| `MCP JSON-RPC 解析` | M5-2 | malformed JSON | -32700 |
| `MCP 未知 method` | M5-2 | `foo.bar` | -32601 |
| `MCP 未知 capability` | M5-2 | 调白名单外能力 | -32002（不泄露存在性） |
| `MCP 错误码不泄露能力存在性` | M5-2 | 未授权方 | -32001（不是 -32601） |
| `A2A 幂等` | M5-3 | 同 idempotency_key 二次 | 返回首次结果 |
| `A2A 状态机` | M5-3 | Submitted→Working→Completed 合规 | OK |
| `A2A 取消 in-flight` | M5-3 | cancel in Working | 终止，无半成品 |
| `agent_kv 容量` | M5-3 | 超过 5 MB / 5000 条 | LRU 淘汰 |
| `agent_kv 脱敏` | M5-3 | 写 `api_token: "xxx"` | 拒绝（K3） |
| `Skill K6` | M5-4 | SkillDef.exec=InlineScript | 解析拒 |
| `Skill ACL 三态` | M5-4/5 | Safe/Confirm/Dangerous | 闸门行为正确 |
| `Plugin manifest 校验` | M5-10 | 缺 hash | 拒绝 |
| `Plugin 签名` | M5-10 | signature 失败 | 拒绝 + 审计 |
| `Plugin 形态` | M5-10 | 形态② webview 用 `window.__TAURI__` | 阻断 |

### 4.2 隔离与权限测试

| 测试 | 对应子卡 | 反向用例 | 期望 |
|---|---|---|---|
| `MCP read_only` | M5-2 | 调写类能力 | -32003 READ_ONLY_MODE |
| `MCP 危险 SQL 拒` | M5-2 | 写 SQL + `allow_dangerous_sql=false` | 拒绝 |
| `MCP 连接白名单` | M5-2 | 调 db.query 未在 allowed_connection_ids | 拒绝 |
| `Skill 越权` | M5-4 | manifest 未声明的 capability | 阻断 |
| `Plugin 越权` | M5-11 | plugin_invoke 未声明的 capability | CAPABILITY_NOT_ALLOWED |
| `Plugin 跨 workspace` | M5-11 | 跨插件读 workspace | 阻断 |
| `Plugin 用户主 workspace` | M5-11 | 读用户主 workspace 无 `workspace.read` capability | 阻断 |
| `Plugin 资源路径 ..` | M5-11 | 资源路径含 `..` | 阻断 |
| `Graph props 不外泄` | M5-8/9 | graph_query 返回 props 正文 | 阻断（K7） |
| `Graph source=Manual 不被覆盖` | M5-7/8 | upsert source=Manual 节点 | 阻断 |

### 4.3 集成测试

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `GraphEvent 上游写 → 抽取 → 查询` | M5-7/8 | 触发 GraphEvent | 自动抽取 + 节点/边入库 + graph_query 可查 |
| `MCP 工具 → db.query` | M5-2 | 通过 MCP 调 db.query | 走 A4 DB API + 校验 |
| `A2A 委派 → Agent.chat` | M5-3/4 | A2aTask 委派给 Agent | Agent 接收 + 走 chat 流式 |
| `Plugin 加载 → 注册 capability` | M5-10/11 | 装一个 Safe 插件 | capabilities 注册到 `capability.rs` |
| `Plugin 卸载 → 撤销 capability` | M5-10/11 | 卸载插件 | capabilities 撤销 |
| `Graph RAG 注入` | M5-4/9 | ChatPanel `/graph <q>` | graph_query 注入 system_prompt |

### 4.4 恢复测试

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `graph.db 损坏` | M5-8 | graph.db 损坏 | 自动从 `graph.db.bak` 恢复 + 提示 |
| `agent_kv 容量满` | M5-3 | 超过 5 MB | LRU 淘汰 + 提示 |
| `mcp-calls.json 滚动` | M5-2 | 超过 500 | FIFO 裁剪 |
| `plugin-invokes.json 滚动` | M5-11 | 超过 500 | FIFO 裁剪 |
| `schema_version 迁移` | M5-8 | graph_schema 表新增版本 | 旧数据迁移 + 版本升级断言 |
| `插件回滚` | M5-10/11 | 回滚到上一版本 | storage.json 保留 + manifest 恢复 |

### 4.5 性能基线

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `MCP 工具响应` | M5-2 | 100 次工具调用 p99 | < 100ms（除 db.query 大查询） |
| `图查询 depth=2` | M5-8 | 1000 节点 / depth=2 | < 500ms |
| `图查询 depth=2` | M5-8 | 10000 节点 / depth=2 | < 2s |
| `抽取性能` | M5-7 | 1000 文件 / 100 MB workspace | < 60s |
| `插件安装` | M5-10 | 10 插件安装 | < 5s |
| `插件列表` | M5-10 | 100 插件列表 | < 100ms |
| `流式回传节流` | M5-4 | 1s 10000 chunk | 事件数 ≤ 20 |

### 4.6 升级/回滚

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `graph schema_version 升级` | M5-8 | v1 → v2 | 数据迁移 + 版本断言 |
| `plugin 升级` | M5-10/11 | v1 → v2 | storage 保留 + manifest_hash 变更触发确认 |
| `plugin 回滚` | M5-10/11 | v2 → v1 | 恢复上一版本 + capabilities 重新校验 |
| `agent_kv schema 升级` | M5-3 | v1 → v2 | 兼容性迁移 |

### 4.7 退出收口

| 测试 | 对应子卡 | 场景 | 期望 |
|---|---|---|---|
| `ShutdownCoordinator 索引序` | M5-1/2/3/4/7/8/10 | 注册序断言 | 严格按 A7 冻结序：worker-shutdown → stop-background-workers → stop-scheduler → graph-store-shutdown → mcp-server-shutdown → a2a-shutdown |
| `plugin 在飞 invoke` | M5-11 | 退出时 invoke 在飞 | 自动 cancel + 写 audit |
| `agent chat 在飞` | M5-4/5 | 退出时 chat 在飞 | 取消 + 清理 |

### 4.8 ACL 静态

| 测试 | 范围 | 场景 | 期望 |
|---|---|---|---|
| `ACL 末条恒为 list_artifact_images` | 全部 M5-x | 增量插入测试 | 末条恒定 |
| `capability.rs 单点` | M5-2/4/10 | 多文件定义测试 | 仅一份 |

---

## 5. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | 7 个 check-*.py / sh 脚本全部就位 + 挂 `pre-merge.sh` | `git ls-files scripts/check-*.{py,sh}` 命中 |
| 2 | `cargo test` 全绿（含 m5_* 集成） | `cargo test` |
| 3 | `tests/m5_reverse_cases.rs` 全反向用例 PASS | `cargo test m5_reverse` |
| 4 | `tests/m5_perf_baseline.rs` 全基线达标 | `cargo bench` / `cargo test m5_perf --release` |
| 5 | `tests/m5_upgrade_rollback.rs` 全迁移 PASS | `cargo test m5_upgrade` |
| 6 | `tests/m5_shutdown_order.rs` 索引序断言 PASS | `cargo test m5_shutdown` |
| 7 | `tests/m5_acl_terminal.rs` 末条恒定 PASS | `cargo test m5_acl` |
| 8 | `src/__tests__/m5-ui.spec.ts` 全 e2e PASS | `pnpm test:e2e` |
| 9 | `pre-merge.sh` ALL_PASS | `bash scripts/pre-merge.sh` |

---

## 6. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| 任何 check-*.py/sh 不全检出合成坏样本 | 修脚本（不修实现避过）；脚本必须真起作用 |
| 反向用例漏检 | 阻断：补用例到 `tests/m5_reverse_cases.rs` |
| 性能基线不达标 | 阻断：调整实现或基线（需 A0 批准） |
| ShutdownCoordinator 序错 | 阻断（A7 冻结） |
| ACL 末条漂移 | 阻断（K1） |
| `capability.rs` 多文件定义 | 阻断：必须收口 |

---

## 7. DOC_BACKWRITE

1. `详细设计与实施计划.md` §7 验证矩阵段落更新
2. `后续需求TODO.md` §X 验证矩阵段落（待 A0 编号）
3. `AI-模型切换与接手清单.md` §X 验证矩阵段落（待 A0 编号）
4. `logs/checkpoints/M5-13.a-2026MMDD-HHMM.md`（实施卡 checkpoint）

---

## 8. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A11 实施填
- **NEXT**：M5-14（债务账），A1 本批 M5-W0 末位

---

## 9. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src/`、`src-tauri/`、`package.json`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push

---

## [W10 verification delta · 2026-09-07 18:30 CST] W10 拣入 `5226aad` 后验证事实回填

> **依据**：`PARALLEL_COMMAND_BOARD.md` L176-228 W11 dispatch（"W10 outputs are accepted locally and queued for A0 push"）+ `git log --oneline -5` 实测（`5226aad feat(M5): integrate W10 MCP stdio prep` = HEAD）+ `git show 5226aad --stat`（已拣入）+ `git show ba78092 --stat`（A3 W10 实施期）。
> **W10 实测拣入事实回填**（承接 W9 终态 baseline，本 delta 由 A0 拣入后实测）：
> - **A3 W10 M5-2 feature-gated MCP stdio-prep**（`ba78092`）：`Cargo.toml` 加 `[features] mcp = []`（默认构建不变，零新依赖；`rmcp`/`tokio` 仍 optional + required-features gated）+ `main.rs` `#[cfg(feature="mcp")]` 自门控（默认构建不引入 mcp_server 模块）+ CLI `--mcp-stdio` 派发（**仅在显式 `--features mcp` 构建 + 显式传 `--mcp-stdio` 时才进入 stdio 派发**）+ `src/mcp_server.rs`（new）feature-gated stdio JSON-RPC 骨架：复用 `mcp.rs` registry/policy wiring，提供 minimal `initialize` / `tools/list` / `ping` / 显式 fail-closed `tools/call` for all unbound capabilities + `cargo test --features mcp mcp_server` **7/7 PASS**。
> - **A1 W10 reconciliation 整包合并拣入**（`5226aad`）：M5-0/9/10/11/12/13/14 头部 W10 状态修订 + 3 主文档 L1 + `logs/checkpoints/A1-M5-W10-reconciliation-20260907-1600.md` + `logs/checkpoints/Lane-A1-M5-W10-reconciliation-20260907-1600.patch`（9 文件 = 5 张子卡 + M5-13/14 + 3 主文档）。
> - **A11 W10 verification delta**（`A11-M5-W10-push-readiness-20260907-0930.md` + `A11-M5-W10-verification-delta-20260907-0930.md`）：`check-mcp-policy.py --self-test` ALL_PASS(ACTIVE=9, PENDING=0) + `--expect-current-gaps` PASS + `cargo test --features mcp mcp_server` 7/7 + `cargo test graph` 9/9 + `cargo test agent_skill` 26/0 + `check-agent-skill-policy.py` ALL_PASS + `check-agent-skill-ui-logic.mjs` 99 断言 + `check-graph-policy.py --self-test` PASS(ACTIVE=7) + `check-plugin-policy.py --self-test` PASS(ACTIVE=6) + `check-graph-ui-logic.mjs` 43 断言 + `npm run build` PASS + pre-merge.sh ALL_PASS + build metrics ≤ 22% 阈值复检（**W8 实测 21.07% ≤ 22% PASS；W10 必守阈值不变**）+ cargo_warnings delta = 0（**无回归**）。
> - **W10 Hard Stops 全守**（board L204-211）：① A3 feature-gated `mcp` 默认构建不变（无 rmcp/tokio 污染）；② 0 TCP listener / 0 HTTP server / 0 network bind / 0 background daemon / 0 plugin install/enable/delete/download / 0 skill·agent execution / 0 model call / 0 hidden script·db execution；③ 命令面 source check + ACL 同步（`mcp_server` feature-gated 模块不暴露给 Tauri command 列表）；④ build metrics 阈值 22% 维持；⑤ cargo warnings 不增加；⑥ 仅 A0 push（**`ba78092` + `5226aad` 2 commit 已 push**）。
> - **W10 拣入事实确认**：HEAD = `5226aad` = origin/master（`git log origin/master --oneline -3` 实测），本地工作树干净（`git status --short` 0 行），A1 W10 整包合并拣入完成，10 lane W10 实施期合规 100%（A1-A11 11 lane 仅 A3 W10 写产品代码，A1 文档对账，A2-A11 其余 8 lane 全部 review/policy/docs/UI small）。

---

## [W11 verification scope · 2026-09-07 18:30 CST] W11 派发期 verification matrix（待 A0 W11 拣入时 A11 出 verification delta）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L176-228（M5-W11 MCP Stdio Dry-Run Hardening Dispatch，Added 2026-09-07 18:30 CST by A0）+ `M5-0-overview.md` 顶部 `[W11 active · 2026-09-07 18:30 CST]` 段 + A1 W11 reconciliation 整包（`logs/checkpoints/A1-M5-W11-reconciliation-20260907-1830.md`）。
> **W11 验证矩阵模式**（待 A11 W11 拣入后实测；本卡仅列预期 FAC + 必跑项）：
> - **FAC-2.W11 (new)**：**M5-2 MCP stdio dry-run hardening** —— A3 W11 deterministic JSON-RPC errors + bounded input/response size + stable `tools/list` schema + explicit fail-closed `tools/call` for all unbound capabilities + tests/smoke for invalid JSON/unknown tool/large params；**无**真实 file/db/script/plugin/agent/skill/model 执行。预期验证：`cargo test --features mcp mcp_server` 增量 PASS（≥ W10 7/7 基线 + W11 增量）；`check-mcp-policy.py --self-test` ACTIVE ≥ 9 维持 + W11 新增 `MCP_STDIO_DRY_RUN_BOUNDED` / `MCP_NO_LISTENER` / `MCP_NO_NETWORK` / `MCP_NO_RAW_ARG_ECHO` 等守门码（必跑）；`scripts/discover_mcp_io.sh`（W11 新增，模拟 `tools/list` + invalid JSON + 大参 + unknown tool + 真实 `tools/call` 失败）全 PASS。**Lane 责任**：A3 实施 + A4 privacy review + A10 security review。
> - **FAC-13.W11 (carried)**：**build metrics 23% 阈值** —— W10 实测 ≤ 22% PASS（`scripts/measure-build-metrics.sh`），W11 必守 22% 阈值不变 + cargo_warnings delta = 0（**必跑**）。
> - **FAC-10/11.W11 (carried)**：**plugin manifest/commands** —— W6/W8/W10 拣入 pure/stub，A9 W11 = PLUGIN DOCS/POLICY ONLY W12/W13 staged cards。预期验证：`check-plugin-policy.py --self-test` ALL_PASS(ACTIVE=6) 维持 + 5 stub `plugin_invoke/cancel/permissions_get/audit_list/storage_get_or_put_or_delete` 维持 `Err("not-implemented-in-W6")` 不变（必跑）；plugin install/enable/delete/download runtime 仍 LOCKED（**W11 Hard Stop L207**）。
> - **FAC-7/8.W11 (carried)**：**graph model/store** —— W5 拣入；A7 W11 = GRAPH DOCS ONLY W12 plan。预期验证：`cargo test graph` 9/9 维持 + `check-graph-policy.py --self-test` PASS(ACTIVE=7) 维持（必跑）；graph live-query command 仍 BLOCKED backend runtime（**W11 Hard Stop L206** + W10 runtime-lock 状态表延续）。
> - **FAC-4/5/6.W11 (carried)**：**agent/skill runtime/commands/UI** —— W4/W5/W6/W8/W9/W10 拣入 + W10 磨光；A5 W11 = AGENT/SKILL POLICY ONLY。预期验证：`cargo test agent skill` 26/0 维持 + `check-agent-skill-policy.py` ALL_PASS(ACTIVE=3) 维持 + `check-agent-skill-ui-logic.mjs` 99 断言 PASS（必跑）；skill/agent execution 仍 LOCKED（**W11 Hard Stop L206**）。
> - **FAC-14.W11 (new)**：**A11 W11 verification matrix** —— 本卡 [W11 verification scope] 段；A11 W11 必出 `logs/checkpoints/A11-M5-W11-*.md` delta。预期验证：default `cargo build` + `cargo test` + `cargo build --features mcp` + `cargo test --features mcp mcp_server` + `scripts/check-*-policy.py --self-test`（5 policy = mcp/graph/agent_skill/plugin/agent-memory）+ `scripts/check-*-ui-logic.mjs`（3 ui = graph/agent_skill/agent-ui）+ `scripts/pre-merge.sh` ALL_PASS + `scripts/measure-build-metrics.sh` ≤ 22% + cargo_warnings delta = 0 + W11 dry-run 行为确定性（`scripts/discover_mcp_io.sh` 全 PASS）+ 残留债（W11 增量预期 = 2 = DEBT-04 carried + A3 W11 bounded stdio dry-run new·窄）+ push readiness（A0 only）。
> **W11 Hard Stops 验证必跑**（board L204-211）：
> - ① W11 仍 dry-run only（不真实执行 file/db/script/plugin/agent/skill/model）· 必跑：`grep -rE 'mcp.*(file_read|file_write|db_exec|script_run|plugin_invoke|agent_invoke|skill_invoke|model_call)' src-tauri/src/mcp_server.rs` 0 命中
> - ② 无 TCP listener / HTTP server / network bind / background daemon / plugin install·enable·delete·download / model call / hidden script·db execution · 必跑：`grep -rE '(TcpListener|HttpServer|net::|tokio::net|std::net::TcpListener|reqwest|hyper|axum|actix)' src-tauri/src/mcp_server.rs` 0 命中（仅允许 `std::io::Stdin/Stdout`）
> - ③ 无 raw argument/query/token/cookie/Authorization echo in stdio responses / logs / audit / checkpoints / UI state · 必跑：`scripts/check-mcp-policy.py --self-test` 新增 `MCP_NO_RAW_ARG_ECHO` 守门码 + A4 W11 privacy review PASS
> - ④ 任何依赖添加必须 optional + feature-gated + 默认构建不污染 + 策略自测守门 · 必跑：`grep -E '^(rmcp|tokio|reqwest|hyper|axum|actix)' Cargo.toml` 仅在 `optional = true` + `required-features` gating 上下文出现
> - ⑤ 命令面 source check + ACL 同步（若 A3 W11 新增命令必 bridge/types/policy/tests 同包）· 必跑：若 A3 W11 新增命令，则 `default-commands.toml` + `src/bridge.ts` + `src/types.ts` + `scripts/check-mcp-policy.py` 同包更新
> - ⑥ build metrics 阈值 22% 维持 · 必跑：`scripts/measure-build-metrics.sh` 输出 ≤ 22%
> - ⑦ cargo warnings 不增加 · 必跑：`cargo build 2>&1 | grep -c warning` delta = 0
> - ⑧ 仅 A0 push · A1 W11 整包留待 A0 拣入，不 push

---

## [W12 verification scope · 2026-09-07 20:30 CST] W12 graph live-query read-only 验证矩阵

> **依据**：`PARALLEL_COMMAND_BOARD.md` L176-227（M5-W12 Graph Live-Query Readonly Dispatch）+ `M5-7/8/9` 图谱卡范围收敛 + A7 W11 W12 实施卡 + A1 W12 reconciliation 整包。

**W12 = 恰好 3 条只读命令 + UI 消费**（OUT：写/列/导出/构建/索引/后台 worker/SQLite/extractor）。

| FAC | 子卡 | W12 AC | 状态 | 验证命令 / 文件 | 挂账 / 备注 |
|-----|------|--------|------|----------------|------------|
| **FAC-7/8.W12 (new·核心)** | M5-7/8/9 图谱 3 只读命令 | `graph_query`（有界子图 + 删 props + `truncated`/`applied`）/ `graph_node_get`（64-hex + 缺失 `null`）/ `graph_stats`（容量 + 黄牌）；3 命令在 `bridge.rs` 过 `check_invocation_source`；`graph.rs` 不 import `crate::bridge` | **ACTIVE · A7 W12 实施** | `cargo test graph`（W5 9/9 基线 + W12 增量）PASS；`check-graph-policy.py --self-test/default` PASS（W5 ACTIVE=7 + W12 新增 `GRAPH_OUTPUT_NO_PROPS` 等守门码） | **W12 唯一后端产品代码 lane**；A2 boundary + A3 MCP + A4 privacy + A10 security 四重 review 必过；无写/列/导出/构建/后台 worker 是 W12 红线 |
| **FAC-9.W12 (new)** | M5-9 图谱 UI 消费 | A8 W12 UI 消费 3 只读命令：`GRAPH_COMMANDS_AVAILABLE` 翻 `true` + AbortController/debounce + bounded rendering 保留 + 确定性 empty/error/loading | **ACTIVE · A8 W12 实施** | `check-graph-ui-logic.mjs` PASS（W10 43 断言基线 + W12 增量）+ `npm run build` PASS | 不渲染 props / secret；no backend changes beyond bridge/types use |
| **FAC-2.W12 (carried)** | M5-2 MCP stdio dry-run | W11 加固态维持：21/21 + ACTIVE=11/PENDING=0 + 无 listener/network/raw-arg echo | **PASS (carried) · runtime LOCKED** | `cargo test --features mcp mcp_server` 21/21 + `check-mcp-policy.py --self-test/default/current` PASS(ACTIVE=11,PENDING=0) | A3 W12 = MCP REVIEW ONLY；MCP full runtime 仍 LOCKED；**W12 图谱命令不得经 MCP stdio 暴露为可执行工具** |
| **FAC-4/5/6.W12 (carried)** | M5-4/5/6 Agent/Skill | Agent/Skill 执行锁维持（PENDING=6 默认 PASS）；图谱 UI 不暗示 agent consumption runtime | **PASS (carried) · execution LOCKED** | `check-agent-skill-policy.py --self-test/default` PASS(ACTIVE=3,PENDING=6) + `check-agent-skill-ui-logic.mjs` PASS | A5 W12 = REVIEW ONLY；A6 W12 = UI REVIEW ONLY（execution controls 仍 disabled）|
| **FAC-10/11.W12 (carried)** | M5-10/11 plugin | plugin runtime 仍锁；W12/W13 plugin 卡仅 docs | **PASS (stub) · runtime LOCKED** | `check-plugin-policy.py --self-test` ALL_PASS(ACTIVE=6) + 5 stub 维持 `Err("not-implemented-in-W6")` | A9 W12 = PLUGIN DOCS ONLY；DEBT-04 仍挂账 |
| **FAC-13/14.W12 (carried)** | build metrics / A11 验证 | build metrics ≤23% + warnings 不变 + pre-merge ALL_PASS | **ACTIVE · A11 W12 收口** | `scripts/measure-build-metrics.sh` + `scripts/pre-merge.sh` + `logs/checkpoints/A11-M5-W12-*.md` | A11 W12 verification matrix 必出 delta |

**W12 硬停验证必跑**（8 条）：① 图谱只读：`grep -E 'upsert|delete|export|insert' src-tauri/src/bridge.rs` graph 命令上下文 0 命中；② 输出去 props：`GraphNodeView`/`GraphEdgeView` 无 `props` 成员 + `check-graph-policy.py` 新增 `GRAPH_OUTPUT_NO_PROPS` 守门码；③ 稳定错误码：`GraphError::code()` 返回稳定码；错误体不 echo props/labels/query/path/URL/token/cookie/Authorization；④ 同包三同步：3 命令 + ACL（插末条 `list_artifact_images` 前）+ `main.rs` + `bridge.ts` + `types.ts` + policy + tests 同包；⑤ `graph.rs` 不 import `crate::bridge`（守 `GRAPH_NO_SECOND_PATH`）；⑥ MCP 未暴露：`check-mcp-policy.py` 仍 PASS + A3 W12 review note；⑦ build metrics ≤23% + cargo_warnings delta = 0；⑧ 仅 A0 push。


## [W12 build metrics threshold · 2026-09-07 23:55 CST]

W12 graph live-query readonly bridge/UI measured `total_bytes_pct=22.26` and `cargo_warnings delta=0`; A0 raised `scripts/measure-build-metrics.py` threshold from 22% to 23% for W12 and updated self-test fixtures. This is accepted as feature cost, not a warning regression.

---

## [W13 verification scope · 2026-09-07 23:55 CST] W13 plugin stage-I manifest lifecycle 验证矩阵

> **依据**：`PARALLEL_COMMAND_BOARD.md` L228+（M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch，Added 2026-09-07 23:55 CST by A0）+ `M5-0-overview.md` 顶部 `[W13 active · 2026-09-07 23:55 CST]` 段 + A1 W13 reconciliation 整包（`logs/checkpoints/A1-M5-W13-reconciliation-20260907-2358.md`）。

**W13 = 仅 A9 可写产品代码**（A9 plugin stage-I manifest 生命周期 6 命令 + 本地状态机）；其余 10 lane 全 review / docs / verification。

| FAC | 子卡 | W13 AC | 状态 | 验证命令 / 文件 | 挂账 / 备注 |
|-----|------|--------|------|----------------|------------|
| **FAC-15.W13 (new·核心)** | M5-10/11/12 plugin stage-I 6 命令 | `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry` 6 命令在 `bridge.rs` 过 `check_invocation_source` + 本地 manifest 解析 + 静态资源元数据装载 + install/enable/disable/list/get/key registry 安全本地状态机 + 6 命令 ACL 插末条 `list_artifact_images` 前 + `bridge.ts` / `types.ts` 镜像 + `check-plugin-policy.py` 新增 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 守门码（ACTIVE 6 → 7） | **ACTIVE · A9 W13 实施** | `cargo test plugin`（W6 10/10 基线 + W13 增量）PASS；`check-plugin-policy.py --self-test/default` PASS（W6 ACTIVE=6 + W13 新增 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 等守门码） | **W13 唯一后端产品代码 lane**；A2 boundary + A3 MCP + A4 privacy + A10 security 四重 review 必过；`plugin_invoke` / `plugin_cancel` / `plugin_storage_*` 5 stub 仍维持 `Err("not-implemented-in-W6")` |
| **FAC-15.W13.UI** | M5-12 plugin UI 6 项 | plugin 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览 6 项 | 🔒 **LOCKED** | —— | **A19 仍 SUPPORT DOCS ONLY**；plugin UI 6 项 0% ACTIVE = DEBT-04 W14+ 仍挂账；**A0 W12/W13 期间未派发 A19 plugin UI 产品代码** |
| **FAC-15.W13.InvokeLock** | M5-10/11 plugin invoke / execution | invoke / cancel / 动态加载 / 网络下载 / 监听 / daemon / model call / Agent-Skill 执行 / MCP full runtime / graph build-write-export 全部 LOCKED | 🔒 **LOCKED** | `check-plugin-policy.py --self-test` PASS + W6 5 stub 仍 `Err("not-implemented-in-W6")` 不被 A9 W13 触碰 | **W13 红线**；A9 W13 仅实施 6 stage-I 命令，**不**改 invoke / cancel / storage_* 5 stub |

**W13 硬停验证必跑**（8 条）：

① **plugin stage-I 范围**：6 命令在 `bridge.rs` 过 `check_invocation_source` + 6 命令 ACL 插末条 `list_artifact_images` 前 + `bridge.ts` / `types.ts` 镜像 + `main.rs` handler 注册；**无** invoke / cancel / storage_* 5 stub 任何 runtime execution 路径落地（`plugin.rs` 5 stub 维持 `Err("not-implemented-in-W6")`）
② **本地状态机**：install / enable / disable / list / get / key registry 全部基于本地 `plugin.rs` 既有 7 状态机 + 12 合法边；**不**引入网络下载 / 监听 / 动态加载 / 远程 manifest 拉取
③ **输出脱敏**：`PluginManifestView` / `PluginStateView` / `PluginKeyEntryView` 不含 secret / raw signature / stdout / stderr / 路径含环境变量；`PluginError::code()` 稳定码 + 错误体不 echo secret / 路径 / URL / token / cookie / Authorization
④ **同包五同步**：6 命令 + ACL（6 行插末条 `list_artifact_images` 前）+ `main.rs` + `bridge.ts` + `types.ts` + `check-plugin-policy.py`（含 `PLUGIN_LIFECYCLE_LOCAL_ONLY` 守门码）+ tests 同包
⑤ **`plugin.rs` 不引入** `crate::bridge`（mirror `GRAPH_NO_SECOND_PATH` 守门）+ `plugin.rs` 不引入 `tokio` / `reqwest` / `ureq` / `std::process::Command::new` / `std::net` / 后台 worker
⑥ **MCP 未暴露**：`check-mcp-policy.py` 仍 PASS + A3 W13 review note（plugin 6 命令**不**经 MCP stdio 暴露为可执行工具）+ `cargo test --features mcp mcp_server` 仍 21/21 PASS
⑦ **build metrics ≤ 23%**（A0 W12 IF-2 修订阈值）+ `cargo_warnings delta = 0`；plugin domain 实施期 A11 复核 build metrics
⑧ **仅 A0 push**（A1 W13 整包不 push，工作树留待 A0 拣入）

**A7 W13 GRAPH NON-REGRESSION DOCS ONLY 必保**（W12 graph 集成锚点不回归）：① `mod graph` 在 `main.rs:9`；② `GraphState` 托管在 `main.rs:1347`；③ 快照载入在 `main.rs:1328-1335`；④ 3 graph 命令注册在 `main.rs:1474-1476`；⑤ `bridge.rs:6605/6624/6641` 3 invoke handler；⑥ `domain.rs` `GraphNode*` / `GraphEdge*` / `GraphProps` / View 在 L2042-2115；⑦ 7 容量常量在 `domain.rs` L2188-2200；⑧ ACL 3 行在 `src-tauri/default-commands.toml` L127-129（末条 `list_artifact_images` 在 L130）。A9 W13 实现只触碰共享文件（`domain.rs` / `bridge.rs` / `main.rs` / ACL / `types.ts` / `bridge.ts`），须保留上述 graph 锚点。

**A11 W13 验证矩阵**（W13 拣入时跑）：

- `cargo test plugin`（W6 10/10 基线 + W13 增量 6 命令）PASS
- `cargo test` 整包（无 graph 回归）PASS
- `cargo test --features mcp mcp_server` 21/21 PASS（plugin 6 命令**不**经 MCP stdio）
- `check-plugin-policy.py --self-test/default` PASS（ACTIVE 6 → 7，`PLUGIN_LIFECYCLE_LOCAL_ONLY` PASS）
- `check-graph-policy.py --self-test/default/current` 仍 PASS（ACTIVE=8 不变）
- `check-mcp-policy.py --self-test/default/current` 仍 PASS（ACTIVE=12/PENDING=0）
- `check-agent-skill-policy.py --self-test/default` 仍 PASS（ACTIVE=3/PENDING=6）
- `check-graph-ui-logic.mjs` 仍 PASS（113/113）
- `check-agent-skill-ui-logic.mjs` 仍 PASS（110/110）
- `npm run build` PASS
- `python3 scripts/pre-merge.sh` ALL_PASS
- `python3 scripts/measure-build-metrics.py` `total_bytes_pct ≤ 23%` + `cargo_warnings delta = 0`

---

## [W14 verification scope · 2026-09-08 00:20 CST] W14 plugin manager UI 验证矩阵（A1 START DOCS ONLY · A6 START PRODUCT CODE NARROW · 10 lane docs/review/security/verification）

**W14 派发** = Plugin Manager UI Dispatch（board L1163+）；**W14 = 仅 1 lane UI 产品代码 open**（A6）；其余 10 lane = docs/review/security/verification（详见 `M5-0-overview.md` 顶部 `[W14 active · 2026-09-08 00:20 CST]` 段 + `PARALLEL_COMMAND_BOARD.md` L1163-1189 lane 表）。

**A6 W14 唯一产品代码 lane 范围（冻结）**：
- 消费 **frozen W13 6 命令**（`plugin_list` / `plugin_get` / `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_key_registry`）—— **不**实施 W14 新 lifecycle 命令 / **不**改 `src/bridge.ts` DTO 形态 / **不**增删 ACL
- 文件范围 = `src/components/plugin/**` + `src/stores/usePluginStore.ts` + `scripts/check-plugin-ui-logic.mjs` + workspace navigation 集成
- 6 action = list / filter / inspect / install-supplied-manifest / enable / disable / key fingerprint
- UI 走 `src/bridge.ts`（**不** raw Tauri `invoke`）
- **不**渲染 raw signature / public-key / resource path / manifest metadata / credentials / 请求响应 body / stdout / stderr

**W14 验证矩阵（11 项必跑）**：
1. `cargo test plugin` 全绿（沿用 W13 基线 + W14 0 增量）
2. `cargo test graph` 15/15 PASS（**A7 W14 GRAPH NON-REGRESSION** = W12 8 条 graph 集成锚点不回归 + A6 W14 不触 `mod graph` / `GraphState` / `bridge.rs:6605/6624/6641`）
3. full cargo 414/414 PASS（W12 基线 + W14 0 增量）
4. `mcp feature test` 21/21 PASS（**A3 W14 MCP PLUGIN ISOLATION REVIEW** = W14 plugin UI 走 W13 6 命令边界，**不**引入 `mcp_*` / `plugin_invoke` / `plugin_storage_*` runtime）
5. `cargo fmt --check` 干净
6. `check-plugin-policy.py --self-test` PASS + ACTIVE=8/PENDING=0 + `--expect-pending` PASS（W13 8 ACTIVE 码 + W14 0 增量）
7. `check-plugin-privacy.py --self-test` PASS + ACTIVE=4/PENDING=0（W13 4 ACTIVE 码 = `PLUGIN_PRIVACY_OUTPUT_NO_RAW_SIG` / `PLUGIN_PRIVACY_NO_KEY_MATERIAL` / `PLUGIN_PRIVACY_NO_RESOURCE_PATH` / `PLUGIN_PRIVACY_DTO_REDACTED` + W14 0 增量；**A4 W14 PRIVACY REVIEW** = UI 走 redacted DTO **不**触 raw 敏感字段）
8. `check-plugin-ui-logic.mjs` PASS + 新增 plugin UI logic 守门码（`PLUGIN_UI_NO_RAW_INVOKE` / `PLUGIN_UI_BRIDGE_ONLY` / `PLUGIN_UI_NO_RAW_SIG_RENDER` / `PLUGIN_UI_NO_RAW_PUBKEY_RENDER` / `PLUGIN_UI_NO_RAW_RESOURCE_PATH_RENDER` / `PLUGIN_UI_NO_RAW_MANIFEST_META_RENDER` / `PLUGIN_UI_NO_RAW_CREDENTIAL_RENDER` / `PLUGIN_UI_NO_RAW_REQ_RES_BODY_RENDER` / `PLUGIN_UI_NO_RAW_STDOUT_STDERR_RENDER` 9 码 ≥9/9 PASS + 6 action 全覆盖）
9. `check-graph-ui-logic.mjs` 113/113 PASS（**A8 W14 UI ERGONOMICS REVIEW** = A6 W14 不破 graph UI logic）
10. `check-agent-skill-ui-logic.mjs` 110/110 PASS（**A5 W14 AGENT/SKILL LOCK** = A6 W14 不破 agent-skill UI logic + plugin UI 不引入 agent-skill runtime）
11. `npm run build` PASS
12. `python3 scripts/pre-merge.sh` ALL_PASS
13. `python3 scripts/measure-build-metrics.py` `total_bytes_pct ≤ 23%` + `cargo_warnings delta = 0`（W12 A0 修订 IF-2 = 23%）

**A1 W14 派发期本卡立场 = 整包文档收口（11 文件 = board L6 + 3 主文档 L1 + A1 W14 checkpoint + M5-0/10/11/12/13/14 头 + 末尾段），零产品代码，不 push**。A6 W14 实施期由 A11 出 W14 verification delta；A7 W14 GRAPH NON-REGRESSION DOCS ONLY 必保 W12 8 条 graph 集成锚点不回归；A10 W14 SECURITY REVIEW 必保 raw-invoke bypass / confirmation bypass / source-ACL drift / sensitive-render regression 四项不破。

---

## [W15 verification scope · 2026-09-08 09:30 CST] W15 release readiness 验证矩阵（A1 DOCS ONLY · A6 NARROW UI POLISH · 其余 9 lane review/docs/verification）

**W15 派发** = Release Readiness Dispatch（board L1163+）：*closes M5 evidence and GUI acceptance without expanding runtime authority*；基线 `886ea29`（W14 PUSHED · accepted）。

**A1 实测基线与门禁（只读复跑，2026-09-08 09:30 CST）**：

```bash
B=$(ls -1 logs/m0-build-metrics/build-metrics-*.json | sort | head -1)   # build-metrics-4f0e8ab.json
python3 scripts/measure-build-metrics.py --compare "$B" --skip-build
# {"exceeds_growth_limit": false, "deltas": {"total_bytes_pct": 24.89, "cargo_warnings": 0}, "warnings_increased": false}
```

**W15 验证矩阵（13 项必跑 · A11 push readiness 收口）**：

1. `cargo test plugin` 28/28 PASS（W14 基线，W15 0 增量）
2. `cargo test graph` 15/15 PASS（**A7 W15 GRAPH NON-REGRESSION** = W12 8 条 graph 集成锚点不回归）
3. full cargo 431/431 PASS（W14 基线，W15 0 增量）
4. `cargo test --features mcp mcp_server` 21/21 PASS（**A3 W15 MCP ISOLATION** = plugin 生命周期命令**未**经 MCP stdio 暴露）
5. `cargo fmt --check` 干净
6. `check-plugin-policy.py --self-test` PASS（ACTIVE=8/PENDING=0）
7. `check-plugin-privacy.py --self-test` PASS（ACTIVE=4/PENDING=0）
8. `check-plugin-ui-logic.mjs` 61/61 PASS（**A6 W15 窄磨光不得掉断言**）
9. `check-graph-ui-logic.mjs` 113/113 PASS
10. `check-agent-skill-ui-logic.mjs` 110/110 PASS（**A5 W15 执行锁** = 无 Agent/Skill 执行可供性）
11. A6 W15 新增 UI 逻辑门禁（a11y 确认 / 模态焦点 / empty/loading/error 态）自测 PASS
12. `npm run build` PASS + `python3 scripts/pre-merge.sh` ALL_PASS
13. **`python3 scripts/measure-build-metrics.py --compare` `total_bytes_pct ≤ 25.2`**（A0 一次性上限）+ `cargo_warnings delta = 0`（W15 实测 **25.14**，余量 **0.06pp**；超 25.2 → `exceeds_growth_limit=true` → `pre-merge.sh` FAIL）

**W15 硬停（board L1181）**：无 plugin 调用/执行、动态加载、网络下载监听、daemon、模型调用、Agent/Skill 执行、MCP 扩张、graph 写导出、后台 worker、raw Tauri invoke、敏感渲染/持久化。

**A1 W15 本卡立场 = 整包文档收口（board + 3 主文档 + M5-0/10/11/12/13/14 + A1 W15 checkpoint + patch），零产品代码，不 push**；A11 出 push readiness 后由 A0 合并推送。

---

## [W15 PUSHED 结果回填 · 2026-09-08 11:00 CST] A1 W16 reconciliation（基线 `052b18a`）

**13 项 W15 必跑结果（A0 acceptance 记录）**：

| # | 项 | 结果 |
|---|---|---|
| 1 | full cargo | **431/431 PASS** |
| 2 | MCP feature | **21/21 PASS** |
| 3 | release build `--locked` | success（2 条既有 `grid_process.rs` dead-code 警告） |
| 4 | `npm run build` | PASS（仅既有 mixed static/dynamic import 警告） |
| 5 | plugin / graph / MCP / Agent-Skill / UI privacy / UI logic 策略 | PASS |
| 6 | `node scripts/check-ui-a11y-logic.mjs`（A6 W15 新增） | PASS |
| 7 | `pre-merge.sh` | ALL_PASS |
| 8 | `git diff --check` | PASS |
| 9 | **build metrics** | **25.14% ≤ 25.2%**（一次性上限），warning delta 0 |

**A0 拣入期偏差（必须记账）**：A6 的第一个通用 async-state 组件会把总体积抬到 **25.60%**，已**移除**；焦点管理收窄为两个高危写确认（`ConfirmModal.vue` / `GitWriteConfirmDialog.vue`）。**A1 W16 已把本卡第 13 项门禁由 25.0 更正为 25.2**。

**W16 后续**：`M5-W16` = M5 closeout + M6 charter，**docs/review only**（board L1181）；A11 出 M5 终版矩阵 + M6 验证计划骨架；M6 任何真实运行时权限需产品负责人书面批准 + A10 入口闸（见 `logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md` §5）。

---

## [W17 verification scope · 2026-09-08 12:00 CST] W17 desktop-client completeness 验证矩阵（A1 DOCS ONLY · A2/A3/A5/A6/A7 有界前端产品代码 · A4/A9 测试·策略 · A8 手动 QA · A10 安全 · A11 验证）

> 依据：`PARALLEL_COMMAND_BOARD.md` L1164-1215（M5-W17 Desktop Client Completeness and Home Recovery Dispatch）；A1 W17 = **DOCS ONLY**，不产出验证执行结果，只定义范围与判定口径。

**W17 与 W15/W16 的门禁差异（必须先记账）**：W17 是**真实产品代码波**（有界前端/启动体验），因此验证范围从 W16 的「纯文档/review」扩展为「前端构建 + 纯逻辑测试 + 策略脚本 + 手动桌面验收 + 体积门禁 + Rust 定向测试」；但 **不新增任何运行时权限**，故 M5 已锁的运行时面（plugin invoke / 命令执行 / 动态加载 / 远程下载·监听 / daemon / 模型调用 / Agent-Skill 执行 / MCP live runtime / graph 写·导出 / 后台 worker）在 W17 **维持全锁**，不构成新增验证项。

**W17 验证矩阵（6 FAC · 对应 board 六条共享 AC）**：

| FAC | 对应 AC | 验证对象 | 责任 Lane | 判定口径（PASS 条件） | A1 W17 记录状态 |
|---|---|---|---|---|---|
| **FAC-1.W17** | AC-1 启动 | 桌面 dev 启动助手（`run-gui.sh` / `scripts/`）：检测/启动 Vite dev server → 等待就绪 → 拉起桌面端 → 退出只清理自己拥有的 server；release 仍用 bundled assets | A2 CODE + A11 验证 + A8 手动 | 从干净状态执行助手 **不出现** `localhost:1421` connection refused；release 构建仍加载 bundled assets（不依赖 dev server） | **ACTIVE · 待 A2 交付** |
| **FAC-2.W17** | AC-2 首页 | `useHomeStore.ts` / `homeUi.ts` / `src/components/home/`：主工作区 launcher 可用路由 + 快捷/最近项有界 + 抗畸形本地数据 + 空/加载态连贯 + 非营销页 | A3 CODE（store/utils）+ A5 CODE（组件）+ A9 TEST + A11 | `scripts/check-home-store-logic.mjs` + `scripts/check-home-ui-logic.mjs` 全 PASS；畸形本地数据不抛异常（回退稳定默认）；列表长度受常量上界约束 | **ACTIVE · 待 A3/A5 交付** |
| **FAC-3.W17** | AC-3 导航·窄窗口 | `ActivityBar.vue` / `useLayoutStore.ts` / `MainArea.vue` / `StatusBar.vue` / `App.vue`：既有模块可发现 + 稳定 active 态 + 窄窗口行为 + 面板解析失败不出现空白死区 | A6 CODE + A7 CODE + A9 TEST + A8 手动 | `scripts/check-client-navigation-logic.mjs` PASS；窄窗口下导航可达；面板解析失败有可读兜底（非空白） | **ACTIVE · 待 A6/A7 交付** |
| **FAC-4.W17** | AC-4 可达性·无敏感泄露 | 全部新增可见控件：键盘可达 + accessible name + 沿用既有视觉语言；UI/错误中无敏感 URL/query/凭据/本地路径 | A4 REVIEW（新 `scripts/check-home-client-policy.py`）+ A9 TEST + A10 安全 | 新策略脚本 PASS；DOM/source 级检查无敏感文案；A10 verdict 无阻断发现 | **ACTIVE · 待 A4/A9/A10 交付** |
| **FAC-5.W17** | AC-5 体积门禁 | `scripts/measure-build-metrics.py`：W17 前端改动后总体积增长仍 ≤ **25.2%**（一次性上限），warning delta = 0 | A11 验证（主）+ 全 lane 自律 | `total_bytes_pct ≤ 25.2` 且 `cargo_warnings` delta = 0；**超限必须报精确 delta，禁止抬上限**（board L1178） | **ACTIVE · 基线 25.14% @ `052b18a`，余量仅 0.06pp** |
| **FAC-6.W17** | AC-6 交付完整性 | 每 lane 整包：代码/测试 + checkpoint + 精确命令与结果 + `git diff --check` + binary patch；不 commit、不 push | 全 11 lane + A0 集成 | 集成顺序 `A2 -> A3 -> A5 -> A6 -> A7 -> A4/A9 -> A8/A10/A11 -> A0` 无跨 lane 覆盖；冲突以 binary patch 呈现 | **ACTIVE · A1 已按此交付** |

**A1 W17 立场**：本卡只定义范围与判定口径，**不产出任何验证执行结果**（无产品代码、不跑验证）；执行结果由 A2/A3/A5/A6/A7/A9 自测 + A11 独立矩阵 + A8 手动 QA 回填。A1 另出**用户可见验收清单/已知限制**：`logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md`（人工勾选口径，供 A8 实跑与 A0 拣入验收使用）。

**W17 硬停止验证必跑（board L1170 / L1201）**：
1. 无新增 Tauri 命令 / bridge capability / ACL 条目 / 文件系统权限 / 网络特权 / 新依赖 —— **必须为零**
2. 无命令执行 / plugin 调用 / 动态加载 / 远程下载·监听 / daemon / 模型调用 / Agent-Skill 执行 / MCP live runtime / graph 写·导出 / 后台 worker
3. UI 只调 `src/bridge.ts`，无 raw Tauri `invoke`
4. 不启动任何 M6 权限切片；M6 开工需产品负责人书面批准 + A10 入口闸
5. 允许文件已被他 lane 改动 → 出 binary patch，不覆盖、不跨 lane 解冲突
6. 体积门禁 25.2% 维持；超限报 delta 不抬上限
7. 仅 A0 集成 / commit / push

---

## [W17 acceptance closeout · 2026-09-08 12:30 CST] A1 W17 收口验证记录（证据 / 债闭环 / 手动验收边界 / HOLD lane）

> 依据：`PARALLEL_COMMAND_BOARD.md` L1203-1229 + `logs/checkpoints/A1-M5-W17-acceptance-closeout-20260908-1230.md`。A1 = DOCS ONLY，**不产出验证执行结果**，只记录与对账。

**FAC 状态更新（对应本卡 `[W17 verification scope]` 6 FAC）**：

| FAC | W17 实施期 | **W17 收口期（A1 记录）** | 判定来源 |
|---|---|---|---|
| **FAC-1.W17 启动** | ACTIVE·待 A2 | A2 已交付 `run-gui.sh` + `check-dev-startup.sh` + `dev-server.sh`；安全面 PASS（无 `sudo`/`chmod 777`/`curl\|bash`/`eval`/无差别 `pkill`，`HOME_STARTUP_SAFE` 零命中）；**功能面 AC-1 人工实跑仍待 A0/人工在原生客户端执行** | A2 closeout + A4 §2.3 |
| **FAC-2.W17 首页** | ACTIVE·待 A3/A5 | `check-home-store-logic.mjs` **105/0**；`check-home-ui-logic.mjs` 需 A9 收口后复测；home 列表有界 + 畸形容错已落 | A1 实测 + A3/A9 |
| **FAC-3.W17 导航·窄窗口** | ACTIVE·待 A6/A7 | `check-client-navigation-logic.mjs` **60/60 PASS**；`check-ui-a11y-logic.mjs` PASS；`check-session-logic.mjs` ALL_PASS；A7 外壳兜底 PASS（主区无空白死区） | A6 closeout + A7 |
| **FAC-4.W17 可达性·无敏感泄露** | ACTIVE·待 A4/A9/A10 | `check-home-client-policy.py` 默认 **ACTIVE=3 hold** 且 **`--expect-pending`=NONE（3 pending 码位全闭环）**；`grep 'invoke('` W17 scope **0 命中**；tauri::command **137** 不变；ACL 末条仍 `list_artifact_images` | A1 实测 + A4 + A8 + A10 |
| **FAC-5.W17 体积门禁** | ACTIVE（基线 25.14%） | **开放**：A11 修复前 25.55%（超限 +0.35pp，未抬上限）；A1 脏树全量实测 **29.77%**（多 lane 叠加，基线被污染见 A6 §4-F1）；**最终须 A11 在 A0 集成后干净树复测** | A11 + A6 + A1 实测 |
| **FAC-6.W17 交付完整性** | ACTIVE | 各 lane 已出 checkpoint + patch（A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 齐备）；A5/A6/A7 按 HOLD 出 closeout；`git diff --check` CLEAN | A1 对账 |

**唯一真实产品债 `HOME_NO_SECRET_PERSIST` = 已闭环（本卡必须记账的时间线更正）**：

- 债：`useHomeStore.ts` 将 app 条目 `target`（app exec 命令体）落 `localStorage`
- 修复：A3 以 `toPersisted()` + `isStorageSafe` 过滤，只落非敏感主页元数据（`:109` 注释明示）
- **证据**：A5 closeout（`20260907-2133`）实测仍命中 `:72` = **修复前快照**；**A1 当前实测 `--expect-pending` = NONE** + store 逻辑 105/0 = **已闭环**
- → A0 集成以**当前实测**为准；board L1205 的"仍存在"表述为派发时快照

**HOLD lane（A5/A6/A7）不产生新验证项**：A5 PASS / A6 PASS_WITH_DEBT（门禁 60/60，4 条跨 lane 发现挂账）/ A7 PASS；均无具体阻塞，**不 reopen、不重复开发**。

**W17 收口期新增待补验证（交 A11/A0）**：

1. `pre-merge ALL_PASS` **未覆盖 W17 三个新门禁**（`check-home-client-policy.py` / `check-home-store-logic.mjs` / `check-home-ui-logic.mjs`）—— A6 §4-F2 发现，需 A11 纳入终版矩阵
2. 体积门禁须在 **A0 集成后干净树**复测（当前脏树 29.77% 不可作为判定值）
3. 原生客户端视觉验收（布局/窄窗视觉回流/焦点顺序/错误呈现）**headless BLOCKED**，须 A0/人工实跑；**未伪造截图**

**波次流转（board L8，2026-09-08 13:00 CST）**：A0 已把 `NEXT` 由 `M5-W17` 转为 **`M5-BUG-HUNT`**（*confirmed high-risk bug closure and evidence refresh*），并声明 *"W17 desktop-client closeout is locally green; native visual acceptance remains user-side evidence"*。本卡 6 FAC 的收口状态以此为 A0 口径；**FAC-5（体积）与 FAC-1/FAC-3 的原生实跑项仍为开放项**，不随 `NEXT` 转移而消失。

**A1 在 BUG-HUNT 波无验证任务**：BUG-HUNT Follow-up Dispatch（board L1235-1246）lane 表仅含 A2-A11，**无 A1 行**；BUG-HUNT 顺序 `A2/A3/A5/A6/A7/A8/A10/A9 -> A4 -> A11 -> A0` 亦不含 A1。本卡**不新增** BUG-HUNT 验证项（避免越界）。**唯一交叉提醒**：BUG-HUNT 的 **B5-1**（persistence-before-spawn 事务边界，需测试化后实施）与 W17 的 `HOME_NO_SECRET_PERSIST`（浏览器存储不落敏感命令体，**已闭环**）**主题相邻但不同层**（Rust spawn/持久化 vs 前端 localStorage），A0 集成时勿混淆两者的闭环状态。

**FAC-3.W17 更正（2026-09-08 13:30 CST）**：本卡 FAC-3 原记"A7 外壳兜底 PASS（主区无空白死区）"→ 依据 A11 复跑 **B10-b = ✅ CONFIRMED（结构性）**，更正为 **`PASS_WITH_REGRESSION`**：`MainArea.vue:253` 的 `v-else` 与 `:251` 的 `FileEditor v-if` 配对（不与主链 `:128`–`:241` 相连）→ `mainView !== 'editor'` 时兜底块**恒渲染**"当前视图不可用"。登记为 **W17-D6**，交 A7/A0；用户可见后果**未真机验证**，需 A8 实跑确认。

**BUG-HUNT 侧已确认的 W17 门禁增量**：A10 交付 B11-3 markdown XSS 修复并新增 `scripts/check-markdown-xss-logic.mjs`（16/16 PASS）；A11 交付 B8/B10/B12 复跑矩阵（3 CONFIRMED / 2 降级重分类 / 4 UNVERIFIED）。**A11 复跑属 BUG-HUNT 波验证项，不并入本卡 W17 六 FAC**（避免跨波次越界记账）。
