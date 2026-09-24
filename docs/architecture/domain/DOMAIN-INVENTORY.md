# Repository Domain Inventory（全仓模块清单）

> Phase 1 交付物（手册 §43）。**所有字段以机器真源为据**：
> `capability-registry/{capabilities,dependencies,resources}.yaml`、`semantic-registry/owners.yaml`、
> `native-boundary/native-commands.yaml`。
> 未证实项一律标 `UNKNOWN`，**不脑补、不高报**（手册 §35 / §45）。
>
> 真值快照 HEAD：`8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`（feature/capability-platform-v1）。

---

## 0. 统计

- 能力（CAPABILITY）：22 个
- 框架服务（SERVICE）：1 个（settings）
- 框架/运行时层：`src/capability/`（RUNTIME_ENGINE）
- 共享基础设施：`src/shared/ src/stores/ src/composables/ src/components/ src/settings/ src/utils/ src/styles/`
- **README 现状：23 个能力目录 README = 0 → Documentation Maturity 全部 D0**（见矩阵）。
- **语义 owner 已进 Semantic Registry 的仅 12 个域**（view_navigation / browser_grid / native_execution / browser_tabs / files / artifact / repo / script / snippet / bookmark / terminal / credential）；其余能力的 `semanticOwner` 仅在 `capabilities.yaml` 登记、**未进语义治理 Registry** → 记 `UNKNOWN`（见 §99）。

---

## 1. 模块清单（CAPABILITY / SERVICE）

