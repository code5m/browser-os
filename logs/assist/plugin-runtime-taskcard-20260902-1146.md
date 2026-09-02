# plugin-runtime-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §7.3 **M5-10 ~ M5-12**（需求 #15 插件系统）
> 状态：⏸ **任务卡 / 未实现运行时 / 未执行验收 / 不宣称 PASS**
> 关联：`M5-15.a-prework-20260902-1055.md`（PluginManifest / 权限分级 / fail-closed）· `plugin-permission-taskcard-20260902-1146.md`（**形态②可行性判定是前置阻塞项**）

---

## 1. 目标

整理 M5 插件运行时卡：**安装包结构、manifest、权限模型、隔离边界、生命周期、审计、禁用/卸载、反向用例**。

🚨 **前置阻塞**：`plugin-permission-taskcard` §5 步骤 0 的可行性判定（Tauri v2 能否按 webview 粒度关闭 `withGlobalTauri`）未完成前，**形态②插件不得开工**。

---

## 2. 现状证据（2026-09-02 实测）

| 项 | 现状 |
|---|---|
| 插件运行时 | ❌ 无 |
| 插件命令 | ❌ 59 个白名单命令中无 `plugin_*` |
| `PluginManifest` 领域模型 | ❌ `domain.rs` 中无 |
| 打包嵌入 | ❌ `src-tauri/build.rs` 仅 40 B，无 `include_dir!` |
| 安全配置 | ⚠️ `withGlobalTauri = true`、`csp = null` → 形态②可行性存疑（勘误 E5） |
| 子 webview 能力 | ✅ `tauri-plugin-browser-tabs`（子 crate），已有 `WebviewBuilder` 使用经验 |
| 进程隔离能力 | ⚠️ 有 PTY（终端），但无通用子进程协议通道（依赖 `A2P-A2A-protocol-taskcard` 的 stdio 通道） |
| 退出收口 | ❌ 无 `ShutdownCoordinator` |
| 已有工具机制 | ⚠️ 需求 #2 的「HTML 小工具」是插件的近亲（`M2-tool-library-ui-static-shell`），但**文件都还不存在**（勘误 E1） |

---

## 3. 两种形态与选型

| 形态 | 描述 | 隔离强度 | 依赖 | 判定 |
|---|---|---|---|---|
| **① 独立进程 / stdio** | 插件是可执行文件，宿主通过 stdio + JSON Lines 通信 | **强**（进程边界 + 能力白名单） | 复用 A2P/A2A 的 stdio 通道 | ✅ **推荐** |
| **② webview 内运行** | 插件是 HTML，跑在子 webview 中 | **弱**（受 `withGlobalTauri` 影响） | 需按 webview 关闭全局 Tauri | ⛔ **待判定，判定不过则放弃** |

**结论**：**形态①为主线**。形态②仅在可行性判定通过后才开，且必须满足 `plugin-permission-taskcard` 的全部整改项。

---

## 4. 契约 / 数据结构

### 4.1 安装包结构

```
my-plugin-1.2.0.mvpx          （本质是一个 zip，扩展名 .mvpx 防误双击执行）
├── manifest.json             （必需，根目录）
├── icon.png                  （可选，≤256 KB，PNG）
├── bin/                      （形态①必需）
│   └── my-plugin             （可执行文件，需有 +x）
├── ui/                       （形态②可选）
│   └── index.html
├── assets/                   （可选，静态资源）
└── README.md                 （可选）
```

| 约束 | 值 |
|---|---|
| 格式 | zip（store 或 deflate） |
| 单包上限 | **10 MB** |
| 解压后上限 | **30 MB** |
| `manifest.json` 位置 | 必须在**根目录**（不在子目录） |
| 路径安全 | 解压时**拒绝**绝对路径、`..`、符号链接（**zip-slip 防护**） |
| 可执行文件 | 解压后保留 `+x`；宿主不得自动执行 |
| 校验 | 包 sha256 记入安装记录；可选签名（**本卡不实现签名**） |

### 4.2 `manifest.json`

