# UI DUPLICATION MAP — Phase UI-0B

> 方法：**不凭视觉相似断言重复**。对每组候选逐项评估
> `SAME_VISUAL / SAME_INTERACTION / SAME_SEMANTICS / SAME_ACCESSIBILITY / SAME_LIFECYCLE`
> 再给出 `EXTRACTION_CONFIDENCE` 与分类。

判定分类：
- `TRUE_DUPLICATE` — 五维全同，可安全提炼
- `SHARED_PATTERN_CANDIDATE` — 多维相同但需抽象设计，待 RULE OF TWO 验证
- `VISUALLY_SIMILAR_BUT_SEMANTICALLY_DIFFERENT` — 形似神不似
- `CAPABILITY_SPECIFIC` — 业务专属，不应进 shared
- `DO_NOT_MERGE` — 明确禁止合并

---

## 总览

| 指标 | 数量 |
|---|---:|
| DUPLICATION_GROUPS | **9** |
| TRUE_DUPLICATE_GROUPS | **3** |
| SHARED_PATTERN_CANDIDATE | **4** |
| VISUALLY_SIMILAR_BUT_SEMANTICALLY_DIFFERENT | **1** |
| DO_NOT_MERGE_GROUPS | **2** |
| CAPABILITY_SPECIFIC | **1** |

---

## G-01 EmptyState（空态）★ 最高优先级

**候选成员（20+ 处，各写各的）**

| 文件:行 | 文案 | 容器类 |
|---|---|---|
| `home/HomeRecents.vue:36` | 还没有最近访问记录 | `.he-title` |
| `home/HomeShortcuts.vue:56` | 还没有快捷方式 | `.he-title` |
| `workspace/GitPanel.vue:52` | 还没有配置仓库… | — |
| `workspace/ScriptParamForm.vue:77` | 暂无参数 | `.empty` |
| `workspace/RunHistoryModal.vue:22` | 暂无运行记录。 | `.empty` |
| `workspace/TaskPanel.vue:114` | 暂无定时任务。 | — |
| `workspace/TaskPanel.vue:179` | 暂无该目标的执行记录。 | `.muted` |
| `workspace/TaskPanel.vue:193` | 暂无运行记录。 | `.muted` |
| `workspace/ToolBox.vue:74` | 暂无工具 | — |
| `browser/CredentialList.vue:138` | 暂无已导入账号 | — |
| `shared/ImageGallery.vue:79` | 该成果暂无可预览的图片 | `.empty-text` |
| `graph/GraphViewer.vue:86` | 暂无节点可绘制… | `.viewer-empty` |
| `graph/EdgeDetail.vue:23` | 选择一条边查看详情 | `.detail.empty` |
| `graph/NodeDetail.vue:23` | 选择一个节点查看详情 | `.detail.empty` |
| `system/ClipboardPanel.vue:38` | 暂无历史记录… | `.empty` |
| `bookmark/ui/BookmarkPanel.vue:160` | 还没有收藏：点地址栏 ☆… | — |
| `capabilities/plugin/ui/PluginManager.vue:145` | 暂无插件… | `.pm-empty` |
| `capabilities/plugin/ui/PluginManager.vue:169` | 暂无登记密钥 | `.pm-empty` |
| `browser/ui/SessionPanel.vue:84` | 暂无历史会话。点上方… | — |
| `capabilities/workspace/ui/FilePanel.vue` | `.ftree-empty` / `.fedit-empty` | 两套 |

| 维度 | 判定 |
|---|---|
| SAME_VISUAL | ⚠️ 部分（`.empty` 居中灰字；但 `.pm-empty`/`.he-title`/`.viewer-empty` 各自不同） |
| SAME_INTERACTION | ✅ 全同（纯展示，无交互） |
| SAME_SEMANTICS | ✅ 全同（"无数据"） |
| SAME_ACCESSIBILITY | ⚠️ 部分（部分带 `role="status"`，多数没有） |
| SAME_LIFECYCLE | ✅ 全同（无生命周期） |

