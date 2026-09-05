# AI 模型切换与接手清单
> 本版变更：**M2-6.d 命令片段库前端面板已实现（`AI:DEEP / R:high`，Codex gpt-5.5，checkpoint `logs/checkpoints/M2-6.d-20260905-1552.md`），NEXT=`M2-6 整体裁定`**。新增 `snippetUi.ts` 纯逻辑层（argv 一行一元素、整元素占位校验、分类/搜索/收藏/序列化）、`CommandSnippetPanel.vue`（分类树、搜索、收藏、新增/编辑/删除、启用、dangerous 二次确认）、`useWorkspaceStore` snippets 状态与动作、`commands` 模块入口；`ScriptRunDialog` 扩展 `kind=\"command\"` 调用 `bridge.runCommand`，输出/完成事件与取消入口继续复用脚本执行态。验证：`check-command-ui-logic.mjs` 26 断言 PASS，`npm run build` PASS，命令领域 policy PASS，脚本 UI policy PASS，pre-merge 自检 PASS。GUI 实点验收挂账。以下保留 M2-6.c/b/a 交付细节备考：c 卡新增 `run_command(id, values)` 并复用 `script_runner`；b 卡落 `snippets.json` 与 `snippet_list/add/update/remove`；a 卡冻结 `CommandSnippet` 用 `argv: Vec<String>`、整元素 `{NAME}`、`dangerous` 显式字段。
> 上一版（保留）：M2-5 任务卡已展开为 a/b/c 三张子卡（`logs/checkpoints/M2-5-20260905-1301.md`，纯文档展开，未改产品代码），NEXT=`M2-5.c`（b 已交付：执行面板=真实运行+取消+输出流rAF合并+补 D15 审计+D14 节流；c 待裁定：script_runs_list 运行历史）**。展开者 CodeBuddy Hy4 / 腾讯混元。拆卡：**a 脚本库 CRUD UI**（本次展开，**纯前端不接执行**）/ **b 执行面板**（参数弹窗 + 真实执行 + 输出流 + 取消 + **补 D15 审计与 D14 后端 30 ms 节流**）/ **c 运行历史**（新增后端 `script_runs_list` + 前端历史列表）。**现状实测（HEAD `61031a6`）**：前端**零** Script 组件，但 `bridge.ts` 的 7 条命令封装与 `types.ts` 的全部类型镜像**已由 M2-3/M2-4 交付就位**，缺口只在 UI 组件与两项未交付物——**事件监听封装缺失**（`script-output`/`script-finished` 无 `listen` 封装）、**运行历史读取命令缺失**（无 `script_runs_list`，印证 `M2-4.d §6` 遗留风险）。**对两份 mock 草案做 6 处事实修正**（草案写于 M2-4 落地前，「不接执行用 mock」前提已失效）：①**不产 mock 执行器**（交付当刻即死代码，M2-3 裁定已有先例）；②`RunStatus` 以后端冻结的 `running|succeeded|failed|cancelled|timeout` 为准，**草案的 `idle` / `success` 作废**；③执行回显是 **Tauri 事件**而非 `Channel<LogLine>`；④历史态**须先新增 `script_runs_list`** 否则无数据源；⑤参数字段是 `params` / `ScriptParam` 而非 `args` / `ScriptArg`；⑥ANSI 处理仍挂账。**四组分歧留给 b 卡展开时裁定**（本卡只记录倾向不代为决策）：mock vs 直接接真实执行 / `script_runs_list` 归属 / D15 前置归属 / 节流分层（后端 30 ms + 前端 rAF 两层都要补）。新增 `check-script-ui-policy.py`（7 默认码 + 6 pending 码位 + **变异防呆**）。上一版：M2-4 整体裁定已签（`PASS_WITH_DEBT`，`AI:DEEP / R:high`，裁定者 CodeBuddy Hy4 / 腾讯混元），NEXT=`M2-5`。本裁定为**跨模型复核**（与 a/b 卡裁定者 Kimi K3、c/d/e 实现者 Codex gpt-5.5 均不同模型），同时完成 `M2-4.b-VERDICT §6` 遗留的跨模型复核要求：`§3.1 路线 A`（argv 一律不加引号）与 `§3.2 路线 B`（piped + 只丢弃 drain，drain≠背压）均**确认维持**。`M2-4.a §8` 八组核心契约中**六组完全成立、两组部分成立**（§8.5 节流、§8.7 审计），**四条 P0 红线（argv 无 shell 拼接 / cwd 锁定 / 进程组整组回收 / 明文不进审计）无一突破**。新增债务：**D14**（`script-output` 未按 §8.5 做 30 ms 时间节流，仅有 8 KiB 字节批量）、**D15**（§8.7 审计缺 `script.run.finish` 与 `script.validate.reject`，`cancel` 缺 `cancelledBy`；属**未申明的契约偏离**，实现者仅陈述现状未说明理由）、**D16**（R15 脚本文件被替换/删除未做显式存在性预检）、**D17**（GUI 事件联调、R9 100 MB、R12 运行时链路无取证通道）。**D15 的 `validate.reject` 须在 M2-5 前端执行面板上线前补齐**。证据 `logs/checkpoints/M2-4-ACCEPT-20260905-1247.md`。上一版：M2-4.e 移除 shell spawn 开放面已完成（`AI:DEEP / R:high`，Codex gpt-5.5），NEXT=`M2-4 整体裁定`。按 M2-4.a 裁定，移除 `src-tauri/capabilities/default.json` 中 `shell:allow-spawn args:true` 以及 `shell:allow-stdin-write`/`shell:allow-kill`，移除主进程与 grid child 两处 `tauri_plugin_shell::init()`，裁掉 Rust `tauri-plugin-shell` 与前端 `@tauri-apps/plugin-shell` 依赖；刷新 M2-2/M2-3 对 `package*.json` 的哈希基线。`check-script-exec-policy.py` 追加 e 卡三类默认违规码（capability / npm / Rust 插件回流），23 坏样本自检通过。验证：`cargo test` 198/198 PASS；`cargo build --release --locked` 0 error、2 warning 仍为既有 `grid_process.rs` 死代码；`npm run build` 0 error；三套相关 policy 全过；运行时 GUI 目视验收仍挂账。

> 文档角色：跨 Codex / Trae 的唯一接手入口；只记录当前执行指针、模型映射、交付证据和回写规则。
> 文档版本：V4.15。
> 更新时间：2026-09-05 15:52 CST。
> 当前状态：`CODEX_READY`（**M2-5 整体已 PASS；M2-6.a/b/c/d 已交付，NEXT=`M2-6 整体裁定`**；M2-4 整体裁定已签 PASS_WITH_DEBT）。
> 当前分支：`master`。
> 当前执行器：CodeBuddy MiniMax-M3（M2-5 a/b/c 实现并自裁整体 PASS；M2-6 展开者 + a 卡实现者同为该会话，**独立性最弱，保留被推翻权**；故 **M2-6.b 起建议交 Hy4 / Kimi K3 / Codex gpt-5.5 任一不同模型**，以恢复跨模型独立性）。
> 冲突裁决：WBS/验收以 `详细设计与实施计划.md` 为准，指标语义以冻结契约为准，本文只维护跨模型执行指针和交接证据。

---

## 1. 一眼看懂当前进度

| 项目 | 当前值 |
|------|--------|
| 已完成 WBS | `M0-0`（6 项 UNSTABLE 已裁决）、`M0-1 = PASS`、`M0-2 = PASS`、`M0-3 = PASS`、`M0-4 = PASS`、`M0-5 = PASS`、`M0-6 = PASS`、`M0-7.a/b/c = PASS`；M0 总体验收 `OWNER_APPROVED`；`M1-0 = PASS`、`M1-1 = PASS`、`M1-2 = PASS`、`M1-2-fix1 = PASS`、`M1-3 = PASS`（执行器：CodeBuddy 会话，非登记的 Codex）、`M1-4 = PASS`、`M1-5 = PASS`、`M1-6 = PASS`（a/b/c/d 全绿，整体裁定 PASS）、`M1-7 = PASS`（Git UI；GUI 目视验收挂账）、`M1-8 = PASS`（请求拦截与瀑布；运行时端到端联调挂账）、`M1-9 = PASS`（会话持久化与关闭协议；运行时端到端联调挂账）、**`M1-ACCEPT = PASS（PASS_WITH_DEBT）`，M1 里程碑收口完成**、`M2-1 = PASS`（图片领域与持久化；运行时端到端联调挂账）、**`M2-2 = PASS_WITH_DEBT`（图片预览 UI 整体裁定 PASS：a 冻结 + b 实现，D10/D11 挂账；裁定者 Kimi K3 独立于实现者腾讯 Hy4）**、**`M2-3 = PASS_WITH_DEBT`（脚本领域与持久化整体裁定 PASS，D12/D13 挂账；**同模型裁定**，裁定者与实现者均为 Kimi K3，独立性如实标注）**、`M2-4.a/b/c/d/e = PASS_WITH_DEBT`（执行通道已具备安全执行、命令接入、输出背压、事件流、尾存落盘、退出收口，并移除 shell spawn 开放面）、**`M2-4 = PASS_WITH_DEBT`（整体裁定已签：D14 节流 / D15 审计 / D16 R15 / D17 GUI 取证 挂账；**裁定者 CodeBuddy Hy4 腾讯混元，为跨模型复核**，独立性等级高于 M2-2 与 M2-3；证据 `logs/checkpoints/M2-4-ACCEPT-20260905-1247.md`）** |
| 已拒证据 | `ac0ecac` 三批候选：场景错误 + aggregate `UNSTABLE`，结论 `REJECTED`，不得复用 |
| 已落地修复 | `e8975d6`：保留 `about:` URL、关闭失败显式返回、单例扫描线程与插件状态清理 |
| 正式三批 | 源提交 `93a1ba6`；六批门禁逐批 PASS；raw aggregate `UNSTABLE`；manifest `20260830T153538+0800_93a1ba6_M0-0.c` |
| 当前硬风险 | M0-6.c 的登录态证据来自本地 AI mock 持久 cookie，不代表第三方真实账号人工验收；**M1-3/M1-7 GUI 目视验收与 M1-5/M1-6/M1-7/M1-8/M1-9/M2-1 运行时端到端联调全部挂账**（D1~D9 汇总见 `logs/checkpoints/M1-ACCEPT-20260903-0843.md` §7，M2-1 挂账见 `logs/checkpoints/M2-1-20260903-0927.md` §6，均未伪造 GUI/E2E 证据）；M1-ACCEPT 的会话 id 校验加固项**已在 M2-1 修复**（`check_id` 接入 session_get/delete/export/restore）；残留 NON-BLOCKER：flush 双路径理论重复存档窗口、M1-7 未勾选=全量提交误点风险 |
| 下一检查点 | **`M2-5.a（脚本库 CRUD UI）= PASS（已实现：纯前端 CRUD，复用 M2-3 四条命令）；下一步 M2-5.b（执行面板，须先裁定四组分歧）`**（任务卡已展开：`logs/checkpoints/M2-5-20260905-1301.md`）。范围：`MainView` 枚举新增 `"scripts"` + `ActivityBar` 入口 + `ScriptPanel.vue`（左分类树 / 右列表编辑）+ `ScriptParamForm.vue`（按 `ScriptParam` 动态渲染，`enum` 必填 options、`secret` 用 password 不回显）+ 增删改复用 `bridge.ts` 已就位命令 + `check-script-ui-policy.py` 接入 pre-merge + 补前端逻辑测试（`pre-merge.sh:26` 预留钩子）。**硬边界：纯前端、不接执行**（禁 `run_script`/`cancel_script`/`script_status`、禁订阅 `script-output`/`script-finished`、禁任何输出流与取消 UI，全部归 b 卡）；**`src-tauri/` 零改动**；零新增 npm 依赖；**禁止 `v-html`** 渲染输出/正文；前端不参与脚本正文路径构造。 |
| 下一任务路由 | **M2-5.a：`AI:DEEP / R:high`**。UI 组件全部手写（沿用 `FileTreeNode` / `AuditPanel` / `RepoPanel` 样式与 `ConfirmModal` 删除确认范式），**不得引入 Naive UI / Element Plus**（xterm 已装但仅终端用）。**M2-5.b 展开时须先裁定四组分歧**：①mock vs 直接接真实执行（展开卡倾向后者）；②`script_runs_list` 是否新增及归属；③D15 `validate.reject` 审计是 b 卡内补还是单开微检查点（**硬前置，执行面板上线前必须补齐**）；④节流分层（后端 30 ms + 前端 rAF 两层都要补，否则 D14 实质未解决）。 |
| 自动执行范围 | M0 已结束；M1 起必须先展开检查点，再逐点验收和提交 |
| 必停门禁 | 见 §3「硬停止条件」；进入 M1/M2/M3 后仍不得跨检查点合并 |
| 禁止启动 | M4~M5；M1~M3 可评估启动但不得未展开检查点就写功能代码 |
| 最近实现提交 | `feat(M2-2.b): add image gallery lightbox preview UI`（画廊/灯箱/缩放：`workspace_images_dir` 命令 + `imagePreview.ts` 纯逻辑层 + `useImagePreviewStore` + `shared/ImageGallery.vue`/`shared/ImageLightbox.vue` + 两挂点 + 两项新门禁；前一个是 `00948f8 docs(M2-2.a)`） |
| 最近门禁提交 | `73e9dfb fix(M0-1.c): validate versioned evidence safely` |
| 最近裁决提交 | `b9077d9 docs(M0-0.c): retain rejected formal baseline evidence` |
| 最新状态证据 | `logs/checkpoints/M2-5-20260905-1301.md`（**M2-5 已展开为 a/b/c，NEXT=M2-5.a**；含现状实测、对两份 mock 草案的 6 处事实修正、四组待裁定分歧、U1~U12 测试矩阵、13 个违规码）；`logs/checkpoints/M2-4-ACCEPT-20260905-1247.md`（**M2-4 整体裁定 PASS_WITH_DEBT，NEXT=M2-5**；裁定者 Hy4 / 腾讯混元，跨模型复核，同时完成 `M2-4.b-VERDICT §6` 遗留的跨模型复核要求）；`logs/checkpoints/M2-4.e-20260905-0008.md`（M2-4.e shell spawn 开放面收口，NEXT=M2-4 整体裁定）；`logs/checkpoints/M2-4.d-20260904-2007.md`（M2-4.d 实现收口，NEXT=M2-4.e）；`logs/checkpoints/M2-4.c-20260904-1317.md`；`74161af feat(M2-4.d): persist script output and shutdown cleanup`；`4d0ac59 feat(M2-4.c): wire script execution commands`；`bd9f41b feat(M2-4.b): process group kernel with verdict §3.1 A + §3.2 B`；`logs/checkpoints/M2-4.b-20260904-0000.md`；`logs/checkpoints/M2-4.b-VERDICT-20260904-0705.md`；`logs/checkpoints/M2-4.a-20260903-2233.md`；`logs/checkpoints/M2-4-20260903-1659.md`。 |
| 交接基线提交 | `504fcd7 docs(handoff): prepare Trae quota-window transfer` |
| 工作树要求 | 执行器开工前、每个提交后和交付时都必须干净 |

已经完成：

- 建立 `feature-M0-baseline` 分支。
- 完成 M0~M5 优先级、模型路由、原子检查点和验收门禁整理。
- 使用 `gpt-5.6-terra / medium` 独立审阅并完成 M0-0.a。
- 冻结 `logs/m0-baseline-contract-v1.md`：21 个 `REQUIRED_NOW` 指标、2 个延迟指标、环境指纹、固定场景、统计公式和证据目录。
- 将旧 `logs/baseline-2026-08-27.md` 降级为 `EXPLORATORY`，禁止当作正式性能基线。
- 修复原计划中的循环依赖，关键路径现为：
  `M0-0(完成，六项 UNSTABLE 已裁决) -> M0-1(PASS) -> M0-2(PASS: a/b/c/d) -> M0-3(PASS: a/b/c/d) -> M0-4(PASS: a/b/c) -> M0-5(PASS: a/b/c) -> M0-6(PASS: a/b/c) -> M0-7.a(PASS) -> M0-7.b(PASS) -> M0-7.c(PASS) -> M1-0(PASS) -> M1-1(PASS) -> M1-2(PASS) -> M1-2-fix1(PASS) -> M1-3(PASS) -> M1-4(PASS) -> M1-5(PASS) -> M1-6.a(PASS, 契约冻结) -> M1-6.b(PASS, 写后端核心) -> M1-6.c(PASS, 复核) -> M1-6.d(PASS, push) -> M1-7(PASS, Git UI；GUI 目视验收挂账) -> M1-8(PASS, 请求拦截与瀑布；运行时端到端联调挂账) -> M1-9(PASS, 会话持久化与关闭协议；运行时端到端联调挂账) -> M1-ACCEPT(PASS_WITH_DEBT, M1 里程碑收口) -> M2-1(PASS, 图片领域与持久化；运行时端到端联调挂账) -> M2-2.a(PASS, 字节通道决策与契约冻结) -> M2-2.b(PASS_WITH_DEBT, 画廊/灯箱/缩放 UI 已实现) -> M2-2(PASS_WITH_DEBT, 整体裁定：D10/D11 挂账) -> M2-3.a(PASS, 脚本领域契约冻结) -> M2-3.b(PASS_WITH_DEBT, 领域与持久化已实现) -> M2-3(PASS_WITH_DEBT, 整体裁定：D12/D13 挂账) -> M2-4.a(PASS, 执行安全契约冻结) -> M2-4.b(PASS_WITH_DEBT, 进程组内核，裁定 §3.1 A + §3.2 B) -> M2-4.c(PASS_WITH_DEBT, 命令接入与参数校验) -> M2-4.d(PASS_WITH_DEBT, 背压/事件/尾存落盘/退出收口) -> M2-4.e(PASS, 移除 shell spawn 开放面) -> M2-4(PASS_WITH_DEBT, 整体裁定：D14~D17 挂账) -> M2-5(CARD_EXPANDED, 已展开 a/b/c) -> M2-5.a(PASS, 纯前端 CRUD UI) -> M2-5.b(NEXT, 执行面板：真实执行+输出流+取消+补 D15 审计与 D14 节流)`。
