# 18 · SECOND-REVIEW-SYNTHESIS — 第二轮对抗性复核的终审合成

> **Final Reviewer（独立终审，未参与 A–F，不投票）**
> 仓库：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> BASELINE：`master` @ `e05160a17b1d3547b3ff8f3c5064550facfc362c`（工作树未变）
> 纪律：**TRUST CODE NOT DOCUMENTS**；FACT ≠ INFERENCE；CAPABILITY ≠ CANONICAL INTENT API；MULTIPLE STATES ≠ MULTIPLE SOURCES OF TRUTH。
> 全程只读。未修改 `src/`、`src-tauri/`、`scripts/`、`tests/`、00–16、17A–17F。未执行任何 git 写操作。

---

## 0. 方法论

本终审**不接受任何 Agent 的结论为事实**，包括 A–F 之间的"多数一致"。对 6 个指定争点逐条回源码重新采样，并额外采样 8 个派生争点。

1. **符号 + 行区间双定位**：每条结论必须落到 `file:symbol:line`。
2. **证据命名空间化**：B/D/F 的 `SR-EVID-0001..003x` 互相撞号，本文引用一律写作 `SR-EVID-B001 / SR-EVID-D001 / SR-EVID-F001`；C 用 `SR-EVID-0201+`；E 用 `SR-EVID-E001+`。
3. **新观察统一登记为 `SR-EVID-FR-0001..0020`**（见 §14）。
4. **判级依据仓库自有 S0–S4 定义**（`docs/architecture/semantic-governance/README.md:56-62`）。
5. **FINAL_S4 入场判据**：五条必须**全部**成立，缺一即降级。

---

## 1. 指定争点的独立复核结果

| # | 争点 | 独立裁决 | 与 A–F 的关系 |
|---|---|---|---|
| 1 | 宫格退出分叉 | **存在，但范围被 B/F 夸大**：活的分叉出口**只有 1 处**（`HomeLaunchers.openArea:47`）；`toggleGridToolbar` 是**零调用者的死导出**；`ActivityBar:398` 是显式关闭按钮（不是分叉） | 修正 B `SR-EVID-0027`、F `SR-EVID-0004` |
| 2 | `gridOpen=true && mainView="browser"` 是否合法 | **合法**（`buildGrid:322-324` 显式产出并保留；`syncFreeze`、`useBrowserHost` 三处正当处理） | 确认 C `SR-EVID-0201..0204`，推翻 round-1 `CLAIM-MT-03` |
| 3 | `move_path` 是否 BROKEN | **决定性坐实：BROKEN**。注册/实现/FE/UI 可达；**全部 ACL 文件中不存在**（148 vs 147，唯一差项） | 确认 D `SR-EVID-0015/0016`，推翻 `BROKEN=0` |
| 4 | 检查器接线与 SessionCloseDialog 立场 | 两脚本**均未接入** `pre-merge.sh`；`check-session-persistence-policy.py` **站在"撤销"一侧** | 确认 E `SR-EVID-E033`、F `SR-EVID-0020/0023`；`CLAIM-XC-01` 一半论据被推翻 |
| 5 | ADR-GRID-001 的 checker 覆盖声明 | **零覆盖**：`check-lifecycle-contract.py` 对 `close_grid`/`kill_child`/`syncViewVisibility` **0 命中** | 确认 F `SR-EVID-0030` |
| 6 | `mainView` 裸赋值与白名单 | 外部裸赋值 **10 处**；`setView/openModule/closeModTab` **从不写** `layout.mainView` → 白名单**不可达** | 确认 E `SR-EVID-E007/E008` |

---

## 2. SECOND-CONFLICT 冲突裁决块

### SECOND-CONFLICT-001 · 宫格退出意图分叉（DESTROY vs HIDE）

- **Original claim**：03/04/16 `CLAIM-S4-02` 判 S4；05/16 `CLAIM-S4-03` 判 `S4=0`；16 `CONFLICT-03` 建议改标 S3；00 L17 仍列 `[S4]`。round-1 内部自相矛盾。
- **Agent findings**：17A `A-01` CONFIRMED S4；17B `C8/C12/C13` CONTRADICTED，自称"至少 1 条真实 S4"并点名 **3 个 DESTROY 出口**；17F `TAC-01/TAC-02` CONTRADICTED。
- **Source evidence A**：`HomeLaunchers.vue:42-54`，关键 **L47** `if (view !== "grid" && browser.gridOpen) await browser.closeGridAll();`（L43 注释"进入非宫格视图前**先关掉宫格**"）。→ **DESTROY-then-switch，活的、可达的**。
- **Source evidence B**：`ActivityBar.vue:150-177` browser 分支 **L171-173** 仅 `layout.setView("browser"); return;` —— **不调 closeGridAll**。`useBrowserStore.ts:646-655` `syncViewVisibility` 与 `useLayoutStore.ts:193-199` `setView` **全函数无 `close_grid`/`kill_child`/`close_tab`**。
- **Final reviewer reconstruction**：两条链路的用户可观测语义**都是"我要离开宫格去看别的视图"**。ActivityBar = HIDE（子进程存活，`gridOpen` 保持 true）；HomeLaunchers = DESTROY（`shutdown_all` kill+wait，用户格子状态丢失）。**生命周期结局由调用方组件隐式选择**。
  **范围修正**：B/F 声称的"第二/第三 DESTROY 出口"不成立 —— `toggleGridToolbar` 在 `src/` 仅 2 处命中（定义 :210、返回 :358），**零调用者**；`ActivityBar.vue:398` 是**显式"关闭宫格"按钮**，不是切换视图意图。→ 活的分叉出口**唯一**：`HomeLaunchers.openArea:47`。
- **Canonical interpretation**：`GRID_EXIT_DIVERGENCE` 成立，是对 **ACCEPTED 红线 ADR-GRID-001 L11「绝不触发 close_grid/kill_child」的活违反**。
- **Risk rating**：**S4（保留）**。驳 `CONFLICT-03` 的"改标 S3"：ADR-GRID-001 是 ACCEPTED 红线且被活路径违反 → 按 ADR-SEVERITY-001（S4 = 红线违反）应 S4。
- **Impact on target contract**：CONTRACT-GRID-EXIT 成立；10 的转移矩阵必须补入 DESTROY 边，"HIDE"标为 **TARGET DESIGN**。
- **Impact on migration**：Phase 1 必须纳入 `HomeLaunchers.openArea:47`，否则 ADR-GRID-001 在 Phase 1 结束仍违反，而"往返 ×20"runtime 验收仍可能通过（走 ActivityBar/TopBar 路径）。
- **Impact on checker**：RULE-004 需要，但目标 API `exitGrid` **不存在**（E `SR-EVID-E011`）；先建 action 再落门禁，且不得把 7 处加白名单。
- **ADR impact**：ADR-GRID-001 改写为"**带已知违反清单的目标政策**"；L18「`check-lifecycle-contract.py` 已覆盖」**必须删除**。
- **Remaining uncertainty**：MEDIUM。① 正确语义（HIDE 优先 vs DESTROY 优先）未裁定；② 未运行时验证子进程确被 kill（静态链闭合）。

### SECOND-CONFLICT-002 · 宫格可见三标志：重复真源 S4 vs 正交态 S2

