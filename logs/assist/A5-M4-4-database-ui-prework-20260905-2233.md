# A5 预研（只读）：M4-4 数据库 UI — Wave 3 阻塞态说明

> LANE=**A5**（M4-4 数据库 UI / `AI:BALANCED / R:high` / Merge Order 6）
> WAVE=**3**（「Start after A2/A3/A4 freeze database command DTOs」）
> 时间：2026-09-05 22:33 CST
> 基线：`3544c09`（`master`，与 `origin/master` 同步）
> 性质：**只读预研**。零产品代码改动、零主文档改动、零提交、零 push。
> 依据：`PARALLEL_COMMAND_BOARD.md`（23:00 版 §Dispatch Waves / §Startup Gate 末条）、`logs/checkpoints/M4-20260905-2225.md`（A1 展开卡）。

---

## 1. 启动门禁实测

| 项 | 实测 | 判定 |
|---|---|---|
| `pwd` | `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3` | 与权威主目录一致 ✓ |
| `.workspace-identity` | `WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master` | ✓ |
| 分支 | `master`，与 `origin/master` 同步 | ✓ |
| 工作树 | **不干净**，但脏文件全部属于他 Lane，A5 未触碰、未回退 | 见下 |

他 Lane 的在途产物（**均非 A5 产生，A5 不得改动或回退**）：

- ` M 详细设计与实施计划.md` —— A1 卡展开回写
- `?? logs/checkpoints/M4-20260905-2225.md` —— A1 展开卡，状态 `CARD_EXPANDED`，`NEXT=M4-1.a`（交 A2）
- `?? logs/assist/A10-M4-security-review-20260905-2240.md` —— A10
- `?? logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md`、`?? logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md`（0 字节）—— A11

## 2. 阻塞判定：A5 未达启动条件

A5 属 **Wave 3**，解除条件是「A2/A3/A4 冻结数据库命令 DTO」。实测：

| 前置 | 应有证据 | 实测 | 结论 |
|---|---|---|---|
| A2 / M4-1.b `SupportedDb` + 连接配置 DTO + 错误码 | `logs/checkpoints/M4-1.b-<ts>.md` | **不存在**（checkpoints 最新为 A1 展开卡与 A11 矩阵） | ❌ |
| A2 / M4-1.c 结果上限·取消·截断契约 | `M4-1.c-<ts>.md` | 不存在 | ❌ |
| A4 / M4-2.s SQL 风险分类 + 生产判定 | `M4-2.s-<ts>.md` | 不存在 | ❌ |
| A3 / M4-3.a~d `db_*` 命令名与 DTO | `M4-3.a~d-<ts>.md` | 不存在 | ❌ |

主文档 `NEXT` 仍为 `M4-1 数据库驱动与生成契约`，A2 尚未认领产出 → **A5 不具备开工条件**。
按指挥板 Startup Gate 末条执行：**不改产品代码，只出本只读说明**。

### 解除阻塞条件（精确、可判定，四条全满足方可开工）

1. A2 产出 `M4-1.b`：`SupportedDb`、连接配置 DTO（**结构性不含 password**）、凭据命名空间 `db:<conn_id>`、错误码枚举。
2. A2 产出 `M4-1.c`：结果行数/字节上限、取消与截断标记契约。
3. A4 产出 `M4-2.s`：SQL 风险分级（含生产判定 fail-closed）纯函数契约，UI 侧可消费的风险等级字段已定义。
4. A3 产出 `M4-3.a~d`：`db_*` **最终命令名与 DTO 稳定**（A1 事实修正 F-3 明确：`db_connect/query/disconnect` 现为占位名，最终以 M4-3.a 为准，A5 不得提前绑定）。

---

## 3. 前端落点实测（只读盘点，供 b 卡施工）

| 落点 | 位置 | 现状 |
|---|---|---|
| 视图类型 | `src/stores/useLayoutStore.ts:5` `MainView`、`src/stores/useLayoutStore.ts:22` `mainView` ref | 新增视图需扩联合类型 |
| 侧栏入口 | `src/components/layout/ActivityBar.vue:16-44` | 三组模块项，形如 `{ view, icon, label }`（`scripts/commands/files/apps/tools/repo/audit`） |
| 主区挂点 | `src/components/layout/MainArea.vue:123-152` | 各视图为 `v-else-if="layout.mainView === 'x'"` 分支（脚本库/命令库/工具箱同范式） |
| 面板范式 | `src/components/workspace/{ScriptPanel,CommandSnippetPanel,GitPanel}.vue` | 面板 = 列表 + 表单 + 执行态三段式 |
| 命令封装 | `src/bridge.ts`（540 行）`snippetList: () => invoke<CommandSnippet[]>("snippet_list")` | **组件不得直接 invoke**（M4-4.a FORBID），一律经 `bridge.ts` |
| 类型镜像 | `src/types.ts`（492 行） | 后端 DTO 的 TS 镜像范式 |
| 危险确认 | `src/components/shared/ConfirmModal.vue` | 实测**当前仅 `src/App.vue` 引用**；b 卡须确认全局确认入口形态（M2-6 观察项曾记「用原生 `confirm` 而非既有 `ConfirmModal`」，不得重蹈） |
| 脱敏工具 | `src/utils/redact.ts:39` `redactSecrets` | UI 侧连接串/凭据展示的复用点（冻结条款 **F2**） |
| 纯逻辑层范式 | `src/utils/{snippetUi,scriptUi,imagePreview}.ts` + `scripts/check-*-ui-logic.mjs` | mjs 用 `nodeModule.registerHooks` 解析 `.ts` 扩展名，直接 import **真实** .ts 断言（非 mock 重实现） |
| 门禁 | `scripts/pre-merge.sh` | `pm_log` 共 **49** 处；末段夹具为 M2-6.d UI 逻辑。**M4-4 夹具项号须开工时现场复核**（A1 展开卡 §1 实测 13 同口径，照抄「第 22 项」有风险） |

