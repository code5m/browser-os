# DOMAIN_MODULARITY_PHASE1_REVIEW_RESULT

> Phase 1（DDD / Modularity Documentation Program）独立 Review + Clean Closeout 结果。
> 真值快照 HEAD：`8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`（branch: feature/capability-platform-v1）。
> Review 日期：2026-09-24。
> 红线遵守：未改业务代码 / 未动目录 / 未拆包拆 Rust / 未改 UI / 未改 semantic owner / 未改 capability maturity / 未 push / 未 merge / 未动历史 tag。
> H01–H15 保持 `PENDING / BLOCKED_BY_RUNTIME_FIX` 独立状态，未伪造 PASS。

---

## STATUS
- **PASS_WITH_DEBT**
  - 文档基于真实代码 / registry 建立，无虚构 DDD、无成熟度高报、无第二真源；诚实记录待治理面。
  - DEBT（非阻塞，均显式推迟）：全仓 20 能力 README 仍 D0；`capabilities.yaml` 与 `manifest.ts` 漂移（A）；23 个 `GOVERNED` 过宣称（B）；17 个 `semanticOwner` 未进 `owners.yaml`。
  - 以上 DEBT 均不在 Phase 1 范围内（用户红线禁止修 registry / 补 owner），留待后续阶段。

---

## 工作树分类（§1）

### A. PHASE1_DOCUMENTATION（12 文件，untracked）
- `docs/architecture/README-STANDARD.md`
- `docs/architecture/DOMAIN_MODULARITY_PHASE1_RESULT.md`
- `docs/architecture/domain/DOMAIN-INVENTORY.md`
- `docs/architecture/domain/CONTEXT-MAP.md`
- `docs/architecture/domain/UBIQUITOUS-LANGUAGE.md`
- `docs/architecture/modularity/DOCUMENTATION-MATURITY-MATRIX.md`
- `docs/architecture/modularity/PHYSICAL-MATURITY-MATRIX.md`
- `docs/architecture/modularity/REVIEWABILITY-MATRIX.md`
- `docs/architecture/modularity/EXTRACTION-READINESS.md`
- `src/capabilities/skill/README.md`
- `src/capabilities/git/README.md`
- `src/capabilities/workspace/README.md`

### B. H01_RUNTIME_FIX（3 文件，tracked modified）
- `mvp-start.sh`、`run-gui.sh`、`src/capabilities/home/index.ts`

### C. UNKNOWN
- 无。工作树改动已干净分为 A / B 两类，无预期外改动。

---

## DOMAIN_INVENTORY
**PASS**（R1 攻击通过）
- 全仓模块以机器真源 `capability-registry/{capabilities,dependencies,resources}.yaml` + `semantic-registry/owners.yaml` + `native-boundary/native-commands.yaml` 建立，非按目录名猜 Domain。
- 23 能力（22 CAPABILITY + 1 SERVICE:settings）+ 框架运行时 + 共享基础设施，均带 evidence 引用。
- 缺口（UNKNOWN 区）诚实登记：17 owner 未进 semantic registry、13 个 C 未声明、HP/M/RV/D 人工派生、资源实测 NOT_AVAILABLE。

---

## CONTEXT_MAP
**PASS**（R2 / R3 攻击通过）
- 24 上下文：23 能力映射 + RUNTIME_ENGINE + SHARED_INFRASTRUCTURE（2 个非能力上下文）。
- 未过度拆分：workspace 聚合 5 子域为 1 上下文；未将 Capability=Bounded Context 机械等同（workspace 显式含子域 owner）。
- 关系 / 防腐层 / 窄缝均基于 `dependencies.yaml` edges 与 `owners.yaml` 越界规则。

---

## UBIQUITOUS_LANGUAGE
**PASS**（R9 攻击后重分类）
- 术语表 + 通用语言 + 历史别名（`useSystemStore`）均带 CODE_MAPPING / SOURCE。
- 原 5 项「SEMANTIC_COLLISION」经独立 Review 重分类为 **TRUE_SEMANTIC_COLLISIONS = 0**（见下）。

---

## README_STANDARD
**PASS**
- 落地手册 §22 的 38 节模板到 `docs/architecture/README-STANDARD.md`，含禁止项与机器真源引用清单。
- 未复制手册原文，只给「如何填写 + 禁止项」，符合 §23。

---

## PILOT README Review（§6）

