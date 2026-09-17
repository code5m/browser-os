# 17F · ADR / 目标契约 / 迁移计划二次评审（Reviewer F）

> 评审对象：`10-TARGET-SEMANTIC-CONTRACTS.md`、`12-MIGRATION-PLAN.md`、`14-DECISIONS.md`，交叉校验 `13-ACCEPTANCE-MATRIX.md`
> 仓库：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> READ-ONLY，未修改 `src/`、`src-tauri/`、`scripts/`、`tests/` 或 docs 00–16。
> 纪律：**FACT ≠ INFERENCE；INFERENCE ≠ RECOMMENDATION；RECOMMENDATION ≠ ACCEPTED DECISION**。

---

## 0. 分类口径

| 分类 | 判定门槛 |
|---|---|
| **FACT** | 可由源码/脚本当前内容直接观测（须给 file + symbol + 行范围） |
| **INFERENCE** | 由多个 FACT 推导，未被直接观测 |
| **RECOMMENDATION** | 建议动作，未被采纳为决策 |
| **PROPOSED ADR** | 已写入 14-DECISIONS 但 Status = PROPOSED |
| **ACCEPTED ADR** | Status = ACCEPTED 的长期契约 |

**本轮最重要的两类缺陷**：
1. **ACCEPTED ADR 被写成"已验证的现状"**：ADR 是政策，但其 `Why`/`Canonical semantics` 用"复核确认已如此"的语气，而源码只在**部分路径**成立。
2. **PLANNED checker 被写成"已有验收"**：13/12 引用了**根本不存在的脚本**，构成"看似有自动化守门、实际零保护"。

---

# TASK 1 — 逐条语句分类

## 1.1 `10-TARGET-SEMANTIC-CONTRACTS.md`

| # | 语句 | 分类 | 源码真相 | 判定 |
|---|---|---|---|---|
| 10-1 | 「isBrowserVisible 当前真义是 `layout.mainView === "browser"`（:149-151）」 | FACT | 吻合 | CONFIRMED |
| 10-2 | 「G2(L112)…属 checker 漂移，需修脚本」 | FACT（漂移）+ INFERENCE（修法） | G2 确断言旧公式；但**该脚本未接入任何门禁** ⇒ "当前 FAIL"不导致门变红，文档未披露 | PARTIALLY_CONFIRMED |
| 10-3 | 「gridOpen 仅两处赋值 :319/:486」 | FACT | 全仓唯一 | CONFIRMED |
| 10-4 | 「可见 = gridOpen && mainView==='grid'，且此派生**未单独存储**」 | FACT + 半真 | 表达式存在，但**分散 inline ≥5 处**，无单一派生常量；且第三标志 gridToolbarOpen 客观上共同驱动 | PARTIALLY_CONFIRMED |
| 10-5 | 「bridge.tabClose/tabNew/… 仅出现在 useBrowserStore.ts」 | FACT | 吻合 | CONFIRMED |
| 10-6 | 「5 个 skill/agent 命令为休眠契约占位」 | FACT | bridge.ts:102=false；src-tauri 无同名 fn | CONFIRMED |
| 10-7 | CONTRACT-GRID-LIFECYCLE「本契约当前**满足**」 | FACT + 外推 | 写入点收敛；但契约含"DESTROY 只经显式 close"，而存在非显式 destroy 入口 | **OVERSTATED** |
| 10-8 | 「VISIBLE→HIDDEN: 视图切换 → hideAllWebviews/HideWindow（**不 kill**）」 | **TARGET 写成现状矩阵** | watch→syncViewVisibility 确实不 kill；但 `HomeLaunchers.openArea` 会 kill | **CONTRADICTED（作为现状描述）** |
| 10-9 | CONTRACT-BROWSER-VISIBILITY「源码公式已正确…脚本当前 FAIL」 | FACT | 见 10-1/10-2 | CONFIRMED（含披露缺口） |
| 10-10 | CONTRACT-VIEW-SWITCH 裸赋值清单 | FACT | 与全仓命中集合**完全一致** | CONFIRMED（高保真） |
| 10-11 | CONTRACT-VIEW-SWITCH 整体 | RECOMMENDATION | 已自标，分类正确 | CONFIRMED（分类正确） |
| 10-12 | CONTRACT-GRID-EXIT | RECOMMENDATION | 已自标目标态并给 drift | CONFIRMED（分类正确） |
| 10-13 | CONTRACT-IPC-CONTRACT 正则盲区 | FACT + RECOMMENDATION | 正则确 untyped-only，KNOWN 仅 4 项 | CONFIRMED |
| 10-14 | 「hide 经屏外 -30000，绝不 set_visible(false)，避免死锁（bridge.rs:567-569）」 | FACT | hide_bounds(570-597)；注释 566-569 | CONFIRMED |
| 10-15 | 「native 命令调用已收敛于两文件」 | FACT | 9 处命中全在这两个文件 | CONFIRMED |
| 10-16 | 「:170 的 -30000 位置当 hide 是待修 hack」 | FACT | useBrowserStore.ts:170 | CONFIRMED |
| 10-17 | 「无 useCredentialStore；CredentialList 直连 bridge」 | FACT | 无该文件；:44/:79-82 直调 | CONFIRMED |
| 10-18 | CONTRACT-TAB-INTENT「已满足」 | FACT | 仅有 store 调用；activateWeb 用精确判定 | CONFIRMED |