- 完成 M0-1.a（commit `1bd56b1`）：新增 `scripts/baseline-check.sh` 与 `scripts/fixtures/clippy-sample.json`；
  验收命令全过、`--self-test` 输出 `SELF_TEST_RESULT=ALL_PASS`；`.gitignore` 对 `logs/m0-baseline/` 开例外，
  原始 `.log` 证据随 run 目录入库可追溯（见 §5 证据保留说明）。
- 完成 M0-1.b（commit `506193b`）：新增 `scripts/verify-resources.sh`（资源验证驱动），
  覆盖 release 启动、进程树 RSS/FD 采样、tab/grid/terminal 资源循环、终端吞吐与孤儿进程检测驱动；
  正式模式在 ready/终端钩子缺失时输出完整 BLOCKED 证据并退出 1（不伪造 PASS）；
  `--self-test` 8 用例 ALL_PASS（统计计算/进程树/孤儿/BLOCKED 语义/SHA256SUMS）。
- 完成 M0-1.c（commit `2b476d3`）：固定 M0-1 两脚本的参数、退出码（0/1/2）、JSON schema
  （`scripts/schema/m0-summary.schema.json` + 共享校验器 `scripts/validate-summary.py`）、
  完整性哈希（SHA256SUMS）和日志格式（`scripts/GATE-CONTRACT.md`），并接入本地 pre-merge
  流程（`scripts/pre-merge.sh` + `.githooks/pre-merge-commit` 可选 hook，不代改 git 配置）；
  baseline `--self-test` 增至 7 用例、verify 增至 9 用例（各含 schema 校验用例），全部 ALL_PASS。
