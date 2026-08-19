import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Artifact,
  RepoConfig,
  SyncPreview,
  SyncJob,
  AuditEntry,
  WorkspaceTree,
  DirEntry,
  BrowserResources,
  TabInfo,
  AppEntry,
} from "./types";

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

  // token 仅在此调用中传给后端，存入系统密钥库；不会被前端持久化/回显
  configureRepo: (p: { config: RepoConfig; token: string }) =>
    invoke("configure_repo", p),

  listRepos: () => invoke<RepoConfig[]>("list_repos"),

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
  createGrid: (n: number) => invoke("create_grid", { n }),

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

  // ====== 真实 PTY 终端 ======
  termSpawn: () => invoke<{ id: string }>("term_spawn"),

  termWrite: (id: string, data: string) => invoke("term_write", { id, data }),

  termResize: (id: string, cols: number, rows: number) =>
    invoke("term_resize", { id, cols, rows }),

  termKill: (id: string) => invoke("term_kill", { id }),

  // 订阅终端输出流
  onTermData: (cb: (d: { id: string; data: string }) => void) =>
    listen<{ id: string; data: string }>("term-data", (e) => cb(e.payload)),
};