## 1.2 `12-MIGRATION-PLAN.md`

| # | 语句 | 分类 | 备注 |
|---|---|---|---|
| 12-1 | Phase 0「check-command-set-consistency.py 修改后须保持 **GATE** 通过」 | **INFERENCE（前提为假）** | 该脚本**未接入** pre-merge.sh / package.json / doctor.mjs |
| 12-2 | Phase 0「现有 pre-merge.sh 全门禁仍 PASS」 | FACT/INFERENCE | 因未接线故未变红；一旦接入即刻 FAIL |
| 12-3 | Phase 0「RULE-012 删 layout.aiNavOpen…列入 Phase 1 非 Phase 0」 | FACT + RECOMMENDATION | 自我矛盾已在括号承认 |
| 12-4 | Phase 1「`mainView` **仍**由 setView 单一写入」 | **TARGET 写成兼容性前提** | 当前 10 处裸赋值 |
| 12-5 | Phase 1 白名单「FileEditor.vue:11、TopBar.vue:21 等」 | RECOMMENDATION（清单不完整） | 漏 store 内部 5 处 + useSessionStore:120 + useWorkspaceStore:912/924 |
| 12-6 | Phase 2 与 Phase 3 均列 `currentLocalPath`（SEM-010）为 Goal | **同一条拆给两阶段** | 归属冲突 |
| 12-7 | Phase 3「破坏型 FS 命令仅经 store」 | FACT（现状已满足）+ RECOMMENDATION | 7 处全在 store，零组件直连 ⇒ Goal 措辞暗示现状违规 |
| 12-8 | Phase 4 终端双出口 | **INFERENCE（未充分验证）** | 未给 file:line |
| 12-10 | Phase 6「所有 RULE-001~013 全绿」 | TARGET DESIGN | 多条对应脚本尚未创建 |

## 1.3 `13-ACCEPTANCE-MATRIX.md`

| # | 语句 | 分类 |
|---|---|---|
| 13-1 | 引用 `check-grid-lifecycle.mjs` | **PLANNED（不存在）**；真实文件是 `check-grid-close-logic.mjs` 且未接入门禁 |
| 13-2 | 「grid → browser（经导航，HIDE）：gridOpen 仍 true」 | **TARGET DESIGN**（当前不成立，见 HomeLaunchers） |
| 13-3 | 引用 check-view-switch / check-tabbar-predicate / check-native-scene-adapter / check-credential-owner / check-grid-exit-intent | **PLANNED（均不存在）** |
| 13-5 | 「G2 改为仅 mainView」 | RECOMMENDATION（方向正确） |

---

## 1.4 ⚠ 显式标记：目标设计被写成当前能力（Target-as-Current）

| 编号 | 文档行 | 应改为 | 真相 |
|---|---|---|---|
| **TAC-01** | 10-TARGET L40「视图切换 → HideWindow（**不 kill**）」 | TARGET DESIGN | `HomeLaunchers.vue:47` 是 kill 型视图切换入口 |
| **TAC-02** | 14-DECISIONS L11「VIEW SWITCH 仅触发 hide…**绝不触发** close_grid/kill_child」 | ACCEPTED ADR（政策）→ 应写"目标不变量，当前存在 1 处已知违反" | 同上 |
| **TAC-03** | 10-TARGET L37「本契约当前**满足**」 | 部分满足（仅写入点） | gridToolbarOpen 共同驱动 |
| **TAC-04** | 14-DECISIONS L43「不改当前赋值点…**仅文档化**」 | 低估 | 仍需第三标志治理与派生去重 |
| **TAC-05** | 12-PLAN Phase 1「`mainView` **仍**由 setView 单一写入」 | TARGET 前提 | 10 处裸赋值 |
| **TAC-06** | 13-MATRIX 引用 `check-grid-lifecycle.mjs` | PLANNED/待建 | 文件不存在 |
| **TAC-07** | README L56「本仓库统一口径」 | PROPOSED 口径泄漏 | 该 ADR Status = PROPOSED |
| **TAC-08** | 12-PLAN Phase 0「须保持 **GATE** 通过」 | 前提为假 | 该 checker 未接入任何 GATE |

