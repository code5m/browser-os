# A10 · M5 Wave 0 安全复审（Security Review of M5 Wave 0 Proposals）

> Lane: `A10` — M5 security review（`AI:DEEP / R:xhigh`）
> 时间: 2026-09-06 15:20 CST（v3，含 A1/A2/A3/A4/A5/A6/A7/A8/A9/A11 全 lane M5-W0 提案完整复审）
> Base: `master` @ `5ca8f9f`（本地领先 `origin/master` 1，A0 已落 `docs(M5): dispatch architecture prework lanes`）
> 依据: `PARALLEL_COMMAND_BOARD.md` §M5 Dispatch Now → M5 Wave 0 Assignments → A10：START REVIEW
> 范围: **只读复核 + 审查评注**。**未改任何产品代码、未改任何策略脚本**（board 硬停止：M5-W0 全 lane 不得写产品代码/加依赖/加命令/ACL/面板）。
> 交付物: 本文件（`logs/assist/A10-M5-security-review-20260906-1410.md`）+ 前序 `A10-M4-security-review-batch-final-20260906-0704.md`（M4 已收口，仍有效）。
> 复用方法: 所有结论基于**提案实证 + 当前源码实查**（§0），非文档互证。

---

## 0. 方法声明与实证清单（Empirical Evidence）

本轮**亲自实查**当前工作树，校准提案"现状证据"的准确性（M5 为 DOCS ONLY，尚无产品代码可扫，故只验证提案所依赖的**当前状态断言**）：

| 检查项 | 命令/来源 | 结果 | 对复审的意义 |
|---|---|---|---|
| 全局 Tauri | `tauri.conf.json:12` | `withGlobalTauri: true` | 插件形态②可行性存疑，证据成立 |
| CSP | `tauri.conf.json:15` | `csp: null` | 无 CSP 硬化 |
| 全局 Tauri 引用点 | `grep -rn "__TAURI__" src` | **仅 1 处**（`useSystemStore.ts:84`） | A9/A1 称"依赖极薄可封"成立 |
| 默认能力窗口 | `capabilities/default.json` | `windows: ["main"]`（**非** 2026-09-02 旧卡所述 `["main","browser"]`） | 旧卡现状已过期 |
| 默认能力内容 | `capabilities/default.json` | 仅 `core:default`/`core:window:allow-create`/`browser-tabs:default`/`default-commands`，**无** `shell:allow-spawn` | 旧插件卡"高危暴露面"证据**已不属实** |
| 远程 webview | `capabilities/browser-remote.json` | `remote.urls=["https://*","http://*"]`，`permissions=["core:default","remote-collect"]` | A9 引用准确 |
| M5 命令前缀 | `grep -nE "db_\|task_\|graph_\|skill_\|plugin_\|a2a_\|mcp_\|agent_" default-commands.toml` | 命中 8 行 = M4 `db_*`×3 + `task_*`×5；`graph_/skill_/plugin_/a2a_/mcp_/agent_` = **0** | M5 命令尚未落地（符合 Wave 0） |
| ACL 末条 | `default-commands.toml:118` | `list_artifact_images`（坑位②） | M5 新命令须插其前，各提案遵守 |
| `Inline` 可执行变体 | `grep Inline src-tauri/src/domain.rs` | 仅 `InlineDataUrl`（图片枚举，无关） | 当前无 `SkillImpl::Inline` 代码执行变体 |
| M5 策略脚本 | `ls scripts/ \| grep -iE "mcp\|graph\|plugin\|agent\|a2a\|skill"` | **NONE** | M5 各实现 lane 须在落地时补策略脚本 + 挂 `pre-merge.sh` |

> 旧 2026-09-02 批次 taskcard（plugin/agent/graph/A2A）其**安全原则仍有效，但"现状证据"章节已与当前代码漂移**（如 `default.json` 窗口、`shell:allow-spawn` 已不存在）。实现期须以当前源码为准，不得照搬旧卡现状段。

---

## 1. M5 Wave 0 提案清单（权威来源 vs 冗余占位）