- **Original claim**：02/08 `DUP-002` / 16 `CLAIM-MT-03` —— 三标志编码"可见"，**无单一真源**，S4。
- **Agent findings**：17C §1 判 **ORTHOGONAL_STATE**，OVERSTATED → S2；17A `A-02` 采纳。
- **Source evidence A**：`useBrowserStore.ts:319-324` —— `gridOpen.value = true`(319) 后紧跟带注释守卫 `if (mainView !== "grid" && mainView !== "browser") { mainView = "grid"; }`(322-324)。→ **显式、带意图地产出并保留 `gridOpen=true && mainView="browser"`**。
- **Source evidence B**：`useBrowserStore.ts:618-631` `syncFreeze` —— `if (gridOpen.value)` 每格下发 `mainView==='grid' ? UNFREEZE : FREEZE`(628) ⇒ 源码**已把"实例存活但当前未展示"语义化为"冻结"**。附带 L620 `const inBrowserView = …` **未被使用**（死变量）。`useBrowserHost.ts:64/99/109` 三处守卫。
- **Final reviewer reconstruction**：三者**不回答同一个问题**，互不派生 → **不是 DUPLICATE_SOURCE_OF_TRUTH**。真实缺陷两条：① `gridVisible` 派生**无单一命名真源**，inline 复制到 ≥5 处；② `gridToolbarOpen` 跨 store 双 writer，**无 invariant owner**。
- **Canonical interpretation**：**ORTHOGONAL_STATE + 派生未单点化 + 耦合无 owner**。round-1"多真源"是**标签错误**。
- **Risk rating**：**S2**（S4 → OVERSTATED）。补充新观察（`SR-EVID-FR-0012`）：`gridToolbarOpen` **无任何外部消费者**，其唯一 mutator `toggleGridToolbar` **零调用者** → **新兴死标志**。
- **Impact**：目标应写"派生单点化 + 耦合契约 owner"，而非"消除重复真源"；ADR-GRID-VISIBILITY-001「仅文档化」降 PROPOSED；RULE-013 rejected。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-003 · isBrowserView / isBrowserVisible：S4 vs S2

- **Original claim**：02 标 S3；08 标 S4；00 L20 列 `[S4]`；16 `CONFLICT-04` 建议统一 S3。
- **Agent findings**：17C §2.4/2.5 CONFIRMED 但建议 S3→**S2**；17A `A-03` PARTIALLY_CONFIRMED → S2。
- **Source evidence**：`useLayoutStore.ts:188-190` = `browser || grid`；`useBrowserStore.ts:149-151` = `mainView === "browser"`（不含 `!gridOpen`）；`UnifiedTabBar.vue:147-158` —— `activateWeb` L152 用 `mainView !== "browser"`，L149-151 注释**明文写出踩坑理由**；`isActiveWeb` L157 **合法**使用 `isBrowserView()`；`BrowserHost.vue:15` 是 `isBrowserVisible` **唯一消费者**；`useBrowserStore.ts:490-494` 注释仍写旧式公式（**过时**）。
- **Final reviewer reconstruction**：风险真实但**等级高估**：二者均为零写入 computed/纯函数、同源同根、刻意不同义；`isBrowserVisible` 单消费者；陷阱点已有防呆注释。"一律禁用 `isBrowserView()`"的门禁会误伤 `isActiveWeb:157`。
- **Risk rating**：**S2**。
- **Impact**：RULE-008 保留为防回归守卫，措辞改"不得依赖 isBrowserView"，修正"须用 `===`"与真实代码 `!==` 不同构问题。ADR-BROWSER-001 维持 ACCEPTED，补"`isActiveWeb` 例外"边界。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-004 · position 蕴含 show：S4（INFERENCE）vs S3（FACT）

- **Original claim**：07 `S4-005` 标 **S4**；05 `CASE-005` 标 **INFERENCE**；03 标 `POSITION != SHOW ✓`。
- **Agent findings**：17A `A-08` CONFIRMED、判级升 **FACT**，风险 **S4→S3**；17B `C9/C16` 同。
- **Source evidence**：`bridge.rs:531-564` `apply_bounds_inner` —— `update_rect`(543) 后 **L555 `set_visible(&id, true)`**；L566-569 注释明文**禁用** `set_visible(false)`（WebKitGTK 死锁）。`main.rs:391-425` `UpdateRect` → **L423 `win.show()`**；`GridCmd` **无 `ShowWindow` 变体**。
- **Final reviewer reconstruction**：`POSITION ⇒ SHOW` 是**可逐字读出的 FACT**，但是**机制性耦合**，**不产生静默错误**。
- **Canonical interpretation**：架构耦合。README:62 明文归 S3。
- **Risk rating**：**S3**。
- **Impact**：CONTRACT-NATIVE-SHOW 须显式声明"position 携带未声明的 show 副作用"；**RULE-007 应 rejected**（它禁止的正是官方 `-30000` 机制，`PROJECT-RULES.md:71,76` 明文）。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-005 · 原生 webview 双层显隐 HIDDEN_STATE（无冲突，保留 S4）

- **Original claim**：01 `SEM-012` / 16 `CLAIM-S4-01` —— 无前端响应式镜像，历史遮屏/空白根因，S4。
- **Agent findings**：17A `A-04` CONFIRMED；17B `C20` CONFIRMED。无实质分歧。
- **Source evidence**：`BrowserHost.vue:9-20` —— 占位壳**始终挂载**，L15 仅 CSS `visibility`（L10-11 注释解释"用 CSS 隐藏而非 v-show，保证 `getBoundingClientRect` 非零"）；`useBrowserStore.ts:646-655` + `:657-664` 原生显隐**命令式下发**，前端**无**响应式镜像；`bridge.rs:566-597` 保持原尺寸只改 `x=-30000`。
- **Canonical interpretation**：**HIDDEN_STATE**。两层（DOM 占位壳 CSS + 原生真实 bounds）**不同源**，历史事故正源于两层不同步。
- **Risk rating**：**S4（保留）**。
- **Impact**：RULE-006/007 应落到**新建 FE 侧脚本**（`check-native.mjs` 只管 Rust 侧）；两脚本均未接入门禁。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-006 · `move_path` BROKEN = 0 vs 1（★ 本轮最具后果的裁决）

- **Original claim**：06 / 16 —— **BROKEN = 0**，「MAIN handler 每条命令都在 ACL」，隐含 GATE PASS。
- **Agent findings**：17D `SR-EVID-0015/0016/0017` —— **CONTRADICTED，BROKEN = 1**。
- **Source evidence A（链路四段）**：
  1. FE 封装 `src/bridge.ts:507-509` `movePath: (src, dstDir) => invoke("move_path", { src, dst_dir: dstDir })`（**untyped**）。
  2. FE 调用与 UI 可达 `useWorkspaceStore.ts:679-690` `confirmMove()` L684 `await bridge.movePath(src, dst);`；`FilePanel.vue:242-258` L256 `@click="ws.confirmMove"`；入口 `requestMove`(667-673) 由拖拽触发。
  3. Rust 实现 `fs_cmds.rs:20-21` `#[tauri::command] pub fn move_path(app, src: String, dst_dir: String) -> Result<(), String>`（参数名与类型**完全匹配**）。
  4. 注册 `main.rs:1432`（夹在 `reveal_path`:1431 与 `open_source`:1433 之间）。