---

# TASK 2 — ACCEPTED ADR 证据充分性裁决

## 2.1 汇总

| ADR | Status | 证据充分 | 复核确认 | 替代方案 | 现状 or 目标 | 裁决 | 建议 |
|---|---|---|---|---|---|---|---|
| **ADR-GRID-001** | ACCEPTED | **NO**（只对 1/3 视图切换路径取证） | **NO** | 部分 | **混合**：宣称现状实为目标政策 | **PARTIALLY_CONFIRMED**（作为现状）/ CONFIRMED（作为决策） | 必须改写，否则按 otherwise 降级 |
| **ADR-BROWSER-001** | ACCEPTED | **YES** | **YES** | YES | **现状** | **CONFIRMED** | 保留 ACCEPTED |
| **ADR-GRID-VISIBILITY-001** | ACCEPTED | 部分 | 部分 | 未记录 | **混合** | **PARTIALLY_CONFIRMED** | 保留 ADR，降级 Migration implication |
| **ADR-NATIVE-SHOW-001** | ACCEPTED | **YES** | **YES** | 隐含 | 现状+政策 | **CONFIRMED** | 保留，补 Alternatives |
| **ADR-IPC-001** | ACCEPTED(原则)/修 checker PROPOSED | **YES**（盲区）/ **NO**（执行力） | YES | YES | 目标+现状 FACT | **PARTIALLY_CONFIRMED**（执行力） | 补前置：先接入网关 |
| **ADR-CHECKER-CONFLICT-001** | PROPOSED | **NO**（Context 一半失真） | NO | 未记录 | 混合 | **PARTIALLY_CONFIRMED / OVERSTATED** | 收窄至 check-ui.mjs 单方 |
| **ADR-CREDENTIAL-001** | PROPOSED | **YES** | YES | 未记录 | 现状 FACT + 目标政策 | **CONFIRMED**（分类正确） | 保持 PROPOSED |
| **ADR-SEVERITY-001** | PROPOSED | YES | YES | 隐含 | 目标政策 | **PARTIALLY_CONFIRMED** | 升级 ACCEPTED 或改 README |

## 2.2 逐 ADR 详裁

### ADR-GRID-001 — 视图切换绝不得销毁活动 Grid
- 证据充分？**NO** · 复核确认？**NO** · 现状 or 目标？**目标政策被写成现状禁令**
- ✅ `syncViewVisibility`(646-655) 非 browser/grid 只 hide，无 close/kill。
- ❌ **`HomeLaunchers.vue:42-53` 是第二条视图切换入口**：L43 注释「进入非宫格视图前**先关掉宫格**」，L47 `if (view !== "grid" && browser.gridOpen) await browser.closeGridAll();` → kill 子进程。**直接反例**。
- ⚠ `ActivityBar.onItem`(150-177) 走 setView/openModule，**不销毁** ⇒ 与 HomeLaunchers 分叉。
- ⚠ `toggleGridToolbar`(210-215) 是第三个非显式 destroy 入口。
- ❌ **Checker implication 无依据**：L18 称「check-lifecycle-contract.py 已覆盖」；实际该脚本只定义 `GRID_PARTIAL_CREATE_ROLLBACK_MISSING` 一个缺口码，全篇无 `close_grid`/`kill_child`/`syncViewVisibility` 匹配 ⇒ **零自动保护**。
- ⚠ 16 号文档自相矛盾：CLAIM-LC-01 判"否/CONFIRMED"，而 CONFLICT-03 承认"**Home** 入口触发销毁"。ADR 只引用前者。
- **ADR 内部矛盾**：L11 断言"绝不 kill"，L17 又要求"Phase 1 抽 exitGrid 使意图单一"——若已单一且从不 kill，则无需 exitGrid。
- **要求改写**：① Decision 改为"目标不变量，当前存在 1 处已知违反（HomeLaunchers:47）+ 1 处 UI 标志驱动（toggleGridToolbar:214）"；② Forbidden 增例外；③ 删除「check-lifecycle-contract.py 已覆盖」。

