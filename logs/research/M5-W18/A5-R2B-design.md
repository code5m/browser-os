# A5 · M5-W18-R2B 数据库工作台 UX 设计整包（Lane A5 · RESEARCH_AND_DESIGN）

> 派发：`M5-W18-R2B`（第二批任务卡 v1，2026-09-08），`NEXT=M5-W18-R2B`，`MODE=RESEARCH_AND_DESIGN`，`W19=CLOSED`。
> 任务卡 §6（A5）权威要求：设计连接/库表树、中央多 SQL 控制台、底部结果/消息、取消/忙碌/已截断提示；每份文档独立保存暂存 SQL/连接引用/执行ID/当前结果，切换文档旧请求不得覆盖新文档；覆盖错误/空/断线/重连/大结果/长字段/NULL/二进制长度；写确认沿用现有机制；产出键盘/焦点/窄窗规格、现有组件拆分表与 J2 测试脚本；编辑器升级须对照现有依赖/收益/包体成本。
> 本文件为**设计整包**，不写产品代码（R2B §1：不改为 `src/`/`src-tauri/`/`scripts/`；研究原型仅合成数据）。旧报告保留，本文件自洽引用。
>
> 证据标签：`CURRENT_PRODUCT`(本仓库已存在代码/配置) ｜ `REFERENCE_SOURCE`(dbx Apache-2.0 本地快照) ｜ `OFFICIAL_DOC`(JetBrains/IDEA 官网帮助页) ｜ `OBSERVED_BEHAVIOR`(已执行观察) ｜ `EXECUTED_SYNTHETIC_TEST`(已运行测试) ｜ `INFERENCE`(推断) ｜ `DESIGN_DECISION`(设计选择)。

---

## §1 更正日志与旧成果复用清单

**旧成果（已保留，位于本分支 `codex/m5-w18-a5`，rebase 后 HEAD=a42b915；其祖先含 R2 提交 bd8fc56 / 71641e5）：**
- `logs/research/M5-W18/A5-workbench-ux-map.md`（R2 证据层：8 项 R1→R2 纠正、真实前端盘点、dbx 逐能力实测映射、体积硬约束）。
- `logs/research/M5-W18/A5-replication-blueprint.md`（R2 最小改动蓝图：零净增/替代既有、345B 红线）。
- `logs/research/M5-W18/A5-checkpoint.md`（R2 检查点，含 R2 输出块）。

**R2 已固化、本包直接复用的结论（`[CURRENT_PRODUCT]`）：**
1. 真实前端已成型：`DatabasePanel.vue`(231 行/11,278B) + `useDatabaseStore.ts`(218 行/6,716B) + `dbUi.ts`(444 行/16,771B) + `bridge.dbConnect/dbQuery/dbDisconnect`；懒加载于 `MainArea.vue:50`。
2. **不存在 `useConnectionStore`、不存在 `src/lib/`**（R2 C1/C5 纠正已 retract R1 误述）。
3. 体积红线：真基线 `build-metrics-4f0e8ab.json` 的 `dist.total_bytes=612,943`；`measure-build-metrics.py:39` 上限 `25.2%` ⇒ 允许 `767,404B`；干净 `052b18a` 实测 `767,059B` ⇒ **净余量≈345B**。任何净新组件须零净增或替代既有行内代码。
4. 产品无 Vitest/test 脚本（`package.json` 仅 dev/build/preview/tauri）；测试范式为既有 `.mjs` 源码逻辑门禁（`check-database-ui-logic.mjs` 119 断言通过，`[EXECUTED_SYNTHETIC_TEST]` 退出 0）。
5. CodeMirror 6（dbx `QueryEditor.vue` 291,246B 内核）在 0 余量下 **REJECT(M5)/DEFER**（R2 C9）：未实测需求与体积即不得选。
6. 写确认沿用现有机制：当前 `DatabasePanel.vue:146` 内联 `<div class="confirm">`，或复用 `shared/ConfirmModal.vue`+`useModalFocus`（a11y 更优但非强制）；二者皆属「现有机制」。