- **Source evidence B（ACL 侧，决定性）**：`default-commands.toml` `commands.allow` **全文无 `move_path`**（L34 `"reveal_path"` → L35 `"open_source"`，**恰好缺这一行**）。全仓 `move_path` 的 17 条命中中，`default-commands.toml`、`capabilities/default.json`、`browser-remote.json`、`dev-capabilities/main.json`、`remote-collect.toml`、`gen/schemas/*` **全部 0 命中**（`sync.rs` 的 4 条是 `remove_path` 子串误命中）。授予面穷尽，无通配符。**计数核对：148 − 147 = 1，唯一差项 = `move_path`。**
- **Final reviewer reconstruction**：**四面闭合、独缺 ACL**。Tauri2 `RuntimeAuthority` 会拒绝未放行命令 → `confirmMove` 的 `catch` 走到 `layout.showToast("移动失败: " + …)`(687-689)。**文件树拖拽移动 100% 失败。**
- **Canonical interpretation**：**BROKEN（用户可达，非崩溃）**。"可调用 ≠ 契约正确"的反例。
- **Risk rating**：**S2**。它**显式报错**（toast）、不污染状态、不涉生命周期/资源安全 → **不满足 S4 的"静默"要件**，**不计入 FINAL_S4**。但它是**当前唯一 BROKEN 且一行可修**，实务优先级 P0。
- **Impact**：新增 **CONTRACT-IPC-ACL**：`registered ⊆ ACL` 必须机器校验。Phase 0 P0-0：在 L34 与 L35 之间补 `"move_path",`。**必须先于门禁接入**。
- **ADR impact**：ADR-IPC-001 的"原则 ACCEPTED"**零执行力**；须补前置"接入 pre-merge.sh"。
- **Remaining uncertainty**：LOW（未运行时执行，静态链闭合度极高）。

### SECOND-CONFLICT-007 · IPC 门禁执行力：闭包牢固 + GATE PASS vs 应 FAIL + 未接线

- **Original claim**：16 隐含 GATE PASS；12 Phase 0 L14/L15「保持 GATE 通过 / 全门禁仍 PASS」。
- **Agent findings**：17D `SR-EVID-0035` CONTRADICTED；17F `SR-EVID-0020` 未接入；17E `SR-EVID-E003/E004` G2 **与** G1#4 两处 FAIL。
- **Source evidence A**：`scripts/pre-merge.sh` 全仓检索 `check-grid-close-logic|check-command-set-consistency` → **0 命中**。实际执行的只有 `check-lifecycle-contract.py`(:216-225)、`check-security-policy.py`、`check-session-persistence-policy.py`(:281-285)、`check-session-logic.mjs`、`check-resource-ui-logic.mjs`、`check-native-webview-overlay.mjs`(:372-373)、Phase03 gate(:522-527)。
- **Source evidence B**：`check-grid-close-logic.mjs` **G2（L106-126）** L112 断言 `!gridOpen.value` → 真实 computed 无 → **FAIL**；**G1 #4（L87-90）** L88 断言 `schedulePosition\(` → `closeGridAll` 体内只有 `relocate()`(509) → **FAIL**（round-1 只报了 G2）。
- **Canonical interpretation**：**门禁执行力缺口**（不是契约缺口）。Phase 0「GATE 仍 PASS」成立的**唯一原因是门禁没接线**，是**空真（vacuously true）**。
- **Risk rating**：**S3**。
- **Impact**：两条"ready"规则（RULE-005/009）的 ready 状态**以接线为前提**；**未接线时 ready = 0**。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-008 · SessionCloseDialog "三方矛盾" vs 实为两方（被点名方立场相反）

- **Original claim**：09 `SMF-005` / 16 `CLAIM-XC-01` / 14 `ADR-CHECKER-CONFLICT-001` L74 —— `check-ui.mjs` **与** `check-session-persistence-policy.py` 都标 `[CURRENT]`，与 `check-native-webview-overlay.mjs` 矛盾 → "三方直接矛盾"。
- **Agent findings**：17E `SR-EVID-E033/E034`、17F `SR-EVID-0022/0023` —— **OVERSTATED**。
- **Source evidence A**：`check-session-persistence-policy.py:17-23` 明写"原关闭协议…**必须已撤销、不得残留**"；**:230-231** `if "SessionCloseDialog" in app_vue: v.append("SP_DIALOG_STILL_MOUNTED")` —— **违规检测器**，不是"正常"。
- **Source evidence B**：`check-ui.mjs:37` 与 `:53` 两条 allowlist 写 `reason: "CURRENT 关闭协议（PROJECT-RULES [CURRENT]）…"`；该组件在 `src/` 下**不存在** → `:223` 的 `existsSync` 守卫使断言静默跳过。
- **Canonical interpretation**：**潜伏的单方冲突**。最小修复：删 `check-ui.mjs:37`/`:53` + 删 `:222-243` 空转块。
- **Risk rating**：**S2**。ADR-CHECKER-CONFLICT-001 Context **收窄为"仅 check-ui.mjs"**，补"潜伏态"说明。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-009 · ADR-GRID-001 的「check-lifecycle-contract.py 已覆盖」

- **Original claim**：14 `ADR-GRID-001` L18「`check-lifecycle-contract.py` 已覆盖」。
- **Source evidence**：`scripts/check-lifecycle-contract.py` 检索 `close_grid|kill_child|syncViewVisibility` → **0 命中**。实际只守 `ShutdownCoordinator`、`WindowEvent::CloseRequested`、`RunEvent::ExitRequested`、`create_grid` 半创建回滚、`close_tab` 短路、终端 kill waited、后台线程可取消。
- **Canonical interpretation**：**ADR 的 Checker implication 为事实错误**。叠加 CONFLICT-001 → 该红线当前**既被违反又无人看守**。
- **Impact**：删除 L18；Phase 0 不得依赖该脚本验收；须新建 FE 侧"宫格退出意图"检查，且**先有合规替代写法**。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-010 · 组件手拼宫格生命周期：1 处 vs 7 处

- **Original claim**：03 `CASE-008` S3，只点名 `UnifiedTabBar.vue:170`。
- **Agent findings**：17E `SR-EVID-E010` 实测 **7 处**；17A `A-05` UNDERSTATED。
- **Source evidence A**：`.vue` 中命中 `App.vue:199`、`HomeLaunchers.vue:47`、`UnifiedTabBar.vue:170`、`ActivityBar.vue:123/134/167/398` = **7 处**。
- **Source evidence B**：另有 `useWorkbenchStore.ts:22`（`.ts`）。惯用法 `if (gridOpen) layoutGrid(); else buildGrid();` 在 `App.vue:199`/`ActivityBar.vue:167`/`UnifiedTabBar.vue:170`/`useWorkbenchStore.ts:22` **逐字重复 4 次**。
- **Final reviewer reconstruction**：范围低估属实。**E 的"5 个 .vue"应为 4 个 .vue + 1 个 .ts**（`SR-EVID-FR-0015`）。需区分：`ActivityBar:398`/`HomeLaunchers:47` 属**纯触发**（owner 仍在 store）；`ActivityBar:123/134`（先写 `gridCount/gridLayout` 再 `buildGrid`）属**真编排**。
- **Risk rating**：**S3**（保留，不进 S4）—— 7 处当前行为**均正确**，正则 A 可检出 → 不满足"难检测"。
- **Impact**：Phase 1 **先加 `exitGrid()`/`ensureGrid()` store action**，再迁移 7 处；**RULE-004 rejected-for-Phase-0**（落地当天 7 条红灯而迁移目标不存在 → 必然被"加白名单"吃掉，**反而固化债务**）。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-011 · `mainView` 裸赋值白名单是否可达

