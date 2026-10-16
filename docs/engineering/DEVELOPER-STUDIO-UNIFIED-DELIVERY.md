# Developer Studio 统一交付记录

## 用户已确认的要求

2026-10-16：将 Git / Database 与 IDEA 对比清单中的功能作为一次完整交付，不拆成 P0/P1/P2，不把已有按钮、代码草稿或通用 CI 通过当作全部功能完成。继续保持 BrowserOS 的浏览器和文件资源管理器稳定、低资源占用、模块边界和中文交互。

基线：master / PR #34 / `1ddc1de1e4fa49bff88523081d0e5d5841706f2e`。
开发分支：`feature/developer-studio-unified-20261016`。
当前状态：**IN_PROGRESS / NOT_READY_TO_MERGE**。

## 完整交付范围

- Git：原始本地与托管仓库统一上下文，HEAD/Index/Working Tree 状态，文件/块/行暂存与取消暂存，精确提交与未发布 Amend；真实全文 Diff、后端分页历史筛选、DAG、Blame、rename-follow、分支比较；分支、远端、Fetch/Pull/Push、Stash 全生命周期、三方冲突、Merge/Rebase/Cherry-pick 的继续与中止、Worktree；GitHub PR 创建/详情/评论/Review/合并与按提交绑定的 CI；独立于 Git 的有界、明确授权文件历史与恢复。
- Database：每个 SQL 文档固定数据源身份，正确关联 schema/结果/历史/风险；语法高亮、元数据补全、格式化、当前/选中语句执行与诊断；按需且有上限的原生会话、事务、Commit/Rollback、取消与关闭清理；有主键的数据编辑、参数化 DML 预览、乐观并发、原子提交；稳定的服务端排序/过滤/分页；MySQL/SQLite/PostgreSQL 真实查询、TLS、授权 SSH 隧道；对象目录、ER、Schema Diff、迁移预览、非执行式计划展示；CSV/TSV/JSON/SQL 导入导出；IDEA 非秘密配置导入及经确认更新。
- 产品：复用停靠式工作台、错误/加载/空状态、快捷键和菜单。禁止在原生网页上增加 HTML 浮层，禁止另造互不一致的工作台。
- 工程：维持原有安全限制、系统密钥库、单一 GitHub 主源、Native/语义登记、Hot-Plug；不提高 25.2% 体积阈值，不换基线，不隐藏产物，不把 skipped 当作实际验收。

## 本次恢复点

中断前创建的 Git Studio 原生草稿已恢复为 `src-tauri/src/capabilities/git/studio.rs`。它需要完成模块注册、前端调用、权限登记、安全复核和编译/行为验证；目前不得宣称已可运行。

用户设备目前离线，不能把 GitHub Runner 的 GUI 验证冒充用户已安装客户端的验收。所有开发提交仅写开发分支，未通过完整验收不得合并 master。

## 验收与完成定义

必须记录同一集成提交上的真实测试：成功路径、拒绝路径、过期/变更预览、连接切换、事务回滚、冲突恢复、资源清理、前端生产构建、Rust fmt/check/test、安装版 GUI、完整 GUI、Full Validation 和 pre-merge。MySQL/PostgreSQL/SSH 等外部服务测试必须注明实际运行或未验证。

所有范围在对应实现、接线和测试证据齐全前保持 OPEN。任何中断均从本文件与分支最新提交继续，禁止以重新写计划替代实现。