M5-W0 实际交付由 **A1 的权威 WBS 卡集**（`logs/checkpoints/M5-20260906/M5-0..M5-14`）统领，A2/A7/A9 的 prework 为子卡输入。

| 需求/WBS | 权威来源 | 其他 lane 同名 doc 状态 | A10 可审 |
|---|---|---|---|
| M5-1 core 下沉 | `M5-1-core-workspace-split.md` | `A2-M5-core-20260906-0749.md`（prework 输入）✅ | 可审 |
| M5-2 rmcp/MCP | `M5-2-rmcp-mcp-policy.md` | `A3-M5-mcp-20260906-0757.md` ⚠️**空文件** | 可审（以 A1 卡为准） |
| M5-3 A2A/agent_kv | `M5-3-a2a-bidir-agent-kv.md` | `A4-M5-a2a-memory-*.md` 缺失 | 可审（以 A1 卡为准） |
| M5-4/5/6 Agent/Skill | `M5-4/M5-5/M5-6-*.md`（部分未读全文，结构已见 M5-0 §2） | `A5-M5-agent-skill`/`A6-M5-agent-ui` ⚠️**空文件** | 可审（以 A1 卡为准） |
| M5-7/8 图模型/存储 | `M5-7/M5-8-*.md` | `A7-M5-graph-core-20260906-0755.md` ✅（权威契约，吸收 A9 越界草案） | 可审 |
| M5-9 图 UI/Agent 消费 | `M5-9-graph-ui-agent-consume.md` | `A8-M5-graph-ui` ⚠️**空文件** | 可审（以 A1 卡为准） |
| M5-10/11/12 插件 | `M5-10/M5-11/M5-12-*.md` | `A9-M5-plugin-system-prework-20260906-1100.md` ✅ + `A9-M5-plugin-form-feasibility`/`A9-M5-A13plus-cards` | 可审 |
| M5-13 验证矩阵 / M5-14 债务 | `M5-13-verification-matrix.md`/`M5-14-debt-ledger.md` | `M5-A11-verification-matrix-20260906-0820.md` ⚠️**空文件** | 可审（以 A1 卡为准） |

**关键发现（H-1 · 文档卫生，已解决）**：首轮复审时 `A3/A4/A5/A6/A8-M5-*` 与 `M5-A11-verification-matrix` 为**空文件**（违反 board 禁空文件规则）。本轮复核时这些文件**已全部补齐内容**（A3≈25KB / A4≈59KB / A5≈24.5KB / A6≈25KB / A8≈25KB / A11 验证矩阵+债务账+GUI 清单齐备），H-1 不再成立，A10 现以各 lane 专文为权威源复审。A11 另交付 `M5-A11-verification-matrix` / `M5-A11-debt-ledger` / `M5-A11-gui-manual-checklist` 三份，与 A1 `M5-13/M5-14` 卡对齐。

---

## 2. 四类绕过风险逐项裁定（board §M5 Wave 0 · A10 四问）

### C1. 重复/第二执行路径（duplicate / second execution path）

**裁定：PASS（设计层完全守住红线）✅**

- **插件形态①（独立进程/stdio）**：`A9-M5-plugin-system-prework` §1 与 `M5-0-overview` §7 红线① 一致**明确否决**；`PluginManifest` 枚举**不提供** `Process/NativeLib/Webview` 变体（`A9-M5-plugin-system-prework` §2.1）。
- **Skill `SkillImpl`**：`M5-0-overview` §7 红线⑫ `K6` 禁内联（仅 `ScriptRef`/`CommandRef`/`Sequence`），与 `domain.rs` 当前无 `Inline` 变体一致。
- **Agent/Skill 执行体**：A5→`M5-4/M5-5` 复用 M2-4 `run_script`。
- **A2A `ExternalCli`**：`M5-3` §4.2 强制 `走 script_runner`，**禁** `std::process::Command` 直起（单测 N9 拒绝）。
- **图谱抽取/重建**：`A7-M5-graph-core` §7 红线① `GRAPH_SECOND_EXEC_PATH` ≥ 纯函数/`script_runner`，禁 `std::process::Command`/`sh -c`。
- **图谱维护任务**：`A7-M5-graph-core` §4.4 执行体仅 `Script/Command`，禁新增 `TaskKind::Builtin`（承 A6 冻结枚举穷尽）。
- **MCP 传输**：`M5-2` §4.1 stdio-only，**禁 TCP 端口**（局域网暴露红线）。

