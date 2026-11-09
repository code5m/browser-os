//! MCP policy and plugin lifecycle commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

#[tauri::command]
pub fn mcp_policy_get(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<McpPolicySnapshot, String> {
    check_invocation_source(&webview, "mcp_policy_get", None, &app)?;
    Ok(crate::mcp::current_policy_snapshot())
}

#[tauri::command]
pub fn mcp_registry_list(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<McpRegistryEntryView>, String> {
    check_invocation_source(&webview, "mcp_registry_list", None, &app)?;
    Ok(crate::mcp::list_registry_entries())
}

#[tauri::command]
pub fn mcp_capability_preview(
    app: AppHandle,
    webview: tauri::Webview,
    capability: String,
    raw_path: Option<String>,
) -> Result<McpDecisionView, String> {
    check_invocation_source(&webview, "mcp_capability_preview", None, &app)?;
    let roots = crate::workspace::allowed_roots(&app);
    Ok(McpDecisionView::from_decision(
        crate::mcp::evaluate_mcp_command(&capability, raw_path.as_deref(), &roots),
    ))
}

// ===========================================================================
// M5-W12 图谱只读 live-query 命令（graph_query / graph_node_get / graph_stats）
// 已迁至 `capabilities/graph/commands.rs`（capability-owned，经 `generate_handler!`
// 以 `crate::capabilities::graph::commands::*` 直接注册），不再经 `bridge::` facade。

// ===========================================================================
// M5-W13 插件 manifest 生命周期 Stage-I 命令（Lane A9）
//
// 只做：来源校验 + 纯登记簿读写 + 原子落盘 + 脱敏审计。
// 不做：invoke / 命令执行 / 动态加载 / 网络下载 / 资源解包 / 真验签——均属后续
// runtime wave，本 wave 硬停锁死。
// 审计只记 id / 版本 / 能力计数 / 稳定错误码，**不**记资源路径、签名原文、公钥原文、
// 请求或响应正文（读命令 `plugin_list` / `plugin_get` / `plugin_keys_list` 不写审计）。
// ===========================================================================

fn plugin_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 审计用 id：截断后再落审计，避免未校验输入撑爆审计条目。
fn plugin_id_for_audit(id: &str) -> String {
    id.chars().take(64).collect()
}

/// 状态过滤参数解析（`snake_case`，与 `PluginState` serde 口径一致）。
fn parse_plugin_state(raw: Option<String>) -> Result<Option<crate::domain::PluginState>, String> {
    use crate::domain::PluginState as S;
    match raw.as_deref().map(str::trim) {
        Some(s) if !s.is_empty() => match s {
            "discovered" => Ok(Some(S::Discovered)),
            "validating" => Ok(Some(S::Validating)),
            "signed_ok" => Ok(Some(S::SignedOk)),
            "signed_failed" => Ok(Some(S::SignedFailed)),
            "loaded" => Ok(Some(S::Loaded)),
            "enabled" => Ok(Some(S::Enabled)),
            "disabled" => Ok(Some(S::Disabled)),
            "uninstalled" => Ok(Some(S::Uninstalled)),
            _ => Err("PLUGIN_STATE_INVALID".to_string()),
        },
        _ => Ok(None),
    }
}

/// 状态迁移公共体（enable / disable 共用），审计脱敏由本函数统一收口。
fn plugin_set_state(
    app: &AppHandle,
    id: &str,
    next: crate::domain::PluginState,
    action: &str,
) -> Result<crate::domain::PluginSummary, String> {
    let file = crate::plugin::plugins_file(app);
    let mut list = crate::plugin::load_plugins_at(&file);
    let id_safe = plugin_id_for_audit(id);
    match crate::plugin::set_plugin_state(&mut list, id, next, &plugin_now()) {
        Ok(rec) => {
            crate::plugin::save_plugins_at(&file, &list)?;
            let summary = crate::plugin::summary_of(&rec);
            let dangerous = rec
                .capabilities
                .iter()
                .filter(|c| {
                    crate::plugin::capability_acl_level(&c.capability)
                        == crate::domain::AclLevel::Dangerous
                })
                .count();
            workspace::log_audit(
                app,
                action,
                format!(
                    "id={id_safe} capability_count={} dangerous_count={dangerous} result=ok",
                    summary.capability_count
                ),
            );
            Ok(summary)
        }
        Err(e) => {
            let code = crate::plugin::error_code(&e);
            workspace::log_audit(app, action, format!("id={id_safe} result=err code={code}"));
            Err(code.to_string())
        }
    }
}

#[tauri::command]
pub fn plugin_install(
    app: AppHandle,
    webview: tauri::Webview,
    manifest: crate::domain::PluginManifest,
    resource_path: Option<String>,
) -> Result<crate::domain::PluginSummary, String> {
    check_invocation_source(&webview, "plugin_install", None, &app)?;
    // 资源路径（可选）：必须在允许根目录内；**只取「是否通过」布尔，绝不落盘路径**。
    let resource_ok = match resource_path.as_deref().map(str::trim) {
        Some(p) if !p.is_empty() => {
            let roots = crate::workspace::allowed_roots(&app);
            if let Err(e) = crate::security_policy::check_path_within_roots(p, &roots) {
                let code = crate::plugin::error_code(&e);
                workspace::log_audit(&app, "plugin.install", format!("result=err code={code}"));
                return Err(code.to_string());
            }
            true
        }
        _ => false,
    };
    let id_safe = plugin_id_for_audit(&manifest.id);
    let file = crate::plugin::plugins_file(&app);
    let mut list = crate::plugin::load_plugins_at(&file);
    match crate::plugin::install_record(&mut list, manifest, resource_ok, &plugin_now()) {
        Ok(rec) => {
            crate::plugin::save_plugins_at(&file, &list)?;
            let summary = crate::plugin::summary_of(&rec);
            workspace::log_audit(
                &app,
                "plugin.install",
                format!(
                    "id={} version={} capability_count={} result=ok",
                    summary.id, summary.version, summary.capability_count
                ),
            );
            Ok(summary)
        }
        Err(e) => {
            let code = crate::plugin::error_code(&e);
            workspace::log_audit(
                &app,
                "plugin.install",
                format!("id={id_safe} result=err code={code}"),
            );
            Err(code.to_string())
        }
    }
}

#[tauri::command]
pub fn plugin_enable(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<crate::domain::PluginSummary, String> {
    check_invocation_source(&webview, "plugin_enable", None, &app)?;
    plugin_set_state(
        &app,
        &id,
        crate::domain::PluginState::Enabled,
        "plugin.enable",
    )
}

#[tauri::command]
pub fn plugin_disable(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<crate::domain::PluginSummary, String> {
    check_invocation_source(&webview, "plugin_disable", None, &app)?;
    plugin_set_state(
        &app,
        &id,
        crate::domain::PluginState::Disabled,
        "plugin.disable",
    )
}

#[tauri::command]
pub fn plugin_list(
    app: AppHandle,
    webview: tauri::Webview,
    state: Option<String>,
) -> Result<Vec<crate::domain::PluginSummary>, String> {
    check_invocation_source(&webview, "plugin_list", None, &app)?;
    let filter = parse_plugin_state(state)?;
    let list = crate::plugin::load_plugins_at(&crate::plugin::plugins_file(&app));
    Ok(crate::plugin::filter_by_state(&list, filter))
}

#[tauri::command]
pub fn plugin_get(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<crate::domain::PluginDetail, String> {
    check_invocation_source(&webview, "plugin_get", None, &app)?;
    let list = crate::plugin::load_plugins_at(&crate::plugin::plugins_file(&app));
    match crate::plugin::find_record(&list, &id) {
        Some(rec) => Ok(crate::plugin::detail_of(rec)),
        None => Err("PLUGIN_NOT_FOUND".to_string()),
    }
}

#[tauri::command]
pub fn plugin_keys_add(
    app: AppHandle,
    webview: tauri::Webview,
    key_id: String,
    pubkey: String,
    note: Option<String>,
) -> Result<Vec<crate::domain::TrustedKeyRecord>, String> {
    check_invocation_source(&webview, "plugin_keys_add", None, &app)?;
    let file = crate::plugin::trusted_keys_file(&app);
    let mut keys = crate::plugin::load_trusted_keys_at(&file);
    let key_safe: String = key_id.chars().take(64).collect();
    let result = crate::plugin::add_trusted_key(
        &mut keys,
        &key_id,
        &pubkey,
        note.unwrap_or_default().as_str(),
        &plugin_now(),
    );
    match result {
        Ok(_) => {
            crate::plugin::save_trusted_keys_at(&file, &keys)?;
            workspace::log_audit(
                &app,
                "plugin.keys.add",
                format!("key_id={key_safe} count={} result=ok", keys.len()),
            );
            Ok(keys)
        }
        Err(e) => {
            let code = crate::plugin::error_code(&e);
            workspace::log_audit(
                &app,
                "plugin.keys.add",
                format!("key_id={key_safe} result=err code={code}"),
            );
            Err(code.to_string())
        }
    }
}

#[tauri::command]
pub fn plugin_keys_list(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Vec<crate::domain::TrustedKeyRecord>, String> {
    check_invocation_source(&webview, "plugin_keys_list", None, &app)?;
    Ok(crate::plugin::load_trusted_keys_at(
        &crate::plugin::trusted_keys_file(&app),
    ))
}

#[tauri::command]
pub fn plugin_keys_remove(
    app: AppHandle,
    webview: tauri::Webview,
    key_id: String,
) -> Result<Vec<crate::domain::TrustedKeyRecord>, String> {
    check_invocation_source(&webview, "plugin_keys_remove", None, &app)?;
    let file = crate::plugin::trusted_keys_file(&app);
    let mut keys = crate::plugin::load_trusted_keys_at(&file);
    let key_safe: String = key_id.chars().take(64).collect();
    match crate::plugin::remove_trusted_key(&mut keys, &key_id) {
        Ok(_) => {
            crate::plugin::save_trusted_keys_at(&file, &keys)?;
            workspace::log_audit(
                &app,
                "plugin.keys.remove",
                format!("key_id={key_safe} count={} result=ok", keys.len()),
            );
            Ok(keys)
        }
        Err(e) => {
            let code = crate::plugin::error_code(&e);
            workspace::log_audit(
                &app,
                "plugin.keys.remove",
                format!("key_id={key_safe} result=err code={code}"),
            );
            Err(code.to_string())
        }
    }
}
