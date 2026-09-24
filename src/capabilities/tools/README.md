# tools 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/tools/manifest.ts`。
> 诚实边界：工具子 webview 归 Rust 侧按需创建，零能力隔离（门禁 `TOOL_CAPABILITY_LEAK`/`SEED_CAPABILITY_LEAK`）；种子离线、零外链。

---

## 1. Purpose
工具箱域。负责内置/用户工具的列举与打开（每个工具以独立子 webview 运行）。

## 2. Domain Classification
- 领域：`tools`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 工具列表加载（`load` → `bridge.listTools`）。
- 工具打开（`open` → `bridge.openTool`，独立子 webview，`label=tool-<id>`）。
- 工具来源分类（`builtin` / `user` computed）。

## 4. Non-Responsibilities
- 不负责工具内部业务逻辑（工具自身在子 webview 内）。
- 不负责其它域。
- 不为工具子 webview 提供能力隔离（门禁记录为 leak 风险）。

## 5. Ubiquitous Language
- `ToolMeta`：工具元数据（来自 `src/types.ts`）。
- `builtin` / `user`：工具来源分类（按 `source`）。
- `MAX_USER_TOOL_BYTES`：用户工具体积上限（2MiB）。

## 6. Domain Model
- 聚合根：工具列表（`useToolsStore` 管理，pinia id `tools`）。
- 关键 state：`tools` / `error`；computed：`builtin` / `user`（`src/capabilities/tools/state/useToolsStore.ts`）。

## 7. Invariants
- `openTool` 必须经 `validate_user_tool_path` 越权防御（防路径穿越）。
- 工具子 webview 零能力隔离（门禁 `TOOL_CAPABILITY_LEAK`/`SEED_CAPABILITY_LEAK` 记录）。
- 种子工具离线、零外链。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/capabilities/tools/state/useToolsStore.ts`（`defineStore("tools")`，从组件内状态抽离，STAGE H）。
- **semanticOwner**：`useToolsStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：无物理债务（新建 store，原无 owner）；无 MULTIPLE_WRITERS。

## 9. Commands / Intents
- 业务 action：`load` / `open`。
- 原生命令（见 §17）：`list_tools` / `open_tool`。

## 10. Queries
- `list_tools`（原生，只读）。
- 前端内存：`builtin`/`user` 派生。

## 11. Events
- NOT_APPLICABLE。

## 12. Public Contract
- 入口：`src/capabilities/tools/public.ts`。
- 暴露：`useToolsStore`（再导出）、`toolsManifest`、`type ToolMeta`。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/useToolsStore.ts` / `ui/ToolBox.vue`。

## 14. Dependencies
- `dependsOn: ["bridge"]`（硬）。
- `optionalDependencies: []`。

## 15. Dependents
- **无**（仅自消费；UI 经 Contribution Registry 到达 Shell）。

## 16. Frontend Boundary
- 贡献组件：`ToolBox.vue`（WORKBENCH_MAIN，view=`tools`）。
- 注册：`registerToolsContributions()`，懒加载。
- **CURRENT PHYSICAL LOCATION**：UI 在 `src/capabilities/tools/ui/`（已隔离）。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/tools.rs`（`list_tools`/`open_tool`，`WebviewWindowBuilder` label=`tool-<id>`，`tool://` 协议，`validate_user_tool_path`，`MAX_USER_TOOL_BYTES=2MiB`）。
- 已注册（main.rs）：`bridge::list_tools`/`open_tool`。
- 前端封装：`src/bridge.ts`（`listTools`/`openTool`）。
- **诚实声明**：Native 仍集中于 `tools.rs`/`bridge.rs`，未物理模块化；工具子 webview 由 Rust 侧创建，前端无独立生命周期。

## 18. Resources
- `resources.class: ["MEDIUM","WEBVIEW"]`；`suspendable: true`；`destroyable: true`。
- `v1.resources: [{kind:"WEBVIEW", owned, evidence:"tools.rs:open_tool"}]`（工具子 webview 由 Rust 侧按需创建/聚焦）。
- `persistence.scope: "runtime_only"`；`sensitive: false`。

