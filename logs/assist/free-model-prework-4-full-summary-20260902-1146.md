# free-model-prework-4-full 汇总（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 目标：趁免费额度充足，批量完成 M2/M3/M4/M5 后续强模型实现前的**低风险前置工作**
> 产出：**22 个 Markdown 文档（21 任务 + 1 汇总），零代码改动，零文档删除，零主任务 PASS**

---

## 0. 批次执行声明（先看这段）

| 约束 | 本批次实际执行情况 |
|---|---|
| 不移动 NEXT | ✅ **未移动**。仓库内无任何 NEXT 标记文件/字段（见 `repo-sanity-audit` §4），本批次也未创建 NEXT 相关产物 |
| 不签 M2/M3/M4/M5 任一主任务 PASS | ✅ **未签任何 PASS**。21 份任务文档全部标注「任务卡 / 草案 / 未实现 / 未执行验收命令」 |
| 不改 `src-tauri` 安全/生命周期/进程核心代码 | ✅ **零改动**（见 §6 工作树核验） |
| 不实现真实脚本执行 / 插件运行时 / 终端核心 / 数据库迁移 | ✅ 均未实现，仅产出契约与任务卡 |
| 不执行破坏性 Git 命令 | ✅ 仅 `git add` / `git commit`（10 个） |
| 不伪造测试结果 | ✅ 所有「验收命令」均标注**未执行**；仅「静态 ls/grep 复核」标注为已执行并给出回显 |
| 不创建替代主文档掩盖文件不存在 | ✅ `AI-模型切换与接手清单.md` 等缺失项**只登记不代建**（见 §2） |
| 不删除现有文档 | ✅ 零删除 |
| `git diff --check` PASS | ✅ 退出码 0（见 §6） |
| 每类任务独立 commit（或相邻合组且说明清楚） | ✅ **9 个** commit（8 个任务组 + 1 个汇总），每个说明含任务号与内容 |
| 工作树干净 | ✅ `git status --porcelain` 无输出（见 §6） |
| 21 个任务全部有独立文档或明确 BLOCKED 原因 | ✅ 21/21 有独立文档；无 BLOCKED（但有 3 个**决策阻塞项**见 §5） |

---

## 1. 仓库口径（起点核验，2026-09-02 11:46 实测）

```bash
$ pwd
/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

$ git status --short --branch
## master...origin/master [领先 16]

$ git diff --check; echo $?
0
```

| 项 | 值 |
|---|---|
| 分支 | `master`（上游 `origin/master`，本地**领先 16 commit**） |
| 起点工作树 | 干净 |
| 最近提交 | `b1a0810 docs(free-prework): M2-M5 前置工作汇总…`（上一批次 `free-model-prework-M2-M5`） |
| `logs/assist/` 既有文档 | 15 份（1055 批次） |
| 本批次新增 | 22 份（1146 批次） |

---

## 2. ❌ 关键文档缺失（**已记录实测输出，未代建**）

| 缺失项 | 探测命令与实测结果 | 处置 |
|---|---|---|
| `AI-模型切换与接手清单.md` | `find . -maxdepth 3 -name "AI-模型切换与接手清单.md"` → **零命中** | ❌ 不代建；编号口径校准因此仍阻塞 |
| `logs/assist/assist-index-*.md` | 同 → **零命中** | ✅ 本批次**新建** `docs-index-recovery-20260902-1146.md`，并明确标注为「新建索引」而非恢复既有文档 |
| `logs/assist/high-risk-acceptance-matrix-*.md` | `ls logs/assist/` → 零命中 | ❌ 不代建；本批次以 `model-routing-matrix` + `acceptance-script-drafts` 覆盖其**部分**职能，高风险验收矩阵**仍缺失** |
| `logs/assist/low-model-third-batch-summary-*.md` | 零命中 | ❌ 不存在；上一批次汇总实为 `free-model-prework-M2-M5-20260902-1055.md` |

**存在**的主文档：`详细设计与实施计划.md`（38 KB）、`后续需求TODO.md`（32 KB）、`PROJECT-RULES.md`、`logs/baseline-2026-08-27.md`。

---

## 3. 🚨 本批次复核实测的关键现状（6 项勘误全部二次确认）

> 全部基于 2026-09-02 11:46 前后执行的真实命令回显，已整理成任务卡：`errata-to-taskcards-20260902-1146.md`。

