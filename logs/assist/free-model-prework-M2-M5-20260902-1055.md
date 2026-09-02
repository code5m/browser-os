# free-model-prework-M2-M5 汇总（2026-09-02 10:55）

> 批次：`free-model-prework-M2-M5` · 路由 `AI:FAST`
> 产出于弱模型（低成本）阶段，目标：**把 M2/M3/M4/M5 的前置文档、契约、测试模板、UI mock/脚手架草案批量铺好**，降低后续强模型的 token 消耗与探索成本。

---

## 0. 批次执行声明（请先看这段）

| 约束 | 本批次实际执行情况 |
|---|---|
| 不移动主线 NEXT | ✅ **未移动**。仓库中未发现 NEXT 标记文件/字段，本批次也未创建或修改任何 NEXT 相关产物 |
| 不宣称任一主任务 PASS | ✅ **未签任何 PASS**。14 份任务文档全部标注「前置草案 / 未开工 / 未执行验收命令」 |
| 不改 `src-tauri` 安全/生命周期/进程管理核心代码 | ✅ **零改动**（见 §6 工作树核验） |
| 不实现 Git 写操作 / 脚本执行 / 终端核心 / 数据库迁移执行器 / 插件运行时 | ✅ 均未实现，仅产出契约与草案 |
| 不删除现有文档 | ✅ 未删除任何文件 |
| 不伪造测试结果 | ✅ 所有「验收命令」均标注**未执行**；仅「静态 grep/ls 复核」标注为已执行并给出回显结论 |
| `git diff --check` PASS | ✅ 见 §6 |
| 每类任务独立 commit | ✅ 15 个 commit（14 任务 + 1 汇总），前缀 `docs(free-prework)` |
| 工作树干净 | ✅ 见 §6 |

**本批次全部产出 = 15 个 Markdown 文档，零代码改动。**

---

## 1. ⚠️ 前置说明：READ 清单中的文件不存在

批次指令要求读取以下文件，**实测均不存在**：

| 要求读取 | 实测 |
|---|---|
| `AI-模型切换与接手清单.md` | ❌ 不存在（全仓无同名文件；仅有 `宫格崩溃多进程改造-接手进度.md`，非同一物） |
| `logs/assist/assist-index-20260901-1643.md` | ❌ 不存在（`logs/` 下原只有 `baseline-2026-08-27.md`） |
| `logs/assist/high-risk-acceptance-matrix-20260901-1643.md` | ❌ 不存在 |

**因此**：
1. 批次任务编号（`M2-1.a` / `M4-14.a` / `M5-7.a` …）与仓库 WBS 编号（`M2-1~9` / `M4-1~8` / `M5-1~12`）**不同源**，本文只能按「任务描述」锚定到仓库 WBS 与需求号（见 §2 映射表）。
2. 缺失的接手清单 / 高风险验收矩阵，本批次**不代为创建**（避免与后续强模型的口径冲突）。建议优先补齐这两份，再让强模型开工。
3. 每份任务文档开头都写了「编号口径与锚定」，强模型可据此快速对齐。

---

## 2. 任务 → 仓库 WBS / 需求号映射总表