**分类：TRUE_DUPLICATE** ｜ `EXTRACTION_CONFIDENCE = HIGH`
**消费者 ≥ 2（实测 20+）→ 满足 RULE OF TWO**
**MIGRATION_ACTION**：提炼 `EmptyState`（props: `text` / `hint?` / `icon?` / `role`）。
**风险**：低。**约束**：提炼时**逐字保留原文案**，不得改写；缺失的 `role="status"` 是**改进**而非回归，需单独记录。

---

## G-02 LoadingState（加载态）

**候选成员（15 文件）**：`HomePanel.vue:39`、`GitWriteConfirmDialog.vue:77`（执行中…）、`VaultPanel.vue:44`（正在读取 Vault…）、`CredentialList.vue:138`、`ImageGallery.vue`（骨架屏）、`FileTreeNode.vue`（`treeLoading`）、`GraphPanel.vue`、`PluginManager.vue`、`TaskPanel.vue`、`DatabasePanel.vue`、`ToolBox.vue`、`SessionPanel.vue`、`ScriptRunHistory.vue`、`GitHistory.vue`、`GitPanel.vue`

| 维度 | 判定 |
|---|---|
| SAME_VISUAL | ❌ 不一致（"加载中…" / "正在读取…" / 骨架屏 / `role=status` 四种） |
| SAME_INTERACTION | ✅ 全同（纯展示） |
| SAME_SEMANTICS | ✅ 全同（"进行中"） |
| SAME_ACCESSIBILITY | ⚠️ 部分 |
| SAME_LIFECYCLE | ✅ 全同 |

**分类：TRUE_DUPLICATE（语义层）／实现需归一** ｜ `EXTRACTION_CONFIDENCE = MEDIUM-HIGH`
**消费者 15 → 满足 RULE OF TWO**
**MIGRATION_ACTION**：提炼 `LoadingState`（props: `text?` / `variant: 'text'|'skeleton'`）。
**风险**：中。**约束**：骨架屏与文本态是两种 variant，**不得强行统一成一种**；先做文本态，骨架屏单独评估。

---

## G-03 Modal / Dialog 遮罩层

**候选成员（7 文件）**：`GitWriteConfirmDialog.vue`、`RunHistoryModal.vue`、`TaskEditDialog.vue`、`PermissionPreviewModal.vue`、`shared/ConfirmModal.vue`、`shared/ImageLightbox.vue`、`workspace/ui/FilePanel.vue`（拖拽移动确认）

CSS 层存在**两套遮罩**：
- `.home-modal-mask` @ `global.css:137` — `background: rgba(0,0,0,.35)`、`z-index: 100`
- `.modal-mask` @ `global.css:203` — `background: rgba(0,0,0,.4)`、`z-index: 999`

| 维度 | 判定 |
|---|---|
| SAME_VISUAL | ⚠️ 近似但**遮罩透明度和 z-index 不同**（35%/100 vs 40%/999）→ 冲突值 |
| SAME_INTERACTION | ✅ 全同（Esc 关闭 + 焦点陷阱 + 焦点归还，均复用 `modalA11y`） |
| SAME_SEMANTICS | ✅ 全同（模态叠加） |
| SAME_ACCESSIBILITY | ✅ 全同（已有 `utils/modalA11y.ts` 支撑） |
| SAME_LIFECYCLE | ✅ 全同（open/close） |

**分类：TRUE_DUPLICATE** ｜ `EXTRACTION_CONFIDENCE = MEDIUM`
**消费者 7 → 满足 RULE OF TWO**
**MIGRATION_ACTION**：提炼 `ModalShell`（只管遮罩 + a11y + 焦点，**不管内容**），内容走 slot。
**风险**：中。**约束**：**不得统一 z-index/透明度**（会改变视觉）。应先确定 CANONICAL 值（建议 `.modal-mask` 40%/999），把 `.home-modal-mask` 作为 CANDIDATE 单独评估，迁移时保持各自原值直至 token 化阶段。

