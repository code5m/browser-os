//! External URL routing, default-browser integration and M0 measurement commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

pub(crate) fn m0_config_of(app: &AppHandle) -> M0Config {
    app.state::<AppState>().m0_config.lock().unwrap().clone()
}

/// 原子写文件：先写临时文件再 rename，避免采集脚本读到半截内容。
fn m0_atomic_write(path: &str, content: &str) -> Result<(), String> {
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, content).map_err(|e| format!("write {tmp}: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("rename -> {path}: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// M1-4 默认浏览器接入：外部打开 URL 路由（xdg-open / 单实例 / RunEvent::Opened）
// ---------------------------------------------------------------------------

/// M1-4：从进程 argv 中提取候选打开参数（xdg-open 经 desktop 文件 `%u` 传入）。
/// 只剔除以 '-' 开头的 flag（如 --grid-child）；其余一律交给 handle_open_url
/// 做 scheme 白名单分类——非 http/https 在分类处拒绝 + 审计 + 前端提示，
/// 不在此静默丢弃（fail-closed，拒绝路径必须留痕）。
pub fn extract_open_urls(argv: &[String]) -> Vec<String> {
    argv.iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect()
}

/// M1-4：处理一个外部打开 URL 请求。安全边界：仅放行 http/https，
/// 其余 scheme 拒绝 + 审计 + 通知前端 toast；合法 URL 进缓存队列，
/// 前端就绪后发轻提示事件（URL 本体由前端拉取，防事件竞态丢失）。
pub fn handle_open_url(app: &AppHandle, url: &str) {
    if let Err(e) = crate::security_policy::check_openable_url(url) {
        eprintln!("[open-url] rejected: {e}");
        workspace::log_audit(app, "open_url_rejected", url.to_string());
        let _ = app.emit("app://open-url-rejected", serde_json::json!({ "url": url }));
        return;
    }
    eprintln!("[open-url] accepted: {url}");
    workspace::log_audit(app, "open_url", url.to_string());
    let state = app.state::<AppState>();
    state
        .pending_open_urls
        .lock()
        .unwrap()
        .push(url.to_string());
    if state.frontend_ready.load(Ordering::Relaxed) {
        let _ = app.emit("app://open-url-pending", ());
    }
}

/// M1-4：前端拉取并清空待打开 URL 队列（m0_ready 后首次拉取 +
/// 收到 app://open-url-pending 提示时拉取）。拉取即清空，天然去重。
#[tauri::command]
pub fn take_pending_open_urls(webview: tauri::Webview, app: AppHandle) -> Vec<String> {
    if crate::shared::native_legacy_policy::require_main(&webview, "take_pending_open_urls")
        .is_err()
    {
        return Vec::new();
    }

    let state = app.state::<AppState>();
    let urls = std::mem::take(&mut *state.pending_open_urls.lock().unwrap());
    urls
}

/// M1-4：desktop 条目是否可用于 URL 路由（声明了 http(s) handler 且 Exec 带 %u）。
#[cfg(target_os = "linux")]
fn desktop_entry_usable(path: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|c| c.contains("x-scheme-handler/http") && c.contains("%u"))
        .unwrap_or(false)
}

/// M1-4：定位本应用可用的 .desktop 文件。按「用户级 → 系统级」顺序找第一个
/// 声明了 x-scheme-handler/http 且 Exec 带 %u 的条目；找不到（或旧条目缺
/// MimeType/%u）时，在 ~/.local/share/applications 生成指向当前二进制的同名
/// 条目（XDG 用户级优先，自然遮蔽旧系统条目）。本函数只声明 MIME，不改变
/// 系统默认浏览器。返回条目文件名（如 mvp-browser-os.desktop）。
#[cfg(target_os = "linux")]
fn ensure_desktop_entry() -> Result<String, String> {
    const DESKTOP_NAME: &str = "mvp-browser-os.desktop";
    let home = std::env::var("HOME").map_err(|e| format!("HOME 未设置: {e}"))?;
    let user_dir = format!("{home}/.local/share/applications");
    let candidates = [
        format!("{user_dir}/{DESKTOP_NAME}"),
        format!("/usr/share/applications/{DESKTOP_NAME}"),
        format!("/usr/share/applications/com.jizhijiandan.mvp.desktop"),
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() && desktop_entry_usable(c) {
            return std::path::Path::new(c)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .ok_or_else(|| format!("desktop 文件名非法: {c}"));
        }
    }
    // 无可用条目：生成用户级条目（Exec 指向当前运行二进制，%u 接收 xdg-open 的 URL）
    let exe = std::env::current_exe().map_err(|e| format!("无法定位当前二进制: {e}"))?;
    std::fs::create_dir_all(&user_dir).map_err(|e| format!("创建 {user_dir}: {e}"))?;
    let path = format!("{user_dir}/{DESKTOP_NAME}");
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=BrowserOS\nExec={} %u\nIcon=mvp-browser-os\nMimeType=x-scheme-handler/http;x-scheme-handler/https;\nCategories=Network;WebBrowser;\nNoDisplay=false\n",
        exe.display()
    );
    std::fs::write(&path, content).map_err(|e| format!("写入 {path}: {e}"))?;
    // 更新用户级 MIME 数据库（尽力而为，失败不阻断）
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&user_dir)
        .output();
    eprintln!("[open-url] created user desktop entry: {path}");
    Ok(DESKTOP_NAME.to_string())
}