| 批次任务号 | 标题 | 仓库 WBS | 需求号 | 文档 |
|---|---|---|---|---|
| M2-1.a | 图片领域契约冻结（ImageRef / MIME / 大小 / 路径 / 反向用例） | §4.4 **M2-8** | #10 | [M2-1.a-prework-20260902-1055.md](./M2-1.a-prework-20260902-1055.md) |
| M2-2.b | 图片预览 UI mock（画廊/灯箱/缩放/错误态/空态） | §4.4 **M2-9** | #10 | [M2-2.b-prework-20260902-1055.md](./M2-2.b-prework-20260902-1055.md) |
| M2-3.a | 脚本领域契约冻结（ScriptMeta / 参数 / 危险参数 / 审计 / 取消超时） | §4.1 **M2-1 + M2-2** | #1 / #4 | [M2-3.a-prework-20260902-1055.md](./M2-3.a-prework-20260902-1055.md) |
| M2-5.b | 脚本库 UI mock（列表/参数弹窗/运行态/历史态） | §4.1 **M2-3** | #1 | [M2-5.b-prework-20260902-1055.md](./M2-5.b-prework-20260902-1055.md) |
| M2-7.b | ToolMeta 清单与打包草案（`include_dir!`） | §4.3 **M2-5 + M2-6 + M2-7** | #2 | [M2-7.b-prework-20260902-1055.md](./M2-7.b-prework-20260902-1055.md) |
| M2-9.b | 5 个种子工具验收表 | §4.3 **M2-6** | #2 | [M2-9.b-prework-20260902-1055.md](./M2-9.b-prework-20260902-1055.md) |
| M3-1.a | 终端现状盘点（PTY/resize/history/进程归属/退出接入点） | §5 **M3-1 + M3-2 + M3-3** | #9 / #3 | [M3-1.a-prework-20260902-1055.md](./M3-1.a-prework-20260902-1055.md) |
| M3-4.b | 终端低风险体验草案（历史上限/resize 静默/前端入口） | §5 **M3-4** | #9 | [M3-4.b-prework-20260902-1055.md](./M3-4.b-prework-20260902-1055.md) |
| M4-14.a | 数据库任务拆解（schema/迁移/回滚/验收/风险） | §6 **M4-1 ~ M4-4** | #6 | [M4-14.a-prework-20260902-1055.md](./M4-14.a-prework-20260902-1055.md) |
| M4-58.a | 定时任务契约（调度/幂等/失败恢复/审计） | §6.1 **M4-5 ~ M4-8** | #11 | [M4-58.a-prework-20260902-1055.md](./M4-58.a-prework-20260902-1055.md) |
| M5-7.a | A2P / A2A 协议草案 | §7 **M5-1 ~ M5-3** | #7 | [M5-7.a-prework-20260902-1055.md](./M5-7.a-prework-20260902-1055.md) |
| M5-12.a | Agent-Skill 契约草案 | §7.1 **M5-4 ~ M5-6** | #12 | [M5-12.a-prework-20260902-1055.md](./M5-12.a-prework-20260902-1055.md) |
| M5-13.a | 图谱数据模型草案 | §7.2 **M5-7 ~ M5-9** | #13 | [M5-13.a-prework-20260902-1055.md](./M5-13.a-prework-20260902-1055.md) |
| M5-15.a | 插件权限模型草案 | §7.3 **M5-10 ~ M5-12** | #15 | [M5-15.a-prework-20260902-1055.md](./M5-15.a-prework-20260902-1055.md) |

> 注：`M5-7.a`/`M5-12.a`/`M5-13.a`/`M5-15.a` 的数字与需求号 #7/#12/#13/#15 一致；`M4-14.a`/`M4-58.a` 的数字与 #6/#11 不一致，**编号口径待接手清单补齐后校准**。

---

## 3. 🚨 本批次发现的 5 项现状勘误（强模型开工前必读）

> 以下均为 **2026-09-02 实测**，与既有文档描述冲突，会直接影响排期与验收口径。

### 3.1 【最高】5 个种子工具「已落盘」是错的 —— 文件根本不存在

- 文档说法：`详细设计与实施计划.md:144/178`、`后续需求TODO.md:37-51` 均称 5 个 HTML「已落位 `src-tauri/src/tools/`」。
- 实测：`ls src-tauri/src/tools` 报 **No such file**；`find . -name "*tool*.html"`（排除 node_modules/target）**零命中**。
- 影响：M2-6 的真实工作量是「**从零编写 5 个单文件 HTML**」+ 加载框架，不是「仅补加载框架」。
- 详见：[M2-7.b §2.1](./M2-7.b-prework-20260902-1055.md) / [M2-9.b §0](./M2-9.b-prework-20260902-1055.md)。

### 3.2 【高】`term_resize` 是空实现，且前端从不调用

- `bridge.rs:2013-2016`：`pub fn term_resize(...) { let _ = (app, id, cols, rows); Ok(()) }` —— 参数全吞，从不调 `MasterPty::resize`。
- `src/bridge.ts:207-209` 定义了 `termResize` 封装，但**全仓零调用方**。
- 表现：拖窗口后 xterm 重排，但 PTY 仍按 24×100 → `vim`/`top`/`htop` 显示错位。
- 隐藏工作量：要修 resize，必须先给 `TerminalSession` 增加 `master` 字段（当前只有 `writer` + `child`）。
- 详见：[M3-1.a §2.3](./M3-1.a-prework-20260902-1055.md) / §4.1、§4.2。