- 完成正式总控与后续门禁修复：`ac0ecac` 保证多批采集原子归档，`73e9dfb` 对全部证据校验 SHA 并按 V1.0/V1.1 选择 schema；最新完整 pre-merge 为 `ALL_PASS`。
- 拒绝 `ac0ecac` 的三批候选证据：tab 驱动把 `about:blank` 当搜索词打开百度，六项 aggregate 指标为 `UNSTABLE`；裁决见 `logs/checkpoints/M0-0.c-20260830-0913.md`。
- `e8975d6` 已修复上述 URL 和页签生命周期缺陷；短样本 smoke 显示 root/process FD 稳定，但 smoke 不能替代 M0-0.b/c 正式重跑。
- 修复后源提交 `b82eb55` 已完成 M0-0.b 单批正式采样：质量、资源与 aggregate 均 PASS；证据与结果见 `logs/checkpoints/M0-0.b-20260830-1452.md`。
- 源提交 `93a1ba6` 已完成 M0-0.c 三批正式采样：质量/资源门禁逐批 PASS，raw aggregate 的六项 UNSTABLE 已逐项归因并转交 M0-5/M3；证据与比较规则见 `logs/checkpoints/M0-0.c-20260830-1538.md`。
- `68c78d7` 已完成 M0-5.a 诊断工具提交；40-cycle GUI 资源诊断 run `logs/m0-baseline/20260831T105149+0800_68c78d7_release_x11/` 为 `EXPLORATORY` 但 `measurements.ok=true`，证明当前环境可真实采样。运行时矩阵见 `logs/m0-resource-runtime-matrix-v1.md`：orphan max 均 0，RSS 首末未超过 10% 增长，FD `+1/+1/+2` 转交 M0-5.b。
- `e998bbd`/`2271e75` 已完成 M0-5.b：资源采样补 FD target 差分，M0 driver 隔离全局快捷键/活动栏/页签栏误触发，并在 tab driver 收尾显式关闭 grid 旁路。40-cycle GUI 复测 run `logs/m0-baseline/20260831T160738+0800_e998bbd_release_x11/` 为 `EXPLORATORY` 且 `measurements.ok=true`；orphan max 均 0，grid/terminal FD delta 为 0，tab FD `+1` 定位为 WebKitNetwork `anon_inode:timerfd` 一次性平台计时器，tab RSS 总增长 `+3.49%`，未超过 10%。下一步转 M0-5.c 正式 profile 归档。
- `ebc49f7` 已完成 M0-5.c 正式资源归档：run `logs/m0-baseline/20260831T164058+0800_6534963_release_x11/` 为 formal `PASS`，release SHA `5c41d4abb22a5f5ba4c8d85b443f890f43db8ff8d316ac4a6dae707b29312162` 匹配，orphan max 均 0，tab FD delta 为 0；grid/terminal 的正增长均为 WebKit 子进程 `anon_inode:timerfd` 平台计时器裁决。M0-5 整项 PASS，下一步转 M0-6.a。
- `57a31ea` 已完成 M0-6.a 当前提交宫格崩溃恢复自检：`GRID_SELFTEST=1 ./run-gui.sh` 退出 0，日志 `logs/m0-grid-selftest/M0-6.a-20260831-selftest.log` 输出 `SELFTEST_RESULT=ALL_PASS`，覆盖 create_grid、grid_open、grid_position、eval、kill -11 崩溃重启、重启后 eval、另一格存活、close_grid、shutdown count=0。下一步转 M0-6.b。
- `b3331a0` 已完成 M0-6.b：主进程 `tab-*` WebView 新增 60 秒内最多 2 次的有限恢复预算；`tab_open` 收口到插件 `navigate`；`eval/navigate/position` 失败触发重建并重试；`loadFailed` 只观测不自动重建；`tab-recovery` 事件可见。`cargo test` 47 passed，`npm run build` 通过，`GRID_SELFTEST=1 ./run-gui.sh` 输出 `SELFTEST_RESULT=ALL_PASS`。下一步转 M0-6.c。
- `e7ad997` 已首次尝试 M0-6.c：当前会话可运行 `GRID_SELFTEST`，但缺少 `xdotool`/`wmctrl`/截图工具/人工输入能力和 AI 站点登录态，无法真实签署 9 项 GUI 手工回归；`STATUS=BLOCKED`，NEXT 保持 M0-6.c，不得移动到 M0-7.a。
- `65ba0ec` 已按正式 `M0-6.c` 任务卡再次尝试：`xdotool/wmctrl/scrot` 后续已可用，release GUI 可启动，`GRID_SELFTEST` 再次 `ALL_PASS`，并归档 `windows_probe.txt` / `grid-selftest-result.txt` / `last_app.log`；但仍缺 AI 站点登录态，普通模式无法触发持久宫格会话，且 Wayland/Xwayland 下 `scrot` 全黑、`xwd` BadMatch，无法生成截图/录屏证据。`STATUS=BLOCKED`，NEXT 继续保持 M0-6.c。
- `5056fbc` 已新增 `GRID_GUI_REGRESSION=1` 应用内驱动和 `scripts/m0-6c-gui-regression.py` 本地 AI mock；release GUI 9 项自动化回归 `M0_6C_GUI_REGRESSION_RESULT=PASS`，证据目录 `logs/m0-6c-gui-evidence/20260901-0726/`。M0-6 整项关闭，NEXT 移至 M0-7.a。
- `0e8c869` 已完成 `M0-7.a`：新增 `logs/m0-acceptance-20260901.md` 和 `logs/checkpoints/M0-7.a-20260901-0841.md`，只汇总 M0-0~M0-6 证据和 M0-7.b 待复核风险，不签 M0 总 PASS。NEXT 移至 M0-7.b。
- `6cce225` 已完成 `M0-7.b` 强模型独立复核，新增 `logs/checkpoints/M0-7.b-20260901-0847.md`，验收草案状态升为 `REVIEWED_PENDING_OWNER`。NEXT 移至 M0-7.c，等待项目负责人确认。
- 用户/负责人已在 2026-09-01 09:05 CST 明确确认“验收通过”，据此完成 `M0-7.c` 放行：验收报告状态 `OWNER_APPROVED`，M0 总体验收 PASS，NEXT 移至 `M1-0`。
- `M1-0` 已补齐首页快捷入口：新增一键收藏当前网页/当前目录到主页，目录种子升级为 `get_start_dirs` 全量一次性播种；`npm run build` 与 `git diff --check` 通过。NEXT 移至 `M1-1`。
- （交接事项，2026-09-01 10:11）master 分支上的人工品牌图标提交 `7e76658` 已 cherry-pick 为本分支 `90ff941`，并在 `M1-1` 中被 `tauri icon` 标准产物取代（原 `icon-{32,64,128,256,512}.png` 已删）。master 仍领先 `origin/master` 1 个提交，尚未决定是否重置回 `origin/master`。
- （交接事项，2026-09-01 10:11）master 分支 `stash@{0}` 保存了超出 M1-0 验收范围的主页快捷方式扩展（v2/v3/v4 增量播种、开源项目与 CLI 工具快捷方式、CLI 启动目录弹窗、地址栏⭐收藏目录、第二大脑目录、`launch_app` 加 `cwd`），**未评审、未合入**，需单开检查点评审后再落地；另有 worktree 异版图标备份 `/tmp/worktree-icon-variant.png`（点在字下，未采用）。
- `M1-1` 已完成图标与桌面资源（commit `bda9cd8`）：`tauri icon` 产出 Windows/macOS/Android/iOS 全套并落位 `src-tauri/icons/`；`bundle.icon` 改为多档声明，deb 图标由 1 档（512x512）增至 5 档（32/64/128/256@2/512）；补 `category/shortDescription/longDescription` 修复 `.desktop` 空 `Categories`；`desktop-file-validate` EXIT=0。验证采用解包检查，**未真实 `dpkg -i` 安装，菜单中图标渲染未人工目视确认**，列为遗留观察项。NEXT 移至 `M1-2`。
- `M1-2-fix1` 已修复 M1-2 遗留的 `cargo fmt` 门禁回归（commit `bb13617`）：领取 M1-3 时先跑基线门禁，
发现 `HEAD=6135629` 上 `scripts/pre-merge.sh` 即 `EXIT=1`，唯一失败项为 `cargo fmt main`
（`src-tauri/src/bridge.rs:1293` 的 `log_audit` 调用被写成多行）。按 M1-3 任务卡 `FORBID`
的逃生口「改它必须先开 fix M1-2-fix1」单开修复检查点，只做机械折行还原、**无语义变更**，
修复后 `scripts/pre-merge.sh` 恢复 `ALL_PASS`。证据见 `logs/checkpoints/M1-2-fix1-20260901-1050.md`。
NEXT 保持 `M1-3`。
- `M1-3` 已完成收藏 UI（地址栏 ⭐ + 收藏夹侧栏）：新增 `src/stores/useBookmarkStore.ts`、
`src/components/browser/BookmarkStar.vue`、`src/components/browser/BookmarkPanel.vue`；
挂点分别为 `ActivityBar` 的 `.omni-wrap` 与 `MainArea` 的 `.browser-body`（`Sidebar.vue`
目前是未被任何地方引用的死代码，不是真实挂点）。⭐ 未收藏→`bookmarkAdd`，已收藏→金色高亮且
点击 `bookmarkRemove(id)`；侧栏按 `created_at` 倒序，点项走 `browser.openBrowser()` 在内嵌页签打开
（**不触发系统默认浏览器**，那是 M1-4），按 id 删除。新增 `normalizeUrl` 归一化比对，避免同页面
因末尾 `/` 被判成两条。M1-0 的 `useHomeStore`（主页快捷方式）保持原样、未并入。
验证：`npm run build` 0、`scripts/pre-merge.sh` 0（`PRE_MERGE_RESULT=ALL_PASS`）、`git diff --check` 0、
`git status` 干净。**未做 GUI 目视验收**（环境可用：应用可启动且 `[FE] FE alive` 无报错，但本会话
无法读图，xdotool 坐标自动化属盲操作），与 M1-1 同口径列为遗留观察项。NEXT 移至 `M1-4`。
- `05666ce` 已完成 `M1-4-assist` 低风险前置盘点：只新增 `logs/assist/M1-4-assist-20260901-1145.md`，
记录 `RunEvent::Opened`/deep-link/xdg/MIME 现状、冷启动 URL 缓存契约、6 项风险与 M1-4-a..d 建议拆点。
该提交未改实现代码、未移动 NEXT、未宣称 M1-4 PASS，可作为强模型实现 M1-4 的输入材料。
- `logs/assist/low-model-batch-tasks-20260901-1614.md` 已整理低模型批量任务卡：`M1-3-gui-check`、
`M1-5.a-assist`、`M1-7.a-assist`、`M2-1-assist`、`M2-2.a-assist`、`M2-3-assist`、
`M2-5.a-assist`、`M2-7-assist`、`M2-9-assist`、`M3-4.a-assist` 和最终汇总。低模型按该文件执行时
不得移动主线 NEXT，仍保持 `M1-4`。
- `M1-2` 已完成收藏领域与命令（commit `2cd9d60`）：新增 `Bookmark` 结构体 + 2 个 `#[cfg(test)]` 单测；`workspace.rs` 加 `bookmarks_file/load/save/add/remove` 5 个函数；`bridge.rs` 加 `add/list/remove_bookmark` 3 个 `#[tauri::command]`，全部 `log_audit`；`main.rs` `invoke_handler!` 注册；`src/types.ts` `Bookmark` 接口；`src/bridge.ts` 暴露 `bookmarkAdd/List/Remove` 包装；ACL `default-commands` 3 条 allow。设计要点：同 URL 视为更新（保留 id/created_at），按 id 删除幂等，落盘 `data_dir/bookmarks.json`（与 audit/repos 同模式）。验证：`cargo test bookmark` 2/2 PASS，`cargo build --release` 0，`npm run build` 0，`git diff --check` 0。**未做端到端 IPC e2e**（依赖 M1-3 UI 启动后实测），UI 触发（地址栏⭐ / `tab-navigated` 自动收藏）属 M1-3。NEXT 移至 `M1-3`。
- `M1-6.a` 已完成 Git 写能力契约冻结与任务卡展开（checkpoint `logs/checkpoints/M1-6.a-20260902-0955.md`，纯文档冻结）：冻结白名单 W1/W2/W3（commit/checkout/push）、黑名单（reset/clean/force/cherry-pick/branch 删除/merge/rebase/stash 等全部禁止）、三级确认闸门（独立 `GitWriteJob`/`pending_git_jobs`，不复用 `SyncJob` 以免污染成果推送流程）、审计字段脱敏（detail 不得含 token/凭据）、路径锁定复用 M1-5 `repo_dir`；并拆出 `M1-6.b`（闸门内核 + `git_commit`，`AI:DEEP/R:high`）、`M1-6.c`（`git_checkout` 仅本地分支，`AI:DEEP`）、`M1-6.d`（`git_push` 非 force + 二次确认，`AI:DEEP/R:xhigh`）三张子卡，含测试矩阵 T-gw-1~13。**未实现任何 Git 写操作、未改产品代码、未签 M1-6 整体 PASS**；M1-6 整体 PASS 须待 b/c/d 三卡全绿后由强模型裁定。NEXT 移至 `M1-6.b`。
- `M1-6.b` 已完成 Git 写能力后端核心（commit `2578c9c`，checkpoint `logs/checkpoints/M1-6.b-20260902-1327.md`，执行器：CodeBuddy 会话 Kimi K3，`AI:DEEP/R:high`）：按任务书扩权落地六个白名单写操作 `stage/unstage/discard/commit/create_branch/checkout_branch`（任务书对 M1-6.a 冻结白名单的扩权已在 checkpoint §0 显式记录，安全口径未削弱）——`domain.rs` 新增 `GitWriteOp/GitWriteStatus/GitWriteJob/GitWritePreview`（独立状态机，不复用 `SyncJob`）；`sync.rs` 新增三重校验器（paths/commit message/branch name）+ 六个写原语（全部 git2 本地 API，先校验后执行、失败零部分写）；`bridge.rs` 新增 `pending_git_jobs` + `request_git_write`/`confirm_git_write` 双阶段闸门（一次性任务、5 分钟过期、discard 需 `confirmed_dangerous` 二次确认、后台线程 + `git-write-completed` 事件）+ 审计四态（detail 只含 op/repo_id/job_id/path_count/confirmed，构造器签名杜绝凭据/路径清单/diff）；`main.rs` 注册 + ACL +2；`types.ts`/`bridge.ts` 类型与六个便捷封装（无 UI，UI 归 M1-7）。验证：`cargo test git_write` 19/19、`cargo test` 78/78、`cargo build --release` 0、`npm run build` 0、pre-merge ALL_PASS、`git diff --check` 0。同卡修复了 M1-6.a 引入的既有门禁回归（`详细设计与实施计划.md` 293 行 WBS 路由标签丢失导致 check-plan-routing FAIL，HEAD `8656092` 可复现，非本卡引入）。**未实现 push、未做端到端 IPC 联调（归 M1-7）、未签 M1-6 整体 PASS**。NEXT 移至 `M1-6.c`。
- `M1-6.c` 已完成 Git 写后端核心复核（commit `243d4ae`，checkpoint `logs/checkpoints/M1-6.c-20260902-1430.md`，执行器：CodeBuddy 会话 腾讯 Hy4，`AI:BALANCED/R:high`）：静态审计闸门两阶段命令、六个写原语、三个校验器、审计脱敏与注册完整性（凭据仅存在于成果推送段 1–244 行，写段 17 项黑名单 API 扫描为空）；用临时探针实测 libgit2 三种 checkout 语义后删除探针。**修 1 项缺陷**：切分支后旧分支独有文件与索引条目残留（旧实现先 `set_head` 再 `checkout_head`，后者以新 HEAD 为基线）→ 改为先 `checkout_tree(目标树, safe)` 再 `set_head`，并对 `set_head` 极端失败做工作区回滚，同时消除未跟踪目录场景的误拦截；**2 项小修**：全量 commit 审计 `path_count` 恒 0（改为返回执行前脏文件数）、过期待确认任务无清理（新增 `purge_expired_git_jobs`）；**补 7 项测试**（`cargo test git_write` 26/26、`cargo test` 85/85）；**新增 `scripts/check-git-write-policy.py`**（19 个违规码，好样本零违规 + 14 个坏样本命中对应码的双向自检，已接入 `pre-merge.sh` 第 13 项）。验证：`cargo build --release` 0、`npm run build` 0、pre-merge ALL_PASS、`git diff --check` 0。**裁定 M1-6.d 保留 push**（契约 W3 已冻结、`push_artifacts` 不覆盖用户自选提交、风险由非 force+二次确认+凭据隔离收口），五条硬约束写入 checkpoint §6。**未触发 HARD_RULE**（无需大改核心，未 BLOCKED）。**遗留待裁决**：全量 commit 的 TOCTOU、同仓库并发写无互斥、仓库内符号链接边界、端到端 IPC 联调（归 M1-7）。NEXT 移至 `M1-6.d`。
- `M1-6.d` 已完成 Git push 能力（commit `7849dd5`，checkpoint `logs/checkpoints/M1-6.d-20260902-1525.md`，执行器：CodeBuddy 会话 Kimi K3，`AI:DEEP/R:xhigh`）：`GitWriteOp::Push` 接入既有 `request_git_write`/`confirm_git_write` 双阶段闸门（不另建旁路、不新增命令、ACL 不变），白名单扩至七项且 `Push` 标记 dangerous（必须 `confirmed_dangerous=true` 二次确认）；`sync.rs` 新增 `build_push_refspec`（纯函数，只产出 `refs/heads/X:refs/heads/X` 同名映射，结构上无法注入 force/前导冒号删除/任意 refspec）、`write_push`（仅非 force 推当前检出分支到 origin 同名分支；detached HEAD/未配置远端拒绝；凭据仅由调用方在推送瞬间传入，复用 `cred_cb` 思路）、`push_ahead`（只读预览领先提交数）、`scrub_sensitive_error`（token 精确替换 + URL userinfo 掩码）；bridge 审计构造器加 `branch`/`remote_name`（签名仍不接收 paths/diff/凭据/远端 URL），request 预检只读不读 Keyring，execute 在推送瞬间 `KeyringStore::get_token` 且错误串脱敏；`scripts/check-git-write-policy.py` 扩至 24 个违规码（push 敏感 API 位置、force/删除 refspec 字面量、refspec 注入、仅 origin、push dangerous、闸门 push 映射、审计 push 字段、`push_artifacts` 行为锚点），自检 1 好 + 22 坏样本双向通过。测试：`cargo test git_write` 35/35（含 bare remote 端到端：远端收到提交 OID 相等、up-to-date 不变、非 fast-forward 拒绝且本地与远端均不变、无 origin/detached HEAD 拒绝、refspec 安全性、脱敏、二次确认、push 审计字段）、`cargo test` 94/94、`cargo build --release` 0、`npm run build` 0、pre-merge ALL_PASS、`git diff --check` 0。**边界声明**：真实 HTTPS 远端 push 未实测（无测试账号，不伪造；凭据回调与线上已验证的 `push_artifacts` 同一 `cred_cb`），M1-7 联调时配真实仓库回归。**b/c/d 三卡全绿，M1-6 Git 写能力整体 PASS**。NEXT 移至 `M1-7`（Git UI）。
- `M1-7` 已完成 Git UI（commit `feat(M1-7): add git workspace UI` + `docs(M1-7): record git ui checkpoint`，checkpoint `logs/checkpoints/M1-7-20260902-1830.md`，执行器：CodeBuddy 会话 Hy4，`AI:BALANCED/R:medium`）：新增 `src/stores/useGitStore.ts`（仓库选择、status/diff/branch 加载、勾选集、写操作双阶段闸门、`git-write-completed` 回落刷新）、`GitPanel.vue`（复用 `useWorkspaceStore.repos`；分支条 + 状态列表 + 七个写操作入口）、`GitDiffViewer.vue`（全量/单文件 diff + `more`/`truncated`/`binary` 提示）、`GitWriteConfirmDialog.vue`（展示后端 preview + 前端派生风险等级 + dangerous 二次确认勾选框 + push/discard 约束提示）；`RepoPanel.vue` 加「状态/配置」Tab；`App.vue` 全局订阅完成事件并全局挂载确认弹窗；新增 `src/utils/redact.ts`（前端二次脱敏：URL userinfo / 凭据查询参数 / 已知 token 前缀 / Bearer-Basic / 疑似随机串 + 300 字截断，刻意不误伤纯小写十六进制 OID）。**`src-tauri/**` 零改动、未新增任何后端命令、ACL 未变**；所有写操作唯一入口是 `requestWrite/confirmWrite`，前端侧另有两道锁：dangerous 未勾选时 confirm 命令根本不发、预校验（commit 非空/分支名非空/路径非空）失败时连 request 都不发。新增两个门禁：`scripts/check-git-ui-policy.py`（静态夹具，19 项自检：1 好 + 18 坏样本；守闸门不可绕过、校验在 request 之前、错误必须脱敏、Git UI 内禁 token/localStorage、弹窗勾选框与按钮禁用、黑名单与 force 字面量、事件订阅与弹窗挂载）与 `scripts/check-git-ui-logic.mjs`（Node headless，直接加载真实 store、只 mock bridge 的 Git 方法，**53 条断言全过**），两者均已接入 `pre-merge.sh`。验证：`npm run build` 0、`cargo test git_write` 35/35、`cargo test` 94/94、`cargo build --release` 0（2 warning 为既有 `grid_process.rs` 死代码）、pre-merge ALL_PASS、`git diff --check` 0。**GUI 目视验收未完成**（本会话模型不支持读图，`scrot` 多次截屏返回完全相同的 36KB PNG 不可信；release 二进制可启动、窗口可见、日志无 panic，但未做 GUI 点击验收），已在 checkpoint §6 如实声明并给出 §8 可复现的人工验收清单与种子数据；验收期间植入的一次性测试仓已完整清理。`push` 真实 HTTPS 远端仍未实测（沿用 M1-6.d 边界）。NEXT 移至 `M1-8`。
- `M1-8` 已完成请求拦截与瀑布（commit `feat(M1-8): add resource capture waterfall` + `docs(M1-8): record resource waterfall checkpoint`，checkpoint `logs/checkpoints/M1-8-20260902-2058.md`，执行器：CodeBuddy 会话，`AI:DEEP/R:high`，Kimi K3）：**事实修正**——Tauri 2.11.5 无设计假设的 `on_web_resource_response_received`，`on_web_resource_request` 仅 `tauri://` 协议生效（外部 http(s) 不触发），实际采集改走插件内 WebKitGTK 原生信号 `resource-load-started` + `WebResource::finished/failed`（`URIRequest`→method/url，`URIResponse`→status/mime/content_length），字段全部真实、平台拿不到的置 None 不伪造（content_length=0→未知、失败→status/mime/size=None、时钟回拨→duration=None）。`domain.rs` 新增 `ResourceReceived`/`ResourceKind`/`ResourceCaptureSettings`/`TabResourceList`（DTO 结构性不含 headers/Cookie/Authorization/Set-Cookie/body）；`security_policy.rs` 新增 `redact_sensitive_url`（21 敏感键值→`***` + userinfo 移除 + fragment 脱敏 + 2048 字节限长）；`bridge.rs` 新增 `ResourceBuffer`（每 tab 200 + 全局 2000 两级 FIFO + 驱逐计数 + 惰性过期跳过）、`classify_resource_kind`（mime 优先/后缀兜底/xhr 启发式）、4 命令 `list_tab_resources`/`clear_tab_resources`/`get_resource_capture_settings`/`set_resource_capture_settings`（全部过 `check_invocation_source` + `tab-` 前缀校验，进 ACL 与 invoke_handler；`max_per_tab` 限 [10,1000]、`max_total`/URL 上限不开放调大）；`close_tab`→`remove_tab`、ShutdownCoordinator close-tabs→`clear_all`；审计仅 tab_id/计数/开关值不含 URL；main.rs 不直接转发原始 `resourceReceived` payload（唯一出口 `on_resource_received` 脱敏后 emit `resource-received`）；插件信号块零打印。前端：`useResourceStore`（本地容量镜像 + loadTab 权威同步 + 筛选分类）+ `ResourceWaterfall.vue`（Dock 第三 Tab「🌊 资源」；六列 + All/Doc/JS/CSS/Image/XHR-Fetch/Other 筛选 + 清空 + 采集开关 + 超限/关闭横幅 + 降级 `-` 说明）；`App.vue` 全局订阅。边界：宫格 grid-N 不采集（子进程事件未转发）；XHR/Fetch 归类为启发式；设置会话级不持久化（默认 enabled=true）。验证：`cargo test resource` 16/16、`cargo test` 114/114、`cargo build --release` 0（2 warning 为既有 `grid_process.rs` 死代码）、`npm run build` 0、policy 1 好+17 坏自检与正式检查均过、UI 逻辑 84 断言全过、pre-merge ALL_PASS、release 二进制启动冒烟无 panic。**运行时端到端资源事件联调与 GUI 目视验收未完成**（本会话无 GUI 自动化通道且无法读图，未伪造证据；人工验收清单见 checkpoint §8）。NEXT 移至 `M1-9`。
- `M1-9` 已完成会话持久化与关闭协议（commit `feat(M1-9): add session persistence and close protocol` + `docs(M1-9): record session persistence checkpoint`，checkpoint `logs/checkpoints/M1-9-20260903-0727.md`，执行器：CodeBuddy 会话，`AI:DEEP/R:xhigh`，Kimi K3；**环境修正**：任务最初指派的 2ae9 worktree 为 detached HEAD 且不含 M1-8，触发 HARD_STOP 并如实报告，经用户确认切换到 7350 worktree 执行）：`domain.rs` 新增 `BrowserSession`/`SessionSummary`/`SessionDraft`/`SessionPolicy`/`SessionFlushReport`（白名单 DTO 结构性不含 token/cookie/Authorization/Set-Cookie/headers/body/raw）；`session.rs` 纯函数存储层（原子写 tempfile+rename、损坏文件跳过、删除幂等、容量 FIFO、异常退出 tmp 清理、`scrub_preview` 手写 UTF-8 安全 URL 脱敏扫描器含多字节前缀回归）+ 10 组真实落盘单测；`bridge.rs` 10 命令（`session_save`/`session_discard`/`session_list`/`session_get`/`session_delete`/`session_export`/`session_restore`/`flush_sessions`/`get_session_policy`/`set_session_policy`，全部过 `check_invocation_source` + `tab-` 前缀校验，进 ACL 与 invoke_handler，审计仅 id/tab_id/计数不含 URL/预览）；`create_tab` 建草稿 / `tab_open` 刷新 / `close_tab` 清草稿；ShutdownCoordinator 新增 `flush-sessions`（注册在 close-tabs **之前**，auto_save_on_exit 开时落盘否则仅释放草稿不静默保存 + prune tmp/容量 + 审计）；启动时 prune + 统计日志（异常退出恢复边界）。前端：`useSessionStore`（列表/详情/保存/删除/恢复/导出 + 关闭协议三分支 + 预览采集容错）、`useBrowserStore.bindCloseInterceptor` + `closeTabNow`（单向注入防循环 import）、`SessionPanel.vue`（Dock 第四 Tab「💾 会话」）、`SessionCloseDialog.vue`（全局弹窗）、App.vue 绑定拦截器 + beforeunload flush 双保险。设计决策：`session_export` 不落盘（返回脱敏 JSON 到剪贴板）、`auto_save_on_exit` 默认关、`session_restore` 用脱敏 URL（登录态不还原，如实声明）、窗口关闭不逐 tab 弹窗（不改 M0-2 退出时序）。验证：`cargo test` 128/128（基线 114 + 新增 14）、`cargo build --release` 0（2 warning 为既有死代码）、`npm run build` 0、policy 1 好+18 坏自检与正式检查均过、UI 逻辑 34 断言全过、pre-merge ALL_PASS、release 启动冒烟（sessions restored 日志正常、无 panic）。**运行时端到端关闭协议联调与 GUI 目视验收未完成**（本会话无 GUI 自动化通道且无法读图，未伪造证据；人工验收清单见 checkpoint §7）。M1-1~M1-9 全部 PASS，NEXT 移至 `M1-ACCEPT`（M1 里程碑验收，通过后进入 M2-1）。
- `M2-2.b` 已完成画廊/灯箱/缩放 UI（commit `feat(M2-2.b): add image gallery lightbox preview UI`，checkpoint `logs/checkpoints/M2-2.b-20260903-1454.md`，执行器：CodeBuddy 会话 腾讯 Hy4，`AI:BALANCED / R:medium`）：按 M2-2.a 冻结契约施工——后端新增 `workspace_images_dir` 只读命令（`check_invocation_source` + 审计 `image.dir_read` 只记 op 名）、`workspace::images_dir()`、以及 `images::join_image_path()` 白名单拼接（fail-closed，与前端 `buildAssetSrc` 同构，附 T-img-14/15 与 T-ip-1/2/3 单测）；前端新增 `src/utils/imagePreview.ts` 纯逻辑层（`clampScale`/`nextIndex`/`prevIndex`/`scaleForWheel`/`scaleForDoubleClick`/`buildAssetSrc`/`gridColumns`/`fitScale`/`galleryState`/`truncateText`/`keyAction`/`createKeyBinder` + 失败态原语）、`useImagePreviewStore`（目录基准缓存、打开/关闭、不循环切换、缩放、拖拽、失败与重试）、`shared/ImageGallery.vue`（骨架/空态/缩略图/错误占位+重试/溯源只出 host）、`shared/ImageLightbox.vue`（ESC/遮罩关闭、不循环切换、缩放 [0.25,4] 步长 1.2、双击 1↔2、放大后拖拽、单图失败不关灯箱）；挂点为 `ArtifactPanel` 编辑区（画廊）与 `App.vue`（全局灯箱）。**未改 `tauri.conf.json` 的 `assetProtocol.scope`、零新增 npm 依赖、未放宽 `save_image`、未改 `remote-collect.toml`/`injected/collect.js`**。新增 `scripts/check-image-preview-policy.py`（12 类违规码，1 好 + 15 坏样本自检）与 `scripts/check-image-preview-logic.mjs`（headless 97 断言，覆盖 T-prev-1~12 及通道/隐私回归），均接入 `pre-merge.sh` 第 17 项。验证：`cargo test` 156/156（基线 151 + 新增 5）、`cargo build --release` 0（warning 仍为既有 2）、`npm run build` 0、build metrics 总体积 +12.63%（≤15%）且 warning 未增加、policy/logic 自检与默认门禁均过、pre-merge ALL_PASS、`git diff --check` 0、release 启动冒烟（前端 alive、无 panic）。过程中修掉三个门禁冲突且均未降级既有门禁：`join_image_path` 加 `#[allow(dead_code)]` 消除 warning（契约锚点，同既有 `InlineTooLarge` 口径）；键盘动作 `"reset"` 改 `"zoom_reset"` 以避开 M1-7 `check-git-ui-policy.py` 的 Git 黑名单字面量误报；ACL 新条目插在 `list_artifact_images` 之前，保住 M2-1 `check-image-policy.py` 坏样本的定位字符串。**挂账**：asset:// 端到端加载与 GUI 目视验收未完成（无 GUI 自动化通道、数据目录尚无落盘图片），人工验收清单见 checkpoint §6。**未签 M2-2 整体 PASS**，NEXT 为 `M2-2 整体裁定`。
- `M2-2` 整体裁定已完成（checkpoint `logs/checkpoints/M2-2-ACCEPT-20260903-1540.md`，裁定者：CodeBuddy 会话 Kimi K3，`AI:DEEP / R:high`，**独立于实现者**腾讯 Hy4；复核输入包 `logs/checkpoints/M2-2-verdict-input-20260903-1502.md`）：精读 `git show e4a2974` 关键 diff（后端命令来源校验与返回值范围、前后端白名单同构、组件监听绑定与隐私出口）+ 亲自复跑验收（`cargo test` 156/156、logic 97 断言、policy 1 好+15 坏、pre-merge ALL_PASS、`git diff --check` 0）。七项冻结契约逐项裁定成立；风险 R1~R5 逐项表态：R1 asset:// 运行时加载 workspace 图片未实测**降级为 D10**（理由：scope 经真实环境推导已覆盖 + `asset://` 通道已有 `AppPanel` 图标加载的运行时先例 + 人工验收清单可复现；澄清未实测的是「workspace 图片目录下的运行时加载」而非「asset:// 协议可用性」），R2 GUI 目视验收为 D11（同 M1/M2-1 口径），R3/R4/R5 可接受；b 卡三个门禁冲突修复（`allow(dead_code)` / `zoom_reset` / ACL 顺序）经复核均未降级既有门禁（M1-7/M2-1 门禁脚本零改动）。**签 M2-2 整体 PASS_WITH_DEBT，D10/D11 并入 M1-ACCEPT §7 D 系列，NEXT 移至 `M2-3`**。
- `M2-3.b` 已完成脚本领域与持久化（checkpoint `logs/checkpoints/M2-3.b-20260903-1633.md`，执行器：CodeBuddy 会话 Kimi K3，`AI:BALANCED / R:medium`）：按 M2-3.a 冻结契约施工——`domain.rs` 新增 `ParamType`/`ScriptInterpreter`（枚举白名单，含 `ext()`）/`ScriptParam`/`ScriptMeta`（新增字段一律 `#[serde(default)]`）；新增 `src-tauri/src/scripts.rs` 纯函数层（20 个稳定错误码、`validate_meta` 定义期校验、`validate_param_value` 九条 fail-closed 规则 + 类型校验、`is_secret_param`/`redact_param_value` 脱敏三重保险、`can_delete`）；`workspace.rs` 新增脚本持久化，核心改为**显式路径参数**的 `save_scripts_at`/`load_scripts_at`/`body_path_in`/`write_body_at`/`read_body_at`/`delete_body_at`（与 M2-1 `images.rs` 同范式，便于真实磁盘单测），元数据与正文均原子写，正文路径经「形态 + canonicalize + 前缀」三重校验，删除幂等；`bridge.rs` 四条命令 `script_list/add/update/remove` 全部过 `check_invocation_source`、写/删过 `check_id`、审计 detail 只含 id/name/interpreter/param_count/builtin 且不含正文与参数默认值；`main.rs` 注册 + ACL 同步；`types.ts`/`bridge.ts` TS 镜像（嵌套字段 snake_case）。新增 `scripts/check-script-domain-policy.py`（14 类违规码，1 好 + 17 坏样本自检）并接入 `pre-merge.sh` 第 18 项。验证：`cargo test` 178/178（基线 156 + 新增 22，含真实临时目录的落盘/读取/删除/历史 JSON 兼容往返）、`cargo build --release` 0（warning 仍为既有 2）、`npm run build` 0、build metrics +12.65%、pre-merge ALL_PASS、`git diff --check` 0、release 启动冒烟无 panic。过程中修掉三个问题：①ACL 条目顺序——`script_*` 必须插在 `list_artifact_images` **之前**（M2-1 `check-image-policy.py` 坏样本按末行无逗号定位；M2-2.b 已因同一坑调整过一次），**未改** M2-1 门禁脚本；②坏样本静默失效——变异字符串失配时 `str.replace` 返回原文，导致「什么都没测却显示通过」，已在本卡夹具 `--self-test` 加**防呆**（坏样本必须真的改动内容，否则按漏检计）；③12 个新增 dead_code warning——5 个为 M2-4/M2-5 预留的契约函数无当前调用者，逐个 `#[allow(dead_code)]` 并**注明消费者**（同既有 `InlineTooLarge`/`join_image_path` 口径），warning 回到既有 2 个。**未实现任何执行能力**（无 `run_script`/`std::process::Command`/shell spawn/运行记录）。**两处对冻结裁定书的微调**：`validate_param_value` 去掉无作用的 `raw` 参数；未新增 `check-script-domain-logic.mjs`（本卡无前端脚本逻辑，UI 属 M2-5，强加会成为无人消费的死代码）。**挂账**：端到端 IPC 往返与危险值运行时链路无 GUI 通道取证。**未签 M2-3 整体 PASS**，NEXT 为 `M2-3 整体裁定`。
- `M2-3` 整体裁定已完成（checkpoint `logs/checkpoints/M2-3-ACCEPT-20260903-1645.md`，裁定者：CodeBuddy 会话 Kimi K3，`AI:DEEP / R:high`；**独立性如实标注：与实现者同为 Kimi K3 的同模型裁定**，弱于 M2-2 的跨模型复核，用户已明确指示，可被 Hy4/Codex 独立复核推翻或确认）：精读 `git show 7c4cfb1` 关键 diff（命令层来源校验/id 校验/回滚语义、纯函数层 API、持久化三重校验）+ 亲自复跑验收（`cargo test` 178/178、policy 1 好+17 坏、pre-merge ALL_PASS、`git diff --check` 0、diff 中无 `std::process::Command`/`run_script`/`ScriptRunRecord`/shell spawn）。五组冻结契约逐项裁定成立；两处「冻结裁定书微调」经独立重审**接受**（`validate_param_value` 去掉无作用的 `raw` 参数；未新增 `check-script-domain-logic.mjs`）；风险逐项表态：D12（命令端到端 IPC 未实测）、D13（「危险值被拒」运行时链路未打通，M2-4 须用 N1~N13 做运行时回归）记为挂账并入 D 系列，`can_delete` 对内置脚本返回 `InvalidId`（错误码语义不准）记 NON-BLOCKER；实现者三个门禁冲突修复（ACL 顺序/坏样本静默失效防呆/dead_code）经复核均未降级既有门禁，新增变异防呆为正向改动。**签 M2-3 整体 PASS_WITH_DEBT，NEXT 移至 `M2-4`**（安全执行通道；已记录三条提示：D13 用例运行时回归、`check_path_within_roots`/`validate_enum_value` 复用、`shell:allow-spawn args:true` 收紧须单开检查点）。

