use crate::models::{
    BrowserTabError, BrowserTabEvent, CreateTabOptions, LogicalRect, Result, TabId,
};
use crate::platform;
use parking_lot::RwLock;
#[cfg(not(target_os = "linux"))]
use std::sync::Mutex;
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "linux"))]
use tauri::webview::PageLoadEvent;
use std::sync::Arc;
use tauri::{
    webview::NewWindowResponse, AppHandle, Emitter, Manager, State, Webview, WebviewBuilder,
    WebviewUrl, Window, Wry,
};

/// M1-8：资源事件时间戳（epoch 毫秒）。
#[cfg(target_os = "linux")]
fn epoch_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

// External browser tabs share one dedicated persistent WebKit context.
// A WebContext without an explicit data_directory does not configure WebKit's
// persistent session storage; a Bilibili login can disappear on restart.
// Keep the browser profile separate from the trusted local app UI WebView and
// never expose its contents through native commands or the BFC readback.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn browser_profile_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("browser-webview-profile")
}

#[cfg(target_os = "linux")]
fn ensure_private_browser_profile(app: &AppHandle<Wry>) -> Result<PathBuf> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|_| BrowserTabError::Platform("browser profile directory unavailable".into()))?;
    let profile = browser_profile_path(&base);
    if let Ok(meta) = std::fs::symlink_metadata(&profile) {
        if !meta.file_type().is_dir() {
            return Err(BrowserTabError::Platform(
                "browser profile is not a directory".into(),
            ));
        }
    }
    std::fs::create_dir_all(&profile)
        .map_err(|_| BrowserTabError::Platform("cannot create browser profile".into()))?;
    let canonical_base = std::fs::canonicalize(&base)
        .map_err(|_| BrowserTabError::Platform("browser profile base unavailable".into()))?;
    let canonical_profile = std::fs::canonicalize(&profile)
        .map_err(|_| BrowserTabError::Platform("browser profile unavailable".into()))?;
    if canonical_profile != canonical_base.join("browser-webview-profile") {
        return Err(BrowserTabError::Platform(
            "browser profile path invalid".into(),
        ));
    }
    std::fs::set_permissions(&profile, std::fs::Permissions::from_mode(0o700))
        .map_err(|_| BrowserTabError::Platform("cannot secure browser profile".into()))?;
    Ok(profile)
}

// Browser sub-WebViews use a single app-owned WebView2 user data folder.
// The trusted main UI keeps its default, separate storage environment. This
// directory inherits the current user's app-data ACL; ACL hardening and
// WebView2 environment-option compatibility require Windows GUI verification.
#[cfg(target_os = "windows")]
fn ensure_private_browser_profile(app: &AppHandle<Wry>) -> Result<PathBuf> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|_| BrowserTabError::Platform("browser profile directory unavailable".into()))?;
    std::fs::create_dir_all(&base)
        .map_err(|_| BrowserTabError::Platform("browser profile base unavailable".into()))?;
    let canonical_base = std::fs::canonicalize(&base)
        .map_err(|_| BrowserTabError::Platform("browser profile base unavailable".into()))?;
    let profile = browser_profile_path(&canonical_base);
    if let Ok(meta) = std::fs::symlink_metadata(&profile) {
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(BrowserTabError::Platform("browser profile path invalid".into()));
        }
    }
    std::fs::create_dir_all(&profile)
        .map_err(|_| BrowserTabError::Platform("cannot create browser profile".into()))?;
    let canonical_profile = std::fs::canonicalize(&profile)
        .map_err(|_| BrowserTabError::Platform("browser profile unavailable".into()))?;
    if canonical_profile != profile {
        return Err(BrowserTabError::Platform("browser profile path invalid".into()));
    }
    Ok(canonical_profile)
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod browser_profile_tests {
    use super::*;

    #[test]
    fn native_browser_tabs_share_one_app_scoped_profile_directory() {
        let base = Path::new("/some-app-user-data");
        assert_eq!(
            browser_profile_path(base),
            base.join("browser-webview-profile")
        );
        assert!(!browser_profile_path(base)
            .to_string_lossy()
            .contains("tab-1"));
        assert!(!browser_profile_path(base).to_string_lossy().contains("bfc"));
    }
}

