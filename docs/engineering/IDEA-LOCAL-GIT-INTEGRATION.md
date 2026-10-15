# IDEA / 本地 Git 集成 — 文档入口

本文件保留早期 PR #33 的上下文。请以 [统一设计与验收清单](./IDEA-DATABASE-LOCAL-GIT-INTEGRATION.md) 为当前事实来源，不要把旧的开发阶段文字视为产品已交付状态。

- 数据源导入有两条通道：授权项目目录 .idea/dataSources.xml 扫描，以及用户主动选择 XML 文件进行浏览器内存预览；不读取 JetBrains Password Safe 或任何明文密钥。
- 数据库非秘密元数据持久保存到 BrowserOS 应用数据目录；密码仅存操作系统 Keyring；断开当前 UI 会话不删除配置，明确删除操作才撤销元数据与 BrowserOS 对应凭据。
- Git Studio 提供真实本地 worktree 的分页历史及工作区补丁；Git Capability 提供动态文件菜单与只读弹窗，二者复用同一个 Inspector 的展示、分页和安全错误处理，不复制仓库，也不执行 Fetch/Pull。
- Git 本地可写暂存/提交、Fetch/Pull、Stash、三方冲突合并、Blame/rename-follow、GitHub PR/CI 的完整原生写闭环尚未完成，不得显示假可执行入口。
- 以 GitHub master 分支为唯一主源，以六项工作流全绿及整合提交的完整验收为合并前提。