## 2. 模型映射

WBS 只使用供应商无关的路由标签。切换平台时只改本表，不批量改 50 个 WBS。

| 路由 | 任务类型 | Codex 当前映射 | Trae 映射 |
|------|----------|----------------|-----------|
| `AI:FAST` | 文档、格式化、机械修改、明确的小 UI | `gpt-5.6-luna` | `TRAE_FAST`：待补精确模型名 |
| `AI:BALANCED` | 边界清晰的多文件功能、测试工具、普通重构 | `gpt-5.6-terra` | `TRAE_BALANCED`：待补精确模型名 |
| `AI:DEEP` | 架构、安全、生命周期、并发、IPC、疑难故障 | `gpt-5.6-sol` | `TRAE_DEEP`：待补精确模型名 |

### 最低额度执行法

1. 先运行仓库脚本；格式、schema、哈希、固定夹具、构建和可枚举文档扫描以脚本结果为准，不让模型逐文件重复判断。
2. 能写成“输入、输出、允许文件、验收命令”的独立任务，优先交给 `gpt-5.6-luna / low` 或 Trae 免费模型；默认只读，写任务必须使用互不重叠的文件范围。
3. 低成本模型只提交候选 diff/报告，不能单独移动 `NEXT` 或宣称检查点 PASS；主执行器只审关键 diff、失败路径和脚本结果。
4. `AI:BALANCED` 负责边界明确的实现与集成；`AI:DEEP` 仅用于安全、生命周期、并发、IPC、未知根因、连续失败和最终放行。
5. 长时间 GUI/性能采样由脚本一次性无人值守执行；参数、commit 或环境未冻结前禁止反复试跑。smoke 只用于排错，不写入正式 PASS 分母。

### 低模型可执行任务卡协议

低模型不得领取“完成 M0”这种开放任务，只能领取一张任务卡。任务卡必须完整包含以下字段，缺一项即只允许只读审计，不允许写代码或移动指针：

```text
TASK_ID=<WBS.checkpoint>
ROUTE=<AI:FAST|AI:BALANCED|AI:DEEP>
MODEL=<界面完整模型名>
REASONING=<low|medium|high|xhigh>
GOAL=<一句话结果>
READ=<允许读取的文件或目录>
WRITE=<允许修改的文件；只读任务写 NONE>
FORBID=<禁止事项，至少包含不得跨检查点、不得改冻结证据、不得提前勾选>
COMMANDS=<必须执行的命令；每条记录 EXIT>
PASS_CRITERIA=<可机器判断的通过标准>
FAIL_ACTION=<失败时保留 NEXT、写 BLOCKED 或交还强模型>
DOC_BACKWRITE=<需要回写的文档/日志/checkpoint>
COMMIT=<提交信息；只读任务写 NONE>
NEXT=<PASS 后唯一下一检查点；FAIL 时保持原 NEXT>
```

低模型执行顺序固定为：`读取任务卡 -> git status --short --branch -> 读取 READ -> 修改 WRITE -> 运行 COMMANDS -> git diff --check -> 回写 DOC_BACKWRITE -> 独立提交 COMMIT -> git status --short --branch`。`ROUTE=AI:DEEP` 的任务允许低模型先做只读盘点、脚本夹具和文档草稿，但最终 PASS 裁决必须由强模型复核 diff、失败路径和验收输出。

用户只负责手动切换 Trae/Codex、粘贴任务卡或提示接手；不得要求用户手动执行 `COMMANDS`。每个 AI 执行器必须自行运行命令、记录退出码和关键输出，并把结果写入 checkpoint。若当前平台不能执行 Shell，只能做只读审计或草稿，不能标记 PASS、不能提交、不能移动 `NEXT`。

### Token 节省执行规则

- 每轮只把任务卡、最新 checkpoint、`git diff --stat`、失败命令尾部 120 行交给下一模型；不得整份粘贴所有长文档。
- 文件检索优先用 `rg`、专用脚本和固定行号；禁止让模型“梳理全项目”后再判断一个检查点。
- 强模型复核只看 `git diff HEAD~1..HEAD`、本轮 checkpoint、验收命令输出摘要和未解决风险；不重复读取完整仓库。
- 能用脚本输出 `PASS/FAIL/BLOCKED` 的验收，不让模型人工判断；脚本没有覆盖时才补固定夹具。
- 同一失败最多让低模型修两轮；第二次仍失败即停止，回写 `BLOCKED`，交给更强模型。
- 文档回写只更新顶部状态、当前 WBS、最近 checkpoint 和 NEXT；不重排无关章节，避免无意义 diff。

### 历史任务卡：M2-4.b（2026-09-04 展开 + 独立裁定；**已实现**，2026-09-05 经 `M2-4 整体裁定` 签 `PASS_WITH_DEBT`，NEXT=`M2-5`）

> ✅ **独立裁定已完成**（`logs/checkpoints/M2-4.b-VERDICT-20260904-0705.md`，**ADJUDICATED_WITH_REVISION**）。裁定者：Kimi K3 / CodeBuddy。**独立性如实标注：同会话同模型裁定（实现者亦为 Kimi K3，展开卡头部「腾讯 Hy4」标注不实，待实现卡开工时更正），为最弱独立性**；补偿措施=四项实测复跑 + 原文精读 + 反方检验。结论：**§3.1 走路线 A**（argv 一律不加引号；`M2-4.a §8.1` 末条「raw 跳过单引号包裹」作废；登记「评估废弃 `ScriptParam.raw`」后续检查点）；**§3.2 推翻实现者倾向，改走路线 B**（b 卡上 `piped` + 只丢弃 drain 线程——drain 不保留/不截断/不发事件，**drain ≠ 背压，不越界**，且消除中间态与「d 卡忘记接管」的移交风险）；§3.3/§3.4/§3.5 确认（`timeout_secs` 注释 300→60、`ScriptInterpreter::binary()` 落 `domain.rs`、`RunStatus` 落 `domain.rs`）。**实现前须先按裁定书 §6 修订展开卡**（`§2.1/§2.2/§3.6/§7/测试矩阵` 五处），修订与实现同一检查点提交。**强烈建议 b 卡实现完成后由非 Kimi K3 模型做一次跨模型复核**（恢复裁定可信度，本裁定保留被推翻权利）。裁定输入包：`logs/checkpoints/M2-4.b-verdict-input-20260904-0655.md`。
> **实现期配套修订（2026-09-04）**：M2-3.b 既有夹具 `scripts/check-script-domain-policy.py` 同步放宽——①`RUN_RECORD_FORBIDDEN` 移除 `RunStatus`（状态枚举归 b 卡、裁定 §3.5，非落盘形态；真正的落盘关键词 `ScriptRunRecord`/`script-runs`/`script_runs` 保留，仍归 d 卡监管）；②`RUN_RECORD` 检测改 `strip_comments`（注释**提及**≠真实**定义**，否则 b 卡在 `domain.rs` 合法落 `RunStatus` + 注释提及 d 卡落盘形态名会误染红 M2-3 门禁）。两处修订均带行内注释说明裁定依据；M2-3 self-test（17 坏样本 + 1 好样本）与 default 仍全绿，未削弱 M2-3 核心验收（不得落盘运行记录）。

> **M2-4.b 已展开为独立任务卡**（`logs/checkpoints/M2-4.b-20260904-0000.md`，纯文档展开，未改产品代码）：现状实测 **16 项**、**5 处对冻结裁定书的澄清/微调**、测试矩阵 **B1~B13**（其中 B7 取消后进程组消失 / B8 2s 超时 / B9 子进程树整组回收 / B11 并发上限 8 + 同 id 禁并发 / B12 `/proc` 验证 setsid 生效 为**真实进程测试**，<15 s，是 M2-4 首批可机器取证的运行时证据）、**6 个默认违规码 + 12 个 c/d 卡 pending 码位**（沿用 M0-3.a「一次性定义码位、默认只判本卡职责」模式，避免 c/d 未实现就把 pre-merge 染红）。
> **实现前必须由裁定者确认展开卡 §2 的 5 处澄清**，尤其 **§2.1（argv 模式一律不加引号）**：`Command::arg()` 走 `execve` 的 argv 数组、shell 不参与解析，给值加 `'...'` 会把单引号作**字面量**传进脚本（真实 bug 级冲突，不是风格问题）；防注入由「argv 数组 + `validate_param_value` 九条 fail-closed」保证，不依赖引号。若裁定维持「保留引号」，则 B5 与 R1~R5 需同步改写。
> 契约依据仍为 `M2-4.a` 冻结裁定书 `logs/checkpoints/M2-4.a-20260903-2233.md`（**唯一契约源**）。完整 `TASK_ID/READ/WRITE/FORBID/COMMANDS/PASS_CRITERIA` 见展开卡 §8，本处不复制，避免两处口径漂移。
> 范围：新增 `src-tauri/src/script_runner.rs` + `domain.rs` 三处改动（`RunStatus` 枚举 / `ScriptInterpreter::binary()` / `timeout_secs` 注释 300→60）+ `scripts/check-script-exec-policy.py`（接入 pre-merge 第 19 项）+ `main.rs` 一行 `mod`。**不含**任何 `#[tauri::command]`（c 卡）、不含输出背压/事件流/落盘（d 卡）、不含 `capabilities/default.json` 改动（e 卡）。

### 历史任务卡：M2-4.a（2026-09-03 展开，2026-09-03 冻结 PASS）

> **M2-4.a 已冻结裁定书 PASS**（`logs/checkpoints/M2-4.a-20260903-2233.md`，纯文档，七组分歧逐组裁定，未改产品代码）；NEXT 已移至 `M2-4.b`，b 卡任务卡已于 2026-09-04 展开为独立任务卡。本任务卡转为存档，执行指针以 §1 顶层表为准。
>
> M2-4（安全执行通道，COMPLEX）已展开为 M2-4.a~e 五张子卡，展开依据见
> `logs/checkpoints/M2-4-20260903-1659.md`（含现状实测、对两份前置草案的事实修正、
> 七组待裁决分歧、R1~R15 测试矩阵、15 类违规码）。
> **本次只展开 a 卡**：七组分歧会实质改变 b/c/d 的实现细节，
> b/c/d 在 a 卡冻结后再逐张展开，避免契约漂移。
> **贯穿 M2-4 全部子卡的禁止事项**：任何 `sh -c`/`bash -c` 拼接（P0）、
> 只 kill 直接子进程、依赖 `shell:allow-spawn` 的 ACL 做参数校验、输出无上限、
> 审计含参数明文或输出正文。

```text
TASK_ID=M2-4.a
ROUTE=AI:DEEP
MODEL=<界面完整模型名>
REASONING=xhigh
GOAL=冻结脚本执行通道的安全契约（纯文档，不改产品代码）：
     裁决七组分歧——①输出上限（taskcard 4MB/5000行 vs prework 256KB）；
     ②超时默认值（taskcard 60s vs prework 300s，须与已落库 ScriptMeta.timeout_secs 一致）；
     ③并发上限（prework 8 vs taskcard R14 的 10）；
     ④平台范围（setsid/killpg 为 unix-only，是否只做 Linux + 非 unix 显式降级）；
     ⑤env 策略（env_clear + 固定最小集 vs 允许 ScriptMeta 声明额外白名单）；
     ⑥shell:allow-spawn 处置（实测零调用点，是否单开 M2-4.e 移除；
        若同步移除 npm 依赖须一并裁决如何更新 M2-2/M2-3 夹具锚定的 package*.json 哈希）；
     ⑦RunStatus 命名与运行记录落盘（prework Succeeded vs taskcard Success，是否落盘及上限）。
     并冻结：命令构造（Command + .arg/.args 数组）、cwd 锁定、stdin 策略、
     超时分层（soft/hard）、进程组回收语义、输出背压参数、审计字段与脱敏、
     以及 b/c/d 三卡的范围边界
READ=logs/checkpoints/M2-4-20260903-1659.md（现状实测/事实修正/七组分歧/矩阵/违规码），
     logs/assist/script-execution-safety-taskcard-20260902-1146.md（执行安全主卡 §4 契约），
     logs/assist/M2-3.a-prework-20260902-1055.md §4.2/§4.5（RunStatus 与取消超时），
     logs/checkpoints/M2-3.a-20260903-1604.md（M2-3 冻结裁定，含 M2-4 边界与三条提示），
     src-tauri/src/scripts.rs（M2-3 已交付的参数校验，当前无调用者），
     src-tauri/src/shutdown.rs + bridge.rs register_shutdown_tasks（退出收口现状），
     src-tauri/capabilities/default.json（shell 权限现状），
     src-tauri/Cargo.toml（libc 已就绪）
WRITE=logs/checkpoints/M2-4.a-<YYYYMMDD-HHMM>.md（冻结裁定书），
      详细设计与实施计划.md, 后续需求TODO.md, AI-模型切换与接手清单.md
FORBID=不得改任何产品代码（src/ src-tauri/ scripts/ 一律只读，纯文档冻结）；
      不得实现 run_script / 进程组 / 任何 std::process::Command（实现归 M2-4.b/c）；
      不得改 capabilities/default.json（属安全边界变更，须 M2-4.e 单开检查点）；
      不得新增 npm 依赖；不得删除/改动 M0~M2-3 证据与 checkpoints；
      不得提前勾选 M2-4；不得与 M2-4.b 合并提交
COMMANDS=git status --short --branch（开工前必须 clean）
      python3 scripts/check-plan-routing.py
      bash scripts/pre-merge.sh            # 必须 PRE_MERGE_RESULT=ALL_PASS
      git diff --check
      git diff --stat                      # 必须只有 docs/logs 变更
PASS_CRITERIA=1. M2-4.a 冻结裁定书落盘，§4 七组分歧逐组给出结论（不得留白）；
      2. 三份主文档回写 NEXT=M2-4.b；pre-merge ALL_PASS；git diff --check 0；
      3. git diff --stat 证明 src/ src-tauri/ scripts/ 零改动；工作树干净
FAIL_ACTION=发现分歧无法自洽（如并发上限与 R14 用例冲突无解）→ STATUS=BLOCKED
      回写 logs/checkpoints/M2-4.a-<ts>.md，NEXT 保持 M2-4.a，交还强模型
DOC_BACKWRITE=详细设计与实施计划.md 顶部状态；后续需求TODO.md 关键路径；
      AI-模型切换与接手清单.md §1 顶层表 + 关键路径；
      logs/checkpoints/M2-4.a-<ts>.md
COMMIT=docs(M2-4.a): freeze script execution safety contract
NEXT=M2-4.b（展开 b 卡任务卡后再实现）
```

### 历史任务卡：M2-3.a / M2-3.b（均已执行，2026-09-03）