- **Original claim**：RULE-003 白名单"位于 `setView`/`openModule`/`closeModTab` 函数体内"。
- **Source evidence A**：`src/` 全仓 `mainView\s*=[^=]` → 外部裸赋值 **10 处**：`FileEditor.vue:11`、`useBrowserStore.ts:161/207/323/503/545`、`useSessionStore.ts:120`、`useWorkspaceStore.ts:912/924`、`TopBar.vue:21`。
- **Source evidence B**：`useLayoutStore.ts` `setView` L194 `mainView.value = v`；`openModule` L258/L264 `setView(...)`；`activateModTab` L286 `setView(t.view)`；`closeModTab` L309/L312 `setView(...)` —— **均不写 `layout.mainView`**。
- **Canonical interpretation**：**事实正确 / 检测设计有误**（白名单形同装饰）。
- **Risk rating**：**S3**。RULE-003 = partial（须换 AST/成员写检测 + 删不可达白名单 + 禁 `$patch`）。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-012 · 凭据 owner 与"前端不持密码"正则

- **Original claim**：04 / 00 L25 —— S3，"无 store owner + 直连 + 前端不得持有明文密码"。
- **Agent findings**：17A `A-06` → **S2**；17E `SR-EVID-E027/E028/E029` —— B 是**伪规则**，红线已由类型层保证。
- **Source evidence A**：`src/types.ts:174-186` `BrowserCredentialItem` 仅 `url / username / has_password / credential_id / origin`，**无 password/secret/token**；L175-176 注释明文写安全红线。
- **Source evidence B**：直连点 3 处（`CredentialList.vue:44`/`:79-82`、`BookmarkPanel.vue:109`）；全仓**无** `useCredentialStore`。
- **Canonical interpretation**：**所有权缺陷（S2） + 已成立的红线（不需新规则）**。
- **Impact**：**RULE-011-B 明确否决**（改类型断言）；RULE-011-A = partial（先建 facade 且**重裁 owner** —— `importBrowserCredentials` 的自然 owner 更像 bookmark/import store）。ADR-CREDENTIAL-001 维持 **PROPOSED**。
- **Remaining uncertainty**：MEDIUM（3 处直连点采信 E/F，未逐字复核）。

### SECOND-CONFLICT-013 · `bmPanelOpen`：DERIVED_STATE_REIMPLEMENTED vs 单派生

- **Original claim**：02 `SEM-006` / 16 `CLAIM-MT-04` —— REIMPLEMENTED / S3。
- **Source evidence**：`MainArea.vue:103` 定义、L150 唯一消费者 → **派生唯一、单消费者，无第二处等价计算**。`ActivityBar.vue:141-148` `onToggleBookmarkPanel` —— 非 browser 视图点击时**先** `setView("browser")` **再**确保 `panelOpen=true`（入口已补偿）。
- **Canonical interpretation**：**消费点未复用派生量**（"REIMPLEMENTED"标签被证伪）。仅"先开面板、再切走"这一窄边界出现不一致，回 browser 视图立即复现。
- **Risk rating**：**S2**。不进 Phase 1 首批。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-014 · `toggleGridToolbar` 是否为"第三 DESTROY 出口"（★ 本轮新冲突）

- **Agent findings**：17B `SR-EVID-0027`「第二个 DESTROY 出口」；17F `SR-EVID-0004`「第三个 destroy 驱动者」；17C `SR-EVID-0202` 称其为"官方路径"。
- **Source evidence A**：`useLayoutStore.ts:210-215` 代码确实存在（ON→`buildGrid()`，OFF→`closeGridAll()`）。
- **Source evidence B（新观察，与 B/F 冲突）**：`src/` 全仓 `toggleGridToolbar` → **仅 2 处命中**（定义 :210、返回 :358），**零调用者**。`gridToolbarOpen` → 仅 5 处（:144/:211/:213/:214/:336）+ `useBrowserStore.ts:488`，**无任何 .vue 绑定、无 computed/getter/watch 外部读取**。
- **Final reviewer reconstruction**：B 与 F 把一个**死导出**当作活的 destroy 出口。该出口**当前不可达**；C 所称"官方路径进入该组合"**今天不成立**。
- **Canonical interpretation**：`toggleGridToolbar` = **DEAD_EXPORT**；`gridToolbarOpen` = **新兴死标志**。
- **Risk rating**：**S1**（死代码）。**不计入 destroy 出口**。
- **Impact**：新增动作 —— 删除 `toggleGridToolbar` 与 `layout.gridToolbarOpen`，或明确复活并接 UI；**不得为其单独立法**。**RULE-013 rejected**（为无消费者标志规范 writer 数量 = 冻结错误语义）。
- **Remaining uncertainty**：LOW。

### SECOND-CONFLICT-015 · `create_grid` 判级与 `aiNavOpen` 检测方案

- **Agent findings**：17B `SR-EVID-0011` 判级应升 **FACT**；17E `SR-EVID-E030` 拟议正则**全仓 0 命中**。
- **Source evidence A**：`bridge.rs:3850-3854` —— `clamp`(3852) 后 **`close_grid(app.clone())?;`(3854)**，**无任何 if 守卫** → 无条件 DESTROY + CREATE，**可直接读出**。
- **Source evidence B**：`src/` 全仓 `aiNavOpen` —— 定义写作 `const aiNavOpen = ref(false);`（非 `layout.aiNavOpen`），返回写作 `aiNavOpen,`；消费者全用 `browser.aiNavOpen`。→ **拟议正则对目标 0 命中**。
- **Canonical interpretation**：`create_grid` = **FACT**；`layout.aiNavOpen` = **DEAD_DUPLICATE_DECLARATION（潜势）**。
- **Risk rating**：`create_grid` **S3**；`aiNavOpen` **S1**。RULE-012 检测方案**必须换**（反向计数 + 返回值禁用）。
- **Remaining uncertainty**：LOW。

---

## 3. REGENERATED FINAL_S4

### FINAL_S4-1 · GRID_EXIT_DIVERGENCE

| 判据 | 判定 | 依据 |
|---|---|---|
| ① 语义歧义 | ✅ | "离开宫格"无命名动作；四动词并存；`HomeLaunchers:47` 与 `ActivityBar:171-173` 同意图异结局 |
| ② 小模型局部合理改动 | ✅ | 新增导航只写 `setView("browser")`（与 ActivityBar 同构、"最干净"）→ 静默保留 N×≈450MB 子进程；或模仿 HomeLaunchers 补 `closeGridAll()` → 静默销毁用户宫格 |
| ③ 编译/本地测试通过 | ✅ | `setView` 纯函数（零 bridge）；`vue-tsc` 与单测不覆盖子进程存活 |
| ④ 运行时破坏 | ✅ | 资源安全（每格 ≈450MB 子进程）+ 生命周期一致性（`gridOpen` 悬挂）+ 用户瞬态页面状态丢失 |
| ⑤ 架构/检查器难阻止 | ✅ | `check-lifecycle-contract.py` 对 `close_grid`/`kill_child`/`syncViewVisibility` **0 命中**；RULE-004 rejected-for-Phase-0；`exitGrid` 不存在 |

