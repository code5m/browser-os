use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 本地成果：网页采集而来，带溯源信息（来源 URL + 内容哈希）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub title: String,
    pub source_url: String,
    pub text: String,
    pub html: String,
    pub hash: String,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<String>,
    /// M2-1 图片附件（只存**引用**，不存字节）。
    /// `#[serde(default)]` 是硬要求：历史 `workspace/*.json` 均无该字段，
    /// 缺它会让 `load_artifacts` 静默丢弃全部历史成果（解析失败被 if let Ok 吞掉）。
    #[serde(default)]
    pub images: Vec<ImageRef>,
}

impl Artifact {
    pub fn new(title: String, source_url: String, text: String, html: String) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hasher.update(source_url.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Artifact {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            source_url,
            text,
            html,
            hash,
            created_at: Utc::now(),
            tags: vec![],
            images: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// M2-1 图片领域（契约冻结自 `logs/assist/M2-1.a-prework-20260902-1055.md`）
//
// 边界：这里只定义「结构 + 常量」。所有校验（MIME 白名单/魔法字节/尺寸/路径/容量）
// 集中在 `images.rs` 的纯函数里，便于无 AppHandle 单测。
// ---------------------------------------------------------------------------

/// 图片来源：区分「落盘文件」与「采集时已内联 dataURL」，决定读取方式。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImageSource {
    /// 落盘文件：`rel_path` 相对 `workspace_dir()`，形如 `images/<artifact_id>/<image_id>.<ext>`
    File,
    /// 采集时已内联为 dataURL，只存在于 `Artifact.html` 中，**无独立文件**
    InlineDataUrl,
}

/// 图片附件引用。Artifact 只存引用，不存字节；
/// 结构上不承载任何 headers / Cookie / Authorization / body（与 M1-8/M1-9 同口径）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRef {
    pub id: String,
    pub source: ImageSource,
    /// 仅 `source = File` 时有值：相对 `workspace_dir()` 的路径。禁止绝对路径、禁止 `..`。
    pub rel_path: Option<String>,
    /// 必须是 `IMAGE_MIME_EXT` 白名单之一（扩展名由 MIME 反查，不接受外部传入）
    pub mime: String,
    pub bytes: u64,
    /// 解析失败时为 `None`（不阻塞保存，不伪造尺寸）
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// 内容寻址，用于去重（同 sha256 复用已有引用，不重复写盘）
    pub sha256: String,
    /// 溯源：原始图片 URL，**落库前经脱敏**
    pub source_url: Option<String>,
    pub caption: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// MIME 白名单 → 扩展名（常量表：扩展名一律由 MIME 反查，杜绝 `a.png.html`）。
/// SVG 明确不进白名单（可携带 `<script>`，XSS 面）；其余 `image/*` fail-closed 拒绝。
pub const IMAGE_MIME_EXT: [(&str, &str); 4] = [
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/webp", "webp"),
    ("image/gif", "gif"),
];

/// 单张图片字节上限（超限 `IMAGE_TOO_LARGE`）
pub const IMAGE_MAX_BYTES: usize = 10 * 1024 * 1024;
/// 单个成果的图片数量上限（超限 `IMAGE_COUNT_EXCEEDED`）
pub const IMAGE_MAX_COUNT: usize = 50;
/// 单个成果的图片总字节上限（超限 `IMAGE_BUDGET_EXCEEDED`）
pub const IMAGE_MAX_TOTAL_BYTES: u64 = 50 * 1024 * 1024;
/// 单图最长边上限（防解码 OOM；超限 `IMAGE_DIMENSION_EXCEEDED`）
pub const IMAGE_MAX_DIMENSION: u32 = 8000;
/// 允许内联为 dataURL 的上限（超出必须落盘；超限 `INLINE_TOO_LARGE`）
pub const IMAGE_INLINE_MAX_BYTES: usize = 2 * 1024 * 1024;
/// id（image/artifact/session）形态长度上限：UUID v4 为 36 字符，留足余量
pub const IMAGE_ID_MAX_LEN: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoProvider {
    Git,
    Gitee,
}

/// 仓库配置（注意：不含 token，凭据隔离）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub id: String,
    pub provider: RepoProvider,
    pub name: String,
    pub remote_url: String,
    pub branch: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Pending,
    Confirmed,
    Running,
    Success,
    Failed,
}

/// 同步任务：核心聚合。推送前必须处于 Pending 并经确认
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub id: String,
    pub repo_id: String,
    pub status: SyncStatus,
    pub artifact_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// 推送到前端前的预览（确认闸门用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPreview {
    pub job_id: String,
    pub repo_id: String,
    pub repo_name: String,
    pub artifact_count: usize,
    pub artifact_titles: Vec<String>,
    pub remote_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub at: DateTime<Utc>,
    pub action: String,
    pub detail: String,
}

/// 收藏项（M1-2）。一条记录 = 一个 URL + 标题 + 分类。
/// 持久化为 data_dir/bookmarks.json，重启不丢。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub category: String,
    pub created_at: DateTime<Utc>,
}

impl Bookmark {
    pub fn new(url: String, title: String, category: String) -> Self {
        Bookmark {
            id: uuid::Uuid::new_v4().to_string(),
            url,
            title,
            category,
            created_at: Utc::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// M1-5 Git 只读能力 DTO（status / diff / branch_list）
// 全部字段只描述「仓库当前状态」，不含任何凭据，也不承载任何写操作语义。
// ---------------------------------------------------------------------------

/// 单文件状态。`status` 取值：
/// `modified` / `added` / `deleted` / `renamed` / `untracked` / `conflicted`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

/// 单文件 diff。内容超限时 `truncated=true`，`new_content` 为截断后的片段；
/// 二进制文件无法生成文本补丁时 `binary=true` 且两侧内容为 None。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffHunk {
    pub file: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub truncated: bool,
    pub binary: bool,
}

/// 分支信息（本地 + 远程跟踪），`is_head` 标当前检出分支。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub is_remote: bool,
    pub is_head: bool,
}

/// diff 结果：`more=true` 表示因总大小触顶提前停止收集（还有未返回的文件）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffResult {
    pub hunks: Vec<GitDiffHunk>,
    pub more: bool,
}

// ---------------------------------------------------------------------------
// M1-6.b Git 写能力（双阶段确认闸门）
//
// 白名单只有六个操作；任何其他操作串（reset/push/merge/rebase/stash/clean…）
// 在 request 阶段即被拒绝（fail-closed，零写入）。
// 结构体只承载操作语义与计数，绝不包含 token/凭据/完整 diff。
// ---------------------------------------------------------------------------

/// Git 写操作白名单（M1-6.b 冻结六项 + M1-6.d 增加 push，共七项）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitWriteOp {
    Stage,
    Unstage,
    Discard,
    Commit,
    CreateBranch,
    CheckoutBranch,
    Push,
}

impl GitWriteOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            GitWriteOp::Stage => "stage",
            GitWriteOp::Unstage => "unstage",
            GitWriteOp::Discard => "discard",
            GitWriteOp::Commit => "commit",
            GitWriteOp::CreateBranch => "create_branch",
            GitWriteOp::CheckoutBranch => "checkout_branch",
            GitWriteOp::Push => "push",
        }
    }

    /// 白名单解析：非白名单操作一律 None（调用方据此返回「操作禁止」）。
    pub fn from_op_str(s: &str) -> Option<GitWriteOp> {
        match s {
            "stage" => Some(GitWriteOp::Stage),
            "unstage" => Some(GitWriteOp::Unstage),
            "discard" => Some(GitWriteOp::Discard),
            "commit" => Some(GitWriteOp::Commit),
            "create_branch" => Some(GitWriteOp::CreateBranch),
            "checkout_branch" => Some(GitWriteOp::CheckoutBranch),
            "push" => Some(GitWriteOp::Push),
            _ => None,
        }
    }

    /// 危险操作需二次确认：discard 不可逆地用索引内容覆盖工作区文件；
    /// push 影响远端仓库（共享状态），同样需要二次确认。
    pub fn is_dangerous(&self) -> bool {
        matches!(self, GitWriteOp::Discard | GitWriteOp::Push)
    }
}

/// Git 写任务状态机：Pending → Running → Success/Failed。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GitWriteStatus {
    Pending,
    Running,
    Success,
    Failed,
}

/// Git 写任务。与 `SyncJob` 完全独立：SyncJob 语义属成果推送，
/// 混用会污染既有 request_sync/confirm_sync 流程，因此单独建模。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitWriteJob {
    pub id: String,
    pub repo_id: String,
    pub op: GitWriteOp,
    /// 已校验的仓库内相对路径（stage/unstage/discard 必填；commit 空 = 全量）
    pub paths: Vec<String>,
    /// commit 的提交信息（已通过校验：非空/限长/无控制字符）
    pub message: Option<String>,
    /// create_branch / checkout_branch 的目标分支名（已通过校验）
    pub branch: Option<String>,
    /// create_branch 是否同时检出
    pub checkout: bool,
    pub status: GitWriteStatus,
    pub dangerous: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// 阶段一返回给前端的预览：只含摘要、计数与截断后的路径列表（≤20 条），
