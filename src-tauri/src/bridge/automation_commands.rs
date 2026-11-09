//! Script and command-snippet CRUD and execution commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

/// 列出全部脚本元数据（只读，不审计：避免刷满 audit 的 1000 条上限）。
#[tauri::command]
pub fn script_list(app: AppHandle, webview: tauri::Webview) -> Result<Vec<ScriptMeta>, String> {
    check_invocation_source(&webview, "script_list", None, &app)?;
    Ok(workspace::load_scripts(&app))
}

/// 新增脚本：先校验后落盘；元数据保存失败时回滚已写的正文，不留孤儿文件。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn script_add(
    app: AppHandle,
    webview: tauri::Webview,
    name: String,
    category: String,
    interpreter: ScriptInterpreter,
    body: String,
    params: Vec<ScriptParam>,
    description: Option<String>,
    timeout_secs: Option<u32>,
) -> Result<ScriptMeta, String> {
    check_invocation_source(&webview, "script_add", None, &app)?;

    let mut list = workspace::load_scripts(&app);
    if list.len() >= crate::scripts::MAX_SCRIPTS {
        return Err(crate::scripts::ScriptError::TooManyScripts.to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let meta = ScriptMeta {
        id: id.clone(),
        name,
        category,
        path: format!("{}.{}", id, interpreter.ext()),
        interpreter,
        params,
        description: description.unwrap_or_default(),
        builtin: false,
        enabled: true,
        timeout_secs: timeout_secs.unwrap_or(0),
        created_at: now,
        updated_at: now,
    };

    crate::scripts::validate_meta(&meta).map_err(|e| e.to_string())?;
    crate::scripts::validate_body(&body).map_err(|e| e.to_string())?;

    // 先写正文（可失败、无副作用残留），再改元数据；元数据保存失败则回滚正文
    workspace::write_script_body(&app, &meta.path, &body)?;
    list.push(meta.clone());
    if let Err(e) = workspace::save_scripts(&app, &list) {
        let _ = workspace::delete_script_body(&app, &meta.path);
        return Err(e);
    }

    workspace::log_audit(
        &app,
        "script.add",
        format!(
            "id={id} name={} interpreter={:?} param_count={}",
            meta.name,
            meta.interpreter,
            meta.params.len()
        ),
    );
    Ok(meta)
}

/// 更新脚本元数据（**解释器不可更改**：改解释器要换扩展名与文件名，
/// 语义超出本卡范围）。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn script_update(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    name: String,
    category: String,
    description: Option<String>,
    params: Vec<ScriptParam>,
    timeout_secs: Option<u32>,
    enabled: Option<bool>,
    body: Option<String>,
) -> Result<ScriptMeta, String> {
    check_invocation_source(&webview, "script_update", None, &app)?;
    crate::images::check_id(&id, "脚本 id")?;

    let mut list = workspace::load_scripts(&app);
    let existing = list
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "脚本不存在".to_string())?;

    let mut meta = existing.clone();
    meta.name = name;
    meta.category = category;
    meta.description = description.unwrap_or_default();
    meta.params = params;
    meta.timeout_secs = timeout_secs.unwrap_or(0);
    if let Some(en) = enabled {
        meta.enabled = en;
    }
    meta.updated_at = chrono::Utc::now();

    crate::scripts::validate_meta(&meta).map_err(|e| e.to_string())?;

    // 正文可选更新：传了才校验并落盘
    if let Some(b) = &body {
        crate::scripts::validate_body(b).map_err(|e| e.to_string())?;
        workspace::write_script_body(&app, &meta.path, b)?;
    }

    let updated = meta.clone();
    for s in list.iter_mut() {
        if s.id == id {
            *s = meta.clone();
        }
    }
    workspace::save_scripts(&app, &list)?;

    workspace::log_audit(
        &app,
        "script.update",
        format!(
            "id={id} name={} param_count={} builtin={}",
            updated.name,
            updated.params.len(),
            updated.builtin
        ),
    );
    Ok(updated)
}

