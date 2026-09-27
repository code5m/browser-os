//! Task 能力原生命令层（CAPABILITY_NATIVE_ADAPTER）。
//!
//! 承载 `task_list` / `task_add` / `task_update` / `task_remove` / `task_run_now`
//! 五个命令及其私有 helper（`task_trigger_label` / `task_target_params`）。
//! 命令体委托 `crate::tasks`（持久化 + 校验）与 `crate::scheduler`（执行引擎）；
//! 触发语义归属 `scheduler.rs`，本层只做 CRUD 与「立即跑一次」入口，执行唯一入口
//! 仍是 `script_runner`（F6）。
//!
//! 迁移自 `src-tauri/src/bridge.rs`（Native Physical Boundary Matrix /
//! native-physical-batch-task）。

use std::collections::HashMap;

use tauri::{AppHandle, Webview};

use crate::domain::{
    MissedRunPolicy, RetryPolicy, ScriptParam, TaskDef, TaskKind, TaskTrigger,
};
use crate::images::check_id;
use crate::script_runner::RunSnapshot;
use crate::shared::invocation::check_invocation_source;
use crate::workspace;

/// 触发方式的审计标签。**不含**用户输入的表达式正文（避免把任意字符串塞进审计）。
fn task_trigger_label(trigger: &TaskTrigger) -> &'static str {
    match trigger {
        TaskTrigger::Cron { .. } => "cron",
        TaskTrigger::Interval { .. } => "interval",
    }
}

/// 取目标（脚本 / 命令片段）的参数定义，用于 R-3 / R-4 校验。
fn task_target_params(
    app: &AppHandle,
    kind: TaskKind,
    target_id: &str,
) -> Result<Vec<ScriptParam>, String> {
    match kind {
        TaskKind::Script => {
            let script = workspace::load_scripts(app)
                .into_iter()
                .find(|s| s.id == target_id)
                .ok_or_else(|| crate::tasks::TASK_TARGET_NOT_FOUND.to_string())?;
            if !script.enabled {
                return Err("SCRIPT_DISABLED".to_string());
            }
            Ok(script.params)
        }
        TaskKind::Command => {
            let snippet = workspace::load_snippets(app)
                .into_iter()
                .find(|s| s.id == target_id)
                .ok_or_else(|| crate::tasks::TASK_TARGET_NOT_FOUND.to_string())?;
            if !snippet.enabled {
                return Err("SNIPPET_DISABLED".to_string());
            }
            Ok(snippet.params)
        }
    }
}

/// 任务列表。**不含** secret 参数值 —— secret 参数在定义期即被拒绝落盘（契约 §3.3 R-3）。
#[tauri::command]
pub fn task_list(app: AppHandle, webview: Webview) -> Result<Vec<TaskDef>, String> {
    check_invocation_source(&webview, "task_list", None, &app)?;
    let _store_guard = crate::tasks::task_store_lock();
    let list = crate::tasks::load_tasks_at(&crate::tasks::tasks_file(&app));
    let count = list.len();
    workspace::log_audit(&app, "task.runs.list", format!("count={count}"));
    Ok(list)
}

/// 新建任务。默认 **不启用**（裁定 R-A6-1：自动执行必须是显式动作）；
/// 创建时即计算并落盘 `next_run_at`，便于 UI 展示「下次执行时间」。
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn task_add(
    app: AppHandle,
    webview: Webview,
    name: String,
    kind: TaskKind,
    target_id: String,
    trigger: TaskTrigger,
    params: Option<HashMap<String, String>>,
    missed_run_policy: Option<MissedRunPolicy>,
    catch_up_limit: Option<u32>,
    misfire_grace_secs: Option<u64>,
    retry: Option<RetryPolicy>,
    timeout_secs: Option<u32>,
    enabled: Option<bool>,
) -> Result<TaskDef, String> {
    check_invocation_source(&webview, "task_add", None, &app)?;
    let _store_guard = crate::tasks::task_store_lock();
    let supplied = params.unwrap_or_default();
    let path = crate::tasks::tasks_file(&app);
    let mut list = crate::tasks::load_tasks_at(&path);
    crate::tasks::check_capacity(list.len()).map_err(|e| e.code().to_string())?;
    let meta = task_target_params(&app, kind, &target_id)?;
    crate::tasks::validate_params(&meta, &supplied).map_err(|e| e.code().to_string())?;

    let now = chrono::Utc::now();
    let mut task = TaskDef {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        kind,
        target_id,
        params: supplied,
        enabled: enabled.unwrap_or(false),
        trigger,
        missed_run_policy: missed_run_policy.unwrap_or_default(),
        catch_up_limit: catch_up_limit.unwrap_or(3),
        misfire_grace_secs: misfire_grace_secs.unwrap_or(60),
        retry: retry.unwrap_or_default(),
        timeout_secs: timeout_secs.unwrap_or(0),
        last_fired_at: None,
        next_run_at: None,
        created_at: now,
        updated_at: now,
    };
    crate::tasks::validate_task(&task).map_err(|e| e.code().to_string())?;
    task.next_run_at = crate::tasks::next_fire_after(&task.trigger, now);
    list.push(task.clone());
    crate::tasks::save_tasks_at(&path, &list)
        .map_err(|_| crate::tasks::TASK_PERSIST_FAILED.to_string())?;

    let kind_label = task.kind.as_str();
    let trig_label = task_trigger_label(&task.trigger);
    let id = task.id.clone();
    workspace::log_audit(
        &app,
        "task.add",
        format!("id={id} kind={kind_label} trigger={trig_label}"),
    );
    Ok(task)
}

