# Overnight Release Train — HANDOFF / 恢复点

> 无人值守长任务的外部长期记忆。**新 Agent（或 context 切换后）先读本文件 + `phase8c0/` 文档 + semantic registry + capability registry，再继续**。
> 生成：2026-09-20（Overnight Release Train）。

## 运行基线

- PROJECT: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
- BRANCH: `feature/capability-platform-v1`
- OVERNIGHT_START_HEAD: `aef86207179d8712b71f74aeefbf4ecad1fe7c89`（= v8c0a-files-owner）
- OVERNIGHT_START_TIME: 2026-09-20 ~21:50
- OVERNIGHT_WORKTREE: clean（起始无未知 WIP）
- OVERNIGHT_FSCK: OK（仅 dangling objects，无 corrupt/missing）
- 硬约束：**不 push / 不 merge master / 不移旧 tag / 不 sudo / 不动用户数据 / 不覆盖已装 /usr/bin/mvp-browser-os**

## 进度

| Train | 范围 | 状态 |
|-------|------|------|
| A | Workspace 单体验证分解（8C-0A..0E） | **PASS**（见 `phase8c0/04-WORKSPACE-CORE-CLOSEOUT.md`） |
| B | Workspace 物理 capability 隔离 + C3 | 进行中/待做 |
| C | Browser/Grid capability 隔离 + C3 | 待做 |
| D | Terminal capability 隔离 + C3 | 待做 |
| E | Developer family（Database/Git） | 待做 |
| F | Resource Governor + Composition Profiles | 待做 |
| G | Final automated acceptance + red team | 待做 |

## 恢复点（annotated tags）

- `v8c0a-files-owner` (aef8620) — 8C-0A Files
- `capability-phase8c0b-artifact-owner-pass` (a6ca383) — 8C-0B Artifact
- `capability-phase8c0c-repo-owner-pass` (a6ddf34) — 8C-0C Repo
- `capability-phase8c0d-script-snippet-owner-pass` (1388166) — 8C-0D/0E Script+Snippet+Core
- `capability-phase8c0-workspace-decomposition-pass` — Train A 收口
- （历史）`capability-phase8b-bookmark-composable-pass` — Bookmark C3

## 现状结构

- 新 owner store：`src/stores/useFileStore.ts` / `useArtifactStore.ts` / `useRepoStore.ts` / `useScriptStore.ts` / `useSnippetStore.ts`
- `src/stores/useWorkspaceStore.ts` = **Workspace Core**（仅 `audit` + `recents` + `refresh()` 跨域编排）
- `src/capability/` = capability runtime（8A 既有）：`index.ts` / `runtime.ts` / `types.ts` / `contribution/{types,registry}.ts`
- `src/capabilities/bookmark/` = 唯一已 C3 的 capability（参考实现）
- 门禁：`scripts/check-workspace-owners.mjs`（WS-OWNER-01..12）、`scripts/check-semantic-registry.mjs`、`scripts/check-capability-boundaries.mjs`、`scripts/check-capability-composition.mjs`
- `scripts/pre-merge.sh` Phase 03 循环已接入 `check-workspace-owners`

## 下一步（Train B 具体清单）

1. 建 `src/capabilities/workspace/`：`manifest.ts` / `public.ts` / `index.ts` / `contributions/` / `core/{files,artifact,repo,script}/` / `adapters/`（**禁止空目录 / 禁止 wrapper**）。
2. Shell 去内部 import：`MainArea.vue` / `ActivityBar.vue` 不再直接 import `useWorkspaceStore`/子域 store，改经 **contribution slot**（复用 `src/capability/contribution`）。
3. **C3 ABSENT 测试**：Workspace 未注册时 Shell 仍启动；Bookmark 等非 Workspace capability 仍成立（不因 MainArea/ActivityBar/bootstrap 硬引用而崩）。
4. 门禁：`check-capability-boundaries`（Shell 零 import capability 内部）、`check-capability-composition`（C3/C4-ABSENT）。
5. 产物：`capability-phase8c-workspace-composable-pass`（CURRENTLY_COMPOSABLE 1→2）。

## 速查命令

```
node scripts/check-semantic-registry.mjs [--self-test]
node scripts/check-workspace-owners.mjs
node scripts/check-capability-boundaries.mjs
node scripts/check-capability-composition.mjs
node scripts/check-semantic-closure-logic.mjs
npm run build   &&   npm run check
bash scripts/pre-merge.sh   # 全门禁（含 cargo，较重）
```

## 继承约束

- 所有 owner 变更：AUDIT → ADR/SCR → REGISTRY → IMPL → CHECKER → TEST，**禁止静默迁移**。
- 禁止 wrapper / 第二状态真源 / shared 垃圾桶 / compatibility facade 永久化。
- 禁止降低门禁强度来换取 PASS。