---

## G-04 Tabs（页签条）— ⚠️ 形似神不似

**候选成员（3 套实现）**
| 实现 | CSS | 用途 | 生命周期 |
|---|---|---|---|
| `.mod-tabbar / .mod-tab` | `global.css:25-29` | 模块页签 | 视图切换 |
| `.tabs` | `global.css:35-38` | Dock / 通用 | 面板切换 |
| `.tabbar / .tab` | `global.css:295-301` | **浏览器网页页签** | **绑定 webview 资源** |

| 维度 | 判定 |
|---|---|
| SAME_VISUAL | ✅ 近似 |
| SAME_INTERACTION | ⚠️ 近似（点击激活 + 关闭按钮） |
| SAME_SEMANTICS | ❌ **不同**：浏览器页签 = 一个 webview 资源；模块页签 = 视图切换；Dock 页签 = 面板切换 |
| SAME_ACCESSIBILITY | ❌ 不一致 |
| SAME_LIFECYCLE | ❌ **根本不同**（浏览器页签关 = 销毁 webview） |

**分类：VISUALLY_SIMILAR_BUT_SEMANTICALLY_DIFFERENT → DO_NOT_MERGE**
`EXTRACTION_CONFIDENCE = LOW`（刻意不提炼）
**理由**：强行合并会让通用 Tabs 被迫理解 webview 生命周期 → 直接违反 §13「shared/ui 不得拥有业务 state」与 §16「禁止 God Component」。
**MIGRATION_ACTION**：保持三套独立。如需收敛，只可提炼**纯样式 tab 外观**（无行为），且必须先证明三处视觉完全一致。

---

## G-05 ContextMenu（右键菜单）

**候选成员（3+）**：`ArtifactPanel.vue:32-41`（成果右键）、`FilePanel.vue:204-240`（文件右键，含 `ctx-title`/`ctx-input`/`ctx-sep`）、`UnifiedTabBar.vue:212-222`（页签右键）
CSS：`global.css:78-82`（`.ctx-menu` / `.ctx-item` / `.ctx-item.danger`）+ `:234-236`（`.ctx-title` / `.ctx-input` / `.ctx-sep`）

| 维度 | 判定 |
|---|---|
| SAME_VISUAL | ✅ 同（共用 `.ctx-menu`） |
| SAME_INTERACTION | ⚠️ 近似（点击项执行；文件菜单内嵌输入框） |
| SAME_SEMANTICS | ⚠️ 菜单项内容不同，但**菜单容器语义相同** |
| SAME_ACCESSIBILITY | ❌ 未知/不一致（未见统一键盘导航） |
| SAME_LIFECYCLE | ✅ 同（open → select → close） |

**分类：SHARED_PATTERN_CANDIDATE** ｜ `EXTRACTION_CONFIDENCE = MEDIUM`
**消费者 3 → 满足 RULE OF TWO**
**MIGRATION_ACTION**：只提炼**菜单容器 + 菜单项**（`ContextMenu` + `ContextMenuItem`），菜单项内容由消费者提供（slot / items prop）。**不得把"新建文件/删除成果"等业务项塞进 shared**。
**风险**：中。注意 `.ctx-menu` 是 `position: fixed; z-index: 999` —— 属既有 architecture rule 允许的既有用法，迁移时必须保留（不得改成新 fixed overlay）。

---

## G-06 PanelHeader（面板标题栏）

**候选成员（5+）**：`.ftree-head`（FilePanel）、`.fedit-head`（FilePanel）、`.git-col-head`（Git）、`.preview-header`（FileEditor）、`.ai-head`（AINavPanel）、`.home-head`

| 维度 | 判定 |
|---|---|
| SAME_VISUAL | ⚠️ 近似（标题 + 右侧操作区） |
| SAME_INTERACTION | ⚠️ 近似 |
| SAME_SEMANTICS | ✅ 同（标题 + 操作） |
| SAME_ACCESSIBILITY | ❌ 不一致 |
| SAME_LIFECYCLE | ✅ 同 |

