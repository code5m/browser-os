# Git（Git 版本控制）

> Pilot README（Phase 1 文档样板 · 中等域）。
> 证据来源：`src/capabilities/git/{index,public,manifest}.ts`、`capability-registry/capabilities.yaml`、
> `semantic-registry/owners.yaml`（注：git 的 owner `useGitStore` **未进** owners.yaml，仅登记于 capabilities.yaml；
> 同 workspace 子域的 `repo` owner `useRepoStore` 已进 owners.yaml，但 git 版本操作本身不是独立 owner 条目——与 skill 同口径）。

## 1. Purpose
提供仓库状态 / diff / 提交历史 / 提交，以及写操作的双阶段闸门（request → confirm）。写操作 spawn git 子进程执行。

## 2. Domain Classification
SUPPORTING_DOMAIN（开发者工具）。

## 3. Responsibilities
- 经通用 Contribution Registry 向 `RepoPanel` 贡献 UI（`REPO_SUBVIEW` 槽，`GitPanel`）。
- 只读：`git.status` / `git.diff` / `git.log`。
- 写：`git.commit` / `git.write`（均经双阶段闸门）。
- 凭据经 `credential`（keyring）访问，前端零明文。

## 4. Non-Responsibilities
- **不拥有** repo 配置/同步（`useRepoStore` 职责，属 Workspace 子域；REPO context ≠ GIT operation）。
- 不直连文件系统写逻辑之外的业务状态；不持有凭据值。

## 5. Ubiquitous Language
- RepoPanel：仓库主视图容器（Workspace 提供），Git 仅贡献其中「状态」页签。
- GitWriteConfirmDialog：写操作确认对话框，经 `public.ts` 暴露给 Shell 全局挂载（避免 Shell 直连 `ui/` 内部）。
- useGitStore：Git 语义 owner。

## 6. Domain Model
NOT_APPLICABLE（Git 操作以命令/子进程为单位，无领域 Aggregate）。

## 7. Invariants
- INV-GIT-1：写操作必须双阶段闸门（request → confirm），禁止静默直写。
- INV-GIT-2：Git absent → `REPO_SUBVIEW` 槽为空 → `RepoPanel` 的 git 页签不渲染 `GitPanel`。
- INV-GIT-3：凭据仅经 `credential` owner，前端不持原始 token（`owners.yaml` credential 越界规则）。
- ENFORCED_BY：`check-developer-owners.mjs`（DEV-* 机器固化）、`contributionRegistry`。

## 8. State Ownership
- STATE：`useGitStore`（状态/差异/日志/提交草稿/确认态）。
- OWNER：`useGitStore`（**仅登记于 capabilities.yaml，未进 semantic-registry/owners.yaml**；与 skill 同口径）。
- WRITER：Git 能力包内（ui/ + store）。
- PERSISTENCE：disk，非敏感（token 走 keyring）。

## 9. Commands / Intents
- `git.status` / `git.diff` / `git.log` / `git.commit` / `git.write`（provides 声明）。

## 10. Queries
- 经 `useGitStore` 读状态/差异/历史。

## 11. Events
NONE（当前无跨边界事件总线）。

## 12. Public Contract
- 入口：`src/capabilities/git/public.ts`（`export * from ../../stores/useGitStore` + `GitWriteConfirmDialog`）。
- 调用方经此消费，不得 import `state/*` 或 `ui/` 内部。

## 13. Internal Boundary
- 禁止外部 import：`src/capabilities/git/ui/*`（除经 `public.ts` 暴露的 `GitWriteConfirmDialog`）、`src/stores/useGitStore.ts` 物理路径。
- 允许：`public.ts`。

## 14. Dependencies
- Required：`credential`（keyring）、`workspace`（RepoPanel 宿主）、`bridge`。
- Optional：无。

## 15. Dependents
- Workspace `RepoPanel`（经 `REPO_SUBVIEW` 槽）。

## 16. Frontend Boundary
- UI 表达 Git 状态、发送意图；`index.ts` 仅注册贡献。
- 写确认对话框全局挂载，不直连内部。

## 17. Native / Backend Boundary
- 后端：`git_status` / `git_diff` / `git_log` / `git_commit` / `git_write` 等（Rust）。
- 写操作 spawn `git` 子进程执行（PROCESS 资源，owned，证据 `scripts/measure-resources.mjs`）。
- Native 归属详见 `docs/architecture/native-boundary/native-commands.yaml`。

## 18. Resources
- class：MEDIUM, NETWORK（声明式，见 `resources.yaml`）。
- PROCESS 资源：`owned`（measure-resources.mjs 测量）。

