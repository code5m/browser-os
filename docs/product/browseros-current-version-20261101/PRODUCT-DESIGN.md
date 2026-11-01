# BrowserOS 当前版本产品设计（AS-IS）· 2026-11-01

> 目标：把 2026-11-01 用户实机提供的 26 张截图整理成当前 BrowserOS 的产品设计基线，而不是未来概念稿。
>
> 视觉证据：`docs/prototypes/browseros-ui2-aplus-20261101/screenshots/`
>
> 代码核对：`origin/master@5df888df`
>
> 状态标记：
> - **截图已验证**：实机截图中可直接确认。
> - **代码已验证**：最新 master 中已存在，但本批截图没有覆盖。
> - **待实机验证**：不能从截图或当前代码片段确认的交互结果。

## 1. 产品定位

当前 BrowserOS 是一套以 Chromium 浏览器壳为统一入口的桌面工作环境，在同一个 Tab / Omnibox / Bookmark Shell 中承载网页、本地文件、开发工具、AI Agent、数据库、Git、自动化和知识管理。

当前版本的核心不是“一个 IDE”，而是：

**Browser Shell + Built-in Workspaces + AI / Automation + Local Tools**

## 2. 当前产品结构

### Browser Shell
- 标签页
- 前进 / 后退 / 刷新 / Home
- Omnibox
- 收藏
- 书签栏
- 内部路由 `browseros://...`
- 底部状态栏

### Home / Launcher
- 主要工作区
- 快捷方式
- 最近访问
- 收藏管理

### Developer Workspaces
- Agent / Code Workspace
- Git
- Database Studio
- Terminal
- Files

### AI / Automation
- Skill Studio
- Agent Center / Agent Workspace
- Scheduled Tasks
- Script Library

### Knowledge / Content
- Knowledge Base
- Knowledge Graph
- BFC Content Library
- Browser-hosted local web apps

### Utilities / System
- Clipboard
- Settings
- Plugins
- Repository
- Audit
- Tool Center
- Commands

## 3. Browser Shell 设计

**截图已验证：01、03~26**

当前 BrowserOS 的产品辨识度首先来自 Chrome 风格 Shell，因此当前版产品设计把 Browser Shell 作为稳定层，不把内部模块各自的导航替代浏览器导航。

### 顶部层级

1. OS / Window chrome
2. Browser Tab Strip
3. Navigation + Omnibox
4. Bookmarks bar
5. Workspace content
6. Bottom status bar

### 交互规则

- 任意内部模块以普通浏览器 Tab 打开。
- 内部路由采用 `browseros://...`。
- 网页与内部模块并存，不建立第二套顶级窗口系统。
- Omnibox 同时承载 URL、历史、内部地址和本地路径。
- 地址栏候选当前为简单纵向列表（截图 24）。
- 收藏栏作为长期可见快捷入口。

## 4. Home / Launcher

**截图已验证：01、24**

### 页面目标
让用户在一个页面进入 BrowserOS 的全部核心能力，并恢复最近上下文。

### 当前组成
- 页面标题：主页
- 主要工作区：浏览、文件、终端、剪贴板、知识库、应用、脚本库、命令库、工具箱、数据库、定时任务、技能、智能体、知识图谱、插件、仓库、审计、BFC 内容库
- 快捷方式
- 最近访问
- 收藏页 / 收藏目录 / 新增 / 默认操作

### 当前交互
- 点击工作区卡片打开相应内部模块。
- 快捷方式支持编辑 / 删除。
- 最近访问可恢复目录或网页。
- 地址栏候选可在首页上方浮层显示。

## 5. Skill Studio

**截图已验证：02**

### 页面结构
- 顶部品牌区：BROWSEROS · SKILL STUDIO
- 总体统计：本地蓝图、步骤完成、已保存定义、安装与运行结论
- 左侧：我的技能 / 搜索 / 技能列表
- 主工作区：蓝图编辑
- 页签：概览 / 任务步骤 / 定义检查 / 权限与运行
- 右上：导入蓝图 / 新建技能 / 复制 / 导出 / 删除

### 当前交互
- 新建技能后在左侧形成技能项。
- 基础信息、目标、约束、输入、预期输出等通过表单配置。
- 保存目标为本机蓝图。

## 6. Agent / Code Workspace

**截图已验证：03、25、26**

### 页面目标
在同一工作区中完成项目上下文浏览、代码编辑、AI 规划与对话。

