# A10 · M5-W6 安全复审门禁说明（predecessor 未产出 → BLOCKED）

> Lane: A10（M4/M5 独立安全复审）
> Dispatch: `PARALLEL_COMMAND_BOARD.md` → "M5-W6 Parallel Dispatch"（A0 于 2026-09-06 19:25 经 `4b438ef`/`77b1e3e` 加入；board 头标 master=`4b438ef`）
> A10 W6 任务：Security review A8/A9 for **secret display / unbounded UI·store growth / plugin path traversal / signature bypass / command exposure / ACL drift**
> 交付形态（board）：`logs/assist/A10-M5-W6-*.md`；仅当修复具体失败时可加 policy 夹具
> 复审时间：2026-09-06 ~23:00 CST
> 交付物：本说明（仅 docs，未改动任何产品代码，未 push）

## 0. Lane Output Template（机器可读结论）

```text
LANE=A10
STATUS=BLOCKED
BASE=77b1e3e
HEAD=logs/assist/A10-M5-W6-security-review-20260906-2300.md（docs only）
FILES=logs/assist/A10-M5-W6-security-review-20260906-2300.md
VERIFY=实证：A8/A9 W6 产品代码在主副本缺席（见 §2，与 A4 W6 复核一致）；复审尚不可运行
CHECKPOINT=logs/assist/A10-M5-W6-security-review-20260906-2300.md
MERGE_NOTES=A8/A9 W6 产出尚未由 A0 集成进 master；A10 无法复审不存在的产品代码。依 Startup Gate（board 第64行）产出只读 assist 说明并给精确解锁条件，不编产品代码
NEXT=A0 集成 A8 W6（*Graph*.vue + src/stores + check-graph-ui-logic.mjs）+ A9 W6（plugin.rs + domain.rs PluginDef + check-plugin-policy.py）后，A10 跑 §4 验证矩阵并写 A10-M5-W6-security-review-<ts>.md（PASS / PASS_WITH_DEBT / BLOCKED）
```

## 1. 范围与方法

按 board W6，A10 复审两块（W6 唯一可写产品代码者）：
- **A8（M5-9）**：图 UI 纯逻辑/面板壳——图列表/搜索/过滤、节点详情摘要、容量/错/空态、helper + headless 逻辑测试。**禁 backend 命令、禁 live agent 消费、禁模型调用、禁图重建 worker**。
- **A9（M5-10/11）**：插件 manifest/lifecycle 策略切片——DTO、校验、生命周期状态机、权限 manifest 规则、策略脚本。**禁 install/delete/download/execute/enable 真实插件；纯 manifest/lifecycle 策略**。

6 类焦点（W6 特化）：secret display / unbounded UI·store growth / plugin path traversal / signature bypass / command exposure / ACL drift。
方法：产出后**实证**跑门 + `git diff` + `grep`（非仅凭读码）。

## 2. 实证发现：A8/A9 W6 产出尚未落库（阻塞项）

证据（2026-09-06 ~23:00 在 `WORKDIR`；与 `A4-M5-W6-memory-privacy-capacity-review-20260906-2237.md` 复核一致）：
- **A8**：`ls scripts/check-graph-ui-logic.mjs` → MISSING；`ls src/components/ | grep -iE 'graph|force'` → 无组件；`ls src/utils/ | grep -i graph` → 无 graphUi util。
- **A9**：`ls scripts/check-plugin-policy.py` → MISSING；`ls src-tauri/src/plugin*.rs` → 无；`domain.rs` grep `PluginManifest|PluginLifecycle|enum Plugin` → 无。
- `git log --oneline --all | grep -iE 'M5-9|M5-10|M5-11|graph-ui|plugin'` → 仅 `77b1e3e`（W6 调度文档）、free-prework 插件权限草案（`af6928a`/`e7d4722`）、M4/M5-assist 拆解、`M0-1.a-fix2` plugin fmt gate。**无 A8/A9 W6 产品提交**。
- `git status`：仅 `M5-0-overview.md`、`M5-9-graph-ui-agent-consume.md`（被改，A1 对账类 checkpoint 文档，非实现）＋ 未跟踪 `A4-M5-W6-memory-privacy-capacity-review-20260906-2237.md`（A4 W6 支持）。**无 A8/A9 产品产物**。

结论：A8/A9 W6 产品代码不在主副本。二者大概率在各自 worktree 开发，A0 尚未集成。

## 3. 解锁条件（精确）

满足任一即触发 A10 实质复审：
- **(a)** A0 将 A8 W6（`feat(M5-9 …)` / `*Graph*.vue` / `check-graph-ui-logic.mjs`）+ A9 W6（`feat(M5-10 …)` / `plugin.rs` / `PluginManifest` DTOs / `check-plugin-policy.py`）提交进 master（文件在场、`git log` 可见）；或
- **(b)** A8/A9 在本主副本暂存/落盘其 W6 产品代码（`git status`/`ls` 可见）。

