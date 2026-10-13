# BrowserOS Database Studio + Git Studio 统一产品与工程蓝图

状态：**IMPLEMENTED_SUBSET / RELEASE_GATED**（2026-10-13）。
范围：对标 JetBrains IDEA / DataGrip 的操作信息架构，绝不复制整套重量级 IDE；浏览器、文件资源管理器为核心，Database/Git 为独立按需启用的 Capability。
本文件区分已经写入代码的功能、可核验的测试与尚未实现的能力。不能用“前端按钮已存在”替代原生执行证据。

## 用户操作路线（统一验收用例）

- 数据库：选择数据源 → 按需打开连接设置 → 测试/保存连接 → 展开资源树 → 单击表查看列、索引、外键和 DDL → 双击表读前 100 行 → 服务器端按 100 行翻页 → SQL 编辑、运行、EXPLAIN、导出 CSV。
- Git：选择受控仓库 → 按冲突/未跟踪/已跟踪对象分组 → 打开真实并排 Patch → 显式勾选路径 → 查看原生双阶段写预览 → 仅提交指定路径 → 查看提交 DAG/真实历史 → 确认后执行非 force push。
- 安全：数据库默认只读；凭据仅 OS Keyring；UI SQL 历史只能用户主动打开并保留在内存，关闭/禁用清除；写 SQL 必须按 Rust 的安全分级及生产保护执行。Git 无选择不得提交，原生 scoped commit 不得顺带提交已暂存的其他文件。
- 视图：对象树与 SQL 编辑/结果分别占据可读区域；编辑与展示皆带错误提示；专业功能不可伪装为已接入。

## 已实现（当前分支）

| 子系统 | 功能 | 实际约束 |
| --- | --- | --- |
| 数据库 | MySQL/SQLite 实际列目录、索引与 DDL，外键详情与选中表局部 ER | 目录查询为只读 SELECT/SHOW，最多 512 列/索引，128 外键，局部 ER 最多 6 条 |
| 数据库 | 双击表打开 SQL 页签并执行、每页 100 行服务器端 LIMIT/OFFSET | 不估计表总行数；offset 仅安全整数、100 的倍数、≤ 50000 |
| 数据库 | EXPLAIN（MySQL SELECT only）、原生 SQL 风险/权限复核 | 明确拒绝 EXPLAIN ANALYZE 和未支持的 SQL，不自动突破读写闸门 |
| 数据库 | SQL 关键字快速插入、会话 SQL 历史（默认关闭）、CSV 复制与受限导出 | 历史最多 30 条、只在内存；CSV 最多 5 MiB；不存口令 |
| 数据库 | 更清晰的连接状态与按需展开的连接表单 | “最近验证成功”不表示长期持有数据库连接 |
| Git | 文件状态分组、真实变更片段并排/补丁视图、文件范围预览 | Diff 只读、最多 4,000 行片段；原生 256 KiB 截断如实提示 |
| Git | 指定路径提交、无勾选拒绝、后端真实隔离已暂存的其他文件 | 不隐式提交全部；使用当前 HEAD 树+受控 Index，保留未选择的暂存内容 |
| Git | Git 提交历史的局部 DAG 可视化和真实提交差异 | 只展示真实元数据的最近 200 条提交 |
| 构建 | Xterm 已隔离 Vendor Chunk 的私有标识符压缩 | 保留公开 API 和 _core；只改该 chunk；Chrome 真实 DOM、FitAddon、写入、Resize smoke PASS；Rust/GUI 仍需全验收 |

## 仍未实现、不得宣传为已完成

1. 完整 CodeMirror 6 SQL 自动补全、格式化、代码诊断和多语句/SQL 方言专业编辑器。
2. 完整数据编辑器：行级/单元格修改与显式事务/撤回/恢复。当前不提供无授权的数据写入。
3. 任意规模的分页数据、排序/筛选服务端生成器：目前只有有界表预览分页；筛选/排序仍作用于已加载行。
4. 全库 ER 可视化、数据库模式差异及迁移脚本：目前仅展示单表局部外键关系。
5. Git 行级暂存与三方冲突合并、Fetch/Pull/Stash/远端管理器：不可在缺少原生安全命令/闸门情况下先出现假的可执行按钮。
6. 应用内 GitHub PR/CI 合并操作和会话身份管理：当前 GitHub 合并依赖已有独立连接器与 CI，Git Studio 仍仅有基本仓库读写操作。

## 扩展技术方案

- Database：沿用既有 `useDatabaseStore` 和 Rust `database.rs`；单对象元数据懒加载、generation fence 防异步错位；数据库 Result 最大行数/超时/字节由原生安全边界决定。真正数据编辑必须追加事务控制、预览差异、生产环境确认、回滚证据，不可绕开已有 SQL 安全分类器。
- SQL 编辑器：未来采用按需加载的轻量 CodeMirror 6，仅数据库视图启用；SQL 格式化/解析须不改执行语义；SQL 编辑内容不跨数据源静默复用。
- Git：沿用 `useGitStore`、`git/write` 双阶段 job、`libgit2` 与密钥库；支持 Fetch/Pull/Stash 时必须引入精确操作类型、权限和状态机、网络错误脱敏、取消/过期、仓库 HEAD/工作区冲突保护；任何 GUI “成功”必须对应 Rust 实际运行回执。
- Git 历史：由真实父提交拓扑绘制，不推断不可见提交。Diff 是受限原始 Patch 的双列投影，非全文件对照。
- 统一体验：Database/Git 共用轻量控件和语义状态颜色，不增加 Browser WebView 浮窗或同步阻塞；避免复制大体积依赖。
- 优化策略：不提高 M0 包体积阈值、不替换基线。Terminal Vendor 私有标识符压缩仅在 Rollup renderChunk 上生效，确保输出内容哈希随编译结果变化；需要安装版实际终端验证。

## 验收闸门与证据

1. Node 回归：`node scripts/check-database-ui-logic.mjs` / `node scripts/check-git-ui-logic.mjs` / `node scripts/check-semantic-registry.mjs`。
2. Rust：`cargo fmt --all --check`、`cargo test ... scoped_commit_excludes_unrelated_pre_staged_files`、`cargo check --locked`，完整 CI 复核。
3. 资源：`npm run build`、`measure-build-metrics.py --compare ... --skip-build` 不超历史 25.2%。
4. 浏览器最小终端：实际 Chrome DOM 中载入打包 xterm、FitAddon、Terminal.write、Resize，所有观察值不伪造。
5. `npm run check` + `bash scripts/pre-merge.sh` + GitHub 相关完整工作流；未绿不得合并 master。
6. GUI / 安装版：冷启动并验证 MySQL/MySQL 8 真实服务、Git 暂存与提交实际隔离、终端完整键入与动态 resize、浏览器文件基础功能；不以截图模拟代替实跑。