```jsonc
{
  "id": "com.example.my-plugin",         // 反向域名，稳定标识
  "name": "我的插件",
  "version": "1.2.0",                    // semver
  "description": "一句话描述",
  "author": "作者名",
  "license": "MIT",
  "homepage": "https://...",             // 仅展示，不自动访问

  "form": "process",                     // "process"（形态①） | "webview"（形态②）
  "entry": {
    "process": "bin/my-plugin"           // 形态①：包内相对路径
    // "webview": "ui/index.html"        // 形态②
  },

  "protocolVersion": "1.0",              // 与 A2P/A2A 协议版本对齐
  "minHostVersion": "0.1.0",

  "capabilities": [                      // 声明的能力（⊆ 白名单）
    { "name": "fs:read",    "scope": ["$WORKSPACE/**"], "reason": "读取成果库生成报告" },
    { "name": "ui:notify",  "scope": [],                 "reason": "完成后通知用户" }
  ],

  "commands": [                          // 插件贡献的命令/菜单项
    { "id": "generate-report", "title": "生成周报", "icon": "icon.png" }
  ],

  "settings": [                          // 插件需要的配置项（宿主代为存储，加密项走 keyring）
    { "key": "apiBase", "label": "API 地址", "type": "string", "default": "https://api.example.com" }
  ],

  "resources": {                         // 资源声明（宿主用于限额与展示）
    "maxMemoryMb": 128,
    "maxCpuPercent": 25,
    "networkDomains": ["api.example.com"]
  }
}
```

### 4.3 权限模型（沿用 `M5-15.a` §5 与 `plugin-permission-taskcard` §4.3）

| 级别 | 名称 | 默认 | 说明 |
|---|---|---|---|
| 0 | `none` | 自动授予 | 纯离线、零宿主能力 |
| 1 | `read` | 安装时询问 | 只读宿主数据（需 scope） |
| 2 | `write` | 安装时询问 | 可写宿主数据（需 scope） |
| 3 | `exec` | **默认拒绝** | 可触发脚本/进程 |
| 4 | `net` | **默认拒绝** | 可出网（需域名白名单） |

**判定链（fail-closed）**：

```
1) manifest.capabilities ⊆ 已知白名单？          否 → 安装拒绝
2) 用户是否授予（granted）？                      否 → 运行期拒绝执行（不降级）
3) 本次请求参数是否在 scope 内？                  否 → 拒绝
4) 级别 ≥ 3（exec/net）？                         是 → 每次调用前二次确认
```

`scope` 占位符：`$WORKSPACE`（成果库）、`$NOTES`（笔记目录）、`$HOME`、`$APPDATA`、`$TEMP`。

### 4.4 隔离边界（形态①）

| 边界 | 措施 |
|---|---|
| **进程** | 独立子进程，`setsid` 建进程组（同 `script-execution-safety-taskcard` §4.7） |
| **文件系统** | 默认**无**文件能力；`fs:read`/`fs:write` 按 scope 前缀校验 + `canonicalize` |
| **网络** | 默认**无**；`net` 能力按 `networkDomains` 白名单 |
| **环境变量** | `env_clear()` + 最小白名单（`PATH`、`HOME`、`LANG`） |
| **工作目录** | 插件专属目录（`<app_data>/plugins/<id>/`），**不可指定** |
| **IPC** | 仅 stdio（JSON Lines，单行 ≤ 1 MB），复用 A2P/A2A 协议 |
| **资源** | `maxMemoryMb` / `maxCpuPercent` 声明式限额（**尽力而为**，见 §7） |
| **调用宿主** | 只经白名单方法（`fs/read`、`fs/write`、`ui/notify`、`skill/invoke`…），**无** `invoke 任意命令` 通道 |
| **凭据** | 宿主**不**向插件传密码；需要时走 keyring 引用，由宿主代为注入到出网请求（**本卡不实现**） |

**红线**：插件**永远**拿不到 `AppHandle`，也**永远**不能调 `#[tauri::command]`。所有宿主交互经协议层。

### 4.5 生命周期

```
install → (disabled) → enable → running → (disable) → uninstall
                            ↓
                        crash → 记录 → 可选自动重启（默认关）
```

| 阶段 | 行为 | 审计 |
|---|---|---|
| **install** | 校验包（zip-slip / 体积 / manifest schema / 能力 ⊆ 白名单）→ 解压到 `<app_data>/plugins/<id>/<version>/` → 写 `plugins.json`（**原子写**）→ 默认 **disabled** | ✅ |
| **enable** | 展示声明能力 → 用户授予 → `granted` 落盘 → 可启动 | ✅ |
| **start** | spawn 子进程（进程组）→ 发 `initialize` → 等 10 s → 超时即失败 | ✅ |
| **running** | 消息循环；超时/取消/背压同 A2P/A2A 契约 | ❌（明细走独立文件） |
| **disable** | `enabled=false`；**in-flight 不受影响**（跑完）；不再自动启动 | ✅ |
| **uninstall** | 若有 in-flight → 先取消；kill 进程组；删除目录与记录 | ✅ |
| **upgrade** | 解压新版本到新目录 → 若 `capabilities` **扩大** → 必须重新授权；成功后清理旧版本 | ✅ |
| **crash** | 记录退出码 + stderr 尾部（≤ 2 KB）→ 可选自动重启（**默认关**） | ✅ |
| **host shutdown** | `PluginShutdown`：按反向依赖序取消 in-flight → SIGTERM → 5 s → SIGKILL 进程组 | ✅ |

