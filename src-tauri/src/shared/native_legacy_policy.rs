//! Runtime caller and argument boundary for legacy main-window native commands.
//! An IPC allowlist is necessary but not sufficient: the command also checks
//! the actual Webview source and bounded parameters before performing effects.
use tauri::Webview;

pub fn require_main(webview: &Webview, command: &str) -> Result<(), String> {
    authorize_label(webview.label(), command)
}

pub fn authorize_label(label: &str, command: &str) -> Result<(), String> {
    if label != "main" {
        return Err("NATIVE_LEGACY_SOURCE_DENIED".into());
    }
    // Only the frozen, reviewed native command set can reuse this guard.
    if !matches!(
        command,
        "add_bookmark"
            | "clipboard_read"
            | "clipboard_write"
            | "close_browser"
            | "close_tools"
            | "debug_log"
            | "eval_in_tab"
            | "get_default_browser"
            | "hide_all_webviews"
            | "hide_webview"
            | "issue_intent"
            | "list_bookmarks"
            | "list_tools"
            | "m0_ready"
            | "open_browser"
            | "open_tool"
            | "position_browser"
            | "remove_bookmark"
            | "resource_stats"
            | "set_default_browser"
            | "set_tab_hibernation"
            | "tab_activate"
            | "tab_close"
            | "tab_go_back"
            | "tab_go_forward"
            | "tab_list"
            | "tab_new"
            | "tab_open"
            | "tab_position"
            | "tab_reload"
            | "tab_set_title"
            | "take_pending_open_urls"
            | "clear_tab_resources"
            | "flush_sessions"
            | "get_resource_capture_settings"
            | "get_session_policy"
            | "graph_node_get"
            | "graph_query"
            | "graph_stats"
            | "list_tab_resources"
            | "session_delete"
            | "session_discard"
            | "session_export"
            | "session_get"
            | "session_list"
            | "session_restore"
            | "session_save"
            | "set_resource_capture_settings"
            | "set_session_policy"
            | "skill_parse"
            | "skill_permission_preview"
            | "skill_validate"
            | "snippet_add"
            | "snippet_list"
            | "snippet_remove"
            | "snippet_update"
    ) {
        return Err("NATIVE_LEGACY_UNREVIEWED_COMMAND".into());
    }
    Ok(())
}

pub fn bound_text(command: &str, name: &str, text: &str) -> Result<(), String> {
    let max = match name {
        "js" => 64 * 1024,
        "text" | "msg" => 256 * 1024,
        "url" | "page_url" => 8 * 1024,
        "description" | "preview" => 16 * 1024,
        "title" | "name" | "category" => 4 * 1024,
        "id" | "tab_id" | "scope" => 128,
        _ => 16 * 1024,
    };
    if text.len() > max || text.contains('\0') {
        return Err(format!("NATIVE_LEGACY_ARGUMENT_LIMIT:{command}:{name}"));
    }
    if matches!(name, "id" | "tab_id" | "scope")
        && (text.is_empty() || text.chars().any(char::is_control))
    {
        return Err(format!("NATIVE_LEGACY_ARGUMENT_INVALID:{command}:{name}"));
    }
    Ok(())
}

pub fn bound_dimension(command: &str, name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value.abs() > 1_000_000.0 {
        return Err(format!("NATIVE_LEGACY_COORDINATE_INVALID:{command}:{name}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deny_untrusted_tab_grid_and_unknown_commands() {
        assert!(authorize_label("tab-1", "clipboard_write").is_err());
        assert!(authorize_label("grid-2", "eval_in_tab").is_err());
        assert!(authorize_label("main", "nonexistent_native_command").is_err());
        assert!(authorize_label("main", "session_delete").is_ok());
    }
    #[test]
    fn reject_invalid_legacy_parameter_shapes() {
        assert!(bound_text("eval_in_tab", "js", &"x".repeat(65537)).is_err());
        assert!(bound_text("session_delete", "id", "../\n").is_err());
        assert!(bound_text("tab_open", "url", &"z".repeat(8193)).is_err());
        assert!(bound_dimension("tab_position", "x", f64::NAN).is_err());
        assert!(bound_dimension("tab_position", "width", f64::INFINITY).is_err());
        assert!(bound_text("clipboard_write", "text", "hello").is_ok());
    }
}
