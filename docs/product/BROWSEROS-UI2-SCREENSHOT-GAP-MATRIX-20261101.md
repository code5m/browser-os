# BrowserOS UI 2.0：真实截图差异矩阵

基线：`master@9607e260f86b15a730219c619ab5a20cc01efe65`，2026-11-01。证据分两类：**S**=用户 2026-11-01 的安装版截图（未获得可在 GitHub 引用的截图路径）；**C**=当前 Master 代码/文档。设计提议不是完成状态。

| 优先级 | 领域 | 真实现状及证据 | 目标态 | 组件/文件 | 风险与验收 |
|---|---|---|---|---|---|
| P0 | Chrome 框架 | S：页签栏、Omnibox、收藏栏已成熟；C：`UnifiedTabBar.vue`、`ActivityBar.vue` | 保留整体架构，精修密度与焦点 | `src/components/layout/UnifiedTabBar.vue` | 无边框拖拽语义不能变；实机检查 720/1024/1440、标签 36+ |
| P0 | Tab Registry | C：`UnifiedTabBar.vue` 并行读取 `browser.tabs` 与 `layout.modTabs`，关闭动作区分 | 一套逻辑页签投影、多 Capability 适配 | `UnifiedTabBar.vue`、`useLayoutStore.ts` | 普通网页关闭不得弹确认/自动持久化；原生资源时序不变 |
| P0 | 设计 Token | S：浅色主页/文件/设置与深色 Agent/DB/Git 缺少一致性；C：`src/styles/tokens.css` v1 且未覆盖 45+ 色值 | 一套 Light/Dark 语义 Token | `src/styles/tokens.css` + 分模块 scoped CSS | Token 等值迁移先通过，视觉重设另设审批，实机逐页面比较 |
| P0 | 首页信息层级 | S：主要工作区卡片 18 个左右，快捷卡片另起一行；C：`ToolBox.vue` 已进行功能分类 | 优先“继续工作”“常用项目”“能力发现”，不删旧能力 | 首页组件、`ToolBox.vue` | 原有功能入口全部可达；避免常用入口缺失 |
| P0 | Agent 状态一致 | S：已接入项目、模型、对话与执行中反馈；工具输出、Agent 审批与编排分散 | 统一任务/运行/审批可追踪界面，显式输出错误和恢复 | `AgentChatPanel.vue`、`CodingWorkbench.vue` | 不将 UI 演示等同真实 Rig/ACP/RMCP Runtime 贯通 |
| P0 | EditorHost | S：Coding Agent 编辑器与文件管理器不同入口；C：`CodingCodeEditor.vue` Monaco，`FileEditor.vue`、`FilePanel.vue` textarea | 唯一 Document Manager + EditorHost，SQL/代码/文档共用可控状态 | `src/capabilities/agent/ui/CodingCodeEditor.vue`、`src/capabilities/workspace/ui/FileEditor.vue`、`FilePanel.vue` | 中文输入法、未保存状态、模型撤销、保存与 Worker 释放回归 |
| P0 | 原生 WebView | C：`PROJECT-RULES.md` 原生 WebView 遮挡、尺寸固定、DPI 规则 | HTML 负责外壳，原生视图始终由原 Runtime 管理 | `src/composables/useBrowserHost.ts`、`tauri-browser-tabs/**`（不修改） | HTML 浮层不能叠盖原生网页；高 DPI 与鼠标真实验收 |
| P1 | 文件管理器 | S：目录树 260px、列表行高较大、右键菜单含 Git/DB/终端；多个操作集中在菜单 | 树/列表/预览清晰，详情可停靠，操作就近可见 | `FilePanel.vue`、`FileTreeNode.vue` | 文件增删改恢复与快捷键兼容 |
| P1 | Database Studio | S：Schema、SQL、结果集、Inspector 和只读提示全部具备，但按钮层级较密 | SQL 编辑器与结果表格分区，可调整大小，突出执行、撤销、只读与事务 | `src/capabilities/database/ui/SqlEditor.vue` 及 DB 工作区 | 只读/事务/SQL 补全/取消/表分页真实回归 |
| P1 | Git Studio | S：历史筛选、变更树、双栏 Diff 有完整功能 | 保留数据密度；统一头部、标签、Diff 与安全审批视觉 | `src/capabilities/git/ui/GitStudio.vue` | Git 冲突与写操作 Preview → Confirm → Execute 不能弱化 |
| P1 | Skills | S：左技能表、概览、步骤/检查/权限 Tab 可达 | 作为 Agent 能力蓝图工作台，支持清晰流程和能力状态 | 技能工作台相关 Capability UI | 不将规划中能力标记为可执行 |
| P1 | Tasks | S：定时任务空态与 Cron 表单具有类型、间隔、重试、补跑、超时设置 | 将长表单做分段校验和风险解释，先目标再调度 | tasks UI | 任务保存不等于启用；误触防护与 Cron 错误提示 |
| P1 | Grid | S：4 格布局工具栏可见，画面为空 | 明确无内容/加载/异常状态，统一布局控制与上下文 | `src/capabilities/grid/ui/*` | 不虚构可用性；原生多 WebView 恢复/资源实测 |
| P1 | 图谱/知识 | S：知识图谱空节点、知识库入口、文件筛选；使用者很难理解为何无数据 | 空态解释数据源/刷新/起始节点，图谱详情 Inspector 一致 | graph/vault/knowledge UI | 不把演示图当成真实用户知识 |
| P1 | Terminal | S：独立终端运行中、单/2/4/9 格模式 | 统一顶栏与 Dock 面板，PTY Session Owner 不迁移 | `TerminalDockPanel.vue` | 隐藏只卸载 xterm 视图，不丢后端 PTY |
| P1 | 设置 | S：AI 模型、外观、快捷键、性能、模块和工程健康位于同一纵向长页面 | 左侧分类导航+局部搜索，保留所有配置和敏感信息安全边界 | settings UI | 不能泄露密钥；设置不可随主题切换丢失 |
| P2 | 脚本库与剪贴板 | S：两者功能页面空旷或线性堆叠 | 统一空态、历史、主操作按钮；配置和执行分离 | scripts/clipboard UI | 原有本地存储语义不变 |
| P2 | 响应式可访问性 | S：主要是 1600px 宽度截图；C：已有 900/1200/1920 的顶部检查记录 | 720/1024/1440 布局，阅读、编辑、Git/DB 大表更紧凑 | Shell + UI product checks | 200% 放大、读屏、焦点转移、中文/英文溢出 |
| P2 | 实际性能 | S：状态栏应用内存读数 390MB~3.2GB 不同场景，不能直接建立性能归因 | 在固定工作负载下测量 RSS/PSS/CPU/p95 输入延迟 | 安装版人工验收报告 | 不能从截图内存数字猜测具体泄漏 |

## 评分说明

这是依据已收到的截图和源码进行的 **设计成熟度初评**，不是量化用户研究，也不是实机性能得分。浏览器外壳 8/10，首页信息架构 6.5/10，文件 7/10，Git 7.5/10，DB 7/10，Agent 7/10，跨模块一致性 5.5/10；**整体保守估分约 7.0/10**。设计目标 A+ 为可验证的原则，不意味着现已通过 QA。

## 不能覆盖的边界

- Ubuntu Dock/GNOME 状态栏不是 BrowserOS UI，不设计进去。
- BFC 本地网页或第三方网站由网站自身控制，不应强制采用 BrowserOS Token。
- 地址栏 HTML 模拟建议仅为产品原型：实际 Browser/Grid 要保留 Native Menu，不能使用 z-index 覆盖。
- 真实 Agent 执行、数据库更新、Git Push 都必须接原业务 Runtime，不能以 DEMO PASS 作为工程 PASS。
