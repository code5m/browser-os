# Lane A6 · M5-W18-R2B Checkpoint — 资源生命周期与恢复合同

- LANE: A6
- ROLE: RESEARCH ONLY（零产品代码、不 push）
- DISPATCH: `M5-W18-R2B`（任务卡 §7「资源生命周期与恢复合同」）· `W19=CLOSED`
- STATUS: PASS_WITH_DEBT（A4 R2B 合同未到位，多文档结果策略待 A5 定稿）
- WORKDIR: `/home/ainfinit/.codex/worktrees/m5-w18-a6/mvp-browser-os-v3`
- BRANCH: `codex/m5-w18-a6`
- BASE: `434e63f`（rebase 后 origin/master 顶；任务卡基线 `d6127c4`）
- HEAD: `38faa2d`（R2 证据闭环，rebase 后）+ 本包待提交（见 VERIFY）
- CONSUMED_PEERS:
  - A4：R1 源映射 `logs/research/M5-W18/A4-dbx-backend-architecture-map.md`（SHA `971378c`）；其 R2B 正式契约截至本包未提交（OPEN_DECISIONS D1）。
  - A9：R2B 任务卡 §10 分工——原生 WebView 边界/debug-release 权限/远程页面威胁侧独立评审；本包 F-* 表中 A6 侧已列，A9 侧交其独立成稿。
  - 蓝图 `WORKBENCH_BLUEPRINT-20260908.md`、指挥板 `PARALLEL_COMMAND_BOARD.md`、A0 `A0-M5-W18-R2B-dispatch-20260908.md`。
- FILES:
  - 新增：`logs/research/M5-W18/A6-R2B-resource-lifecycle-recovery.md`
  - 新增：`logs/checkpoints/A6-M5-W18-R2B-checkpoint.md`
  - 复用（不改）：`logs/research/M5-W18/A6-security-lifecycle-audit.md`（HEAD `38faa2d`）、`A4-dbx-backend-architecture-map.md`
  - 引用源码（只读）：`src-tauri/src/scheduler.rs`、`tasks.rs`、`shutdown.rs`、`session.rs`、`database.rs`；`src/stores/useLayoutStore.ts`、`useDatabaseStore.ts`；`src/App.vue`
- CORRECTIONS:
  - 修订 R2 中"凭据从未进内存"的笼统表述：凭据瞬时经 JS/Tauri 参数为受控过渡态，要求不误持久化/不泄漏，且不得虚称"从未进内存"（蓝图书写为永久防回退项 #5）。
  - 明示"前端布局/SQL 文档恢复"当前未持久化（grep 无 localStorage/atomic_write 落点）= 设计目标，非已实现；避免 R2 报告被误读为"已落盘"。
- VERIFY:
  - 静态核对：`grep -rn "password" src/stores/useDatabaseStore.ts` → 仅 F2 注释（"绝不写入 form/store/localStorage"），无落盘路径（F-CRED-1 READY）。
  - 行号证据：scheduler.rs:415-428（先落盘后启动）、last_fired_at:118/368/961（判重真相源）、tasks.rs:463-466/900-983（原子写/损坏不静默清空）、shutdown.rs:152-174/248-334（关闭幂等/注册拒绝）。
  - 产品构建/GUI=NOT_RUN（R2B 研究包不跑全量构建，蓝图 §7）；git diff --check = 待跑（见提交前自检）。
- PROPOSED_SLICES（PROPOSED_NOT_AUTHORIZED，非开工许可）:
  - `A6-R2B-layout-doc-persistence`：新增 Tauri 命令经 `session::atomic_write` 持久化 `layout-state.json`/`doc-recovery.json`；前端 store 增加 `schema_version` 恢复 action；策略脚本校验"不存 password/不存完整输出/损坏回退"。前置：A1 壳层契约冻结布局字段、A4 R2B 连接契约可用。
- OPEN_DECISIONS:
  - D1：`WAITING_DEPENDENCY(A4-R2B)` — A4 正式请求合同未提交；§2 查询状态机与 F-QUERY-* 待其更新 `conn_id`/`exec_id` 取消句柄契约。
  - D2：多 SQL 文档独立**结果**是否持久化交 A5（蓝图第 93 行禁止完整 SQL/输出全文跟布局落盘，故结果行默认不持久化，仅 pendingSqlText 可选）。
  - D3：持久化 Tauri 命令属产品代码，R2B 禁止 A6 落地，列入候选卡，由 A0 在 W19 授权后交实现 lane。
  - D4：`doc-recovery.json`/`layout-state.json` 容量与淘汰阈值待 A1/A5 冻结。
- NEXT:
  1. 提交本包两文件（NO_PUSH）；
  2. 等待 A4 R2B 合同 → 回填 §2/F-QUERY-*；
  3. W19 授权后由对应实现 lane 取候选卡落地；
  4. 与 A9 交叉评审 S0/S5 故障注入表。
- NO_PRODUCT_CODE: 已确认 `git status` 仅含 logs/ 下两文件，未触碰 `src/`/`src-tauri/`/`scripts/`/ACL。
- NO_PUSH: 本包不执行 push；仅 A0 可在 W19 集成时 push master。
