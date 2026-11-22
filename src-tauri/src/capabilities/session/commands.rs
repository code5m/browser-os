//!
//! Session 能力：会话快照的持久化、列举、恢复与退出策略。
//! - URL 脱敏与预览上限仍由 crate::session 负责。
//! - 普通 Tab 关闭直接释放内存，不保存、不确认（PROJECT-RULES）。
//! - 应用退出先完成可失败的自动保存预检，成功后才开始不可逆资源清理。
//! - 内部逐项报告保留失败与并发更新；公共 Tauri DTO/参数保持兼容。
//! - Session→Browser 仅经 create_tab 窄契约，不反向依赖 bridge。
use crate::capabilities::browser::commands::create_tab;
use crate::domain::*;
use crate::shared::invocation::{check_invocation_source, check_tab_id};
use crate::workspace;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard, TryLockError};
use tauri::{AppHandle, Manager};

// Manual save, explicit flush, exit preflight and deletion share one writer.
// A second caller receives a visible busy result rather than blocking the UI
// or racing a successful write with capacity cleanup.
static SESSION_WRITES: Mutex<()> = Mutex::new(());
static SESSION_IO: crate::shared::blocking::BlockingLane =
    crate::shared::blocking::BlockingLane::new(4, "SESSION_IO_BUSY", "SESSION_IO_WORKER_FAILED");

fn session_write_guard() -> Result<MutexGuard<'static, ()>, &'static str> {
    match SESSION_WRITES.try_lock() {
        Ok(guard) => Ok(guard),
        Err(TryLockError::WouldBlock) => Err("SESSION_FLUSH_BUSY"),
        Err(TryLockError::Poisoned(error)) => {
            // This mutex contains no mutable state. After the exit owner catches
            // a panic, a fresh snapshot/write may retry the atomic storage path.
            SESSION_WRITES.clear_poison();
            Ok(error.into_inner())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionFlushSkipReason {
    AutoSaveDisabled,
    NoOpenTab,
    ExitAlreadyAccepted,
}

#[derive(Debug, Clone)]
pub(crate) enum SessionFlushOutcome {
    Saved,
    Failed { error: String },
    Skipped { reason: SessionFlushSkipReason },
}

/// Internal only: identifiers and native failures never expand the public DTO.
#[derive(Debug, Clone)]
pub(crate) struct SessionFlushEntry {
    pub tab_id: String,
    pub session_id: Option<String>,
    pub outcome: SessionFlushOutcome,
    pub draft_removed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionFlushOutcomeReport {
    pub summary: SessionFlushReport,
    pub outcomes: Vec<SessionFlushEntry>,
    pub changed_tabs: Vec<String>,
    pub policy_changed: bool,
    pub operation_error: Option<&'static str>,
}

impl SessionFlushOutcomeReport {
    fn empty() -> Self {
        Self {
            summary: SessionFlushReport {
                persisted: 0,
                drafts_dropped: 0,
                tmp_removed: 0,
                capacity_removed: 0,
            },
            outcomes: Vec::new(),
            changed_tabs: Vec::new(),
            policy_changed: false,
            operation_error: None,
        }
    }

    fn unavailable(code: &'static str) -> Self {
        let mut report = Self::empty();
        report.operation_error = Some(code);
        report
    }

    pub fn failed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|entry| matches!(&entry.outcome, SessionFlushOutcome::Failed { .. }))
            .count()
            + usize::from(self.operation_error.is_some())
    }

    pub fn skipped_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|entry| matches!(&entry.outcome, SessionFlushOutcome::Skipped { .. }))
            .count()
    }

    pub fn changed_count(&self) -> usize {
        self.changed_tabs.len() + usize::from(self.policy_changed)
    }

    pub fn can_exit(&self) -> bool {
        self.failed_count() == 0 && self.changed_count() == 0
    }

    /// Safe for status events/logging: no error text, URL, title or preview.
    pub fn failure_summary(&self) -> String {
        format!(
            "{} persisted={} failed={} skipped={} changed={}",
            self.operation_error.unwrap_or("SESSION_FLUSH_INCOMPLETE"),
            self.summary.persisted,
            self.failed_count(),
            self.skipped_count(),
            self.changed_count(),
        )
    }

    pub fn into_public_report(self) -> Result<SessionFlushReport, String> {
        if self.can_exit() {
            Ok(self.summary)
        } else {
            Err(self.failure_summary())
        }
    }

    fn changed(&mut self, tab_id: &str) {
        if !self.changed_tabs.iter().any(|id| id == tab_id) {
            self.changed_tabs.push(tab_id.to_string());
        }
    }
}

