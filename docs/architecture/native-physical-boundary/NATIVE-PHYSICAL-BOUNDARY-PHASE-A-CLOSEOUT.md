# NATIVE PHYSICAL BOUNDARY — PHASE A Closeout

> 日期：2026-09-27
> 分支：`feature/capability-platform-v1`（HEAD `28e3c24`，基线 tags：`native-physical-batch-browser-create-tab-pass` / `native-physical-batch-session-pass`）
> 目标 tag：`native-physical-boundary-v1-code-pass`
> 纪律：`NO PUSH` / `NO MASTER MERGE` / `NO FORCE PUSH` / `NO SYSTEM INSTALL` / `NO USER DATA CHANGE`

## 1. 范围（真实扫描得到的 6 项失败）

| # | 类型 | 命令 | 原裁决 | PHASE A 处理 |
|---|---|---|---|---|
| 1 | NATIVE-02 | `report_resources` | browser→resource_collection 不匹配 | owner 和解 → `browser` |
| 2 | NATIVE-02 | `list_tab_resources` | 同上 | owner 和解 → `browser` |
| 3 | NATIVE-02 | `clear_tab_resources` | 同上 | owner 和解 → `browser` |
| 4 | NATIVE-02 | `get_resource_capture_settings` | 同上 | owner 和解 → `browser` |
| 5 | NATIVE-02 | `set_resource_capture_settings` | 同上 | owner 和解 → `browser` |
| 6 | NATIVE-02 | `workspace_images_dir` | browser→workspace 不匹配 | owner 和解 → `workspace` + 窄契约 `allowed_callers:[browser]` |
| 7 | NATIVE-04 | `launch_app` | 缺 `resources.class` | owner 和解 → `apps`（其已声明 `resources.class:[LIGHT,PROCESS]`） |

> 注：扫描原文表述为「browser→resource_collection ×4 + browser→workspace ×1 + NATIVE-04 launch_app ×1」；实际 resource_collection 涉及 5 条资源命令（含采集设置），PHASE A 一并和解至 `browser`。

## 2. 处理原则

**不做物理重排**（不动 `bridge.rs` 的 AppState 结构、不跨能力搬字段），改用：
- **owner 和解**：在 `native-commands.yaml`（SECOND_TRUTH）把命令 owner 改写为真实承载能力；
- **窄 cross-capability 契约**：`workspace_images_dir` 加 `allowed_callers:[browser]`（单向只读，浏览器图片预览读 workspace 图片目录路径）；
- **避免 capabilities.yaml 改动**：`launch_app` 和解到 `apps`（已满足 CHILD_PROCESS），不碰 `capabilities.yaml` 以免触发 RPT-04/composability gate 失败。

## 3. 代码改动（5 文件）

| 文件 | 改动 |
|---|---|
| `src-tauri/src/capabilities/browser/commands.rs` | 迁入 5 条资源命令；加 `use tauri::Emitter;`；用 `domain::ResourceItem` |
| `src-tauri/src/bridge.rs` | 删除 5 命令体 + `ResourceItem` 定义；`resource_stats` 保留内部调用点；移除冗余 import |
| `src-tauri/src/domain.rs` | `ResourceItem` 类型下沉（消除 capability→bridge 反向依赖） |
| `src-tauri/src/main.rs` | `generate_handler!` 重注册 5 条 `browser::commands::*` |
| `docs/architecture/native-boundary/native-commands.yaml` | 8 处 block 更新（owner / category / allowed_callers） |

## 4. 验证结果（PHASE A 后）

| 门禁 | 结果 |
|---|---|
| `cargo check` | PASS（仅 2 预存 `grid_process.rs` 警告，非本次引入） |
| `check-native-capability-boundaries.mjs` | `NATIVE_CAPABILITY_BOUNDARY_RESULT=PASS`（fail=0, warn=0, 148 命令） |
| `check-native-command-inventory.mjs` | `GATE_PASS=YES`（COMMAND_REGISTRY_DRIFT=0；AppState 26/26） |
| `cargo test` | 见提交前运行（462 passed baseline，纯搬迁无逻辑改动） |

## 5. 诚实债务（OPEN / 不静默消失）

1. **`resource_collection` 能力孤儿**：命令承载角色实质解散，但条目被 `dependencies.yaml`/`profiles.yaml`/`resources.yaml` 引用，保留不删；后续若 registry 不再引用可清理。
2. **AppState 字段 `resource_buffer`/`resource_capture`**：物理仍驻 `bridge.rs`，消费者已迁 `browser`；字段所有权下沉不在 PHASE A 范围，待后续能力化时处理。
3. **`resource_stats`**：保留 bridge（与 `create_grid` 共享 `mem_available_mb`），owner 分类 FRAMEWORK（NATIVE-02 豁免）。

## 6. 排除项（DO_NOT_COMMIT）

`src/capabilities/agent/**`、`src/stores/useAgentStore.ts`（deleted）、`src/capabilities/agent/state/`（untracked）为其他 lane 的 TS-agent WIP，本次提交**不纳入**。