### ADR-BROWSER-001 — 浏览器家族视图 ≠ 纯浏览器主视图
- 证据充分 **YES** · 复核确认 **YES** · **现状**（已实现约束）
- `isBrowserView()`(188-190)=browser‖grid；`isBrowserVisible`(149-151)=mainView==='browser'；`activateWeb`(147-154) 显式用 `mainView !== "browser"` + 注释理由；`isActiveWeb`(157) 合法使用 `isBrowserView()`。
- 观察：`isActiveWeb:157` 仍合法使用 ⇒ 任何"一律禁用 isBrowserView"的门禁会误伤；ADR 应写死边界。
- **裁决：CONFIRMED，维持 ACCEPTED。**

### ADR-GRID-VISIBILITY-001 — gridOpen = lifecycle only
- 证据 **部分** · 复核 **部分** · **混合**
- ✅ 仅 2 处赋值（:319/:486）。
- ❌ "不得单独存储第三个标志"**未被满足**：`gridToolbarOpen`(:144) 经 `toggleGridToolbar`(210-215) 双向驱动 gridOpen ⇒ 事实上的三标志。
- ❌ 派生表达式存在但 **inline 重复 ≥5 处**（useBrowserHost 64/99/109、ActivityBar 358/369；useGridArchiveStore:48 另用 gridSession+gridOpen）⇒ **无单一派生真源**。
- ❌ L43「仅文档化」**低估**：至少还需 (a) 派生单点化；(b) gridToolbarOpen 耦合治理。
- **要求**：① 显式声明"当前是分散 inline 成立，非单一派生"；② Migration implication 降级为 PROPOSED；③ Forbidden 改写为"禁止新增；现有 gridToolbarOpen 列为已知例外，Phase 1 处理"。

### ADR-NATIVE-SHOW-001 — 无独立 show 命令
- 证据 **YES** · 复核 **YES**（4 个机制点逐一重验）· **现状 FACT + 政策**
- `apply_bounds_inner`(531-564) `set_visible(true)`(:555)；`GridCmd::UpdateRect`(391-425) `win.show()`(:423)；`HideWindow`(426-431) 只 `win.hide()`(:430)；全仓 `fn show_webview`/`fn show_all_webviews` **0 命中**；`hide_bounds` 注释 566-569 禁用 `set_visible(false)`（WebKitGTK 死锁），实现为 `update_rect(-30000,…)` 保尺寸。
- 已知例外 `useBrowserStore.ts:170`（黑闪修复）**ADR 处理得当**。
- **裁决：CONFIRMED，维持 ACCEPTED。** 建议补 Alternatives（新增显式 hide 原语已推迟至 Phase 6，应记录而非隐式否决）。

### ADR-IPC-001 — 三源闭包
- 证据 **YES**（盲区）/ **NO**（执行力）· **目标政策 + 现状 FACT**
- ✅ 正则 untyped-only(82-83)；KNOWN 仅 4 个无类型占位；bridge.ts 401/403/418/421/432 typed 占位；`AGENT_SKILL_COMMANDS_AVAILABLE=false`(:102)；src-tauri 无同名 fn。
- ❌ **关键前提被忽略**：`check-command-set-consistency.py` **未接入任何自动门禁**（pre-merge.sh / package.json / doctor.mjs 均无）⇒ 即便修好正则也不会在 GATE 生效。
- **裁决：PARTIALLY_CONFIRMED（执行力）。** 维持"原则 ACCEPTED / 修改 checker PROPOSED"的分级（该分级本身正确），**但 Phase 0 必须增加"接入 pre-merge.sh"**。

### ADR-CHECKER-CONFLICT-001（PROPOSED）
- 证据 **NO**（Context 一半失真）· 复核 **NO**
- ✅ `check-native-webview-overlay.mjs:25-26` 禁止 SessionCloseDialog，且**已在 pre-merge.sh:372-373 生效**。
- ✅ `check-ui.mjs:37/53` 标 "CURRENT 关闭协议"，经 pre-merge.sh:522-527 与 package.json:11 **双向生效**。
- ❌ **对第二个检查器的指控不成立**：`check-session-persistence-policy.py` L19-23/230-231/573-575 明确主张"原关闭协议**必须已撤销**"，`SP_DIALOG_STILL_MOUNTED` 正是对残留的报错 ⇒ **它站在 check-native 一侧，不是冲突方**。
- ⚠ 冲突当前为**潜伏态**：`src/` 下已不存在该组件，`check-ui.mjs:223` 的 existsSync 守卫使断言静默跳过。
- **要求**：① Context 收窄为"仅 check-ui.mjs"；② Decision 只清 check-ui.mjs:37/:53 + :222-241；③ 补"潜伏态"说明；④ 保留 PROPOSED。

