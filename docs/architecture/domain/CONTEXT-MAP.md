# Bounded Context Map（限界上下文地图）

> Phase 1 交付物（手册 §6 / §7）。**证据来源**：`capability-registry/{capabilities,dependencies}.yaml`、
> `semantic-registry/owners.yaml`。上下文**不得预设**，由代码与 registry 裁决。
>
> 真值快照 HEAD：`8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`。

---

## 0. 上下文清单（一句话裁决）

| CONTEXT_ID | NAME | TYPE | OWNER | 集成风格 | 成熟度 |
|---|---|---|---|---|---|
| CTX-BROWSER | Browser/Grid | CORE_DOMAIN | useBrowserStore | OPEN_HOST_SERVICE（webview runtime） | 深度耦合 |
| CTX-WORKSPACE | Workspace(+Files/Artifact/Repo/Script/Snippet) | CORE_DOMAIN | useWorkspaceStore + 5 子 owner | SHARED_KERNEL 已拆解（子域独立 owner） | 部分收口 |
| CTX-TERMINAL | Terminal | SUPPORTING_DOMAIN | useTerminalStore | OPEN_HOST_SERVICE（PTY） | 已收口(Phase 8E) |
| CTX-BOOKMARK | Bookmark | SUPPORTING_DOMAIN | useBookmarkStore | PUBLISHED_LANGUAGE（store 唯一真源） | 已收口(Phase 3) |
| CTX-CREDENTIAL | Credential | FRAMEWORK_SERVICE(SECURITY) | KeyringStore(Rust) | ANTI_CORRUPTION_LAYER（前端只持 opaque id） | 已收口(Phase 5) |
| CTX-DATABASE | Database | SUPPORTING_DOMAIN | useDatabaseStore | CUSTOMER_SUPPLIER（依赖 credential） | C2 |
| CTX-GIT | Git | SUPPORTING_DOMAIN | useGitStore | CUSTOMER_SUPPLIER（依赖 credential/workspace） | C2 |
| CTX-AGENT | Agent | CORE_DOMAIN | useAgentStore | CONFORMIST（可选依赖 graph） | C1(只读壳) |
| CTX-SKILL | Skill | SUPPORTING_DOMAIN | useSkillStore | CONFORMIST | C1 |
| CTX-PLUGIN | Plugin | SUPPORTING_DOMAIN | usePluginStore | OPEN_HOST_SERVICE（runtime LOCKED） | C2 |
| CTX-GRAPH | KnowledgeGraph | SUPPORTING_DOMAIN | useGraphStore | CONFORMIST（可选被 agent 依赖） | C2 |
| CTX-VAULT | Vault | SUPPORTING_DOMAIN | useVaultStore | PUBLISHED_LANGUAGE | — |
| CTX-RESOURCE | ResourceCollection | SUPPORTING_DOMAIN | useResourceStore | SEPARATE_WAYS | NOT_INTEGRATED |
| CTX-TASK | Task | SUPPORTING_DOMAIN | useTaskStore | CUSTOMER_SUPPLIER（依赖 workspace） | C2 |
| CTX-CLIPBOARD | Clipboard | SUPPORTING_DOMAIN | useClipboardStore | PUBLISHED_LANGUAGE | C2 |
| CTX-APPS | Apps | SUPPORTING_DOMAIN | useAppsStore | ANTI_CORRUPTION_LAYER（security_policy 白名单） | C2 |
| CTX-TOOLS | Tools | SUPPORTING_DOMAIN | useToolsStore | OPEN_HOST_SERVICE（tool:// 子 webview） | C2 |
| CTX-SESSION | Session | FRAMEWORK_SERVICE | useSessionStore | SEPARATE_WAYS（关停链路） | NOT_INTEGRATED |
| CTX-SCRIPT | Script | SUPPORTING_DOMAIN | useScriptStore | SHARED_KERNEL 弱化（驻 workspace/ui） | NOT_INTEGRATED |
| CTX-WORKBENCH | Workbench | WORKBENCH_DOMAIN | useWorkbenchStore | PUBLISHED_LANGUAGE | NOT_INTEGRATED |
| CTX-SETTINGS | Settings | FRAMEWORK_SERVICE | useSettingsStore | PUBLISHED_LANGUAGE | COMPAT_WRAPPED |
| CTX-HOME | Home | WORKBENCH_DOMAIN | useHomeStore | CUSTOMER_SUPPLIER（依赖 browser/workspace/apps） | COMPAT_WRAPPED |
| CTX-RUNTIME | Capability Runtime / Platform | RUNTIME_ENGINE | framework | OPEN_HOST_SERVICE（全局） | — |
| CTX-SHARED | Shared Infra（bridge/pinia/security_policy/shell/shared） | SHARED_INFRASTRUCTURE | framework | SHARED_KERNEL（受限：仅 4 节点） | — |

---

## 1. 关键关系（UPSTREAM / DOWNSTREAM，来自 dependencies.yaml edges）