struct SessionSaveSnapshot {
    session: BrowserSession,
    draft: Option<SessionDraft>,
}

fn same_draft(left: Option<&SessionDraft>, right: Option<&SessionDraft>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.tab_id == right.tab_id && left.url == right.url && left.title == right.title
        }
        _ => false,
    }
}

fn same_tab(left: Option<&TabInfo>, right: Option<&TabInfo>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.id == right.id && left.url == right.url && left.title == right.title
        }
        _ => false,
    }
}

/// Compare and remove under the same mutex. An equivalent value is already
/// represented by the saved snapshot; a newer different value must survive.
/// No draft mutex is held while serializing or doing filesystem I/O.
fn remove_matching_draft(
    drafts: &mut HashMap<String, SessionDraft>,
    tab_id: &str,
    expected: Option<&SessionDraft>,
) -> Result<bool, ()> {
    if !same_draft(drafts.get(tab_id), expected) {
        return Err(());
    }
    Ok(drafts.remove(tab_id).is_some())
}

fn build_session_snapshot(
    tab: &TabInfo,
    draft: Option<&SessionDraft>,
    preview: &str,
    reason: &str,
    records: &[ResourceReceived],
) -> SessionSaveSnapshot {
    let (url, title) = match draft {
        Some(draft) if !draft.url.is_empty() => (&draft.url, &draft.title),
        _ => (&tab.url, &tab.title),
    };
    SessionSaveSnapshot {
        session: crate::session::build_session(&tab.id, url, title, preview, records, reason),
        draft: draft.cloned(),
    }
}

/// Browser's tabs table is authoritative; stale closed-tab drafts cannot save
/// a tab that no longer exists. Resource records remain a point-in-time snapshot.
fn build_session_for_tab(
    app: &AppHandle,
    tab_id: &str,
    preview: &str,
    reason: &str,
) -> Option<SessionSaveSnapshot> {
    let state = app.state::<crate::AppState>();
    let (tab, draft) = {
        let drafts = state.session_drafts.lock().unwrap();
        let tabs = state.tabs.lock().unwrap();
        (tabs.get(tab_id)?.clone(), drafts.get(tab_id).cloned())
    };
    let records = state.resource_buffer.lock().unwrap().records(tab_id);
    Some(build_session_snapshot(
        &tab,
        draft.as_ref(),
        preview,
        reason,
        &records,
    ))
}

/// Caller holds SESSION_WRITES. Report the actual capacity pruning count.
fn persist_session(app: &AppHandle, session: &BrowserSession) -> Result<usize, String> {
    let dir = workspace::sessions_dir(app);
    crate::session::save_session(&dir, session)?;
    Ok(crate::session::prune_sessions(&dir, SESSION_MAX_COUNT))
}

fn persist_snapshots(
    snapshots: &[SessionSaveSnapshot],
    drafts: &Mutex<HashMap<String, SessionDraft>>,
    report: &mut SessionFlushOutcomeReport,
    mut persist: impl FnMut(&BrowserSession) -> Result<usize, String>,
) {
    for snapshot in snapshots {
        let session = &snapshot.session;
        let mut draft_removed = false;
        let outcome = match persist(session) {
            Ok(capacity_removed) => {
                report.summary.persisted += 1;
                report.summary.capacity_removed += capacity_removed;
                match remove_matching_draft(
                    &mut drafts.lock().unwrap(),
                    &session.tab_id,
                    snapshot.draft.as_ref(),
                ) {
                    Ok(removed) => {
                        draft_removed = removed;
                        report.summary.drafts_dropped += usize::from(removed);
                    }
                    Err(()) => report.changed(&session.tab_id),
                }
                SessionFlushOutcome::Saved
            }
            Err(error) => SessionFlushOutcome::Failed { error },
        };
        report.outcomes.push(SessionFlushEntry {
            tab_id: session.tab_id.clone(),
            session_id: Some(session.id.clone()),
            outcome,
            draft_removed,
        });
    }
}