解锁前 A10 不得改产品代码（Startup Gate 第64行）。本说明即当期交付。

## 4. 预飞验证矩阵（解锁后执行）

### A8（M5-9 graph UI）
- UI 逻辑测试：`node scripts/check-graph-ui-logic.mjs`（须 PASS）。
- 构建：`npm run build`（须 PASS，无新依赖、无 live agent 消费）。
- 消费 bounded DTO + 分页：`grep -rnE 'GRAPH_QUERY_LIMIT|NodeCapacityExceeded|EdgeCapacityExceeded|GraphError' src/components/**/*Graph* src/stores/**` → 列表/搜索遵守 `GRAPH_QUERY_LIMIT=1000` 且 >1000 分页；容量/错/空态映射到 `GraphError` 变体（R8-1）。
- 无原始 props 渲染：`grep -rnE 'props\b|rawProps|node\.props' src/components/**/*Graph*` → 节点详情不得渲染原始 props 值；须双扫过滤（R8-2，纵深防御）。
- 无 live agent 消费 / 无 backend 命令 / 无模型调用：`grep -rnE 'agent_consumption|invoke\("|runSkill|executeSkill|fetch\(|reqwest|model_call|HttpClient' src/components/**/*Graph* src/stores/**` → 空（W6 硬停：A8 禁 backend 命令/live agent 消费/模型调用/图重建 worker）。
- 无跨 store 读 agent memory：`grep -rnE 'agent_memory|agent_kv|AgentKv' src/components/**/*Graph* src/stores/**` → 空（R8-4，UI 不得直读 KV store）。
- 引用完整性：`grep -rnE 'GRAPH_NODE_ID_HEX_LEN|SkillDef\.id|AgentDef\.id|64' src/components/**/*Graph*` → id 须按 64-hex 校验展示（R8-5）。
- 前端态局部持有：`grep -rnE 'localStorage|sessionStorage|audit|console\.log' src/components/**/*Graph*` 对搜索/过滤态 → 不得外泄至审计/日志/持久化（R8-3，W6 硬停无 token/cookie/Authorization/body/prompt-secret 日志/持久化）。

### A9（M5-10/11 plugin manifest/lifecycle）
- 策略门三模式：`python3 scripts/check-plugin-policy.py --self-test` / 默认 / `--expect-pending`（pending 守门须 NONE）。
- Rust 测试：`cargo test --manifest-path src-tauri/Cargo.toml plugin`（须 PASS）。
- 隐私双扫（关键）：`grep -rnE 'SENSITIVE_KEY_NAMES|SENSITIVE_VALUE_PATTERNS|double_scan|token|password|api_key|Authorization' src-tauri/src/plugin.rs scripts/check-plugin-policy.py` → `check-plugin-policy.py` 须含**字段名 + 值模式**双扫，复用 A4/A7 同源常量（R9-1 / A4 F1）。
- manifest 无 secret + 长度上限：`grep -rnE 'secret|password|token|api_key|MAX_TEXT_FIELD_BYTES|GRAPH_LABEL_MAX_BYTES' src-tauri/src/plugin.rs` → manifest 元数据不得含 secret 字段；name/version/description/author 须有长度上限（R9-1）。
- 有界 registry：`grep -rnE 'BOUNDED|max_plugins|capacity|Vec<|HashMap<' src-tauri/src/plugin.rs` → 插件登记表须有全局上限（R9-2，对齐 `AGENT_KV_BOUNDED_TOTAL`/`GRAPH_BOUNDED_STORE`）。
- 无第二执行路径 / 无 install runtime / 无路径穿越：`grep -rnE 'std::process|spawn|install|uninstall|download|execute|remove_dir|remove_file|fs::write|Path::join|"\.\."|parent\(' src-tauri/src/plugin.rs` → 空（R9-4；lifecycle 纯状态机，不得 install/delete/download/exec/网络/路径穿越进 agent memory 目录）。
- **签名绕过（W6 焦点）**：若 `check-plugin-policy.py`/`plugin.rs` 含任何签名校验，须断言**无** `verify=false` / `skip_verify` / `allow_unsigned` / 空 pubkey 放行 / 异常被吞后继续加载等绕过；W6 硬停："no signature enforcement beyond pure validation unless fully local" → 纯校验可存在，但不得有可被禁用/跳过的分支（R9 签名项）。
- 不跨 store 读 agent memory：`grep -rnE 'agent_memory|agent_kv|AgentKv' src-tauri/src/plugin.rs` → 权限白名单不得含 agent_kv 读/写（R9-3）。
- 命令暴露：若新增 Tauri 命令，须 5 处原子一致（Rust handler + `main.rs` invoke + ACL 位于 `list_artifact_images` 之前 + `src/bridge.ts` + `src/types.ts`）+ source check（`check_invocation_source`）+ 测试（W6 硬停：prefer no command）。

