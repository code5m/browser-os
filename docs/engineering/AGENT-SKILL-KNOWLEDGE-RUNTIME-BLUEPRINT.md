# BrowserOS Agent / Skill / Notes / Graph Runtime Blueprint

状态：DESIGNED；本文件不是已完成声明。基线：PR #28 统一蓝图页，仍待 CI / 合并。

## 一、产品原则

BrowserOS 以浏览器 + 文件管理器为核心。笔记以本机 Markdown 文件为内容真源，
索引是可重建的投影，知识图谱是统一索引的视图，Agent/Skill 是有权限限制的消费者。
不要复制一套 Obsidian 数据库；无需给笔记安装特殊格式才能被 Obsidian 阅读。

## 二、当前代码审查（2026-10-12）

- skill: domain.SkillDef / SkillExec(script_ref, command_ref, sequence) 已定义；skill_parse / validate / permission_preview 已实现。
- agent: AgentDef / agent_parse / validate / permission_preview 已实现；agent_chat、agent_chat_cancel、agent_runs_list 是前端占位，原生命令缺失。
- script_runner 已实现脚本/命令启动、超时、进程组取消、有界输出和终态落盘；Skill 应调用该内核，禁止出现第二路径。
- graph: graph_import_json / graph_rebuild_documents / graph_query 存在；重建只读取允许根目录的文件和目录元数据，不解析 Markdown 内容，尚无 Wikilink 双链。
- notes: workspace.notes_dir 使用本地 Markdown 文件；vault_open 提供有限目录读取。
- 本机 Codex CLI 检测存在，但最小对话测试遇到 TLS websocket eof；不得宣称 Provider 已经就绪。
- PR #28 仍是 open；不应把蓝图页当作 AI/Skill 已执行的证据。

## 三、统一知识模型