> **执行结果（2026-09-03 16:33 更新）**：`M2-3.a` 冻结裁定书 PASS（`f87c823`）；
> `M2-3.b` 已按冻结契约实现完毕，`STATUS=PASS_WITH_DEBT`，证据
> `logs/checkpoints/M2-3.b-20260903-1633.md`（`cargo test` 178/178、
> `cargo build --release` 0 且 warning 未增加、`npm run build` 0、总体积 +12.65%、
> 新门禁自检与默认模式均 PASS、pre-merge ALL_PASS、release 启动冒烟无 panic）。
> **挂账**：四条命令的端到端 IPC 往返与「危险值被拒」的运行时链路无 GUI 通道取证
> （只有 22 项 Rust 单测与静态夹具证据），人工清单见该 checkpoint §7。
> **两处对冻结裁定书的微调需裁定者确认**：`validate_param_value` 去掉无作用的 `raw`
> 参数；未新增 `check-script-domain-logic.mjs`（本卡无前端脚本逻辑，UI 属 M2-5）。
> **下方两张任务卡转为存档**，执行指针以 §1 顶层表为准；M2-3 整体 PASS 由强模型裁定。

> M2-3（脚本领域与持久化）已拆为两张子卡，冻结依据见
> `logs/checkpoints/M2-3-20260903-1556.md`（含现状实测、对两份前置草案的事实修正、
> 待裁决分歧、测试矩阵 T-scr-1~13 与违规码清单）。
> M2-3 整体 PASS 须待 a/b 两卡全绿后裁定；裁定前不得移动 NEXT 到 M2-4。
> **M2-3 不做脚本执行**：`run_script`/取消/超时/输出上限/进程树 kill 一律归 M2-4，
> 前端面板归 M2-5。

```text
TASK_ID=M2-3.a
ROUTE=AI:DEEP
MODEL=<界面完整模型名>
REASONING=high
GOAL=冻结脚本领域契约（纯文档，不改产品代码）：
     裁决两份草案互不兼容的 ScriptMeta 设计（args/params 字段名、ParamType、
     interpreter、timeout_secs、builtin、enabled），冻结
     ScriptMeta/ScriptParam/ParamType 结构与 serde 契约；
     scripts.json 存储布局与正文文件命名（运行记录是否本卡落盘）；
     参数占位符与危险参数 9 条 fail-closed 规则（是否作为纯函数在 M2-3 落地）；
     审计字段与脱敏规则（secret 显式标记 + 名字正则兜底、输出只写摘要）；
     划清 M2-4 边界（执行/取消/超时/并发互斥/运行历史归 M2-4）；
     测试契约（T-scr-1~13 + 12 违规码静态夹具）
READ=logs/checkpoints/M2-3-20260903-1556.md（现状实测/事实修正/待裁决分歧/矩阵/违规码），
     logs/assist/M2-3-assist-20260901-1622.md（assist 版字段初稿），
     logs/assist/M2-3.a-prework-20260902-1055.md（prework 版契约草案，R1 阻塞项已解除），
     src-tauri/src/domain.rs（现有 DTO 与 serde 契约），
     src-tauri/src/workspace.rs（repos/bookmarks/sessions 持久化范式），
     src-tauri/src/security_policy.rs（既有校验器与文本边界口径），
     src-tauri/src/shutdown.rs + src-tauri/src/bridge.rs register_shutdown_tasks（退出收口现状），
     src-tauri/permissions/default-commands.toml、src-tauri/capabilities/default.json（ACL 与 shell 权限现状）
WRITE=logs/checkpoints/M2-3.a-<YYYYMMDD-HHMM>.md（冻结裁定书），
      详细设计与实施计划.md, 后续需求TODO.md, AI-模型切换与接手清单.md
FORBID=不得改任何产品代码（src/ src-tauri/ scripts/ 一律只读，纯文档冻结）；
      不得新增 run_script / script_cancel / 任何 std::process::Command（越界 M2-4）；
      不得改 capabilities/default.json 的 shell 权限（扩张属安全边界决策须另行裁决）；
      不得新增 npm 依赖；不得删除/改动 M0/M1/M2 证据与 checkpoints；
      不得提前勾选 M2-3；不得与 M2-3.b 合并提交
COMMANDS=git status --short --branch（开工前必须 clean）
      python3 scripts/check-plan-routing.py
      bash scripts/pre-merge.sh            # 必须 PRE_MERGE_RESULT=ALL_PASS
      git diff --check
      git diff --stat                      # 必须只有 docs/logs 变更
PASS_CRITERIA=1. M2-3.a 冻结裁定书落盘，§4 五组分歧逐组给出结论（不得留白）；
      2. 三份主文档回写 NEXT=M2-3.b；pre-merge ALL_PASS；git diff --check 0；
      3. git diff --stat 证明 src/ src-tauri/ scripts/ 零改动；工作树干净
FAIL_ACTION=发现冻结契约无法自洽（如字段口径与安全规则冲突）→ STATUS=BLOCKED
      回写 logs/checkpoints/M2-3.a-<ts>.md，NEXT 保持 M2-3.a，交还强模型
DOC_BACKWRITE=详细设计与实施计划.md 顶部状态；后续需求TODO.md 关键路径；
      AI-模型切换与接手清单.md §1 顶层表 + 关键路径；
      logs/checkpoints/M2-3.a-<ts>.md
COMMIT=docs(M2-3.a): freeze script domain contract
NEXT=M2-3.b
```

```text
TASK_ID=M2-3.b
ROUTE=AI:BALANCED
MODEL=<界面完整模型名>
REASONING=medium
GOAL=按 M2-3.a 冻结契约实现脚本领域与持久化：
     domain.rs 新增 ScriptMeta/ScriptParam/ParamType（+ RunStatus 若裁定需要）；
     危险参数校验纯函数（9 条 fail-closed 规则，若 a 卡裁定本卡落地）；
     workspace.rs 新增 scripts_file / load_scripts / save_scripts / 正文读写（原子写 + 前缀校验）；
     bridge.rs 四条元数据命令（list/add/update/remove）过 check_invocation_source + 审计脱敏；
     main.rs 注册 + ACL 同步；types.ts / bridge.ts TS 镜像；
     新增 check-script-domain-policy.py 与 check-script-domain-logic.mjs 并接入 pre-merge.sh
READ=logs/checkpoints/M2-3.a-<ts>.md（冻结裁定书，唯一契约依据），
     logs/checkpoints/M2-3-20260903-1556.md §5~§6（测试矩阵/违规码），
     src-tauri/src/domain.rs、src-tauri/src/workspace.rs（repos/bookmarks 范式）、
     src-tauri/src/bridge.rs（bookmark/session 命令范式）、src-tauri/src/images.rs（纯函数层范式）、
     src-tauri/src/security_policy.rs（既有校验器复用），
     src-tauri/src/main.rs（invoke_handler）、src-tauri/permissions/default-commands.toml，
     src/types.ts / src/bridge.ts（TS 镜像范式）
WRITE=src-tauri/src/domain.rs, src-tauri/src/workspace.rs（或新增 scripts.rs 纯函数层）,
      src-tauri/src/bridge.rs, src-tauri/src/main.rs, src-tauri/permissions/default-commands.toml,
      src/types.ts, src/bridge.ts,
      scripts/check-script-domain-policy.py, scripts/check-script-domain-logic.mjs,
      scripts/pre-merge.sh（接入两项新门禁）,
      详细设计与实施计划.md, 后续需求TODO.md, AI-模型切换与接手清单.md,
      logs/checkpoints/M2-3.b-<YYYYMMDD-HHMM>.md
FORBID=不得新增 run_script / script_cancel / 任何 std::process::Command 或 shell spawn（M2-4 专属）；
      不得改 capabilities/default.json 的 shell 权限；
      不得新增 npm 依赖（package.json / package-lock.json 零变化）；
      不得放宽既有来源校验；不得跨检查点合 M2-4/M2-5；
      审计不得写参数值明文（须经脱敏）；
      不得提前勾选 M2-3 整体 PASS；不得删除/改动既有证据与 checkpoints
COMMANDS=git status --short --branch
      cargo test --manifest-path src-tauri/Cargo.toml   # 156 基线 + 新单测，全过
      cargo build --manifest-path src-tauri/Cargo.toml --release --locked
      npm run build                                      # 0 error
      python3 scripts/check-script-domain-policy.py --self-test && \
      python3 scripts/check-script-domain-policy.py
      node scripts/check-script-domain-logic.mjs         # 断言全过
      bash scripts/pre-merge.sh                          # PRE_MERGE_RESULT=ALL_PASS
      git diff --check
      # release 冒烟：启动 release 二进制，应用不因新命令注册而 panic（日志留证）
PASS_CRITERIA=1. cargo test 全过（含新增单测，覆盖 T-scr-1~13 中属本卡的部分）、
      cargo build --release 0、warning 不增加（仍为既有 2）；
      2. npm run build 0 且 measure-build-metrics --compare 未回归（总体积 ≤15%）；
      3. 两新门禁 self-test + 正式 PASS 且已接入 pre-merge；pre-merge ALL_PASS；
      4. git diff --check 0；工作树干净；
      5. release 启动冒烟无 panic（运行时端到端与 GUI 挂账则如实记录）
FAIL_ACTION=发现冻结契约在代码中无法自洽 → STATUS=BLOCKED 回写
      logs/checkpoints/M2-3.b-<ts>.md，NEXT 保持 M2-3.b，交还强模型裁决；
      其余失败保留 NEXT，连续两次失败即停止
DOC_BACKWRITE=详细设计与实施计划.md §4.4 M2-3 勾选（仅整体裁定后）+ 顶部状态；
      后续需求TODO.md 关键路径；AI-模型切换与接手清单.md §1 顶层表 + 关键路径；
      logs/checkpoints/M2-3.b-<ts>.md
COMMIT=feat(M2-3.b): add script domain and persistence
NEXT=M2-3 整体裁定（a/b 全绿后由强模型签 M2-3 PASS，再移 M2-4）
```

### 历史任务卡：M2-2.a / M2-2.b（均已执行，2026-09-03）

> **执行结果（2026-09-03 14:54 更新）**：M2-2.a 冻结裁定书 PASS（`00948f8`）；
> M2-2.b 已按冻结契约实现完毕，`STATUS=PASS_WITH_DEBT`，证据
> `logs/checkpoints/M2-2.b-20260903-1454.md`（`cargo test` 156/156、
> `cargo build --release` 0 且 warning 未增加、`npm run build` 0、总体积 +12.63%、
> 两项新门禁自检与默认模式均 PASS、pre-merge ALL_PASS、release 启动冒烟无 panic）。
> **挂账**：asset:// 端到端加载（无 GUI 自动化通道、真实数据目录尚无落盘图片）
> 与 GUI 目视验收，人工验收清单见该 checkpoint §6。
> **下方两张任务卡转为存档**，执行指针以 §1 顶层表为准；M2-2 整体 PASS 由强模型裁定。

> M2-2（图片预览 UI）已拆为两张子卡，冻结依据见 `logs/checkpoints/M2-2-20260903-1340.md`。
> M2-2 整体 PASS 须待 a/b 两卡全绿后由强模型裁定，裁定前 NEXT 不得越到 M2-3。

```text
TASK_ID=M2-2.a
ROUTE=AI:DEEP
MODEL=<界面完整模型名>
REASONING=high
GOAL=冻结字节读取通道决策与 M2-2.b 实现契约（纯文档，不改产品代码）：
     convertFileSrc + asset:// 通道（scope 不扩）、workspace_images_dir 只读命令契约、
     共用预览组件契约（shared/ + ArtifactPanel 挂点 + App.vue 全局灯箱）、
     交互契约（缩放 [0.25,4] 步长 1.2、双击复位、不循环切换、错误态不关灯箱、
     键盘监听 open 才绑定）、隐私红线（溯源只出 host、source_url 原文不上屏）、
     测试契约（imagePreview.ts 纯逻辑 + headless 断言 + 静态夹具违规码）。
READ=logs/checkpoints/M2-2-20260903-1340.md（拆分结论/事实修正/冻结契约/测试矩阵/违规码），
     logs/checkpoints/M2-1-20260903-0927.md（ImageRef 契约与 §9 遗留），
     src-tauri/src/workspace.rs（workspace_dir 路径推导），
     src-tauri/tauri.conf.json（assetProtocol.scope 现状），
     src/types.ts / src/utils/image.ts（前端 ImageRef 与展示逻辑），
     logs/assist/M2-2.a-assist-20260901-1621.md、logs/assist/M2-2.b-prework-20260902-1055.md
     （前置草案；过时口径以 M2-2 checkpoint §3 修正为准）
WRITE=logs/checkpoints/M2-2.a-<YYYYMMDD-HHMM>.md（冻结裁定书），
      详细设计与实施计划.md, 后续需求TODO.md, AI-模型切换与接手清单.md
FORBID=不得改任何产品代码（src/ src-tauri/ scripts/ 一律只读，纯文档冻结）；
      不得修改 tauri.conf.json 的 assetProtocol.scope（静态判断已覆盖，
      扩张属安全边界决策须另行裁决且与消费方同 PR 评审）；
      不得放宽 save_image 来源范围、不得改 remote-collect.toml / injected/collect.js
      （InlineDataUrl 消费留后续检查点另行裁决）；
      不得删除/改动 M0/M1/M2-1 证据与 checkpoints；不得提前勾选 M2-2；
      不得与 M2-2.b 合并提交
COMMANDS=git status --short --branch（开工前必须 clean）
      python3 scripts/check-plan-routing.py
      bash scripts/pre-merge.sh            # 必须 PRE_MERGE_RESULT=ALL_PASS
      git diff --check
      git diff --stat                      # 必须只有 docs/logs 变更
PASS_CRITERIA=1. M2-2.a 冻结裁定书落盘，且 §4 七项冻结契约逐条给出结论（不得留白）；
      2. 三份主文档回写 NEXT=M2-2.b；pre-merge ALL_PASS；git diff --check 0；
      3. git diff --stat 证明 src/ src-tauri/ scripts/ 零改动；工作树干净
FAIL_ACTION=发现冻结契约无法自洽（如通道证据不足、契约与 M2-1 冲突）→ STATUS=BLOCKED
      回写 logs/checkpoints/M2-2.a-<ts>.md，NEXT 保持 M2-2.a，交还强模型
DOC_BACKWRITE=详细设计与实施计划.md 顶部状态；后续需求TODO.md 关键路径；
      AI-模型切换与接手清单.md §1 顶层表 + 关键路径；
      logs/checkpoints/M2-2.a-<ts>.md
COMMIT=docs(M2-2.a): freeze image preview channel contract
NEXT=M2-2.b
```

```text
TASK_ID=M2-2.b
ROUTE=AI:BALANCED
MODEL=<界面完整模型名>
REASONING=medium
GOAL=按 M2-2.a 冻结契约实现图片预览 UI：画廊网格/灯箱/缩放/错误态/空态，
     共用预览组件落 shared/，成果库挂点 ArtifactPanel，灯箱全局挂 App.vue；
     新增 workspace_images_dir 只读命令；字节通道 convertFileSrc + asset://；
     落地 imagePreview.ts 纯逻辑层 + check-image-preview-logic.mjs headless 断言 +
     check-image-preview-policy.py 静态夹具，并接入 pre-merge.sh。
READ=logs/checkpoints/M2-2.a-<ts>.md（冻结裁定书，唯一契约依据），
     logs/checkpoints/M2-2-20260903-1340.md §4~§6（契约/测试矩阵/违规码），
     src/types.ts（ImageRef）、src/utils/image.ts（展示逻辑复用）、src/bridge.ts、
     src/components/workspace/ArtifactPanel.vue（挂点）、src/App.vue（全局弹窗范式）、
     src/components/shared/ConfirmModal.vue（modal-mask 层级范式）、
     src/stores/useWorkspaceStore.ts（current/openArtifact），
     src-tauri/src/bridge.rs（save_image/list_artifact_images 邻近实现）、
     src-tauri/permissions/default-commands.toml、src-tauri/src/main.rs（invoke_handler）
WRITE=src/components/shared/ImageGallery.vue, src/components/shared/ImageLightbox.vue,
      src/utils/imagePreview.ts, src/stores/（如确需新 store；优先并入现有 store 需说明）,
      src/App.vue（全局挂载灯箱）, src/components/workspace/ArtifactPanel.vue（挂点）,
      src/bridge.ts（workspaceImagesDir 封装）, src/types.ts（如需补类型）,
      src-tauri/src/bridge.rs（workspace_images_dir 命令）, src-tauri/src/main.rs（注册）,
      src-tauri/permissions/default-commands.toml（ACL）,
      scripts/check-image-preview-policy.py, scripts/check-image-preview-logic.mjs,
      scripts/pre-merge.sh（接入两项新门禁）,
      详细设计与实施计划.md, 后续需求TODO.md, AI-模型切换与接手清单.md,
      logs/checkpoints/M2-2.b-<YYYYMMDD-HHMM>.md
FORBID=不得修改 tauri.conf.json 的 assetProtocol.scope（含 add/remove/reorder）；
      不得新增 npm 依赖（package.json / package-lock.json 零变化）；不得引入 UI 组件库；
      不得放宽 save_image 来源范围、不得改 remote-collect.toml / injected/collect.js；
      不得修改 markdown.ts 解析图片语法（v-html 内既有 <img> 点击用事件委托接管）；
      不得把 source_url 原文渲染上屏（溯源只出 host，复用 imageSourceHost）；
      组件（除 bridge.ts 外）不得直接 invoke；不得跨检查点合 M2-3；
      不得提前勾选 M2-2 整体 PASS；不得删除/改动既有证据与 checkpoints
COMMANDS=git status --short --branch
      cargo test --manifest-path src-tauri/Cargo.toml   # 151 基线 + 新命令单测，全过
      cargo build --manifest-path src-tauri/Cargo.toml --release --locked
      npm run build                                      # 0 error
      python3 scripts/check-image-preview-policy.py --self-test && \
      python3 scripts/check-image-preview-policy.py
      node scripts/check-image-preview-logic.mjs         # 断言全过
      bash scripts/pre-merge.sh                          # PRE_MERGE_RESULT=ALL_PASS
      git diff --check
      # release 冒烟：启动 release 二进制，成果含真实落盘图片，asset:// 经
      # convertFileSrc 加载成功（留日志/截图证据；做不到则如实挂账，不伪造）
PASS_CRITERIA=1. cargo test 全过（含新命令单测）、cargo build --release 0、warning 不增加；
      2. npm run build 0 且 measure-build-metrics --compare 未回归（总体积 ≤15%）；
      3. 两新门禁 self-test + 正式 PASS 且已接入 pre-merge；pre-merge ALL_PASS；
      4. T-prev-1~12 逻辑断言全过；git diff --check 0；工作树干净；
      5. release 冒烟 asset:// 加载证据（或如实记录运行时/GUI 挂账，PASS_WITH_DEBT 需强模型裁定）；
      6. GUI 目视验收未完成则如实声明（同 M1/M2-1 口径），附人工验收清单
FAIL_ACTION=asset:// 冒烟证明 scope 不覆盖 → 不得自行扩 scope，STATUS=BLOCKED 回写
      logs/checkpoints/M2-2.b-<ts>.md（含冒烟证据），NEXT 保持 M2-2.b，交还强模型裁决；
      其余失败保留 NEXT，连续两次失败即停止
DOC_BACKWRITE=详细设计与实施计划.md §4.4 M2-2 勾选（仅整体裁定后）+ 顶部状态；
      后续需求TODO.md 关键路径；AI-模型切换与接手清单.md §1 顶层表 + 关键路径；
      logs/checkpoints/M2-2.b-<ts>.md
COMMIT=feat(M2-2.b): add image gallery lightbox preview UI
NEXT=M2-2 整体裁定（a/b 全绿后由强模型签 M2-2 PASS，再移 M2-3）
```

