# Phase 1 Runtime GUI Acceptance — 验收记录（第一阶段）

> 本轮最高原则：**保留现有安装版本**。全程未执行安装、覆盖、替换、删除动作。
> R1–R6 为交互式视觉场景，由用户在桌面执行；R7（关闭无残留）为机器可观测项，由 Agent 完成验证。

---

## 1. 约束执行情况

| 禁止项 | 实际执行 |
|---|---|
| `sudo apt install` 新 deb | 未执行 ✅ |
| `sudo dpkg -i` 新 deb | 未执行 ✅ |
| 覆盖 `/usr/bin/mvp-browser-os` | 未执行 ✅ |
| 修改系统 desktop entry | 未执行 ✅ |
| 修改现有用户数据 | 未执行 ✅（业务文件 mtime 未变） |
| 删除旧安装 | 未执行 ✅ |
| build / install / upgrade deb | 未执行 ✅ |

## 2. 当前安装状态（基线，全程未变）

| 项目 | 值 |
|---|---|
| 二进制路径 | `/usr/bin/mvp-browser-os` |
| 所属包 | `mvp-browser-os` |
| 包版本 | `0.1.0`（`Status: install ok installed`） |
| 二进制大小 | `11957440` 字节 |
| 二进制哈希 | `96ebcbd03cf7355113aa2fc500605c7ba9df77298eed1f26f6aa2b8fdbc2b6e4` |
| 修改时间 | `2026-09-16 09:24:55 +0800` |

验收结束后复核：哈希、大小、mtime **全部未变**。

## 3. 应用数据目录

| 项目 | 值 |
|---|---|
| 应用数据根 | `~/.local/share/com.jizhijiandan.mvp` |
| 存在 / 大小 | 是 / **421M** |
| 顶层条目 / mtime | 12 个 / `2026-09-16 10:09:23` |
| 业务目录内容 | `audit.json`、`bookmarks.json`、`browser-credentials.json`、`notes`、`sessions`、`workspace` |

- 未复制、未读取内容；未触碰 cookie / keyring / password / credential secret。
- 另存在 `~/.local/share/mvp-browser-os`（已知 resolver 缝隙：`path().data_dir()` 与 `app_data_dir()` 口径不一致），仅记录未改动。

## 4. 运行前快照

```
路径: .snapshots/2026-09-19-before-phase1-gui-validation.txt
HEAD: 88f898707fa9e02d7f84f088356ed7b272ced876
分支: master
状态: 干净（仅未跟踪的 .snapshots/ 与 diagnostics/）
```

## 5. 被测二进制（确保测的是 Phase 1 代码）

**方案 A 被否决（关键）**：`src-tauri/target/release/mvp-browser-os` 构建于 `2026-09-16 09:24`，大小与已安装二进制一致 —— 打包的是 **Phase 1 之前的前端**（Phase 1 提交 `a30fd57` 时间为 `2026-09-18 17:45`）。使用它等于测错代码。

**采用方案 B**：`npm run tauri dev`（不产出 deb，直供当前源码）。

| 项目 | 值 |
|---|---|
| 路径 | `src-tauri/target/debug/mvp-browser-os` |
| 哈希 | `218106d638e115789311f4ec031592d4237f595b8f5093680a95c22eb44d700c` |
| git HEAD | `88f8987`（含 Phase 1 的 `a30fd57`） |
| 当前代码证据 | 主窗口加载 `http://localhost:1421/`（vite 开发服务器直供 `src/`），非打包 dist |

## 6. 验收结果总览

| 场景 | 状态 | 执行方 |
|---|---|---|
| R1 打开宫格 | **PASS** | 用户（人工验收） |
| R2 宫格 → 浏览器 | **PASS** | 用户（人工验收） |
| R3 浏览器 → 宫格（状态保留） | **PASS** | 用户（人工验收） |
| R4 连续切换 ≥10 次 | **PASS** | 用户（人工验收） |
| R5 缩放窗口 | **PASS** | 用户（人工验收） |
| R6 最大化 / 还原 | **PASS** | 用户（人工验收） |
| **R7 关闭无残留** | **PASS** | **Agent（已验证，见 §8 / §9）** |
| 启动与渲染（附加） | PASS | Agent |

**R1–R6 的依据声明（重要）**：这六项判据本质为视觉判断（无白屏、无偏移、布局正确、宫格不被覆盖、页面不重新登录），Agent 无屏幕可见性、无可靠点击能力，且本机 `wmctrl`（物理像素）与 `xdotool`（逻辑像素）存在 **2 倍坐标差**，坐标点击不安全。因此 **R1–R6 的 PASS 来自用户在真实桌面的人工验收确认，而非 Agent 自动化证据**。Agent 提供的旁证仅限于：应用在当前 Phase 1 代码下正常启动渲染、宫格子进程被创建并正常关闭、全程无 panic、无进程残留。