### ADR-CREDENTIAL-001（PROPOSED）
- 证据 **YES** · 复核 **YES** · **现状 FACT + 目标政策**
- 全仓无 `useCredentialStore`；`CredentialList.vue:44`/`:79-82` 直连；红线注释 :10-15 + 组件 ref :20-23 + pageOrigin :26-34 真实。
- **裁决：CONFIRMED，分类正确，维持 PROPOSED**（facade 不存在，不能标 ACCEPTED）。建议补 Alternatives（仅加 lint、不引入 store 的轻量方案）。

### ADR-SEVERITY-001（PROPOSED）
- 证据 **YES**（16 的 CONFLICT-02/03/04 确为口径差异）· **目标政策（零运行时风险）**
- **分类泄漏**：README:56 已写「本仓库统一口径」，:62 已按该提议执行降级 ⇒ **一个 PROPOSED ADR 已在事实层面生效**。
- **建议**：升级至 ACCEPTED 并同步 14 的 Status；或把 README 改为"待裁决提议"。二者必选其一。

---

# TASK 3 — `12-MIGRATION-PLAN.md` 评审

| Phase | 增量 | Scope | Forbidden | 回滚 | 验收 | 静默依赖 | 裁决 |
|---|---|---|---|---|---|---|---|
| Phase 0 | ✅ | ✅ | ✅ | ✅ | ⚠ | ❌ 2 项 | PARTIALLY_CONFIRMED |
| Phase 1 | ✅ | ⚠ 白名单不全 | ✅ | ⚠ | ⚠ 依赖不存在脚本 | ❌ 3 项 | PARTIALLY_CONFIRMED |
| Phase 2 | ✅ | ⚠ 与 P3 重叠 | ✅ | ✅ | ⚠ | ❌ 1 项 | PARTIALLY_CONFIRMED |
| Phase 3 | ✅ | ⚠ 目标已达成 | ✅ | ✅ | ⚠ | ❌ 1 项 | **UNDERSTATED** |
| Phase 4 | ⚠ 未验证 | ⚠ 无 file:line | ⚠ | ✅ | ⚠ | ❌ 1 项 | **INSUFFICIENT_EVIDENCE** |
| Phase 5 | ✅ | ✅ | ✅ | ✅ | ⚠ | ❌ 1 项 | PARTIALLY_CONFIRMED |
| Phase 6 | ✅ 可选 | ✅ | ✅ | ⚠ | ✅ | ⚠ | CONFIRMED（可选） |

## 与源码现实的矛盾项（必须修正）

- **MP-01（Phase 0）** L14「须保持 GATE 通过」前提为假 → 该脚本未接入门禁。Allowed scope 增加"接入 pre-merge.sh"。
- **MP-02（Phase 0，隐藏地雷）** L15「全门禁仍 PASS」成立，但原因是**未接线**而非健康；`check-grid-close-logic.mjs` G2 与当前源码直接冲突，一旦接入即刻 FAIL。须写明"接入 G2 前必须先修 G2"。
- **MP-03（Phase 1）** 白名单只列 2 个组件点，漏 store 内部 5 处 + useSessionStore:120 + useWorkspaceStore:912/924。
- **MP-04（Phase 1）** L30「`mainView` **仍**由 setView 单一写入」是迁移后承诺，非现状。
- **MP-05（Phase 1）** 引用 `check-grid-exit-intent.mjs`、`check-view-switch.mjs`、`check-grid-lifecycle.mjs` —— **均不存在**；须标 [PLANNED]，回滚边界不得依赖。
- **MP-06（Phase 1，最大静默风险）** ADR-GRID-001 宣称"视图切换不销毁"，但 Phase 1 未包含修 `HomeLaunchers.openArea:47` ⇒ Phase 1 结束时 ADR 仍处于违反态，而"往返 ×20"runtime acceptance 仍可能通过（走的是 ActivityBar/TopBar 路径）。
- **MP-07** SEM-010 `currentLocalPath` 同时归属 Phase 2 与 Phase 3 → 单一归属。
- **MP-08（Phase 3）** 7 处 FS bridge 调用**全部**已在 useWorkspaceStore，零组件直连 ⇒ Goal 改为"为已达成的不变量补 RULE-010 看门狗"。
- **MP-09（Phase 4）** 终端双出口未给 file:line，规则代号"待补" ⇒ 先补证据再列入可执行计划。

