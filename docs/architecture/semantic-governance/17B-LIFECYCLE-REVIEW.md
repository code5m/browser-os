# 17B-LIFECYCLE-REVIEW.md

> Round-2 · Adversarial Independent Lifecycle Review · **Reviewer B**
> 项目：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`（Tauri2 + Vue3）
> 任务：从源码独立重建真实生命周期状态机，并对第一轮 `05-LIFECYCLE-MODEL.md`（及其"S4=0 / 红线全部成立"结论）做证伪性复核。
> 纪律：**TRUST CODE NOT DOCUMENTS**；VIEW SWITCH ≠ DESTROY；HIDE ≠ CLOSE；CLOSE ≠ DESTROY；POSITION ≠ SHOW（除非源码证明）；ACTIVATE ≠ NAVIGATE。
> 方法：只读静态溯源（`read_file` / `rg`）。所有结论必须落到符号 + 行区间。**源码里不存在的状态，不发明。**

---

## 0. 执行摘要（先看这个）

| 结论 | 判定 |
|---|---|
| VIEW SWITCH ≠ DESTROY（**浏览器页签**） | **IMPLEMENTED FACT**（成立） |
| VIEW SWITCH ≠ DESTROY（**终端/PTY**） | **IMPLEMENTED FACT**（成立） |
| VIEW SWITCH ≠ DESTROY（**宫格**） | **PARTIALLY IMPLEMENTED**——同一"离开宫格"意图存在 HIDE 与 DESTROY 两条真实出口，取决于入口 |
| HIDE ≠ CLOSE | **CONFIRMED**（页签与宫格均成立） |
| CLOSE ≠ DESTROY | 页签：**PARTIALLY**（普通关闭 fused；HIBERNATED 是真实分裂态）；宫格：**CONFIRMED 为融合**（CLOSE==KILL==DESTROY） |
| POSITION ≠ SHOW | **不成立（源码级）**：`main.rs:423` 与 `bridge.rs:555` |
| ACTIVATE ≠ NAVIGATE | **CONFIRMED**；但 ACTIVATE **内含 CREATE**（休眠重建），05 未写 |
| 05 的 "S4 = 0" | **被推翻**（至少 1 条真实 S4：宫格退出语义分歧） |

**一句话**：05 的技术溯源在 happy path 上大体准确，但它把**入口相关的条件行为**写成了**无条件的红线结论**，且在宫格退出这一核心用例上与同一治理文档集的 03（S4=1）自相矛盾。"所有红线成立 / S4=0"不足以支撑。

---

## 1. 真实状态机（只列源码中真实存在的状态）

### 1.1 浏览器页签 Browser Tab（Rust 为权威源）

| 状态 | 源码判据 | 是否存在 |
|---|---|---|
| ABSENT | `!tabs.contains_key(id)` | 是 |
| ALIVE · ACTIVE | `tabs[id]` + 插件表有 webview + `active_tab == Some(id)` | 是 |
| ALIVE · OFFSCREEN(=HIDDEN=INACTIVE) | `child_layouts[id] == (-30000, y, w, h)` | 是（`hide_bounds` 唯一产生方式） |
| ALIVE · UNPOSITIONED | 有 webview 但 `child_layouts` 无记录（源码显式区分） | 是（05 未列） |
| HIBERNATED | `hibernated_tabs.contains(id)`：webview 已 destroy，但 tabs 条目 + URL 保留 | 是 |
| CLOSED | 无独立状态 | **不存在** |
| DESTROYED | `tabs.remove(id)` 后；普通关闭与 DESTROYED **fused** | 是（与 CLOSE 同一动作） |

**不存在**：VISIBLE 与 HIDDEN 无独立 API 表达（只能由 `x == -30000` 推断）；无"可见但非激活"；无 CLOSED≠DESTROYED 的分离。

```
ABSENT --create_tab/spawn_child_window--> ALIVE(UNPOSITIONED,1x1,visible=false)
ALIVE  --tab_position/apply_bounds_inner--> ALIVE(ACTIVE)      // set_visible(true) 内嵌
ALIVE  --hide_bounds(-30000)--------------> ALIVE(OFFSCREEN)   // 不销毁
ALIVE  --hibernate_tab--------------------> HIBERNATED         // destroy webview, keep entry
HIBERNATED --tab_activate----------------> ALIVE(UNPOSITIONED) // 按 URL 重建
ALIVE  --tab_close/close_tab-------------> DESTROYED(=ABSENT)  // CLOSE==DESTROY
ALIVE  --tab_position 失败 → recover-----> DESTROYED + 立即 CREATE（隐藏自愈）
```

### 1.2 宫格 Grid（**每格 = 一个 OS 子进程**，权威源 `GridProcessManager.children[index]`）

`GridChildHandle` 真实字段：子表成员身份、`last_url`、`last_rect`、`hidden`、`blur_hidden`（grid_process.rs:79-98）。

| 状态 | 源码判据 | 是否存在 |
|---|---|---|
| ABSENT | `!children.contains_key(index)` | 是 |
| SPAWNED（进程活，壳窗 hidden） | 存在，`last_rect == None`；壳窗 `visible(false)`（main.rs:137） | 是（05 未建模） |
| VISIBLE | 存在 + `last_rect.is_some()` + `!hidden && !blur_hidden` + 壳窗已 show | 是 |
| HIDDEN（显式） | `hidden == true`（`record_hidden`，视图切换） | 是 |
| HIDDEN（失焦） | `blur_hidden == true`（`hide_for_blur`） | 是（与显式 HIDDEN 是两个位） |
| CLOSED | 无独立状态 | **不存在** |
| DESTROYED | 已从 `children` 移除 + `kill()`+`wait()` | 是（CLOSE==KILL==DESTROY 融合） |

**不存在**：CREATED≠VISIBLE 的软关闭态；"关闭但保留进程"的软关闭；ACTIVATE/INACTIVATE。

```
ABSENT --get_or_spawn + CreateTab----------> SPAWNED（进程活，窗口 hidden）
SPAWNED--UpdateRect(win.show())------------> VISIBLE
VISIBLE--HideWindow------------------------> HIDDEN(explicit)
VISIBLE--主窗失焦 hide_for_blur------------> HIDDEN(blur)
HIDDEN(blur) --show_for_focus--------------> VISIBLE
HIDDEN(explicit) --grid_position-----------> VISIBLE
*/DESTROYED <--close_grid/grid_close_one---- // HideWindow→CloseTab→kill_child/shutdown_all
任意非 ABSENT --子进程 crash→monitor--------> DESTROYED → 立即 SPAWNED → replay → VISIBLE
```

### 1.3 终端 PTY

| 状态 | 判据 | 存在 |
|---|---|---|
| ABSENT | 不在 `terminals` | 是 |
| ALIVE（xterm 未挂载） | 在 `terminals`；前端无该项/面板已卸载 | 是（**存活独立于视图**） |
| ALIVE（xterm 已挂载） | 同上 + `termWriters[id]` 已绑定 | 是 |
| DESTROYED | `term_kill` → remove + `terminate_session`（已 wait） | 是 |
| CLOSED | 无独立状态（`✕` 只翻 `terminalOpen=false` = HIDE） | **不存在** |

### 1.4 Workspace / File preview

**不存在任何原生资源生命周期**：纯前端状态，文件预览是 DOM。Rust 侧无 `switch_workspace`/`switchWorkspace`（双向 0 命中）。→ 第一轮 EVID-WS-01 标 UNVERIFIED 属**过度保守**，可收敛为 FACT(no effect)。

---

## 2. 动词真值表（原生层**实际**做了什么）

| 动词 | Browser Tab | Grid（每格） | Grid 子进程 | Terminal/PTY | Workspace/预览 |
|---|---|---|---|---|---|
| CREATE | `manager.create_tab`（1×1, visible=false）+ `tabs.insert` | `get_or_spawn` + UDS `CreateTab` | `Command::new(exe) --grid-child i` spawn | `openpty` + `setsid` shell + worker/pump | 无（纯 DOM） |
| SHOW | **无专用命令**；`apply_bounds_inner:555` 内 `set_visible(true)` | **无 ShowWindow 变体**；`UpdateRect` 末尾 `win.show()`（main.rs:423） | 同上（壳窗 show） | xterm 重挂载 + `replayTermHistory` | `fileEditorOpen=true` |
| HIDE | `update_rect(-30000,…)`，**刻意不用 set_visible(false)** | `GridCmd::HideWindow` → `win.hide()` | 进程不受影响 | `terminalOpen=false`（仅隐藏面板，PTY 存活） | DOM v-if |
| ACTIVATE | 设 `active_tab` + 其余 `hide_bounds` + **若休眠则 CREATE** | 不存在 | 不存在 | `setActiveTerm`（仅焦点） | 不存在 |
| INACTIVATE | 无独立命令；作为 activate/create 的循环副作用 | 不存在 | 不存在 | 不存在 | 不存在 |
| POSITION | `update_rect` **+ set_visible(true)** + `remember_layout` | `record_rect`（**清 hidden/blur_hidden**）→ `UpdateRect` → **show** | `set_position`+`set_size`+`update_rect`+`show` | `term_resize`（真实改 PTY 行列） | 无 |
| NAVIGATE | `manager.navigate`（`tab_open`） | `GridCmd::Navigate` | 子进程内 navigate | N/A | 无 |
| RELOAD | **两套**：`tab_reload`=`location.reload()`；`store.tabReload`=重新 `tab_open`（**无外部调用者**） | **未实现专用 reload** | 同上 | 无（↻ = kill+spawn） | 无 |
| CLOSE | `close_tab` = 销毁 webview **且**移除条目 → 与 DESTROY 融合 | `close_grid`/`grid_close_one` = HideWindow→CloseTab→kill | `kill_child`（移表后 kill+wait） | `term_kill`（先移表解锁→terminate_session） | 无副作用 |
| DESTROY | 同上（fused）；例外 `hibernate_tab` | 与 CLOSE/KILL 融合 | `shutdown_all`（kill+wait+clear） | `terminate_session`（stop→SIGTERM→轮询→SIGKILL→wait→join） | 无 |
| KILL | N/A | N/A（就是 DESTROY） | `child.kill()+wait()` | 进程组 `killpg` | N/A |

---

## 3. 宫格专题（逐条精确回答）

### 3.1 `buildGrid`/`createGrid`/`create_grid` 到底创建了什么？
`bridge.rs:3854` `close_grid(app.clone())?;` —— 位于 clamp 之后、**无任何 if 守卫**，是**无条件 DESTROY**。
副作用链：`HideWindow(全部)` → `sleep(100ms)` → `CloseTab(全部)` → `shutdown_all()` → 清 `child_layouts/grid_zooms/last_position_at` 的 `grid-0..MAX_GRID` ⇒ `create_grid` = **无条件 DESTROY + CREATE**。
第一轮标 `INFERENCE(设计性耦合)` **判级偏低**：它是可直接读出的 FACT。

### 3.2 `hideWebview`/`hide_all_webviews` 会 DESTROY 吗？
**不会。** 页签：`hide_bounds` → `update_rect(-30000,…)` 保持原尺寸；宫格：逐格 `record_hidden` + `GridCmd::HideWindow` → `win.hide()`。子进程、UDS、webview **全部存活**。
`set_visible(false)` **刻意回避**（全仓唯一 `set_visible` 调用为 `bridge.rs:555` 且为 `true`；注释 L566-569 说明 WebKitGTK 死锁）。

### 3.3 `gridPosition` → `UpdateRect` 是否蕴含 `win.show()`？
**是。** `main.rs:391-424`：`set_position` → `set_size` → `update_rect` → `child_layouts` → **`win.show()` 第 423 行**。
`GridCmd` **不存在 ShowWindow 变体**（全仓 0 命中）⇒ 宫格不存在正交 show，`POSITION == SHOW` 是机制事实。
补充：`grid_position` 内部 `record_rect` 把 `hidden`/`blur_hidden` 双双置 false ⇒ 定位同时清除隐藏位。

### 3.4 `closeGridAll` 完整副作用（按执行顺序）
`useBrowserStore.ts:483-511`：
1. `gridOpen.value = false`（先翻位，早于 IPC 往返）
2. `await bridge.closeGrid()` → `close_grid`：HideWindow → sleep100ms → CloseTab → `shutdown_all()` + 清三类元数据
3. `layout.gridToolbarOpen = false`
4. `gridRects.splice(0)`
5. `await bridge.tabActivate(activeTabId)` ← **重激活在 mainView 复位之前**
6. `if (mainView === "grid") mainView = "browser"`
7. `syncFreeze()`
8. `relocate()`
9. `layout.showToast("已关闭宫格")`
**未做**：不重置 `gridCount`/`gridUrls`/`gridSession`。

### 3.5 `shutdown_all` 何时发生？
仅两处：① `close_grid`（bridge.rs:3936）；② `ShutdownCoordinator` `"shutdown-grid"`（bridge.rs:926-936，由 `CloseRequested` main.rs:1338-1341 与 `RunEvent::ExitRequested` 1569-1571 触发）。
**视图切换路径绝不触碰 `shutdown_all`。**

### 3.6 grid → browser 与 browser → grid
**A. HIDE 出口**（ActivityBar onItem:171-173 等）：`setView("browser")` → `mainView` 变更 → `watch` → `syncViewVisibility` → `hide_all_webviews`（宫格逐格 HideWindow；页签 -30000）→ `relocate` → `schedulePosition`（grid 分支不成立）→ `tabPosition` → `set_visible(true)`。**宫格进程全部存活，gridOpen 保持 true，无任何 destroy/recreate。**
**B. DESTROY 出口**：`HomeLaunchers.openArea:47` `if (view !== "grid" && browser.gridOpen) await browser.closeGridAll();`；`toggleGridToolbar:214` 收起 → `closeGridAll()`。
**C. browser → grid**：`onItem:159-168` → `openModule("grid")` → `setView("grid")` → `syncViewVisibility` → `hide_all_webviews`（**连宫格自己也 HideWindow 一遍**）→ `relocate` → grid 分支成立 → `scheduleGrid` → `layoutGridNow` → 逐格 `gridPosition`+`gridSetZoom`+`hideWebview(tab)` → `record_rect`（清 hidden/blur_hidden）+ `UpdateRect` → `win.show()`。
**反直觉但为真**：每次进入 grid 视图都会先把所有格子 HideWindow，再 UpdateRect show 一遍。

### 3.7 纯视图切换是否存在任何 destroy/recreate？
在 `watch(mainView) → syncViewVisibility` 链上：**绝对没有**。`setView` 本体零 bridge 调用；`close_grid`/`grid_close_one`/`kill_child`/`shutdown_all`/`close_tab` 在视图切换链上调用数 = 0。
但存在**非用户触发**的 destroy/create：`create_grid`（无条件先 destroy）、monitor 崩溃自愈、`recover_tab_webview`（tab_position 失败 → close+create）。

---

## 4. 🔴 VIEW SWITCH ≠ DESTROY 现在是"已实现的事实"吗？

| 资源 | 判定 | 证明 |
|---|---|---|
| Browser Tab | **IMPLEMENTED FACT** | `setView` 无 bridge 调用；watch 链唯一原生动作是 hide（离屏）+ relocate |
| Terminal / PTY | **IMPLEMENTED FACT** | `MainArea.vue:107-111` watch 只 `ensureTerm()`；`TerminalPane.onBeforeUnmount` 不 kill |
| Grid | **PARTIALLY IMPLEMENTED** | HIDE 出口是主路径；但同一意图存在 DESTROY 出口（HomeLaunchers / toggleGridToolbar）；无统一 `exitGrid(intent)` |
| Workspace/预览 | N/A | 无原生资源 |

把 05 的"CASE-002 成立 / 红线成立"作为**无条件结论**不成立——它是**条件成立**，取决于用户从哪个入口离开宫格。

---

## 5. 终端 / PTY 复核
- **xterm 卸载但 PTY 存活：CONFIRMED**。`TerminalPane.vue:223-233` unmount 只 dispose xterm/解绑 writer，**不调 killTerm**。唯一 kill 入口是 ↻/⏹ 按钮。
- **面板重建靠回放**：`replayTermHistory`（L199）+ `termHistories`（上限 40 块）+ `termBuffers`。**注意：是"最近 40 块"的近似真值，不是完整重建。**
- **切视图不 spawn 也不 kill**：`ensureTerm` 仅在 `termPanes.length === 0` 时 spawn。
- **kill 正确 wait**：`terminate_session` → `terminate_group`（SIGTERM→轮询 2s→SIGKILL）→ **`child.wait()`** → `join_pipeline`。
- **风险（新增）**：`useSystemStore.killTerm:258-265` `.catch(()=>{})` 吞错后**仍从 FE 移除**；Rust 侧除 `term_kill` 与退出 `drain()` 外**无孤儿回收** ⇒ 潜在"Rust 里活着的 PTY、前端已无 pane"泄漏窗口（INFERENCE, MEDIUM）。

---

## 6. 对第一轮 05 的逐条裁决

| # | 05 断言 | Source fact | Verdict |
|---|---|---|---|
| C1 | 页签状态集含 HIBERNATED，前端镜像 = tabs[] | 状态集对；但**前端无任何休眠表征** | PARTIALLY_CONFIRMED |
| C2 | `tab_activate` 仅改激活态+隐藏+重定位 | 还含：休眠重建 CREATE；仅当 `x > -1000` 才立即恢复；清 `last_position_at` | PARTIALLY_CONFIRMED |
| C3 | 视图切走页签仅隐藏不销毁 | 成立 | CONFIRMED |
| C4 | 普通 tabClose fused | 成立（+7 表清理） | CONFIRMED |
| C5 | HIBERNATED 需开关+600s | `TAB_HIBERNATE_IDLE_SECS=600`(4546)；需 `hibernation_enabled` | CONFIRMED |
| C6 | CreateTab 建 1×1 **隐藏** webview | 偏移：`main.rs:376` 是 `visible:true`；隐藏的是**壳窗**(137) | PARTIALLY_CONFIRMED |
| C7 | EVID-GR-02 首行 close_grid（INFERENCE） | **无守卫**，无条件 kill+wait | CONFIRMED（应升 FACT） |
| C8 | EVID-GR-03 / CASE-002 宫格视图切换仅 HideWindow | 对 ActivityBar 成立，对 HomeLaunchers/工具条**不成立** | **CONTRADICTED** |
| C9 | EVID-GR-04 `UpdateRect` 末行 `win.show()` | `main.rs:423`；无 ShowWindow 变体 | CONFIRMED（升 FACT） |
| C10a | 宫格 CLOSE==KILL==DESTROY | 成立 | CONFIRMED |
| C10b | closeGridAll 副作用清单与顺序 | 顺序错（tabActivate 在 mainView 复位前）；漏 gridToolbarOpen/syncFreeze/relocate/toast | PARTIALLY_CONFIRMED |
| C11 | resize/maximize 仅重定位 | 成立；但页签 `tab_position` 失败会 recover（close+create） | CONFIRMED（附例外） |
| C12 | §2.3 `grid→browser` = 仅 HideWindow | 入口决定 | **CONTRADICTED** |
| C13 | §8 "S4 = 0" | 至少 1 条真实 S4；与 03(S4=1)、07(S4=3) 矛盾 | **CONTRADICTED** |
| C14 | HIDE ≠ CLOSE | 成立 | CONFIRMED |
| C15 | ACTIVATE ≠ NAVIGATE | 成立，但可 CREATE | CONFIRMED |
| C16 | POSITION == SHOW（CASE-005） | 成立 | CONFIRMED（升 FACT） |
| C17 | xterm 卸载不杀 PTY | 成立 | CONFIRMED |
| C18 | kill 先释放锁再 wait | 成立 | CONFIRMED |
| C19 | EVID-WS-01 UNVERIFIED | 双向 0 命中；纯 DOM | **UNDERSTATED** |
| C20 | CASE-007 双层显隐 | 公式对；注释与 checker G2 漂移 | CONFIRMED（附漂移） |

**计数**：CONFIRMED 10 · PARTIALLY_CONFIRMED 4 · CONTRADICTED 3 · UNDERSTATED 1 · OVERSTATED 1 · INSUFFICIENT_EVIDENCE 0（共 19 条）

---

## 7. 第一轮 05 的错误清单
- **E1（核心）**：入口条件行为写成无条件红线 → CASE-002/S4=0 被推翻。
- **E2**：文档集内部矛盾（03 S4=1 / 07 S4=3 / 05 S4=0）。
- **E3**：判级错误（EVID-GR-02/GR-04 应 FACT 而非 INFERENCE）。
- **E4**：EVID-GR-01 事实失真（visible:true vs false 对象搞错）。
- **E5**：转移表遗漏"主页 launcher"与"工具条收起"两条 DESTROY 出口。
- **E6**：HIBERNATED 无 FE 镜像（ONE LIFECYCLE OWNER 违反）未记。
- **E7**：遗漏非用户触发的 destroy/create（monitor 自愈、`recover_tab_webview`）。
- **E8**：遗漏 `ActivityBar.vue:156-157` 注释漂移（声称"离开宫格自动关闭"，代码里没有）。
- **E9**：RELOAD 两套实现，其中 `store.tabReload` **无任何调用者**（疑似死代码）。
- **E10**：遗漏 `MainArea.vue:109 ensureTerm()`（VIEW SWITCH 可 CREATE）。
- **E11（新增缺陷）**：`show_for_focus` **只按 `blur_hidden` 过滤、不检查 `hidden`**，且 `record_hidden` 不清 `blur_hidden` ⇒ 两位置同时为真时重聚焦会把已显式隐藏的宫格重新 `win.show()`，产生幽灵浮层（SR-EVID-0021）。

---

## 8. 本复核自己的计数（生命周期维度）
- **S4 = 1**：`GRID_EXIT_DIVERGENCE`（同一"离开宫格"意图 → HIDE vs DESTROY，owner 在 UI 组件）。
- **S3 = 9**：① create_grid 无条件先 destroy；② POSITION⇒SHOW；③ 宫格 CLOSE==KILL==DESTROY；④ ACTIVATE⊇CREATE；⑤ POSITION⊇DESTROY+CREATE（recover 自愈）；⑥ 前端无 HIBERNATED 镜像；⑦ `show_for_focus` 缺 hidden 守卫；⑧ killTerm 吞错+Rust 无孤儿回收；⑨ RELOAD 双实现 + `closeGridOne` 在 count==2 时语义跃变。
- **S2 = 7**：页签视图切换仅离屏；PTY 存活于视图切换且 kill 正确 wait；`setView` 纯净；`syncViewVisibility` 唯一同步 hub；宫格 resize 无 destroy；崩溃自愈重放完整；workspace/预览无原生生命周期。

---

## 9. SR-EVID 证据块（Reviewer B，内联）

**SR-EVID-0001** Claim: 页签创建为 1×1、`visible=false` 子 webview，同一步把旧激活页签异步移出屏幕。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `create_tab`/`spawn_child_window` Line: 723-791 / 104-135
Observed: `spawn_child_window(…,0,0,1,1)`，`visible:false`；随后 `tabs.insert`、`active_tab=Some(id)`、`upsert_session_draft`；其余经 `run_on_main_thread` 排 `hide_bounds`。
Caller: `tab_new`(4266) Callee: `TabManagerState::create_tab` Side effects: 创建 webview + 改 active_tab + 隐藏其它 + 写会话草稿 + 启动资源扫描
Why: 支持 05 CREATED 描述，补出"创建即隐藏其它"。 Confidence: HIGH

**SR-EVID-0002** Claim: 页签 POSITION 蕴含 SHOW，无独立 show 命令。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `apply_bounds_inner` Line: 531-564（关键 555）
Observed: `update_rect` 后立即 `set_visible(&id,true)`，再 `remember_layout`。
Caller: `apply_bounds`(526)/`tab_position`(4299)/`tab_activate`(4723) Callee: 插件 update_rect/set_visible
Why: 坐实 CASE-005 为 **FACT**。 Confidence: HIGH

**SR-EVID-0003** Claim: 页签隐藏一律离屏位移，`set_visible(false)` 被刻意禁用。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `hide_bounds` Line: 566-597
Observed: 保持原 w/h 只改 x 为 -30000；写入 `child_layouts` 供守护线程压制 GTK 漂移。
Why: 支持 HIDE != CLOSE 与"不用 set_visible(false)"。 Confidence: HIGH

**SR-EVID-0004** Claim: `hide_webview`/`hide_all_webviews` 对宫格只发 HideWindow（进程存活），对页签只离屏；无 destroy。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Line: 608-632 / 635-685
Observed: 宫格 `record_hidden(index)` + `GridCmd::HideWindow`；页签跳过 hibernated、有 layout 走 hide_bounds，无 layout 兜底 `update_rect(-30000,-30000,800,600)`。
Why: 支持 HIDE 出口；同时说明一次 hide_all_webviews 会**连带把正在显示的宫格也隐藏**。 Confidence: HIGH

**SR-EVID-0005** Claim: 页签 CLOSE 与 DESTROY 融合；仅 `hibernate_tab` 例外。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `close_tab` Line: 794-823
Observed: `close_tab` ⇒ webview 销毁；清 7 类元数据；`tabs.remove`。无 close-but-keep。
Why: 支持 fused 结论；说明 destroy 副作用面很宽。 Confidence: HIGH

**SR-EVID-0006** Claim: HIBERNATED 是真实第四态（webview 销毁但条目保留），**前端无任何镜像**。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `hibernate_tab`/`start_hibernation_sweeper`/`TAB_HIBERNATE_IDLE_SECS` Line: 4546, 4559-4574, 4578-4608
Observed: 需 `hibernation_enabled`（默认关）；sweeper 60s；仅非激活且 idle>600s；`close_tab` 成功后加入 `hibernated_tabs`，**tabs 表与 url 保留**；前端 tabs[]/activeTabId 完全不变。
Why: **反驳** 05 把"前端镜像 = tabs[]"列为有效镜像 ⇒ ONE LIFECYCLE OWNER 失配（05 未记）。 Confidence: HIGH

**SR-EVID-0007** Claim: ACTIVATE 不导航，但可 CREATE（休眠重建）。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `tab_activate` Line: 4663-4741（4665-4683）
Observed: `hibernated_tabs.remove(id)` 为真 → 取 url → `spawn_child_window(1×1 hidden)`；设 active_tab；仅当记忆布局 `x > -1000` 才立即恢复；清 `last_position_at`。
Why: 支持 ACTIVATE≠NAVIGATE；**挑战** 05 的"仅"字描述。 Confidence: HIGH

**SR-EVID-0008** Claim: POSITION 内部藏着 DESTROY+CREATE（自愈）。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `tab_position`→`recover_tab_webview` Line: 4299-4313 / 163-228
Observed: `apply_bounds` 失败即 `close_tab` + `spawn_child_window(同 id, 原 url, 记忆 rect)`；受 `reserve_tab_recovery_attempt` 预算限制并 emit `tab-recovery`。
Why: 05 矩阵 POSITION 行只写"重定位"，遗漏该分支。 Confidence: HIGH

**SR-EVID-0009** Claim: RELOAD 两套语义，后者无调用者。 Reviewer: B Classification: FACT
File: `src/stores/useBrowserStore.ts` + bridge.rs Symbol: `tabReload`(230-237)/`reloadActive`(256-262)/`tab_reload`(4642-4644)
Observed: `reloadActive`→`tab_reload`→`plugin_eval("location.reload();")`（UI 绑定 ActivityBar:306/TopBar:13/App:205）。`store.tabReload(id)`→`tabOpen` = navigate；`rg tabReload` 在 src 中除定义外 **0 调用者**。
Why: 05 §3 只列一行 reload，属 DUPLICATE SEMANTICS 增补。 Confidence: HIGH

**SR-EVID-0010** Claim: 创建页签时的"隐藏其它"是**异步**的，存在短暂可见窗口。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `create_tab` Line: 771-784
Observed: 注释明确须在 `run_on_main_thread` 排队否则与 webview build 死锁；命令返回时不保证旧页签已离屏。
Why: 生命周期转移非原子，05 转移图未标注。 Confidence: HIGH

**SR-EVID-0011** Claim: `create_grid` 第一行（clamp 后）即无条件 `close_grid`。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `create_grid` Line: 3851-3909（**3854**）
Observed: `close_grid(app.clone())?;` 无守卫 → 即使无旧宫格也执行；其后内存预算、`get_or_spawn`、`CreateTab`、300ms 错峰；失败回滚 `kill_child`。
Why: **确认 EVID-GR-02**，判级 INFERENCE→FACT；"幂等"措辞掩盖"每次必杀"。 Confidence: HIGH

**SR-EVID-0012** Claim: `close_grid` 精确步骤与"先 hide 后 kill"设计意图。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs` Symbol: `close_grid` Line: 3911-3944
Observed: ① 每格 HideWindow；② `sleep(100ms)`；③ CloseTab；④ `shutdown_all()`；⑤ 清 grid-0..MAX_GRID 三类元数据。
Why: 支持 EVID-GR-05；补出精确顺序与 100ms 同步睡眠。 Confidence: HIGH