**核心证据**：`HomeLaunchers.vue:42-54`(L47) vs `ActivityBar.vue:150-177`(L171-173) vs `useBrowserStore.ts:483-511` vs `:646-655`。
**风险等级**：**S4**（违反 ACCEPTED 红线 ADR-GRID-001）。

### FINAL_S4-2 · NATIVE_VISIBILITY_HIDDEN_STATE

| 判据 | 判定 | 依据 |
|---|---|---|
| ① 语义歧义 | ✅ | DOM 占位壳 CSS `visibility`（`BrowserHost.vue:15`）与原生 bounds（`bridge.rs:586` `-30000` / `:555` `set_visible(true)`）**两层不同源** |
| ② 小模型局部合理改动 | ✅ | 把 `BrowserHost` 改成 `v-if`、调整 `:style`、或"补一个" `bridge.hideWebview()` / 直接 `tabPosition` |
| ③ 编译/本地测试通过 | ✅ | 纯模板/CSS/一行 bridge 改动 |
| ④ 运行时破坏 | ✅ | **历史真实事故**："切走/切回视图后宫格或网页残留遮屏 / 浏览器空白" |
| ⑤ 架构/检查器难阻止 | ✅ | 无 FE 侧脚本守 RULE-006/007；两候选脚本均未接入 `pre-merge.sh` |

**核心证据**：`BrowserHost.vue:9-20`(L15)；`useBrowserStore.ts:633-655`；`bridge.rs:531-564`/`:566-597`；`main.rs:391-425`(L423)。
**风险等级**：**S4**。

### 明确降级（不计入 FINAL_S4）

| 原判 | 新判 | 降级理由 |
|---|---|---|
| `DUP-002` 宫格可见三标志 **S4** | **S2** | 正交态，组合由 `buildGrid:322-324` 显式产出并被 3 处正当处理 |
| `DUP-001` isBrowserView/isBrowserVisible **S4** | **S2** | 同源双谓词、单消费者、已有防呆注释 |
| `S4-005` position⇒show **S4** | **S3** | README:62 明文归 S3（架构耦合，无静默错误） |
| `move_path` BROKEN（新发现） | **S2** | 显式 toast、可恢复、不涉生命周期/资源安全 → 不满足"静默" |
| `CASE-008` 组件手拼（7 处） | **S3** | 7 处当前行为均正确；正则 A 可检出 → 不满足"难检测" |

---

## 4. 最终风险计数

| 级别 | 数量 | 条目 |
|---|---|---|
| **S4** | **2** | FINAL_S4-1 GRID_EXIT_DIVERGENCE；FINAL_S4-2 NATIVE_VISIBILITY_HIDDEN_STATE |
| **S3** | **8** | ① position⇒show（架构耦合）② `create_grid` 无条件 destroy+create ③ 宫格 CLOSE==KILL==DESTROY 无软关闭原语 ④ 组件手拼宫格生命周期（7 处）⑤ ACTIVATE ⊇ CREATE（HIBERNATED 前端零镜像）⑥ IPC DRIFT 11 条无守卫占位 ⑦ IPC 门禁零执行力（两脚本未接入 + G2/G1#4 已 FAIL）⑧ `gridOpen` 离开意图无统一出口导致悬挂（含 `ActivityBar:156-157`、`useBrowserStore:490-494` 两处 STALE 注释） |
| **S2** | **6** | ① `move_path` ACL 断裂（唯一 BROKEN）② 凭据无 store owner + 3 处直连 ③ isBrowserView/isBrowserVisible 命名相近 ④ `gridVisible` 派生未单点化（≥5 处 inline）⑤ `bmPanelOpen` 消费点未复用派生 ⑥ `check-ui.mjs` 保留 SessionCloseDialog [CURRENT] allowlist（潜伏冲突） |
| **S1** | **3** | ① `layout.aiNavOpen` 死重复声明（零读者）② `layout.gridToolbarOpen` + `toggleGridToolbar` 新兴死标志/死导出 ③ `currentLocalPath` 优先级顺序为隐式约定 |

**FINAL_RISK_COUNTS: S4=2 / S3=8 / S2=6 / S1=3**

> B 的 `SR-EVID-0021`（`show_for_focus` 缺 `hidden` 守卫）与 `SR-EVID-0035`（PTY 泄漏窗口）**未独立重采样**，故**不计入**计数，列入 §15 剩余不确定性。

---

## 5. IPC_FINAL

| Bucket | Round-1 | D | **终审** | 核验程度 |
|---|---|---|---|---|
| CONSISTENT | 39 | 51 | **51** | 结构基准确认（148/147/1）；+12 来源抽查约 13 项；**未逐条复核全部 51 条** |
| DRIFT_RISK | 10 | 11 | **11** | 7 条 typed 逐条确认（`bridge.ts:401/403/417-418/420-421/431-432/462/463` 均不在注册表）；4 条 armed 已在 KNOWN |
| BROKEN | 0 | 1 | **1** | **决定性独立坐实**（`SR-EVID-FR-0007/0008`） |
| DEAD_SURFACE | — | 2 | **2** | 采信 D，未独立复核 |
| GOVERNANCE_DRIFT | — | 1 | **1** | `sync_browser_scene`（代码 0 命中，架构基线 APPROVED_TARGET），认可 D 的重分类 |
| UNVERIFIED | 0 | 0 | **0** | 桶级 0；CONSISTENT 桶内约 38 条为"接受 D，未逐条复核" |

**IPC_FINAL: consistent=51 / drift-risk=11 / broken=1 / unverified=0**

> 与 D 的唯一分歧：D 把 `move_path` 归 S2 —— 同意，但强调它是**当前唯一 BROKEN 且一行可修**，实务优先级高于全部 11 条 DRIFT。

---

## 6. ADR 裁决

| ADR | 现状 | 终审 | 处置 |
|---|---|---|---|
| **ADR-BROWSER-001** | ACCEPTED | **ACCEPTED confirmed** | 补写"`isActiveWeb`(UnifiedTabBar:157) 可合法使用 `isBrowserView()`"边界 |
| **ADR-NATIVE-SHOW-001** | ACCEPTED | **ACCEPTED confirmed** | 补 Alternatives（显式 hide 原语已推 Phase 6，应记录而非隐式否决） |
| **ADR-GRID-001** | ACCEPTED | **整条降 PROPOSED**（若不先改写） | Decision 有未登记反例（`HomeLaunchers:47`）；L18「已覆盖」为**事实错误**。改写为"带已知违反清单的目标政策"后可维持 ACCEPTED |
| **ADR-GRID-VISIBILITY-001** | ACCEPTED | **ACCEPTED，子条款降 PROPOSED** | Forbidden「不得第三标志」已被 `gridToolbarOpen` 违反（且已死）→ 列已知例外；Migration「仅文档化」→ PROPOSED |
| **ADR-IPC-001** | ACCEPTED(原则)/PROPOSED(改 checker) | **ACCEPTED(原则)，Checker implication 降 PROPOSED** | 缺前置"接入 pre-merge.sh"，否则零执行力 |
| **ADR-CHECKER-CONFLICT-001** | PROPOSED | **PROPOSED（Context 收窄）** | L74 把 `check-session-persistence-policy.py` 列为矛盾方**被证伪**；收窄为"仅 check-ui.mjs" |
| **ADR-CREDENTIAL-001** | PROPOSED | **PROPOSED（分类正确）** | facade 不存在；补 Alternatives；owner 需重裁 |
| **ADR-SEVERITY-001** | PROPOSED | **PROPOSED，须与 README 二选一** | `semantic-governance/README.md:56` 已按该 PROPOSED 口径生效 |

