//!
//! Session 能力：会话快照的持久化、列举、恢复与关闭策略。
//! PHASE 3 从 bridge.rs 物理迁移（native-physical-batch-session）。
//! 仅做模块归属，不改任何冻结语义：
//!   - `build_session` 始终对 URL 脱敏、预览 512B 截断（见 crate::session）。
//!   - `SessionDraft` 仅内存、绝不自动落盘（字段保留于 AppState）。
//!   - `flush_sessions` 命令与 ShutdownCoordinator 任务共用 `flush_sessions_inner`，
//!     两路径行为一致、可审计。
//!   - Session→Browser 仅经窄契约 `crate::capabilities::browser::commands::create_tab`
//!     （session_restore 用已脱敏 URL 建 tab），绝不反向依赖 bridge。
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};
use crate::domain::*;
use crate::workspace;
use crate::shared::invocation::{check_invocation_source, check_tab_id};
use crate::capabilities::browser::commands::create_tab;


/// 建立/刷新某 tab 的会话草稿（打开 tab 与导航时调用；仅内存）。
fn drop_session_draft(app: &AppHandle, tab_id: &str) {
    app.state::<crate::AppState>()
        .session_drafts
        .lock()
        .unwrap()
        .remove(tab_id);
}

/// 从当前 tab 状态构建会话（URL 已脱敏，资源取自 M1-8 缓冲）。
fn build_session_for_tab(
    app: &AppHandle,
    tab_id: &str,
    preview: &str,
    reason: &str,
) -> Option<BrowserSession> {
    let state = app.state::<crate::AppState>();
    // 优先取草稿（url 已脱敏、标题为最近一次上报），回退 tabs 表。
    let (url, title) = {
        let drafts = state.session_drafts.lock().unwrap();
        match drafts.get(tab_id) {
            Some(d) if !d.url.is_empty() => (d.url.clone(), d.title.clone()),
            _ => {
                let tabs = state.tabs.lock().unwrap();
                match tabs.get(tab_id) {
                    Some(t) => (t.url.clone(), t.title.clone()),
                    None => return None,
                }
            }
        }
    };
    let records = state.resource_buffer.lock().unwrap().records(tab_id);
    Some(crate::session::build_session(
        tab_id, &url, &title, preview, &records, reason,
    ))
}

/// 会话落盘 + 容量守卫（超出 `SESSION_MAX_COUNT` 删最旧）。
fn persist_session(app: &AppHandle, session: &BrowserSession) -> Result<(), String> {
    let dir = workspace::sessions_dir(app);
    crate::session::save_session(&dir, session)?;
    crate::session::prune_sessions(&dir, SESSION_MAX_COUNT);
    Ok(())
}

/// 保存当前 tab 为会话（立即落盘）。`preview` 为前端采集的最小文本预览
/// （可为空），落盘前由 `build_session` 做 URL 脱敏与 512B 截断。
#[tauri::command]
pub fn session_save(
    app: AppHandle,
    webview: tauri::Webview,
    tab_id: String,
    preview: Option<String>,
) -> Result<SessionSummary, String> {
    check_invocation_source(&webview, "session_save", None, &app)?;
    check_tab_id(&tab_id)?;
    let text = preview.unwrap_or_default();
    let session = build_session_for_tab(&app, &tab_id, &text, CLOSE_REASON_SAVED)
        .ok_or_else(|| "页签不存在".to_string())?;
    persist_session(&app, &session)?;
    drop_session_draft(&app, &tab_id);
    workspace::log_audit(
        &app,
        "session_save",
        format!(
            "tab_id={} resources={} preview_bytes={}",
            tab_id,
            session.resource_count,
            session.preview.len()
        ),
    );
    Ok(crate::session::summarize(&session))
}

/// 明确丢弃某 tab 的会话草稿（关闭弹窗选「删除」）：不落盘，仅审计。
#[tauri::command]
pub fn session_discard(
    app: AppHandle,
    webview: tauri::Webview,
    tab_id: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "session_discard", None, &app)?;
    check_tab_id(&tab_id)?;
    let existed = app
        .state::<crate::AppState>()
        .session_drafts
        .lock()
        .unwrap()
        .remove(&tab_id)
        .is_some();
    workspace::log_audit(
        &app,
        "session_discard",
        format!("tab_id={tab_id} draft_existed={existed}"),
    );
    Ok(())
}

