# DBX 数据库客户端产品原型规格

## 证据与范围

- 上游仓库：<https://github.com/t8y2/dbx>
- 本次核对版本：主分支 `204e5a1efc1af02c61fea6bea7703bb07260480d`
- 上游技术证据：`apps/desktop/src/App.vue`、`src-tauri/src/commands/`、`README.md`
- 官方视觉证据：README 引用的 `screenshot-light.png`、`screenshot-dark.png`、`screenshot-er.png`、`screenshot-grid.png`
- 本原型文件：[dbx-product-prototype.html](../design/dbx-product-prototype.html)
- 结论边界：这是源代码和官方 README 证据驱动的产品原型。页面明确处于 Demo mode，不宣称已接入真实数据库、CLI 或 MCP。

## 产品定位

DBX 是统一数据库工作台：桌面端使用 Tauri 2，Web 端通过 Docker 提供团队访问，CLI 用于脚本/CI，MCP Server 用于 AI Agent 数据库工具调用。四个入口共享连接配置、查询历史、SQL 文件、权限和审计策略。

## 信息架构

1. 连接工作区：连接列表、搜索、连接状态、新增/编辑/测试连接。
2. 查询工作区：SQL 编辑器、格式化、执行、Explain、结果表格、导出。
3. 对象浏览器：schema、表、视图、字段、索引、行数。
4. 数据视图：表格浏览、筛选、单元格编辑、分页和导出。
5. ER 图：表关系、节点拖动、缩放、只读查看。
6. AI Assistant：数据库上下文问答、SQL 生成、风险提示、执行前确认。
7. 自动化面：CLI、Docker Web 服务、MCP 工具、查询历史和任务状态。

## 原型布局

- 顶部：DBX 品牌、Query/Schema/AI Assistant 工作区标签、主题与设置。
- 左侧 250px：连接树和对象树。
- 中央：顶部操作栏；上方左右分栏为 SQL 编辑器和结果表格；底部为查询历史与 CLI/MCP 状态。
- 右侧 292px：当前连接、表结构摘要、ER 图入口和 AI Assistant。
- 底部状态栏：连接状态、数据库、行数、运行模式。
- 编辑器与结果区之间支持拖动调整宽度；当前比例只存在于页面会话，正式实现需加入 splitter 状态持久化。

## 交互契约

| 控件 | 成功行为 | 失败/边界 |
| --- | --- | --- |
| 新增连接 | 打开连接表单，测试成功后保存 | 地址/凭证缺失时阻止保存，密码不得写入日志 |
| Run | 在演示结果区恢复样例表格、更新行数和历史 | 当前明确是 Demo，不调用外部数据库；真实接入时语法错误进入结果区错误状态 |
| Explain | 把结果区切换为演示执行计划 | 当前明确是 Demo；真实接入时需适配数据库方言 |
| Format | 保持语义不变，格式化编辑器 | 空编辑器不报错 |
| Export | 切换结果区到导出预览并提供演示 CSV 下载 | 当前只生成演示 CSV；真实接入时大结果集显示进度和取消入口 |
| 对象树节点 | 展开 schema、表、视图和字段 | 未连接时显示空状态 |
| ER 图 | 把结果区切换为关系图演示 | 当前关系为样例数据；真实接入时无外键显示“暂无关系” |
| AI Assistant | 在侧栏更新演示回复 | 当前不访问数据库；真实接入时执行写操作前必须二次确认 |
| CLI/MCP 状态 | 显示服务端口和工具可用性 | 服务未启动必须标记 unavailable，不得显示 ready |

## 状态矩阵

- 连接：未配置 / 测试中 / 已连接 / 凭证失效 / 网络错误 / 只读。
- 查询：空闲 / 运行中 / 成功 / SQL 错误 / 被取消 / 超时。
- 结果：无结果 / 有结果 / 大结果集分页 / 导出中 / 导出失败。
- AI：空闲 / 生成中 / 需要确认 / 已执行 / 被拒绝。
- 服务：Docker Web、CLI、MCP 分别独立显示，不用一个总状态替代三个真实状态。

## 视觉规则

- 默认深色工作台，使用低对比面板、细边框、蓝色主操作、绿色连接状态。
- 表格采用紧凑行高、粘性表头、悬停高亮；编辑器使用等宽字体。
- 危险写操作必须使用明确动词和确认态；成功反馈进入状态栏或布局内结果区。
- 浅色主题、空状态、错误状态和只读状态必须与深色主态同等设计。

## 四端交付边界

- Desktop：Tauri 2 原生窗口、系统密钥链、文件拖放和本地连接。
- Docker Web：默认 `4224`，不能依赖桌面窗口 API；必须有独立的服务健康状态。
- CLI：`dbx query`、文件执行、导出和 CI 退出码；不把桌面 UI 状态当作 CLI 成功证据。
- MCP：数据库工具列表、参数校验、只读/写权限边界、执行前确认和审计。

## 验收标准

1. 目标 HTML 在已启动服务上可打开，所有原型控件有明确反馈；服务状态和 Demo 状态不能混淆。启动前必须先确认端口进程存活，HTTP 200 不替代交互验证。
2. 桌面、Web、CLI、MCP 的状态分开显示，不能互相冒充。
3. 连接、查询、结果、对象树、ER 图、AI Assistant 六个核心区域均有正常/空/错误态。
4. 产品文档能追溯到上游版本和源码/README 证据。
5. 原型交互与真实后端能力必须分开标注；当前原型不宣称真实 SQL 写入、数据库连接、CLI 执行或 MCP 调用。

