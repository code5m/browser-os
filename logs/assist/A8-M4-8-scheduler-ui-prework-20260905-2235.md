# A8 预研（只读）：M4-8 定时任务 UI — Wave 3 阻塞态说明

> LANE=**A8**（M4-8 定时任务 UI / `AI:BALANCED / R:medium` / Merge Order 7）
> WAVE=**3**（指挥板原文：「A8: Start after A6/A7 freeze scheduler command DTOs」）
> 时间：2026-09-05 22:35 CST（本机 `date -Is` = `2026-09-05T22:34:54+08:00`；指挥板头部记 23:00，两者时钟口径不同，排序以文件名时间戳为准）
> 基线：`3544c09`（`master`，`git status` 未显示 ahead/behind，与 `origin/master` 同步）
> 性质：**只读预研**。零产品代码改动、零主文档改动、零提交、零 push。
> 依据：`PARALLEL_COMMAND_BOARD.md`（§Startup Gate 末条 + §Dispatch Waves Wave 3）、`logs/checkpoints/M4-20260905-2225.md`（A1 展开卡 §7 M4-8 三张子卡）。
> `IMPLEMENTER_MODEL`（M2-6 观察项 O1 硬要求）：本会话 = CodeBuddy（Hy4 / 腾讯混元）；**后续 M4-8.b/c 实现与裁定须换不同模型**，恢复「展开者 / 实现者 / 裁定者三者不同模型」。

---

## 1. 启动门禁实测

| 项 | 实测 | 判定 |
|---|---|---|
| `pwd` | `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3` | 与权威主目录一致 ✓ |
| `.workspace-identity` | `WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master` | ✓ |
| 分支 | `master`，无 ahead/behind 标记 | ✓ |
| HEAD | `3544c09`（板记 mainline `f376346`，HEAD 领先 1 个提交 `docs: add dispatch waves for parallel lanes`） | 非失配，如实登记 |
| 工作树 | **不干净**，脏文件全部属于他 Lane，A8 未触碰、未回退 | 见下 |

他 Lane 的在途产物（**均非 A8 产生，A8 不得改动或回退**）：

- ` M AI-模型切换与接手清单.md`、` M 详细设计与实施计划.md` —— A1 卡展开回写
- `?? logs/checkpoints/M4-20260905-2225.md` —— A1 展开卡，`CARD_EXPANDED`，`NEXT=M4-1.a`
- `?? logs/assist/A10-M4-security-review-20260905-2240.md` —— A10
- `?? logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md`、`?? logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md` —— A11

> 启动门禁关于「工作树脏且脏文件不属于本 Lane」的硬停止条款在本仓库当前并行状态下**必然为真**（多 Lane 同时在途、只有 A0 提交）。处置口径与 A5 一致：**不动、不回退、不与之混改**，只向 `logs/assist/` 追加本文件；开工前须先由 A0 集成清理（见 §5 R-A8-6）。

## 2. 阻塞判定：A8 未达启动条件

A8 属 **Wave 3**，解除条件为「A6/A7 冻结调度器命令 DTO」。实测：

| 前置 | 应有证据 | 实测 | 结论 |
|---|---|---|---|
| A6 / M4-5.a `TaskDef` 契约 | `logs/checkpoints/M4-5.a-<ts>.md` | **不存在**（checkpoints 目录最新为 A1 展开卡与 A11 矩阵，无任何 `M4-5*` / `M4-6*` / `M4-7*`） | ❌ |
| A6 / M4-5.b 时钟语义、M4-5.c 错过执行与取消 | `M4-5.b/c-<ts>.md` | 不存在 | ❌ |
| A6 / M4-5.d 冻结裁定 + `check-scheduler-policy.py` | `M4-5.d-<ts>.md` | 不存在，`scripts/` 下无 `check-scheduler-policy.py` | ❌ |
| A7 / M4-6.b 五命令名与 DTO | `M4-6.b-<ts>.md` | 不存在；`logs/assist/A7-M4-6-7-scheduler-backend-prework-20260905-2315.md` 存在但 **`wc -l` = 0**（A7 预研在途未填） | ❌ |
| 后端落点 | `src-tauri/src/tasks.rs` / `scheduler*.rs` / `task_*` 命令 | grep `task_|scheduler|db_connect|db_query` 于 `src-tauri/src` 仅命中 `shutdown.rs`（通用 shutdown task 语义），**无调度模块、无 `task_*` 命令** | ❌ |

