# A10 · M5-W5 安全复审门禁说明（predecessor 未产出 → BLOCKED）

> Lane: A10（M4/M5 独立安全复审）
> Dispatch: `PARALLEL_COMMAND_BOARD.md` → "M5-W5 Parallel Dispatch"（A0 于 2026-09-06 18:35 经 `0e76a89` 加入；board 头标 master=`1610939`，本地实际头 `0e76a89`）
> A10 W5 任务：Security review A6/A7 for **secret display / unbounded graph growth / command exposure / source-check·ACL drift / prompt·body persistence**
> 交付形态（board）：`logs/assist/A10-M5-W5-*.md`；仅当修复具体失败时可加 policy 夹具
> 复审时间：2026-09-06 ~19:00 CST
> 交付物：本说明（仅 docs，未改动任何产品代码，未 push）

## 0. Lane Output Template（机器可读结论）

```text
LANE=A10
STATUS=BLOCKED
BASE=0e76a89
HEAD=logs/assist/A10-M5-W5-security-review-20260906-1900.md（docs only）
FILES=logs/assist/A10-M5-W5-security-review-20260906-1900.md
VERIFY=实证：A6/A7 W5 产品代码在主副本缺席（见 §2 证据）；复审尚不可运行
CHECKPOINT=logs/assist/A10-M5-W5-security-review-20260906-1900.md
MERGE_NOTES=A6/A7 W5 产出尚未由 A0 集成进 master；A10 无法复审不存在的产品代码。依 Startup Gate（board 第64行）产出只读 assist 说明并给精确解锁条件，不编产品代码
NEXT=A0 集成 A6 W5（Agent/Skill UI 组件 + check-agent-skill-ui-logic.mjs + 必要 types/bridge）+ A7 W5（graph.rs + GraphNode/GraphEdge DTOs + check-graph-policy.py + security_policy 增补）后，A10 跑 §4 验证矩阵并写 A10-M5-W5-security-review-<ts>.md（PASS / PASS_WITH_DEBT / BLOCKED）
```

## 1. 范围与方法

按 board W5，A10 复审两块：
- **A6（M5-6）**：Agent/Skill UI 纯逻辑与面板壳——校验展示、权限预览、能力清单、空/错状态；helper 模块 + headless 逻辑测试。**不执行 skill、不装插件、不调 live runtime**。
- **A7（M5-7/8）**：图模型/存储策略切片——`GraphNode`/`GraphEdge` DTOs、容量/脱敏规则、纯图存储/查询助手、策略脚本。**无图 UI、无 agent 消费、无后台图重建 worker、无网络访问**。

5 类焦点（W5 特化）：secret display / unbounded graph growth / command exposure / source-check·ACL drift / prompt·body persistence。
方法：产出后**实证**跑门 + `git diff` + `grep`（非仅凭读码）。

## 2. 实证发现：A6/A7 W5 产出尚未落库（阻塞项）

证据（2026-09-06 ~19:00 在 `WORKDIR` 运行）：
- **A7**：`ls src-tauri/src/graph.rs` → MISSING；`ls scripts/check-graph-policy.py` → MISSING；`domain.rs` grep `GraphNode|GraphEdge|enum Graph` → 无（仅有 A5 W4 已落地的 `SkillDef`@1968、`AgentDef`@2006）。
- **A6**：`ls scripts/check-agent-skill-ui-logic.mjs` → MISSING；`ls src/components/ | grep -iE 'agent|skill'` → 无组件；`src/types.ts` grep `AgentDef|SkillDef|AgentSkill` → 无（前端类型尚未镜像，符合 W4 A5 仅落 domain.rs 侧）。
- `git log --oneline --all | grep -iE 'M5-6|M5-7|M5-8|graph|agent-skill-ui'` → 仅 `0e76a89`（W5 调度文档）、`ae3df2c`/`c52adb5`（free-prework 图谱草案文档）、`67fe62d`/`15c8ff6`（free-prework A2P/A2A 草案）。**无 A6/A7 W5 产品提交**。
- `git status`：仅 `M5-0-overview.md`、`M5-6-agent-skill-ui.md`（被改，属 A1 对账类 checkpoint 文档，非实现）＋ 未跟踪 `A2-M5-W5-seam-graph-review-20260906-1525.md`（A2 W5 支持）、`M5-9-graph-ui-card-20260906-1525.md`（A8 W5 文档）、`A3-M5-W5-mcp-compat-20260906-1842.md`（A3 W5 空文件，不可用）。**无 A6/A7 产品产物**。

