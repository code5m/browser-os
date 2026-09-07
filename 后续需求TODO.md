> A0 update 2026-09-07 14:30 CST: W8 focused validation passed after A0 fixes (CredentialLeak redaction, cargo fmt, A3 patch whitespace). Build metrics accepted at 21.07% <= 22%, cargo warnings unchanged. NEXT=`M5-W9` runtime-free polish and final verification; no MCP/plugin/skill runtime expansion.
> A0 update 2026-09-07 10:05 CST: A3 W7 MCP read-only bridge focused checks PASS except old `check-mcp-policy.py --expect-pending` phase debt; NEXT=`M5-W8` full-lane follow-up, A3 assigned policy-fix-only and all lanes may continue by board.
> A0 update 2026-09-07 09:45 CST: W7 non-A3 focused checks PASS; NEXT=`M5-W8` excluding A3. A3 local commit exists but is held out of this wave; do not dispatch A3 until A0 resolves history/push boundary.
# 后续需求 TODO 列表
> A0 update 2026-09-07 00:50 CST: `origin/master` pushed to `5f92ece`; NEXT=`M5-W7`. W7 opens only A3 (MCP read-only command bridge) and A5 (Agent/Skill read-only command bridge) for product code; all other lanes remain docs/review/support.
> A0 update 2026-09-06 19:25 CST: `origin/master` pushed to `4b438ef`; NEXT=`M5-W7`. W6 opens only A8 (M5-9 graph UI pure logic/panel shell) and A9 (M5-10/11 plugin manifest/lifecycle policy slice) for product code; all other lanes remain docs/review/support.
> A0 update 2026-09-06 18:35 CST: `origin/master` pushed to `1610939`; NEXT=`M5-W6`. W5 opens only A6 (M5-6 Agent/Skill UI pure logic/panel shell) and A7 (M5-7/8 graph model/store policy slice) for product code; all other lanes remain docs/review/support.
> A0 update 2026-09-06 17:55 CST: `origin/master` pushed to `f8f1f49`; NEXT=`M5-W5`. W4 opens only A4 (M5-3 A2A/agent memory KV contract) and A5 (M5-4/5 Agent/Skill domain + policy shell) for product code; all other lanes remain docs/review/support.
> A0 update 2026-09-06 17:10 CST: `origin/master` pushed to `712a14c`; NEXT=`M5-W4`. W3 opens only A2 (M5-1.b core extraction) and A3 (M5-2 MCP registry/policy shell) for product code; A4-A11 remain docs/review/support.

## 0. 工作目录身份与自救规则（所有智能体先读）

本仓库多智能体并行时必须先确认自己在哪个 checkout。不要只相信聊天提示词；以仓库内身份文件和当前 `pwd` 为准。

并行开发指挥板：`PARALLEL_COMMAND_BOARD.md`。所有智能体领取 M4/M5 任务前必须先读该文件，按 Lane 范围、Dispatch Waves、合并顺序和硬停止规则执行；当前只应直接启动 Wave 1（A1/A2/A6/A9/A10），只有 A0 集成总控可以向 `master` 提交并推送。

2026-09-05 23:30 A0 覆盖裁决：若 `logs/checkpoints/M4-20260905-2225.md` 与 `PARALLEL_COMMAND_BOARD.md` 的 Lane 归属冲突，一律以 `PARALLEL_COMMAND_BOARD.md` 为准。当前口径：A3 负责 M4-2 PoolKind 与生产安全策略（含 SQL 分类器和 `security_policy.rs`），A4 负责 M4-3 数据库命令层；不要再按旧的 `M4-2.s -> A4` 施工。

- 权威主目录：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
- 主目录身份文件：`.workspace-identity`、`WORKSPACE_IDENTITY.md`
- Codex 临时 worktree 示例：`/home/ainfinit/.codex/worktrees/*/mvp-browser-os-v3`
- 临时 worktree 只用于隔离实验或明确指定的任务，不用于最终集成、提交和推送。
- 需要统一代码、最终验证、拆分提交、推送远程时，必须回到权威主目录。

所有智能体开工前必须运行：

```bash
cat .workspace-identity
pwd
git status --short --branch
git log --oneline -12
```

硬停止：

- `pwd` 与任务指定 `WORKDIR` 不一致，立即停止并报告。
- 任务要求主目录/BackV3/推送，但当前在 `/home/ainfinit/.codex/worktrees/*`，立即停止并切换到权威主目录。
- 任务要求 Codex worktree，但当前在权威主目录，立即停止并切换到指定 worktree。
- 工作树不干净且脏文件来源不属于自己，立即停止并报告，不要覆盖其它智能体的改动。
- 准备 push 前发现未审核的跨任务改动，立即停止并要求先拆分/确认提交边界。

自救流程：先读当前目录的 `WORKSPACE_IDENTITY.md`，再对照任务里的 `WORKDIR`、分支、NEXT 值；不一致时只做状态报告，不继续写代码。