fn skip_snapshot_drafts(
    tabs: &HashMap<String, TabInfo>,
    snapshot_drafts: &HashMap<String, SessionDraft>,
    drafts: &Mutex<HashMap<String, SessionDraft>>,
    report: &mut SessionFlushOutcomeReport,
) {
    let mut ids: Vec<String> = tabs.keys().chain(snapshot_drafts.keys()).cloned().collect();
    ids.sort();
    ids.dedup();
    let mut current = drafts.lock().unwrap();
    for tab_id in ids {
        let draft_removed =
            match remove_matching_draft(&mut current, &tab_id, snapshot_drafts.get(&tab_id)) {
                Ok(removed) => removed,
                Err(()) => {
                    report.changed(&tab_id);
                    false
                }
            };
        report.summary.drafts_dropped += usize::from(draft_removed);
        report.outcomes.push(SessionFlushEntry {
            tab_id,
            session_id: None,
            outcome: SessionFlushOutcome::Skipped {
                reason: SessionFlushSkipReason::AutoSaveDisabled,
            },
            draft_removed,
        });
    }
}

/// Check both authoritative tabs and expected remaining drafts after all writes.
/// A new/closed/navigated tab or a replaced draft invalidates exit approval.
// This is a content comparison, not a new Browser->Session mutation/revision owner.
fn reconcile_snapshot_changes(
    before_tabs: &HashMap<String, TabInfo>,
    before_drafts: &HashMap<String, SessionDraft>,
    current_tabs: &HashMap<String, TabInfo>,
    current_drafts: &HashMap<String, SessionDraft>,
    report: &mut SessionFlushOutcomeReport,
) {
    let mut expected_drafts = before_drafts.clone();
    for entry in &report.outcomes {
        if entry.draft_removed {
            expected_drafts.remove(&entry.tab_id);
        }
    }
    for id in before_tabs.keys().chain(current_tabs.keys()) {
        if !same_tab(before_tabs.get(id), current_tabs.get(id)) {
            report.changed(id);
        }
    }
    for id in expected_drafts.keys().chain(current_drafts.keys()) {
        if !same_draft(expected_drafts.get(id), current_drafts.get(id)) {
            report.changed(id);
        }
    }
    report.changed_tabs.sort();
}

/// 保存当前 tab 为会话（立即落盘）。`preview` 为前端采集的最小文本预览
/// （可为空），落盘前由 `build_session` 做 URL 脱敏与 512B 截断。
#[tauri::command]
pub async fn session_save(
    app: AppHandle,
    webview: tauri::Webview,
    tab_id: String,
    preview: Option<String>,
) -> Result<SessionSummary, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "session_save")?;
    crate::shared::native_legacy_policy::bound_text("session_save", "tab_id", &tab_id)?;
    if let Some(value) = preview.as_deref() {
        crate::shared::native_legacy_policy::bound_text("session_save", "preview", value)?;
    }

    check_invocation_source(&webview, "session_save", None, &app)?;
    check_tab_id(&tab_id)?;
    SESSION_IO
        .run(move || {
            let _write = session_write_guard().map_err(str::to_string)?;
            if app
                .state::<crate::AppState>()
                .shutdown_requested
                .load(Ordering::Acquire)
            {
                return Err("SESSION_EXIT_ALREADY_ACCEPTED".to_string());
            }
            let text = preview.unwrap_or_default();
            let snapshot = build_session_for_tab(&app, &tab_id, &text, CLOSE_REASON_SAVED)
                .ok_or_else(|| "页签不存在".to_string())?;
            let session = &snapshot.session;
            persist_session(&app, session)?;
            // Explicit save is a point-in-time snapshot. A newer draft remains available.
            let _ = remove_matching_draft(
                &mut app
                    .state::<crate::AppState>()
                    .session_drafts
                    .lock()
                    .unwrap(),
                &tab_id,
                snapshot.draft.as_ref(),
            );
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
            Ok(crate::session::summarize(session))
        })
        .await
}

