# Workspace（工作区）

> Pilot README（Phase 1 文档样板 · 复杂域）。
> 证据来源：`src/capabilities/workspace/{index,public,manifest}.ts`、`state/*`、`ui/*`、
> `capability-registry/capabilities.yaml`、`semantic-registry/owners.yaml`（Files/Artifact/Repo/Script/Snippet 已收口为独立 owner）。
>
> Workspace 是**聚合能力**：本身为 CORE_DOMAIN 编排器，内部包含 5 个已抽离的语义子域。

## 1. Purpose
提供文件 / 产物 / 仓库 / 脚本 / 命令片段 / 审计六大主视图与文件编辑器，以及浏览器 Dock 文件面板。是平台核心工作区。

## 2. Domain Classification
CORE_DOMAIN（平台核心）；内部子域为 SUPPORTING_DOMAIN 级细分。

## 3. Responsibilities
- 经 Contribution Registry 向 Shell 贡献 7 个 `WORKBENCH_MAIN` 主视图（`files/arts/repo/scripts/commands/audit/editor`）+ 1 个 `BROWSER_DOCK`（`files`）。
- 跨子域编排（如 `openRecent → useFileStore.openFile`）。
- 持有 Workspace Core 状态（跨域协调态）。

## 4. Non-Responsibilities
- **不拥有**文件态（`useFileStore`）、产物态（`useArtifactStore`）、仓库态（`useRepoStore`）、脚本态（`useScriptStore`）、片段态（`useSnippetStore`）——这些为独立语义 owner，Workspace Core 只编排不持第二份。
- **不拥有** git 操作（Git 能力职责）、终端派生（Terminal 能力）、凭据（Credential 能力）。
- 禁止 `useWorkspaceStore` 再声明 Files/Artifact/Repo/Script/Snippet-owned state（owners.yaml `WS_OWNER_*` 越界规则）。

## 5. Ubiquitous Language
- MainArea / ActivityBar / StatusBar / UnifiedTabBar：Shell 容器。
- 子域：Files（文件浏览/编辑/树/预览）、Artifact（成果树/编辑/采集）、Repo（仓库配置/同步）、Script（脚本库 CRUD）、Snippet（命令片段库 CRUD）。
- Contribution/Slot 模型：Shell 遍历 slot 按 view 渲染，不 import 内部。

## 6. Domain Model
- Aggregate Root：Workspace Core（编排者）。
- 子域 Bounded Contexts（同属本能力包，由 `owner_implementations` locator 解析）：
  - Files（`useFileStore`，Phase 8C-0A）
  - Artifact（`useArtifactStore`，Phase 8C-0B）
  - Repo（`useRepoStore`，Phase 8C-0C）
  - Script（`useScriptStore`，Phase 8C-0D）
  - Snippet（`useSnippetStore`，Phase 8C-0D）
- 非 NOT_APPLICABLE：存在明确一致性边界（每个子域为独立 owner）。

## 7. Invariants
- INV-WS-1：子域状态只能由其 canonical owner 写；组件/其它 store 不得直写（`WS_OWNER_01..12`）。
- INV-WS-2：视图导航经 `useLayoutStore.setView`（mainView 唯一真源），组件不得直写 `mainView`。
- INV-WS-3：Workspace absent → 主视图贡献整体摘除 → Shell 不渲染对应视图（不得留死视图）。
- ENFORCED_BY：`check-semantic-registry.mjs`（WS_OWNER_* / COMPONENT_WRITES_*）、`contributionRegistry`。

## 8. State Ownership
- STATE：
  - Workspace Core：`useWorkspaceStore`（跨域协调态）。
  - 子域：`useFileStore` / `useArtifactStore` / `useRepoStore` / `useScriptStore` / `useSnippetStore`（各自唯一真源）。
- OWNER：`useWorkspaceStore` + 5 子域 owner（均已在 Semantic Registry）。
- WRITER：各 owner 包内（均在 `src/capabilities/workspace/state/`）。
- PERSISTENCE：disk，非敏感。

## 9. Commands / Intents
- `workspace.files` / `workspace.artifact` / `workspace.repo` / `workspace.script` / `workspace.snippet` / `workspace.main-view`（provides）。
- 跨域编排意图：`openRecent`、`refresh`、`requestSync` 等（调子域 owner action）。

## 10. Queries
- 经各 owner store 只读消费；Shell 经 `public.ts` 读取。

## 11. Events
NONE（当前无跨边界事件总线）。

## 12. Public Contract
- 入口：`src/capabilities/workspace/public.ts`（re-export 6 个 store：`useWorkspaceStore/useFileStore/useArtifactStore/useRepoStore/useScriptStore/useSnippetStore`）。
- 调用方经此消费，不得 import `state/*` 或 `ui/*` 内部。

## 13. Internal Boundary
- 禁止外部 import：`src/capabilities/workspace/state/*`、`src/capabilities/workspace/ui/*`（除经 Shell 注册渲染）。
- 允许：`public.ts`。

## 14. Dependencies
- Required：无（能力层零硬依赖，利于 absent 可启动）。
- Optional：`browser`（`BROWSER_DOCK` 文件面板依赖浏览器可见态）。
- 注意：`workspace→home` **不**声明依赖，经 `src/composables/homeNav.ts` 窄缝（避免 CB-04 必须依赖环）。

## 15. Dependents
- Shell（App / MainArea / ActivityBar / StatusBar / UnifiedTabBar）经 `WORKBENCH_MAIN` / `BROWSER_DOCK` 槽。
- Home（经 `homeNav` 窄缝消费收藏目录）。

## 16. Frontend Boundary
- UI 表达各子域状态、发送意图；`index.ts` 仅注册贡献，不持状态。
- 7 个主视图 + 编辑器 + Dock 文件面板，均异步懒加载（`defineAsyncComponent` + loading/error 态）。