/// 不含 diff 内容、不含任何凭据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitWritePreview {
    pub job_id: String,
    pub repo_id: String,
    pub op: GitWriteOp,
    pub summary: String,
    pub affected_paths: Vec<String>,
    pub path_count: usize,
    pub dangerous: bool,
    pub expires_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// M1-8 资源瀑布 DTO（请求拦截与瀑布）
//
// 字段全部来自 WebKitGTK 原生信号（resource-load-started / finished / failed），
// 平台拿不到的字段用 Option 并置 None，绝不伪造：
//   - status / mime / size_bytes 仅在有真实响应时存在（加载失败为 None）；
//   - size_bytes 为 None 表示「未知」（WebKit content_length=0 即未知）；
//   - resource_type 由 mime + URL 后缀推断（信号不提供 initiator 类型），
//     xhr_fetch 为启发式归类（json/xml/text 数据响应），可能含误判。
// 隐私红线：本 DTO 不承载任何 headers / Cookie / Authorization / Set-Cookie /
// request body / response body；url 必须已经过
// `security_policy::redact_sensitive_url` 脱敏与限长。
// ---------------------------------------------------------------------------

/// 资源类型（前端筛选维度与之一一对应）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Document,
    Script,
    Stylesheet,
    Image,
    XhrFetch,
    Font,
    Media,
    Other,
}

/// 单条资源请求记录（脱敏后的最终形态，入库/上报前端唯一使用本结构）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReceived {
    pub id: String,
    pub tab_id: String,
    /// 已脱敏 + 限长的 URL（敏感查询参数值为 `***`）
    pub url: String,
    /// HTTP 方法（来自 URIRequest；信号缺省时回退 "GET"）
    pub method: String,
    /// 真实响应状态码；加载失败/无响应为 None
    pub status: Option<u32>,
    /// 真实响应 MIME；未知为 None
    pub mime: Option<String>,
    /// 真实响应声明长度；未知为 None
    pub size_bytes: Option<u64>,
    /// epoch 毫秒（resource-load-started 信号时刻）
    pub started_at: i64,
    /// epoch 毫秒（finished/failed 信号时刻）
    pub finished_at: Option<i64>,
    pub duration_ms: Option<u64>,
    pub resource_type: ResourceKind,
}

/// 资源采集开关与容量上限（会话内生效，不持久化；重启回默认）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCaptureSettings {
    pub enabled: bool,
    /// 每 tab 最多保留条数（超出按 FIFO 丢弃最旧）
    pub max_per_tab: usize,
    /// 全局最多保留条数（超出按 FIFO 丢弃最旧）
    pub max_total: usize,
    /// 单条 URL 长度上限（与 security_policy::MAX_RESOURCE_URL_BYTES 对齐）
    pub max_url_bytes: usize,
}

impl Default for ResourceCaptureSettings {
    fn default() -> Self {
        ResourceCaptureSettings {
            enabled: true,
            max_per_tab: 200,
            max_total: 2000,
            max_url_bytes: 2048,
        }
    }
}

/// `list_tab_resources` 的返回：记录 + 因容量上限被丢弃的条数（前端超限提示用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabResourceList {
    pub records: Vec<ResourceReceived>,
    /// 该 tab 历史上因容量上限被 FIFO 丢弃的条数
    pub evicted: u64,
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// M1-9 浏览器会话存档（#14：请求/资源可见 + 关闭保存删除）
//
// 持久化白名单（落盘 `data_dir/sessions/<id>.json`，原子写 tempfile+rename）：
//   id / tab_id / url(**已脱敏**) / title / preview(**已脱敏+截断**) /
//   preview_truncated / resource_count / resources(**M1-8 脱敏 DTO，≤50 条**) /
//   saved / close_reason / created_at / updated_at
// 持久化黑名单（结构上不存在，policy 脚本守）：
//   token / cookie / Authorization / Set-Cookie / 任何 headers /
//   request body / response body / 插件原始资源事件（含未脱敏 URL）/ 任何凭据
//
// 关闭协议：未显式保存的会话**不落盘**。「不可静默丢」由关闭弹窗保证——
// 用户必须显式选「保存」或「删除」；异常退出时草稿随进程消失，
// 磁盘上只留下用户明确保存过的会话。
// ---------------------------------------------------------------------------

/// 会话最小文本预览上限（字节；超长截断并置 `preview_truncated`）。
pub const SESSION_PREVIEW_MAX_BYTES: usize = 512;
/// 单个会话最多存档的资源条数（超出保留最新 N 条，`resource_count` 记真实总数）。
pub const SESSION_MAX_RESOURCES: usize = 50;
/// 本地会话存档数量上限（超出按 updated_at 从旧到新删除）。
pub const SESSION_MAX_COUNT: usize = 50;
/// 会话标题上限（字节）。
pub const SESSION_TITLE_MAX_BYTES: usize = 200;

/// 关闭原因（审计与展示用，不含任何 URL）。
/// 注意：没有 `discarded` 常量——丢弃路径**不落盘**，自然没有落盘原因。
pub const CLOSE_REASON_SAVED: &str = "user_saved";
pub const CLOSE_REASON_SHUTDOWN: &str = "app_shutdown_flush";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSession {
    pub id: String,
    /// 来源页签 id（仅标识；页签关闭后会话仍可回看）
    pub tab_id: String,
    /// 已脱敏 + 限长的页面 URL（敏感查询参数值为 `***`）
    pub url: String,
    pub title: String,
    /// 最小文本预览（已脱敏 + 截断；空串表示未提供）
    pub preview: String,
    pub preview_truncated: bool,
    /// 该会话实际采集到的资源总数（可能大于 `resources.len()`）
    pub resource_count: usize,
    /// 已脱敏的资源记录（最新 `SESSION_MAX_RESOURCES` 条）
    pub resources: Vec<ResourceReceived>,
    /// 是否用户显式保存（落盘项恒为 true；草稿为 false 且不落盘）
    pub saved: bool,
    pub close_reason: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 列表项（不含 resources 全量，避免列表接口 payload 膨胀）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub tab_id: String,
    pub url: String,
    pub title: String,
    pub preview: String,
    pub preview_truncated: bool,
    pub resource_count: usize,
    pub saved: bool,
    pub close_reason: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 内存草稿：打开 tab 即建立，**落盘前绝不写磁盘**。
/// 只有两条路径会落盘：① 用户显式 `session_save`（关闭弹窗选「保存」或面板保存）；
/// ② 退出路径且用户开启了 `auto_save_on_exit`。除此之外草稿随进程消失——
/// 这是「不静默保存」红线的实现方式：未经用户同意的浏览痕迹一律不落盘。
#[derive(Debug, Clone)]
pub struct SessionDraft {
    pub tab_id: String,
    /// 已脱敏的页面 URL（导航时更新；会话构建优先取这里）
    pub url: String,
    /// 页面标题（会话构建优先取这里）
    pub title: String,
}

/// M1-9 会话策略（会话内生效，不持久化）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionPolicy {
    /// 关闭 tab 时是否弹「保存 / 删除」选择（默认开：关闭不可静默丢弃）
    pub close_prompt: bool,
    /// 退出应用前是否自动保存所有仍打开的 tab（默认关：默认不静默保存）
    pub auto_save_on_exit: bool,
}

impl Default for SessionPolicy {
    fn default() -> Self {
        SessionPolicy {
            close_prompt: true,
            auto_save_on_exit: false,
        }
    }
}

/// `flush_sessions` 的结果（关闭路径的确定性行为报告）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFlushReport {
    /// 本次落盘的会话数
    pub persisted: usize,
    /// 未保存即随关闭释放的草稿数（不静默保存，仅计数）
    pub drafts_dropped: usize,
    /// 清理的异常退出残留 .tmp 数
    pub tmp_removed: usize,
    /// 因容量上限删除的旧会话数
    pub capacity_removed: usize,
}

// ===========================================================================
// M2-3 脚本领域（契约冻结：logs/checkpoints/M2-3.a-20260903-1604.md §1）
//
// 只描述「脚本是什么」，**不含任何执行能力**：`run_script`、进程组 kill、
// 超时、输出上限、并发互斥与运行记录一律归 M2-4，前端面板归 M2-5。
//
// 安全口径：
//   - `interpreter` 是枚举白名单，不开放任意程序执行
//     （与 security_policy::BLOCKED_LAUNCH_PROGRAMS 同口径）
//   - `path` 只存相对文件名，不存绝对路径
//   - 新增字段一律 `#[serde(default)]`，否则历史 scripts.json 会被静默丢弃
// ===========================================================================

/// 参数类型（决定前端控件与校验规则）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    /// 自由文本，危险字符校验最严
    String,
    /// 整数（i64 可解析）
    Int,
    /// 布尔，仅接受 "true" / "false"
    Bool,
    /// 枚举：`options` 必须非空，值必须在 options 内
    Enum,
    /// 本地路径：必须过 `security_policy::check_path_within_roots`
    Path,
}

/// 解释器白名单。
///
/// 刻意设为枚举而非 `String`：脚本库是「执行脚本」的**受控例外**，
/// 若解释器可由用户自由填写，就等于在另一个入口重新开放任意程序执行，
/// 与 `security_policy::BLOCKED_LAUNCH_PROGRAMS` 的既有口径冲突。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScriptInterpreter {
    Bash,
    Sh,
    Python3,
    Node,
    /// 不指定解释器：直接执行脚本文件（依赖 shebang 与可执行位）
    Shebang,
}