**SR-EVID-0013** Claim: 每格 = 一个真实 OS 子进程（CLOSE==KILL==DESTROY 物理根因）。 Reviewer: B Classification: FACT
File: `src-tauri/src/grid_process.rs` Symbol: `get_or_spawn`/`spawn`/`spawn_with_state` Line: 279-285/236-238/242-276
Observed: `Command::new(current_exe()).arg("--grid-child").arg(index)`；env `GRID_CHILD_INDEX/GRID_SOCK_PATH`；`start_monitor()`。
Why: 坐实"宫格无软关闭"为架构必然。 Confidence: HIGH

**SR-EVID-0014** Claim: 子进程壳窗创建即 `visible(false)`；`CreateTab` 的 webview 却是 `visible:true`。 Reviewer: B Classification: FACT
File: `src-tauri/src/main.rs` Line: 124-139（**137 visible(false)**）/ 367-390（**376 visible:true**）
Why: 修正 EVID-GR-01 的"webview 隐藏"措辞错误。 Confidence: HIGH

**SR-EVID-0015** Claim: `UpdateRect` 末尾 `win.show()`（**423**），且无 ShowWindow 变体。 Reviewer: B Classification: FACT
File: `src-tauri/src/main.rs` Symbol: `dispatch_grid_cmd` Line: 391-424（397 set_position / 399-403 set_size / 414 update_rect / **423 win.show()**）
Observed: `rg GridCmd::ShowWindow` 0 命中。
Why: **确认 EVID-GR-04**，INFERENCE→FACT。 Confidence: HIGH