/// M1-4：查询当前系统默认浏览器（xdg-settings get）。
#[tauri::command]
pub fn get_default_browser(webview: tauri::Webview) -> Result<String, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "get_default_browser")?;

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ,
        };
        let wide = |value: &str| -> Vec<u16> {
            value.encode_utf16().chain(std::iter::once(0)).collect()
        };
        let subkey = wide(r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice");
        let name = wide("ProgId");
        let mut value = vec![0u16; 1024];
        let mut bytes = (value.len() * 2) as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER, subkey.as_ptr(), name.as_ptr(),
                RRF_RT_REG_SZ, std::ptr::null_mut(),
                value.as_mut_ptr() as *mut std::ffi::c_void, &mut bytes,
            )
        };
        if status != 0 {
            return Err(format!("WINDOWS_DEFAULT_BROWSER_LOOKUP_FAILED: {status}"));
        }
        let len = (bytes as usize / 2).min(value.len());
        let name = String::from_utf16_lossy(&value[..len])
            .trim_end_matches('\0').to_string();
        if name.trim().is_empty() {
            return Err("WINDOWS_DEFAULT_BROWSER_UNKNOWN".into());
        }
        return Ok(name);
    }
    #[cfg(target_os = "linux")]
    {
    let out = std::process::Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .map_err(|e| format!("xdg-settings 不可用: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "xdg-settings get 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

}

/// M1-4：把本应用设为系统默认浏览器。
/// 硬约束：本命令只能由设置页「设为默认浏览器」按钮经用户显式确认后触发；
/// 应用启动、安装脚本、首启流程一律不得调用（禁静默改写系统默认）。
#[tauri::command]
pub fn set_default_browser(
    webview: tauri::Webview,
    app: AppHandle,
    intent: Option<String>,
) -> Result<String, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "set_default_browser")?;
    app.state::<crate::security_policy::IntentRegistry>()
        .consume(
            "set_default_browser",
            intent
                .as_deref()
                .ok_or("NATIVE_USER_CONFIRMATION_REQUIRED")?,
        )
        .map_err(|_| "NATIVE_USER_CONFIRMATION_DENIED")?;

    #[cfg(target_os = "windows")]
    {
        // Windows 11 requires the user to choose their defaults in Settings.
        // BrowserOS opens the official UI only after the existing native
        // one-time intent has been consumed; it does not rewrite UserChoice.
        open::that("ms-settings:defaultapps")
            .map_err(|e| format!("WINDOWS_DEFAULT_APPS_OPEN_FAILED: {e}"))?;
        workspace::log_audit(&app, "open_windows_default_apps", "user_confirmed".into());
        return Ok("WINDOWS_DEFAULT_APPS_SETTINGS_OPENED".into());
    }
    #[cfg(target_os = "linux")]
    {
    let desktop = ensure_desktop_entry()?;
    let out = std::process::Command::new("xdg-settings")
        .args(["set", "default-web-browser", &desktop])
        .output()
        .map_err(|e| format!("xdg-settings 不可用: {e}"))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        eprintln!("[open-url] set default browser failed: {msg}");
        workspace::log_audit(
            &app,
            "set_default_browser_failed",
            format!("{desktop}: {msg}"),
        );
        return Err(format!("设为默认浏览器失败: {msg}"));
    }
    eprintln!("[open-url] set default browser -> {desktop}");
    workspace::log_audit(&app, "set_default_browser", desktop.clone());
    Ok(desktop)
    }
}

