import { invoke } from "@tauri-apps/api/core";
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
} from "./types";

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

  // 本地文件浏览器
  listDir: (path: string) => invoke<DirEntry[]>("list_dir", { path }),

  readFile: (path: string) => invoke<string>("read_file", { path }),

  writeFile: (path: string, content: string) => invoke("write_file", { path, content }),

  getStartDirs: () => invoke<DirEntry[]>("get_start_dirs"),

  // 用系统文件管理器定位到成果所在目录
  revealArtifact: (id: string) => invoke("reveal_artifact", { id }),

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
  createGrid: (n: number) => invoke<number>("create_grid", { n }),

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

  // ====== 真实 PTY 终端 ======
  termSpawn: () => invoke<{ id: string }>("term_spawn"),

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
};
