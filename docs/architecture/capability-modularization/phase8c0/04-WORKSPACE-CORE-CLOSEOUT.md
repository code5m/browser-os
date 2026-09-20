# 04 — Workspace Core Closeout（Phase 8C-0 完成）

> Phase 8C-0 目标：把 35KB 单体 `useWorkspaceStore`（God Store）按其真实语义边界分解为独立 owner store。
> 本文件记录 8C-0A..0E 的最终状态、证据与债务，是 Train A 的收口。
> 生成：2026-09-20（Overnight Release Train）。

## 1. 切片账本（每片独立 commit + tag，均可回滚）

| 切片 | 域 | 新 owner | commit | tag |
|------|----|---------|--------|-----|
| 8C-0A | Files | `useFileStore` | `aef8620` | `v8c0a-files-owner` |
| 8C-0B | Artifact（Knowledge/笔记 Vault） | `useArtifactStore` | `a6ca383` | `capability-phase8c0b-artifact-owner-pass` |
| 8C-0C | Repo（仓库配置+同步） | `useRepoStore` | `a6ddf34` | `capability-phase8c0c-repo-owner-pass` |
| 8C-0D | Script / Snippet | `useScriptStore` / `useSnippetStore` | `1388166` | `capability-phase8c0d-script-snippet-owner-pass` |
| 8C-0E | Workspace Core 收敛 | （本片随 0D 提交） | `1388166` | — |

## 2. 最终 owner 矩阵（SECOND_TRUTHS = 0）

| 域 | owner store | 状态归属 |
|----|------------|---------|
| Files | `useFileStore` | 33 存储态 + `currentLocalPath`(派生) |
| Artifact | `useArtifactStore` | tree/current/edit*/selected/flatArtifacts/ctxMenu |
| Repo | `useRepoStore` | repos/form/preview/busy/job |
| Script | `useScriptStore` | scripts/scriptForm |
| Snippet | `useSnippetStore` | snippets/snippetForm |
| Workspace Core | `useWorkspaceStore` | **仅** AUDIT 镜像 + recents（跨域）+ `refresh()` 跨域编排 |
| Layout | `useLayoutStore` | mainView / 面板开关（未动） |
| Browser | `useBrowserStore` | tab/grid（Train C 目标） |
| Terminal | `useSystemStore` | termPanes/...（Train D 目标） |
| Credential | `KeyringStore`(Rust) | 凭据（不变） |

- **UNKNOWN = 0**：单体 store 每个成员都在 `01-WORKSPACE-STORE-INVENTORY.md` 完成分类，并落到上述 owner。
- **SECOND_TRUTHS = 0**：`check-workspace-owners.mjs`（WS-OWNER-01）断言 `useWorkspaceStore` 不再声明任何已抽出子域状态。

## 3. 跨域依赖（显式、单向，无第二真源）

```
useWorkspaceStore.refresh()  ──→  useArtifactStore.loadTree() + useRepoStore.loadRepos() + bridge.auditLog()
useWorkspaceStore.openRecent() ──→ useFileStore.openFile()（url 侧 useBrowserStore）
useRepoStore.requestSync()    ──→ useArtifactStore.selected（跨域读 selection）
useTaskStore.loadTargets()    ──→ useScriptStore.scripts + useSnippetStore.snippets（只读）
useArtifactStore.collectSelection() ──→ useBrowserStore.url/activeTab
```

## 4. 门禁证据（每片均复跑）

- `check-workspace-owners.mjs`（WS-OWNER-01 声明守护 + WS_OWNER_02/06/08 直写守护 + WS_OWNER_09..12）→ PASS
- `check-semantic-registry.mjs` 默认扫描 fail=0 / warn=6（该 6 为历史 side-effect 提示，与本次无关）/ `--self-test` ALL_PASS
- `npm run build` PASS（Rollup 解析全部命名导出）
- `npm run check` GATE PASS（仅既有 known drift）
- `check-capability-boundaries` PASS / `check-capability-composition` PASS / `check-semantic-closure-logic` 27/27 PASS

## 5. 债务（显式，不静默）

- D-8C0-1：AUDIT 仍留在 `useWorkspaceStore`（按 `02-DOMAIN-BOUNDARIES.md` 归 Governance，本阶段不动）。
- D-8C0-2：recents（url+file 跨域）暂留 Workspace Core，待 8C-0E 后续或 Knowledge/Shell 层决定归属。
- D-8C0-3：WS-OWNER 的「直写」守护对 reactive 表单对象（fileCtx/ctxMenu/scriptForm/snippetForm/repo.form）不设 write 规则（v-model 于字段属可接受 UI 态），仅由 WS-OWNER-01 守护其「不发生成第二处声明」。
- D-8C0-4：`useWorkspaceStore` 尚非 capability（无 manifest/contribution/public contract）——属 Train B 范围。

## 6. 下一步（Train B）

Workspace 物理 capability 隔离：`src/capabilities/workspace/`（manifest/public/contributions/core/{files,artifact,repo,script}/adapters）+ Shell 去内部 import + C3 ABSENT 测试。见 `HANDOFF_OVERNIGHT_TRAIN.md`。