/// ready 信号（契约 §6.1）：主前端完成 mount + 2×rAF 后调用本命令，作为
/// 「一次轻量 IPC 往返」的终点；后端校验 run_id 后将带 run_id 的 ready 信号
/// 原子写入 ready_file。返回 run_id（非测量运行返回空串）。
#[tauri::command]
pub fn m0_ready(webview: tauri::Webview, app: AppHandle) -> Result<String, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "m0_ready")?;

    // M1-4：标记前端就绪。此后到达的外部 URL 会发 pending 提示让前端立即拉取；
    // 就绪前到达的已在队列中，由前端 m0_ready 后的首次拉取兜底（不丢）。
    app.state::<AppState>()
        .frontend_ready
        .store(true, Ordering::Relaxed);
    let cfg = m0_config_of(&app);
    eprintln!(
        "[m0] m0_ready called run_id={:?} ready_file={:?}",
        cfg.run_id, cfg.ready_file
    );
    if cfg.run_id.is_empty() {
        return Ok(String::new());
    }
    if let Some(path) = &cfg.ready_file {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        m0_atomic_write(path, &format!("{}\n{}\n", cfg.run_id, now_ms))?;
    }
    Ok(cfg.run_id)
}

/// 终端吞吐报告（契约 §6.3）：前端检测到 `__M0_TERM_END__` 并完成下一次 animation
/// frame 后上报。计时起点为前端发送负载命令的时刻（report.start_ts_ms），终点为
/// end 标记被消费并完成下一次 rAF（report.end_ts_ms）；elapsed = end - start。
/// 报告原子写入 report_dir/term-throughput-report.json（驱动据此判定完成并写 done 标记）。
#[tauri::command]
pub fn m0_term_report(app: AppHandle, report: serde_json::Value) -> Result<(), String> {
    let cfg = m0_config_of(&app);
    let Some(dir) = &cfg.report_dir else {
        return Ok(());
    };
    if cfg.run_id.is_empty() || cfg.driver != "term-throughput" {
        return Err("terminal report outside term-throughput measurement".into());
    }
    let end_ms = report
        .get("end_ts_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u128;
    let start_ms = report
        .get("start_ts_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u128;
    let begin_seen = report
        .get("begin_seen")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let end_seen = report.get("end_seen").and_then(|v| v.as_u64()).unwrap_or(0);
    let consumed_bytes = report
        .get("consumed_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let frame_gaps = report
        .get("frame_gaps_ms")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut errors = Vec::new();
    if begin_seen != 1 {
        errors.push(format!("begin_seen={begin_seen}, expected 1"));
    }
    if end_seen != 1 {
        errors.push(format!("end_seen={end_seen}, expected 1"));
    }
    if consumed_bytes != 10 * 1024 * 1024 {
        errors.push(format!(
            "consumed_bytes={consumed_bytes}, expected {}",
            10 * 1024 * 1024
        ));
    }
    if start_ms == 0 || end_ms < start_ms {
        errors.push(format!("invalid timestamps start={start_ms} end={end_ms}"));
    }
    if frame_gaps.is_empty() {
        errors.push("frame_gaps_ms is empty".into());
    }
    let valid = errors.is_empty();
    let out = serde_json::json!({
        "run_id": cfg.run_id,
        "start_ts_ms": start_ms,
        "end_ts_ms": end_ms,
        "elapsed_ms": end_ms.saturating_sub(start_ms),
        "begin_seen": begin_seen,
        "end_seen": end_seen,
        "consumed_bytes": consumed_bytes,
        "expected_bytes": 10 * 1024 * 1024,
        "frame_gaps_ms": frame_gaps,
        "valid": valid,
        "errors": errors,
    });
    let json = serde_json::to_string_pretty(&out).map_err(|e| format!("serialize report: {e}"))?;
    m0_atomic_write(&format!("{dir}/term-throughput-report.json"), &json)?;
    if valid {
        Ok(())
    } else {
        Err("terminal throughput report failed validation".into())
    }
}

/// 查询当前 M0 测量配置（非测量运行返回 null）。前端据此在 term-throughput 模式下
/// 自动挂载终端面板并驱动 10 MiB 负载（契约 §6.3）。
#[tauri::command]
pub fn m0_config(app: AppHandle) -> Option<serde_json::Value> {
    let cfg = m0_config_of(&app);
    if cfg.run_id.is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "run_id": cfg.run_id,
        "driver": cfg.driver,
        "ready_file": cfg.ready_file,
        "report_dir": cfg.report_dir,
    }))
}
