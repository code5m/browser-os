# NATIVE_CAPABILITY_BOUNDARY_AUDIT
> 阶段：STAGE H-C（Platform Hardening，P0 / D3）。真源：`docs/architecture/native-boundary/native-commands.yaml`。
> 扫描：`src-tauri/src/**/*.rs` 的 `#[tauri::command]`；门禁：`scripts/check-native-capability-boundaries.mjs`。

## 1. 总量与归属分布

| 指标 | 数值 |
|---|---|
| NATIVE_COMMANDS_TOTAL | 148 |
| NATIVE_COMMANDS_OWNED | 148 |
| NATIVE_UNKNOWN_OWNER / LEGACY_UNOWNED | 0 |
| registry↔Rust 漂移 | 0 |

### 按 owner

| Owner | 命令数 |
|---|---|
| `workspace` | 31 |
| `browser` | 28 |
| `session` | 10 |
| `script` | 9 |
| `terminal` | 8 |
| `plugin` | 8 |
| `git` | 7 |
| `resource_collection` | 6 |
| `agent` | 6 |
| `task` | 5 |
| `database` | 5 |
| `workbench` | 4 |
| `bookmark` | 3 |
| `skill` | 3 |
| `credential` | 3 |
| `graph` | 3 |
| `vault` | 2 |
| `clipboard` | 2 |
| `apps` | 2 |
| `tools` | 2 |
| `settings` | 1 |

## 2. 归属模型分类

| 分类 | 数量 |
|---|---|
| CAPABILITY_NATIVE_ADAPTER | 140 |
| FRAMEWORK_NATIVE_SERVICE | 5 |
| SECURITY_INFRASTRUCTURE | 3 |

- CAPABILITY_NATIVE_ADAPTER：能力专属原生适配器（每命令归属一个能力）。
- FRAMEWORK_NATIVE_SERVICE：框架自身服务（诊断/意图总线/启动/URL 交接/休眠策略）。
- SECURITY_INFRASTRUCTURE：安全基础设施（credential / keyring），禁止明文回传。
- LEGACY_UNOWNED：**0**（UNKNOWN=0）。

## 3. 资源与权限

| 资源 | Owner 集合 | 命令数 |
|---|---|---|
| `NONE` | `bookmark`, `clipboard`, `graph`, `resource_collection`, `session`, `settings`, `skill`, `vault`, `workbench`, `workspace` | 49 |
| `CHILD_PROCESS` | `apps`, `script`, `task`, `workspace` | 20 |
| `WEBVIEW` | `browser` | 19 |
| `FILESYSTEM` | `workspace` | 12 |
| `GRID_CHILD` | `browser` | 9 |
| `PTY` | `terminal` | 8 |
| `PLUGIN_RUNTIME` | `plugin` | 8 |
| `GIT_PROCESS` | `git` | 7 |
| `AGENT_EXECUTION` | `agent` | 6 |
| `DB_CONNECTION` | `database` | 5 |
| `KEYRING` | `credential` | 3 |
| `TOOL_WEBVIEW` | `tools` | 2 |

## 4. 显式登记的跨域直调（allowed_callers）

| 命令 | Owner | 允许的调用方 | 理由 |
|---|---|---|---|
| `cancel_script` | `script` | `workspace,task` | script 为 workspace 子能力（H-A 裁决），其 store 物理托管于 capabilities/workspace/state；task 在 capabilities.yaml 声明 optionalDependencies: [script]。 |
| `clipboard_write` | `clipboard` | `workspace` | workspace『复制路径』写入系统剪贴板；clipboard 无重资源、无后台任务，属轻量共享动作。 |
| `get_start_dirs` | `workspace` | `home` | home 在 manifest 声明 required dependsOn workspace；主页首次播种后端起始目录。 |
| `launch_app` | `apps` | `home` | home 在 manifest 声明 required dependsOn apps；主页启动器启动系统应用（apps 为 detached 进程，非驻留重资源）。 |
| `run_command` | `script` | `workspace,task` | script 为 workspace 子能力（H-A 裁决），其 store 物理托管于 capabilities/workspace/state；task 在 capabilities.yaml 声明 optionalDependencies: [script]。 |
| `run_script` | `script` | `workspace,task` | script 为 workspace 子能力（H-A 裁决），其 store 物理托管于 capabilities/workspace/state；task 在 capabilities.yaml 声明 optionalDependencies: [script]。 |
| `script_add` | `script` | `workspace,task` | script 为 workspace 子能力（H-A 裁决），其 store 物理托管于 capabilities/workspace/state；task 在 capabilities.yaml 声明 optionalDependencies: [script]。 |
| `script_list` | `script` | `workspace,task` | script 为 workspace 子能力（H-A 裁决），其 store 物理托管于 capabilities/workspace/state；task 在 capabilities.yaml 声明 optionalDependencies: [script]。 |
| `script_remove` | `script` | `workspace,task` | script 为 workspace 子能力（H-A 裁决），其 store 物理托管于 capabilities/workspace/state；task 在 capabilities.yaml 声明 optionalDependencies: [script]。 |
| `script_runs_list` | `script` | `workspace,task` | script 为 workspace 子能力（H-A 裁决），其 store 物理托管于 capabilities/workspace/state；task 在 capabilities.yaml 声明 optionalDependencies: [script]。 |
| `script_status` | `script` | `workspace,task` | script 为 workspace 子能力（H-A 裁决），其 store 物理托管于 capabilities/workspace/state；task 在 capabilities.yaml 声明 optionalDependencies: [script]。 |
| `script_update` | `script` | `workspace,task` | script 为 workspace 子能力（H-A 裁决），其 store 物理托管于 capabilities/workspace/state；task 在 capabilities.yaml 声明 optionalDependencies: [script]。 |

> 除上表外，**任何**跨能力直连 `bridge.<cmd>()` 都会被 NATIVE-02 判 FAIL。
> 能力 public store 的同名动作（如 `browser.tabNew()`）**不**算直连 native——那正是能力 public 边界的正确用法。

## 5. 安全要点

- credential/keyring（`import_browser_credentials` / `list_browser_credentials` / `fill_browser_credential`）归 `credential`（SECURITY_INFRASTRUCTURE）：
  NATIVE-05 检查其调用点附近是否流入 console/localStorage 泄露汇（与 `check-sensitive-side-effects.mjs` R7 互补）。
- process.spawn 类命令（script/terminal/git/apps/task/plugin）均为能力归属重资源；Shell 直连即 NATIVE-03 FAIL。

## 6. CURRENT → TARGET

| 维度 | CURRENT | TARGET |
|---|---|---|
| 物理位置 | 148 命令集中在 `src-tauri/src/bridge.rs` | 按能力拆 `src-tauri/src/capabilities/<id>/` |
| 归属声明 | 无 → 本轮补齐 `native-commands.yaml` | 与能力清单同源、机器校验 |
| 门禁 | 无 | `check-native-capability-boundaries.mjs`（NATIVE-01..07 + 8 夹具 + self-test） |

> 本轮**不**做大搬迁（避免高风险重写 Rust 边界）；先使 native 从盲区变为**可机器判定**。物理拆分列为 FUTURE 债务。