**分类：SHARED_PATTERN_CANDIDATE** ｜ `EXTRACTION_CONFIDENCE = MEDIUM`
**消费者 5+ → 满足 RULE OF TWO**
**MIGRATION_ACTION**：提炼 `PanelHeader`（props: `title`；slot: `actions`）。**风险**：中低。注意 `.ai-head` 有 `.close` 特殊布局，需 slot 容纳。

---

## G-07 FilterInput / SearchInput

**候选成员（3+）**：`.app-filter`（AppPanel，`global.css:276`）、`.res-filter`（ResourceWaterfall，`global.css:55`）、`.recent-select`（HomeRecents，`global.css:289`）、Home 搜索、WorkbenchCommands 搜索框

| 维度 | 判定 |
|---|---|
| SAME_VISUAL | ⚠️ 近似（100% 宽 + 5px 8px padding + 12px 字） |
| SAME_INTERACTION | ✅ 同（输入过滤） |
| SAME_SEMANTICS | ✅ 同（过滤） |
| SAME_ACCESSIBILITY | ⚠️ 部分（aria-label 有/无） |
| SAME_LIFECYCLE | ✅ 同 |

**分类：SHARED_PATTERN_CANDIDATE** ｜ `EXTRACTION_CONFIDENCE = MEDIUM`
**消费者 4+ → 满足 RULE OF TWO**
**MIGRATION_ACTION**：提炼 `FilterInput`（props: `modelValue` / `placeholder` / `ariaLabel`）。**风险**：低。

---

## G-08 IconButton（lucide 图标按钮）

**候选成员**：`GridArchiveBar.vue`（`<Download :size="14"/>` 等 4 个）、`.home-ops .op`、`.gcb-close`、`ActivityBar` 各图标按钮、`TerminalPane` 头部按钮

| 维度 | 判定 |
|---|---|
| SAME_VISUAL | ❌ 差异大（22px / 18px / 14px / 原生 button） |
| SAME_INTERACTION | ✅ 同 |
| SAME_SEMANTICS | ✅ 同（图标触发） |
| SAME_ACCESSIBILITY | ❌ 不一致（部分有 title/aria-label，部分没有） |
| SAME_LIFECYCLE | ✅ 同 |

**分类：SHARED_PATTERN_CANDIDATE** ｜ `EXTRACTION_CONFIDENCE = LOW-MEDIUM`
**MIGRATION_ACTION**：**Pilot 阶段不建议动**。图标尺寸/内边距差异大，过早提炼会引发 prop explosion（§17）。先做 EmptyState / LoadingState / ModalShell 三个高置信项。

---

## G-09 Git 同步确认弹窗 vs 通用确认弹窗 — DO_NOT_MERGE

`shared/ConfirmModal.vue`（27 行）名字像通用确认框，实际硬编码 Git 推送字段（仓库名 / 远程地址 / 成果清单 / 进度），绑定 `rp.confirmSync`。

| 维度 | 判定 |
|---|---|
| SAME_VISUAL | ⚠️ 与通用 modal 近似 |
| SAME_SEMANTICS | ❌ **Git 专属** |

**分类：CAPABILITY_SPECIFIC + DO_NOT_MERGE**
**MIGRATION_ACTION**：**不要**把它"泛化"成通用 ConfirmDialog（会产生 prop explosion 与业务 switch）。正确做法是把它**迁出 `shared/`** → 归入 GIT 能力 UI，并让真正的通用弹窗走 G-03 `ModalShell`。

---

## 明确禁止合并的清单（DO_NOT_MERGE）

1. **三套 Tabs**（G-04）— 生命周期根本不同
2. **Git ConfirmModal 泛化**（G-09）— 业务专属

## 禁止事项重申

- ❌ 为降低组件数量强行合并
- ❌ 为"复用率"制造带 `type/mode/variant/businessType` 的万能组件
- ❌ 让 shared/ui 理解 webview / PTY / Git / DB 生命周期