### 跨切（两块皆查）
- `cargo test --manifest-path src-tauri/Cargo.toml`（全量，基线含 W4/W5 新增）。
- `git diff HEAD -- src-tauri/src/domain.rs src-tauri/src/security_policy.rs src-tauri/src/bridge.rs src-tauri/src/main.rs src/tauri/permissions/default-commands.toml src/components/** src/stores/** src/types.ts src/bridge.ts scripts/pre-merge.sh` → 查 ACL/命令/前端包漂移（高冲突文件清单）。
- `bash scripts/pre-merge.sh`（A8/A9 接好各自脚本后须全过）。
- `git diff --check`。
- 敏感常量单一真源（承接 A4 F1）：`grep -rnE 'SENSITIVE_KEY_NAMES|SENSITIVE_VALUE_PATTERNS' src-tauri/src/*.rs scripts/check-*.py` → 须仅 `domain.rs` 定义一份，A4/A7/A9 均 `use crate::domain::*` 引用，不得第三份拷贝。

## 5. 预飞风险假设（基于已落地契约）

- **边界真相源（A4 W6 复核）**：A4 W4 5 码（`AGENT_KV_POLICY`）+ A7 W5 7 码（`GRAPH_POLICY`），均已 PASS（ACTIVE=5 / ACTIVE=7）。A8/A9 必须对齐这四族：隐私双扫、容量全局上限、审计脱敏、无第二执行路径。
- **图不存 agent_kv 值**：`graph.rs` 明示图谱仅存 `Memorizes` 关系边，Skill/Agent 节点按 id 引用（`GRAPH_REF_NODE_INTEGRITY`、`GRAPH_NODE_ID_HEX_LEN=64`）。当前无跨 store 泄露路径；A8 UI 亦不得直读 KV（R8-4）。
- **单一真源 / DRY（A4 F1）**：`SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS` 现重复于 `agent_memory.rs:134` 与 `graph.rs:18`；A9 的 `check-plugin-policy.py` 须复用同一份，否则成第三拷贝——A10 将断言单一真源（抽 `domain.rs`）。
- **容量常量（已知）**：`GRAPH_MAX_NODES=5000`/`GRAPH_MAX_EDGES=20000`（`domain.rs:2114`）、`GRAPH_TRAVERSAL_BOUNDED`（depth4/limit1000）、`GRAPH_QUERY_LIMIT=1000`、`AGENT_KV_BOUNDED_TOTAL`（5MiB/5000/32 agents/LRU）。A8 UI 须遵守查询上限与分页；A9 registry 须有全局上限。
- **W6 硬停止断言**：A8 禁 backend 命令/live agent 消费/模型调用/图重建 worker；A9 禁 install/delete/download/execute/enable 真实插件、纯 manifest/lifecycle；所有 store/map/list 有界；无 token/cookie/Authorization/body/prompt-secret 日志或持久化；任何新命令须 source-check+ACL+bridge/types+策略+测试同包。A10 逐条 grep 断言。
- **签名绕过（A9 特化）**：纯本地校验允许，但不得存在可禁用/跳过分支；若 A9 引入签名字段，须 fail-closed（无签名/校验失败即拒）。

## 6. 延续 / 引用

- **W4/W5 复审积压（现已解锁，建议补做）**：W6 "Current facts" 明示 **Agent/Skill UI shell（A6 W5）已集成、graph model/store policy slice（A7 W5）已集成**；W5 facts 明示 **agent memory KV（A4 W4）+ Agent/Skill domain（A5 W4）已集成**。故 A10 既有的 `A10-M5-W4-security-review-20260906-1805.md`（A4/A5，原 BLOCKED）与 `A10-M5-W5-security-review-20260906-1900.md`（A6/A7，原 BLOCKED）现**已具备复审条件**，建议 A0/A10 在 W6 之后补做这两份实质复审（本说明不越位执行，待指令）。
- 同波 W6 支持文档：`A4-M5-W6-memory-privacy-capacity-review-20260906-2237.md`（已读，提供 R8-1~5 / R9-1~6 红线与 F1 DRY 项）、`A2-M5-W6-*`（core 边界/seam 方向）、`A3-M5-W6-*`（plugin/graph vs MCP 注册表）。
- free-prework 草案（A9 契约背景）：`af6928a`/`e7d4722` 插件权限模型（`PluginManifest`/权限分级/fail-closed）。
- board W6 段：第 136-172 行（Assignments + Hard Stops）。

## 7. 声明

- 仅 docs；未改产品代码；未 push。
- 工作树其余未提交物（`M5-0-overview.md`、`M5-9-graph-ui-agent-consume.md` 属 A1 对账；`A4` W6 文档属他 lane）均未触碰。
- HEAD = `77b1e3e`，本地与 `origin/master` 一致（领先 0）。
