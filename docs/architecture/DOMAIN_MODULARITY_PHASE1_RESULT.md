# DOMAIN_MODULARITY_PHASE1_RESULT

> Phase 1（DDD / Modularity Documentation Program）交付结果。
> 范围：仅文档与真值梳理。**未改业务代码、未动目录、未拆包/拆 Rust、未改 UI / semantic owner / capability maturity、未 push / merge / 动历史 tag。**
> H01–H15 保持 `PENDING / BLOCKED_BY_RUNTIME_FIX` 独立状态（参见 §BLOCKING_DEBT），未伪造 PASS。
>
> 真值快照 HEAD：`8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`（branch: feature/capability-platform-v1）。

---

## STATUS
- PHASE: 1 (Documentation & Understanding Model)
- RESULT: COMPLETE（文档交付完成，待独立 Review 与后续 rollout）
- AI_MAINTAINABILITY: PARTIAL（3 个 Pilot README + 标准已建；20 个能力仍 D0）
- UI_PRESERVATION: PASS（Phase 1 仅新增文档，未触 UI）
- NEW_REGRESSION: NONE（无代码改动）
- PUSHED: NO
- MERGED_MASTER: NO
- SYSTEM_INSTALL_MODIFIED: NO
- USER_DATA_MODIFIED: NO

---

## START / FINAL HEAD
- START_HEAD: `8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`
- FINAL_HEAD: `8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`（Phase 1 未提交；仅新增未跟踪文档）
- WORKTREE: DIRTY（新增 11 个未跟踪文档文件；另有 3 个早前未提交的冻屏修复：mvp-start.sh / run-gui.sh / src/capabilities/home/index.ts）

---

## 交付物（新增文件）
1. `docs/architecture/README-STANDARD.md` — README 标准（落地手册 §22 38 节）
2. `docs/architecture/domain/DOMAIN-INVENTORY.md` — 全仓模块清单（23 模块 + 框架 + 共享基础设施）+ UNKNOWN 缺口
3. `docs/architecture/domain/CONTEXT-MAP.md` — 限界上下文地图（24 上下文 + 关系 + 防腐层 + 语义碰撞）
4. `docs/architecture/domain/UBIQUITOUS-LANGUAGE.md` — 通用语言（术语表 + 碰撞 + 历史别名）
5. `docs/architecture/modularity/DOCUMENTATION-MATURITY-MATRIX.md`
6. `docs/architecture/modularity/PHYSICAL-MATURITY-MATRIX.md`
7. `docs/architecture/modularity/REVIEWABILITY-MATRIX.md`
8. `docs/architecture/modularity/EXTRACTION-READINESS.md`
9. `src/capabilities/skill/README.md` — Pilot（简单域）
10. `src/capabilities/git/README.md` — Pilot（中等域）
11. `src/capabilities/workspace/README.md` — Pilot（复杂域）

---

## 计数
- BOUNDED_CONTEXTS: 24（含 RUNTIME_ENGINE + SHARED_INFRASTRUCTURE）
- CAPABILITIES: 22（CAPABILITY 类）+ 1（SERVICE: settings）
- FRAMEWORK_SERVICES: settings, session, credential(security)
- SHARED_INFRASTRUCTURE: bridge / pinia / security_policy / shell + shared / stores / composables / components / utils / styles
- NATIVE_MODULES: `src-tauri/`（security_policy.rs / skills.rs / git 命令族等），归属见 `docs/architecture/native-boundary/native-commands.yaml`
- UI_MODULES: `src/components/` + 各能力 `ui/`

---

## 关键发现（Phase 1 真正价值：暴露漂移/过宣称）

### A. 注册表(capabilities.yaml) 与 代码(manifest.ts) 漂移
| 能力 | capabilities.yaml | manifest.ts(v1) | 冲突点 |
|---|---|---|---|
| skill | `activatable:false`、无 maturity | `activatable:true`、`maturity:C1`、`hotPlug:HP0` | activatable 与 maturity 不一致 |
| workspace | 无 maturity | `maturity:C3` | maturity 仅在代码声明 |
| git/skill/workspace |（无 hotPlug 声明）| `hotPlug.level:HP0` | HP 以代码 manifest 为准，注册表缺 HP 字段 |

