# A5 · M5-W18-R2B 检查点（Lane Checkpoint · 设计整包）

```text
LANE=A5
DISPATCH=M5-W18-R2B
STATUS=READY_FOR_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a5/mvp-browser-os-v3
BRANCH=codex/m5-w18-a5
BASE=434e63f  (origin/master @ 2026-09-08，含 R2B 派发文档；本分支 rebase 基线)
HEAD=5f361aa7806d91d3369a6f0e99478a9b76a5e415  (本整包内容提交；checkpoint 为补充提交)
CONSUMED_PEERS=A1(W18-R 产品基线 gap inventory G1-G8 / checklist R1-R12, logs/research/M5-W18/A1-product-baseline-20260908.md) ; A4(dbx backend 架构映射, HEAD=4bce98fd1ca036baff9d95eb98dd40b965c1d378, logs/research/M5-W18/A4-dbx-backend-architecture-map.md) ; A6(凭据/取消/生命周期, 本仓 security_policy.rs + database.rs 既有守卫) ; R2 旧报告(A5-workbench-ux-map.md / A5-replication-blueprint.md / A5-checkpoint.md, 本分支 HEAD=a42b915)
FILES=logs/research/M5-W18/A5-R2B-design.md, logs/research/M5-W18/A5-R2B-state-model.mjs, logs/research/M5-W18/A5-R2B-wireframe.html, logs/checkpoints/A5-M5-W18-R2B-checkpoint.md
CORRECTIONS=R2 已 retract 全部 R1 误述（A5-checkpoint.md §关键纠正 C1-C8）；本包重申：CodeMirror 升级维持 REJECT(M5)/DEFER（无实测体积即不得选），仅作设计分类、不写产品代码；体验参照(IDEA)与源码参照(dbx)分清（A0-R2B 抽查 L40）。
VERIFY=EXECUTED_SYNTHETIC_TEST: node logs/research/M5-W18/A5-R2B-state-model.mjs → 9 断言全过 (exit 0)，覆盖双文档独立/陈旧execId丢弃/取消幂等+重试/截断透传。CURRENT_PRODUCT 复核：未改 src/ src-tauri/ scripts/；仅 logs/ 下新增。git diff --cached --check：PASS。产品 build/cargo/pre-merge/GUI：NOT_RUN（设计包，不越界）。既有 node scripts/check-database-ui-logic.mjs 119 断言基线（未重跑，本包不改其覆盖）。
PROPOSED_SLICES=S2 数据库日常闭环（多 SQL 文档 + 连接树 + 结果窗口）— PROPOSED_NOT_AUTHORIZED；owner=W19 编码 lane；文件范围=useDatabaseStore.ts(+documents Map/activeDocId/runDoc/cancelDoc/applyDocResult)/DatabasePanel.vue(+标签栏/树面板/底部窗口/aria+keydown)/dbUi.ts(+createSqlDoc/applyDocResult/docResultView)/types.ts(+SqlDoc)；依赖SHA=A4 冻结 db_list_connections/db_cancel/历史 DTO(4bce98f) + 25.2% 预算；验收=≥2份SQL文档独立、查询→结果→取消→重试完整、写确认沿用现有机制、覆盖空/NULL/长字段/二进制/截断/错误/断线、无新增依赖、包体不破25.2%。
OPEN_DECISIONS=B-A5-1(High,依赖A4): db_list_connections/db_cancel/历史DTO未冻结 → 连接树填充/取消令牌/历史 W19 才能落地。B-A5-2(High,体积): 25.2%余量≈345B → 任何净新组件须替代既有或A0抬限。B-A5-3(Med,DbValue漂移): database.rs(I64/F64/Binary) vs domain.rs(Int/Float/BlobLen) 双真源，归A1/A4。B-A5-4(Low): 面板级a11y/键盘断言待扩写。编辑器升级: CodeMirror 维持 REJECT(M5)/DEFER，待 A0 在 disposable 工作区授权 footprint 测量。
NEXT=整包交 A10/A11 评审；W19 实现卡（S2）待 A0 开门，依赖 A4 DTO 冻结 + 25.2% 预算解决。
NO_PRODUCT_CODE=true
NO_PUSH=true
```

## 本轮 R2B 交付物

| 文件 | 内容 | 性质 |
|---|---|---|
| `A5-R2B-design.md` | 主报告：7 点结构（更正/证据/流程状态/逐文件计划/测试/候选卡/自检）+ 证据标签 | 新增 |
| `A5-R2B-state-model.mjs` | 零依赖合成状态模型（execId 陈旧守卫/取消幂等/双文档独立），可运行验证 | 新增 |
| `A5-R2B-wireframe.html` | 自包含静态线框（桌面/窄窗、连接树/多SQL标签/底部结果/取消忙碌截断态，合成数据无真实IPC） | 新增 |
| `A5-M5-W18-R2B-checkpoint.md` | 本检查点 | 新增 |

## 设计要点（对齐 R2B §6 A5 任务）

1. **多 SQL 文档状态机**：`documents: Map<docId,SqlDoc>` + `activeDocId`；`applyDocResult(doc,execId,res)` 仅当 `doc.execId===execId` 才提交（陈旧守卫），保证「切换文档旧请求不得覆盖新文档」。已用 state-model 验证（S1/S2）。
2. **连接/库表树**：复用现有 `connections`；schema 树为占位契约，待 A4 `db_list_schema` 落地（明确 WAITING_DEPENDENCY）。
3. **中央多 SQL 控制台 + 底部结果/消息**：结果区即 `WORKBENCH_BLUEPRINT` §3.5 底部工具窗口；每文档独立保存暂存 SQL/连接引用/执行ID/当前结果。
4. **失败分支全覆盖**：空/NULL/长字段/二进制长度/大结果截断/错误/超时取消/断线重连 → 复用 `dbUi.formatCellValue`/`buildResultView`/`DB_LIMITS` 与既有 `useDatabaseStore.connected`。
5. **写确认沿用现有机制**：内联 `.confirm` 或 `ConfirmModal`+`useModalFocus`（二选一，皆属现有）。
6. **键盘/焦点/窄窗**：`Ctrl/Cmd+Enter` 运行、`Esc` 关确认、`Ctrl/Cmd+Alt+[ ]` 切标签；IME 组合键不误触；窄窗收起侧栏、底部降高；复用 `useModalFocus` 做焦点归还。
7. **组件拆分表**：全部扩展既有 `DatabasePanel.vue`/`useDatabaseStore.ts`/`dbUi.ts`/`types.ts`，无净新文件（除可选 composable）；无新增依赖；编辑器保持 `<textarea>`（CodeMirror REJECT）。
8. **J2 测试脚本**：建议扩写 `scripts/check-database-ui-logic.mjs`（陈旧守卫/截断/写确认断言），随 W19 产品代码一并提交。

## 状态

✅ R2B 设计整包完成：主报告 + 状态模型（已运行通过）+ 静态线框 + 检查点。全部位于 `logs/`，**未改任何产品代码、未引入依赖、未 push**。旧 R2 成果保留。待 A0/A10/A11 评审；W19 实现卡（S2）以 `PROPOSED_NOT_AUTHORIZED` 形式就绪，依赖 A4 DTO 冻结 + 体积预算。
