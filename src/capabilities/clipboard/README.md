# clipboard 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/clipboard/manifest.ts`。
> 诚实边界（BUG-HUNT 修复）：剪贴板历史**不落盘**（`loadClipHistory` 故意 no-op、`saveClipHistory` 仅内存裁剪），UI 经 `redactSecrets` 脱敏；明文落盘已修复（B11-1 红线）。

---

## 1. Purpose
剪贴板域。负责剪贴板读写、历史记录（仅会话内存）、历史条目复用，以及明文脱敏展示。

## 2. Domain Classification
- 领域：`clipboard`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 剪贴板读取（`clipReadSilent`）/ 写入（`clipCopy` / `clipPaste`）。
- 历史记录（内存 `clipHistory`，上限 `CLIP_CAP=30`），条目复用（`useClipItem`/`copyClipItem`）、清空（`clearClipHistory`）。
- 明文脱敏展示（UI 经 `redactSecrets`）。

## 4. Non-Responsibilities
- **不持久化剪贴板历史到磁盘**（B11-1 红线，明文落盘已修复）。
- 不创建 WebView/PTY/进程/数据库连接。
- 不负责其它域。

## 5. Ubiquitous Language
- `ClipItem`：一条剪贴板历史（来自 `src/types.ts`）。
- `clipText`：当前活动剪贴板文本（编辑区，非历史）。
- `CLIP_CAP`：内存历史上限（30）。
- `redactSecrets`：敏感信息脱敏函数（`src/utils/redact.ts`）。

## 6. Domain Model
- 聚合根：剪贴板历史（`useClipboardStore` 管理）。
- 关键 state：`clipText` / `clipHistory`（`ClipItem[]`）/ `CLIP_CAP`（`src/capabilities/clipboard/state/useClipboardStore.ts`）。
- 面板开合态 `clipOpen` 仍归 `useLayoutStore`（视图态非本域，属债务）。

## 7. Invariants
- `loadClipHistory` 必须为无副作用空函数（不读 localStorage）。
- `saveClipHistory` 仅做内存上限裁剪，**绝不**写 localStorage。
- 历史条目展示必须经 `redactSecrets` 脱敏。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/capabilities/clipboard/state/useClipboardStore.ts`（`defineStore("clipboard")`）。
- **semanticOwner**：`useClipboardStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：物理迁移已完成（从 `useSystemStore` 拆出，解除 Debt-8E-1）；UI 已迁入 `ui/`。残留债务：`clipOpen` 视图态仍归 `useLayoutStore`（视图态非本域）。

## 9. Commands / Intents
- 业务 action：`loadClipHistory`（no-op）/ `saveClipHistory`（内存裁剪）/ `clipReadSilent` / `clipCopy` / `clipPaste` / `useClipItem` / `copyClipItem` / `clearClipHistory` / `bindClipFocus` / `startClipWatch`。
- 原生命令（见 §17）：`clipboard_read` / `clipboard_write`。

## 10. Queries
- `clipboard_read`（原生）。
- 前端内存：`clipHistory` 派生。

## 11. Events
- NOT_APPLICABLE（事件驱动 `bindClipFocus`，无轮询）。

## 12. Public Contract
- 入口：`src/capabilities/clipboard/public.ts`。
- 暴露：`useClipboardStore`（再导出）、`clipboardManifest`、`type ClipItem`。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/useClipboardStore.ts` / `ui/ClipboardPanel.vue`。

## 14. Dependencies
- `dependsOn: ["bridge"]`（硬）。
- `optionalDependencies: []`。

## 15. Dependents
- `src/App.vue`（启动期 `startClipWatch`）、`src/capability/index.ts`、`profiles.ts`、`catalog.ts`。

## 16. Frontend Boundary
- 贡献组件：`ClipboardPanel.vue`（WORKBENCH_MAIN，view=`clip`）。
- 注册：`registerClipboardContributions()`，懒加载。
- **CURRENT PHYSICAL LOCATION**：UI 已在 `src/capabilities/clipboard/ui/`（已确认 `src/components/system/ClipboardPanel.vue` 0 残留，全迁入）。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/bridge.rs`（`clipboard_read` / `clipboard_write`，基于 `arboard::Clipboard`）。
- 已注册（main.rs）：`bridge::clipboard_read` / `clipboard_write`。
- 前端封装：`src/bridge.ts`（`clipboardRead→clipboard_read` / `clipboardWrite→clipboard_write`）。
- **诚实声明**：Native 仍集中于 `bridge.rs`，未物理模块化。