/// 显式丢弃内存草稿：不落盘。普通 Tab 关闭不调用保存或确认协议。
#[tauri::command]
pub async fn session_discard(
    app: AppHandle,
    webview: tauri::Webview,
    tab_id: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "session_discard")?;
    crate::shared::native_legacy_policy::bound_text("session_discard", "tab_id", &tab_id)?;

    check_invocation_source(&webview, "session_discard", None, &app)?;
    check_tab_id(&tab_id)?;
    SESSION_IO
        .run(move || {
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
        })
        .await
}

/// 列出本地会话存档（按 updated_at 倒序；损坏文件跳过）。
#[tauri::command]
pub async fn session_list(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<SessionSummary>, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "session_list")?;

    check_invocation_source(&webview, "session_list", None, &app)?;
    SESSION_IO
        .run(move || {
            Ok(crate::session::list_sessions(&workspace::sessions_dir(
                &app,
            )))
        })
        .await
}

/// 读取单个会话详情（含已脱敏资源列表）。
#[tauri::command]
pub async fn session_get(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<BrowserSession, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "session_get")?;
    crate::shared::native_legacy_policy::bound_text("session_get", "id", &id)?;

    check_invocation_source(&webview, "session_get", None, &app)?;
    crate::images::check_id(&id, "会话 id")?;
    SESSION_IO
        .run(move || crate::session::load_session(&workspace::sessions_dir(&app), &id))
        .await
}

/// 删除会话存档（幂等）。删除的是存档，不影响仍打开的同名页签。
#[tauri::command]
pub async fn session_delete(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    intent: Option<String>,
) -> Result<bool, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "session_delete")?;
    crate::shared::native_legacy_policy::bound_text("session_delete", "id", &id)?;

    check_invocation_source(&webview, "session_delete", None, &app)?;
    crate::images::check_id(&id, "会话 id")?;
    SESSION_IO
        .run(move || {
            let _write = session_write_guard().map_err(str::to_string)?;
            if app
                .state::<crate::AppState>()
                .shutdown_requested
                .load(Ordering::Acquire)
            {
                return Err("SESSION_EXIT_ALREADY_ACCEPTED".to_string());
            }
            app.state::<crate::security_policy::IntentRegistry>()
                .consume(
                    &format!("session_delete:{id}"),
                    intent
                        .as_deref()
                        .ok_or("NATIVE_USER_CONFIRMATION_REQUIRED")?,
                )
                .map_err(|_| "NATIVE_USER_CONFIRMATION_DENIED")?;
            let removed = crate::session::delete_session(&workspace::sessions_dir(&app), &id)?;
            workspace::log_audit(&app, "session_delete", format!("id={id} removed={removed}"));
            Ok(removed)
        })
        .await
}

/// 导出会话为脱敏 JSON 文本（**不写磁盘**：不引入新的路径写入面，
/// 由前端自行决定保存位置）。
#[tauri::command]
pub async fn session_export(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<String, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "session_export")?;
    crate::shared::native_legacy_policy::bound_text("session_export", "id", &id)?;

    check_invocation_source(&webview, "session_export", None, &app)?;
    crate::images::check_id(&id, "会话 id")?;
    SESSION_IO
        .run(move || {
            let session = crate::session::load_session(&workspace::sessions_dir(&app), &id)?;
            let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
            workspace::log_audit(
                &app,
                "session_export",
                format!("id={id} bytes={}", json.len()),
            );
            Ok(json)
        })
        .await
}