### PILOT_SKILL（简单域）
- README_ACCURACY：PASS（明确「Skill 未进 owners.yaml，仅登记于 capabilities.yaml」，与真相一致）
- RESPONSIBILITY / NON_RESPONSIBILITY：PASS（代码证据：public.ts re-export、guard 拦截 run/install）
- UBIQUITOUS_LANGUAGE：PASS
- INVARIANTS：PASS（INV-SKILL-1/2 + ENFORCED_BY）
- STATE_OWNER：PASS
- PUBLIC_CONTRACT：PASS
- DEPENDENCIES：PASS（仅 bridge）
- RESOURCE_BOUNDARY：PASS（LIGHT，实测 NOT_AVAILABLE 诚实标注）
- NATIVE_BOUNDARY：PASS（skills.rs readonly，执行未实现）
- REVIEW_SURFACE：PASS（§28 PRIMARY/OUT_OF_SCOPE）
- AI_CHANGE_SURFACE：PASS（§29）
- SOURCE_OF_TRUTH_LINKS：PASS（§38 链接 YAML）
- SECOND_TRUTH_RISK：NONE（明确「不是第二真源」）
- **结论：PASS**

### PILOT_GIT（中等域）
- README_ACCURACY：**修正后 PASS**（原 §0/§8/§38 误称「git 已进 Semantic Registry」，经核验 `owners.yaml` 无 git 条目，已改为「仅登记于 capabilities.yaml，与 skill 同口径」）
- RESPONSIBILITY / NON_RESPONSIBILITY：PASS（双阶段闸门、不拥有 repo/sync、凭据经 keyring）
- UBIQUITOUS_LANGUAGE：PASS
- INVARIANTS：PASS（INV-GIT-1/2/3）
- STATE_OWNER：PASS（useGitStore，已诚实标注未进 owners.yaml）
- PUBLIC_CONTRACT：PASS
- DEPENDENCIES：PASS（credential/workspace/bridge）
- RESOURCE_BOUNDARY：PASS（PROCESS owned，measure-resources.mjs）
- NATIVE_BOUNDARY：PASS（git_* 命令族，native-commands.yaml）
- REVIEW_SURFACE：PASS（§28）
- AI_CHANGE_SURFACE：PASS（§29，禁改双阶段闸门语义）
- SOURCE_OF_TRUTH_LINKS：PASS（§38 已更正，不再引用不存在的 git owners 条目）
- SECOND_TRUTH_RISK：NONE
- **结论：PASS_WITH_DEBT**（修正了 1 处不准确声明，其余达标）

### PILOT_WORKSPACE（复杂域）
- README_ACCURACY：PASS（聚合能力，5 子域 owner 已进 owners.yaml）
- RESPONSIBILITY / NON_RESPONSIBILITY：PASS（明确不拥有 Files/Artifact/Repo/Script/Snippet 态）
- UBIQUITOUS_LANGUAGE：PASS（子域术语 + Contribution/Slot 模型）
- INVARIANTS：PASS（INV-WS-1/2/3 + WS_OWNER_* 越界规则引用）
- STATE_OWNER：PASS（Core + 5 子域 owner）
- PUBLIC_CONTRACT：PASS（public.ts re-export 6 store）
- DEPENDENCIES：PASS（零硬依赖 + browser 可选 + homeNav 窄缝）
- RESOURCE_BOUNDARY：PASS（LIGHT/MEDIUM，无 native handle）
- NATIVE_BOUNDARY：PASS（委托 git/terminal/credential）
- REVIEW_SURFACE：PASS（§28，10 面板 + 窄缝）
- AI_CHANGE_SURFACE：PASS（§29，禁从 Core 写子域态）
- SOURCE_OF_TRUTH_LINKS：PASS（§38 链接 YAML）
- SECOND_TRUTH_RISK：NONE（非 manifest 翻译，是真 DDD 文档）
- **结论：PASS**

> R15（覆盖简单/中等/复杂 + 可推广）：**PASS** —— skill(C1 只读壳) / git(C2 写闸门+子进程) / workspace(C3 聚合+5子域) 覆盖三档，38 节模板可推广至其余 20 能力。

---

## CAPABILITY_METADATA_TRUTH_MODEL（§3 关键发现 A）