**SR-EVID-0016** Claim: `HideWindow` 只 `win.hide()`；`CloseTab` 才销毁 webview。 Reviewer: B Classification: FACT
File: `src-tauri/src/main.rs` Line: 426-432 / 451-462
Why: HIDE≠CLOSE 在宫格侧的**进程级**证明。 Confidence: HIGH

**SR-EVID-0017** Claim: `grid_position` 除定位外还清除 hidden/blur_hidden。 Reviewer: B Classification: FACT
File: `src-tauri/src/bridge.rs`+`grid_process.rs` Symbol: `grid_position`/`record_rect` Line: 4022-4054 / 403-409
Observed: 50ms 同 rect 去重 → `abs_rect` → `record_rect`（写 `hidden=false; blur_hidden=false`）→ `request(UpdateRect,3000)`。
Why: POSITION ⇒ UNSHOW-HIDDEN ⇒ SHOW（三重隐含语义）。 Confidence: HIGH

**SR-EVID-0018** Claim: `kill_child`/`shutdown_all` 先移表再 kill，监控线程不会误重启。 Reviewer: B Classification: FACT
File: `src-tauri/src/grid_process.rs` Line: 689-699 / 754-763
Why: 支持"grid kill 是 waited 的正确实现"，给出全部销毁入口清单。 Confidence: HIGH

