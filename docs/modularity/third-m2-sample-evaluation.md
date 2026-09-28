# 第三个 M2 Package 样本评估（directive §9）

> 评估目标：在 Generic Capability Package Checker（§8，已交付+tagged `modularity-generic-capability-package-checker-pass`）之后，
> 自动挑选 **LOWEST_RISK + HIGH_INFORMATION_VALUE** 的第三个真实 M2 样本，
> 用于后续 §10 Three-Package Contract Review，验证 shared contracts 是否该抽。
>
> 评估日期：2026-09-28｜评估人：Modularity Train（autonomous）
> 真源：`src/capabilities/*/manifest.ts`（Building Block Contract v1 内联）、`scripts/check-capability-package.mjs` SPECS。

## 0. 评估方法

对候选能力按 9 个维度打分（H=高 / M=中 / L=低），最终取 **风险最低 + 信息价值最高**。

| 维度 | 含义 | 越低越好 / 越高越好 |
|---|---|---|
| BOUNDARY_GAIN | 升格为包后边界清晰度提升 | 高好 |
| AI_REVIEW_GAIN | 可被 AI 审查/自动检查的结构增益 | 高好 |
| HOST_COUPLING | 与 Host 耦合度（高=改动面大） | 低好 |
| PACKAGE_VALUE | 作为独立包的内在价值 | 高好 |
| MIGRATION_COST | 迁移成本（文件数/引用点） | 低好 |
| RESOURCE_COMPLEXITY | 资源复杂度 | 低好 |
| NATIVE_COMPLEXITY | 是否牵动 Native 命令 | 低好 |
| PERSISTENCE_COMPLEXITY | 持久化契约复杂度 | 低好 |
| CYCLE_RISK | 包循环依赖风险 | 低好 |

## 1. 候选速评

| 候选 | HOST_COUPLING | MIGRATION_COST | NATIVE | 结论 |
|---|---|---|---|---|
| **bookmark** | L（Shell 经 public.ts 消费，无 bridge 依赖） | L（单能力目录，store/ui/manifest 齐全） | 无 Native 命令 | **SELECTED** |
| apps | L | L | 无 | 备选（更薄，信息价值低于 bookmark） |
| tools | L | L | 无 | 备选（极薄，验证价值有限） |
| browser | H（Grid/ActivityBar/MainArea 深度耦合） | H | 部分 | KEEP_M1（中央 hub，见 §14） |
| grid | H（Browser heavy sub-resource） | H | 无 | KEEP_M1（§14/§15 frozen semantics） |
| workspace | H（中央 hub） | H | 无 | KEEP_M1（§14） |
| home / settings | M | M | 无 | KEEP_M1（framework/profile 已收口） |
| terminal | M | M | **有**（term_spawn/Channel） | DEFER（Native 面，待 §16 冻结评估） |
| task / plugin / skill / agent | M~H | H | 部分 | DEFER（Agent 正在 main 树 WIP 迁移，DO_NOT_TOUCH） |
| graph / database | H | H | 有（Rust backend） | DEFER（跨 Rust/TS，超出前端 modularity 范围） |
| credential | — | — | — | FROZEN（§14 安全边界） |

## 2. 选定：bookmark

**理由（对照 9 维度）：**
- **NATIVE_COMPLEXITY = 无**：bookmark 不定义任何 `bridge.ts` 命令（grep 确认 `src/capabilities/bookmark` 无 `bridge.` 调用），不牵动 Native 冻结面（§16）。
- **HOST_COUPLING = 低**：Shell（MainArea / ActivityBar）经 `public.ts` 再导出 `useBookmarkStore` / `canBookmark` 消费，无深 import 内部 store；升格为包后 Host 改为经 `@browser-os/capability-bookmark` 消费，模式与 Vault 完全一致。
- **MIGRATION_COST = 低**：已是完整 M1 形态——`manifest.ts`（含 Building Block Contract v1 内联 `v1`）、`public.ts`、`index.ts`、`state/`、`ui/`、`lifecycle/`、`resource/`、`contracts/`、`intents/`、`README.md` 齐全，物理迁移只需把目录搬入 `packages/capability-bookmark/` 并套用 Vault 干净模式（窄 Ports 反转、state owner 不变、贡献描述符由 Host 注册）。
- **PACKAGE_VALUE / BOUNDARY_GAIN / AI_REVIEW_GAIN = 高**：bookmark 是「收藏夹」，有明确语义 owner（`useBookmarkStore`）、明确贡献槽（`browser-sidebar` / `address-bar-actions` / `activity-bar-trailing`），升格后可由 Generic Checker 全自动校验，是验证 shared contracts 的优质第三样本。
- **CYCLE_RISK = 低**：`optionalDependencies: ["browser"]`，无反向依赖，无循环。

**预置 spec（已写入 `scripts/check-capability-package.mjs` SPECS.bookmark）：**
- `packageRoot: packages/capability-bookmark`
- `packageName: @browser-os/capability-bookmark`
- `capabilityId: bookmark` / `storeId: bookmark` / `contributionId: bookmark.main`
- `oldImplementationPaths: ["src/capabilities/bookmark", "src/stores/useBookmarkStore.ts"]`
- `specialRules.publicSymbols` 按 Vault 模式投影（`createBookmarkCapability` / `bookmarkContribution` / `BOOKMARK_PORTS_KEY`），升格时按真实命名校准。

## 3. 执行顺序（下一 Wave）

1. 在 Train worktree 创建 `packages/capability-bookmark/`，严格镜像 Vault 干净模式：
   - `package.json`（name/version/private/type=module/exports `{".":"./src/index.ts","./manifest":"./src/manifest.ts"}`/deps）
   - `src/manifest.ts`（保留 Building Block Contract v1，去掉 `v1` 内联第二真源——单一 manifest 文件即真源）
   - `src/index.ts`（导出 `createBookmarkCapability` + `bookmarkContribution` + `BOOKMARK_PORTS_KEY`，不导出 sink symbol）
   - `src/ports.ts`（窄 Ports 反转：Shell 经 Ports 读收藏，不直接 import store）
   - `src/state/useBookmarkStore.ts`（state owner 不变，`defineStore('bookmark')`）
   - `src/ui/*`（经 Ports 消费，不直连 bridge）
   - `scripts/check-bookmark-logic.mjs`（领域不变量，能力专属，留在包内）
2. 删除旧 `src/capabilities/bookmark/`（物理迁移，非复制——遵守 directive「旧实现删除非复制」）。
3. Host 消费点改为 `@browser-os/capability-bookmark`（catalog / ActivityBar / MainArea）。
4. 用 `node scripts/check-capability-package.mjs --capability bookmark` 验证 `fail=0`。
5. 跑 `npm run check`（root 13 门禁）确认无回归。
6. 提交 + detached 验证 + tag `frontend-m2-bookmark-package-pass`（M2 wave）。

## 4. 风险与护栏

- **不触碰 main dirty tree**：bookmark 不在 main 树当前 WIP 清单（Clipboard + Agent），可安全在 Train worktree 升格；若升格中发现与 main WIP 重叠，按 directive §6 停手并标注冲突。
- **共享契约（§10）推迟**：third sample 完成后才进入 Three-Package Contract Review；当前 `SHARED_PACKAGE_NEEDED = NO`（RULE_OF_TWO 即时候选 SECOND_REAL_PACKAGE_CONSUMER 仍=0，待 bookmark 真正建包后再 just-in-time 评估）。
- **Runtime Package（§11）= NO**：Host runtime 保持 Host-owned，不抽 `capability-runtime`。
