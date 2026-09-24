# Reviewability Matrix（可审核性矩阵）

> Phase 1 交付物（手册 §34 / §49）。
> 真值快照 HEAD：`8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`。
>
> **判定口径**：RV0=需全局上下文；RV1=owner 可导航；RV2=有限审核面；RV3=独立测试面；RV4=独立发布审核。
> 本仓**无机器字段记录 RV**，下表人工派生（依据：单一 owner 清晰度 + entrypoint 明确度 + README 缺失）。

---

## 汇总

| 等级 | 模块 |
|---|---|
| RV2（BOUNDED_REVIEW） | skill, git, workspace（Pilot README §28 Review Surface，审核面有限） |
| RV1（OWNER_NAVIGABLE） | 13 个 M1 能力（browser, terminal, bookmark, credential, database, agent, plugin, graph, vault, task, clipboard, apps, tools, home 中除 Pilot 三模块外的 owner 单一、entrypoint 明确者） |
| RV0（GLOBAL_CONTEXT_REQUIRED） | grid, resource_collection, session, script, workbench（owner 共享/分散/物理缺失） |

> **关键诚实声明**：由于 README 全 D0（无 Review Surface 章节），**没有任何模块能达到 RV2**。
> RV2 要求「README 明确 PRIMARY/SECONDARY/OUT_OF_SCOPE」——该章节不存在。
> 故本表最高只到 RV1，不得高报（手册 §35）。

---

## 逐模块 RV 判定（节选自 DOMAIN-INVENTORY §1）

| 模块 | RV | 依据 |
|---|---|---|
| browser | RV1 | 单一 owner(useBrowserStore) + 明确 entrypoint + 子层结构清晰 |
| grid | RV0 | 与 browser 共享 owner + entrypoint 在 components（上下文需跨 browser） |
| workspace | RV1 | owner(useWorkspaceStore) 明确，但子域 5 owner 需额外导航 |
| terminal | RV1 | 已收口独立 owner(useTerminalStore)，有 checker 固化 |
| bookmark | RV1 | 已进 Semantic Registry，owner 唯一 |
| credential | RV1 | owner=KeyringStore(Rust)，边界经 ACL 固化 |
| database | RV1 | owner 单一 + `check-developer-owners.mjs` 局部固化 |
| git | RV1 | owner 单一 + `check-developer-owners.mjs` 局部固化 |
| agent | RV1 | owner 单一（但后端未实现，审核需知 C1 限制） |
| skill | RV1 | owner 单一 |
| plugin | RV1 | owner 单一（runtime LOCKED 需知） |
| graph | RV1 | owner 单一 |
| vault | RV1 | owner 单一 |
| resource_collection | RV0 | store 在 src/stores，无独立目录/owner 导航面 |
| task | RV1 | owner 单一 |
| clipboard | RV1 | owner 单一 |
| apps | RV1 | owner 单一 |
| tools | RV1 | owner 单一 |
| session | RV0 | store 在 src/stores，关停链路需全局上下文 |
| script | RV0 | 物理在 workspace/ui，owner 与位置分离 |
| workbench | RV0 | store 在 src/stores，无独立目录 |
| settings | RV1 | 框架 SERVICE，entrypoint 明确(src/settings) |
| home | RV1 | owner 单一，但跨 3 能力依赖需知窄契约 |

---

## 提升路径（Phase 1 Pilot 将示范）

- 为 Pilot 三模块补 README §31 REVIEW SURFACE → 达到 RV2（BOUNDED_REVIEW）。
- 推广后由 `check-module-documentation.mjs` 固化 RV 证据。