### 3.3 【高】关闭主窗口不杀任何 PTY，且 `.run()` 无 `RunEvent` 处理

- `main.rs:591` `CloseRequested` 只调 `grid_manager.shutdown_all()`，**不清理 terminals**。
- `main.rs:674-678` `.run()` 只有 `unwrap_or_else`，**无 `RunEvent` 分支**。
- `main.rs` 中 `std::process::exit` 出现 6 次 → 硬退出跳过析构，`Drop` 兜底不可靠。
- `TerminalPane.vue:66-70` 卸载时只 `dispose()` xterm，**不调 `term_kill`**。
- 影响：与 #3「关闭窗口资源释放」是同一个洞；也是 M2-3.a（脚本）、M4-58.a（定时任务）、M5-15.a（插件）的**共同阻塞项**。
- 详见：[M3-1.a §2.5 / §2.6](./M3-1.a-prework-20260902-1055.md)。

### 3.4 【中】`withGlobalTauri = true` 是 HTML 工具 / 插件的安全红线

- `src-tauri/tauri.conf.json`：`app.withGlobalTauri = true`。
- 影响：若 HTML 工具（#2）或形态 ② 插件（#15）运行在继承该设置的 webview 中，可直接 `window.__TAURI__.invoke(...)` 调任意已注册命令，权限模型形同虚设。
- 待确认：Tauri v2 能否按 webview 粒度关闭；若不能，形态 ② 插件不得实现。
- 详见：[M2-7.b §5.R3/R3注](./M2-7.b-prework-20260902-1055.md) / [M5-15.a §5.R1](./M5-15.a-prework-20260902-1055.md)。

### 3.5 【中】死依赖与遗留物

| 项 | 状态 |
|---|---|
| `strip-ansi-escapes = "0.2"` | `Cargo.toml` 声明但 `src-tauri/src` **零引用**（`bridge.rs:1942` 注释说明 ANSI 过滤是有意移除的） |
| `gtk = "0.18"` / `wry = "0.55"` | M0-6 待清理项，**仍在 Cargo.toml 中** |
| `useSystemStore.termLines` | 注释「兼容保留，不再用于渲染」，可清理 |

---

## 4. 跨任务阻塞依赖图（强模型排期依据）

```
M0-2 / M3-1.a 退出收口（request_app_exit + ShutdownCoordinator）
   ├──> M2-3.a 脚本执行（长生命周期进程，无收口 = 新泄漏源）
   ├──> M4-14.a 数据库连接池关闭
   ├──> M4-58.a 定时任务 timer + 被触发进程
   ├──> M5-12.a Agent 流式任务 / Skill 执行
   └──> M5-15.a 插件资源释放

M2-3.a run_script（执行通道）
   ├──> M4-58.a 定时任务（只调度，复用执行通道）
   └──> M5-12.a Skill（只能引用 ScriptMeta）

M2-1.a ImageRef 契约 ──> M2-2.b 图片 UI mock
M2-3.a ScriptMeta 契约 ──> M2-5.b 脚本库 UI mock
M2-7.b ToolMeta 契约 ──> M2-9.b 5 个种子工具（先写工具，再按表验收）
M3-1.a 终端盘点 ──> M3-4.b 终端低风险体验（resize 静默须与后端修复同 PR）
M5-7.a 能力白名单 ──> M5-12.a Agent 能力边界（同一份白名单两处复用）
M2-1.a / M2-3.a / M4-58.a 领域模型 ──> M5-13.a 图谱（派生索引层，上游未落地则只能围绕 Artifact 建图）
```

**建议实施顺序**：
1. **先做退出收口**（M0-2 + M3-1.a 的 P0-1/P0-2，改动极小、收益最大、解锁 5 个下游）。
2. **M2-1.a → M2-2.b**（图片，纯增量，风险最低）。
3. **M3-1.a §4.1 resize 修复 + M3-4.b 静默窗口（同 PR）**。
4. **M2-3.a → M2-5.b → M2-7.b → M2-9.b**（脚本 → 工具）。
5. **M4-14.a → M4-58.a → M5-13.a**（数据 → 调度 → 图谱）。
6. **M5-7.a → M5-12.a → M5-15.a**（协议 → 生态 → 插件，风险递增）。