> 本版变更：**M4 任务卡已展开（Lane A1，`AI:DEEP / R:high`，CodeBuddy 会话 Hy4 / 腾讯混元；纯文档展开，产品代码零改动，基线 `c9c7576`），NEXT=`M4-1.a`（Lane A2 认领；`M4-5.a` 由 Lane A6 并行认领）**。展开卡 `logs/checkpoints/M4-20260905-2225.md`：①**现状实测 14 项**——`sqlx`/`rusqlite`/`mysql`/`postgres`/`serde_yaml`/`tokio-cron-scheduler` 在 `Cargo.toml` 与 `Cargo.lock` 均为零；`tokio 1.53.1` 只是 tauri 的传递依赖（`use tokio::...` 当前不可编译）；`build.rs` 无代码生成；命令 105 / ACL 108 且末条仍是 `list_artifact_images`；`ShutdownCoordinator` 已注册 6 任务；持久化全部 JSON 原子写。②**对 #6/#11 需求原文的 7 处事实修正**——F-1 #11 切入点写「`tokio::spawn` 启动」但 tokio 非直接依赖，须 M4-1.a 裁决；F-2 `详细设计 §6` 的「YAML 扩库 + build.rs 生成 `SupportedDb`」建立在 dbx 五成员 workspace 之上、本项目单 crate，须 M4-1.b 重新裁决；F-3 `db_connect/db_query/db_disconnect` 只作占位登记；F-4 `tasks.yaml` 统一为 `tasks.json`；F-5 `TaskKind` 首期只做 `Script`/`Command`，「调用 HTML 工具」**不做**（`list_tools` 是只读清单、工具无 headless 执行入口，强加即死代码，同 M2-3 拒绝空结构口径）；F-6「导出成果库」列 M4-4 显式范围项；F-7 db 凭据键须用 `db:<conn_id>` 命名空间（`KeyringStore` 按精确字符串取键，与 git `repo_id` 碰撞会静默取回错误凭据）。③**依赖图（已按 A0 覆盖裁决更新）**：库链 `M4-1(A2) → M4-2(A3，含连接池/查询核心 + SQL 分类器 + 生产判定，落 security_policy.rs) → M4-3(A4，db_* 命令层) → M4-4(A5)`；调度链 `M4-5(A6) → M4-6+M4-7(A7) → M4-8(A8)`；**两链互不阻塞**。④**~~裁决 R-A1-1~~ 已被 A0 覆盖裁决（2026-09-05 23:30）推翻**：A1 曾把 SQL 风险分类切片拆为 `M4-2.s` 划归 A4；**A0 裁定：A3 = M4-2 全卡（含分类器、生产判定、`security_policy.rs`、连接池/查询核心），A4 = M4-3 命令层，`M4-2.s` 编号作废**。归属与顺序一律以最新 `PARALLEL_COMMAND_BOARD.md` 为准。⑤**冻结条款 F1~F12** 与 **30 张子卡**、测试矩阵 ID 段、6 个新夹具码位（`DB_*`/`DBUI_*`/`SCHED_*`/`SCHEDUI_*` + 两个 `.mjs`）已一次性预留。⑥挂账 O-A1-1~O-A1-7，其中 **O-A1-3 需 A0/负责人裁定**：M4 验收门禁第 2 条要求三库各有连接/只读查询/断开/失败用例，若无真实 MySQL/PG 实例则只能 `SKIPPED`；**不得伪造 PASS**。**A1 未提交、未 push**（只有 A0 向 `master` 提交并推送）。
> 上一版（保留）：**M2-6 整体裁定已签（`PASS_WITH_DEBT`，`AI:DEEP / R:high`，裁定者 CodeBuddy Hy4 / 腾讯混元）；两份独立复核报告的 Must Fix 已单开 `M2-6-fix1` 落地，NEXT=`M2-7.a 工具清单契约冻结（待认领）`**。整改（`logs/checkpoints/M2-6-REVIEW-VERDICT-20260905-1648.md`）：**P1-1** 补命令片段域运行时来源校验单测（远程 webview 调 `snippet_*` + `run_command` 全拒）；**P1-2 书面裁定不上程序名黑名单**（`/bin/bash`/`env`/`python3 -c` 均可绕过），最小收敛为定义期拒绝 `argv[0]` 占位符（`ARGV_PROGRAM_PLACEHOLDER`）；**P1-3** 执行层 `script_runner.rs` 纳入命令领域夹具扫描 + 4 条不变量（坏样本 11→15）；**F-1** 新建态 `enabled` 贯通，修掉启用勾选被静默覆盖；**F-2** 命令库面板与 `snippetUi.ts` 纳入 UI 夹具（13→16 码位）；**D-1** 关键路径串追平至 M2-7。UI 断言 26→36。复跑：`cargo test` 220/220、release 0 error、两套夹具自检全过、pre-merge ALL_PASS。**新增债务 D21**（运行记录无 `kind` 判别，命令片段运行以裸 UUID 出现在历史面板）/ **D22**（缺 `cmd.run.cancel` 与 `cmd.run.finish`）。命令片段库 a/b/c/d 四卡全交付，`CommandSnippet` 保持 `argv: Vec<String>` + 整元素 `{NAME}` 占位 + `dangerous` 显式字段；`snippet_*` 四命令与 `run_command` 复用 `script_runner` 内核（**不另建执行旁路**）；`cmd.*` 审计脱敏；前端分类/搜索/收藏/CRUD/启用/二次确认齐备。九项核查全成立、四条 P0 红线无一突破。复跑：`cargo test` 215/215、`npm run build` 0 error、release 构建 0 error、命令领域 policy 自检 11 坏样本全检出、UI 逻辑 26 断言、pre-merge ALL_PASS。**新增债务 D18**（片段 `argv[0]` 未过 `check_launch_target`，判非阻塞：与脚本库允许用户自写 bash 脚本同授信、红线针对应用侧拼接、名字黑名单可被绝对路径绕过；但属对 `M2-4.a §8.1` 的未申明偏离，须单开检查点裁定）/ **D19**（**本需求 #4 的「内置常用命令分类」只交付分类框架、未交付内置片段种子**，首次使用命令库为空；b 卡范围未含种子故非漏交付，登记后续卡 `M2-6.e`）/ **D20**（GUI 实点验收挂账，未伪造）。证据 `logs/checkpoints/M2-6-ACCEPT-20260905-1625.md`。
> 上一版（保留，M2-6.d 交付详情）：**M2-6.d 命令片段库前端面板已实现（`AI:DEEP / R:high`，Codex gpt-5.5），NEXT=`M2-6 整体裁定`**。新增 `snippetUi.ts`、`CommandSnippetPanel.vue`、`commands` 模块入口与 store 动作；支持分类/搜索/收藏、新增/编辑/删除、参数表单复用、dangerous 二次确认、启用开关，并通过 `ScriptRunDialog kind=\"command\"` 调用 `bridge.runCommand` 复用执行态。新增 `check-command-ui-logic.mjs` 26 断言并接入 pre-merge。GUI 实点验收挂账。证据 `logs/checkpoints/M2-6.d-20260905-1552.md`。
> 再上一版（保留）：M2-5 任务卡已展开为 a/b/c 三张子卡（`logs/checkpoints/M2-5-20260905-1301.md`，纯文档展开，未改产品代码），NEXT=`M2-5.c`（b 已交付：执行面板=真实运行+取消+输出流rAF合并+补 D15 审计+D14 节流；c 待裁定：script_runs_list 运行历史）**。拆卡：a 脚本库 CRUD UI（纯前端不接执行）/ b 执行面板（真实执行+输出流+取消+补 D15 审计与 D14 后端 30ms 节流）/ c 运行历史（新增后端 `script_runs_list`）。现状实测：前端零 Script 组件，但 `bridge.ts` 7 条命令与 `types.ts` 类型镜像已由 M2-3/M2-4 交付就位；缺口仅 UI 组件与两项未交付物（**事件监听封装缺失 / 运行历史读取命令缺失**）。对两份 mock 草案做 6 处事实修正（草案写于 M2-4 落地前，「不接执行用 mock」前提已失效）；四组分歧留 b 卡裁定。新增 `check-script-ui-policy.py`（7 默认码+6 pending 码位+变异防呆）。上一版：M2-4 整体裁定已签（`PASS_WITH_DEBT`，`AI:DEEP / R:high`，裁定者 CodeBuddy Hy4 / 腾讯混元），NEXT=`M2-5`。本裁定为**跨模型复核**（与 a/b 卡裁定者 Kimi K3、c/d/e 实现者 Codex gpt-5.5 均不同模型），同时完成 `M2-4.b-VERDICT §6` 遗留要求：`§3.1 路线 A`（argv 一律不加引号）与 `§3.2 路线 B`（piped + 只丢弃 drain，drain≠背压）均**确认维持**。`M2-4.a §8` 八组契约六组完全成立、两组部分成立，**四条 P0 红线无一突破**。新增债务 **D14**（`script-output` 未按 §8.5 做 30 ms 时间节流）/ **D15**（§8.7 审计缺 `script.run.finish` 与 `script.validate.reject`，`cancel` 缺 `cancelledBy`，属未申明的契约偏离）/ **D16**（R15 脚本文件被替换/删除未显式预检）/ **D17**（GUI 事件联调、R9 100 MB、R12 运行时链路无取证）。**新增后续检查点（与「评估废弃 `ScriptParam.raw`」同级）：`script.validate.reject` 审计须在 M2-5 前端执行面板上线前补齐**——一旦有 UI，用户可控参数即可触达 `build_argv`，缺失审计等于失去攻击尝试的可观测性。上一版：M2-4.e 移除 shell spawn 开放面已完成（`AI:DEEP / R:high`，Codex gpt-5.5），NEXT=`M2-4 整体裁定`。按 M2-4.a 裁定，移除 `src-tauri/capabilities/default.json` 中 `shell:allow-spawn args:true` 以及 `shell:allow-stdin-write`/`shell:allow-kill`，移除主进程与 grid child 两处 `tauri_plugin_shell::init()`，裁掉 Rust `tauri-plugin-shell` 与前端 `@tauri-apps/plugin-shell` 依赖；刷新 M2-2/M2-3 对 `package*.json` 的哈希基线。`check-script-exec-policy.py` 追加 e 卡三类默认违规码（capability / npm / Rust 插件回流），23 坏样本自检通过。验证：`cargo test` 198/198 PASS；`cargo build --release --locked` 0 error、2 warning 仍为既有 `grid_process.rs` 死代码；`npm run build` 0 error；三套相关 policy 全过；运行时 GUI 目视验收仍挂账。

