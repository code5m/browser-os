# 05-LIFECYCLE-MODEL.md — 语义治理生命周期审计

- 审计目标：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`（Tauri2 + Vue3 桌面浏览器-OS）
- 范围：浏览器页签 / 宫格 / 终端 三种核心资源在 Rust 后端（`src-tauri/src/bridge.rs`, `grid_process.rs`, `grid_ipc.rs`, `main.rs`, `terminal.rs`）与前端（`src/stores/useBrowserStore.ts`, `useLayoutStore.ts`, `useSystemStore.ts`, `useGridArchiveStore.ts`, `src/composables/useBrowserHost.ts`, `useTerminalResize.ts`, `src/components/browser/BrowserHost.vue`, `src/components/system/TerminalPane.vue`）的真实生命周期状态机。
- 方法：只读静态溯源（rg / read_file）。每条证据标注符号 + 文件 + 行区间，并显式区分 FACT / INFERENCE / RECOMMENDATION / UNVERIFIED。
- 治理红线：ONE LIFECYCLE OWNER；VIEW SWITCH != DESTROY；HIDE != CLOSE；CLOSE != DESTROY；POSITION != SHOW；ACTIVATE != NAVIGATE。

---

## 1. 浏览器页签生命周期（BROWSER TAB）

### 1.1 代码中真实存在的状态
| 概念状态 | 后端真源（Rust） | 前端镜像（TS） | 是否独立存在 |
|---|---|---|---|
| ABSENT | 不在 `AppState.tabs` | 不在 `tabs[]` | 是 |
| CREATED / ALIVE | `manager.create_tab` 已建 webview | `tabs.push` | 是 |
| ACTIVE | `AppState.active_tab == id`，且 `child_layouts` 记真实 rect，`set_visible(true)` | `activeTabId == id` | 是 |
| INACTIVE（=HIDDEN） | webview 仍存在但被 `hide_bounds` 移到屏外 `(-30000,...)`；**不存在"可见但非激活"态** | `activeTabId != id` | 是（与 HIDDEN 不区分） |
| HIBERNATED | `hibernated_tabs` 集合命中；webview 已 `close_tab` 销毁，但 `tabs` 表与 URL 保留 | 同 `tabs[]`（仍是条目） | 是（与 CLOSED 区分的关键） |
| CLOSED / DESTROYED | 普通 `close_tab`：销毁 webview **且**从 `tabs` 移除 → **二者 fused** | `tabs.splice` | 否（页签不分 CLOSED/DESTROYED） |

**结论（FACT）**：页签的实际状态集为 ABSENT / ALIVE(ACTIVE|OFFSCREEN) / HIBERNATED / CLOSED。INACTIVE 与 HIDDEN 在代码中是同一态（offscreen-move）；CLOSED 与 DESTROYED 对普通页签是 fused；HIBERNATED 是"关闭 webview 但保留条目"的混合态，正是 CLOSE != DESTROY 原则的正面体现。

### 1.2 真实状态转移（含 owner）
- ABSENT → CREATED/ACTIVE：`create_tab` (bridge.rs:723-791) → `spawn_child_window` (104-135) 建 1×1 `visible=false` webview；命令壳 `tab_new` (4266-4269)；前端 `tabNew` (useBrowserStore.ts:158-174)。**owner：Rust `AppState.tabs` + browser-tabs 插件**（权威），前端 `useBrowserStore` 为镜像控制器。
- CREATED → ACTIVE：`tab_activate` (4663-4741)：设 `active_tab`、对其余 `hide_bounds`、对激活页 `apply_bounds_inner`（→`set_visible(true)`）；前端 `tabSwitch` (175-183)。
- ACTIVE → INACTIVE(HIDDEN)：任何切换激活 / 视图切走 → `hide_bounds` (570-597) 屏外移；**不关闭**（FACT：CLOSE != HIDE）。
- ACTIVE → HIBERNATED：`hibernate_tab` (4559-4574)：`manager.close_tab` 销毁 webview，但保留 `tabs` 条目 + 插入 `hibernated_tabs`；由 `start_hibernation_sweeper` (4578-4608) 在开启且仅非激活超时 600s 触发。激活时 `tab_activate` (4664-4683) 按 URL 重建（1×1 隐藏）。
- ALIVE → CLOSED/DESTROYED：`close_tab` (794-823) + 前端 `tabClose`/`closeTabNow` (188-211) 先 `recordClose` 入内存栈再 `bridge.tabClose`；同步从 `tabs`、各布局表、资源缓冲、会话草稿移除。
- CLOSED → ABSENT：条目已从 `tabs` 移除即视为 ABSENT。
- CLOSED → 恢复(近似 CREATED)：`restoreRecent` (222-229) 仅按 URL 重新 `tabNew`，**不是复活原 webview**。

---

## 2. 宫格生命周期（GRID）

### 2.1 真实状态
| 概念状态 | 后端真源 | 前端镜像 | 说明 |
|---|---|---|---|
| ABSENT | `GridProcessManager.children` 无该 index | `gridOpen=false` / `gridCount` | 进程未 spawn |
| VISIBLE | 子进程 alive + `last_rect` 记真实值 + 非 hidden/blur_hidden；壳窗口 `show()` | `gridOpen=true && mainView==='grid'` | 子进程 webview 铺满屏幕 |
| HIDDEN | `hidden=true` 或 `blur_hidden=true`；壳窗口 `hide()`，但子进程 + webview 仍存活 | `gridOpen=true && mainView!=='grid'`（或失焦） | **视图切换仅 hide，不 kill** |
| DESTROYED | `kill_child`/`shutdown_all` 后从 `children` 移除；子进程退出 | `gridOpen=false` | 进程被杀 |

### 2.2 转移 + 触发的动作
- ABSENT → VISIBLE（create）：`create_grid` (3851-3909)。**首行即 `close_grid(app.clone())?`（3854）→ 隐式 DESTROY 旧宫格**（idempotent 重建）。随后 `get_or_spawn` (grid_process.rs:279) + UDS `GridCmd::CreateTab`；子进程 `dispatch_grid_cmd` CreateTab (main.rs:367) 建 webview（1×1, `auto_resize`）。前端 `buildGrid` (useBrowserStore.ts:300-343) 先 `gridSession++`、调 `createGrid`、置 `gridOpen=true`，并以 `mainView` 守卫避免二次覆盖（322-324）。
- VISIBLE → HIDDEN（视图切换 hide）：`useBrowserStore.syncViewVisibility` (646-655) → `hideAllWebviews` (bridge.rs:635-685) → 对每格 `record_hidden` + `GridCmd::HideWindow`（main.rs:426 调 `win.hide()`）。`mainView` watch (658-664) 驱动。**FACT：切到 home/files/term/browser 仅 hide，不 kill**（CASE-002 未违反）。
- HIDDEN → VISIBLE（resume show）：`mainView` 切回 `'grid'` → `layoutGrid` → `scheduleGrid`/`layoutGridNow` (useBrowserHost.ts:97-157) → `grid_position` (bridge.rs:4022) → `GridCmd::UpdateRect` → 子进程 `dispatch_grid_cmd` UpdateRect (main.rs:391-424) `set_position`+`set_size`+**`win.show()`**。
- VISIBLE → move/resize（窗口 resize/maximize/restore）：`main.rs` `WindowEvent::Moved|Resized` → `reposition_visible` (grid_process.rs:437-457) 按记忆 rect 重发 `UpdateRect`。前端 `useBrowserHost` 监听 resize 触发 `schedulePosition` (useBrowserHost.ts:175-179)。**无 destroy**。
- VISIBLE → navigate：`grid_open` (3949-3977) → `GridCmd::Navigate`（main.rs:439）。
- reload：宫格无独立 reload 命令；复用 `grid_open`/Navigate。INFERENCE。
- VISIBLE/HIDDEN → DESTROYED（close）：`close_grid` (3915-3944)：先 `HideWindow` 再 `CloseTab` 再 `shutdown_all`（kill 进程）；前端 `closeGridAll` (useBrowserStore.ts:483-511) 先翻 `gridOpen=false` 再 `bridge.closeGrid`、复位 `mainView='browser'`、重激活活动页签。`closeGridOne` (513-527) → `grid_close_one` (4059-4071) 单格 `HideWindow`+`CloseTab`+`kill_child`。
- 崩溃自愈（非用户触发）：`start_monitor` (grid_process.rs:540-616) 检测退出 → 保状态重启 → `replay` (619-685) 重放 `CreateTab`+`UpdateRect`+`SetZoom`。
- 失焦隐藏（blur）：`hide_for_blur`/`hide_for_blur_if_no_child_focus` (grid_process.rs:470-508)；聚焦恢复 `show_for_focus` (511-534)；主窗最小化轮询同样隐藏/恢复。**均不 kill**。
- 应用关闭：`register_shutdown_tasks` → `close-tabs` + grid `shutdown_all`。DESTROY（预期）。

### 2.3 转移 → 动作 速查
| 转移 | create | show | hide | move/resize | navigate | reload | close | destroy | kill |
|---|---|---|---|---|---|---|---|---|---|
| grid→browser | – | – | ✅HideWindow | – | – | – | – | – | – |
| browser→grid | – | ✅UpdateRect+show | – | – | (re-show) | – | – | – | – |
| grid→home | – | – | ✅ | – | – | – | – | – | – |
| home→grid | – | ✅ | – | – | – | – | – | – | – |
| grid→files | – | – | ✅ | – | – | – | – | – | – |
| files→grid | – | ✅ | – | – | – | – | – | – | – |
| workspace switch | UNVERIFIED | | | | | | | | |
| window resize | – | – | – | ✅reposition_visible | – | – | – | – | – |
| maximize/restore | – | – | – | ✅Resized→reposition | – | – | – | – | – |
| app shutdown | – | – | – | – | – | – | ✅close-tabs | ✅shutdown_all | ✅ |

---

## 3. 生命周期矩阵（Resource × From × Event × To × Expected × Actual × Violation × Evidence）

| Resource | From | Event | To | Expected | Actual | Violation | Evidence |
|---|---|---|---|---|---|---|---|
| tab | ACTIVE | 视图切到 home/files/term | OFFSCREEN(隐藏) | 仅隐藏不销毁 | `hideAllWebviews` 屏外移 | 否 | EVID-BT-03 |
| tab | OFFSCREEN | 切回 browser/激活 | ACTIVE | 重定位显示 | `tab_activate`→`apply_bounds_inner`+`set_visible` | 否 | EVID-BT-02 |
| tab | ALIVE | `tabClose` | CLOSED/DESTROYED | 销毁 webview 并从列表移除 | `close_tab` 双清 | 否 | EVID-BT-04 |
| tab | ALIVE | 休眠开关+超时 | HIBERNATED | 关 webview 留条目 | `hibernate_tab` | 否（正面 CLOSE≠DESTROY） | EVID-BT-05 |
| tab | HIBERNATED | `tab_activate` | ACTIVE | 按 URL 重建 | `spawn_child_window` | 否 | EVID-BT-06 |
| grid | ABSENT | `create_grid` | VISIBLE | 建子进程+webview | `get_or_spawn`+`CreateTab` | 否（但隐式销毁旧格） | EVID-GR-01 |
| grid | VISIBLE | 视图切换 | HIDDEN | 仅 hide 不 kill | `HideWindow` | **否（CASE-002 通过）** | EVID-GR-03 |
| grid | HIDDEN | 视图切回 grid | VISIBLE | show | `UpdateRect`+`show()` | 否 | EVID-GR-04 |
| grid | VISIBLE | `closeGridAll` | DESTROYED | 关并 kill 进程 | `HideWindow`+`CloseTab`+`shutdown_all` | **耦合：CLOSE==KILL==DESTROY** | EVID-GR-05 |
| grid | VISIBLE | 主窗 resize/最大化 | VISIBLE(重定位) | move/resize 不销毁 | `reposition_visible` | 否 | EVID-GR-06 |
| grid | VISIBLE | 创建新宫格 | DESTROYED(旧)+VISIBLE(新) | 幂等重建 | `create_grid` 先 `close_grid` | 否（设计性耦合） | EVID-GR-02 |
| term | ABSENT | `termSpawnChannel` | ALIVE(PTY) | 建 PTY+管道 | `spawn_terminal` | 否 | EVID-TR-01 |
| term | ALIVE | 视图切走（pane unmount） | ALIVE(会话留存) | **不销毁**，仅 xterm 卸载 | `TerminalPane.onBeforeUnmount` 不 kill | 否（PTY 存活，回放） | EVID-TR-02 |
| term | ALIVE | `termResize` | ALIVE | 真实 resize | `terminal::resize` | 否 | EVID-TR-03 |
| term | ALIVE | `termKill`/重启/关闭 | DESTROYED | 置 stop+杀进程组+回收线程 | `terminate_session` | 否 | EVID-TR-04 |
| 前端/原生 | 切视图 | 同步显隐 | 双图层一致 | CSS 与原生同步 | `syncViewVisibility` + `BrowserHost` visibility | 否（CASE-007 见 §5） | EVID-LY-01 |

---

## 4. CASE 核验（红线）

- **CASE-002 VIEW SWITCH == DESTROY？ → 否（未违反）。** 视图切换路径 `syncViewVisibility`→`hideAllWebviews` 仅把子 webview 屏外移（`hide_bounds`/`HideWindow`），`close_tab`/`close_grid` 均不由视图切换触发。Classification: FACT, Confidence: HIGH.
- **HIDE == CLOSE？ → 否（未违反）。** `hide_bounds`/`hide_webview`/`hide_all_webviews`/`HideWindow` 全部只移屏外/隐藏窗口，**不调用 `manager.close_tab`/`kill_child`**。Classification: FACT, Confidence: HIGH.
- **CLOSE == KILL？ → 页签：不适用；宫格：是（架构性耦合，非 bug）。** 宫格每格 = 一个 OS 子进程，关闭即 `kill_child`/`shutdown_all`。Classification: INFERENCE(架构耦合), Confidence: HIGH.
- **POSITION == SHOW（CASE-005）？ → 是（机制性耦合，非运行时 bug）。** 隐藏用屏外移（`-30000`），显示只能经重定位（`apply_bounds_inner` 内 `set_visible(true)`，bridge.rs:555；宫格 `UpdateRect` 内 `win.show()`，main.rs:423）。Classification: INFERENCE, Confidence: HIGH.
- **ACTIVATE == NAVIGATE？ → 否（未违反）。** `tab_activate`(4663-4741) 仅设 `active_tab`、隐藏其余、重定位激活页；**调用 `navigate` 的是 `tab_open`/`tabNavigate`/`tabReload`，非 `tab_activate`**。Classification: FACT, Confidence: HIGH.

---

## 5. CASE-007：原生 WebView bounds 与 BrowserHost 可见性的双层

- 前端层：`BrowserHost.vue:15` 对宿主 div 设 `visibility: isBrowserVisible ? 'visible' : 'hidden'`（`isBrowserVisible = mainView==='browser'`, useBrowserStore.ts:149-151）。该 div 本体无真实网页内容（网页是独立顶层原生 webview），此 CSS 仅作用于占位壳。
- 原生层：真实显隐由 Rust 控制——`hideAllWebviews`/`hide_bounds`（屏外移）与 `tab_position`/`apply_bounds_inner`（`set_visible(true)`）；宫格由 `HideWindow`/`UpdateRect+show`。
- 同步点：`useBrowserStore` 的 `watch(mainView)`(658-664)→`nextTick(syncViewVisibility)`(646-655) 在视图切换后统一重排/隐藏原生 webview；`BrowserHost` 的 CSS 与 `syncViewVisibility` 共同收敛。**两层若失配会导致"占位隐藏但原生网页仍盖屏"的幽灵浮层**——代码已用 `syncViewVisibility` 兜底。
- Classification: FACT, Confidence: HIGH.

---

## 6. 终端生命周期（简版）

- 创建：`spawn_terminal`(terminal.rs:552-645) 开 PTY、`setsid` 起 shell、起 worker/pump 管道；命令壳 `term_spawn_channel`(bridge.rs:4770-4783) 把 session 存入 `AppState.terminals`。前端 `spawnTerm`(useSystemStore.ts:231-250)。
- 写入：`term_write`(bridge.rs:4787) 经 session.writer 写 PTY。
- resize：`term_resize`(4801)→`terminal::resize`(648-658) 真实改 PTY 尺寸；前端 `useTerminalResize`(useTerminalResize.ts) 做去重+140ms 静默+500ms 硬上界。
- kill/destroy：`term_kill`(4817-4829) 先从 `terminals` 表移除并**释放锁**再 `terminate_session`(661-675)：置 `stop`→`terminate_group`(678) `SIGTERM`→轮询→仍活 `SIGKILL`→`wait`→`join_pipeline`。
- **PTY 存活于 Tab/视图切换**：`TerminalPane.vue:199` 挂载时 `replayTermHistory`；`onBeforeUnmount`(223-233) 仅 `term.dispose()` **不调用 `killTerm`**；PTY 会话存于 Rust `terminals` 与前端 `termHistories`，面板重建后回放最近 40 块。FACT：xterm 卸载但 PTY 存活，需回放。
- workspace switch：Rust 无 `switchWorkspace`/`switch_workspace` 命令（search 0 命中）；前端 `useWorkspaceStore` 仅管理文件树/artifact/仓库。其对页签/宫格生命周期的影响 UNVERIFIED。EVID-WS-01, Classification: UNVERIFIED, Confidence: MEDIUM。

---

## 7. 证据附录（EVID）

**EVID-BT-01** Claim: 页签创建在后端经 `spawn_child_window` 建 1×1 `visible=false` webview，前端 `tabs.push` 并记录 `active_tab`。Classification: FACT. File: src-tauri/src/bridge.rs. Symbol: `create_tab` (723-791), `spawn_child_window` (104-135). Current line: 723 / 104. Confidence: HIGH.

**EVID-BT-02** Claim: `tab_activate` 仅改激活态+隐藏其余+重定位激活页，不导航。Classification: FACT. File: src-tauri/src/bridge.rs. Symbol: `tab_activate` (4663-4741). Current line: 4663. Confidence: HIGH.

**EVID-BT-03** Claim: 视图切走仅隐藏页签（屏外移），不销毁。Classification: FACT. File: src/stores/useBrowserStore.ts + src-tauri/src/bridge.rs. Symbol: `syncViewVisibility`(646-655) / `hideAllWebviews`(635-685) / `hide_bounds`(570-597). Current line: 646 / 635 / 570. Confidence: HIGH.

**EVID-BT-04** Claim: 普通 `tabClose` = 销毁 webview 且从列表移除。Classification: FACT. File: src-tauri/src/bridge.rs. Symbol: `close_tab` (794-823). Current line: 794. Confidence: HIGH.

**EVID-BT-05** Claim: 休眠态销毁 webview 但保留 `tabs` 条目与 URL（HIBERNATED 是 CLOSE!=DESTROY 的正面实例）。Classification: FACT. File: src-tauri/src/bridge.rs. Symbol: `hibernate_tab` (4559-4574), `start_hibernation_sweeper` (4578-4608). Current line: 4559. Confidence: HIGH.

**EVID-BT-06** Claim: 休眠页签激活时按 URL 重建 webview（1×1 隐藏），前端 `tab_position` 放大。Classification: FACT. File: src-tauri/src/bridge.rs. Symbol: `tab_activate` 休眠重建分支 (4664-4683). Current line: 4664. Confidence: HIGH.

**EVID-GR-01** Claim: `create_grid` 经 `get_or_spawn`+UDS `CreateTab` 建子进程 webview 进入 VISIBLE。Classification: FACT. File: src-tauri/src/bridge.rs + grid_process.rs + main.rs. Symbol: `create_grid`(3851-3909) / `get_or_spawn`(279-285) / `dispatch_grid_cmd::CreateTab`(main.rs:367). Confidence: HIGH.

**EVID-GR-02** Claim: `create_grid` 首行即 `close_grid`，即"创建"隐式 DESTROY 旧宫格（幂等重建）。Classification: INFERENCE(设计性耦合). File: src-tauri/src/bridge.rs. Symbol: `create_grid` (3854). Confidence: HIGH.

**EVID-GR-03** Claim: 视图切换对宫格仅 `HideWindow`，不 kill（CASE-002 通过）。Classification: FACT. File: src/stores/useBrowserStore.ts + bridge.rs + main.rs. Symbol: `syncViewVisibility`(646-655) / `hide_all_webviews`(635-685) / `dispatch_grid_cmd::HideWindow`(main.rs:426). Confidence: HIGH.

**EVID-GR-04** Claim: 宫格 `UpdateRect` 末端执行 `win.show()`，故 position 蕴含 show（CASE-005 机制耦合）。Classification: INFERENCE. File: src-tauri/src/main.rs. Symbol: `dispatch_grid_cmd::UpdateRect` (391-424, 关键 423 `win.show()`). Confidence: HIGH.

**EVID-GR-05** Claim: 宫格 `close_grid`/`grid_close_one` 最终 `kill_child`/`shutdown_all`——CLOSE==KILL==DESTROY（架构性耦合）。Classification: INFERENCE(架构约束). File: src-tauri/src/bridge.rs + grid_process.rs. Symbol: `close_grid`(3915-3944) / `grid_close_one`(4059-4071) / `kill_child`(689-699) / `shutdown_all`(755-763). Confidence: HIGH.

**EVID-GR-06** Claim: 主窗 resize/maximize/restore 仅 `reposition_visible` 重定位宫格，不销毁。Classification: FACT. File: src-tauri/src/main.rs + grid_process.rs. Symbol: `on_window_event Moved|Resized`(main.rs:1321-1323) / `reposition_visible`(grid_process.rs:437-457). Confidence: HIGH.

**EVID-TR-01** Claim: 终端创建开 PTY+shell+管道并登记到 `AppState.terminals`。Classification: FACT. File: src-tauri/src/terminal.rs + bridge.rs. Symbol: `spawn_terminal`(552-645) / `term_spawn_channel`(4770-4783). Confidence: HIGH.

**EVID-TR-02** Claim: 终端面板（xterm）卸载不杀 PTY，会话存活并回放。Classification: FACT. File: src/components/system/TerminalPane.vue + useSystemStore.ts. Symbol: `onBeforeUnmount`(223-233) / `replayTermHistory`(186-192). Confidence: HIGH.

**EVID-TR-03** Claim: `term_resize` 真实改 PTY 行列，前端经静默窗口去重下发。Classification: FACT. File: src-tauri/src/bridge.rs + terminal.rs + useTerminalResize.ts. Symbol: `term_resize`(4801-4813) / `terminal::resize`(648-658) / `useTerminalResize`(37-97). Confidence: HIGH.

**EVID-TR-04** Claim: `term_kill` 置 stop+杀进程组+回收线程，并先释放表锁避免阻塞其它终端。Classification: FACT. File: src-tauri/src/bridge.rs + terminal.rs. Symbol: `term_kill`(4817-4829) / `terminate_session`(661-675) / `terminate_group`(678-707). Confidence: HIGH.

**EVID-LY-01** Claim: 视图切换时前端 CSS 可见性与原生 webview bounds 双层同步（CASE-007）。Classification: FACT. File: src/components/browser/BrowserHost.vue + useBrowserStore.ts. Symbol: `BrowserHost` template `visibility`(15) / `syncViewVisibility`(646-655) / `watch(mainView)`(658-664). Confidence: HIGH.

**EVID-LY-02** Claim: 页签显示由 `apply_bounds_inner` 内 `set_visible(true)` 完成，显示与定位熔接（POSITION==SHOW）。Classification: INFERENCE. File: src-tauri/src/bridge.rs. Symbol: `apply_bounds_inner` (531-564, 关键 555). Confidence: HIGH.

**EVID-WS-01** Claim: 工作区切换对浏览器页签/宫格生命周期的影响——源码未发现专用 Rust 命令或拆除路径。Classification: UNVERIFIED. File: src-tauri/src/* + useWorkspaceStore.ts. Symbol: `useWorkspaceStore`(`workspace` 树/artifact) ; Rust `search switchWorkspace=0`. Confidence: MEDIUM.

---

## 8. 计数、违规清单、与其它 Agent 的冲突

### S 计数
- **S4（第一轮局部统计）= 0 → 已被第二轮推翻**：本 §8 第一轮的 `S4=0`（"未发现运行时红线违反"）被 17B（`SR-EVID-0023`/`C13` CONTRADICTED）与终审 `18` SECOND-CONFLICT-001 推翻——`HomeLaunchers.openArea:47` 在切到非宫格视图前 `closeGridAll`→`shutdown_all`（DESTROY），是**活的视图切换型红线违反**（ADR-GRID-001）。**生命周期主题实际存在 1 条 S4 = GRID_EXIT_DIVERGENCE**；Tab/PTY 侧红线（VIEW SWITCH≠DESTROY、HIDE≠CLOSE、ACTIVATE≠NAVIGATE、PTY 存活）仍成立。本 `S4=0` 作废，全局以 `00`/`18` §4 的 `S4=2` 为准。
- **S3 = 3**（架构性耦合，非 bug 但值得记录）：
  1. 宫格 CLOSE == KILL == DESTROY（per-cell 进程模型的固有约束）。
  2. POSITION == SHOW（隐藏=屏外移，显示只能经重定位，无正交 show 命令）。
  3. create_grid 隐式 DESTROY 旧宫格（幂等重建的副作用耦合）。
- **S2 = 7**（观察/合规，含正面实例）。

### 发现的生命周期真源分散（ONE LIFECYCLE OWNER 视角）
页签/宫格生命周期真源分散在 Rust（`AppState.tabs`/`active_tab`/`hibernated_tabs`/`child_layouts`/`grid_manager`）与前端（`useBrowserStore.tabs`/`activeTabId`/`gridOpen`）两份镜像——属 S3 级耦合，需警惕二者失配（前端翻 `gridOpen=false` 先于 IPC 往返正是为此）。

### 与 Agent A / B / E 的冲突提示
- 与 Agent A：若其将 INACTIVE 与 HIDDEN 建模为两个独立持久态，本报告认定二者为同一态（offscreen-move）；若其未列出 HIBERNATED 态，应补入。
- 与 Agent B：若其动作表包含独立 `show` 动作，本报告结论为**无独立 show 命令**——显示动作熔接在 `position`/`UpdateRect` 内；若其 `create` 动作未标注"隐式 destroy 旧实例"，需补。
- 与 Agent E：本报告的 native 副作用真源是 `browser-tabs` 插件的 `TabManagerState`（页签）与每格独立子进程 `GridProcessManager`（宫格）及 `portable_pty`（终端）。若 Agent E 将"hide"理解为 `webview.hide()`，需注意本代码刻意**不用 `set_visible(false)`**（WebKitGTK 死锁，bridge.rs:567-569 注释），隐藏一律用屏外 `update_rect(-30000)`。

### 与终审（18 SECOND-CONFLICT，第二轮）的衔接
- **S4=0 作废**：第一轮 16 CONFLICT-03 建议把"离开宫格意图分叉"降 S3，但 17B/18 终审驳回——`HomeLaunchers.openArea:47` 是活的 DESTROY 出口，活违反 ACCEPTED 红线 ADR-GRID-001，按 ADR-SEVERITY-001（S4=红线违反）应为 **S4（GRID_EXIT_DIVERGENCE）**。`05` 原 `S4=0` 已作废，全局以 `00`/`18` §4 的 `S4=2` 为准（另 1 条 S4 = NATIVE_VISIBILITY_HIDDEN_STATE，属 01/07 主题，非本生命周期章节）。
- 本报告 S3=3（CLOSE==KILL==DESTROY / POSITION==SHOW / create_grid 隐式 destroy）与 18 终审一致，保留为 S3（架构耦合，非 bug）：18 SECOND-CONFLICT-004 确认 POSITION==SHOW 为机制耦合、无静默错误（S3）；SECOND-CONFLICT-015 确认 `create_grid` 无条件 destroy+create 为 FACT、S3。
- **hide 能力澄清（修正"缺少 hide 原语"误读）**：底层 hide capability **已存在**——原生层 `GridCmd::HideWindow`（main.rs:426）、bridge 层 `hideWebview`（bridge.rs:608）/`hideAllWebviews`（:635），隐藏一律走屏外 `-30000`（bridge.rs:566-569 明文禁用 `set_visible(false)`）。本段与 08 DUP-006 / 09 SMF-003 所称"无独立 hide 原语"指的**不是缺失底层能力**，而是**缺少单一、业务级 canonical grid-hide / view-switch lifecycle entrypoint（如目标 `exitGrid`）**——不得据此引导后续 Agent 重复新增 Rust `HideWindow`（它已存在）。
- "宫格可见三标志"（08 DUP-002）经 18 SECOND-CONFLICT-002 由 S4 降 **S2**（正交态，非多真源）；isBrowserView/isBrowserVisible（08 DUP-001）经 SECOND-CONFLICT-003 降 **S2**。二者均不在本报告 S3 列表，与终审一致。
