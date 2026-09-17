# 00 · Executive Summary — mvp-browser-os 单语义治理审计
> Chief（只读审计合成）· 推荐阅读顺序见 `README.md`。
> 本文件只做导航与结论，不复制大表格。所有结论均有源码证据（见 `15-EVIDENCE-INDEX.md` 与 `16-REVIEW-REPORT.md`）。

---

## CURRENT STATE
- 项目：Tauri2 + Vue3 桌面浏览器-OS。前端状态集中在 Pinia（`useBrowserStore`/`useLayoutStore`/`useWorkspaceStore`/`useSystemStore`/`useBookmarkStore`）；原生 webview/宫格子进程/PTY 在 Rust（`bridge.rs`/`grid_process.rs`/`main.rs`/`terminal.rs`）。
- 治理红线（VIEW SWITCH≠DESTROY / HIDE≠CLOSE / CLOSE≠KILL 对页签 / ACTIVATE≠NAVIGATE / 终端 PTY 存活于视图切换）：Tab/PTY 侧经独立复核全部成立（05/16 CLAIM-S4-03）；但 **GRID 侧存在活违反**——`HomeLaunchers.openArea:47` 在切到非宫格视图前 `closeGridAll`→`shutdown_all`（DESTROY），违反 ACCEPTED 红线 ADR-GRID-001，构成 FINAL_S4-1（18 SECOND-CONFLICT-001）。全局 S4 计数见下方全局计数块。
- 但**语义密度高、入口分散、隐藏副作用多**：存在多处重复状态/入口与未声明的原生副作用，模型容易"编译通过、运行语义错误"。

## SYSTEM SEMANTIC COMPLEXITY
- 关键语义约 16 个（01 SEM-001..016），覆盖主视图、激活页签、宫格生命周期/可见、收藏夹、终端、工作区当前位置、凭据、原生 webview 显隐。
- 跨层契约约 50+ IPC 命令（06），绝大多数一致；检查器已有 ~10 个（scripts/check-*.{py,mjs}）但存在盲区与互相矛盾。

## TOP 10 S4 / S3 RISKS（按危害排序）
1. **[S4] 离开宫格意图分叉（GRID_EXIT_DIVERGENCE）**：同一"离开宫格"经 `HomeLaunchers.openArea:47`=DESTROY（closeGridAll→shutdown_all kill），经 `ActivityBar:171-173`=HIDE（仅 setView）。无单一 `exitGrid` 意图动作，活违反 ACCEPTED 红线 ADR-GRID-001（18 SECOND-CONFLICT-001，FINAL_S4-1）。范围收窄：活分叉出口仅 1 处（`toggleGridToolbar` 为死导出、`ActivityBar:398` 为显式关闭按钮）。
2. **[S4] 原生 webview 显隐 HIDDEN_STATE，无前端响应式镜像**（01 SEM-012，历史遮屏/空白根因，18 FINAL_S4-2）。两层（DOM 占位壳 CSS visibility + 原生 bounds -30000/set_visible(true)）不同源，仅靠 `watch(mainView)→syncViewVisibility` 命令式同步；无 FE 侧检查器。
3. **[S3] position 命令携带未声明的 show 副作用**（05/16 CLAIM-LC-04，07 EVID-005）：经 18 SECOND-CONFLICT-004 由 S4 降 S3——可逐字读出的 FACT，但为机制性架构耦合、不产生静默错误（README:62 归 S3）。
4. **[S3] 宫格 CLOSE==KILL==DESTROY，无软关闭原语**（08 DUP-006，SMF-003）：每格=子进程，close_grid 必 kill。底层 hide capability 已存在（见 GRID_HIDE_CAPABILITY）；缺的是单一、业务级 canonical grid-hide / view-switch lifecycle entrypoint（`exitGrid`），**非缺 Rust HideWindow**。
5. **[S3] 组件手拼宫格生命周期（7 处）**（03 CASE-008，18 SECOND-CONFLICT-010）：`App.vue:199`/`HomeLaunchers:47`/`UnifiedTabBar:170`/`ActivityBar:123/134/167/398` 等，惯用法 `if(gridOpen) layoutGrid(); else buildGrid()` 重复 4 次；当前行为均正确但 owner 分散。
6. **[S3] IPC 门禁零执行力 + DRIFT 11 条无守卫占位**（06，18 SECOND-CONFLICT-007/006）：`check-command-set-consistency.py`/`check-grid-close-logic.mjs` 均未接入 `pre-merge.sh`（空真 PASS）；正则盲区漏 7 条 typed 占位。
7. **[S2] isBrowserView / isBrowserVisible 命名相近易误用**（08 DUP-001，18 SECOND-CONFLICT-003）：同源双谓词、单消费者、已有防呆注释，由 S4 降 S2。
8. **[S2] 宫格可见三标志正交态，派生未单点化**（08 DUP-002，18 SECOND-CONFLICT-002）：`gridOpen`+`mainView==='grid'`+`gridToolbarOpen` 互不派生（非多真源），但 `gridVisible` 派生 inline 复制到 ≥5 处、无单一命名真源；由 S4 降 S2。
9. **[S2] 收藏夹 panelOpen 高亮 vs bmPanelOpen 挂载不一致**（02 SEM-006，18 SECOND-CONFLICT-013）：消费点未复用派生量，仅"先开面板再切走"窄边界不一致。
10. **[S2] 凭据无 store owner + 3 处直连**（04，18 SECOND-CONFLICT-012）：所有权缺陷；"前端不持明文密码"红线已由 `types.ts:174-186` 类型层保证。