- **桥接中枢**：几乎所有能力 → `bridge`（ADAPTER，唯一 Native 通道）。下游即 Rust native commands。
- **凭据扇入**：`database`、`git` 必须 → `credential`（keyring 访问）。Credential 不反向依赖任何业务能力（forbidden_edges 固化）。
- **Workspace 扇出**：`git → workspace`（repo 配置）、`task → workspace`（脚本/目标）、`home → workspace`（收藏目录经 shared 窄缝）。
- **Browser 扇出**：`bookmark`、`workspace`、`grid`、`home` 依赖 browser（视图态）。其中 `home→workspace` 已存在，**禁止**再声明 `workspace→home`（否则 CB-04 必须依赖环）→ 经 `src/composables/homeNav.ts` 窄缝。
- **Grid 强依赖 Browser**：`grid → browser → bridge`；Grid 资源（多 webview）归母属 `useBrowserStore`（Debt-7B-1，暂不拆）。
- **Session 关停隔离**：`session` 不得依赖 `browser` / `grid`（forbidden_edges，避免关闭顺序耦合）。

---

## 2. 防腐层 / 窄缝（ANTI_CORRUPTION_LAYER，已建）

| 缝 | 路径 | 作用 | 避免的坏味道 |
|---|---|---|---|
| homeNav | `src/composables/homeNav.ts` | Workspace「收藏目录」经 Home public 边界，不 import Home | workspace→home 依赖环(CB-04) |
| terminalNav | `src/composables/terminalNav.ts` | Workspace「在终端打开」经 Terminal public 边界 | 声明 required 边破坏 absent 可启动 |
| browserNav / recentsNav | `src/composables/` | 跨能力导航只读消费 | 直写 mainView（越界） |
| security_policy | `src-tauri/src/security_policy.rs` | apps.launch / credential 经白名单+审计 | 任意命令执行 / 凭据泄露 |
| Contribution Registry | `src/capability/contribution/registry.ts` | MainArea 按槽渲染，不 import 能力内部 | 能力内部被 Shell 直耦 |

---

## 3. 语义关系登记（独立 Review 重分类：非全部为碰撞）

> 独立 Review 重新裁决（手册 §5 / §8）：以下 5 项**没有一项是真正的「语义碰撞」**
> （即两个相异概念争夺同一名称/含义）。它们分属「相邻但不同 / 同名异层 / 同 owner 派生 / 逻辑-物理分离」。
> 故 **TRUE_SEMANTIC_COLLISIONS = 0**；逐项按精确分类登记，避免为「找问题」而制造碰撞（手册 §5 红线）。

1. **repo vs git** → RELATED_BUT_DISTINCT（相邻但不同）：`useRepoStore`（仓库配置/同步）≠ `useGitStore`（git 版本操作）。
   同在「版本控制」语义邻域，但操作与 owner 均不同；owners.yaml `repo` 条目已明确「REPO context ≠ GIT operation；token 走 keyring」。无需重命名，仅维持 cross-reference 注释。
2. **script（owner）vs workspace ScriptPanel（物理位置）** → PHYSICAL_LOGICAL_SEPARATION（逻辑/物理分离）：
   `script` 是独立语义 owner(`useScriptStore`)，但 entrypoint 仍挂在 `workspace/ui/ScriptPanel.vue`，物理未独立（M0）。这是物理漂移（Debt），非语义碰撞。
3. **home（能力）vs mainView='home'（导航态）** → NAMING_AMBIGUITY（同名异层）：`home` 能力（useHomeStore，主页语义）vs `useLayoutStore.mainView='home'`（导航状态）。
   同名不同层，UI 名称与 Domain 名称混用风险，需在文档/代码注释持续消歧，但不重命名 frozen semantics。
4. **session（能力）vs runtime session** → NAMING_AMBIGUITY（同名异义）：`useSessionStore`（持久化/恢复）vs 浏览器/PTY 运行时会话态。概念不同、同名，需区分。
5. **grid（capability）vs gridOpen（状态）** → VALID_DERIVATION（同 owner 派生，非碰撞）：`grid` 上下文与 `useBrowserStore` 拥有的 `gridOpen/grid lifecycle` 本就同属一个生命周期域（同 owner）；所谓「分裂」实为 browser↔grid 深度耦合（Debt-7B-1），是设计耦合问题，不是两个相异概念的语义碰撞。

> 以上均不擅自重命名 frozen semantics（手册 §8）；仅按精确分类登记，供后续 ADR/SCR 参考。

---

## 4. 集成风格约定（§7）

- **默认禁止 SHARED_KERNEL**：仅 `bridge/pinia/security_policy/shell` 4 个共享基础设施节点允许被任意依赖。
- 两能力共享业务 state → 优先认为边界设计有问题（如 browser/grid 共享 owner 已登记为 Debt-7B-1）。
- 跨能力写操作一律经 **public 边界 + 窄缝**，不得跨 import 内部 store（owner 越界由 `check-semantic-registry.mjs` 机检）。
