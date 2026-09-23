# NOT-INTEGRATED-DECISION-MATRIX（STAGE H-E）

> 真源：`docs/architecture/capability-registry/capabilities.yaml`（本次**重新扫描**，不沿用旧清单）。
> 全量 **23** 条能力，其中 `status: NOT_INTEGRATED` = **6** 条。
> 目标**不是** `NOT_INTEGRATED = 0`，而是每一条都有正确理由；**UNKNOWN = 0**。

## 1. 全量扫描结果

```text
TOTAL capabilities        23
NOT_INTEGRATED             6   → grid, credential, resource_collection, session, script, workbench
COMPATIBILITY_WRAPPED     17   → browser, workspace, terminal, bookmark, database, git, agent, skill,
                                 plugin, graph, vault, task, clipboard, apps, tools, settings, home
UNKNOWN                    0
```

## 2. 逐项裁决

| ID | CURRENT | DECISION | RATIONALE（真实证据） | RELEASE IMPACT | NEXT ACTION |
|---|---|---|---|---|---|
| **grid** | NOT_INTEGRATED | **VALID_SUB_CAPABILITY** | 与 browser **共 owner**（`useBrowserStore`）；grid 是 browser 的宫格视图，无独立 store/UI 包；`resources.yaml` 有独立 grid policy（VERY_HEAVY/MULTI_WEBVIEW）以保留资源治理面。RRA-01/07 已证明其资源出生点唯一（`create_grid`）且 DESTROY 生效 | NON_BLOCKING | 保持子能力登记；在文档固化「grid ≡ browser.grid view」 |
| **credential** | NOT_INTEGRATED | **SHARED_INFRASTRUCTURE** | 实现在 Rust `src-tauri/src/security_policy.rs`（KeyringStore），**无前端能力包**；`resources.yaml` 标 SECURITY_SENSITIVE 且 `destroyable: false`（安全边界常驻）。按 §13：安全基础设施 ≠ 用户可组合能力 | NON_BLOCKING | 在 §13 口径中固化为 SHARED_INFRASTRUCTURE，避免被误当普通能力 |
| **resource_collection** | NOT_INTEGRATED | **ACCEPTED_DEBT** | 资源遥测/采集由后端与少量 `src/stores` 承担，无 `capabilities/resource_collection/` 包；无用户可组合 UI，缺失不影响启动与装配（framework 装配 PASS） | NON_BLOCKING | FUTURE：若要用户可见的资源面板再建包 |
| **session** | NOT_INTEGRATED | **VALID_FRAMEWORK_SERVICE** | 会话保存/恢复/flush 属**关停链路框架服务**（`resources.yaml`: `resident: true`, `destroyable: false`）；native 侧 `session_*` 命令归 session，但前端无能力包 | NON_BLOCKING | 归类为框架服务（与 settings 同类），不并入用户能力矩阵 |
| **script** | NOT_INTEGRATED | **VALID_SUB_CAPABILITY** | **H-A 已收口**：语义 owner = `useScriptStore`，`governanceStatus: GOVERNED`；物理宿主在 `src/capabilities/workspace/`（store 在 `workspace/state`、UI 在 `workspace/ui`、由 workspace 贡献 `view='scripts'`）。**不因 owner 已裁决而错误升格为独立 Capability** | NON_BLOCKING | 保持 NOT_INTEGRATED 直到（若）拆出独立包 |
| **workbench** | NOT_INTEGRATED | **VALID_WORKBENCH_SERVICE** | `provides: [workbench.summary, workbench.artifact]`，store 仍在 `src/stores/useWorkbenchStore.ts`，属框架汇总服务；无用户可组合 UI | NON_BLOCKING | 与 settings/session 同列框架服务 |

## 3. 汇总

| 分类 | 数量 | 成员 |
|---|---|---|
| MUST_MIGRATE_BEFORE_RC | **0** | — |
| VALID_SUB_CAPABILITY | 2 | grid, script |
| VALID_FRAMEWORK_SERVICE | 1 | session |
| VALID_WORKBENCH_SERVICE | 1 | workbench |
| SHARED_INFRASTRUCTURE | 1 | credential |
| ACCEPTED_DEBT | 1 | resource_collection |
| FUTURE_CAPABILITY | **0**（resource_collection 若建面板则转入） | — |
| **UNKNOWN** | **0** | — |

**BLOCKING debt = 0** —— 无一条必须迁移才能发布 RC。

> 诚实声明：6 条未集成项**不是被忽略**，而是各自属于「子能力 / 框架服务 / 安全基础设施 / 已登记债务」四类之一，
> 且 grid/script 的关键资源与 owner 已分别由 RRA 门禁与 H-A SCR 覆盖。