→ 以代码 `manifest.ts` 为运行时权威；`capabilities.yaml` 待同步。属 **DOCUMENTATION_DRIFT = FAIL**。

### B. `governanceStatus: GOVERNED` 过宣称
- `capabilities.yaml` 给 23 个能力标 `GOVERNED`；但 `semantic-registry/owners.yaml` 实际只固化 **12 个 owner 上下文**
  （view_navigation / browser_grid / native_execution / browser_tabs / files / artifact / repo / script / snippet / bookmark / terminal / credential）。
- 由此，**17 个能力的 `semanticOwner` 未在 Semantic Registry 登记**（无 `forbidden_callers` / `authorized_callers` 机检）：
  workspace(core) / database / git / agent / skill / plugin / graph / vault / resource_collection / task / clipboard / apps / tools / session / workbench / settings / home。
- 即这些 owner 的越界治理**未机器化**（仅 database/git 有 `check-developer-owners.mjs` 局部固化）。

### C. 语义关系（独立 Review 重分类：TRUE_SEMANTIC_COLLISIONS = 0）
> 原草案将以下 5 项全标为「SEMANTIC_COLLISION」，经独立 Review 重分类，**无一为真正的语义碰撞**（详见 CONTEXT-MAP §3 / UBIQUITOUS-LANGUAGE §碰撞与别名）：
1. repo（workspace 子域） ≠ git（git 操作） → RELATED_BUT_DISTINCT
2. script（逻辑 owner `useScriptStore`）物理在 `workspace/ui/ScriptPanel.vue`（M0 漂移） → PHYSICAL_LOGICAL_SEPARATION（物理漂移 Debt）
3. home（能力） vs `mainView='home'`（导航态） → NAMING_AMBIGUITY
4. session（能力） vs runtime session → NAMING_AMBIGUITY
5. grid（capability） vs `gridOpen`（状态，同 owner `useBrowserStore`） → VALID_DERIVATION（同 owner，实为 browser↔grid 耦合 Debt-7B-1）

---

## 成熟度现状（MATURITY）
- DOCUMENTATION (D): 全仓能力 **D0**（README=0），Pilot 3 个达 D1→D3 样例。
- PHYSICAL (M): 17 个 M1、4 个 M0（resource_collection/session/workbench/script）；无 M2+。
- REVIEWABILITY (RV): 16 个 RV1、5 个 RV0；**无 RV2**（缺 README Review Surface）。
- EXTRACTION: 12 个 DIRECTORY_READY、11 个 NOT_READY；无 PACKAGE/REPO 级。
- CAPABILITY (C): 仅 skill C1、agent C1、database/git/plugin/graph/task/clipboard/apps/tools C2、workspace C3（代码 manifest）；其余 UNKNOWN。
- HOT-PLUG (HP): skill/git/workspace 显式 HP0；其余按 `activatable` 派生但**与代码 manifest 冲突**，以 HP0/HP1 需逐能力核实。

---

## 边界判定
- DOMAIN_BOUNDARIES: PARTIAL（Contribution Registry + owners.yaml 覆盖部分；17 owner 未固化）
- PUBLIC_CONTRACTS: PARTIAL（`public.ts` re-export 模式存在，但未全覆盖机检）
- NATIVE_BOUNDARIES: PARTIAL（`native-commands.yaml` + `check-native*` 存在，但未逐能力映射进 Inventory）
- RESOURCE_BOUNDARIES: PARTIAL（声明式口径 + `check-capability-resource-boundary.mjs`；实测 NOT_AVAILABLE）
- UI_BOUNDARIES: PARTIAL（槽渲染 + `check-ui-boundaries.mjs`）
- MODULE_RESOLUTION: UNVERIFIED（`check-module-resolution.mjs` 存在，未在本 Phase 跑）
- DOCUMENTATION_DRIFT: FAIL（见发现 A/B）