/// Internal state for managing tabs.
///
/// The host window is resolved lazily on every operation, so the plugin works
/// even when the host window is created after the plugin's `setup` hook runs
/// (e.g. apps that build their main window manually inside `.setup()`).
pub struct TabManager {
    tabs: RwLock<HashMap<TabId, Webview<Wry>>>,
    app: AppHandle<Wry>,
    host_label: String,
}

impl TabManager {
    pub fn new(app: AppHandle<Wry>, host_label: impl Into<String>) -> Self {
        Self {
            tabs: RwLock::new(HashMap::new()),
            app,
            host_label: host_label.into(),
        }
    }

    fn host_window(&self) -> Result<Window<Wry>> {
        self.app
            .get_window(&self.host_label)
            .ok_or(BrowserTabError::WindowNotFound)
    }

    pub fn create_tab(&self, options: CreateTabOptions) -> Result<()> {
        let mut tabs = self.tabs.write();

        if tabs.contains_key(&options.id) {
            return Err(BrowserTabError::TabAlreadyExists(options.id));
        }

        if options.rect.is_empty() {
            return Err(BrowserTabError::InvalidRect(options.rect));
        }

        let url: tauri::Url = options
            .url
            .parse()
            .map_err(|_| BrowserTabError::InvalidUrl(options.url.clone()))?;

        // Navigation permission callbacks are NOT completion notifications.
        // On Windows, use Tauri's page-load lifecycle rather than emitting
        // NavigationFinished before the document actually finishes loading.
        // Linux continues to use native WebKit main-frame URI observations.
        let mut builder = WebviewBuilder::new(&options.id, WebviewUrl::External(url))
            .on_navigation(|_| true)
            .transparent(options.transparent);

        #[cfg(not(target_os = "linux"))]
        {
            let nav_id = options.id.clone();
            let nav_app = self.app.clone();
            builder = builder.on_page_load(move |_webview, payload| {
                let url = payload.url().to_string();
                let event = match payload.event() {
                    PageLoadEvent::Started => BrowserTabEvent::NavigationStarted {
                        id: nav_id.clone(),
                        url,
                    },
                    PageLoadEvent::Finished => BrowserTabEvent::NavigationFinished {
                        id: nav_id.clone(),
                        url,
                    },
                };
                let _ = nav_app.emit("browser-tabs://event", event);
            });
        }

        // Linux and Windows browser tabs share one app-scoped persistent
        // external-page profile. The trusted main UI uses a separate profile;
        // BFC never reads or exports browser session credentials.
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let profile = ensure_private_browser_profile(&self.app)?;
            builder = builder.data_directory(profile);
        }

        // Intercept window.open / target="_blank": deny the popup and notify
        // the host app so it can open a new tab instead.
        let id_nw = options.id.clone();
        let app_nw = self.app.clone();
        builder = builder.on_new_window(move |url, _features| {
            eprintln!(
                "[browser-tabs] newWindowRequested id={} url_bytes={}",
                id_nw,
                url.as_str().len()
            );
            let _ = app_nw.emit(
                "browser-tabs://event",
                BrowserTabEvent::NewWindowRequested {
                    id: id_nw.clone(),
                    url: url.to_string(),
                },
            );
            NewWindowResponse::Deny
        });

        // auto_resize 让子 webview 跟随主窗 resize 自动调整，配合 force_allocation
        // 的 set_size_request + queue_resize 保证撑满。此为 v0.3.0 正确行为，
        // 【禁止改动】（曾因误以为与前端定位冲突而关闭，导致网页撑不满回退）。
        if options.auto_resize {
            builder = builder.auto_resize();
        }

