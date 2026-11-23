# Windows 本地构建修复说明

目标分支：`feature/browser-home-fullpage-multiplatform-20261123`。

这是**手动本地 release 打包入口**，不是 CI、自动测试或安装验收。改动未执行构建，也未执行任何校验。

使用 PowerShell 7，在项目根目录执行：

```powershell
pwsh -NoProfile -File scripts/build-windows-local.ps1
```

如需**显式复用**此前在可信本机已构建的、同一 `source.json` 来源契约的 LSP：

```powershell
pwsh -NoProfile -File scripts/build-windows-local.ps1 -ReuseBundleDirectory "D:\work\old-browser-os\src-tauri\resources\windows-lsp"
```

不允许使用未知来源的缓存。脚本会核对 manifest 字段、Rust commit、二进制 SHA256、Cargo.lock 和许可证；仍保持原始 pinned source、patch、hygiene 机制。生产默认使用 Terser；本机极慢时可明确指定 `-Minifier esbuild`，但不能把不同压缩器的产物宣称为生产等价。

本地构建会自动检测 MSVC/Windows SDK，设置 INCLUDE/LIB/LIBPATH 和绝对路径 MSVC linker，绕过 WindowsApps npm/WSL 别名，使用 PowerShell 7 与 Node CLI，并通过持久化 Cargo target 复用增量缓存。文件访问冲突按特征有限重试，不自动清理 target、不杀进程。使用 `-VsInstallDir`、`-WindowsSdkRoot`、`-CacheDirectory` 指定无法自动发现的路径。

LSP 脚本不再先克隆 Rust 1.100.0 tag 再跨 commit checkout，而是直接浅抓取经 source.json 固定的 source commit，保留 SHA256 校验、补丁变更集约束和 8 项 hygiene 测试。其 stdout 捕获与 stderr 分离，避免 native 输出错误判定；清理异常不再覆盖构建错误。构建入口先生成新前端 dist，临时 Tauri 配置仅省去二次 beforeBuildCommand；build.rs 原始 LSP 校验仍存在。

**操作边界**：本次只修改目标功能分支，不合并 master；提交信息 `[skip ci]`；不主动触发 GitHub Actions、不执行测试和其他校验。最终状态为 **代码已修改，构建、安装、原生 GUI 验收待运行**。

## 跨工作区缓存与一次性依赖恢复

- 首次执行时，若缺少本构建入口写入的锁文件指纹戳（由整个 `package-lock.json` 与 `package.json` 的 SHA256 构成），即通过 `node npm-cli.js ci --no-audit --no-fund` 恢复完整依赖树；随后执行已有直接依赖版本检查。以后只有锁文件/清单发生变化、或直接依赖检查失败时才再次安装。这是构建前置条件处理，不代表安全扫描或项目自动验证通过。
- 同一用户的 LSP 构建缓存位于 `%LOCALAPPDATA%/BrowserOS/BuildCache/windows-lsp-bundles/<source-json-sha256>/`。一次成功的本机产物先复制到独立临时目录，再发布为版本绑定的缓存目录。第二个工作区自动选用该缓存，原始 `Valid-Bundle` 仍重新验证五文件、固定 Rust commit、patch/lock 元信息和各项散列。
- **信任边界**：缓存 manifest 内的可执行文件 SHA256 本身并非独立的代码签名。仅在可信本地环境中复用先前自主构建的缓存；不得把不可信网络下载目录当作已验证供应链产物。如果复用目录损坏，原脚本拒绝继续，不自动掩盖失败或降级安全检查。
- 主程序增量编译目录固定为 `%LOCALAPPDATA%/BrowserOS/BuildCache/tauri-target`，不同工作区不会强制从零开始。Cargo 仍根据输入和目标配置管理构建指纹。每次构建日志使用唯一运行 ID，保存在缓存目录的 `logs` 子目录。
- 首次工具链编译依旧可能耗时很久。上面的优化目标是**避免第二次重复劳动**，不是保证冷编译必定在十几分钟内完成。

## 已审查的验证与 CI 边界（未执行）

- 开发者 Quick Gate、Full Validation、原生安全测试、NSIS 安装和 GUI 实测是不同证据层。跳过的步骤必须标记 `SKIPPED` 或 `NOT_RUN`，不能合并写成 `PASS`。
- Windows 原生 CI 中，`npm run build` 已在同一个 Windows Job 内执行，LSP 源码准备随后完成。只有用户显式开启 `build_installer` 时，NSIS 步骤才使用 `src-tauri/tauri.local-prebuilt.windows.conf.json` 复用这两份产物。**不会跳过 Tauri `build.rs` 内的 LSP 再校验**，也不继承其他 Runner 或其他 SHA 的产物。
- Windows 可选原生流程的总时间上限和 LSP 步骤上限已经调整为更符合冷构建的有限值。这仅避免超时误判，不减少安全测试数量。
- `pre-merge.sh` 与完整 `npm run check` 有重叠，但也负责独立的 release 产物完整性、源码治理、Rust 跨层和边界检查。未在没有验证证据的情况下删减这些门禁。
- 本轮只对代码和配置做了静态审查与修改；没有运行 `npm run check`、`cargo test`、`pre-merge.sh`、GitHub Actions、构建或安装。功能和性能改进需要后续实机证据才能确认。

## Historical blocker: WebView2 private API

The previously available Windows native job log for source commit `449b9949` reports `E0603` in `tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs` after successful pinned LSP/hygiene preparation. The Windows full-page screenshot call now constructs public `windows::core::PCWSTR` arguments from live UTF-16 arrays instead of importing the private `webview2_com::pwstr` module. The existing `tests/check-full-page-screenshot.mjs` records that narrow source invariant and is included in Quick Gate, but the actual Windows build remains **NOT_RUN** for this branch revision.