/// 删除脚本（内置脚本拒绝删除；正文删除幂等）。
#[tauri::command]
pub fn script_remove(app: AppHandle, webview: tauri::Webview, id: String) -> Result<(), String> {
    check_invocation_source(&webview, "script_remove", None, &app)?;
    crate::images::check_id(&id, "脚本 id")?;

    let mut list = workspace::load_scripts(&app);
    let existing = list
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "脚本不存在".to_string())?;
    // 删除前置判定复用纯函数，避免「能否删除」的规则在两处各写一遍
    crate::scripts::can_delete(existing).map_err(|e| e.to_string())?;
    let file_name = existing.path.clone();

    list.retain(|s| s.id != id);
    workspace::save_scripts(&app, &list)?;
    // 元数据已移除，正文删不掉也只是残留文件，不影响功能（幂等）
    let _ = workspace::delete_script_body(&app, &file_name);

    workspace::log_audit(&app, "script.remove", format!("id={id} builtin=false"));
    Ok(())
}

// ---------------------------------------------------------------------------
// M2-6.b 命令片段持久化与 CRUD
//
// 契约见 `logs/checkpoints/M2-6-20260905-1700.md` §7（M2-6.a 冻结）与
// `logs/checkpoints/B-M2-6.a-command-snippet-contract-20260905-1710.md`。
// 范围：只做 `snippets.json` 的增删改查，**不接执行**（执行接入归 M2-6.c）。
// 与脚本库的差异：命令片段**无正文文件**，故保存是单阶段原子写，
// 不存在「先写正文、失败回滚」的两阶段提交。
// 审计 detail 只含 id/name/dangerous，**不含 argv 与参数值**（命令行可能含
// secret 参数，写盘即等于泄露；F7）。
// ---------------------------------------------------------------------------

/// 列出全部命令片段。
#[tauri::command]
pub fn snippet_list(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<CommandSnippet>, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "snippet_list")?;

    check_invocation_source(&webview, "snippet_list", None, &app)?;
    Ok(crate::snippets::merge_builtin_snippets(
        workspace::load_snippets(&app),
    ))
}

/// 新增命令片段。
#[tauri::command]
pub fn snippet_add(
    app: AppHandle,
    webview: tauri::Webview,
    name: String,
    category: String,
    interpreter: ScriptInterpreter,
    argv: Vec<String>,
    params: Vec<ScriptParam>,
    description: Option<String>,
    dangerous: Option<bool>,
    // M2-6-fix1（复核 F-1）：新建态也接受 `enabled`。此前后端硬编码 `true`，
    // 导致前端新建表单的「启用」勾选被静默丢弃（UI 承诺与后端行为不一致）。
    // 缺省仍为 `true`，不改变既有调用方行为。
    enabled: Option<bool>,
    timeout_secs: Option<u32>,
) -> Result<CommandSnippet, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "snippet_add")?;
    crate::shared::native_legacy_policy::bound_text("snippet_add", "name", &name)?;
    crate::shared::native_legacy_policy::bound_text("snippet_add", "category", &category)?;
    if argv.len() > 128 {
        return Err("NATIVE_LEGACY_LIST_LIMIT:snippet_add:argv".into());
    }
    for element in &argv {
        crate::shared::native_legacy_policy::bound_text("snippet_add", "argv", element)?;
    }
    if let Some(value) = description.as_deref() {
        crate::shared::native_legacy_policy::bound_text("snippet_add", "description", value)?;
    }

    check_invocation_source(&webview, "snippet_add", None, &app)?;

    let mut list = workspace::load_snippets(&app);
    if list.len() >= crate::snippets::MAX_SNIPPETS {
        return Err(crate::snippets::SnippetError::TooManySnippets.to_string());
    }

    let now = chrono::Utc::now();
    let snippet = CommandSnippet {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        category,
        interpreter,
        argv,
        params,
        description: description.unwrap_or_default(),
        dangerous: dangerous.unwrap_or(false),
        builtin: false,
        enabled: enabled.unwrap_or(true),
        timeout_secs: timeout_secs.unwrap_or(0),
        created_at: now,
        updated_at: now,
    };

    crate::snippets::validate_snippet(&snippet).map_err(|e| e.to_string())?;

    list.push(snippet.clone());
    workspace::save_snippets(&app, &list)?;

    workspace::log_audit(
        &app,
        "cmd.add",
        format!(
            "id={} name={} dangerous={}",
            snippet.id, snippet.name, snippet.dangerous
        ),
    );
    Ok(snippet)
}

