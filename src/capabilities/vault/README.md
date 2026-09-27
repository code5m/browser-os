# vault 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/vault/manifest.ts`。
> 诚实边界：仅 `vault.open` 有原生命令；`vault.search`/`vault.follow` 由本能力包内的 `internal/vault.mjs` 计算（无对应后端）；HP2 由 bookmark 试点背书，vault 已有专属边界门禁 `scripts/check-vault-boundary.mjs`（VB-01..VB-10，含 10 条 self-test）。

---

## 1. Purpose
笔记库域。负责本地笔记目录的打开、全文搜索、链接跟随与笔记图谱（链接图，上限 5000 边）。

## 2. Domain Classification
- 领域：`vault`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 打开笔记目录（`open` → `bridge.vaultOpen`，含 generation 竞态防护）。
- 本地目录选择（`pickDirectory`）。
- 笔记搜索/链接跟随（`searchNotes`/`noteLinks`/`resolveNote`，前端计算）。
- 链接图派生（`vaultEdges`/`vaultBacklinks`，上限 5000 边）。

## 4. Non-Responsibilities
- 不负责远端笔记同步/云存储。
- 不创建 WebView/PTY/数据库连接（除文件读取）。
- 不负责其它域。

## 5. Ubiquitous Language
- `vaultCurrent`：当前笔记。
- `vaultResults`：搜索结果（前端计算）。
- `vaultEdges` / `vaultBacklinks`：链接图 / 反向链接。
- `sourceMode` / `anchor` / `line`：定位态。

## 6. Domain Model
- 聚合根：笔记库（前端内存索引，`useVaultStore` 管理，pinia id `vault`）。
- 关键 state：`path` / `root` / `notes` / `selected` / `query` / `line` / `anchor` / `busy` / `error` / `warning` / `choices` / `sourceMode`（`src/capabilities/vault/state/useVaultStore.ts`）。

## 7. Invariants
- `open` 必须含 generation 竞态防护（避免陈旧结果覆盖新结果）。
- search/follow 由本能力包内的 `internal/vault.mjs` 计算，无后端命令依赖。
- 链接图边上限 5000（防爆炸）。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/capabilities/vault/state/useVaultStore.ts`（`defineStore("vault")`）。
- **CURRENT PHYSICAL LOCATION（搜索逻辑）**：`src/capabilities/vault/internal/vault.mjs`（`noteLinks`/`resolveNote`/`searchNotes`，前端计算）。
- **semanticOwner**：`useVaultStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：无物理债务（state/ui 均在包内）；无 MULTIPLE_WRITERS。

## 9. Commands / Intents
- 业务 action：`open` / `pickDirectory` / `select` / `follow`；computed `vaultCurrent`/`vaultResults`/`vaultEdges`/`vaultBacklinks`。
- 原生命令（见 §17）：仅 `vault_open`（1 个）。

## 10. Queries
- `vault_open`（原生，打开目录）。
- 前端计算：`searchNotes`/`vaultResults`/`vaultEdges`/`vaultBacklinks`（**无对应原生命令**）。

## 11. Events
- NOT_APPLICABLE。

## 12. Public Contract
- 入口：`src/capabilities/vault/public.ts`。
- 暴露：`useVaultStore`（再导出，**无** manifest 再导出、**无**类型导出）。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/useVaultStore.ts` / `ui/VaultPanel.vue`。

## 14. Dependencies
- `dependsOn: ["bridge"]`（硬）。
- `optionalDependencies: ["browser"]`（只读快照可独立展示，无强制依赖）。

## 15. Dependents
- **无**（仅自消费；UI 经 Contribution Registry 到达 Shell）。

## 16. Frontend Boundary
- 贡献组件：`VaultPanel.vue`（WORKBENCH_MAIN，view=`vault`）。
- 注册：因模块加载即调用 `registerVaultContributions()`（不走 onActivate，注释解释：对象未展开 manifest，否则贡献表无 `vault` 条目导致不渲染）。
- **CURRENT PHYSICAL LOCATION**：UI 在 `src/capabilities/vault/ui/`（已隔离）。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/bridge.rs`（`vault_open`）。
- 已注册（main.rs）：`bridge::vault_open`。
- 前端封装：`src/bridge.ts`（`vaultOpen→vault_open`）。
- **诚实声明**：Native 仍集中于 `bridge.rs`，未物理模块化；`vault_search`/`vault_follow` 无原生命令（前端计算）。

## 18. Resources
- `resources.class: ["LIGHT"]`；`suspendable: true`；`destroyable: true`。
- `v1.resources: [{kind:"CACHE", owned, evidence:"vault/state/useVaultStore.ts"}]`（内存快照）。
- `permissions: ["fs.read"]`。