> **全局风险计数（FINAL，唯一真源 —— `18-SECOND-REVIEW-SYNTHESIS.md` §4，第二轮终审裁定）**：**`S4=2 / S3=8 / S2=6 / S1=3`**。
> - **S4（2）**：① GRID_EXIT_DIVERGENCE（离开宫格意图分叉，活违反 ADR-GRID-001）；② NATIVE_VISIBILITY_HIDDEN_STATE（原生 webview 双层显隐无前端镜像，历史事故根因）。
> - **S3（8）**：① position⇒show 架构耦合 ② `create_grid` 无条件 destroy+create ③ 宫格 CLOSE==KILL==DESTROY 无软关闭原语 ④ 组件手拼宫格生命周期(7 处) ⑤ ACTIVATE ⊇ CREATE（HIBERNATED 前端零镜像）⑥ IPC DRIFT 11 条无守卫占位 ⑦ IPC 门禁零执行力 ⑧ gridOpen 离开意图无统一出口导致悬挂（含 2 处 STALE 注释）。
> - **S2（6）**：① `move_path` ACL 断裂（唯一 BROKEN，用户可达 100% 失败）② 凭据无 store owner+3 直连 ③ isBrowserView/isBrowserVisible 命名相近 ④ `gridVisible` 派生未单点化(≥5 inline) ⑤ `bmPanelOpen` 消费点未复用派生 ⑥ `check-ui.mjs` 保留 SessionCloseDialog `[CURRENT]` allowlist（潜伏冲突）。
> - **S1（3）**：① `layout.aiNavOpen` 死重复声明 ② `layout.gridToolbarOpen`+`toggleGridToolbar` 死标志/死导出 ③ `currentLocalPath` 优先级隐式约定。
>
> **严重度口径（ADR-SEVERITY-001，18 终审保留 PROPOSED，须与 `semantic-governance/README.md:56` 二选一）**：S4=红线违反/生产静默错误；S3=架构耦合/分叉风险（无静默错误）；S2=命名-多源混淆/派生未单点化/已由类型层保证的红线；S1=死代码/隐式约定。
>
> **关于"05 S4=0"**：`05-LIFECYCLE-MODEL.md` §8 的 `S4=0` 是**第一轮生命周期章节局部统计，且已被第二轮 17B 推翻**——漏计了 GRID_EXIT_DIVERGENCE（活的视图切换型 DESTROY，违反 ADR-GRID-001）。**全局 S4 只能以本块（=2）为准，不得再出现"S4=0"式全局断言。**

## TOP SMALL-MODEL FAILURE MODES
1. SMF-002 create_grid FE/Rust 契约半改 → 内存爆炸/宫格不开。
2. SMF-003 "最小化宫格"只置 gridOpen=false → 450MB 孤儿进程。
3. SMF-004 删 closeGridAll 的 mainView 复位 → 浏览器空白（B9-4 回归）。
4. SMF-001 activateWeb 误用 isBrowserView() → grid 视图点页签失效。
5. SMF-008 组件裸 bridge.tabClose → 恢复栈/tabs[] 失配。

## KNOWN REAL INCIDENTS（历史真实事故，已 Root-Caused）
- "切走/切回视图后宫格或网页残留遮屏 / 浏览器空白"——原生 webview 显隐双层不同步（01 SEM-012/013，07 CASE-007）。
- B9-4 浏览器空白——closeGridAll 未复位 mainView（SMF-004，16 CLAIM-S4-02）。
- 黑闪——tabNew 用 tabPosition(-30000) 绕过去重隐藏新 webview（07 EVID-BYPASS-007）。

## QUICK WINS（零业务代码、仅修 checker/脚本）
- 修 `check-command-set-consistency.py` 正则覆盖 typed invoke + 5 命令入 KNOWN（RULE-005）。
- 修 `check-ui.mjs`：删 SessionCloseDialog `[CURRENT]` 标注（L37/L53）+ 死块（L222-243）。`check-session-persistence-policy.py` 实际已站"撤销"一侧（L21-23 + L230-231 违规检测器），**非矛盾方、无需改**（18 SECOND-CONFLICT-008，ADR-CHECKER-CONFLICT-001 Context 收窄为仅 check-ui.mjs）。
- 修 `check-grid-close-logic.mjs` G2 公式漂移（RULE-002）。
- 删除死重复 `layout.aiNavOpen`（RULE-012，需小步源码改动）。