### C2. 命令暴露无 ACL / 无 source check

**裁定：PASS（设计层合规，待实现期机检 parity）✅**

- 所有 M5 命令（`graph_*`/`skill_*`/`plugin_*`/`mcp_*`/`a2a_*`/`agent_kv_*`）均写明"插 `list_artifact_images` 之前"（坑位②）；§0 实查 ACL 末条确为 `list_artifact_images`。✅
- `M5-2` §4.3 / `A9-M5-plugin-system-prework` §3.2 / `M5-3` §4.4 均列"命令过 `check_invocation_source` + 进 ACL"。✅
- ⚠️ **待补（D-2）**：M5 各实现 lane 命令须三处 parity（Rust handler / `src/bridge.ts`+`src/types.ts` / ACL），与 M4 `db_*`/`task_*` 一致。A1 卡未强制 parity 自检脚本。建议 A0 在 M5 首卡要求加 `check-m5-command-parity.py`（或并入 `check-tools-policy.py`）。

### C3. 凭据泄漏（DTO / 审计 / 前端态 / 文件 / 日志 / Keyring）

**裁定：PASS（设计层有具体机检闸门；残留=实现期落地）✅**

- **DB 凭据（沿用 M4）**：`DbConnectionConfig` 无 password/dsn；Keyring `db:`；审计不含 SQL。M5 复用。✅
- **MCP（M5-2）**：`McpGlobalPolicy`（`M5-2` §4.3）`read_only` 默认 true、`allow_dangerous_sql` 默认 false、`allowed_connection_ids` 默认空；`detail` 禁记凭据/DSN/SQL 正文（§4.5）。✅
- **A2A / agent_kv（M5-3）**：`agent_kv` **严格分离**于 `workspace/`（§4.3 表）；**禁**存 token/密码（K3），写入前键名黑名单（复用 M2-3.a §4.4）；独立 `agent-kv-audit.json` 仅记 key hash。✅ **此设计直接封堵"AI 记忆层泄凭据"**。
- **Skill/Agent/插件 password 型参数**：走 Keyring 引用、审计 `***`。✅
- 🟢 **图谱凭据泄漏（首轮最高风险，已被 A7 具体化闸门）**：`A7-M5-graph-core` §6 定义 **`GRAPH_SECRET_IN_GRAPH`**（私密字段进 `props` 即违规）+ `GRAPH_NO_KEYRING_CRED` + `GRAPH_PRIVACY_BYPASS`；§3.2 `props` "隐私字段脱敏后入"；§5.5 `privacy=public` 永不返回私密节点；§9 `T-graph-priv1` 断言。→ 首轮 G-3 已由**设计层**解决，落 `check-graph-policy.py` 即机检（D-1）。
- **`withGlobalTauri=true` + `csp=null` 间接面**：form③（声明式，不加载第三方代码）下无第三方代码可滥用；form② 已被否决。`M5-0` §7 红线⑨ + `M5-14-debt-ledger` 将其列为 M5-1 评估期**封死** `useSystemStore.ts:84` 全局引用。→ 跟踪债务（D-3），非当前阻塞。

### C4. 不安全的插件/Skill 安装

**裁定：PASS（安装模型扎实；形态冲突已裁定）✅**

