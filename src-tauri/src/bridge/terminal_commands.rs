//! PTY terminal command adapters
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

fn term_measure_mode(app: &AppHandle) -> bool {
    m0_config_of(app).driver == "term-throughput"
}

/// 创建并启动一个 PTY 终端，输出走 `term-data` **全局事件**（F4 Event sink）。
/// 仅 M0 内部回归驱动（`main.rs` scenario-8 / `M0_DRIVER` 资源循环）使用；前端一律走
/// `term_spawn_channel`（夹具锚定前端不得调用本命令）。
#[tauri::command]
pub fn term_spawn(app: AppHandle) -> Result<TermInfo, String> {
    let measure = term_measure_mode(&app);
    let (info, session) = terminal::spawn_terminal(Arc::new(EventSink(app.clone())), measure)?;
    app.state::<AppState>()
        .terminals
        .lock()
        .unwrap()
        .insert(info.id.clone(), session);
    Ok(info)
}

/// 创建并启动一个 PTY 终端，输出走**每终端独立 Channel 单播**（F4 Channel sink，前端唯一入口）。
#[tauri::command]
pub fn term_spawn_channel(
    app: AppHandle,
    channel: tauri::ipc::Channel<serde_json::Value>,
) -> Result<TermInfo, String> {
    let measure = term_measure_mode(&app);
    let (info, session) = terminal::spawn_terminal(Arc::new(ChannelSink(channel)), measure)?;
    app.state::<AppState>()
        .terminals
        .lock()
        .unwrap()
        .insert(info.id.clone(), session);
    Ok(info)
}

/// Queue input for one terminal without holding the global table across I/O.
/// Queue saturation/oversized input is explicit; the caller may retry.
#[tauri::command]
pub fn term_write(app: AppHandle, id: String, data: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let input = {
        let terms = state.terminals.lock().unwrap();
        terms.get(&id).ok_or("终端不存在")?.input()
    };
    input.enqueue(data.as_bytes())
}

/// 调整终端大小（列/行）——M3.a 起真实生效（F5）。
#[tauri::command]
pub async fn term_resize(app: AppHandle, id: String, cols: u16, rows: u16) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || term_resize_sync(app, id, cols, rows))
        .await.map_err(|error| format!("TERM_RESIZE_WORKER_FAILED: {error}"))?
}

/// Rust-internal callers already running off the UI thread use this helper.
pub(crate) fn term_resize_sync(app: AppHandle, id: String, cols: u16, rows: u16) -> Result<(), String> {
    let state = app.state::<AppState>();
    let control = {
        let terms = state.terminals.lock().unwrap();
        terms.get(&id).ok_or("终端不存在")?.control()
    };
    // Native resize may wait on ConPTY; it must not own the table mutex.
    let result = control.resize(cols, rows);
    if terminal::probe_enabled() {
        eprintln!(
            "[TERM_PROBE] term.resize id={id} cols={cols} rows={rows} ok={}",
            result.is_ok()
        );
    }
    result
}

/// Close through the production resource owner. A pending result keeps its
/// cleanup task registered; a repeated kill waits for that same task.
#[tauri::command]
pub async fn term_kill(app: AppHandle, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || term_kill_sync(app, id))
        .await.map_err(|error| format!("TERM_CLOSE_WORKER_FAILED: {error}"))?
}

/// Keep the bounded cleanup wait away from GTK/WebView event callbacks.
pub(crate) fn term_kill_sync(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let removed = {
        let mut terms = state.terminals.lock().unwrap();
        if let Some(session) = terms.get_mut(&id) {
            // Linearize removal with registration. A concurrent repeated kill
            // can then always find either this session or its cleanup task.
            session.request_close();
        }
        terms.remove(&id)
    };
    match removed {
        Some(mut session) => terminal::terminate_session(&mut session),
        None => terminal::retry_cleanup(&id),
    }
}

// ====== M0-0.b 测量钩子（契约 logs/m0-baseline-contract-v1.md §6.1/§6.3） ======
// 日常运行（无 M0_RUN_ID）全部零影响：命令存在但直接返回，不写任何文件。