## 17. Native / Backend Boundary
- 无直接 native 资源；委托 git（Git 能力）/ terminal（Terminal 能力）/ credential（Credential 能力）经窄缝。
- 子域后端命令见各自 store（Files/Artifact/Repo/Script/Snippet 的 Rust 命令）。

## 18. Resources
- class：LIGHT, MEDIUM（声明式，见 `resources.yaml`）；无 native handle 需回收。
- 实测：`NOT_AVAILABLE`（Debt-7A-1）。

## 19. Side Effects
- `fs.read` / `fs.write`（经 Files/Script/Snippet）；无网络/进程由本能力直接派生。

## 20. Permissions / Security
- permissions：无（敏感操作委托 credential/git）。

## 21. Persistence
- scope：disk；sensitive：false。

## 22. Failure Model
- UI 失败：异步组件 `errorComponent` 显示「该面板暂时无法显示」，不白屏。
- 槽渲染模型避免「能力缺失 → 死视图」。

## 23. Capability Absence
- Workspace 未注册 → 无主视图贡献 → Shell 不渲染 files/arts/repo/scripts/commands/audit/editor（核心能力，实践中极少 absent，但模型一致）。

## 24. Runtime Lifecycle
- register：`onActivate` 调 `registerWorkspaceContributions`（注册 7 主视图 + 1 Dock）。
- HP：**HP0（STATIC）**（manifest `v1.hotPlug.level`，理由：需验证主视图贡献整体摘除后 Shell 不残留死视图）。
- activationPolicy：auto；deactivationPolicy：manual。

## 25. UI Contribution
- `WORKBENCH_MAIN`：`files` / `arts` / `repo` / `scripts` / `commands` / `audit` / `editor`。
- `BROWSER_DOCK`：`files`（label「文件」，icon 📂，order 10）。

## 26. Testing
- contract：`scripts/check-capability-pilot.mjs`、`scripts/check-composition-profiles.mjs`、`check-semantic-registry.mjs`（WS_OWNER_*）。
- 其余：UNVERIFIED。

## 27. Gates
- `scripts/check-semantic-registry.mjs`、`scripts/check-capability-pilot.mjs`、`scripts/check-composition-profiles.mjs`、`scripts/check-capability-registry.mjs`。

## 28. Review Guide
- PRIMARY：`src/capabilities/workspace/{index,public,manifest}.ts`、`state/useWorkspaceStore.ts` + 5 子域 store。
- SECONDARY：`ui/*.vue`（10 个面板）、`homeNav` 窄缝。
- OUT_OF_SCOPE：git/terminal/credential 内部、bridge 实现。

## 29. AI Modification Guide
- 修改前必读：本 README + `manifest.ts` + 目标子域 store + 对应 `owners.yaml` 条目。
- 允许改：目标子域 `ui/` 与 `state/` 内部。
- **禁止**：从 `useWorkspaceStore` 写子域 owned state；新增旧 Workspace API 消费点；跨 import 子域内部。
- 改后必跑：`npm run check`（含 WS_OWNER_* 越界机检）。

## 30. Known Debt
- BLOCKING：无。
- NON_BLOCKING：
  - 子域物理抽取仍在进行（`script` 的 entrypoint 实为 `workspace/ui/ScriptPanel.vue`，逻辑 owner 与物理位置分离，M0）；
  - HP0（主视图整体摘除未验证无死视图）；
  - `mainView='repo'` 等导航项仍部分硬编码（未完全贡献驱动）；
  - 子域 store 部分仍驻 `src/stores`（如 useFileStore/useArtifactStore 路径以 locator 解析为准）。
- FUTURE：物理彻底抽离子域 → 升 M；验证 absent 后升 HP。

## 31. Physical Modularity
M1（DIRECTORY_ISOLATED：`src/capabilities/workspace/{manifest,public,index}.ts` + `ui/`(10) + `state/`）。子域物理抽离未完成 → 未达 M2。

## 32. Reviewability
RV1（owner 明确，但 5 子域需额外导航）；未达 RV2（无历史 README）。

## 33. Extraction Readiness
NOT_READY（子域分散、物理未完全抽离、无 README/测试面实证）。子域（如 Files/Artifact）单独可达 DIRECTORY_READY。

## 34. Package Extraction Notes
子域（Files/Artifact/Repo/Script/Snippet）具备独立 owner，是未来拆 package 的候选；需先 D3 + 测试面。

## 35. Repository Extraction Notes
不适用（§40 默认禁止新建仓库）。

## 36. Architecture Decisions
- Phase 8C Train B：Workspace 物理隔离 + 子域 owner 抽取（Files/Artifact/Repo/Script/Snippet）。
- 参考：`capability-registry/capabilities.yaml` workspace 条目、`semantic-registry/owners.yaml` 各子域条目。

## 37. Related Documentation
- `docs/architecture/domain/DOMAIN-INVENTORY.md`（workspace 行）
- `docs/architecture/domain/CONTEXT-MAP.md`（CTX-WORKSPACE）
- `docs/architecture/modularity/*-MATRIX.md`

## 38. Source of Truth
- 机器真源：`capability-registry/capabilities.yaml`、`semantic-registry/owners.yaml`（Files/Artifact/Repo/Script/Snippet/workspace 条目）、`native-boundary/native-commands.yaml`。
- **漂移警示**：`capabilities.yaml` workspace 无 `maturity`，但代码 `manifest.ts v1.maturity:C3` —— 以代码 manifest 为运行时权威，注册表待同步。另：子域 store 物理路径（部分仍驻 `src/stores`）与 `owner_implementations` locator 需以 locator 解析为准。
