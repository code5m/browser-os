//! Framework-native service commands (FRAMEWORK_NATIVE_SERVICE).
//!
//! 与业务 capability 无关的原生服务：前端链路追踪、审计读取、应用启动等。
//! 命令体只依赖 `crate::workspace` / `crate::security_policy` 等共享基座，
//! 不反向依赖 `crate::bridge`（Native Physical Boundary 分解从 `bridge.rs` 迁出）。

use std::process::{Command, Stdio};
use tauri::AppHandle;

use crate::domain::AuditEntry;
use crate::security_policy as sp;
use crate::workspace::{load_audit, log_audit};

#[tauri::command]
pub fn debug_log(msg: String) {
    eprintln!("[FE] {msg}");
}

#[tauri::command]
pub fn audit_log(app: AppHandle) -> Vec<AuditEntry> {
    load_audit(&app)
}

#[tauri::command]
pub fn launch_app(app: AppHandle, exec: String) -> Result<(), String> {
    let cmd = exec.trim();
    if cmd.is_empty() {
        return Err("没有可执行命令".into());
    }
    // M0-3.d：不再用 `sh -c` 执行任意字符串。改为解析成 (程序, 参数) 直接 spawn：
    // 元字符一律拒绝、shell 解释器禁为启动目标、程序必须能解析到可执行文件。
    let (program, args) = sp::check_launch_target(cmd).map_err(|e| e.to_string())?;
    log_audit(
        &app,
        "launch",
        format!("{program} {}", args.join(" ")).trim().to_string(),
    );
    Command::new(&program)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("启动失败: {e}"))?;
    Ok(())
}