> 文档角色：#1~#15 需求池、优先级与里程碑映射；不作为“已经做到哪里”的单独证明。
> 进度与验收 SSOT：`详细设计与实施计划.md`。
> 文档版本：V5.24。
> 更新时间：2026-09-05 22:25 CST。
> 本版变更：**M2-4.b 独立裁定已完成（`logs/checkpoints/M2-4.b-VERDICT-20260904-0705.md`，ADJUDICATED_WITH_REVISION），NEXT=`M2-4.b`（先修订展开卡再实现）**。裁定者 Kimi K3（**同会话同模型裁定，最弱独立性**，补偿=实测复跑+精读+反方检验）：§3.1 走路线 A（argv 不加引号，`M2-4.a §8.1` 末条作废）；**§3.2 推翻实现者倾向改走路线 B**（piped+只丢弃 drain，drain≠背压）；§3.3/§3.4/§3.5 确认。**新增后续检查点（与 M2-4.e 同级，b/c/d 稳定后单开）：评估废弃 `ScriptParam.raw`**——argv 模式下永无语义（`Command::arg()` 不经 shell，不存在单引号包裹概念），留着会形成「看似有用实则 no-op」的误导字段；废弃需改 `domain.rs` + `scripts.rs::validate_meta`（去 `INVALID_RAW_PARAM`）+ 重跑 M2-3 门禁，故不与进程组实现混在 b 卡。实现前须按裁定书 §6 修订展开卡；建议 b 卡实现后跨模型复核。上一版：M2-4.b 任务卡已展开（`logs/checkpoints/M2-4.b-20260904-0000.md`），NEXT=`M2-4.b`（可直接实现）**。b 卡 = 进程组与生命周期内核（`AI:DEEP/R:xhigh`）：`script_runner.rs`（setsid/killpg、超时软硬分层、轮询回收、平台降级、进程表、`build_argv`）+ `domain.rs` 落 `RunStatus`/`ScriptInterpreter::binary()`/`timeout_secs` 注释 300→60 + `check-script-exec-policy.py` 接 pre-merge 第 19 项。实测：`libc`/`uuid`/`chrono` 已就绪、`setsid`/`killpg`/`pre_exec` 全仓为零、既有两处 spawn 范式均不可复用（`bridge.rs:3499` 无进程组无超时无 wait；`grid_process.rs:677/743` 的 `child.kill()` 留孤儿）、门禁未锚定 `timeout` 故改注释安全。**5 处对冻结裁定书的澄清/微调**：①argv 模式一律不加引号（`raw` 为 no-op，加引号会把 `'` 字面量传进脚本，bug 级冲突，须裁定）；②b 卡 stdio 用 `null` 中间态（piped 不读会因 64KB 管道缓冲阻塞子进程），d 卡接 piped+reader；③`timeout_secs` 注释 300→60；④新增 `ScriptInterpreter::binary()` 落 `domain.rs`；⑤`RunStatus` 落 `domain.rs`。测试矩阵 B1~B13，**B7/B8/B9/B11/B12 为真实进程测试**（起真实 `sleep` + `/proc` 取证进程组回收，<15s），是 M2-4 首批机器可取的运行时证据。**纯文档展开，未改产品代码**。上一版：M2-4.a 执行安全契约冻结裁定书已落盘（`AI:DEEP / R:xhigh`，Kimi K3，`logs/checkpoints/M2-4.a-20260903-2233.md`），NEXT=`M2-4.b`。七组分歧逐组裁定（双层输出上限 4MB/256KB、全局默认超时 60s、并发同 id 禁+全局 8、只做 Unix、env 固定最小集、shell 权限归 e 卡、RunStatus=Succeeded+落盘 script-runs.json 上限 200）；冻结执行通道核心契约（argv 数组/cwd 锁定/setsid+killpg/背压/退出收口 kill-running-scripts/审计脱敏）。**未改任何产品代码**。b/c/d 在 a 卡冻结后逐张展开。上一版：M2-4 任务卡已展开为 a~e，NEXT=`M2-4.a`（`logs/checkpoints/M2-4-20260903-1659.md`）：M2-4 安全执行通道是项目最高风险面，拆为 a 执行安全契约冻结 / b 进程组与生命周期内核 / c 执行命令与校验接入 / d 输出背压·事件流·退出收口 / e（可选）移除 `shell:allow-spawn`；**本次只展开 a 卡**。实测：进程组能力全仓为零需新建（`libc` 已就绪）、既有 `child.kill()` 只杀直接子进程不得沿用、`ShutdownCoordinator` 已注册 5 个任务（草案「被阻塞」已解除）、**`shell:allow-spawn` 零调用点**。a 卡须裁决七组真实冲突（输出上限 4MB vs 256KB、超时 60s vs 300s、并发 8 vs 10、平台范围、env 策略、shell 权限处置、RunStatus 命名与落盘）。上一版：M2-3 整体裁定 PASS_WITH_DEBT（`AI:DEEP / R:high` Kimi K3）：五组冻结契约逐项裁定成立；两处「冻结裁定书微调」经独立重审接受；D12（命令端到端 IPC 未实测）、D13（「危险值被拒」运行时链路未打通）挂账，`can_delete` 返回 `InvalidId` 记 NON-BLOCKER；三个门禁冲突修复经复核均未降级既有门禁，新增变异防呆为正向改动。**独立性如实标注**：同模型裁定（弱于 M2-2 跨模型复核），用户明确指示，可被 Hy4/Codex 复核推翻或确认。证据 `logs/checkpoints/M2-3-ACCEPT-20260903-1645.md`。上一版：M2-3.b 已实现（PASS_WITH_DEBT）：新增 `src-tauri/src/scripts.rs` 纯函数层（20 个稳定错误码、定义期校验、九条 fail-closed 参数规则、脱敏三重保险）+ `domain.rs` 四个新类型（`ScriptInterpreter` 为枚举白名单）+ `workspace.rs` 脚本持久化（显式路径参数、原子写、路径三重校验、删除幂等）+ `bridge.rs` 四条命令 + 注册/ACL + TS 镜像 + `check-script-domain-policy.py`（14 违规码，1 好 + 17 坏，含变异防呆）接入 pre-merge 第 18 项；`cargo test` 178/178（新增 22 项含真实磁盘往返）、`cargo build --release` 0（warning 仍既有 2）、`npm run build` 0、总体积 +12.65%、pre-merge ALL_PASS、release 冒烟无 panic。两处对冻结裁定书的微调：`validate_param_value` 去掉无作用的 `raw` 参数；**未新增 `check-script-domain-logic.mjs`**（本卡无前端脚本逻辑，UI 属 M2-5，强加会成为无人消费的死代码）。**挂账**：端到端 IPC 往返与危险值运行时链路无 GUI 通道取证。**未签 M2-3 整体 PASS**。上一版：M2-3.a 冻结裁定书已落盘（`logs/checkpoints/M2-3.a-20260903-1604.md`，`AI:DEEP / R:high` Kimi K3）：五组分歧逐组裁定——字段口径（`params` + `ParamType` + `interpreter` 收紧为枚举 + `path` 相对文件名 + 新增字段 `#[serde(default)]`）、存储布局（`scripts.json` + `scripts/<id>.<ext>` 原子写；`ScriptRunRecord`/`RunStatus` 推迟 M2-4；既有 `save_repos`/`save_bookmarks` 非原子写属技术债不倒改）、危险参数 9 条规则落地为纯函数 `validate_param_value`（不复用/不修改 `check_shell_command`，元字符集合以其为下界扩展）、审计脱敏三重保险、M2-4/M2-5 边界。测试矩阵 T-scr-1~17、违规码 14 条。**M2-3 全程不做脚本执行**。上一版：M2-3 任务卡已展开为 a/b（`logs/checkpoints/M2-3-20260903-1556.md`）：实测确认 `ScriptMeta` 尚不存在、命令与 ACL 均 91 条、前端无脚本库面板、`shell:allow-spawn` 对 bash/sh/powershell 仍 `args: true` 全开；**修正两份前置草案的事实**——`ScriptMeta` 两套设计互不兼容（assist 版 `args`/prework 版 `params`+`ParamType`+`interpreter`），且 prework 的「`request_app_exit` 未落地」阻塞项**已解除**（`ShutdownCoordinator` 已落地并注册 6 个退出任务、支持 `register` 任意任务）。a 卡须裁决五组分歧；测试矩阵 T-scr-1~13 + 12 类违规码已定义；**M2-3 不做脚本执行**。上一版：
> 编写/裁决模型：Codex gpt-5.5 high；机械文档审计：`gpt-5.6-luna / low`；M2-2.b 实现：CodeBuddy 会话（腾讯 Hy4）；M2-2 整体裁定：CodeBuddy 会话（Kimi K3）；M2-3 整体裁定：CodeBuddy 会话（Kimi K3，同模型裁定）；**M2-4 整体裁定：CodeBuddy 会话（Hy4 / 腾讯混元，跨模型复核）**；M2-5 整体裁定：CodeBuddy 会话（MiniMax-M3，实现者自裁）；**M2-6 整体裁定：CodeBuddy 会话（Hy4 / 腾讯混元；独立于 a 卡实现者 MiniMax-M3 与 d 卡实现者 Codex gpt-5.5，b/c 卡未标实现模型）**。
> 本版变更：**M2-2 整体裁定 PASS_WITH_DEBT（`AI:DEEP / R:high`，Kimi K3 独立于实现者），NEXT=`M2-3`**：a/b 两卡全绿，七项冻结契约逐项裁定成立；R1（asset:// 运行时加载未实测）降级为 D10（scope 真实环境推导已覆盖 + asset:// 通道已有 AppPanel 图标加载的运行时先例 + 人工清单可复现）；R2（GUI 目视验收）为 D11；R3/R4/R5 可接受；三个门禁冲突修复经复核均未降级既有门禁。证据 `logs/checkpoints/M2-2-ACCEPT-20260903-1540.md`，复核输入包 `logs/checkpoints/M2-2-verdict-input-20260903-1502.md`。上一版：M2-2.b 画廊/灯箱/缩放 UI 已实现（PASS_WITH_DEBT）：后端新增 `workspace_images_dir` 只读命令（`check_invocation_source` + 审计只记 op 名）、`workspace::images_dir()`、`images::join_image_path()` 白名单拼接（fail-closed）；前端新增 `src/utils/imagePreview.ts` 纯逻辑层 + `useImagePreviewStore` + `shared/ImageGallery.vue` / `shared/ImageLightbox.vue`（成果库挂 `ArtifactPanel`、灯箱全局挂 `App.vue`）；字节通道 `convertFileSrc + asset://` 且**未扩 scope**、零新增 npm 依赖；新增 `check-image-preview-policy.py`（1 好 + 15 坏样本）与 `check-image-preview-logic.mjs`（97 断言，覆盖 T-prev-1~12）并接入 pre-merge 第 17 项；`cargo test` 156/156、`cargo build --release` 0（warning 仍为既有 2）、`npm run build` 0、总体积 +12.63%（≤15%）、pre-merge ALL_PASS、release 启动冒烟无 panic。**挂账**：asset:// 端到端加载与 GUI 目视验收未完成（无 GUI 自动化通道、真实数据目录尚无落盘图片），证据与人工验收清单见 `logs/checkpoints/M2-2.b-20260903-1454.md`。**未签 M2-2 整体 PASS**。上一版：M2-2.a 冻结裁定书已 PASS（`logs/checkpoints/M2-2.a-20260903-1352.md`）：七项冻结契约逐项结论，M2-2.b 按裁定书施工。再上一版：M2-2 任务卡已展开为 a（冻结）/b（实现）两张子卡，静态判断 workspace 图片目录已在现有 `assetProtocol.scope` 的 `$HOME/.local/share/**` 内无需扩 scope；证据 `logs/checkpoints/M2-2-20260903-1340.md`。再上一版：M2-1 图片领域与持久化已 PASS（`ImageRef`/`Artifact.images`/`save_image`，MIME 白名单 fail-closed + magic bytes 比对 + 10MB/50张/50MB/8000px 上限 + sha256 去重 + 原子写 + 删除联动，cargo test 151/151、pre-merge ALL_PASS，顺带修复 M1-ACCEPT 会话 id 校验挂账；运行时端到端联调挂账，证据 `logs/checkpoints/M2-1-20260903-0927.md`）。

---

## 当前执行结论（截至 2026-09-06 · M4 已合流推送 + M5-W1 已安排）

> **本版变更：A0 已将 M4 数据库 + 定时任务并行成果合流为 `a1a2061 feat(M4): integrate database and scheduler lanes` 并推送 `origin/master`。机器门禁：`cargo test` 329/329、`npm run build` PASS、`pre-merge` ALL_PASS；M4-9 红灯已处置。当前 NEXT=`M5-W3`：A1 展开 M5 任务卡，A2-A9 只做架构/契约 prework 文档，A10 安全复核，A11 验证矩阵；M5 产品代码仍锁定到 A0 签第一张实现卡。**


| 里程碑 | 状态 | 当前口径 |
|--------|------|----------|
| **M0 安全与稳定性基线** | **PASS，已验收** | M0-0~M0-7 全部关闭；验收报告 `OWNER_APPROVED` |
| M1 浏览器与版本控制 | **PASS，里程碑已验收（PASS_WITH_DEBT）** | `M1-0`~`M1-9` 全部 PASS；`M1-ACCEPT` 收口验收通过（无 BLOCKER；GUI/E2E 挂账 D1~D9 保留，汇总见 `logs/checkpoints/M1-ACCEPT-20260903-0843.md` §7） |
| M2 本地资产与执行 | **PASS（含 M2-6.e 内置片段种子）** | M2-1~M2-6.d 逐卡 PASS_WITH_DEBT；M2-7/8/9 均 PASS；M2-6.e 关闭 D19 |
| M3 终端增强 | **PASS_WITH_DEBT，整体裁定已签；M3.c 已交付（WBS M3-1~M3-4 全关闭）** | M3-1/2/3 已落地并通过裁定（证据 `logs/checkpoints/M3-REVIEW-VERDICT-20260905-2101.md` + `B-M3.a-terminal-pipeline-20260905-2035.md`）；M3.c = 临时历史 40 条 + resize 静默窗口（证据 `logs/checkpoints/M3.c-20260905-2202.md`，后端 `src-tauri/**` 零改动）；挂账 D23（GUI 实点）/ D24（M0-0.b 吞吐基线未重采）/ D25（`on_channel_dead` 未 wait）/ **D26**（历史按块计数，上界 40 × ≤64 KiB ≈ 2.5 MB，未按字符封顶） |
| **M4 数据与调度** | **PASS_WITH_DEBT（A0 已合流推送 `a1a2061`；机器门禁全绿，GUI/真实 MySQL/PG 验收挂账）** | 前置已满足，M4 已解锁。展开卡 `logs/checkpoints/M4-20260905-2225.md`：M4-1~M4-8 → 30 张子卡 + 依赖图 + 冻结条款 F1~F12 + 测试矩阵 ID 段 + 6 个新夹具码位。库链 `M4-1(A2)→M4-2(A3)→M4-3(A4)→M4-4(A5)`；调度链 `M4-5(A6)→M4-6+M4-7(A7)→M4-8(A8)`；两链互不阻塞，库链内部 A3 先于 A4。并行施工按 `PARALLEL_COMMAND_BOARD.md`，**只有 A0 可 push** |
| M5 协议与智能生态 | **M5-W1 已安排（A2 core boundary 首实现 + 其余 Lane docs/review）** | 仅 A2 可写 M5-1.a core boundary；MCP/Agent/Graph/Plugin 产品代码仍锁定 |