**ADR: accepted confirmed = 2 / accepted downgrade to proposed = 3 / proposed = 3**

> **引用精度更正**：F 的 `TAC-07` 写作 "README.md:56" —— 实际是 **`docs/architecture/semantic-governance/README.md:56`**，不是仓库根 README（根 README L56 是 apt 依赖安装说明）。事实成立，路径需精确化。

---

## 7. 检查器裁决

| RULE | E 裁定 | 终审 | 最短落地前提 |
|---|---|---|---|
| 005 IPC 三源闭包 | ready | **ready（以接线为前提）** | 定 7 条 typed 占位入 KNOWN 还是转 FAIL |
| 009 页签命令裸调 | ready | **ready（以接线为前提）** | 与 RULE-006 共用 `tabActivate` allowlist |
| 001 gridOpen 写点 | partial | **partial** | 加 `$patch` 禁用 + toggle 形态 + 写点计数冻结 |
| 002 isBrowserVisible 公式 | partial | **partial（round-1 少报一处）** | 同时修 G2(L112) **与 G1#4(L88)** |
| 003 mainView 裸赋值 | partial | **partial** | 改 AST + **删不可达白名单** + 禁 `$patch` |
| 006 原生显隐出口 | partial | **partial** | 与 RULE-009 共用 allowlist（二者在 `bridge.tabActivate` 上**互斥**） |
| 008 activateWeb 谓词 | partial | **partial** | 措辞改"不得依赖 isBrowserView"；提取器须处理同步函数 |
| 010 破坏型 FS | partial | **partial** | 补 `createDir` 后再落地 |
| 011-A 凭据 facade | partial | **partial** | 先建 facade 且重裁 owner |
| 011-B 前端不持密码 | rejected | **rejected（确认）** | 改类型断言（`types.ts:177-186`） |
| 004 组件手拼宫格 | rejected | **rejected-for-Phase-0（确认）** | 先加 `exitGrid()`/`ensureGrid()` 再迁移 7 处 |
| 007 position-as-hide | rejected | **rejected（确认）** | 禁止的正是官方 `-30000` 机制；降级 advisory |
| 012 aiNavOpen | rejected | **rejected（检测）/ confirmed（语义）** | 断言改"计数==0 且返回块不含 aiNavOpen" |
| 013 gridToolbarOpen | rejected | **rejected（理由更强）** | 该标志**零外部消费者**且唯一 mutator **零调用者** |

**CHECKERS: partial**（ready 2 / partial 7 / rejected 4，另 011-B 单独否决）

**但 ready 数按"今日实际接线状态"计为 0**：两脚本**均未出现在 `scripts/pre-merge.sh`** 任何位置。**"能检出"≠"在门禁中"** —— 这是 round-1 最严重的治理误判之一。

---

## 8. FIRST_REVIEW_ERRORS_FOUND

1. **`BROKEN = 0` 事实错误**：`move_path` 四面闭合独缺 ACL，用户可达 100% 失败。
2. **「注册↔ACL 闭包牢固」事实错误**：148 vs 147，差集 = `{move_path}`。
3. **隐含 GATE PASS 事实错误**：`check-command-set-consistency.py` 按当前代码必 exit 1；"PASS"是因为**从未被执行**。
4. **「现有全门禁仍 PASS」前提为假**：`check-grid-close-logic.mjs` 的 **G2(L112)** 与 **G1#4(L88)** 两处 FAIL，round-1 只报了 G2。
5. **两个门禁脚本未接入 `pre-merge.sh` 未被发现**。
6. **`CLAIM-XC-01`「三方矛盾」一半论据被推翻**：真实冲突只 `check-ui.mjs` 一方，且为潜伏态。
7. **`CLAIM-LC-01` / 00 L9「红线全部成立」事实错误**：`HomeLaunchers.vue:47` 是活的视图切换型 DESTROY。
8. **`ADR-GRID-001` L18「`check-lifecycle-contract.py` 已覆盖」事实错误**。
9. **`CONFLICT-03` 自相矛盾**：建议改标 S3 而 00 L17 仍列 `[S4]`。
10. **`CLAIM-MT-03` 分类错误**：三正交态判 MULTIPLE_SOURCES_OF_TRUTH/S4。
11. **`CLAIM-S4-06` DUP-001 评级偏高** → S2。
12. **`CLAIM-MT-04` 分类错误**：`bmPanelOpen` 非 REIMPLEMENTED → S2。
13. **`CLAIM-MT-05` 分类错误**：`currentLocalPath` 非 REIMPLEMENTED → DERIVED_STATE/S1。
14. **`CLAIM-LC-06` 判级偏低**：`create_grid`（`bridge.rs:3854` 无守卫）与 `EVID-GR-04`（`main.rs:423`）应由 INFERENCE 升 FACT。
15. **`CASE-008` 范围低估 7 倍**：只点名 `UnifiedTabBar:170`。
16. **`RULE-012` 检测方案无效**：拟议正则全仓 0 命中 → 当日假 PASS。
17. **`RULE-013` 检测方案无效且理据存疑**：匹配不到它打算保留的合法 writer。
18. **「已满足只需看守」过于乐观**：RULE-010 漏 `createDir`；RULE-006/009 白名单互斥；四条无一接入门禁。
19. **13/12 引用不存在的脚本**（≥8 个）→ "看似有自动化守门、实际零保护"。
20. **00 L69「无任何事实性误报」结论不成立**：以上至少 8 条为事实级错误。

---

## 9. TARGET_CONTRACT_CHANGES_REQUIRED

1. 10-TARGET 新增 CONTRACT-GRID-EXIT 的**现状边** —— 视图切换**可能** DESTROY（`HomeLaunchers.openArea:47`）。
2. 10-TARGET L40 / 13-MATRIX L15「grid→browser（HIDE）」标为 **TARGET DESIGN**。
3. 10-TARGET L12 改为「`gridVisible` 以 inline 形式重复 ≥5 处，**无单一派生真源**」。
4. 10-TARGET L37「当前**满足**」限定为「仅 `gridOpen` 写入点满足」。
5. 披露 `check-grid-close-logic.mjs` 与 `check-command-set-consistency.py` **未接入任何门禁**。
6. 新增 **CONTRACT-IPC-ACL**：`registered ⊆ ACL` 必须机器校验。
7. 13-MATRIX：`check-grid-lifecycle.mjs` → `check-grid-close-logic.mjs` 并标 `[PLANNED]`；所有未建 checker 加 `[PLANNED]`。
8. ADR-GRID-001 Decision 改写为「目标不变量，当前存在 1 处已知违反」；**删除** L18。
9. ADR-GRID-VISIBILITY-001 Forbidden 增「`gridToolbarOpen` 为已知例外（且已死）」；Migration「仅文档化」→ PROPOSED。
10. ADR-IPC-001 Migration 增「Phase 0 必须接入 `pre-merge.sh`」。
11. ADR-CHECKER-CONFLICT-001 Context 收窄为「仅 `check-ui.mjs`」；补"潜伏态"。
12. ADR-SEVERITY-001 与 `semantic-governance/README.md:56` 状态对齐（二选一）。
13. **新增 DECISION REQUIRED**：`gridOpen=true && mainView='browser'` 的法律地位 —— 保留 alive-but-hidden（HIDE 优先）还是强制"离开 grid 必关"（DESTROY 优先）。这是 `exitGrid` 语义的前置决策，**目前未裁定**。