| 层 | 文件 | 运行时是否消费 | 判定 |
|---|---|---|---|
| CANONICAL（单一运行时真源） | `src/capabilities/<id>/manifest.ts`（`CapabilityDefinition`，含 `v1` 子块） | 是：`catalog.ts` 直接 import → `CATALOG_SOURCES` → `CAPABILITY_DEFINITIONS` → `runtime.register(def)`；零复制 | **CANONICAL** |
| RUNTIME_CONSUMED（顶层字段） | 同上 `status`/`lifecycle.activatable`/`lifecycle.resident`/`governanceStatus`/`resources.class`/`dependsOn` | 是：`runtime.ts` activate/suspend/disable 读取 | **RUNTIME_CONSUMED** |
| RUNTIME_CONSUMED（v1 子块） | 同上 `hotPlug.level`/`contributions`/`dependencies`/`publicContract`/`resources[]` | 是：`orchestrator.ts`/`assembly.ts`/`profiles.ts` 用于可用性/依赖环/hotplug/槽挂载 | **RUNTIME_CONSUMED** |
| VALIDATED_PROJECTION（DRIFTED） | `docs/architecture/capability-registry/capabilities.yaml` | **否**：`src/` 零引用；仅 11 个 `scripts/check-*.mjs` 门禁校验用 | **DOCUMENTATION_ONLY / STALE(DRIFTED)** |
| CANONICAL（治理，gate-time 非运行时） | `semantic-registry/{owners,intents,states,side-effects}.yaml` | 否：`check-semantic-registry.mjs` 门禁消费；运行时 `disable()` 仅按 manifest 的 `GOVERNED` 标签判断 | **CANONICAL(GATE)** |
| DOCUMENTATION_ONLY | 3 Pilot README + Inventory/ContextMap/UbiquitousLanguage/矩阵/RESULT | 否 | **DOCUMENTATION_ONLY** |

> 结论：**`manifest.ts` 是运行时权威**（代码证明 runtime 消费它），`capabilities.yaml` 仅为门禁校验用的声明镜像且已漂移（skill `activatable:false` vs manifest `true`、缺 `maturity`/`hotPlug` 字段）。R6 攻击：Phase 1 文档对 skill 已诚实标注此漂移；git README 曾误称「已进 Semantic Registry」，已修正。

---

## SEMANTIC_GOVERNANCE_COVERAGE（§4 关键发现 B）

> 判定口径：owner 真源 = `semantic-registry/owners.yaml`（12 条目，覆盖 11 个 distinct store：useLayoutStore/useBrowserStore/bridge/useFileStore/useArtifactStore/useRepoStore/useScriptStore/useSnippetStore/useBookmarkStore/useTerminalStore/KeyringStore）。capability `semanticOwner`（23 个）逐一比对。

- **FULLY_GOVERNED：0**
  （无能力在 owner + state + intent + writer + checker 五维全机器化）
- **PARTIALLY_GOVERNED：7**
  `browser`、`grid`（owners: browser_grid_lifecycle + browser_tabs）、`terminal`、`bookmark`、`credential`、`script`（owner 在 owners.yaml + 越界规则 + checker）；`workspace`（CORE owner 未进 owners.yaml，但 5 子域 files/artifact/repo/snippet 已进，混合并记 partial）
- **OWNER_DECLARED_ONLY：16**
  `database`、`git`、`agent`、`skill`、`plugin`、`graph`、`vault`、`resource_collection`、`task`、`clipboard`、`apps`、`tools`、`session`、`workbench`、`settings`、`home`
  （`semanticOwner` 仅在 capabilities.yaml 登记，owners.yaml 无条目，无语义 checker；其中 `database`/`git` 另有 `check-developer-owners.mjs` 的 DEV-* 局部固化，记为 partial-checker）
- **OWNER_PENDING_SCR：0**
  （registry 头部铁律要求「未登记 owner 须填 OWNER_PENDING_SCR」，但实际 17 个均被误标 GOVERNED —— 这是过宣称，非真实 PENDING_SCR）
- **UNGOVERNED：0**
  （23 个均带 GOVERNED 标签 + 已声明 semanticOwner，即便过宣称也不算 UNGOVERNED）
- **UNKNOWN：0**

> R7 核实：「17 个 owner 未进 semantic registry」**数字真实**。清单：workspace(core)/database/git/agent/skill/plugin/graph/vault/resource_collection/task/clipboard/apps/tools/session/workbench/settings/home（17）。
> R8 核实：「23 个 GOVERNED」字面真实（全 23 标 GOVERNED），但机器化治理仅 6–7 个；为**过宣称**，Phase 1 已诚实登记为文档漂移 / 后续修正项。

---

## TRUE_SEMANTIC_COLLISIONS（§5 关键发现 C）

**0**（R9 攻击后重分类；原草案标 5 项，经核验无一为真正语义碰撞）

逐项判定：
1. repo(capability 子域) vs git → **RELATED_BUT_DISTINCT**（相邻但不同；owners.yaml `repo` 条目已声明 REPO≠GIT）
2. script(owner) vs workspace ScriptPanel(物理) → **PHYSICAL_LOGICAL_SEPARATION**（逻辑/物理分离，记 Debt-7B-1，非碰撞）
3. home(能力) vs mainView='home'(导航态) → **NAMING_AMBIGUITY**（同名异层，需消歧不重命名）
4. session(能力) vs runtime session → **NAMING_AMBIGUITY**（同名异义）
5. grid(capability) vs gridOpen(状态) → **VALID_DERIVATION**（同 owner useBrowserStore，实为 browser↔grid 耦合 Debt-7B-1，非碰撞）

