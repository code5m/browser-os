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

/// 向终端写入数据（按键/命令/回车/Tab 补全都由真实 shell 处理）。
#[tauri::command]
pub fn term_write(app: AppHandle, id: String, data: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut terms = state.terminals.lock().unwrap();
    let session = terms.get_mut(&id).ok_or("终端不存在")?;
    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|e| format!("写入失败: {e}"))?;
    let _ = session.writer.flush();
    Ok(())
}

/// 调整终端大小（列/行）——M3.a 起真实生效（F5）。
#[tauri::command]
pub fn term_resize(app: AppHandle, id: String, cols: u16, rows: u16) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut terms = state.terminals.lock().unwrap();
    let session = terms.get_mut(&id).ok_or("终端不存在")?;
    let result = terminal::resize(session, cols, rows);
    if terminal::probe_enabled() {
        eprintln!(
            "[TERM_PROBE] term.resize id={id} cols={cols} rows={rows} ok={}",
            result.is_ok()
        );
    }
    result
}

/// 关闭终端：置 stop → 回收进程组 → 回收 worker/pump 线程（F6/F7）。
#[tauri::command]
pub fn term_kill(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut terms = state.terminals.lock().unwrap();
    let removed = terms.remove(&id);
    // 复核整改（M3 整体裁定 P2）：`terminate_session` 最长约 4 s（进程组宽限 2 s +
    // 线程回收 2 s）。必须先释放这张表的锁，否则期间 term_write / term_resize /
    // term_spawn 全部阻塞在互斥量上，前端点「关闭/重启」时会连带卡住其它终端调用。
    drop(terms);
    let Some(mut session) = removed else {
        return Ok(());
    };
    terminal::terminate_session(&mut session)
}

// ====== M0-0.b 测量钩子（契约 logs/m0-baseline-contract-v1.md §6.1/§6.3） ======
// 日常运行（无 M0_RUN_ID）全部零影响：命令存在但直接返回，不写任何文件。