## 19. Side Effects
- 打开本地目录（文件读取）。
- 当前**无**写盘副作用（搜索/链接为前端计算）。

## 20. Security
- `fs.read` 权限受限；仅本地目录读取。
- `open` generation 竞态防护避免陈旧数据。

## 21. Persistence
- 顶层 `persistence.scope: "none"` vs v1 `persistenceScope: "runtime_only"`（**两处口径漂移**，见债务）；实际为内存态（runtime_only），无磁盘持久化。

## 22. Failure Model
- 目录打开失败：`error`/`warning` 态 + UI 提示。
- 搜索失败：前端计算失败，`vaultResults` 回退空。

## 23. Capability Absence
- Absent 时：`registerVaultContributions` 未执行 → WORKBENCH_MAIN 无 view=`vault` → MainArea 不渲染。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","SUSPENDED"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`deactivationPolicy: manual`（笔记库无重资源/无后台任务，手动停用即可）。

## 25. UI Contributions
- `vault.main`（WORKBENCH_MAIN / surface / view=`vault` / VaultPanel）。

## 26. Testing
- `src/capabilities/vault/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 门禁：maturityEvidence 为通用平台/组合门禁（见 §27）。

## 27. Gates
- `scripts/check-capability-platform.mjs`、`scripts/check-capability-composition.mjs`（**通用平台/组合门禁**，全文未出现 `vault`/`笔记库` 字符串）。
- **HP2 能力由 bookmark 试点演示**（`check-capability-platform.mjs` PLT2-14 register/unregister bookmark），vault 已有专属边界门禁 `scripts/check-vault-boundary.mjs`（VB-01..VB-10，含 10 条 self-test）（RV2 间接/弱）。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/useVaultStore.ts` → `ui/VaultPanel.vue` → `src/capabilities/vault/internal/vault.mjs`。
- 关注点：persistence 口径漂移、maturityEvidence 漂移、search/follow 前端计算无后端。

## 29. AI Modification Guide
- 改搜索/链接逻辑：必须同步 `src/capabilities/vault/internal/vault.mjs` 与门禁（若有）。
- 禁止：用裸 `invoke`、把 store 移出时不同步 manifest、新增 UI 不进 Contribution Registry。
- 若需后端化 search/follow，须新增 `vault_search`/`vault_follow` 命令并注册 + ACL + 门禁。

## 30. Known Debt
- **persistence 漂移**：顶层 `persistence.scope="none"` 与 v1 `persistenceScope="runtime_only"` 不一致 —— 无磁盘持久化但内存态属运行时态，两处口径需对齐。
- **maturityEvidence 漂移**：两个证据脚本为通用平台/组合门禁，未特指 vault；HP2 由 bookmark 试点背书，vault 已有专属边界门禁 `scripts/check-vault-boundary.mjs`（VB-01..VB-10，含 10 条 self-test）（RV2 间接/弱）。
- `deactivationPolicy=manual`（与 task/terminal/tools 的 `graceful` 不同，设计如此）。

## 31. C / HP / M / RV / D
- **C = C2**：`manifest.v1.maturity="C2"`；有通用平台/组合门禁；自身无 absence 门禁 + nav 硬编码（非 C3）。
- **HP = HP2**：`manifest.v1.hotPlug.level="HP2"`（enable/disable/register/unregister=true；install/uninstall=false），由 `check-capability-platform.mjs` 验证；但 vault 已有专属边界门禁 `scripts/check-vault-boundary.mjs`（VB-01..VB-10，含 10 条 self-test）（间接证据）。
- **M = M1（完全）**：`src/capabilities/vault/` 目录隔离（state/ui 均在包内）。无独立 npm 包（非 M2）。
- **RV = RV1 + RV2（间接/弱） + RV3（否）**：owner 已登记（RV1）；maturityEvidence 为通用门禁、HP2 由 bookmark 试点背书（RV2 间接/弱）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：无 vitest、persistence 口径漂移、maturityEvidence 漂移、absence 门禁。
- 物理隔离完备，可作 Package Extraction 范本候选（需先消漂移）。

## 33. Source of Truth
- manifest：`src/capabilities/vault/manifest.ts`
- public：`src/capabilities/vault/public.ts`
- state：`src/capabilities/vault/state/useVaultStore.ts`
- UI：`src/capabilities/vault/ui/`
- search logic：`src/capabilities/vault/internal/vault.mjs`
- native：`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useVaultStore"`
- gates：`scripts/check-capability-platform.mjs`、`scripts/check-capability-composition.mjs`（见 §27）
