//! # tauri-plugin-browser-tabs
//!
//! A Tauri v2 plugin for embedding multiple browser tabs as child webviews.
//!
//! ## Features
//!
//! - Create, position, resize, and close child webviews
//! - Unified logical coordinate system (CSS pixels)
//! - Platform-specific fixes for Linux WebKitGTK resize issues
//! - Event emission for navigation and `window.open` interception
//! - Lazy host-window binding (works with manually created windows)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_browser_tabs::init())
//!     .run(tauri::generate_context!())
//!     .expect("error while running tauri application");
//! ```
//!
//! Then from the frontend:
//!
//! ```ts
//! import { createTab, updateRect } from '@tauri-browser-tabs/core';
//!
//! await createTab({
//!   id: 'tab-1',
//!   url: 'https://example.com',
//!   rect: { x: 0, y: 100, width: 1200, height: 600 },
//! });
//! ```

mod commands;
mod models;
mod platform;

pub use commands::{TabManager, TabManagerState};
pub use models::{
    BrowserTabError, BrowserTabEvent, CreateTabOptions, LogicalRect, Result, TabId,
};
pub use platform::ensure_native_layout;

use std::sync::Arc;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Emitter, Manager, WindowEvent, Wry,
};

/// The default label of the window that hosts child webviews.
const DEFAULT_HOST_WINDOW: &str = "main";

/// Initialize the browser tabs plugin with the default host window (`main`).
///
/// The plugin is bound to the default `Wry` runtime, which covers all
/// desktop targets (Windows, macOS, Linux).
pub fn init() -> TauriPlugin<Wry> {
    init_with_host(DEFAULT_HOST_WINDOW)
}

/// Initialize the plugin with a custom host window label.
///
/// The host window is resolved lazily on every tab operation, so it may be
/// created after this plugin's `setup` hook runs (e.g. via `WindowBuilder`
/// inside the app's own `.setup()`).
pub fn init_with_host(host_label: &str) -> TauriPlugin<Wry> {
    let host_label = host_label.to_string();

    Builder::new("browser-tabs")
        .invoke_handler(tauri::generate_handler![
            commands::create_tab,
            commands::update_rect,
            commands::set_visible,
            commands::close_tab,
            commands::navigate,
            commands::list_tabs,
            commands::set_zoom,
        ])
        .setup(move |app, _api| {
            let manager: TabManagerState =
                Arc::new(TabManager::new(app.app_handle().clone(), host_label.clone()));
            app.manage(manager);

            // Notify the frontend when the host window resizes so it can
            // recalculate and push updated rects. If the host window does not
            // exist yet (created later in the app's own setup), this hook is
            // skipped; frontends should also rely on their own
            // ResizeObserver / window resize listeners.
            if let Some(host) = app.get_window(&host_label) {
                let handle = app.app_handle().clone();
                host.on_window_event(move |event| {
                    if let WindowEvent::Resized(_) = event {
                        let _ = handle.emit("browser-tabs://window-resized", ());
                    }
                });
            }

            Ok(())
        })
        .build()
}

/// Re-exports for convenience.
pub mod prelude {
    pub use crate::{
        ensure_native_layout, init, init_with_host, BrowserTabError, BrowserTabEvent,
        CreateTabOptions, LogicalRect, Result, TabId, TabManager, TabManagerState,
    };
}