- **安装包安全**：`M5-15.a-prework` + `A9-M5-plugin-system-prework` §2.2 规定 zip-slip 防护、半损坏 `.corrupt` 备份不静默清空、原子写复用 `session.rs:30`。
- **默认禁用**：`A9-M5-plugin-system-prework` §2.4 / `M5-0` §7 红线⑧ `R-A6-1`：安装即 `Disabled`，需显式 `enable`。
- **fail-closed 能力判定**：`can_invoke` 7 步链（`A9-M5-plugin-system-prework` §3.2）：ACL 白名单→声明白名单→等级→fs_roots canonicalize 前缀→net_hosts 精确匹配（**不支持通配**）→危险确认识别→Allow；默认拒绝、声明≠授予、`manifest_hash` 变更授权失效。
- **能力白名单共用（防漂移）**：`M5-0` §7 红线③ A2P/A2A/Skill/Plugin/Agent **共用** `src-tauri/src/capability.rs`；`M5-2` §3 落 `capability.rs` 常量只此一份。
- 🟢 **形态结论冲突已解决（首轮 G-6）**：旧 `plugin-runtime-taskcard` 把形态①标"✅推荐"，`A9-M5-plugin-system-prework` §1 已 SUPERSEDED 旧卡 form① 假设，裁定"默认③、①否决、②作壳"；`M5-0` §7 红线①/⑨ 一致。A0 无需再裁决，仅需把旧卡措辞更新（建议 A0 集成时一并回填）。
- 🟢 **图谱越界处置（治理良好）**：`A7-M5-graph-core` §0.1 显式裁定 A9 两份图谱草稿"越界 M5-7/M5-8，已吸收为输入并裁定冲突，以本文为准"——避免了双源冲突，A10 支持此治理。

---

## 3. 分 Lane 复审要点（含 A1 权威卡）

