# Phase 1 Browser/Grid — FINAL REPORT

> 本文件是 Phase 1 的**最终状态固化**（Closeout 资产），不是设计稿、不是待办。
> 设计过程见同目录 `00-PHASE1A-EXECUTIVE.md` … `10-IMPLEMENTATION-PLAN.md`；本文件只记录**最终结论与证据**。

| 项 | 值 |
|---|---|
| 范围 | Browser / Grid 单一语义（Single Semantics） |
| 最终 tag | `semantic-phase1-browser-grid-pass` → `7e8d86771f24133780b6637117166e88624df005` |
| 代码 tag | `semantic-phase1-browser-grid-code-pass` → `a30fd573091fac50513ad2e23bde26ee5e0ab6aa` |
| 状态 | **PASS**（架构 / 实现 / 静态 / 运行时 harness / 原生 GUI 验收 全通过） |
| 推送 | 未推送（本地） |

---

## 1.1 Architecture Summary（最终状态模型）

```text
mainView
=
用户当前希望看到的 Main Surface
（Grid 是 mainView === "grid" 这一合法主表面，不是叠加维度）

gridOpen
=
Grid resource 是否存在
（存在 ≠ 可见；可见还需要 mainView 参与）
```

唯一冻结的派生公式：

```text
desiredGridVisibility
=
gridOpen && mainView === "grid"
```

| 状态 | 含义 | 是否存储 |
|---|---|---|
| `mainView` | 期望可见的主表面（`useLayoutStore`） | ✅ 真源 |
| `gridOpen` | Grid 原生资源是否存在（`useBrowserStore`） | ✅ 真源 |
| `desiredGridVisibility` | Grid 是否可见 | ❌ **纯派生，不存储** |
| `isBrowserVisible` | `mainView === "browser"` | ❌ 纯派生（不得重新耦合 `gridOpen`） |
| native 显隐 | `tab_position` / `grid_position` / `hide_all_webviews` 的执行结果 | ❌ 非域真源，禁止回读 |

**明确禁止**：

```text
gridVisible stored state
```

即不得新增 `gridVisible` 存储字段（无论 ref / computed / reactive 形式）。`gridVisible` 永远等于 `gridOpen && mainView === "grid"`，无独立自由度，存下来就是第二真源。由 `scripts/check-view-intent.mjs` 的 C4 / C5 常驻守护。

**同时冻结的否决设计**（见 `07-ALTERNATIVES.md`）：`exitGrid(mode)`、`GridLifecycle` enum、新增 Rust `show_grid`/`hide_grid`、拆分 `position`/`show`（保留 `position → show` 既有契约）。

## 1.2 Ownership（最终职责划分）

| 层 | Owner | 职责 |
|---|---|---|
| **View Navigation** | `useLayoutStore` | `mainView`、`activateView`、视图切换（`activateBrowser` / `activateHome` / `activateFiles` / `activateTerm` / `activateEditor` / `activateWorkspace`）。`setView` 是 `mainView` 的**唯一底层写入点** |
| **Browser/Grid Resource** | `useBrowserStore` | `gridOpen`、`openGrid`、`rebuildGrid`、`closeGrid`、`closeGridCell`、Browser/Grid 生命周期决策、状态不变量收敛 |
| **Execution** | `useBrowserHost` / `bridge` / Rust | native effect、bounds、visibility。只执行，**不得成为 domain state owner** |

要点：

- 组件**只表达意图**（调 Intent API），不得手拼 `buildGrid` / `closeGridAll` / `gridCloseOne`，不得直写 `mainView`。
- `closeGrid()` 只销毁资源；若当前 `mainView === "grid"`，视图收敛由 `useBrowserStore` 的不变量守卫**派生**（B9-4 类空白从构造上消除）。
- `openGrid()` = 确保资源存在；`rebuildGrid()` = 显式真重建（改格数/布局必须走后者，不得合并这两个语义）。

## 1.3 Lifecycle Contract（生命周期契约）

### View switch

```text
Grid → Browser
Grid → Home
Grid → Workspace(Files)
```

行为：

```text
HIDE ONLY
```

禁止：

- destroy
- recreate
- reload

即：切换视图只隐藏（native 移出屏幕），Grid 资源保持存活；URL、状态、页签输入全部保留；回到 Grid 时立即可见，不重建。

### Explicit close

```text
closeGrid()
```

行为：

```text
DESTROY RESOURCE
```

即：`gridOpen = false` + `bridge.closeGrid()`，销毁 Grid 原生资源。`closeGrid` **不承担**普通视图切换语义；App shutdown 与产品级 `closeGrid()` 意图分离（底层可复用同一 destroy 原语，但语义不得混为一体）。

### 状态不变量

```text
mainView === "grid" ⇒ gridOpen === true
```

非法长期状态 `mainView="grid" + gridOpen=false` 由 canonical owner 收敛。收敛规则：**仅在 `gridOpen` 由 `true → false`（资源消失）且仍在 grid 时收敛为 `browser`**。