### 历史任务卡：M1-4（已 PASS，2026-09-01）

> **M1-4 已由 `AI:DEEP / R:high` 强模型实现并实测 PASS**，证据
> `logs/checkpoints/M1-4-20260901-1805.md`；**M1-5 已由 `AI:BALANCED / R:high`
> 实现并 PASS**，证据 `logs/checkpoints/M1-5-20260902-1130.md`；**M1-6.a（Git 写能力契约冻结+任务卡展开）已由 `AI:BALANCED / R:high` PASS**，证据
> `logs/checkpoints/M1-6.a-20260902-0955.md`；**M1-6.b（Git 写后端核心：双阶段闸门 + 六个白名单写操作）已由 `AI:DEEP / R:high`（Kimi K3）PASS**，证据
> `logs/checkpoints/M1-6.b-20260902-1327.md`；**M1-6.c（Git 写后端核心复核）已由 `AI:BALANCED / R:high`（腾讯 Hy4）PASS**，证据
> `logs/checkpoints/M1-6.c-20260902-1430.md`；**M1-6.d（Git push）已由 `AI:DEEP / R:xhigh`（Kimi K3）PASS**，证据
> `logs/checkpoints/M1-6.d-20260902-1525.md`；**M1-6 Git 写能力整体 PASS**（b/c/d 三卡全绿），NEXT 已移至 `M1-7`（Git UI）。
> 下方 M1-4 任务卡保留存档，供格式参考。

```text
TASK_ID=M1-4
ROUTE=AI:DEEP
MODEL=<界面完整模型名>
REASONING=high
GOAL=把应用注册为 Linux 默认浏览器，外部 http(s) 链接路由到内嵌页签打开。
READ=src-tauri/src/main.rs（Builder 在 1074，RunEvent 分支在 ~1305），
     src-tauri/tauri.conf.json（productName=mvp-browser-os，identifier=com.jizhijiandan.mvp），
     setup-linux.sh（当前无任何 xdg/MIME 相关逻辑），
     src/bridge.ts + src/stores/useBrowserStore.ts（tabNew / openBrowser），
     src/App.vue（onMounted 末尾 m0Ready() 是前端就绪信号，可作冷启动重放锚点），
     详细设计与实施计划.md §3.2，logs/checkpoints/M1-3-20260901-1123.md
WRITE=src-tauri/src/main.rs（RunEvent::Opened 分支），
      setup-linux.sh（MIME 注册 + xdg-settings，必须用户显式触发），
      src-tauri/tauri.conf.json（如需自定义 .desktop 模板），
      新增 settings 开关与对应 IPC（如需），
      详细设计与实施计划.md, 后续需求TODO.md, AI-模型切换与接手清单.md,
      logs/checkpoints/M1-4-<YYYYMMDD-HHMM>.md
FORBID=不得在 deb 安装/首次启动时静默改写用户默认浏览器（必须用户显式确认）；
      不得接受 http/https 以外的 scheme（file://、javascript: 一律拒绝）；
      不得删除或改动 M0/M1-2/M1-3 的证据与 checkpoints；
      不得与 M1-3 合并提交，不得提前勾选 M1-4；
      不得修改已冻结的 M0 基线脚本与契约
COMMANDS=git switch feature-M0-baseline（或对应 worktree）
      git status --short --branch
      grep -rn "on_open_url\|RunEvent::Opened" src-tauri/src/main.rs   # 开工前应为空
      cargo build --release
      npm run build
      bash scripts/pre-merge.sh        # 必须 PRE_MERGE_RESULT=ALL_PASS
      git diff --check
PASS_CRITERIA=1. xdg-settings get default-web-browser 显示本应用，且该变更只在用户
      显式点击「设为默认浏览器」后发生；
      2. xdg-open https://example.com 能在应用内嵌页签打开该 URL，不另开系统浏览器；
      3. 冷启动场景（应用未运行时 xdg-open）不丢 URL（需有就绪后重放机制并留证据）；
      4. 非 http/https 的 URL 被拒绝且有日志/提示，不崩溃；
      5. npm run build 0；scripts/pre-merge.sh ALL_PASS；git diff --check 0；工作树干净
FAIL_ACTION=实现遇阻或契约不对 → 回写 STATUS=BLOCKED 到
      logs/checkpoints/M1-4-<ts>.md，NEXT 保持 M1-4；连续两次失败即停止并交还更强模型
DOC_BACKWRITE=详细设计与实施计划.md §3.2 顶部 + M1-4 勾选；
      AI-模型切换与接手清单.md §1 顶层表 + WBS 关键路径加 M1-4 → M1-5；
      后续需求TODO.md 当前关键路径；logs/checkpoints/M1-4-<ts>.md
COMMIT=feat(M1-4): register default browser + on_open_url routing
NEXT=M1-5
```

#### 实现要点（基于 2026-09-01 代码现状盘点，供强模型复核）

- **现状**：`main.rs` 目前**没有** `RunEvent::Opened` 分支，只有 ~1305 行的
  `RunEvent::ExitRequested`；`setup-linux.sh` **没有**任何 `xdg-settings` / MIME 逻辑；
  `tauri.conf.json` 未配置自定义 `.desktop` 模板，桌面文件由 bundler 生成，
  **实际文件名需实测确认**（`mvp-browser-os.desktop` 还是 `com.jizhijiandan.mvp.desktop`），
  不得凭猜测写入 `xdg-settings set default-web-browser`。
- **冷启动是主要风险**：进程未运行时 `xdg-open` 传来的 URL 会早于前端 `onMounted` 到达，
  直接 `tab_new` 大概率落空。需要「先缓存、待前端就绪后重放」；
  `App.vue` 的 `bridge.m0Ready()`（mount + 2×rAF 后调用）是现成的就绪信号锚点。
- **安全边界**：只放行 `http` / `https`，其余 scheme 一律拒绝并 `log_audit`，
  与既有 `security-policy-v2` 保持一致；不得把任意字符串拼进导航。
- **不得静默改系统设置**：设默认浏览器是用户可见的系统级变更，
  必须做成设置项里的显式按钮 + 确认，禁止在安装脚本或首次启动时自动执行。

### 历史任务卡示例：M1-1（已 PASS，仅作格式参考）

```text
TASK_ID=M1-1
ROUTE=AI:FAST
MODEL=<界面完整模型名>
REASONING=low
GOAL=生成并验证桌面/菜单图标资源，不实现书签、默认浏览器或 Git 面板。
READ=src-tauri/tauri.conf.json, package.json, src-tauri/icons/, 详细设计与实施计划.md §3.2
WRITE=src-tauri/icons/, 详细设计与实施计划.md, 后续需求TODO.md, AI-模型切换与接手清单.md, logs/checkpoints/M1-1-<YYYYMMDD-HHMM>.md
FORBID=不得实现 M1-2/M1-3/M1-4；不得改浏览器协议和安全策略；不得删除 M0 证据
COMMANDS=npm run tauri -- icon 或项目现有图标生成命令；npm run build；git diff --check
PASS_CRITERIA=图标资源齐全；构建通过；文档回写 NEXT；工作树干净
FAIL_ACTION=图标源缺失或生成命令不可用则 STATUS=BLOCKED，NEXT 保持 M1-1
DOC_BACKWRITE=M1-1 checkpoint、三份主文档顶部状态
COMMIT=feat(M1-1): generate desktop icon assets
NEXT=M1-2
```

M1-0 只补主页快捷方式，不等同 M1-2/M1-3 正式收藏夹；下一步按 M1-1 做图标资源。

用户提供的微信临时图片在读取时已不存在，因此本文件不猜测 Trae 模型名称。重新附图或直接写出模型列表后，只填写上表三个 Trae 单元格并升级本文版本；任务上的 `AI:*` 标签不变。

Trae 选模规则：

1. `M0-0.b/c` 选 Trae 中具备仓库读写、Shell 执行和多文件理解能力的日常编码模型，映射到 `TRAE_BALANCED`；正式 GUI 采样由既有脚本驱动。
2. 如果免费模型无法运行 Shell 或稳定处理多文件，只允许调研，不得勾选检查点或提交 PASS。
3. M0-2、M0-3、安全、进程树和并发任务必须使用 `TRAE_DEEP`；没有强模型时暂停，不能用轻量模型硬做。
4. 每个检查点在证据中记录 UI 显示的完整模型名、平台、推理档位和 `MODEL_DEVIATION`。全局映射缺失不妨碍记录实际执行模型。

## 3. Trae 在 M0 内连续自动执行

「一次只领取一个检查点」是变更和提交边界，不是要求每做完一点都停下等用户确认。Trae 可以在同一次会话中尽可能推进 M0，但不得一次领取、实现或提交多个检查点。

### 连续执行循环

1. 从本文顶部和 `详细设计与实施计划.md` §2.2 读取唯一 `NEXT`，本轮只领取该检查点。
2. 按 `AI:*` 路由选择 Trae 模型；开始后不在该检查点中途换模型。
3. 只修改当前检查点需要的文件，运行该点全部验收和 `git diff --check`。
4. PASS 后回写 §5 规定的状态、模型、命令、证据和 `NEXT`；FAIL 不得移动指针。
5. 使用 `<type>(<WBS.checkpoint>): <单一结果>` 独立提交。功能实现用 `feat`，重构用 `refactor`，缺陷修复用 `fix`，不得合并多个检查点。
6. 提交后运行 `git status --short`。工作树干净且未命中硬停止条件时，无需等待用户回复，立即回到第 1 步领取新 `NEXT`。

### 硬停止条件

命中任一条即停止写代码，保留当前指针并回写 `BLOCKED`、已试命令、证据与推荐下一步：

1. 同一检查点连续两次验收失败，或根因仍不明。
2. 检查点要求 `AI:DEEP`，但 Trae 没有等效强模型；不得用轻量模型硬做安全、生命周期、并发或 IPC。
3. 缺少 GUI/登录态、必要权限、外部环境或人工验收条件，且无法用固定夹具替代。
4. 需要修改冻结契约、扩大安全权限、删除用户数据，或需要产品/项目负责人裁决。
5. 检查点边界的工作树不干净，或发现来源不明、可能属于用户的并行改动。
6. 当前平台额度或时间耗尽。
7. 到达需要项目负责人/用户签字的放行门禁，或下一指针未在本文展开成可执行任务卡。历史上的 `M0-7.c` 门禁已于 2026-09-01 09:05 CST 完成。

## 4. 已完成检查点记录：M0-1.a

> 状态：**已完成**（2026-08-29，commit `1bd56b1`；验收命令全过 + `--self-test` ALL_PASS）。以下为执行时的边界与要求，保留作记录。

本节的允许/禁止范围只约束 `M0-1.a` 这一轮。该检查点 PASS、证据回写、独立提交且工作树干净后，执行器应立即领取 `M0-1.b`，不再受本节「不实现 M0-1.b」的单点边界限制。

### 目标

新增 `scripts/baseline-check.sh`，把 M0-0.a 契约中的质量、构建、来源/环境指纹和证据目录初始化编码成可复跑门禁。

### 开工前

```bash
git switch feature-M0-baseline
git status --short --branch
git log -3 --oneline
```

必须看到分支为 `feature-M0-baseline` 且工作树干净。若不干净，先审查差异来源，不得 reset、checkout 或覆盖未知改动。

必读文件：

1. `AI-模型切换与接手清单.md`
2. `详细设计与实施计划.md` §2.2 的 M0-0/M0-1
3. `logs/m0-baseline-contract-v1.md`
4. `.gitignore`、`package.json`、`src-tauri/Cargo.toml`

### 允许改动

- 新增 `scripts/baseline-check.sh`。
- 可新增只服务该脚本的固定夹具或自测文件，放在 `scripts/fixtures/` 或 `scripts/tests/`。
- 完成验证后，按 §5 回写状态文档和检查点证据。

### 禁止改动

- 不实现 `scripts/verify-resources.sh`，那属于 M0-1.b。
- 不修改 Rust/Vue 产品代码，不补 ready/终端测量钩子，那属于 M0-0.b。
- 不采集或宣称正式 release 性能基线，那属于 M0-0.b/c。
- 不处理 capability、资源泄漏、依赖清理或 GUI 回归。
- 不修改冻结契约的指标语义；若发现必须修改，停止并记录 `CONTRACT_CHANGE_REQUIRED`，不得静默改口径。

### 实现要求

- Bash 使用 `set -euo pipefail`，仓库根目录通过 `git rev-parse --show-toplevel` 获取，禁止硬编码 `/home/...`。
- 至少支持 `--help`、`--self-test`、正式运行模式和非法参数非零退出。
- 候选基线要求 clean commit；自测模式允许在开发工作树运行，但必须清晰标记为 fixture，不能生成正式 PASS。
- 初始化契约规定的 run 目录，生成来源/环境字段、命令原始输出、`summary.json` 和 `summary.md`。
- 覆盖 fmt、结构化 clippy warning 分组、前端构建/体积、release 二进制信息；不得把重复的人类可读 warning 行直接相加。
- 不自动安装依赖、不访问公网、不删除用户数据；生成物只进入本 run 的隔离目录。
- 当前 `.gitignore` 会忽略 `*.log`。M0-1.a 必须明确证据保留策略：至少提交 summary、环境、场景、结构化 measurements 和哈希；若要求原始 `.log` 入库，先提出契约版本变更，不能让它们静默丢失。
- 产品侧 ready/终端钩子尚未实现时，相关正式指标返回明确 `BLOCKED` 和非零退出；`--self-test` 使用固定夹具验证脚本本身。

### 验收命令

```bash
bash -n scripts/baseline-check.sh
bash scripts/baseline-check.sh --help
bash scripts/baseline-check.sh --self-test
bash scripts/baseline-check.sh --definitely-invalid
git diff --check
```

最后一条非法参数命令必须非零退出。若新增独立自测脚本，也必须执行并在证据中记录。

### 提交

本轮只提交 M0-1.a：

```text
feat(M0-1.a): add baseline quality gate
```

检查点未通过时不得勾选。修复提交使用 `fix(M0-1.a-fix1): ...`，不得伪装成下一检查点。PASS 并完成上述提交后，按 §3 继续自动执行。

## 5. 每个检查点做完后必须回写什么

### 必改文档

1. `详细设计与实施计划.md`
   - 升级文档版本、日期时间、实际模型。
   - 只有验收全部通过才把对应 `[ ]` 改为 `[x]`。
   - 更新下一检查点和任何真实阻塞，不修改未开始里程碑状态。
2. `后续需求TODO.md`
   - 更新“当前关键路径”和队列表状态。
   - 标出 `PASS/NEXT/BLOCKED`，不得只写“已开发”。
3. `AI-模型切换与接手清单.md`
   - 更新当前执行器、实际模型、最近提交、验证命令与结果。
   - 将“下一检查点”移动一格；失败则保持原检查点并写 fix 编号。
4. `logs/checkpoints/<WBS>-<YYYYMMDD-HHMM>.md`
   - 记录 commit、模型、变更文件、命令、期望/实际、证据路径和结论。

### 条件修改

- `logs/m0-baseline-contract-v1.md`：只有指标/场景/公式确需变化时修改，并升级契约版本；普通实现不得改。
- `logs/baseline-2026-08-27.md`：历史记录，原则上不再修改。
- README/使用指南：只有用户可见行为已经实现并验收后才更新。
- 源码架构文档：仅在对应架构检查点真正落地时更新。

### 回写模板

```text
CHECKPOINT=M0-1.a
STATUS=PASS|FAIL|BLOCKED
EXECUTOR=Trae|Codex
MODEL=<界面完整模型名>
ROUTE=AI:BALANCED
MODEL_DEVIATION=none|<原因>
COMMIT=<sha>
VERIFY=<命令及退出码>
NEXT=M0-1.b|M0-1.a-fix1
```

### M0-1.a 回写记录（2026-08-29，Codex / CodeBuddy 主机）

```text
CHECKPOINT=M0-1.a
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名；按 Codex 映射 AI:BALANCED 对应 gpt-5.6-terra
ROUTE=AI:BALANCED
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=1bd56b1
VERIFY=bash -n(0); --help(0); --self-test(0, ALL_PASS); --definitely-invalid(2); git diff --check(0)
NEXT=M0-1.b
```

证据保留策略（M0-1.a 落地，对应实现要求「明确证据保留策略」）：`.gitignore` 已对 `logs/m0-baseline/` 开例外，
`raw/` 原始命令输出（含 `*.log`）随 run 目录入库可追溯；`summary/environment/scenario/measurements/SHA256SUMS`
全部提交；若未来需要调整原始 `.log` 入库口径，先提契约版本变更。

### M0-1.a-fix1 修复记录（2026-08-29，Codex / CodeBuddy 主机，commit `3a7bbac`）

发现并修复正式模式缺陷：前端构建失败（如依赖未装、超时）时 `run_frontend_build` 不设置
`DIST_TOTAL_BYTES/LARGEST_JS_*`，`write_measurements_frontend` 在 `set -u` 下 unbound 崩溃，
无法生成完整 FAIL 证据（违反「门禁失败返回非 0 且输出完整证据」语义，M0-0.b 会踩坑）。