/// 列出本地会话存档（按 updated_at 倒序；损坏文件跳过）。
#[tauri::command]
pub fn session_list(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<SessionSummary>, String> {
    check_invocation_source(&webview, "session_list", None, &app)?;
    Ok(crate::session::list_sessions(&workspace::sessions_dir(
        &app,
    )))
}

/// 读取单个会话详情（含已脱敏资源列表）。
#[tauri::command]
pub fn session_get(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<BrowserSession, String> {
    check_invocation_source(&webview, "session_get", None, &app)?;
    crate::images::check_id(&id, "会话 id")?;
    crate::session::load_session(&workspace::sessions_dir(&app), &id)
}

/// 删除会话存档（幂等）。删除的是存档，不影响仍打开的同名页签。
#[tauri::command]
pub fn session_delete(app: AppHandle, webview: tauri::Webview, id: String) -> Result<bool, String> {
    check_invocation_source(&webview, "session_delete", None, &app)?;
    crate::images::check_id(&id, "会话 id")?;
    let removed = crate::session::delete_session(&workspace::sessions_dir(&app), &id)?;
    workspace::log_audit(&app, "session_delete", format!("id={id} removed={removed}"));
    Ok(removed)
}

/// 导出会话为脱敏 JSON 文本（**不写磁盘**：不引入新的路径写入面，
/// 由前端自行决定保存位置）。
#[tauri::command]
pub fn session_export(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<String, String> {
    check_invocation_source(&webview, "session_export", None, &app)?;
    crate::images::check_id(&id, "会话 id")?;
    let session = crate::session::load_session(&workspace::sessions_dir(&app), &id)?;
    let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    workspace::log_audit(
        &app,
        "session_export",
        format!("id={id} bytes={}", json.len()),
    );
    Ok(json)
}

/// 用会话中**已脱敏**的 URL 新建页签（重启/回看场景）。
/// 边界：URL 在落盘时已脱敏，一次性 token / 登录态参数不会恢复，
/// 因此还原出的页面可能需要重新登录——这是隐私红线的必然代价，不伪造原 URL。
#[tauri::command]
pub fn session_restore(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<TabInfo, String> {
    check_invocation_source(&webview, "session_restore", None, &app)?;
    crate::images::check_id(&id, "会话 id")?;
    let session = crate::session::load_session(&workspace::sessions_dir(&app), &id)?;
    let tab = create_tab(app.clone(), &session.url)?;
    workspace::log_audit(
        &app,
        "session_restore",
        format!(
            "id={id} tab_id={} resources={}",
            tab.id, session.resource_count
        ),
    );
    Ok(tab)
}

/// 关闭路径 flush（ShutdownCoordinator 任务 + 前端 beforeunload 双保险）：
/// ① `auto_save_on_exit` 打开时落盘仍打开 tab 的会话；否则仅释放草稿（不静默保存）；
/// ② 清理异常退出残留 `.tmp`；③ 容量裁剪。行为确定、可审计。
#[tauri::command]
pub fn flush_sessions(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<SessionFlushReport, String> {
    check_invocation_source(&webview, "flush_sessions", None, &app)?;
    Ok(flush_sessions_inner(&app))
}

/// flush 的实际执行体（命令与 ShutdownCoordinator 任务共用，保证两路径行为一致）。
pub fn flush_sessions_inner(app: &AppHandle) -> SessionFlushReport {
    let state = app.state::<crate::AppState>();
    let auto_save = state.session_auto_save_on_exit.load(Ordering::SeqCst);
    let dir = workspace::sessions_dir(app);

    let mut persisted = 0usize;
    if auto_save {
        let tab_ids: Vec<String> = state.tabs.lock().unwrap().keys().cloned().collect();
        for tab_id in tab_ids {
            if let Some(session) =
                build_session_for_tab(app, &tab_id, "", CLOSE_REASON_SHUTDOWN)
            {
                if persist_session(app, &session).is_ok() {
                    persisted += 1;
                }
            }
        }
    }
    let drafts_dropped = {
        let mut drafts = state.session_drafts.lock().unwrap();
        let n = drafts.len();
        drafts.clear();
        n
    };
    let tmp_removed = crate::session::prune_tmp_files(&dir);
    let capacity_removed = crate::session::prune_sessions(&dir, SESSION_MAX_COUNT);

    workspace::log_audit(
        app,
        "session_flush",
        format!(
            "auto_save={auto_save} persisted={persisted} drafts_dropped={drafts_dropped} tmp_removed={tmp_removed}"
        ),
    );
    SessionFlushReport {
        persisted,
        drafts_dropped,
        tmp_removed,
        capacity_removed,
    }
}

/// 查询会话策略（关闭弹窗 / 退出自动保存）。
#[tauri::command]
pub fn get_session_policy(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<SessionPolicy, String> {
    check_invocation_source(&webview, "get_session_policy", None, &app)?;
    let state = app.state::<crate::AppState>();
    Ok(SessionPolicy {
        close_prompt: state.session_close_prompt.load(Ordering::SeqCst),
        auto_save_on_exit: state.session_auto_save_on_exit.load(Ordering::SeqCst),
    })
}

/// 设置会话策略（审计记录开关变化，不含任何 URL/预览内容）。
#[tauri::command]
pub fn set_session_policy(
    app: AppHandle,
    webview: tauri::Webview,
    close_prompt: Option<bool>,
    auto_save_on_exit: Option<bool>,
) -> Result<SessionPolicy, String> {
    check_invocation_source(&webview, "set_session_policy", None, &app)?;
    let state = app.state::<crate::AppState>();
    if let Some(v) = close_prompt {
        state.session_close_prompt.store(v, Ordering::SeqCst);
    }
    if let Some(v) = auto_save_on_exit {
        state.session_auto_save_on_exit.store(v, Ordering::SeqCst);
    }
    let policy = SessionPolicy {
        close_prompt: state.session_close_prompt.load(Ordering::SeqCst),
        auto_save_on_exit: state.session_auto_save_on_exit.load(Ordering::SeqCst),
    };
    workspace::log_audit(
        &app,
        "session_policy",
        format!(
            "close_prompt={} auto_save_on_exit={}",
            policy.close_prompt, policy.auto_save_on_exit
        ),
    );
    Ok(policy)
}
