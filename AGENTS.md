# AGENTS.md

## 1. 开始前必读

任何任务开始前必须依次阅读：

1. `AGENTS.md`
2. `PROJECT-RULES.md`
3. 当前任务或阶段文件
4. 与任务直接相关的源码和测试
5. 若任务涉及 CI、门禁、Git Hook、发布、镜像、恢复、证据或工程规范，还必须阅读 `docs/engineering/README.md` 与 `docs/engineering/governance.json`

不得把 `[PENDING]` 决策作为已批准需求实施。

工程治理资产不得绕过 `npm run check:engineering-governance` 私自新增：新增/删除 GitHub Actions workflow 或版本化 Git Hook 时，必须在同一改动中更新 `docs/engineering/governance.json`。

## 2. 项目架构摘要

本规范仅适用于 mvp-browser-os-v3。Phantom 支付中台的 Java/Spring/Maven 规则不定义本项目身份；不得据此运行 Maven 或把客户端功能归入支付业务领域。

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
node scripts/check-workspace-startup.mjs
node scripts/check-image-preview-lazy-loading.mjs
bash scripts/pre-merge.sh
git diff --check
```

Rust 修改至少运行：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
bash scripts/pre-merge.sh
git diff --check
```

安装包修改按任务要求运行：

```bash
npm run tauri build -- --bundles deb
bash scripts/verify-installed-client.sh --no-start
```

涉及白屏、桌面图标、安装包、单实例旧进程、客户端文件系统能力时，必须优先运行安装版客户端验收脚本：

```bash
npm run verify:client
```

脚本会构建 deb、运行 Store 启动回归、解包校验、比对 `/usr/bin/mvp-browser-os`、检查 desktop 入口、发现旧进程，并在桌面会话里冷启动解包后的真实客户端。失败时按脚本的 `[FAIL]` 与 `->` 提示处理；不得跳过脚本后只凭浏览器预览、Vite 构建或 IPC ready 宣称修复。
用户要“一键打包安装”时使用：

```bash
npm run release:install
```

该命令会构建 deb、停止已核对的本项目旧实例、调用 `sudo apt install --reinstall` 强制覆盖同版本本地包，再继续做安装后校验和真实冷启动检查。sudo 密码由用户在终端输入；Agent 不得索要或记录密码。

### 9.1 打包铁律（任何 Agent 都必须遵守，禁止发布开发模式产物）

必须分别调查三种情况，不能互相替代：产物包含开发地址、desktop 指向错误入口、单实例把新启动转交已有进程。
安装成功只替换磁盘文件，不会替换运行中的进程。即使 Exec 正确，旧实例仍可能接管新启动。
先核对 PID、启动时间、`/proc/<PID>/exe`、二进制 SHA256 和主窗口实际 URL，再判断根因。

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
6. 冷启动验收前正常关闭本项目旧客户端，确认主进程退出；只停止已核对路径的本项目开发服务，不影响其他项目。

参考：`服务启动关闭说明.md`（「主配置不得有 devUrl」）、`PROJECT-RULES.md`（回归门禁）。

### 9.2 安装 / 升级安装包（一键命令）

打包产物在 `src-tauri/target/release/bundle/`（`deb` / `appimage` / `rpm`）。
本机 Debian/Ubuntu 用 `.deb`，**包名固定为 `mvp-browser-os`**，版本见 `tauri.conf.json`。

**① 普通升级安装（推荐，自动覆盖旧版，无需先卸载）：**

```bash
sudo apt install --reinstall -y ./src-tauri/target/release/bundle/deb/mvp-browser-os_0.1.0_amd64.deb
```

> 用 `apt --reinstall` 而非普通 `apt install`：开发期版本号常保持 `0.1.0`，普通 apt 可能判断“已是最新版”而不覆盖；`--reinstall` 会强制覆盖同版本本地 deb，并保留依赖处理能力（WebKitGTK 等运行库）。
> `dpkg -i` 不解析依赖，缺库时安装会停在「未配置」状态，需再 `sudo apt -f install`。

**② 仍出现旧行为时：**先检查旧实例、入口和实际加载地址。不得用删除用户数据代替诊断；确需隔离配置时先备份，并在测试后恢复。

说明：

