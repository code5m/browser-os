# BrowserOS · 浏览器与本地工作台

> 信息源(浏览器) + 操作台(控制台) → 本地成果(带溯源) → 受控同步到自有 git / 码云(gitee)
> 凭据存系统密钥库 · 推送需显式确认 · 全程审计

👉 **第一次用？先看 [`使用指南.md`](./使用指南.md)**：大白话项目说明 + 手把手操作步骤 + 最佳实践，小白零基础也能跑起来。

## 架构（对应 `23-融合讨论V2-全场景闭环.md`）

```
网页(信息源) ──注入脚本──▶ collect_selection
                              │  (渲染进程零特权, 仅能 invoke 白名单命令)
                              ▼
                    ┌─────────────────────┐
                    │   Rust 桥 (App)     │  持有 OS 能力 + 系统密钥库
                    │  BridgeCall 权限内核 │
                    └─────────────────────┘
                       │            │
          save         │            │  SyncJob(待确认)
         本地成果库     ▼            ▼
   workspace/<id>.json   request_sync → confirm_sync(闸门) → git2 push
                                              │
                                      凭据仅从 keyring 读取
                                              ▼
                                  你的 git / 码云 仓库
```

## 安全红线（强制）
1. 网页 JS 永不可见仓库 token；token 只存系统密钥库（`keyring`），桥进程读取。
2. 推送前必须 `request_sync` 生成待确认 `SyncJob`，用户在前端点确认后才 `confirm_sync` 真正推送。
3. 所有出网/落盘动作写入审计日志（`audit.json`）。
4. 注入脚本仅在 `browser` 子窗口启用 IPC，主控制台窗口默认禁用远程 IPC。

## 目录
```
mvp-browser-os/
├── package.json / vite.config.ts / index.html      # 前端(Vue3)
├── src/                         # 控制台 UI + 类型化 IPC 封装
└── src-tauri/
    ├── Cargo.toml / build.rs / tauri.conf.json / capabilities/
    ├── injected/collect.js      # 注入到浏览窗口的右键“保存到成果库”
    └── src/
        ├── main.rs              # 启动 + 注入 + 命令注册
        ├── domain.rs            # Artifact / RepoConfig / SyncJob / AuditEntry
        ├── bridge.rs            # BridgeCall 命令 + 权限内核 + AppState
        ├── workspace.rs         # 本地成果/仓库配置/审计 持久化
        ├── keyring_store.rs     # 凭据隔离（系统密钥库）
        └── sync.rs              # SyncJob: clone/pull/commit/push (git + gitee)
```

## 运行
```bash
# 1. 安装 Rust + Tauri 前置（见 https://tauri.app/start/prerequisites/）
#    Ubuntu: sudo apt install libwebkit2gtk-4.1-dev build-essential \
#            curl wget file libssl-dev libayatana-appindicator3-dev \
#            librsvg2-dev cmake pkg-config
# 2. 安装前端依赖
npm install
# 3. 开发模式（自动拉起 Tauri 窗口）
npm run tauri dev
# 4. 打包
npm run tauri build
```

## 依赖说明
- `git2`：默认链接系统 libgit2/openssl；若需静态，改用
  `git2 = { version = "0.19", default-features = false, features = ["vendored-libgit2"] }`（需 cmake）。
- `keyring`：macOS→Keychain，Windows→Credential Manager，Linux→secret-service（需 dbus 桌面环境）。

## 已知简化（骨架，非生产）
- ~~`confirm_sync` 为同步阻塞调用~~ → 已改为 `std::thread::spawn` 后台线程执行，推送期间 UI 不卡顿；成功/失败通过 `sync-completed` 事件（`app.emit`）回调前端，前端弹窗显示“推送中…”。
- ~~仓库冲突采用“强制 checkout HEAD”~~ → 已实现**真正三方合并**（`sync.rs`）：快进(fast-forward)直接前移；分叉时做索引级合并并生成 merge commit；**检测到冲突则中止合并并回退**，返回冲突文件清单交用户人工处理，绝不丢弃任何一方数据。
- ~~注入脚本未做富文本保真~~ → 已实现**富文本选区保真**（`injected/collect.js`）：克隆选区 DOM、绝对化 `img/a/source` 链接、内联关键排版样式、剔除 `script/style/iframe`；同步时除 `.md` 外额外落盘保真 `.html`（`render_html`，带溯源头）。
- 多端沿用同一 `BridgeCall` 契约：`android-bridge/` 提供 **Android 原生桥示意**（Kotlin），命令名/数据结构与桌面一致，凭据用 Android Keystore + EncryptedSharedPreferences，推送用 JGit（含三方合并冲突保护），确认闸门与审计同构。iOS/鸿蒙/Flutter 可照此适配。
- **图标**：已生成正式 512×512 圆角图标（`src-tauri/icons/icon.png`，蓝绿渐变 + 白色聚焦环），可直接 `tauri build`。如要更精致，替换该 PNG 即可。
- **gitee 配置示例**：前端仓库表单有「填入码云示例」按钮，一键填好 provider=gitee、分支 master、远程地址模板，用户只需补全用户名与 Token 即可保存。
- **远程页面 IPC 放行方式**：Tauri v2 按**窗口能力**（`capabilities/default.json`）而非来源域名放行；`browser` 窗口已在其中，故网页选区脚本可调用 `collect_selection`，无需 `dangerousRemoteDomainIpcAccess`（该字段在 2.11.x 不存在）。若某版本需更严格限制，再按需收紧能力。
- 本环境为无头（headless），无法启动 GUI；请在本地桌面执行 `npm run tauri dev` 实际体验。

## 下一步
按 `23` 文档的 MVP 切入口，先打通：右键存本地 → 富文本保真 → 系统通知 → 一键推 git/gitee。