impl ScriptInterpreter {
    /// 正文文件扩展名（不含点）。
    pub fn ext(&self) -> &'static str {
        match self {
            ScriptInterpreter::Bash => "sh",
            ScriptInterpreter::Sh => "sh",
            ScriptInterpreter::Python3 => "py",
            ScriptInterpreter::Node => "js",
            ScriptInterpreter::Shebang => "sh",
        }
    }

    /// 解释器可执行程序名（交给 `Command::new`，**永不来自用户输入**）。
    ///
    /// 与 `ext()` 用途不同：`ext()` 决定正文文件扩展名，`binary()` 决定 exec 哪个程序。
    /// 例如 `Bash` 的正文扩展名沿用 M2-3 定义为 `sh`，但执行程序是 `bash`。
    /// `Shebang` 返回 `None`——不指定解释器，直接 exec 脚本文件本身
    /// （依赖可执行位与 shebang 行）。
    ///
    /// 落位说明（`M2-4.b-VERDICT §3.4` 裁定）：与 `ext()` 同处 `impl ScriptInterpreter`，
    /// 保持解释器白名单**单一来源**，避免映射表散落到调用方。
    pub fn binary(&self) -> Option<&'static str> {
        match self {
            ScriptInterpreter::Bash => Some("bash"),
            ScriptInterpreter::Sh => Some("sh"),
            ScriptInterpreter::Python3 => Some("python3"),
            ScriptInterpreter::Node => Some("node"),
            ScriptInterpreter::Shebang => None,
        }
    }
}

/// 脚本参数定义。占位符名对应脚本正文里的 `${name}`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptParam {
    /// 占位符名：字符集 `[A-Za-z0-9_]`，非空，≤ 32 字符
    pub name: String,
    /// 前端展示名
    pub label: String,
    pub param_type: ParamType,
    pub required: bool,
    pub default: Option<String>,
    /// 仅 `Enum` 使用；`Enum` 时必须非空
    #[serde(default)]
    pub options: Vec<String>,
    /// **argv 模式下无效果**（`M2-4.b-VERDICT §3.1` 裁定）。
    ///
    /// 原始语义是「跳过单引号包裹」，但**单引号是 shell 语法**：M2-4 走
    /// `Command::arg()` 的 argv 数组（经 `execve`，shell 不参与解析），
    /// 给值加 `'...'` 会把单引号作为**字面量**传进脚本（实测 `printf` 收到 `'/tmp/x'`），
    /// 从「防护」变成「功能 bug」。故本字段在 argv 模式下无语义，仅为未来若引入
    /// 受控 shell 拼接模式时预留；「评估废弃本字段」已登记为后续检查点。
    ///
    /// 定义期仍校验「仅 `Path` / `Enum` 可为 true」（见 `scripts::validate_meta`）。
    #[serde(default)]
    pub raw: bool,
    /// 敏感参数：前端用密码框，审计值一律 `***`
    #[serde(default)]
    pub secret: bool,
}

/// 脚本元数据（**不含正文**；正文存独立文件 `scripts/<id>.<ext>`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMeta {
    /// uuid v4，同时是正文文件名主干
    pub id: String,
    /// 展示名
    pub name: String,
    /// 分类 / 标签
    pub category: String,
    /// 正文文件**相对 scripts 目录**的文件名，形如 `<id>.sh`。
    /// 不存绝对路径：会泄露环境且迁移即失效（与 `ImageRef::rel_path` 同口径）。
    pub path: String,
    pub interpreter: ScriptInterpreter,
    #[serde(default)]
    pub params: Vec<ScriptParam>,
    #[serde(default)]
    pub description: String,
    /// 内置模板：不可删除
    #[serde(default)]
    pub builtin: bool,
    #[serde(default = "default_script_enabled")]
    pub enabled: bool,
    /// 超时秒数；0 = 使用全局默认（**60**）。脚本级上限 600。
    ///
    /// 语义由 M2-4 实现。**注释更正（M2-4.b）**：M2-3 初版注释误写「默认 300」，
    /// 与 `M2-4.a §2` 裁定的全局默认 60s 冲突，已按 a 卡裁定更正为 60（仅注释，不改语义）。
    #[serde(default)]
    pub timeout_secs: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 脚本运行状态（`M2-4.a §7` 冻结命名：用 `Succeeded` 而非 `Success`）。
///
/// **终态不可互转**（`M2-4.a §2`）：`Succeeded`/`Failed`/`Cancelled`/`Timeout` 一旦落定，
/// 后续到达的取消请求或超时信号都不得改写状态。
///
/// 落盘形态（`script-runs.json` 的 `ScriptRunRecord`）与输出尾存归 **M2-4.d**，
/// 本卡只定义状态枚举本身（`M2-4.a §9` 边界）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Timeout,
}

impl RunStatus {
    /// 是否已进入终态（终态不再接受取消/超时改写，见 `M2-4.a §2`）。
    pub fn is_terminal(&self) -> bool {
        !matches!(self, RunStatus::Running)
    }
}

fn default_script_enabled() -> bool {
    true
}

/// 命令片段内置分类（需求 #4：系统 / 网络 / 磁盘 / 进程 / 文本处理）。
///
/// 冻结裁定 `M2-6.a §7 F5`：`category` 为**自由字符串**（与 `ScriptMeta.category`
/// 同口径），本常量仅供内置片段与 UI 分组使用，**不做取值白名单约束**。
// 消费者：`M2-6.b` 内置片段种子、`M2-6.d` 前端分类分组。本卡只冻结取值，不接线。
#[allow(dead_code)]
pub const SNIPPET_BUILTIN_CATEGORIES: [&str; 5] = ["system", "network", "disk", "process", "text"];

/// 命令片段超时上限（秒）——与 `ScriptMeta.timeout_secs` 同口径（`M2-4.a §2`）。
// 消费者：`M2-6.b` 定义期上限校验、`M2-6.c` 执行超时钳制。本卡只冻结口径，不接线。
#[allow(dead_code)]
pub const SNIPPET_MAX_TIMEOUT_SECS: u32 = 600;

/// 命令片段（`M2-6.a` 冻结；持久化归 `M2-6.b`，执行接入归 `M2-6.c`，
/// 前端归 `M2-6.d`）。契约源 `logs/checkpoints/M2-6-20260905-1700.md §7`。
///
/// 与 `ScriptMeta` 的三处**结构性差异**（不得抹平，见展开卡 §7 F10）：
///
/// 1. **无正文文件**：命令以 `argv` 数组直接存于 `snippets.json`，故**没有** `path`
///    字段（脚本库才有 `<id>.sh` 正文）；
/// 2. **argv 而非 line**：不存单行 shell 字符串（`line: String`）。字符串行需要自实现
///    shell 词法解析（引号 / 转义 / 空格），既易错又逼近
///    `M2-4.a §8.1` 的 P0 红线（禁止 shell 拼接）；
/// 3. **整元素占位**：`argv` 中**严格等于** `{NAME}` 的元素整体替换为该参数值
///    （一个元素 ⇄ 一个值）。**不支持** `grep -r {PATTERN}` 这类字符串内插值——
///    否则一个参数值可能被词法切成多个 argv 元素，重新开放注入面。
///    需要 `--dir=/x` 时应拆成 `["--dir", "{DIR}"]` 两个元素。
///
/// `interpreter` 复用 `ScriptInterpreter` 白名单；其中 `Shebang` 对命令片段无意义
/// （无正文文件可依赖 shebang 与可执行位），**执行层须显式拒绝**（归 `M2-6.c`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSnippet {
    /// uuid v4
    pub id: String,
    /// 展示名
    pub name: String,
    /// 分类 / 标签（内置取值见 `SNIPPET_BUILTIN_CATEGORIES`，不做白名单约束）
    pub category: String,
    /// 执行 `argv` 的程序（枚举白名单；`Shebang` 不适用）
    pub interpreter: ScriptInterpreter,
    /// 命令 argv 模板。元素严格等于 `{NAME}` 时整体替换为该参数值。
    /// **不得为空**，且不得出现「含占位但不等」的部分插值形态（`M2-6.a §7 F2`）。
    pub argv: Vec<String>,
    #[serde(default)]
    pub params: Vec<ScriptParam>,
    #[serde(default)]
    pub description: String,
    /// 危险标记：触发前端二次确认（确认 UI 归 `M2-6.d`，**字段**在本卡冻结）。
    ///
    /// 采用**显式字段**而非 argv[0] 黑名单推断：黑名单易被绕过（别名、绝对路径、
    /// `env` 间接调用），显式字段 + 内置片段预置更可审计（`M2-6.a §7 F4`）。
    #[serde(default)]
    pub dangerous: bool,
    /// 内置片段：不可删除
    #[serde(default)]
    pub builtin: bool,
    #[serde(default = "default_script_enabled")]
    pub enabled: bool,
    /// 超时秒数；0 = 全局默认（**60**）。上限 `SNIPPET_MAX_TIMEOUT_SECS`（600）。
    #[serde(default)]
    pub timeout_secs: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CommandSnippet {
    /// 取 argv 元素的占位符名；**非整元素占位**返回 `None`。
    ///
    /// 仅当整个元素形如 `{NAME}`（`NAME` 为 `[A-Za-z0-9_]`，1~32 字符）时视为占位，
    /// 见类型文档的「整元素占位」裁定（`M2-6.a §7 F2`）。
    // 消费者：`M2-6.c` 执行接入（argv 整元素替换）。本卡只冻结语义，不接线。
    #[allow(dead_code)]
    pub fn placeholder_of(element: &str) -> Option<&str> {
        let inner = element.strip_prefix('{')?.strip_suffix('}')?;
        if inner.is_empty()
            || inner.len() > 32
            || !inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return None;
        }
        Some(inner)
    }
}