---

# SR-EVID 证据登记（Reviewer F）

**SR-EVID-0001** Claim: `syncViewVisibility` 非 browser/grid 只隐藏不销毁。 Reviewer: F Classification: FACT
File: useBrowserStore.ts Symbol: syncViewVisibility Line: 646-655
Observed: browser/grid 分支 hideAllWebviews+relocate；其它分支 hideAllWebviews。无 kill/close。
Caller: watch(mainView)(658-664) Callee: bridge.hideAllWebviews/relocate/syncFreeze
Why: **支持** ADR-GRID-001 所引路径的一半；**不能外推**到全部视图切换入口。Confidence: HIGH

**SR-EVID-0002** Claim: 存在第二条"视图切换会销毁宫格"的入口。 Reviewer: F Classification: FACT
File: HomeLaunchers.vue Symbol: openArea Line: 42-53（注释 43，代码 47）
Observed: `if (view !== "grid" && browser.gridOpen) await browser.closeGridAll();`
Callee: closeGridAll → bridge.closeGrid → close_grid Side effects: **DESTROY**（杀子进程 ≈450MB/格）
Why: **直接反驳** ADR-GRID-001 L11/L14 与 10-TARGET L40。Confidence: HIGH

**SR-EVID-0003** Claim: 导航类视图切换只隐藏 → 与 HomeLaunchers 分叉；且存在过期注释。 Reviewer: F Classification: FACT
File: ActivityBar.vue Symbol: onItem Line: 150-177（过期注释 156-157）
Observed: grid 分支 openModule("grid")+layoutGrid/buildGrid；browser 分支仅 setView("browser")，不调 closeGridAll。注释"离开宫格视图时自动关闭宫格"当前**无对应实现**。Confidence: HIGH

**SR-EVID-0004** Claim: 第三个非显式 destroy 驱动者（gridToolbarOpen）。 Reviewer: F Classification: FACT
File: useLayoutStore.ts Symbol: toggleGridToolbar Line: 210-215
Observed: 翻转后 ON→buildGrid()，OFF→closeGridAll()。Why: 挑战 ADR-GRID-VISIBILITY-001"现状已收敛"。Confidence: HIGH

**SR-EVID-0005** Claim: closeGridAll 含过期注释。 Reviewer: F Classification: FACT
File: useBrowserStore.ts Line: 483-510（注释 490-494）— 仍写 `isBrowserVisible = !gridOpen && mainView==="browser"`。Confidence: HIGH

**SR-EVID-0006** Claim: gridOpen 全仓仅 2 处赋值。 Reviewer: F Classification: FACT Line: :20/:319/:486；其余全为读取。Confidence: HIGH

**SR-EVID-0007** isBrowserVisible 仅 mainView==='browser'，不含 !gridOpen（149-151）。FACT HIGH
**SR-EVID-0008** isBrowserView() 含 grid（188-190）。FACT HIGH
**SR-EVID-0009** activateWeb 刻意不用 isBrowserView；isActiveWeb(157) 仍合法使用（UnifiedTabBar 147-158）。FACT HIGH

**SR-EVID-0010** Claim: "可见 = gridOpen && mainView==='grid'" 存在但分散 inline，且有第三标志参与。 Reviewer: F Classification: FACT
File: useBrowserHost.ts:64/98-99/109；ActivityBar.vue:358/369；useBrowserStore.ts:625；useGridArchiveStore.ts:48
Observed: 同一逻辑式在 ≥5 处独立重复，**无单一派生常量**。Confidence: HIGH

**SR-EVID-0011** position 蕴含 show（bridge.rs apply_bounds_inner 531-564，:555 set_visible(true)）。FACT HIGH
**SR-EVID-0012** hide 一律 -30000 并明文禁用 set_visible(false)（hide_bounds 566-597；注释 566-569；:576 X11 int16 说明）。FACT HIGH
**SR-EVID-0013** 子进程侧无独立 show：UpdateRect `win.show()`(:423)，HideWindow 只 `win.hide()`(:430)（main.rs 391-431）。FACT HIGH
**SR-EVID-0014** 原生 API 不对称：`hide_webview`/`hide_all_webviews` 存在；`fn show_webview`/`fn show_all_webviews` 全仓 0 命中。FACT HIGH
**SR-EVID-0015** position-as-hide hack 存在（useBrowserStore.ts:170 tabPosition x:-30000，注释解释黑闪修复）。FACT HIGH
**SR-EVID-0016** native 显隐/定位命令 9 处调用全落 useBrowserStore/useBrowserHost，零组件直连。FACT HIGH
**SR-EVID-0017** IPC 门禁正则仅 untyped（check-command-set-consistency.py:82-83）；KNOWN 仅 4 项（42-48）。FACT HIGH
**SR-EVID-0018** 5 命令为休眠占位，FE 运行期拦截（bridge.ts:102 / :401,403,418,421,432 / 注释 399-400）。FACT HIGH
**SR-EVID-0019** src-tauri 全仓无 `fn skill_list|agent_list|skill_install|agent_install|skill_run`（0 命中）。FACT HIGH