**红线**：安装后**不得自动启用**。

### 4.6 审计

| 事件 | 落哪里 | 内容 |
|---|---|---|
| 安装 / 启用 / 禁用 / 卸载 / 升级 | `audit.json` | pluginId、version、能力变化 |
| 能力被拒（fail-closed 触发） | `audit.json` | pluginId、capability、traceId |
| 崩溃 / 异常退出 | `audit.json` | pluginId、退出码、stderr 尾部（≤2 KB） |
| **每次方法调用** | ⚠️ **独立 `plugin-audit.json`**（**K5**） | traceId、method、耗时、结果 |
| 插件输出正文 | ❌ 不记 | 隐私 + 体积 |

---

## 5. 实现要点（步骤化）

1. **先完成阻塞依赖**：
   - `plugin-permission-taskcard` 步骤 0 可行性判定（决定形态②是否可做）
   - `M3-terminal-shutdown-taskcard`（ShutdownCoordinator）
   - `A2P-A2A-protocol-taskcard`（stdio 通道与能力层）
2. `domain.rs` 加 `PluginManifest` / `PluginInstall`。
3. 新 `src-tauri/src/plugin/mod.rs` + `installer.rs` + `runtime.rs`：
   - 解压（**zip-slip 防护** + 体积校验 + 保留 `+x`）
   - manifest 校验（schema + 能力 ⊆ 白名单）
   - 进程启动（进程组 + `env_clear` + 专属 cwd）
4. `bridge.rs` 加命令：`plugin_list` / `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_uninstall` / `plugin_invoke` / `plugin_logs` → `default-commands.toml`（**K1**）。
5. 持久化用**原子写** + 损坏备份（**K6**）。
6. 注册 `PluginShutdown`。
7. 审计按 §4.6 拆分落盘。
8. UI：插件列表 / 详情 / 权限授予（复用 `M2-tool-library-ui-static-shell` 的卡片与详情面板形态）。

---

## 6. 禁止事项

| # | 禁止 | 原因 |
|---|---|---|
| 1 | ❌ 在可行性判定通过前实现形态② | 权限模型形同虚设（勘误 E5） |
| 2 | ❌ 给插件 `AppHandle` 或任意 `invoke` 通道 | 直接绕过全部隔离 |
| 3 | ❌ 解压时接受绝对路径 / `..` / 符号链接 | zip-slip 任意文件写 |
| 4 | ❌ 安装后自动启用 | 未授权即运行 |
| 5 | ❌ 能力未授予时降级放行 | fail-closed |
| 6 | ❌ 只 kill 直接子进程 | 插件常派生子进程 |
| 7 | ❌ 每次调用写 `audit.json` | 刷爆 1000 条上限（K5） |
| 8 | ❌ 非原子写 `plugins.json` | 崩溃 = 全丢（K6） |
| 9 | ❌ 把宿主凭据传给插件 | K3 |
| 10 | ❌ 新增命令忘进 ACL | K1 |

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| **高** | 形态②可行性未判定 | 阻塞项；判定不过则形态②标记 BLOCKED，主线走形态① |
| **高** | 三个上游依赖未落地（Shutdown / stdio 协议 / 能力层） | 未完成时不得开工 |
| 高 | 恶意插件（任意代码） | 形态①进程隔离 + 能力白名单 + fail-closed + 默认不启用 + 网络默认关闭 |
| 中 | zip-slip / 解压炸弹 | 路径校验（§4.1）+ 体积上限 |
| 中 | 资源限额（`maxMemoryMb` / `maxCpuPercent`）**跨平台难精确执行** | 声明式为**尽力而为**；Linux 可用 cgroups（**本卡不实现**），明确记录为限制 |
| 中 | 插件崩溃后自动重启风暴 | 默认**关**自动重启；若开启需退避 + 次数上限 |
| 中 | 与「HTML 小工具」（#2）概念重叠 | 明确分工：工具=单文件 HTML、零宿主能力、离线；插件=可带进程、可声明能力。**不得**让工具获得插件能力 |
| 低 | 插件版本目录堆积 | 升级成功后清理旧版本（保留最近 1 个旧版便于回退） |

---

