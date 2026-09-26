// 文件系统相关命令（独立于 bridge.rs，避免触碰 IPC 命令面 danger-zone）。
// 文件树右键"资源管理器打开"：用系统文件管理器打开指定路径。
use crate::security_policy as sp;
use tauri::AppHandle;

/// 用系统文件管理器打开指定路径（文件树右键"资源管理器打开"）
#[tauri::command]
pub fn reveal_path(_app: AppHandle, path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let target = if p.is_dir() {
        p
    } else {
        p.parent().ok_or("无法取得父目录")?
    };
    open::that(target).map_err(|e| format!("无法打开目录: {e}"))
}

/// 将文件或目录移动到目标目录（文件树拖拽移动用）。
/// 源与目标都必须在允许根目录内；禁止移动到自身或自身子目录内。
#[tauri::command]
pub fn move_path(app: AppHandle, src: String, dst_dir: String) -> Result<(), String> {
    let src_p = std::path::Path::new(&src);
    if !src_p.exists() {
        return Err("源路径不存在".into());
    }
    let dst_p = std::path::Path::new(&dst_dir);
    if !dst_p.is_dir() {
        return Err("目标必须是目录".into());
    }
    let roots = crate::bridge::allowed_roots(&app);
    let src_canon = sp::check_path_within_roots(&src, &roots).map_err(|e| e.to_string())?;
    let dst_canon = sp::check_path_within_roots(&dst_dir, &roots).map_err(|e| e.to_string())?;
    // 禁止移动到自身或自身子目录内（否则源会凭空消失）
    if src_canon == dst_canon || src_canon.starts_with(&dst_canon) {
        return Err("不能移动到自身或子目录内".into());
    }
    let name = src_canon
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("无法取得文件名")?;
    let dst_file = dst_canon.join(name);
    if dst_file.exists() {
        return Err("目标目录已存在同名项".into());
    }
    std::fs::rename(&src_canon, &dst_file).map_err(|e| format!("移动失败: {e}"))?;
    Ok(())
}
