> 基于最新 master 的生产能力复审与设计准入：[复审报告](../../product/BROWSEROS-UI2-MASTER-REAUDIT-C90EB564-20261101.md)（master@c90eb564）。

> **审核状态：NOT READY。** 独立原型目前仍存在浏览器交互、路由、任务默认安全策略和窗口布局缺陷。详见 [正式审核报告](../../product/BROWSEROS-UI2-DESIGN-AUDIT-20261101.md)。

# BrowserOS UI 2.0 A+ Prototype

分支：`browseros-ui2-aplus-20261101`

## 分支边界

本目录是 **2026-11-01 全新 A+ 多页面产品原型**，与上一版 `design/browseros-ui2-aplus-20261101` 隔离。

- 基于最新 `master@8a5e8380592bb772c7c9e5f63788f7443579169a`
- 不继承上一版 `docs/prototypes/browseros-ui2/**`
- 不修改 `src/**`
- 不修改 `src-tauri/**`
- 不修改 `tauri-browser-tabs/**`
- 不代表生产功能已迁移

## 打开方式

直接用浏览器打开：

`docs/prototypes/browseros-ui2-aplus-20261101/index.html`

这是单文件 HTML 原型，不依赖后端，不执行真实 Git / SQL / Shell / Agent 副作用。

## 设计目标

1. Chrome 级浏览器操作习惯。
2. Apple 式层级、间距、圆角和克制感。
3. 专业工具区允许高信息密度。
4. 全页面使用统一导航、按钮、卡片、状态和间距。
5. 功能性与交互性优先于纯视觉装饰。
6. BrowserOS 保持“浏览器 + 工作区 + Agent”的独立产品身份。

## 当前原型覆盖

- 工作台 / 首页
- 编码工作区 + Agent
- Database Studio
- Automation Center / 定时任务
- Knowledge & Content Center
- 开发工具中心（终端 / 剪贴板 / 脚本库）
- 文件
- Git
- Skill Studio
- Knowledge Graph
- Settings

详细比较：`../../product/BROWSEROS-UI2-APLUS-COMPARISON-20261101.md`

## 实机截图基线

本轮 UI 评估使用的 26 张原始截图已归档到 [`screenshots/`](./screenshots/README.md)，按对话出现顺序编号，并通过 SHA-256 与会话上传附件逐字节校验。