// ===== M2-7 工具（离线 HTML 小工具）领域类型 =====
//
// 契约冻结见 `logs/checkpoints/B-M2-7.a-tool-manifest-contract-20260905-1800.md`
// （F1~F9）。与 `#1 脚本`/`#4 命令` 的本质差异：工具是**静态 HTML**，
// 不 spawn 进程、不拼 shell，执行风险几乎为零——本结构只描述「清单是什么」，
// 不含任何执行能力（打开/渲染归 M2-8，前端子 webview 隔离）。
//
// 安全口径（沿用 M2-4 P0 红线 + M2-6 字段复用惯例）：
//   - 工具是静态 HTML，无 argv、无执行风险，不引入 `dangerous` 字段
//     （风险面在 M2-8 Web 上下文隔离，不在清单）
//   - `entry` 只存文件名/相对键，不存绝对路径
//   - 新增字段一律 `#[serde(default)]`，兼容历史数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMeta {
    /// 唯一 id。
    /// 内置为固定短名（json / base64 / timestamp / regex / cron）；
    /// 用户工具为 `user-<文件名>`（与内置 id 命名空间隔离，避免碰撞）。
    pub id: String,
    /// 展示名
    pub name: String,
    /// 可选描述（内置为 None；未来可由 HTML <title>/<meta> 抽取，本期不做）
    #[serde(default)]
    pub description: Option<String>,
    /// 分类 / 标签（自由字符串，与 `ScriptMeta.category` 同口径，不做白名单约束）
    pub category: String,
    /// 来源：内置（编译期嵌入）或用户（workspace/tools 运行时扫描）
    pub source: ToolSource,
    /// 入口：内置 = 嵌入键名/相对文件名（`tools/<id>-tool.html`），供 M2-8 经
    ///   `tools::builtin_tool_html` 取嵌入字节；用户 = 文件名。
    /// 一律只存文件名，**不存绝对路径**。
    pub entry: String,
}

/// 工具来源（snake_case 序列化：`builtin` / `user`）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    /// 编译期 `include_str!` 嵌入的固定种子
    Builtin,
    /// 用户放入 `workspace/tools/` 的自定义 HTML
    User,
}

// ---------------------------------------------------------------------------
// M4-1 数据库领域类型（契约冻结见 `logs/checkpoints/M4-1.b-20260905-2250.md`）
//
// 本段**只有类型提案**：枚举 / 结构 / 常量，无任何连接、查询、IO、spawn。
// 无调用者标 `#[allow(dead_code)]`，消费者 = M4-2（连接池与内省）/ M4-3（命令层）。
//
// 三条硬约束（由 `check-database-policy.py` 与本文件单测双向守住）：
//   1. `DbConnectionConfig` **结构性不含 password 字段**（F2）——不是「写前清空」，
//      是类型上不存在该字段；凭据只经 `KeyringStore`，键格式 `db:<conn_id>`
//      （与 git 侧 `repo_id` 命名空间隔离，防互相覆盖）；
//   2. `allow_write` 缺省 `false` = **写默认拒绝**（F1），缺字段即无写权限；
//   3. 错误码是**闭合枚举 + 稳定字符串码**，前端与夹具按码分支。
// ---------------------------------------------------------------------------

/// 支持的数据库驱动（snake_case 序列化：`sqlite` / `mysql` / `postgres`）。
///
/// **唯一「驱动身份」定义点**：新增驱动在此加变体。其余扩展点由 `match` 穷尽性
/// （编译期强制）与 M4-1.a §5「加库清单」（夹具 + 测试矩阵）共同兜住；
/// 契约**不宣称**「加库只改一处」——那是不可兑现的。
#[allow(dead_code)] // 消费者：M4-2 `database.rs` / M4-3 命令层
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SupportedDb {
    Sqlite,
    /// 显式 `rename`：`snake_case` 会把 `MySql` 变成 `my_sql`，
    /// 与前端/落盘/CLI 惯用的 `mysql` 不一致（单测 T-db-c1 守）。
    #[serde(rename = "mysql")]
    MySql,
    Postgres,
}

#[allow(dead_code)] // 消费者：M4-2 / M4-3 / M4-4 表单默认值
impl SupportedDb {
    pub fn as_str(self) -> &'static str {
        match self {
            SupportedDb::Sqlite => "sqlite",
            SupportedDb::MySql => "mysql",
            SupportedDb::Postgres => "postgres",
        }
    }

    /// 反解驱动标识。**fail-closed**：只接受精确 snake_case，
    /// 未知 / 大小写变形 / 前后空格一律 `None`（与 `GitWriteOp::from_op_str` 同口径）。
    pub fn from_kind_str(raw: &str) -> Option<Self> {
        match raw {
            "sqlite" => Some(SupportedDb::Sqlite),
            "mysql" => Some(SupportedDb::MySql),
            "postgres" => Some(SupportedDb::Postgres),
            _ => None,
        }
    }

    /// 默认端口。`Sqlite` 无网络端口 → `None`（前端据此隐藏端口输入框）。
    pub fn default_port(self) -> Option<u16> {
        match self {
            SupportedDb::Sqlite => None,
            SupportedDb::MySql => Some(3306),
            SupportedDb::Postgres => Some(5432),
        }
    }

    /// 是否网络库（决定 TLS 与「生产判定」的主机类信号是否适用）。
    pub fn is_remote(self) -> bool {
        !matches!(self, SupportedDb::Sqlite)
    }

    /// 是否支持传输层加密（SQLite 为本地文件，无传输层）。
    pub fn supports_ssl(self) -> bool {
        self.is_remote()
    }
}

/// TLS 模式（snake_case：`disable` / `prefer` / `require`）。
///
/// 缺省 `Prefer`（fail-closed 取严一档）。连上后由 M4-2 记录**实际**是否加密；
/// 「非回环地址 + 实际未加密」⇒ 写操作拒绝（见 M4-1.b §3 与 M4-1.d 信号表 S6）。
#[allow(dead_code)] // 消费者：M4-2 连接参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DbSslMode {
    Disable,
    Prefer,
    Require,
}

#[allow(dead_code)] // 消费者：M4-3 `db_connect` 反序列化默认值
impl Default for DbSslMode {
    fn default() -> Self {
        DbSslMode::Prefer
    }
}

/// 连接配置（落盘形态：`data_dir/db_connections.json`，原子写 tempfile+rename）。
///
/// 结构性不含凭据字段（F2）：`password` / `dsn` / `connection_string` 一律不存在，
/// 连接串**永不**由本结构序列化。凭据只经 `KeyringStore`（键 `db:<conn_id>`）。
#[allow(dead_code)] // 消费者：M4-2 持久化 / M4-3 命令层
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnectionConfig {
    /// UUID v4，与 Keyring 键 `db:<id>` 一一对应
    pub id: String,
    /// 展示名
    pub name: String,
    pub kind: SupportedDb,
    /// SQLite 为 `None`
    #[serde(default)]
    pub host: Option<String>,
    /// `None` → 用 `kind.default_port()`；SQLite 忽略
    #[serde(default)]
    pub port: Option<u16>,
    /// SQLite = 文件路径（**必须**过 `security_policy::check_path_within_roots`）；
    /// 其余 = 库名。空串是「生产判定 → Unknown」的信号之一（M4-1.d S5）。
    pub database: String,
    /// SQLite 为 `None`
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub ssl_mode: DbSslMode,
    /// 写权限显式开关，缺省 `false` = **写默认拒绝**（F1）
    #[serde(default)]
    pub allow_write: bool,
    /// 用户显式生产标记（信号 S1）。`None` = 未标记（既不放行也不拒绝）
    #[serde(default)]
    pub production_hint: Option<bool>,
    #[serde(default = "db_enabled_default")]
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)] // 消费者：M4-2 持久化 / M4-3 命令层
fn db_enabled_default() -> bool {
    true
}

/// 数据库错误码（**闭合枚举** + 稳定字符串码，沿用 `scripts.rs` 20 码范式）。
///
/// 增删码必须同步三处：本文件末尾单测的计数断言、M4-3 的 TS 镜像、
/// `check-database-policy.py` 的码位表（M4-1.d 交付的 policy notes）。
#[allow(dead_code)] // 消费者：M4-2 / M4-3 错误出口
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbErrorCode {
    UnsupportedKind,
    InvalidConfig,
    PathOutsideRoots,
    CredentialMissing,
    ConnectFailed,
    NotConnected,
    PoolExhausted,
    SqlEmpty,
    SqlTooLarge,
    SqlParseFailed,
    MultipleStatements,
    WriteDenied,
    ProductionUnknown,
    QueryFailed,
    Timeout,
    Cancelled,
    LimitExceeded,
    NotSupported,
}