## 4. 预计改动清单（预测，非承诺）

- **M4-4.a**：新增 `src/utils/dbUi.ts`（危险 SQL 高亮 / 结果截断展示 / 确认弹窗前置校验 / 连接表单校验）+ 新增 `scripts/check-database-ui-logic.mjs`（沿用 `check-command-ui-logic.mjs` 的 registerHooks 范式，测试 ID 段 `T-db-ui-1~N`）。
- **M4-4.b**：新增 `src/components/workspace/DatabasePanel.vue` + `src/stores/useDatabaseStore.ts`；改 `src/bridge.ts`（`db_*` 封装）与 `src/types.ts`（DTO 镜像）；挂点三处同步（`useLayoutStore.ts` 的 `MainView` / `ActivityBar.vue` 模块项 / `MainArea.vue` 分支）。
- **M4-4.c**：新增 `scripts/check-database-ui-policy.py`（`DBUI_*` 码位，1 好 + N 坏双向自检 + 变异防呆 + `--expect-pending`）+ `scripts/pre-merge.sh` 接入。

## 5. 风险清单（交 A0 / 后继 Lane）

| 编号 | 风险 | 处置建议 |
|---|---|---|
| **R-A5-1** | 占位命令名提前绑定：`db_connect/query/disconnect` 仅为登记名，最终以 M4-3.a 为准（A1 F-3） | a 卡开工前先读 `M4-3.a~d` checkpoint 取最终签名，禁止先写死再改 |
| **R-A5-2** | 凭据落地（F2）：连接串/密码进前端 state、localStorage、日志、表格或 toast | 连接配置 DTO 结构性无 password 字段；UI 展示一律过 `redactSecrets`；不落 localStorage |
| **R-A5-3** | 危险操作确认时序被绕过（M4-4.b FORBID） | 未二次确认前**连 invoke 都不发**；用既有 `ConfirmModal` 而非原生 `confirm`；此项应进 `DBUI_*` 静态夹具 |
| **R-A5-4** | 结果截断被静默（F3） | UI 必须显式呈现行数/字节上限与 `truncated` 标记，不得只渲染前半段 |
| **R-A5-5** | UI 自造第二套 SQL 风险判定（破 F4「纯函数落 `security_policy.rs`」） | 风险等级只消费后端 DTO 字段，前端仅做**着色/提示**，不做分类决策 |
| **R-A5-6** | 「导出成果库」范围未定（A1 F-6） | a 卡先确认是否要求溯源字段；未定则首期只做复制/CSV，**不碰 `save_artifact`** |
| **R-A5-7** | 门禁项号漂移 / 新增 npm 依赖 | 改 `pre-merge.sh` 前现场复核项号；`package*.json` 被哈希夹具锚定，零新增 npm 依赖（F11） |
| **R-A5-8** | GUI 实点验收代签 | 沿用 M1-7 / M2-2.b / M3.c 口径：未目视不得签字，附人工验收清单；M4 新增债务从 **D27** 起编号（F12），不得顺手关闭 D23~D26 |
| **R-A5-9** | 与 A8 撞车 | A5 与 A8 同为 Wave 3、共用 `src/components/**` `src/stores/**` `src/bridge.ts` `src/types.ts` 四个高冲突文件；按合并顺序 A5(6) 先于 A8(7) 落地，A8 后 rebase |

## 6. 本次自检（A5 未做什么）

- 未改 `src/**`、`src-tauri/**`、`scripts/**`、`package*.json`。
- **未改三份主文档**；其中 `详细设计与实施计划.md` 当前为 A1 的在途未提交改动，A5 未触碰、未回滚。
- 未提交、未 push、未 reset、未 force-push。
- 本次唯一写入：`logs/assist/A5-M4-4-database-ui-prework-20260905-2233.md`（本文件，指挥板 Wave 2/3 阻塞态许可位置）。

## 7. 复入检查清单（解除阻塞后第一步）

1. 确认 `M4-1.b/c` + `M4-2.s` + `M4-3.a~d` 四份 checkpoint 存在，且 `db_*` 命令名与 DTO 已稳定。
2. 重跑启动门禁（`cat .workspace-identity` / `pwd` / `git status --short --branch` / `git log --oneline -12`），确认起点干净且脏文件归属本 Lane。
3. 按 M4-4.a 卡正文开工：先 `src/utils/dbUi.ts` + `scripts/check-database-ui-logic.mjs`（**先纯逻辑层与夹具，后组件**，与 M2-2.b / M2-5 / M2-6.d 同序）。
4. `pre-merge.sh` 项号现场复核后再接入（归 M4-4.c）。
5. 显式标注 `IMPLEMENTER_MODEL`（M2-6 观察项 O1 硬要求）。

---

**NEXT**：等待 A2（M4-1.a→d）→ A4（M4-2.s）→ A3（M4-3.a~d）依次冻结；三者齐备后 A5 从 `M4-4.a` 开工。当前保持 `STATUS=BLOCKED`，不移动主文档 `NEXT`。