## 上游证据到原型映射

| 上游证据 | 可核对内容 | 原型落点 |
| --- | --- | --- |
| `apps/desktop/src/App.vue` | 桌面工作区壳与工作区切换 | 原型 `.layout`、`.toolbar`、`.editor-row`、`.bottom` |
| `apps/desktop/src/components/grid/` | 结果网格的列/行展示与交互 | `renderFixture()` 生成表头/行；`#gridBtn` 恢复表格视图 |
| `src-tauri/src/commands/query.rs` | 查询、Explain、导出命令边界 | `#runBtn` → `parseDemoQueryStrict`；`#explainBtn` → 计划面板；`#exportBtn` → Blob 下载；原型不调用 Tauri |
| `src-tauri/src/commands/connection.rs` | 连接配置、测试和生命周期 | `#addConn` 打开表单；`#testConn` 校验名称/地址并插入 `.tree .node`；不伪造网络成功 |
| `src-tauri/src/mcp_http_server.rs` | Web/CLI/MCP 服务边界 | 底部服务状态明确 `demo only` / `not connected`，不把页面可访问当作服务已连接 |
| `README.md` 中的官方截图链接 | 深色工作台、结果表格、ER 图视觉方向 | 深色面板、蓝色主按钮、ER 图结果态、Inspector |

## 本轮可复验的本地闭环

- 在编辑器输入空内容后点击 Run：进入空状态。
- 在编辑器输入非 `SELECT` 语句后点击 Run：进入错误状态，不伪造成功结果。
- 在编辑器输入 `SELECT ...` 后点击 Run：显示演示结果并写入查询历史。
- Format 读取当前编辑器内容并原地格式化；字符串、反引号标识符和 `--` 注释中的空格/逗号保持不变。
- 新增连接要求名称和地址，成功后插入连接树；不声称真实网络测试成功。
- Export 预览中的下载按钮会生成可用的演示 CSV 文件。
- Query / Schema / AI Assistant 标签会切换中央工作区内容；Schema/AI 使用单列结果布局，返回 Query 保留上次结果和分栏比例。
- Demo 查询支持字段投影、单条件 `WHERE`、`ORDER BY` 和 `LIMIT`；不支持的语法明确进入错误态，不伪造结果。
- 只读模式会阻止格式化、清空和草稿恢复等编辑器写入路径。
- 分栏拖拽扣除 5px splitter 宽度，比例保持在 24%~76% 之间。
- 主题切换不刷新页面，编辑器、结果和历史状态保持不变。

以上是源代码驱动的行为重建，不等同于上游应用的像素级复刻；真实数据库、凭证存储、Tauri 命令、CLI 和 MCP 仍属于后续接入范围。

## 验收证据账本

| ID | 要求 | 证据 | 状态 |
| --- | --- | --- | --- |
| DBX-01 | 页面可访问且能渲染主要工作区 | `curl` HTTP 200；Chrome headless 截图 `/tmp/dbx-audit/dbx-live.png` | PASS |
| DBX-02 | 服务未启动不冒充可用 | Web 标记 `demo only`，CLI/MCP 标记 `not connected` | PASS |
| DBX-03 | 编辑器接受输入并驱动 Run | Chrome dump DOM 确认运行时生成 `textarea.sql-input`；代码读取 `editor.value` | PASS（脚本级） |
| DBX-04 | 空态、错误态、成功态可到达 | `Run` 分支分别处理空字符串、非 SELECT、SELECT | PASS（脚本级） |
| DBX-05 | 连接校验并插入本地连接树 | `testConn` 校验名称/地址后插入 `.tree .node` | PASS（脚本级） |
| DBX-06 | Query/Schema/AI 标签改变工作区内容 | 标签处理器改变结果标题、内容和编辑区显示 | PASS（脚本级） |
| DBX-07 | 导出产生可用演示文件 | Blob + `<a download="dbx-demo-users.csv">` | PASS（脚本级） |
| DBX-08 | 上游源码到原型映射可追溯 | 本文“上游证据到原型映射”表 | PASS |
| DBX-09 | 真实数据库/Tauri/CLI/MCP 接入 | 本原型没有接入这些外部能力 | OUT OF SCOPE |
| DBX-10 | 代表性浏览器交互回归 | Chrome CDP 真实执行：默认查询、`orders` 投影/过滤/`LIMIT 0`、格式化保留字符串与注释、Schema/Query 往返、只读编辑保护 | PASS（代表性路径） |
| DBX-11 | 主题、导出内容和拖拽边界回归 | Chrome CDP 实际解析下载文件：5 行、正确换行；拖拽左右边界分别得到 `89.9531px 5px 290.297px` 与 `290.297px 5px 89.9531px`，未越界 | PASS |
| DBX-12 | 全部可见控制项逐项回归 | Chrome CDP 逐项断言 Query/Schema/AI、主题、设置、帮助、新增/取消连接、Run/Explain/Format/Save/Restore/Clear/Grid/Export/历史/文档/ER/AI/Inspector；结果对象 22/22 预期状态均为 true | PASS（逐项状态断言） |