/// 用会话中**已脱敏**的 URL 新建页签（重启/回看场景）。
/// 边界：URL 在落盘时已脱敏，一次性 token / 登录态参数不会恢复，
/// 因此还原出的页面可能需要重新登录——这是隐私红线的必然代价，不伪造原 URL。
#[tauri::command]
pub async fn session_restore(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<TabInfo, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "session_restore")?;
    crate::shared::native_legacy_policy::bound_text("session_restore", "id", &id)?;

    check_invocation_source(&webview, "session_restore", None, &app)?;
    crate::images::check_id(&id, "会话 id")?;
    // Disk access and audit remain in the bounded owner. Native creation retains
    // the original main-thread boundary; no session/tab mutex crosses the wait.
    SESSION_IO
        .run(move || {
            let session = crate::session::load_session(&workspace::sessions_dir(&app), &id)?;
            let url = session.url.clone();
            let native_app = app.clone();
            let (send, receive) = std::sync::mpsc::sync_channel(1);
            app.run_on_main_thread(move || {
                let _ = send.send(create_tab(native_app, &url));
            })
            .map_err(|_| "SESSION_RESTORE_DISPATCH_FAILED")?;
            let tab = receive.recv().map_err(|_| "SESSION_RESTORE_REPLY_LOST")??;
            workspace::log_audit(
                &app,
                "session_restore",
                format!(
                    "id={id} tab_id={} resources={}",
                    tab.id, session.resource_count
                ),
            );
            Ok(tab)
        })
        .await
}

/// Explicit flush keeps the existing Tauri result shape. A partial save or a
/// concurrent state change is an Err, never a successful count-only response.
#[tauri::command]
pub async fn flush_sessions(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<SessionFlushReport, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "flush_sessions")?;

    check_invocation_source(&webview, "flush_sessions", None, &app)?;
    SESSION_IO
        .run(move || flush_sessions_inner(&app).into_public_report())
        .await
}

/// Ordinary flush only reports persistence; it never accepts application exit.
pub(crate) fn flush_sessions_inner(app: &AppHandle) -> SessionFlushOutcomeReport {
    flush_sessions_with_intent(app, false)
}

/// Application-exit preflight. Success commits the existing bridge admission
/// flag while the writer, drafts and authoritative tabs locks are still held.
/// Main must check can_exit() before starting any irreversible native cleanup.
pub(crate) fn prepare_exit_sessions(app: &AppHandle) -> SessionFlushOutcomeReport {
    flush_sessions_with_intent(app, true)
}