/// 更新命令片段（整字段覆盖；`enabled` 不传则保持原值）。
///
/// 内置片段允许改名/改参数，但**不可删除**（`snippets::can_delete`）。
#[tauri::command]
pub fn snippet_update(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    name: String,
    category: String,
    interpreter: ScriptInterpreter,
    argv: Vec<String>,
    params: Vec<ScriptParam>,
    description: Option<String>,
    dangerous: Option<bool>,
    enabled: Option<bool>,
    timeout_secs: Option<u32>,
) -> Result<CommandSnippet, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "snippet_update")?;
    crate::shared::native_legacy_policy::bound_text("snippet_update", "id", &id)?;
    crate::shared::native_legacy_policy::bound_text("snippet_update", "name", &name)?;
    crate::shared::native_legacy_policy::bound_text("snippet_update", "category", &category)?;
    if argv.len() > 128 {
        return Err("NATIVE_LEGACY_LIST_LIMIT:snippet_update:argv".into());
    }
    for element in &argv {
        crate::shared::native_legacy_policy::bound_text("snippet_update", "argv", element)?;
    }
    if let Some(value) = description.as_deref() {
        crate::shared::native_legacy_policy::bound_text("snippet_update", "description", value)?;
    }

    check_invocation_source(&webview, "snippet_update", None, &app)?;
    crate::images::check_id(&id, "命令片段 id")?;
    // M2-6.e：内置片段（id 带 `builtin:` 前缀）不可经用户接口修改，防命名空间冲突。
    if id.starts_with(crate::snippets::BUILTIN_SNIPPET_ID_PREFIX) {
        return Err("内置片段不可修改".to_string());
    }

    let mut list = workspace::load_snippets(&app);
    let pos = list
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| "命令片段不存在".to_string())?;

    let mut snip = list[pos].clone();
    snip.name = name;
    snip.category = category;
    snip.interpreter = interpreter;
    snip.argv = argv;
    snip.params = params;
    snip.description = description.unwrap_or_default();
    if let Some(d) = dangerous {
        snip.dangerous = d;
    }
    if let Some(en) = enabled {
        snip.enabled = en;
    }
    snip.timeout_secs = timeout_secs.unwrap_or(0);
    snip.updated_at = chrono::Utc::now();

    crate::snippets::validate_snippet(&snip).map_err(|e| e.to_string())?;

    list[pos] = snip.clone();
    workspace::save_snippets(&app, &list)?;

    workspace::log_audit(
        &app,
        "cmd.update",
        format!(
            "id={} name={} dangerous={}",
            snip.id, snip.name, snip.dangerous
        ),
    );
    Ok(snip)
}

/// 删除命令片段（内置片段拒绝）。
#[tauri::command]
pub fn snippet_remove(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    intent: Option<String>,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "snippet_remove")?;
    crate::shared::native_legacy_policy::bound_text("snippet_remove", "id", &id)?;

    check_invocation_source(&webview, "snippet_remove", None, &app)?;
    crate::images::check_id(&id, "命令片段 id")?;

    let mut list = workspace::load_snippets(&app);
    let existing = list
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "命令片段不存在".to_string())?;
    // 删除前置判定复用纯函数，避免「能否删除」的规则在两处各写一遍
    crate::snippets::can_delete(existing).map_err(|e| e.to_string())?;
    app.state::<crate::security_policy::IntentRegistry>()
        .consume(
            &format!("snippet_remove:{id}"),
            intent
                .as_deref()
                .ok_or("NATIVE_USER_CONFIRMATION_REQUIRED")?,
        )
        .map_err(|_| "NATIVE_USER_CONFIRMATION_DENIED")?;

    list.retain(|s| s.id != id);
    workspace::save_snippets(&app, &list)?;

    workspace::log_audit(&app, "cmd.remove", format!("id={id} builtin=false"));
    Ok(())
}

