# BrowserOS 独立产品蓝图：知识中心、智能体中心、数据库与 Git

> BrowserOS 自主设计，不复用 Asset Workbench V3 的架构、状态或数据模型。
> 本文是下一阶段产品契约。实施与验收状态以代码和 CI 证据为准，设计不算功能已交付。

## 一、基本定位

BrowserOS 是浏览器 + 文件资源管理器构成的桌面工作现场；知识能力是上下文，智能体通过经授权的技能完成任务。主界面不再依赖占据空间的常驻工具侧栏。开发者工具采用 IntelliJ IDEA 熟悉的工作流程，但继续遵守 BrowserOS 单 owner / Capability Public API / Rust 权限边界及低资源预算。

## 二、Database Tool Window

IDEA 参考要素与 BrowserOS 自主取舍：

- 连接资源管理器：连接配置、最近连接、连接状态、按数据源分类、Schema → Table/View → Columns/Indexes 展开；具有过滤输入和刷新按钮。
- SQL Console：可复用多文档 Tab，Ctrl+Enter 执行，SQL 草稿保留/恢复与关闭前确认；不同 Tab 和连接关系明确。不要把 SQL 编辑状态塞进主浏览器地址栏。
- 对象预览：点击表生成受限制 SELECT 查询；结构视图展示表 DDL、列类型、主键、索引。读取元数据走现有后端安全闸门，不绕过只读限制。
- 数据表结果：分页、列排序、筛选、复制 CSV。当前为最多 1000 行的单次查询和局部筛选排序，**并非真正数据库服务端分页**；需另开发参数化服务端分页及行级可编辑提交。
- 支持矩阵：SQLite 已有真实查询链路；Rust 端有 MySQL/PostgreSQL 连接握手，但查询返回未支持，故不能把数据库下拉中的两项直接标记可用。
- 安全：默认只读；写操作和生产环境风险二次确认；密码只经 OS keyring，不进 Vue store、localStorage、日志和 Git；大结果与超时强制截断、取消可用。

## 三、Git Tool Window

- Changes 视图：IDEA 风格的文件筛选、未跟踪、冲突聚合、单文件 Diff、暂存与取消暂存。后续精确区分 INDEX 与 WT 状态须更新 Rust GitFileStatus DTO，不能依据一个笼统 status 字段猜 staged/unstaged。
- Log 视图：提交历史、作者/消息搜索、选中提交查看差异、复制 SHA，分支可见、限制列表大小。
- Branches 视图：当前分支、新建和切换本地分支。Remote status/outgoing/incoming、fetch/pull 和 rebase 尚未提供安全受控 IPC 时绝不伪装已实现。
- Commit / Push：复用现有 request → confirm 双阶段闸门，特别保护 discard、push，禁止 force 和隐式写操作；所有操作完成后刷新源状态。
- Credentials：仅密钥库引用；不将 Token 暴露前端。
- 凡用户无法完成相应闭环的能力，明确列为债务，不能仅有装饰性按钮。

## 四、知识中心

- 资料库：内容资源、来源、标签、版本与可证明摘要。
- 笔记：人工观点与原始资料片段的引用关系。
- 关系图谱：从知识资产衍生，不复制第二份真实数据；图谱关系有来源、版本、置信度、人工/机器来源字段。
- 检索：关键词 + 结构化关系优先，语义索引插件可选，避免后台无端常驻大模型。
- 核心体验：浏览网页或打开本地文件→引用资料→知识中心统一检索→图谱探索→点击回源。

## 五、智能体中心

- Agent 是目标、会话、权限范围、执行状态与任务记录的所有者；Skill 是可复用、可版本化、经过许可的操作能力。
- 交互层级：智能体 → 已授权技能、知识访问范围、会话、运行记录。Skill 与 Agent 在底层模块化边界保持解耦。
- 使用现有 Rust ScriptRunner/Permission Gate，不提供可绕过授权的二次命令执行接口。
- 目前 Agent/Skill 的安装/执行后端不完整，绝不通过切换前端 flag 假装可用。

## 六、本次切片与验收

本轮可实际合入的切片：安装前锁定依赖恢复和构建失败硬停；数据库对象搜索并复用既有只读表预览。Git Changes 路径查找/状态聚合、SQLite DDL 结构查看等功能尚未实现，仍为 OPEN。本轮曾尝试 UI 方案，但冻结体积预算未通过，已主动回撤，不宣称“已经替代 IDEA”。

关闭条件：真实命令成功及失败输入测试、权限拒绝、正确提示、TypeScript/原生边界、Rust/GUI、体积 25.2% 门禁、完整 pre-merge、GitHub CI 与最终合并。