        if let Some(ua) = &options.user_agent {
            builder = builder.user_agent(ua);
        }

        if let Some(script) = &options.initialization_script {
            builder = builder.initialization_script(script);
        }

        let host = self.host_window()?;
        let webview = platform::create_child_webview(&host, builder, options.rect)?;

        // Main-frame navigation truth on Linux. Gtk/WebKit's WebView.uri
        // property changes for the top-level document, not its iframe
        // documents. notify::uri also catches redirects and history changes.
        // This listener ONLY observes URL state; it never changes GTK sizing,
        // visibility or the browser lifecycle.
        #[cfg(target_os = "linux")]
        {
            let uri_id = options.id.clone();
            let uri_app = self.app.clone();
            let _ = webview.with_webview(move |native| {
                use webkit2gtk::WebViewExt;
                let gtk_wv = native.inner();
                let last_uri = std::cell::RefCell::new(String::new());
                gtk_wv.connect_uri_notify(move |wv| {
                    let Some(uri) = wv.uri() else { return };
                    let value = uri.as_str();
                    let mut previous = last_uri.borrow_mut();
                    if value.is_empty() || value == previous.as_str() {
                        return;
                    }
                    *previous = value.to_owned();
                    // Keep URL in the trusted IPC payload only. Never write
                    // a URL/query containing credentials into stderr.
                    let _ = uri_app.emit(
                        "browser-tabs://event",
                        BrowserTabEvent::NavigationFinished {
                            id: uri_id.clone(),
                            url: value.to_owned(),
                        },
                    );
                });
            });
        }

        // Shell shortcuts belong to the trusted GTK host, not remote-page JS/IPC.
        // Capture only three app-owned shortcuts from a focused child WebView.
        #[cfg(target_os = "linux")]
        {
            let id_key = options.id.clone();
            let app_key = self.app.clone();
            let _ = webview.with_webview(move |native| {
                use gtk::prelude::*;
                let child = native.inner();
                child.connect_key_press_event(move |_wv, event| {
                    use gtk::gdk::ModifierType;
                    let state = event.state();
                    let ctrl = state.contains(ModifierType::CONTROL_MASK);
                    let shift = state.contains(ModifierType::SHIFT_MASK);
                    let alt = state.contains(ModifierType::MOD1_MASK);
                    let meta = state.contains(ModifierType::META_MASK)
                        || state.contains(ModifierType::SUPER_MASK);
                    if alt || meta {
                        return gtk::glib::Propagation::Proceed;
                    }
                    let key = event
                        .keyval()
                        .name()
                        .map(|n| n.to_string())
                        .unwrap_or_default();
                    let action = if key == "F11" && !ctrl {
                        if shift {
                            None
                        } else {
                            Some("immersive")
                        }
                    } else if key == "F11" && ctrl && shift {
                        Some("compact")
                    } else if (key == "l" || key == "L") && ctrl && !shift {
                        Some("address")
                    } else {
                        None
                    };
                    if let Some(action) = action {
                        let _ = app_key.emit(
                            "browser-tabs://shell-shortcut",
                            serde_json::json!({
                                "id": id_key, "action": action
                            }),
                        );
                        gtk::glib::Propagation::Stop
                    } else {
                        gtk::glib::Propagation::Proceed
                    }
                });
            });
        }