- **A1（M5-0..M5-14 卡集）** ✅：**最完整、最权威**。§7 12 条红线把 M4 护栏（无第二路径/K3 凭据/K5 审计/ACL 末条/ShutdownCoordinator/enabled=false/atomic_write）**全量继承并落到每个 WBS 卡**；依赖图与合并顺序清晰。A10 以本卡集为复审基准。
- **A2（M5-1 core 下沉）** ✅：`crate 边界硬规则`（`A2-M5-core` §3.3）"mvp-core 禁 `use tauri`/`use crate::bridge`、解除 `scheduler→bridge::AppState` 反向边"直接把 M4 护栏转为**机器可守门**（`check-core-boundary.sh`）。R6 整文件带测试迁移。无安全倒退。
- **A3（M5-2 MCP/rmcp）** ✅（设计层严密，但 **B1/B2/B3 为开工前必须裁决的硬阻塞**）：stdio-only、纯 Rust `rmcp` 禁 npm、read-only 默认、`McpGlobalPolicy` fail-closed、能力黑名单（10+ 执行类能力首期不暴露）、`mcp_calls.json` 500 上限、无 TCP 监听、共享 `capability.rs`、`mcp-server-shutdown` 序不破 `stop-scheduler`。**A10 重点标记 3 项**：① **B1** `rmcp 3.2.0` 把 `tokio` 列为 normal 依赖，与 M4-1.a F-1「不引 tokio 直接依赖」冲突 → 须 A0 裁决（A3 建议独立 crate/bin 仅依赖 `mvp-core` 以保 F-1）；② **B3** MCP 请求无 `tauri::Webview`，`check_invocation_source` 无法直接套用 → 须增 MCP 专用来源通道，**禁止伪造 label 冒充 webview**（属 C2 绕过点）；③ **F-1（既有缺口）** `open_tool` 在 `main.rs:1411` handler + `bridge.ts:97` 调用但**不在 ACL**（§2.2），前端调用会被 Tauri 拒；若 MCP 机械扫描 handler 建工具清单将产生「前端被拒、MCP 却能调」的**权限倒挂** → A3 正确裁定工具真源须是能力白名单而非 handler 扫描，并建议补 handler↔ACL↔bridge 全量奇偶门禁（与本审查 D-2 一致）。
- **A4（M5-3 A2A/agent_kv）** ✅（极扎实，14 条冻结裁定 F-A4-1~14 + 6 条机器可检红线 N1~N6 + 25 条单测 + 两策略脚本码位表）：agent_kv 严格分离（`data_dir` 非 `workspace/`、不引 SQLite、单文件 JSON+`atomic_write`）、三层键空间白名单（防自建 `secrets` 桶）、**三重隐私闸**（键名黑名单 + 值扫描 + 出口不落审计/前端）、幂等键封顶、ExternalCli 走 `script_runner`、两段式确认闸门、`a2a-shutdown` 位置 idx2、`agent:` Keyring 与 `db:` 同级、能力真源唯一=`capability.rs`+`McpGlobalPolicy`。**A10 注意一处与 A1 卡的差异**：A1 卡 `agent_kv` 为扁平 `HashMap`（且其要求 LRU 淘汰但无 `updated_at`，内部自相矛盾）；A4 建议改三层嵌套（更严）。交 A0 裁决，两者均安全，A4 更严。
- **A5/A6（M5-4/5/6 Agent/Skill）** ✅：A5 冻结 AGSK_1~9 机器可守门（`SkillImpl` 无 `Inline`、无第二执行路径、`SkillImpl::Script` 复用 `script_runner`、危险安装/执行走 `request_*`/`confirm_*` 双阶段、凭据走 Keyring `llm:<provider>`、审计 `***`）；A6 UI 预研 `PASS_WITH_DEBT`（M5-6 实际 UI 实现仍 BLOCKED，上游 A5/A16 未冻结 DTO，符合预期），并定义 `check-agent-skill-ui-policy.py` / `check-agent-skill-ui-logic.mjs`（密码不进 store、危险须 ConfirmModal、流式须 Channel 非轮询、ACL 顺序）。与 M4 护栏与 A10 红线完全一致。
- **A7（M5-7/8 graph core）** ✅：`A7-M5-graph-core` 极扎实——邻接表复用 `rusqlite`（零新依赖）、稳定 id（禁 uuid，`GRAPH_STABLE_ID_UUID`）、容量上限（`GRAPH_MAX_*`）、depth≤2/limit≤1000、`GRAPH_SECRET_IN_GRAPH`/`GRAPH_NO_KEYRING_CRED`/`GRAPH_PRIVACY_BYPASS`/`GRAPH_AUDIT_LEAKS` 等 12 个机检码位、G-D1 裁定存储位置 A、G-D5 定义 `check-graph-policy.py` 码位。C3 最高风险在此被**设计层封堵**。
- **A8（M5-9 graph UI）** ✅：`backendReady=false` 只读壳（零 invoke，复用 M4-8 `guard()` 范式）、隐私过滤只在后端查询层（前端仅展示 `filtered_by_privacy` 计数）、容量/`truncated` 显式提示、子图注入只取+格式化+复制（不出网，真注入走 A4/A5）、力导向图依赖懒加载保体积（IF-2）。
- **A9（M5-10/11/12 插件 + 拆分）** ✅：最完整。`A9-M5-plugin-system-prework` 把 form③ 裁定、PluginManifest（无 Process/NativeLib/Webview 变体）、`can_invoke` 7 步 fail-closed 链（fs_roots canonicalize + net_hosts 精确匹配禁通配）、SB-1..SB-10 安全阻塞项（SB-1 全局 Tauri / SB-10 仅本地安装禁远程市场）、N1~N16 反向用例全列齐；与 A1 卡一致。**形态冲突已解决、能力白名单共用已钉死**。

---

## 4. M4 护栏跨 M5 保持检查表（Guardrail Preservation）

| M4 护栏 | M5 提案是否保持 | 证据 |
|---|---|---|
| DB 写默认拒 / 生产 fail-closed | ✅ | `McpGlobalPolicy.allow_dangerous_sql` 默认 false（M5-2 §4.3） |
| 凭据 Keyring + 不出日志/前端/文件/审计 | ✅（含新增图谱/agent_kv 脱敏） | `GRAPH_SECRET_IN_GRAPH`/`GRAPH_PRIVACY_BYPASS`（A7）、agent_kv 键名黑名单（M5-3）、K3（A1 §7） |
| SQL 结果行/字节上限 + 取消 | ✅ | graph 复用 `DB_MAX_*` 口径（A7 §1） |
| 调度复用 M2-4，无第二执行路径 | ✅ | C1 全项 |
| 停机经 `ShutdownCoordinator` | ✅ | `stop-scheduler` 索引1；`mcp-server-shutdown`/`a2a-shutdown`/`flush-graph`/`PluginShutdown`/`AgentShutdown` 均挂协调器 |
| 新命令过 source check + ACL | ✅（待机检） | C2 |
| 容量上限 + 审计低频（K5 1000） | ✅ | graph 5万/20万；mcp/plugin/skill/a2a/agent_kv 均拆独立 JSON（mcp 500 / agent_kv 1000 / plugin 独立） |
| `pre-merge.sh` 覆盖 | ⚠️ | **M5 策略脚本尚未建（D-1）** |

