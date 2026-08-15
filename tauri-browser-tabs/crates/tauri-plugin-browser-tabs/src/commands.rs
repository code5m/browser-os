use crate::models::{BrowserTabError, BrowserTabEvent, CreateTabOptions, LogicalRect, Result, TabId};
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

    pub fn close_tab(&self, id: &TabId) -> Result<()> {
        let mut tabs = self.tabs.write();
        let webview = tabs
            .remove(id)
            .ok_or_else(|| BrowserTabError::TabNotFound(id.clone()))?;

        webview.close()?;
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
}

/// Shared state type for Tauri commands and host-app integration.
pub type TabManagerState = Arc<TabManager>;

#[tauri::command]
pub async fn create_tab(state: State<'_, TabManagerState>, options: CreateTabOptions) -> Result<()> {
    state.create_tab(options)
}

#[tauri::command]
pub async fn update_rect(state: State<'_, TabManagerState>, id: TabId, rect: LogicalRect) -> Result<()> {
    state.update_rect(&id, rect)
}

#[tauri::command]
pub async fn set_visible(state: State<'_, TabManagerState>, id: TabId, visible: bool) -> Result<()> {
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
