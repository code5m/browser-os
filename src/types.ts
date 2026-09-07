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

// ====== M5-4 / M5-5 / M5-6 Agent/Skill UI 类型（前端 DTO 镜像；与 src-tauri/src/domain.rs 同义）======
// 仅数据镜像，不含任何执行 / 安装 / 网络 / 凭据字段。
// 后端 domain.rs（Lane A5，W4）为权威源；本段与其逐字段对齐（serde rename_all = "snake_case"）。
// UI 侧类型名沿用域类型名（与 A6 W4 数据契约一致），避免 W3 -era 的 SkillMeta/SkillParam 误名。

/// 安装/运行闸门三档。与 M5-5 命令 ACL 同义；末条 ACL 恒为 `list_artifact_images`（K1）。
export type AclLevel = "safe" | "confirm" | "dangerous";

/// Skill 执行体的**唯一**合法形态：脚本/命令引用或串联（类型层面排除内联 shell，K6）。
export type SkillExec =
  | { kind: "script_ref"; scriptId: string; params: Record<string, unknown> }
  | { kind: "command_ref"; commandId: string; params: Record<string, unknown> }
  | { kind: "sequence"; steps: SkillExec[] };

/// 能力引用；单一真源在后端 security_policy.rs（SKILL_CAPABILITY_V1 / AGENT_CAPABILITY_V1）。
export interface CapabilityRef {
  id: string;
}

/// 输入字段（镜像 SkillDef.inputs，后端当前仅 name/required/description）。
export interface SkillInput {
  name: string;
  required: boolean;
  description: string;
}

/// 自检用例（可选，UI 仅展示）。
export interface SkillTest {
  name: string;
  args: unknown;
}

export interface SkillDef {
  id: string;
  version: string;
  displayName: string;
  description: string;
  acl: AclLevel;
  exec: SkillExec;
  inputs: SkillInput[];
  capabilities: CapabilityRef[];
  tests: SkillTest[];
  metadata: unknown;
}

/// Agent 方言标记（仅数据；endpoint 配置在 M5-4.b 执行期再加，避免 W4 引入模型供应商集成）。
export type AgentDialect = "open_ai_compatible" | "external_cli" | "custom";

export interface A2aConfig {
  delegateTo: boolean;
  delegatedFrom: boolean;
}

/// Agent 定义。注意：AgentDef 自身**无** acl 字段；其闸门来自后端 PermissionPreview.gate。
export interface AgentDef {
  id: string;
  version: string;
  displayName: string;
  description: string;
  dialect: AgentDialect;
  systemPrompt: string;
  defaultCapabilities: CapabilityRef[];
  a2a: A2aConfig;
  metadata: unknown;
}

/// 后端提供的权限预览（安装/运行前 UI 展示闸门档与所需能力）。
export interface PermissionPreview {
  gate: AclLevel;
  capabilities: string[];
}

/// 校验报告（后端 agent_validate / skill_validate 返回）：valid 为 true 表示通过校验，errors 非空时携带具体错误。
export interface ValidationReport {
  valid: boolean;
  errors: string[];
}

/// 安装态（镜像后端 SkillInstallState / AgentInstallState；字段由 A5 落码时定，此处取最小集）。
export interface SkillInstallState {
  meta: SkillDef;
  enabled: boolean;
  installedAt: string;
  updatedAt: string;
  grantedCapabilities: string[];
}

export interface AgentInstallState {
  meta: AgentDef;
  enabled: boolean;
  installedAt: string;
  updatedAt: string;
  grantedCapabilities: string[];
}

/// 运行态（复用 M4 RunStatus；前端侧记录，后端落盘各自 500 上限 FIFO）。
export interface SkillRunRecord {
  runId: string;
  skillId: string;
  version: string;
  status: RunStatus;
  startedAt: string;
  finishedAt?: string;
  exitCode?: number;
  truncated: boolean;
  error?: { code: string; message: string; retriable: boolean; details?: string };
}

