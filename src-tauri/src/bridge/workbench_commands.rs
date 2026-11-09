//! Vault, archive, grid reply and Agent definition commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

#[tauri::command]
pub async fn vault_open(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
) -> Result<crate::workbench::VaultSnapshot, String> {
    check_invocation_source(&webview, "vault_open", None, &app)?;
    let root = crate::security_policy::check_path_within_roots(
        &path,
        &crate::workspace::allowed_roots(&app),
    )
    .map_err(|_| "VAULT_PATH_DENIED")?;
    tauri::async_runtime::spawn_blocking(move || crate::workbench::read_vault(&root))
        .await
        .map_err(|_| "VAULT_WORKER_FAILED")?
}

/// Save an existing Markdown note with conflict detection. Never write outside an
/// explicitly allowed Vault root; never accept an unchecked absolute note path.
#[tauri::command]
pub async fn vault_save_note(
    app: AppHandle,
    webview: tauri::Webview,
    root: String,
    relative_path: String,
    expected_text: String,
    content: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "vault_save_note", None, &app)?;
    let root = crate::security_policy::check_path_within_roots(
        &root,
        &crate::workspace::allowed_roots(&app),
    )
    .map_err(|_| "VAULT_EDIT_PATH_DENIED")?;
    let root_copy = root.clone();
    let path_copy = relative_path.clone();
    let new_text = content.clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::workbench::save_vault_note(&root_copy, &path_copy, &expected_text, &new_text)
    })
    .await
    .map_err(|_| "VAULT_EDIT_WORKER_FAILED".to_string())??;
    workspace::log_audit(&app, "vault.note.update", "markdown note updated".into());
    // The file has already been committed: report sync failure so the UI can
    // retain a dirty-index warning and allow explicit rebuild; never claim both succeeded.
    crate::capabilities::graph::commands::refresh_saved_note(&app, &root, &relative_path, &content)
        .map_err(|_| "VAULT_NOTE_SAVED_GRAPH_SYNC_PENDING".to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn grid_read_replies(
    app: AppHandle,
    webview: tauri::Webview,
    index: u32,
) -> Result<serde_json::Value, String> {
    check_invocation_source(&webview, "grid_read_replies", None, &app)?;
    if index >= 12 {
        return Err("GRID_INDEX_INVALID".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let text = app.state::<AppState>().grid_manager.read_replies(index)?;
        if text.len() > crate::workbench::MAX_NOTE_BYTES as usize {
            return Err("GRID_RESULT_LIMIT".into());
        }
        serde_json::from_str(&text).map_err(|_| "GRID_RESULT_INVALID".into())
    })
    .await
    .map_err(|_| "GRID_WORKER_FAILED")?
}

#[tauri::command]
pub async fn archive_replies(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
    items: Vec<crate::workbench::ArchiveItem>,
    tags: Vec<String>,
) -> Result<Vec<crate::workbench::ArchiveResult>, String> {
    check_invocation_source(&webview, "archive_replies", None, &app)?;
    let root = crate::security_policy::check_path_within_roots(
        &path,
        &crate::workspace::allowed_roots(&app),
    )
    .map_err(|_| "ARCHIVE_PATH_DENIED")?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        crate::workbench::save_archives(&root, items, &tags)
    })
    .await
    .map_err(|_| "ARCHIVE_WORKER_FAILED")??;
    workspace::log_audit(
        &app,
        "archive.save",
        format!(
            "count={} saved={}",
            result.len(),
            result.iter().filter(|r| r.path.is_some()).count()
        ),
    );
    Ok(result)
}

// ===== M5-W7（Lane A5）：Agent 只读命令桥 =====
// 仅 parse / validate / permission_preview，绝不执行/安装/联网/持久化写。
// 每个命令首行做 source check（防前端裸 invoke）；返回类型可序列化供前端消费。
// 命名与 A6 M5-6 UI 冻结的 agent_* 前缀一致，但本波仅「只读」子集，不含 M5-5 运行时命令。
// Skill 只读命令已迁 `capabilities/skill/commands.rs`（native-physical-batch-skill）。
// 通用体积守卫 `reject_oversized_def` 与校验报告 `ValidationReport` 已迁
// `shared/validation.rs`（agent/skill 共用，属 SHARED_NATIVE_INFRASTRUCTURE）。

pub(super) fn agent_validate_inner(text: &str) -> crate::shared::validation::ValidationReport {
    if text.len() > crate::shared::validation::AGENT_DEF_MAX_BYTES {
        return crate::shared::validation::ValidationReport::with_errors(vec![format!(
            "Agent 定义超出大小上限（{} 字节），已拒绝解析",
            crate::shared::validation::AGENT_DEF_MAX_BYTES
        )]);
    }
    match crate::domain::AgentDef::parse(text) {
        Ok(def) => match def.validate() {
            Ok(()) => crate::shared::validation::ValidationReport::ok(),
            Err(e) => {
                crate::shared::validation::ValidationReport::with_errors(vec![format!("{e}")])
            }
        },
        Err(e) => crate::shared::validation::ValidationReport::with_errors(vec![e]),
    }
}

#[tauri::command]
pub fn agent_parse(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<crate::domain::AgentDef, String> {
    check_invocation_source(&webview, "agent_parse", None, &app)?;
    agent_parse_inner(&text)
}

#[tauri::command]
pub fn agent_validate(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<crate::shared::validation::ValidationReport, String> {
    check_invocation_source(&webview, "agent_validate", None, &app)?;
    Ok(agent_validate_inner(&text))
}

#[tauri::command]
pub fn agent_permission_preview(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<crate::domain::PermissionPreview, String> {
    check_invocation_source(&webview, "agent_permission_preview", None, &app)?;
    agent_permission_preview_inner(&text)
}

pub(super) fn agent_parse_inner(text: &str) -> Result<crate::domain::AgentDef, String> {
    crate::shared::validation::reject_oversized_def(
        text,
        crate::shared::validation::AGENT_DEF_MAX_BYTES,
        "Agent",
    )?;
    crate::domain::AgentDef::parse(text)
}

pub(super) fn agent_permission_preview_inner(
    text: &str,
) -> Result<crate::domain::PermissionPreview, String> {
    crate::shared::validation::reject_oversized_def(
        text,
        crate::shared::validation::AGENT_DEF_MAX_BYTES,
        "Agent",
    )?;
    let def = crate::domain::AgentDef::parse(text)?;
    Ok(def.permission_preview())
}

// ====== 浏览器凭据导入（CSV → 内存 → 系统密钥库）======
//
// 安全约束（不可放宽）：
//   1) password 只经内存传入并立即写入系统 keyring；绝不写入日志 / JSON / SQLite /
//      临时文件 / localStorage，也不进入任何错误信息。
//   2) 不做明文降级：keyring 不可用时该条记为失败并如实报错。
// 注意：本结构含 password，**故意不派生 Debug**，避免未来 `{:?}` / `dbg!` 把密码打进日志。
