# 19 · FRAMEWORK RESOURCE OWNERSHIP（启动期资源归属清单）

> 本清单回答：**framework profile 启动后，进程里每一个资源是谁的？该不该存在？**
> 分类：`FRAMEWORK_REQUIRED` / `CAPABILITY_OWNED` / `LEGACY_RESOURCE_LEAK` / `UNKNOWN`
>
> 原则：**「看不见」不能作为「资源不存在」的证据。**
> 结构性缺席（槽为空）≠ 运行期资源缺席（真的没有进程/WebView/PTY）。

---

## 1. 汇总（修复后）

| 分类 | 数量 | 说明 |
|---|---|---|
| FRAMEWORK_REQUIRED | 3 | 主窗口 WebKit 进程族 + UDS listener 惰性 |
| CAPABILITY_OWNED | 2 类 | 宫格子进程（grid-child）、PTY |
| LEGACY_RESOURCE_LEAK | 0（修复前 1） | 宫格子进程 ×4 —— **H-G blocker** |
| UNKNOWN | 1 | Browser capability WebView 与主窗口 WebKit 的进程级区分（见 §4） |

---

## 2. 逐项清单

| # | 资源 | 创建点 | 分类 | framework 下应为 |
|---|---|---|---|---|
| R1 | 主窗口 `WebKitWebProcess` | Tauri 主窗口初始化 | FRAMEWORK_REQUIRED | 1（EXPECTED） |
| R2 | `WebKitNetworkProcess` | WebKit 网络栈 | FRAMEWORK_REQUIRED | 1（EXPECTED） |
| R3 | 宫格子进程 `mvp-browser-os --grid-child N` | `grid_process.rs::spawn_with_state`（L242） ← `bridge.rs::create_grid`（L3877 `get_or_spawn`） ← 前端 `bridge.createGrid` ← `useBrowserStore.buildGrid` | **CAPABILITY_OWNED**（Browser） | **0** |
| R4 | PTY（伪终端） | `useTerminalStore.spawnTerm` → `bridge.termSpawnChannel` → `term_spawn_channel` | **CAPABILITY_OWNED**（Terminal） | **0** |
| R5 | 宫格 UDS socket listener（`/tmp/...grid-N.sock`） | `grid_process.rs::ensure_listener` | CAPABILITY_OWNED（随 R3 惰性创建） | 0 |
| R6 | DB 连接（sqlite/mysql/postgres） | `database.rs`，**按需**打开，`drop` 即释放 | FRAMEWORK_REQUIRED（按需） | 0（未使用则不建） |
| R7 | 剪贴板监听 | `useSystemStore.startClipWatch` → `bindClipFocus()`，**事件驱动，无轮询/无守护** | FRAMEWORK_REQUIRED | 0 常驻资源 |
| R8 | 文件系统 watcher | 全仓 grep：启动路径无 fs.watch | — | 0 |
| R9 | 后台定时任务 / setInterval 轮询 | 全仓 grep `setInterval`：src 内 0 | — | 0 |
| R10 | 网络监听 socket | 无（MCP stdio 仅 `--features mcp` 编译，默认不含） | — | 0 |

---

## 3. H-G BLOCKER：宫格子进程泄漏

### 3.1 人工验收观测（修复前）

```
VITE_CAPABILITY_PROFILE=framework npm run tauri dev   ← 无任何点击

1442358 mvp-browser-os
 ├─ WebKitNetworkProcess          ← EXPECTED (R2)
 ├─ WebKitWebProcess              ← EXPECTED (R1)
 ├─ mvp-browser-os --grid-child 0 ← UNEXPECTED
 ├─ mvp-browser-os --grid-child 1 ← UNEXPECTED
 ├─ mvp-browser-os --grid-child 2 ← UNEXPECTED
 └─ mvp-browser-os --grid-child 3 ← UNEXPECTED
```

判定：**H-G RESOURCE ABSENCE = FAIL**（RELEASE BLOCKER）。
不得弱化为 WARN / KNOWN DEBT / STRUCTURAL PASS。

### 3.2 根因

宫格子进程是 **Browser capability-owned 重资源**（`browser/manifest.ts`
声明 `resources: [{kind:"CHILD_PROCESS", ownership:"owned"}, {kind:"WEBVIEW", ownership:"owned"}]`），
但其创建路径曾经可以被「能力可用性之外的原因」触发：

- **PROBLEM A**：Framework Core（`useLayoutStore`）反向依赖 Browser 内部实现，
  直接调用 `browser.openGrid()` / `browser.closeGrid()`。
- **PROBLEM B**：`gridToolbarOpen` 是 **持久化 UI preference**，却被当成
  「立即创建 Grid 重资源」的许可。
  `preference ≠ availability ≠ activation ≠ resource existence`。

> 补充审计事实（诚实记录）：当前 HEAD 的 `useLayoutStore.toggleGridToolbar()`
> 实际是**死代码**（全仓无调用方），且函数体内引用的 `useBrowserStore` 在本文件
> **从未 import**，一旦被调用会抛 `ReferenceError`。
> 也就是说：人工验收时的具体触发点无法由 HEAD 源码静态复现（可能与历史构建/缓存有关）。
> **正因如此，修复选择收口在唯一的资源出生点，而不是逐个调用方打补丁** ——
> 这样无论触发链来自何处（已知入口 / 历史遗留入口 / 未来新增入口 / 尚未定位的入口），
> absent Browser 都不可能产出宫格子进程。

