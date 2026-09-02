export interface Artifact {
  id: string;
  title: string;
  source_url: string;
  text: string;
  html: string;
  hash: string;
  created_at: string;
  tags: string[];
}

export type RepoProvider = "git" | "gitee";

// 注意：token 永远不会出现在该类型里（凭据隔离）
export interface RepoConfig {
  id: string;
  provider: RepoProvider;
  name: string;
  remote_url: string;
  branch: string;
  username: string;
}

// ====== M1-5 Git 只读能力 ======
// 与后端 domain.rs 的 DTO 一一对应（只描述仓库状态，不含任何凭据）。

export type GitFileStatusKind =
  | "modified"
  | "added"
  | "deleted"
  | "renamed"
  | "untracked"
  | "conflicted";

export interface GitFileStatus {
  path: string;
  status: GitFileStatusKind;
}

export interface GitDiffHunk {
  file: string;
  old_content: string | null;
  new_content: string | null;
  truncated: boolean;
  binary: boolean;
}

export interface GitBranch {
  name: string;
  is_remote: boolean;
  is_head: boolean;
}

export interface GitDiffResult {
  hunks: GitDiffHunk[];
  more: boolean;
}

// ====== M1-6.b Git 写能力（双阶段确认闸门） ======
// 与后端 domain.rs 的 DTO 一一对应：只承载操作语义与计数，
// 不含 token/凭据/完整 diff。写操作必须先 request（生成预览+待确认任务），
// 再 confirm（dangerous 操作需 confirmedDangerous=true 二次确认）才执行。

export type GitWriteOp =
  | "stage"
  | "unstage"
  | "discard"
  | "commit"
  | "create_branch"
  | "checkout_branch"
  | "push";

export type GitWriteStatus = "pending" | "running" | "success" | "failed";

export interface GitWriteJob {
  id: string;
  repo_id: string;
  op: GitWriteOp;
  paths: string[];
  message?: string | null;
  branch?: string | null;
  checkout: boolean;
  status: GitWriteStatus;
  dangerous: boolean;
  created_at: string;
  expires_at: string;
  finished_at?: string | null;
  error?: string | null;
}

export interface GitWritePreview {
  job_id: string;
  repo_id: string;
  op: GitWriteOp;
  summary: string;
  affected_paths: string[];
  path_count: number;
  dangerous: boolean;
  expires_at: string;
}

// ====== M1-7 Git UI 派生字段 ======
// 后端 `GitWritePreview`（domain.rs）**没有** risk 字段，只有 `dangerous` 布尔。
// UI 展示的「风险等级」由前端按 op 白名单 + dangerous 派生（useGitStore.gitWriteRisk），
// 属纯展示派生值，不参与后端任何安全判定（判定只看 dangerous）。
export type GitWriteRisk = "low" | "medium" | "high";

export interface SyncPreview {
  job_id: string;
  repo_id: string;
  repo_name: string;
  artifact_count: number;
  artifact_titles: string[];
  remote_url: string;
}

export type SyncStatus =
  | "pending"
  | "confirmed"
  | "running"
  | "success"
  | "failed";

export interface SyncJob {
  id: string;
  repo_id: string;
  status: SyncStatus;
  artifact_ids: string[];
  created_at: string;
  finished_at?: string;
  error?: string;
}

export interface AuditEntry {
  at: string;
  action: string;
  detail: string;
}

// 收藏项（M1-2：与 Rust domain.rs::Bookmark 对齐）
export interface Bookmark {
  id: string;
  url: string;
  title: string;
  category: string;
  created_at: string;
}

// 工作区目录树
export interface WorkspaceTree {
  nodes: DomainNode[];
}
export interface DomainNode {
  host: string;
  items: DomainItem[];
}
export interface DomainItem {
  id: string;
  title: string;
  created_at: string;
  tags: string[];
}

// 本地文件浏览器
export interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

// 网页资源项（js/css/svg 等）
export interface ResourceItem {
  res_type: string;
  url: string;
  absolute: string;
}

export interface BrowserResources {
  page_url: string;
  items: ResourceItem[];
}

// 浏览器页签
export interface TabInfo {
  id: string;
  url: string;
  title: string;
}

// ====== M1-8 资源瀑布（与后端 domain.rs 一一对应） ======
// 隐私红线：DTO 不含 headers/Cookie/Authorization/Set-Cookie/任何 body；
// url 已由后端脱敏（敏感查询参数值为 ***）。Option 字段为 null 表示平台
// 拿不到（降级采集，不伪造）。
export type ResourceKind =
  | "document"
  | "script"
  | "stylesheet"
  | "image"
  | "xhr_fetch"
  | "font"
  | "media"
  | "other";

export interface ResourceReceived {
  id: string;
  tab_id: string;
  url: string;
  method: string;
  status: number | null;
  mime: string | null;
  size_bytes: number | null;
  started_at: number;
  finished_at: number | null;
  duration_ms: number | null;
  resource_type: ResourceKind;
}

export interface ResourceCaptureSettings {
  enabled: boolean;
  max_per_tab: number;
  max_total: number;
  max_url_bytes: number;
}

// list_tab_resources 返回：记录 + 因容量上限被 FIFO 丢弃的累计条数
export interface TabResourceList {
  records: ResourceReceived[];
  evicted: number;
  enabled: boolean;
}

export type TabRecoveryStatus =
  | "attempting"
  | "recovered"
  | "failed"
  | "budget-exhausted"
  | "load-failed";

export interface TabRecoveryEvent {
  id: string;
  url: string;
  reason: string;
  status: TabRecoveryStatus;
  attempt: number;
  max_attempts: number;
  window_secs: number;
  message: string;
}

// 系统应用（含图标路径）
export interface AppEntry {
  name: string;
  exec: string;
  icon: string;
  icon_path?: string;
}

// 进程资源占用（RSS 进程树合计，MB）
export interface ProcStat {
  pid: number;
  name: string;
  rss_mb: number;
}

// 应用资源统计（主进程树 + 每宫格子进程树）
export interface ResourceStats {
  mem_total_mb: number;
  mem_available_mb: number;
  app_total_mb: number;
  main: ProcStat;
  grids: ProcStat[];
  // 页签休眠开关状态
  hibernation_enabled: boolean;
  // 当前已休眠页签数
  hibernated_count: number;
  // 内存预算守卫：当前可用内存最多支撑几格
  grid_budget: number;
}
