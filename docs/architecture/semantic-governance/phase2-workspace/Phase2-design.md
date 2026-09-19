# Phase 2 — Workspace / FilePanel Semantic Governance

> 状态：DESIGN（实现见 `scripts/check-semantic-registry.mjs` R6 + `docs/architecture/semantic-registry/*.yaml` 扩展）
> 基线：继承 `semantic-phase1.7-git-integrity-pass` + Semantic Registry + Semantic Gate + Recovery Layer + Handoff System
> 触发：Workspace/FilePanel 域（useWorkspaceStore.ts 1150 行 + FilePanel.vue）承载「当前本地位置」等多概念，
> 存在第二真源（second source of truth）风险：filePath / inlineFile / previewDir / currentLocalPath / pathInput
> 语义未冻结，未来易再造重复状态或越权直写。

---

## 1. State Model（受治理状态）

```text
filePath
  = 文件浏览器/覆盖编辑器当前激活位置（address 真源）
  = 注意：编辑器模式下其值为“当前打开的文件路径”，浏览器模式下为“当前目录路径”（overloaded 语义，冻结为单一存储态）

inlineFile
  = 行内编辑器当前打开的文件路径（IDE 右栏，与覆盖编辑器 filePath 隔离）

previewDir
  = 目录预览（图片画廊）当前预览的目录路径

pathInput
  = 地址栏 UI 缓冲（镜像 filePath；仅 UI 输入态，非真源）

currentLocalPath   [DERIVED]
  = 当前“打开的本地位置”派生量
  = inlineFile || previewDir || layout.modTabs.path || filePath
  = 纯派生，禁止存储（= 第二真源红线，类比 Phase 1 的 desiredGridVisibility）
```

### 观测但不治理（observed_not_governed）

useWorkspaceStore.ts 其余 37 个存储声明（tree/current/editTitle/editTags/editText/fileEntries/fileContent/
editingFile/mdPreview/mdHtml/startDirs/previewEntries/previewLoading/previewError/previewTileSize/
compareImages/repos/form/preview/busy/job/audit/scripts/scriptForm/snippets/snippetForm/recents/
flatArtifacts/treeRoots/locateTarget/dragSource/dropTarget/moveConfirm/inlineText/inlineIsMd/inlineEdit/
inlineHtml）与 FilePanel.vue 的 ftreeBody —— 本 Phase 不裁决，显式登记（info 级），纳入治理需走 SCR。

> 注：selected/treeChildren/treeExpanded/treeLoading/treeErrors 使用 `reactive<Set/Map>(new ...)` 嵌套泛型，
> 当前 checker R2 正则不匹配（属 checker 已知盲区，非本 Phase 范围，交专项）。

---

## 2. Intent Model（意图）

```text
intent: openFile
  语义：在覆盖编辑器打开文件（overlay editor）
  owner：useWorkspaceStore.openFile
  duplicate_names：openLocalFile / viewFile / editFile / showFile

intent: openFileInline
  语义：在行内编辑器打开文件（IDE 右栏）
  owner：useWorkspaceStore.openFileInline
  duplicate_names：openInlineFile

intent: enterDir
  语义：进入目录（文件浏览器导航）
  owner：useWorkspaceStore.enterDir
  duplicate_names：openDir / gotoDir / cdDir / navigateDir

intent: saveFile
  语义：持久化当前文件内容到磁盘
  owner：useWorkspaceStore.saveFile
  duplicate_names：writeFileContent / persistFile

intent: closeFileEditor
  语义：关闭覆盖编辑器（editingFile=false；非资源销毁）
  owner：useWorkspaceStore（经 layout.fileEditorOpen=false）
  duplicate_names：dismissFileEditor
  rejected：closeEditor（已被 FileEditor.vue / useTaskStore 定义，禁止作为本域重复入口）
```

### rejected（明确不做）

```text
rejected: 把 openFile 与 openFileInline 合并
  原因：覆盖编辑器 vs 行内编辑器是两条独立 UX 路径，合并会丢失隔离（类比 openGrid≠rebuildGrid）
```

---

## 3. Owner Model（Owner）

```text
workspace_filepanel
  owner = useWorkspaceStore
  owns  = filePath / inlineFile / previewDir / currentLocalPath / pathInput
  owner_only_api = enterDir / openFile / openFileInline / saveFile / closeFileEditor /
                   refreshTree / locateTo
  forbidden_callers = components/**（不得直写 filePath/inlineFile/previewDir）
  authorized_callers = useLayoutStore（经 setView 切到 editor/files 视图）
```

---

## 4. Lifecycle（生命周期）