## DO NOT REFACTOR YET（尚未达收敛条件 / 需先门禁）
- 不重命名 gridOpen→gridInstanceAlive（重命名非必须，文档化即可）。
- 不新增独立 show webview 命令（会引入 `set_visible(false)` 死锁风险，bridge.rs:567-569）。
- 不抽 `sync_browser_scene` 适配器（Phase 6 可选，先确认无回归）。
- 不重构宫格子进程模型 / PTY 后端（架构耦合，非 bug）。

## RECOMMENDED FIRST BATCH（满足全部六条件：真实歧义+高概率故障+范围小+边界明确+兼容可控+可测可看门）
- **Phase 1 Browser & Grid**：引入 `exitGrid(mode:'hide'|'destroy')` 意图；收敛组件手拼宫格生命周期（RULE-004）；守 `isBrowserVisible` 公式（RULE-002）；守 gridOpen 仅两处赋值（RULE-001）；修 `mainView` 裸赋值（RULE-003）。详见 `12-MIGRATION-PLAN.md` Phase 1 与 `13-ACCEPTANCE-MATRIX.md`。

## CHECKER OPPORTUNITIES（11-CHECKER-DESIGN.md，RULE-001..013）
- 已满足只需看守：RULE-001/006/009/010。
- 修脚本即可：RULE-002/005。
- 需整改源码：RULE-003/004/007/011/012/013。
- 建议新增元检查：检查器之间对同一符号的矛盾断言（SMF-005 暴露）。

## DECISIONS REQUIRED（经第二轮终审 18 裁定，状态更新）
1. **[已裁决·采纳]** ADR-SEVERITY-001 统一 S 评级。终审保留 PROPOSED，须与 `semantic-governance/README.md:56` 二选一；最终评级唯一真源 = `18` §4（S4=2/S3=8/S2=6/S1=3）。
2. **[已裁决·收窄 Context]** ADR-CHECKER-CONFLICT-001 解决 SessionCloseDialog 检查器矛盾。真实冲突**仅 `check-ui.mjs` 一方**（且其 `[CURRENT]` 块因组件已撤销而潜伏）；`check-session-persistence-policy.py` 站"撤销"一侧（非矛盾方）；canonical rule = SessionCloseDialog 已撤销、不得存在，由 `check-native-webview-overlay.mjs` + `check-session-persistence-policy.py` 看守。**仅 `check-ui.mjs` 待授权后修订**（本次未改 scripts）。
3. **[已裁决·READY=NO]** Phase 0/1 尚未就绪：存在 `move_path` BROKEN、两门禁脚本未接线、RULE-012/013 检测手段失效等 PHASE_0_BLOCKERS（18 §11）。建议先清阻断项再 proceed。
4. **[已裁决·维持休眠占位并显式纳入 KNOWN]** 7 个未落地 agent/skill 命令（第二轮核实为 7 非 5）：因 `AGENT_SKILL_COMMANDS_AVAILABLE=false` 不运行时触发，维持休眠占位，但须显式纳入 `check-command-set-consistency.py` 的 KNOWN 并修正正则盲区（18 SECOND-CONFLICT-007）。
5. **[已裁决·纳入 Phase 5]** 凭据 facade（useCredentialStore）纳入 Phase 5（18 §6，ADR-CREDENTIAL-001 维持 PROPOSED）。
6. **[新增·未裁决]** `gridOpen=true && mainView='browser'` 法律地位（HIDE 优先 vs DESTROY 优先）—— 决定 `exitGrid` 语义，PHASE_1_BLOCKER-1（18 §9#13）。

## 总体结论
第一轮 Review（`16`）的结论已被**第二轮对抗性终审（`17A–17F` + `18-SECOND-REVIEW-SYNTHESIS.md`）推翻/修正至少 8 处事实级错误**（含 `BROKEN=0`、`05 S4=0`、三方检查器矛盾、`ADR-GRID-001` L18 覆盖声明等，见 `18` §8）。**最终权威以 `18` 为准**：全局风险计数 `S4=2 / S3=8 / S2=6 / S1=3`，FINAL_S4 仅 GRID_EXIT_DIVERGENCE 与 NATIVE_VISIBILITY_HIDDEN_STATE。项目**尚未 Ready for Phase 0/1**（`18` §13：`READY_FOR_PHASE_0=NO`），须先清 `move_path` ACL 缺失、两门禁脚本接入、RULE-012/013 检测手段更换等阻断项。语义收敛路线见 `12-MIGRATION-PLAN.md`，但须按 `18` §9/§10 的 13+13 项文档与计划更正同步修正（含 `move_path` P0-0 一行补 ACL）。
