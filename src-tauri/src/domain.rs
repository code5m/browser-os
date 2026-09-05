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
