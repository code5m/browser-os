# BrowserOS UI 2.0 · P0–P2 迁移与验收计划

基线：`master@9607e260f86b15a730219c619ab5a20cc01efe65`。设计分支：`design/browseros-ui2-aplus-20261101`。当前仅包含独立原型、设计规范和审计文档；**所有生产功能迁移均未实施**。

## P0：统一状态与视觉，严守资源 Owner

- [ ] **P0-01 设计系统等值迁移**：盘点 `src/styles/tokens.css`、`src/styles/global.css`、所有模块 scoped CSS；先将硬编码替换为 computed value 相同的语义变量，保持测试兼容；主题重设计单独批准。验收：Light/Dark 对照截图和 `verify-design-tokens.mjs`。
- [ ] **P0-02 Tab Registry**：`UnifiedTabBar.vue` + `useLayoutStore.ts` + Browser Public API 建立统一视图投影（稳定 ID、激活、关闭、排序、恢复、dirty 状态）。**生命周期仍各归 Web/File/Terminal/Agent Capability**。普通网页 Tab 关闭不弹框、不写 Session、只保留当前 recentlyClosed 语义。验收：36+ 页签、跨资源切换、关闭同类/其他、恢复、WebView 无异常。
- [ ] **P0-03 统一 Shell 视觉**：保留 Home/Browser/Files/Tools 四目的；根据设计稿统一 tab/omnibox/bookmarkbar/statusbar 的间距和焦点。保持 `ActivityBar.vue` 原生菜单策略。验收：720/1024/1440、Ctrl+L、Enter/Esc、窗口拖拽、收藏夹显隐与菜单。
- [ ] **P0-04 Document Manager**：`FileEditor.vue` / `FilePanel.vue` 与 `CodingCodeEditor.vue` 共享文档身份、模型引用计数、dirty/readonly/revision。先补测试，后迁移。一份文档只存在一个内容模型，不覆盖当前文件数据。验收：双入口同文件保存、撤销、关闭、模型释放。
- [ ] **P0-05 EditorHost**：在通用 EditorHost 接管文档编辑视觉后，将 SQL `SqlEditor.vue` 的输入承载迁入而不迁移数据库事务/SQL 解析/补全协议。验收：IME、LSP、Diff、选中执行、结果集、权限回归。
- [ ] **P0-06 Agent Task Surface**：统一 Coding Agent 真正已有的会话/工具/修改提议/审批，其他 Agent 仅用明确可用性状态；任务状态机唯一 owner。验收：拒绝/超时/取消/失败/重试，确认按钮不可执行两次；无隐式 Side Effect。

## P1：工作台组合与所有模块视觉统一

- [ ] **P1-01 WorkspaceLayout / DockManager**：左资源/中央内容/右 Inspector 或 Agent/底部 Dock 使用配置式注册。不得让不同模块同时保有独立布局真源。切换视图不释放 PTY 或 WebView。验收：窗口缩放、资源重连、Dock 开关与多屏 DPI。
- [ ] **P1-02 Developer Studio**：以已有 CodingWorkbench 为基础，使项目、编辑器、Diff、Git、Database、Terminal 和 Agent 能按场景组合；仍通过 Capability Public API 进行业务调用。验收：工作区恢复、焦点、编辑器会话完整。
- [ ] **P1-03 Database Studio**：统一 SQL 标签、Schema、结果集、Inspector、只读/事务状态视觉。验收：SELECT 与写入保护、表格虚拟滚动、导出、未连接时状态。
- [ ] **P1-04 Git Studio**：统一历史/变更/Diff 工具条，保留既有 Git Adapter 与 Preview→Confirm→Execute。验收：变更暂存、冲突、历史、Diff 和所有确认。
- [ ] **P1-05 Browser/Grid**：对 1/2/4 设计稿做兼容性映射，原项目实际能力有 2/3/4/6/9 格；不改变真实 Grid 默认布局或 WebView 生命周期。验收：加载、连接恢复、背压和原生矩形。
- [ ] **P1-06 Skill Studio / Tasks**：技能步骤/定义检查/权限与运行分层；任务 Cron/类型/超时/补跑/开关分段。验收：配置合法性、未批准不会执行、失败可追踪。
- [ ] **P1-07 Capability Center**：复用 `ToolBox.vue` 与 `useToolsStore` 唯一真源，去掉研发术语；搜索/分类/最近使用/可用性，所有原模块仍可到达。验收：模块未安装/只读/权限拒绝/空态。
- [ ] **P1-08 知识库/图谱**：区分无数据、加载失败、需授权、起始节点缺失；节点详情与搜索一体。验收：大图缩放/选择、0 节点、错误反馈。
- [ ] **P1-09 设置**：AI 模型、外观、快捷键、性能、模块、关于按类别重排，敏感值不透出日志或演示状态。验收：切换主题不丢配置，设置可搜索且长文案不溢出。