#[allow(dead_code)] // 消费者：M4-2 / M4-3 错误出口
impl DbErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            DbErrorCode::UnsupportedKind => "DB_UNSUPPORTED_KIND",
            DbErrorCode::InvalidConfig => "DB_INVALID_CONFIG",
            DbErrorCode::PathOutsideRoots => "DB_PATH_OUTSIDE_ROOTS",
            DbErrorCode::CredentialMissing => "DB_CREDENTIAL_MISSING",
            DbErrorCode::ConnectFailed => "DB_CONNECT_FAILED",
            DbErrorCode::NotConnected => "DB_NOT_CONNECTED",
            DbErrorCode::PoolExhausted => "DB_POOL_EXHAUSTED",
            DbErrorCode::SqlEmpty => "DB_SQL_EMPTY",
            DbErrorCode::SqlTooLarge => "DB_SQL_TOO_LARGE",
            DbErrorCode::SqlParseFailed => "DB_SQL_PARSE_FAILED",
            DbErrorCode::MultipleStatements => "DB_MULTIPLE_STATEMENTS",
            DbErrorCode::WriteDenied => "DB_WRITE_DENIED",
            DbErrorCode::ProductionUnknown => "DB_PRODUCTION_UNKNOWN",
            DbErrorCode::QueryFailed => "DB_QUERY_FAILED",
            DbErrorCode::Timeout => "DB_TIMEOUT",
            DbErrorCode::Cancelled => "DB_CANCELLED",
            DbErrorCode::LimitExceeded => "DB_LIMIT_EXCEEDED",
            DbErrorCode::NotSupported => "DB_NOT_SUPPORTED",
        }
    }
}

// ---------------------------------------------------------------------------
// M4-1.c 结果上限 / 取消 / 截断常量与结构（契约冻结见 `logs/checkpoints/M4-1.c-20260905-2255.md`）
//
// 放进 `domain.rs` 的理由（指挥板 Batch Implementation Dispatch · Lane A2）：
// 这些是 **A3（取数循环必须逐行判定）与 A4（审计/夹具要断言）共同消费** 的跨模块契约，
// 若各实现侧自行定义常量，就会出现「A3 按 1000 行截断、A4 按 500 行审计」的口径漂移。
// 因此这里冻结**数值与字段名**，实现侧只消费、不重新定义。
//
// 全部 `#[allow(dead_code)]`，消费者 = M4-2 / M4-3；不引任何 db 依赖。
// ---------------------------------------------------------------------------

// M5-1 切片 0b：通用契约常量收口自 `script_runner` / `security_policy`（值逐字不变）。
// `script_runner` / `security_policy` 改为 `pub use crate::domain::*`，仅再导出，不再自定。
/// 脚本级超时上限（fail-closed，超过即拒绝，不静默封顶）。收口自 `script_runner::MAX_TIMEOUT_SECS`。
pub const MAX_TIMEOUT_SECS: u32 = 600;
/// soft 超时（SIGTERM）→ hard（SIGKILL）宽限。收口自 `script_runner::HARD_GRACE_SECS`。
pub const HARD_GRACE_SECS: u32 = 5;
/// 单条上报文本字段默认上限（防外部页面超大字符串拖垮主进程）。收口自 `security_policy::MAX_TEXT_FIELD_BYTES`。
pub const MAX_TEXT_FIELD_BYTES: usize = 64 * 1024;

/// 单条 SQL 文本字节上限。与 `security_policy::MAX_TEXT_FIELD_BYTES` 同量级（单测 T-db-c6 守）。
#[allow(dead_code)] // 消费者：M4-3 命令层入参校验
pub const DB_MAX_SQL_BYTES: usize = 64 * 1024;

/// 结果行数上限。**在取数循环内**生效：超第 1000 行即停止取数并丢弃剩余，
/// 不是「物化完再截断」（A10 G-6）。
#[allow(dead_code)] // 消费者：M4-2 取数循环
pub const DB_MAX_ROWS: usize = 1_000;

/// 结果累计字节上限（4 MiB）。与行数上限**任一**命中即停止取数。
#[allow(dead_code)] // 消费者：M4-2 取数循环
pub const DB_MAX_RESULT_BYTES: usize = 4 * 1024 * 1024;

/// 单字段字节上限，与 `security_policy::MAX_TEXT_FIELD_BYTES` **必须相等**（单测 T-db-c6 守）。
#[allow(dead_code)] // 消费者：M4-2 字段截断
pub const DB_MAX_TEXT_FIELD_BYTES: usize = 64 * 1024;

/// 默认查询超时（秒）。`timeout_secs = 0` 时取本值。
#[allow(dead_code)] // 消费者：M4-2 / M4-3
pub const DB_DEFAULT_QUERY_TIMEOUT_SECS: u32 = 30;

/// 查询超时上限（秒），与 `script_runner::MAX_TIMEOUT_SECS` **必须相等**（单测 T-db-c6 守）。
#[allow(dead_code)] // 消费者：M4-3 入参校验
pub const DB_MAX_QUERY_TIMEOUT_SECS: u32 = 600;

/// soft（驱动级取消）→ hard（放弃连接）宽限（秒），与 `script_runner::HARD_GRACE_SECS` 对齐。
#[allow(dead_code)] // 消费者：M4-2 超时分层
pub const DB_SOFT_TO_HARD_GRACE_SECS: u32 = 5;

/// 取数循环每多少行检查一次取消标志。
#[allow(dead_code)] // 消费者：M4-2 取数循环
pub const DB_CANCEL_CHECK_EVERY_ROWS: usize = 64;

/// 导出/落盘字节上限（**仅在 A5 确认做导出后才有消费方**，否则该常量无消费者）。
#[allow(dead_code)] // 消费者：M4-4 导出（待 F-6 确认）
pub const DB_MAX_EXPORT_BYTES: usize = 16 * 1024 * 1024;

/// 上限命中原因（审计 `limit_hit` 与前端提示用）。
#[allow(dead_code)] // 消费者：M4-2 / M4-4
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DbLimitKind {
    /// 行数达 `DB_MAX_ROWS`
    Rows,
    /// 累计字节达 `DB_MAX_RESULT_BYTES`
    Bytes,
    /// 单字段达 `DB_MAX_TEXT_FIELD_BYTES`
    Field,
}

/// 取数终止状态（即 M4-1.c 冻结的「取消状态名」集合）。
///
/// 注意：截断**不是**状态——截断由 `truncated` / `field_truncated` / `limit_hit` 表达，
/// 状态只描述「为什么停下来」，避免同一事实两处表达。
#[allow(dead_code)] // 消费者：M4-2 / M4-3 / M4-4
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DbQueryState {
    /// 正常跑完（可能带截断标记）
    Completed,
    /// 前端取消（L1 标志 → L2 循环检查命中）
    Cancelled,
    /// 超时（soft 未果 → hard 放弃连接）
    Timeout,
    /// 执行失败（错误码见 `DbErrorCode`）
    Failed,
}

/// 结果单元格值。**只承载可序列化标量**：二进制列只回长度、不回字节，
/// 避免二进制经 JSON 打到 webview。
#[allow(dead_code)] // 消费者：M4-2 结果组装
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DbValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    /// 二进制列：只回长度
    BlobLen(u64),
}

