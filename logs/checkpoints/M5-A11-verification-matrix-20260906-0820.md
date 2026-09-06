# M5-A11 验证证据包 · M5 验证矩阵（前瞻预案）

> Lane：**A11（M5 verification matrix）** ｜ 路由 `AI:BALANCED / R:medium`
> 时间：2026-09-06 08:20 CST ｜ 作者：CodeBuddy Hy4
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5 Dispatch Now → M5 Wave 0 Assignments → A11：**START VERIFICATION PLAN**
> 范围声明：本包**只产出验证日志 / 清单 / 台账，零产品代码改动**；不新增 Cargo/npm 依赖、不新增命令/ACL/UI 面板（严守 §M5 Wave 0 Hard Stops）；`git push` 归 A0，本车道不提交、不推送。
> 性质：**前瞻预案**。M5 Wave 0 全为 DOCS ONLY（A1~A10 只展开卡 / 预研，零产品代码）。本矩阵定义 M5-1~M5-12 各能力落地后**必须跑的验证命令、policy 脚本、UI 逻辑测试、manual/GUI 清单与债务编号延续规则**；M5 实现卡落地前，命令矩阵中"定向命令"一律记 `NOT_RUN(实现未到)`，不得代签 PASS。
> 姊妹交付：`logs/assist/M5-A11-debt-ledger-20260906-0820.md`（M5 债务台账预案，D43 起）、`logs/assist/M5-A11-gui-manual-checklist-20260906-0820.md`（M5 人工/GUI 清单）。
> 对齐：`logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md`（M4 终版矩阵，本文件继承其 T0/T1-a/T2 骨架与 G1~G12 护栏映射）。

---

## 1. 启动门禁实跑（WORKSPACE_IDENTITY.md / PARALLEL_COMMAND_BOARD.md 要求）

```bash
cat .workspace-identity            # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master
pwd                                # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git status --short --branch        # ## master...origin/master [领先 1]；未跟踪 = 各 lane 的 M5 prework 文档（?? logs/assist/A2-M5-*.md 等）  ✅
git log --oneline -3              # 5ca8f9f docs(M5): dispatch architecture prework lanes / a1a2061 feat(M4): integrate database and scheduler lanes
```

判定：**通过**（目录 = 规范主副本、分支 = `master`、工作树脏文件均为其他 lane 的 M5 prework 文档，不属于本车道，且均为 `logs/**` 机器证据，不构成冲突）。

> A11 在工作树的写入目标：`logs/checkpoints/M5-A11-verification-matrix-20260906-0820.md` + `logs/assist/M5-A11-*.md`（2~3 个新增文件），不触碰任何高冲突文件（bridge.rs/domain.rs/main.rs/ACL/types.ts/pre-merge.sh/三份主文档）。

---

## 2. BASE 证据（M5 起点，取 A0 最新 spot-check，非初始 dispatch 声明）

> **证据真实性声明（回应 board §Integration Fix Wave IF-5）**：A0 在 `a1a2061` 的初始 M5 Dispatch 声明为「cargo test 329/329、pre-merge ALL_PASS」，但该声明已被 A0 后续 spot-check（board §Integration Fix Wave，2026-09-06）**修正**。本矩阵以 A0 最新 spot-check 为准，不沿用过时声明（IF-5 已点名 A11 旧报告 stale）。

| 项 | 命令 | 实测结果（A0 最新 spot-check / IF 段） | 取样点 |
|---|---|---|---|
| Rust 全量单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | **328 passed；0 failed**（IF-5 提及，较初始 329 少 1） | board §Integration Fix Wave |
| 前端构建 | `npm run build` | **PASS** | 同 |
| 集成门禁 | `bash scripts/pre-merge.sh` | **FAIL**（IF-1~IF-4 红绿灯未解） | 同 |
| IF-1 | `cargo fmt --manifest-path src-tauri/Cargo.toml` | **对 M4 Rust 文件失败**（需 A3/A4/A7 跑 fmt） | IF-1 |
| IF-2 | 前端 main JS 197.04kB vs 161.36kB 基线 | **超 15% 门（≈22% 增长）**，A5/A8 查因、A0 裁定 | IF-2 |
| IF-3 | `python3 scripts/check-tools-policy.py --self-test` | **失败**（A11 称既有非 M4 缺陷，待 A10 比 `origin/master` 确认） | IF-3 |
| IF-4 | `Cargo.toml` 误加 `rusqlite`/`mysql`/`postgres`；`cargo check` warning 2→5 | **需 A3/A4 清理** | IF-4 |
| WBS 路由 | `python3 scripts/check-plan-routing.py` | `ok (50 WBS rows)`，EXIT=0 | `5ca8f9f` |
| M5 预研文档 | `ls logs/assist/A{2,3,4,5,6,7,8,9,10}-M5-*.md` | A1~A10 均已交 M5 prework（见 §3.5） | 工作树 |

> **M5 前提状态（board §M5 Dispatch Now 135）**：「Do not write M5 product code until A1 has produced the M5 task cards and A0 has signed the first implementation dispatch」；且「M4 整体 PASS」为 M5 实现波的隐性前提。**当前状态**：① M4 `pre-merge = FAIL`（IF-1~IF-4 未解）；② A1 子卡已展开（`M5-20260906/` 15 文件）但 **A0 尚未签署首个实现 dispatch**。故 M5 产品代码**当前不可写**；本矩阵所有 T1 定向命令一律 `NOT_RUN(实现未到)`，待 IF-1~IF-4 清零、`pre-merge` 转 `ALL_PASS`、A0 签署实现 dispatch 后，各 lane 方可实施。

