# A11 · M5-W18-R2B S0–S5 候选任务卡与测试矩阵

> 全部 `PROPOSED_NOT_AUTHORIZED`：本文件是交付 A0/编码 lane 的候选卡，不是开工许可。命令均为**可实际解析**的设计，未实际运行（研究边界），统一标记 `PROPOSED`。**不把两个 cargo 过滤参数并排假装有效命令**；每个 cargo 命令只带一个 `--lib/--test <filter>` 或独立 `--features`。

## 通用验收约定（蓝图 §7）

- native 验收同时覆盖 debug/release、真实 IPC、长查询取消、浏览器切换/布局变化、终端持续运行、退出后进程回收。
- 本轮仅文档/合成研究资产；未来每个合入候选跑一次针对性测试，最终同一候选 SHA 跑全量 + native 验收。
- 只按场景验收计数，不按文档页数/提交数/单测数估算完成百分比（蓝图 §9）。

---

## S0 · 现有契约与可运行基线

- **范围**：DbValue 真实序列化确定唯一源；release 无 Vite 依赖、debug IPC 正确；保留已修正的调度并发/崩溃行为。
- **前置**：A4 DbValue 契约结论、A6 凭据/生命周期、A9 边界、A11 实证。
- **步骤**：读 `database.rs:122` 活契约 → 删除/同源派生 `domain.rs:1094`+`types.ts:603` 镜像 → 校验 release bundle 无 `build.devUrl` → 校验 `dev-capabilities/main.json` 仅 `#[cfg(debug_assertions)]`。
- **测试矩阵（PROPOSED）**：
  - `cargo test --package mvp-browser-os --lib db::` （DbValue serde 单测）
  - `python3 scripts/check-command-set-consistency.py`
  - `bash scripts/check-dev-startup.sh`
  - `bash scripts/pre-merge.sh`
- **失败路径**：DbValue 跨语言字段不一致 → 回退到 `I64/F64/Binary` 单一活契约；release 误带 devUrl → 拒绝合入。
- **GUI 步骤**：启动 Tauri 窗口（非 localhost:1421 浏览器），执行至少一条 IPC 命令验证原生客户端。
- **回滚**：保留 `domain.rs`/`types.ts` 镜像分支，标签回退。

## S1 · 工作台外壳

- **范围**：项目上下文、工具窗口、文档标签、焦点与旧入口映射；以现有文件/终端/浏览器完成 J1。
- **前置**：A1 壳层契约、A3/A5 跨模块签收、原生 WebView 验收方案。
- **步骤**：`MainArea.vue` 挂载工具区/底栏 → `useLayoutStore`/`useWorkspaceStore` 焦点与标签状态 → 旧文件/终端/浏览器入口映射。
- **测试矩阵（PROPOSED）**：
  - `node scripts/check-graph-ui-logic.mjs` （既有 113 断言基线，关注焦点/标签）
  - `node scripts/check-database-ui-logic.mjs`
  - `python3 scripts/measure-build-metrics.py` （确认 ≤25.2% 红线）
- **失败路径**：布局跳动/焦点丢失 → 收起侧栏优先级回退（蓝图 §3 尺寸约束）。
- **GUI 步骤**：桌面 1440×900 / 1920×1080、窄窗 1024×720 / 800×600、DPI 1/1.5/2 各测一次；WebView 坐标遵守 CSS 像素契约。
- **回滚**：布局状态持久化失败时保留上一可用布局。

## S2 · 数据库日常闭环

- **范围**：连接树、独立 SQL 文档、查询/取消/结果。
- **前置**：S0、S1 契约、A4/A5/A6 一致结论。
- **步骤**：`DatabasePanel.vue` + `useDatabaseStore` + `dbUi.ts` 打磨 → 接 A4 `db_list_connections/db_cancel`/历史 DTO → 扩 `check-database-ui-logic.mjs` a11y/键盘断言。
- **测试矩阵（PROPOSED）**：
  - `node scripts/check-database-ui-logic.mjs` （目标 119+ 断言全绿）
  - `node scripts/check-database-policy.py` （DB_* 门禁）
  - `cargo test --package mvp-browser-os --lib db::pool` （连接池/取消）
