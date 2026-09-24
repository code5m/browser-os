# README Standard（模块文档标准）

> 本文件是 **Phase 1** 建立的「README 标准」，落地自项目治理手册 §21–§38。
> 它是所有 Capability / Workbench Module / Framework Service / Shared Infrastructure /
> Native Module / UI Package 的 **MODULE FRONT DOOR** 强制模板。
>
> 目的：让一个不了解项目的工程师或 Agent，**先读 `capabilities/<id>/README.md`**，
> 就能在有限上下文内理解、修改、审核该模块，而不必 grep 整个仓库。
>
> **本文件不重复手册 §22 的 38 节原文**，只给出「如何填写」与「禁止项」。

---

## 0. 适用范围

凡是以下之一，必须有 `README.md`：

- `capabilities/<id>/`
- 重要 Workbench Module（如 `workspace` 的子域 `files / artifact / repo / script / snippet`）
- `src/capability/`（框架服务）
- `src/shared/`、`src/stores/`、`src/composables/`（共享基础设施）
- `src/components/`（UI 系统）
- `src-tauri/src/` 中的 Native Module（如 `security_policy.rs`）

---

## 1. 必填 38 节（映射到手册 §22）

每个 README 至少包含以下章节（编号对应手册 §22 节号）：

| § | 章节 | 说明 |
|---|---|---|
| 1 | Purpose | 为什么存在 |
| 2 | Domain Classification | CORE_DOMAIN / SUPPORTING_DOMAIN / GENERIC_SUBDOMAIN / WORKBENCH_DOMAIN / FRAMEWORK_SERVICE / SHARED_INFRASTRUCTURE / UI_SYSTEM / NATIVE_ADAPTER / RUNTIME_ENGINE |
| 3 | Responsibilities | 负责什么 |
| 4 | Non-Responsibilities | 明确不负责什么（与 Responsibilities 同等重要） |
| 5 | Ubiquitous Language | 本模块核心术语 |
| 6 | Domain Model | Aggregate / Entity / Value Object / Domain Service / Policy；没有写 `NOT_APPLICABLE` |
| 7 | Invariants | 不可破坏规则（每条有 INV-ID / ENFORCED_BY / TESTED_BY） |
| 8 | State Ownership | STATE / OWNER / WRITER / DERIVED/STORED / PERSISTENCE |
| 9 | Commands / Intents | 改变什么 |
| 10 | Queries | 读取什么 |
| 11 | Events | Produced / Consumed |
| 12 | Public Contract | 允许外部使用什么（`public.ts` 入口） |
| 13 | Internal Boundary | 绝对不能被外部 import 的文件 |
| 14 | Dependencies | Required / Optional / Infrastructure |
| 15 | Dependents | 谁依赖本模块 |
| 16 | Frontend Boundary | UI / state / application 关系 |
| 17 | Native / Backend Boundary | commands / adapter / runtime |
| 18 | Resources | 资源 ownership / lifecycle（引用 `resources.yaml` 口径） |
| 19 | Side Effects | filesystem / network / process / keyring / database / webview / PTY |
| 20 | Permissions / Security | 敏感能力 |
| 21 | Persistence | 存什么 / 存哪里 / 谁恢复 |
| 22 | Failure Model | 失败表现 / 恢复 / 禁止 silent failure |
| 23 | Capability Absence | 模块不存在时系统如何工作（§19 resource absence） |
| 24 | Runtime Lifecycle | register / activate / suspend / disable / destroy |
| 25 | UI Contribution | 贡献哪些槽（Main / Dock / Navigation / Command / Toolbar / Settings） |
| 26 | Testing | unit / contract / capability / composition / runtime / human |
| 27 | Gates | 哪些 checker 保护它（列 `scripts/check-*.mjs`） |
| 28 | Review Guide | 审核应看哪些文件 |
| 29 | AI Modification Guide | 改前必读 / 允许改 / 禁止改 / 改后必跑 |
| 30 | Known Debt | BLOCKING / NON_BLOCKING / FUTURE |
| 31 | Physical Modularity | M0–M5 |
| 32 | Reviewability | RV0–RV4 |
| 33 | Extraction Readiness | NOT_READY / DIRECTORY_READY / PACKAGE_READY / REPOSITORY_CANDIDATE |
| 34 | Package Extraction Notes | |
| 35 | Repository Extraction Notes | |
| 36 | Architecture Decisions | 引用 ADR |
| 37 | Related Documentation | 链接真实文档 |
| 38 | Source of Truth | 明确哪些文件是真源（README 不是第二真源） |

---

## 2. Front Door 规则（§29–§32）

每个成熟模块 README 必须提供 **CODE MAP**（§30）：

```
UI:        ...
State:     ...
Domain:    ...
Application: ...
Public:    ...
Native:    ...
Tests:     ...
Registry:  ...
Resources: ...
Semantic:  ...
```

并明确 **REVIEW SURFACE**（§31，分 PRIMARY / SECONDARY / OUT_OF_SCOPE）与
**AI_CHANGE_SURFACE**（§32，按 UI / DOMAIN / NATIVE / RESOURCE / PUBLIC CONTRACT 变更分别列必读文件）。

---

## 3. 禁止项（来自手册）

- **README 不得成为第二真源**（§23）：不手写易漂移事实（如「148 native commands」），
  改写「Native command ownership source: `docs/architecture/native-boundary/native-commands.yaml`」。
- **不得先写 README 再让代码迎合 README**（§44 STEP 16）。
- **每个结论必须可追到 CODE / REGISTRY / TEST / CHECKER / ADR / SCR**（§45）；无法证明写 `UNVERIFIED` 或 `UNKNOWN`，**不得脑补**。
- **简单模块 README 足够时不拆多余文档**（§26）；复杂模块才加 SECURITY / PERSISTENCE / FAILURE-MODEL / EVENTS / MIGRATION。
- **不得高报成熟度**（§35）：C / HP / M / RV / D / Extraction Readiness 没有证据就填 `UNKNOWN`。

---

## 4. 机器真源引用（写 README 时直接链接，不要复制内容）

- 能力分类 / 生命周期 / 资源：`docs/architecture/capability-registry/{capabilities,dependencies,resources,profiles}.yaml`
- 语义归属 / 越界规则：`docs/architecture/semantic-registry/{owners,states,intents,side-effects}.yaml`
- Native 边界：`docs/architecture/native-boundary/{native-commands.yaml,NATIVE_CAPABILITY_BOUNDARY_AUDIT.md}`
- 门禁清单：`scripts/check-*.mjs`
- 语义变更记录：`docs/architecture/semantic-changes/`、`docs/architecture/semantic-governance/`