判定规则：`P0/P1/P2` 表示业务与风险优先级，`M0~M5` 表示执行顺序。M0 已放行后，后续需求仍必须按 `详细设计与实施计划.md` 展开检查点、逐点验收、独立提交。

M0 当前收口面共 8 项：完整基线、自动门禁、统一生命周期重构、统一安全边界重构、构建/依赖清理、资源生命周期闭环、崩溃与 9 项 GUI 回归、最终验收。任务定义、检查点和证据要求统一见 `详细设计与实施计划.md` §2。

## AI 下一任务领取队列（唯一入口）

AI 必须按 `详细设计与实施计划.md` §2.2 的**检查点关键路径**领取任务，不能只扫描最靠上的未完成 WBS。排序先看阻塞性和降低后续复杂度的收益，再看依赖，最后才在同级里把简单任务排前；模型标签及自动升级规则见详细计划的“AI 执行排序、模型路由与提交协议”。

当前关键路径：`M0-0(完成，六项 UNSTABLE 已裁决) -> M0-1(PASS) -> M0-2(PASS: a/b/c/d) -> M0-3(PASS: a/b/c/d) -> M0-4(PASS: a/b/c) -> M0-5(PASS: a/b/c) -> M0-6(PASS: a/b/c) -> M0-7.a(PASS) -> M0-7.b(PASS) -> M0-7.c(PASS) -> M1-0(PASS) -> M1-1(PASS) -> M1-2(PASS) -> M1-2-fix1(PASS) -> M1-3(PASS) -> M1-4(PASS) -> M1-5(PASS) -> M1-6.a(PASS, 契约冻结) -> M1-6.b(PASS, 写后端核心) -> M1-6.c(PASS, 复核) -> M1-6.d(PASS, push) -> M1-7(PASS, Git UI；GUI 目视验收挂账) -> M1-8(PASS, 请求拦截与瀑布；运行时端到端联调挂账) -> M1-9(PASS, 会话持久化与关闭协议；运行时端到端联调挂账) -> M1-ACCEPT(PASS_WITH_DEBT, M1 里程碑收口) -> M2-1(PASS, 图片领域与持久化；运行时端到端联调挂账) -> M2-2.a(PASS, 字节通道决策与契约冻结) -> M2-2.b(PASS_WITH_DEBT, 画廊/灯箱/缩放 UI 已实现) -> M2-2(PASS_WITH_DEBT, 整体裁定：D10 asset:// 运行时加载未实测 / D11 GUI 目视验收) -> M2-3.a(PASS, 脚本领域契约冻结) -> M2-3.b(PASS_WITH_DEBT, 领域与持久化已实现) -> M2-3(PASS_WITH_DEBT, 整体裁定：D12/D13 挂账) -> M2-4.a(PASS, 执行安全契约冻结；M2-4 已展开为 a~e) -> M2-4.b(PASS, 进程组与生命周期内核) -> M2-4.c(PASS_WITH_DEBT, 执行命令与校验接入) -> M2-4.d(PASS_WITH_DEBT, 输出背压·事件流·退出收口) -> M2-4.e(PASS, 移除 shell spawn 开放面) -> M2-4(PASS_WITH_DEBT, 整体裁定：D14~D17 挂账) -> M2-5.a(PASS, 脚本库 CRUD UI) -> M2-5.b(PASS, 执行面板：真实执行+输出流+取消+补 D15 审计与 D14 节流) -> M2-5.c(PASS, 运行历史 script_runs_list) -> M2-5(PASS, 整体裁定) -> M2-6.a(PASS, 命令片段契约冻结) -> M2-6.b(PASS_WITH_DEBT, snippets.json 持久化与 snippet_* 四命令) -> M2-6.c(PASS, run_command 复用 script_runner 内核) -> M2-6.d(PASS_WITH_DEBT, 命令片段库前端面板) -> M2-6(PASS_WITH_DEBT, 整体裁定：D18/D19/D20 挂账) -> M2-6-fix1(PASS, 复核整改：P1-1/P1-2/P1-3/F-1/F-2) -> M2-7(PASS, 工具清单与打包；BUILTIN_TOOLS + include_str! 嵌入) -> M2-8(PASS, 工具箱 UI + 子 webview 打开 + Web 隔离) -> M2-9(PASS, 五个种子验收；Rust 单测 + check-seed-tools.py) -> M2-6.e(PASS, 内置片段种子，关闭 D19) -> M3 展开卡(PASS, 冻结 F1~F13) -> M3.a(PASS_WITH_DEBT, 输出管道与生命周期内核：mpsc+pump/进程组回收/Channel 退避；含裁定整改 P1 真实 resize 取证 + P2 放锁后再终止) -> M3(PASS_WITH_DEBT, 整体裁定：D23/D24/D25 挂账) -> M3.c(PASS, 临时历史 40 条 + resize 静默窗口；后端零改动、新增 D26) -> **M4 展开卡(CARD_EXPANDED, 2026-09-05 22:25，Lane A1：M4-1~M4-8 → 30 张子卡 + 依赖图 + 冻结条款 F1~F12 + 测试矩阵 ID 段 + 6 个新夹具码位；证据 logs/checkpoints/M4-20260905-2225.md)；NEXT=M4-1.a（Lane A2；M4-5.a 由 Lane A6 并行）**`。M3-4 体验项范围见 `logs/checkpoints/M3-20260905-2030.md` §4 F12，交付证据见 `logs/checkpoints/M3.c-20260905-2202.md`。M2-4.e 证据见 `logs/checkpoints/M2-4.e-20260905-0008.md`；M2-6 整体裁定证据见 `logs/checkpoints/M2-6-ACCEPT-20260905-1625.md`，复核整改证据见 `logs/checkpoints/M2-6-REVIEW-VERDICT-20260905-1648.md`。 -> **M4 子卡落地（A2/A3/A4/A5/A6/A7/A8 在工作树内交付代码包，未提交/未 push；A2=M4-1.c COMPLETE、A6=M4-5.d DELIVERED、A7=M4-6/7 DONE、A5=M4-4 与 A8=M4-8 在途）-> M4-9 合流收口（NEXT=M4-9a：IF-1~IF-5，见 详细设计与实施计划.md §6.2；IF-3 已由 A1 定位并出 `A1-IF3-tools-policy-fixture-fix` 补丁，A0 落地后自检回 ALL_PASS）**`。

当前执行器：Codex 主任务（`CODEX_READY`）；机械审计与独立脚本任务优先委派 `gpt-5.6-luna / low`。M0 已关闭，M1 起继续按关键路径逐点推进；每个检查点必须先验收、回写证据、独立提交并确认工作树干净，再领取下一点。硬停止条件和跨模型回归步骤统一见 `AI-模型切换与接手清单.md`。

| 顺序 | WBS | 为什么排在这里 | 模型路由 | 最少提交 | 状态 |
|------|-----|----------------|----------|----------|------|
| 1 | M0-0 完整基线 | 没有可比基线就无法判断后续重构是否退化 | `AI:BALANCED/R:medium` | 3 | 已关闭；新三批逐批 PASS，raw aggregate UNSTABLE 的六项已裁决 |
| 2 | M0-1 自动门禁 | 将冻结契约固化为脚本，再为 M0-0.b/c 采数 | `AI:BALANCED/R:high` | 3 | 已完成；核心 `2b476d3`，正式总控 `ac0ecac`，版本化证据门禁 `73e9dfb` |
| 3 | M0-2 生命周期重构 | 先统一资源所有权，避免每个功能重复造退出逻辑 | `AI:DEEP/R:high` | 4 | `M0-2.a/b/c/d = PASS`；下一 WBS 为 M0-3（`AI:DEEP/R:xhigh`） |
| 4 | M0-3 安全边界重构 | 先统一来源、路径和执行策略，避免 M1~M5 重复且不一致 | `AI:DEEP/R:xhigh` | 4 | 已关闭 |
| 5 | M0-4 构建/依赖清理 | 上游边界明确后的快速、低风险清理 | `AI:BALANCED/R:medium` | 3 | 已关闭 |
| 6 | M0-5 资源闭环 | 基于统一生命周期做真实 release 压测 | `AI:DEEP/R:high` | 3 | 已关闭 |
| 7 | M0-6 崩溃与 GUI 回归 | 复杂、耗时，必须在前置结构稳定后验证 | `AI:DEEP/R:high` | 3 | 已关闭 |
| 8 | M0-7 验收放行 | 小模型汇总证据，强模型独立复核后才能放行 | `AI:FAST/R:medium; ESCALATE:DEEP` | 3 | 已关闭 |

执行单位必须是 `WBS.checkpoint`，例如 `M0-2.b`，不能是“完成 M0”。`SIMPLE/MEDIUM/COMPLEX` 分别至少拆成 2/3/4 个独立、可编译、可回滚提交；上一检查点 FAIL 时禁止进入下一检查点。「逐点执行」不等于「逐点停机」：PASS 且提交边界干净后可立即继续。检索、格式、schema、哈希和固定夹具先交给脚本或 `AI:FAST/R:low`，主模型只做关键 diff 复核；安全、生命周期、并发、IPC 和最终放行不得降级。

---

> 以下 #1~#15 按稳定需求 ID 展开，仅用于检索与范围说明，不代表执行顺序。AI 必须以上方“AI 下一任务领取队列”为当前入口，并以文末“有序需求表”判断后续里程碑顺序。

## 1. 脚本库中心：快速调用各种 shell 脚本  `P1`

**目标**：用户在控制台内维护一套 shell 脚本库，点击即可在本地/目标环境快速执行，结果回显。

**范围**
- 脚本库管理（新增 / 编辑 / 删除 / 分类标签）。
- 一键执行（选择脚本 → 跑 → 输出捕获）。
- 参数化脚本（占位符 `${ARG}` 在执行前弹窗填值）。
- 脚本来源：本地文件 + 可选的内置常用模板（备份、清理、部署等）。

**与现有架构关系**
- 现有 `bridge.rs` 已具备命令执行能力（`request_sync` 用 `std::thread::spawn` 后台跑）。可复用「后台线程 + 事件回传前端」模式做脚本执行，避免 UI 阻塞。**注意**：高吞吐的 mpsc+pump task 解耦是 #9（M3）的改造，#1 在 M2 先行，首期不依赖该改造。
- 输出回显可借鉴终端改造的 `Channel` 推送模型（详见 `fileterm-study/终端改造-详细设计.md` §3.2），避免大输出卡死。

**建议切入点**
1. `domain.rs` 加 `ScriptMeta`（id / name / category / path / args）。
2. `workspace.rs` 持久化脚本库（与成果库同目录）。
3. `bridge.rs` 加 `run_script(meta, args) -> Channel<LogLine>`。
4. 前端 `ActivityBar` 加「脚本库」面板。

---

## 2. 小工具框架：内置 + 可扩展的 HTML 小工具  `P1`

**目标**：控制台内置一组离线可用的 HTML 小工具（JSON 格式化、Base64、时间戳、正则、Cron、Markdown 预览等），并做成**可扩展的小工具框架**——程序后续支持各种小工具（内置种子 + 用户自定义丢进目录即加载）。

**范围**
- 工具以独立单文件 HTML（自包含 CSS/JS，数据不出本机）挂在控制台「工具箱」，类似"应用"。
- **内置种子工具（文件已落盘 `src-tauri/src/tools/`，但加载框架未实现）**：**⚠️ 现状（2026-08-27 复核）**：5 个 HTML 仅物理落盘，`build.rs` 仍极简无 `include_dir!`、`tauri.conf.json` 无 `bundle.resources`、`bridge.rs` 无 `list_tools`，故当前应用**无法加载/打开这些工具**。须待 #2 的 `list_tools`+子 webview 打开+嵌入机制三处落地后才可用，不要误以为"已落位即可用"。
- **⚠️ 现状更正（2026-09-05 实测，`M2-7-20260905-1701.md` §3 #1）**：`src-tauri/src/tools/` 目录与 5 个 HTML **均不存在**，上文「文件已物理落盘」陈述**不成立**——不是「只缺加载框架」，而是「文件 + 框架都缺」。故 **M2-7.b 须先创建 5+ 个种子 HTML**（JSON 格式化 / Base64 / 时间戳 / 正则 / Cron / Markdown 预览），否则 `list_tools` 无内置项、M2-9 验收无对象。内置嵌入倾向 `include_str!`（编译期、零新依赖），用户工具走 `workspace/tools/` 运行时扫描。
  - `cron-tool.html`：Cron 生成/检测/未来执行时间推算（纯前端 JS + localStorage）。
  - `regex-tool.html`：Regex Forge 正则工坊——自动识别 Java/Python/Rust/Node/IDEA/VSCode 格式、高亮匹配、多语言转换。
  - `json-tool.html`：JSON 格式化（2 空格）/ 压缩 / 校验 / 键排序（纯前端，自包含）。
  - `base64-tool.html`：Base64 文本↔编码、UTF-8 安全编码、URL-Safe 变体。
  - `timestamp-tool.html`：时间戳↔日期双向转换（秒/毫秒、本地/UTC/ISO 8601、实时时钟）。
- **扩展机制**：用户把自写的单文件 HTML 丢进 `workspace/tools/`（或 `src/tools/`）即被工具箱动态枚举加载，无需改代码。
- 工具可通过 `bridge` 暴露的本地能力（读写文件、调脚本）做增强（可选）。

**与现有架构关系**
- 复用现有 browser-tabs 的 `WebviewBuilder` 子 webview 加载本地 `file://` 或 `asset:` 资源的能力。**注意**：内置工具在打包后不在运行目录，需用 `include_dir!` 嵌入或 `tauri.conf.json` 的 `bundle.resources` 声明；用户工具 `workspace/tools/` 走运行期读盘。
- 工具清单 = 内置 `src/tools/` + 用户 `workspace/tools/` 合并枚举；与 #15 插件系统协同（工具可包装为插件）。
- 单文件 HTML 不引入构建步骤，保持"丢进去就能用"。