## P2：细节与实际测量

- [ ] **P2-01 剪贴板/脚本库**：统一空态、主按钮和历史；执行入口须遵循权限限制。验收：无数据、错误、用户确认。
- [ ] **P2-02 动效与无障碍**：120–180ms 小动效，支持 `prefers-reduced-motion`；可见 focus、语义标签、键盘全流程、屏幕阅读器与 200% 缩放。验收：人工键盘 + 读屏报告。
- [ ] **P2-03 性能基线**：相同安装包和工作负载下记录启动首帧、导航 p95/p99、CPU、PSS/RSS、WebView 数量、PTY/Agent sessions；只在实测有数据时给 PASS/FAIL。
- [ ] **P2-04 GUI 最终验收**：一套截图矩阵：Home、Browser、Files、Studio、Agent 执行+审批、DB SQL+结果、Git Log+Diff、Grid 2/4、Skills、Tasks、Graph、Tools、Settings，分别测试亮暗、720/1024/1440 并记录视频、日志和对应 commit。

## 停止规则、协作和分支边界

- 本设计分支只新增 `docs/prototypes/browseros-ui2/**` 与 `docs/product/BROWSEROS-UI2-*.md`；不修改其他已在进行的 perf、Agent、Editor、Grid 等并行分支。禁止 worktree。
- 不动 `src-tauri/src/bridge.rs`、`src-tauri/src/**/linux.rs`、`tauri-browser-tabs/**` 和 Capability 权限；需原生变更时独立立项并停止本轮迁移。
- 不改变 `gtk_fixed_move + size_allocate`、offscreen `x=-30000`、CSS 像素无 DPR 乘算；不得使用 `webview.hide()` 或缩至 1×1。
- 不恢复 `SessionCloseDialog` 或关闭页面自动 `sessionSave`；已批准、已否决的行为均遵循 `PROJECT-RULES.md`。
- 不新增 HTML 浮层覆盖原生 Browser/Grid；应用浏览器区域的错误进状态栏，完整界面面板要参与布局，触发 WebView rect 同步。
- 有未知 Owner、跨 Capability 私有依赖、用户未裁决的 [PENDING] 需求，停下而不是推断。
- UI 代码迁移时：至少 `npm run build`、`node scripts/check-workspace-startup.mjs`、`node scripts/check-image-preview-lazy-loading.mjs`、`bash scripts/pre-merge.sh`、`git diff --check`；另外跑实际 UI / capability / installed GUI。文档提交不冒充这些运行结果。

## 独立原型验收（不是生产验收）

- [ ] `index.html` 可本地打开，无 CDN、无 Tauri 依赖、无用户数据读取。
- [ ] 页面间可切换、标签关闭、地址建议用 Enter/Esc/上下键控制。
- [ ] 浅色/暗色主题和 720/1024/1440 响应式布局。
- [ ] 进入 File/Studio 编辑、切换底部 Dock、Agent 审批、Database SELECT 模拟、Git Diff、Grid 1/2/4、Skills、Tasks、Tools、Settings。
- [ ] 所有虚拟写操作明确为演示，不给用户错误成功预期。
- [ ] **原生 GUI 验收：PENDING**（此分支无真实运行时修改）。
