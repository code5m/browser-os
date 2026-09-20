# 03 — Owner Migration ADR：Files 域抽出独立 owner（8C-0A）

> 配套 `01-WORKSPACE-STORE-INVENTORY.md` / `02-DOMAIN-BOUNDARIES.md`。
> 本 ADR 记录 8C-0A 的执行决策与契约，确保「改 owner 不静默」——所有迁移、registry 同步、门禁加固均显式落档。
> 生成：2026-09-20。

## 决策（ADR）

将 `useWorkspaceStore`（God Store）中的 **FILES 域** 抽出为独立 owner `useFileStore`，作为文件浏览 / overlay+inline 编辑 / 文件夹树 / 目录图片预览 / 文件右键菜单 / 拖拽移动 的**唯一状态真源**。

- **Workspace Core**（`useWorkspaceStore`）收敛为：Knowledge/Artifact、Script、Snippet、Repo-Sync、Audit、Recents + 跨域编排（`refresh` / `openRecent`）。
- **禁止**：`useWorkspaceStore` 重新声明任何 Files 状态；任何组件/store 直写 Files 状态（须经 canonical writer）。

## 迁移范围

### 状态（33 个存储态 + 1 派生）

治理态（states.yaml 标 owner=useFileStore）：`filePath` `inlineFile` `previewDir` `pathInput` `currentLocalPath`(派生)。

其余列入 `observed_not_governed.useFileStore`：`fileEntries` `fileContent` `editingFile` `mdPreview` `mdHtml` `startDirs` `previewEntries` `previewImages` `previewImageErrors` `previewLoading` `previewError` `previewTileSize` `compareImages` `treeRoots` `treeChildren` `treeExpanded` `treeLoading` `treeErrors` `locateTarget` `dragSource` `dropTarget` `moveConfirm` `fileCtx` `inlineText` `inlineIsMd` `inlineEdit` `inlineHtml`。

### Actions（约 40 个）

`enterDir` `goUp` `goPath` `openFile` `openMd` `saveFile` `openFileInline` `closeInline` `inlineToggleEdit` `saveInline` `locateTo` `locateCurrent` `loadTree` `toggleTreeDir` `ensureTreeChildren` `refreshTree` `expandAllTree` `collapseAllTree` `loadStartDirs` `startDrag` `onDirDragOver` `onDirDragLeave` `onDirDrop` `requestMove` `confirmMove` `cancelMove` `openDirPreview` `setPreviewTileSize` `toggleCompareImage` `moveCompareImage` `clearCompareImages` `isImageEntry` `loadPreviewImage` `onFileContext` `closeFileCtx` `ctxNewFile` `ctxNewDir` `ctxDelete` `ctxRename` `ctxFavorite` `copyPath` `ctxOpenInNewTab` `ctxOpenInExplorer` `ctxOpenInTerminal` `quickNew` + 派生 `currentLocalPath`。

### 新增 canonical writers（FILE-03 防第二真源）

组件只允许经下列入口写 Files 状态（不得直写 ref）：
- `setFileContent(v)`、`setInlineText(v)`、`setEditorView(isPreview)`、`clearDragSource()`
- 地址/文件内容/编辑意图：`enterDir` `openFile` `openFileInline` `openDirPreview` `saveFile` `closeInline`

对应组件侧改造：`FileEditor` 的 `v-model="ws.fileContent"` → `:value`+`@input="ws.setFileContent"`、`mdPreview/editingFile` 直写 → `ws.setEditorView`、`FilePanel` 的 `v-model="ws.inlineText"` → `:value`+`@input="ws.setInlineText"`、`FileTreeNode` 的 `ws.dragSource = ""` → `ws.clearDragSource()`。

## 跨域编排（不引入循环依赖）

- `useWorkspaceStore.openRecent` → 动态 `import("./useFileStore")` 调 `useFileStore().openFile`（repo/recents 仍在 Workspace Core，文件打开归 Files owner）。
- `useWorkspaceStore` 仅**静态**保留 `useFileStore` 的引用用于 openRecent 的运行时调用；未把文件态以 facade 形式回挂，属**真实抽取**而非兼容壳。

## Registry 同步（states / owners / intents）

- `states.yaml`：`owner_implementations` 新增 `useFileStore`（候选路径 `src/stores/useFileStore.ts` + `src/capabilities/workspace/files/state/useFileStore.ts`）；5 个治理态 owner 改 `useFileStore`；`observed_not_governed` 中文件态从 `useWorkspaceStore` 迁移到 `useFileStore`。
- `owners.yaml`：`workspace_filepanel` → `files` owner=`useFileStore`，`owns`/`owner_only_api` 补全；新增 `WS_OWNER_01..04` / `WS_FACADE_01..02` 越界规则文档；`COMPONENT_WRITES_WORKSPACE_PATH` → `COMPONENT_WRITES_FILES_PATH`。
- `intents.yaml`：`openFile` `openFileInline` `enterDir` `saveFile` `closeFileEditor` owner 改 `useFileStore`。

## 新增门禁（WS-OWNER）

- `scripts/check-workspace-files-owner.mjs`：
  - **WS-OWNER-01**：`useWorkspaceStore.ts` 不得再声明 Files-owned state（ref/reactive/computed）。
  - **WS-OWNER-02**：`useFileStore.ts` 之外不得直写 Files 核心态（`filePath`/`inlineFile`/`previewDir`/`pathInput`/`fileContent`/`inlineText`/`mdPreview`/`editingFile`/`dragSource`/`dropTarget`/`moveConfirm`），禁止 `v-model` 直绑、`<alias>.<state> =`、` <state>.value =`。
- 已接入 `pre-merge.sh` Phase 03 checker gate 循环。

## 验收（本切片交付门禁）

- `npm run build` PASS（Rollup 解析所有命名导出，无缺失）。
- `node scripts/check-semantic-registry.mjs` 默认扫描 PASS（fail=0）；`--self-test` ALL_PASS（含新增 `useFileStore` anchor）。
- `node scripts/check-workspace-files-owner.mjs` PASS。
- `useWorkspaceStore` 行数由 ≈1150 降至约 470；Files 域零残留。

## 后续切片（8C-0B..0E，不在本切片）

ARTIFACT(Knowledge) / REPO(+Git) / SCRIPT / SNIPPET / AUDIT / WORKSPACE_CORE 收敛，依 `02-DOMAIN-BOUNDARIES.md` 顺序推进。