**本包新增（R2B）：** 多 SQL 文档状态模型、连接/库表树占位契约、底部结果窗口布局、键盘/焦点/窄窗规格、组件拆分表、J2 测试脚本、候选实现卡。

---

## §2 参考身份与来源对账（证据标签）

- **体验参照（OFFICIAL_DOC）**：IDEA Database Tools（`https://www.jetbrains.com/help/idea/relational-databases.html`）、IDEA 工具窗口/统一搜索帮助页——仅作研究入口，非「功能已实现」证明（与 `WORKBENCH_BLUEPRINT-20260908.md` §2 一致）。
- **已定位代码参照（REFERENCE_SOURCE）**：本地 `/home/ainfinit/Documents/极智简单/V3/research/dbx-src`（README 指向 `t8y2/dbx`，Apache-2.0；`Cargo.lock` SHA-256 `c0a7be12…`，由任务板 L1364 记录）。A0-R2B 抽查（L40）明确：**该源码不等于用户所说的 IDEA 插件/功能**，A4 负责身份核对，本 lane 分清「体验参照」与「源码参照」，既有 dbx 研究可复用。
- **检索参照**：与本 lane 无关（属 A7/A8），此处不采纳。
- **当前产品（CURRENT_PRODUCT）**：见 §1 与 §4 真实文件清单。
- **DbValue 漂移（REFERENCE 与 CURRENT 双真源）**：`types.ts:600-611` 已与 `domain.rs` 对齐为 `snake_case`（null/bool/int/float/text/blob_len）；但 `src-tauri/src/database.rs` 后端 Rust 侧变体为 `I64/F64/Binary{bytes}`（A0-R2B L41 提及 23 个独立 `#[test]` 在 database.rs:814-1262）。漂移修复归 A1/G4/A4；本 lane 仅消费 `types.ts` 镜像，不自行裁决。
- **许可证**：dbx=Apache-2.0；本仓 `LICENSE`=MulanPSL-2.0（A0-R2B 未改）；移植署名归 A10 账本。

---

## §3 完整操作流程、状态与失败分支 + 线框

### 3.1 主流程（J2 数据库工作，`WORKBENCH_BLUEPRINT-20260908.md` §4 J2）
```
连接列表 → 选库/表(树) → 打开 SQL 控制台(标签) → 执行只读查询 → 浏览有限结果 → 取消长查询 → 返回控制台
```
- 至少**两份 SQL 文档**分别保留：文本、连接引用、执行状态、结果（R2B §6 验收）。
- 结果支持 NULL/数字/长文本/二进制长度/显式截断（`dbUi.formatCellValue`/`buildResultView` 已覆盖）。
- 写查询沿用当前确认 + 生产环境判定（`dbUi.needsWriteConfirm`/`writeBlockedReason` 已落地）。

### 3.2 单文档状态机（核心不变量，已用 `A5-R2B-state-model.mjs` 验证）
```
idle ──run(sql,connId)──▶ running ──applyResult(execId匹配)──▶ success/error/timeout/cancelled
  ▲                         │  └── applyResult(execId不匹配=陈旧) ──▶ 丢弃(不覆盖)
  │                         └── cancel ──▶ cancelled ──run──▶ running(新 execId)
  └── 切到别的文档再切回，本文档状态/结果不变（doc.execId 守卫）
```
- **陈旧守卫**：`applyDocResult(doc, execId, result)` 仅当 `doc.execId===execId` 才提交（见 state-model S2）。这是「切换文档旧请求不得覆盖新文档」的硬保证。
- **取消幂等**：`cancelDoc` 仅对 `running` 有效，重复调用无副作用（state-model S3）。
- 取消超时语义对齐 dbx `queryStore.ts:105-106`：`CANCEL_QUERY_TIMEOUT_MS=10_000` / `CANCEL_ACK_SETTLE_TIMEOUT_MS=2_000`（`[REFERENCE_SOURCE]`），实现归 W19，本包只冻结行为契约。