**建议切入点**
1. `src-tauri/src/tools/` 内置种子工具（cron-tool / regex-tool 已落位）。
2. `bridge.rs` 加 `list_tools() -> Vec<ToolMeta>`（扫描内置+用户目录，进 ACL）；打开走子 webview/iframe 加载。
3. 主控制台加「工具箱」入口，动态枚举并渲染卡片。
4. `ToolMeta`（name / path / builtin: bool / icon）登记，新增工具无需改前端硬编码。

---

## 3. 验证：关闭窗口能否正确释放资源  `P0`（待验证，非纯开发）

**目标**：确认关闭窗口（主窗口 / 子 webview / 终端页签）时，相关资源被彻底回收，无泄漏（僵尸进程、孤儿 PTY、文件句柄、内存堆积）。

**待验证清单**
- [ ] 关闭宫格子 webview（`grid-0..N`）：GTK 子窗口 + WebView2/WebKit 实例是否销毁？
- [ ] 关闭终端页签：PTY master/slave、子 shell 进程树是否被 kill（参考 `bridge.rs` 的 `term_kill` / child drop 逻辑）？
- [ ] 关闭主窗口：整个 App 进程是否干净退出，还是残留后台线程 / channel 监听？
- [ ] 连续开关 N 次后内存是否线性增长（泄漏）？

**与现有架构关系**
- 现有 `grid_process.rs` 的 `GridChildHandle`（`pub struct`，line 75）含 `pub child: Child`（line 77），**无 `impl Drop`**；多进程改造（2026-08-24 落地）后关闭逻辑收口到 `GridProcessManager` 的 `kill_child`（line 619）/`restart`（line 631）/`shutdown_all`（line 680）显式 `child.kill()`（原 `close_grid`/`close_one` 已随多进程改造删除/改名）。需接入 M0-2 统一生命周期，避免残留。
- 终端改造详细设计里已规划 worker 线程退出机制（reader EOF → 线程自然结束）。
- 本项目日志系统有 `session-*.log`，可借此观察关闭前后的进程/句柄变化。

**建议切入点**
1. 写一键验证脚本：`lsof` / `ps --ppid` / `cat /proc/<pid>/status` 对比开关前后。
2. 在 `bridge.rs` 的窗口 `on_window_close` 钩子里补显式清理（kill 子进程树、drop channel）。
3. 若发现泄漏，定位到具体资源（PTY 还是 webview）再修。

**注意**：此项先验证再开发。当前 `run-gui.sh` 启动的应用可直接用来复现开关窗口场景。

---

## 4. 常用 Linux 命令库：快速执行常用命令  `P1`

**目标**：把高频 Linux 命令做成「命令库」，点击/搜索即执行，不用手敲。与脚本库（#1）的差异是：命令库是**单条命令片段**（如 `df -h`、`systemctl status`、`du -sh *`），脚本库是**多行文件**。

**范围**
- 内置常用命令分类（系统 / 网络 / 磁盘 / 进程 / 文本处理）。
- 搜索 + 收藏常用命令。
- 一键执行 + 结果回显（复用 #1 的执行/回显通道）。

**落地状态（2026-09-05，M2-6 整体裁定 `PASS_WITH_DEBT`）**
- ✅ 搜索 + 收藏：已交付（收藏为 localStorage 本地态，按片段 id）。
- ✅ 一键执行 + 结果回显：已交付，`run_command` 复用 #1 的 `script_runner` 内核（**未另建执行旁路**），执行态组件与脚本库共用。
- ⚠️ **内置常用命令分类：只交付分类框架，未交付内置命令本身**——`SNIPPET_BUILTIN_CATEGORIES`（system / network / disk / process / text）已冻结，`builtin` 字段与「内置不可删」逻辑已就位，但**全仓无内置片段种子**，首次打开命令库为空。登记债务 **D19** + 后续卡 **M2-6.e 内置片段种子**。
- ⚠️ 需求原文的 `run_command(line, args)` 口径**已被 M2-6.a 冻结条款 F1 推翻并替代**为 `argv: Vec<String>` + 整元素 `{NAME}` 占位（字符串行需自实现 shell 词法解析，逼近 P0 红线），替代理由见 `logs/checkpoints/M2-6-20260905-1700.md §3 修正 4`。
- ⚠️ 命令片段运行记录混入 `script-runs.json` 且无 `kind` 判别，历史面板以裸 UUID 展示、无法区分脚本/命令来源（债务 **D21**；`M2-6-fix1` 复核新增）。
- ✅ 复核整改：`M2-6-fix1` 已闭环（运行时来源校验单测 / `argv[0]` 禁占位符 / 执行层纳入静态门禁 / 新建 `enabled` 贯通 / 命令库前端纳入 UI 夹具），证据 `logs/checkpoints/M2-6-REVIEW-VERDICT-20260905-1648.md`。

