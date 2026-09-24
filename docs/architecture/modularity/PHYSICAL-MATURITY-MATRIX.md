# Physical Modularity Matrix（物理模块化矩阵）

> Phase 1 交付物（手册 §33 / §49）。
> 真值快照 HEAD：`8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`。
>
> **判定口径**：M0=仅逻辑；M1=目录隔离；M2=包隔离；M3=可独立构建；M4=可独立版本；M5=独立仓库。
> 本仓**无机器字段记录 M**，下表人工派生（依据：`src/capabilities/<id>/` 目录结构 + `capabilities.yaml:entrypoint`）。

---

## 汇总

| 等级 | 模块 |
|---|---|
| M1（DIRECTORY_ISOLATED） | browser, workspace, terminal, bookmark, database, git, agent, skill, plugin, graph, vault, task, clipboard, apps, tools, home, settings |
| M0（LOGICAL_ONLY） | resource_collection, session, workbench, script |

> `browser/` 已具完整子层（`manifest.ts/public.ts/index.ts/ui/state/resource/`）→ M1 进阶；
> `settings` 在 `src/settings/`（独立目录）→ M1；
> `resource_collection/session/workbench` 的 store 仍驻 `src/stores/`（无专属目录）→ M0；
> `script` 的 entrypoint 为 `workspace/ui/ScriptPanel.vue`（逻辑 owner 与物理位置分离）→ M0。

---

## 逐模块 M 判定（节选自 DOMAIN-INVENTORY §1）

| 模块 | M | 物理依据 |
|---|---|---|
| browser | M1 | `src/capabilities/browser/{manifest,public,index}.ts` + `ui/ state/ resource/` |
| grid | M0 | entrypoint=`src/components/browser`（未独立目录） |
| workspace | M1 | `src/capabilities/workspace/` 含子域 ui |
| terminal | M1 | `src/capabilities/terminal/` |
| bookmark | M1 | `src/capabilities/bookmark/` |
| credential | M0 | entrypoint=`src-tauri/src/security_policy.rs`（Rust，无前端目录） |
| database | M1 | `src/capabilities/database/` |
| git | M1 | `src/capabilities/git/` |
| agent | M1 | `src/capabilities/agent/` |
| skill | M1 | `src/capabilities/skill/` |
| plugin | M1 | `src/capabilities/plugin/` |
| graph | M1 | `src/capabilities/graph/` |
| vault | M1 | `src/capabilities/vault/` |
| resource_collection | M0 | entrypoint=`src/stores/useResourceStore.ts` |
| task | M1 | `src/capabilities/task/` |
| clipboard | M1 | `src/capabilities/clipboard/` |
| apps | M1 | `src/capabilities/apps/` |
| tools | M1 | `src/capabilities/tools/` |
| session | M0 | entrypoint=`src/stores/useSessionStore.ts` |
| script | M0 | entrypoint=`src/capabilities/workspace/ui/ScriptPanel.vue` |
| workbench | M0 | entrypoint=`src/stores/useWorkbenchStore.ts` |
| settings | M1 | `src/settings/` |
| home | M1 | `src/capabilities/home/` |

---

## 说明

- 全仓**无 M2+**（无独立 package.json 拆分为子包），符合「规模小留 Monorepo」（手册 §63）。
- 提升 M 等级需先完成 D3 文档 + 公共契约稳定 + review surface 有限（手册 §38 / §39），Phase 1 不做物理迁移。
- `script` 的 M0 与独立 owner(`useScriptStore`) 冲突，是待治理的物理漂移（见 CONTEXT-MAP §3.2）。