**SR-EVID-0019** Claim: `shutdown_all` 仅两处调用。 Reviewer: B Classification: FACT
File: bridge.rs 924-937 / main.rs 1338-1341 / 1569-1571
Why: 证明"视图切换链从不调用 shutdown_all"。 Confidence: HIGH

**SR-EVID-0020** Claim: 存在非用户触发的 DESTROY+CREATE 循环（崩溃自愈）。 Reviewer: B Classification: FACT
File: `src-tauri/src/grid_process.rs` Symbol: `start_monitor`/`replay` Line: 540-616 / 619-685
Observed: 每 500ms `try_wait`；退出即 remove + `spawn_with_state(保存 last_url/last_rect/hidden/blur_hidden)` + `replay()`。
Why: 05 §3 矩阵缺这一行。 Confidence: HIGH

**SR-EVID-0021** Claim: **新发现缺陷**——`show_for_focus` 只按 `blur_hidden` 过滤，忽略 `hidden`；`record_hidden` 不清 `blur_hidden`。 Reviewer: B Classification: INFERENCE
File: `src-tauri/src/grid_process.rs` Symbol: `record_hidden`/`hide_for_blur`/`show_for_focus` Line: 411-415 / 486-508 / 511-534
Observed: 两独立位；`hide_for_blur` 只在 `!hidden && !blur_hidden` 时置位；`show_for_focus` 过滤条件**只有** `blur_hidden==true`。
Why: 若两位置同时为真，重聚焦会把已显式隐藏的宫格重新 show ⇒ 幽灵浮层。05/03/07 均未记录。 Confidence: MEDIUM