---

## 3. 定向命令矩阵

### T0 · 全车道通用（每次 M5 交付前必跑，任一项失败 = 该 lane 不具备送审资格）

> 继承 M4 矩阵 §3 T0-1~T0-10，M5 新增两条（T0-11 / T0-12）以守"core 边界"与"无新增依赖"。

| # | 命令 | 期望 | 失败口径 |
|---|---|---|---|
| T0-1 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` | EXIT 0，无 diff | 阻塞 |
| T0-2 | `cargo fmt --manifest-path tauri-browser-tabs/Cargo.toml --all --check` | EXIT 0 | 阻塞 |
| T0-3 | `cargo test --manifest-path src-tauri/Cargo.toml` | `passed ≥ 328`，`failed = 0`（**计数只增不减**；M5 新增测试须上调下限；基线取自 A0 最新 spot-check 328，非初始 dispatch 的 329） | 阻塞 |
| T0-4 | `cargo check --manifest-path src-tauri/Cargo.toml --locked` | error 0；warning 不增（现状 2 类 `grid_process.rs`） | 阻塞 |
| T0-5 | `npm run build` | 0 error；主 JS 增长 ≤ A0 书面阈值 16% | 阻塞 |
| T0-6 | `python3 scripts/measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-<最早>.json --skip-build` | 总体积增长 ≤16%、cargo warning 不增 | 阻塞 |
| T0-7 | `python3 scripts/check-plan-routing.py` | `ok (… WBS rows)`（A1 展开 M5 子卡后行数同步增加） | 阻塞 |
| T0-8 | `bash scripts/pre-merge.sh` | `PRE_MERGE_RESULT=ALL_PASS`（**当前 FAIL：IF-1~IF-4 未解，M5 实现波前须清零**） | 阻塞 |
| T0-9 | `git diff --check` + `git diff --cached --check` | 输出为空 | 阻塞 |
| T0-10 | `python3 scripts/check-lifecycle-contract.py --self-test` + 默认 | `ALL_PASS` / EXIT 0 | M5 新增 `*Shutdown`（图 flush / 插件收口）须含断言 |
| **T0-11** | `grep -rl 'use tauri' crates/core/src 2>/dev/null`（M5-1 落地后） | **输出为空**（core 边界） | M5-1 阻塞；实现前记 `NOT_RUN` |
| **T0-12** | `cargo tree -p mvp-core 2>/dev/null \| grep -i 'tauri\|tokio\|sqlx\|diesel\|neo4j'` | **无命中**（禁反向依赖 / 禁重依赖） | M5-1/2/7 阻塞；实现前记 `NOT_RUN` |

### T1 · 按车道定向（M5 实现批落地后由 A0 逐 lane 复跑；当前全 `NOT_RUN(实现未到)`）

| Lane（M5） | 定向命令 | 期望 | 现状（2026-09-06 08:20） |
|---|---|---|---|
| **A2 / A13**（M5-1 core 下沉） | `cargo test -p mvp-core`；`grep -rl 'use tauri' crates/core/src`（空）；`cargo tree -p mvp-core`（无 tauri）；`bash scripts/check-core-boundary.sh` | core 全绿；边界断言通过 | **NOT_RUN**：M5-1.a 未签署；切片 0a/0b 未实施（A2 `A2-M5-core-20260906-0749.md` 已给切片与红线 R1~R7） |
| **A3 / A14**（M5-2 MCP/rmcp） | `cargo test mcp`（或 `rmcp`）；`python3 scripts/check-mcp-policy.py --self-test` + 默认 + `--expect-pending`；`grep -rn 'npm' src-tauri/Cargo.toml`（应为空，禁 npm 分包） | MCP 工具复用 M4 `classify_sql_risk` / `is_production_database`；read_only/allow_dangerous_sql/allowed_connection_ids 守门 | **NOT_RUN**：A3 M5-2 文档未生成；脚本未建 |
| **A4 / A15**（M5-3 A2A/memory） | `cargo test a2a`；`agent_kv` 容量/保留单测；方言归一测试 | 多 Agent 方言归一、协议兼容；`agent_kv` 仅存 AI 上下文、有容量上限 | **NOT_RUN**：A4 M5-3 文档未生成 |
| **A5 / A16**（M5-4/5 Agent/Skill） | `cargo test agent_skill`（或 `skill_runtime`/`agent_runtime`）；`python3 scripts/check-agent-skill-policy.py --self-test` + 默认 + `--expect-pending`；`node scripts/check-agent-skill-ui-logic.mjs`（M5-6 后） | AGSK_1~9 全绿；`grep -nE 'std::process\|Command::new\|sh -c' src-tauri/src/skill_runtime.rs src-tauri/src/agent_runtime.rs` 为 0（无第二执行路径） | **NOT_RUN**：A5 预研已交付（`A5-M5-agent-skill-20260906-0800.md`，9 条 AGSK_* 码位已定义、脚本提案已列 §11.1），实现卡 `M5-4.a`/`M5-5.a` 未签署 |
| **A6 / A19**（M5-6 Agent/Skill UI） | `node scripts/check-agent-skill-ui-logic.mjs`；`npm run build` | 对话/流式/Skill 管理/权限预览/失败恢复 UI 逻辑断言全过 | **NOT_RUN**：A6 预研已交付（`A6-M5-agent-ui-20260906-1200.md`），UI 未实施 |
| **A7 / A17**（M5-7/8 图谱） | `cargo test graph`；`python3 scripts/check-graph-policy.py --self-test` + 默认 + `--expect-pending`；`cargo test graph_feed`（图谱维护任务复用 `task_*`） | GRAPH_* 全绿；G-D1~G-D7 处置；容量/隐私/派生不覆盖主数据 | **NOT_RUN**：A7 契约已冻结（`A7-M5-graph-core-20260906-0755.md`，GRAPH_* 码位 + G-D1~G-D7 已定义），实现卡未签署；A9 越界草案已吸收为输入 |
| **A8 / A19**（M5-9 图谱 UI） | `node scripts/check-graph-ui-logic.mjs`；`npm run build` | 力导向/搜索/节点详情/容量/可访问性/子图注入 UI 逻辑断言全过 | **NOT_RUN**：A8 预研已交付（`A8-M5-graph-ui-20260906-0805.md`），UI 未实施 |
| **A9 / A18 / A19**（M5-10/11/12 插件） | `cargo test plugin_perm_`；`python3 scripts/check-plugin-policy.py --self-test` + 默认 + `--expect-pending`；`node scripts/check-plugin-ui-logic.mjs`；插件卸载清理测试 | SB-1~SB-8 全绿；声明式插件（form③）零第二执行路径；uninstall 不删共享实体 | **NOT_RUN**：A9 契约已交付（`A9-M5-plugin-system-prework-20260906-1100.md`，形态裁定 form③ 推荐、form① 否决、form② 条件可行；SB-* 红线 + 任务拆分 M5-10.a~M5-12.b 已列），实现卡未签署 |
| **A10**（M5 安全复核） | 复跑 A2~A9 全部门禁命令 + 读其检查点 | 与 lane 自报一致；重点查：无第二执行路径 / 凭据不出 core / ACL 末条锚点 / 审计脱敏 / 容量上限 | **NOT_RUN**：A10 M5 复核未生成（board §M5 Wave 0 A10 = START REVIEW，待 A1~A9 笔记齐） |

### T1-a · M5 新命令「三处同步 + 末条锚点」通用核查（每个 M5 lane 落地新命令时必跑）

继承 M4 矩阵 §3 T1-a，M5 命令名扩展为（示意，以 A0 最终签名为准）：
`skill_list/skill_install/skill_run/skill_enable/skill_disable/skill_uninstall`、`agent_chat`、
`graph_query/graph_rebuild/graph_prune`、`plugin_list/plugin_install/plugin_enable/plugin_disable/plugin_uninstall`、
`mcp_*`、`a2a_*`。一键检查：

```bash
for c in skill_list skill_install skill_run agent_chat graph_query graph_rebuild plugin_list plugin_install plugin_enable plugin_disable plugin_uninstall; do
  a=$(grep -c "\"$c\"" src-tauri/permissions/default-commands.toml)
  b=$(grep -c "$c" src-tauri/src/main.rs)
  [ "$a" -ge 1 ] && [ "$b" -ge 1 ] && echo "OK  $c" || echo "MISSING  $c (acl=$a main=$b)"