- **失败路径**：取消令牌未传播 → 引用 A6 竞态时间线 G3 fail-closed 断言。
- **GUI 步骤**：长查询中点击取消，验证结果集中止且连接可复用；窄窗下工具栏不跳动。
- **回滚**：保留旧 `DatabasePanel` 分支。

## S3 · 笔记与关联闭环

- **范围**：已有笔记查看、链接导航、反向链接/局部图谱。
- **前置**：S1 契约、A2/A3 行为证据；**不要求先完成向量搜索**。
- **步骤**：`graph_import_vault` 只读命令（A2）→ `GraphStore` 叠加 `Note/Attachment` 节点 → 右栏 inline 反链/局部图（`useWorkspaceStore.openFileInline`）。
- **测试矩阵（PROPOSED）**：
  - `node scripts/check-graph-ui-logic.mjs` （复用 A3 113 断言 + 反链导航）
  - 移植 A2 §5 合成夹具为 Rust `#[test]`：`cargo test --package mvp-browser-os --lib graph::import_vault`
  - `python3 scripts/check-obsidian-semantics.py` （若 A2 提供；否则 PROPOSED 占位）
- **失败路径**：8370 节点 > `GRAPH_MAX_NODES=5000`（A2 B1）→ 由 A1 容量决策：抬高上限或限定子树。
- **GUI 步骤**：打开 md → 点链接 → 看反链/局部图 → 返回原位（蓝图 J3）。
- **回滚**：rename/delete 链接重写 DEFER 到独立写事务卡，不在 S3 落地。

## S4 · 工作区明确匹配搜索

- **范围**：范围、忽略规则、有界结果、取消、导航。
- **前置**：A7/A8/A9 相同契约；**不要求 zvec 全路线先通过**。
- **步骤**：首波 ADAPT `managed-ripgrep`（A8 R1，亚 20ms 零索引）→ FTS/BM25/vector 经 sidecar ADAPT（默认 local-only）→ 远端嵌入默认拒绝（A9 B2/B3/B7 先修）。
- **测试矩阵（PROPOSED）**：
  - `node logs/research/M5-W18/A8-benchmark-ripgrep.mjs` （回归 11-17ms/420 文件）
  - `node logs/research/M5-W18/A8-benchmark-routes.mjs` （FTS/vector/hybrid 真实执行，recall@10=1.0 复测）
  - `cargo test --package mvp-browser-os --lib search::` （范围/忽略/取消）
- **失败路径**：远端嵌入被启用 → fail-closed 拒绝（A9 授权门 HMAC + 签名 + replay）。
- **GUI 步骤**：输入查询 → 有界结果 + 取消按钮 → 跳转定位；窄窗结果不溢出。
- **回滚**：sidecar 路线回退到纯 managed-ripgrep。

## S5 · 状态恢复与连续使用验收

- **范围**：J1–J6 组合、故障恢复、debug/release、窄窗/DPI、旧功能回归。
- **前置**：S0–S4 至少验收通过。
- **步骤**：组合工作流脚本 → 崩溃/退出恢复 → 回归旧浏览器/终端能力。
- **测试矩阵（PROPOSED）**：
  - `bash scripts/pre-merge.sh`
  - `python3 scripts/measure-build-metrics.py`
  - `python3 scripts/check-command-set-consistency.py`
  - 全量：`cargo test --package mvp-browser-os --lib`
- **失败路径**：恢复会话自动再执行命令 → 违反蓝图 J1（需显式执行）；报告原因不静默。
- **GUI 步骤**：debug + release 双包原生验收；长查询取消 + 浏览器切换 + 终端持续 + 退出后进程回收。
- **回滚**：整体状态快照回退到 S4 验收点。

---

## 跨切片共享约束（写入每张卡的硬停）

- 体积预算 25.2% 红线：单波增量 ≤ 余量（W17 实测 25.14%，余量仅 ~0.06pp）；禁止实现 lane 自抬限，重基线须 A0 书面（A11 R2 §9.2）。
- 禁止引入 daemon/MCP/远程嵌入/模型下载（除非该切片显式授权且 A9 门齐备）。
- 共享壳层、`src/bridge.ts`、`src/types.ts`、`main.rs`、ACL 按明确归属串行落位，避免 11 lane 同改一组文件。
- 许可：产品 MulanPSL-2.0；dbx Apache-2.0 移植署名归 A10；新增 inbound 须 A10 NOTICE/台账。
