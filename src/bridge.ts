import { invoke, Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Artifact,
  ImageRef,
  ScriptMeta,
  ScriptParam,
  ScriptInterpreter,
  CommandSnippet,
  RunSnapshot,
  ScriptRunRecord,
  ScriptOutputEvent,
  ScriptFinishedEvent,
  RepoConfig,
  SyncPreview,
  SyncJob,
  AuditEntry,
  WorkspaceTree,
  Bookmark,
  BrowserCredentialItem,
  AutofillResult,
  DirEntry,
  BrowserResources,
  TabInfo,
  TabRecoveryEvent,
  AppEntry,
  ResourceStats,
  GitFileStatus,
  GitDiffResult,
  GitBranch,
  GitWriteOp,
  GitWriteJob,
  GitWritePreview,
  ResourceReceived,
  ResourceCaptureSettings,
  TabResourceList,
  BrowserSession,
  SessionSummary,
  SessionPolicy,
  SessionFlushReport,
  ToolMeta,
  TermMessage,
  TaskAddArgs,
  TaskDef,
  TaskRunRecord,
  DbConnectionConfig,
  DbConnectResult,
  DbQueryResult,
  // M5-6 Agent/Skill 类型（仅方法返回类型用到）
  AgentDef,
  AgentRunRecord,
  SkillDef,
  SkillRunRecord,
  GraphEdge,
  GraphNode,
  ValidationReport,
  // M5-W12 图谱 live-query View DTO（与 A7 §7.5 镜像对齐；删 props，K7 双闸）
  GraphNodeView,
  GraphEdgeView,
  GraphStats,
  GraphQueryRequest,
  GraphQueryResult,
  // M5-W13 插件 manifest 生命周期 Stage-I DTO（与后端 domain.rs 镜像对齐）
  PluginManifest,
  PluginState,
  PluginSummary,
  PluginDetail,
  TrustedKeyRecord,
} from "./types";

/// M5-W12 图谱 live-query 守卫常量：与 useGraphStore 复用。
/// 当 `false` 时 `bridge.graphQuery / graphNodeGet / graphStats` 三个方法直接 reject 一个
/// 稳定码错误（code=`GRAPH_UNKNOWN_ERROR` + 本地 message），**绝不 invoke**；
/// 当 A0 集成完成 A7 §3 命令落地后翻 `true` 即可解锁命令面。集中可回滚。
/// （**注**：本常量集中定义在下方 L100 区域，本文件内仅此一处。`makeGraphCommandDisabledError` 共享使用。）

/** 生成"图谱命令未就绪"的稳定错误对象，供三个 W12 封装共享。 */
function makeGraphCommandDisabledError(cmd: string): Error {
  // 错误信息仅含"图谱命令面尚未落地"语义文案，**绝不**回显 cmd 之外的任何状态/请求体/URL。
  // 这是 A4 W10 §4-C "错误稳定码 + 零 secret echo" 的前端镜像（A8 W12 §3.4 1:1 表）。
  const err = new Error(`图谱命令 ${cmd} 尚未落地，请稍候刷新或联系维护者`) as Error & {
    code?: string;
  };
  err.code = "GRAPH_UNKNOWN_ERROR";
  return err;
}

// 为上方注释提供 re-export 占位（实际常量声明在下方，import 时按名解析）。
// 这里**不**做重复 export —— TS 仅一处定义（下方 L100），所有消费方统一引用。
export { };

/// M4-8 可用性开关：A7（M4-6 / M4-7）已落地后端 `task_list / task_add / task_update /
/// `task_remove / task_run_now` 五条命令（tasks.rs / scheduler.rs / bridge.rs / main.rs / ACL），
/// 故置 `true` → 面板为**实时面板**；置 `false` 时 `useTaskStore` 不发出任何 invoke（只读壳）。
/// 这是 board「LIMITED START」的落地方式：命令未就绪时**不假借未实现的命令名假装可用**。
export const TASK_COMMANDS_AVAILABLE = true;

// M5-6 可用性开关：A5（M5-4/5）已落地 Agent/Skill domain + policy（domain.rs / skills.rs /
// agent.rs / check-agent-skill-policy.py），但 **skill_*/agent_* 后端命令尚未落地**；故置
// `false` → useAgentStore 不发出任何 invoke（只读壳）。命令落地后置 true 即可解锁面板动作。
// 这是 board「LIMITED START」的落地方式：命令未就绪时**不假借未实现的命令名假装可用**。
export const AGENT_SKILL_COMMANDS_AVAILABLE = false;

// M5-W7（A5）只读桥命令（agent_parse / agent_validate / agent_permission_preview + skill_* 同款）已落地，故置 true。
// 注意：这是「只读解析/校验/预览」，不是上述 M5-5 运行时命令。
export const AGENT_SKILL_READONLY_COMMANDS_AVAILABLE = true;

// M5-W12 图谱 live-query 守卫常量（W12 A7 已落地 graph_query/graph_node_get/graph_stats 三只读命令，置 true 解锁；回滚路径见 makeGraphCommandDisabledError）。
export const GRAPH_COMMANDS_AVAILABLE = true;


