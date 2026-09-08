# A11 · M5-W18-R2B A0 摘要（滚动更新 3，≤120 行）

> 用途：A0 低成本接手（见 `A0-HANDOFF-LOW-COST-20260908.md` §4）：读此摘要 + manifest，必要时抽查关键原证据。A11 不合并、不 push。**W18-R2B 全 11 lane 整包现已 READY_FOR_REVIEW**；两项此前缺口（A4 R2、A10 R2B 复核）均已闭合。

## 1. 提交索引（固定 SHA，本包消费方式：`git show <sha>:<path>`）

| Lane | HEAD (full) | R2→R2B 状态 | 关键 R2B 交付 |
|---|---|---|---|
| A1 | eba0d52f0882b3a6a648fc7c8c75c23cec93a692 | READY_FOR_REVIEW | 工作台壳层 + 线框 + checkpoint |
| A2 | 1eb86084ebacded1057b85e49f103d48b5c8f16a | PASS_WITH_DEBT | 笔记语义 + 可复现夹具 19/19（撤回部分 R2 主张） |
| A3 | ab455c1db9d3fe732cdff1aa95a2dff9c51955e6 | PASS_WITH_DEBT | 图谱/笔记导航（未动） |
| A4 | 7f282d0b9891deadd1e2d3be628361794c3b33b9 | READY_FOR_REVIEW | **R2B 证据闭环**：DbValue 根因(B8-1)、23 `#[test]` 声明、dbx=t8y2/dbx |
| A5 | 94c988bdcc92b5b73415475bf14a8d50d0a9baf0 | READY_FOR_REVIEW | DB 工作台设计 + 状态模型 + 线框 |
| A6 | ac239538e6185b595784ddbfbafd0d1d5ce27263 | PASS_WITH_DEBT | 资源生命周期与恢复合同（A4 依赖已解） |
| A7 | 37be95eed91f27526615a3c50b90fc3c376b8f0a | PASS_WITH_DEBT | 检索接入/索引事务证据（与 A8 追平） |
| A8 | ed5c0c8a8c1e0e3490a34cc85dfc9227f519bd8a | READY_FOR_REVIEW | 指标修正（修 A0 #7/#8：命名/RSS 边界） |
| A9 | 66bd51140dc07829840ada02deb2306762d4d72a | PASS | 能力预算 + 信任边界 + WebView 边界 |
| A10 | 3551184865fb76a71eac3a177b46ab6ebc79be82 | READY_FOR_REVIEW | **`A10-R2B-review-findings.md`（F1–F5）** + ledger Z8 和解 |
| A11 | <FILLED_ON_COMMIT> | READY_FOR_REVIEW | 本包（progress/manifest/brief/cards/checkpoint） |

A0 调度文档（origin/master `434e63f`）：`WORKBENCH_BLUEPRINT-20260908.md`、`A0-HANDOFF-LOW-COST-20260908.md`、`A0-M5-W18-R2B-dispatch-20260908.md`、`M5-W18-R2B-TASKS-20260908.md`。

## 2. 阻塞决策（本轮两项缺口均已闭合）

- ~~A4 `b737e5d` R2 缺失~~ → **已闭**：A4 `7f282d0b` 产出 R2B 证据闭环（DbValue 唯一源、23 声明测试、dbx 身份核对）。
- ~~A10 `b81fa69` R2B findings 缺失~~ → **已闭**：A10 `530d7086` 产出 `A10-R2B-review-findings.md` + ledger §9 Z8 和解。
- **整包状态：READY_FOR_REVIEW**。残留“跨 lane 待 A0 裁决”项（§3）不阻塞 A11 交付，仅作为 W19 开放前的决策输入。

## 3. 独立复核结果（A10 findings F1–F5，A11 不重复裁决）

