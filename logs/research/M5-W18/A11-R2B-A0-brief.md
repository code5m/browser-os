# A11 · M5-W18-R2B A0 摘要（≤120 行）

> 用途：A0 低成本接手（见 `A0-HANDOFF-LOW-COST-20260908.md` §4）：读此摘要 + manifest，必要时抽查关键原证据。A11 不合并、不 push。

## 1. 提交索引（固定 SHA，本包消费方式：`git show <sha>:<path>`）

| Lane | HEAD (full) | 状态 | 关键交付 |
|---|---|---|---|
| A1 | 526e2ef628c0f10617a816c7783a9b3ef72de3ad | PASS_WITH_DEBT | R2 基线 + 6 项 R1 纠正 + 机器可查事实 |
| A2 | bcdfc3bca922c398422f8efd652900896eb1652e | PASS_WITH_DEBT | Obsidian 语义 + 合成夹具 20/20 |
| A3 | ab455c1db9d3fe732cdff1aa95a2dff9c51955e6 | R2B_PASS_WITH_DEBT | 图谱/笔记导航设计，113 UI 断言 |
| A4 | b737e5d4cc285ceea507154cc6e0f261e2e8c01e | **R1 only** | dbx 架构图（**缺 R2 闭环**） |
| A5 | a42b9154a8e2524a13f64d26f7a9815cc1345515 | PASS_WITH_DEBT | 数据库工作台蓝图 + 8 项纠正，119 UI 断言 |
| A6 | 38faa2da7d521840966c83575cf152a573a68f91 | PASS_WITH_DEBT | 凭据审计 REWORK_CLOSED + fail-closed 时间线 |
| A7 | 07fda2f87c319f6731d065f69da1e00dae944a00 | PASS | zvec 原生绑定/索引（A0 抽查 2 项待核） |
| A8 | f4a4f3bab71df7322a96c499facf201e35ffd441 | PASS_WITH_DEBT | 检索路线真实 benchmark，COPY=0 |
| A9 | 28a934add0e09e26d42cd4759438e29f1300cba3 | PASS | zvec 威胁模型，egress 门强 |
| A10 | b81fa690e9ae3d4e12267cbabc8548ab1095e96f | PASS_WITH_DEBT/HOLD | 移植台账/BOM/NOTICE（**R2B findings 未产**） |
| A11 | <FILLED_ON_COMMIT> | R2B_READY | 本包（progress/manifest/brief/cards/checkpoint） |

A0 调度文档（origin/master `434e63f`）：`WORKBENCH_BLUEPRINT-20260908.md`、`A0-HANDOFF-LOW-COST-20260908.md`、`A0-M5-W18-R2B-dispatch-20260908.md`、`M5-W18-R2B-TASKS-20260908.md`。

## 2. 阻塞决策（WAITING_DEPENDENCY，精确 SHA 缺口）

- **A4 `b737e5d` — R2 证据闭环缺失**：dispatch 原要求 A4 补 R2（身份核对/DbValue 契约）；当前仍是 R1 架构图。A11 终稿门禁不达标，**不宣布 W18 完成**直至 A4 补 R2。
- **A10 `b81fa69` — R2B 独立复核 findings 未产出**：peer R2 现已齐，A10 应重读后产 `A10-R2B-review-findings.md` + ≤2 页决策摘要（任务卡 §11）。A10 当前仍 HOLD。

二者为外部缺口；A11 本包自身已可审（READY_FOR_REVIEW）。

## 3. 独立复核结果（引用 A0 抽查，待 A10 复核对照）

- **A2**：§5 自写 resolver 20/20 中 alias/同名/大小写推断未见独立验证 → 兼容性结论需官方/实际观察或标设计选择。
- **A7**：跨绑定互读/崩溃实测未提供（WAL/写锁互换、索引互换），不能 PROVE/ADOPT；缓存 `force_rebuild` 清理用户根 `.zvec-grep` 会冲突外部索引归属 → 改产品自有缓存命名空间 + 代际提交。
- **A8**：vRecall/fRecall/zRecall 分母=结果数或 10（相关结果占比，非真实召回率）；向量为 topic 中心加噪声（不测真实 embedding 语义）；RSS/duration 不等价全程 OS 峰值/p95。
- **A7/A8 矛盾**：A7 称可用 Rust binding，A8 总结只能 Node sidecar → 分别验证后 A0 裁决。
- **A0 自身更正**：`src-tauri/src/database.rs` 实际 **23** 行独立 `#[test]`（R1 audit 写 22 不准确）；声明计数 ≠ 本次执行通过数。
- **许可**：产品根 LICENSE = **MulanPSL-2.0**（非 Apache-2.0）；dbx = Apache-2.0，移植署名归 A10。
- **前端测试口误**：A1/A11“0 frontend”未覆盖既有 `scripts/check-*-ui-logic.mjs` → 改说“缺某框架”而非“无”。

## 4. 可开放切片（S0–S5，PROPOSED_NOT_AUTHORIZED）

按 `WORKBENCH_BLUEPRINT-20260908.md` §6，均**非即时编码授权**：

- **S0 现有契约与可运行基线**：查 DbValue 真实序列化定唯一源；确认 release 无 Vite、debug IPC 正确；保留调度并发/崩溃行为。阻塞：A4/A6/A9 及 A11 实证。
- **S1 工作台外壳**：项目上下文/工具窗口/文档标签/焦点 + 旧入口映射。以现有文件/终端/浏览器完成 J1。阻塞：A1 壳层契约、A3/A5 签收、原生 WebView 验收方案。
- **S2 数据库日常闭环**：连接树/独立 SQL 文档/查询-取消-结果。阻塞：S0、S1 及 A4/A5/A6 一致结论。
- **S3 笔记与关联闭环**：已有笔记查看/链接导航/反链-局部图。阻塞：S1 + A2/A3 行为证据；不要求先完成向量搜索。
- **S4 工作区匹配搜索**：范围/忽略/有界结果/取消/导航。阻塞：A7/A8/A9 相同契约；不要求 zvec 全路线先过。
- **S5 状态恢复与连续使用验收**：J1–J6 组合、故障恢复、debug/release、窄窗/DPI、旧功能回归。

建议开放顺序：S0 → S1 →（S2/S3/S4 并行）→ S5。每个切片单独登记 `READY/BLOCKED/DEFERRED`，唯一 owner + 文件范围 + 依赖 SHA + 验收（蓝图 §7）。**禁止全仓默认开放**。

## 5. 证据链接

- 完整逐 lane 矩阵：`A11-R2B-progress.md`
- 合入清单（含文件/范围/顺序）：`A11-R2B-integration-manifest.json`
- 候选卡 + 可解析测试矩阵：`A11-R2B-S0-S5-task-cards.md`
- 跨 lane 待裁决差异：本摘要 §3 + progress §3

## 6. 结论

A11 R2B 整包就绪可审；**全 W18 闭环仍 WAITING** A4 R2 与 A10 R2B 复核两项缺口。不 push、未改产品代码。
