# 14 · Architecture Decisions — 语义治理决策
> Chief（只读审计合成）· 仅收录"已有足够证据支撑、值得成为长期契约"的决定。
> 格式：ADR-XXX / Title / Status(PROPOSED|ACCEPTED) / Context / Observed failure / Decision / Canonical semantics / Allowed / Forbidden / Why / Evidence / Migration implication / Checker implication。

---

## ADR-GRID-001 — 视图切换绝不得销毁活动 Grid 实例
- **Status**: ACCEPTED（目标不变量；**当前存在 1 处已知违反**，18 终审要求改写为"带已知违反清单的目标政策"，否则整条降 PROPOSED）
- **Context**: 宫格子进程重（≈450MB/格），误销毁会丢失瞬态页面状态且开销巨大。
- **Observed failure**: 历史"切走视图后宫格/页签残留屏幕"与"切回后空白"均源于显隐与销毁混淆。
- **Decision**: VIEW SWITCH 仅触发 hide（屏外移 / HideWindow），绝不触发 `close_grid`/`kill_child`。销毁只经显式 close/destroy 生命周期。
- **Known violation（★ 18 SECOND-CONFLICT-001）**: `HomeLaunchers.openArea:47` 在切到非宫格视图前 `await browser.closeGridAll()`（→`shutdown_all` kill），是**活的视图切换型 DESTROY**，构成 FINAL_S4-1 GRID_EXIT_DIVERGENCE；活分叉出口仅此 1 处（`toggleGridToolbar` 为死导出、`ActivityBar:398` 为显式关闭按钮）。
- **Canonical semantics**: "离开宫格"意图须收敛到单一 `exitGrid(mode:'hide'|'destroy')`；当前由调用方组件隐式选择 HIDE/DESTROY。
- **Allowed**: HIDE（offscreen，`hideAllWebviews`/`HideWindow`）、SHOW（经 position）。
- **Forbidden**: 视图切换路径调用 `close_grid`/`close_tab`（除已登记的 `HomeLaunchers:47` 违反点，须迁移）。
- **Why**: `syncViewVisibility`（useBrowserStore.ts:646-654）只隐藏；但 `HomeLaunchers:47` 独立违反（18 `SR-EVID-FR-0001`）。
- **Evidence**: 16 CLAIM-LC-01/02（Tab/PTY 侧成立）；05 EVID-GR-03；07 EVID-HIDE-007；18 SECOND-CONFLICT-001 / `SR-EVID-FR-0001`。
- **Migration implication**: Phase 1 抽 `exitGrid(mode)`；`HomeLaunchers.openArea:47` 显式纳入出口迁移清单（18 §10#6）；先建目标 API 再上门禁（RULE-004 rejected-for-Phase-0）。
- **Checker implication**: RULE-006/007（native 命令仅经适配）。**删除原"check-lifecycle-contract.py 已覆盖"——18 SECOND-CONFLICT-009 证实该脚本对 `close_grid`/`kill_child`/`syncViewVisibility` 0 命中，覆盖声明为事实错误。**

## ADR-BROWSER-001 — 浏览器家族视图 ≠ 纯浏览器主视图
- **Status**: ACCEPTED
- **Context**: `isBrowserView()`（browser||grid）与 `isBrowserVisible`（仅 browser）命名相近但语义不同。
- **Observed failure**: `UnifiedTabBar.activateWeb` 若误用 `isBrowserView()`，grid 视图下点击页签失效（SMF-001 / CASE-001）。
- **Decision**: 两谓词刻意不同义，保留；所有调用点显式区分"浏览器类视图"与"严格浏览器可见"。
- **Canonical semantics**: `isBrowserView = mainView==='browser' || 'grid'`；`isBrowserVisible = mainView==='browser'`。
- **Allowed**: 二者并存。
- **Forbidden**: 在 `activateWeb` 中用 `isBrowserView()`；在 BrowserHost 可见性中混入 grid。
- **Why**: 复核确认两谓词定义（16 CLAIM-S4-06 DUP-001；01 EVID-0002/0003）。
- **Evidence**: 01 EVID-0002/0003/0004；08 DUP-001/009；09 SMF-001。
- **Migration implication**: 无（现状正确，仅看守 + 改名消歧义可选）。
- **Checker implication**: RULE-008（activateWeb 禁用 isBrowserView）。

## ADR-GRID-VISIBILITY-001 — gridOpen 表示生命周期，可见性单独派生
- **Status**: ACCEPTED
- **Context**: `gridOpen` 长期被误读为"可见"。
- **Observed failure**: 若把"可见"等同于 `gridOpen`，则当 gridOpen=true 但 mainView≠grid（停留 browser 视图但宫格未关）会误判可见（SEM-004 / DUP-002）。
- **Decision**: `gridOpen` = 宫格实例/进程是否存在（CREATE 后置 true，DESTROY 后置 false）。"可见" = `gridOpen && mainView==='grid'`，不得单独存储第三个标志。
- **Canonical semantics**: 生命周期与可见解耦（CONTRACT-GRID-LIFECYCLE）。
- **Allowed**: `gridOpen` 仅两处赋值（buildGrid/closeGridAll）。
- **Forbidden**: 把 gridOpen 当可见短路；新增第三标志。
- **Why**: 复核确认 gridOpen 仅 2 处赋值（16 CLAIM-MT-03；10 复核）。
- **Evidence**: 01 EVID-0005；02 SEM-004；08 DUP-002；10 CONTRACT-GRID-LIFECYCLE。
- **Migration implication**: 不改当前赋值点（已收敛），仅文档化。
- **Checker implication**: RULE-001。

