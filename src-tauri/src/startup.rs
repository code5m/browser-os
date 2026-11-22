//! Prepare private workspace storage before business state or WebViews open.
//! Linux's existing early crash logger is intentionally independent of this gate.
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const MAX_PENDING_OPEN_URLS: usize = 64;
const MAX_OPEN_URL_BYTES: usize = 8192;

#[derive(Default)]
struct OpenUrlState {
    ready: bool,
    pending: VecDeque<String>,
}

/// Plugins can receive a second instance's URLs before Tauri calls app setup.
/// Keep those requests in bounded memory until private workspace state exists.
#[derive(Default)]
pub struct PendingOpenUrls(Mutex<OpenUrlState>);

pub fn dispatch_open_url(app: &AppHandle, url: &str) {
    if url.len() > MAX_OPEN_URL_BYTES {
        eprintln!("[startup] open URL rejected: URL_TOO_LONG");
        return;
    }
    let Some(queue) = app.try_state::<PendingOpenUrls>() else {
        eprintln!("[startup] open URL rejected: STARTUP_QUEUE_UNAVAILABLE");
        return;
    };
    let Ok(mut state) = queue.0.lock() else {
        eprintln!("[startup] open URL rejected: STARTUP_QUEUE_POISONED");
        return;
    };
    if !state.ready {
        if state.pending.len() >= MAX_PENDING_OPEN_URLS {
            eprintln!("[startup] open URL rejected: STARTUP_QUEUE_FULL");
            return;
        }
        state.pending.push_back(url.to_owned());
        return;
    }
    drop(state);
    crate::bridge::handle_open_url(app, url);
}

fn release_pending_open_urls(app: &AppHandle) -> io::Result<()> {
    let queue = app.try_state::<PendingOpenUrls>()
        .ok_or_else(|| io::Error::other("STARTUP_QUEUE_UNAVAILABLE"))?;
    let pending = {
        let mut state = queue.0.lock()
            .map_err(|_| io::Error::other("STARTUP_QUEUE_POISONED"))?;
        // Publish readiness and detach the bounded queue atomically. A concurrent
        // callback either queues before this point or routes after it; no URL
        // is stranded between startup and the normal handler. Never hold the
        // queue lock while calling handlers that may emit application events.
        state.ready = true;
        std::mem::take(&mut state.pending)
    };
    for url in pending {
        crate::bridge::handle_open_url(app, &url);
    }
    Ok(())
}

pub fn prepare_workspace(app: &AppHandle) -> io::Result<()> {
    crate::workspace::prepare_data_dir(app)?;
    #[cfg(not(target_os = "linux"))]
    if let Err(error) = crate::crashlog::use_private_directory(&crate::workspace::data_dir(app)) {
        // Diagnostics are optional. Never fall back to a public temporary file.
        eprintln!("[startup] private diagnostic log unavailable: {:?}", error.kind());
    }
    let tabs = app.try_state::<tauri_plugin_browser_tabs::TabManagerState>()
        .ok_or_else(|| io::Error::other("BROWSER_PROFILE_MANAGER_UNAVAILABLE"))?;
    tabs.prepare_storage().map_err(|error| io::Error::other(error.to_string()))?;
    release_pending_open_urls(app)
}

pub fn report_failure(error: &dyn std::fmt::Display) {
    let detail = error.to_string();
    eprintln!("[startup] initialization failed: {detail}");
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND,
        };
        // This OS dialog needs no WebView profile and remains available when
        // private-directory preparation itself failed. Never offer an unsafe
        // fallback, delete user data, or request elevation automatically.
        let message = if detail.contains("PRIVATE_DATA_DIR") || detail.contains("BROWSER_PROFILE") {
            // Private-storage errors contain stage/counters/native codes, never
            // private filenames or account names. Keep the dialog bounded.
            let diagnostic: String = detail.chars().take(1024).collect();
            format!("BrowserOS 无法安全打开应用数据或浏览器配置目录。\n\n{diagnostic}\n\n请关闭其他 BrowserOS 实例，检查当前用户对应用数据目录的所有权、访问权限和磁盘状态后重试。数据未被删除；程序没有使用不安全的备用目录。")
        } else {
            "BrowserOS 初始化失败，程序已安全退出。\n\n错误代码：STARTUP_INITIALIZATION_FAILED\n\n请查看应用诊断日志后重试。".to_string()
        };
        let text: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
        let title: Vec<u16> = "BrowserOS 启动失败".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
            );
        }
    }
}