结论：A6/A7 W5 产品代码不在主副本。二者大概率在各自 worktree 开发，A0 尚未集成。

## 3. 解锁条件（精确）

满足任一即触发 A10 实质复审：
- **(a)** A0 将 A6 W5（`feat(M5-6 …)` / Agent/Skill 面板组件 / `check-agent-skill-ui-logic.mjs`）+ A7 W5（`feat(M5-7 …)` / `graph.rs` / `GraphNode`+`GraphEdge` DTOs / `check-graph-policy.py`）提交进 master（文件在场、`git log` 可见）；或
- **(b)** A6/A7 在本主副本暂存/落盘其 W5 产品代码（`git status`/`ls` 可见）。

解锁前 A10 不得改产品代码（Startup Gate 第64行）。本说明即当期交付。

## 4. 预飞验证矩阵（解锁后执行）

### A6（M5-6 Agent/Skill UI）
- UI 逻辑测试：`node scripts/check-agent-skill-ui-logic.mjs`（须 PASS）。
- 构建：`npm run build`（须 PASS，无新依赖、无 live 执行）。
- 无执行/安装/网络/模型调用：`grep -rnE 'spawn|shell|install|download|fetch\(|reqwest|HttpClient|executeSkill|runSkill' src/components/**/*Agent* src/components/**/*Skill* src/stores/** src/utils/**agentSkill**` → 空（A6 禁运行时/安装器/网络/模型调用/新后端命令）。
- 无 secret 展示：`grep -rnE 'token|cookie|Authorization|password|secret|body|prompt' src/components/**/*Agent* src/components/**/*Skill*` → 仅脱敏占位或零出现；DTO 展示层不得渲染凭据/body/prompt 秘文。
- 前端类型/桥接 parity：若 `src/types.ts`/`src/bridge.ts` 新增，须与 `domain.rs` 的 `AgentDef`@2006/`SkillDef`@1968 一致；无新 Tauri 命令（W5 硬停止：A6 禁新后端命令）。
- 容量/边界：面板渲染须对能力清单/权限项做有界截断与空/错态。

### A7（M5-7/8 graph model/store）
- 策略门三模式：`python3 scripts/check-graph-policy.py --self-test` / 默认 / `--expect-pending`（pending 守门须 NONE）。
- Rust 测试：`cargo test --manifest-path src-tauri/Cargo.toml graph`（须 PASS）。
- 有界图增长：确认 `GraphNode`/`GraphEdge` DTO + 纯 store/query 助手含节点/边容量上限；测试断言超容被拒/截断；`check-graph-policy.py` 守容量漂移（类 `MCP_CAPABILITY_DRIFT`/`DB_LIMIT` 模式）。
- 脱敏/无 secret 持久化：`grep -rnE 'token|cookie|Authorization|password|secret|body|prompt' src-tauri/src/graph.rs` → 节点/边载荷不得承载凭据/body/prompt-secret；审计/文件/日志同理（W5 硬停止：无 token/cookie/Authorization/body/prompt-secret 日志或持久化）。
- 无网络：`grep -rnE 'TcpListener|\.bind\(|reqwest|ureq|tokio::net|rusqlite.*network|http' src-tauri/src/graph.rs` → 空（图存储须复用既有 SQLite/工作区根，禁网络访问）。
- 无 agent 消费、无后台重建：`grep -rnE 'agent_runtime|rebuild_worker|spawn|thread::spawn|tokio::spawn' src-tauri/src/graph.rs` → 空（W5 硬停止：无 agent 消费、无后台图重建 worker）。
- core 侧复用 seam 而非 tauri：若图模块需路径/根，须用 `mvp_core::seam`（`lib.rs` 已 `pub mod core; pub use crate::core::*`），不得重引 tauri/bridge/AppHandle；`python3 scripts/check-core-boundary.py` 仍 PASS。
- 命令暴露：若新增 Tauri 命令，须 5 处原子一致（Rust handler + `main.rs` invoke + ACL 位于 `list_artifact_images` 之前 + `src/bridge.ts` + `src/types.ts`）+ source check（`check_invocation_source`）+ 测试（W5 硬停止：prefer no command）。