---

## BLOCKING_DEBT
- 无运行时阻塞。
- 文档阻塞（NON_BLOCKING）：注册表与代码漂移（A）、GOVERNED 过宣称（B）—— 两者均为**文档/真源一致性债**，不阻断运行，但阻碍「机器真源单一化」。

## NON_BLOCKING_DEBT
- 全仓 20 个能力 README 缺失（D0）。
- 13 个能力 `maturity(C)` 未声明。
- `script` M0 物理漂移。
- 资源实测缺失（Debt-7A-1）。
- browser↔grid 深度耦合（Debt-7B-1）。

## REPOSITORY_CANDIDATES
- 无（手册 §40 默认禁止新建仓库）。

---

## COMMITS / TAGS
- COMMITS: 0（Phase 1 未提交；新增文档为 untracked）
- TAGS: 0（未创建任何 tag，含未动历史 tag）
- 建议：经独立 Review（手册 §56）通过后，按 §58 提交 Phase 1 文档（annotated commit），**本地提交不 push**。

---

## REVIEW UPDATE（2026-09-24 独立 Review 修正）

> 进入 Phase 1 Independent Review 后，对文档做了如下准确性修正（均为文档层，未动 registry / 代码 / 红线）：
> - **git/README.md**：原 §0/§8/§38 误称「git 已进 Semantic Registry (ACCEPTED/CURRENT_FACT)」。经核验 `owners.yaml` 实际**无 git 条目**（仅 workspace 子域 `repo` 在列）。已改为「useGitStore 仅登记于 capabilities.yaml，未进 owners.yaml（与 skill 同口径）」。
> - **CONTEXT-MAP §3 / UBIQUITOUS-LANGUAGE §碰撞 / 本文件 §C**：原将 5 项全标 `SEMANTIC_COLLISION`。独立 Review 重分类为 **TRUE_SEMANTIC_COLLISIONS = 0**，逐项归为 RELATED_BUT_DISTINCT / PHYSICAL_LOGICAL_SEPARATION / NAMING_AMBIGUITY / VALID_DERIVATION（避免为找问题而制造碰撞，手册 §5 红线）。
> - **REVIEWABILITY-MATRIX**：3 个 Pilot README 已引入 §28 Review Surface，故 skill/git/workspace 达 RV2，原「全仓无 RV2」结论修正为「3 个 RV2 + 13 个 RV1 + 5 个 RV0」。
> - **CAPABILITY_METADATA_TRUTH_MODEL**：确认 `manifest.ts` 为唯一运行时真源（`catalog.ts` 直接 import → runtime.register），`capabilities.yaml` 在 `src/` 零引用、仅门禁校验用（DRIFTED 声明镜像）；semantic-registry YAML 为 gate-time 治理真源、非运行时消费。

---

## NEXT_EXACT_TASK（下一步，待你确认后执行）
1. **独立 Review**（手册 §56）：对 3 个 Pilot README 跑独立 Review Agent → 输出 PASS / PASS_WITH_DEBT / FAIL。
2. **Phase D3 全量 rollout**：将 README 标准推广到其余 20 个能力（D0→D1+），并由 `check-module-documentation.mjs` 固化门禁。
3. **修漂移（A/B）**：同步 `capabilities.yaml` 与 `manifest.ts`（activatable/maturity/HP），并将 17 个 `semanticOwner` 补登 `owners.yaml`（或把 `GOVERNED` 降级为 `OWNER_PENDING_SCR` 以诚实反映未固化态）。
4. **收口 H01–H15**（独立线）：提交早前 3 处冻屏修复 → 你跑 `npm run tauri dev` 目视。

> Phase 1 已停止。以上 4 项均不在本 Phase 内，需你拍板再继续。
