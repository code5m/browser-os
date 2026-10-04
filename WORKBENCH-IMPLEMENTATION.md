# A0 工作台集中实现

本轮依据：用户明确授权一次完成浏览器式外壳、宫格回复归档、Obsidian Vault、数据库/Git 实际工作流。该授权解除这四项的研究-only 限制，不开放 MCP/Agent 执行器等无关范围。

- 唯一集成目录：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`，分支 `master`。
- 起点：`m5-w18-r3b-accepted` / `58e50b1`。该标签仅代表研究成果，不能冒充客户端功能验收。
- A0 实现；A1–A11 保持现有成果，不重复修改共享文件。
- 先分模块测试，再集成构建、客户端验收；未测项必须明确列出。
- 保留 debug-only localhost capability，release 不引入 devUrl；不扩大远程页面 ACL。
- SQL/笔记/聊天不写入布局持久化；归档必须由用户显式触发，不覆盖现有文件。

## 验收流程

1. 打开模块，隐藏/恢复工具窗；命令搜索和页签右键；窄窗内容区无遮挡。
2. 多宫格提取回复，一格失败不影响其他格；复制成功项、逐文件保存、失败重试。
3. 打开本地 Vault，浏览 Markdown，跳转内部链接，查看反链/局部图谱，搜索并定位。
4. SQLite 连接/对象树/独立查询/取消/结果复制；Git 状态/历史/差异/确认后写入。

状态：IMPLEMENTED，待用户在自己的客户端窗口做最终视觉确认。完成证据与限制在本文件收口，不以静态检查代替真实联调。

当前已接入：原生宫格结果返回通道、页面内早期 URL 脱敏、归档后端、Vault 有界读取后端、数据库取消与独立 SQL 页签、Git 历史/差异、命令入口、工具窗折叠和统一工具条。

## 交付证据（2026-09-09）

- cargo test：450 个二进制测试 + 2 个核心库测试通过。
- cargo build --release：通过；仅保留既有 grid_process.rs 两个 dead_code 警告。
- npm run build：通过；Vite 仅报告既有动态/静态导入提示。
- node scripts/check-workbench-logic.mjs：通过，覆盖 Vault 搜索/路径、SQL 多页签/取消/确认、Git 过期响应。
- node scripts/check-workbench-ui.cjs：通过，覆盖命令入口、Vault 链接/搜索/图谱、SQL 分页/独立页签、折叠恢复与三种窗口尺寸。
- 隔离浏览器提取回归：ChatGPT 适配器、40 条上限、流式保护、注入脚本内敏感 URL 脱敏通过。
- 隔离真实 Tauri smoke：Vault、归档部分成功、Git 历史/差异、SQLite 对象树/查询/取消、宫格索引保护和断开共 17 项通过。
- bash scripts/pre-merge.sh：通过（最终需要先提交本文件与两处历史 Markdown 空白清理后，分支范围检查才会包含修复）。

## 已知边界

- 数据库当前实际驱动为 SQLite；MySQL/PostgreSQL 仍以禁用项显示，未伪造可用能力。
- 宫格回复适配器按站点 DOM 选择器工作，第三方站点改版或回复仍在生成时会返回明确失败/降级提示；本地隔离 fixture 证明了 ChatGPT 适配器，不等同于真实登录站点验收。
- Vault 只读取允许根目录内的 Markdown，跳过隐藏目录、node_modules、符号链接，并有文件/数量/总字节上限。
- Git 历史与差异为只读；现有 Git 写入仍必须走原有预览与二次确认闸门。未复制或执行 rebased 的闭源/外部代码。
- 真正的桌面视觉验收、登录后的第三方 AI 页面适配和用户选择的 Vault 路径，需要用户在客户端窗口完成。

## 依赖变更裁定

本轮仅保留 `@lucide/vue` 作为标准图标依赖。Vault Markdown 安全渲染与回复 HTML 转 Markdown 已收口为仓库内纯函数，并由 XSS、危险标签、链接协议和常用结构测试保护；不引入网络抓取服务，不执行笔记插件，也不复制闭源产品代码。已移除未再使用的 `marked`、`dompurify`、`turndown` 与弃用的 `lucide-vue-next`。三份旧 M2 门禁的 package 指纹随此明确依赖清单更新，继续精确检查包文件，未删除任何反例测试。图片 ACL 末条保持原样，避免旧夹具依赖末条格式失效。