## 8. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 安装含 `../../etc/passwd` 条目的包 | 拒绝安装（zip-slip 防护） |
| R2 | 安装含符号链接条目的包 | 拒绝安装 |
| R3 | 安装 20 MB 的包 | 拒绝（>10 MB 上限） |
| R4 | manifest 缺 `id` / `version` | 拒绝 + 逐字段错误 |
| R5 | manifest 声明未知能力 `foo:bar` | 拒绝安装，列出未知能力 |
| R6 | 安装后未启用就调用 | 拒绝（未启用） |
| R7 | 只授予 `fs:read`，插件请求 `fs:write` | fail-closed 拒绝 + 审计 |
| R8 | `fs:read` scope 为 `$WORKSPACE/**`，请求 `$HOME/.ssh/id_rsa` | 拒绝（scope 不匹配） |
| R9 | 插件无 `net` 能力尝试出网 | 无网络能力（env/沙箱层面阻断） |
| R10 | 插件崩溃（exit 139） | 记录退出码 + stderr 尾部；**不自动重启**（默认） |
| R11 | 插件 30 s 不响应 `initialize` | 启动失败，进程组回收 |
| R12 | 卸载时有 in-flight 调用 | 先取消再删；无残留进程 |
| R13 | 宿主退出时有运行中的插件 | 走 `PluginShutdown`，进程组全部回收 |
| R14 | 升级时能力扩大 | 必须重新授权；未授权则保持旧版可用 |
| R15 | 高频调用 1000 次 | `audit.json` 不爆（明细在 `plugin-audit.json`） |
| R16 | `plugins.json` 半截损坏 | 备份为 `.corrupt`，不静默清空，应用可启动 |
| R17 | 插件声明 `form: webview` 但形态②未获批准 | 安装拒绝并给出明确原因（不静默失败） |

---

## 9. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 9.1 模块存在
ls src-tauri/src/plugin/                       # 期望 mod.rs / installer.rs / runtime.rs

# 9.2 zip-slip 防护
grep -n "\.\.\|canonicalize\|symlink\|Symlink" src-tauri/src/plugin/installer.rs   # 期望命中

# 9.3 体积上限
grep -n "10 \* 1024 \* 1024\|MAX_PACKAGE\|30 \* 1024" src-tauri/src/plugin/installer.rs  # 期望命中

# 9.4 进程组 + env_clear + cwd 锁定
grep -n "setsid\|killpg" src-tauri/src/plugin/runtime.rs        # 期望命中
grep -n "env_clear\|current_dir" src-tauri/src/plugin/runtime.rs  # 期望命中

# 9.5 拿不到 AppHandle / 无法 invoke 任意命令
grep -rn "AppHandle" src-tauri/src/plugin/runtime.rs | wc -l    # 期望 0（或仅内部宿主侧，不传给插件）

# 9.6 安装后不自动启用
grep -n "enabled" src-tauri/src/plugin/installer.rs | head      # 期望默认 false

# 9.7 原子写（K6）
grep -n "atomic_write\|rename\|corrupt" src-tauri/src/plugin/installer.rs  # 期望命中

# 9.8 审计不刷爆（K5）
grep -n "plugin-audit\|plugin_audit" src-tauri/src/plugin/*.rs  # 期望命中

# 9.9 命令已进 ACL（K1）
grep -c "plugin_list\|plugin_install\|plugin_enable\|plugin_disable\|plugin_uninstall\|plugin_invoke\|plugin_logs" \
  src-tauri/permissions/default-commands.toml                    # 期望 7

# 9.10 退出收口
grep -n "PluginShutdown" src-tauri/src/shutdown.rs               # 期望命中

# 9.11 编译门槛
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# 对照 baseline 13 warning
```

---

## 10. 失败动作

| 失败 | 动作 |
|---|---|
| 形态②可行性判定未通过却已实现 | 立即移除形态②代码路径；在汇总与本文档登记 BLOCKED |
| 发现 zip-slip 隐患 | 视为 P0 安全缺陷，立即修复 |
| 插件能调到任意 `#[tauri::command]` | 视为 P0 安全缺陷，立即切断通道 |
| 能力未授予却被放行 | fail-closed 修复 |
| 卸载/退出后进程残留 | 补进程组 kill；不得只 kill 直接子进程 |
| `plugins.json` 静默清空 | 补 `load_or_backup`（K6） |
| 资源限额无法落地 | 明确记录为「声明式、尽力而为」的限制，**不得**假装已强制 |
| clippy warning 增加 | 对照 baseline 回退 |

---

## 11. 推荐模型

- 形态①（进程 / stdio）：`AI:DEEP`
- 安全方案评审（隔离边界、zip-slip、权限）：`AI:DEEP-xhigh` + **人工安全评审**
- UI（列表/详情/授权）：`AI:BALANCED`
- **人工验收必做**：R1/R2（zip-slip）、R7/R8（权限）、R12/R13（进程回收）