---

## 5. 跨任务共性红线（每条都有对应反向用例）

| # | 红线 | 涉及任务 | 为何重要 |
|---|---|---|---|
| **K1** | 新增 `#[tauri::command]` 必须同步进 `permissions/default-commands.toml` | 全部涉及命令的任务 | 项目历史坑：漏加被 ACL 静默拒绝，无日志、前端 catch 吞错，极难排查（当前 59 个命令） |
| **K2** | 任何出网/落盘/执行动作写 `audit.json` | M2-3.a / M4-14.a / M4-58.a / M5-7.a / M5-12.a / M5-15.a | 安全红线；注意 `audit.json` 只有 1000 条上限 |
| **K3** | 凭据只存 keyring，不入日志/审计/领域模型 | M4-14.a / M5-12.a | `DbConnConfig`/`AgentDef` 结构体内**不得有密码字段** |
| **K4** | 用户输入的参数必须 fail-closed 校验（命令注入面） | M2-3.a（主）、M4-58.a、M5-12.a | `shell:allow-spawn` 已是 `args: true`，ACL 不兜底，校验必须在 `run_script` 内部 |
| **K5** | 高频自动行为不得刷空 `audit.json`（1000 条上限） | M4-58.a / M5-7.a / M5-15.a | 已给出统一方案：自动行为进独立 JSON，摘要进 audit |
| **K6** | 数据文件必须原子写（临时文件 + rename），损坏要备份不静默清空 | M4-58.a / M5-13.a / M4-14.a | 半截 JSON = 用户数据全丢 |
| **K7** | 路径必须 `canonicalize()` + 前缀校验 | M2-1.a / M2-3.a / M5-15.a | 防 `..` 与软链接逃逸 |
| **K8** | 前端渲染不可信内容禁用 `v-html` | M2-2.b / M2-5.b | 脚本输出 / 图片 HTML 是 XSS 面 |
| **K9** | UI mock 不得触碰真实执行通道 | M2-2.b / M2-5.b | 已有 grep 校验命令（应 0 命中） |

---

## 6. 工作树与提交核验

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 6.1 零代码改动核验（应为空）
git status --porcelain -- src/ src-tauri/ package.json vite.config.ts

# 6.2 git diff --check（应无输出 = PASS）
git diff --check

# 6.3 提交清单（15 个，前缀 docs(free-prework)）
git --no-pager log --oneline -15