### 3.3 失败分支覆盖（R2B §6「覆盖错误/空数据/断线/重连/大结果/长字段/NULL/二进制长度」）
| 情形 | 状态/UI | 数据来源 |
|---|---|---|
| 空结果 | `success` + 0 行 + 「无行」提示 | `DbResultSet.row_count===0` |
| NULL | 单元格渲染 `NULL` | `dbUi.formatCellValue` |
| 长文本/二进制长度 | 显示宽度截断 + `<binary N B>`；超 `maxTextFieldBytes` 显式截断告警 | `dbUi.formatCellValue`/`DB_LIMITS` |
| 大结果截断 | `truncated=true` + 行数上限告警 | `dbUi.buildResultView.warnings` |
| 错误 | `error` + 脱敏消息（`role=alert`） | `bridge` 返回 error / `dbUi` 文案 |
| 超时/取消 | `timeout`/`cancelled` + 「结果不完整」告警 | `dbUi.buildResultView` |
| 断线 | `connected=false` + 「连接已断开，重连后重试」；重连沿用现有 `dbConnect` | `useDatabaseStore.connected` |
| 写被拒 | 沿用现有确认 + `writeBlockedReason` 文案 | `dbUi.writeBlockedReason` |

### 3.4 线框（自包含静态原型：`A5-R2B-wireframe.html`，合成数据、零真实 IPC）
- **桌面 1440×900**：顶栏（项目/上下文/全局搜索/连接状态）→ 左 260px 数据库对象树 → 中央多 SQL 标签 + textarea 控制台 + 状态行（运行/取消/复制CSV/清空 + `execId` 徽标 + 快捷键提示）→ 底部 240px 结果/消息/输出 工具窗口。
- **窄窗 ≤1100px**：收起左侧树（库表浏览器为后续片），底部降高；≤820px 进一步压缩。
- 关键态演示：运行徽标、已截断告警、错误告警、长字段截断单元格。
- 与底部结果窗口/侧栏的衔接：结果区即 `WORKBENCH_BLUEPRINT` §3.5 的「底部工具窗口」；窄窗时按「中央剩余区不足则收起侧栏」优先级（蓝图 §3 尺寸约束）。

### 3.5 键盘 / 焦点 / IME / 与其它工具衔接
- **运行**：`Ctrl/Cmd+Enter`（textarea `@keydown`），不抢占全局快捷键，IME 输入中不误触（仅捕获组合键，忽略 composition 中）。
- **取消**：`running` 时 `■ 取消` 可点；`Esc` 关闭确认框。
- **切标签**：`Ctrl/Cmd+Alt+[` / `]`（IDEA 风格，避免与浏览器原生 `Ctrl+Tab` 冲突）。
- **焦点返回**：确认弹窗用 `useModalFocus`（现有），`role=alertdialog`；关闭后焦点回触发按钮。
- **结果表**：`th[scope=col]`，结果区 `role=region aria-label="查询结果"`；空/错/忙各用 `role=status`/`alert` 区分。
- **与底部窗口/侧栏**：仅消费 A1 壳层契约（`useLayoutStore.mainView`/`modTabs`），不自行管理布局；数据库文档身份由本 lane store 持有，不混入 A1 全局文档 store（避免「巨型全局 store」反模式，蓝图 §5）。

---

## §4 逐文件改动计划（真实现有文件 + 拟增 + 调用点 + 契约 + 迁移/回滚 + 依赖）

> 全部为**设计/计划**，落地归 W19（受 A4 DTO 阻塞）。改动须满足 §1.3 的 345B 红线：优先扩展既有文件，必要时用内联替代。

