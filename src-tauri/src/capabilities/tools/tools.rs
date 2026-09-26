//! M2-7 工具清单与打包（契约见 `logs/checkpoints/B-M2-7.a-tool-manifest-contract-20260905-1800.md`）。
//!
//! 责任：
//!   - 编译期 `include_str!` 嵌入 5 个内置种子 HTML（F8：零新依赖，不引入
//!     `asset` 协议面；build.rs 保持极简，无 `include_dir!`）
//!   - `list_tools` 命令：只读枚举内置 + 用户（workspace/tools 运行时扫描）工具，
//!     返回 `ToolMeta` 清单（不含任何 HTML 字节 / 绝对路径）
//!   - 用户目录不存在时返回空数组（F3/F4：不 panic、不臆造）
//!
//! 不含任何执行能力：打开/渲染归 M2-8（前端子 webview 隔离）。

use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::domain::{ToolMeta, ToolSource};
use crate::workspace;

/// Maximum user-tool document size kept in memory by the local tool protocol.
/// Built-in tools are compile-time assets and are intentionally not limited here.
const MAX_USER_TOOL_BYTES: u64 = 2 * 1024 * 1024;

/// 内置种子清单：`(id, 展示名, 分类, 入口文件名)`。
///
/// 与 `builtin_tool_html` 的 match 臂一一对应（F9：种子为
/// JSON 格式化 / Base64 / 时间戳 / 正则 / Cron；Markdown 预览留待后续扩展）。
const BUILTIN_TOOLS: &[(&str, &str, &str, &str)] = &[
    ("json", "JSON 格式化", "format", "json-tool.html"),
    ("base64", "Base64 编解码", "format", "base64-tool.html"),
    ("timestamp", "时间戳转换", "format", "timestamp-tool.html"),
    ("regex", "正则工坊", "text", "regex-tool.html"),
    ("cron", "Cron 生成/检测", "text", "cron-tool.html"),
];

/// 返回内置工具嵌入的 HTML 字节（编译期 `include_str!`），供 M2-8 经子 webview 打开。
///
/// 全部内置字节只在此处嵌入一份（单一真源），M2-8 不得再 `include_str!` 或读盘。
pub fn builtin_tool_html(id: &str) -> Option<&'static str> {
    match id {
        "json" => Some(include_str!("json-tool.html")),
        "base64" => Some(include_str!("base64-tool.html")),
        "timestamp" => Some(include_str!("timestamp-tool.html")),
        "regex" => Some(include_str!("regex-tool.html")),
        "cron" => Some(include_str!("cron-tool.html")),
        _ => None,
    }
}

/// 构造工具清单：内置（固定）+ 用户（workspace/tools 运行时扫描）。
fn build_tool_list(app: &AppHandle) -> Vec<ToolMeta> {
    let mut out: Vec<ToolMeta> = Vec::new();

    // 内置：编译期嵌入，清单固定
    for (id, name, category, entry) in BUILTIN_TOOLS {
        out.push(ToolMeta {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            category: category.to_string(),
            source: ToolSource::Builtin,
            entry: entry.to_string(),
        });
    }

    // 用户工具：workspace/tools/*.html 运行时扫描。
    // 目录不存在 → 跳过（F3/F4：不 panic、不臆造），仅返回内置清单。
    let user_dir: PathBuf = workspace::workspace_dir(app).join("tools");
    if let Ok(entries) = fs::read_dir(&user_dir) {
        for e in entries.flatten() {
            let path = e.path();
            // 只认 `.html`，避免把非工具文件当工具（R4 同口径）
            if path.extension().and_then(|s| s.to_str()) != Some("html") {
                continue;
            }
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                out.push(ToolMeta {
                    id: format!("user-{file_name}"),
                    name: file_name.to_string(),
                    description: None,
                    category: "user".to_string(),
                    source: ToolSource::User,
                    entry: file_name.to_string(),
                });
            }
        }
    }

    out
}

/// 只读枚举全部工具（内置 + 用户）。
///
/// 范式对齐 `bridge::list_artifacts` / `list_repos`：只读列表、无 webview 来源校验、
/// 不写审计（避免刷满 audit 上限）。返回静态元数据，**不含 HTML 字节与绝对路径**。
#[tauri::command]
pub fn list_tools(app: AppHandle) -> Vec<ToolMeta> {
    build_tool_list(&app)
}