fn flush_sessions_with_intent(app: &AppHandle, accept_exit: bool) -> SessionFlushOutcomeReport {
    let state = app.state::<crate::AppState>();
    let accepted_report = || {
        let mut report = SessionFlushOutcomeReport::empty();
        for tab_id in state.tabs.lock().unwrap().keys() {
            report.outcomes.push(SessionFlushEntry {
                tab_id: tab_id.clone(),
                session_id: None,
                outcome: SessionFlushOutcome::Skipped {
                    reason: SessionFlushSkipReason::ExitAlreadyAccepted,
                },
                draft_removed: false,
            });
        }
        report
    };
    // Once exit is accepted, even a concurrent caller holding the writer mutex
    // must not turn this idempotent no-op into another fallible save.
    if state.shutdown_requested.load(Ordering::Acquire) {
        return accepted_report();
    }
    let _write = match session_write_guard() {
        Ok(guard) => guard,
        Err(code) => return SessionFlushOutcomeReport::unavailable(code),
    };
    if state.shutdown_requested.load(Ordering::Acquire) {
        return accepted_report();
    }
    let mut report = SessionFlushOutcomeReport::empty();

    let auto_save = state.session_auto_save_on_exit.load(Ordering::SeqCst);
    let (tabs, snapshot_drafts) = {
        // Fixed lock order; hold only while cloning, never during disk I/O.
        let drafts = state.session_drafts.lock().unwrap();
        let tabs = state.tabs.lock().unwrap();
        (tabs.clone(), drafts.clone())
    };
    if auto_save {
        let mut ids: Vec<&String> = tabs.keys().collect();
        ids.sort();
        let snapshots: Vec<SessionSaveSnapshot> = ids
            .into_iter()
            .map(|id| {
                let records = state.resource_buffer.lock().unwrap().records(id);
                build_session_snapshot(
                    &tabs[id],
                    snapshot_drafts.get(id),
                    "",
                    CLOSE_REASON_SHUTDOWN,
                    &records,
                )
            })
            .collect();
        persist_snapshots(&snapshots, &state.session_drafts, &mut report, |session| {
            persist_session(app, session)
        });
        // Only open Browser tabs participate in opt-in exit saving. A legacy
        // orphan draft is not permission to restore or save an already closed tab.
        for tab_id in snapshot_drafts.keys().filter(|id| !tabs.contains_key(*id)) {
            report.outcomes.push(SessionFlushEntry {
                tab_id: tab_id.clone(),
                session_id: None,
                outcome: SessionFlushOutcome::Skipped {
                    reason: SessionFlushSkipReason::NoOpenTab,
                },
                draft_removed: false,
            });
        }
    } else {
        // Preserve the opt-out: no snapshot is serialized or written.
        skip_snapshot_drafts(&tabs, &snapshot_drafts, &state.session_drafts, &mut report);
    }

    // Startup owns orphan .tmp cleanup. Removing .tmp files from a live flush
    // could delete an active atomic-write staging file from another process.
    // Keep tmp_removed at zero. Audit only completed save outcomes here; no
    // fallible work or I/O may follow the final exit-acceptance decision below.
    workspace::log_audit(
        app,
        "session_flush",
        format!(
            "phase=save auto_save={auto_save} persisted={} failed={} skipped={} drafts_dropped={}",
            report.summary.persisted,
            report.failed_count(),
            report.skipped_count(),
            report.summary.drafts_dropped,
        ),
    );

    {
        let drafts = state.session_drafts.lock().unwrap();
        let current_tabs = state.tabs.lock().unwrap();
        reconcile_snapshot_changes(&tabs, &snapshot_drafts, &current_tabs, &drafts, &mut report);
        // set_session_policy shares SESSION_WRITES, so it cannot change the
        // policy between this check and acceptance of the same snapshot.
        report.policy_changed = auto_save != state.session_auto_save_on_exit.load(Ordering::SeqCst);
        if accept_exit && report.can_exit() {
            state.shutdown_requested.store(true, Ordering::Release);
        }
    }
    // All later native cleanup runs outside these locks. No new shutdown latch
    // was introduced; failure leaves the existing bridge gate open for retry.
    report
}

/// 查询会话策略（关闭弹窗 / 退出自动保存）。
#[tauri::command]
pub fn get_session_policy(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<SessionPolicy, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "get_session_policy")?;

    check_invocation_source(&webview, "get_session_policy", None, &app)?;
    let state = app.state::<crate::AppState>();
    Ok(SessionPolicy {
        close_prompt: state.session_close_prompt.load(Ordering::SeqCst),
        auto_save_on_exit: state.session_auto_save_on_exit.load(Ordering::SeqCst),
    })
}

/// 设置会话策略（审计记录开关变化，不含任何 URL/预览内容）。
#[tauri::command]
pub async fn set_session_policy(
    app: AppHandle,
    webview: tauri::Webview,
    close_prompt: Option<bool>,
    auto_save_on_exit: Option<bool>,
) -> Result<SessionPolicy, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "set_session_policy")?;

    check_invocation_source(&webview, "set_session_policy", None, &app)?;
    SESSION_IO
        .run(move || {
            let _write = session_write_guard().map_err(str::to_string)?;
            let state = app.state::<crate::AppState>();
            if state.shutdown_requested.load(Ordering::Acquire) {
                return Err("SESSION_EXIT_ALREADY_ACCEPTED".to_string());
            }
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
        })
        .await
}

