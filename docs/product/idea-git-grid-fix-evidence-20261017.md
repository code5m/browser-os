# BrowserOS IDEA / Git / 宫格修复阶段：技术裁决与实际验收

> 2026-10-17。对应 [IDEA 完整对标审计](./idea-database-git-parity-audit-20261017.md)。当前状态：**代码已提交 PR #41，必须以该 PR 同一 HEAD 的自动化及真实 Ubuntu GUI 证据判断可交付，未验收前不可写「完成」。**

## 1. 真实故障与根因

| 领域 | 根因（源码定位） | 处理 | 客观边界 |
|---|---|---|---|
| Grid 2 格空白/拒启动 | `src-tauri/src/bridge.rs` 的 `MIN_GRID=4` 与前端可选 2 格不一致，核心内存保护拒绝低内存 4 格 | 允许 **2 格**，仍保留 1800MB 核心内存预留与资源检查 | 无法保证所有机器有足够 2 格内存 |
| Grid 连续切换格数 | `useGridStore.buildGrid()` 原来直接丢弃创建中的新请求 | 记录末次重建意图，前次完成后再处理；关闭过程代际保护 | 真正 WebKit/Wayland/X11 窗口层级仍需装机运行 |
| Grid 关闭中间格 | 前端数组左移，原生 `grid-N` 子进程下标未同步 | 关闭后按保留 URL 重建连续 `grid-0..n-1` | 重建会重新加载网页，会丢失原页面临时会话输入，应继续研究无损重排 |
| Grid 网址 | `gridSetUrl()` 仍含百度默认回退 | 改为空 `about:blank` | 已配置网站登录状态需独立检测 |
| Git 提交详情 | `commit_diff` 只取整块 `git show`，UI 无逐文件审查 | 新增 `commit_files`、`commit_file_diff`、`commit_file_worktree_diff`；左分支、中提交、右 Changed Files / Diff Preview | merge commit 当前按**第一父提交**差异，二进制/大文件不显示虚假的文本差异 |
| IDEA 路径 | 猜 `~/IdeaProjects`、仅打开单个项目 XML | 用户显式启动有界只读发现：已授权文件工作区（包含指定项目）→ `.idea/dataSources.xml` | 不扫描未授权全盘、不会隐式读取 JetBrains 隐藏全局配置或密码 |
| IDEA 一键导入 | 原代码只填表，未保存批量配置 | 新原生命令 `db_import_profiles` 最多 32 条，原子持久化、去重、`enabled=false`、`allow_write=false` | 导入元数据**不等于连通数据库**；每条须单独输入凭据验证、激活 |
| IDEA 全局数据源 | 项目 XML 与全局配置不是同一位置 | 保持原“手动选 XML”入口并明确局限 | JetBrains 全局导出 ZIP 自动解析尚未实现，须显式导出/授权才可导入 |

## 2. 研发与安全约束

- 宫格的资源模型：原生子 WebView 与顶层窗口，经 X11/Wayland 定位，HTML 的 CSS z-index 不可假装覆盖它；空白应分为 **未获 ACTIVE 许可 / 内存保护 / Native 创建失败 / 宿主 rect=0 / 子进程崩溃 / 页面自身登录失败** 六类取证。
- 保留 Grid capability ACTIVE 判定、单一重资源 owner、非浏览器/宫格视图的隐藏逻辑、Grid 进程资源回收。不能为了视觉问题取消重资源门禁。
- IDEA 批量导入只持久化 MySQL/SQLite 安全元数据，凭据**不得**复制自 IDEA、不得写入 DB JSON / 日志。禁用状态必须和连接成功状态区分；链接测试后才可改为启用。原始 `dataSources.xml` 只读，不触碰 `dataSources.local.xml`、Keyring。
- 本地 Git Diff 不能隐式 fetch、checkout、覆盖工作区。只读 API 对 SHA、相对路径严格校验；历史 Diff 基于 Git 对象，不通过 GitHub 的 PR API 伪造本地历史。
- 前端体积遵循已批准的 **30% 预警 / 35% 硬上限**，不能降低 Rust / GUI / 数据库 / 权限 / 供应链 / 语义检查。

## 3. 自动化与人工验收记录

以仓库 CI 实际输出为准，待本 PR HEAD 确定后填写：

| 测试 | 所需证据 | 当前状态 |
|---|---|---|
| IDEA 路径发现 | `scripts/check-idea-product-parity.mjs` 两项目、无授权、取消扫描样本 | 代码已加入 CI，尚未验证最终 SHA |
| IDEA 原生批量保存 | Rust 编译 + `db_import_profiles` 的权限、去重、上限、禁用态 | 未取得完整真实 GUI 证据 |
| Git 历史逐文件 | 原生临时真实 Git 提交、root/普通/本地变更，Diff 输出 | 测试已写，待 Rust 测试 |
| Grid 生命周期 | `check-browser-grid-navigation-regression.mjs` + 真实 Tauri GUI，2/4 格、重建、关闭单格、视图切换 | 尚未完成 Ubuntu 装机人工验证 |
| Full Validation + MySQL E2E | 同一提交 6/6 工作流 ALL_PASS；门禁不得跳过 | 等待 PR HEAD |
| 安装版 UI | 按真实工作流：Grid 2→4→2、缩放/重开；IDEA XML 导入→验证；Git 选择提交→文件→Diff | **未执行**，不得伪报 |

## 4. 继续调研与产品债务

必须继续补齐并另行严格验收：
1. JetBrains 官方的**全局数据源**配置，区分显示、导出 ZIP 方案、非默认路径；考虑受控 native 读取与 ZIP 解码限制。
2. Git Log 的独立文件历史 / rename-follow / 比较两个任意提交 / Merge parents / Ctrl+D / Diff Preview 下方或右侧自定义定位；按用户之前的矩阵一项项核对。
3. Grid 在关闭中间格时尽可能不重建其他 WebView：先取得 Native 子进程重编号可行性，考虑状态持久化；测试真实服务站点登录失败与资源不足降级。
4. 原生浏览器与宫格地址栏悬浮建议：需要 Linux GTK 原生悬浮方案，不是 CSS absolute/position fixed。

参考官方文档：
- [JetBrains Data Sources](https://www.jetbrains.com/help/idea/managing-data-sources.html)
- [JetBrains Git Log](https://www.jetbrains.com/help/idea/log-tab.html)
- [JetBrains Diff](https://www.jetbrains.com/help/idea/comparing-file-versions.html)