/// 打开一个工具到独立的子 webview 窗口（M2-8）。
///
/// - label = `tool-<id>`，已存在则聚焦（去重，不重复开）。
/// - 窗口 URL 指向自定义 `tool://localhost/<id>` 协议（由 main.rs `.setup` 注册），
///   协议处理器（`tool_html`）返回工具 HTML 字节。
/// - **隔离（F4）**：工具窗口不授予任何 bridge 命令能力（不在任何 capability 文件），
///   故工具 HTML 即便调 `invoke(...)` 也被能力层拒绝。本命令本身只创建窗口，不触碰文件/进程。
#[tauri::command]
pub fn open_tool(id: String, app: AppHandle) -> Result<(), String> {
    if id.contains('/') || id.contains('\\') {
        return Err("非法工具 id".into());
    }
    let label = format!("tool-{id}");
    // 去重：已开则聚焦，不重建
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    let url = format!("tool://localhost/{id}");
    WebviewWindowBuilder::new(
        &app,
        label,
        WebviewUrl::External(tauri::Url::parse(&url).map_err(|e| e.to_string())?),
    )
    .title(format!("工具 · {id}"))
    .inner_size(900.0, 650.0)
    .min_inner_size(480.0, 360.0)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 协议层：根据 `tool://` URI 返回工具 HTML（或安全错误页）。返回纯字符串，
/// 由 main.rs 包装成 `tauri::http::Response`（隔离 Tauri 版本 API 细节）。
///
/// - builtin：`builtin_tool_html(id)` 嵌入字节。
/// - user：`id` 形如 `user-<文件名>` → `workspace/tools/<文件名>`，经路径校验后读取（F5）。
/// - 未知 id / 读失败 / 越权 → 安全错误页（内联、零外链，不泄露绝对路径）。
pub fn tool_html(app: &AppHandle, uri: &str) -> String {
    let id = uri
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim_end_matches('/')
        .to_string();
    if id.is_empty() {
        return error_page("缺少工具 id");
    }
    if let Some(html) = builtin_tool_html(&id) {
        return html.to_string();
    }
    if let Some(file_name) = id.strip_prefix("user-") {
        return match read_user_tool(app, file_name) {
            Ok(html) => html,
            Err(reason) => error_page(&reason),
        };
    }
    error_page("未知工具 id")
}

/// 读取用户工具 HTML：仅允许 `workspace/tools/<文件名>.html`，强制路径校验防 `..` 穿越。
fn read_user_tool(app: &AppHandle, file_name: &str) -> Result<String, String> {
    let base = crate::workspace::workspace_dir(app).join("tools");
    let target = validate_user_tool_path(&base, file_name)?;
    let metadata = fs::metadata(&target).map_err(|_| "读取工具失败".to_string())?;
    if metadata.len() > MAX_USER_TOOL_BYTES {
        return Err("工具文件过大（上限 2 MiB）".to_string());
    }
    fs::read_to_string(&target).map_err(|_| "读取工具失败".to_string())
}

/// 纯路径校验（无 `AppHandle` 依赖，便于单测，M2-9）：
/// 文件名不得含分隔符、须以 `.html` 结尾；`canonicalize` 后仍须在 `base` 内（防 `..` 跳出）。
fn validate_user_tool_path(base: &PathBuf, file_name: &str) -> Result<PathBuf, String> {
    if file_name.contains('/') || file_name.contains('\\') || !file_name.ends_with(".html") {
        return Err("非法文件名（禁止路径分隔符 / 非 .html）".into());
    }
    let target = base.join(file_name);
    let canon_base = base
        .canonicalize()
        .map_err(|_| "工具目录不存在".to_string())?;
    let canon = target
        .canonicalize()
        .map_err(|_| "工具文件不存在".to_string())?;
    if !canon.starts_with(&canon_base) {
        return Err("路径越权（禁止跳出 workspace/tools）".to_string());
    }
    Ok(canon)
}

/// 安全错误页：纯离线内联 HTML，零外链；不泄露绝对路径/系统信息。
/// `msg` 为本端受控的静态文案（非用户输入），仍做基础转义兜底。
fn error_page(msg: &str) -> String {
    format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>工具打开失败</title>\
         <style>body{{font-family:system-ui,-apple-system,sans-serif;background:#1e1e1e;color:#ddd;\
         margin:0;display:flex;align-items:center;justify-content:center;height:100vh}}\
         .box{{max-width:440px;padding:24px;border:1px solid #444;border-radius:10px;line-height:1.6}}\
         h1{{font-size:16px;color:#e06c75;margin:0 0 8px}}\
         p{{font-size:13px;color:#aaa;margin:6px 0}}</style></head>\
         <body><div class=\"box\"><h1>⚠️ 工具打开失败</h1>\
         <p>{}</p>\
         <p>工具是离线 HTML，无法访问网络或本地任意文件。</p></div></body></html>",
        html_escape(msg)
    )
}

/// 极简 HTML 转义（仅用于错误页静态文本兜底），不依赖外部 crate。
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    const BUILTIN_IDS: &[&str] = &["json", "base64", "timestamp", "regex", "cron"];

    /// F3/F4/F5：内置种子 HTML 经 `builtin_tool_html` 嵌入字节，须非空、零外链、零 bridge 写原语。
    #[test]
    fn builtin_seed_html_is_offline_and_no_invoke() {
        for id in BUILTIN_IDS {
            let html =
                builtin_tool_html(id).unwrap_or_else(|| panic!("builtin_tool_html 缺失种子 {id}"));
            assert!(!html.is_empty(), "种子 {id} 嵌入字节为空");
            // F3 零外链
            assert!(
                !html.contains("http://") && !html.contains("https://"),
                "种子 {id} 含外链 http(s)://"
            );
            assert!(
                !external_asset(html),
                "种子 {id} 含外部 <script src>/<link href>/@import 外链"
            );
            // F4 零 bridge 写原语
            assert!(!has_invoke(html), "种子 {id} 含 invoke/__TAURI__ 写原语");
        }
    }

    /// F5：BUILTIN_TOOLS 与 `builtin_tool_html` match 臂一一对应（数量对齐，入口均 .html）。
    #[test]
    fn builtin_tools_table_matches_html_arms() {
        assert_eq!(BUILTIN_TOOLS.len(), BUILTIN_IDS.len());
        for (id, _, _, entry) in BUILTIN_TOOLS {
            assert!(
                builtin_tool_html(id).is_some(),
                "BUILTIN_TOOLS 含 {id} 但无嵌入字节"
            );
            assert!(
                entry.ends_with(".html"),
                "BUILTIN_TOOLS 入口 {entry} 非 .html"
            );
        }
    }

    /// F7/F8：打开与路径越权防御（纯函数，无需 AppHandle）。
    #[test]
    fn user_tool_path_defense() {
        let tmp = std::env::temp_dir().join(format!("m2_9_tools_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);
        // 合法文件：Ok
        let good = tmp.join("good.html");
        let mut f = fs::File::create(&good).unwrap();
        f.write_all(b"<html>ok</html>").unwrap();
        assert!(validate_user_tool_path(&tmp, "good.html").is_ok());
        // 非 .html → Err
        assert!(validate_user_tool_path(&tmp, "evil.txt").is_err());
        // 含 '/' → Err（目录穿越）
        assert!(validate_user_tool_path(&tmp, "../secret.html").is_err());
        // 含 '\\' → Err
        assert!(validate_user_tool_path(&tmp, "a\\b.html").is_err());
        // symlink 跳出 base → Err（starts_with 防线，仅 unix 可构造）
        #[cfg(unix)]
        {
            let outside = std::env::temp_dir().join(format!("m2_9_outside_{}", std::process::id()));
            let _ = fs::File::create(&outside);
            let link = tmp.join("escape.html");
            let _ = std::os::unix::fs::symlink(&outside, &link);
            assert!(validate_user_tool_path(&tmp, "escape.html").is_err());
            let _ = fs::remove_file(&outside);
            let _ = fs::remove_file(&link);
        }
        // 非法 id（含分隔符）在 open_tool 层也被拒（结构复验，这里复测纯函数口径）
        assert!(validate_user_tool_path(&tmp, "x/../y.html").is_err());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn user_tool_size_limit_is_bounded() {
        assert_eq!(MAX_USER_TOOL_BYTES, 2 * 1024 * 1024);
    }

    // ---- 内容判定辅助（与 scripts/check-seed-tools.py 同口径） ----
    fn has_invoke(html: &str) -> bool {
        html.contains("invoke(")
            || html.contains("__TAURI__")
            || html.contains("@tauri-apps")
            || html.contains("__TAURI_INVOKE__")
            || html.contains("window.__TAURI__")
    }

    fn external_asset(html: &str) -> bool {
        let l = html.to_ascii_lowercase();
        l.contains("<script src=\"http")
            || (l.contains("<link") && l.contains("href=\"http"))
            || (l.contains("@import") && l.contains("url(http"))
    }
}