| ID | 勘误 | 关键证据 | 影响 |
|---|---|---|---|
| **E1** | 5 个种子工具 HTML **不存在** | `ls src-tauri/src/tools` → `No such file or directory`；`find -name "*tool*.html"` 零命中 | M2 排期需重算：是「从零写 5 个 HTML」而非「补加载框架」 |
| **E2** | `term_resize` **空实现** | `bridge.rs:2013-2016`：`let _ = (app, id, cols, rows); Ok(())` | `vim`/`top`/`htop` 错位 |
| **E3** | 前端 `termResize` **零调用方** | `grep -rn "termResize" src/` 仅命中 `bridge.ts:208-209` 的定义 | 与 E2 必须**同 PR** |
| **E4** | 主窗关闭**不杀 PTY** | `main.rs:591` 只 `grid_manager.shutdown_all()`；`.run()` 无 `RunEvent` 分支；`process::exit` 共 **8 处** | **P0，阻塞 5 个下游任务** |
| **E5** | `withGlobalTauri = true` | `tauri.conf.json`；且 `csp: null`、59 个高危命令暴露 | 安全红线；决定 HTML 工具加载方式与形态②插件可行性 |
| **E6** | `strip-ansi-escapes` **死依赖** | `Cargo.toml` 声明但 `src-tauri/src` 零引用 | 清理类，低风险 |

**新增实测事实**（补充上一批次）：

| 项 | 值 |
|---|---|
| `default-commands.toml` 命令数 | **59**（含 `term_*`，无 `list_tools` / `run_script` / `task_*` / `skill_*` / `plugin_*` / `graph_*` / `a2a_*`） |
| `TerminalSession` 字段 | 仅 `writer` + `child`，**无 `master`** → resize 无法落地 |
| `capabilities/default.json` | `windows: ["main","browser"]`；`shell:allow-spawn` 的 `args: true`（**ACL 不校验参数**） |
| `capabilities/browser-remote.json` | `remote.urls = ["https://*","http://*"]` |
| `workspace.rs` 审计上限 | `audit.json` **1000 条** |
| 前端 | 22 个 `.vue`；依赖仅 `vue`/`pinia`/`@tauri-apps/api`/`plugin-shell`/`@xterm/*`；**无路由、无 UI 库、无测试框架** |
| `main.rs` 常驻线程挂载点 | `:598-605`（layout enforcer / hibernation sweeper / grid load retry） |

---

## 4. 交付清单（22 份文档）