- 版本号不同（旧版 → 新版）**不是先卸载的理由**；apt 按包名升级，版本不同正是升级目的。
- `_apt` 无权读取本地 deb 后回退 root 的提示不等于安装失败；以包状态和二进制校验为准。
- 安装后验收：即使没有 vite、没有 1421 进程，软件也能独立打开运行（见 §9.1 第 4 条）。
- 安装属于部署动作，需用户明确授权（见 §8）。

### 9.3 图标启动 / 安装版验收铁律

安装包生成后，禁止只验证 `target/release/mvp-browser-os` 或只看 `[FE] m0_ready ok` 就宣称安装版正常。
先运行统一脚本：

```bash
npm run verify:client
```

脚本通过后仍只能说明本机脚本可验证的安装链路正常；若用户报告图标启动异常，继续用脚本输出里的 PID、desktop、hash、冷启动日志逐项定位。
必须验证 **desktop 图标对应的已安装入口**：

```bash
which mvp-browser-os
dpkg -s mvp-browser-os
/usr/bin/mvp-browser-os
```

安装版验收必须同时检查：

1. `/usr/share/applications/mvp-browser-os.desktop` 的 `Exec` 指向本次安装入口，不得指向 debug、vite、`run-gui.sh` 或旧路径。
2. `/usr/bin/mvp-browser-os` 与刚生成 deb 内 `data/usr/bin/mvp-browser-os` 内容一致。
3. 图标/`/usr/bin/mvp-browser-os` 启动后真实窗口几何尺寸正常，禁止只看进程存在：
   ```bash
   wmctrl -lG | grep '浏览器OS融合'
   xdotool search --name '浏览器OS融合' getwindowgeometry
   ```
   出现 `10x10`、`1x1`、屏外坐标、不可见窗口，都视为失败。
4. 必须做真实截图或像素验收，确认不是白屏/黑屏：
   ```bash
   xdotool windowactivate <窗口id>
   scrot -u /tmp/mvp-installed.png
   ```
   只看到 `[FE] FE alive` / `[M0] m0_ready ok` 不代表 GUI 正常；这些只能证明 JS 跑过，不能证明窗口绘制成功。
5. 若启动日志没有 `[main] main window url=...`，说明主窗口创建逻辑未执行，优先排查 single-instance 旧进程或启动早退。
6. 若日志有 `[main] main window url=...` 且窗口尺寸正常但截图白/黑，优先排查主 WebView 绘制、启动遮罩、异步初始化卡住、子 WebView 覆盖。

这类问题属于安装版 GUI 回归，不得归类为普通前端构建通过。

Wayland 下 `scrot` 可能连其他正常窗口也返回全黑；必须先核实截图通道。截图被桌面权限拒绝或通道失效时标记 GUI_PENDING，不得据此修改 GTK 渲染代码。测试结束退出本次启动的客户端，避免它接管用户后续图标启动。

已复现的启动回归：`watch(..., { immediate: true })` 会同步求值 computed；依赖的 `ref` 必须先初始化。否则 Store 抛 ReferenceError，根组件无法生成，而 Vite 构建和 IPC 存活日志仍可成功。修改 Store 启动逻辑必须实际创建 Store，并检查根组件非空。

### 9.4 图片预览性能铁律

文件目录预览不得在进入目录时批量读取原图或批量生成大量 data URL。图片目录必须先展示元数据列表，缩略图只允许按可视区域懒加载，并限制并发和缓存上限。对比区只保留少量候选图，原图级读取必须由用户主动打开触发。

禁止模式：`Promise.all(entries.filter(isImage).map(readImageDataUrl))`、把几百张图片 data URL 长期放入 Pinia、一次渲染大量已解码原图。

允许模式：目录扫描只拿 `path/name/size`，可视区域触发 `loadPreviewImage`，并发不超过 4，缓存有上限，切换目录时丢弃旧队列；超过大小限制的图片显示不可预览提示，不影响目录继续浏览。

## 10. 完成状态

只能使用以下结论：

- `CODE_PASS`：代码和自动检查通过。
- `PACKAGE_PASS`：安装包成功生成并检查。
- `GUI_PENDING`：真实桌面尚未验收。
- `GUI_PASS`：用户明确完成真实桌面验收。
- `BLOCKED`：存在阻断问题。

不得把编译成功写成 GUI PASS。
不得把源码修复写成安装版已经生效。
这份文件不需要详细解释每个历史 Bug，只记录模型每次都必须遵守的操作规则。
