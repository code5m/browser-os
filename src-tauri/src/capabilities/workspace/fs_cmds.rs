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
    let roots = crate::workspace::allowed_roots(&app);
    let src_canon = sp::check_path_within_roots(&src, &roots).map_err(|e| e.to_string())?;
    let dst_canon = sp::check_path_within_roots(&dst_dir, &roots).map_err(|e| e.to_string())?;
    let dst_file = validate_move_destination(&src_canon, &dst_canon)?;
    rename_no_replace(&src_canon, &dst_file).map_err(|e| format!("移动失败: {e}"))?;
    Ok(())
}

/// Both paths must already be canonicalized and authorized by the caller.
/// Only a directory may contain its target; files may be moved into a parent.
fn validate_move_destination(
    src: &std::path::Path,
    dst_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    if src == dst_dir || (src.is_dir() && dst_dir.starts_with(src)) {
        return Err("不能移动到自身或子目录内".into());
    }
    let name = src.file_name().ok_or("无法取得文件名")?;
    let target = dst_dir.join(name);
    if std::fs::symlink_metadata(&target).is_ok() {
        return Err("目标目录已存在同名项".into());
    }
    Ok(target)
}

/// Linux atomic no-clobber rename closes the destination-creation race between
/// validation and filesystem mutation (including broken symlink destinations).
#[cfg(target_os = "linux")]
fn rename_no_replace(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let source = std::ffi::CString::new(src.as_os_str().as_bytes())?;
    let target = std::ffi::CString::new(dst.as_os_str().as_bytes())?;
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(target_os = "linux"))]
fn rename_no_replace(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    // On platforms lacking renameat2, the preflight conflict check remains best effort.
    std::fs::rename(src, dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "browseros_fs_{}_{}_{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn move_directory_containment_directions_and_peers() {
        let root = test_dir("move_containment");
        let source = root.join("source");
        let child = source.join("child");
        let peer = root.join("peer");
        let grandchild = child.join("grandchild");
        std::fs::create_dir_all(&grandchild).unwrap();
        std::fs::create_dir(&peer).unwrap();
        assert!(
            validate_move_destination(&source, &child).is_err(),
            "target inside source"
        );
        assert!(
            validate_move_destination(&source, &source).is_err(),
            "same path"
        );
        assert!(
            validate_move_destination(&grandchild, &source).is_ok(),
            "source inside target is allowed before conflict check"
        );
        assert!(
            validate_move_destination(&source, &peer).is_ok(),
            "siblings"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_conflicts_and_root_authorization() {
        let root = test_dir("move_containment");
        let outside = test_dir("move_outside");
        let source = root.join("item.txt");
        let target = root.join("target");
        std::fs::write(&source, b"source").unwrap();
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("item.txt"), b"existing").unwrap();
        let roots = [root.canonicalize().unwrap()];
        let checked_source = sp::check_path_within_roots(source.to_str().unwrap(), &roots).unwrap();
        let checked_target = sp::check_path_within_roots(target.to_str().unwrap(), &roots).unwrap();
        assert!(
            validate_move_destination(&checked_source, &checked_target)
                .unwrap_err()
                .contains("同名"),
            "same-name destination must be rejected"
        );
        assert!(
            sp::check_path_within_roots(outside.to_str().unwrap(), &roots).is_err(),
            "outside roots must be denied"
        );
        assert!(
            rename_no_replace(&checked_source, &checked_target.join("item.txt")).is_err(),
            "existing destination must not be overwritten"
        );
        assert_eq!(
            std::fs::read(checked_target.join("item.txt")).unwrap(),
            b"existing"
        );
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(outside).unwrap();
    }
}