---

## NAMING_AMBIGUITIES（§5）
- `home`：能力语义 vs 导航态 `mainView='home'`
- `session`：持久化 store vs 浏览器/PTY 运行时会话态
（仅 2 项需持续消歧；均不重命名 frozen semantics）

---

## DOCUMENTATION_SECOND_TRUTHS
**0**
- 所有 Pilot README 明确声明「本 README 不是第二真源」并链接机器真源（capabilities.yaml/resources.yaml/owners.yaml/native-commands.yaml）。
- 矩阵类文档显式标注「人工派生 / UNKNOWN / 未机器化」，未把推断写死为事实。

---

## README_COVERAGE
- 23 能力（22 CAPABILITY + settings）：Pilot 3 个达 D1→D3；其余 20 个 D0（待 Phase D3 rollout）。
- 框架/共享基础设施 README：D0。
- **覆盖率：3 / 23 ≈ 13%**（Pilot 样例已建立，全量未推广）。

---

## PHYSICAL_MATURITY
- M1（DIRECTORY_ISOLATED）：17（browser/workspace/terminal/bookmark/database/git/agent/skill/plugin/graph/vault/task/clipboard/apps/tools/home/settings）
- M0（LOGICAL_ONLY）：4（grid/resource_collection/session/script/workbench 中未独立目录者；实际 grid+resource_collection+session+workbench+script = 5 个 M0，其中 script 为物理漂移）
- M2–M5：0（无独立 package / 版本 / 仓库）

---

## REVIEWABILITY
- RV2（BOUNDED_REVIEW）：3（skill/git/workspace，Pilot §28 Review Surface）
- RV1（OWNER_NAVIGABLE）：13（其余 owner 单一、entrypoint 明确者）
- RV0（GLOBAL_CONTEXT_REQUIRED）：5（grid/resource_collection/session/script/workbench）
- RV3–RV4：0（无独立测试面 / 独立发布审核）

---

## EXTRACTION_READINESS
- DIRECTORY_READY：12（bookmark/database/git/task/clipboard/apps/tools/graph/skill/plugin/agent/vault/terminal）
- NOT_READY：11（browser/grid/workspace/credential/resource_collection/session/script/workbench/settings/home）
- PACKAGE_READY：0（缺 D3 + 独立测试面 + review surface 实证）
- REPOSITORY_CANDIDATE：0（手册 §40 默认禁止新建仓库）

---

## DOC_COMMIT
- 提交对象：仅 §A 的 12 个 Phase 1 文档文件（不含 H01_RUNTIME_FIX 3 文件）。
- Commit message：`docs(domain): establish DDD and modularity phase1 baseline`
- Annnotated tag：`domain-modularity-phase1-pass`（本地，不 push）。

---

## TAG
- `domain-modularity-phase1-pass` / **CREATED（本地 annotated，未 push）**

---

## H01_RUNTIME_FIX_STATUS
- **UNCOMMITTED_WIP**
  - 3 文件（mvp-start.sh / run-gui.sh / home/index.ts）改动未提交。
  - 验证证据缺口：本 Review 会话未重新跑 `npm run check` / `npm run build` / fresh `npm run tauri dev` 确认；无用户 H01 目视复测证据。
  - 依用户红线（须满足 root cause + check PASS + build PASS + fresh tauri dev PASS + Vite ENOENT=0 + 用户 H01 retest evidence 方可提交），当前**禁止提交**，保持 uncommitted 并记录于 handoff。

---

## PUSHED
**NO**

## MERGED_MASTER
**NO**

## WORKTREE
**DIRTY_WITH_H01_WIP**（Phase 1 文档已提交 + 打 tag；H01 3 文件仍 uncommitted）

---

## NEXT_EXACT_TASK
1. **Phase D3 README rollout**：将 README 标准推广到其余 20 能力（D0→D1+），并由 `check-module-documentation.mjs` 固化门禁（非本 Phase）。
2. **修漂移（A/B）**：同步 `capabilities.yaml` 与 `manifest.ts`（activatable/maturity/hotPlug），并将 17 个 `semanticOwner` 补登 `owners.yaml` 或把 `GOVERNED` 降级为 `OWNER_PENDING_SCR`（诚实反映未固化态）—— 用户红线禁止在 Phase 1 做，留待后续。
3. **H01 runtime repair closeout**：重新验证 3 文件修复（root cause + 4 项 PASS + 用户目视），通过后单独提交 `fix(runtime): ...`；否则保持 WIP。

> Phase 1 Independent Review + Clean Closeout 已完成，已停止。以上 3 项均不在本 Phase 内，需用户拍板再继续。