| 真实现有文件 | 改动（设计） | 契约 / 调用点 |
|---|---|---|
| `src/stores/useDatabaseStore.ts`(6,716B) | 新增 `documents: Map<docId,SqlDoc>`、`activeDocId`、`openDoc()/closeDoc()/setActive()`、`runDoc(docId)`/`cancelDoc(docId)`/`applyDocResult(docId,execId,res)`（execId 守卫，见 state-model）、`schemaTree`（待 A4）、保留现有 `connections/form/sql` 兼容或迁移。 | `bridge.dbQuery` 调用不变；`execId` 由 `crypto.randomUUID()` 生成（与 `dbUi.newConnectionId` 同源做法）。 |
| `src/components/workspace/DatabasePanel.vue`(11,278B) | 新增：标签栏（每文档一个 tab，含关闭）、连接/库表树面板（占位，A4 落地前显示「待 schema 命令」）、底部结果/消息窗口（替代当前内联结果块）、写确认沿用现有内联 `.confirm` 或 `ConfirmModal`。加 `aria-label`/`role`/`@keydown`/`tabindex`。 | 绑定 `useDatabaseStore` 的 `documents`/`activeDocId`；`runDoc`/`cancelDoc`；`dbUi.buildResultView` 渲染告警。 |
| `src/utils/dbUi.ts`(16,771B) | 新增纯函数：`createSqlDoc()`、`applyDocResult(doc,execId,res)`（陈旧守卫，与 state-model 同算法）、`docResultView(doc)`（复用 `buildResultView`）。 | 可被 `check-database-ui-logic.mjs` 直接单测；不新增依赖。 |
| `src/types.ts`(31,148B) | 新增 `SqlDoc` 接口（含 `id/title/sql/connId/execId/status/result/error`，镜像已有 `DbResultSet`）。 | 仅类型，无运行时。 |
| `src/bridge.ts`(33,478B) | **不改**。`dbConnect/dbQuery/dbDisconnect` 已存在；取消依赖 A4 新增 `db_cancel`（阻塞项 B-A5-1）。 | — |
| **PROPOSED_NEW（可选，非必须）** `src/composables/useSqlDocTabs.ts` | 仅当 store 过于臃肿时抽出文档标签逻辑；否则内联于 store。标 `PROPOSED_NEW`，非现有模块。 | — |

- **迁移**：多文档模式可用 store 字段开关灰度；旧 `db.sql` 单控制台映射为默认首文档，迁移无破坏性。
- **回滚**：撤销 store/panel/dbUi 的上述 diff 即回退；无 schema 变更、无持久化格式变化（文档状态为内存态，不落盘；落盘归 A6/A1 持久化契约）。
- **依赖**：无新依赖；编辑器保持 `<textarea>`（CodeMirror REJECT，见 §1.5 / §6）。
- **包体**：扩展既有文件，无净新文件（除可选 composable）；预估净增 ≤ 345B 需 W19 实测 `measure-build-metrics.py`，本包不保证，标注为待验证。

---

## §5 可复跑测试与未运行项

### 5.1 已运行（EXECUTED_SYNTHETIC_TEST）
- `node logs/research/M5-W18/A5-R2B-state-model.mjs` → **9 断言全部通过，EXIT=0**。覆盖：双文档独立、陈旧 execId 丢弃、取消幂等+重试、截断字段透传。
- 既有 `node scripts/check-database-ui-logic.mjs` → **119 断言通过**（基线，未重跑但本包不改其覆盖；新增断言见 5.2）。

### 5.2 J2 测试脚本（建议加进 `check-database-ui-logic.mjs`，W19 落地时随产品代码一并提交）
- `createSqlDoc/applyDocResult` 陈旧守卫单测（复用 state-model 算法）。
- `docResultView` 对 `truncated/field_truncated/state∈{timeout,cancelled,failed}` 的告警文案（复用 `buildResultView` 现有断言风格）。
- 写确认：`writeBlockedReason` 三态（未开写/Unknown/Production）仍 PASS（既有已覆盖）。
- 组件级 a11y 断言：结果区 `role=region`、确认框 `useModalFocus`、textarea `Ctrl/Cmd+Enter` 触发（源码契约，非 Vitest）。