**SR-EVID-0020** Claim: IPC 门禁与 grid 门禁脚本**未接入任何自动门禁**。 Reviewer: F Classification: FACT
File: pre-merge.sh（接入列表 74-110，pm_fail 216-527）/ package.json:11 / doctor.mjs:22-28
Observed: pre-merge 执行 check-lifecycle-contract.py(:217-225)、check-session-persistence-policy.py(:282-285)、check-native-webview-overlay.mjs(:372-373)、Phase03 gate(:522-527)；**完全没有** check-command-set-consistency.py 与 check-grid-close-logic.mjs。
Why: **反驳** 12-PLAN Phase 0 L14 前提；**支持** ADR-IPC-001"零保护"。Confidence: HIGH

**SR-EVID-0021** G2 断言旧公式（check-grid-close-logic.mjs:106-126，关键 112/117 仿真）→ 对当前源码必 fail。FACT HIGH

**SR-EVID-0022** Claim: check-ui.mjs 保留 [CURRENT]，但断言因目标文件不存在而静默失效。 Reviewer: F Classification: FACT
File: check-ui.mjs:37/53（白名单标 "PROJECT-RULES.md [CURRENT]"）、:222-241（结构断言）、:196/:223（existsSync 守卫）；`src/` 下 SessionClose* **0 命中**
Why: 冲突真实但**潜伏**。Confidence: HIGH

**SR-EVID-0023** Claim: `check-session-persistence-policy.py` 实为**撤销口径**，不是 [CURRENT]。 Reviewer: F Classification: FACT
File: :19-23「原关闭协议…**必须已撤销、不得残留**」；:230-231 `SP_DIALOG_STILL_MOUNTED`；:573-575 负向自测
Why: **直接反驳** ADR-CHECKER-CONFLICT-001 Context L74 与 16 CLAIM-XC-01"两个检查器都标 [CURRENT]"。Confidence: HIGH

**SR-EVID-0024** 禁止侧 check-native-webview-overlay.mjs 已在门禁生效（:25-26 + pre-merge.sh:371-373）。FACT HIGH
**SR-EVID-0025** 凭据无前端 store，组件直连 bridge，红线注释真实（CredentialList.vue:10-15/20-23/26-34/44/68-89）。FACT HIGH

**SR-EVID-0026** Claim: PROPOSED ADR 泄漏为规范性口径。 Reviewer: F Classification: FACT
File: README.md:56「本仓库统一口径」+ :62 已按提议降级；14-DECISIONS:99「Status: PROPOSED」
Why: 构成 TAC-07。Confidence: HIGH

**SR-EVID-0027** 外部裸 `layout.mainView =` 赋值集合与 doc-10 清单**完全一致**（useBrowserStore 161/207/323/503/545；useSessionStore 120；useWorkspaceStore 912/924；FileEditor 11；TopBar 21）。FACT HIGH

**SR-EVID-0028** `layout.aiNavOpen` 是零消费者死重复（useLayoutStore 146/338；消费者全用 browser.aiNavOpen）。FACT HIGH

**SR-EVID-0029** 破坏型 FS bridge 调用已 100% 收敛于 store（useWorkspaceStore 275/317/335/351/684/739/929；零组件直连）。FACT HIGH

**SR-EVID-0030** Claim: `check-lifecycle-contract.py` **并未**覆盖"视图切换不得销毁"。 Reviewer: F Classification: FACT
File: check-lifecycle-contract.py:16（唯一缺口码 GRID_PARTIAL_CREATE_ROLLBACK_MISSING）、46-66
Observed: 全篇对 close_grid/kill_child/syncViewVisibility **0 命中**
Why: **直接反驳** ADR-GRID-001 L18「已覆盖」⇒ 该红线当前**零自动保护**。Confidence: HIGH

**SR-EVID-0031** 16 号报告对同一事实两种口径，ADR-GRID-001 选择性引用（CLAIM-LC-01:70-73 vs CONFLICT-03:223-226 承认 Home 入口销毁）。FACT HIGH