---

## 10. MIGRATION_PLAN_CHANGES_REQUIRED

1. **Phase 0 · P0-0（最高优先级）**：修 `move_path` ACL —— `default-commands.toml` L34 与 L35 之间补 `"move_path",`。**一行**，当前唯一 BROKEN。
2. **Phase 0**：把 `check-command-set-consistency.py` **接入 `pre-merge.sh`**（接入即 FAIL，故必须在 P0-0 之后）。
3. **Phase 0**：修 `check-grid-close-logic.mjs` **G2(L112)** 与 **G1#4(L88)** 后接入（round-1 只列 G2）。
4. **Phase 0**：增"不可达导出扫描"，一次性裁定 `layout.aiNavOpen` 与 `layout.gridToolbarOpen` + `toggleGridToolbar` 存废。
5. **Phase 1**：**先建 store action**（`exitGrid(mode)`/`ensureGrid()`），**再**迁移 7 处；RULE-004 落地前**不得上门禁**。
6. **Phase 1**：`HomeLaunchers.openArea:47` 显式纳入出口迁移清单。
7. **Phase 1**：`mainView` 白名单补全 10 处，并**删除不可达的 `setView`/`openModule`/`closeModTab` 白名单**。
8. **Phase 1**：L30「`mainView` **仍**由 setView 单一写入」改为**迁移后承诺**。
9. **Phase 2/3**：SEM-010 `currentLocalPath` 单一归属。
10. **Phase 3**：Goal 改为「为**已达成**的不变量补 RULE-010 看门狗」，补 `createDir`；可考虑与 Phase 2 合并。
11. **Phase 4**：先补 file:line 与规则代号。
12. **全局**：回滚边界**不得依赖** `[PLANNED]` 脚本。
13. **新增**：删除或明确复活 `gridToolbarOpen` + `toggleGridToolbar`；删除 `layout.aiNavOpen`；修 `ActivityBar:156-157` 与 `useBrowserStore:490-494` 两处 STALE 注释；清理 `syncFreeze:620` 死变量。

---

## 11. PHASE_0_BLOCKERS

1. `move_path` ACL 缺失（BROKEN=1）—— 接入门禁前必须修。
2. `check-command-set-consistency.py` 未接入 `pre-merge.sh`。
3. `check-grid-close-logic.mjs` 未接入，且 G2(L112) 与 G1#4(L88) 两处 FAIL。
4. RULE-005 的 KNOWN 决策未定（**7** 条 typed 占位，round-1 说 5）。
5. RULE-012 / RULE-013 检测手段未换。
6. RULE-006 与 RULE-009 在 `bridge.tabActivate` 白名单互斥未决。
7. round-1 文档尚未回写（至少 8 条事实级错误仍在 05/06/10/12/13/14/00 中）。

---

## 12. PHASE_1_BLOCKERS

1. 目标语义不清晰：`gridOpen`+`mainView` 组合法律地位与 `gridVisible` owner 未裁定。
2. `exitGrid`/`ensureGrid` 目标 API **不存在**。
3. `HomeLaunchers.openArea:47` 未纳入 Phase 1 范围。
4. `mainView` 裸赋值白名单不全。
5. `gridVisible` 派生单点化的 owner 未定。
6. 验收路径不可执行（≥5 个不存在 checker；两真实 checker 未接入；"往返 ×20"不触及 `HomeLaunchers:47`）。
7. ADR-GRID-001 未改写，仍会被误读为"现状安全"。
8. 兼容性边界不清（`move_path` 修复是行为变更；此前从未成功，无既有基线）。

---

## 13. READY 判定

### READY_FOR_PHASE_0: **NO**

| 判据 | 结论 | 依据 |
|---|---|---|
| checker 规则已证明可检出 | **部分满足** | ✅ RULE-005（会抓到 `move_path`）、RULE-002（G2 当前 FAIL）可检出。❌ RULE-012 拟议正则 0 命中（当日假 PASS）；RULE-013 匹配不到合法 writer；RULE-003 白名单不可达；RULE-006/009 白名单互斥未决 |
| 规则不会冻结错误语义 | **不满足** | RULE-007 禁止官方 `-30000` 机制；RULE-013 为死标志立法；RULE-012 给死重复发合格证；RULE-004 落地必被白名单吃掉并**固化债务** |
| 重大 FACT 冲突已解决 | **不满足（裁决已完成但未回写）** | `BROKEN=0→1`、"注册↔ACL 牢固"被推翻、三方冲突→两方、05 `S4=0` 与 03/04 `S4=1` 已统一；但 05/06/10/12/13/14/00 **尚未回写** |

**解除条件**：完成 PHASE_0_BLOCKERS 1–3 + 回写 §9 的 1–12 项文档更正。

### READY_FOR_PHASE_1: **NO**

| 判据 | 结论 |
|---|---|
| 目标语义清晰 | **否**（组合法律地位与 `gridVisible` owner 未裁定） |
| 生命周期清晰 | **部分**（Tab/PTY 清晰；宫格：1 活分叉 + 1 死导出 + 1 显式关闭按钮） |
| owner 清晰 | **否**（`gridVisible` 派生 / `gridToolbarOpen` 耦合 / 凭据 facade owner 存疑） |
| 兼容性清晰 | **否**（白名单不全；`move_path` 修复为行为变更） |
| 无未决 S4 FACT 冲突 | **是（已裁决），但 ADR 仍以"现状断言"存在会被误读** |
| 验收路径清晰 | **否**（依赖不存在脚本；runtime 验收不触及 `HomeLaunchers:47`） |
| 回滚边界清晰 | **否**（依赖 `[PLANNED]` 脚本） |

---

## 14. 新证据登记（SR-EVID-FR-XXXX）