export interface AgentRunRecord {
  runId: string;
  agentId: string;
  sessionId: string;
  status: RunStatus;
  startedAt: string;
  finishedAt?: string;
  truncated: boolean;
  error?: { code: string; message: string; retriable: boolean; details?: string };
}

/// 流式载荷（前端侧）。后端经 Tauri event 推流；终止用独立的 done/error/canceled 事件，
/// 与 M5-4 §4.3 的 `agent://<id>/stream` + `.../done|error|canceled` 四个 event 对应。
export type StreamChunk =
  | { kind: "data"; data: string; runId?: string; sessionId?: string }
  | { kind: "flow"; droppedChunks?: number; droppedBytes?: number; runId?: string; sessionId?: string };

/// 前端 agent 会话态（仅 UI 展示，不落盘）。
export interface AgentSessionUI {
  sessionId: string;
  agentId: string;
  status: "active" | "streaming" | "done" | "error" | "canceled";
  chunks: StreamChunk[];
  error?: { code: string; message: string };
}

/// 二段式闸门待确认项（A1 M5-6 §4.3：CONFIRM_REQUIRED → 弹窗 → confirm_<action>）。
export type PendingConfirmAction = "install_skill" | "run_skill" | "install_agent";
export interface PendingConfirm {
  action: PendingConfirmAction;
  payload: unknown;
  expiresAt: number;
}

/// 面板三态（空 / 错误 / 加载），供 shell 组件统一渲染。
export type PanelState = "loading" | "empty" | "ready" | "error";

// ====== M5-9 图谱领域（前端 DTO 镜像；与 src-tauri/src/domain.rs 同义）======
// 仅数据镜像，不含任何后端命令调用 / 凭据 / 正文。
// 后端 domain.rs（Lane A7，W5）为权威源；本段与其逐字段对齐（serde rename_all = "snake_case"）。
//
// 隐私红线（K7）：GraphProps 已在后端脱敏（不含量/凭据/body），UI 仍**禁止渲染 props 正文**；
// GraphNode/Edge 无 privacy 字段——脱敏属后端职责，前端只负责不显示 props。

export type GraphNodeKind =
  | "file" | "dir" | "tab" | "script" | "skill" | "agent" | "tag" | "topic";

export type GraphEdgeKind =
  | "in_dir" | "references" | "related_to" | "tagged_with"
  | "uses" | "a2a_with" | "memorizes";

/// 已脱敏的键值对（单一真源在后端：不得含凭据/正文/body）。
export type GraphProps = Record<string, string>;

export interface GraphNode {
  id: string;
  kind: GraphNodeKind;
  label: string;
  /// 后端已脱敏；UI 不得渲染其正文（K7）
  props: GraphProps;
}

export interface GraphEdge {
  from: string;
  to: string;
  kind: GraphEdgeKind;
  weight: number;
  /// 同上，UI 不得渲染（K7）
  props: GraphProps;
}

// ====== M5-W12 图谱 live-query View DTO（与后端 domain.rs GraphNodeView/GraphEdgeView 对齐）======
// 严格删 props（K7 双闸后端 + 前端），仅 {id,kind,label} / {from,to,kind}。
// 这是 A7 W12 实施卡 §7.5 的前端镜像；后端命令返回 View 而非 GraphNode/GraphEdge，
// 故 UI 即便强制遍历 props 也不存在该字段（编译期 + 序列化双闸）。
//
// 字段 snake_case：与后端 GraphNodeView {id,kind,label} / GraphEdgeView {from,to,kind}
// 逐字对齐（serde rename_all 不影响这两条命令的入参出参结构顶层）。

export interface GraphNodeView {
  id: string;
  kind: GraphNodeKind;
  label: string;
}

export interface GraphEdgeView {
  from: string;
  to: string;
  kind: GraphEdgeKind;
}

export interface GraphQueryLimits {
  depth: number;
  limit: number;
}