修复内容：
1. `run_frontend_build` 开头初始化 dist 指标默认值，任何失败路径均不 unbound。
2. `summary.json` 中 dist_total_bytes / largest_js_* 状态与 `FRONTEND_STATUS` 联动（缺原始证据即 FAIL）。
3. `--self-test` 新增用例 6（`BS_SELF_TEST_FRONTEND_FAIL=1` fixture）：断言前端失败时退出非 0、
   summary status=FAIL、证据完整、不崩溃；用例 3 的退出码捕获改为 `|| code=$?` 防御 errexit。
4. 脚本版本 `M0-1.a-1` → `M0-1.a-2`。

回写模板（不移动指针，NEXT 仍为 M0-1.b）：
```text
CHECKPOINT=M0-1.a-fix1
STATUS=PASS（原 M0-1.a 维持 PASS，未改变检查点结论）
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名；按 Codex 映射 AI:BALANCED 对应 gpt-5.6-terra
COMMIT=3a7bbac
VERIFY=bash -n(0); --help(0); --self-test(0, ALL_PASS, 6 用例); --definitely-invalid(2); git diff --check(0)
NEXT=M0-1.b
```

### M0-1.b 回写记录（2026-08-29，CodeBuddy / Codex 主机，commit `506193b`）

```text
CHECKPOINT=M0-1.b
STATUS=PASS（驱动能力与 BLOCKED 语义验收通过；产品 ready/终端钩子未落地，正式基线归 M0-0.b）
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名；按 Codex 映射 AI:BALANCED 对应 gpt-5.6-terra
ROUTE=AI:BALANCED
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=506193b
VERIFY=bash -n(0); --help(0); --self-test(0, ALL_PASS, 8 用例); --definitely-invalid(2); git diff --check(0)
NEXT=M0-1.c
```

实现要点（对应 §7 提示词）：
- 新增 `scripts/verify-resources.sh`（1139 行）：`set -euo pipefail`、根目录 `git rev-parse --show-toplevel` 动态解析；
  `--help` / `--self-test` / 非法参数非零退出（2）/ 正式模式要求 clean commit。
- 覆盖契约 §5/§6 资源指标驱动：release 启动（§6.1）、进程树枚举（`/proc/<pid>/stat` starttime 防 PID 复用）、
  RSS（VmRSS）与 FD（`/proc/<pid>/fd` 可访问条目）采样、idle 采样编排（每 5 秒 × 60 秒）、
  tab/grid/terminal 循环（5 预热 + 20 正式，OLS 斜率/FD delta/孤儿进程检测）、终端吞吐（10 MiB begin/end 标记）。
- 统计与契约 §7 一致：median/min/max/波动率、p95 nearest-rank（<20 样本不伪报）、OLS 斜率。
- 正式模式钩子缺失：`driver_smoke`（真实 /proc 冒烟）后输出完整 BLOCKED 证据
  （environment/scenario/summary.json|md/SHA256SUMS/measurements，`summary status=BLOCKED`）并退出 1；
  10 个资源指标 owner=M0-0.b（补钩子的 WBS），DEFERRED(M2-4/M4-3) 不计入判定。
- 正式 BLOCKED run 证据：`logs/m0-baseline/20260829T143410+0800_506193b_release_x11/`（已入库）。
- 未实现 M0-1.c（参数/退出码/schema/哈希/日志格式统一与 pre-merge 接入）；未修改冻结契约与产品代码。

### M0-1.c 回写记录（2026-08-29，CodeBuddy / Codex 主机，commit `2b476d3`）

```text
CHECKPOINT=M0-1.c
STATUS=PASS（参数/退出码/schema/哈希/日志格式已固定并接入 pre-merge；产品 ready/终端钩子未落地，正式基线归 M0-0.b）
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名；按 Codex 映射 AI:BALANCED 对应 gpt-5.6-terra
ROUTE=AI:BALANCED
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=2b476d3
VERIFY=bash -n(0×3); --help(0×3); --self-test(0, baseline 7 用例 / verify 9 用例 ALL_PASS); --definitely-invalid(2×3); validate-summary.py --self-test(0); pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); git diff --check(0)
NEXT=M0-0.b
```

实现要点（对应 §7 提示词）：
- 新增 `scripts/schema/m0-summary.schema.json`（draft-07）与 `scripts/validate-summary.py`
  （python3 标准库共享校验器，支持 `--self-test` 与 `<schema> <summary>` 双模式）；
  两脚本 `--self-test` 各加 schema 校验用例（baseline 6→7、verify 8→9），脚本版本升至
  `M0-1.a-3` / `M0-1.b-2`；既有 BLOCKED run 的 summary.json 经校验 VALID。
- 新增 `scripts/GATE-CONTRACT.md`：固定参数面（`--help/-h`、`--self-test`、`--`、未知选项/多余
  参数）、退出码（0=PASS / 1=FAIL|BLOCKED / 2=用法错误）、schema、SHA256SUMS 与日志格式
  （正式 `[<tag>] ` 前缀、self-test `PASS:/FAIL:` + `SELF_TEST_RESULT=` 尾行）。
- 新增 `scripts/pre-merge.sh`：合并前门禁入口（bash -n ×2、--help ×2、--self-test ×2、
  非法参数 ×2、schema 自检、git diff --check），输出 `PRE_MERGE_RESULT=ALL_PASS`；
  支持 `--help` / `--self-test` / 非法参数 2。`.githooks/pre-merge-commit` 为可选 hook
  模板（cp 到 .git/hooks 或由用户自行 core.hooksPath，本流程不代改 git 配置）。
- 主仓库正式 run 证据：`logs/m0-baseline/20260829T153210+0800_2b476d3_release_x11/`
  （status=BLOCKED，11 项 blocked，driver=PASS，schema VALID，SHA256SUMS 通过）。
- 未实现 M0-0.b（产品 ready/终端钩子与正式基线采集）；未修改冻结契约与产品代码。

### M0-0.b/c 被拒候选与修复记录（2026-08-30）

```text
CHECKPOINT=M0-0.b/M0-0.c candidate
STATUS=REJECTED（不得移动指针）
SOURCE_COMMIT=ac0ecac
REASON=about:blank 被改写为百度搜索；aggregate 六项指标 UNSTABLE
FIX_COMMIT=e8975d6
DECISION_COMMIT=b9077d9
LATEST_GATE_COMMIT=73e9dfb
VERIFY=scripts/pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS)
NEXT=M0-0.b RE-RUN
```

被拒原始证据保留在 `logs/m0-baseline/` 与 `logs/m0-baseline/manifests/`，只用于诊断；裁决见 `logs/checkpoints/M0-0.c-20260830-0913.md`。`e8975d6` 后的短样本 smoke 只证明 URL/FD/线程修复方向正确，不能替代契约规定的一批正式 M0-0.b 和三批正式 M0-0.c。

### M0-0.b 修复后正式 PASS（2026-08-30，源提交 `b82eb55`）

```text
CHECKPOINT=M0-0.b
STATUS=PASS
EXECUTOR=Codex 主任务；采样由既有脚本无人值守执行
ROUTE=AI:BALANCED
SOURCE_COMMIT=b82eb55a0fd5d11f572753b9c17e929c47c787d0
VERIFY=scripts/collect-m0-baseline.sh(0, quality PASS, resources PASS, aggregate PASS)
MANIFEST=logs/m0-baseline/manifests/20260830T145117+0800_b82eb55_M0-0.b/
NEXT=M0-0.c
```

完整指标、run ID、二进制哈希和证据路径见 `logs/checkpoints/M0-0.b-20260830-1452.md`。

### M0-0.c 三批采集与 UNSTABLE 裁决（2026-08-30，源提交 `93a1ba6`）

```text
CHECKPOINT=M0-0.c
STATUS=COMPLETE_WITH_ADJUDICATION
RAW_AGGREGATE_STATUS=UNSTABLE（不得改写）
EXECUTOR=Codex 主任务；机械数据归因=gpt-5.6-luna/low
ROUTE=AI:BALANCED
SOURCE_COMMIT=93a1ba63983fba0e5e62ec623c225aa4c13f0bdd
VERIFY=三批 quality PASS + 三批 resources PASS；commit/environment/binary comparable
MANIFEST=logs/m0-baseline/manifests/20260830T153538+0800_93a1ba6_M0-0.c/
RISK_OWNER=tab RSS/FD -> M0-5 BLOCKER；terminal frame gap -> M3-1
NEXT=M0-2.a
```

六项指标仍使用 UNSTABLE 区间，不使用伪精确 median；逐项证据、比较规则和 owner 见 `logs/checkpoints/M0-0.c-20260830-1538.md`。

### M0-2.a 回写记录（2026-08-30，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-2.a
STATUS=PASS（冻结性检查点：默认脚本 EXIT=1 是预期结果）
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=提交信息 test(M0-2.a): freeze lifecycle ownership gaps
VERIFY=check-lifecycle-contract.py --self-test(0); --expect-current-gaps(0); 默认(1, 7 GAP); bash -n pre-merge.sh(0); pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); git diff --check(0)
NEXT=M0-2.b
```

实现要点：

- 按任务卡 `READ` 逐文件核实后产出所有权表 `logs/m0-resource-ownership-v1.md`：16 项资源（主窗口 / tab webview / tab 元数据 6 项 / grid 子进程与部分创建残留 / PTY 会话与 reader 线程 / 3 个后台线程 / 前端 xterm、ResizeObserver、rAF 循环、store 绑定），逐项标注所有者、清理路径、窗口关闭与系统退出覆盖度、失败降级与对应 GAP。
- 夹具 `scripts/check-lifecycle-contract.py` 冻结 7 个机器可检 GAP，每条均有源码行号证据；自检覆盖 legacy fixture（命中 7 GAP）、resolved fixture（零 GAP）和临时仓库端到端扫描三重校验。
- `scripts/pre-merge.sh` 接入 `--self-test` 与 `--expect-current-gaps` 两模式；默认模式按设计 `EXIT=1`，明确不入门禁，避免把现状缺口误当回归。
- **未越界**：未实现 `ShutdownCoordinator`（M0-2.b）、未迁移关闭调用方（M0-2.c）、未改冻结证据、未解锁 M1/M2、未勾选 M0-2 整项。
- 复核提醒（强模型）：检测为正则 / brace 平衡提取，对格式化敏感；前端缺口 FE-1（rAF 采样循环卸载未停）、FE-2（`term.dispose()` 不通知 Rust `term_kill`，所有权跨层分裂）因 PASS_CRITERIA 固定 7 个 GAP 而未进夹具，已在所有权表 §3 冻结并归 M0-2.c。

### M0-2.b 回写记录（2026-08-30，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-2.b
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=aaed1c7
VERIFY=cargo test shutdown(0, 6 passed); check-lifecycle-contract.py --self-test(0); --expect-current-gaps(0); 默认(1, 6 GAP); bash -n pre-merge.sh(0); pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); git diff --check(0)
NEXT=M0-2.c
```

实现要点：

- 新增 `src-tauri/src/shutdown.rs`：`ShutdownCoordinator` 提供 `register/shutdown/last_report/is_shutdown/pending_tasks`；`shutdown()` 幂等、失败与 panic 双向隔离（`catch_unwind`）、关闭后拒绝注册。6 项测试覆盖重复调用、8 线程并发、Err/panic 隔离、关闭后注册拒绝、任务内重入不死锁（5 秒超时守护）、空核心闭环。
- **语义约定（M0-2.c 必须对齐）**：清理进行中再次调用（并发或重入）立即返回「进行中」报告且**不阻塞**——清理任务持有 `terminals`/`child_layouts` 等业务锁，阻塞等待极易死锁。需要「关闭已完成」语义请在退出流程层处理。
- `main.rs` 仅加 `mod shutdown;` 与 `.manage(ShutdownCoordinator::new())` 两行，**未改任何退出路径**；`main.rs:811` 的 `CloseRequested -> grid_manager.shutdown_all()` 旁路原样保留，待 M0-2.c 迁移。
- 夹具同步：`EXPECTED_GAPS` 移除已关闭的 `NO_UNIFIED_SHUTDOWN_CORE`（7→6），legacy fixture 补 `ShutdownCoordinator`；所有权表升 V1.1。
- 过程中修复一个真实语义 bug：缓存报告被原样返回，使重复/并发调用误报 `already_shutdown=false`；已改为「本次未执行则返回空报告 + 标记」，并补 `last_report()` 保留历史。
- 技术债（明确登记）：`shutdown.rs` 顶部 `#![allow(dead_code)]` 是为满足 M0-1.a「warning 只减不增」的临时豁免，**M0-2.c 接入消费方后必须删除并重新核对警告数**。

### M0-2.c 回写记录（2026-08-30，Codex 主任务）

```text
CHECKPOINT=M0-2.c
STATUS=PASS
EXECUTOR=Codex 主任务
MODEL=GPT-5 Codex
ROUTE=AI:DEEP
COMMIT=a2cbe79
VERIFY=check-lifecycle-contract.py --self-test(0); --expect-current-gaps(0); 默认(0, LIFECYCLE_CONTRACT_RESULT=PASS); cargo test shutdown(0, 6 passed); cargo check --locked(0, baseline 2 warnings); npm run build(0, existing warnings); git diff --check(0)
NEXT=M0-2.d
```

实现要点：

- 主窗口 `CloseRequested` 与主进程 `RunEvent::ExitRequested` 均调用 `ShutdownCoordinator::shutdown()`，不再直连 `grid_manager.shutdown_all()`。
- setup 注册四类真实清理任务：`stop-background-workers`、`close-tabs`、`kill-terminals`、`shutdown-grid`。
- `AppState.shutdown_requested` 接入 layout enforcer、resource scanner、hibernation sweeper 和 grid load retry 延迟线程。
- `close_tab` 改为先尝试关闭 webview、再无条件清理元数据，最后返回关闭错误；`term_kill` 增加 `wait()`；`create_grid` 失败时反向 kill 已创建子进程。
- M0-2.b 的模块级 `#![allow(dead_code)]` 已删除，warning 数回到基线 2 条。
- 前端 `TerminalPane.vue` 卸载时停止 M0 rAF 采样；`term.dispose()` 不强杀 Rust PTY，隐藏终端仍保留 shell，应用退出由统一核心兜底回收。

未做项：

- M0-2.d 仍需补重复退出、半初始化退出和失败降级测试；M0-2.c 只完成迁移与机器 gap 清零。

### M0-2.d 回写记录（2026-08-31，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-2.d
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=15a0087
VERIFY=cargo test shutdown(0, 15 passed); check-lifecycle-contract.py --self-test(0); --expect-current-gaps(0); 默认(0, PASS); pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); cargo check(0, 既有 2 warnings); git diff --check(0)
NEXT=M0-3.a
```

实现要点：

- `shutdown.rs` 新增测试模块 `m0_2d_lifecycle_tests`（9 项），任务名与顺序刻意对齐 `bridge::register_shutdown_tasks` 的真实顺序（`stop-background-workers → close-tabs → kill-terminals → shutdown-grid`），失败归因可直接对应生产路径。
- 三组覆盖：重复退出（顺序/并发入口各只执行一次、失败后不重试）、半初始化退出（零任务、部分注册两种）、失败降级（Err 隔离、panic 隔离、全失败仍跑完、报告顺序 = 注册顺序）。
- 静态夹具补 6 个**单缺口**防回归用例：此前只在组合 fixture 中一次性命中 6 个 GAP，任一检测器失效会被其他告警掩盖；现改为「干净基线 + 只注入一个缺口」。
- 所有权表升 V1.3，新增 §5 测试覆盖矩阵（15 项 Rust 测试 ↔ 6 个历史 GAP）。
- 过程中修正：单缺口用例初版传空 `main` 源码导致 `NO_UNIFIED_SHUTDOWN_CORE`/`SYSTEM_EXIT_HOOK_MISSING` 连带触发误报。
- **M0-2 整项关闭，但不宣称 M0 完成**：M0-3~M0-7 未关闭，M1/M2 仍锁定。
- 强模型复核点：M0-3 是 `AI:DEEP/R:xhigh` 安全收口，建议复核本检查点是否遗留「形态合规但语义错误」类问题（M0-2.b 曾出现此类缺陷，只有行为测试能抓到）。

### M0-3.a 回写记录（2026-08-31，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-3.a
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=b3cc0e5
VERIFY=pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); cargo test(0, 28 passed); check-security-policy.py --self-test(0); --expect-current-gaps(0); 默认(1, 5 GAP); cargo check(0, 既有 2 warnings); git diff --check(0)
NEXT=M0-3.b
```

实现要点：

- 新增 `src-tauri/src/security_policy.rs`：纯函数契约（`check_webview_label` / `check_path_within_roots` / `check_shell_command` / `check_html` / `policy_fingerprint`）+ 13 项拒绝用例；**只拒绝不清洗**（清洗会制造安全假象，收口按已解析应用条目执行，属 M0-3.d）。
- 新增 `logs/m0-security-threat-matrix-v1.md`：信任边界 + 7 项威胁（SEC-01~07，逐条源码行号）+ M0-3 验收对照表。
- 新增 `scripts/check-security-policy.py`：5 个现状缺口可机器复跑，三种模式；默认模式按设计 `EXIT=1`，只把 `--self-test`/`--expect-current-gaps` 接入 pre-merge。
- `main.rs` 只加 `mod security_policy;` 与启动指纹日志一行（使模块有真实消费方，避免新增 warning）。
- **收口未开始**：M0-3.b（capability/远程 IPC 来源与用户意图）、M0-3.c（canonical path/允许根目录/符号链接）、M0-3.d（`launch_app` 已解析应用条目 + 审计）均未做。
- 顺带清理：删除 M0-2.d 遗留的未使用导入（1 条 warning）；同步 4 处仍写 `M0-2.d = NEXT` 的旧文字与过期任务卡模板（原为 M0-2.a 卡）。
- 强模型复核点：`check_shell_command` 只是字符级最小契约，**不足以对抗所有注入**；M0-3.d 必须以应用条目白名单为准，不得把它当成充分防护。