/// 全量更新（按 id 替换）。`created_at` 由服务端保留，不可被前端改写；
/// 改 `trigger` 后重算 `next_run_at`（契约 §5.4）。
#[tauri::command]
pub fn task_update(
    app: AppHandle,
    webview: Webview,
    task: TaskDef,
) -> Result<TaskDef, String> {
    check_invocation_source(&webview, "task_update", None, &app)?;
    let _store_guard = crate::tasks::task_store_lock();
    let path = crate::tasks::tasks_file(&app);
    let mut list = crate::tasks::load_tasks_at(&path);
    let index = list
        .iter()
        .position(|t| t.id == task.id)
        .ok_or_else(|| crate::tasks::TASK_NOT_FOUND.to_string())?;
    // A10 R-4：参数校验在**更新期同样复跑** —— 防止脚本事后把参数改标 secret，
    // 而既有任务仍持有明文值。
    let meta = task_target_params(&app, task.kind, &task.target_id)?;
    crate::tasks::validate_params(&meta, &task.params).map_err(|e| e.code().to_string())?;
    crate::tasks::validate_task(&task).map_err(|e| e.code().to_string())?;

    let created_at = list[index].created_at;
    let mut next = task;
    next.created_at = created_at;
    next.updated_at = chrono::Utc::now();
    next.next_run_at = crate::tasks::next_fire_after(&next.trigger, next.updated_at);
    list[index] = next.clone();
    crate::tasks::save_tasks_at(&path, &list)
        .map_err(|_| crate::tasks::TASK_PERSIST_FAILED.to_string())?;

    let kind_label = next.kind.as_str();
    let trig_label = task_trigger_label(&next.trigger);
    let id = next.id.clone();
    workspace::log_audit(
        &app,
        "task.update",
        format!("id={id} kind={kind_label} trigger={trig_label}"),
    );
    Ok(next)
}

/// 删除任务：**先取消该任务的在飞运行**，再删持久化条目（契约 §5.4）。
/// `task-runs.json` 的历史**保留**（审计与排障需要），不随任务删除。
/// 幂等：id 不存在返回稳定错误码，不 panic。
#[tauri::command]
pub fn task_remove(app: AppHandle, webview: Webview, id: String) -> Result<bool, String> {
    check_invocation_source(&webview, "task_remove", None, &app)?;
    let _store_guard = crate::tasks::task_store_lock();
    check_id(&id, "任务 id")?;
    let path = crate::tasks::tasks_file(&app);
    let mut list = crate::tasks::load_tasks_at(&path);
    let before = list.len();
    let cancelled = crate::scheduler::cancel_in_flight(&app, &id);
    list.retain(|t| t.id != id);
    if list.len() == before {
        return Err(crate::tasks::TASK_NOT_FOUND.to_string());
    }
    crate::tasks::save_tasks_at(&path, &list)
        .map_err(|_| crate::tasks::TASK_PERSIST_FAILED.to_string())?;
    workspace::log_audit(
        &app,
        "task.remove",
        format!("id={id} cancelled={cancelled}"),
    );
    Ok(true)
}

/// 立即触发一次（`trigger = Manual`）。**不推进** `last_fired_at` / `next_run_at`；
/// 同样受「同任务 in_flight」与全局并发约束，冲突时返回 `TASK_ALREADY_RUNNING`。
#[tauri::command]
pub fn task_run_now(
    app: AppHandle,
    webview: Webview,
    id: String,
) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "task_run_now", None, &app)?;
    let _store_guard = crate::tasks::task_store_lock();
    check_id(&id, "任务 id")?;
    crate::scheduler::fire_now(&app, &id)
}