- **F1–F5**（此前已录）：DbValue 序列化 ✅（`database.rs:118-168`）；DB 测试数 = 23（真实缺口 = live-DB 集成测试）；同步驱动 ✅（无 tokio 第二运行时）；dbx pin `c0a7be12` ✅；zvec 绑定/格式 ✅（npm N-API sidecar，Z8 HOLD-(a) 解除；`.node` 41.7MB + 引擎 RSS ~294MB，bundle 影响交 A3/A10）。
- **F6–F12**：许可 MulanPSL-2.0 ✅；F7 zvec 指标命名引用口径交 A0（A8 R2B 已修）；F8 egress 默认关 ✅；F9 apiKey 明文 → keyring（A9 B3）；F10 dispatch 1358 行归属笔误（文档修正）；F11 共享 `ConfirmModal`/`useModalFocus`（A1/A5 壳层协调）；F12 `db_cancel` 冻结顺序 A4 先于 A5。
- **F13 ★HIGH-VALUE 升级（滚动 3 新增）**：产品**双 `DbValue` 枚举线格式分歧**——`database.rs:122`（`i64/f64/binary`，`DbQueryResult.rows` 实线使用）vs `domain.rs:1094`+`types.ts:603`（`int/float/blob_len`）；`dbUi.ts:273 decodeDbValue` 只认后者 → **数值/二进制单元格渲染成原始 JSON**（如 `{"i64":7}`）。B8-1 只修了 `domain.rs`。最小修正：统一单枚举（建议 `int/float/blob_len`）+ 序列化往返测试；**W19 由 A4+A5 修复**（属产品代码，A10 不改）。
- **F14**：dbx 凭据活路径 = SQLite 明文 `connection_secrets` 表（`FileSecretStore` 死代码），A6 更正 A4/A10 ledger D6 已和解；产品 `keyring_store` 正确。
- **F15**：A11 R2 矩阵“无 peer R2”已过时 → A11 滚动更新 2 已解决。

A10 候选实现片（PROPOSED_NOT_AUTHORIZED）：P1 dbx 同步移植（首波）、P2 zvec-grep npm sidecar（须先移 A9 egress 门，F9 阻塞）、P3 远程嵌入凭据 keyring（独立）。

## 4. 可开放切片（S0–S5，PROPOSED_NOT_AUTHORIZED）

按 `WORKBENCH_BLUEPRINT-20260908.md` §6，均**非即时编码授权**：

- **S0 现有契约与可运行基线**：DbValue 唯一源、release 无 Vite、debug IPC、调度并发/崩溃。阻塞：A4/A6/A9 实证（均已 READY）。
- **S1 工作台外壳**：项目上下文/工具窗口/文档标签/焦点 + 旧入口映射（A1 R2B 壳层 + 线框就绪）。阻塞：A3/A5 签收、原生 WebView 验收方案（A9 R2B 已给场景）。
- **S2 数据库日常闭环**：连接树/SQL 文档/查询-取消-结果（A4/A5/A6 就绪）。
- **S3 笔记与关联闭环**：笔记查看/链接导航/反链-局部图（A2/A3 就绪；不要求先完成向量）。
- **S4 工作区匹配搜索**：范围/忽略/有界/取消/导航（A7/A8/A9 契约就绪；不要求 zvec 全路线先过）。
- **S5 状态恢复与连续使用验收**：J1–J6 组合、故障恢复、debug/release、窄窗/DPI、旧功能回归。

建议开放顺序：S0 → S1 →（S2/S3/S4 并行）→ S5；每片单独登记 `READY/BLOCKED/DEFERRED` + 唯一 owner + 文件范围 + 依赖 SHA + 验收（蓝图 §7）。**禁止全仓默认开放**。A9 B2/B3/B7 须在任意 zvec 能力发货前修复。
- A10 候选片对齐：P1 ↔ S0/S2（dbx 同步移植 + F13 DbValue 统一）、P2 ↔ S4（zvec npm sidecar，先 A9 门）、P3 ↔ S0（凭据 keyring）。

## 5. 证据链接

- 逐 lane 矩阵：`A11-R2B-progress.md`
- 合入清单（文件/范围/顺序）：`A11-R2B-integration-manifest.json`
- 候选卡 + 可解析测试矩阵：`A11-R2B-S0-S5-task-cards.md`
- A10 独立复核：`logs/research/M5-W18/A10-R2B-review-findings.md`
- A4 证据闭环：`logs/research/M5-W18/A4-R2B-backend-evidence-closure.md`

## 6. 结论

A11 R2B 整包就绪可审；**全 W18-R2B 闭环现已齐备**（A4 R2B、A10 R2B findings 均到位）。残留仅跨 lane 待 A0 裁决项（A7 跨绑定/缓存、dbx 身份、许可、前端测试措辞、A9 B2/B3/B7）。未 push、未改产品代码。