// ---------------------------------------------------------------------------
// M2-4.c 脚本执行命令接入（契约见 logs/checkpoints/M2-4.a-20260903-2233.md §8.8）
//
// 范围：只把 `run_script` / `cancel_script` / `script_status` 接到 M2-4.b 的
// `script_runner` 内核，并补齐来源校验、参数校验、审计、ACL 与 handler。
// 输出环形缓冲、事件流、运行记录落盘与 shutdown 注册仍归 M2-4.d。
// 审计 detail 只含 id/run_id/状态/计数，不含参数值、脚本正文、URL 或任何凭据。
// ---------------------------------------------------------------------------

fn map_run_error(e: RunError) -> String {
    e.to_string()
}

fn get_enabled_script(app: &AppHandle, id: &str) -> Result<ScriptMeta, String> {
    crate::images::check_id(id, "脚本 id")?;
    let script = workspace::load_scripts(app)
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "SCRIPT_NOT_FOUND".to_string())?;
    if !script.enabled {
        return Err("SCRIPT_DISABLED".to_string());
    }
    crate::scripts::validate_meta(&script).map_err(|e| e.to_string())?;
    Ok(script)
}

fn get_enabled_snippet(app: &AppHandle, id: &str) -> Result<CommandSnippet, String> {
    crate::images::check_id(id, "命令片段 id")?;
    let snippet = workspace::load_snippets(app)
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "SNIPPET_NOT_FOUND".to_string())?;
    if !snippet.enabled {
        return Err("SNIPPET_DISABLED".to_string());
    }
    crate::snippets::validate_snippet(&snippet).map_err(|e| e.to_string())?;
    Ok(snippet)
}

/// 启动命令片段：命令层只传参数 map，真正 argv 整元素替换和 fail-closed 校验在
/// `script_runner::start_command`，并复用脚本运行的进程表/事件/取消/落盘链路。
#[tauri::command]
pub fn run_command(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    values: HashMap<String, String>,
) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "run_command", None, &app)?;
    crate::images::check_id(&id, "命令片段 id")?;
    let snippet = get_enabled_snippet(&app, &id)?;
    let count = snippet.params.len();
    let roots = crate::workspace::allowed_roots(&app);
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let table = Arc::clone(&app.state::<AppState>().script_runs);
    let start = crate::script_runner::start_command(
        &table,
        &snippet,
        &values,
        &roots,
        &home,
        Some(app.clone()),
        Some(workspace::script_runs_file(&app)),
    );
    let run_id = match start {
        Ok(rid) => rid,
        Err(e) => {
            if let RunError::InvalidParam(ref inner) = e {
                workspace::log_audit(
                    &app,
                    "cmd.validate.reject",
                    format!("id={} error_code={}", snippet.id, inner.code()),
                );
            }
            return Err(map_run_error(e));
        }
    };
    let snapshot = table
        .snapshot(&run_id)
        .ok_or_else(|| "UNKNOWN_RUN".to_string())?;
    workspace::log_audit(
        &app,
        "cmd.run.start",
        format!(
            "id={} run_id={} count={} dangerous={}",
            snippet.id, run_id, count, snippet.dangerous
        ),
    );
    Ok(snapshot)
}