| # | TASK | 文档 | 大小级别 | 核心内容 |
|---|---|---|---|---|
| 1 | repo-sanity-audit | [repo-sanity-audit-20260902-1146.md](./repo-sanity-audit-20260902-1146.md) | 中 | pwd/branch/提交/文档存在性/NEXT 来源 + 6 类实测证据 |
| 2 | docs-index-recovery | [docs-index-recovery-20260902-1146.md](./docs-index-recovery-20260902-1146.md) | 中 | M1~M5 分类索引 + 🔴必读/🟡可选/⚪归档/⛔过期 四色标注 + 6 条口径冲突清单 |
| 3 | errata-to-taskcards | [errata-to-taskcards-20260902-1146.md](./errata-to-taskcards-20260902-1146.md) | 大 | E1~E6 六张卡，每卡含 目标/证据/必改文件/禁止事项/实现要点/反向用例/验收命令/失败动作/推荐模型 |
| 4 | M2-tools-seed-html-prework | [M2-tools-seed-html-prework-20260902-1146.md](./M2-tools-seed-html-prework-20260902-1146.md) | 大 | 5 个 HTML 从零落盘：路径规划、HTML 骨架契约、CSS/JS 约束、离线红线、输入输出契约、错误态/空态矩阵、验收命令 |
| 5 | M2-image-ui-static-shell | [M2-image-ui-static-shell-20260902-1146.md](./M2-image-ui-static-shell-20260902-1146.md) | 大 | 画廊/灯箱/缩放/旋转/删除确认/键盘表/mock/组件拆分/14 个截图点 |
| 6 | M2-script-library-ui-static-shell | [M2-script-library-ui-static-shell-20260902-1146.md](./M2-script-library-ui-static-shell-20260902-1146.md) | 大 | 列表/参数弹窗/危险参数二次确认/6 态状态机/输出区/历史 mock/审计位/14 截图点 |
| 7 | M2-tool-library-ui-static-shell | [M2-tool-library-ui-static-shell-20260902-1146.md](./M2-tool-library-ui-static-shell-20260902-1146.md) | 大 | 工具卡片/分类搜索/离线三态/6 种状态徽标/启动失败态/权限提示/详情面板/15 截图点 |
| 8 | M3-terminal-resize-taskcard | [M3-terminal-resize-taskcard-20260902-1146.md](./M3-terminal-resize-taskcard-20260902-1146.md) | 中 | `TerminalSession.master` 方案、`proposeDimensions` 链路、80 ms 节流去重、静默失败策略、vim/top 反向用例 |
| 9 | M3-terminal-history-taskcard | [M3-terminal-history-taskcard-20260902-1146.md](./M3-terminal-history-taskcard-20260902-1146.md) | 中 | 5000 行上限、敏感过滤四方案比选（选「不落盘+明示」）、不落盘红线、清空入口、8 个测试夹具 |
| 10 | M3-terminal-shutdown-taskcard | [M3-terminal-shutdown-taskcard-20260902-1146.md](./M3-terminal-shutdown-taskcard-20260902-1146.md) | 大 | `ShutdownCoordinator` 契约、7 类注册项、6 条退出路径矩阵、进程组回收、**5 个下游阻塞清单** |
| 11 | plugin-permission-taskcard | [plugin-permission-taskcard-20260902-1146.md](./plugin-permission-taskcard-20260902-1146.md) | 大 | `withGlobalTauri` 整改：步骤 0 可行性判定、CSP、capability 拆分、权限分级、错误页、审计 |
| 12 | script-execution-safety-taskcard | [script-execution-safety-taskcard-20260902-1146.md](./script-execution-safety-taskcard-20260902-1146.md) | 大 | 禁 `sh -c`、argv 数组传递、cwd 锁定、`env_clear`、fail-closed 参数校验、双超时、`setsid`+`killpg`、背压、审计脱敏 |
| 13 | scheduled-task-taskcard | [scheduled-task-taskcard-20260902-1146.md](./scheduled-task-taskcard-20260902-1146.md) | 大 | 调度模型、幂等键 `task_id:unix_secs`、重试退避、misfire 三策略（CatchUp 上限 3）、取消、审计拆分、原子写 |
| 14 | database-schema-taskcard | [database-schema-taskcard-20260902-1146.md](./database-schema-taskcard-20260902-1146.md) | 大 | schema 草案（7 表）、迁移策略（checksum/不可变/单事务）、回滚（备份而非 down）、旧 JSON 兼容、`VACUUM INTO` 备份、6 个验收脚本草案 |
| 15 | A2P-A2A-protocol-taskcard | [A2P-A2A-protocol-taskcard-20260902-1146.md](./A2P-A2A-protocol-taskcard-20260902-1146.md) | 大 | JSON-RPC 2.0 消息结构、能力声明、权限判定 5 层、超时/硬超时、`$/cancel`、12 个平台错误码、审计拆分 |
| 16 | agent-skill-contract-taskcard | [agent-skill-contract-taskcard-20260902-1146.md](./agent-skill-contract-taskcard-20260902-1146.md) | 大 | SkillMeta/SkillInstall、输入输出契约、权限判定、**禁 `SkillImpl::Inline`**、安装不自动启用、版本 semver、10 种失败态、审计 |
| 17 | graph-model-taskcard | [graph-model-taskcard-20260902-1146.md](./graph-model-taskcard-20260902-1146.md) | 大 | 11 种节点/10 种边/GraphSource 追溯/稳定 id 规则/增量事件/查询接口/隐私四级过滤/8 项容量硬上限 |
| 18 | plugin-runtime-taskcard | [plugin-runtime-taskcard-20260902-1146.md](./plugin-runtime-taskcard-20260902-1146.md) | 大 | 两种形态比选（形态①为主线）、`.mvpx` 包结构、manifest、权限分级、6 类隔离边界、生命周期、zip-slip 防护 |
| 19 | acceptance-script-drafts | [acceptance-script-drafts-20260902-1146.md](./acceptance-script-drafts-20260902-1146.md) | 大 | **35 个脚本草案**（M2×5 / M3×6 / M4×9 / M5×8 / 通用×7），每个含 检查什么/输入/退出码/失败信息/是否 CI 化；**27 个可 CI 化** |
| 20 | model-routing-matrix | [model-routing-matrix-20260902-1146.md](./model-routing-matrix-20260902-1146.md) | 中 | 4 档路由矩阵、快模型可做清单（9 项）、**只能强模型做清单（10 项）**、必须人工清单（8 类）、7 步路由速查、执行顺序建议 |
| 21 | strong-model-minimal-read-list | [strong-model-minimal-read-list-20260902-1146.md](./strong-model-minimal-read-list-20260902-1146.md) | 中 | 全局必读 6 项 + 每个主任务 3~8 项 + **全局不必读清单（13 项）** + 5 条硬提示 |
| — | **汇总（本文）** | [free-model-prework-4-full-summary-20260902-1146.md](./free-model-prework-4-full-summary-20260902-1146.md) | — | 本文 |

---

