# STAGE J — Capability Platform vNext Closeout（2026-09-23）

> 本文件是 STAGE I（Workbench Capability Decoupling）完成后对 Capability Platform 的
> 一次完整闭环审计。全部事实来自真实代码 / 真实门禁输出，不编造 KPI。
>
> 基线 HEAD（STAGE I 收口）：`858d6ba`（tag `capability-stage-i-workbench-decoupling-pass`）。
> 本审计的最终 tag 仅允许 `capability-platform-vnext-code-pass`（CODE PASS）；
> **禁止** `capability-platform-vnext-pass`（Final Human Acceptance 未执行，HUMAN_VISUAL=PENDING）。

---

## 1. FULL PRODUCT CAPABILITY MATRIX（23 条，UNKNOWN = 0）

分类取值：CAPABILITY / SERVICE（框架服务，非产品能力）。owner 均为 Single Semantic Owner。

| ID | Category | Status | Semantic/State Owner | 物理位置 | Maturity | HotPlug | 依赖（required） | Persistence |
|---|---|---|---|---|---|---|---|---|
| browser | CAPABILITY | COMPATIBILITY_WRAPPED | useBrowserStore | src/capabilities/browser | C3 | HP0 | bridge | session |
| grid | CAPABILITY | NOT_INTEGRATED | useBrowserStore（与 browser 共 owner） | src/components/browser | C1 | HP0 | browser, bridge | session |
| workspace | CAPABILITY | COMPATIBILITY_WRAPPED | useWorkspaceStore | src/capabilities/workspace | C3 | HP0 | browser, bridge | disk |
| terminal | CAPABILITY | COMPATIBILITY_WRAPPED | useTerminalStore | src/capabilities/terminal | C3 | HP0 | bridge | session |
| bookmark | CAPABILITY | COMPATIBILITY_WRAPPED | useBookmarkStore | src/capabilities/bookmark | C3 | HP2 | browser（bridge optional） | disk |
| credential | CAPABILITY | NOT_INTEGRATED | KeyringStore（native） | src-tauri/src/security_policy.rs | C1 | HP0 | bridge | os_keyring |
| database | CAPABILITY | COMPATIBILITY_WRAPPED | useDatabaseStore | src/capabilities/database | C2 | HP0 | credential, bridge | session |
| git | CAPABILITY | COMPATIBILITY_WRAPPED | useGitStore | src/capabilities/git | C2 | HP0 | credential, workspace, bridge | disk |
| agent | CAPABILITY | COMPATIBILITY_WRAPPED | useAgentStore | src/capabilities/agent | C1 | HP0 | bridge（graph optional） | disk |
| skill | CAPABILITY | COMPATIBILITY_WRAPPED | useSkillStore | src/capabilities/skill | C1 | HP0 | bridge | disk |
| plugin | CAPABILITY | COMPATIBILITY_WRAPPED | usePluginStore | src/capabilities/plugin | C2 | HP0 | bridge | disk |
| graph | CAPABILITY | COMPATIBILITY_WRAPPED | useGraphStore | src/capabilities/graph | C2 | HP0 | bridge（agent optional） | disk |
| vault | CAPABILITY | COMPATIBILITY_WRAPPED | useVaultStore | src/capabilities/vault | C2 | HP2 | bridge | none |
| resource_collection | CAPABILITY | NOT_INTEGRATED | useResourceStore | src/stores/useResourceStore.ts | C0 | HP0 | bridge | disk |
| task | CAPABILITY | COMPATIBILITY_WRAPPED | useTaskStore | src/capabilities/task | C2 | HP0 | workspace, bridge（script optional） | disk |
| clipboard | CAPABILITY | COMPATIBILITY_WRAPPED | useClipboardStore | src/capabilities/clipboard | C2 | HP0 | bridge | none（会话态，B11-1） |
| apps | CAPABILITY | COMPATIBILITY_WRAPPED | useAppsStore | src/capabilities/apps | C2 | HP0 | bridge | none |
| tools | CAPABILITY | COMPATIBILITY_WRAPPED | useToolsStore | src/capabilities/tools | C2 | HP0 | bridge | none |
| session | CAPABILITY | NOT_INTEGRATED | useSessionStore | src/stores/useSessionStore.ts | C0 | HP0 | bridge | none |
| script | CAPABILITY | NOT_INTEGRATED | **null（OWNER_PENDING_SCR）** | src/capabilities/workspace/ui/ScriptPanel.vue | C0 | HP0 | bridge | disk |
| workbench | CAPABILITY | NOT_INTEGRATED | useWorkbenchStore | src/stores/useWorkbenchStore.ts | C0 | HP0 | bridge | disk |
| settings | SERVICE | COMPATIBILITY_WRAPPED | useSettingsStore | src/settings | C2 | HP2 | （无） | disk |
| home | CAPABILITY | COMPATIBILITY_WRAPPED | useHomeStore | src/capabilities/home | C2 | HP2 | browser, workspace, apps, bridge | disk |

