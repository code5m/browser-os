# errata-to-taskcards（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 性质：把 **2026-09-02 实测发现的 6 项勘误**转为强模型可直接执行的任务卡
> 状态：⏸ **全部未开工 / 未执行实现 / 未签任何 PASS**

每张任务卡统一包含：**目标 / 证据 / 必改文件候选 / 禁止事项 / 实现要点 / 反向用例 / 验收命令 / 失败动作 / 推荐模型**。

共性红线（上一批次 `free-model-prework-M2-M5-20260902-1055.md` §5 的 K1~K9）对所有卡生效，尤其：

- **K1**：新增 `#[tauri::command]` 必须同步进 `src-tauri/permissions/default-commands.toml`（当前 59 个命令）。
- **K2**：出网/落盘/执行动作写 `audit.json`（1000 条上限）。
- **K7**：路径必须 `canonicalize()` + 前缀校验。

---

## E1 · 5 个种子工具 HTML 不存在

| 字段 | 内容 |
|---|---|
| **ID** | E1 |
| **锚定** | WBS §4.3 **M2-6**（需求 #2 小工具框架） |
| **严重度** | 🔴 最高（排期级） |

### 目标

把 M2-6 的性质从「补加载框架」纠正为「**先编写 5 个单文件 HTML，再写加载框架**」，并给出可机械核验的落盘标准。

### 证据（2026-09-02 实测）

```bash
$ ls -la src-tauri/src/tools
ls: 无法访问 'src-tauri/src/tools': 没有那个文件或目录

$ find . -name "*.html" -not -path "./node_modules/*" -not -path "./target/*" \
    -not -path "./dist/*" -not -path "./.git/*" -not -path "./variants/*" \
    -not -path "./tauri-browser-tabs/*"
./index.html
./prototype-index.html
./prototype.html
# （+ src-tauri/target 下的构建产物，非业务源码）
```

- `*tool*.html` **零命中**。
- 59 个已注册命令中**无 `list_tools`**。
- 前端 22 个 `.vue` 中**无工具箱面板**。

冲突出处：`详细设计与实施计划.md:144/178`、`后续需求TODO.md:37-51` 均称「已落盘」。

### 必改文件候选

| 文件 | 性质 |
|---|---|
| `src-tauri/src/tools/cron-tool.html` | 新增 |
| `src-tauri/src/tools/regex-tool.html` | 新增 |
| `src-tauri/src/tools/json-tool.html` | 新增 |
| `src-tauri/src/tools/base64-tool.html` | 新增 |
| `src-tauri/src/tools/timestamp-tool.html` | 新增 |
| `src-tauri/build.rs` | 加 `include_dir!` |
| `src-tauri/src/bridge.rs` | 加 `list_tools` |
| `src-tauri/permissions/default-commands.toml` | 加 `list_tools`（K1） |
| `src/components/system/ToolLibraryPanel.vue` | 新增前端面板 |

### 禁止事项

- ❌ 不得用「文件应该存在」的假设推进，必须先 `ls` 复核。
- ❌ 不得引入 CDN / 外部字体 / 外部 JS（离线红线）。
- ❌ 不得在 HTML 里直接 `window.__TAURI__.invoke()` 调任意命令（见 E5）。
- ❌ 不得为了「看起来齐了」创建空壳 HTML。

### 实现要点

详见 `M2-tools-seed-html-prework-20260902-1146.md`（TASK-4 专卡）：
单文件自包含（内联 CSS/JS）、体积 ≤100 KB/个、仅 `localStorage`（键前缀 `mvp-tool:<tool-id>:`）、必须含 `<meta name="tool-name">` / `tool-description` / `tool-category`。

### 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 断网后打开 5 个工具 | 全部功能可用（离线） |
| R2 | 任一工具输入非法值 | 中文可读提示，不白屏、不 `alert`、不抛未捕获异常 |
| R3 | `grep -rn "https\?://" src-tauri/src/tools/*.html` | 零命中（排除注释中的说明性 URL） |
| R4 | 删除 `localStorage` 后重开 | 恢复默认态，不崩溃 |