### 当前结构
- 顶部模式：编码工作区 / 智能体定义 / 蓝图
- 项目路径输入 + 打开项目
- 左侧 Activity Rail
- 左侧资源管理器
- 中央编辑器
- 底部工具输出 / 受控命令 / 问题
- 右侧 Agent：模型选择、刷新模型、Plan 模式、会话恢复、对话历史、输入框 / 发送 / 停止

### 已验证状态
- 未打开项目空态（03）
- 项目已连接（25、26）
- 本轮请求进行中（25）
- 模型回复完成（26）

### 当前边界
- Agent 可读取项目与工具上下文。
- 用户截图中的 Plan 模式说明为“分析规划”。
- 右侧会话属于项目会话。

## 7. Agent Runtime Lab

**代码已验证，截图未覆盖**

最新 `origin/master@5df888df` 新增 Settings 中的 Agent Runtime 实验面板：

- Native / Rig AgentRun / Canary 三种策略
- Canary 比例
- 最大模型轮次
- Benchmark 开关
- 应急回退策略
- Runtime benchmark
- 实际策略 fixture
- 只读工具验证：Browser / Git / Database / Coding LSP

该页面属于当前代码版本，但不列入“截图已验证”视觉基线；需要安装版补拍后再加入视觉验收。

## 8. Database Studio

**截图已验证：04、22、23**

### 页面结构
- 顶部 Database Studio + 当前连接
- 左侧：数据库连接、IDEA 导入、连接搜索、对象资源树、表 / 视图搜索
- 中央：SQL Tab、Query / Schema / AI Assistant、会话与事务状态、SQL 编辑器、查找 / 替换、查询历史、表数据 / 结果集
- 右侧 Inspector：当前连接、对象结构、Database Assistant

### 当前状态
- 只读事务
- 写事务入口存在
- SQL 查询执行与结果表格已验证
- 表数据浏览、筛选、导出入口已验证
- Inspector 独立于主查询工作区

## 9. Files

**截图已验证：05、08**

### 页面结构
- 左侧目录树
- 顶部当前路径
- 中央文件 / 文件夹列表
- 缩略图尺寸控制
- 右键上下文菜单

### 右键能力
- 新建文件 / 新建目录
- 新标签打开 / 资源管理器打开 / 命令行终端打开
- Git 改动 / 暂存 / 提交
- Git 提交历史 / Diff / 分支与远端
- Database 查找 IDEA 项目连接
- 收藏到主页
- 复制相对 / 绝对路径
- Git 查看目录提交历史 / 暂存目录 / 提交目录
- 重命名 / 删除

## 10. Git Workspace

**截图已验证：09、10**

### 页面结构
- 左侧 Commit / Shelf
- 变更文件 / 未版本化文件
- Commit message / Commit 按钮
- 顶部仓库 / 分支 / Update / Push
- 中间 Log / Branches / 筛选
- 右侧变更文件树 / commit metadata
- Diff 页面支持双栏对比

### 当前交互
- 提交历史选择
- 文件树定位
- Diff 双栏审阅
- Update / Push
- 暂存 / 取消暂存 / 提交

## 11. Grid

**截图已验证：06**

### 当前结构
- 群发输入
- 最新回复 / 提取回复 / 复制 / 重试失败项
- 成功 / 失败统计
- 对比工作区
- AI 组合
- 浏览 / AI
- 2格 / 4格
- 更多设置

### 当前截图状态
- 已选 4 格
- 工作区为空白

因此当前产品设计只能确认控制层，不能把 Grid 内容渲染效果标为已验证。

## 12. Knowledge Graph

**截图已验证：07**

### 当前结构
- 节点 / 边统计
- 查询 / 刷新 / 导入 JSON / 重建知识索引
- 起始节点 ID
- 搜索
- 类型过滤：文件 / 目录 / 页签 / 脚本 / 技能 / 智能体 / 标签 / 主题

### 当前截图状态
- 0 节点 / 0 边
- 空态提示“点击刷新从后端载入”

## 13. Scheduled Tasks

**截图已验证：13、14**

### 列表页
- AUTOMATION WORKSPACE
- 任务数 / 运行中数量
- 刷新 / 新建任务
- Shell 脚本 / 命令片段 / Agent 任务类型图例
- 运行历史

### 新建任务弹窗
- 任务名
- 类型
- 执行目标
- 触发方式
- Cron
- 错过执行策略
- 补跑上限
- 重试次数
- 退避
- 基准延迟 / 最大延迟
- 超时
- 迟到宽限
- 启用开关

