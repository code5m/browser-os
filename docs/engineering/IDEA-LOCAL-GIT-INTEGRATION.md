# IDEA 数据库配置与本地 Git 资源管理器集成

## 当前落实
- Database Studio 已提供**手动选择 IDEA 项目 `.idea/dataSources.xml`** 的导入入口。
- XML 解析只在前端内存进行，不上传原文件；限制 2 MiB / 200 数据源，拒绝 DTD/实体；仅解析能直接识别的 MySQL JDBC 地址。
- 导入只填入 BrowserOS 的 MySQL 连接表单；强制初始只读、TLS require、生产标记未知、密码为空。用户必须核对配置并实际验证才能连接。
- 遇到 SSH/代理/证书、非 MySQL 数据库与无法识别的 JDBC 配置时拒绝自动导入，避免出现假的“连接成功”。
- 不读取、不修改 JetBrains 原文件及其 Password Safe，不复制密钥；不以读取 IDE 数据源代替数据库查询门禁。

## 与 IDEA 的完整集成目标（仍未实现）
1. OS 授权下发现本地 JetBrains 项目和 `.idea/dataSources.xml`；增量更新、去重、来源指纹和显式确认。
2. 持久化 DataSource Registry；现有 `DbConnectionRegistry` 仅进程内存，重启后配置会丢失。将断开连接与删除连接分开、凭据仅 OS Keyring。
3. Git 文件/目录右键需要先绑定**真实本地 Git 根目录**，不能使用当前 `repo_id` 管理的应用克隆替代。添加仓库发现、嵌套/worktree 边界、只读 path scoped history/diff/blame，再接原生双阶段写任务。
4. 后续 Git 索引状态、逐块暂存、分页提交 DAG、Fetch/Pull/Stash、冲突合并、GitHub PR/CI 需要完善真实权限、回滚与全 GUI 回归。

## 发布限制
不得称本 PR 已经实现自动扫描或任意本地 Git 右键操作。完整能力只能在底层仓库来源和数据源持久化契约真实接通后放行。