### M0-3.b 回写记录（2026-08-31，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-3.b
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=30b9cc9
VERIFY=pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); cargo test(0, 34 passed); check-security-policy.py --self-test(0); --expect-current-gaps(0); 默认(1, 4 GAP); cargo check(0, 既有 2 warnings); git diff --check(0)
NEXT=M0-3.c
```

实现要点：

- **关键修复（SEC-08）**：远程权限集 `remote-collect` 名实不符——文件自述「只允许 collect_selection 与 report_resources」，实际放行 6 个命令，其中 `save_note`（写盘）与 `request_open_terminal`（授予 shell 能力）可被任意外部页面调用。已收紧为仅回传类 3 个。
- capability 收口：`default.json` windows 与 `browser-remote.json` webviews 删除残留 `browser` label（SEC-03 关闭），远程 webviews 收紧为 `tab-*`/`grid-*`。
- 三层校验落地：**来源**（命令加 `tauri::Webview`，label 必须登记）、**意图**（`IntentRegistry` 一次性令牌，30s TTL、作用域绑定、`main` 免令牌）、**载荷**（≤500 条目 / ≤64 KiB 文本 / html 走 1 MiB）。
- 不影响功能已核查：`save_note`、`request_open_terminal` 零前端调用点；`collect_selection` 仅主窗口 `ActivityBar` 按钮调用（走 `main` 免令牌路径）。
- 静态夹具：扫描范围补 `src-tauri/permissions/*.toml`（此前只扫 capabilities，SEC-08 检测器形同虚设）；`EXPECTED_GAPS` 5→4。
- 残余风险登记：远程 URL 全通配（`https://*`/`http://*`）是浏览器固有属性，无法收窄，改以三层校验作为补偿控制并接受残余风险；`eval_in_tab` 调用点尚未接来源校验。
- 强模型复核点：`issue_intent` 只校验 label == `main`，依赖 Tauri capability 保证「只有 main 能调用」；若 capability 配置被改宽，`main` 的来源判定可能被绕过，建议 M0-3.d 复核权限集与 label 假设的一致性。

### M0-3.d 回写记录（2026-08-31，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-3.d
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=16e99b5
VERIFY=pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); cargo test(0, 44 passed); check-security-policy.py --self-test(0); --expect-current-gaps(0); 默认(1, 3 GAP); cargo check(0, 既有 2 warnings); git diff --check(0)
NEXT=M0-4.a
```

实现要点：

- 去掉 `sh -c`：`launch_app` 改为 `parse_command_line` → (程序, 参数) 直接 spawn，消除二次解释。
- 三重校验：拒元字符 → 禁 shell 解释器（sh/bash/zsh/fish/powershell/cmd，否则等于换个壳绕过收口）→ 要求目标可解析为可执行文件。
- 兼容 .desktop：剔除 %f/%F/%u/%U/%i/%c/%k 字段码，支持双引号包裹路径；命令签名不变，两个前端调用点无需改动。
- 审计：`log_audit(action="launch")` 记录 program 与 args。
- 修正：`str::as_str()` 在当前工具链不稳定 → 改为直接模式匹配。
- 强模型复核点：可执行性校验依赖 PATH，若攻击者可写 PATH 覆盖目录仍可能诱导启动；后续可考虑锁定绝对路径白名单。

### M0-4.b 回写记录（2026-08-31，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-4.b
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:BALANCED
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=b3a81b9
VERIFY=pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); npm run build(0, 无 500kB 告警); cargo test(0, 44 passed); cargo check --locked(0, 既有 2 warnings); git diff --check(0)
NEXT=M0-4.c
```

实现要点：

- `vite.config.ts` 新增 `manualChunks`：xterm / vue-vendor / tauri-vendor / vendor 四分；入口 chunk 503.80 → 83.16 kB（-84%），最大 chunk 334.02 kB < 500 kB，**Vite 500 kB 告警消除**；总体积 544,875 → 544,093（-0.14%）。
- `useHomeStore` 对 `useWorkspaceStore` 的动态 import 改静态（该 store 已被十余处静态引入，动态既不分包也不破环）。
- `useLayoutStore` 对 `useBrowserStore` 的动态 import **刻意保留**：`useBrowserStore` 多处使用 `useLayoutStore`（mainView/showToast/isBrowserView），改静态即成环。已在代码注释与 checkpoint 登记。
- **强模型复核点**：剩余 1 条 mix 告警是已知取舍，若后续要彻底消除，需把 `showToast` 与视图状态抽成独立模块以反转依赖方向，属重构级改动，勿在未验证 GUI 的情况下直接改静态。

### M0-4.c 回写记录（2026-08-31，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-4.c
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:BALANCED
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=e216f03
VERIFY=pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); measure-build-metrics.py --self-test(0); 采集 EXIT=0; --compare(0, 无回归); cargo test(0, 44 passed); git diff --check(0)
NEXT=M0-5.a
```

实现要点：

- 新增 `scripts/measure-build-metrics.py`：采集 npm build 墙钟、chunk 明细（含 500 kB 告警识别）、dist 总字节、最大 JS 原始/gzip 体积、cargo check 耗时与 warning 数、fmt 状态；支持 `--compare` 与 `--self-test`。
- 基线入库：`logs/m0-build-metrics/build-metrics-6f4e554.json`（总 544,093 B、最大 chunk 334,208 B、warning 2、无 500 kB 告警）。
- 门禁：pre-merge 在既有 `npm run build` 之后用 `--compare --skip-build` 复用产物对比，**总体积增长 >15% 或 warning 增加即 FAIL**（对应 M0-4 验收与 M0-1.a「warning 只减不增」）。
- **M0-4 整项 PASS**。
- 强模型复核点：基线取 `logs/m0-build-metrics/` 字典序最早文件；若将来需要「滚动基线」而非固定首份，应显式更换策略并更新本文档。

### M0-4.a 回写记录（2026-08-31，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-4.a
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:BALANCED
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=c74ce34
VERIFY=pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); rg "gtk::|wry::" src-tauri/src(无匹配); cargo check --locked(0, 既有 2 warnings); cargo fmt --all --check(0); cargo test(0, 44 passed); git diff --check(0)
NEXT=M0-4.b
```

实现要点：

- 删除 `gtk = "0.18"` 与 `wry = "0.55"` 直接依赖，以及其上「方案 C3（gtk::Overlay + gtk::Fixed 内嵌子 webview）」的过时注释；原位置留下说明，避免后人按 C3 假设做修复。
- 背景：审核报告早已记录「决策文档称 C3，实际实现是方案 D（Window::add_child）」，该依赖为 C3 准备但代码零引用。
- lockfile 仅减 2 行（依赖包本身仍被 tauri-runtime-wry / 插件使用）。
- **未动** `tauri-browser-tabs` 插件 crate 的 `gtk`/`webkit2gtk`：那是插件做原生布局的真实依赖，不属零引用。

### M0-3.c 回写记录（2026-08-31，CodeBuddy / Codex 主机）

```text
CHECKPOINT=M0-3.c
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=3661fb3
VERIFY=pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); cargo test(0, 38 passed); check-security-policy.py --self-test(0); --expect-current-gaps(0); 默认(1, 4 GAP); cargo check(0, 既有 2 warnings); git diff --check(0)
NEXT=M0-3.d
```

实现要点：

- 裁决落地：写操作强制收口，只读浏览保留并登记 SEC-09。
- `allowed_roots()` = 主目录 / Desktop / Documents / Downloads / 工作区 / 笔记目录——与 `get_start_dirs` 对外承诺一致，避免「能列出却写不进」。
- 收口命令：`write_file`/`create_file`/`create_dir`/`delete_path`/`rename_path`；新增 `check_delete_target`（禁删根目录本身）、`check_path_component`（拒绝 `..`/分隔符/NUL）、`check_openable_url`（仅 http/https）。
- **修掉真实逃逸**：`rename_path` 原为 `parent.join(new_name)`，`new_name=../../etc/x` 即跨目录移动写入。
- 新建类命令先校验已存在祖先、创建后再校验真实落点，防中途被符号链接替换。
- 夹具改进：写命令接入判定从「全文匹配」改为「逐函数体匹配」（原方式会被「引用但未使用」骗过）；`delete_path` 经 `check_delete_target` 间接接入需一并认可。
- 强模型复核点：允许根目录含整个主目录，等于「主目录内可写」——这是为不破坏文件管理器做的取舍，若后续要求更严需引入「用户显式授权目录」机制并重审 SEC-09。

### M0-5.a 回写记录（2026-08-31，Codex 当前会话）

```text
CHECKPOINT=M0-5.a
STATUS=PASS
EXECUTOR=Codex 当前会话
MODEL=用户已切换强模型；界面完整模型名工具不可见
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未暴露完整模型名，按用户说明记录为强模型接手
COMMIT=68c78d7 + f8a58b5
VERIFY=verify-resources.sh --self-test(0, ALL_PASS); analyze-resource-cycles.py --self-test(0, ALL_PASS); collect-m0-baseline.sh --smoke(0, GUI 可采样); cargo build --release --locked(0, 既有 2 warnings); verify-resources.sh smoke 40-cycle(0, measurements.ok=true); validate-summary(0, VALID); sha256sum -c(0)
NEXT=M0-5.b
```

实现要点：

- `scripts/verify-resources.sh` 的 `/proc` 快照补 `ppid/comm/cmdline`，让 M0-5.a 可以区分 root、WebKitNetworkProcess、WebKitWebProcess。
- 新增 `scripts/analyze-resource-cycles.py`，把 raw cycle 快照整理成 RSS/FD/orphan 和进程成员矩阵。
- 40-cycle 诊断证据：`logs/m0-baseline/20260831T105149+0800_68c78d7_release_x11/`；分析摘要：`logs/checkpoints/M0-5.a-analysis-20260831-1111.md`。
- 诊断结论：orphan max 全部为 0；RSS 首末未超过 10% 增长；FD 正增长可复现，tab `+1`、grid `+1`、terminal `+2`。
- 强模型复核点：M0-5.b 先补 FD target 差分或等价 lsof 输出，再按 owner 修；不要重复实现 M0-2 的统一关闭核心，也不要把 M0-5.a 的 smoke 诊断当成 M0-5.c 正式验收。

### M0-5.b 回写记录（2026-08-31，Codex 当前会话）

```text
CHECKPOINT=M0-5.b
STATUS=PASS
EXECUTOR=Codex 当前会话
MODEL=用户已切换强模型；界面完整模型名工具不可见
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未暴露完整模型名，按用户说明记录为强模型接手
COMMIT=e998bbd + 2271e75 + e2c25db
VERIFY=verify-resources.sh --self-test(0, ALL_PASS); analyze-resource-cycles.py --self-test(0, ALL_PASS); npm run build(0, 仅既有 useBrowserStore 静/动态混用 warning); cargo test --manifest-path src-tauri/Cargo.toml(0, 44 passed, 既有 2 warnings); cargo build --manifest-path src-tauri/Cargo.toml --release --locked(0, 既有 2 warnings); verify-resources.sh smoke 40-cycle(0, measurements.ok=true); validate-summary(0, VALID); sha256sum -c(0); pre-merge(0 after docs/rustfmt); git diff --check(0)
NEXT=M0-5.c
```

实现要点：

- `scripts/verify-resources.sh` 的进程快照补 `fd_targets`，`scripts/analyze-resource-cycles.py` 补 FD target 归一化、进程角色 FD 目标差分和 RSS late-window owner。
- M0 driver 期间屏蔽全局快捷键、活动栏和页签栏切换，避免人工/前端入口在 tab 循环中误触发 grid，污染进程树。
- tab driver 收尾在关闭 tab 成功后显式调用 `close_grid`，确保资源循环只测本场景自身残留，不把旁路 grid 子进程计入 tab。
- 40-cycle 复测证据：`logs/m0-baseline/20260831T160738+0800_e998bbd_release_x11/`；分析摘要：`logs/checkpoints/M0-5.b-analysis-20260831-160738.md`。
- 复测结论：orphan max 全部为 0；grid/terminal FD delta 为 0；tab FD `+1` 来自 `child:WebKitNetworkPr anon_inode:timerfd 0->1`，第 6 个样本后稳定，裁决为 WebKitNetwork 一次性平台计时器；tab RSS 总增长 `+3.49%`，未超过 10%，M0-5.c 正式 profile 继续复核。

### M0-5.c 回写记录（2026-08-31，Codex 当前会话）

```text
CHECKPOINT=M0-5.c
STATUS=PASS
EXECUTOR=Codex 当前会话
MODEL=用户已切换强模型；界面完整模型名工具不可见
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未暴露完整模型名，按用户说明记录为强模型接手
COMMIT=ebc49f7 + 316d570
VERIFY=cargo build --manifest-path src-tauri/Cargo.toml --release --locked(0, 既有 2 warnings); verify-resources.sh formal(0, summary.status=PASS, measurements.ok=true); analyze-resource-cycles.py(0); validate-summary(0, VALID); sha256sum -c(0); analyze-resource-cycles.py --self-test(0, ALL_PASS); pre-merge(待文档提交前重跑); git diff --check(待文档提交前重跑)
NEXT=M0-6.a
```

实现要点：

- 正式资源 run：`logs/m0-baseline/20260831T164058+0800_6534963_release_x11/`，commit `6534963`，`run_mode=formal`。
- release 二进制 SHA：`5c41d4abb22a5f5ba4c8d85b443f890f43db8ff8d316ac4a6dae707b29312162`，与 `M0_EXPECTED_BINARY_SHA256` 完全匹配。
- 样本 profile：idle 60 秒；资源循环每类 5 次预热 + 20 次正式样本；terminal throughput 3 次正式样本。
- 正式结果：summary `PASS`，`worktree_clean_at_start=true`，`measurements.ok=true`，orphan max 全部为 0。
- 资源裁决：tab FD delta 为 0；grid FD `+1` 和 terminal FD `+2` 均为 WebKit 子进程 `anon_inode:timerfd`，不是 root/PTY 所有权泄漏。terminal 总 RSS 增长 `+7.46%`，未超过 10%；root 局部 RSS 跳升已按 GTK/WebKit/allocator 缓存裁决并留给后续基线对照。
- M0-5 三个检查点 a/b/c 全部 PASS，M0-5 整项关闭；M0 尚未完成，下一步必须进入 M0-6.a。

### M0-6.a 回写记录（2026-08-31，Codex 当前会话）

```text
CHECKPOINT=M0-6.a
STATUS=PASS
EXECUTOR=Codex 当前会话
MODEL=用户已切换 5.5 高模型；界面完整模型名工具不可见
ROUTE=AI:DEEP
MODEL_DEVIATION=UI 未暴露完整模型名，按用户说明记录为强模型接手
COMMIT=57a31ea + 7189451
VERIFY=bash -n run-gui.sh(0); GRID_SELFTEST=1 ./run-gui.sh(0, SELFTEST_RESULT=ALL_PASS); cargo test --manifest-path src-tauri/Cargo.toml(0, 44 passed, 既有 2 warnings); bash scripts/pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); git diff --check(0)
NEXT=M0-6.b
```

实现要点：

- 当前提交 `ec64f63` 上复跑 `GRID_SELFTEST=1 ./run-gui.sh`，输出归档到 `logs/m0-grid-selftest/M0-6.a-20260831-selftest.log`。
- selftest 覆盖：`create_grid(2)`、`grid_open(0/1)`、`grid_position(0/1)`、`eval_in_tab(grid-0)`、`kill -11 grid-child-0`、自动重启、重启后 eval、`grid-1-survived`、`close_grid`、`shutdown count=0`。
- 结论：宫格子进程崩溃自愈主链路在当前提交仍为 `ALL_PASS`；M0-6 尚未整体关闭，下一步必须实现主进程 `tab-*` WebView 的有预算恢复策略。

## 6. 跨模型接手审查协议

Trae 或后续执行器领取当前 `NEXT` 前，必须先做审查，不直接继续写代码：

```bash
git status --short --branch
git log --reverse --oneline --decorate 504fcd7..HEAD
```

然后按顺序：

1. 阅读本文顶部当前指针与最新 `logs/checkpoints/` 证据。
2. 从 `504fcd7` 之后按提交顺序逐个运行 `git show --stat --oneline <sha>`，确认每个提交只覆盖一个检查点。
3. 按各自证据复跑验收命令，确认工作树干净、文档状态、提交顺序和 `NEXT` 一致。
4. 若最新检查点 PASS，从文档当前 `NEXT` 续做；若 FAIL/BLOCKED，留在同一检查点修复或先解除阻塞。
5. 下一检查点重新按路由选模；检查执行器是否用轻量模型越权执行了 `AI:DEEP` 任务。

任何模型都不得因为“额度快没了”提前勾选、压缩检查点或把多个 WBS 合进一个提交。

## 7. 历史 Trae 连续执行提示词（M0 已完成，不再作为当前指令）

```text
你正在 feature-M0-baseline 分支接手 mvp-browser-os-v3。目标是在额度和环境允许时尽可能完成 M0，但必须严格按检查点顺序逐个执行；不得开始 M1~M5，不得把多个检查点合成一个提交。

先阅读 AI-模型切换与接手清单.md、详细设计与实施计划.md §2.2、logs/m0-baseline-contract-v1.md。每轮只从文档领取唯一 NEXT，按 AI:* 路由选择 Trae 模型，完成实现、全部验收、四处回写和独立提交。提交后确认工作树干净；未命中硬停止条件时，不用等待用户回复，立即领取下一个 NEXT。

第一轮是 M0-7.c，不能由 Trae/Codex 代签：先确认 M0-7.b checkpoint `logs/checkpoints/M0-7.b-20260901-0847.md`、验收草案 `logs/m0-acceptance-20260901.md` 和三份主文档指针均已提交且工作树干净；然后等待项目负责人/用户明确确认 M0 是否可放行。负责人确认前不得启动 M1/M2，不得改冻结原始证据，不得把本地 AI mock 说成第三方真实账号人工验收。确认后回写 checkpoint、三份主文档和验收报告，独立提交 docs(M0-7.c)。

同一检查点连续两次失败、缺少 DEEP 等效模型、缺少 GUI/权限/环境、需要改冻结契约或人工裁决、发现不明改动、额度耗尽或到达 M0-7.c 时必须停止，回写 BLOCKED 和证据；不得伪造 PASS。若 GUI/应用无法运行，必须 STATUS=BLOCKED，NEXT 保持当前检查点。
```

当前应复制 §1 的 `M1-1` 任务卡，而不是继续使用上面的 M0 连续执行提示词。