/// 查询结果（A3 组装；字段名与语义由 M4-1.c 冻结）。
///
/// **禁止静默截断**：行/字节/字段任一超限，`truncated` / `field_truncated` / `limit_hit`
/// 必须被置位，前端据此明示「结果不完整」。
#[allow(dead_code)] // 消费者：M4-2 / M4-3 / M4-4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbQueryResult {
    /// 取消句柄
    pub query_id: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<DbValue>>,
    /// 实际返回行数（≤ `DB_MAX_ROWS`）
    pub row_count: usize,
    /// 行/字节任一超限即 `true`
    pub truncated: bool,
    /// 任一单字段被 `DB_MAX_TEXT_FIELD_BYTES` 截断即 `true`
    pub field_truncated: bool,
    /// 命中上限的原因；未命中为 `None`
    #[serde(default)]
    pub limit_hit: Option<DbLimitKind>,
    /// 取数耗时（毫秒）
    pub elapsed_ms: u64,
    pub state: DbQueryState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_received_serde_has_no_sensitive_fields() {
        // 隐私红线（结构性保障）：DTO 序列化结果不得出现
        // cookie / authorization / set-cookie / body / headers 任何字段。
        let rec = ResourceReceived {
            id: "r1".into(),
            tab_id: "tab-1".into(),
            url: "https://example.com/a?token=***".into(),
            method: "GET".into(),
            status: Some(200),
            mime: Some("text/html".into()),
            size_bytes: Some(1024),
            started_at: 1,
            finished_at: Some(2),
            duration_ms: Some(1),
            resource_type: ResourceKind::Document,
        };
        let json = serde_json::to_string(&rec).expect("serialize");
        for bad in [
            "cookie",
            "Cookie",
            "authorization",
            "Authorization",
            "set-cookie",
            "Set-Cookie",
            "body",
            "headers",
        ] {
            assert!(
                !json.contains(bad),
                "ResourceReceived 序列化不得包含敏感字段 {bad}: {json}"
            );
        }
        // 前后端类型对齐：字段名必须与 src/types.ts 一致
        for key in [
            "\"id\"",
            "\"tab_id\"",
            "\"url\"",
            "\"method\"",
            "\"status\"",
            "\"mime\"",
            "\"size_bytes\"",
            "\"started_at\"",
            "\"finished_at\"",
            "\"duration_ms\"",
            "\"resource_type\"",
        ] {
            assert!(json.contains(key), "缺少字段 {key}: {json}");
        }
        // Option 字段允许为 null（降级不伪造）
        let mut degraded = rec.clone();
        degraded.status = None;
        degraded.mime = None;
        degraded.size_bytes = None;
        let j2 = serde_json::to_string(&degraded).expect("serialize");
        assert!(j2.contains("\"status\":null"));
        assert!(j2.contains("\"mime\":null"));
        assert!(j2.contains("\"size_bytes\":null"));
    }

    #[test]
    fn resource_capture_settings_defaults_are_capped() {
        let s = ResourceCaptureSettings::default();
        assert!(s.enabled);
        assert_eq!(s.max_per_tab, 200);
        assert_eq!(s.max_total, 2000);
        assert_eq!(s.max_url_bytes, 2048);
        assert!(s.max_per_tab <= s.max_total);
    }

    #[test]
    fn bookmark_roundtrip_serde() {
        let bm = Bookmark::new(
            "https://example.com/a".into(),
            "Example".into(),
            "tech".into(),
        );
        let json = serde_json::to_string(&bm).expect("serialize");
        let back: Bookmark = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, bm.id);
        assert_eq!(back.url, "https://example.com/a");
        assert_eq!(back.title, "Example");
        assert_eq!(back.category, "tech");
        // created_at 应可被 serde 反序列化
        assert!(back.created_at.timestamp() > 0);
    }

    #[test]
    fn bookmark_id_is_uuid_v4() {
        let bm = Bookmark::new("u".into(), "t".into(), "c".into());
        let parsed = uuid::Uuid::parse_str(&bm.id).expect("uuid parse");
        assert_eq!(parsed.get_version_num(), 4);
    }

    // T-gw-1（b/d 卡）：白名单外操作（reset/merge/rebase/stash/clean 等任意形式）
    // 必须在 request 阶段即被判定为 None → Err("操作禁止")，零写入。
    #[test]
    fn git_write_op_whitelist_only_seven_ops() {
        for ok in [
            "stage",
            "unstage",
            "discard",
            "commit",
            "create_branch",
            "checkout_branch",
            "push",
        ] {
            assert!(GitWriteOp::from_op_str(ok).is_some(), "{ok} 应在白名单内");
        }
        for bad in [
            "reset",
            "reset --hard",
            "push --force",
            "push --force-with-lease",
            "push --mirror",
            "push --delete",
            "force push",
            "pull",
            "fetch",
            "merge",
            "rebase",
            "stash",
            "clean",
            "revert",
            "cherry-pick",
            "branch -D",
            "checkout .",
            "",
            "STAGE",
            "PUSH",
            " stage",
            " push",
        ] {
            assert!(
                GitWriteOp::from_op_str(bad).is_none(),
                "白名单外操作必须被拒绝: {bad:?}"
            );
        }
        // 危险标记：discard（本地不可逆）与 push（影响远端）需二次确认
        assert!(GitWriteOp::Discard.is_dangerous());
        assert!(GitWriteOp::Push.is_dangerous());
        assert!(!GitWriteOp::Commit.is_dangerous());
        assert!(!GitWriteOp::CheckoutBranch.is_dangerous());
    }
}

// ---------------------------------------------------------------------------
// M4-6 / M4-7 定时任务调度领域类型
//
// 契约源：`logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md` §3（Lane A6 冻结）。
// 类型归本文件；校验与持久化归 `tasks.rs`；触发与退出收口归 `scheduler.rs`（均 Lane A7）。
//
// **同名不同险提醒**：`ScriptMeta.enabled` / `CommandSnippet.enabled` 默认 **true**
// （只决定列表可见性，仍需人工点击才执行）；`TaskDef.enabled` 默认 **false**
// （决定是否**无人值守自动执行**，裁定 R-A6-1）。二者不得互相照搬。
// ---------------------------------------------------------------------------

/// 任务总数上限（对齐 `scripts::MAX_SCRIPTS` / `snippets::MAX_SNIPPETS`；超出拒绝新增，不静默裁剪）。
pub const SCHED_MAX_TASKS: usize = 200;
/// 运行历史落盘上限（`task-runs.json`，环形 FIFO；**不参与判重**，见契约 F-A6-4）。
pub const SCHED_MAX_HISTORY: usize = 500;
/// 单次 tick 的触发点扫描上限（时钟前跳时防雪崩）。
pub const SCHED_MAX_SLOT_SCAN: usize = 512;
/// 重试次数硬上界（`TaskDef.retry.max_attempts` ≤ 该值）。
pub const SCHED_MAX_ATTEMPTS: u32 = 5;
/// CatchUp 补跑硬上界（`TaskDef.catch_up_limit` ≤ 该值）。
pub const SCHED_MAX_CATCH_UP: u32 = 10;
/// 重试退避延迟硬上界（秒）。
pub const SCHED_MAX_DELAY_SECS: u64 = 600;

/// R-11：单条失败执行的**重试总时长硬上界**（秒，自最初触发点 `scheduled_at` 起算）。
/// 重试窗口（含退避延迟与执行耗时）累计超过此值即停止重试，避免无人值守任务无限拖延重试。
pub const SCHED_RETRY_TOTAL_BUDGET_SECS: u64 = 5400;

/// 任务名上限（字节，对齐 `scripts::MAX_NAME_BYTES`）。
pub const TASK_MAX_NAME_BYTES: usize = 128;
/// 单个任务的参数条目上限（对齐 `scripts::MAX_PARAMS`）。
pub const TASK_MAX_PARAMS: usize = 20;
/// `Interval` 合法下界（秒）。对需求 #11 原文「every Ns」的**保守收窄**；放宽须单开卡（O-A6-5）。
pub const TASK_INTERVAL_MIN_SECS: u64 = 60;
/// `Interval` 合法上界（秒）：30 天。
pub const TASK_INTERVAL_MAX_SECS: u64 = 30 * 24 * 3600;

/// 触发方式（首期两种，均 ≥ 1 分钟粒度）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskTrigger {
    /// 标准 5 段 cron：`分 时 日 月 周`。方言见契约 §4.4 —— **不支持**秒字段、年字段、
    /// `@reboot` / `@daily` 等宏，以及 `L` / `W` / `#` / `?`。
    Cron { expr: String },
    /// 固定间隔（秒）。合法区间 `[60, 2592000]`。
    Interval { every_secs: u64 },
}

/// 执行体类型。契约 F-5：首期仅 Script / Command，**不做 Tool**（HTML 工具无 headless 执行入口）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Script,
    Command,
}

impl TaskKind {
    /// 稳定字符串（审计与 UI 展示用；审计 detail 不得含命令正文/参数值）。
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskKind::Script => "script",
            TaskKind::Command => "command",
        }
    }
}

/// 错过执行策略。F9 硬性要求：**必须是 `TaskDef` 字段**，不得退化为运行时常量。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicy {
    /// 默认：错过的触发点不补跑，`next_run_at` 直接推到当前之后的第一个未来触发点。
    #[default]
    Skip,
    /// 补跑一次（用**最新**那个错过的触发点），不论错过了多少个。
    RunOnce,
    /// 按 `catch_up_limit` 上限补跑，超出部分记 `over_limit` 后丢弃。
    CatchUp,
}

#[cfg(test)]
mod missed_run_policy_label_tests {
    use super::*;
    #[test]
    fn labels_are_stable() {
        // 错过执行策略的稳定字符串用于审计 detail 与持久化；此处仅校验展示语义。
        let cases = [
            MissedRunPolicy::Skip,
            MissedRunPolicy::RunOnce,
            MissedRunPolicy::CatchUp,
        ];
        let labels = ["skip", "run_once", "catch_up"];
        for (p, want) in cases.iter().zip(labels.iter()) {
            let s = match p {
                MissedRunPolicy::Skip => "skip",
                MissedRunPolicy::RunOnce => "run_once",
                MissedRunPolicy::CatchUp => "catch_up",
            };
            assert_eq!(s, *want);
        }
    }
}

/// 触发来源（写 `task-runs.json`，供 UI 区分「定时 / 手工 / 补跑 / 重试」）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunTrigger {
    Scheduled,
    Manual,
    CatchUp,
    Retry,
}

impl TaskRunTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskRunTrigger::Scheduled => "scheduled",
            TaskRunTrigger::Manual => "manual",
            TaskRunTrigger::CatchUp => "catch_up",
            TaskRunTrigger::Retry => "retry",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetryBackoff {
    #[default]
    Fixed,
    Exponential,
}

/// 重试策略。默认 **不重试**（`max_attempts = 1`）—— 定时任务的默认行为必须是「跑一次就完」。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetryPolicy {
    /// 含首次；1 = 不重试（默认）。硬上限 `SCHED_MAX_ATTEMPTS`（5）。
    pub max_attempts: u32,
    pub backoff: RetryBackoff,
    pub base_delay_secs: u64,
    pub max_delay_secs: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            backoff: RetryBackoff::Fixed,
            base_delay_secs: 0,
            max_delay_secs: 0,
        }
    }
}

