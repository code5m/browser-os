# AGENTS.md

## 1. 开始前必读

任何任务开始前必须依次阅读：

1. `AGENTS.md`
2. `PROJECT-RULES.md`
3. 当前任务或阶段文件
4. 与任务直接相关的源码和测试

不得把 `[PENDING]` 决策作为已批准需求实施。

## 2. 项目架构摘要

本项目是：

- Vue + TypeScript 前端外壳
- Tauri + Rust 原生后端
- Linux GTK/WebKitGTK 原生子 WebView
- 自研 `tauri-plugin-browser-tabs`
- Debian `.deb` 发布

原生浏览器 WebView 不是普通 DOM 元素。
HTML `z-index` 不能保证覆盖原生子 WebView。

## 3. 普通模型安全区

通常允许修改：

- `src/components/**`
- `src/domain/**`
- `src/application/**`
- `src/stores/**`
- `src/styles/**`
- `tests/**`
- `scripts/check-*.mjs`
- 文档

## 4. 原生危险区

未经任务明确授权不得修改：

- `src-tauri/src/bridge.rs`
- `src-tauri/src/**/linux.rs`
- `src-tauri/capabilities/**`
- `tauri-browser-tabs/**`

这些目录涉及：

- GTK/WebKitGTK
- 原生窗口层级
- 逻辑/物理像素
- WebView 显隐和销毁
- Wayland/X11
- Tauri 权限

## 5. UI 硬性规则

- 禁止在浏览器区域上新增 HTML Modal、Toast、Popover。
- 禁止使用增大 `z-index` 解决原生 WebView 遮挡。
- 成功操作不得弹浮窗。
- 错误只进入状态栏或独立布局面板。
- 独立面板必须改变布局并触发 WebView 矩形同步。
- 保留自定义无边框窗口。
- 普通 UI 修改不得改变窗口拖动事件路径。

## 6. 原生调用边界

目标架构中：

- Vue 组件不得直接调用 Tauri `invoke`。
- Store 不得直接操作 GTK/WebView。
- 原生调用应通过统一 BrowserRuntime adapter。
- 在该迁移完成前，不得假装该边界已经完全存在。

## 7. 修改原则

- 先进行只读调查，再修改。
- 一次只处理一个明确任务。
- 不顺便大规模重构。
- 不修改任务范围外的文件。
- 不自行决定待定产品需求。
- 不删除无法证明无调用方的状态或接口。
- 优先增加回归测试，再修改实现。

## 8. 停止条件

出现以下情况必须停止并报告：

- 需要修改 GTK/WebKitGTK 原生实现。
- 需要改变 Tauri Command 参数契约。
- 需要改变 WebView 创建、隐藏、销毁时序。
- 需要决定一个 `[PENDING]` 产品需求。
- 测试失败且根因超出当前任务。
- 工作树出现不属于本任务的修改。
- 需要 commit、push、安装或部署但未被明确授权。

## 9. 最低验证

普通前端修改至少运行：

```bash
npm run build
bash scripts/pre-merge.sh
git diff --check

Rust 修改至少运行：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
bash scripts/pre-merge.sh
git diff --check
```

安装包修改按任务要求运行：

```bash
npm run tauri build -- --bundles deb
```

### 9.1 打包铁律（任何 Agent 都必须遵守，禁止发布开发模式产物）

曾经踩坑：发布包是「开发情况」，安装后图标/启动项仍指向**旧的开发进程**（vite dev 1421 或旧的 debug 二进制），
导致装完打开的还是旧版本、且依赖开发服务器才跑得起来。根因 = 用 debug 构建或残留 `devUrl` 当发布包。

发布安装包必须满足：

1. **只用 release 构建**：`npm run tauri build`（默认 `--release`）。
   禁止把以下产物当发布包：
   - `tauri dev` / `vite dev` 起的开发服务；
   - `run-gui.sh`（默认会拉起 vite dev 连 1421）；
   - `run-gui.sh --force-dist`（仍是 **debug 二进制**，仅跳过 dev server，不是 release 发布物）。
2. **`tauri.conf.json` 的 `build` 段不得写 `devUrl`**（回归门禁）。release 二进制 `debug_assertions=false`，
   `main.rs` 里 `cfg!(debug_assertions)` → `External("http://localhost:1421")` 分支**不编译生效**，
   正确走 `WebviewUrl::App("index.html")` 加载内嵌 `dist`（`frontendDist`）。
3. **打包前前端必须最新**：`beforeBuildCommand` 已是 `npm run build`，会刷新 `dist`；
   不要对 `npm run tauri build` 加 `--no-build` 之类跳过（否则嵌入旧 dist，同 `run-gui.sh --no-build` 白屏）。
4. **安装/启动验收必须确认不依赖开发服务器**：装包后启动软件，
   **即使没有 vite、没有 1421 进程也能正常打开并运行**。否则就是开发模式产物混进了发布包，必须重打。
5. **desktop / .deb 内的 Exec 必须指向本次 release 二进制**，不得残留指向旧开发进程或旧 debug 二进制。
6. 打包前先停掉本地 vite dev（`bash mvp-stop.sh`），避免与「是否在用开发地址」的验收判断互相干扰。

参考：`服务启动关闭说明.md`（「主配置不得有 devUrl」）、`PROJECT-RULES.md`（回归门禁）。

## 10. 完成状态

只能使用以下结论：

- `CODE_PASS`：代码和自动检查通过。
- `PACKAGE_PASS`：安装包成功生成并检查。
- `GUI_PENDING`：真实桌面尚未验收。
- `GUI_PASS`：用户明确完成真实桌面验收。
- `BLOCKED`：存在阻断问题。

不得把编译成功写成 GUI PASS。
不得把源码修复写成安装版已经生效。
```

这份文件不需要详细解释每个历史 Bug，只记录模型每次都必须遵守的操作规则。