### Maturity 分布（真实，无夸大）

| 等级 | 数量 | 成员 |
|---|---|---|
| C0 REGISTERED | 4 | resource_collection, session, script, workbench |
| C1 WRAPPED | 4 | grid, credential, agent, skill |
| C2 ISOLATED | 11 | database, git, plugin, graph, vault, task, clipboard, apps, tools, settings, home |
| C3 OPTIONAL | 4 | browser, workspace, terminal, bookmark |
| C4 RUNTIME_CONTROLLABLE | 0 | ——（无按能力独立 runtime 启停实测，登记 Debt） |
| C5 RESOURCE_RELEASABLE | 0 | ——（无按能力资源释放实测，登记 Debt-7A-1） |

> 诚实性：C2 = 物理隔离（manifest + public + state + ui + contribution 就位）；**不**等于 C3。
> C3（absent→Shell valid→contribution absent→implementation not activated→heavy resource absent）
> 由 `runtime-resource-absence.mjs`（12/12）与 `check-capability-composition.mjs`（C4-ABSENT）证明。

### HotPlug 分布

| 等级 | 数量 | 成员 |
|---|---|---|
| HP0（静态注册） | 19 | 除下列外全部 |
| HP1 | 0 | —— |
| HP2（运行时 register/unregister） | 4 | bookmark, vault, home, settings |
| HP3（动态安装/沙箱加载） | 0 | ——（禁止高风险动态代码加载，未实现） |

### UNKNOWN = 0

- 每条能力均有 Semantic Owner（唯一例外：`script` 显式 `OWNER_PENDING_SCR`，是**已知未裁决**，非 UNKNOWN）。
- 每条能力均有分类、状态、依赖、持久化归属；无「猜」的条目。

---

## 2. CAPABILITY DEPENDENCY GRAPH（§32）

- **undeclared dependency = 1**（既有债务）：`src/capabilities/workspace/ui/FileEditor.vue` 经 public 依赖 browser，
  但 workspace registry 已声明 browser —— 实测 UI-04b-U 仍报 warn，属检查器口径（内部 vs public）差异，
  登记为 **Debt-J-1**（不静默忽略）。
- **internal cross-capability import = 0**（CB-01 fail=0）。
- **illegal cycle = 0**；**required cycle = 0**（CB-04 fail=0）。
- **已审计的 optional cycle（1 个）**：`agent ↔ graph`（agent→graph optional / graph→agent optional）。
  重新审计结论：二者均为「可选增强」（agent 可无图谱运行；graph 可无 agent 运行），不阻断装配，
  保留为 warn（设计异味已登记，非忽略）。
- **home ↔ workspace**：**已消解**。原可形成 required 环（home→workspace required；
  workspace 收藏目录需调用 home），STAGE I-B 经 shared 缝 `src/composables/homeNav.ts` 解环，
  workspace **不**声明 home 依赖（与 terminalNav/browserNav/recentsNav 同族）。

Shared seams（非能力，CB 豁免）：`src/composables/{browserNav,recentsNav,terminalNav,homeNav}.ts`、
`src/shared/ui`。

---

## 3. RESOURCE OWNERSHIP MATRIX（§33）