**建议切入点**
- 与 #1 共用 `run_command(line, args)` 后端能力，前端做"命令片段库"独立面板。

---

## 5. Git 等版本控制功能  `P0`

**目标**：在控制台内提供 Git 可视化操作（不止现有 `request_sync / confirm_sync` 推送闸门），覆盖日常提交工作流。

**范围**
- 仓库状态可视化（改动文件、分支、未推/未拉）。
- 提交（stage / commit / push）、拉取、分支切换。
- diff 预览、历史查看。
- 多仓库管理（现有 `RepoConfig` 已支持多仓库配置）。

**与现有架构关系**
- 现有 `sync.rs` 已实现 git2 三方合并、`request_sync` / `confirm_sync` 闸门 + 审计（`audit.json`）。
- 安全红线：token 只存 keyring，推送需确认——新功能必须遵守，不能绕过确认闸门。

**建议切入点**
1. `bridge.rs` 加 `git_status(repo) / git_diff(repo, file) / git_commit(repo, msg) / git_branch_*`。
2. 前端加「Git」面板（复用成果库目录视图）。
3. 命令注册后**必须同步加进 `permissions/default-commands.toml`**（本项目历史坑：ACL 白名单缺命令会被静默拒绝，详见 `未完成的任务.md`）。

---

## 6. 数据库等功能  `P1`

**目标**：在控制台内连接并操作数据库（至少 SQLite / MySQL / PostgreSQL），做查询、表浏览、结果导出到成果库。

**范围**
- 连接管理（多数据源、凭据存 keyring，复用 `keyring_store.rs`）。
- SQL 执行 + 结果表格展示。
- 查询结果一键存入本地成果库（带溯源）。
- 只读优先，写操作需二次确认（对齐现有安全闸门理念）。

**与现有架构关系**
- 凭据隔离可复用 `keyring_store.rs`（系统密钥库）。
- 审计可复用 `audit.json` 写入规范。
- 结果导出复用 `workspace.rs` 成果落盘。

**建议切入点**
1. `Cargo.toml` 引 `sqlx`（或 `rusqlite` + `mysql` + `postgres`）。
2. `bridge.rs` 加 `db_query(conn_id, sql) -> rows`。
3. 前端加「数据库」面板（连接表单 + SQL 编辑器 + 结果表）。

**⚠️ 展开状态（2026-09-05 22:25，Lane A1）**：本需求对应 WBS **M4-1~M4-4**，已展开为 15 张子卡，`logs/checkpoints/M4-20260905-2225.md`。上列切入点为 2026-08 原始草案，**三处已被实测修正，不得照抄**：①`sqlx`/`rusqlite`/`mysql`/`postgres` 当前在 `Cargo.toml` 与 `Cargo.lock` 中**均为零**，选型须由 `M4-1.a` 裁定（含 `tokio` 是否提升为直接依赖）；②命令名以 `M4-1.b` 冻结为准，`db_query(conn_id, sql)` 只是占位；③凭据键必须用 `db:<conn_id>` 命名空间（F-7），直接用 `conn_id` 会与 git `repo_id` 碰撞而静默取回错误凭据。「导出到成果库」为 `M4-4` 显式范围项（F-6），不确定则首期不做。

---

## 7. A2P / A2A 等 Agent 协议  `P2`

**目标**：支持 Agent-to-Protocol / Agent-to-Agent 通信协议，让本应用能作为 Agent 节点对外提供能力或与其他 Agent 协作。

**范围**（待澄清，先列可能方向）
- A2A：接收/发起 Agent 间任务委派、消息路由。
- A2P：把本应用的本地能力（脚本执行、git、浏览器操作）以协议接口暴露给外部 Agent 调用。
- 鉴权、会话、能力声明（capability manifest）。

**与现有架构关系**
- 现有 `BridgeCall` 权限内核可演进为「能力清单 + 调用闸门」，天然适配 A2A/A2P 的能力暴露与鉴权。
- 已有审计机制可记录跨 Agent 调用。

**建议切入点**
- 先定义本应用对外暴露的能力清单（复用 #1~#6 的命令），再做协议层（HTTP/gRPC/消息总线）。此项偏架构，建议放在 #1~#6 稳定后再做。

---

## 8. 浏览器收藏 + 设为默认浏览器 + 增加图标  `P0`

**目标**：完善浏览器体验——收藏夹、系统级默认浏览器绑定、桌面/菜单图标。

**范围**
- 收藏功能：收藏当前页（标题 + URL + 缩略图/分类），收藏夹面板管理，点击即开。
- 设为默认浏览器：注册 `xdg-settings set default-web-browser`（Linux）/ 系统协议处理器（`http/https` scheme handler），让外部链接用本应用打开。
- 增加图标：桌面快捷方式、开始菜单项、文件管理器关联；打磨 `src-tauri/icons/` 全套尺寸（32/128/512 + 平台特定 icns/ico）。

**与现有架构关系**
- 现有 `browser-tabs` 插件管理页签，`BrowserHost` 负责 url 加载——收藏可挂在其导航完成事件 `tab-navigated` 上。
- 图标已有 512 圆角 `icon.png`（README 已说明），需补齐其余尺寸与安装脚本。
- 默认浏览器涉及 `tauri.conf.json` 的 `protocol` / 系统 `xdg` 注册，可能要加一个 `setup` 命令或安装后钩子。

**建议切入点**
1. `bridge.rs` 加 `add_bookmark(url, title) / list_bookmarks() / remove_bookmark()`（+ 进 ACL）。
2. 前端 `BrowserHost` 加地址栏「收藏」按钮 + 收藏夹侧栏。
3. 默认浏览器：`setup-linux.sh` 里加 `xdg-settings` 注册 + `*.desktop` 文件；macOS/Windows 用 Tauri 的 `setAsDefault` 或打包配置。
4. 图标：用现有 `icon.png` 生成全套尺寸（`npm run tauri -- icon` 或 `tauri icon`）。

---

## 9. 借鉴/集成开源 fileterm 终端项目  `P1`

**目标**：吸收开源项目 **fileterm**（已在 `fileterm-study/` 做过 V1~V5 五轮源码级审核与借鉴分析）的终端能力，落地到本应用的终端/控制台。

**已有基础（来自 `fileterm-study/` 分析结论）**
- Tauri v2 `Channel` IPC：前端从 `@tauri-apps/api/core` 导入；底层 `Arc<ChannelInner>`，可跨 `std::thread::spawn` 发送（`文件` `终端改造-详细设计.md` §3.2）。
- `portable_pty 0.8.1`：`MasterPty::resize(&self, PtySize)`（非 `set_size`）、`try_clone_reader()`、`take_writer()` 已实证。
- PTY 输出推送模型：用 `mpsc(128) + 独立 pump task` 解耦，避免 `Channel::send` 同步阻塞（本项目当前为直接 `send`，单终端低吞吐可接受）。
- 退避重试：`TERMINAL_DATA_RETRY_MAX_BACKOFF_MS=30_000` 指数退避（本项目暂无，建议补）。
- worker 线程退出机制：`term_kill` drop child → reader EOF → 线程自然结束。
- 资源监控维度差异：本项目 `resource_stats` 为进程树 RSS 常驻监控（StatusBar/ActivityBar），fileterm 为全机 CPU%/内存%。

**范围**
- 终端多实例 / 页签化（复用现有终端页签 + `bridge.rs` 的 `term_*` 命令）。
- 引入 mpsc + pump task 解耦，支撑高吞吐输出不卡 UI。
- 补 `Channel` 跨线程透传 + 断线退避重试。
- 把 fileterm 的可用特性（如临时历史 `TEMPORARY_HISTORY_LIMIT=40`、resize 静默窗口 `TERMINAL_RESIZE_OUTPUT_QUIET_MS`）按本项目需要采纳。

**建议切入点**
1. 先读 `fileterm-study/终端改造-详细设计.md`（V5 标记，含 §0 依赖溯源表、§3.2 跨线程+阻塞双注、worker 退出机制、§6 风险表）。
2. `bridge.rs` 终端输出改为 mpsc + pump task；`term_kill` 确保 child 进程树回收（与 #3 资源释放联动）。
3. 新命令注册后同步进 `permissions/default-commands.toml`。

---

## 10. 支持图片展示  `P1`

**目标**：在控制台/成果库/工具箱内支持图片的展示与预览（当前成果以 `.md`/`.html` 文本为主，缺图片渲染能力）。

**范围**
- 成果库图片预览：右键保存的图文成果中的图片、本地图片文件直接预览。
- 画廊/缩略图：多图浏览、网格视图、点击放大。
- 图片来源：本地文件（`workspace/` 下）、网页选区保真的图片（现有 `injected/collect.js` 已绝对化 `img` 链接）。
- 拖拽/粘贴上传图片到成果库或聊天/笔记。

**与现有架构关系**
- 现有 `injected/collect.js` 已做富文本选区保真（克隆 DOM、内联图片链接），图片展示可消费其产物。
- 成果库持久化在 `workspace.rs`，图片可作为 artifact 的附件（需扩展 `domain.rs` 的 `Artifact` 结构，增加 `images: Vec<ImageRef>`）。
- 渲染复用前端 Vue 组件 + 静态资源服务（`asset:` 协议或本地路径加载）。

**建议切入点**
1. `domain.rs` 的 `Artifact` 增加图片附件字段；`workspace.rs` 落盘时同步存图片到 `workspace/<id>/images/`。
2. 前端加图片预览组件（支持缩放、灯箱）。
3. 成果列表/详情页支持图文混排渲染。

---

## 11. 定时任务调度  `P1`

**目标**：在控制台内提供定时任务能力，让脚本/命令/工具按 cron 或间隔周期自动执行，无需人工点击。