| ID | Name | Class(判定) | semanticOwner | status | provides | dependsOn | optDeps | resources | C | HP* | M | RV | D | Ext.Readiness | KNOWN_DEBT |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| browser | Browser | CORE_DOMAIN | useBrowserStore | COMPAT_WRAPPED | navigate/tab.open/tab.close/host/grid | bridge | — | HEAVY,WEBVIEW,NATIVE | UNKNOWN | HP1 | M1 | RV1 | D0 | NOT_READY | Debt-7B-1(browser↔grid); webview 不可物理卸载 |
| grid | Grid | CORE_DOMAIN | useBrowserStore | NOT_INTEGRATED | open/layout/archive | browser,bridge | — | VERY_HEAVY,MULTI_WEBVIEW,NATIVE | UNKNOWN | HP0 | M0 | RV0 | D0 | NOT_READY | 与 browser 深度耦合(Debt-7B-1); entrypoint 在 src/components/browser |
| workspace | Workspace | CORE_DOMAIN | useWorkspaceStore | COMPAT_WRAPPED | files/artifact/repo/script/snippet/main-view | — | browser | LIGHT,MEDIUM | UNKNOWN | HP1 | M1 | RV1 | D0 | NOT_READY | 子域分散(files/artifact/repo/script/snippet 各有专属 owner) |
| terminal | Terminal | SUPPORTING_DOMAIN | useTerminalStore | COMPAT_WRAPPED | spawn/write/kill/resize/grid | bridge | — | PROCESS,PTY | UNKNOWN | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | PTY 不能安全冻结(destroyable=true 但 release TARGET) |
| bookmark | Bookmark | SUPPORTING_DOMAIN | useBookmarkStore | COMPAT_WRAPPED | list/add/remove/togglePanel | — | browser | LIGHT | UNKNOWN | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | 已进 Semantic Registry(ACCEPTED) |
| credential | Credential | FRAMEWORK_SERVICE(SECURITY) | KeyringStore(Rust) | NOT_INTEGRATED | save/get/delete | bridge | — | SECURITY_SENSITIVE | UNKNOWN | HP0 | M0 | RV1 | D0 | NOT_READY | owner 在 Rust; 前端零明文(已收口 Debt-5-2) |
| database | Database | SUPPORTING_DOMAIN | useDatabaseStore | COMPAT_WRAPPED | connect/query/cancel | credential,bridge | — | NETWORK,SECRET | C2 | HP0 | M1 | RV1 | D0 | DIRECTORY_READY | store 仍在 src/stores(未迁入); 缺 absence 运行时门禁; mainView 硬编码 |
| git | Git | SUPPORTING_DOMAIN | useGitStore | COMPAT_WRAPPED | status/diff/log/commit/write | credential,workspace,bridge | — | MEDIUM,NETWORK | C2 | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | 写操作 spawn git 子进程 |
| agent | Agent | CORE_DOMAIN | useAgentStore | COMPAT_WRAPPED | chat/run/memory | bridge | graph | MEDIUM,NETWORK | C1 | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | 后端未实现(只读壳); 持有 skill 状态待拆 |
| skill | Skill | SUPPORTING_DOMAIN | useSkillStore | COMPAT_WRAPPED | list/run/install | bridge | — | LIGHT | C1 | HP0 | M1 | RV1 | D0 | DIRECTORY_READY | activatable=false; 无专属 store 时被 agent 持有 |
| plugin | Plugin | SUPPORTING_DOMAIN | usePluginStore | COMPAT_WRAPPED | list/manifest | bridge | — | HEAVY,NATIVE | C2 | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | runtime LOCKED(不解包/不验签/不执行) |
| graph | KnowledgeGraph | SUPPORTING_DOMAIN | useGraphStore | COMPAT_WRAPPED | query/node.get/stats | bridge | agent | MEDIUM | C2 | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | 只读内存快照 |
| vault | 笔记库 | SUPPORTING_DOMAIN | useVaultStore | COMPAT_WRAPPED | open/search/follow | bridge | browser | LIGHT | UNKNOWN | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | 只读 Markdown 快照,无原生资源 |
| resource_collection | ResourceCollection | SUPPORTING_DOMAIN | useResourceStore | NOT_INTEGRATED | capture/settings | bridge | — | MEDIUM,BACKGROUND | UNKNOWN | HP0 | M0 | RV0 | D0 | NOT_READY | store 在 src/stores(无独立目录) |
| task | Task | SUPPORTING_DOMAIN | useTaskStore | COMPAT_WRAPPED | schedule/run/cancel | workspace,bridge | script | BACKGROUND | C2 | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | 调度线程在 Rust(scheduler.rs) |
| clipboard | Clipboard | SUPPORTING_DOMAIN | useClipboardStore | COMPAT_WRAPPED | read/write/history | bridge | — | LIGHT | C2 | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | 历史不落盘(session,B11-1) |
| apps | Apps | SUPPORTING_DOMAIN | useAppsStore | COMPAT_WRAPPED | list/launch | bridge | — | LIGHT,PROCESS | C2 | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | launch 经 security_policy 白名单 |
| tools | Tools | SUPPORTING_DOMAIN | useToolsStore | COMPAT_WRAPPED | list/open | bridge | — | MEDIUM,WEBVIEW | C2 | HP1 | M1 | RV1 | D0 | DIRECTORY_READY | 子 webview(tool://)，零隔离 |
| session | Session | FRAMEWORK_SERVICE | useSessionStore | NOT_INTEGRATED | persist/restore | bridge | — | LIGHT | UNKNOWN | HP0 | M0 | RV0 | D0 | NOT_READY | 关停链路; store 在 src/stores |
| script | Script | SUPPORTING_DOMAIN | useScriptStore | NOT_INTEGRATED | run/snippet | bridge | — | PROCESS | UNKNOWN | HP0 | M0 | RV0 | D0 | NOT_READY | entrypoint=workspace/ui/ScriptPanel.vue(无独立目录) |
| workbench | Workbench | WORKBENCH_DOMAIN | useWorkbenchStore | NOT_INTEGRATED | summary/artifact | bridge | — | MEDIUM | UNKNOWN | HP0 | M0 | RV0 | D0 | NOT_READY | store 在 src/stores |
| settings | Settings | FRAMEWORK_SERVICE | useSettingsStore | COMPAT_WRAPPED | theme/keymapScheme/tabHibernation | — | — | LIGHT | UNKNOWN | HP0 | M1 | RV1 | D0 | NOT_READY | 框架偏好 SERVICE,非用户可组合能力 |
| home | 主页 | WORKBENCH_DOMAIN | useHomeStore | COMPAT_WRAPPED | shortcut/recent/launch/favorite | browser,workspace,apps | — | LIGHT | UNKNOWN | HP1 | M1 | RV1 | D0 | NOT_READY | 跨 3 能力依赖; 窄契约 favoriteDirectory |

\* HP 由 `lifecycle.activatable` 派生：true→HP1(RUNTIME_ENABLE_DISABLE)；`NOT_INTEGRATED`/activatable=false→HP0(STATIC)。

---

## 2. 框架 / 运行时层（RUNTIME_ENGINE）

| 模块 | 路径 | 职责 | 备注 |
|---|---|---|---|
| Capability Runtime | `src/capability/runtime.ts` / `runtimeSingleton.ts` | 能力注册/激活/生命周期编排 | 注意：`activate` 不调 `def.registerContributions`（见 home 修复注释）→ 贡献须模块加载自注册 |
| Resource Governor | `src/capability/resourceGovernor.ts` | 资源归属/释放治理（§19） | 与 `resources.yaml` 口径对应 |
| Contribution Registry | `src/capability/contribution/registry.ts` | 通用贡献槽（WORKBENCH_MAIN / REPO_SUBVIEW 等） | MainArea 只按槽渲染、不 import 内部 |
| Platform | `src/capability/platform/` | 平台适配 | — |
| Profiles | `src/capability/profiles.ts` | 组合剖面（composition profiles） | 对应 `capability-registry/profiles.yaml` |

---

## 3. 共享基础设施（SHARED_INFRASTRUCTURE）

| 模块 | 路径 | 角色 |
|---|---|---|
| bridge | `src/bridge.ts` | 唯一 Native 通道（ADAPTER），48 处 invoke；绕过即违规 |
| pinia | `src/main.ts` | 状态容器宿主 |
| security_policy | `src-tauri/src/security_policy.rs` | 安全边界（程序黑名单 / 敏感数据） |
| shell | `src/App.vue` + `useLayoutStore` | 窗口 / 布局 / 导航 |
| shared | `src/shared/` | 跨能力窄缝（homeNav / terminalNav / browserNav / recentsNav） |
| stores | `src/stores/` | 仍驻留的 store（useResourceStore / useSessionStore / useWorkbenchStore / useDatabaseStore 等） |
| composables | `src/composables/` | 组合式逻辑 |
| components | `src/components/` | UI 系统（含 browser 组件、grid 组件） |
| settings | `src/settings/` | 框架偏好 public 契约 |
| utils / styles | `src/utils/` `src/styles/` | 通用工具 / 样式 |

---

## 99. DOMAIN_INVENTORY_UNKNOWN（必须趋零的缺口）

1. **语义 owner 未进 Semantic Registry**：`capabilities.yaml` 中除上方 12 个已收口域外，
   其余能力（database/git/agent/skill/plugin/graph/vault/task/clipboard/apps/tools/workspace/
   home/resource_collection/session/script/workbench/settings）的 `semanticOwner` 仅登记、
   **未进 `semantic-registry/owners.yaml` 的越界治理** → 这些 owner 的 `forbidden_callers` /
   `authorized_callers` 未被机器固化（仅 database/git 有 `check-developer-owners.mjs` 局部固化）。
2. **maturity(C) 未声明**：仅 10 个能力声明 `maturity`（C1×2、C2×8），其余 13 个 `UNKNOWN`。
3. **HP/M/RV/D 未声明**：仓库无机器字段记录 HP/M/RV/D，本表为**人工派生**（依据 lifecycle / 目录结构 / README 缺失），需后续固化到 registry。
4. **资源实测缺失**：`resources.yaml` 全为 DECLARED 口径，`measured_memory_per_capability: NOT_AVAILABLE`（Debt-7A-1）→ 资源释放收益为「目标」非「已测」。
5. **Extraction Readiness 未机器化**：本表判定基于目录隔离 + 单一 owner + 窄契约，缺 tests/review-surface 实证（README 全 D0）。
6. **Native owner 未逐能力映射到本表**：`native-commands.yaml`(1517 行) 按能力登记 native 命令归属，
   但本 Inventory 未逐行展开（避免编造）；Native Boundary 列以「见 native-commands.yaml」为权威引用。

> 以上 UNKNOWN 不构成「第二真源」，只是诚实记录待治理面；Phase 1 不修改这些 registry（红线禁止改 semantic owner / capability maturity）。
