export interface Artifact {
  id: string;
  title: string;
  source_url: string;
  text: string;
  html: string;
  hash: string;
  created_at: string;
  tags: string[];
  /** M2-1 图片附件（只存引用，不存字节）；旧数据无该字段时按空数组处理 */
  images: ImageRef[];
}

// ====== M2-1 图片领域 ======
// 与后端 domain.rs 的 ImageSource / ImageRef 一一对应。
// 结构上不含 headers / Cookie / Authorization / body（与 M1-8/M1-9 同口径）。

export type ImageSource = "file" | "inline_data_url";

export interface ImageRef {
  id: string;
  source: ImageSource;
  /** 仅 source=file 时有值：相对 workspace 目录的路径 */
  rel_path: string | null;
  mime: string;
  bytes: number;
  /** 解析失败时为 null（后端不伪造尺寸） */
  width: number | null;
  height: number | null;
  sha256: string;
  /** 溯源 URL，后端落库前已脱敏（敏感查询值为 ***） */
  source_url: string | null;
  caption: string | null;
  created_at: string;
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

// ====== M1-9 会话存档与关闭协议 ======
// 与后端 domain.rs 一一对应。落盘白名单：id/tab_id/url(已脱敏)/title/
// preview(已脱敏+截断)/preview_truncated/resource_count/resources(脱敏 DTO)/
// saved/close_reason/created_at/updated_at。
// 落盘黑名单（结构上不存在）：token/cookie/Authorization/Set-Cookie/headers/
// request body/response body/插件原始资源事件/任何凭据。

export interface BrowserSession {
  id: string;
  tab_id: string;
  url: string;
  title: string;
  preview: string;
  preview_truncated: boolean;
  resource_count: number;
  resources: ResourceReceived[];
  saved: boolean;
  close_reason: string;
  created_at: string;
  updated_at: string;
}

// 列表项（不含 resources 全量）
export interface SessionSummary {
  id: string;
  tab_id: string;
  url: string;
  title: string;
  preview: string;
  preview_truncated: boolean;
  resource_count: number;
  saved: boolean;
  close_reason: string;
  created_at: string;
  updated_at: string;
}

// 会话策略（会话内生效，不持久化）
export interface SessionPolicy {
  // 关闭 tab 时是否弹「保存 / 删除」（默认开：关闭不可静默丢弃）
  close_prompt: boolean;
  // 退出应用前是否自动保存仍打开的 tab（默认关：不静默保存）
  auto_save_on_exit: boolean;
}

// flush 结果（关闭路径的确定性行为报告）
export interface SessionFlushReport {
  persisted: number;
  drafts_dropped: number;
  tmp_removed: number;
  capacity_removed: number;
}

// 关闭 tab 时用户的选择（关闭协议）
export type SessionCloseChoice = "save" | "discard" | "cancel";

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

// ====== M2-3 脚本领域 ======
// 与后端 domain.rs 的 ParamType / ScriptInterpreter / ScriptParam / ScriptMeta
// 一一对应（serde rename_all = "snake_case"，故嵌套字段一律 snake_case：
// Tauri 只对**顶层命令参数名**做 camel→snake 转换，嵌套结构体不做转换）。
//
// 只描述「脚本是什么」，**无执行能力**：run_script / 取消 / 超时 / 运行记录
// 属 M2-4，前端面板属 M2-5。

export type ParamType = "string" | "int" | "bool" | "enum" | "path";

/** 解释器白名单：后端是枚举，前端不得提供白名单之外的取值 */
export type ScriptInterpreter = "bash" | "sh" | "python3" | "node" | "shebang";

export interface ScriptParam {
  /** 占位符名，对应脚本正文里的 `${name}` */
  name: string;
  label: string;
  param_type: ParamType;
  required: boolean;
  default: string | null;
  /** 仅 enum 使用 */
  options: string[];
  /** argv 模式下无效果；仅保留兼容字段 */
  raw: boolean;
  /** 敏感参数：UI 用密码框，审计值一律 *** */
  secret: boolean;
}

export interface ScriptMeta {
  id: string;
  name: string;
  category: string;
  /** 正文文件相对 scripts 目录的文件名（非绝对路径） */
  path: string;
  interpreter: ScriptInterpreter;
  params: ScriptParam[];
  description: string;
  builtin: boolean;
  enabled: boolean;
  /** 0 = 使用全局默认（60） */
  timeout_secs: number;
  created_at: string;
  updated_at: string;
}

// ====== M2-6 命令片段领域 ======
// 与后端 domain.rs 的 CommandSnippet 一一对应。命令以 argv 数组存储；
// 无 path/正文文件，不含 token/cookie/authorization/body。
export interface CommandSnippet {
  id: string;
  name: string;
  category: string;
  interpreter: ScriptInterpreter;
  argv: string[];
  params: ScriptParam[];
  description: string;
  dangerous: boolean;
  builtin: boolean;
  enabled: boolean;
  /** 0 = 使用全局默认（60），上限由后端校验为 600 */
  timeout_secs: number;
  created_at: string;
  updated_at: string;
}

export type RunStatus = "running" | "succeeded" | "failed" | "cancelled" | "timeout";

export interface RunSnapshot {
  run_id: string;
  script_id: string;
  status: RunStatus;
  started_at: string;
  finished_at: string | null;
  exit_code: number | null;
  error: string | null;
  output_tail: string;
  truncated: boolean;
  output_seq: number;
}

export interface ScriptRunRecord {
  run_id: string;
  script_id: string;
  status: RunStatus;
  started_at: string;
  finished_at: string;
  exit_code: number | null;
  error: string | null;
  output_tail: string;
  truncated: boolean;
}

export interface ScriptOutputEvent {
  run_id: string;
  chunk: string;
  seq: number;
}

export interface ScriptFinishedEvent {
  snapshot: RunSnapshot;
}

// ====== M2-7 / M2-8 工具领域 ======
// 与后端 domain.rs 的 ToolMeta / ToolSource 一一对应（serde rename_all = "snake_case"）。
// 工具是静态离线 HTML，无执行能力；打开/隔离在 M2-8（tool:// 协议 + 零能力子 webview）。
export type ToolSource = "builtin" | "user";

export interface ToolMeta {
  id: string;
  name: string;
  description: string | null;
  category: string;
  source: ToolSource;
  entry: string;
}

// ====== M3.a 终端输出契约 ======
// 与后端 `terminal.rs::TermMessage::to_value` 一一对应：{ id, kind, ... }。
// - data：PTY 原始输出（直接喂 xterm）
// - flow：丢弃统计（仅告知，不进终端字节流，避免污染 ANSI 语义）
// - exit：会话结束（eof 正常 / channel_dead 输出通道不可恢复）
export type TermExitReason = "eof" | "channel_dead";

export interface TermMessage {
  id: string;
  kind: "data" | "flow" | "exit";
  data?: string;
  reason?: TermExitReason;
  dropped_chunks?: number;
  dropped_bytes?: number;
}

// ====== M4-5 / M4-8 定时任务领域 ======
// 与后端 domain.rs 的 TaskDef / TaskRunRecord 一一对应（serde rename_all = "snake_case"）。
// 契约源：`logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md` §3.1 / §6。
// 命令名沿用 A1 展开卡已冻结的 5 条（task_list/add/update/remove/run_now），本文件不改名。
export type TaskKind = "script" | "command";

export type MissedRunPolicy = "skip" | "run_once" | "catch_up";

export type RetryBackoff = "fixed" | "exponential";

export type TaskRunTrigger = "scheduled" | "manual" | "catch_up" | "retry";

/// 触发方式：外部标签枚举，序列化为 { cron: { expr } } 或 { interval: { every_secs } }。
/// 后端只接受标准 5 段 cron（分 时 日 月 周）；间隔下界 60 秒（A6 F-A6-6 保守收窄）。
export type TaskTrigger =
  | { cron: { expr: string } }
  | { interval: { every_secs: number } };

export interface RetryPolicy {
  /// 含首次；1 = 不重试（默认），硬上限 5
  max_attempts: number;
  backoff: RetryBackoff;
  base_delay_secs: number;
  max_delay_secs: number;
}

export interface TaskDef {
  id: string;
  name: string;
  kind: TaskKind;
  /// 目标 id：ScriptMeta.id（script）或 CommandSnippet.id（command）
  target_id: string;
  /// 非 secret 参数值；secret 参数值不落盘（A6 §3.3 R-3）
  params: Record<string, string>;
  /// 默认 false（A6 裁定 R-A6-1：自动执行默认关闭）
  enabled: boolean;
  trigger: TaskTrigger;
  missed_run_policy: MissedRunPolicy;
  /// 仅 catch_up 生效；默认 3，硬上限 10
  catch_up_limit: number;
  /// 迟到超过该秒数才判定为「错过」；默认 60
  misfire_grace_secs: number;
  retry: RetryPolicy;
  /// 0 = 沿用 script_runner 默认 60；上限 600
  timeout_secs: number;
  /// 判重真相源：上一次「已触发」的计划时刻（不是完成时刻）
  last_fired_at: string | null;
  /// 下一次计划触发时刻（永远指向未来）
  next_run_at: string | null;
  created_at: string;
  updated_at: string;
}

// M4-8（A8 接线）：`task_add` 命令的入参形态。
// 后端 `task_add` 是**平铺命名参数**（不是 TaskDef 结构体）：`name, kind, target_id,
// trigger, params, missed_run_policy, catch_up_limit, misfire_grace_secs, retry,
// timeout_secs, enabled`。Tauri 默认把 Rust 命令参数从 snake_case 转成 camelCase，
// 因此前端调用时多词键必须写成 camelCase（targetId / catchUpLimit / misfireGraceSecs /
// timeoutSecs）。该结构体即此契约的类型落地，由 `utils/taskUi.ts::serializeTaskAddArgs` 生成。
export interface TaskAddArgs {
  name: string;
  kind: TaskKind;
  targetId: string;
  trigger: TaskTrigger;
  params?: Record<string, string>;
  missedRunPolicy?: MissedRunPolicy;
  catchUpLimit?: number;
  misfireGraceSecs?: number;
  retry?: RetryPolicy;
  timeoutSecs?: number;
  enabled?: boolean;
}

export interface TaskRunRecord {
  task_id: string;
  /// 与 ScriptRunRecord.run_id 同值（join key）
  run_id: string;
  trigger: TaskRunTrigger;
  /// 计划触发时刻；重试沿用同一值，只递增 attempt
  scheduled_at: string;
  attempt: number;
  started_at: string;
  finished_at: string | null;
  status: RunStatus;
  exit_code: number | null;
  /// 稳定错误码（如 SCRIPT_ALREADY_RUNNING / TASK_TARGET_NOT_FOUND）
  error_code: string | null;
}

/// 跳过原因（A6 §5.2）。UI 必须能展示（O-A6-9），否则用户只看到「没跑」。
export type TaskSkipReason = "reentrant" | "target_busy" | "global_limit";

// ====== M4-4 数据库领域 ======
// 与后端 domain.rs 的 DbConnectionConfig / DbConnectResult / DbQueryResult 一一对应
// （serde rename_all = "snake_case"，故嵌套字段一律 snake_case：Tauri 只对顶层命令参数名
// 做 camel→snake 转换，嵌套结构体不做转换）。
// 命令名沿用 A1 展开卡已冻结的 3 条（db_connect / db_query / db_disconnect），本文件不改名。
// 后端实现归 A4（M4-2.s / M4-3）；前端封装归 A5（M4-4），A4 检查点已明确「前端 wiring 归 A5」。
export type DbKind = "sqlite" | "mysql" | "postgres";

export type DbSslMode = "disable" | "prefer" | "require";

export type DbLimitKind = "rows" | "bytes" | "field";

export type DbQueryState = "completed" | "cancelled" | "timeout" | "failed";

// 与 domain.rs::DbValue 一致（带标签枚举：Null/Bool/Int/Float/Text/BlobLen）。
export type DbValue =
  | "Null"
  | { Bool: boolean }
  | { Int: number }
  | { Float: number }
  | { Text: string }
  | { BlobLen: number };

// db_connect 入参（结构性无 password 字段，F2：凭据只走瞬时参数，绝不进本 DTO）。
export interface DbConnectionConfig {
  id: string;
  name: string;
  kind: DbKind;
  host: string | null;
  port: number | null;
  database: string;
  username: string | null;
  ssl_mode: DbSslMode;
  allow_write: boolean;
  production_hint: boolean | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface DbConnectResult {
  conn_id: string;
  kind: DbKind;
}

// db_query 返回（与 domain.rs::DbQueryResult 一致；rows 内为带标签 DbValue）。
export interface DbQueryResult {
  query_id: string;
  columns: string[];
  rows: DbValue[][];
  row_count: number;
  truncated: boolean;
  field_truncated: boolean;
  limit_hit: DbLimitKind | null;
  elapsed_ms: number;
  state: DbQueryState;
}
