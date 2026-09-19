# Semantic Closure Audit v1 — Executive Report

> 审计类型：**READ-ONLY Registry Audit**（三层模型：Registry → Code Alignment → Checker Enforcement）
> 不实施 Code Migration，不修改 `src/` `src-tauri/` 业务代码。
> 项目：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> 日期：2026-09-19

## 1. 审计范围

当前 Semantic Registry 的 `registry_scope` 明确为：

```text
Browser/Grid + View Navigation（Phase 1）
+ Workspace/FilePanel（Phase 2）
+ Bookmark（Phase 3）
+ Terminal Lifecycle（Phase 4）
+ Credential Security（Phase 5 / 5.1）
```

本审计在该范围内做 **closure** 核验，并对范围外的新增业务域（git / agent / task / db / graph / session /
vault / image / plugin / resource / settings / home / grid-archive / workbench）做 **gap 标注**（不擅自纳入治理）。

## 2. 三层闭合结论

| 维度 | 范围内（Phase 1–5.1） | 范围外（新增域） |
|---|---|---|
| State | 已闭合（33 个受治理状态，owner/canonical_writer/forbidden_writers 齐备） | 未闭合（大批业务状态无 owner/registry） |
| Intent | 已闭合（canonical 入口 + rejected_intents 红线） | 未闭合（无入口登记） |
| Owner | 已闭合（7 个 owner + R3 通用化到 credential） | 未闭合（无 owner 约束） |
| Side Effect | 大部分闭合（11 项登记 + keyring 机器约束） | 未闭合（script/plugin/git/db/agent 副作用未登记） |
| Checker | 覆盖（R1–R7 + S1/S2） | 不覆盖（范围外无 checker） |

## 3. 关键发现（按严重度）

1. **【P0】语义治理范围未覆盖新增业务域**：git/agent/task/db/graph/session/vault/image/plugin/resource/
   settings/home/grid-archive/workbench 共 14 个 store、~148 个 Tauri 命令、大量副作用**完全不在 Semantic
   Registry 内**，无 owner、无 checker。一旦这些域出现重复语义/越界调用，现有 R1–R7 不会拦截。
   → 见 05-REGISTRY-GAP.md §5、04-SIDE-EFFECT-CLOSURE.md §4。

2. **【P1】范围内少量重复/游离状态**：`aiNavOpen` 在 `useBrowserStore:43` 与 `useLayoutStore:146`
   同时声明（同名双真源嫌疑）；若干 panel 开关布尔（`clipOpen`/`fileEditorOpen`/`browserDockOpen`）
   散落 layout，未统一。→ 见 01-STATE-CLOSURE.md §3、06-OBSERVED-CLASSIFICATION.md。

3. **【P2】副作用登记缺口**：`run_script`/`run_command`（spawn OS 进程）、`plugin_install`（fs+process）、
   `git_*` push（网络）、`db_connect`/`db_query`（network DB）、`agent_*`/`skill_*` install（process+fs）、
   `session_export`（fs）、`vault_open`（fs）均未登记 side-effect。→ 见 04-SIDE-EFFECT-CLOSURE.md §3。

4. **【P2】Checker 盲区**：R3 仅覆盖 Browser/Grid/Credential 三个 owner；Terminal/Bookmark/Workspace 的
   越界模式无对应 rule（Debt-4-2 已知）；R2 只在 `governed_files` 内生效，对范围外 store 完全静默。
   → 见 07-CHECKER-GAP.md。

## 4. 是否过度治理？（独立 Review 结论）

- 范围内：未过度。`observed_not_governed` 中大量条目为合法的实现细节（tree/fileContent/mdHtml/treeRoots
  等纯 UI/编辑缓冲），不应进入 registry。本审计保留它们为 OBSERVED/IMPLEMENTATION_DETAIL，未强行提升。
- 范围外：本审计**拒绝**把 14 个新增域整体塞入 Registry（违反"禁止把所有未知塞入 Registry"）。仅标注 gap，
  是否治理交由人工 SCR 决策（见 08-MIGRATION-PLAN.md）。

## 5. 是否遗漏重要语义？

- 范围内无重大遗漏（credential 三命名空间 + DatabasePanel 敏感输入均已收口）。
- 范围内一处小遗漏：`gridSession`（`useBrowserStore:23`）是前端自增 epoch 代表 Rust webview 生命周期，
  边界权属略含糊（frontend 计数 vs Rust 生命周期），标记为 OBSERVED 待澄清（见 01 §3）。

## 6. 总体结论

```text
SEMANTIC CLOSURE v1:
  范围内（Phase 1–5.1）: CLOSED（有 Registry + Owner + Checker，三处小修可后续 SCR）
  范围外（14 新增域）   : OPEN（无 Registry / Owner / Checker —— 最大残留风险）
```

本审计**停止于 Registry Audit**。迁移计划（08）仅提议，不执行，等待人工确认。

## 7. 产物清单

```text
00-EXECUTIVE-REPORT.md
01-STATE-CLOSURE.md
02-INTENT-CLOSURE.md
03-OWNER-CLOSURE.md
04-SIDE-EFFECT-CLOSURE.md
05-REGISTRY-GAP.md
06-OBSERVED-CLASSIFICATION.md
07-CHECKER-GAP.md
08-MIGRATION-PLAN.md
09-FINAL-DECISION.md
```
