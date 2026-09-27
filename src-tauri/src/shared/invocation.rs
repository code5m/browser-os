//! Native invocation-source authorization helper — `SHARED_NATIVE_INFRASTRUCTURE`.
//!
//! 来源校验：远程 webview 调用原生命令前必须过 `security_policy::IntentRegistry`
//! 的 `check_remote_invocation`。原定义位于 `bridge.rs`（被 84 处命令体调用），
//! 迁至本共享模块后，所有 capability 命令体可直接 `use` 本函数，不再反向依赖
//! `crate::bridge`（消除「move command」时的 bridge 耦合，便于逐 owner 拆分）。

use crate::security_policy as sp;
use tauri::{AppHandle, Manager, Webview};

/// 校验调用来源（webview label + scope + 可选 intent），不通过则返回错误串。
pub fn check_invocation_source(
    webview: &Webview,
    scope: &str,
    intent: Option<&str>,
    app: &AppHandle,
) -> Result<(), String> {
    let registry = app.state::<sp::IntentRegistry>();
    sp::check_remote_invocation(webview.label(), scope, intent, &registry)
        .map_err(|e| e.to_string())
}