三份主文档 `NEXT` 仍为 `M4-1 数据库驱动与生成契约`；调度链串行点 `M4-5` 未冻结 → **A7/A8 同时悬空**（A1 展开卡 §3.3 同口径）。
按指挥板 Startup Gate 末条执行：**不改产品代码，只出本只读说明**。

### 解除阻塞条件（精确、可判定，四条全满足方可开工）

1. **A6 产齐 `M4-5.a~d`**：`TaskDef` 字段逐项冻结（`TaskKind` 仅 `Script`/`Command`，F-5）、持久化 = `tasks.json`（F-4/F-10）、可注入 `Clock`（F8）、**错过执行策略落为 `TaskDef` 字段**（F9，不得是运行时常量）、取消语义、容量上限与 ID 口径。
2. **A7 产出 `M4-6.b`**：`task_list / task_add / task_update / task_remove / task_run_now` 五个**最终命令名与 DTO 稳定**（A1 展开卡 §7 M4-6.b 现为登记名，最终以该卡为准），且 `src/bridge.ts` / `src/types.ts` 镜像已落 —— A8 只消费、不造名。
3. **运行记录 DTO 可支撑「历史 + 重试态」**（见 §5 R-A8-1）：现有 `ScriptRunRecord` 无 `kind` / `task_id` / 尝试次数字段，`RunStatus` 无 `missed` / `retry` / `queued` 语义。若 A6/A7 不补，M4-8.b 的「运行历史 + 失败重试状态」**无数据源**，须在契约层补齐或由 A0 显式缩小 M4-8 范围并登记债务。
4. **起点干净**：A1 / A10 / A11 的在途脏文件已由 A0 集成或清理，A8 重跑启动门禁时脏文件归属 A8 自身。

## 3. 前端落点实测（只读盘点，供 M4-8.b 施工）

| 落点 | 位置 | 现状 |
|---|---|---|
| 视图类型 | `src/stores/useLayoutStore.ts:5-19` `MainView` 联合类型 | 新增视图**必须**扩联合类型，否则 `npm run build` 直接失败 |
| 页签元数据 | `src/stores/useLayoutStore.ts:99-113` `MOD_META`（icon/label）；`openModule` 于 `:123` 按 view 去重 | 新增项形如 `tasks: { icon: "⏰", label: "定时任务" }` |
| 活动栏入口 | `src/components/layout/ActivityBar.vue:24-47` `menuSections`（工作区 / 工具 / 同步 三组；**工具组**现含 `apps` `scripts` `commands` `tools`） | 建议挂「工具」组，与脚本库/命令库同级 |
| 主区挂点 | `src/components/layout/MainArea.vue:137-151` | 各模块为 `v-else-if="layout.mainView === 'x'"` 分支（`scripts` / `commands` / `tools` 同范式） |
| Dock Tab（**不选**） | `useLayoutStore.ts:34` `browserDockTab` 联合类型 + `MainArea.vue:88-103` | Dock 是「边浏览边操作」位（文件/终端/资源/会话）；定时任务非该场景，且需改两处联合类型，收益为负 |
| 命令封装 | `src/bridge.ts:213-249`（`scriptList/Add/Update/Remove`、`scriptRunsList`）、`:252-281`（`snippet*`） | 组件**不得直接 invoke**（M4-8.a FORBID），一律经 `bridge.ts`；`scriptRunsList(scriptId?)` 是可选参数形态的可借鉴签名 |
| 类型镜像 | `src/types.ts`：`ScriptMeta:390-405`、`CommandSnippet:410-425`、`RunStatus:427`、`RunSnapshot:429-440`、`ScriptRunRecord:442-452` | 镜像文件内有「与后端 `domain.rs` 一一对应」注释范式，新增 `TaskDef` 须沿用 |
| 运行历史既有实现 | `src/components/workspace/ScriptRunHistory.vue`（M2-5.c） | 数据源 `bridge.scriptRunsList()`，打开时取一次；状态筛选 + 按日分组 + `output_tail` 行展开；展示逻辑全在 `src/utils/scriptUi.ts`（`filterRuns`/`groupRunsByDay`/`runStatusClass`/`runStatusLabel`/`formatRunDuration`/`summarizeRuns`/`tailPreview`） |
| 纯逻辑层范式 | `src/utils/snippetUi.ts`（导出 11 个纯函数：`emptySnippetForm` / `loadSnippetForm` / `parseArgvText` / `placeholderOf` / `validateSnippetForm` / `serializeSnippetForm` / `buildSnippetCategoryTree` / `canDeleteSnippet` / `isFavoriteSnippet` / `loadSnippetFavorites` / `toggleSnippetFavorite`）、`scriptUi.ts`、`imagePreview.ts` | M4-8.a 的 `taskUi.ts` 应对齐该形状（纯函数、无 invoke、无 DOM） |
| 确认弹窗 | `src/components/shared/ConfirmModal.vue` | 实测**仅 `src/App.vue:17/219` 引用**；`ScriptPanel.vue:56`、`CommandSnippetPanel.vue:64/72`、`useWorkspaceStore.ts:288/370` 仍在用**原生 `confirm`**（M2-6 观察项同口径）。M4-8 的「删除任务 / 立即执行危险任务」须用 `ConfirmModal`，不重蹈 |
| 脱敏 | `src/utils/redact.ts` | 任务参数值/输出尾片段若含敏感串，展示前过脱敏（与 Git UI 二次脱敏同口径） |
| 门禁 | `scripts/pre-merge.sh` | `pm_log` 共 48 处（自测段另计）；最新 UI 逻辑夹具为 `:337` M3.c 终端、`:341` M2-6.d 命令片段。三项注意：①**夹具按里程碑名标签追加，脚本内无数字编号**，A1 展开卡 §1 实测 13「第 22 项起」为文档口径，**接入时须现场复核**；②新增夹具须同时补进 `--self-test` 段（`:380-478` 逐脚本登记）；③`package*.json` 被哈希夹具锚定，**零新增 npm 依赖**（F11） |

