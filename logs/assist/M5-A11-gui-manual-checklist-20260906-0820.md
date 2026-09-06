# M5 人工验收清单（GUI / 真机）· Lane A11

> 日期：2026-09-06 08:20 CST ｜ 归属：`logs/assist/`（过程性材料，结论回填 `logs/checkpoints/M5-A11-verification-matrix-20260906-0820.md`）
> 定位：**T2 层**，命令矩阵跑不到的部分在此人工取证。M5 Wave 0 全为 DOCS ONLY，本清单为**前瞻预案**，所有用例当前记 `NOT_RUN(实现未到)`。A1 已完成 M5 子卡展开（`logs/checkpoints/M5-20260906/`），A3/A4/A10 M5 文档已交付；用例设计已吸收其红线（MCP_*/F-A4-*/AGSK_*/GRAPH_*/SB-*/M5SEC_*），但真机取证仍待 M5 实现批落地。
> 硬规则：**每条只允许填 `PASS` / `FAIL` / `NOT_RUN`。环境不具备写 `NOT_RUN` + 缺什么；未目视不得填 `PASS`（沿用 D20 / D23 口径）**。
> 用例来源：M5 验收门禁（详细设计与实施计划.md §M5 验收门禁 578~584）+ 各 lane M5 prework 红线（AGSK_*/GRAPH_*/SB-*）+ M4 G1~G12 继承。
> 对齐：`logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md`（M4 终版清单，本文件沿用其 A/D 组框架）。

## 0. 通用约定

- 状态列：`PASS` = 已目视且行为符合判据；`FAIL` = 已目视且不符合（必须附截图/输出并开缺陷）；`NOT_RUN` = 未执行（必须写原因，如 `NOT_RUN(实现未到)` / `NOT_RUN(环境不备)`）。
- 每条至少留一种取证：截图 / 终端输出 / 文件片段（三选一）。
- 执行者填在表尾「验收记录」，含：时间、机器指纹（`uname -a`）、HEAD SHA（`git rev-parse --short HEAD`）。
- 退出后统一跑一次残留检查（D 组）。

---

## A. 环境准备（先做，不备齐就不往下走）

| # | 项目 | 准备动作 | 通过判据 |
|---|---|---|---|
| A-1 | 构建产物 | `npm run build` + `bash run-gui.sh` 可启动 | 应用可启动，无启动错误日志 |
| A-2 | core 边界（M5-1） | M5-1 落地后确认 `crates/core/src` 无 `use tauri` | `grep -rl 'use tauri' crates/core/src` 空（T0-11） |
| A-3 | MCP 测试服务（M5-2） | 准备一个本地 MCP server 或 rmcp 内置工具端点 | 可连通（无 → M5-2 条目记 NOT_RUN） |
| A-4 | LLM 测试账号（M5-4/7 智能期） | 准备一个 LLM provider key（走 `llm:<provider>` Keyring） | key 可注入 Keyring（无 → 智能期条目记 NOT_RUN） |

---

## B. 核心 workspace 下沉（M5-1 / A2/A13）

| # | 用例 | 步骤 | 通过判据 | 归属 |
|---|---|---|---|---|
| B-1 | 主应用仍可构建运行 | M5-1 切片后 `npm run build` + `cargo build` + 启动 | 功能无回归；core 抽取不破坏现有命令 | A2/A13 |
| B-2 | core 边界零 Tauri | `grep -rl 'use tauri' crates/core/src` | 空；`cargo tree -p mvp-core` 无 tauri（T0-11/T0-12） | A2/A13 |
| B-3 | 回归不丢调用 | 抽模块后跑全量 `cargo test` | 计数 ≥ 迁移前；M4 护栏测试随迁通过 | A2/A13 |

---

## C. MCP / 全局护栏（M5-2 / A3/A14）

| # | 用例 | 步骤 | 通过判据 | 归属 |
|---|---|---|---|---|
| C-1 | 禁 npm 分包 | 查 `src-tauri/Cargo.toml` 与 `cargo tree` | 无 npm 依赖；纯 Rust rmcp | A3/A14 |
| C-2 | read_only 默认拒绝写 | 经 MCP 发写 SQL | 默认拒绝；`allow_dangerous_sql` 未开则拒 | A3/A14 |
| C-3 | 复用 SQL 分类器 | MCP db 工具走 `classify_sql_risk`/`is_production_database` | 与 M4 G1/G2 同判据 | A3/A14 |

---

## D. Agent / Skill（M5-4/5/6 / A5/A16/A6/A19）