// M0-0.b 测量配置（契约 logs/m0-baseline-contract-v1.md；非测量运行后端返回 null）
export interface M0Config {
  run_id: string;
  driver: string;
  ready_file: string | null;
  report_dir: string | null;
}

// 类型化 IPC 封装：前端永远只传“意图”，不直接碰 OS / 凭据
export const bridge = {
  openBrowser: (url: string) => invoke("open_browser", { url }),

  closeBrowser: () => invoke("close_browser"),

  positionBrowser: (p: { x: number; y: number; width: number; height: number }) =>
    invoke("position_browser", p),

  collectSelection: (p: {
    url: string;
    title: string;
    html: string;
    text: string;
  }) => invoke<Artifact>("collect_selection", p),

  // 网页右键"选区存为 Markdown 笔记"：默认笔记目录，文件名=时间戳+选中内容
  saveNote: (p: { url: string; title: string; text: string }) =>
    invoke<string>("save_note", p),

  // 网页右键"打开终端"：发意图给后端，后端广播事件，主窗前端切到终端视图
  requestOpenTerminal: () => invoke("request_open_terminal"),

  // 订阅"打开终端"请求（来自子 webview 右键菜单）
  onOpenTerminal: (cb: () => void) => listen("open-terminal", () => cb()),

  // 订阅笔记保存成功事件（payload 为完整文件路径）
  onNoteSaved: (cb: (path: string) => void) =>
    listen<string>("note-saved", (e) => cb(e.payload)),

  listArtifacts: () => invoke<Artifact[]>("list_artifacts"),
  // M2-7/M2-8 工具：list_tools 只读枚举；open_tool 在独立子 webview 打开
  // （协议加载 + 零能力隔离在后端 tools.rs / main.rs）
  listTools: () => invoke<ToolMeta[]>("list_tools"),
  openTool: (id: string) => invoke<void>("open_tool", { id }),
  closeTools: () => invoke<void>("close_tools"),

  // token 仅在此调用中传给后端，存入系统密钥库；不会被前端持久化/回显
  configureRepo: (p: { config: RepoConfig; token: string }) =>
    invoke("configure_repo", p),

  listRepos: () => invoke<RepoConfig[]>("list_repos"),

  // ====== M1-5 Git 只读能力（status / diff / branch_list） ======
  // 只读取仓库状态：不可写、不联网、不回传凭据；diff 超限时由后端截断。
  gitStatus: (p: { repoId: string }) =>
    invoke<GitFileStatus[]>("git_status", p),

  gitDiff: (p: { repoId: string; path?: string; maxBytes?: number }) =>
    invoke<GitDiffResult>("git_diff", p),

  gitBranchList: (p: { repoId: string }) =>
    invoke<GitBranch[]>("git_branch_list", p),

  // ====== M1-6.b Git 写能力（双阶段确认闸门；本期无 UI，UI 归 M1-7） ======
  // 所有写操作都必须先 request（生成预览 + 待确认任务，不执行任何写），
  // 再 confirm 才真正执行；discard 等 dangerous 操作 confirm 时必须带
  // confirmedDangerous=true 二次确认。任务一次性、5 分钟过期。

  // 阶段一：生成待确认 GitWriteJob 与预览
  requestGitWrite: (p: {
    repoId: string;
    op: GitWriteOp;
    paths?: string[];
    message?: string;
    branch?: string;
    checkout?: boolean;
  }) => invoke<GitWritePreview>("request_git_write", p),

  // 阶段二：确认执行（后台线程执行，立即返回 Running 任务）
  confirmGitWrite: (p: { jobId: string; confirmedDangerous?: boolean }) =>
    invoke<GitWriteJob>("confirm_git_write", p),

  // 订阅后台写任务完成事件（成功/失败都会触发，payload 为最终 GitWriteJob）
  onGitWriteCompleted: (cb: (job: GitWriteJob) => void) =>
    listen<GitWriteJob>("git-write-completed", (e) => cb(e.payload)),

  // 六个白名单操作的便捷封装（只生成待确认任务，仍需 confirmGitWrite 才执行）
  gitStage: (repoId: string, paths: string[]) =>
    invoke<GitWritePreview>("request_git_write", { repoId, op: "stage", paths }),

  gitUnstage: (repoId: string, paths: string[]) =>
    invoke<GitWritePreview>("request_git_write", { repoId, op: "unstage", paths }),

  gitDiscard: (repoId: string, paths: string[]) =>
    invoke<GitWritePreview>("request_git_write", { repoId, op: "discard", paths }),

  gitCommit: (repoId: string, message: string, paths?: string[]) =>
    invoke<GitWritePreview>("request_git_write", {
      repoId,
      op: "commit",
      message,
      paths,
    }),

  gitCreateBranch: (repoId: string, name: string, checkout?: boolean) =>
    invoke<GitWritePreview>("request_git_write", {
      repoId,
      op: "create_branch",
      branch: name,
      checkout,
    }),

  gitCheckoutBranch: (repoId: string, name: string) =>
    invoke<GitWritePreview>("request_git_write", {
      repoId,
      op: "checkout_branch",
      branch: name,
    }),

  // push 是 dangerous 操作（影响远端）：confirm 时必须带
  // confirmedDangerous=true 二次确认；仅非 force 推当前分支到 origin 同名分支
  gitPush: (repoId: string) =>
    invoke<GitWritePreview>("request_git_write", { repoId, op: "push" }),

  // 第一步：生成“待确认”SyncJob（不真正推送）
  requestSync: (p: { artifactIds: string[]; repoId: string }) =>
    invoke<SyncPreview>("request_sync", p),

  // 第二步：用户确认后才真正推送（闸门）。后台线程执行，立即返回 Running 任务
  confirmSync: (p: { jobId: string }) => invoke<SyncJob>("confirm_sync", p),

  // 订阅后台推送完成事件（成功/失败都会触发，payload 为最终 SyncJob）
  onSyncCompleted: (cb: (job: SyncJob) => void) =>
    listen<SyncJob>("sync-completed", (e) => cb(e.payload)),

  // 订阅网页资源上报事件（浏览器面板展示 js/css/svg 等）
  onBrowserResources: (cb: (r: BrowserResources) => void) =>
    listen<BrowserResources>("browser-resources", (e) => cb(e.payload)),

  // Trusted browser-tabs plugin GTK shortcut; no remote page invokes Tauri IPC.
  onChildShellShortcut: (cb: (event: { id: string; action: string }) => void) =>
    listen<{ id: string; action: string }>("browser-tabs://shell-shortcut", (e) => cb(e.payload)),

  // 订阅成果保存成功事件（浏览器子 webview 中保存后自动刷新列表）
  onArtifactCollected: (cb: () => void) =>
    listen("artifact-collected", () => cb()),

  auditLog: () => invoke<AuditEntry[]>("audit_log"),

  // 产出端：读取/编辑/删除/目录树
  readArtifact: (id: string) => invoke<Artifact>("read_artifact", { id }),

  updateArtifact: (p: { id: string; title: string; text: string; tags: string[] }) =>
    invoke<Artifact>("update_artifact", p),

  deleteArtifact: (id: string) => invoke("delete_artifact", { id }),

  browseWorkspace: () => invoke<WorkspaceTree>("browse_workspace"),

  // M2-1 图片领域与持久化（画廊/灯箱/缩放属 M2-2，本卡不实现）
  saveImage: (p: {
    artifactId: string;
    data: number[];
    mime: string;
    sourceUrl?: string | null;
    caption?: string | null;
  }) => invoke<ImageRef>("save_image", p),

  listArtifactImages: (artifactId: string) =>
    invoke<ImageRef[]>("list_artifact_images", { artifactId }),

  // M2-2.b 预览通道：只回目录基准（不含任何图片相对路径），
  // 前端用它 + ImageRef.rel_path 拼绝对路径后交给 convertFileSrc 转 asset://。
  workspaceImagesDir: () => invoke<string>("workspace_images_dir"),

  // M2-3 脚本领域与持久化（**无执行能力**，run_script 归 M2-4）
  scriptList: () => invoke<ScriptMeta[]>("script_list"),

  scriptAdd: (p: {
    name: string;
    category: string;
    interpreter: ScriptInterpreter;
    body: string;
    params: ScriptParam[];
    description?: string | null;
    timeoutSecs?: number | null;
  }) => invoke<ScriptMeta>("script_add", p),

  scriptUpdate: (p: {
    id: string;
    name: string;
    category: string;
    description?: string | null;
    params: ScriptParam[];
    timeoutSecs?: number | null;
    enabled?: boolean | null;
    body?: string | null;
  }) => invoke<ScriptMeta>("script_update", p),

  scriptRemove: (id: string) => invoke("script_remove", { id }),

  // M2-4.c 脚本执行命令层：输出流/运行记录落盘归 M2-4.d
  runScript: (id: string, values: Record<string, string>) =>
    invoke<RunSnapshot>("run_script", { id, values }),

  cancelScript: (runId: string) => invoke("cancel_script", { runId }),

  scriptStatus: (runId: string) =>
    invoke<RunSnapshot>("script_status", { runId }),

  // M2-5.c 运行历史：读 script-runs.json 全量或按 script_id 过滤
  scriptRunsList: (scriptId?: string) =>
    invoke<ScriptRunRecord[]>("script_runs_list", { scriptId: scriptId ?? null }),

  // M2-6.b 命令片段持久化与 CRUD（不接执行；run_command 归 M2-6.c）
  snippetList: () => invoke<CommandSnippet[]>("snippet_list"),

  snippetAdd: (p: {
    name: string;
    category: string;
    interpreter: ScriptInterpreter;
    argv: string[];
    params: ScriptParam[];
    description?: string | null;
    dangerous?: boolean | null;
    // M2-6-fix1（复核 F-1）：新建态同样传 enabled，否则用户在新建表单取消
    // 「启用」勾选会被后端硬编码的 true 静默覆盖。
    enabled?: boolean | null;
    timeoutSecs?: number | null;
  }) => invoke<CommandSnippet>("snippet_add", p),

  snippetUpdate: (p: {
    id: string;
    name: string;
    category: string;
    interpreter: ScriptInterpreter;
    argv: string[];
    params: ScriptParam[];
    description?: string | null;
    dangerous?: boolean | null;
    enabled?: boolean | null;
    timeoutSecs?: number | null;
  }) => invoke<CommandSnippet>("snippet_update", p),

  snippetRemove: (id: string) => invoke("snippet_remove", { id }),

  // M2-6.c 命令片段执行：复用脚本执行态事件和运行历史。
  runCommand: (id: string, values: Record<string, string>) =>
    invoke<RunSnapshot>("run_command", { id, values }),

  // ====== M4-8 定时任务（命令名沿用 A1 冻结的 5 条，A6 契约 §6 定义 DTO）======
  // 后端实现归 A7（M4-6.b）；前端只在 TASK_COMMANDS_AVAILABLE 为 true 时调用。
  taskList: () => invoke<TaskDef[]>("task_list"),

  // M4-8 `task_add`：后端是**平铺命名参数**（非 TaskDef 结构体），且 Tauri 默认把参数名
  // 从 snake_case 转成 camelCase，故多词键必须是 camelCase（targetId / catchUpLimit …）。
  // 载荷由 `utils/taskUi.ts::serializeTaskAddArgs` 生成，整包传出、不做字段解构。
  taskAdd: (p: TaskAddArgs) => invoke<TaskDef>("task_add", p),

  // M4-8 `task_update`：后端参数为单个 `task: TaskDef` 结构体（serde snake_case），
  // 必须包成 `{ task: p }`；多词键随 TaskDef 的 snake_case 落在此结构体内。
  taskUpdate: (p: TaskDef) => invoke<TaskDef>("task_update", { task: p }),

  taskRemove: (id: string) => invoke<boolean>("task_remove", { id }),

  taskRunNow: (id: string) => invoke<RunSnapshot>("task_run_now", { id }),

  // ====== M4-4 数据库面板（命令名沿用 A1 冻结的 3 条：db_connect / db_query / db_disconnect）======
  // 后端实现归 A4（M4-2.s / M4-3）；前端封装归 A5（M4-4）。A4 检查点 A4-M4-2s-database-safety
  // 已明确「前端 wiring 归 A5」，故本文件新增 db_* 封装属 A5 授权范围。
  // 嵌套结构（DbConnectionConfig / DbQueryResult）一律 snake_case（Tauri 仅转换顶层 camelCase）。
  dbConnect: (cfg: DbConnectionConfig, password: string) =>
    invoke<DbConnectResult>("db_connect", { cfg, password: password || null }),

  dbQuery: (p: { conn_id: string; sql: string; timeout_secs?: number | null; confirm_write: boolean; query_id?: string }) =>
    invoke<DbQueryResult>("db_query", { connId: p.conn_id, sql: p.sql, timeoutSecs: p.timeout_secs ?? null, confirmWrite: p.confirm_write, queryId: p.query_id ?? null }),

  // 后端 db_disconnect 返回 Result<(), String>，Tauri 序列化为 null。
  dbDisconnect: (conn_id: string) => invoke<null>("db_disconnect", { connId: conn_id }),
  dbListConnections: () => invoke<DbConnectionConfig[]>("db_list_connections"),
  dbCancel: (queryId: string) => invoke<boolean>("db_cancel", { queryId }),
  vaultOpen: (path: string) => invoke<{ root: string; notes: { path: string; text: string }[]; skipped: number; truncated: boolean }>("vault_open", { path }),
  gridReadReplies: (index: number) => invoke<{ provider?: string; replies: string[]; error?: string; truncated?: boolean }>("grid_read_replies", { index }),
  archiveReplies: (path: string, items: { label: string; markdown: string }[], tags: string[]) => invoke<{ label: string; path?: string; error?: string }[]>("archive_replies", { path, items, tags }),
  gitLog: (repoId: string) => invoke<{ oid: string; parents: string[]; summary: string; author: string; time: number }[]>("git_log", { repoId }),
  gitCommitDiff: (repoId: string, oid: string) => invoke<string>("git_commit_diff", { repoId, oid }),

  // 说明：A6 §6 / F-A6-7 明确**没有** task_history / task_cancel 两条命令——
  // 历史由 task_list 附带返回，取消复用既有 cancel_script(run_id)。
  // 因此本文件不登记任何 A1/A6 之外的命令名。

  // ====== M5-6 Agent/Skill 面板（命令名沿用 A5 M5-4/5 冻结的 skill_*/agent_*；A6 前端封装）======
  // 后端实现归 A5（M5-4/5）；前端只在 AGENT_SKILL_COMMANDS_AVAILABLE 为 true 时调用。
  // W5 当前后端命令尚未落地（A5 W4 仅 domain + policy），故该标志为 false，
  // useAgentStore 在调用前一律拦截（零 invoke）。以下封装是「契约占位」，命令落地后组件无需改动。
  skillList: () => invoke<SkillDef[]>("skill_list"),

  agentList: () => invoke<AgentDef[]>("agent_list"),

  // ====== M5-W7（A5）Agent/Skill 只读桥：parse/validate/permission_preview ======
  // 与 W7 后端命令一一对应；仅解析/校验/预览，无执行/安装/联网/写持久化。
  agentParse: (text: string) => invoke<AgentDef>("agent_parse", { text }),
  agentValidate: (text: string) => invoke<ValidationReport>("agent_validate", { text }),
  agentPermissionPreview: (text: string) =>
    invoke<PermissionPreview>("agent_permission_preview", { text }),
  skillParse: (text: string) => invoke<SkillDef>("skill_parse", { text }),
  skillValidate: (text: string) => invoke<ValidationReport>("skill_validate", { text }),
  skillPermissionPreview: (text: string) =>
    invoke<PermissionPreview>("skill_permission_preview", { text }),

  // 安装：返回 { request_id } 时进入二段式闸门（UI 弹 PermissionPreviewModal）。
  skillInstall: (id: string) =>
    invoke<{ request_id?: string }>("skill_install", { id }),

  agentInstall: (id: string) =>
    invoke<{ request_id?: string }>("agent_install", { id }),

  // 二段式闸门确认（install_skill / install_agent 共用 confirm 语义）。
  confirmSkill: (requestId: string, decision: "approve" | "deny") =>
    invoke("confirm_skill_install", { requestId, decision }),

  confirmAgent: (requestId: string, decision: "approve" | "deny") =>
    invoke("confirm_agent_install", { requestId, decision }),

  // 运行 Skill（经后端 script_runner，不另起执行路径）。
  skillRun: (id: string, inputs: Record<string, unknown>) =>
    invoke<{ run_id: string }>("skill_run", { id, inputs }),

  // 对话（流式经 agent://<id>/stream + done/error/canceled 事件）。
  agentChat: (agentId: string, prompt: string, sessionId: string) =>
    invoke("agent_chat", { agentId, prompt, sessionId }),

  agentRunCancel: (runId: string) => invoke("agent_chat_cancel", { runId }),

  // ====== M5-W12 图谱 live-query（A7 后端只读命令 + A8 前端消费）======
  // graphQuery：现在接受入参 + AbortSignal（与 A7 §5 cancellation 对齐）。
  //   - 旧占位 `(req: GraphQueryRequest, signal?: AbortSignal) => invoke<GraphQueryResult>(...)`。
  //   - 守卫：GRAPH_COMMANDS_AVAILABLE=false 时 reject 一个稳定码错误（不 invoke）。
  //   - 返回 GraphQueryResult（删 props，K7 双闸），含 truncated/applied 信号。
  // graphNodeGet：单节点查询（按 id）；id 校验在 A7 后端走 validate_id_public（Skill/Agent 64-hex）。
  // graphStats：容量概览（计数 + 容量 + 90% 黄牌 approaching_*_capacity）。
  //   - 三者共享 makeGraphCommandDisabledError（稳定码 GRAPH_UNKNOWN_ERROR + 静态文案，零 secret）。
  graphQuery: (req: GraphQueryRequest, signal?: AbortSignal) =>
    GRAPH_COMMANDS_AVAILABLE
      ? invoke<GraphQueryResult>("graph_query", { ...req }, signal ? { signal } : {})
      : Promise.reject<GraphQueryResult>(makeGraphCommandDisabledError("graph_query")),
  graphNodeGet: (id: string) =>
    GRAPH_COMMANDS_AVAILABLE
      ? invoke<GraphNodeView | null>("graph_node_get", { id })
      : Promise.reject<GraphNodeView | null>(makeGraphCommandDisabledError("graph_node_get")),
  graphStats: () =>
    GRAPH_COMMANDS_AVAILABLE
      ? invoke<GraphStats>("graph_stats")
      : Promise.reject<GraphStats>(makeGraphCommandDisabledError("graph_stats")),

  // 运行历史（读 skill-runs.json / agent-runs.json，各自 500 上限 FIFO）。
  skillRunsList: (id: string) => invoke<SkillRunRecord[]>("skill_runs_list", { id }),
  agentRunsList: (id: string) => invoke<AgentRunRecord[]>("agent_runs_list", { id }),

  // 订阅脚本输出流（payload 为 ScriptOutputEvent；前端用 rAF 合并，避免高频打满渲染）
  onScriptOutput: (cb: (e: ScriptOutputEvent) => void) =>
    listen<ScriptOutputEvent>("script-output", (e) => cb(e.payload)),

  // 订阅脚本运行结束事件（payload 为 ScriptFinishedEvent，含最终 RunSnapshot）
  onScriptFinished: (cb: (e: ScriptFinishedEvent) => void) =>
    listen<ScriptFinishedEvent>("script-finished", (e) => cb(e.payload)),

  // M1-2 收藏领域
  bookmarkAdd: (p: { url: string; title: string; category: string }) =>
    invoke<Bookmark>("add_bookmark", p),
  bookmarkList: () => invoke<Bookmark[]>("list_bookmarks"),
  bookmarkRemove: (id: string) => invoke("remove_bookmark", { id }),
  importBrowserCredentials: (rows: { url: string; username: string; password: string }[]) =>
    invoke<number>("import_browser_credentials", { rows }),
  // 已导入账号列表（只读）：只返回 url / username / has_password，后端绝不返回密码
  listBrowserCredentials: () => invoke<BrowserCredentialItem[]>("list_browser_credentials"),
  // 用户主动触发的一次性填充：只传不透明句柄 + 页签 id，密码不出 Rust，
  // 返回值只是非敏感状态码（AutofillResult）
  fillBrowserCredential: (credentialId: string, tabId: string) =>
    invoke<AutofillResult>("fill_browser_credential", {
      credentialId,
      tabId,
    }),

  // 本地文件浏览器
  listDir: (path: string) => invoke<DirEntry[]>("list_dir", { path }),

  readFile: (path: string) => invoke<string>("read_file", { path }),

  readImageDataUrl: (path: string) => invoke<string>("read_image_data_url", { path }),

  writeFile: (path: string, content: string) => invoke("write_file", { path, content }),

  getStartDirs: () => invoke<DirEntry[]>("get_start_dirs"),

  // 用系统文件管理器定位到成果所在目录
  revealArtifact: (id: string) => invoke("reveal_artifact", { id }),

  // 用系统文件管理器打开指定路径（文件树右键"资源管理器打开"）
  revealPath: (path: string) => invoke("reveal_path", { path }),

  // 跨目录移动文件/目录（文件树拖拽移动）
  movePath: (src: string, dstDir: string) =>
    invoke("move_path", { src, dstDir }),

  // 用系统默认浏览器打开成果来源 URL
  openSource: (url: string) => invoke("open_source", { url }),

  // ====== 文件管理：新建 / 删除 / 重命名 ======
  createFile: (path: string, content?: string) =>
    invoke("create_file", { path, content: content ?? null }),

  createDir: (path: string) => invoke("create_dir", { path }),

  deletePath: (path: string) => invoke("delete_path", { path }),

  renamePath: (path: string, newName: string) =>
    invoke("rename_path", { path, newName }),

  // ====== 剪贴板 ======
  clipboardRead: () => invoke<string>("clipboard_read"),

  clipboardWrite: (text: string) => invoke("clipboard_write", { text }),

  // ====== 宫格浏览器 ======
  // 返回实际创建的格数：内存预算守卫在可用内存不足时会自动降级（保底 2）
  // urls：每格首导航 URL（长度 ≥ n），由后端 create_grid 直接用于创建子 webview，
  // 避免"先占位再二次导航"的双导航；未配置的格子传 "about:blank"。
  createGrid: (n: number, urls: string[]) =>
    invoke<number>("create_grid", { n, urls }),

  closeGrid: () => invoke("close_grid"),

  gridOpen: (index: number, url: string) =>
    invoke("grid_open", { index, url }),

  gridPosition: (
    index: number,
    p: { x: number; y: number; width: number; height: number }
  ) => invoke("grid_position", { index, ...p }),

  // 宫格缩放单独管理（与定位解耦，避免每次定位都重复 zoom 卡顿）
  gridSetZoom: (index: number, zoom: number) =>
    invoke("grid_set_zoom", { index, zoom }),

  gridCloseOne: (index: number) => invoke("grid_close_one", { index }),

  // 强制隐藏所有子 webview（页签+宫格），切到非浏览器视图时调用。
  // 后端用无去重的 hide_bounds，避免 grid_position 的 50ms 去重把隐藏请求丢弃。
  hideAllWebviews: () => invoke("hide_all_webviews"),

  // 隐藏单个子 webview（无去重），宫格显示时移出激活页签用
  hideWebview: (id: string) => invoke("hide_webview", { id }),

  // 前端链路追踪：把关键步骤打到后端终端，定位"请求在哪一步丢失"（fire-and-forget）
  debugLog: (msg: string) => {
    invoke("debug_log", { msg }).catch(() => {});
  },

  // ====== 系统应用 ======
  listApps: () => invoke<AppEntry[]>("list_apps"),

  launchApp: (exec: string) => invoke("launch_app", { exec }),

  // ====== 浏览器页签 ======
  tabNew: (url: string) => invoke<TabInfo>("tab_new", { url }),

  tabClose: (id: string) => invoke("tab_close", { id }),

  // 激活页签：后端隐藏其它页签的子 webview 并恢复目标页签显示
  tabActivate: (id: string) => invoke("tab_activate", { id }),

  tabOpen: (id: string, url: string) => invoke("tab_open", { id, url }),

  tabPosition: (
    id: string,
    p: { x: number; y: number; width: number; height: number }
  ) => invoke("tab_position", { id, ...p }),

  tabList: () => invoke<TabInfo[]>("tab_list"),

  tabSetTitle: (id: string, title: string) =>
    invoke("tab_set_title", { id, title }),

  tabGoBack: (id: string) => invoke("tab_go_back", { id }),

  tabGoForward: (id: string) => invoke("tab_go_forward", { id }),

  tabReload: (id: string) => invoke("tab_reload", { id }),

  // 在指定子 webview 中执行 JavaScript（用于 AI 模式向宫格注入问题）
  evalInTab: (id: string, js: string) => invoke<string>("eval_in_tab", { id, js }),

  // 页签休眠开关（默认关；开启后非激活超 10 分钟的页签销毁 webview 仅留 URL，激活时重建）
  setTabHibernation: (enabled: boolean) =>
    invoke("set_tab_hibernation", { enabled }),

  // 资源占用统计（主进程树 + 每宫格子进程树 RSS）
  resourceStats: () => invoke<ResourceStats>("resource_stats"),

  // 订阅页签标题更新事件（后端在页面加载完成后回传真实标题）
  onTabTitle: (cb: (t: TabInfo) => void) =>
    listen<TabInfo>("tab-title", (e) => cb(e.payload)),

  // 子 webview 里的 target="_blank" / window.open 被 Rust 拦截后，
  // 通过此事件通知前端代开新页签（避免在事件回调里直接创建窗口死锁）。
  onNewTabRequest: (cb: (u: { url: string }) => void) =>
    listen<{ url: string }>("new-tab-request", (e) => cb(e.payload)),

  // 子 webview 内导航完成（点链接/前进/后退/刷新后）同步地址栏与页签 URL。
  onTabNavigated: (cb: (d: { id: string; url: string }) => void) =>
    listen<{ id: string; url: string }>("tab-navigated", (e) => cb(e.payload)),

  // tab-N 主进程 WebView 恢复状态（有限预算；失败/耗尽可被前端提示和日志观测）。
  onTabRecovery: (cb: (d: TabRecoveryEvent) => void) =>
    listen<TabRecoveryEvent>("tab-recovery", (e) => cb(e.payload)),

  // ====== M1-4 默认浏览器接入（外部打开 URL 路由） ======
  // 拉取并清空后端 pending 队列（冷启动 argv / 单实例转发 / RunEvent::Opened
  // 统一进队；拉取即清空，天然去重）。
  takePendingOpenUrls: () => invoke<string[]>("take_pending_open_urls"),
  // 后端在就绪后收到新外部 URL 时发的轻提示（URL 本体须用 takePendingOpenUrls 拉取）
  onOpenUrlPending: (cb: () => void) =>
    listen("app://open-url-pending", () => cb()),
  // 非 http/https 的外部打开请求被后端拒绝（安全策略）——前端据此 toast
  onOpenUrlRejected: (cb: (u: { url: string }) => void) =>
    listen<{ url: string }>("app://open-url-rejected", (e) => cb(e.payload)),
  // 查询当前系统默认浏览器（xdg-settings get）
  getDefaultBrowser: () => invoke<string>("get_default_browser"),
  // 把本应用设为系统默认浏览器。硬约束：只能由设置页按钮经用户显式确认后调用
  setDefaultBrowser: () => invoke<string>("set_default_browser"),

  // ====== 真实 PTY 终端（M3.a：输出走每终端独立 Channel 单播）======
  // Tauri v2 的 Channel 只能从 `@tauri-apps/api/core` 子路径导入（非顶层）。
  createTermChannel: (
    cb: (msg: TermMessage) => void
  ): Channel<TermMessage> => {
    const ch = new Channel<TermMessage>();
    ch.onmessage = (msg) => cb(msg);
    return ch;
  },

  // 注意：invoke 的 key 必须与 Rust 命令参数名一致（snake_case = "channel"），
  // 否则 Tauri 反射不到参数。Channel 由 Tauri 序列化为 IPC 句柄自动传参。
  // probe 字段：后端按 MVP_TERMINAL_PROBE=1 注入，前端据此启用终端对账打点
  termSpawnChannel: (channel: Channel<TermMessage>) =>
    invoke<{ id: string; probe?: boolean }>("term_spawn_channel", { channel }),

  termWrite: (id: string, data: string) => invoke("term_write", { id, data }),

  termResize: (id: string, cols: number, rows: number) =>
    invoke("term_resize", { id, cols, rows }),

  termKill: (id: string) => invoke("term_kill", { id }),

  // 订阅终端输出流
  onTermData: (cb: (d: { id: string; data: string }) => void) =>
    listen<{ id: string; data: string }>("term-data", (e) => cb(e.payload)),

  // ====== M1-8 资源瀑布（请求拦截与瀑布） ======
  // 所有数据均已由后端脱敏（敏感查询参数值为 ***），不含任何 headers/body。
  // 查询某 tab 的资源瀑布记录（含容量驱逐计数）
  listTabResources: (tabId: string) =>
    invoke<TabResourceList>("list_tab_resources", { tabId }),

  // 清空某 tab 的资源瀑布记录（后端写审计，仅 tab_id + 计数，不含 URL）
  clearTabResources: (tabId: string) =>
    invoke("clear_tab_resources", { tabId }),

  // 查询资源采集设置（开关 + 容量上限）
  getResourceCaptureSettings: () =>
    invoke<ResourceCaptureSettings>("get_resource_capture_settings"),

  // 设置采集开关与每 tab 容量（会话内生效，不持久化）
  setResourceCaptureSettings: (enabled: boolean, maxPerTab?: number) =>
    invoke<ResourceCaptureSettings>("set_resource_capture_settings", {
      enabled,
      maxPerTab: maxPerTab ?? null,
    }),

  // 订阅资源事件（payload 为脱敏后的 ResourceReceived DTO）
  onResourceReceived: (cb: (r: ResourceReceived) => void) =>
    listen<ResourceReceived>("resource-received", (e) => cb(e.payload)),

  // ====== M1-9 会话存档与关闭协议 ======
  // 落盘数据全部为后端脱敏形态（URL 敏感参数值为 ***，无 headers/body）；
  // 未显式保存的草稿不落盘（auto_save_on_exit 默认关）。

  // 保存当前页签为会话（立即落盘）。preview 为最小文本预览（可空，后端截断 512B）
  sessionSave: (tabId: string, preview?: string) =>
    invoke<SessionSummary>("session_save", { tabId, preview: preview ?? null }),

  // 明确丢弃草稿（关闭弹窗选「删除」）：不落盘
  sessionDiscard: (tabId: string) => invoke("session_discard", { tabId }),

  // 列出本地会话存档（按 updated_at 倒序）
  sessionList: () => invoke<SessionSummary[]>("session_list"),

  // 读取会话详情（含已脱敏资源列表）
  sessionGet: (id: string) => invoke<BrowserSession>("session_get", { id }),

  // 删除会话存档（幂等；删除存档不影响仍打开的同名页签）
  sessionDelete: (id: string) => invoke<boolean>("session_delete", { id }),

  // 导出会话为脱敏 JSON 文本（不写磁盘，由前端决定保存位置）
  sessionExport: (id: string) => invoke<string>("session_export", { id }),

  // 用会话中已脱敏的 URL 新建页签（登录态/一次性 token 不会恢复）
  sessionRestore: (id: string) => invoke<TabInfo>("session_restore", { id }),

  // 关闭路径 flush：按策略落盘/释放草稿 + 清理 tmp + 容量裁剪
  flushSessions: () => invoke<SessionFlushReport>("flush_sessions"),

  // 会话策略：关闭弹窗 / 退出自动保存
  getSessionPolicy: () => invoke<SessionPolicy>("get_session_policy"),

  setSessionPolicy: (closePrompt?: boolean, autoSaveOnExit?: boolean) =>
    invoke<SessionPolicy>("set_session_policy", {
      closePrompt: closePrompt ?? null,
      autoSaveOnExit: autoSaveOnExit ?? null,
    }),

  // ====== M0-0.b 测量钩子（契约 logs/m0-baseline-contract-v1.md §6.1/§6.3） ======
  // ready 信号：前端 mount + 2×rAF 后调用；后端写带 run_id 的 ready 信号（轻量 IPC 往返）
  m0Ready: () => invoke<string>("m0_ready"),
  // 查询当前 M0 测量配置（非测量运行返回 null）
  m0Config: () => invoke<M0Config | null>("m0_config"),
  // 终端吞吐报告：前端检测 __M0_TERM_END__ 并完成下一次 animation frame 后上报
  m0TermReport: (report: {
    begin_seen: number;
    end_seen: number;
    consumed_bytes: number;
    start_ts_ms: number;
    end_ts_ms: number;
    frame_gaps_ms: number[];
  }) => invoke("m0_term_report", { report }),

  // ====== M5-2 MCP 只读注册表/策略桥（W7）======
  // 仅暴露只读 introspection：策略快照 / 注册表 / 能力裁决预览。
  // 后端来源校验确保只有受信任的 main 主窗口可调用（tab-*/grid-* 无令牌被拒）。
  mcpPolicyGet: () => invoke<McpPolicySnapshot>("mcp_policy_get"),
  mcpRegistryList: () => invoke<McpRegistryEntry[]>("mcp_registry_list"),
  mcpCapabilityPreview: (p: { capability: string; rawPath?: string | null }) =>
    invoke<McpDecision>("mcp_capability_preview", p),

  // ====== M5-W13 插件 manifest 生命周期 Stage-I（仅本地状态，无执行）======
  // 后端来源校验确保只有受信任的 main 主窗口可调用（tab-*/grid-* 无令牌被拒）。
  // 本 wave **不**提供 invoke / 执行 / 动态加载 / 下载相关封装；公钥只回 16-hex 指纹。
  pluginInstall: (p: { manifest: PluginManifest; resourcePath?: string | null }) =>
    invoke<PluginSummary>("plugin_install", {
      manifest: p.manifest,
      resourcePath: p.resourcePath ?? null,
    }),
  pluginEnable: (id: string) => invoke<PluginSummary>("plugin_enable", { id }),
  pluginDisable: (id: string) => invoke<PluginSummary>("plugin_disable", { id }),
  pluginList: (state?: PluginState | null) =>
    invoke<PluginSummary[]>("plugin_list", { state: state ?? null }),
  pluginGet: (id: string) => invoke<PluginDetail>("plugin_get", { id }),
  pluginKeysAdd: (p: { keyId: string; pubkey: string; note?: string | null }) =>
    invoke<TrustedKeyRecord[]>("plugin_keys_add", {
      keyId: p.keyId,
      pubkey: p.pubkey,
      note: p.note ?? null,
    }),
  pluginKeysList: () => invoke<TrustedKeyRecord[]>("plugin_keys_list"),
  pluginKeysRemove: (keyId: string) =>
    invoke<TrustedKeyRecord[]>("plugin_keys_remove", { keyId }),
};