#[cfg(test)]
mod flush_tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    struct SessionFixture(PathBuf);

    impl SessionFixture {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("browseros-session-flush-{}", uuid::Uuid::new_v4(),));
            fs::create_dir_all(&path).expect("create session fixture");
            Self(path)
        }

        fn persist(&self, session: &BrowserSession) -> Result<usize, String> {
            crate::session::save_session(&self.0.join("sessions"), session).map(|_| 0)
        }

        fn directory(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for SessionFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn tab(id: &str) -> TabInfo {
        TabInfo {
            id: id.to_string(),
            url: "https://example.invalid/page?token=private-value".to_string(),
            title: "original".to_string(),
        }
    }

    fn draft(tab: &TabInfo) -> SessionDraft {
        SessionDraft {
            tab_id: tab.id.clone(),
            url: crate::security_policy::redact_sensitive_url(&tab.url),
            title: tab.title.clone(),
        }
    }

    #[test]
    fn exit_flush_reports_partial_write_failure_and_retains_failed_draft() {
        let fixture = SessionFixture::new();
        let first = tab("tab-1");
        let second = tab("tab-2");
        let first_draft = draft(&first);
        let second_draft = draft(&second);
        let snapshots = vec![
            build_session_snapshot(&first, Some(&first_draft), "", CLOSE_REASON_SHUTDOWN, &[]),
            build_session_snapshot(&second, Some(&second_draft), "", CLOSE_REASON_SHUTDOWN, &[]),
        ];
        let drafts = Mutex::new(HashMap::from([
            (first.id.clone(), first_draft),
            (second.id.clone(), second_draft),
        ]));
        // A regular file cannot be a directory: a real, portable filesystem
        // error, independent of elevated-user permission semantics.
        let blocked = fixture.directory().join("blocked-directory");
        fs::write(&blocked, b"not a directory").expect("create failure fixture");
        let mut report = SessionFlushOutcomeReport::empty();
        persist_snapshots(&snapshots, &drafts, &mut report, |session| {
            if session.tab_id == second.id {
                crate::session::save_session(&blocked, session).map(|_| 0)
            } else {
                fixture.persist(session)
            }
        });

        assert_eq!(report.summary.persisted, 1);
        assert_eq!(report.summary.drafts_dropped, 1);
        assert_eq!(report.failed_count(), 1);
        assert!(!report.can_exit());
        assert!(!drafts.lock().unwrap().contains_key(&first.id));
        assert!(drafts.lock().unwrap().contains_key(&second.id));
        assert_eq!(report.outcomes[1].tab_id, second.id);
        assert_eq!(
            report.outcomes[1].session_id.as_deref(),
            Some(snapshots[1].session.id.as_str())
        );
        assert!(matches!(&report.outcomes[1].outcome,
            SessionFlushOutcome::Failed { error } if !error.is_empty()));
        let saved = crate::session::load_session(
            &fixture.directory().join("sessions"),
            &snapshots[0].session.id,
        )
        .expect("successful sibling is actually persisted");
        assert_eq!(saved.tab_id, first.id);
        assert!(!saved.url.contains("private-value"));
        assert!(report.into_public_report().is_err());
    }

    #[test]
    fn exit_flush_keeps_a_draft_replaced_during_persist() {
        let fixture = SessionFixture::new();
        let original = tab("tab-3");
        let original_draft = draft(&original);
        let snapshot = build_session_snapshot(
            &original,
            Some(&original_draft),
            "",
            CLOSE_REASON_SHUTDOWN,
            &[],
        );
        let drafts = Mutex::new(HashMap::from([(original.id.clone(), original_draft)]));
        let mut newer = draft(&original);
        newer.title = "updated while writing".to_string();
        let mut report = SessionFlushOutcomeReport::empty();
        persist_snapshots(&[snapshot], &drafts, &mut report, |session| {
            let result = fixture.persist(session)?;
            drafts
                .lock()
                .unwrap()
                .insert(original.id.clone(), newer.clone());
            Ok(result)
        });

        assert_eq!(report.summary.persisted, 1);
        assert_eq!(report.summary.drafts_dropped, 0);
        assert_eq!(report.changed_tabs, vec![original.id.clone()]);
        assert_eq!(report.failed_count(), 0);
        assert!(!report.can_exit());
        assert_eq!(drafts.lock().unwrap()[&original.id].title, newer.title);
    }

    #[test]
    fn exit_flush_keeps_a_draft_created_after_snapshot() {
        let fixture = SessionFixture::new();
        let original = tab("tab-4");
        let snapshot = build_session_snapshot(&original, None, "", CLOSE_REASON_SHUTDOWN, &[]);
        let drafts = Mutex::new(HashMap::new());
        let mut report = SessionFlushOutcomeReport::empty();
        persist_snapshots(&[snapshot], &drafts, &mut report, |session| {
            let result = fixture.persist(session)?;
            drafts
                .lock()
                .unwrap()
                .insert(original.id.clone(), draft(&original));
            Ok(result)
        });

        assert_eq!(report.summary.persisted, 1);
        assert_eq!(report.summary.drafts_dropped, 0);
        assert_eq!(report.changed_count(), 1);
        assert!(!report.can_exit());
        assert!(drafts.lock().unwrap().contains_key(&original.id));
    }

    #[test]
    fn exit_flush_detects_changed_closed_and_new_authoritative_tabs() {
        let first = tab("tab-5");
        let second = tab("tab-6");
        let third = tab("tab-7");
        let before = HashMap::from([
            (first.id.clone(), first.clone()),
            (second.id.clone(), second),
        ]);
        let mut navigated = first;
        navigated.url = "https://example.invalid/changed".to_string();
        let current = HashMap::from([(navigated.id.clone(), navigated), (third.id.clone(), third)]);
        let mut report = SessionFlushOutcomeReport::empty();
        reconcile_snapshot_changes(
            &before,
            &HashMap::new(),
            &current,
            &HashMap::new(),
            &mut report,
        );

        assert_eq!(report.changed_tabs, vec!["tab-5", "tab-6", "tab-7"]);
        assert!(!report.can_exit());
        report.policy_changed = true;
        assert_eq!(report.changed_count(), 4);
    }

    #[test]
    fn exit_flush_disabled_policy_reports_skips_and_preserves_newer_drafts() {
        let original = tab("tab-8");
        let before_draft = draft(&original);
        let before = HashMap::from([(original.id.clone(), before_draft)]);
        let mut newer = draft(&original);
        newer.title = "new draft".to_string();
        let drafts = Mutex::new(HashMap::from([(original.id.clone(), newer.clone())]));
        let tabs = HashMap::from([(original.id.clone(), original.clone())]);
        let mut report = SessionFlushOutcomeReport::empty();
        // The disabled-policy operation has no persistence callback at all.
        skip_snapshot_drafts(&tabs, &before, &drafts, &mut report);

        assert_eq!(report.summary.persisted, 0);
        assert_eq!(report.summary.drafts_dropped, 0);
        assert_eq!(report.skipped_count(), 1);
        assert_eq!(report.outcomes[0].session_id, None);
        assert!(matches!(
            &report.outcomes[0].outcome,
            SessionFlushOutcome::Skipped {
                reason: SessionFlushSkipReason::AutoSaveDisabled
            }
        ));
        assert_eq!(drafts.lock().unwrap()[&original.id].title, newer.title);
        assert!(!report.can_exit());
    }

    #[test]
    fn exit_flush_public_result_preserves_shape_and_redacts_failure_details() {
        let public = SessionFlushOutcomeReport::empty()
            .into_public_report()
            .unwrap();
        let value = serde_json::to_value(public).expect("serialize legacy DTO");
        let keys: std::collections::BTreeSet<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            std::collections::BTreeSet::from([
                "persisted",
                "drafts_dropped",
                "tmp_removed",
                "capacity_removed",
            ])
        );

        let mut report = SessionFlushOutcomeReport::empty();
        report.outcomes.push(SessionFlushEntry {
            tab_id: "tab-9".to_string(),
            session_id: Some("session-id".to_string()),
            outcome: SessionFlushOutcome::Failed {
                error: "native detail with https://example.invalid/?token=private-value"
                    .to_string(),
            },
            draft_removed: false,
        });
        let error = report.into_public_report().unwrap_err();
        assert!(error.starts_with("SESSION_FLUSH_INCOMPLETE "));
        assert!(error.contains("failed=1"));
        assert!(!error.contains("https://"));
        assert!(!error.contains("private-value"));
        assert!(!SessionFlushOutcomeReport::unavailable("SESSION_FLUSH_BUSY").can_exit());
    }
}