        // 加载失败上报（Linux）：WebKit 内部错误页不注入用户脚本，前端无法感知
        // TLS/网络失败，必须连原生 load-failed / load-failed-with-tls-errors 信号。
        #[cfg(target_os = "linux")]
        {
            let id_lf = options.id.clone();
            let app_lf = self.app.clone();
            let _ = webview.with_webview(move |pwv| {
                use webkit2gtk::WebViewExt;
                let gtk_wv = pwv.inner();
                let id1 = id_lf.clone();
                let app1 = app_lf.clone();
                gtk_wv.connect_load_failed(move |_wv, _event, uri, error| {
                    eprintln!(
                        "[browser-tabs] loadFailed id={} uri={} err={}",
                        id1, uri, error
                    );
                    let _ = app1.emit(
                        "browser-tabs://event",
                        BrowserTabEvent::LoadFailed {
                            id: id1.clone(),
                            url: uri.to_string(),
                            error: error.to_string(),
                        },
                    );
                    false // 保留 WebKit 默认错误页（用户可见错误原因）
                });
                let id2 = id_lf.clone();
                let app2 = app_lf.clone();
                gtk_wv.connect_load_failed_with_tls_errors(move |_wv, uri, _cert, _errors| {
                    eprintln!("[browser-tabs] loadFailedTLS id={} uri={}", id2, uri);
                    let _ = app2.emit(
                        "browser-tabs://event",
                        BrowserTabEvent::LoadFailed {
                            id: id2.clone(),
                            url: uri.to_string(),
                            error: "tls-handshake".to_string(),
                        },
                    );
                    false
                });
            });
        }

        // M1-8 资源瀑布采集（Linux WebKitGTK 原生信号）：
        // resource-load-started 记录方法与开始时刻，finished/failed 时取真实
        // URIResponse（status/mime/content_length）后上报。字段全部来自原生信号，
        // 无响应时 status/mime/size_bytes 为 None，绝不伪造；错误信息不上报
        // （GError 文本可能含原始 URL，隐私红线）。
        // 隐私注意：此处**不打印、不落盘**任何资源 URL（含 query 的原始 URL
        // 可能带 token）；脱敏与容量上限由主进程 bridge 统一负责。
        #[cfg(target_os = "linux")]
        {
            let id_rs = options.id.clone();
            let app_rs = self.app.clone();
            let _ = webview.with_webview(move |pwv| {
                use webkit2gtk::{URIRequestExt, URIResponseExt, WebResourceExt, WebViewExt};
                let gtk_wv = pwv.inner();
                gtk_wv.connect_resource_load_started(move |_wv, resource, request| {
                    let started = std::time::Instant::now();
                    let started_at = epoch_millis();
                    let url = request.uri().map(|u| u.to_string()).unwrap_or_default();
                    if url.is_empty() {
                        return;
                    }
                    let method = request.http_method().map(|m| m.to_string());
                    let emit = {
                        let id = id_rs.clone();
                        let app = app_rs.clone();
                        let url = url.clone();
                        let method = method.clone();
                        move |status: Option<u32>, mime: Option<String>, size: Option<u64>| {
                            let _ = app.emit(
                                "browser-tabs://event",
                                BrowserTabEvent::ResourceReceived {
                                    id: id.clone(),
                                    url: url.clone(),
                                    method: method.clone(),
                                    status,
                                    mime,
                                    size_bytes: size,
                                    started_at,
                                    finished_at: epoch_millis(),
                                },
                            );
                        }
                    };
                    let emit_finished = emit.clone();
                    resource.connect_finished(move |r| {
                        let _elapsed = started.elapsed();
                        match r.response() {
                            Some(resp) => {
                                let size = match resp.content_length() {
                                    0 => None, // WebKit 未知长度返回 0，映射为 None（不伪造）
                                    n => Some(n),
                                };
                                emit_finished(
                                    Some(resp.status_code()),
                                    resp.mime_type().map(|m| m.to_string()),
                                    size,
                                );
                            }
                            None => emit_finished(None, None, None),
                        }
                    });
                    resource.connect_failed(move |_r, _error| {
                        // 加载失败：只上报能确定的真实字段，status/mime/size 为 None
                        emit(None, None, None);
                    });
                });
            });
        }

        if !options.visible {
            webview.hide()?;
        }