### 跨切（两块皆查）
- `cargo test --manifest-path src-tauri/Cargo.toml`（全量，基线含 W4 新增）。
- `git diff HEAD -- src-tauri/src/domain.rs src-tauri/src/security_policy.rs src-tauri/src/bridge.rs src-tauri/src/main.rs src/tauri/permissions/default-commands.toml src/components/** src/stores/** src/types.ts src/bridge.ts scripts/pre-merge.sh` → 查 ACL/命令/前端包漂移（高冲突文件清单）。
- `bash scripts/pre-merge.sh`（A6/A7 接好各自脚本后须全过）。
- `git diff --check`。

## 5. 预飞风险假设（基于已落地契约）

- **A5 W4 已落地 DTO 真源**：`domain.rs` 的 `SkillDef`@1968、`AgentDef`@2006（随 `1610939 feat(M5): add agent memory and skill policy shells`）。A6 W5 UI 必须**仅消费**这些 DTO，镜像到 `src/types.ts`/`src/bridge.ts`，并把校验/权限预览/能力清单做成 headless 逻辑（参考既有 `src/utils/*Ui.ts` + `scripts/check-*-ui-logic.mjs` 模式）。
- **A2 W5 边界建议**：`A2-M5-W5-seam-graph-review-20260906-1525.md` 讨论「图存储用 core seam 还是 bin 侧」。A7 图模型若属 core 侧，须复用 `mvp_core::seam`，否则不得越界引入 tauri/bridge（core 边界门强制）。
- **单一真源 / 容量脱敏**：A7 图能力/白名单/上限应在 `domain.rs` 定义一处；`check-graph-policy.py` 守漂移；节点/边载荷强制执行脱敏与容量（对齐 MCP/DB/agent-memory 既有模式）。
- **无第二执行路径 / 无网络**：A6 禁 skill 执行（复用 M2-4 仅在未来命令层，非 W5）；A7 禁 agent 消费与后台重建 → 两块均无新进程/网络路径。
- **W5 硬停止断言**：A6 禁执行运行时/安装器/网络·模型调用/新后端命令；A7 禁 live UI/agent 消费/后台重建/网络；所有 store/map/list 有界；无 token/cookie/Authorization/body/prompt-secret 日志或持久化。A10 逐条 grep 断言。
- **高冲突文件漂移**：A7 触 `domain.rs`/`security_policy.rs`/`pre-merge.sh`；A6 触 `src/components`/`src/stores`/`src/types.ts`/`src/bridge.ts`/scripts。A10 验 5 路命令 parity 与 ACL 顺序。

## 6. 延续 / 引用

- A10 M5-W4 复审：`logs/assist/A10-M5-W4-security-review-20260906-1805.md`（A4/A5 W4 未产出 → BLOCKED 门禁）。
- A10 M5-W3 复审：`logs/assist/A10-M5-W3-security-review-20260906-1730.md`。
- 同波 W5 支持文档（他 lane，未消费）：`A2-M5-W5-seam-graph-review-20260906-1525.md`、`M5-9-graph-ui-card-20260906-1525.md`（A8）；`A3-M5-W5-mcp-compat-20260906-1842.md`（**空文件，不可用**）。
- 已落地契约：`1610939` 的 AgentDef/SkillDef（A5 W4）+ agent memory KV（A4 W4）；`f8f1f49` 的 core seam（A2 W3）。
- free-prework 草案（背景参考）：`ae3df2c`/`c52adb5` 图谱数据模型、`67fe62d`/`15c8ff6` A2P/A2A 协议。
- board W5 段：第 135-170 行（Assignments + Hard Stops）。

## 7. 声明

- 仅 docs；未改产品代码；未 push。
- 工作树其余未提交物（`M5-0-overview.md`、`M5-6-agent-skill-ui.md` 属 A1 对账；`A2`/`A8`/`A3` W5 文档属他 lane）均未触碰。
- HEAD = `0e76a89`，本地与 `origin/master` 一致（领先 0）。