**SR-EVID-0022** Claim: `buildGrid` 完整前端步骤（含不覆盖 mainView 的守卫）。 Reviewer: B Classification: FACT
File: `src/stores/useBrowserStore.ts` Symbol: `buildGrid` Line: 300-343
Observed: `gridSession++` → urls → `createGrid` → 降级 gridCount → `gridOpen=true` → **仅当 mainView 非 grid/browser 才置 grid**(322-324) → `layoutGrid()` + 400ms 后再排 → `syncFreeze()`。
Why: 补出"进入 grid 视图不由 buildGrid 负责"。 Confidence: HIGH

**SR-EVID-0023** Claim: `closeGridAll` 真实顺序（tabActivate 在 mainView 复位之前）。 Reviewer: B Classification: FACT
File: `src/stores/useBrowserStore.ts` Symbol: `closeGridAll` Line: 483-511
Why: **修正** 05 §2.2 顺序，补齐遗漏项。 Confidence: HIGH

**SR-EVID-0024** Claim: `setView` 本身零原生副作用；视图切换后果 100% 由 watch→syncViewVisibility 发出。 Reviewer: B Classification: FACT
File: `src/stores/useLayoutStore.ts` + useBrowserStore.ts Line: 193-199 / 658-664 / 646-655
Observed: `setView` 只改 mainView + fileEditorOpen + navSection；browser/grid 分支 `await hideAllWebviews()` → `relocate()`。
Why: 页签/终端侧红线成立的核心证据。 Confidence: HIGH