**SR-EVID-FR-0001** 宫格退出存在活的分叉。 FACT · `HomeLaunchers.vue:42-54`(L47 DESTROY) vs `ActivityBar.vue:150-177`(L171-173 HIDE) · Confidence: HIGH
**SR-EVID-FR-0002** `closeGridAll` 顺序：`tabActivate`(499-501) **早于** `mainView` 复位(502-504)；体内无 `schedulePosition(`。 FACT · `useBrowserStore.ts:483-511` · HIGH
**SR-EVID-FR-0003** `syncViewVisibility`(646-655) 与 `setView`(193-199) 在视图切换链上零 destroy。 FACT · HIGH
**SR-EVID-FR-0004** `gridOpen=true && mainView="browser"` 是**被设计保留**的合法表达式（`buildGrid:319-324` 带注释守卫）。 FACT · **推翻 `CLAIM-MT-03`** · HIGH
**SR-EVID-FR-0005** `syncFreeze:618-631` 已把该组合语义化为"冻结"；L620 `inBrowserView` 为死变量。 FACT · HIGH
**SR-EVID-FR-0006** `useBrowserHost.ts:64/99/109` 三处守卫把该组合当合法态处理（真实缺陷是守卫 inline 复制无命名派生）。 FACT · HIGH
**SR-EVID-FR-0007（★决定性）** `move_path` 是用户可达 BROKEN —— 四面闭合独缺 ACL。 FACT · `bridge.ts:507-509`→`useWorkspaceStore.ts:684`→`FilePanel.vue:256`→`main.rs:1432`→`fs_cmds.rs:20-21`→**ACL 全仓不存在** · **推翻 `BROKEN=0`** · HIGH
**SR-EVID-FR-0008** 注册 148 vs ACL 147，唯一差项 = `move_path`。 FACT · HIGH
**SR-EVID-FR-0009** `check-lifecycle-contract.py` 对 `close_grid|kill_child|syncViewVisibility` **0 命中**。 FACT · **推翻 ADR-GRID-001 L18** · HIGH
**SR-EVID-FR-0010** 两个关键 checker 未接入 `pre-merge.sh`（检索 0 命中）。 FACT · HIGH
**SR-EVID-FR-0011** `check-grid-close-logic.mjs` 当前**两处** FAIL（G2 L112 + G1#4 L88），round-1 只报一处。 FACT · HIGH
**SR-EVID-FR-0012（★新发现）** `toggleGridToolbar` 零调用者（死导出）；`layout.gridToolbarOpen` 无外部消费者。 FACT · **修正 17B `SR-EVID-0027`、17F `SR-EVID-0004`、17C `SR-EVID-0202`；强化 17E `SR-EVID-E032`** · HIGH
**SR-EVID-FR-0013** `mainView` 外部裸赋值 10 处；三个白名单函数对该正则不可达。 FACT · HIGH
**SR-EVID-FR-0014** `check-session-persistence-policy.py` 站在**撤销**一侧（L21-23 文档串 + L230-231 违规检测器）。 FACT · **推翻 `CLAIM-XC-01` 一半论据** · HIGH
**SR-EVID-FR-0015** 组件手拼宫格实测 **7 处 / 4 个 .vue（+1 处 .ts）**；惯用法重复 4 次。 FACT · **修正 17E「5 个 .vue」** · HIGH
**SR-EVID-FR-0016** `POSITION⇒SHOW` 与 `create_grid` 无条件 destroy 均为可逐字读出的 FACT（`main.rs:423`、`bridge.rs:555`、`bridge.rs:3854` 无守卫）。 FACT · HIGH
**SR-EVID-FR-0017** `layout.aiNavOpen` 零读者；拟议正则对目标 **0 命中**。 FACT · HIGH
**SR-EVID-FR-0018** "前端不持明文密码"红线已由**类型层**保证（`types.ts:174-186`）。 FACT · HIGH
**SR-EVID-FR-0019** 无守卫 typed agent/skill 占位实为 **7** 条（round-1 说 5）。 FACT · HIGH
**SR-EVID-FR-0020** 13-MATRIX / 12-PLAN 引用的多个 checker **不存在**（`scripts/` 共 63 个 `check-*`）。 FACT · HIGH

---

## 15. 剩余不确定性

1. **`show_for_focus` 缺 `hidden` 守卫**（17B `SR-EVID-0021`，INFERENCE/MEDIUM）：未独立重采样，未计入计数。若成立是真实缺陷，建议 Phase 1 前补证。
2. **PTY 泄漏窗口**（17B `SR-EVID-0035`，INFERENCE/MEDIUM）：未独立复核。
3. **`move_path` 运行时确认**：静态链闭合（置信 HIGH），受只读约束未实际 invoke 观察拒绝。
4. **CONSISTENT 桶内 51 条**：只逐条复核约 13 条 + 结构基准，其余"接受 D"。
5. **凭据 3 处直连点**：采信 17E/17F，未逐字复核。
6. **`gridOpen=true && mainView='browser'` 触发频率**：守卫明确存在，但未穷举所有可达路径（不影响合法性裁决）。
7. **DECISION REQUIRED**：组合法律地位（HIDE 优先 vs DESTROY 优先）—— 决定 `exitGrid` 语义，PHASE_1_BLOCKER-1。

---

# 最终报告块

```
SECOND_INDEPENDENT_SEMANTIC_REVIEW
STATUS: COMPLETE
BASELINE: branch=master, HEAD=e05160a17b1d3547b3ff8f3c5064550facfc362c
CLAIMS_REVIEWED: 34
CONFIRMED: 14
PARTIALLY_CONFIRMED: 5
OVERSTATED: 6
UNDERSTATED: 3
CONTRADICTED: 4
INSUFFICIENT_EVIDENCE: 2
FINAL_RISK_COUNTS: S4=2 / S3=8 / S2=6 / S1=3
FINAL_S4:
  1. GRID_EXIT_DIVERGENCE — 同一"离开宫格"意图：HomeLaunchers.vue:47 = DESTROY(closeGridAll→shutdown_all)，ActivityBar.vue:171-173 = HIDE(仅 setView)；无单一 exitGrid 意图动作；违反 ACCEPTED 红线 ADR-GRID-001；check-lifecycle-contract.py 零覆盖。范围收窄：活分叉出口仅 1 处（toggleGridToolbar 为死导出、ActivityBar:398 为显式关闭按钮）。
  2. NATIVE_VISIBILITY_HIDDEN_STATE — 双层显隐（BrowserHost.vue:15 CSS visibility 占位壳 + Rust bounds -30000/set_visible(true)），前端无原生层响应式镜像，仅靠 watch(mainView)→syncViewVisibility 命令式同步；历史遮屏/空白根因；无 FE 侧检查器。
IPC_FINAL: consistent=51 / drift-risk=11 / broken=1 (move_path) / unverified=0
ADR: accepted confirmed=2 (ADR-BROWSER-001, ADR-NATIVE-SHOW-001) / accepted downgrade to proposed=3 (ADR-GRID-001 整条, ADR-GRID-VISIBILITY-001 子条款, ADR-IPC-001 Checker implication) / proposed=3 (ADR-CHECKER-CONFLICT-001 需收窄 Context, ADR-CREDENTIAL-001, ADR-SEVERITY-001)
CHECKERS: partial (ready=2 但今日接线数为 0 — RULE-005/RULE-009；partial=7 — RULE-001/002/003/006/008/010/011-A；rejected=4 — RULE-004/007/012/013；另 RULE-011-B 单独否决)
FIRST_REVIEW_ERRORS_FOUND: 20 (事实级至少 8 条)
TARGET_CONTRACT_CHANGES_REQUIRED: 13 项（见 §9）
MIGRATION_PLAN_CHANGES_REQUIRED: 13 项（见 §10）
PHASE_0_BLOCKERS: 7 (见 §11)
PHASE_1_BLOCKERS: 8 (见 §12)
READY_FOR_PHASE_0: NO
READY_FOR_PHASE_1: NO
CODE_MODIFIED: NO
```

**签收**：Final Reviewer · READ-ONLY · 未修改 `src/`、`src-tauri/`、`scripts/`、`tests/`、docs 00–16 与 17A–17F · 未执行任何 git 写操作 · 新增证据 20 条。