## 4. 预计改动清单（预测，非承诺）

- **M4-8.a**：新增 `src/utils/taskUi.ts`（下次执行时间格式化 / 启停态 / 重试态 / 历史排序 / cron 表达式合法性**前置**校验）+ 新增 `scripts/check-scheduler-ui-logic.mjs`（沿用 `check-terminal-ui-logic.mjs` 的 `nodeModule.registerHooks` 解析 `.ts` 扩展名范式，直接 import **真实** `taskUi.ts`，只把 `bridge.ts` 换记录型 mock；测试 ID 段 `T-sched-ui-1~N`）。
- **M4-8.b**：新增 `src/components/workspace/TaskPanel.vue`（CRUD + 启用开关 + 下次执行 + 历史 + 重试态，可拆 `TaskRunHistory.vue`）+ `src/stores/useTaskStore.ts`；改 `src/bridge.ts`（`task_*` 五封装）、`src/types.ts`（`TaskDef` 与运行记录镜像）、`src/stores/useLayoutStore.ts`（`MainView` + `MOD_META`）、`src/components/layout/ActivityBar.vue`（工具组新增 ⏰ 定时任务）、`src/components/layout/MainArea.vue`（`modview` 分支）。**五处联合类型/分支必须同步，漏一处即构建失败。**
- **M4-8.c**：新增 `scripts/check-scheduler-ui-policy.py`（`SCHEDUI_*` 码位，1 好 + N 坏双向自检 + **变异防呆** + `--expect-pending`）+ `scripts/pre-merge.sh` 接入（项号现场复核）。

## 5. 风险清单（交 A0 / A6 / A7）