---

## 5. 残留债务与阻塞（Debt / Blockers for A0）

1. **D-1（高）**：M5 各实现 lane 的**策略脚本尚未建立**（`check-mcp-policy.py`/`check-graph-policy.py`/`check-plugin-policy.py`/`check-agent-policy.py`/`check-a2a-policy.py`/`check-skill-policy.py`）。码位已由 A7（`GRAPH_*`）、A9（反向用例）、A1（M5-2 §6/M5-3 §6）定义，**但未落地为文件也未挂 `pre-merge.sh`**。M4 护栏要求"每个产品码 lane 须加/扩策略脚本与测试"。→ **unblock = A14/A15/A16/A17/A18 实现期随代码落地脚本并挂 `pre-merge.sh`**（位置在 `git diff --check` 之前）。
2. **D-2（中）**：M5 命令三处 parity（Rust/`bridge.ts`/`types.ts`/ACL）需机器断言，防"注册了命令但漏桥/漏类型/漏 ACL"。→ unblock = 加 `check-m5-command-parity.py` 或并入 `check-tools-policy.py`。
3. **D-3（低·跟踪）**：`withGlobalTauri=true` + `csp=null` 间接面，由 M5-1 评估期**封死** `useSystemStore.ts:84` 全局引用（M5-0 §7 红线⑨ + M5-14 债务）。非当前阻塞。
4. **D-4（低·文档）**：2026-09-02 旧 taskcard（plugin-runtime/permission、agent-skill、graph-model、A2A）"现状证据"章节已过时（§0）；其**安全原则有效**但"形态①推荐"等措辞已被 A9 §1 SUPERSEDED。→ unblock = A0 集成时回填旧卡或标注作废，避免实现期误读。
5. **D-5（高·A0 裁决·A10 新增）**：`rmcp 3.2.0` 把 `tokio` 列为 **normal 依赖**，与 M4-1.a F-1「不引 tokio 直接依赖」冲突（A3 §3.3 B1）。若把 `tokio` 提为**主二进制直接依赖**会拓宽攻击面并破坏同步模型红线。→ unblock = A0 书面裁决，A3 建议解=**MCP 落独立 crate/bin，仅依赖 `mvp-core`+`rmcp`+`tokio`**，主二进制 `Cargo.toml` 依赖图零变化（F-1 原样成立）；或 A0 显式豁免并指定 `check-mcp-policy.py` 断言 `tokio::` 仅出现在 mcp 文件。
6. **D-6（高·A0 裁决·A10 新增，属 C2）**：MCP 请求**没有 `tauri::Webview`**，`check_invocation_source` 无法直接套用（A3 §2.4 B3）。要么伪造 label（红线），要么为 MCP 单列来源通道。→ unblock = A0 在 `security_policy.rs` 增一条 MCP 专用来源通道（新 `scope`+策略开关），**不得**把 `mcp` 塞进 `is_known_webview_label` 冒充 webview；M5-2.a 必须先把此通道定形，否则所有 `mcp_*` 命令都将绕过来源校验。
7. **D-7（中·C2，A10 新增）**：既有缺口 `open_tool` 在 `main.rs:1411` handler + `bridge.ts:97` 调用但**不在 ACL**（A3 §2.2 F-1），前端调用会被 Tauri 拒；更无 handler↔ACL↔`bridge.ts` 全量奇偶门禁（A3 §2.3 F-2）。风险：MCP 若机械扫描 handler 建工具清单将产生「前端被拒、MCP 却能调」的权限倒挂。→ unblock = ① A0/A11 决定补 ACL 或确认有意不下发并注释；② 补全量三处奇偶门禁（并入 D-2 的 `check-m5-command-parity.py`）；③ MCP 工具真源强制为能力白名单（非 handler 扫描，A3 已裁定）。
8. **H-1（低·卫生，已解决）**：首轮 `A3/A4/A5/A6/A8-M5-*` 与 `M5-A11-verification-matrix` 为空文件，本轮复核已全部补齐（§1），H-1 不再成立。

