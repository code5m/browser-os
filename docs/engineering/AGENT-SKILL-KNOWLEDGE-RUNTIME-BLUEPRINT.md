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