Markdown 为内容真源，每篇笔记存稳定 note_id、相对路径、标题、可选 YAML frontmatter、
mtime/hash。双链中支持 [[笔记名]]、[[笔记名|显示名]]、[[笔记名#章节]]；
普通 Markdown 相对链接也可解析。显式未解析链接单独展示，绝不静默匹配错误笔记。
正向链接和反向链接由同一关系索引推导，引用只存 note_id + 来源位置，不复制正文。
GraphStore 的 File/Dir 仍保留，增加笔记到笔记的 references 边；
不把正文、口令或绝对路径放到 props；关系随增量索引/重建保持确定性。
重命名笔记：先改写定位与索引，再提示可选修复入链，不强制批量改写用户原文件。
目录与 vault 先复用现有 notes_dir 和受控根；外部 Obsidian vault 必须明确授权导入/只读挂载。

## 四、技能最小真实闭环

1. 用户导入 SkillDef JSON，原生 validate + SkillExec.validate；显式告知引用脚本或命令是否存在且已启用。
2. 安装：生成版本化记录并持久化到受控 app data；保存 definition hash，不保存 secret、命令输出。
3. 授权：读取 ACL safe/confirm/dangerous；危险操作二次确认并与定义 hash/参数摘要绑定，过期失效。
4. 运行：ScriptRef 调现有 start_run；CommandRef 调 start_command。Sequence 按顺序串行且首个失败停止，任何一步取消/超时终止后续步骤；禁止 inline shell。
5. 状态：run_id、每步状态、输出有界、取消/超时、审计与恢复；UI 只消费真实 Rust 状态。
6. 不能直接把任意 JSON exec 的字符串当 shell 命令；用户导入定义不能直接变成主机执行权限。
7. 测试：有效/无效定义、脚本缺失、权限拒绝、失败中断、取消进程组、重复点击、重启后历史、安全凭据不落盘。

## 五、智能体最小真实对话

1. Provider 管理：可选外部 CLI 或兼容 API；凭据仅 OS keyring，不出现在会话文件、URL、日志。
2. 模型/Provider 先做健康检查；无可用 Provider 时展示诊断和修复路径，不生成假回答。
3. 会话：创建 session_id，消息使用 role、time、content、status、provider、model、source_note_ids；
输入校验、单会话有界持久化、版本兼容、取消和错误可见。
4. 执行器从独立 Provider adapter 进入，有独立于聊天内容的权限策略；知识引用视为不可信数据，
严禁来自笔记/网页的提示自动提升文件系统、网络或 shell 权限。
5. 初期只读问答，不提供未经批准的自动调用技能；工具调用必须有可验证授权。
6. 先实测真实 CLI/API 的短回合、网络失败、流结束、停止、重启历史、并行会话，再标记 Ready。

## 六、笔记 UI 与图谱 UI

- 笔记：三栏（文件树 / Markdown 编辑预览 / 大纲与反向链接）；支持新建、保存、搜索、标签、链接补全、
  Ctrl+点击跳转、未解析引用、历史/冲突提示。
- 图谱：从受控关系索引投影局部图和全局图，筛选节点类型、路径、标签，点击节点打开笔记原文件；
  边明确区分目录包含、笔记引用、Agent/Skill 使用知识，不伪装关系推断。
- 对外明确区分“笔记内容已保存”、“索引已同步”和“图谱已渲染”，避免状态混淆。

## 七、实现顺序（合并与验收时仍须全量检查）

A. 先让 PR #28 完成 CI/合并，保持正式 master 干净，已有 MySQL WIP stash 不混入本次。
B. 建 note-link parser 纯函数 + fixtures：嵌套/转义/代码块/重名/孤链/重命名/循环链接；
再经原有 graph 命令重建索引（不允许引第二个 GraphStore）。
C. 笔记编辑与反向链接 UI，基于现有文件 I/O 权限、原子写和冲突检测。
D. 技能持久化与单步骤运行，随后 sequence/取消/审计；完整安全回归通过才开放运行按钮。
E. Provider 设置、凭据/授权、真实对话与历史；无真实 provider 成功证据不得宣布完成。
F. Markdown 知识检索接入 Agent：每段引用有文件来源，权限先行。
G. npm run check / npm run build / Rust check+tests / pre-merge / CI / GUI / 安装版冷启动；
绝不降低门禁、不绕过失败测试。仅成功完成的阶段可标 READY。

## 八、不可破坏的约束

保持现有 WebView/GTK 运行时、浏览器和文件管理器稳定；所有新 IPC 原子更新 Rust handler、
Tauri ACL、bridge.ts、TS 类型、命令注册与契约测试；保留末条 ACL K1；
遵循 AGENTS.md/PROJECT-RULES.md 停止条件；避免跨 capability 内部 import；
执行内核复用 script_runner，知识索引复用唯一 GraphStore，用户笔记 Markdown 是内容真源。

## 九、2026-10-12 蓝图复审与扩展

- 当前真实状态：Vault Markdown 编辑与保存冲突测试已落地；现有 Vault 包已经有局部双链、反向链接和局部图谱；全局 GraphStore 仍只有文件/目录索引，不能宣称完成知识图谱整合。
- 本轮新增 `src-tauri/src/capabilities/graph/markdown_links.rs` 纯解析器，支持 Wiki/Markdown 相对链接、代码块排除、歧义解析、路径逃逸拒绝和有界收集；两条 Rust 单元测试已通过。该文件尚未接入全局 GraphStore，因此仅能称为解析能力完成。
- 全局知识索引应当复用现有 GraphStore 和原子 graph.json，提供显式全量导入与变更单笔增量，保留边的来源、未解析引用和版本；不能创建第二套索引或将正文与凭据放入 graph props。
- 写入笔记成功后，图谱同步必须显式返回成功/失败；不能将“笔记已保存”冒充“索引已同步”。若同步失败则保留脏标志，可重试、可重建。
- 模型 Provider：本地未检测到运行中的 Ollama；Codex CLI 先前真实连接报 TLS/websocket 失败。先做配置、健康检查、真实错误、失败重试及会话状态机，再接文本问答；不可用时界面必须告知没有连接，不得假造回答。
- 强化安全：笔记/网页是未经信任的输入，任何调用技能、文件写入、命令执行的授权必须来自用户实际确认，不可由模型输出决定。
- 新增验收：零真实 Provider 时无假回答；在线真实 Provider 一问一答；聊天历史恢复和取消；增量笔记修改、外部编辑、重命名、删除、同名歧义与全量重建一致性；图谱存量关系不得被误清；运行时资源预算；安装版 GUI。
- 仍待：模型 Provider 原生协议、消息持久化、全局 GraphStore 引用边、增量同步联动、端到端 GUI 以及所有仓库门禁通过。不得标记 ALL_PASS。

## 十、2026-10-12 本地实验实施复审（未发布）

- Vault 索引：新增 vault_index.rs：Markdown 是唯一正文来源，索引写入原有 GraphStore / graph.json；节点通过哈希关联笔记，References 有界、歧义链接不建立边。全量同步只替换本 Vault 投影，单篇保存增量仅更新该笔记出边。节点元数据存受控相对路径与根标记，不存正文或凭据。
- 真实对话：新增仅 localhost:11434 的 Ollama 非流式 generate 适配器；不接任意 URL，不提供工具调用/文件读取/命令执行；响应、输入、socket 超时有限制；无本地 Ollama 或模型时真实报错，不提供假回复。
- 生命周期欠账：本机没有检测到 Ollama 服务，本轮只能完成编译和离线拒绝测试，不能标注 REAL_PROVIDER_E2E_PASS；尚需真实模型下载、联网异常、重启后历史、取消和用户可解释的模型管理 UI。当前页面仅单轮对话，消息不持久化。
- 增量索引边界：当前保存后同步依赖原生命令。若保存成功而索引失败，应保留可见“笔记已保存、图谱未同步”状态。全量同步需显式打开 Vault，外部编辑暂时不能自动感知，后续应加 hash/mtime 差异检查和可靠的增量调度，禁止资源密集型后台全盘扫描。
- 质量债务：本轮尚未通过全部 npm check / pre-merge / GUI 安装版，不能合并 master。PR #28 最近 Full Validation 实际 failed，具体因为旧 build metrics baseline，必须修复真实体积问题，而不是降低门禁。
- 新增规划建议：提供 Provider 连接管理页面（本地 Ollama 状态、模型清单、运行成本、隐私说明）、知识引用来源卡、图谱脏状态恢复按钮、跨应用 Markdown 文件更改冲突解决、Agent/Skill 明确运行证据及撤销能力。下一次开发应同时验证性能与界面易用性。