## 19. Side Effects
- `fs.read` / `fs.write` / `keyring.access` / `network.connect`（spawn git）。

## 20. Permissions / Security
- permissions：`fs.read` / `fs.write` / `keyring.access`。
- 安全：凭据经 keyring，前端零明文。

## 21. Persistence
- scope：disk；sensitive：false（token 不在本能力持久化）。

## 22. Failure Model
- 写操作：双阶段闸门确保「确认前不落地」，避免部分写入。
- 停用：deactivationPolicy **graceful**（须先处理进行中写任务，不得静默摘除）。
- UI 失败：异步 `GitPanel` 加载/错误态。

## 23. Capability Absence
- Git 未注册 → `REPO_SUBVIEW` 槽为空 → `RepoPanel` 不渲染 Git 页签（能力缺席即无面板）。

## 24. Runtime Lifecycle
- register：`onActivate` 调 `registerGitContributions`。
- HP：**HP0（STATIC）**（manifest `v1.hotPlug.level`，理由：写操作需双阶段闸门，未验证 absent 无残留前不宣称 HP1）。
- activationPolicy：auto；deactivationPolicy：graceful。

## 25. UI Contribution
- `REPO_SUBVIEW` 槽（GitPanel 贡献给 RepoPanel）。

## 26. Testing
- contract：`scripts/check-developer-owners.mjs`、`scripts/measure-resources.mjs`。
- 其余：UNVERIFIED。

## 27. Gates
- `scripts/check-developer-owners.mjs`、`scripts/check-capability-registry.mjs`、`scripts/check-native-capability-boundaries.mjs`。

## 28. Review Guide
- PRIMARY：`src/capabilities/git/{index,public,manifest}.ts`、`src/stores/useGitStore.ts`、`ui/GitPanel.vue`、`ui/GitWriteConfirmDialog.vue`。
- SECONDARY：`credential` 公共边界、`workspace` RepoPanel。
- OUT_OF_SCOPE：bridge 实现、git 子进程逻辑（Rust）。

## 29. AI Modification Guide
- 修改前必读：本 README + `manifest.ts` + `useGitStore.ts` + `GitWriteConfirmDialog.vue`。
- 允许改：ui/ 与 store 内部。
- 禁止改：双阶段闸门语义、`public.ts` 契约（需 SCR）。
- 改后必跑：`npm run check`。

## 30. Known Debt
- BLOCKING：无。
- NON_BLOCKING：HP0（写闸门未验证 absent 无残留）；`mainView='repo'` 导航项仍硬编码于 LayoutStore/Home（未贡献驱动，见 database 同类注记）。
- FUTURE：验证 absent 后可升 HP1。

## 31. Physical Modularity
M1（DIRECTORY_ISOLATED）。

## 32. Reviewability
RV1（owner 单一 + checker 固化）；未达 RV2（无历史 README）。

## 33. Extraction Readiness
DIRECTORY_READY（单一 owner + 窄契约 + 目录隔离 + checker 固化）；未达 PACKAGE_READY（缺 README/测试面实证）。

## 34. Package Extraction Notes
待 D3 + 测试面 + review surface。

## 35. Repository Extraction Notes
不适用。

## 36. Architecture Decisions
- Capability Library Expansion v1：Git 从 Workspace 内嵌面板升格为独立 Capability，经 `REPO_SUBVIEW` 槽贡献（避免 workspace→git 反向依赖环）。
- 参考：`capability-registry/capabilities.yaml` git 条目。

## 37. Related Documentation
- `docs/architecture/domain/DOMAIN-INVENTORY.md`（git 行）
- `docs/architecture/domain/CONTEXT-MAP.md`（CTX-GIT）
- `docs/architecture/modularity/*-MATRIX.md`

## 38. Source of Truth
- 机器真源（运行时）：`src/capabilities/git/manifest.ts`（被 `catalog.ts` 直接 import，单一运行时真源；`capability-registry/capabilities.yaml` 仅为门禁校验用的声明镜像，**运行时零引用**）。
- 语义 owner 真源：`semantic-registry/owners.yaml` —— **git 的 `useGitStore` 不在其中**（仅 capabilities.yaml 登记；勿误读为已固化治理）。
- Native 边界：`docs/architecture/native-boundary/native-commands.yaml`。
- **漂移警示**：`capabilities.yaml` 缺 `hotPlug` 字段（manifest `v1.hotPlug.level:HP0`），且 git `activatable:true` 两处一致；注册表为声明镜像、与 manifest 待同步；运行时以 manifest 为准。