        tabs.insert(options.id, webview);
        Ok(())
    }

    pub fn update_rect(&self, id: &TabId, rect: LogicalRect) -> Result<()> {
        if rect.is_empty() {
            return Err(BrowserTabError::InvalidRect(rect));
        }

        let tabs = self.tabs.read();
        let webview = tabs
            .get(id)
            .ok_or_else(|| BrowserTabError::TabNotFound(id.clone()))?;

        platform::apply_rect(webview, rect)
    }

    pub fn set_visible(&self, id: &TabId, visible: bool) -> Result<()> {
        let tabs = self.tabs.read();
        let webview = tabs
            .get(id)
            .ok_or_else(|| BrowserTabError::TabNotFound(id.clone()))?;

        if visible {
            webview.show()?;
        } else {
            webview.hide()?;
        }
        Ok(())
    }

    /// 设置子 webview 内容缩放（WebKitGTK set_zoom_level）。
    /// 用于宫格小格子按比例缩小网页内容，实现"自适应"展示。
    pub fn set_zoom(&self, id: &TabId, scale_factor: f64) -> Result<()> {
        let tabs = self.tabs.read();
        let webview = tabs
            .get(id)
            .ok_or_else(|| BrowserTabError::TabNotFound(id.clone()))?;
        webview.set_zoom(scale_factor)?;
        Ok(())
    }

    pub fn close_tab(&self, id: &TabId) -> Result<()> {
        let mut tabs = self.tabs.write();
        let webview = tabs
            .get(id)
            .ok_or_else(|| BrowserTabError::TabNotFound(id.clone()))?;

        webview.close()?;
        tabs.remove(id);
        platform::forget_native_layout(id);
        Ok(())
    }

    pub fn navigate(&self, id: &TabId, url: &str) -> Result<()> {
        let tabs = self.tabs.read();
        let webview = tabs
            .get(id)
            .ok_or_else(|| BrowserTabError::TabNotFound(id.clone()))?;

        let url: tauri::Url = url
            .parse()
            .map_err(|_| BrowserTabError::InvalidUrl(url.to_string()))?;
        webview.navigate(url)?;
        Ok(())
    }

    pub fn get_tab_ids(&self) -> Vec<TabId> {
        self.tabs.read().keys().cloned().collect()
    }

    /// 在指定子 webview 中执行 JavaScript（fire-and-forget）。
    /// 注意：子 webview 由本插件持有，app.get_webview 找不到它们，
    /// 宿主应用必须走这个方法（否则 eval 静默空转，AI 群发/前进后退全失效）。
    pub fn eval(&self, id: &TabId, js: &str) -> Result<()> {
        let tabs = self.tabs.read();
        let webview = tabs
            .get(id)
            .ok_or_else(|| BrowserTabError::TabNotFound(id.clone()))?;
        webview.eval(js)?;
        Ok(())
    }

    /// Native return channel; never grants the remote document an IPC permission.
    #[allow(deprecated)] // Supports WebKitGTK 2.22 as well as 2.40+.
    pub fn eval_result(
        &self,
        id: &TabId,
        js: String,
        callback: impl FnOnce(std::result::Result<String, String>) + Send + 'static,
    ) -> Result<()> {
        let webview = self
            .tabs
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| BrowserTabError::TabNotFound(id.clone()))?;
        #[cfg(target_os = "linux")]
        webview.with_webview(move |native| {
            use java_script_core::ValueExt;
            use webkit2gtk::WebViewExt;
            native.inner().run_javascript(
                &js,
                None::<&webkit2gtk::gio::Cancellable>,
                move |result| {
                    let value = result
                        .map_err(|_| "GRID_EXTRACT_FAILED".to_string())
                        .and_then(|r| {
                            r.js_value()
                                .map(|v| v.to_str().to_string())
                                .ok_or_else(|| "GRID_NO_RESULT".into())
                        })
                        .and_then(|s| {
                            if s.len() <= 512 * 1024 {
                                Ok(s)
                            } else {
                                Err("GRID_RESULT_LIMIT".into())
                            }
                        });
                    callback(value);
                },
            );
        })?;
        #[cfg(not(target_os = "linux"))]
        {
            // Tauri delegates to WebView2's native script evaluation callback.
            // It returns a JSON-serialized value; normalize a JS string result
            // to match Linux WebKit's existing raw-string callback contract.
            // The mutex enforces the original FnOnce callback even if a
            // platform runtime unexpectedly invokes its Fn callback twice.
            let pending = Mutex::new(Some(callback));
            webview.eval_with_callback(js, move |raw_json| {
                if let Ok(mut guard) = pending.lock() {
                    if let Some(complete) = guard.take() {
                        complete(decode_native_eval_result(&raw_json));
                    }
                }
            })?;
        }
        Ok(())
    }
}