## 7. 已验证：真实桌面启动与渲染

- cargo 构建 17.77s → 进程 pid `633914` 启动。
- `[FE] vue mounted` → 首屏探针渲染出界面文本（`主页 / 浏览 / 宫格 / 收藏夹 / 菜单 / 文件 …`）。
- 窗口尺寸 `1854x1048`；无启动崩溃、无 panic。
- 结论：**当前 Phase 1 代码可在真实桌面正常编译、启动、渲染**（无启动回归）。

## 8. 已验证：R7 关闭无残留（判据：`ps aux | grep mvp-browser-os` 无异常残留）

| 检查项 | 结果 |
|---|---|
| 应用主进程残留 | **无**（`ps` 查 `mvp-browser-os` 为空）✅ |
| 宫格子进程 | **正常关闭** —— 日志 `grid-child-0..3 UDS 对端关闭 / shutdown` ✅ |
| 僵尸进程 | **0** ✅ |
| panic / SIGSEGV / abort | **0**（日志 grep 计数 0）✅ |
| WebKit 残留进程 | 仅 1 个 `WebKitNetworkProcess`（pid 5501），**父进程为 `/usr/bin/clash-verge`**，与本项目无关 ✅ |
| 关闭路径 | 日志 `[shutdown] completed ok=true already_shutdown=false executed=7`，随后二次调用幂等返回 `already_shutdown=true` ✅ |

日志证据（`logs/session-20260919-060535-main-pid633914.log` 尾部）：

```
[scheduler] 调度线程已退出
[grid-manager] grid-child-2 UDS 对端关闭
[grid-manager] shutdown grid-child-2
…（0/1/3 同）
[shutdown] kill-running-scripts killed=0
[shutdown] completed ok=true already_shutdown=false executed=7
[shutdown] completed ok=false already_shutdown=true executed=0
```

**R7 判定：PASS**（按既定判据，进程层面完全干净）。

## 9. 重要发现：宫格 socket 文件残留（既有债务，非 Phase 1 引入）

进程干净，但**文件层面存在残留**：

- 本次实例 `grid-633914-0..3.sock`（`06:07` 创建）在应用退出后**未被 unlink**，仍留在 `~/.local/share/com.jizhijiandan.mvp/sock/`。
- 该目录**累计残留 134 个** `grid-<pid>-<n>.sock` 文件，最早可追溯到 `2026-08-25`。

| 属性 | 判断 |
|---|---|
| 是否 Phase 1 引入 | **否** —— 8 月 25 日即存在，远早于 Phase 1（9-18） |
| 是否影响功能 | 未见功能影响（每次按 pid 命名，不冲突） |
| 性质 | 文件级资源泄漏（UDS socket 未在退出时清理） |
| 建议 | 作为既有债务挂账；若后续清理，须先备份且不得影响当前用户数据 |

> 注意：本次实例虽未点击宫格，日志显示启动时即创建了 4 个 grid child 与对应 socket，故仍会留下 4 个文件。

## 10. 用户关闭后的 R7 复查（第二次）

用户在桌面完成 R1–R6 并关闭应用后，Agent 再次复查：

| 检查项 | 结果 |
|---|---|
| 应用主进程残留 | **无** —— `ps` 精确匹配 `target/debug/mvp-browser-os` 为空 ✅ |
| 误报说明 | `pgrep -f` 曾命中 pid 681077，但其父进程为 Electron utility 进程（IDE），仅命令行含该路径字符串，**非本应用残留** |
| 僵尸进程 | 0 ✅ |
| panic / SIGSEGV | 0 ✅ |
| 关闭日志 | `[shutdown] completed ok=true already_shutdown=false executed=7`，宫格子进程全部 shutdown ✅ |
| socket 残留 | 本次 4 个；`sock/` 累计 **138** 个（既有债务，见 §9，非 Phase 1 引入） |

## 11. 结论

```text
DATA_MODIFIED: NO
SYSTEM_INSTALL_MODIFIED: NO
INSTALLED_VERSION_UNCHANGED: YES (96ebcbd0…)
R1–R6: PASS（用户人工验收）
R7_SHUTDOWN_NO_RESIDUE: PASS（Agent 两次验证）
READY_FOR_FINAL_TAG: YES
```

用户在桌面完成 R1–R6 人工验收并确认通过后，已创建 annotated tag
**`semantic-phase1-browser-grid-pass`**（本地，**未推送**）。

遗留（不影响本次通过，挂账）：

- 宫格 UDS socket 文件退出时不清理，`sock/` 累计 138 个（自 2026-08-25，非 Phase 1 引入）。
- R1–R6 无自动化回归覆盖（视觉判据不可自动化）；后续若要机器守护，需补 DOM/进程级探针。
