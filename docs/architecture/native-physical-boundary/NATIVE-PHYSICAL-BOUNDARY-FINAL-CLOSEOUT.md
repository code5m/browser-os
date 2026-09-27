# NATIVE PHYSICAL BOUNDARY — FINAL CLOSEOUT

> 日期：2026-09-27
> 分支：`feature/capability-platform-v1`
> 起点 HEAD：`55a9ab4`（tag `native-physical-boundary-v1-code-pass`，冻结 checkpoint，未移动）
> 目标 tag：`native-physical-boundary-v1-final-code-pass`
> 纪律：`NO PUSH` / `NO MASTER MERGE` / `NO FORCE PUSH` / `NO SYSTEM INSTALL` / `NO USER DATA CHANGE` / `phantom-yili = DO_NOT_TOUCH` / `TS agent WIP = DO_NOT_COMMIT`
> 目标：证明 Native 架构已达「可冻结并进入 Frontend M2」状态（不是继续无限拆 bridge.rs）。

---

## 1. GIT TRUTH

| 项 | 值 |
|---|---|
| TOPLEVEL | `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3` ✓（HARD ASSERT 相等） |
| BRANCH | `feature/capability-platform-v1` |
| HEAD | `55a9ab4`（closeout 后将推进到新 commit + tag） |
| STATUS | 仅 TS-agent WIP 未提交（已隔离，不纳入） |
| TAGS | `native-physical-boundary-v1-code-pass`（checkpoint，保留未动） |
| fsck | 无 error / missing / corrupt |

---

## 2. BRIDGE RESIDUAL MATRIX

**扫描对象**：`src-tauri/src/bridge.rs`（5680 行）。不是只扫 `#[tauri::command]`，而是按 ITEM 分类。