**SR-EVID-0025** Claim: 每次进入 grid/browser 视图都会先 HideWindow 再 UpdateRect show（hide→show 往返）。 Reviewer: B Classification: FACT
File: useBrowserStore.ts + useBrowserHost.ts Line: 646-655 / 59-95 / 108-157
Why: 05 §2.2 只写"UpdateRect+show"，漏掉前置 hide_all_webviews。 Confidence: HIGH

**SR-EVID-0026** Claim: **反例**——`HomeLaunchers.openArea` 在切换视图意图下直接 `closeGridAll()`（DESTROY）。 Reviewer: B Classification: FACT
File: `src/components/home/HomeLaunchers.vue` Symbol: `openArea` Line: 42-54（**47**）
Observed: `if (view !== "grid" && browser.gridOpen) await browser.closeGridAll();`
Why: **直接推翻** 05 §2.3/§8 的"视图切换仅 hide / CASE-002 通过 / S4=0"。 Confidence: HIGH

**SR-EVID-0027** Claim: 第二个 DESTROY 出口：`toggleGridToolbar` 收起 = closeGridAll。 Reviewer: B Classification: FACT
File: `src/stores/useLayoutStore.ts` Symbol: `toggleGridToolbar` Line: 210-215
Why: 05 §2.3 转移表未收录该行。 Confidence: HIGH

