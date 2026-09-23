# RESOURCE-OWNERSHIP-MATRIX（STAGE H-D）

> 真源：`docs/architecture/capability-registry/resources.yaml`（资源分类/生命周期策略，DECLARED 口径）
> + `docs/architecture/native-boundary/native-commands.yaml`（native 命令 → 资源/权限）
> + `scripts/runtime-resource-absence.mjs`（RRA-01..08，**RUNTIME_OBSERVED** 缺席证据，含区分力自检）。
>
> **证据等级口径（严禁高报）**
> - `MEASURED`：有 OS/运行时量化测量（本项目**当前为 0**，见 resources.yaml measurement_status）。
> - `RUNTIME_OBSERVED`：真实装配 + 插桩计数证明「资源出生点被调用 0 次 / N 次」。
> - `STRUCTURAL`：由真实代码结构与门禁静态证明（owner/入口唯一、无第二出生点）。
> - `DECLARED`：仅声明（resources.yaml policy），**无**运行时探针。
> - `UNKNOWN`：**0**（不允许）。
>
> 本文件是上述真源的**视图**，不制造第三份 truth；一致性由
> `check-capability-resource-boundary.mjs`（12/12）与 `check-native-capability-boundaries.mjs`
> NATIVE-04（native 资源 ↔ capability `resources.class` 对齐）机器校验 —— **resource owner drift → FAIL**。

---

## 1. 资源归属矩阵

| RESOURCE_ID | TYPE | OWNER | CREATE_ENTRY | ACTIVATE | SUSPEND | DESTROY/RELEASE | ABSENCE_BEHAVIOR | NATIVE_COMMAND | CAPABILITY | EVIDENCE_LEVEL |
|---|---|---|---|---|---|---|---|---|---|---|
| MAIN_WINDOW_WEBVIEW | WEBVIEW | workbench(framework) | Tauri 主窗口建立 | 启动即激活 | 不支持 | 应用退出 | 恒为 1（框架自身） | —（框架） | workbench | STRUCTURAL |
| BROWSER_WEBVIEW | WEBVIEW | browser | `open_browser` / `tab_new` | 视图激活 | 不支持（不可冻结） | `close_browser`（经 `browser.closeBrowser()` public 动作） | browser absent → 0 webview | open_browser, tab_new, tab_close | browser | RUNTIME_OBSERVED |
| GRID_CHILD | CHILD_WEBVIEW/PROCESS | browser（grid 视图） | `create_grid` | grid 视图 + `openGrid` | 不支持 | `close_grid`（RRA-07 证明 DESTROY） | framework → createGrid **0 次** ⇒ grid-child ≡ 0 | create_grid, close_grid, grid_position | browser | RUNTIME_OBSERVED |
| PTY | PTY | terminal | `term_spawn` / `term_spawn_channel` | 终端激活 | 不支持（PTY 不能安全冻结） | `term_kill` | framework → PTY 创建 **0 次**；terminal absent → 0 | term_spawn, term_spawn_channel, term_kill | terminal | RUNTIME_OBSERVED |
| TERMINAL_CHILD_PROCESS | PROCESS | terminal | `term_spawn` | 终端激活 | 不支持 | `term_kill` | terminal absent → 0 子进程 | term_spawn, term_kill | terminal | RUNTIME_OBSERVED |
| DB_CONNECTION | NETWORK/SECRET | database | `db_connect`（查询瞬态建连即弃） | 查询 | `db_cancel` | `db_disconnect` | database absent → 0 连接（**无运行时探针**） | db_connect, db_query, db_disconnect, db_cancel | database | STRUCTURAL |
| GIT_PROCESS | PROCESS/NETWORK | git | `git_status`/`git_diff`/`git_log` 等 | 操作触发 | 支持（TARGET） | 命令结束即释放 | git absent → 0 git 进程 | git_status, git_diff, git_log, request_git_write | git | STRUCTURAL |
| FILESYSTEM_WATCHER | BACKGROUND | resource_collection / task | 后台采集启动 | 后台 | 支持（TARGET） | 停止采集 | absent → 0 watcher | （resource 采集相关） | resource_collection, task | DECLARED |
| FILE_PREVIEW | FILESYSTEM | workspace | `read_file` / `read_image_data_url` | 打开预览 | 不适用 | 关闭即释放 | workspace absent → 无预览 | read_file, read_image_data_url | workspace | STRUCTURAL |
| PLUGIN_RUNTIME | NATIVE/PROCESS | plugin | `plugin_install` / `plugin_enable` | 启用 | 不支持（LOCKED） | 不支持（LOCKED，见 release_note） | plugin absent → 0 runtime | plugin_install, plugin_enable, plugin_disable | plugin | DECLARED |
| AGENT_EXECUTION | NETWORK/PROCESS | agent | agent 会话发起 | 会话 | 支持（TARGET） | 停止会话 | agent absent → 0 请求 | agent_parse/validate（只读）+ 会话命令 | agent | DECLARED |
| TASK_EXECUTION | BACKGROUND/PROCESS | task | `task_run_now` / 调度 | 触发 | 支持（TARGET） | 停止调度 | task absent → 0 定时唤醒 | task_run_now, task_add | task | DECLARED |
| SCRIPT_PROCESS | PROCESS | script（workspace 子能力） | `run_script` / `run_command` | 执行 | 不支持 | `cancel_script`（TARGET） | workspace absent → 无 scripts 视图、无脚本进程出生点 | run_script, run_command, cancel_script | script | DECLARED |
| APP_CHILD_PROCESS | PROCESS(detached) | apps | `launch_app` | 用户动作 | 不支持（detached 不由能力管） | 不由能力回收 | apps absent → 无枚举/启动 | launch_app, list_apps | apps | DECLARED |
| TOOL_WEBVIEW | WEBVIEW | tools | `open_tool`（`tool://` 子 webview） | 打开工具 | 支持（TARGET） | 关闭子 webview | tools absent → 0 工具 webview | open_tool, list_tools | tools | DECLARED |
| NETWORK_SOCKET | NETWORK | database / git / agent | 各能力自身 | 请求时 | 请求结束 | 请求结束释放 | 对应能力 absent → 0 socket | db_* / git_* / agent_* | database, git, agent | DECLARED |
| KEYRING | SECURITY_SENSITIVE | credential | Rust `security_policy.rs`（KeyringStore） | 常驻 | 不支持（安全边界） | 不支持（常驻句柄） | credential 为安全基础设施，不参与用户组合 | （Rust 侧） | credential | STRUCTURAL |
| SESSION_PERSISTENCE | DISK/LIGHT | session | `session_save` | 会话保存 | 不支持（关停链路） | `flush_sessions`（退出前 flush） | session absent → 无会话持久化 | session_save, flush_sessions | session | DECLARED |
| RESOURCE_MONITORING | BACKGROUND | resource_collection | 采集开关 | 后台 | 支持（TARGET） | 停止采集 | absent → 0 采集 | report_resources, resource_stats | resource_collection | DECLARED |
| CLIPBOARD_MEMORY | LIGHT | clipboard | 历史入内存（**不落盘**） | 读写 | 支持 | 清空会话历史 | clipboard absent → 无历史 | clipboard_read, clipboard_write | clipboard | STRUCTURAL |
| VAULT_SNAPSHOT | FILESYSTEM(read-only) | vault | `vault_open` 目录快照 | 打开 | 支持 | 释放内存快照 | vault absent → 0 快照 | vault_open, save_note | vault | STRUCTURAL |