### 5.3 明确未运行（NOT_RUN，按 R2B §1 与蓝图 §7 要求显式标注）
- **任何产品代码 / 构建 / GUI / native IPC**：本包为设计，未改 `src/`、`src-tauri/`、`scripts/`；`npm run build`、`pre-merge.sh`、`cargo test` 均 NOT_RUN。
- **A4 契约未到**：连接树填充、`db_cancel` 取消令牌、schema 浏览、历史持久化 的端到端验证 NOT_RUN（阻塞于 B-A5-1）。
- **CodeMirror 体积实测**：NOT_RUN（无实测即不采纳，符合 R2 规则）。

---

## §6 候选实现卡（PROPOSED_NOT_AUTHORIZED — 供 W19 编码 lane，非开工许可）

```
PROPOSED_NOT_AUTHORIZED
标题：S2 数据库日常闭环（多 SQL 文档 + 连接树 + 结果窗口）
范围：
  - 在 useDatabaseStore 引入 documents Map + activeDocId + runDoc/cancelDoc/applyDocResult(execId守卫)。
  - DatabasePanel 增加标签栏、连接/库表树面板（占位）、底部结果/消息窗口；补齐 aria/keydown。
  - dbUi 增加 createSqlDoc/applyDocResult/docResultView 纯函数并接 check-database-ui-logic.mjs 断言。
前置（阻塞）：
  - A4 冻结：db_list_connections / db_cancel(conn_id,query_id) /（可选）db_list_schema DTO 与命令名。
  - 体积预算：净增需 ≤ 345B 或替代既有行内代码；否则须 A0 书面抬 25.2% 上限（AC-5 当前禁止）。
步骤：
  1) 先落 documents 状态模型（纯函数，可单测）→ 2) 接 bridge.dbQuery 的 execId 回带 →
  3) 面板标签栏/底部窗口/树占位 → 4) a11y/键盘 → 5) 接 A4 取消命令后补 cancelDoc 实际令牌。
测试：
  - node scripts/check-database-ui-logic.mjs（扩写后）全过；A5-R2B-state-model.mjs 全过。
  - native 验收（A9/A11）：debug/release 下长查询取消、切换文档不串结果、IME 中 Ctrl+Enter 不误触。
完成条件：
  - 至少两份 SQL 文档相互独立（查询→结果→取消→重试完整）；写确认沿用现有机制；
    覆盖空/NULL/长字段/二进制/截断/错误/断线态；无新增依赖；包体不破 25.2%。
```

---

## §7 自检与提交说明（本包交付）

- 证据闭环：R1 误述已 retract（R2 三文件）；本包复用 R2 结论并补齐 R2B §6 整包设计。
- 不越界：未改 `src/`/`src-tauri/`/`scripts/`、未引入依赖、未读取密钥、未接真实 IPC、未 push。
- 交付物：本文件 + `A5-R2B-state-model.mjs`(运行通过) + `A5-R2B-wireframe.html`(合成线框) + `A5-M5-W18-R2B-checkpoint.md`。
- 提交：单一整包提交（不补空洞 HEAD 回填），commit ID 见 checkpoint；文档不引用自身未来提交号。

---

## 附录：依赖与开放决策（OPEN_DECISIONS）
- **B-A5-1（High, 依赖 A4）**：`db_list_connections`/`db_cancel`/历史 DTO 未冻结 → 连接树填充、取消令牌、历史 W19 才能落地。
- **B-A5-2（High, 体积）**：25.2% 余量≈345B，任何净新组件须替代既有或 A0 抬限。
- **B-A5-3（Med, DbValue 漂移）**：`database.rs`(`I64/F64/Binary`) vs `domain.rs`(`Int/Float/BlobLen`) 双真源，归 A1/A4。
- **B-A5-4（Low）**：面板级 a11y/键盘断言待扩写（既有门禁只覆盖 `dbUi.ts` 纯逻辑）。
- **编辑器升级**：CodeMirror 维持 REJECT(M5)/DEFER，待 A0 在 disposable 工作区授权 footprint 测量（参照 A8 `/tmp/m5-w18-a8-zvec` 做法）。