**SR-EVID-0028** Claim: 不销毁的 grid 入口枚举（已开则仅重排）。 Reviewer: B Classification: FACT
File: ActivityBar.vue / UnifiedTabBar.vue / App.vue / useWorkbenchStore.ts Line: 159-177(167) / 170 / 196-199 / 22
Observed: 全部形如 `if (browser.gridOpen) layoutGrid(); else buildGrid();`
Why: 与 0026/0027 共同构成"入口决定论"两侧证据。 Confidence: HIGH

**SR-EVID-0029** Claim: `closeGridOne` 剩 2 格时再关升级为整体 closeGridAll。 Reviewer: B Classification: FACT
File: useBrowserStore.ts + bridge.rs Line: 513-527 / 4059-4071
Why: 又一个"意图相同而结局不同"的例子。 Confidence: HIGH

**SR-EVID-0030** Claim: 宫格**无**专用 reload 动词。 Reviewer: B Classification: FACT
File: bridge.rs `pub fn grid_*` 仅 4 个（3949/3999/4022/4059）；GridCmd 无 Reload。
Why: 05 的 INFERENCE 可升为 FACT（未实现）。 Confidence: HIGH

**SR-EVID-0031** Claim: 终端面板卸载不杀 PTY；唯一 kill 入口是 ↻/⏹。 Reviewer: B Classification: FACT
File: `src/components/system/TerminalPane.vue` Line: 223-233 / 215-218 / 219-221
Why: **确认** EVID-TR-02。 Confidence: HIGH