> 关键：创建期间 `mainView=grid` 而 `gridOpen` 仍为 `false` 是**合法短暂中间态**（`createGrid` 是异步 IPC）。守卫若按"只要 false 就收敛"实现，会导致**首次打开宫格需要点两次** —— 该回归已修复并由 harness 固化，不得改回。

## 1.4 Runtime Evidence（运行时证据）

### Harness

```text
35/35 PASS
```

脚本：`scripts/runtime-phase1-browser-grid.mjs`（执行**真实** store 逻辑：`useBrowserStore` + `useLayoutStore`，bridge 打桩，非静态扫描、非重新实现）。

覆盖：

| 场景 | 断言要点 |
|---|---|
| Browser → Grid | `mainView=grid`、`gridOpen=true`、`desiredGridVisibility=true` |
| Grid → Browser | 资源存活（`gridOpen` 仍 true）、**无 `closeGrid` 调用**、`isBrowserVisible=true` |
| Grid → Home | 资源存活、Grid 不可见、**不重建** |
| Grid → Files | 资源存活、**不重建** |
| 20 switches | 无泄漏 / 无重建（`createGrid` 恒为 1 次）/ 无空白 |
| Explicit close | `gridOpen=false`、native `closeGrid` 已调用、视图收敛为 browser（B9-4 不回归） |
| State preservation | `gridSession` 不变、无 rebuild |

### GUI Acceptance

```text
R1 PASS
R2 PASS
R3 PASS
R4 PASS
R5 PASS
R6 PASS
R7 PASS
```

| 场景 | 结论 | 验收方式 |
|---|---|---|
| R1 打开宫格 | PASS | **用户真实桌面验收** |
| R2 宫格 → 浏览器 | PASS | **用户真实桌面验收** |
| R3 浏览器 → 宫格（状态保留） | PASS | **用户真实桌面验收** |
| R4 连续切换 ≥10 次 | PASS | **用户真实桌面验收** |
| R5 缩放窗口 | PASS | **用户真实桌面验收** |
| R6 最大化 / 还原 | PASS | **用户真实桌面验收** |
| R7 关闭无残留 | PASS | **进程 / 日志验收（Agent 执行，两次）** |

明确声明：

- **R1–R6 的 PASS 来自用户在真实桌面的人工验收**，不是 Agent 自动化证据。这些判据（无白屏、无偏移、布局正确、宫格不覆盖、页面不重新登录）本质是视觉判断；Agent 无屏幕可见性，且本机 `wmctrl`（物理像素）与 `xdotool`（逻辑像素）存在 **2 倍坐标差**，坐标点击不可靠。
- **R7 的 PASS 是进程 / 日志验收**：主进程无残留（精确 `ps` 匹配为空）、宫格子进程正常 `shutdown`、僵尸 0、panic/SIGSEGV 0、关闭日志 `[shutdown] completed ok=true`。已执行两次（Agent 自关闭一次、用户关闭后一次）。

### 被测对象与安装版本隔离

| 项 | 值 |
|---|---|
| 被测二进制 | `src-tauri/target/debug/mvp-browser-os` @ `218106d638e115789311f4ec031592d4237f595b8f5093680a95c22eb44d700c` |
| 被测 git HEAD | `88f8987`（= Phase 1 实现 `a30fd57` + harness `ad587bc` + 恢复层） |
| 运行方式 | `npm run tauri dev` 独立实例（前端由 `localhost:1421` 直供当前源码） |
| 已安装版本 | `/usr/bin/mvp-browser-os` @ `96ebcbd03cf7355113aa2fc500605c7ba9df77298eed1f26f6aa2b8fdbc2b6e4` — **全程未改** |
| 安装动作 | 无（未 build deb、未 install、未覆盖 `/usr/bin`、未改 desktop entry） |

> 注意：`src-tauri/target/release/mvp-browser-os` 打包的是**构建时的旧 dist**，不能用于验证新前端代码（本次它构建于 Phase 1 之前，已据此否决方案 A）。

## 1.5 门禁与复现

```bash
npm run build                                 # 编译
npm run check                                 # 全量静态门禁（含 check-view-intent）
node scripts/check-view-intent.mjs --self-test # C1–C11 双向自检
node scripts/check-grid-close-logic.mjs        # G1–G3 12/12
node scripts/runtime-phase1-browser-grid.mjs   # 运行时契约 35/35
```

| 门禁 | 结果 |
|---|---|
| `check-view-intent.mjs` C1–C11 | PASS（含 T11/T12 契约仿真） |
| `check-grid-close-logic.mjs` | 12/12 PASS |
| `npm run check` / `npm run build` | PASS |
| 全仓非法 `mainView` 直写 | **0** |

## 1.6 本阶段的边界

- 本 Phase **不修改**行为、不清理历史债务、不重构 Store、不进 Phase 2。
- 遗留问题统一登记于 `docs/architecture/semantic-governance/Known-Debt.md`，保持 Known Debt 状态。