# 6.4 产出物清单
ls -1 logs/assist/
```

**核验结果（本批次收尾时执行）**：见 §0 表格与文末 §8。

---

## 7. ⭐ 后续强模型最小阅读清单

> 目标：用**最少的 token**让强模型进入可施工状态。按优先级排列，**不必全读本批次 14 份文档**。

### 7.1 必读（约 30 分钟，约 1.2 万 token）

| 顺序 | 读什么 | 为什么 |
|---|---|---|
| 1 | **本文 §0 / §1 / §3 / §4 / §5** | 批次边界、READ 文件缺失、5 项勘误、依赖图、共性红线 |
| 2 | `src-tauri/src/domain.rs`（92 行） | 唯一已有的领域模型，所有新类型都加这里 |
| 3 | `src-tauri/src/workspace.rs`（110 行） | 持久化 + 审计 + 1000 条上限，所有落盘任务都要复用 |
| 4 | `src-tauri/permissions/default-commands.toml` | 59 个命令白名单，K1 红线入口 |
| 5 | `src-tauri/tauri.conf.json` | `withGlobalTauri` / `assetProtocol.scope` 两个红线配置 |
| 6 | `logs/baseline-2026-08-27.md` | clippy 13 warning 与 505 KB 主 JS 的**硬门槛** |

### 7.2 按接手任务选读（只读对应的 1~2 份）

| 接手任务 | 选读文档 | 配套源码 |
|---|---|---|
| M2 图片 | M2-1.a（契约）+ M2-2.b（UI 草案） | `src/components/workspace/ArtifactPanel.vue`、`src/utils/markdown.ts`、`src-tauri/injected/collect.js:38-85` |
| M2 脚本 | M2-3.a（契约）+ M2-5.b（UI 草案） | `src/components/workspace/AuditPanel.vue`、`src/components/shared/ConfirmModal.vue` |
| M2 工具 | M2-7.b（打包方案）+ M2-9.b（验收表） | `src-tauri/build.rs`、`variants/*.html`（单文件 HTML 风格参照，读 1 个即可） |
| M3 终端 | M3-1.a（盘点 + 接入点）→ M3-4.b（体验草案） | **`bridge.rs:1917-2027`（必读，111 行）**、`main.rs:567-610` + `:670-679`、`TerminalPane.vue`（124 行） |
| M4 数据库 | M4-14.a（拆解 + 迁移回滚） | `src-tauri/src/keyring_store.rs`（25 行）、`bridge.rs:688-790`（两段式闸门范式） |
| M4 定时 | M4-58.a（契约） | `workspace.rs`（全读）、`main.rs:598-610`（挂常驻线程的位置） |
| M5 协议 | M5-7.a（草案） | `bridge.rs:688-790`、M4-14.a §5-bis（迁移回滚范式） |
| M5 Agent/Skill | M5-12.a（契约） | `src/components/browser/AINavPanel.vue`（UI 参照）、M2-3.a §4.3（危险参数，必须复用） |
| M5 图谱 | M5-13.a（模型） | `domain.rs` + `workspace.rs`（孤儿问题根源） |
| M5 插件 | M5-15.a（权限） | `permissions/default-commands.toml`、`capabilities/default.json`、`tauri.conf.json` 的 `withGlobalTauri` |

### 7.3 明确不必读

- `bridge.rs` 全文（2029 行）——按任务读对应区段即可，文档已标好行号。
- `grid_process.rs`（705 行）——仅 #3 宫格相关，本批次 14 个任务均不涉及。
- `sync.rs`（287 行）——只需读 `bridge.rs:688-790` 的闸门范式，不必读 sync 实现细节。
- `超详细审核报告-严重问题与缺陷.md` 等历史长文档——除 M0-6 依赖清理外，与本批次无关。
- `tauri-browser-tabs/` 子 crate——除非接手 M2-7.b 的子 webview 加载通道。

### 7.4 给强模型的三条硬提示

1. **先做退出收口再动功能**（M3-1.a §2.6 的 P0-1/P0-2 只有 2 处改动，却解锁 5 个下游任务）。
2. **改 `domain.rs` 时 `Artifact` 新字段必须 `#[serde(default)]`** —— 否则历史 `workspace/*.json` 会被 `load_artifacts` 静默丢弃（`workspace.rs:53` 的 `if let Ok` 吞错）。
3. **每个任务文档第 7 节的验收命令都标了「未执行」** —— 不要把它们当成已通过的证据；实现后必须真跑，且对照 `logs/baseline-2026-08-27.md` 的 clippy/体积硬门槛。

---

## 8. 批次交付清单

| 项 | 数量 |
|---|---|
| 任务文档（每个含：目标/现状/必改文件候选/接口数据契约/风险/反向用例/验收命令/失败动作） | 14 |
| 汇总文档（本文） | 1 |
| commit（前缀 `docs(free-prework)`，每任务独立） | 15 |
| 代码改动 | **0** |
| 文档删除 | **0** |
| 宣称 PASS 的主任务 | **0** |

**遗留待强模型/人拍板的高优先级问题（Top 5）**：

1. `AI-模型切换与接手清单.md` 与 `logs/assist/*` 缺失 → 编号口径与高风险验收矩阵无从对齐，**建议优先补齐**。
2. 5 个种子工具文件不存在 → M2 排期需重算（§3.1）。
3. 退出收口（M0-2 / ShutdownCoordinator）未落地 → 阻塞 5 个下游任务（§3.3）。
4. `withGlobalTauri=true` 能否按 webview 关闭 → 决定 HTML 工具与形态 ② 插件是否可行（§3.4）。
5. M5-13.a 的存储选型（`rusqlite` 独立引 vs 扩 M4-14.a 范围）→ 未决策则图谱无法开工。