| 编号 | 风险 | 处置建议 |
|---|---|---|
| **R-A8-1** | **运行记录 DTO 缺口（本 Lane 最大阻塞）**：`ScriptRunRecord`（`types.ts:442-452`）只有 `run_id/script_id/status/started_at/finished_at/exit_code/error/output_tail/truncated`，**无 `kind` / `task_id` / 尝试次数**；`RunStatus`（`:427`）只有 `running/succeeded/failed/cancelled/timeout`，**无 `missed`/`retry`/`queued`**。→ M4-8 的「运行历史」与「失败重试状态」在当前 DTO 下**无法表达** | 由 A6/A7 在契约层补字段（建议 `kind` + `task_id` + `attempt` + 错过执行态）；未补前 A8 不得用裸 UUID 顶替（否则直接复制债务 D21 的展示缺陷）。若 A0 决定不补，须显式缩小 M4-8 范围并新开债务（从 **D27** 起，F12） |
| **R-A8-2** | 命令名提前绑定：`task_*` 现为 A1 展开卡的登记名，最终以 `M4-6.b` 为准 | a/b 卡开工前先读 `M4-6.b` checkpoint 取最终签名；**前端先写死会被运行时静默拒绝**（本项目历史坑：ACL 缺命令静默拒绝） |
| **R-A8-3** | 时钟/cron 语义未冻结（M4-5.b/c）→ 前端自造 cron 解析或 `next_run` 推算会与后端分叉（破 F8） | 「下次执行时间」一律消费后端字段；前端最多做**校验前置提示**，不得自行计算触发时刻 |
| **R-A8-4** | 前端实现调度/计时（M4-8.b FORBID） | 倒计时只允许展示；触发一律经 `task_run_now` 或后端调度器，前端不持有 `setInterval` 触发路径 |
| **R-A8-5** | 与 A5（M4-4 数据库 UI，merge 6）共用四个高冲突文件：`src/bridge.ts` `src/types.ts` `src/stores/*` `src/components/**`；另 `useLayoutStore.ts` / `ActivityBar.vue` / `MainArea.vue` 三处挂点也会撞 | 按冲突图 A5(6) 先落、A8(7) 后 rebase；挂点改动各自独立提交，便于 A0 分拆 |
| **R-A8-6** | 工作树在途脏文件（A1 三处 + A10 + A11 两处）污染 diff --check 与门禁基线 | A8 开工前须先确认 A0 已集成/清理；期间 A8 只追加 `logs/assist/` 文件 |
| **R-A8-7** | 危险任务「立即执行」与删除缺少规范确认入口 | 用既有 `ConfirmModal`，不用原生 `confirm`；**未确认前连 invoke 都不发**（同 M4-4.b FORBID 口径），并纳入 `SCHEDUI_*` 夹具 |
| **R-A8-8** | GUI 实点验收代签 / 误关挂账债务 | 沿用 M1-7 / M2-2.b / M3.c 口径：未目视不得签字，附人工验收清单；**不得顺手关闭 D23~D26**（F12），M4 新债务从 D27 起 |
| **R-A8-9** | 门禁项号漂移、新增 npm 依赖、体积增长 | 改 `pre-merge.sh` 前现场复核；零新增 npm 依赖；`npm run build` 0 error 且总体积增长 ≤15% |
| **R-A8-10** | 执行通道旁路风险（F6）：UI 若自行拼命令执行即成第二套路径 | 定时任务执行只能由后端经 `script_runner`；前端 `task_run_now` 只是**触发命令**，不得传命令正文去前端侧执行 |

## 6. 本次自检（A8 未做什么）

- 未改 `src/**`、`src-tauri/**`、`scripts/**`、`package*.json`。
- **未改三份主文档**（其中两份为 A1 的在途未提交改动，A8 未触碰、未回滚）。
- 未提交、未 push、未 reset、未 force-push。
- 本次唯一写入：`logs/assist/A8-M4-8-scheduler-ui-prework-20260905-2235.md`（本文件，指挥板 Wave 2/3 阻塞态许可位置）。

## 7. 复入检查清单（解除阻塞后第一步）

1. 确认 `M4-5.a~d` 与 `M4-6.b` 两份 checkpoint 存在，且 `task_*` 最终命令名、DTO、运行记录字段（含 `kind`/`task_id`/重试语义）已稳定。
2. 重跑启动门禁（`cat .workspace-identity` / `pwd` / `git status --short --branch` / `git log --oneline -12`），确认起点干净且脏文件归属本 Lane。
3. 按 M4-8.a 卡正文开工：**先纯逻辑层 `taskUi.ts` + `check-scheduler-ui-logic.mjs`，后组件**（与 M2-2.b / M2-5 / M2-6.d 同序）。
4. 挂点五处同步（`MainView` / `MOD_META` / `ActivityBar` / `MainArea` / store），一次提交内完成，避免半改状态。
5. `pre-merge.sh` 项号现场复核后再接入（归 M4-8.c），并同步补 `--self-test` 段登记。
6. 显式标注 `IMPLEMENTER_MODEL`；b/c 卡裁定须换与实现者不同的模型。

---

**NEXT**：等待 A6（M4-5.a→d）冻结 `TaskDef` 与调度语义 → A7（M4-6.a→c / M4-7.a→d）冻结 `task_*` 命令 DTO 与运行记录字段 → A8 从 `M4-8.a` 开工。当前保持 `STATUS=BLOCKED`，**不移动主文档 `NEXT`**、不写产品代码。