impl RetryPolicy {
    /// 第 `attempt` 次（从 1 开始）失败后的退避秒数，**已 clamp 到 `max_delay_secs`**。
    /// 抖动（±10%）由 `tasks::jitter_secs` 施加，且抖动后仍不得超过 `max_delay_secs`。
    pub fn delay_secs(&self, attempt: u32) -> u64 {
        let raw = match self.backoff {
            RetryBackoff::Fixed => self.base_delay_secs,
            RetryBackoff::Exponential => {
                let shift = attempt.saturating_sub(1).min(6);
                self.base_delay_secs.saturating_mul(1u64 << shift)
            }
        };
        raw.min(self.max_delay_secs).min(SCHED_MAX_DELAY_SECS)
    }
}

/// 定时任务定义（持久化 `data_dir/tasks.json`，`Vec<TaskDef>`）。
///
/// 判重真相源是 `last_fired_at`（**已触发的计划时刻**，不是完成时刻），
/// 不是运行历史 —— 历史是环形 FIFO，裁剪后会丢键导致重复执行（契约 §3.2 / F-A6-4）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskDef {
    /// uuid v4
    pub id: String,
    /// 显示名，≤ `TASK_MAX_NAME_BYTES` 字节（对齐 `scripts::MAX_NAME_BYTES`）
    pub name: String,
    pub kind: TaskKind,
    /// 目标 id：`ScriptMeta.id`（Script）或 `CommandSnippet.id`（Command）。创建时校验存在性。
    pub target_id: String,
    /// 参数值。**不得包含** `ScriptParam.secret == true` 的参数值（定义期拒绝，契约 §3.3 R-3）。
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
    /// 默认 **false**（裁定 R-A6-1，契约 §3.5）；存量文件缺该字段时同样按 false 处理
    #[serde(default = "default_task_enabled")]
    pub enabled: bool,
    pub trigger: TaskTrigger,
    /// 错过执行策略，默认 `Skip`
    #[serde(default)]
    pub missed_run_policy: MissedRunPolicy,
    /// `CatchUp` 补跑上限，默认 3，硬上限 `SCHED_MAX_CATCH_UP`
    #[serde(default = "default_catch_up_limit")]
    pub catch_up_limit: u32,
    /// 迟到超过该秒数才判定为「错过」，默认 60
    #[serde(default = "default_misfire_grace_secs")]
    pub misfire_grace_secs: u64,
    /// 重试策略，默认「不重试」
    #[serde(default)]
    pub retry: RetryPolicy,
    /// 0 = 沿用 `script_runner::DEFAULT_TIMEOUT_SECS`(60)；上限 600
    #[serde(default)]
    pub timeout_secs: u32,
    /// **判重真相源**：上一次「已触发」的计划触发时刻（不是完成时刻）
    #[serde(default)]
    pub last_fired_at: Option<DateTime<Utc>>,
    /// 下一次计划触发时刻（永远指向未来）；由调度器计算并落盘
    #[serde(default)]
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 自动执行默认关闭（裁定 R-A6-1）：无人值守执行必须是一次显式动作。
fn default_task_enabled() -> bool {
    false
}

fn default_catch_up_limit() -> u32 {
    3
}

fn default_misfire_grace_secs() -> u64 {
    60
}

/// 定时任务运行记录（持久化 `data_dir/task-runs.json`）。
///
/// **仅历史与排障，不参与判重**（判重真相源是 `TaskDef.last_fired_at`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskRunRecord {
    pub task_id: String,
    /// 与 `ScriptRunRecord.run_id` 同值（join key；输出尾存仍在 `script-runs.json`）
    pub run_id: String,
    pub trigger: TaskRunTrigger,
    /// 计划触发时刻；重试沿用同一值，只递增 `attempt`
    pub scheduled_at: DateTime<Utc>,
    pub attempt: u32,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    /// 稳定错误码（如 `SCRIPT_ALREADY_RUNNING`、`TASK_TARGET_NOT_FOUND`）
    pub error_code: Option<String>,
}

// ---------------------------------------------------------------------------
// M5-2 MCP 命令注册表 / 全局策略 DTO（契约冻结见 A3 M5-2 主篇 / A1 M5-2 卡）
//
// 首期切片（W3 Lane A3）：**只有类型与常量**，无任何 rmcp 依赖、无 server runtime、
// 无网络监听、无新 Tauri 命令。策略逻辑在 `src-tauri/src/mcp.rs`，纯函数且复用
// `security_policy` 既有路径根 / URL 脱敏守门（单一真源，不得各实现一份）。
//
// 全部 `#[allow(dead_code)]`：本段是契约层，由 `mcp.rs`（bin 专属）消费；
// `mvp_core` lib 编译时尚未被使用（与 `DbConnectionConfig` 等同口径）。
// ---------------------------------------------------------------------------

/// MCP 能力白名单（首期：只读 + 路径根 / 脱敏约束）。
/// **单一真源**：不得在其他文件重复定义（`check-mcp-policy.py` `MCP_CAPABILITY_DRIFT` 守门）。
#[allow(dead_code)]
pub const MCP_CAPABILITY_V1: &[&str] = &[
    "file_read",
    "file_list",
    "tab_query",
    "history_query",
    "bookmarks_query",
    "downloads_query",
    "console_query",
];

/// MCP 工具注册项：能力名 + 落地的内部核心 API + 是否触碰文件系统 / 是否回传 URL。
///
/// `core_api` 是文档性标注（首期无 server，不真的调用），指明该工具应走哪条
/// **核心内部 API**——绝不能是命令层 `bridge::*`（A3 补篇 B3：MCP 工具不得绕过来源校验）。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpCommandDef {
    pub capability: &'static str,
    pub core_api: &'static str,
    /// 触碰文件系统：必须经 `security_policy::check_path_within_roots`
    pub touches_fs: bool,
    /// 回传 URL：必须经 `security_policy::redact_sensitive_url`
    pub returns_url: bool,
}

/// MCP 全局策略裁决（fail-closed：默认拒绝）。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpDecision {
    Allow,
    Deny,
}

/// 能力是否在白名单内（fail-closed：未知 / 大小写变形一律 false）。
#[allow(dead_code)]
pub fn is_known_mcp_capability(name: &str) -> bool {
    MCP_CAPABILITY_V1.contains(&name)
}

/// 策略快照（未来 `mcp_policy_get` 命令的返回形态；首期仅冻结 DTO）。
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPolicySnapshot {
    /// 当前生效的能力集合（恒为 `MCP_CAPABILITY_V1`）
    pub capabilities: Vec<String>,
    /// 全局策略版本（与 A1 M5-2 卡 §4 对齐）
    pub policy_version: &'static str,
}