## 5. 🧭 三个决策阻塞项（**必须由人工拍板，AI 不得替决策**）

| # | 决策 | 出处 | 影响面 | 建议 |
|---|---|---|---|---|
| **B1** | **存储选型（SQLite 与否）** | `database-schema-taskcard` §9 D1~D4 | M4-1~M4-4 数据库 **+** M5-7~M5-9 图谱（两处共用同一决策） | 建议 `rusqlite` bundled；但**未拍板前两者都不得开工** |
| **B2** | **形态②插件可行性**（Tauri v2 能否按 webview 粒度关闭 `withGlobalTauri`） | `plugin-permission-taskcard` §5 步骤 0 | 需求 #2 工具加载方式 **+** M5-10~M5-12 插件形态 | 当前已知 `withGlobalTauri` 是 **app 级**配置；若无法按 webview 关闭 → **形态②插件 BLOCKED**，转形态①（独立进程/stdio） |
| **B3** | **编号口径校准** | 缺 `AI-模型切换与接手清单.md` | 批次任务号（`M2-1.a`）与仓库 WBS（`M2-8`）不同源 | 建议优先补齐该清单，再让强模型对齐 |

> 以上三项**均未在本批次代为决定**，只给出结论所依赖的事实与建议。

---

## 6. 工作树与提交核验

### 6.1 提交清单（10 个，前缀 `docs(free-prework-4)`）

```
b0ac3b0 docs(free-prework-4): TASK19-21 验收脚本草案清单(35 个/27 可 CI 化) + 模型路由矩阵 + 强模型最小阅读清单
14f9b5c docs(free-prework-4): TASK15-18 M5 四卡（A2P/A2A 协议、Agent-Skill 契约、图谱数据模型、插件运行时）
360f540 docs(free-prework-4): TASK13-14 M4 两卡（定时任务调度/幂等/错过补偿 + 数据库 schema 与迁移回滚，仅文档不执行迁移）
8549055 docs(free-prework-4): TASK11-12 安全两卡（withGlobalTauri 插件隔离整改 + 脚本执行安全/禁 sh -c/进程组回收）
75e380c docs(free-prework-4): TASK8-10 M3 终端三卡（resize 整改/历史与不落盘/PTY 生命周期 ShutdownCoordinator）
294d875 docs(free-prework-4): TASK4-7 M2 前置（5 个种子 HTML 从零任务卡 + 图片/脚本库/工具库三份 UI 静态壳方案）
05934b0 docs(free-prework-4): TASK3 六项勘误转强模型任务卡（E1 种子工具缺失/E2-E3 resize/E4 PTY 收口/E5 withGlobalTauri/E6 死依赖）
11eba62 docs(free-prework-4): TASK1 仓库口径核对 + TASK2 assist 文档索引（M1-M5 分类/必读·可选·归档·过期标注）
```

（共 8 个任务 commit + 1 个汇总 commit 待提交 + 起始时已有上一批次 commit）

### 6.2 核验命令与结果

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 6.2.1 零代码改动（应为空）
git status --porcelain -- src/ src-tauri/ package.json vite.config.ts
# → 无输出 ✅

# 6.2.2 git diff --check（应无输出 = PASS）
git diff --check; echo "exit=$?"
# → exit=0 ✅