## ADR-NATIVE-SHOW-001 — 无独立 show webview 命令；show 经 position 隐式完成
- **Status**: ACCEPTED（架构约束，18 终审确认）
- **Context**: 隐藏用屏外 `-30000` 移出（避免 `set_visible(false)` 死锁，bridge.rs:567-569）；显示只能经重定位。底层 native/bridge **hide capability 已存在**（`GridCmd::HideWindow` main.rs:426；`hideWebview` bridge.rs:608；`hideAllWebviews` :635）。
- **Observed failure**: position 命令携带未声明的 show 副作用（07 S4-005，**18 SECOND-CONFLICT-004 由 S4 降 S3**：机制耦合、无静默错误）；IPC 契约表未声明副作用（CONFLICT-01 / 06 Hidden Side-Effect Column 已补）。
- **Decision**: 承认 position==show 机制耦合；隐藏一律走 hide_webview/hide_all_webviews/HideWindow（offscreen）；frontend 不得 `set_visible(false)`。
- **Canonical semantics**: 显示 = position→set_visible(true)/win.show()；隐藏 = offscreen move。
- **Allowed**: 现有 position/hide 命令；`syncViewVisibility` 单一适配。
- **Forbidden**: `set_visible(false)`；position-as-hide hack（除已存在 :170 黑闪修复需重构）。
- **Why**: 复核确认（16 CLAIM-LC-04；07 EVID-005-TAB/GRID）。
- **Evidence**: 05 EVID-LY-02；07 EVID-005-TAB/GRID/SHOW-007；10 CONTRACT-NATIVE-SHOW；18 SECOND-CONFLICT-004。
- **Migration implication**: Phase 6 可选落地 `sync_browser_scene` 单一适配器。**Alternatives（18 要求记录而非隐式否决）：native hide 原语已存在，无需新增；缺的是业务级 canonical hide / view-switch entrypoint（`exitGrid`），而非 Rust `HideWindow`。**
- **Checker implication**: RULE-006（native 命令仅经适配）；**RULE-007 经 18 复审应 rejected**——它禁止的正是官方 `-30000` 屏外隐藏机制（PROJECT-RULES.md:71,76 明文），降级为 advisory。

## ADR-IPC-001 — 跨层 IPC 必须三源闭包，门禁覆盖 typed invoke
- **Status**: ACCEPTED（原则）/ 修正 checker PROPOSED
- **Context**: `check-command-set-consistency.py` 正则只匹配 untyped invoke，使 5 个未落地 agent/skill 命令零保护（06 EVID-013/015）。
- **Observed failure**: 未来"单边改 FE 或 Rust 签名"会在 IPC 字符串边界静默失配（SMF-002）。
- **Decision**: 每个 FE `invoke` ↔ Rust `#[tauri::command]` ↔ ACL 三源闭包；看门狗正则改为 `invoke(?:<[^>]*>)?\(\s*["\']...`；5 命令入 KNOWN 或落地后端。
- **Canonical semantics**: 单一 IPC 契约闭包（CONTRACT-IPC-CONTRACT）。
- **Allowed**: 已注册 + ACL 放行的命令；`AGENT_SKILL_COMMANDS_AVAILABLE=false` 拦截休眠占位。
- **Forbidden**: FE invoke 指向无 Rust 实现且未入 KNOWN；create_grid 单边改签名（须保留 `urls`）。
- **Why**: 复核确认正则盲区（16 CLAIM-IPC-04/05）。
- **Evidence**: 06 EVID-013/014/015/027/028；16 CLAIM-IPC-01~06。
- **Migration implication**: Phase 0 修 checker。
- **Checker implication**: RULE-005。