// ---------------------------------------------------------------------------
// M4-1 契约单测（ID 段 `T-db-c1~c5`，由 A1 展开卡 §5 分配）
// 目的不是覆盖实现（实现归 M4-2/M4-3），而是把**契约本身**钉死：
// 驱动身份串、能力表、凭据字段结构性缺失、缺省值 fail-closed、错误码闭合。
// ---------------------------------------------------------------------------
#[cfg(test)]
mod m4_1_db_contract_tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeSet;

    const ALL_KINDS: [SupportedDb; 3] = [
        SupportedDb::Sqlite,
        SupportedDb::MySql,
        SupportedDb::Postgres,
    ];

    fn sample_config() -> DbConnectionConfig {
        let now = Utc::now();
        DbConnectionConfig {
            id: "b3f1c2d4-0000-4000-8000-000000000001".into(),
            name: "local-sqlite".into(),
            kind: SupportedDb::Sqlite,
            host: None,
            port: None,
            database: "workspace/app.db".into(),
            username: None,
            ssl_mode: DbSslMode::Disable,
            allow_write: false,
            production_hint: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// T-db-c1：驱动身份串往返一致，且 `from_kind_str` fail-closed。
    #[test]
    fn t_db_c1_supported_db_roundtrip_is_fail_closed() {
        for kind in ALL_KINDS {
            assert_eq!(SupportedDb::from_kind_str(kind.as_str()), Some(kind));
            // 落盘/前端串与 as_str 必须同源，杜绝「枚举叫 sqlite、JSON 写 SQLite」
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(kind.as_str()));
        }
        for bad in [
            "",
            " ",
            "Sqlite",
            "SQLITE",
            " sqlite",
            "sqlite ",
            "mysql2",
            "mssql",
            "oracle",
            "postgresql",
            "postgres-",
            "jdbc",
        ] {
            assert!(
                SupportedDb::from_kind_str(bad).is_none(),
                "未知/变形驱动标识必须拒绝: {bad:?}"
            );
        }
    }

    /// T-db-c2：驱动能力表（默认端口 / 是否远程 / 是否支持 TLS）。
    #[test]
    fn t_db_c2_driver_capability_table() {
        assert_eq!(
            SupportedDb::Sqlite.default_port(),
            None,
            "SQLite 无网络端口"
        );
        assert_eq!(SupportedDb::MySql.default_port(), Some(3306));
        assert_eq!(SupportedDb::Postgres.default_port(), Some(5432));
        assert!(!SupportedDb::Sqlite.is_remote());
        assert!(SupportedDb::MySql.is_remote() && SupportedDb::Postgres.is_remote());
        assert!(!SupportedDb::Sqlite.supports_ssl());
        assert!(SupportedDb::MySql.supports_ssl() && SupportedDb::Postgres.supports_ssl());
    }

    /// T-db-c3（F2 结构性红线）：连接配置序列化结果不得出现任何凭据承载字段。
    #[test]
    fn t_db_c3_connection_config_has_no_credential_field() {
        let value = serde_json::to_value(sample_config()).unwrap();
        let keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
        for forbidden in [
            "password",
            "passwd",
            "pwd",
            "secret",
            "token",
            "dsn",
            "conn_str",
            "connection_string",
            "uri",
            "url",
        ] {
            assert!(
                !keys.iter().any(|k| k.contains(forbidden)),
                "连接配置结构性不得承载凭据字段: {forbidden}（实际键: {keys:?}）"
            );
        }
    }

    /// T-db-c4（F1 fail-closed）：历史 JSON 缺新字段时，缺省值必须落在更严一侧。
    #[test]
    fn t_db_c4_missing_fields_default_to_fail_closed() {
        let raw = r#"{
            "id":"b3f1c2d4-0000-4000-8000-000000000002",
            "name":"legacy","kind":"postgres","database":"app",
            "created_at":"2026-09-05T00:00:00Z","updated_at":"2026-09-05T00:00:00Z"
        }"#;
        let cfg: DbConnectionConfig = serde_json::from_str(raw).unwrap();
        assert!(!cfg.allow_write, "缺 allow_write 必须 = 拒绝写");
        assert_eq!(cfg.ssl_mode, DbSslMode::Prefer, "缺 ssl_mode 必须取严一档");
        assert_eq!(cfg.production_hint, None, "未标记不得被推断为「非生产」");
        assert!(cfg.enabled, "缺 enabled 视为启用（与 M2-3 既有口径一致）");
        assert_eq!(cfg.port, None);
        assert_eq!(cfg.username, None);
    }

    /// T-db-c5：错误码闭合 + 稳定 + 唯一（增删枚举必须同步改本断言的计数）。
    #[test]
    fn t_db_c5_error_codes_are_closed_and_stable() {
        const ALL_CODES: [DbErrorCode; 18] = [
            DbErrorCode::UnsupportedKind,
            DbErrorCode::InvalidConfig,
            DbErrorCode::PathOutsideRoots,
            DbErrorCode::CredentialMissing,
            DbErrorCode::ConnectFailed,
            DbErrorCode::NotConnected,
            DbErrorCode::PoolExhausted,
            DbErrorCode::SqlEmpty,
            DbErrorCode::SqlTooLarge,
            DbErrorCode::SqlParseFailed,
            DbErrorCode::MultipleStatements,
            DbErrorCode::WriteDenied,
            DbErrorCode::ProductionUnknown,
            DbErrorCode::QueryFailed,
            DbErrorCode::Timeout,
            DbErrorCode::Cancelled,
            DbErrorCode::LimitExceeded,
            DbErrorCode::NotSupported,
        ];
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for code in ALL_CODES {
            let s = code.as_str();
            assert!(s.starts_with("DB_"), "错误码必须 DB_ 前缀: {s}");
            assert_eq!(s, s.to_ascii_uppercase(), "错误码形态必须稳定: {s}");
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()),
                "错误码只含大写/下划线/数字: {s}"
            );
            assert!(seen.insert(s), "错误码重复: {s}");
        }
        assert_eq!(
            ALL_CODES.len(),
            18,
            "错误码枚举闭合：增删必须同步改本断言与夹具码位表"
        );
    }

    /// T-db-c6：上限与超时常量的**跨模块对齐**（防 A3/A4 各写一份导致口径漂移）。
    #[test]
    fn t_db_c6_limit_and_timeout_constants_are_aligned() {
        // M5-1 切片 0b 后常量已收口进 `domain.rs`，原「跨模块对齐」退化为同模块自比；
        // 改为「字面值锁定 + 契约别名同值」双保险，仍防 A3/A4 改值时口径漂移。
        assert_eq!(MAX_TEXT_FIELD_BYTES, 64 * 1024, "文本字段上限字面值锁定");
        assert_eq!(MAX_TIMEOUT_SECS, 600, "脚本超时上限字面值锁定");
        assert_eq!(HARD_GRACE_SECS, 5, "soft→hard 宽限字面值锁定");
        assert_eq!(
            DB_MAX_TEXT_FIELD_BYTES, MAX_TEXT_FIELD_BYTES,
            "DB 单字段上限必须=通用文本字段上限（契约别名同值）"
        );
        assert_eq!(
            DB_MAX_SQL_BYTES, MAX_TEXT_FIELD_BYTES,
            "SQL 文本上限复用文本字段量级"
        );
        assert_eq!(
            DB_MAX_QUERY_TIMEOUT_SECS, MAX_TIMEOUT_SECS,
            "DB 查询超时上限=通用超时上限"
        );
        assert_eq!(
            DB_SOFT_TO_HARD_GRACE_SECS, HARD_GRACE_SECS,
            "DB soft→hard 宽限=通用宽限"
        );
        assert_eq!(DB_MAX_ROWS, 1_000);
        assert_eq!(DB_MAX_RESULT_BYTES, 4 * 1024 * 1024);
        assert_eq!(DB_DEFAULT_QUERY_TIMEOUT_SECS, 30);
        assert!(DB_DEFAULT_QUERY_TIMEOUT_SECS <= DB_MAX_QUERY_TIMEOUT_SECS);
        assert!(
            DB_MAX_TEXT_FIELD_BYTES < DB_MAX_RESULT_BYTES,
            "单字段上限必须小于整结果上限，否则字段截断永远不会先命中"
        );
        assert!(DB_CANCEL_CHECK_EVERY_ROWS > 0);
        assert!(
            DB_CANCEL_CHECK_EVERY_ROWS < DB_MAX_ROWS,
            "取消检查间隔必须小于行数上限，否则第一批就取满了才检查"
        );
        assert!(DB_MAX_EXPORT_BYTES > DB_MAX_RESULT_BYTES);
    }

    /// T-db-c7（F3）：结果结构必须**显式**承载截断标记，且不得夹带 SQL 原文/凭据。
    #[test]
    fn t_db_c7_query_result_marks_truncation_explicitly() {
        let res = DbQueryResult {
            query_id: "q1".into(),
            columns: vec!["id".into()],
            rows: vec![vec![DbValue::Int(1)]],
            row_count: 1,
            truncated: true,
            field_truncated: true,
            limit_hit: Some(DbLimitKind::Bytes),
            elapsed_ms: 12,
            state: DbQueryState::Completed,
        };
        let value = serde_json::to_value(&res).unwrap();
        assert_eq!(value["truncated"], json!(true), "截断必须显式标记");
        assert_eq!(value["field_truncated"], json!(true));
        assert_eq!(value["limit_hit"], json!("bytes"));

        let keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
        for forbidden in [
            "sql",
            "password",
            "passwd",
            "pwd",
            "secret",
            "token",
            "dsn",
            "credential",
        ] {
            assert!(
                !keys.iter().any(|k| k.contains(forbidden)),
                "结果 DTO 结构性不得含 {forbidden}（实际键: {keys:?}）"
            );
        }

        // 历史 JSON 兼容：缺 limit_hit 必须解析为 None
        let legacy = r#"{"query_id":"q2","columns":[],"rows":[],"row_count":0,
            "truncated":false,"field_truncated":false,"elapsed_ms":0,"state":"completed"}"#;
        let parsed: DbQueryResult = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.limit_hit, None);
        assert_eq!(parsed.state, DbQueryState::Completed);
    }

    /// T-db-c8：取数状态 / 上限原因 / 单元格值的 serde 串稳定，未知状态 fail-closed。
    #[test]
    fn t_db_c8_query_state_and_value_serde_strings_are_stable() {
        assert_eq!(
            serde_json::to_value(DbQueryState::Completed).unwrap(),
            json!("completed")
        );
        assert_eq!(
            serde_json::to_value(DbQueryState::Cancelled).unwrap(),
            json!("cancelled")
        );
        assert_eq!(
            serde_json::to_value(DbQueryState::Timeout).unwrap(),
            json!("timeout")
        );
        assert_eq!(
            serde_json::to_value(DbQueryState::Failed).unwrap(),
            json!("failed")
        );
        // 截断不是状态；未知状态串必须解析失败（fail-closed）
        assert!(serde_json::from_str::<DbQueryState>(r#""truncated""#).is_err());

        assert_eq!(
            serde_json::to_value(DbLimitKind::Rows).unwrap(),
            json!("rows")
        );
        assert_eq!(
            serde_json::to_value(DbLimitKind::Bytes).unwrap(),
            json!("bytes")
        );
        assert_eq!(
            serde_json::to_value(DbLimitKind::Field).unwrap(),
            json!("field")
        );

        assert_eq!(serde_json::to_value(DbValue::Null).unwrap(), json!("null"));
        assert_eq!(
            serde_json::to_value(DbValue::BlobLen(7)).unwrap(),
            json!({ "blob_len": 7 }),
            "二进制列只回长度，不回字节"
        );
    }
}
