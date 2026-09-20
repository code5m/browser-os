# 02 — Domain Boundaries（Phase 8C-0 裁决）

> 配套 `01-WORKSPACE-STORE-INVENTORY.md`。本文件给出每个真实域的 owner 成立依据与 8C-0 切片顺序。
> 裁决原则（接管指令 §5）：不止按名字拆，须满足「独立状态 + 独立 intent + 独立 lifecycle/persistence/side-effect/native-dep + 可定义 public contract」中的若干项。

## 真实域清单（7 + 1 跨域）

| Domain | 是否独立 owner | 成立依据 | 当前 store 内成员 | 8C-0 切片 |
|--------|--------------|----------|------------------|-----------|
| **FILES** | ✅ 独立 | 完整文件子域：浏览/编辑/树/目录预览/行内编辑/右键/拖拽移动；独立 FS side-effect；被 FilePanel/FileTreeNode/FileEditor 独占消费 | 见 01 表 #7–#57 中 FILES + IMPLEMENTATION_DETAIL | **8C-0A（本切片）** |
| **ARTIFACT（笔记 Vault / Knowledge）** | ✅ 独立（后续） | `tree/current/edit*/selected/flatArtifacts/collectSelection/ctxMenu*`；独立后端持久化（browseWorkspace/readArtifact）；与 Files 零共享状态 | #1–#6,#40,#50(部分),#51,#ctxMenu 系动作 | 8C-0E 之后，归 Knowledge Capability |
| **REPO（仓库 + 同步）** | ✅ 独立（后续） | `repos/form/preview/busy/job` + `saveRepo/requestSync/confirmSync/onSyncCompleted/loadGiteeExample`；独立后端持久化（listRepos/configureRepo）；凭据走 keyring（credential owner 已治理） | #28–#32,Repo 系动作 | 8C-0C（与 Git 关系见下） |
| **SCRIPT** | ✅ 独立（后续） | `scripts/scriptForm` + CRUD；后端持久化（scriptList/add/update/remove） | #34–#35,Script 系动作 | 归 Developer Capability Family（8F），本 8C-0 不动 |
| **SNIPPET** | ✅ 独立（后续） | `snippets/snippetForm` + CRUD；后端持久化 | #36–#37,Snippet 系动作 | 同 SCRIPT（8F） |
| **AUDIT** | ✅ 独立（后续） | `audit`（审计日志镜像） | #33 | 归 Governance，本 8C-0 不动 |
| **CROSS_DOMAIN（recents）** | 暂留 Workspace Core | `recents`（url+file 混合，localStorage）；`openRecent/addRecentUrl/addRecentFile` | #38,recents 系动作 | 8C-0E 决定（不进 shared 垃圾桶） |
| **WORKSPACE_CORE** | ✅ 收敛目标 | 仅自身语义（身份/选择/组合）+ 跨域编排（`refresh`）；不再持有子域内部状态 | `refresh`,`recents`(过渡),`enterDirCompat`(过渡) | 8C-0E |

## 接管指令候选域的更正裁定

- **Editor**：不是独立域。overlay 编辑（`filePath/fileContent/editingFile/mdPreview/mdHtml`）与 inline 编辑（`inlineFile/inlineText/inlineIsMd/inlineEdit/inlineHtml`）都是 Files 的**编辑表面**，与 Files 同 owner。→ 并入 FILES。
- **Vault**：指令认为可能是独立 persistence 域；实测「笔记 Vault」= ARTIFACT 域（`tree/current/edit*`），**不在**本 store 的待抽部分，归 Knowledge Capability（8F/G 之后）。
- **Git**：不在本 store（GitPanel 经 bridge git 命令，无 store 态）。→ 不归 8C-0 抽取；Repo 与 Git 的边界在 8C-0C 裁定（Repo=仓库上下文/凭据；Git=版本操作，走 bridge）。
- **Repo vs Git**：本 store 只有 Repo 配置/同步态，**没有** Git status/branch/commit 态 → 本阶段只抽 REPO owner；Git 留待 8C-0C 与 GitPanel 一并裁决。

## 8C-0 切片顺序（接管指令 §2 无人值守）

```
8C-0A  FILES          ← 本切片（最大且最自洽，独占消费）
8C-0B  ARTIFACT       笔记 Vault / Knowledge（tree/current/edit*/collectSelection）
8C-0C  REPO(+Git 裁决) 仓库配置 + 同步；Git 边界裁定
8C-0D  SCRIPT/SNIPPET 脚本/片段库（或归 8F Developer Family）
8C-0E  WORKSPACE_CORE  收敛 refresh / recents / 过渡别名，删空 God Store
```

每个切片遵循：Audit(已做) → ADR(本目录) → Registry(states/owners/intents/side-effects) → Code migration → Checker → Build → Commit → Tag(稳定点)。

## 禁止事项（继承 + 8C-0 新增）

- 禁止把整个 `useWorkspaceStore` 搬到 `capabilities/workspace/` 当 wrapper（验收 Q1 驳回）。
- 禁止第二状态源：Files state 只在 `useFileStore`，`useWorkspaceStore` 不得重新声明同义 ref。
- 禁止 shared 垃圾桶（`shared/business.ts` / `shared/workspaceHelpers.ts`）。recents 不进 shared，留 Workspace Core 过渡。
- 禁止为迁移降低 Semantic Governance 门禁（registry/closure 仍 PASS）。
- 禁止顺手清 Terminal/Grid debt、改 credential 边界、改 mainView 导航。
- 禁止 push / merge master / 移动旧 tag。
