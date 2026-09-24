# Ubiquitous Language（通用语言）

> Phase 1 交付物（手册 §8）。消除「同词不同义 / 不同词同义 / 历史别名 / UI 名与 Domain 名混用」。
> **SOURCE** 均为机器真源或代码映射；无法证实写 `UNVERIFIED`。

---

## 核心术语表

| TERM | DEFINITION | DOMAIN | NOT_THE_SAME_AS | CODE_MAPPING |
|---|---|---|---|---|
| Capability | 可独立治理的产品能力单元；经 Runtime 注册/激活 | Runtime | Module / Package / Directory | `src/capabilities/<id>/` + `capabilities.yaml` 条目 |
| Bounded Context | 语义一致性边界；一个 owner 拥有的状态集合 | DDD | Capability（1:N 可能） | `semantic-registry/owners.yaml` 的 `owners.*` |
| Semantic Owner | 某状态/意图的唯一写入者 store 或 Rust 模块 | Semantic | dependsOn（依赖≠拥有） | `owners.*.owner` / `capabilities.yaml:semanticOwner` |
| Contribution Registry | 通用贡献槽机制，MainArea 按槽渲染不 import 内部 | Runtime | import 内部组件 | `src/capability/contribution/registry.ts` |
| WORKBENCH_MAIN 槽 | 主区贡献槽（view='home'/'db'/'settings'...） | Runtime | 直接路由 | `registry.contribute(WORKBENCH_MAIN, {...})` |
| view / mainView | 当前主区视图导航状态 | View Navigation | Capability 本身 | `useLayoutStore.mainView` |
| Grid | 多 webview 宫格工作区（宫格子窗） | Browser/Grid | browser tab | `gridOpen`(useBrowserStore) + `src/components/browser` |
| WebView | 原生浏览器视图实例（持有 native 资源） | Browser | tab（逻辑）/ grid cell | `bridge.createGrid` / webview adapter |
| PTY | 伪终端，Terminal 派生进程 | Terminal | terminal pane（UI） | `useTerminalStore.termPanes` + Rust pty |
| KEYRING | 系统密钥库，凭据唯一真源 | Credential | 前端 reactive 状态 | `KeyringStore`(Rust) / `security_policy.rs` |
| Resource Class | 声明式资源分类（LIGHT..SECURITY_SENSITIVE） | Resource Gov | 实测资源占用 | `resources.yaml:resource_classes` |
| Lifecycle | ACTIVE/SUSPENDED/HIBERNATED/DESTROYED + BACKGROUND | Runtime | status（集成态） | `capabilities.yaml:lifecycle` |
| Hot-Plug(HP) | HP0 静态 … HP3 安装卸载 | Modularity | maturity(C) | 手册 §36（本仓未机器化） |
| Maturity(C) | C0 注册 … C5 资源可释放 | Modularity | status | `capabilities.yaml:maturity`（仅部分声明） |
| Physical(M) | M0 逻辑 … M5 独立仓库 | Modularity | 目录 | 手册 §33（本仓未机器化） |
| Reviewability(RV) | RV0 全局 … RV4 独立发布审核 | Modularity | — | 手册 §34（未机器化） |
| Doc Maturity(D) | D0 无 … D5 独立审核就绪 | Docs | — | 手册 §37（本仓全 D0） |
| SCR | Semantic Change Request（语义变更治理） | Governance | ADR（决策） | `docs/architecture/semantic-changes/` |
| ADR | Architecture Decision Record | Governance | SCR | `docs/architecture/decisions/` |

---

## 碰撞与别名（重分类，详见 CONTEXT-MAP §3）

> **TRUE_SEMANTIC_COLLISIONS = 0**（详见 CONTEXT-MAP §3 裁决；以下均非「相异概念争夺同义」）。

- NAMING_AMBIGUITY（同名异层/异义，需消歧不重命名）：
  - **home（能力）/ home（视图态）**：`useHomeStore`（主页语义）vs `mainView='home'`（导航态）。
  - **session（能力）/ runtime session**：持久化 store vs 浏览器/PTY 运行时会话。
- RELATED_BUT_DISTINCT（相邻但不同，维持 cross-reference）：
  - **repo / git**：`repo`（仓库配置/同步，workspace 子域）≠ `git`（git 版本操作）。
- PHYSICAL_LOGICAL_SEPARATION（逻辑/物理分离，记作 Debt 非碰撞）：
  - **script / ScriptPanel**：逻辑 owner `useScriptStore` 但物理在 `workspace/ui/ScriptPanel.vue`。
- VALID_DERIVATION（同 owner 派生，非碰撞）：
  - **grid（capability）/ gridOpen（状态）**：同 owner(`useBrowserStore`)，实为 browser↔grid 耦合（Debt-7B-1）。
- HISTORICAL_ALIAS（历史别名，已删除）：
  6. **useSystemStore（历史）**：Phase 8E/H 前 Terminal+Clipboard+Apps 混居；现已删除，
     凡文档/代码提及 `useSystemStore` 均属**历史别名**，对应现 `useTerminalStore/useClipboardStore/useAppsStore`。

---

## 使用约束

- UI 名称不得与 Domain 名称混用：组件只消费 store/导航，不持第二份状态真源（见 owners.yaml `violation_patterns`）。
- 发现新碰撞 → 登记于本表 + CONTEXT-MAP §3，**不擅自重命名 frozen semantics**（手册 §8）。
- 本文件为自然语言层；机器真源以 registry YAML 为准，README 不得复制这些事实成第二真源。