# 6.2.3 产出物计数
ls -1 logs/assist/*-20260902-1146.md | wc -l     # → 22（含本文）

# 6.2.4 索引覆盖度（应无 MISSING 输出）
for f in logs/assist/*-20260902-1055.md; do
  n=$(basename "$f")
  grep -q "$n" logs/assist/docs-index-recovery-20260902-1146.md || echo "MISSING-IN-INDEX: $n"
done
# → 无输出 ✅（15 份 1055 批次文档全部入索引）
```

---

## 7. 跨任务阻塞依赖（**强模型排期硬约束**）

```
【第 0 步·人工决策】
  B1 存储选型 ──────────────┬──> M4-1~M4-4 数据库
                            └──> M5-7~M5-9 图谱
  B2 形态②可行性判定 ───────┬──> #2 工具加载方式
                            └──> M5-10~M5-12 插件形态
  B3 编号口径校准 ──────────> 全部任务

【第 1 步·P0】
  E4 / M3-1 退出收口 ShutdownCoordinator
     ├──> M2-1/M2-2 脚本执行（无收口 = 新泄漏源）
     ├──> M4-1~M4-4 数据库连接池关闭
     ├──> M4-5~M4-8 定时任务 timer + 被触发进程
     ├──> M5-4~M5-6 Agent 流式任务 / Skill
     └──> M5-10~M5-12 插件资源释放

【第 2 步·低风险可并行】
  E1 3 个简单种子工具 / 三份 UI 静态壳 / E6 死依赖清理 / M3-3 终端历史

【第 3 步】
  M2-1/M2-2 run_script 执行通道 ──> M4-5~M4-8 定时任务 ──> M4-1~M4-4 数据库

【第 4 步】
  E2 + E3 term_resize（必须同 PR）

【第 5 步】
  E5 / 插件权限整改（withGlobalTauri + CSP + capability 拆分）

【第 6 步·风险递增】
  M5-1~M5-3 A2P/A2A ──> M5-4~M5-6 Skill ──> M5-7~M5-9 图谱 ──> M5-10~M5-12 插件运行时
```

---

## 8. 哪些任务仍必须强模型（**快模型碰不得，共 10 项**）

| # | 任务 | 最低档位 | 理由 |
|---|---|---|---|
| 1 | ShutdownCoordinator 退出收口 | `AI:DEEP` | 进程生命周期 + 事件循环 + 并发 + 幂等 |
| 2 | `run_script` 执行通道 | `AI:DEEP`（评审 xhigh） | 命令注入防线 |
| 3 | `withGlobalTauri` / 插件隔离整改 | `AI:DEEP` | 安全红线 + 全前端回归 |
| 4 | 插件运行时 | `AI:DEEP`（评审 xhigh） | zip-slip / 进程隔离 / 权限 |
| 5 | A2P/A2A 协议 | `AI:DEEP` | 协议 + 子进程 + 并发 |
| 6 | 数据库 schema / 迁移 / 回滚 | `AI:DEEP` | 不可逆数据操作 |
| 7 | 定时任务调度 | `AI:DEEP` | 幂等 + misfire + 持久化 + 进程 |
| 8 | 图谱数据模型 | `AI:DEEP` | 依赖 B1 + 隐私过滤 |
| 9 | term_resize（E2+E3） | `AI:DEEP` | Rust 所有权 + xterm fit 链路 |
| 10 | cron / regex 两个种子工具 | `AI:DEEP` | 边界用例多 |

**快模型可做（建议趁免费额度清掉，共 9 项）**：3 个简单种子工具（json/base64/timestamp）、图片 UI 静态壳、脚本库 UI 静态壳、工具库 UI 静态壳、终端历史与不落盘、X 系列 7 个验收脚本、M2 静态类 3 个验收脚本、死依赖清理（`strip-ansi-escapes` + `termLines`）、图谱阶段一确定性抽取。

**必须人工 GUI 验收（AI 不得代签，8 类）**：UI 截图点（图片 14 / 脚本库 14 / 工具库 15）、`vim`+`top`+`htop` 的 resize 表现、孤儿进程 `ps -ef` 检查、定时任务 misfire（需关应用/休眠）、安全验证（DevTools `__TAURI__`、注入 payload、恶意插件包）、数据库备份恢复与迁移回滚演练、三个决策拍板（B1/B2/B3）、图谱隐私与导出审查。

---

## 9. 遗留待人工处理的高优先级问题（Top 6）

1. **补齐 `AI-模型切换与接手清单.md`**（B3）—— 否则编号口径无法官方校准。
2. **拍板存储选型**（B1）—— 卡住 M4 数据库与 M5 图谱两条线。
3. **判定形态②插件可行性**（B2）—— 卡住工具加载方式与插件形态；若判定不可行，**形态②必须显式标记 BLOCKED**。
4. **先做退出收口 E4** —— 只有少量改动，却解锁 5 个下游；这是**投入产出比最高**的一步。
5. **补齐高风险验收矩阵** —— 本批次用 `model-routing-matrix` + `acceptance-script-drafts` 覆盖了部分职能，但**原文档仍缺失**。
6. **本地领先 16 commit 未 push** —— 若换机/换人接手，上游看不到全部前置文档；建议尽快 push（**本批次未执行 push**）。

---

## 10. 批次交付统计

| 项 | 数量 |
|---|---|
| 任务文档 | 21 |
| 汇总文档 | 1 |
| commit（前缀 `docs(free-prework-4)`） | 9（8 任务 + 1 汇总） |
| 代码改动 | **0** |
| 文档删除 | **0** |
| 宣称 PASS 的主任务 | **0** |
| 验收脚本实际创建 | **0**（仅产出 35 个草案） |
| 迁移/执行/运行时实现 | **0** |

**下一批次建议**：优先用快模型清 §8 的 9 项「快模型可做」清单（其中 3 个种子工具 + 3 份 UI 静态壳 + X 系列验收脚本的**收益最大且零阻塞**）；强模型额度留给 §7 第 1 步的退出收口。