**范围**
- 任务定义：名称、类型（执行脚本 #1 / 运行命令片段 #4 / 调用 HTML 工具 #2 / 自定义 shell）、调度表达式（cron 或 `every Ns/min/h`）、启用开关、上次/下次运行时间。
- 调度引擎：应用常驻后台，到点触发任务，捕获输出并落日志。
- 结果可回写成果库（#10）或仅记运行历史。
- 持久化任务列表（与脚本库同目录 `workspace/tasks.yaml` 或 `tasks.json`）。

**与现有架构关系**
- 执行通道直接复用 #1 的 `run_script` / #4 的 `run_command`（`Channel<LogLine>` 回显模型），定时任务只是"触发方"。
- 调度器挂在 `main.rs` 的 `setup` 钩子里（`tokio::spawn` 常驻 task 循环扫描 next_run），并注册到 M0-2 统一生命周期（退出时取消所有 timer）。
- 每个任务执行仍走审计 `audit.json`（安全红线）+ 凭据 `KeyringStore`。

**建议切入点**
1. 新增 `scheduler.rs`：`TaskDef` 领域模型 + `Scheduler`（`tokio::time::sleep` 轮询或 `tokio_cron_scheduler` crate），`setup` 时 `tokio::spawn` 启动。
2. `bridge.rs` 加 `task_list() / task_add(def) / task_update(def) / task_remove(id) / task_run_now(id)`（**必须进 `permissions/default-commands.toml`**）。
3. `domain.rs` 加 `TaskDef`（cron / command / enabled / last_run / next_run）。
4. 前端加「定时任务」面板（CRUD + 开关 + 运行历史）。
5. 退出收口：把 `scheduler.abort()` 注册到 M0-2 生命周期协调器，不预设尚未实现的公开命令名。

**⚠️ 展开状态（2026-09-05 22:25，Lane A1）**：本需求对应 WBS **M4-5~M4-8**，已展开为 15 张子卡，`logs/checkpoints/M4-20260905-2225.md`。上列切入点为原始草案，**四处已被实测修正，不得照抄**：①**`tokio` 当前只是 tauri 的传递依赖**（`Cargo.lock` 1.53.1，`Cargo.toml` 无），「`tokio::spawn` 启动」**当前不可编译**，须由 `M4-1.a` 裁决（提升为直接依赖 vs `std::thread` + `Condvar` 可中断 sleep）；②持久化统一 `tasks.json`（`tasks.yaml` 作废，既有持久化全为 JSON 原子写，F-4）；③**任务类型中的「调用 HTML 工具 #2」首期不做**——`list_tools` 是 M2-7 冻结的只读清单命令，工具无 headless 执行入口，强加即无人消费的死代码（F-5）；④退出收口任务名定为 `stop-scheduler`，其注册索引**必须小于** `kill-running-scripts`（先停触发再杀运行中的，F7）。另：时钟必须可注入（F8），否则「错过执行 / 系统时间变化」不可测。

---

## 12. Agent 与 Skill 生态  `P2`

**目标**：在控制台内构建可扩展的 Agent 与 Skill 生态——既能内置/外接 Agent 能力，又能以"Skill（技能包）"形式沉淀可复用能力，供用户或 Agent 调用。

**范围**
- **Agent 生态**：
  - 内置 Agent 宿主：应用内可承载一个本地 Agent（对接 LLM API 或外部 CLI Agent 方言，借鉴 dbx `ai_cli_agent.rs` 的 Codex/ClaudeCode/CodeBuddy/Qoder/OpenCode/Cursor/Grok 多方言归一思路）。
  - Agent 可调用的"能力清单"= 本项目已有命令（#1 脚本 / #4 命令 / #5 git / #6 数据库 / #8 浏览器 / #11 定时任务）。
  - 与 #7 A2P/A2A 协同：Agent 既可作为 A2A 节点，也可通过 MCP（#7 内嵌 rmcp）把能力暴露给外部 Agent。
- **Skill 生态**：
  - Skill = 可复用能力单元（类似 VibeKit 的 SKILL.md 升级版：带元数据 + 执行脚本 + 触发条件）。
  - 来源：内置常用 skill（如"一键清理日志""备份工作区"= 复用 #1 脚本）、用户自定义 skill 目录（`workspace/skills/` 下 Markdown/YAML 描述 + 执行体）、可选从远程拉取。
  - 触发方式：手动调用 / Agent 自动匹配 / 定时任务（#11）挂载。
- **记忆/上下文**：借鉴 dbx `agent_kv.rs` 做 Agent 记忆层（KV 持久化），注意本项目有独立成果库，`agent_kv` 仅作 AI 上下文，不混为用户成果。

**与现有架构关系**
- 能力底层全部复用现有命令（#1~#11），Agent/Skill 是"编排层"而非"新执行层"。
- 安全红线全覆盖：Agent/Skill 执行任何动作（执行/出网/落盘）均走 `audit.json` + 确认闸门 + `KeyringStore`，绝不开放绕过通道（对应 dbx 红线"命令不加 ACL"不可借鉴）。
- 与 #7 共用 `phantom-core` 下沉 + 内嵌 `rmcp`，Skill 可同时注册为 MCP 工具。

**建议切入点**
1. `domain.rs` 加 `AgentDef` / `SkillDef`（name / description / trigger / exec / acl_level）。
2. `bridge.rs` 加 `skill_list() / skill_install(path) / skill_run(id, args) / agent_chat(msg) -> Channel<Token>`（**进 ACL**）。
3. 新增 `agent_runtime.rs` / `skill_runtime.rs`：skill 执行沙盒（复用 #1 执行通道）、agent 对话循环（流式 token 回传用 Channel）。
4. 前端加「Agent」面板（对话）+「Skill 市场/管理」面板。
5. 与 #7 协议层共用护栏（McpGlobalPolicy + fail-closed）。

---

## 13. 知识图谱  `P2`

**目标**：在控制台内构建可检索、可可视化的知识图谱——把成果库（#10）、脚本（#1）、命令（#4）、Skill（#12）、网页收藏（#8）等散落资产，抽取为「实体—关系—属性」的图谱，支撑关联浏览、影响分析、智能问答上下文。

**范围**
- **实体来源**：成果库 Artifact（#10）、脚本 ScriptMeta（#1）、命令片段（#4）、Skill（#12）、书签 Bookmark（#8）、Git 仓库/分支（#5）。
- **关系类型**：`引用`/`属于分类`/`由Agent生成`/`定时触发`/`同仓库`/`相似` 等（可扩展）。
- **抽取方式**（两阶段）：
  - 轻量期：基于元数据 + 正则/规则抽取（如 Artifact 内链、脚本引用的命令、Skill 触发的任务），纯本地、离线。
  - 智能期：借助 #12 Agent 做语义抽取（实体识别 + 关系推断），结果存图谱（标注"AI生成"，可人工校正）。
- **存储**：本地图存储——首期用 SQLite + 邻接表/JSON 存图（复用 #6 的 sqlx 与 Keyring 思路，不引 Neo4j 等重依赖）；可选导出 GraphML/JSON。
- **可视化与查询**：前端图谱视图（力导向图，节点=实体、边=关系）+ 图查询（"X 影响了哪些成果""Y 被哪些脚本依赖"）。
- **消费方**：#12 Agent 问答时把子图作为上下文注入（RAG 式），提升回答准确性。

**与现有架构关系**
- 实体全部来自既有领域模型（`domain.rs` 的 `Artifact`/`ScriptMeta`/`RepoConfig`/`Bookmark`/`TaskDef`/`SkillDef`），图谱是**派生索引层**，不重复存储主数据。
- 复用 #6 的数据库连接与查询能力；复用 #10 的图片/富文本展示做节点预览。
- 智能抽取复用 #12 的 Agent 能力与 `agent_kv` 记忆层。
- 安全红线：图谱抽取若触发执行/出网动作（如 Agent 抽取需调 LLM），走 `audit.json` + 确认闸门 + `KeyringStore`。

**建议切入点**
1. `domain.rs` 加 `GraphNode`（id / kind / ref_id / label / props）/ `GraphEdge`（src / dst / relation / weight / source=rule|ai）。
2. `graph.rs`：图谱构建器（扫描既有领域对象 → 产出 nodes/edges，持久化到 `workspace/graph.db` 或复用 #6 库）、查询 API（`neighbors(id)` / `paths(a,b)` / `by_kind(kind)`）。
3. `bridge.rs` 加 `graph_build() / graph_query(q) / graph_neighbors(id) / graph_export()`（**进 ACL**）。
4. 前端加「知识图谱」面板（力导向可视化 + 查询框 + 节点详情）。
5. 与 #12 协同：Agent 问答前 `graph_query` 取子图注入上下文。

---

## 14. 浏览器会话存档：请求/资源可见 + 关闭保存删除  `P1`

**目标**：浏览器访问的内容在本地全程可见——能看到页面发起的所有网络请求与加载资源（JS/CSS/图片/XHR/文档），关闭页面时弹出「保存会话 / 删除会话」选择，保存后本地可回看该次浏览的完整痕迹（含请求列表、资源、截图/快照）。

**范围**
- **请求与资源可见**：监听每个 tab 的网络请求（`on_web_resource_response_received` / WebKit `WebResource` 事件），汇总为请求列表（URL / 方法 / 状态码 / 类型 / 大小 / 耗时）。资源可按类型（文档/脚本/样式/图片/媒体/XHR）筛选与预览（图片直接看、文本可查）。
- **本地存档**：关闭 tab 或主动「保存会话」时，把该会话的请求清单 + 关键资源 + 页面快照落盘到 `workspace/sessions/<id>/`。M1 内置会话范围的最小文本/图片预览，不依赖 M2 的 #10；M2 再把预览契约抽成成果库/工具箱共用能力。#6 数据库在 M4，M1 不依赖数据库。
- **关闭 UX**：关闭页签/窗口时弹「保存 / 删除」选择（不可静默丢）；已保存会话在「历史会话」面板可回看、可删除。
- **隐私红线**：存档默认仅本地，不自动出网；存档落盘走 `audit.json`，资源含敏感内容时标注。

