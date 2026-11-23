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