## 18. Resources
- `resources.class: ["LIGHT"]`；`suspendable: true`；`destroyable: true`。
- `v1.resources: []`（无 owned 资源）。
- `persistence.scope: "session"`（不落盘，B11-1 红线）；`sensitive: false`。

## 19. Side Effects
- 写系统剪贴板（`clipboard_write`）。
- 当前**无**写磁盘副作用（历史仅内存）。

## 20. Security
- 明文落盘已修复（B11-1）：历史不进 localStorage。
- UI 经 `redactSecrets` 脱敏展示（防密码/token 明文渲染）。

## 21. Persistence
- 声明 `session`；实际仅内存 `clipHistory`（上限 30），关闭应用清空，**不写入磁盘**。

## 22. Failure Model
- 读取失败：`error` 态；UI 提示。
- 写入失败：toast/error。

## 23. Capability Absence
- Absent 时：`registerClipboardContributions` 未执行 → WORKBENCH_MAIN 无 view=`clip` → MainArea 不渲染。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","SUSPENDED"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`installPolicy: static`。

## 25. UI Contributions
- `clipboard.main.panel`（WORKBENCH_MAIN / surface / view=`clip` / ClipboardPanel）。

## 26. Testing
- `src/capabilities/clipboard/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 门禁：`scripts/check-clipboard-persistence-logic.mjs`（16 断言，验证不落盘，源码级）。

## 27. Gates
- `scripts/check-clipboard-persistence-logic.mjs`（**强**，验证不落盘 + `redactSecrets` 接线）。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/useClipboardStore.ts` → `ui/ClipboardPanel.vue`。
- 关注点：不落盘红线、脱敏接线、`clipOpen` 视图态债务。

## 29. AI Modification Guide
- 改历史逻辑：必须保持不落盘，并补/更新 `check-clipboard-persistence-logic.mjs` 断言。
- 禁止：把历史写回 localStorage、移除 `redactSecrets` 脱敏、用裸 `invoke`。
- `clipOpen` 若迁回本域需同步 `useLayoutStore` 消费者。

## 30. Known Debt
- C2→C3 缺口：`mainView='clip'` 硬编码 + 面板开合态 `clipOpen` 仍归 `useLayoutStore`（视图态非本域）。
- Debt-8E-1 已解除（从 `useSystemStore` 拆出）。
- 无 state/UI 物理债务（已迁入包内）。

## 31. C / HP / M / RV / D
- **C = C2**：`manifest.v1.maturity="C2"`，自述非 C3（无 absence 门禁 + nav 硬编码 + clipOpen 留 layout）；有不落盘强 checker。
- **HP = HP0**：`manifest.v1.hotPlug.level="HP0"`（全 false）；会话内存态，无运行时装卸需求声明。
- **M = M1（完全）**：`src/capabilities/clipboard/` 目录隔离（state/ui 均在包内，旧 `src/components` 0 残留）。无独立 npm 包（非 M2）。
- **RV = RV1 + RV2 + RV3（否）**：owner 已登记（RV1）；`check-clipboard-persistence-logic.mjs` 定向覆盖（RV2）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：无 vitest、absence 门禁缺失、clipOpen 视图态归一。
- 物理隔离完备，可作 Package Extraction 范本候选。

## 33. Source of Truth
- manifest：`src/capabilities/clipboard/manifest.ts`
- public：`src/capabilities/clipboard/public.ts`
- state：`src/capabilities/clipboard/state/useClipboardStore.ts`
- UI：`src/capabilities/clipboard/ui/`
- native：`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useClipboardStore"`
- gates：`scripts/check-clipboard-persistence-logic.mjs`（见 §27）