**SR-EVID-0032** useBrowserStore.ts:184-190 Owner 裁决注释真实（"M1-9 关闭协议已撤销"）。FACT HIGH

**SR-EVID-0033** Claim: 13/12 引用的多个 checkers **不存在**。 Reviewer: F Classification: FACT
File: scripts/ 目录清单（31 个 check-*.mjs）
**不存在**：check-grid-lifecycle.mjs、check-grid-exit-intent.mjs、check-view-switch.mjs、check-tab-intent.mjs、check-tabbar-predicate.mjs、check-native-scene-adapter.mjs、check-credential-owner.mjs、check-fs-intent.mjs、check-terminal-intent.mjs
Why: 构成 TAC-06 与 MP-05。Confidence: HIGH

---

# 最终结论

## 4.2 ACCEPTED ADR 确认 vs 降级计数
- **ACCEPTED ADR 总数：5**
- **维持 ACCEPTED 且无需更正：2** → ADR-BROWSER-001、ADR-NATIVE-SHOW-001
- **维持 ACCEPTED 但必须更正指定条款：3** → ADR-GRID-001、ADR-GRID-VISIBILITY-001、ADR-IPC-001
- **建议降级的条目：2（子条款级）**：
  - ADR-GRID-VISIBILITY-001 的 Migration implication「仅文档化」→ **PROPOSED**
  - ADR-GRID-001 的 Checker implication「check-lifecycle-contract.py 已覆盖」→ **PROPOSED（待建 RULE-004）**
- **若不应用更正则必须整条降级：1** → ADR-GRID-001（Decision 与现实存在未登记的反例）
- **分类不一致需二选一：1** → ADR-SEVERITY-001（PROPOSED）vs README「本仓库统一口径」

## 4.3 目标契约（10 / 13）必须更正
1. 10-TARGET L40 补入"视图切换**可能** DESTROY（HomeLaunchers.openArea）"这条边。
2. 10-TARGET L37「当前满足」限定为"仅 gridOpen 赋值点满足"，点名 gridToolbarOpen 第三标志。
3. 10-TARGET L12「此派生未单独存储」改为"以 inline 形式重复于 ≥5 处，无单一派生真源"。
4. 10-TARGET L11/L57 增加说明：`check-grid-close-logic.mjs` **未接入任何门禁**。
5. 13-MATRIX L16/L30 的 `check-grid-lifecycle.mjs` 不存在 → 改为 `check-grid-close-logic.mjs` 并标 [PLANNED]。
6. 13-MATRIX 所有引用 checker 的单元格加 [PLANNED]。
7. 13-MATRIX L15「grid→browser HIDE：gridOpen 仍 true」标注为**目标态**。
8. README L56 与 ADR-SEVERITY-001 的 PROPOSED 状态对齐。

## 4.4 迁移计划（12）必须更正
1. Phase 0：Allowed scope 增加"把 check-command-set-consistency.py 接入 pre-merge.sh"。
2. Phase 0：写明"G2 当前对源码 FAIL 且未接线；接入前必须先修 G2"。
3. Phase 1：裸赋值白名单补全至 doc-10 自身清单。
4. Phase 1：L30 改为迁移后承诺。
5. Phase 1：明确把 `HomeLaunchers.openArea:47` 纳入出口。
6. Phase 2/3：SEM-010 单一归属。
7. Phase 3：Goal 改为"为已达成的不变量补 RULE-010 看门狗"；可考虑与 Phase 2 合并。
8. Phase 4：先补 file:line 与规则代号。
9. 全局：Rollback boundary 不得依赖 [PLANNED] 脚本。

## 4.5 一句话总结

> 本轮最需防止的误读是：**"ADR-GRID-001 已被接受，所以视图切换当前不会销毁宫格"** —— 这是错的。`syncViewVisibility` 那条路径确实只隐藏，但 `HomeLaunchers.openArea:47` 这条同样是视图切换入口的路径会 `closeGridAll()`；同时**没有任何检查器**（包括 ADR 声称"已覆盖"的 `check-lifecycle-contract.py`）在执行这条红线，相关验收脚本也尚未接入 `pre-merge.sh`。请把 ADR-GRID-001 明确落位为**带已知违反清单的目标政策**，并把"接入门禁"显式写入 Phase 0。

**评审者**：F · READ-ONLY，未修改 `src/`、`src-tauri/`、`scripts/`、`tests/` 与 docs 00–16 中任一文件。