## 19. Side Effects
- 创建/聚焦工具子 webview（`open_tool`）。

## 20. Security
- `validate_user_tool_path` 越权防御（防路径穿越）。
- 种子工具离线、零外链（`SEED_CAPABILITY_LEAK` 门禁）。

## 21. Persistence
- 声明 `runtime_only`；无磁盘持久化（工具列表来自后端枚举，用户工具路径受校验）。

## 22. Failure Model
- 列表失败：`error` 态 + 空列表。
- 打开失败：toast/error（路径校验失败）。

## 23. Capability Absence
- Absent 时：`registerToolsContributions` 未执行 → WORKBENCH_MAIN 无 view=`tools` → MainArea 不渲染。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","SUSPENDED"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`deactivationPolicy: graceful`。

## 25. UI Contributions
- `tools.main.panel`（WORKBENCH_MAIN / surface / view=`tools` / ToolBox）。

## 26. Testing
- `src/capabilities/tools/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 门禁：`scripts/check-tools-policy.py`、`scripts/check-seed-tools.py`。

## 27. Gates
- `scripts/check-tools-policy.py`（**强**，M2-7/M2-8：ToolMeta 结构、`ToolSource` 枚举、`include_str!` 种子嵌入≥5、`list_tools` 只读、`open_tool` 注册、路径越权防御、`TOOL_CAPABILITY_LEAK`/`SEED_CAPABILITY_LEAK` 隔离门禁）。
- `scripts/check-seed-tools.py`（种子完整性）。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/useToolsStore.ts` → `ui/ToolBox.vue` → 后端 `tools.rs`。
- 关注点：工具子 webview 零能力隔离、种子离线、absence 门禁缺失。

## 29. AI Modification Guide
- 改工具加载：必须保持 `validate_user_tool_path` 与 `TOOL_CAPABILITY_LEAK` 门禁，并同步 `check-tools-policy.py` 断言。
- 禁止：用裸 `invoke`、把 store 移出时不同步 manifest、新增 UI 不进 Contribution Registry。
- 新增种子工具：须 `include_str!` 嵌入并过 `check-seed-tools.py`。

## 30. Known Debt
- 工具子 webview 零能力隔离（`TOOL_CAPABILITY_LEAK`/`SEED_CAPABILITY_LEAK` 门禁记录）。
- 非 C3 缺口：无 absence 门禁 + `mainView='tools'` 导航硬编码 + 工具子 webview 归 workspace 工具目录（manifest 注释）。

## 31. C / HP / M / RV / D
- **C = C2**：`manifest.v1.maturity="C2"`，有强 tools checker；自述非 C3（absence 门禁 + nav 硬编码 + 子 webview 归 workspace）。
- **HP = HP0**：`manifest.v1.hotPlug.level="HP0"`（全 false）；子 webview 由 Rust 侧创建，前端无独立生命周期。
- **M = M1（完全）**：`src/capabilities/tools/` 目录隔离（state/ui 均在包内，新建 store）。无独立 npm 包（非 M2）。
- **RV = RV1 + RV2（强） + RV3（否）**：owner 已登记（RV1）；`check-tools-policy.py` + `check-seed-tools.py` 定向覆盖（RV2 强）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：无 vitest、absence 门禁缺失、工具子 webview 零能力隔离（需补隔离或明确为设计约束）。
- 物理隔离完备，可作 Package Extraction 范本候选。

## 33. Source of Truth
- manifest：`src/capabilities/tools/manifest.ts`
- public：`src/capabilities/tools/public.ts`
- state：`src/capabilities/tools/state/useToolsStore.ts`
- UI：`src/capabilities/tools/ui/`
- native：`src-tauri/src/tools.rs`、`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useToolsStore"`
- gates：`scripts/check-tools-policy.py`、`scripts/check-seed-tools.py`（见 §27）