> 注：首轮 G-3（图谱抽取脱敏拦截点）与 G-6（插件形态冲突）**已在设计层解决**（A7 `GRAPH_SECRET_IN_GRAPH` + A9 form③ 裁定），不再列为阻塞，转为 D-1 的"机检落地"要求。

---

## 6. 裁定结论（Verdict）

| 类别 | 结论 |
|---|---|
| C1 第二执行路径 | ✅ PASS（form①否决、Skill 禁 Inline、ExternalCli/script_runner、graph 无 std::process、MCP stdio-only） |
| C2 命令暴露无 ACL | ✅ PASS（坑位② + source check；parity 机检待补 D-2） |
| C3 凭据泄漏 | ✅ PASS（K3 贯穿 graph/agent_kv/MCP/plugin；`GRAPH_SECRET_IN_GRAPH`/`GRAPH_PRIVACY_BYPASS`/agent_kv 分离/键名黑名单均设计层封堵；`withGlobalTauri` 间接面跟踪 D-3） |
| C4 不安全安装 | ✅ PASS（form③/默认禁用/fail-closed/can_invoke 7 步/能力共用/zip-slip 防护；形态冲突已裁定） |

**整体 STATUS = PASS_WITH_DEBT（设计层对四类绕过全覆盖，且多数已落到具体机检码位；硬门槛 = ① D-1 实现期补策略脚本并挂 pre-merge；② D-5/D-6 须 A0 在 M5-2.a 开工前书面裁决（rmcp→tokio vs F-1、MCP 来源通道）；③ D-7 补 `open_tool`/奇偶门禁；④ A4 vs A1 的 agent_kv 扁平/三层口径交 A0 裁决。其余 D-2/D-3/D-4 为跟踪项）**。

> 注：A3 的 B1/B2/B3 本质是**架构红线裁决项**，非「绕过漏洞」，但其未裁决会直接阻塞 M5-2 首张实现卡开工；A10 将其列为安全复审的**前置裁决依赖**，供 A0 在派发 M5-2.a 前收口。

M5 Wave 0 提案在**安全架构层面无倒退、且大幅加固了 M4 护栏的机器可守门性**（A1 12 红线继承 + A2 core 边界 + A7 `GRAPH_*` 码位 + A9 `can_invoke` + 共享 `capability.rs`）。A10 当前可对**全部 M5-W0 提案做安全放行（设计层）**；最终安全背书取决于 D-1 策略脚本随代码落地且 `pre-merge` ALL_PASS。

---

## 7. Lane Output Template