### 验收命令

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
ls -1 src-tauri/src/tools/*.html | wc -l                       # 期望 5
grep -rn "https\?://" src-tauri/src/tools/*.html | grep -v "<!--" | wc -l   # 期望 0
for f in src-tauri/src/tools/*.html; do
  wc -c "$f" | awk -v F="$f" '$1>102400{print "TOO-BIG: "F" "$1}'
done                                                            # 期望无输出
grep -n "list_tools" src-tauri/permissions/default-commands.toml # 期望命中
grep -rn "tool-name" src-tauri/src/tools/*.html | wc -l          # 期望 5
```

### 失败动作

| 失败 | 动作 |
|---|---|
| 工具数量 < 5 | 停止，报告缺哪几个，不得用占位文件凑数 |
| 体积超限 | 拆分内联资源/压缩，不得放宽阈值 |
| `list_tools` 未在 ACL | 按 K1 补，并复跑全部命令调用 |
| 离线校验不通过 | 定位外链来源，改为内联 |

### 推荐模型

`AI:BALANCED`（5 个纯前端单文件 HTML，逻辑清晰但量大）；`cron` 的「未来执行时间推算」边界用例较多 → 该工具可单独 `AI:DEEP`。

---

## E2 · `term_resize` 是空实现

| 字段 | 内容 |
|---|---|
| **ID** | E2 |
| **锚定** | WBS §5 **M3-2**（需求 #9） |
| **严重度** | 🔴 高 |

### 目标

让 PTY 真实跟随前端尺寸变化，`vim` / `top` / `htop` 等全屏程序显示正确。

### 证据

```rust
// src-tauri/src/bridge.rs:2011-2016
/// 调整终端大小（列/行）。
#[tauri::command]
pub fn term_resize(app: AppHandle, id: String, cols: u16, rows: u16) -> Result<(), String> {
    let _ = (app, id, cols, rows);
    Ok(())
}
```

```rust
// src-tauri/src/bridge.rs:114-117
pub struct TerminalSession {
    pub writer: Box<dyn std::io::Write + Send>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}
```

- `TerminalSession` **没有 `master` 字段** → 即使想 resize 也无从下手。
- `term_spawn`（`bridge.rs:1926-1995`）在 `pair.master.take_writer()` 后**丢弃了 master**（未保存到 session）。

### 必改文件候选

| 文件 | 改动 |
|---|---|
| `src-tauri/src/bridge.rs` | `TerminalSession` 增 `master: Box<dyn portable_pty::MasterPty + Send>`；`term_spawn` 保存 master；`term_resize` 调 `master.resize(PtySize{..})` |
| `src/bridge.ts` | `termResize` 增加节流与错误吞掉的说明（见 E3） |
| `src/components/system/TerminalPane.vue` | `ResizeObserver` 回调里调 `system.resizeShell(cols, rows)` |

### 禁止事项

- ❌ 不得为了拿 master 而改 `term_spawn` 的读取线程所有权结构（会引入死锁）。
- ❌ 不得在 resize 失败时返回 `Err` 让前端弹窗（高频调用，应静默）。
- ❌ 不得在持有 `terminals` 锁时做阻塞 IO。

### 实现要点

完整方案见 `M3-terminal-resize-taskcard-20260902-1146.md`（TASK-8）。核心：

```rust
pub struct TerminalSession {
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub writer: Box<dyn std::io::Write + Send>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}
```

`portable-pty 0.8` 的 `MasterPty::resize(&self, size: PtySize)` 只需 `&self`，可在 `&mut` 会话或只读引用下调；`take_writer()` 之后 master 仍可保留（注意：`take_writer` 借用 `&mut self`，须在插入 session 前完成）。

### 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 开终端 → `vim` → 拖动窗口改变宽度 → `:set nu` | 行号列不串行、底部状态栏不残影 |
| R2 | `top` → 拉高窗口 | 进程列表行数随窗口变化 |
| R3 | 极端窄（< 20 列） | 不崩溃；PTY resize 失败静默，xterm 仍可用 |
| R4 | 快速连续拖动 50 次 | 不卡死；后端无 panic；CPU 无尖峰 |

### 验收命令

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 1) 空实现消失
grep -n "let _ = (app, id, cols, rows)" src-tauri/src/bridge.rs   # 期望零命中
# 2) master 已持有
grep -n "master" src-tauri/src/bridge.rs | sed -n '1,20p'
# 3) 真实调用 resize
grep -n "\.resize(" src-tauri/src/bridge.rs
# 4) 前端调用方存在
grep -rn "termResize\|resizeShell" src/ | grep -v "bridge.ts:"      # 期望 >=1 命中
# 5) 编译与静态门槛
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20   # 对照 logs/baseline-2026-08-27.md
```

> ⚠️ 以上为**静态检查**；`vim`/`top` 的真实表现**必须人工 GUI 验收**（见 `model-routing-matrix`）。

### 失败动作

| 失败 | 动作 |
|---|---|
| resize 后 `vim` 仍错位 | 检查 cols/rows 是否取自 `fitAddon.proposeDimensions()`，而非容器像素 |
| 加 `master` 后编译报 trait 不满足 `Send` | 用 `Box<dyn MasterPty + Send>` 并确保不跨线程共享同一 `&mut` |
| clippy 新增 warning | 对照 baseline，新增即回退 |

### 推荐模型

`AI:DEEP`（涉及 `portable-pty` 所有权 + 前端 fit 链路 + 人工验收三处协同）。

---

## E3 · 前端 `termResize` 零调用

| 字段 | 内容 |
|---|---|
| **ID** | E3 |
| **锚定** | WBS §5 **M3-2**（与 E2 同 PR） |
| **严重度** | 🔴 高 |

### 目标

把 `bridge.termResize` 接到 xterm 的 `ResizeObserver` / `fitAddon` 链路上，并补齐 store 层封装。

### 证据

```bash
$ grep -rn "term_resize\|termResize" src src-tauri/src
src/bridge.ts:208:  termResize: (id: string, cols: number, rows: number) =>
src/bridge.ts:209:    invoke("term_resize", { id, cols, rows }),
src-tauri/src/main.rs:671:            bridge::term_resize,
src-tauri/src/bridge.rs:2013:pub fn term_resize(app: AppHandle, id: String, cols: u16, rows: u16) -> Result<(), String> {
```

→ 只有「定义 + 注册 + 实现」，**零调用方**。

```ts
// src/components/system/TerminalPane.vue:55-58
resizeObserver = new ResizeObserver(() => {
  fit?.fit();          // 只改 xterm 前端，不通知后端 PTY
});
```

```vue
<!-- src/components/system/TerminalPane.vue:66-70 -->
onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  term?.dispose();
  system.bindTermWriter(null);   // 不调 term_kill（见 E4）
});
```

### 必改文件候选

| 文件 | 改动 |
|---|---|
| `src/components/system/TerminalPane.vue` | `ResizeObserver` 回调内：`fit.fit()` → 取 `fit.proposeDimensions()` → 调 `system.resizeShell(cols, rows)` |
| `src/stores/useSystemStore.ts` | 新增 `resizeShell(cols, rows)`；节流（≥80 ms 合并）；`termId` 为空时 no-op |

### 禁止事项

- ❌ 不得每个 resize 事件都 `invoke`（拖动会触发数十次/秒）→ 必须节流。
- ❌ 不得在无 `termId` 时调用（会打到后端报「终端不存在」）。
- ❌ 不得把 resize 失败抛给用户（静默吞 + `debugLog`）。

### 实现要点

```ts
// useSystemStore.ts 伪码
let resizeTimer: number | null = null;
let lastDims = { cols: 0, rows: 0 };

function resizeShell(cols: number, rows: number) {
  if (!termId.value) return;
  if (cols === lastDims.cols && rows === lastDims.rows) return;
  lastDims = { cols, rows };
  if (resizeTimer) clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    bridge.termResize(termId.value!, cols, rows).catch(() => {}); // 静默
  }, 80);
}
```

```ts
// TerminalPane.vue 伪码
resizeObserver = new ResizeObserver(() => {
  const dims = fit?.proposeDimensions();
  if (dims && dims.cols > 0 && dims.rows > 0) {
    if (term.cols !== dims.cols || term.rows !== dims.rows) term.resize(dims.cols, dims.rows);
    system.resizeShell(dims.cols, dims.rows);
  } else {
    fit?.fit();
  }
});
```

### 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 终端未启动（`termId` 为空）时触发 resize | 零 invoke，后端无报错 |
| R2 | 1 秒内拖动 30 次 | 实际 invoke ≤ 13 次（80 ms 节流） |
| R3 | resize 期间后端返回 Err | 前端静默，`/tmp/mvp-debug.log` 有记录，UI 无弹窗 |
| R4 | 尺寸未变（`cols`/`rows` 相同） | 零 invoke（去重） |

### 验收命令

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
grep -rn "termResize" src/            # 期望 >= 2 处（bridge.ts 定义 + store 调用）
grep -n "resizeShell" src/stores/useSystemStore.ts src/components/system/TerminalPane.vue
npm run build                          # 期望构建通过，主 JS 体积对照 baseline（505 KB 硬门槛）
```

### 失败动作

| 失败 | 动作 |
|---|---|
| 主 JS 体积增长超过 baseline | 检查是否误引入依赖；不得放宽阈值 |
| resize 后终端仍错位 | 先确认后端 E2 已合入，**E2/E3 必须同 PR** |
| 拖动卡顿 | 加大节流窗口至 120 ms，或改用 `requestAnimationFrame` 合并 |

### 推荐模型

`AI:BALANCED`（纯前端，逻辑简单；但必须与 E2 同 PR，故整体建议按 `AI:DEEP` 排）。

---

## E4 · 主窗关闭不杀 PTY

| 字段 | 内容 |
|---|---|
| **ID** | E4 |
| **锚定** | WBS §5 **M3-1** + **M0-2**（需求 #3「关闭窗口资源释放」） |
| **严重度** | 🔴 高（**阻塞 5 个下游任务**） |

### 目标

建立统一的 `ShutdownCoordinator`：窗口关闭 / 应用退出 / tab 关闭三条路径都能杀掉 PTY 及其子进程树。

### 证据

```rust
// src-tauri/src/main.rs:591-593
WindowEvent::CloseRequested { .. } => {
    state.grid_manager.shutdown_all();     // 只收宫格子进程，不碰 terminals
}
```

```rust
// src-tauri/src/main.rs:674-678
.run(tauri::generate_context!())
    .unwrap_or_else(|e| {
        let _ = std::fs::write("/tmp/mvp-life.log", format!("RUN_ERROR: {e}\n"));
        std::process::exit(1);
    });          // ❌ 无 RunEvent 分支 → 无 Exit/ExitRequested 钩子
```

```bash
$ grep -n "std::process::exit" src-tauri/src/main.rs
107: 146: 157: 248: 252: 256: 468: 677:   # 共 8 处硬退出 → 跳过析构，Drop 兜底不可靠
```

```vue
<!-- src/components/system/TerminalPane.vue:66-70 -->
onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  term?.dispose();                 // 只销毁 xterm 视图
  system.bindTermWriter(null);     // ❌ 不调 term_kill
});
```

### 必改文件候选

| 文件 | 改动 |
|---|---|
| `src-tauri/src/main.rs` | `.run()` 加 `RunEvent::ExitRequested` / `Exit` 分支；`CloseRequested` 加 `shutdown_all_terminals()`；清理 8 处 `process::exit` 中的业务路径 |
| `src-tauri/src/bridge.rs` | 新增 `term_kill_all`（或复用 `term_kill` 遍历） |
| `src-tauri/permissions/default-commands.toml` | 新增命令（K1） |
| `src/components/system/TerminalPane.vue` | `onBeforeUnmount` 增加销毁语义（区分「隐藏」与「关闭 tab」） |
| 新增 `src-tauri/src/shutdown.rs` | `ShutdownCoordinator`（建议，见 TASK-10） |

### 禁止事项

- ❌ 不得只靠 `Drop` 兜底（8 处 `process::exit` 会跳过）。
- ❌ 不得在事件回调里同步 `join` 子进程线程（会死锁 UI 线程）。
- ❌ 不得用 `kill -9` 全家桶作为唯一手段（先 `SIGTERM`，超时再 `SIGKILL`）。
- ❌ 不得「隐藏面板即杀进程」——用户切面板回来会话就没了（需区分 UI 隐藏 vs 真实关闭）。

### 实现要点

详见 `M3-terminal-shutdown-taskcard-20260902-1146.md`（TASK-10）。要点：

1. `ShutdownCoordinator` 集中注册「可关闭资源」：PTY 会话、脚本子进程、宫格子进程、定时任务线程、数据库连接、插件资源。
2. `RunEvent::ExitRequested` 里统一 `coordinator.shutdown(Duration::from_secs(3))`。
3. 每条路径（CloseRequested / tab 关闭 / 系统退出）都只调 coordinator，不各自为政。

### 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 开 3 个终端（含 `sleep 300`）→ 关闭主窗口 → `ps -ef \| grep sleep` | 零残留 |
| R2 | 开终端运行 `yes > /dev/null` → 退出应用 | 进程消失，CPU 回落 |
| R3 | 直接 `kill -9` 主进程（模拟崩溃） | PTY 孤儿被内核回收（尽力而为，不强求） |
| R4 | 隐藏终端面板再显示 | 会话仍在（未被误杀） |
| R5 | 关 tab（非主窗） | 只杀该 tab 的终端，不影响其它 |

### 验收命令

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 静态
grep -n "RunEvent" src-tauri/src/main.rs                 # 期望命中 ExitRequested/Exit
grep -n "process::exit" src-tauri/src/main.rs | wc -l    # 期望显著少于 8
grep -rn "term_kill\|killShell" src/components/system/TerminalPane.vue src/stores/useSystemStore.ts

# 动态（人工 GUI 验收，AI 不得代签）
# 1) 启动应用，开终端执行：sleep 300 &
# 2) 关闭窗口
# 3) 在宿主机执行：
ps -ef | grep -c "[s]leep 300"       # 期望 0
ls /proc/*/cwd 2>/dev/null | wc -l   # 仅作参考
```

### 失败动作

| 失败 | 动作 |
|---|---|
| 仍有 `sleep` 残留 | 检查 `ExitRequested` 是否真的被触发（Tauri v2 需配合 `api.prevent_close()` 语义）；不要靠加大 `sleep` 掩盖 |
| 关闭窗口后应用不退出（卡在等待） | 检查 `join` 是否阻塞主线程 → 改为 `Duration` 超时后强制 |
| 隐藏面板被误杀 | 回退「隐藏即杀」改动，改为显式关闭按钮才杀 |

### 推荐模型

`AI:DEEP`（生命周期/进程/事件循环交叉，且需人工 GUI 验收）。

---

## E5 · `withGlobalTauri = true` 插件隔离风险

| 字段 | 内容 |
|---|---|
| **ID** | E5 |
| **锚定** | WBS §7.3 **M5-10~M5-12**（需求 #15）+ §4.3 **M2-5/M2-6**（需求 #2） |
| **严重度** | 🟠 中（**安全红线 / 可行性决定项**） |

### 目标

判定并整改：HTML 工具与形态②插件能否在不暴露全局 `window.__TAURI__` 的前提下运行；若不能，形态②插件**不得实现**。

### 证据

```json
// src-tauri/tauri.conf.json
{
  "app": {
    "withGlobalTauri": true,
    "windows": [],
    "security": {
      "csp": null,
      "assetProtocol": {
        "enable": true,
        "scope": ["/usr/share/**", "/usr/local/share/**", "$HOME/.local/share/**", "$HOME/.icons/**"]
      }
    }
  }
}
```

- `withGlobalTauri=true` → 任何 webview 内 `window.__TAURI__.invoke("<任意已注册命令>")` 直接可用。
- `csp: null` → 无内容安全策略约束。
- `capabilities/browser-remote.json` 的 `remote.urls = ["https://*", "http://*"]` 已把能力授予**外部域**的 webview。
- 59 个命令包含 `write_file` / `delete_path` / `launch_app` / `eval_in_tab` 等高危命令。

→ 若外部页面/工具 HTML 落在授予 `default-commands` 的 webview 中，**ACL 形同虚设**。

### 必改文件候选

| 文件 | 改动 |
|---|---|
| `src-tauri/tauri.conf.json` | `withGlobalTauri` → `false`（或按 webview 粒度关闭）；`csp` 显式设置；收窄 `assetProtocol.scope` |
| `src-tauri/capabilities/default.json` | `windows` 精确到 `main`；把高危命令拆到独立 capability |
| `src-tauri/capabilities/browser-remote.json` | `remote.urls` 由 `*/*` 收窄；权限仅保留必需 |
| `src-tauri/permissions/default-commands.toml` | 拆分为「安全命令集」与「高危命令集」 |
| 前端 | 全部改用 `import { invoke } from "@tauri-apps/api/core"` |

### 禁止事项

- ❌ 不得在打开 `withGlobalTauri=true` 的前提下实现形态②插件（可直接调任意命令）。
- ❌ 不得用「插件是本地文件所以可信」作为理由放行（工具可从 `workspace/tools/` 用户目录加载）。
- ❌ 不得把 `remote.urls` 继续留 `*`。
- ❌ 不得在 CSP 里加 `unsafe-inline` / `unsafe-eval` 来「让插件跑起来」。

### 实现要点

完整方案见 `plugin-permission-taskcard-20260902-1146.md`（TASK-11）：

1. **先做可行性判定**（阻塞项）：Tauri v2 是否支持按 webview 粒度控制 `withGlobalTauri`。当前已知事实：`withGlobalTauri` 是 **app 级**配置，无法按 webview 关闭 → **若属实，形态②插件必须改为形态①（独立进程 / stdio）**。
2. 工具 HTML 用 `asset://` 协议加载，capability 仅授予 `tools:*` 的最小命令集。
3. 前端改 `import { invoke }`，全局 grep 校验零 `__TAURI__`。

### 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 在工具 HTML 中执行 `window.__TAURI__` | `undefined` |
| R2 | 外部页面（`https://example.com`）在 `tab-*` webview 中调 `write_file` | 被 ACL 拒绝，且**有日志** |
| R3 | 工具 HTML 调未授权命令 | 被拒绝，返回可读错误 |
| R4 | 设 `withGlobalTauri=false` 后跑全量功能 | 主窗口所有功能正常（无遗漏的 `__TAURI__` 依赖） |
| R5 | CSP 生效后 | 工具内 `eval()` / 内联脚本被拦截（若策略如此定义） |

### 验收命令

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 静态
grep -n "withGlobalTauri" src-tauri/tauri.conf.json     # 期望 false
grep -rn "__TAURI__" src/ src-tauri/src/ | wc -l        # 期望 0
grep -n "csp" src-tauri/tauri.conf.json                 # 期望非 null
grep -n "remote" -A 3 src-tauri/capabilities/browser-remote.json
# 动态（人工）
# 在浏览器页签打开任意外网页，DevTools 执行 window.__TAURI__ → 期望 undefined
```

### 失败动作

| 失败 | 动作 |
|---|---|
| 关闭全局 Tauri 后主窗口功能异常 | 逐个补 `import { invoke }`，不得回滚配置 |
| 判定为「无法按 webview 关闭」 | **形态②插件标记 BLOCKED**，转形态①；在汇总与 `plugin-runtime-taskcard` 中显式登记 |
| CSP 导致工具白屏 | 改用 nonce/hash 白名单，**不得**加 `unsafe-inline` |

### 推荐模型

`AI:DEEP`（安全配置 + 可行性判定 + 全量回归；建议配合 `AI:DEEP-xhigh` 做方案评审）。

---

## E6 · `strip-ansi-escapes` 死依赖（及遗留物）

| 字段 | 内容 |
|---|---|
| **ID** | E6 |
| **锚定** | **M0-6**（`后续需求TODO.md` §11.2，P0 待执行修复项） |
| **严重度** | 🟡 中低（清理类，风险低但收益明确） |

### 目标

清理已确认无用的依赖与遗留字段，降低构建体积与认知负担。

### 证据

```toml
# src-tauri/Cargo.toml
strip-ansi-escapes = "0.2"
gtk = "0.18"     # 注释称用于「方案 C3」
wry = "0.55"     # 注释称用于「方案 C3」
```

```bash
$ grep -rn "strip_ansi_escapes" src-tauri/src
# 零命中
```

- `src-tauri/src/bridge.rs:1942` 注释：「xterm.js 自己解析 ANSI 序列，不再过滤」→ 有意移除。
- `后续需求TODO.md` §11.2（第 415 行）：「M0-6 清理 `gtk`/`wry` 死亡依赖（来自审核报告高危25，P0）」→ **已登记但未执行**。
- 另：`src/stores/useSystemStore.ts` 的 `termLines` 注释「兼容保留，不再用于渲染」。

### 必改文件候选

| 文件 | 改动 |
|---|---|
| `src-tauri/Cargo.toml` | 移除 `strip-ansi-escapes`（**先确认零引用**） |
| `src-tauri/Cargo.toml` | 移除 `gtk` / `wry`（⚠️ 需先确认方案 C3 是否已废弃） |
| `src-tauri/Cargo.lock` | 随 `cargo build` 自动更新 |
| `src/stores/useSystemStore.ts` | 清理 `termLines`（需 grep 确认零引用） |

### 禁止事项

- ❌ **不得在未验证零引用前删除** `gtk` / `wry`（`grid_process.rs` 可能间接依赖 GTK 类型）。
- ❌ 不得在同一次改动中顺手重构其它代码。
- ❌ 不得删除仍被注释引用的能力（先确认方案 C3 已废弃，需人工确认）。

### 实现要点

1. `strip-ansi-escapes`：已确认零引用 → **可直接删**。
2. `gtk` / `wry`：**先跑** `grep -rn "gtk::\|wry::" src-tauri/src` 与 `cargo tree -i gtk`；若确认为死依赖再删。删除后必须 `cargo build --release` 通过。
3. `termLines`：grep 前端零引用后删。
4. 每删一项单独 commit，便于二分回退。

### 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 删除后 `cargo build --release` | 编译通过，无 undefined crate 错误 |
| R2 | 删除后启动应用 | 终端、宫格、页签全部功能正常 |
| R3 | `cargo clippy` warning 数 | 不高于 `logs/baseline-2026-08-27.md` 记录的 13 |
| R4 | 产物体积 | 不增大（删依赖应减小或持平） |

### 验收命令

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 删前确认
grep -rn "strip_ansi_escapes" src-tauri/src | wc -l     # 期望 0（已确认）
grep -rn "gtk::\|wry::" src-tauri/src | wc -l           # 记录基线，决定能否删
cargo tree --manifest-path src-tauri/Cargo.toml -i gtk 2>&1 | head -20
grep -rn "termLines" src/ | wc -l                        # 期望仅定义处

# 删后验证
cargo build --manifest-path src-tauri/Cargo.toml --release 2>&1 | tail -5
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | grep -c "^warning"
```

### 失败动作

| 失败 | 动作 |
|---|---|
| 删 `gtk` 后编译失败 | 立即 `git revert` 该 commit，并把 `gtk` 标为「实际在用，非死依赖」，更正 `后续需求TODO.md` §11.2 的口径（以勘误形式追加，不改原文） |
| clippy warning 增加 | 定位新增项，不得放宽基线 |
| 应用启动异常 | 回退到删除前 commit，逐项重新验证 |

### 推荐模型

`AI:FAST` 可完成 `strip-ansi-escapes` + `termLines`；`gtk`/`wry` 涉及 `grid_process.rs` 的 GTK 类型使用判定 → `AI:BALANCED`，且**必须人工确认方案 C3 已废弃**。

---

## 附：6 项勘误优先级与依赖

```
E4 退出收口（阻塞 5 个下游）  ──最高优先──
  ├── E2 term_resize 后端
  └── E3 termResize 前端调用   （E2+E3 必须同 PR）
E1 5 个种子工具 HTML  ──> M2 排期重算
E5 withGlobalTauri    ──> 决定 #2 工具加载方式 + #15 形态②插件可行性
E6 死依赖清理         ──> 低风险，可随时插入
```

| ID | 优先级 | 推荐模型 | 是否需人工 GUI 验收 |
|---|---|---|---|
| E4 | P0 | `AI:DEEP` | ✅ 必须 |
| E1 | P1 | `AI:BALANCED`（cron 单独 `AI:DEEP`） | ✅ 必须 |
| E2 | P1 | `AI:DEEP` | ✅ 必须（vim/top） |
| E3 | P1 | 与 E2 同 PR → `AI:DEEP` | ✅ 必须 |
| E5 | P1（可行性判定 P0） | `AI:DEEP` / 方案评审 `AI:DEEP-xhigh` | ✅ 必须（DevTools 验证） |
| E6 | P2 | `AI:FAST` / `AI:BALANCED` | 部分（启动回归） |
