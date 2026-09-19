# 06 — observed_not_governed 清理分析

> 对 `states.yaml` 的 `observed_not_governed` 每个条目分类。取值：
> KEEP_AS_OBSERVED / PROMOTE_TO_REGISTRY / MIGRATION_REQUIRED / DEPRECATE / IMPLEMENTATION_DETAIL
> 禁止 UNKNOWN。

## 1. useWorkspaceStore（43 项）

| Field | Classification | Reason |
|---|---|---|
| tree | IMPLEMENTATION_DETAIL | 文件树 UI 数据 |
| current | IMPLEMENTATION_DETAIL | 当前节点 |
| selected (Set) | IMPLEMENTATION_DETAIL | 选择集（见 01-S-15） |
| fileEntries | IMPLEMENTATION_DETAIL | 目录条目 UI |
| fileContent | IMPLEMENTATION_DETAIL | 编辑器缓冲 |
| editingFile | IMPLEMENTATION_DETAIL | 覆盖编辑器开关缓冲 |
| mdPreview / mdHtml | IMPLEMENTATION_DETAIL | Markdown 预览缓冲 |
| startDirs | IMPLEMENTATION_DETAIL | 起始目录列表 UI |
| previewEntries / previewLoading / previewError / previewTileSize | IMPLEMENTATION_DETAIL | 目录预览 UI |
| compareImages | IMPLEMENTATION_DETAIL | 图片对比 UI |
| repos | IMPLEMENTATION_DETAIL | 仓库列表（git 域镜像，范围外） |
| form | IMPLEMENTATION_DETAIL | 表单缓冲 |
| preview (SyncPreview) | IMPLEMENTATION_DETAIL | 同步预览对象（见 01-S-17） |
| busy | IMPLEMENTATION_DETAIL | 异步标志（局部） |
| job / audit | IMPLEMENTATION_DETAIL | 工作区 job/audit 局部态 |
| scripts / snippetForm | IMPLEMENTATION_DETAIL | 脚本/片段编辑器缓冲（scripts 域范围外） |
| snippets | IMPLEMENTATION_DETAIL | 片段列表（scripts 域范围外） |
| recents | OBSERVED (KEEP) | 工作区最近路径，独立生命周期（见 01-S-14），owner=useWorkspaceStore 已存在 |
| flatArtifacts | IMPLEMENTATION_DETAIL | 扁平化物件 UI |
| treeRoots / treeChildren / treeExpanded / treeLoading / treeErrors | IMPLEMENTATION_DETAIL | 文件树 UI |
| locateTarget / dragSource / dropTarget / moveConfirm | IMPLEMENTATION_DETAIL | 拖拽/移动 UI |
| inlineText / inlineHtml / inlineEdit / inlineIsMd | IMPLEMENTATION_DETAIL | 行内编辑器缓冲 |
| fileCtx / ctxMenu | IMPLEMENTATION_DETAIL | 右键菜单 UI |
| editTitle / editTags / editText | IMPLEMENTATION_DETAIL | 编辑表单缓冲 |

## 2. FilePanel.vue（1 项）

| Field | Classification | Reason |
|---|---|---|
| ftreeBody | IMPLEMENTATION_DETAIL | 组件内文件树 DOM ref |

## 3. useBrowserStore（16 项）

| Field | Classification | Reason |
|---|---|---|
| url | OBSERVED (KEEP) | 真实状态，owner=useBrowserStore（browser_tabs），仅未在 states 枚举 |
| tabs | OBSERVED (KEEP) | 同上；activeTabId 已治理，tabs 为其容器 |
| activeTab | OBSERVED (KEEP) | 派生量（states 注释已注明"另有派生 activeTab"） |
| gridSession | OBSERVED (KEEP) + 待澄清 | 前端 epoch 镜像 Rust webview 生命周期（01-S-18/G-03） |
| gridCount | OBSERVED (KEEP) | 浏览器 quad-grid 数，独立语义（≠termGridCount） |
| gridUrl / gridUrls / gridLayout / gridMode / gridRects | OBSERVED (KEEP) | 宫格 webview 前端镜像 |
| resources | OBSERVED (KEEP) | 宫格资源列表镜像 |
| gridAiInput | OBSERVED (KEEP) | AI 导航输入缓冲 |
| aiFilter / aiFiltered | OBSERVED (KEEP) | AI 过滤 UI |
| aiNavOpen | **MIGRATION_REQUIRED** | 与 useLayoutStore:146 双声明（01-S-11/G-01），须合并 |
| recentlyClosed | OBSERVED (KEEP) | 关闭页签历史，独立生命周期（01-S-14） |

## 4. useLayoutStore（19 项）

| Field | Classification | Reason |
|---|---|---|
| sidebarOpen | OBSERVED (KEEP) | 侧栏开关（类 panelOpen） |
| sidebarWidth / fileTreeWidth | IMPLEMENTATION_DETAIL | 布局像素 |
| windowWidth | IMPLEMENTATION_DETAIL | 窗口宽度 |
| navSection / navDensity / navTopViews / leftResizing | IMPLEMENTATION_DETAIL | 导航布局 |
| modTabs / activeModTab / leftTab | IMPLEMENTATION_DETAIL | 模块页签 UI |
| addrMode / compactMode | IMPLEMENTATION_DETAIL | UI 模式 |
| browserDockOpen / browserDockTab | OBSERVED (KEEP) | 浏览器 dock 子表面（01-S-12 候选） |
| clipOpen / fileEditorOpen | OBSERVED (KEEP) | panel 开关布尔（01-S-12 候选） |
| msg | IMPLEMENTATION_DETAIL | 瞬时 toast |

## 5. useBrowserHost（1 项）

| Field | Classification | Reason |
|---|---|---|
| browserHost | IMPLEMENTATION_DETAIL | DOM ref（非业务状态） |

## 6. useSystemStore（5 项）

| Field | Classification | Reason |
|---|---|---|
| clipText | IMPLEMENTATION_DETAIL | 当前剪贴板文本缓冲 |
| clipHistory | OBSERVED (KEEP) | 应用内剪贴板历史（Phase 5.1 Bug-HUNT 改为内存不落盘），owner=useSystemStore |
| apps | IMPLEMENTATION_DETAIL | 应用列表 UI |
| appFilter / filteredApps | IMPLEMENTATION_DETAIL | 应用过滤 UI |

## 7. 汇总

| 分类 | 数量 | 说明 |
|---|---|---|
| IMPLEMENTATION_DETAIL | ~70 | 局部 UI/编辑缓冲/DOM ref，不进 Registry |
| OBSERVED (KEEP) | ~10 | 真实状态、owner 已存在、仅未在 states 枚举；保留观察 |
| MIGRATION_REQUIRED | 1 | `aiNavOpen` 双真源（须合并） |
| PROMOTE_TO_REGISTRY | 0 | 本次未强制提升（OBSERVED 已足够表达"已登记不治理"） |
| DEPRECATE | 0 | 无废弃项 |

> 结论：`observed_not_governed` 设计目标达成——显式登记、不静默消失。仅 1 项（aiNavOpen）需迁移修复；
> 其余为合法的 implementation-detail 或已观察项，无 UNKNOWN。