```
LANE=A10
STATUS=PASS_WITH_DEBT
BASE=5ca8f9f (master, ahead origin/master by 1)
HEAD=logs/assist/A10-M5-security-review-20260906-1410.md (+ A10-M4-security-review-batch-final-20260906-0704.md)
FILES=logs/assist/A10-M5-security-review-20260906-1410.md
VERIFY=
  tauri.conf.json:12 withGlobalTauri=true / :15 csp=null
  grep __TAURI__ src -> 1 (useSystemStore.ts:84, 可封)
  capabilities/default.json windows=["main"] (旧卡现状过期)
  ACL M5 前缀命中=8=M4 db_*/task_*; graph_/skill_/plugin_/a2a_/mcp_/agent_=0
  ACL 末条=list_artifact_images (坑位②)
  domain.rs Inline 仅 InlineDataUrl (无 SkillImpl::Inline)
  scripts/ M5 policy 脚本=NONE (D-1 待实现期补)
CHECKPOINT=logs/assist/A10-M5-security-review-20260906-1410.md
MERGE_NOTES=
  1. 以 A1 M5-0..M5-14 卡集为权威复审基准；A2/A7/A9 prework 为子卡输入，设计层一致且加固 M4 护栏。
  2. 四类绕过(C1-C4)设计层 PASS；形态①否决/③推荐已由 A9 §1 裁定，旧 plugin-runtime 卡"①推荐"措辞作废(D-4)。
  3. 图谱凭据泄漏首轮最高风险已被 A7 设计层封堵(GRAPH_SECRET_IN_GRAPH/PRIVACY_BYPASS)；agent_kv 严格分离+键名黑名单(M5-3)。
  4. 硬门槛 D-1：M5 策略脚本(check-mcp/graph/plugin/agent/a2a/skill-policy.py + UI logic 脚本)各 lane 已定义码位，但尚未建且未挂 pre-merge，须实现期随代码落地。
  5. D-5/D-6（A3 B1/B3，A0 裁决）：rmcp→tokio vs M4-1.a F-1「不引 tokio 直接依赖」冲突、MCP 无 webview 致 check_invocation_source 不可套用——须 A0 在 M5-2.a 开工前书面裁决（独立 crate/bin 保 F-1 / MCP 专用来源通道）。
  6. D-7（A3 F-1）：既有 open_tool 在 handler+bridge.ts 但不在 ACL，且无 handler↔ACL↔bridge 全量奇偶门禁；MCP 工具真源须强制能力白名单而非 handler 扫描，并补奇偶门禁。
  7. D-2 命令三处 parity 机检、D-3 封死 withGlobalTauri 全局引用(M5-1)、D-4 旧卡措辞回填、A4 vs A1 agent_kv 扁平/三层口径裁决 为跟踪项。
  8. H-1 已解决：A3/A4/A5/A6/A8-M5-* 与 A11 验证矩阵空文件已补齐内容，不再视为空白占位。
NEXT=A0 集成 M5 卡；派发 A13~A20 实现 lane 时强制 D-1(策略脚本+pre-merge)；A0 回填旧卡(D-4)/清理空文件(H-1)
```

---

## 8. 声明（避免误读）

- 本轮**零产品代码改动、零策略脚本改动**（M5-W0 硬停止禁止）。
- 未 rebase / 未 commit / 未 push（board Merge Rule：仅 A0 可推送）。本文件为新增独立文档，与工作树中 A2/A5/A6/A8/A9/A11 未提交产物无交集（空文件属其各自 lane，A10 不擅改）。
- 结论全部基于提案实证 + 当前源码实查（§0），非文档互证。前序 `A10-M4-security-review-batch-final-*` 仍有效，本文件为 M5 Wave 0 增量裁定。
- **v3 更新点（2026-09-06 15:20）**：在 v2（已含 A1 卡/A7/A9-plugin）基础上，补审了**全部** M5-W0 lane 专文（A3-M5-mcp、A4-M5-a2a-memory、A5-M5-agent-skill、A6-M5-agent-ui、A8-M5-graph-ui、A11 verification/debt/gui 三份），将首轮误判为"空文件/缺失"的 H-1 更正为**已解决**；并据 A3 提案新增 3 项安全复审发现 **D-5（rmcp→tokio vs F-1）、D-6（MCP 无 webview 致来源校验不可套用）、D-7（open_tool ACL 缺口 + 奇偶门禁缺失）**，裁定整体仍为 `PASS_WITH_DEBT`，硬门槛新增 D-5/D-6（须 A0 在 M5-2.a 开工前裁决）。
- v2 更新点：纳入 A1 `M5-20260906/` 卡集、A7 graph core、A9 plugin prework 后，将原"PASS_WITH_DEBT（G-1/G-3/G-4 阻塞）"升级为"四类绕过设计层全 PASS，唯一硬门槛 D-1 策略脚本机检落地"。