done
# 末条锚点：所有 M5 新命令行号必须 < list_artifact_images 行号
grep -n "list_artifact_images\|\"skill_\|\"agent_\|\"graph_\|\"plugin_\|\"mcp_\|\"a2a_" src-tauri/permissions/default-commands.toml
```

期望：全部 `OK` 且新命令行号 < `list_artifact_images`（坑位②：ACL 末条恒为 `list_artifact_images`）。

### T2 · GUI / 人工（**无真机取证一律记 `NOT_RUN`，不得代签 PASS**）

| # | 命令 | 用途 |
|---|---|---|
| T2-1 | `python3 scripts/m0-6c-gui-regression.py --self-test` | GUI 回归汇总脚本自检（不启动 GUI，CI 可跑） |
| T2-2 | `bash run-gui.sh` | 真机人工验收，逐条走 `logs/assist/M5-A11-gui-manual-checklist-20260906-0820.md` |
| T2-3 | `ps -eo pid,pgid,cmd \| grep mvp-browser-os` | 退出后无残留进程 / 进程组（M5 不得引入长生命周期子进程，form① 已否决） |
| T2-4 | `cat ~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/audit.json` | 审计取证：**不得出现**密码 / token / 连接串 / LLM key / 私密参数 |
| T2-5 | `bash scripts/collect-m0-baseline.sh` | M5 验收门禁第 5 条要求的 M0 基线对比 |

---

## 3.5 M5 预研 / 契约现状看板（A11 追踪 · 2026-09-06 08:20）

### 3.5.0 总表（M5 Wave 0 文档交付）

> **M5 Wave 0 文档交付全貌（2026-09-06 14:10 更新）**：① **A1 已完成 M5 子卡展开**——`logs/checkpoints/M5-20260906/` 目录含 15 文件（`M5-0-overview` 总览 + `M5-1`~`M5-12` 子卡 + `M5-13` 验证矩阵卡 + `M5-14` 债务账卡）；子卡尚未回写主文档 WBS 表（故 `check-plan-routing.py` 仍 50 行），待 A0 集成时由 A1 套用。② **A3/A4/A10 的 M5 文档已生成**（带时间戳，有内容）：A3 `A3-M5-mcp-20260906-0757.md`、A4 `A4-M5-a2a-memory-20260906-0755.md`、A10 `A10-M5-security-review-20260906-1410.md`。③ 本 A11 矩阵与 A1 `M5-13` **互补**：A1 列需求/反向用例（横切卡），A11 落**可执行命令矩阵 + 实物取证**（Lane 责任交付）；本文件带时间戳且有 28KB 实质内容，**非** board 禁空文件规则所指的空占位（A10 H-1 疑将 A1 `M5-20260906/M5-13-verification-matrix.md` 误读为本文件，A0 集成时核对）。

| 卡（M5） | Lane | 交付物 | 状态 | 产品代码 | 本矩阵提取的验证要点 |
|---|---|---|---|---|---|
| **M5-1** core workspace 下沉 | A2（预研）/ A13（实施） | `logs/assist/A2-M5-core-20260906-0749.md` | `PASS_WITH_DOCS`（DOCS ONLY） | 0 | core 边界 R1~R7；切片 0a/0b/1/2/3；`check-core-boundary.sh` 提案（§3.3/§8） |
| **M5-2** MCP/rmcp + 全局护栏 | A3（预研） | `logs/assist/A3-M5-mcp-20260906-0757.md` | `PASS_WITH_DOCS`（DOCS ONLY） | 0 | **`MCP_*` 码位**（MCP_NPM_SDK_PRESENT / MCP_NODE_RUNTIME_PRESENT / read_only / allow_dangerous_sql / allowed_connection_ids）；`check-mcp-policy.py` 提案；stdio-only 禁 TCP；纯 Rust rmcp 禁 npm 分包；McpGlobalPolicy fail-closed |
| **M5-3** A2A/memory | A4（预研） | `logs/assist/A4-M5-a2a-memory-20260906-0755.md` | `PASS_WITH_DOCS`（DOCS ONLY） | 0 | **F-A4-1~14** 冻结裁定 + `agent_kv` 14 条机器可检红线 + 错误码 -32013~-32018（AGENT_KV_KEY_FORBIDDEN / SECRET_SUSPECTED / CAPACITY_EXCEEDED / VALUE_TOO_LARGE / NAMESPACE_UNKNOWN 等）；`check-a2a-policy.py` + `check-agent-kv-policy.py` 提案 |
| **M5-4/5** Agent/Skill runtime+command | A5（预研） | `logs/assist/A5-M5-agent-skill-20260906-0800.md` | `PASS_WITH_DOCS`（DOCS ONLY） | 0 | **AGSK_1~9** 码位 + `check-agent-skill-policy.py` + `check-agent-skill-ui-logic.mjs` 提案（§11） |
| **M5-6** Agent/Skill UI | A6（预研） | `logs/assist/A6-M5-agent-ui-20260906-1200.md` | `PASS_WITH_DOCS`（DOCS ONLY） | 0 | 承接 `check-agent-skill-ui-logic.mjs` |
| **M5-7/8** 图谱 model/storage | A7（契约冻结）/ A17（实施） | `logs/assist/A7-M5-graph-core-20260906-0755.md` | `CONTRACT_PROPOSED`（吸收 A9 越界草案） | 0 | **GRAPH_*** 码位 + **G-D1~G-D7** 开放项 + `check-graph-policy.py` 提案（§6） |
| **M5-9** 图谱 UI | A8（预研） | `logs/assist/A8-M5-graph-ui-20260906-0805.md` | `PASS_WITH_DOCS`（DOCS ONLY） | 0 | 承接 `check-graph-ui-logic.mjs` |
| **M5-10/11/12** 插件系统 | A9（契约冻结）/ A18/A19（实施） | `logs/assist/A9-M5-plugin-system-prework-20260906-1100.md` | `CONTRACT_PROPOSED`（形态③推荐/①否决/②条件可行） | 0 | **SB-1~SB-8** 红线 + `check-plugin-policy.py` + `check-plugin-ui-logic.mjs` 提案；任务拆分 M5-10.a~M5-12.b |
| **M5 安全复核** | A10 | `logs/assist/A10-M5-security-review-20260906-1410.md` | `PASS_WITH_DEBT`（设计层安全放行；D-1/D-2/D-3 局部债务） | 0 | 五问裁定全绿；重点：无第二执行路径 / 凭据不出 core / ACL 末条锚点 / 审计脱敏 / 容量上限 / 共用 capability.rs |

### 3.5.1 已知文档缺失 / 状态（硬停止遵守情况，2026-09-06 14:10 复核）

- **A3（M5-2）/ A4（M5-3）M5 prework 已生成**（带时间戳，有内容）：`A3-M5-mcp-20260906-0757.md`、`A4-M5-a2a-memory-20260906-0755.md`。M5-2/M5-3 的验证要点已非推断，正式纳入 §3.5.0 / §3.5.2 / §4.3。
- **A1 M5 子卡已展开**：`logs/checkpoints/M5-20260906/` 含 `M5-0-overview` + `M5-1`~`M5-12` + `M5-13`（验证矩阵卡）+ `M5-14`（债务账卡）共 15 文件（DOCS ONLY）。子卡尚未**回写主文档 WBS 表**（高冲突文件，A1 不碰），故 `check-plan-routing.py` 仍 50 行；T0-7 在 A0 集成回写后行数会增加。本矩阵不代 A1 展开主文档卡，仅引用 `M5-20260906/` 目录成果。
- **A10 M5 安全复核已交付**：`A10-M5-security-review-20260906-1410.md`，设计层五问裁定全绿，登记局部债务 D-1/D-2/D-3（见 §6 衔接）。
- **本矩阵与 A1 M5-13 的关系**：A1 `M5-13-verification-matrix.md` 是横切验证矩阵卡（列需求/反向用例/责任 lane 候选 A11），本 A11 矩阵是 Lane 责任交付（可执行命令矩阵 + 实物取证），二者互补不冲突。本文件带时间戳且有实质内容，**非**空占位（A10 H-1 疑误读，A0 核对）。

### 3.5.2 各 lane 预研已定义的"机器可守门"码位（直接喂给验证脚本）

| 来源文档 | 码位前缀 | 条数 | 守门内容（摘要） |
|---|---|---|---|
| A5 `A5-M5-agent-skill-20260906-0800.md` §6 / §11 | `AGSK_*` | 9 | 无第二执行路径 / 每命令进 ACL / SkillImpl 无 Inline / 凭据不出 / skill-runs.json 明细 / 容量上限 / 安装默认禁用+checksum / 双阶段确认 / pre-merge 覆盖 |
| A7 `A7-M5-graph-core-20260906-0755.md` §6 / §7 | `GRAPH_*` | 见 §6（含 `GRAPH_CAP_EXCEEDED` 等） | 容量上限断言 / 派生不覆盖主数据 / 隐私过滤 / 审计低频 / 查询深度≤2 / 稳定 id 幂等 |
| A7 开放项 | `G-D1~G-D7` | 7 | 图库位置 A/B / QueryLimiter 提升 pub / 审计低频 / 迁移框架 / policy 挂 pre-merge / flush 注册序 / ai 出网闸门 |
| A3 `A3-M5-mcp-20260906-0757.md` | `MCP_*` | 见 §8（MCP_NPM_SDK_PRESENT / MCP_NODE_RUNTIME_PRESENT / read_only / allow_dangerous_sql / allowed_connection_ids） | 纯 Rust rmcp 禁 npm 分包 / stdio-only 禁 TCP / 复用 security_policy / McpGlobalPolicy fail-closed |
| A4 `A4-M5-a2a-memory-20260906-0755.md` | `F-A4-1~14` + `agent_kv` 错误码 -32013~-32018 | 14 条机器可检 | agent_kv 仅存 AI 上下文 / 键命名空间 / 容量 / 值大小 / 私密扫描 / 方言归一 / 离线降级 |
| A9 `A9-M5-plugin-system-prework-20260906-1100.md` §6 / §7 | `SB-1~SB-10`（红线）/ `K-*`（审计/凭据） | 10+ | 禁第二执行路径 / 形态①否决 / 全局 Tauri 关闭（form②）/ capabilities 清理 / 禁 npm/tokio 新依赖 / 卸载清理不删共享实体 / 声明式零代码加载 / can_invoke 7 步 fail-closed |
| A10 `A10-M5-security-review-20260906-1410.md` | `M5SEC_*` | 见 §6（M5SEC_POLICY_SCRIPT_MISSING / M5SEC_COMMAND_PARITY / M5SEC_WITHTAURI_TRACKING） | M5 设计层五问裁定 + 3 条局部债务 D-1/D-2/D-3 |

> **A11 提示**：上述码位是 M5 实现批的"验证契约"。A11 不写这些脚本（属 A0 指派实施 lane），但本矩阵把它们列为 M5 门禁的**必达项**——任一码位未落地 `pre-merge.sh`，对应 M5 lane 不具备 PASS。

---

## 4. 护栏 → 命令可追溯矩阵

### 4.1 M5 验收门禁（详细设计与实施计划.md §M5 验收门禁 578~584）→ 验证命令

| # | 护栏 | 验证命令 | 责任车道 |
|---|---|---|---|
| M5-G1 | 每个 crate/runtime 有稳定公开契约 + 依赖方向检查；迁移中主应用始终可构建 | `cargo test -p mvp-core` + T0-11/T0-12 + `npm run build` | A2/A13 |
| M5-G2 | MCP/A2A/Agent/Skill/插件全部复用同一安全策略；未声明/未确认/策略不确定 → fail-closed | AGSK_2/AGSK_8、GRAPH_*、SB-*、`check-mcp-policy.py`、复用 `security_policy` | A3/A4/A5/A7/A9 |
| M5-G3 | 恶意/损坏 Skill 与插件、超大输出、无限工具循环、连接中断、卸载残留均有测试与预算上限 | AGSK_6（max_tool_iters/token_budget）、GRAPH_CAP_EXCEEDED、`check-plugin-policy.py` 卸载清理、`cargo test agent_skill/graph/plugin` | A5/A7/A9 |
| M5-G4 | 图谱 AI 结果可追溯/可删除/可重建；派生数据不覆盖主数据；容量超限可降级 | GRAPH_*（派生不覆盖）/ G-D3（审计低频）/ G-D4（迁移）/ `cargo test graph` | A7/A17 |
| M5-G5 | M5-1~M5-12 严格按检查点提交；协议/安全/集成/恢复/性能/升级回滚测试全 PASS | T0 全套 + T1 全套 + T2-5 | A0 + 各 lane |

### 4.2 继承 M4 红线（G1~G12，M5 同样适用，不得退化）

| M4 编号 | 含义 | M5 映射 |
|---|---|---|
| G1 | 数据库写默认拒绝 | M5-2 MCP db 工具复用 `classify_sql_risk`/`is_production_database`（A5 §0.4） |
| G3 | 凭据走 Keyring，不出 DTO/审计/前端/文件/日志 | M5 扩展：LLM key 键 `llm:<provider>`（AGSK_4）；插件配置走 keyring 引用（A9 §0 K3） |
| G4 | SQL 结果行/字节上限 + 可取消 | M5-8 图存储复用 `QueryLimiter`/`QueryCancel`（G-D2） |
| G5 | 调度/执行复用 M2-4 通道，禁第二执行路径 | **M5 最高频红线**：Agent/Skill/插件/图谱维护任务全部复用 `script_runner::start_run`/`start_command`；AGSK_1 / SB-4 / G-D6 静态守门 |
| G6 | 退出经生命周期协调器 | M5 新增 `flush-graph`（G-D6，序在 kill-running-scripts 之后）；插件无独立进程（form③） |
| G7 | 新命令过 source check + 进 ACL | T1-a 三处同步 + 末条锚点（本矩阵 §T1-a） |
| G8 | 每产品代码 lane 必须新增/扩展 policy 脚本与测试 | M5 各 lane 的 `check-*-policy.py` 必须进 `pre-merge.sh`（AGSK_9 / GRAPH_*/ SB-*） |
| G9 | 三库各有连接/只读/断开/失败用例 | M5-2 MCP db 工具同口径 |
| G10 | 休眠/重启/错过/重复/时钟/损坏/重入 | M5-7/8 图谱维护任务复用 `task_*` |
| G11 | 定时执行复用 M2-4 + 写审计；退出无残留 | M5 同口径，T2-3 |
| G12 | 契约/集成/安全/M0 基线对比全 PASS | T0 + T2-5 |

### 4.3 与详细设计与实施计划.md §10 红线表的映射

| 红线（§10） | M5 能力 | 验证命令 |
|---|---|---|
| MCP 引入 Node 依赖（625 行） | M5-2 | `grep -rn 'npm' src-tauri/Cargo.toml` 空；`cargo tree -p mvp-core` 无 npm（T0-12） |
| Agent/Skill 绕过安全闸门（628 行） | M5-4/5 | AGSK_1/AGSK_2/AGSK_8 + `check-agent-skill-policy.py` |
| Skill 执行体任意代码（629 行） | M5-4/5 | AGSK_3（`SkillImpl` 无 Inline）+ 安装确认 `request_*`/`confirm_*` |
| 知识图谱引重依赖（630 行） | M5-7/8 | `cargo tree` 无 neo4j；首期 SQLite+邻接表（G-D1=A） |
| 图谱 AI 抽取越权出网（631 行） | M5-7 智能期 | G-D7（`graph_ai_extract` 审计 + 确认闸门 + Keyring） |
| 数据库凭据泄露（624 行） | M5-2 | 复用 M4 G3 + AGSK_4 |
| 大输出卡死 UI（626 行） | M5-4/6 | AGSK_6（输出字节截断复用 `ScriptRunRecord.truncated`）+ mpsc+pump 范式 |
| 定时任务残留 timer（627 行） | M5-7/8 | G-D6（flush 注册序）+ `check-lifecycle-contract.py` |

---

## 5. M5 预期新增 policy / 逻辑脚本清单与接入约定

> 沿用 `scripts/GATE-CONTRACT.md` 与 M4 20+ 脚本口径（参数面 / PENDING_CODES / 变异防呆 / 零新增依赖 / 接入 pre-merge 第 24 项起）。
> **A11 不写这些脚本**（属 A0 指派实施 lane）；本表把它们列为 M5 门禁的必达项。

| 脚本 | 码位前缀 | 归属 lane（实施） | 守门内容 |
|---|---|---|---|
| `scripts/check-core-boundary.sh` | —（bash 断言） | A2/A13（M5-1） | `grep -rl 'use tauri' crates/core/src` 空；`cargo tree -p mvp-core` 无 tauri/tokio/sqlx/diesel/neo4j（T0-11/T0-12） |
| `scripts/check-mcp-policy.py` | `MCP_*` | A3/A14（M5-2） | read_only / allow_dangerous_sql / allowed_connection_ids；禁 npm 分包；复用 security_policy |
| `scripts/check-agent-skill-policy.py` | `AGSK_*` | A5/A16（M5-4/5） | 9 条 AGSK 断言（§3.5.2） |
| `scripts/check-agent-skill-ui-logic.mjs` | — | A6/A19（M5-6） | Agent/Skill UI 逻辑断言 |
| `scripts/check-graph-policy.py` | `GRAPH_*` | A7/A17（M5-7/8） | GRAPH_* + G-D1~G-D7 处置 |
| `scripts/check-graph-ui-logic.mjs` | — | A8/A19（M5-9） | 图谱 UI 逻辑断言 |
| `scripts/check-plugin-policy.py` | `PLUGIN_*` / `SB_*` | A9/A18（M5-10/11） | SB-1~SB-8 红线 + 声明式零代码加载 + 卸载清理 |
| `scripts/check-plugin-ui-logic.mjs` | — | A19（M5-12） | 插件管理 UI 逻辑断言 |

**接入约定**（给 A0 / 实施 lane）：
1. 每个脚本必须含 `--self-test` / 默认 / `--expect-pending` 三模式，全进 `pre-merge.sh`（M4 末项号 23，M5 新增从**第 24 项**起，A0 统一接入，lane 不得自改 pre-merge.sh）。
2. 未实现码位用 `PENDING_CODES`，M5 实现批提交后转 DEFAULT（防"已实现却仍挂账"）。
3. 变异防呆：坏样本 token 不含原串子串、全量替换；键名与 `read_repo` 一致。
4. 码位前缀以各 lane 预研文档（A5 AGSK_*/A7 GRAPH_*/A9 SB_*）为冻结值，本矩阵不重排。

---

## 6. M5 债务编号延续规则与已知风险候选

> 继承 `logs/assist/M4-A11-debt-ledger-20260905-2240.md`（M4 债务止于 **D42**）。M5 新增债务从 **D43** 起。
> **A11 口径**：本矩阵只做"风险候选 → 验证命令"映射，**不代为正式登记 D 编号**（登记权属 A0，沿用 M4 台账"A11 只汇总不裁决"纪律）。下表为 M5 实现批落地后**应由 A0 正式登记的债务候选**，每条已带关闭验证命令。

| 候选编号 | 简述（来源） | 关闭验证命令 | 归属 |
|---|---|---|---|
| **D43**（候选） | core 抽取不得静默丢调用（A2 R6：M4 护栏随模块搬入 core 且测试随迁） | `cargo test -p mvp-core` + `git diff --check` | A2/A13 |
| **D44**（候选） | `scheduler → bridge::AppState` 反向边须在 M5-1 切片 2 解除（A2 R1，HIGH） | `grep -n 'crate::bridge' crates/core/src` 空（T0-11 同口径） | A2/A13 |
| **D45**（候选） | `ProgressSink`/`PathResolver`/`RootsProvider` 注入前，script_runner/scheduler 单测无法脱 Tauri（A2 R2） | `cargo test -p mvp-core script_runner` 脱离 AppHandle | A2/A13 |
| **D46**（候选） | 能力白名单两份漂移（Skill vs A2A 各写一份 → 须共用 `capability.rs`，A9 §8.2） | `grep -rn 'capability' src-tauri/src` 单一定义 + AGSK/ MCP 共用断言 | A3/A4/A5 |
| **D47**（候选） | 插件形态②可行性未证（`withGlobalTauri=true` 能否按 webview 关闭；A9 P-D1~D4，交 A0 裁定） | A0 裁定后补形态②隔离测试，或 form③ 落地 | A0/A18 |
| **D48**（候选） | 图谱审计刷爆 `audit.json`（1000 FIFO）：高频 `graph_rebuild` 须低频/独立（A7 G-D3 / A9 K5） | `cargo test graph` + `grep -c graph_rebuild audit.json` 低频断言 | A7/A17 |
| **D49**（候选） | 图谱 AI 抽取越权出网（G-D7：须 `graph_ai_extract` 审计+确认+Keyring，首期可仅留 trait） | G-D7 断言 + LLM key 走 `llm:<provider>` | A7（智能期） |
| **D50**（候选） | 插件卸载残留/共享实体误删（SB-8 清理纪律；uninstall 不删 ScriptMeta/ToolMeta/SkillDef） | `cargo test plugin_perm_` 卸载清理用例 | A9/A18 |
| **D51**（候选） | MCP 误引 Node/重依赖（§10 红线 625；内嵌纯 Rust rmcp、禁 npm 分包） | T0-12 `cargo tree` 无 npm + `grep npm src-tauri/Cargo.toml` 空 | A3/A14 |
| **D52**（候选） | Agent 无限工具循环 / 超大输出（AGSK_6：max_tool_iters / token_budget / 输出字节截断） | `cargo test agent_skill` 容量单测 | A5/A16 |
| **D53**（候选，承接 A10 D-1） | M5 策略脚本未落地（A10 H-1/D-1：A2~A9 各自须出 `check-*-policy.py` 且进 `pre-merge.sh`，当前全部 PENDING） | `python3 scripts/check-{core-boundary,mcp,agent-skill,graph,plugin}-policy.py --self-test` 全转 DEFAULT + 进 pre-merge.sh 第 24+ 项 | A0（统一接入） |
| **D54**（候选，承接 A10 D-2，与 D46 合并） | M5 新命令三处 parity 漂移（A10 D-2：main.rs/ACL/types.ts 任一处漏） | T1-a 三处同步核查全 OK + 末条锚点 | A0 + 各 lane |
| **D55**（候选，承接 A10 D-3，与 D47 合并） | `withGlobalTauri=true` 跟踪（A10 D-3：form② 全局 Tauri 是否按 webview 关闭，未证） | 同 D47（合并跟踪） | A0/A18 |

> **编号纪律**：M5 实现批正式登记时，从 D43 起顺序分配。A10 M5 安全复核登记的局部债务 **D-1/D-2/D-3** 与本候选表衔接为 **D53 / D54(并入 D46) / D55(并入 D47)**，最终全局编号由 A0 在 M4 全局债务台账（D42 之后）裁定。若 A0 在 M5 展开前已就 M4 遗留项新增 D43+，本候选表须与正式台账合并去重（以 A0 最终 `M4-A11-debt-ledger` 更新版 + A1 `M5-20260906/M5-14-debt-ledger.md` 为准）。

---

## 7. A11 判定 / 收口顺序（M5 实现波）

按 board merge order，M5 实现批落地后由 A0 逐条收口，每收一条跑 **T0 全套 + 该 lane 的 T1**：

1. A1 展开 M5-1.a~M5-12.b 子卡（当前未展开）→ 2. A2/A13（M5-1 core）+ A3（M5-2）+ A4（M5-3）→ 3. A5/A16（M5-4/5）+ A7/A17（M5-7/8）+ A9/A18（M5-10/11）→ 4. A6/A8/A19（M5-6/9/12 UI）→ 5. A10（M5 安全复核）→ 6. **A11 复跑全矩阵（T0 + 全部 T1 + T2-1/T2-3/T2-4）** → 7. A0 push。

**A11 在 M5 实现波第 6 步的终态证据**：每条命令 + 原文结果 + HEAD SHA + 环境指纹；`NOT_RUN(实现未到)` / `NOT_RUN(环境不备)` 项必须写明缺什么，不得留空、不得填 PASS。

---

## 8. 判定口径（沿用 M4 纪律，不新发明）

- 证据三要素：**命令 + 结果 + 环境指纹**；三者缺一不构成证据。
- GUI 类用例未目视验收的一律挂账（D20/D23 口径），**不伪造 PASS、不代签**。
- 单测计数只增不减；若删测试需写替代覆盖。
- `cargo check` warning 现状 2 类（`grid_process.rs`），M5 不得新增第 3 类（T0-4）。
- 二进制体积增长 > A0 书面阈值 16% 须阻断或形成书面豁免（T0-5/T0-6）。

---

## 9. 未覆盖声明与风险

1. 本包**不修改** `scripts/pre-merge.sh` 与任何产品代码；新增项只给脚本名建议（§5），实施归 A0 指派 lane。
2. **A3（M5-2）/ A4（M5-3）M5 prework 文档缺失**：本矩阵对 M5-2/M5-3 的验证要点为推断（基于 §10 红线 + board），待 A3/A4 文档落地后由 A11 补登；当前记 `NOT_RUN`。
3. **A1 M5 子卡未展开**：WBS 仍 50 行；T0-7 在展开后行数增加，本矩阵不代 A1 展开。
4. M5 全部为"前瞻预案"，无真机取证；所有 T1 定向命令在 M5 实现卡落地前记 `NOT_RUN(实现未到)`。
5. A11 交付物为工作树新增文件（`logs/**`），不构成对其他 lane 的脏工作树阻塞；其他 lane 启动门禁应把 `logs/**` 未跟踪文件视为机器证据。
6. **本包不代为裁决**：M5 形态②裁定（D47）、M4 遗留债务（D28/D30/D37 等）、core 切片边界，全部只做"事实记录 + 取证命令 + 影响面"，结论权归 A0。
7. 本包未复跑 `cargo test`/`npm run build`/`pre-merge.sh`；BASE 以 A0 最新 spot-check（board §Integration Fix Wave：328 passed、`pre-merge` FAIL、IF-1~IF-4 红绿灯）为准，不沿用初始 M5 Dispatch 的 329/ALL_PASS 过时声明（回应 IF-5「A11 报告 stale」）。M5 无产品代码改动，复跑无意义；A0 在 IF-1~IF-4 清零后重跑 T0。
8. 本次只写 `logs/checkpoints/M5-A11-verification-matrix-20260906-0820.md` + `logs/assist/M5-A11-debt-ledger-20260906-0820.md` + `logs/assist/M5-A11-gui-manual-checklist-20260906-0820.md`；**未触碰任何产品代码、未改 `scripts/`、未改三份主文档、未提交、未 push**。

---

## 10. Lane Output Template（本车道回填）

```text
LANE=A11
STATUS=PASS_WITH_DEBT（M5 前瞻预案；A3/A4 M5 文档缺失 + M5 实现未到，记 NOT_RUN）
BASE=5ca8f9f（本地 master，领先 origin/master 1；M4 已集成推送 a1a2061）
HEAD=docs only（logs/checkpoints/M5-A11-verification-matrix-20260906-0820.md + logs/assist/M5-A11-debt-ledger-20260906-0820.md + logs/assist/M5-A11-gui-manual-checklist-20260906-0820.md）
FILES=上述 3 个新增文档（均 logs/**，零产品代码）
VERIFY=
  git status --short --branch        -> master 领先 1；未跟踪 = 各 lane M5 prework 文档
  python3 scripts/check-plan-routing.py -> ok (50 WBS rows)
  （M5 定向命令 T1 全 NOT_RUN：实现卡未签署）
  （M4 BASE 最新 spot-check：cargo test 328 passed、pre-merge FAIL（IF-1~IF-4）；引 board §Integration Fix Wave，非初始 dispatch 的 329/ALL_PASS）
CHECKPOINT=logs/checkpoints/M5-A11-verification-matrix-20260906-0820.md
MERGE_NOTES=
  1. M5 Wave 0 全 DOCS ONLY；本包零产品代码，不 push。
  2. A3(M5-2)/A4(M5-3) M5 prework 文档缺失，验证要点为推断，待补登。
  3. A1 M5 子卡未展开（WBS 50 行）；T0-7 展开后行数增加。
  4. M5 债务从 D43 起延续（候选 D43~D52 已映射验证命令，待 A0 正式登记）。
  5. M5 预期 policy 脚本（check-core-boundary/check-mcp-policy/check-agent-skill-policy/check-graph-policy/check-plugin-policy 等）列为必达项，实施归 A0 指派 lane。
NEXT=A1 展开 M5 子卡 → A0 签署 M5-1.a 等实现 dispatch → 各 lane 实施批落地后 A11 复跑全矩阵
```
