# A5 · M5-W18-R2 检查点（Lane Checkpoint · 证据闭环）

- **Lane**：A5 — dbx 桌面工作台 & 数据网格 UX（RESEARCH，第 5 合并序）
- **工作树**：`/home/ainfinit/.codex/worktrees/m5-w18-a5/mvp-browser-os-v3`（分支 `codex/m5-w18-a5`，已 rebase `origin/master`=`d6127c4`，干净）
- **R1 审计结论**：`REWORK`（`logs/checkpoints/A0-M5-W18-R1-audit-20260908.md` L40：引用不存在的 `useConnectionStore`/过时契约，未对齐真实 `DatabasePanel.vue`）
- **R2 状态**：`PASS_WITH_DEBT`（已 retract 全部误述、补齐真实前端盘点、给出最小改动蓝图；剩余债务为外部依赖与体积红线）
- **研究性质**：仅研究 + 报告/checkpoint，**未改任何 `src/` 产品代码，未 push**（符合 W18-R 与 R2 限制）

## R2 要求输出块（board L1416）

```
LANE=A5
STATUS=PASS_WITH_DEBT
BASE=d6127c4  (origin/master @ 2026-09-08)
HEAD=169dfc45980a61e25583ccad88ddccf0878e5c60
REFERENCE_EVIDENCE=logs/research/M5-W18/A5-workbench-ux-map.md §3 (dbx 实测字节/符号); src/components/workspace/DatabasePanel.vue; src/stores/useDatabaseStore.ts; src/utils/dbUi.ts; src/bridge.ts:375-382; src/types.ts:587-644; src/components/layout/MainArea.vue:50; scripts/check-database-ui-logic.mjs; scripts/measure-build-metrics.py:39,63-77; logs/m0-build-metrics/build-metrics-4f0e8ab.json; /home/ainfinit/Documents/极智简单/V3/research/dbx-src (Apache-2.0)
FILES=logs/research/M5-W18/A5-workbench-ux-map.md, logs/research/M5-W18/A5-replication-blueprint.md, logs/research/M5-W18/A5-checkpoint.md
SOURCE_MAP=dbx: ConnectionTree.vue(132311B)/queryStore.ts:105-106(CANCEL常量)/EditorToolbar.vue(32173B)/DataGridToolbar.vue:18(Capability类型)/historyStore.ts(6794B)/QueryLoadingState.vue(1843B)/ErrorBanner.vue(4207B)/DataGrid.vue(661097B); 本仓: DatabasePanel.vue(11278B)/useDatabaseStore.ts(6716B)/dbUi.ts(16771B)/bridge.ts(3命令)
CLASSIFICATION=COPY(2: 能力对象数据形态, requestSerial模式) | ADAPT(连接树/执行工具栏/结果网格/键盘a11y/加载错误/响应式) | REIMPLEMENT_FROM_BEHAVIOR(取消进度/历史, 等A4 DTO) | DEFER(Schema浏览器/wifi历史持久化/CodeMirror) | REJECT(290KB编辑器内核/macOS红绿灯/useConnectionStore新store)
VERIFY=EXECUTED_SYNTHETIC_TEST: node scripts/check-database-ui-logic.mjs → 119 断言全部通过 (exit 0). CURRENT_PRODUCT 复核: grep -rn useConnectionStore src = 0; package.json 无 vitest/test; ls src/ 无 lib/; DatabasePanel aria/keydown grep -c = 0.
CHECKPOINT=logs/research/M5-W18/A5-checkpoint.md
MERGE_NOTES=依赖 A4(db_list_connections/db_cancel/历史 DTO 冻结)、A6(凭据/取消生命周期)、A3(连接桥)；体积红线 25.2%(余量≈345B) 约束所有 UI 改动；本仓库 LICENSE=MulanPSL-2.0，dbx=Apache-2.0，移植署名归 A10。
NEXT=W19 实现卡（待 A0 开门）：按本蓝图打磨既有 DatabasePanel/useDatabaseStore/dbUi；扩 check-database-ui-logic.mjs 的面板 a11y/键盘断言；接 A4 DTO 后落地取消令牌与历史。
```

## 本轮 R2 交付物（本目录）

| 文件 | 内容 | 变更性质 |
|---|---|---|
| `A5-workbench-ux-map.md` | 证据层：R1→R2 纠正表(8 项)、CURRENT_PRODUCT 真实盘点、dbx 逐能力实测映射、体积硬约束、许可证（MulanPSL-2.0 更正） | 重写 |
| `A5-replication-blueprint.md` | 最小改动蓝图：零净增/替代既有、分类修正、测试对齐真实 `.mjs` 门禁、345B 红线 | 重写 |
| `A5-checkpoint.md` | 本文件 | 更新 |

## 关键纠正（retract R1）

- C1 `useConnectionStore` 不存在 → 真实仅 `useDatabaseStore.ts`。
- C2 `check-database-ui-logic.mjs` 已存在且 119 断言通过，非待办。
- C3 懒加载仍计入 `total_bytes` 门禁（collect_dist 全量求和），不解决体积上限。
- C4 产品无 Vitest，须沿用 `.mjs` 门禁，不引入 Vitest。
- C5 `src/lib/` 不存在，键盘归一化落 `src/composables/` 或 `dbUi.ts`。
- C6 取消常量在 `queryStore.ts:105-106`（非 queryExecutionState.ts）。
- C7 `DataGridToolbarActionCapability` 在 `DataGridToolbar.vue:18`（非 lib/dataGrid）。
- C8 `check-ui-a11y-logic.mjs` 仅覆盖 modalA11y（9 断言），不覆盖 grid/toolbar a11y。

## 未解决依赖（交 A0 / 等待，记债务）

- **B-A5-1（High）**：`db_list_connections` / `db_cancel` / 历史命令 DTO 未冻结（归 A4）→ 连接树填充、取消令牌、历史三大能力 W19 才能落地。
- **B-A5-2（High）**：25.2% 余量 ≈ 345 B → M5 内任何净新组件不可行；须替代既有代码或 A0 抬限。
- **B-A5-3（Med）**：后端 `database.rs` 的 `DbValue`（`I64/F64/Binary`）与 `domain.rs`（`Int/Float/BlobLen`）双真源（A1 G4），A5 仅消费 `types.ts` 镜像，漂移修复归 A1/A4。
- **B-A5-4（Low）**：面板组件级 a11y/键盘断言待扩写（现有门禁只覆盖 `dbUi.ts` 纯逻辑）。

## 状态

✅ R2 证据闭环完成：全部 R1 误述已显式 retract，真实前端已盘点，最小改动蓝图已给出，体积红线已用实测数据论证。工作树干净（仅 `logs/research/M5-W18/A5-*` 三文件）。**未 push**。待 A0 评审并打开 W19 实现（依赖 A4 DTO 冻结 + 体积预算解决）。