---

## 2. 证据等级统计（诚实）

| 等级 | 数量 | 说明 |
|---|---|---|
| **MEASURED** | **0** | 项目无按能力资源实测机制（`resources.yaml: measurement_status`） |
| **RUNTIME_OBSERVED** | **4** | BROWSER_WEBVIEW / GRID_CHILD / PTY / TERMINAL_CHILD_PROCESS（RRA-01/03/04/07/08 插桩计数） |
| **STRUCTURAL** | **7** | MAIN_WINDOW_WEBVIEW / DB_CONNECTION / GIT_PROCESS / FILE_PREVIEW / KEYRING / CLIPBOARD_MEMORY / VAULT_SNAPSHOT |
| **DECLARED** | **10** | 其余（仅 resources.yaml policy，无运行时探针） |
| **UNKNOWN** | **0** | — |

> **没有**把 DECLARED 写成 MEASURED，**没有**把 STRUCTURAL 写成 RUNTIME PASS。

---

## 3. 资源硬不变量复核（§5）

| 不变量 | 结论 | 证据 | 等级 |
|---|---|---|---|
| framework-only: grid-child = 0 | **PASS** | RRA-01（createGrid 调用 0 次；createGrid 是 grid-child 唯一出生点）+ RRA-02（gridOpen 恒 false） | RUNTIME_OBSERVED |
| framework-only: PTY = 0 | **PASS** | RRA-03（ensureTerm → PTY 创建 0 次） | RUNTIME_OBSERVED |
| Terminal absent: no PTY / no terminal child | **PASS** | 同 RRA-03（framework 即 terminal absent 场景） | RUNTIME_OBSERVED |
| Browser absent: no browser heavy resource / no grid child | **PASS** | RRA-01/02（framework 即 browser absent） | RUNTIME_OBSERVED |
| Database absent: no DB connection | **未实测** | 无运行时探针；仅 resources.yaml DECLARED + NATIVE-04 对齐 | **STRUCTURAL（不高报）** |
| Plugin absent: no plugin runtime | **未实测** | runtime LOCKED，无启停探针 | **DECLARED** |
| Agent absent: no agent execution runtime | **未实测** | 无会话探针 | **DECLARED** |

> 未实测项**明确标为 STRUCTURAL / DECLARED**，不以“应该没有”冒充已验证。

---

## 4. C5 诚实性（§6）

**C5 RESOURCE_RELEASABLE 声称的能力 = 0。**

依据 `resources.yaml` 的 `release_note`：绝大多数为「**TARGET，当前不可**」（browser/grid webview 不可物理卸载、terminal PTY 不可安全冻结、plugin LOCKED）。
仅有 `destroyable: true` 的**声明**与 `destroy()` 入口，**不构成 C5** —— C5 需证明「资源存在 → deactivate/destroy → 资源不再存在」，当前无对应实测。

- 唯一具备真实编排试点语义的是 `bookmark`（LIGHT，`destroyable: true`，HP2 运行时 register/unregister 已由 `check-capability-platform.mjs` 验证），
  但它释放的是**轻量内存态**，非 native 资源 ⇒ **仍不声称 C5**。
- 因此成熟度分布维持：C4 = 0、C5 = 0（登记 Debt-7A-1）。

---

## 5. resource owner drift → FAIL 的机器保障

| 门禁 | 作用 |
|---|---|
| `check-capability-resource-boundary.mjs` | 能力资源边界（12/12 PASS） |
| `check-capability-registry.mjs` | 每个能力必须有 resources policy |
| `check-native-capability-boundaries.mjs` **NATIVE-04** | native 命令声明的 resource 必须与 capability `resources.class` 对齐（如 PTY↔PROCESS、DB_CONNECTION↔NETWORK/SECRET） |
| `runtime-resource-absence.mjs` | absent 不创建重资源（12/12 PASS，含区分力自检） |

**结论：H-D = PASS**（无 UNKNOWN；未实测项诚实标记；C5 不虚标）。