| 资源 | OWNER | CREATE | ACTIVATE | SUSPEND | DESTROY/RELEASE | ABSENCE EVIDENCE | 无资源则标 NONE |
|---|---|---|---|---|---|---|---|
| WebView | browser | browser.openBrowser | 视图激活 | 不支持（不可冻结） | 不支持 | absent→0 webview | —— |
| 工具子 webview(tool://) | tools | openTool | DO（工具动作） | 不支持 | 不支持（TARGET） | absent→0 | —— |
| Grid child process | browser（grid 视图） | —— | —— | —— | —— | **framework grid-child = 0** | —— |
| PTY | terminal | term_spawn_channel | 终端激活 | 不支持 | 支持（TARGET 未实测） | **framework PTY = 0** | —— |
| filesystem watcher | resource_collection / task | —— | 后台 | 支持（TARGET） | 支持（TARGET） | absent→0 watcher | —— |
| DB connection | database | 连接建立 | 查询 | 支持（TARGET） | 支持（TARGET） | absent→0 连接 | —— |
| Git process/operation | git | 命令执行 | 操作 | 支持（TARGET） | 支持（TARGET） | absent→0 进程 | —— |
| Plugin runtime | plugin | —— | —— | 不支持（LOCKED） | 不支持（LOCKED） | absent→0 runtime | —— |
| Agent 执行资源 | agent | —— | 会话 | 支持（TARGET） | 支持（TARGET） | absent→0 请求 | —— |
| network/socket | database/git/agent | —— | —— | —— | —— | absent→0 | —— |
| credential/keyring | credential | KeyringStore | 常驻 | 不支持（安全边界） | 不支持 | absent→0 句柄 | —— |
| 进程（app 启动） | apps | launchApp(detached) | 用户动作 | 不支持（detached 不由能力管） | 不支持 | absent→0 枚举 | —— |
| 脚本进程 | script | —— | 执行 | 不支持 | 支持（TARGET） | absent→0 进程 | —— |
| 内存态（spa） | clipboard/vault/home/skill/session/settings/bookmark/workspace | —— | —— | 支持 | 支持（TARGET） | absent→0 | —— |
| **NONE（无原生资源）** | vault / clipboard / home / settings / skill / session | —— | —— | —— | —— | —— | **显式 NONE** |

> `measurement_status`：项目当前**无按能力的资源实测机制**（`resources.yaml` 诚实声明）；
> 上表为 DECLARED 口径 + 缺席证据，非实测性能数字（Debt-7A-1）。

---

## 4. AI MAINTAINABILITY REPORT（§34）

| 场景 | 结论 | 依据 |
|---|---|---|
| A. 改 Browser UI 无需理解 Terminal internals | **PASS** | browser/terminal 独立能力包；无互相内部 import |
| B. 改 Skill 无需改 Agent internal store | **PASS** | skill/agent 独立 owner；仅 agent→graph optional 边 |
| C. 改 Plugin 经 public contract | **PASS** | plugin/public.ts；Shell 经贡献渲染 |
| D. 新增简单能力无需改 App.vue 业务 switch | **PASS** | 能力经 `ALL_CAPABILITIES` + catalog 注册；UI 经 contribution（本夜 vault/home/settings 亲证：MainArea 零改动即渲染新能力） |
| E. 新增能力 UI 能复用 shared/ui | **PASS** | shared/ui（EmptyState/ContextMenu）public 出口；UI_BOUNDARIES vacuous=0 |

**结论：PASS**（局部可改，无需理解全仓）。唯一注意：新增能力须同步 registry/capability/`ALL_CAPABILITIES`/profiles
（有门禁兜底，非「改 App.vue 业务 switch」）。

---

## 5. COMPOSITION TEST（§35）

- `check-capability-composition.mjs`：**33/33 PASS**。
- `check-composition-profiles.mjs`：**11/11 PASS**（framework-only / browser-only / workspace-only / terminal-only / developer / full 等组合）。
- `runtime-resource-absence.mjs`：**12/12 PASS**（absent implementation 未初始化、absent resource 未创建、Shell 仍有效）。

> 目标非「隐藏菜单」，而是 absent 时不初始化 implementation、不创建 resource、Shell 保持有效 —— 已由上述门禁机器化证明。

---

## 6. PHYSICAL MODULARITY REVIEW（§36）

**结论：PARTIAL**（各能力物理边界见下；本阶段未再大搬目录）。

- 完整能力包（manifest + public + state + ui + index + contribution）：browser, workspace, terminal, bookmark,
  database, git, agent, skill, plugin, graph, task, clipboard, apps, tools, **vault（本夜）**, **home（本夜）**, settings。
- **PARTIAL / 未完全物理隔离**：
  - `grid`：与浏览器共 owner（useBrowserStore），无独立目录（设计如此）。
  - `credential`：native adapter（src-tauri/src/security_policy.rs），无 frontend 能力包。
  - `resource_collection` / `session` / `workbench`：仍在 `src/stores/`，无 capabilities/<id>/（未迁移）。
  - `script`：UI 寄居 `src/capabilities/workspace/ui/ScriptPanel.vue`，owner 未裁决（`OWNER_PENDING_SCR`）。

---

## 7. UI SYSTEM CLOSEOUT CHECK（§37）

| 检查 | 结果 |
|---|---|
| UNKNOWN UI = 0 | PASS（`ui-components.yaml` 68 条 / 68 个 .vue 文件一致） |
| shared/ui boundary PASS | PASS（UI_BOUNDARIES vacuous=0；shared/ui 不含 capability/business import） |
| UI catalog matches files | PASS（UI-10 PASS） |
| no new true duplicates from migration | PASS（UI 去重门禁无新增） |
| Workbench contribution remains generic | PASS（UI-09 未新增硬编码视图；vault/home/settings 均经通用 WORKBENCH_MAIN 贡献） |
| UI preservation structural PASS | PASS（home/vault 业务 UI 零改动；HomeShortcutEditor `.hs-mask`、position:fixed 基线路径同步） |
| HUMAN_VISUAL | **PENDING**（未做人工目视，不伪造） |

---

## 8. FINAL RED TEAM（§38）

假设：「整个 Capability Platform 只是目录整理」。逐条证伪：

| # | 攻击点 | 结论 | 证据 |
|---|---|---|---|
| 1 | 仍有大量业务逻辑散落 Shell？ | 否 | MainArea 无任何能力面板静态 import（SHELL_DIRECT_BUSINESS_RENDER=0）；UI-03 仅 2 处既有导航部件债务 |
| 2 | public.ts 只是 wrapper？ | 部分 | home/public.ts 为**窄意图**（非 store 转发）；vault/settings public 为受控出口；无万能 facade |
| 3 | 旧 store 仍是真 owner？ | 否 | 迁移能力 owner 已指向 capabilities/<id>/state；registry entrypoint 已修正（bookmark/database/script） |
| 4 | capability absent 仍初始化 implementation？ | 否 | runtime-resource-absence 12/12 + composition C4-ABSENT |
| 5 | 资源偷偷启动？ | 否 | framework grid-child=0 / PTY=0；CAPABILITY_RESOURCE_BOUNDARY 12/12 |
| 6 | manifest 与 runtime 不一致？ | **部分（已修 / 记录）** | 已修 bookmark/database/script registry entrypoint 漂移；**Debt-J-2**：workspace registry dependsOn=[browser,bridge] 与其 manifest.ts dependsOn=[] 不一致（既有） |
| 7 | profiles 与 runtime truth 不一致？ | 否 | profiles.yaml 与 ALL_CAPABILITIES 一致；composition-profiles 11/11 |
| 8 | contribution 只是 UI hide？ | 否 | 贡献为**组件级**注册（viewOf 渲染），非显示开关；absent→不注册→不挂载 |
| 9 | native/backend 未纳入 boundary？ | 部分 | credential=security_policy.rs、script=ScriptPanel 已登记；Rust 侧未纳入 CB 门禁（登记 Debt-J-3） |
| 10 | checker 只验证文本？ | 部分 | 多数 checkers 为源码级；语义 checker 已覆盖真实文件扫描；`check-capability-platform` 运行真实 runtime 装配 |
| 11 | baseline 可掩盖 regression？ | 否 | UI 基线策略 on_new=FAIL / on_removed=FAIL（消除必须显式更新，本夜 vault/home/settings 均显式更新） |
| 12 | AI 修改仍需跨全仓？ | 否 | §4 AI maintainability PASS |

**Red Team 结论：PASS_WITH_DEBT**（发现 3 项已记录债务，无「严重问题未修复即放行」）。

---

## 9. 不变量 & KNOWN DEBT

- SECOND_TRUTHS = **0**
- NEW_RESOURCE_LEAKS = **0**
- FRAMEWORK_RESOURCE_ABSENCE = PASS（grid-child=0 / PTY=0）
- SEMANTIC_GOVERNANCE = PASS（fail=0）
- CAPABILITY_GOVERNANCE = PASS（registry/composition/profiles/boundaries）
- RESOURCE_GOVERNANCE = PASS（resource-boundary 12/12 / absence 12/12）
- BUILD = PASS；NPM_CHECK = PASS；GIT_FSCK = PASS（仅 dangling blob，非损坏）

### Known Debt（显式，不静默消失）

- **Debt-J-1**：`workspace/ui/FileEditor.vue` 跨能力 public 依赖 browser，UI-04b-U 仍报 warn（检查器口径差异）。
- **Debt-J-2**：`workspace/manifest.ts`（dependsOn=[]）与 `capabilities.yaml`（dependsOn=[browser,bridge]）不一致（既有）。
- **Debt-J-3**：native/Rust 侧能力（credential=security_policy.rs、脚本/PTY 出生点）未纳入 CB 静态边界门禁。
- **Debt-7A-1**：无按能力的资源实测机制（C4/C5 = 0，DECLARED 口径）。
- **OWNER_PENDING_SCR**：`script` 语义 owner 未裁决（如 notes 迁移前）。
- **HUMAN_VISUAL = PENDING**：本夜不做人工验收。

---

## 10. 结论

- STAGE I（Workbench Decoupling）**PASS**（tag `capability-stage-i-workbench-decoupling-pass`）。
- STAGE J（Platform vNext Closeout）**PASS_WITH_DEBT**（本文件）。
- 允许创建 **`capability-platform-vnext-code-pass`**（CODE PASS）。
- **禁止**创建 `capability-platform-vnext-pass`（Final Human Acceptance 未执行）。