**与现有架构关系**
- 当前 `tauri-plugin-browser-tabs` 仅有 `on_navigation` / `on_new_window` 拦截（已核查 `commands.rs:57/74`），**无请求层监听**——需新增 `on_web_resource_response_received` 钩子（Tauri v2 支持；Linux WebKit 后端可用），把事件经 `app.emit` 推前端。
- 前端 `collect.js` / `resources.js` 已在做 DOM 级收集，可扩展为"请求层 + DOM 层"双源。
- 存档由 M1 提供会话范围最小预览并用 `workspace.rs` 落盘；#10 在 M2 抽取/增强为全局图片能力，#6 在 M4 可选升级索引与查询，二者都不是 M1 启动依赖。
- 与 #8 收藏互补：收藏=主动存 URL，会话存档=被动存完整浏览痕迹。

**建议切入点**
1. `tauri-plugin-browser-tabs` 的 `WebviewBuilder` 加 `on_web_resource_response_received`（emit `ResourceReceived` 事件）。
2. `bridge.rs` 加 `session_save(tab_id) / session_list() / session_get(id) / session_delete(id) / session_export(id)`（**进 ACL**）。
3. `domain.rs` 加 `BrowserSession`（tab_id / url / title / requests: Vec<ResourceReq> / snapshot / created_at）。
4. 前端加「会话存档」面板（请求瀑布 + 资源预览 + 关闭弹窗保存/删除）。
5. 与 M0-2 统一生命周期协同：关闭 tab 时先 flush 会话缓冲，再清理子 webview。

---

## 15. 插件系统  `P2`

**目标**：提供可扩展的插件机制，让脚本（#1）、HTML 工具（#2）、Skill（#12）、浏览器会话处理器等能以"插件"形式注册、启用、配置，形成生态。

**范围**
- **插件模型**：`PluginManifest`（name / version / kind=script|tool|skill|session-handler / entry / permissions / config schema）。
- **加载与隔离**：从 `workspace/plugins/` 扫描加载（Markdown/YAML manifest + 执行体），复用 #1 执行通道与 #12 skill_runtime 沙盒。
- **能力注册**：插件可声明要暴露的命令/工具（自动并入 #7 MCP 工具集与 #12 Skill 列表）。
- **权限分级**：插件声明 `acl_level`，安装需确认（对标 Skill 红线），越权动作走确认闸门。
- **管理 UI**：「插件市场/管理」面板（安装/启用/禁用/卸载/配置）。

**与现有架构关系**
- 完全复用既有能力层（#1 脚本 / #2 工具 / #12 Skill / #14 会话），插件系统只是**统一编排与生命周期外壳**，不新建执行逻辑。
- 与 #7 共用 `phantom-core` 下沉 + 内嵌 rmcp：插件能力即 MCP 工具。
- 安全红线：插件执行任何动作走 `audit.json` + 确认闸门 + `KeyringStore`（对标 dbx 红线"命令不加 ACL"不可借鉴）。

**建议切入点**
1. `domain.rs` 加 `PluginManifest`。
2. `bridge.rs` 加 `plugin_list() / plugin_install(path) / plugin_enable(id) / plugin_disable(id) / plugin_uninstall(id)`（**进 ACL**）。
3. 新增 `plugin_runtime.rs`：manifest 解析 + 生命周期（load/unload/reload）+ 能力注册到 #7/#12。
4. 前端「插件管理」面板。
5. 与 #12 共用护栏（McpGlobalPolicy + fail-closed）。

---

## 优先级总览与里程碑

> 下表用于回答“需求放在哪个里程碑”，不是开工授权。M0 放行前，M1~M5 全部保持锁定。

### 有序需求表

> 同一里程碑先过滤未满足的硬依赖，再按阻塞级别、降复杂度收益和 `SIMPLE -> MEDIUM -> COMPLEX` 排序。下表只给需求级主路由；可执行检查点必须读取详细计划中的完整机器标签。

| 顺序 | ID | 需求 | 优先级 | 最早里程碑 | AI 主路由 | 升级/降级边界 | 主要依赖 |
|------|----|------|--------|------------|-----------|---------------|----------|
| 1 | 3 | M0 安全与资源收口 | P0 | **M0** | `AI:BALANCED` | 机械采集可用 `FAST`；生命周期/安全转 `DEEP` | 当前执行面 |
| 2 | 8 | 收藏、默认浏览器、图标 | P0 | M1 | `AI:BALANCED` | 图标可用 `FAST`；系统协议接入转 `DEEP` | M0 PASS、browser-tabs |
| 3 | 5 | Git 功能 | P0 | M1 | `AI:BALANCED` | 写操作、权限与恢复转 `DEEP` | M0 PASS、现有 `sync.rs` |
| 4 | 14 | 浏览器会话存档 | P1 | M1 | `AI:DEEP` | 不得降级 | M0 PASS、browser-tabs |
| 5 | 10 | 图片展示 | P1 | M2 | `AI:BALANCED` | 纯 UI 机械项可用 `FAST` | M0 PASS、`workspace`/`Artifact` |
| 6 | 1 | 脚本库中心 | P1 | M2 | `AI:BALANCED` | 执行通道与注入边界转 `DEEP` | M0 PASS、统一执行策略 |
| 7 | 4 | Linux 命令库 | P1 | M2 | `AI:DEEP` | 不得降级 | 复用 #1 安全执行通道（**已落实**：`run_command` 走 `script_runner::start_command`，M2-6 整体 PASS_WITH_DEBT；内置片段种子缺失＝D19） |
| 8 | 2 | HTML 工具框架 | P1 | M2 | `AI:BALANCED` | WebView 隔离与 ACL 转 `DEEP` | 子 WebView 隔离与打包 |
| 9 | 9 | fileterm 终端增强 | P1 | M3 | `AI:DEEP` | 可选体验项可用 `FAST` | M0 PASS、现有 `term_*` |
| 10 | 11 | 定时任务调度 | P1 | M4 | `AI:BALANCED` | 并发、触发与退出转 `DEEP` | M2 安全执行通道 |
| 11 | 6 | 数据库功能 | P1 | M4 | `AI:DEEP` | 不得降级 | M2 PASS、Keyring/审计 |
| 12 | 7 | A2P/A2A 协议 | P2 | M5 | `AI:DEEP` | 不得降级 | M1~M4 稳定 |
| 13 | 12 | Agent 与 Skill 生态 | P2 | M5 | `AI:DEEP` | UI 机械项可用 `BALANCED` | #1/#4/#6/#7/#11 稳定 |
| 14 | 15 | 插件系统 | P2 | M5 | `AI:DEEP` | UI 机械项可用 `BALANCED` | #1/#2/#12/#14 稳定 |
| 15 | 13 | 知识图谱 | P2 | M5 | `AI:DEEP` | 展示层可用 `BALANCED` | #6/#8/#10/#12 稳定 |

### 里程碑与滚动估算（约 14~16 周，单人）

| 里程碑 | 状态 | 覆盖 | 子任务数 | 剩余估时 | 放行标准 |
|--------|------|------|----------|----------|----------|
| **M0 安全与稳定性基线** | **PASS，已验收** | #3 + 基线/安全收口 | 8（M0-0~7） | 0 | 自动验证、资源闭环、9 项 GUI 回归、安全红线清零、验收报告 PASS |
| M1 浏览器与版本控制 | **PASS，里程碑已验收（PASS_WITH_DEBT）** | #8、#14、#5 | 9 | 2.3 周 | M0 PASS；M1-0~M1-9 全部完成且 M1-ACCEPT 验收通过（M1-7 GUI 目视验收挂账、M1-8/M1-9 运行时端到端联调挂账等 D1~D9 保留）；M2-1 已完成，NEXT=M2-2 |
| M2 本地资产与执行 | **PASS（含 M2-6.e 内置片段种子）** | #1、#4、#2、#10 | 9 + M2-6.e | 3 周 | M0 PASS 后具备资格；M2-7/8/9 + M2-6.e 已补齐工具与种子域 |
| M3 终端增强 | 可并行评估、未开始 | #9 | 4 | 1 周 | 至少 M0 PASS；并行需资源数据支持 |
| M4 数据与调度 | 已展开（CARD_EXPANDED），NEXT=`M4-1.a` | #6、#11 | 8（→30 张子卡） | 2 周 | 数据库面板+护栏；定时任务调度；展开卡 `logs/checkpoints/M4-20260905-2225.md` |
| M5 协议与智能生态 | 锁定 | #7、#12、#13、#15 | 12 | 3 周 | M1~M4 PASS 后启动 |
| 缓冲 | 未启用 | 测试/联调/需求微调 | - | 1~1.5 周 | 按实际风险启用 |

WBS 共 50 项（M0:8 / M1:9 / M2:9 / M3:4 / M4:8 / M5:12）。旧版 11~15 周和 M0 0.5~0.7 周已废弃：它遗漏了高风险入口收口、当前提交的 GUI 回归和验收证据建设。M0 的实时状态只维护在本文顶部领取队列，详细定义与勾选权归 `详细设计与实施计划.md` §2，避免双份状态镜像漂移。

## 通用红线

- 每新增一个 `#[tauri::command]`，同步更新 `permissions/default-commands.toml`；远程 webview 调用还需更新 `remote-collect.toml` 与对应 capability。2026-08-27 实测命令数为 59，仅作历史基线，不应硬编码为永久数量。
- 任何出网、落盘、命令执行和高风险删除动作写入 `audit.json`；凭据只存系统 keyring。
- 远程页面只获得完成当前交互所需的最小命令；本地写入、进程启动和危险文件操作必须绑定可验证的用户意图。
- M1~M5 每次合入必须对比 M0 基线；性能回退 >10%、clippy 新增 warning、前端总体积增长 >15% 均需阻断或书面豁免。
- 状态只依据当前提交的可复跑证据更新；“文件已落盘”“历史跑通过”“代码看起来存在”均不能单独标记完成。
