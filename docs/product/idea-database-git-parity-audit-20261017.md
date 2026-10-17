# BrowserOS × IntelliJ IDEA 数据库与 Git 工作台：对标研究、实现审计及端到端验收契约

> 日期：2026-10-17。研究依据：JetBrains IntelliJ IDEA 2026.2 官方帮助 + BrowserOS `master` 合并提交 `197bb1d731168e0a0a235755199071dc93d827a9` 的源码 + 用户提供的 Ubuntu 两张实际截图。
> **裁决：当前未达到「一键导入 IDEA 数据源」和「IDEA 级 Git 历史差异预览」的产品验收标准。此前把“有入口、能读 XML/补丁”说成“功能完成”，属于需求理解和验收覆盖遗漏。**
> 状态说明：✅ 已接入；◐ 基础存在但端到端未完成；❌ 缺失；⚠ 需要实际 Ubuntu 安装版取证。文档为**审核及实施规范**，不冒充功能已经交付。

## 1. 研究依据及边界

官方来源（访问于 2026-10-17；官方 2026.2 Help，后续版本可能变化）：

- [JetBrains: Data sources / 管理数据源](https://www.jetbrains.com/help/idea/managing-data-sources.html) — 项目级、全局级、`.idea/dataSources.xml`、导出设置 ZIP、密码边界
- [JetBrains: Data Sources and Drivers dialog](https://www.jetbrains.com/help/idea/data-sources-and-drivers-dialog.html) — 数据源元数据、连接验证
- [JetBrains: IDE directories on Linux](https://www.jetbrains.com/help/idea/directories-used-by-the-ide-to-store-settings-caches-plugins-and-logs.html) — Linux `~/.config/JetBrains/<product><version>`，可由用户自定义
- [JetBrains: Git Log tab](https://www.jetbrains.com/help/idea/log-tab.html) — 左分支、中提交、右变更文件及提交详情
- [JetBrains: Diff viewer](https://www.jetbrains.com/help/idea/comparing-file-versions.html) — 逐文件历史 Diff、并排/统一、快捷键、与本地对比
- [JetBrains: Compare branches](https://www.jetbrains.com/help/idea/manage-branches.html) — 分支与 HEAD/工作树对比
- [JetBrains: Investigate changes](https://www.jetbrains.com/help/idea/investigate-changes.html) — 比较两个提交、文件历史
- [JetBrains: Git Log settings](https://www.jetbrains.com/help/idea/settings-version-control-log.html) — Diff Preview 独立面板与可配置列

**不能推断**：截图不能证明机器上究竟有多少 IDEA 项目、IDEA 是否保存过全局数据源、用户目录是否对 BrowserOS 授权、IDEA 密码能否读取。任何 Linux 默认路径只是候选，不是发现结果。不能把 BrowserOS 的“本地工作区读文件权限”扩大成遍历整个 HOME 的默认权限。

## 2. 两张用户截图的证据与实际缺陷

### 2.1 数据库截图

- BrowserOS 数据库界面中，IDEA 导入栏是文本路径输入 + “预览此项目的数据源” + XML 手选；没有自动展示已发现的 IDEA 项目/数据源。
- 预填路径来自 `src/capabilities/database/ui/DatabasePanel.vue` `ideaSuggestedBase`：从某条 `/home/<user>` 路径拼接 `IdeaProjects`，**并未调用文件存在性检查或扫描近期 IDEA 项目**。Screenshot 所示 Ubuntu 项目未必位于 `~/IdeaProjects`。
- `scanIdeaProject()` 仅读用户指定目录或其最多六级祖先的 `.idea/dataSources.xml`；若填的是 `~/IdeaProjects` 上级目录，不会枚举其下项目。**预填目录本身还被按钮禁用**，与“自动发现”相反。
- `readAuthorizedWorkspaceTextFile` 仅是 `bridge.readFile` 封装，要求路径在 native allowed roots 中，不代表已获全部 IDEA 项目访问权。
- `chooseIdeaSource` 只调用 `fillIdeaForm`（填表、默认只读、清空密码），不调用 `db.connect`、批量保存或批量验证；因此不是“一键导入”。
- `parseIdeaDataSourcesXml` 支持部分 MySQL/SQLite 元数据，对 PostgreSQL、隧道、客户端证书、特殊 JDBC URL 标为 unsupported；这是**刻意安全限制**，不能误标“已导入”。手选 XML 目前 `previewIdeaDataSources` 只展示 MySQL。

### 2.2 Git 截图

- Git Studio 的本地/远程分支与提交列表已显示，说明基础 Git refs → log 链路工作。
- `src/capabilities/git/ui/GitStudio.vue`：`selectCommit(oid)` 调用 `read({action:'commit_diff',revision:oid})`，将**整条提交文本补丁**填进 `rawDetail`；模板只有提交列表下方 `<details><pre class="patch">...`，没有**提交修改文件列表、逐文件选择、右侧 Diff Preview、并排文件差异**。
- Native `src-tauri/src/capabilities/git/studio.rs` `read` 的 `commit_diff` 仅是 `git show --format=fuller ... --`；没有 `commit_files` / `commit_file_diff` / `compare_file_with_worktree` 对应的用户可视化入口。
- 工作区未提交变更页已有 split/patch/全文三版本，但**该 Diff 逻辑没有复用于历史提交选中态**；与“Git 有并排 Diff”等价的说法不成立。
- 已有 Native 比较两个分支的 diff，但目前用长文本 Patch 展示，缺少比较结果的文件级导航。截图中历史滚动过长，也缺少 IDEA 右侧详情与区块结构。

## 3. IDEA 数据源对标矩阵

| # | IDEA 能力（官方） | BrowserOS 当前实现 | 状态 | 必需的闭环 |
|---|---|---|---|---|
| DB01 | 项目级 `<project>/.idea/dataSources.xml` | 手填目录、限定祖先只读 | ◐ | 发现**真实存在**的项目文件；支持点击预览 |
| DB02 | 全局数据源（跨项目） | 不读取全局配置或 ZIP | ❌ | 基于用户明确选择的导出 ZIP/获授权只读元数据导入；不能默默读取 Keyring |
| DB03 | 连接管理和数据源列表 | 连接表单/已保存连接 | ✅ | 与导入列表共享身份键与去重逻辑 |
| DB04 | 导入/共享项目 XML | 手动单 XML、解析部分 JDBC | ◐ | 批量预览、类型兼容性、错误原因、来源路径 |
| DB05 | Ubuntu 不同目录存放的 IDEA 项目 | `/home/<user>/IdeaProjects` 猜测 | ❌ | 当前文件树、已登记仓库、用户显式授权的项目路径和标准候选**受限枚举**；不得仅硬编码 IdeaProjects |
| DB06 | JDBC 多驱动/SSH/TLS | 仅 MySQL/SQLite 部分元数据 | ◐ | MySQL/SQLite 可导；其他类型逐条说明“不支持”，不得静默省略或假装成功 |
| DB07 | 连接凭据 | BrowserOS 自身 OS Keyring | ◐ | 不读取/继承 IDEA 密码；可单独输入并入 BrowserOS Keyring；需要 SSH/证书时明确阻断与手工配置 |
| DB08 | 连接 Test Connection | BrowserOS `db.connect()` | ✅ | 导入后每条显示“仅元数据/待验证/连接成功/失败”，不把预览当连接成功 |
| DB09 | 导入多条连接 | 无批量导入通路 | ❌ | 选择多条→一次点击导入配置→逐条成功/失败、保留失败记录、可重试；只读默认 |
| DB10 | 不修改 IDEA 原始文件 | 解析元数据、不写原配置 | ✅ | 哈希或时间戳取证，禁止编辑/删除 IDEA 文件；错误日志无密码/完整 JDBC URL |

**产品应实现的一键语义**：点击“发现本机 IDEA 数据源”→在已获授权的范围内有限、可取消、可解释地找到项目 XML 和经用户指定的全局导出→展示候选及未支持条目→选择后“一键导入**连接配置**”（允许缺少密码）→在 BrowserOS 列表显示结果→可逐条 Test Connection。不要用“自动复制 IDEA 运行时会话/凭据”替代上述目标。

**安全实施建议**：后台不做无授权全盘扫；显式按钮触发，扫描路径白名单来源可见；目录深度/数量/文件大小/累计耗时上限；拒绝符号链接逃逸、DTD/XXE、URL 凭据、目录穿越；逐条报告跳过原因。全局数据源的实际文件格式应通过受控样本证实，不能猜测某个固定 `~/.config/JetBrains/...` 内部 XML 名称后当作普适路径。

## 4. IDEA Git 对标矩阵

| # | IDEA 能力 | BrowserOS 当前实现 | 状态 | 缺口 |
|---|---|---|---|---|
| G01 | 左侧本地/远程分支树 | `for-each-ref refs/heads refs/remotes` | ✅ | 更完整排序、搜索、上游 ahead/behind |
| G02 | 中部提交图谱、作者时间/分支标签 | 简易 DAG + summary/author | ◐ | 显示时间、作者可配置列、commit hash、ref 装饰 |
| G03 | 右侧选中提交的 **Changed Files** 列表 | 无 | ❌ | Native 按 SHA 返回变更路径/类型/增删，树/平铺过滤 |
| G04 | 右侧 **Commit Details** | 整块 `git show` 文本 | ◐ | 单独 hash、author、date、parents、commit message |
| G05 | 选中变更文件 → **Diff Preview** | 仅整个提交 Patch；未绑定单文件 | ❌ | 按 `commit_sha + file_path` 的只读 Diff |
| G06 | 历史文件 **Side by side / Unified diff** | 仅当前工作区的 split | ❌ | 历史文件双版本只读/统一切换、行号、高亮、上下块跳转 |
| G07 | **Compare with Local** | 当前历史页不可用 | ❌ | 历史修订 vs 工作树，明确未保存状态 |
| G08 | **Compare branches** | Native 有 `compare`，全文补丁 | ◐ | 提交列表+变更文件树+逐文件差异 |
| G09 | **Compare two commits** | 当前未实现双提交 UI | ❌ | 两 SHA 固定、独立显示文件列表+Diff |
| G10 | File History / follow rename / Blame | 有基础后端与入口 | ◐ | 历史文件版本导航和逐文件预览与主 Log 贯通 |
| G11 | 键盘（Ctrl+D, Enter, 上下选项） | 未按 IDEA 工作流一致实现 | ❌ | 聚焦管理、键盘覆盖和可达性验收 |
| G12 | 批量多选提交、文件级操作 | 部分 native 写入确认门禁 | ◐ | 比较为只读；回滚等危险动作必须独立确认 |
| G13 | 大仓库响应性 | 每页 50 log，长 Patch 统一文本 | ◐ | 分页/虚拟列表、单文件按需加载、缓存淘汰、请求失效保护 |
| G14 | Merge commit、root commit、rename、binary、submodule | 未见完整可见证据 | ⚠ | 明确首次父提交 diff、root、重命名、二进制与大文件状态 |

**交互设计（应与 IDEA 操作链一致）：**

```text
Git Studio
┌───────分支（可折叠）───────┬──────────── Commit Log ────────────┬──── Changed Files ────┐
│ 本地 / 远程 / 搜索 / Fetch │ DAG + 标题 + 作者 + 时间 + hash      │ 文件状态/路径/统计      │
│ 选择分支 → 过滤历史        │ 点击 commit → 固定选择态            ├──── Commit Details ──┤
│                           │                                      │ 作者/时间/父提交/说明   │
└───────────────────────────┴──────────────────────────────────────┴──────────────────────┘
选中修改文件 / Ctrl+D → 文件级 Diff Preview（并排/统一）
历史旧版本 vs 新版本 | 历史版本 vs 本地工作树 | 上/下一个文件、上/下一差异
```

只读 Git 操作不能自动 `fetch`（避免网络耗时、认证与无界流量）；Fetch、checkout、merge、rebase、discard、reset 等仍遵守本仓库的目标明确/预览/指纹/二次确认安全模型。GitHub PR 视图不能替代**本地 Git**历史。

## 5. 导致上次“测试通过但体验缺失”的门禁空洞

1. `scripts/check-git-ui-logic.mjs` 主测 `useGitStore` status/branch/diff 及导航意图，未执行“点历史提交→获得文件列表→选择文件→并排差异”的端到端路径。
2. `scripts/check-developer-studio.mjs` 验证 Git Patch/SQL 部分纯逻辑，不等于 GUI Git Log 全流程。
3. `src/capabilities/database/model/jetbrainsImport.ts` 与 `utils/ideaDataSources.ts` 具备 XML 解析，但单项目/单连接的静态单测没有发现流程、获权提示、批量持久化等体验验收。
4. 全部 CI PASS 说明**现有测试被满足**，而不是**用户要的 IDEA 行为存在**。今后必须将以下黑盒 E2E 作为发布门禁，不得用源码字符串/假列表替代。

## 6. 开发一次收口的自动验收矩阵（避免逐条询问用户）

数据库 E2E（必须具备可运行测试夹具）：

- [ ] D1 标准 `~/IdeaProjects/project/.idea/dataSources.xml` 自动发现、导入
- [ ] D2 项目放在 `~/Documents/Projects/foo`、空格与非 ASCII 路径也能从获授权目录发现
- [ ] D3 同一项目 2+ 连接、不同项目同名连接：去重正确，来源可追踪
- [ ] D4 没有项目/没有 XML、配置损坏、无权限、目录包含符号链接：能理解的反馈，不挂起
- [ ] D5 全局级配置仅经用户显式授权的受支持来源识别，不能假报成功
- [ ] D6 MySQL、SQLite、PostgreSQL、SSH/TLS/证书/URL-only 条目明确区分支持/阻断；不丢条目
- [ ] D7 密码不出现在快照、日志、仓库、网络请求、HTML 报告；不读取 IDEA KeePass/系统钥匙串
- [ ] D8 批量 3 条配置中 2 成功 1 失败，明确结果、失败可重试，禁止重复复制
- [ ] D9 导入后列表真实可见、连接测试成功才显示连接成功；失败不得误报
- [ ] D10 Ubuntu 安装版真实工作流、窗口缩放、键盘/屏幕阅读器、权限弹窗取证

Git E2E（必须使用真实临时 Git 仓库、至少两个 remotes/refs 的固定夹具）：

- [ ] G1 local + origin remote + HEAD 的正确分组与筛选，点击只显示该 ref 的提交
- [ ] G2 选中普通提交显示完整 变更文件列表 + 详情，不只是整个 Patch
- [ ] G3 点单文件：逐文件 diff 准确、带行号；Ctrl+D 与并排/统一切换
- [ ] G4 选中文件比较其历史版本与当前 worktree；未提交本地改动有清晰提示
- [ ] G5 root、merge、rename、delete、binary、submodule 及长路径不崩溃
- [ ] G6 多提交对比、分支对比显示差异文件列表，禁止操作对象随筛选漂移
- [ ] G7 大仓库 1000+ 提交分页，快速切换分支/仓库时过期异步请求不能污染新视图
- [ ] G8 全程只读不写工作树/索引，危险写入二次确认，失败报错脱敏
- [ ] G9 Ubuntu Tauri 安装版完成“分支→提交→文件→并排对比→与本地对比”的录屏或截图验收
- [ ] G10 GUI 与 Native 验收同 SHA，真实 `pre-merge`、性能体积/语义门禁 PASS

## 7. 交付门槛、证据和完成定义

- **Research complete ≠ Development complete ≠ E2E complete ≠ Human acceptance complete**，必须独立记录。
- 一个统一产品需求、一个实施 PR（可以内部多提交）；不得在一个 UI 假入口完成后就标绿。每个编号必须关联：真实代码路径、自动化测试、GUI 验收证据、对应 SHA。
- 先把这份审计作为独立文档提交；随后实施时补上端到端自动验证，不再要求用户逐项截图提 Bug。
- 本轮**仍不能证明**本机真实的 JetBrains 项目地址和全局配置；在本地原生授权扫描、最少化读取及用户明确批准之前，不使用推断路径声明“检测成功”。
- 未来的结果页要直观展示：来源、当前进度、错误、跳过原因、导入成功数、待验证数、最后一次 E2E 证据。IDEA 数据源配置**可导入**不等于凭据**可复用**。

## 8. 根因裁决

- **调研缺口**：之前仅关注项目级 `.idea/dataSources.xml` 和 Git 原始 diff，没有严格对齐 IDEA 的项目/全局双域、Branch/Commits/Changed Files/Commit Details/Diff Preview 的完整交互。
- **开发缺口**：硬编码猜测项目基目录、没有自动发现与批量落库；Git 提交没有 Changed Files 与 File Diff 预览；界面将 commit Patch 当成完整历史审查。
- **验收缺口**：测试覆盖实现细节而非用户任务闭环；多次“CI PASS”被错误延伸成产品功能完整。这次以 D1–D10 和 G1–G10 修正。
