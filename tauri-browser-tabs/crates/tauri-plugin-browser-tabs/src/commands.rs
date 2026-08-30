use crate::models::{
    BrowserTabError, BrowserTabEvent, CreateTabOptions, LogicalRect, Result, TabId,
};
use crate::platform;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{
    webview::NewWindowResponse, AppHandle, Emitter, Manager, State, Webview, WebviewBuilder,
    WebviewUrl, Window, Wry,
};

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

        // Emit navigation events back to the frontend.
        let id = options.id.clone();
        let app = self.app.clone();
        let mut builder = WebviewBuilder::new(&options.id, WebviewUrl::External(url))
            .on_navigation(move |url| {
                eprintln!("[browser-tabs] navigationFinished id={} url={}", id, url);
                let _ = app.emit(
                    "browser-tabs://event",
                    BrowserTabEvent::NavigationFinished {
                        id: id.clone(),
                        url: url.to_string(),
                    },
                );
                true
            })
            .transparent(options.transparent);

        // Intercept window.open / target="_blank": deny the popup and notify
        // the host app so it can open a new tab instead.
        let id_nw = options.id.clone();
        let app_nw = self.app.clone();
        builder = builder.on_new_window(move |url, _features| {
            eprintln!("[browser-tabs] newWindowRequested id={} url={}", id_nw, url);
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
