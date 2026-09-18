# 07 — Alternatives (Rejected Designs)

本节记录被明确否决的设计，及其否决理由。对应 §2 禁止预设。

## 1. ❌ `gridVisible` 存储状态（Model A）

- 主张：`mainView`+`gridOpen`+`gridVisible`+`nativeVisible` 四真源，显式建模 Grid 可见。
- 否决：
  - `gridVisible` 永远 = `gridOpen && mainView === "grid"`（Final Reconciliation 冻结公式），无独立自由度。
  - 存为状态必引入第二真源，任何 `gridOpen`/`mainView` 变更需同步维护，漂移风险正是 Phase 0 反复踩的坑。
  - 违反"减少真源"。
- 采用 Model B（派生，不存）。见 `01-STATE-MODEL.md §5`。

## 2. ❌ `exitGrid(mode)` 多义万能 API

- 主张：`exitGrid("hide" | "destroy")` 一个动词承担隐藏/销毁/切视图。
- 否决：
  - 多义 API：`mode` 参数使小模型易误用（传错 = 资源泄漏或误销毁）。
  - Checker 难静态分辨每次调用是 hide 还是 destroy。
  - 违反"一个用户意图一个 canonical entrypoint"。
- 采用 `activate*` + `closeGrid()` 单义 API。见 `02-INTENT-MODEL.md §3`。

## 3. ❌ 新 Rust `show_*` / `hide_*` 原生命令（拆分 position/show）

- 主张：把 `grid_position`/`tab_position` 拆成 `position` + `show`/`hide` 两个命令。
- 否决（按 04 §4 维度）：
  - 风险高：需新 Rust 命令 + 全量迁移 + 改造定位去重缓存；B9-4 修复依赖现有 `position→show`。
  - 兼容性：破坏 `create_grid`/`close_grid`/定位去重既有稳定逻辑。
  - 调用数量：+1 命令/窗，定位与 show 需配对，易漏。
  - 测试难度：需重写定位+show 双路径测试。
  - **不因为函数名不好看就重构**（违反"NO LARGE REFACTOR / 不扩大重构范围"）。
- 采用：保留 `position → show` 契约；新增**前端 Visibility Controller 派生层**（不新增 Rust 命令）。

## 4. ❌ 新 `GridLifecycle` enum（ABSENT/CREATED/VISIBLE/HIDDEN/DESTROYED）

- 主张：用 enum 显式建模 Grid 生命周期状态机。
- 否决：
  - `VISIBLE`/`HIDDEN` 是 `desiredGridVisibility` 的派生，非独立状态。
  - `ABSENT` == `DESTROYED`（gridOpen=false 即无窗），无区分必要。
  - 已有 `gridOpen: boolean` + 原生窗存在性足以表达全状态，新增 enum 是把派生量提升为真源（重复状态）。
  - 违反 §2 禁止预设"新 GridLifecycle enum"。
- 采用：以 `gridOpen` + `mainView` 表达，状态名仅作文档化映射（见 `03-LIFECYCLE.md §1`）。

## 5. ❌ 让 `useLayoutStore` 当 Browser/Grid lifecycle owner

- 主张：通用导航 store 兼管 Grid 资源。
- 否决：
  - `useLayoutStore` 不持有 `gridOpen`/`schedulePosition`，会反向依赖 `useBrowserStore` 形成循环。
  - 把资源 build/destroy 耦合进通用导航 store，违反单一职责。
- 采用：`useBrowserStore` 为 owner，`useLayoutStore` 退为 `mainView` 状态执行点。见 `05-OWNERSHIP.md §3`。

## 6. ❌ 把 native visibility 当作域真源

- 主张：从 `webview.bounds`/原生状态回读"当前是否可见"作为真相。
- 否决：native 显隐是 `mainView`+`gridOpen` 的执行结果；回读会造成"执行结果当真源"的反向依赖，且离屏坐标 `-30000` 等是约定非事实。
- 采用：Visibility Controller 单向派生。见 `04-NATIVE-VISIBILITY.md §2`。