export interface GraphQueryRequest {
  start_id: string;
  depth?: number | null;
  limit?: number | null;
  /** 用于 AbortController map 关联，避免乱序回包覆盖（与 A7 W12 §5 契约对齐） */
  request_id?: string | null;
}

export interface GraphQueryResult {
  /** start_id 不在 store 时为 false；nodes/edges 为空数组，但不报错 */
  found: boolean;
  nodes: GraphNodeView[];
  edges: GraphEdgeView[];
  /** total > applied.limit 或 depth 被 domain.rs GRAPH_MAX_DEPTH 截断时为 true */
  truncated: boolean;
  /** 实际生效的 depth/limit（可能被 domain.rs 常量 min 截断） */
  applied: GraphQueryLimits;
  node_count: number;
  edge_count: number;
}

export interface GraphStats {
  node_count: number;
  edge_count: number;
  node_capacity: number;
  edge_capacity: number;
  /** ≥ 90% 节点容量 */
  approaching_node_capacity: boolean;
  /** ≥ 90% 边容量 */
  approaching_edge_capacity: boolean;
}

// ====== M5-W12 稳定错误码（与后端 GraphError::code() 1:1 镜像；零 secret echo）======
// 来源：A7 W12 实施卡 §3.4（A7 实施时落地）。前端的 `applyGraphErrorView` /
// `formatGraphStableError`（utils/graphUi.ts）按此表做 1:1 映射 → 稳定 UI 态。
// 错误文本绝不含 label / props / 查询体 / 路径 / URL / token / cookie / Authorization
// （board W12 Hard Stop 第 2 条 + A4 W10 §4-C）。
export type GraphStableErrorCode =
  | "GRAPH_INVALID_ID"
  | "GRAPH_REF_ID_NOT_HEX"
  | "GRAPH_LABEL_TOO_LONG"
  | "GRAPH_PROP_KEY_TOO_LONG"
  | "GRAPH_PROP_VALUE_TOO_LONG"
  | "GRAPH_PROP_COUNT_EXCEEDED"
  | "GRAPH_SECRET_IN_PROPS"
  | "GRAPH_NODE_CAPACITY_EXCEEDED"
  | "GRAPH_EDGE_CAPACITY_EXCEEDED"
  | "GRAPH_DUPLICATE_NODE"
  | "GRAPH_DUPLICATE_EDGE"
  | "GRAPH_STORE_LOAD_FAILED"
  | "GRAPH_UNKNOWN_ERROR";

// 稳定错误视图（前端统一错误模型，UI 不直接消费裸 Promise rejection 文本）。
export interface GraphErrorView {
  code: GraphStableErrorCode;
  /** 用户可见的本地化中文文案（不含 props/secret/URL/路径/查询体；脱敏过） */
  message: string;
}

// ====== M5-2 MCP 只读桥 DTO（与后端 domain.rs / mcp.rs 对齐）======
// 仅描述能力白名单 / 注册表映射 / 裁决结果，不含任何凭据、URL 明文或内部状态。

/** 能力裁决结果（snake_case，与后端 McpDecisionView 对应）。 */
export type McpDecision = "allow" | "deny";

/** 当前生效的 MCP 全局策略快照（与后端 McpPolicySnapshot 对应）。 */
export interface McpPolicySnapshot {
  /** 当前生效的能力集合（恒为 MCP_CAPABILITY_V1）。 */
  capabilities: string[];
  /** 全局策略版本（与 A1 M5-2 卡 §4 对齐）。 */
  policy_version: string;
}

/** 注册表单条目的可序列化视图（与后端 McpRegistryEntryView 对应）。 */
export interface McpRegistryEntry {
  capability: string;
  /** 该能力对应的核心内部 API（只读展示用，非凭据）。 */
  core_api: string;
  /** 是否触碰文件系统（须经路径根校验）。 */
  touches_fs: boolean;
  /** 是否回传 URL（须经脱敏）。 */
  returns_url: boolean;
}