当前版本明确默认关闭任务，确认后才会无人值守运行。

## 14. Terminal

**截图已验证：15**

### 当前结构
- 独立 `browseros://term`
- 单 / 2 / 4 / 9 布局
- 新建终端
- 自动确认 CLI 开关
- 终端主视图

截图中已验证 Java / Maven / Gradle / Node.js / npm 环境输出。

## 15. Clipboard

**截图已验证：20**

### 当前结构
- 粘贴 / 复制 / 清空历史
- 当前文本编辑区
- 本次会话历史
- 历史项“使用 / 复制”

## 16. Script Library

**截图已验证：21**

### 当前结构
- 脚本 / 历史
- 新建脚本
- 空态

## 17. Content Library / Local Web Apps

**截图已验证：16、17、18、19**

BrowserOS 当前允许普通网页 / 本地 Web App 与内部页面共享同一浏览器 Shell。

本批截图中的“自选”内容库展示：
- 专题书架
- 全部视频
- 继续观看
- 数据管理
- 分类 / 置信度 / 重复检查
- 账号访问与安全恢复
- 离线任务中心
- 收藏夹扫描覆盖率

这些页面属于网页内容本身，不属于 BrowserOS 原生内部页面设计；当前产品设计只规定其运行在 Browser Shell 中，不强制继承 BrowserOS 内部组件样式。

## 18. Settings

**截图已验证：11、12；代码补充：Agent Runtime Lab**

### 当前截图中的设置
- 统一 AI 配置
- 全局默认模型
- Database / Git / Browser / Workbench 模型继承
- 模型列表 / 插件凭据
- 外观
- 快捷键方案 / 快捷键映射
- 自适应资源休眠
- 默认浏览器
- 模块管理
- 工程健康 / 验证与证据
- 关于 / 版本号

### 最新代码新增
- Agent Channel · Runtime Lab

## 19. 当前视觉系统

当前版本客观上存在两套主要视觉密度。

### Light Utility Surface
用于 Home / Files / Skills / Settings / Tasks / Clipboard / Scripts / Graph。
- 页面背景：近白 / 浅灰
- 边框：浅灰蓝
- Primary：系统蓝 / 部分任务使用绿色
- 内容以表单、列表、卡片为主

### Dark Professional Surface
用于 Agent / Database / Git / Terminal。
- 深蓝黑背景
- 面板分隔线明显
- 高信息密度
- 状态色用于 Git / SQL / Runtime
- 适合长时间开发工作

当前版产品设计不把两者强行统一成单一页面模板；它们通过 Browser Shell、标题、控件语义与状态规则建立共同产品身份。

## 20. 当前主要交互模式

- Tab 打开 / 关闭 / 切换
- Omnibox 导航
- 内部路由
- 左右分栏
- 三栏专业工作区
- 表格 / 树
- Tab 内二级 Tab
- Modal 配置
- 右键菜单
- Empty State
- 状态栏
- 本地路径作为工作区入口
- Preview / Diff / Inspector

## 21. 当前版产品设计基线图

| 模块 | 截图 |
|---|---|
| Home | 01、24 |
| Skills | 02 |
| Agent / Coding | 03、25、26 |
| Database | 04、22、23 |
| Files | 05、08 |
| Grid | 06 |
| Graph | 07 |
| Git | 09、10 |
| Settings | 11、12 |
| Tasks | 13、14 |
| Terminal | 15 |
| Content | 16、17、18、19 |
| Clipboard | 20 |
| Scripts | 21 |

## 22. 当前版已知设计问题（AS-IS 事实记录）

- 首页功能入口同级数量较多。
- 浅色工具与深色专业工具存在视觉割裂。
- 地址栏候选信息层级较弱。
- Grid 空态缺少下一步引导。
- Knowledge Graph 空态信息较少。
- Settings 页面较长。
- Agent 的模型 / 模式 / 会话 / 工具输出控制区密度较高。
- Database / Git 功能成熟度高，但工具条主次需要进一步统一。

未来目标改版见独立 A+ 原型与比较文档，不在当前版本设计中混写。

## 23. 验收原则

当前版本产品设计的“通过”意味着：

1. 页面与 26 张实机截图一一可追溯。
2. 页面结构与当前代码能力不冲突。
3. 截图未覆盖的新代码能力明确标注。
4. 不把未来 A+ 方案写成当前事实。
5. 不因设计文档存在而声称底层功能完成。