```text
用户打开文件
  └─ openFile(entry)      → filePath = entry.path; editingFile=true; layout.setView("editor")
  └─ openFileInline(entry)→ inlineFile = entry.path（filePath 同步为父目录，隔离不混用）
用户导航目录
  └─ enterDir(path)       → filePath = path（单一地址真源）
用户关闭编辑器
  └─ closeFileEditor()    → editingFile=false; layout.fileEditorOpen=false（不销毁任何资源）
派生
  └─ currentLocalPath     = computed（inlineFile || previewDir || modTabs.path || filePath）
                             禁止任何 .value = 写入（R6 护栏）
```

---

## 5. Side Effect（副作用）

```text
writeFile（bridge.writeFile / createFile / deletePath / renamePath / movePath）
  - 真实行为：文件系统变更（创建/删除/改名/移动/写盘）
  - 声明：registry 文档化（requires_declaration=false，避免对既有合法调用产生 R5 噪声）
  - 本 Phase 不要求调用点加 side-effect 标记（属文档级登记，非阻断）
```

---

## 6. Checker Plan（门禁设计）

### 6.1 复用 `scripts/check-semantic-registry.mjs`（registry 驱动）

- **R2（扩展）**：将 `src/stores/useWorkspaceStore.ts` 与 `src/components/workspace/FilePanel.vue`
  加入 `states.yaml` 的 `governed_files`。其内任何 `ref/reactive/computed` 声明若未登记于
  `states` 且未列 `observed_not_governed` → FAIL（强制显式登记，不静默消失）。
- **R4（扩展）**：5 个 workspace intent 的 `duplicate_names` 在任意文件被定义即 FAIL（一个意图一个入口）。
- **R3（扩展）**：`workspace_filepanel` owner 的 `forbidden_callers`（components/** 直写 filePath 等）→ FAIL。
- **R6（新增）SEMANTIC_DERIVED_STATE_STORED**：对任意 registry 中 `derived: true` 的状态
  （currentLocalPath / desiredGridVisibility / isBrowserVisible）：
  - 禁止声明为 `ref/reactive/shallowRef`（存储态）
  - 禁止 `.value =` 赋值（第二真源）
  → 直接固化 currentLocalPath 派生不变量。

### 6.2 接入 pre-merge

`check-semantic-registry.mjs` 自 Phase 1.5 已接入 pre-merge（正式 + self-test）。本 Phase 扩展 YAML + R6 后
自动覆盖，无需改 pre-merge 接线。

---

## 7. Acceptance Matrix（验收矩阵）

```text
AC-1  R2 扩展：useWorkspaceStore.ts 内 37 个既有声明均列 observed_not_governed → 真实仓库 fail=0   [REAL]
AC-2  R2 扩展：FilePanel.vue 的 ftreeBody 列 observed_not_governed → 真实仓库 fail=0              [REAL]
AC-3  R4 扩展：workspace intent 的 duplicate_names 在真实仓库无定义 → fail=0                      [REAL]
AC-4  R6：currentLocalPath 为 computed 且无误赋值 → 真实仓库 R6 fail=0                            [REAL]
AC-5  R6：注入 fixture（currentLocalPath = ref("") + .value=）→ 检出 SEMANTIC_DERIVED_STATE_STORED [NEG FIXTURE]
AC-6  R6：positive fixture（currentLocalPath = computed(...)）→ 0 fail/warn                       [POS FIXTURE]
AC-7  check-semantic-registry.mjs --self-test 全量 ALL_PASS（含 R6）                               [SELF-TEST]
AC-8  pre-merge 中 semantic-registry 项仍绿（不引入新 FAIL；terminal 债为既有）                    [GATE]
AC-9  不降低任何既有 Checker；build/其它门禁仍 PASS                                              [NO REGRESSION]
AC-10 HANDOFF_CURRENT_STATE.md 更新 Phase 2 状态 + tag 分类                                      [HANDOFF]
```

---

## 8. 与既有基础设施的关系（避免重复语义）

```text
check-semantic-registry.mjs   复用并扩展（R2/R3/R4 域扩展 + 新增 R6），不重造
states/intents/owners/side-effects.yaml   扩展 workspace 域，不重造
pre-merge.sh               已接入（registry checker），无需改接线
SCR 流程                   新增语义走 SCR-20260919-workspace-filepanel.md
```

---

## 9. Known Debt（本 Phase 内）

```text
DEBT-2-1  filePath overloaded 语义（编辑器模式=文件、浏览器模式=目录）未拆分
  当前仅冻结为单一存储态；彻底拆分需业务重构，超出治理范围，交专项。

DEBT-2-2  R2 正则不识别 reactive<Set/Map>(new ...) 嵌套泛型
  selected/treeChildren/treeExpanded/treeLoading/treeErrors 不受 R2 治理；
  属 checker 盲区，交专项（不影响本 Phase 治理目标）。

DEBT-2-3  writeFile 副作用仅文档化（requires_declaration=false）
  未强制调用点加 side-effect 标记；如需收紧改 side-effects.yaml + 调用点加标记（业务代码改动）。
```
