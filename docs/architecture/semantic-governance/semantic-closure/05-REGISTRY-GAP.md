# 05 — Registry Gap Analysis

> 对比「实际代码」vs「semantic-registry」。分类：已治理 / 需治理(SCR) / 不需要治理(implementation-detail)。

## 1. 已治理（代码与 Registry 一致，范围内）

- State：33 项受治理状态（01 §1）全部在代码中找到对应 owner store 字段，canonical_writer/forbidden_writers 与代码一致。
- Intent：canonical 入口 + `rejected_intents` 红线与代码一致；无重复入口。
- Owner：7 owner + native_execution，越界模式有 R3/R2 约束，真实扫描 fail=0。
- Side Effect：10 类登记，keyring 有机器约束（S1/S2）+ R7。
- **结论：范围内（Phase 1–5.1）Registry 与代码对齐，CLOSED。**

## 2. 需治理（进入 SCR → Review → ADR → Registry）

### 2.1 范围内小修（低优先级，可随下一 SCR 收口）

| Gap | 代码位置 | 建议 |
|---|---|---|
| G-01 | `aiNavOpen` 双声明（useBrowserStore:43 + useLayoutStore:146） | 合并到单一 owner（倾向 useLayoutStore），删另一处 |
| G-02 | panel 开关布尔散落（clipOpen/fileEditorOpen/browserDockOpen/browserDockTab） | 评估统一 panel 注册表（S-12），或至少逐个登记 owner |
| G-03 | `gridSession` 边界权属（useBrowserStore:23） | 注明"前端 epoch 镜像，生命周期真源在 Rust"，明确 owner |

### 2.2 范围外新增域（最大缺口，需大型 SCR 或保持 out-of-scope）

| Gap | 域 | 缺失 |
|---|---|---|
| G-04 | Git（useGitStore + git_* 命令） | 无 State/Intent/Owner/SideEffect 登记；git push 网络副作用未登记 |
| G-05 | Agent/Skill（useAgentStore） | 同上；agent install 进程副作用未登记 |
| G-06 | Task/Scheduler（useTaskStore） | 同上 |
| G-07 | Database（useDatabaseStore） | 同上；db_connect/query 网络副作用未登记（凭据侧已登记） |
| G-08 | Graph（useGraphStore） | 同上 |
| G-09 | Session/Vault/Image/Resource/Plugin/Settings/Home/GridArchive/Workbench | 同上 |

### 2.3 未登记副作用（对应 04 §3，SE-11..20）

`run_script`(进程) / `plugin_install`(fs+进程) / `git_*`(网络) / `db_*`(网络 DB) / `agent_*`/`skill_*`(进程+fs)
/ `session_export`(fs) / `vault_open`(fs) / `set_resource_capture_settings`(fs/截图) / `clipboard_*`(OS 剪贴板)
/ `mcp_server`(进程) —— 均需 SCR 登记 side-effect（SE-12 插件安装、SE-19 剪贴板为安全敏感，优先级最高）。

## 3. 不需要治理（implementation-detail，禁止塞入 Registry）

以下代码状态是局部 UI/编辑缓冲/组件 ref，不构成跨切面业务语义，**不进入 Registry**：

- 文件编辑缓冲：`fileContent`/`mdPreview`/`mdHtml`/`inlineText`/`inlineHtml`/`inlineEdit`/`inlineIsMd`/`editTitle`/`editTags`/`editText`/`editingFile`
- 文件树 UI：`tree`/`treeRoots`/`treeChildren`/`treeExpanded`/`treeLoading`/`treeErrors`/`fileEntries`/`current`/`startDirs`/`previewEntries`/`compareImages`
- 表单/弹窗缓冲：`form`/`scriptForm`/`snippetForm`/`moveConfirm`/`fileCtx`/`ctxMenu`/`dragSource`/`dropTarget`/`locateTarget`
- 布局几何：`fileTreeWidth`/`sidebarWidth`/`windowWidth`/`navSection`/`navDensity`/`navTopViews`/`leftResizing`/`modTabs`/`activeModTab`/`leftTab`/`addrMode`/`compactMode`
- 瞬时提示：`msg`（toast）
- DOM ref：`browserHost`（useBrowserHost）
- 预览/镜像对象：`preview`(SyncPreview)/`preview`(GitWritePreview)/`useImagePreviewStore` 整体

> 理由：这些状态生命周期局限于单一组件/store 内部，无跨域真源竞争，纳入 Registry 会造成过度治理噪声。
> 维持为 `observed_not_governed` 或 implementation-detail 即可（见 06）。

## 4. 决策原则（防过度/防遗漏）

- **过度治理红线**：不把 local ref / 编辑缓冲 / 布局像素 当业务语义登记。
- **遗漏红线**：跨切面语义（凭据、终端面板、收藏夹、网格资源、导航表面）必须有 owner + canonical_writer + checker。
- **范围外域**：本审计**不擅自**把 14 个新增域整体纳入 Registry；是否扩展由人工 SCR 决策（08）。