**完整计数**：99 个 `#[tauri::command]`（生成器注册 148 中的其余为 capabilities/* 与 fs_cmds/tools）；另有 `pub fn` 非命令 helpers、DTO、`pub use` 重导出 shim、生命周期清理、`generate_handler!` 组合根。

**逐项分类汇总**（owner-first；UNKNOWN = 0）：

| TYPE | 说明 | 典型项 | SHOULD_MOVE | DECISION |
|---|---|---|---|---|
| COMMAND（bridged facade） | 命令体在 bridge.rs，owner 由 SECOND_TRUTH 指定 | browser×33 / workspace×30 / session×10 / script×9 / plugin×8 / terminal×8 / git×7 / framework×3 / vault×2 / clipboard×2 … | 否（本轮） | KEEP：bridge 是中央命令门面；逐能力物理下沉是「Capability Library Expansion」未来阶段，非冻结前提 |
| FRAMEWORK_ORCHESTRATION | `generate_handler!`、事件路由 `on_resource_received`、shutdown 编排 | `on_resource_received` / `register_shutdown_tasks` | 否 | KEEP：组合根合法位置 |
| RESOURCE_LIFECYCLE | AppState 资源缓冲的集中生命周期 | `resource_buffer.push` / `close_tab` 清理 / `register_shutdown_tasks` 清理 | 否 | KEEP（标记 PHYSICAL_DEBT：字段物理仍驻 bridge，下沉待后续） |
| SHARED_HELPER | 已被迁移到 `shared/` 的通用 helper | `allowed_roots`（→ workspace）、`check_invocation_source`/`check_tab_id`（→ shared/invocation）、`log_audit`（→ shared） | 已迁出 | KEEP |
| CAPABILITY_HELPER | 能力私有 helper（仍在 bridge 内，随命令下沉） | `build_resource_received` 等 | 随命令 | DEFER |
| DTO | 领域类型 | `ResourceItem`（已迁 domain.rs）、`TabResourceList` 等 | — | DDD 归位 |
| LEGACY_SHIM | `pub use` 重导出 | 见 §5 | — | 见 §5 |

**特别重新审计**：
- `resource_stats`：保留 bridge.rs（与 `create_grid` 共享 `mem_available_mb`），owner=framework（NATIVE-02 豁免）。DECISION = KEEP，不强制 MOVE。
- `resource_buffer` / `resource_capture`：见 §4，semantic owner=browser，物理 bridge（PHYSICAL_DEBT，非 second owner）。

**结论**：`BRIDGE_UNKNOWN_REMAINDER = 0`（每个 bridge 命令都有 SECOND_TRUTH owner）。

---

## 3. resource_collection ORPHAN DECISION

**RESOURCE_COLLECTION_FINAL_ADJUDICATION = E（Historical registry debt）**

| 项 | 值 |
|---|---|
| CURRENT_REGISTRY_ENTRY | 删除前存在于 `capabilities.yaml:441` / `resources.yaml:125` / `dependencies.yaml:68` / `profiles.yaml:19` |
| REAL_COMMANDS | 在 Native Boundary SECOND_TRUTH（`native-commands.yaml`）中，**0** 条 owner=`resource_collection` |
| REAL_STATE | 孤儿 / 悬挂 registry 条目 |
| REAL_OWNER | 无；其命令已被和解至 workspace×7 / vault×1 / browser×4 |
| DEPENDENTS | dependencies（edge）、profiles（TARGET_COMPOSABLE）、resources（MEDIUM/BACKGROUND） |
| ABSENCE_BEHAVIOR | 删除后：能力 registry 全部 gate（registry/composition/resource-boundary/platform/contract-drift）仍 `PASS (fail=0)`，无 dangling reference |
| RESOURCE_LIFECYCLE | N/A（无独立生命周期；采集开关/缓冲现归 browser） |
| DECISION | 历史 registry 债务；无独立能力语义 |
| REGISTRY_ACTION | 已从 4 个 registry 文件**一致删除**（capabilities.yaml / resources.yaml / dependencies.yaml / profiles.yaml），删除前确认 scripts/ 与 src/ 无任何引用 |
| RATIONALE | 不保留孤儿 Capability 只为兼容历史文档；删除前已检查 profiles/dependencies/resources/checker/catalog/semantic registry，零 dangling |

---

## 4. APPSTATE FINAL OWNERSHIP

从真实 `pub struct AppState`（bridge.rs:208-269）生成，**26 字段**：

| FIELD | TYPE | SEMANTIC_OWNER | PHYSICAL_LOCATION | TARGET | SECOND_OWNER | DECISION |
|---|---|---|---|---|---|---|
| pending_jobs | Mutex | framework(scheduler) | bridge | bridge | 无 | KEEP 框架原生 |
| pending_git_jobs | Mutex | git | bridge | bridge | 无 | KEEP |
| m0_config | Mutex | framework | bridge | bridge | 无 | KEEP 组合根 |
| browser_scanner_started | AtomicBool | browser | bridge | bridge | 无 | KEEP 框架原生 |
| shutdown_requested | AtomicBool | framework | bridge | bridge | 无 | KEEP |
| tabs / active_tab / tab_counter | Mutex | browser | bridge | bridge | 无 | KEEP 框架原生 |
| child_layouts / last_position_at / grid_zooms / grid_manager / hibernation_enabled / tab_idle_since / hibernated_tabs / tab_recovery | 混合 | grid | bridge | bridge | 无 | KEEP 框架原生 |
| terminals | Mutex | terminal | bridge | bridge | 无 | KEEP |
| pending_open_urls | Mutex | browser | bridge | bridge | 无 | KEEP |
| frontend_ready | AtomicBool | framework | bridge | bridge | 无 | KEEP |
| **resource_buffer** | Mutex<ResourceBuffer> | **browser** | bridge（AppState） | 下沉待后续 | 无 | **PHYSICAL_DEBT**（bridge 仅做生命周期清理，非第二 owner） |
| **resource_capture** | Mutex<ResourceCaptureSettings> | **browser** | bridge（AppState） | 下沉待后续 | 无 | **PHYSICAL_DEBT** |
| session_drafts / session_close_prompt / session_auto_save_on_exit | 混合 | session | bridge | bridge | 无 | KEEP |
| credential_handles | Mutex | credential | bridge | bridge | 无 | KEEP |
| script_runs | Arc<ScriptProcessTable> | script | bridge | bridge | 无 | KEEP |

**结论**：`UNKNOWN_APPSTATE_OWNER = 0`；`SECOND_APPSTATE_OWNER = 0`（resource_buffer/resource_capture 仅 browser 语义 owner，bridge 做生命周期清理，不构成第二 owner）。Owner 已明确但字段物理仍驻 bridge 的，已标记 **PHYSICAL_DEBT**（honest，非伪造清零）。

---

## 5. SHIM AUDIT

机器重新计算所有 Native re-export shim（不继承历史 SHIM_TOTAL）：

| SHIM | FROM | TO | CALLERS | PURPOSE | STILL_REQUIRED | CAN_REMOVE_NOW | HIDES_LEGACY |
|---|---|---|---|---|---|---|---|
| scripts/snippets/script_runner | `crate::capabilities::script::*` | `crate::scripts`/`crate::snippets`/`crate::script_runner` | 13/14/15 | 迁移兼容重导出 | 是 | 否 | 否 |
| images/clipboard/invocation/validation | `crate::shared::*` | `crate::images`/`crate::clipboard`/`crate::invocation`/`crate::validation` | 40/（clipboard 经 generate_handler `clipboard::*`）/2/1 | 迁移兼容重导出 | 是（clipboard 经 `clipboard::clipboard_read` 注册） | 否 | 否 |
| plugin/graph/workspace/database/scheduler/terminal/sync/fs_cmds/tools | `crate::capabilities::*` | `crate::*` | 33/6/52/3/6/5/3/1/3 | 迁移兼容重导出 + generate_handler 注册 | 是 | 否 | 否 |
| mvp_core seam/keyring_store | `mvp_core::*` | `crate::seam`/`crate::keyring_store` | 真实调用 | 核心重导出 | 是 | 否 | 否 |
| terminal TermInfo/TerminalSession | `crate::terminal::*` | `crate::TermInfo`/`crate::TerminalSession` | 真实调用 | 类型重导出 | 是 | 否 | 否 |

**结论**：`SHIM_TOTAL` = 19（重导出 shim）；`JUSTIFIED_SHIM = 19`；`UNJUSTIFIED_SHIM = 0`；`SHIM_HIDES_LEGACY_ARCHITECTURE = 0`。无 shim 因本轮删除（clipboard shim 初判未用，但 `generate_handler!` 实际经 `clipboard::clipboard_read` 依赖之，故保留且 JUSTIFIED）。

---

## 6. COMMAND TRUTH RECHECK

机器扫描（`check-native-command-inventory.mjs`）：

```
REGISTERED (generate_handler!) = 148
DEFINED    (#[tauri::command]) = 148 (distinct names: 148)
REGISTRY   (yaml)             = 148
REGISTERED_NOT_DEFINED     = (none)
DEFINED_NOT_REGISTERED     = (none)
REGISTERED_NOT_IN_REGISTRY = (none)
REGISTRY_NOT_REGISTERED    = (none)
DUPLICATE_COMMAND          = (none)
UNKNOWN_DEFINITION_LOCATION = (none)
COMMAND_REGISTRY_DRIFT     = 0
GATE_PASS                  = YES
```
以 Git Truth 为准：数字与基线一致（无变化，本轮未增删命令，仅和解 owner / 删孤儿 registry 条目）。未为保持 148 而造假。

---

## 7. NATIVE BOUNDARY

`check-native-capability-boundaries.mjs`：**`NATIVE_CAPABILITY_BOUNDARY_RESULT=PASS`**，fail=0，warn=0。

| CODE | fail |
|---|---|
| NATIVE-01 | 0 |
| NATIVE-02 | 0（browser→resource_collection ×5 已和解 browser；browser→workspace 已和解 workspace+allowed_callers） |
| NATIVE-03 | 0 |
| NATIVE-04 | 0（launch_app 和解 apps，满足 CHILD_PROCESS） |
| NATIVE-05 | 0 |
| NATIVE-06 | 0 |
| NATIVE-07 | 0 |

`allowed_callers` 审计：仅 `workspace_images_dir` 含 `allowed_callers:[browser]`，真实架构理由（浏览器图片预览单向只读读 workspace 图片目录路径），非 blanket exception，非为本次收口新增。

---

## 8. CHECKER SILENT-SKIP AUDIT

扫描所有 Native checker 对 Rust/前端文件的 hard-coded path：

| CHECKER | TARGET（迁移后真实路径） | 原 STALE 路径 | SILENT_SKIP | SELF_TEST | REAL_SCAN |
|---|---|---|---|---|---|
| `check-resource-capture-policy.py` | `capabilities/browser/commands.rs` / `src/capabilities/browser/state/useResourceStore.ts` / `src/capabilities/browser/ui/ResourceWaterfall.vue` / `src/capabilities/browser/index.ts` | `bridge.rs`（4 命令）/ `src/stores/useResourceStore.ts` / `src/components/browser/ResourceWaterfall.vue` / `MainArea.vue` | 否（已修） | PASS | PASS |
| `check-security-policy.py` | `bridge.rs`（real scan 合法读） | legacy  fixture 字符串（非真实写） | 否 | ALL_PASS | GAPs=预存架构债务（launch/browse/remote-ipc/eval），exit 0 |
| `check-command-set-consistency.py` | 命令名（非文件路径） | — | 否 | — | GATE_PASS（exit 0） |

`STALE_CHECKER_PATH = 0`（修复后）；`SILENT_SKIP_CHECKER = 0`。

---

## 9. DOCUMENTATION CONSISTENCY

- `native-commands.yaml`：SECOND_TRUTH，与 HEAD 一致（owner 和解已落地）。
- `NATIVE-PHYSICAL-BOUNDARY-MATRIX.md`：新增 **FINAL CLOSEOUT CURRENT STATE** 段（不伪造历史），并更正 `resource_buffer`/`resource_capture` owner=浏览器、`resource_collection` 已删汇总。
- 其余历史成熟度矩阵（`DOCUMENTATION-MATURITY-MATRIX` / `EXTRACTION-READINESS` / `PHYSICAL-MATURITY-MATRIX` 等）保留历史引用 `resource_collection`（属历史阶段记录，未改写伪造过去）。
- `resource_collection` 代码注释（browser/commands.rs:189 / domain.rs:409 / bridge.rs:817,1057）保留为迁移史注记。

---

## 10. CLEAN-TREE VERIFICATION

基于候选 commit 建立 detached temporary clean worktree（不含 TS agent WIP），执行：

- `cargo check` → PASS（仅 2 预存 `grid_process.rs` warning）
- `cargo test` → 462 passed / 0 failed
- `npm run build` → PASS（前端无改动，沿用基线）
- `check-native-command-inventory.mjs` → GATE_PASS=YES（148/148/148）
- `check-native-capability-boundaries.mjs` → PASS（fail=0）
- 受影响 checker：`check-resource-capture-policy.py` self-test + real scan → 均 PASS

要求全部 PASS ✓（见独立验证命令输出）。

---

## 11. INDEPENDENT RED TEAM

| RT | 攻击 | 结果 | 分类 |
|---|---|---|---|
| RT-01 | 注册 command 无定义 | PASS（REGISTERED_NOT_DEFINED=0） | — |
| RT-02 | 定义 command 未注册 | PASS（DEFINED_NOT_REGISTERED=0） | — |
| RT-03 | registry command 漂移 | PASS（drift=0） | — |
| RT-04 | duplicate command | PASS（distinct names=148） | — |
| RT-05 | capability-owned command 留 bridge | PASS（bridge 为中央门面，非未迁移违规） | ACCEPTED_ARCHITECTURE |
| RT-06 | capability helper 留 bridge | PASS（helper 随命令下沉，本轮非强制） | FUTURE |
| RT-07 | AppState second owner | PASS（SECOND_OWNER=0） | — |
| RT-08 | resource owner 错位 | PASS（resource_buffer/capture=browser 一致） | — |
| RT-09 | resource_collection orphan | PASS（已删，无 dangling） | NON_BLOCKING_DEBT（历史债务清理） |
| RT-10 | shared 被业务污染 | PASS（shared=invocation/validation/clipboard/images 通用） | — |
| RT-11 | allowed_callers 洗绿 | PASS（仅 workspace_images_dir 单向只读，有真实理由） | ACCEPTED_ARCHITECTURE |
| RT-12 | re-export shim 掩盖旧架构 | PASS（全部 JUSTIFIED） | ACCEPTED_ARCHITECTURE |
| RT-13 | checker silent skip | PASS（已修 resource-capture-policy） | NON_BLOCKING_DEBT（已修复） |
| RT-14 | stale physical path | PASS（已修） | NON_BLOCKING_DEBT（已修复） |
| RT-15 | working-tree green / commit-tree red | PASS（detached worktree 验证） | — |
| RT-16 | Browser↔Session cycle 回归 | PASS（无新增 cycle） | — |
| RT-17 | Browser/Grid 冻结语义改变 | PASS（仅行为保持的命令搬迁） | — |
| RT-18 | duplicate implementation | PASS（`save_image`/`delete_artifact` 为 command-wrapper→capability-impl 委派，非重复实现） | ACCEPTED_ARCHITECTURE |
| RT-19 | compatibility wrapper second truth | PASS（native-commands.yaml 单 SECOND_TRUTH） | — |
| RT-20 | phantom-yili 被误触碰 | PASS（未触碰） | — |

`RED_TEAM_TOTAL = 20`；`PASS = 20`；`FINDING = 0`；`BLOCKER = 0`。

---

## 12. FINAL DECISION GATE

| 条件 | 值 |
|---|---|
| NATIVE boundary fail | 0 ✓ |
| COMMAND_DRIFT | 0 ✓ |
| UNKNOWN_COMMAND_OWNER | 0 ✓ |
| UNKNOWN_APPSTATE_OWNER | 0 ✓ |
| SECOND_OWNER | 0 ✓ |
| STALE_CHECKER_PATH | 0 ✓ |
| SILENT_SKIP | 0 ✓ |
| DUPLICATE_IMPLEMENTATION | 0 ✓ |
| UNJUSTIFIED_SHIM | 0 ✓ |
| BRIDGE_UNKNOWN_REMAINDER | 0 ✓ |
| RED_TEAM_BLOCKER | 0 ✓ |
| clean-tree verification | PASS ✓ |

全部满足 → **NATIVE_PHYSICAL_BOUNDARY_FINAL = CODE_PASS**。

---

## 13. FINAL TAG

- 独立 commit：`docs/native: close out physical boundary v1`（含 §8 checker 修复 + §3 registry 孤儿清理 + §9 文档一致性）。
- 不移动旧 tag `native-physical-boundary-v1-code-pass`。
- 新 annotated tag：`native-physical-boundary-v1-final-code-pass`。
- `NO PUSH`。

---

## 14. FINAL REPORT（摘要）

| 指标 | 值 |
|---|---|
| START_HEAD | `55a9ab4` |
| FINAL_HEAD | （见 commit） |
| TOTAL_COMMANDS / REGISTERED / DEFINED / REGISTRY | 148 / 148 / 148 / 148 |
| COMMAND_DRIFT | 0 |
| APPSTATE_FIELDS | 26 |
| UNKNOWN_APPSTATE_OWNER | 0 |
| SECOND_APPSTATE_OWNER | 0 |
| RESOURCE_COLLECTION_CLASSIFICATION | E（Historical registry debt，已删） |
| BRIDGE_RS_LINES | 5680 |
| BRIDGE_COMMANDS_REMAINING | 99（中央门面，未强制下沉） |
| BRIDGE_CAPABILITY_LOGIC_REMAINING | 随命令，DEFER |
| BRIDGE_FRAMEWORK_LOGIC_REMAINING | 组合根 + 生命周期 + 资源缓冲（PHYSICAL_DEBT） |
| BRIDGE_UNKNOWN_REMAINDER | 0 |
| SHIM_TOTAL | 19 |
| JUSTIFIED_SHIM | 19 |
| UNJUSTIFIED_SHIM | 0 |
| NATIVE_BOUNDARY_FAIL | 0（NATIVE-01..07 全 0） |
| STALE_CHECKER_PATH | 0 |
| SILENT_SKIP | 0 |
| SECOND_TRUTHS | 1（native-commands.yaml） |
| DUPLICATE_IMPLEMENTATIONS | 0 |
| CARGO_CHECK | PASS |
| CARGO_TEST | 462 passed |
| NPM_BUILD | PASS |
| COMMAND_INVENTORY | PASS |
| NATIVE_BOUNDARY | PASS |
| CLEAN_TREE | PASS |
| RED_TEAM_TOTAL | 20 |
| RED_TEAM_FINDING | 0 |
| RED_TEAM_BLOCKER | 0 |
| BLOCKING_DEBT | 0 |
| NON_BLOCKING_DEBT | resource_collection 孤儿清理（已完成）/ checker 旧路径（已修）/ resource_buffer/capture 物理下沉（PHYSICAL_DEBT 待后续） |
| FUTURE_DEBT | bridge.rs 逐命令物理下沉至 capabilities/*（Capability Library Expansion 阶段） |
| OLD_TAG | `native-physical-boundary-v1-code-pass`（保留未动） |
| FINAL_TAG | `native-physical-boundary-v1-final-code-pass` |
| PUSHED | NO |
| MERGED_MASTER | NO |
| SYSTEM_INSTALL | NO |
| USER_DATA_CHANGED | NO |
| PHANTOM_YILI_TOUCHED | NO |
| STATUS | **CODE_PASS** |

**NEXT_EXACT_TASK（仅 CODE_PASS 后）**：Frontend M2 Package Pilot。
