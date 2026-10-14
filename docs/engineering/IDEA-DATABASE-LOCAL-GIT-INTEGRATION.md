# IDEA 数据库配置与本地 Git 工程统一设计（2026-10-14）

状态：PR #32 DRAFT / NOT MERGED。本文区分实际能力和待实施内容。

## IDEA 数据库配置
用户在文件管理器右击 IDEA 项目文件夹或文件，选择「数据库 · 查找 IDEA 项目连接」，Database Studio 只检查选中路径及最多 5 个上级目录中的 .idea/dataSources.xml，不递归扫描整个家目录。也可以手动输入项目路径扫描。XML 上限 512 KiB，最多 128 条数据源。

支持典型 MySQL JDBC URL 与绝对 SQLite JDBC 文件；不支持的 PostgreSQL、SSH 隧道和特殊驱动会明确标识。选择连接后仅填充非秘密的名称、host、port、database、username；默认只读、生产判定未知、MySQL 默认安全 TLS prefer。不读取 IDEA 的 dataSources.local.xml、Keychain、KeePass、密码或 SSH 凭据，也不修改 IDEA 配置。JDBC URL 查询参数中的密码不会进入候选模型或 UI。

用户补全密码后通过原生连接验证。非秘密配置以 0600 权限的应用数据文件原子保存，密码只存 OS Keyring。下次启动从持久配置恢复，查询通过 Keyring 取密钥按需建连。「断开当前会话」只清除当前 UI 选中态，「删除连接及凭据」需要确认并删除 BrowserOS 的配置与密钥，绝不影响 IDEA 原文件。

限制：本 PR 不提供实时监听 IDEA 文件变更，不复用 IDEA 连接池/密码库，不支持自动 SSH 隧道。

## 文件/目录 Git 右键
右击文件或文件夹 → 「Git · 查看提交历史」或「Git · 查看未提交的改动」。Rust libgit2 从用户选中的原始文件路径发现真实仓库，兼容 worktree 的 .git 文件，不会额外 clone，也不会自动 fetch/pull。命令执行来源校验、真实路径 canonicalize 和允许根目录验证；不允许读取 .git 内部路径或裸仓库。

Git 历史按所选文件/目录的相对路径分量过滤，支持分页，每页最多 75 条、扫描最多 4000 提交，不会伪称能无限读取；历史跨文件重命名的链路不保证全部自动跟随。已提交和工作区补丁上限 256 KiB，超限在 UI 上明示。无 Git 仓库、未授权目录、离线场景均有明确错误。

## 专业 Git 写功能边界
旧 Git Studio 支持托管仓库下 stage/unstage/commit/branch/checkout/push，通过现有双阶段写入闸门；新增本地目录入口仅限安全只读。要实现 IDEA 全功能，还需本地可写授权事务、行级暂存、fetch/pull、stash、三方冲突、rebase、cherry-pick 和 PR/CI，不能直接把托管克隆写接口偷换成任意本地目录的写权限。未完成能力必须标识，不得显示伪按钮。

## 合并验证
Node UI/逻辑测试、Rust fmt/check/test、生产构建、原生安全与语义登记、MySQL 8 E2E、Hot-Plug 和安装版 GUI。任何门禁失败不得降低阈值；PR 在完成验收前维持 Draft。