**SR-EVID-0032** Claim: 切到终端视图唯一动作是"没有才建"，永不 kill。 Reviewer: B Classification: FACT
File: MainArea.vue + useSystemStore.ts Line: 105-112 / 252-254
Why: "VIEW SWITCH 可 CREATE 但绝不 DESTROY"的实例。 Confidence: HIGH

**SR-EVID-0033** Claim: `term_kill` 正确 wait（进程组+线程回收），先释放表锁。 Reviewer: B Classification: FACT
File: bridge.rs + terminal.rs Line: 4817-4829 / 661-675 / 678-707 / 731-739
Why: **确认** EVID-TR-04。 Confidence: HIGH

**SR-EVID-0034** Claim: PTY 状态真源在 Rust；前端只有输出历史快照（上限 40 块）。 Reviewer: B Classification: FACT
File: useSystemStore.ts + TerminalPane.vue Line: 146-150, 172-192 / 197-199
Why: 确认 PTY 存活；回放是**近似真值**，非完整重建。 Confidence: HIGH

**SR-EVID-0035** Claim: PTY 泄漏窗口（killTerm 吞错 + Rust 无孤儿回收）。 Reviewer: B Classification: INFERENCE
File: useSystemStore.ts + bridge.rs Line: 258-265 / 4820, 899-921
Why: 05/07 判定 PTY kill「S2 干净」低估了前端吞错带来的 owner 失配。 Confidence: MEDIUM

**SR-EVID-0036** Claim: 终端 resize 是真实 PTY 尺寸变更，前端只做去重。 Reviewer: B Classification: FACT
File: terminal.rs + useTerminalResize.ts Line: 648-658 / 15-18
Why: **确认** EVID-TR-03。 Confidence: HIGH

**SR-EVID-0037** Claim: 工作区切换对任何原生资源生命周期**无影响**。 Reviewer: B Classification: FACT
File: 双向 rg `switch_workspace`/`switchWorkspace`/`setWorkspace` 全仓 0 命中
Why: EVID-WS-01 由 UNVERIFIED 收紧为 FACT(no-op)。 Confidence: HIGH

**SR-EVID-0038** Claim: `ActivityBar` 注释声称"离开宫格视图时自动关闭宫格"，但代码中并未执行。 Reviewer: B Classification: FACT
File: `src/components/layout/ActivityBar.vue` Line: 156-177
Why: 证明"宫格退出语义"在代码库内部连注释层都不统一（Trust code not comments）。 Confidence: HIGH