// Tauri returns a JSON value on Windows; Linux WebKit already returns the
// string payload directly. Avoid double-encoding JSON.stringify(...) results.
#[cfg(any(not(target_os = "linux"), test))]
fn decode_native_eval_result(raw: &str) -> std::result::Result<String, String> {
    const MAX_BYTES: usize = 512 * 1024;
    // JSON escaping can inflate the serialized WebView2 response. Bound it
    // before parsing so an untrusted remote page cannot force huge allocations.
    if raw.len() > MAX_BYTES * 6 {
        return Err("GRID_RESULT_LIMIT".into());
    }
    if raw == "null" {
        return Err("GRID_NO_RESULT".into());
    }
    let value = serde_json::from_str::<String>(raw).unwrap_or_else(|_| raw.to_string());
    if value.len() > MAX_BYTES {
        Err("GRID_RESULT_LIMIT".into())
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod native_eval_result_tests {
    use super::decode_native_eval_result;

    #[test]
    fn decodes_windows_json_string_into_the_linux_callback_contract() {
        assert_eq!(
            decode_native_eval_result(r#""{\"safe\":true,\"url\":\"about:blank\"}""#),
            Ok(r#"{"safe":true,"url":"about:blank"}"#.to_string())
        );
        assert_eq!(decode_native_eval_result("42"), Ok("42".into()));
        assert_eq!(decode_native_eval_result("null"), Err("GRID_NO_RESULT".into()));
    }

    #[test]
    fn script_result_is_bounded_before_decoding() {
        let long = format!("\"{}\"", "x".repeat(512 * 1024 + 1));
        assert_eq!(decode_native_eval_result(&long), Err("GRID_RESULT_LIMIT".into()));
    }
}

/// Shared state type for Tauri commands and host-app integration.
pub type TabManagerState = Arc<TabManager>;

#[tauri::command]
pub async fn create_tab(
    state: State<'_, TabManagerState>,
    options: CreateTabOptions,
) -> Result<()> {
    state.create_tab(options)
}

#[tauri::command]
pub async fn update_rect(
    state: State<'_, TabManagerState>,
    id: TabId,
    rect: LogicalRect,
) -> Result<()> {
    state.update_rect(&id, rect)
}

#[tauri::command]
pub async fn set_visible(
    state: State<'_, TabManagerState>,
    id: TabId,
    visible: bool,
) -> Result<()> {
    state.set_visible(&id, visible)
}

#[tauri::command]
pub async fn close_tab(state: State<'_, TabManagerState>, id: TabId) -> Result<()> {
    state.close_tab(&id)
}

#[tauri::command]
pub async fn navigate(state: State<'_, TabManagerState>, id: TabId, url: String) -> Result<()> {
    state.navigate(&id, &url)
}

#[tauri::command]
pub async fn list_tabs(state: State<'_, TabManagerState>) -> Result<Vec<TabId>> {
    Ok(state.get_tab_ids())
}

#[tauri::command]
pub async fn set_zoom(
    state: State<'_, TabManagerState>,
    id: TabId,
    scale_factor: f64,
) -> Result<()> {
    state.set_zoom(&id, scale_factor)
}