/// 启动脚本：命令层只传输入 map，真正展开和 fail-closed 校验在 `script_runner`。
#[tauri::command]
pub fn run_script(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    values: HashMap<String, String>,
) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "run_script", None, &app)?;
    let script = get_enabled_script(&app, &id)?;
    let script_path = workspace::script_body_path(&app, &script.path)?;
    let roots = crate::workspace::allowed_roots(&app);
    let home = app.path().home_dir().map_err(|e| e.to_string())?;
    let table = Arc::clone(&app.state::<AppState>().script_runs);
    let start = crate::script_runner::start_run(
        &table,
        &script,
        &script_path,
        &values,
        &roots,
        &home,
        Some(app.clone()),
        Some(workspace::script_runs_file(&app)),
    );
    let run_id = match start {
        Ok(rid) => rid,
        Err(e) => {
            // D15：参数校验失败（fail-closed）落审计，detail 只含 script id 与稳定错误码，
            // 不含任何参数值明文（脱敏三重保险），保证攻击尝试可观测。
            if let RunError::InvalidParam(ref inner) = e {
                workspace::log_audit(
                    &app,
                    "script.validate.reject",
                    format!("id={} error_code={}", script.id, inner.code()),
                );
            }
            return Err(map_run_error(e));
        }
    };
    let snapshot = table
        .snapshot(&run_id)
        .ok_or_else(|| "UNKNOWN_RUN".to_string())?;
    workspace::log_audit(
        &app,
        "script.run.start",
        format!(
            "id={} run_id={} param_count={}",
            script.id,
            run_id,
            script.params.len()
        ),
    );
    Ok(snapshot)
}

/// 请求取消脚本：只置取消位并立即返回；进程组回收由 supervisor 完成。
#[tauri::command]
pub fn cancel_script(
    app: AppHandle,
    webview: tauri::Webview,
    run_id: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "cancel_script", None, &app)?;
    crate::images::check_id(&run_id, "运行 id")?;
    let table = Arc::clone(&app.state::<AppState>().script_runs);
    table.cancel(&run_id).map_err(map_run_error)?;
    workspace::log_audit(&app, "script.run.cancel", format!("run_id={run_id}"));
    Ok(())
}

/// 查询脚本运行状态。M2-4.c 只返回内存快照，不含输出正文。
#[tauri::command]
pub fn script_status(
    app: AppHandle,
    webview: tauri::Webview,
    run_id: String,
) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "script_status", None, &app)?;
    crate::images::check_id(&run_id, "运行 id")?;
    let snapshot = app
        .state::<AppState>()
        .script_runs
        .snapshot(&run_id)
        .ok_or_else(|| "UNKNOWN_RUN".to_string())?;
    workspace::log_audit(
        &app,
        "script.run.status",
        format!("run_id={} status={:?}", snapshot.run_id, snapshot.status),
    );
    Ok(snapshot)
}

/// 列出脚本运行历史：读 `script-runs.json`（M2-4.d 落盘），全量或按 `script_id` 过滤。
///
/// 仅终态记录（运行态查询走 `script_status` 内存快照）。记录已含 `output_tail`
/// 与 `truncated` 标志，前端可直接展示。
/// 文件不存在或为空时返回空 `Vec`，不报错（首跑无历史属正常路径）。
#[tauri::command]
pub fn script_runs_list(
    app: AppHandle,
    webview: tauri::Webview,
    script_id: Option<String>,
) -> Result<Vec<ScriptRunRecord>, String> {
    check_invocation_source(&webview, "script_runs_list", None, &app)?;
    let path = workspace::script_runs_file(&app);
    let mut records = crate::script_runner::load_run_records(&path);
    if let Some(sid) = script_id.as_deref() {
        crate::images::check_id(sid, "脚本 id")?;
        records.retain(|r| r.script_id == sid);
    }
    // 审计 detail 只含 count + 可选 script_id，不含任何参数值（沿用 M2-4.b-VERDICT
    // 脱敏三重保险）。
    workspace::log_audit(
        &app,
        "script.runs.list",
        format!(
            "count={} script_id={}",
            records.len(),
            script_id.as_deref().unwrap_or("*")
        ),
    );
    Ok(records)
}