| # | 用例 | 步骤 | 通过判据 | 归属 |
|---|---|---|---|---|
| D-1 | 无第二执行路径 | 审查 `skill_runtime.rs`/`agent_runtime.rs` | `grep -nE 'std::process\|Command::new\|sh -c'` 为 0（AGSK_1） | A5/A16 |
| D-2 | SkillImpl 无 Inline | 安装一个含 `Inline{code}` 的 Skill | 拒绝/类型断言失败（AGSK_3） | A5/A16 |
| D-3 | 凭据不出 DTO/审计 | Agent 对话 / Skill 运行后查 audit.json | 无 LLM key / 私密参数明文（AGSK_4） | A5/A16 |
| D-4 | 容量上限 | 触发超大输出 / 无限工具循环 | `max_tool_iters` / `token_budget` / 输出字节截断生效（AGSK_6） | A5/A16 |
| D-5 | 双阶段确认 | 安装/执行危险 Skill | `request_*`/`confirm_*` 未 confirm 前不 invoke（AGSK_8） | A5/A16 |
| D-6 | 运行明细独立 | 跑若干次 Skill | 明细落 `skill-runs.json`（非 audit.json），不刷爆（AGSK_5） | A5/A16 |

---

## E. 知识图谱（M5-7/8/9 / A7/A17/A8/A19）

| # | 用例 | 步骤 | 通过判据 | 归属 |
|---|---|---|---|---|
| E-1 | 派生不覆盖主数据 | 建图后查主库（Artifact/Script/Task） | 主数据未被图写覆盖 | A7/A17 |
| E-2 | 容量/隐私上限 | 注入超量节点/边、私有节点查询 | `GRAPH_CAP_EXCEEDED` 触发；隐私过滤在查询层（Agent 默认 public） | A7/A17 |
| E-3 | 可追溯/可删/可重建 | 删一条 AI 抽取边 / 重建图 | 操作可逆；重建后一致 | A7/A17 |
| E-4 | 图谱维护任务复用 task_* | 注册内部 `graph_rebuild` 任务 | 随 `stop-scheduler` 收口；flush 序在 kill-running-scripts 之后（G-D6） | A7/A17 |
| E-5 | 退出无残留 | 有图连接/维护任务时退出 | `ps` 无残留（T2-3） | A7/A17 |

---

## F. 插件系统（M5-10/11/12 / A9/A18/A19）

| # | 用例 | 步骤 | 通过判据 | 归属 |
|---|---|---|---|---|
| F-1 | 声明式零代码加载 | 安装一个 form③ 插件 | 不加载外部代码、不开新进程（SB-4） | A9/A18 |
| F-2 | 执行复用 M2-4 | 启用插件后触发其 entry | 经 `script_runner::start_run`/`start_command`（无第二路径） | A9/A18 |
| F-3 | 安装默认禁用 | 安装插件 | 默认 `Disabled`，需显式 enable + 确认（SB-7/AGSK_7） | A9/A18 |
| F-4 | 卸载清理不误删 | 卸载插件 | 不删共享 ScriptMeta/ToolMeta/SkillDef（SB-8/D50） | A9/A18 |
| F-5 | 权限分级/确认 | 启高权限插件 | 确认闸门 + 审计（K-*） | A9/A18 |
| F-6 | 插件审计不刷爆 | 高频 `plugin_invoke` | 走独立 `plugin-audit.json`（K5） | A9/A18 |

---

## G. 退出与残留（每次真机验收收尾必跑）

```bash
ps -eo pid,pgid,cmd | grep -c "[m]vp-browser-os"     # 期望 0（M5 不得引入长生命周期子进程）
cat ~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/audit.json   # 期望无凭据/LLM key/私密参数明文
git status --short                                    # 期望无意外落盘文件（M5 新增持久化须是已知路径：skills.json/agents.json/plugins/）
```

| # | 检查 | 通过判据 |
|---|---|---|
| G-1 | 进程与进程组 | 无残留进程 / 进程组（form① 已否决，form③ 天然满足） |
| G-2 | 审计脱敏 | 全文无密码 / token / LLM key / 连接串 / 私密参数 |
| G-3 | 落盘面 | 新增持久化文件仅限 M5 冻结路径（skills.json/agents.json/plugins/），不含凭据明文 |
| G-4 | 工作树 | 验收动作不产生未申报的文件改动（`logs/**` 视为机器证据） |

---

## 验收记录（执行者填写）

| 时间 | 机器指纹（`uname -a`） | HEAD SHA | 执行者 | A 组 | B 组 | C 组 | D 组 | E 组 | F 组 | G 组 | 结论 |
|---|---|---|---|---|---|---|---|---|---|---|---|
|  |  |  |  | NOT_RUN | NOT_RUN | NOT_RUN | NOT_RUN | NOT_RUN | NOT_RUN | NOT_RUN | 全部 NOT_RUN(实现未到) |

结论口径：任一 `FAIL` → 该 lane 不具备 PASS；存在 `NOT_RUN` → 标记 `PASS_WITH_DEBT` 并在债务台账登记（见 `logs/assist/M5-A11-debt-ledger-20260906-0820.md`）。