## ADR-CHECKER-CONFLICT-001 — SessionCloseDialog 已撤销，检查器口径统一（Context 收窄）
- **Status**: PROPOSED（解决 09 SMF-005 / 16 CLAIM-XC-01 冲突；**18 SECOND-CONFLICT-008 已收窄 Context**）
- **Context（修正）**: 第一轮称 `check-ui.mjs` 与 `check-session-persistence-policy.py` 都标 `[CURRENT]` 与 `check-native-webview-overlay.mjs` 矛盾（"三方矛盾"）。**18 终审推翻一半论据**：`check-session-persistence-policy.py` 实际**站在"撤销"一侧**（L21-23 文档串"必须已撤销、不得残留" + L230-231 `SP_DIALOG_STILL_MOUNTED` 违规检测器），**并非矛盾方**。
- **真实冲突（仅一方，且潜伏）**: `check-ui.mjs` L37/L53 仍把 SessionCloseDialog.vue 标 `[CURRENT] 关闭协议`，且 L222-243 的挂载断言块被 `existsSync(sessionClosePath)` 守卫——因该组件**已在 src/ 撤销不存在**，此块**静默跳过（潜伏态）**，不实际执行。
- **Observed failure**: `check-ui.mjs` 的 `[CURRENT]` 标注与"组件已撤销"现实脱节；若组件被误复活，该脚本会要求保留它，与 check-native 矛盾。
- **Decision**: canonical rule = **SessionCloseDialog 已撤销、不得存在**；由 `check-native-webview-overlay.mjs`（`assert.doesNotMatch(app, /SessionCloseDialog/)`）+ `check-session-persistence-policy.py`（若仍挂载报 `SP_DIALOG_STILL_MOUNTED`）共同看守。**仅 `check-ui.mjs` 需修订**（删 L37/L53 `[CURRENT]` 标注 + L222-243 死块）。
- **Canonical semantics**: SessionCloseDialog 不是当前关闭协议（已由 useBrowserStore.ts:185-187 撤销，2026-09-12 裁决）。
- **Allowed**: 单一真相源 = check-native-webview-overlay（断言不存在）+ check-session-persistence-policy（若残留则报错）。
- **Forbidden**: 任何检查器把 SessionCloseDialog 描述为 `[CURRENT]`（仅 check-ui.mjs 残留）。
- **Why**: 18 SECOND-CONFLICT-008 / `SR-EVID-FR-0014` 推翻"三方矛盾"一半论据。
- **Evidence**: 09 SMF-005；16 CLAIM-XC-01（部分被推翻）；18 SECOND-CONFLICT-008；`check-session-persistence-policy.py:21-23,230-231`；`check-ui.mjs:37,53,222-243`；`check-native-webview-overlay.mjs:6,25-26,33`。
- **Migration implication**: 仅 `check-ui.mjs` 需修订（非业务代码，待用户授权后改 scripts，本次未改）。
- **Checker implication**: 元检查建议保留（11 §3），但范围收窄为"check-ui.mjs 单方 [CURRENT] 表述"。

## ADR-CREDENTIAL-001 — 凭据需 store/intent facade，前端不持密码
- **Status**: PROPOSED
- **Context**: 凭据无前端 store，CredentialList.vue 直连 bridge（04 STORE_OWNERSHIP_LEAK + NATIVE_POLICY_LEAK）。
- **Observed failure**: 安全域与 UI 域混用（FEAT-009）；直连 bridge 违反 ONE SIDE-EFFECT EXIT。
- **Decision**: 引入 `useCredentialStore` facade 封装 list/fill/import；密码只存 Rust keyring。
- **Canonical semantics**: 权威在 Rust keyring；前端只读瞬时镜像。
- **Allowed**: facade 调 bridge；fill 传 `{credential_id, activeTabId}`。
- **Forbidden**: 前端持有明文 password / 写 Pinia / 进 localStorage / 进日志；组件直连 bridge。
- **Why**: 现状无 facade（04 / 11 RULE-011）。
- **Evidence**: 01 SEM-011；04 跨表；08 FEAT-005/009；11 RULE-011。
- **Migration implication**: Phase 5。
- **Checker implication**: RULE-011。

## ADR-SEVERITY-001 — 统一 S 严重度定义，消除评级漂移
- **Status**: PROPOSED（18 终审保留 PROPOSED，须与 `semantic-governance/README.md:56` 二选一 —— 后者已按此 PROPOSED 口径生效）
- **Context**: 02/05/07/08/09 对同一现象给出不同 S 评级（如 isBrowserVisible 混淆 02 标 S3、08 标 S4；grid 分叉 03/04 标 S4、05 标 S4=0 但指红线）。
- **Observed failure**: 评级不一致削弱审计可信度。
- **Decision**: S4 = 红线违反 / 生产静默错误；S3 = 架构耦合 / 分叉风险（无静默错误，如 close==kill==destroy、position==show）；S2 = 命名-多源混淆 / 派生未单点化 / 已由类型层保证的红线；S1 = 死代码 / 隐式约定。
- **Canonical semantics**: S0..S4 在本仓库的统一定义（见 `semantic-governance/README.md:56` 与 02 §一）。
- **Why**: 第二轮复核确认最终评级以 `18` §4 为准（S4=2/S3=8/S2=6/S1=3），均为口径差异非事实对立。
- **Evidence**: 16 CONFLICT-02/03/04/05；18 SECOND-CONFLICT-002/003/004；`18` §4 FINAL_RISK_COUNTS。
- **Migration implication**: 更新各文档评级（不改事实）；最终评级唯一真源 = `18` §4 与 `00` 全局计数块。
- **Checker implication**: 无（文档治理）。