### 3.3 修复

| 层 | 改动 | 文件 |
|---|---|---|
| 判定真源 | 新增 `isCapabilityActive(id)`：只读能力编排记录（注册 + ACTIVE + 未 disable），fail-closed | `src/capability/runtimeSingleton.ts` |
| 单例发布 | bootstrap 成功后把 Runtime 发布到叶子模块（避免 ESM 循环） | `src/capability/index.ts` |
| Browser 闸 | `isBrowserResourceAllowed()` | `src/capabilities/browser/resource/guard.ts` |
| **唯一出生点收口** | `buildGrid()` 开头 `if (!isBrowserResourceAllowed()) return;` | `useBrowserStore.ts` |
| Terminal 闸（同类防护） | `isTerminalResourceAllowed()` + `spawnTerm()` 前置 | `terminal/resource/guard.ts`, `useTerminalStore.ts` |
| Shell 解耦 | `toggleGridToolbar()` 只翻转 UI 偏好，不再触碰 Browser | `src/stores/useLayoutStore.ts` |
| 依赖方向纠正 | Browser **单向** watch Shell 的 `gridToolbarOpen`，且先过闸 | `useBrowserStore.ts` |

### 3.4 修复后证据

**A. 真实进程级证据（OS 层，`npm run tauri dev` 原样复现人工验收命令）**

```
$ VITE_CAPABILITY_PROFILE=framework npm run tauri dev     # 无任何点击
APP_PID=3804378
GRID_CHILD_COUNT=0                 ← 修复前为 4
--- direct children ---
3805316 WebKitNetworkPr            ← EXPECTED（主窗口）
3805325 WebKitWebProces            ← EXPECTED（主窗口）
```

同一轮启动的前端存活证据（`logs/capability-hg/framework-process-evidence-20260922.txt`）：

```
[FE] [capability] bootstrap activated=true error=none
[FE] [FE] vue mounted
[FE] [FE] first-paint probe ... text="＋−□×🏠主页📁浏览🗂️宫格▾☰菜单前往🤖📥采集⚙️
      🏠主页 ☆ 收藏网页  📁 收藏目录 ＋ 新增↺ 默认 主要工作区🌐浏览📂文件💻终端📋剪..."
```

> 这段 first-paint 文本同时是 **UI_PRESERVATION 的直接证据**：
> 窗口结构、🏠主页/📁浏览/🗂️宫格/☰菜单/🤖/📥采集/⚙️ 全部在位，
> Home 的 17 张启动卡（🌐浏览📂文件💻终端📋剪…）全部在位。
> **不是空壳。**

**B. 应用逻辑层运行期证据（可重复自动化）**

| 证据 | 来源 | 结果 |
|---|---|---|
| `createGrid` 调用次数（Browser absent，含偏好 true + activateGrid + openGrid） | `scripts/runtime-resource-absence.mjs` RRA-01 | **0** |
| `gridOpen`（Browser absent） | RRA-02 | **false** |
| PTY 创建次数（Terminal absent） | RRA-03 | **0** |
| 负例：闸恒放行后同场景 | `--self-test` SELF-03 | **createGrid=1（泄漏被复现）** → 断言有区分力 |

> `createGrid` 是 `--grid-child` 进程的**唯一**出生点（全仓 grep 仅 1 处），
> 故 `createGrid = 0` ⇒ `GRID_CHILDREN = 0`。
> A 与 B 互为印证：B 是可重复的自动化门禁，A 是 OS 层真值。

---

## 4. UNKNOWN 项（不伪造 PASS）

| 项 | 状态 | 说明 |
|---|---|---|
| Browser capability WebView vs 主窗口 WebKit 进程 | **UNKNOWN** | 二者同为 `WebKitWebProcess` 形态，自动化**无法可靠按进程区分**。故只断言可确证的 `grid-child` 计数，不以 WebKit 进程数冒充 Browser 泄漏证据（红队 §28-16）。 |
| 宫格 UDS socket 残留文件 | UNKNOWN | 本次未做文件系统级采集（只断言创建调用）。 |

诚实记录（不伪造 PASS）：
```
MAIN_WINDOW_WEBKIT   = EXPECTED    （实测 2 个直接子进程：Network + Web）
CAPABILITY_WEBVIEW   = UNKNOWN     （无法与主窗口 WebKit 区分）
GRID_CHILDREN        = VERIFIED 0  （OS 层实测，进程名 --grid-child）
PTY                  = VERIFIED 0  （应用层出生点计数；无 PS 级 PTY 观测手段）
```

---

## 5. 门禁

| 门禁 | 覆盖 |
|---|---|
| `scripts/check-capability-resource-boundary.mjs` | 声明式边界（manifest resource ownership + 已声明 Shell/Core 边界 + 已知生命周期入口）；含正例/负例/self-test |
| `scripts/runtime-resource-absence.mjs` | 运行期资源缺席（真跑产品代码，统计出生点调用）；含正例/负例/self-test |

两者均已接入 `npm run check`。
