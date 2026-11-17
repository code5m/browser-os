//! Workspace tree, file operations, bookmarks and image commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

/// M1-2: 新增或更新一条收藏。同 URL 视为更新。
#[tauri::command]
pub fn add_bookmark(
    webview: tauri::Webview,
    app: AppHandle,
    url: String,
    title: String,
    category: String,
) -> Result<Bookmark, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "add_bookmark")?;
    crate::shared::native_legacy_policy::bound_text("add_bookmark", "url", &url)?;
    crate::shared::native_legacy_policy::bound_text("add_bookmark", "title", &title)?;
    crate::shared::native_legacy_policy::bound_text("add_bookmark", "category", &category)?;

    let bm = workspace::add_bookmark(&app, url.clone(), title.clone(), category.clone())?;
    workspace::log_audit(&app, "bookmark.add", format!("{} -> {}", url, bm.title));
    Ok(bm)
}

/// M1-2: 列出全部收藏，按 created_at 倒序。
#[tauri::command]
pub fn list_bookmarks(webview: tauri::Webview, app: AppHandle) -> Vec<Bookmark> {
    if crate::shared::native_legacy_policy::require_main(&webview, "list_bookmarks").is_err() {
        return Vec::new();
    }

    let mut list = workspace::load_bookmarks(&app);
    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    list
}

/// M1-2: 按 id 删除收藏（幂等）。
#[tauri::command]
pub fn remove_bookmark(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "remove_bookmark")?;
    crate::shared::native_legacy_policy::bound_text("remove_bookmark", "id", &id)?;

    workspace::remove_bookmark(&app, &id)?;
    workspace::log_audit(&app, "bookmark.remove", format!("id={}", id));
    Ok(())
}

/// 工作区目录树：按来源域名聚合成果，便于在本地成果浏览器里浏览
#[tauri::command]
pub fn browse_workspace(app: AppHandle) -> WorkspaceTree {
    let arts = workspace::load_artifacts(&app);
    let mut domains: std::collections::BTreeMap<String, Vec<Artifact>> =
        std::collections::BTreeMap::new();
    for a in arts {
        let host = url::Url::parse(&a.source_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| "未知来源".into());
        domains.entry(host).or_default().push(a);
    }
    let nodes = domains
        .into_iter()
        .map(|(host, items)| DomainNode {
            host,
            items: items
                .into_iter()
                .map(|a| DomainItem {
                    id: a.id.clone(),
                    title: a.title.clone(),
                    created_at: a.created_at,
                    tags: a.tags.clone(),
                })
                .collect(),
        })
        .collect();
    WorkspaceTree { nodes }
}

#[derive(serde::Serialize)]
pub struct WorkspaceTree {
    pub nodes: Vec<DomainNode>,
}

#[derive(serde::Serialize)]
pub struct DomainNode {
    pub host: String,
    pub items: Vec<DomainItem>,
}

#[derive(serde::Serialize)]
pub struct DomainItem {
    pub id: String,
    pub title: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

// ====== 本地文件浏览器 ======

#[derive(serde::Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64, // bytes
}

/// 列出指定路径的目录内容（不递归）
#[tauri::command]
pub async fn list_dir(
    app: AppHandle, webview: tauri::Webview, path: String,
) -> Result<Vec<DirEntry>, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "list_dir")?;
    let roots = workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move || list_dir_blocking(path, roots))
        .await
        .map_err(|_| "LIST_DIR_WORKER_FAILED".to_string())?
}

fn list_dir_blocking(path: String, roots: Vec<std::path::PathBuf>) -> Result<Vec<DirEntry>, String> {
    let dir = crate::security_policy::check_path_within_roots(&path, &roots)
        .map_err(|e| e.to_string())?;
    if !dir.exists() {
        return Err("路径不存在".into());
    }
    if !dir.is_dir() {
        return Err("不是目录".into());
    }
    let mut entries = vec![];
    for e in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        // 单个条目失败时跳过（continue），不中断整个列表 —— 修复根目录/系统目录下
        // 因符号链接失效、无权限条目导致 metadata() 失败而丢失大量条目的问题。
        let Ok(e) = e else { continue };
        // 用 file_type 判断目录（不跟随符号链接、不会因目标无权限失败）；
        // metadata 取不到则 size 记 0，仍保留条目。
        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let size = e.metadata().map(|m| m.len()).unwrap_or(0);
        entries.push(DirEntry {
            name: e.file_name().to_string_lossy().to_string(),
            path: e.path().to_string_lossy().to_string(),
            is_dir,
            size,
        });
    }
    // 目录在前、按名字排序
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok(entries)
}

/// 读取文本文件内容
#[tauri::command]
pub async fn read_file(
    app: AppHandle, webview: tauri::Webview, path: String,
) -> Result<String, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "read_file")?;
    let roots = workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move || read_file_blocking(path, roots))
        .await
        .map_err(|_| "READ_FILE_WORKER_FAILED".to_string())?
}

fn read_file_blocking(
    path: String, roots: Vec<std::path::PathBuf>,
) -> Result<String, String> {
    const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;
    let canonical = crate::security_policy::check_path_within_roots(&path, &roots)
        .map_err(|e| e.to_string())?;
    let bytes = crate::capabilities::workspace::safe_paths::read_regular(
        &canonical, &roots, MAX_TEXT_BYTES,
    ).map_err(|e| format!("FILE_READ_DENIED: {e}"))?;
    String::from_utf8(bytes).map_err(|_| "文件不是 UTF-8 文本".into())
}

fn image_mime_from_path(path: &std::path::Path) -> Option<&'static str> {
    match path
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

/// 读取本地图片为 data URL（仅主窗口文件预览使用，不扩大 asset:// 读盘范围）
#[tauri::command]
pub fn read_image_data_url(
    app: AppHandle, webview: tauri::Webview, path: String,
) -> Result<String, String> {
    use base64::Engine;
    const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
    crate::shared::native_legacy_policy::require_main(&webview, "read_image_data_url")?;
    let roots = workspace::allowed_roots(&app);
    let p = crate::security_policy::check_path_within_roots(&path, &roots)
        .map_err(|e| e.to_string())?;
    let mime = image_mime_from_path(&p).ok_or("不支持的图片格式")?;
    let bytes = crate::capabilities::workspace::safe_paths::read_regular(
        &p, &roots, MAX_IMAGE_BYTES,
    ).map_err(|_| "图片读取被拒绝或超过 8MB".to_string())?;
    Ok(format!(
        "data:{};base64,{}", mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// 写入文本文件（创建或覆盖）。M0-3.c：路径必须落在允许根目录内。
#[tauri::command]
pub fn write_file(
    app: AppHandle, webview: tauri::Webview, path: String, content: String,
) -> Result<(), String> {
    use crate::security_policy as sp;
    crate::shared::native_legacy_policy::require_main(&webview, "write_file")?;
    sp::check_text_field("content", &content, sp::MAX_HTML_BYTES)
        .map_err(|e| e.to_string())?;
    let roots = workspace::allowed_roots(&app);
    let canonical = sp::check_path_within_roots(&path, &roots)
        .map_err(|e| e.to_string())?;
    // The prior version called fs::write after a path-only check, leaving a
    // reparse/junction/symlink swap window. Both platforms now use pinned
    // directories, content-checked replacement and no unsafe fallback.
    let before = crate::capabilities::workspace::safe_paths::read_regular(
        &canonical, &roots, sp::MAX_HTML_BYTES,
    ).map_err(|_| "FILE_READ_DENIED".to_string())?;
    crate::capabilities::workspace::safe_paths::save_if_unchanged(
        &canonical, &roots, &before, content.as_bytes(), sp::MAX_HTML_BYTES,
    )
}

/// 打开成果所在的本地目录（用系统文件管理器定位到该 JSON 文件）
#[tauri::command]
pub fn reveal_artifact(
    app: AppHandle, webview: tauri::Webview, id: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "reveal_artifact")?;
    crate::security_policy::check_path_component(&id).map_err(|e| e.to_string())?;
    let file = workspace::workspace_dir(&app).join(format!("{}.json", id));
    if !file.exists() {
        return Err("成果文件不存在".into());
    }
    // 打开文件所在目录，而不是直接打开 JSON 文件（避免被浏览器等关联应用打开）
    let dir = file.parent().ok_or("无法取得目录")?;
    open::that(dir).map_err(|e| format!("无法打开目录: {e}"))
}

/// 用系统默认浏览器打开成果来源 URL
#[tauri::command]
pub fn open_source(
    _app: AppHandle, webview: tauri::Webview, url: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "open_source")?;
    if url.trim().is_empty() {
        return Err("来源 URL 为空".into());
    }
    // M0-3.c：`open::that` 会把任意 scheme 交给桌面环境执行，只允许 http/https。
    crate::security_policy::check_openable_url(&url).map_err(|e| e.to_string())?;
    open::that(&url).map_err(|e| format!("无法打开链接: {e}"))
}

/// 获取常用起始目录列表（Home / Desktop / Documents / Downloads / 工作区）
#[tauri::command]
pub fn get_start_dirs(app: AppHandle, webview: tauri::Webview) -> Vec<DirEntry> {
    if crate::shared::native_legacy_policy::require_main(&webview, "get_start_dirs").is_err() {
        return Vec::new();
    }
    let mut dirs = vec![];
    let mut add = |name: &str, p: &std::path::Path| {
        if p.exists() {
            let meta = std::fs::metadata(p);
            dirs.push(DirEntry {
                name: name.into(),
                path: p.to_string_lossy().to_string(),
                is_dir: true,
                size: meta.map(|m| m.len()).unwrap_or(0),
            });
        }
    };
    if let Ok(home) = app.path().home_dir() {
        add("🏠 主目录", &home);
        add("📁 桌面", &home.join("Desktop"));
        add("📄 文档", &home.join("Documents"));
        add("⬇ 下载", &home.join("Downloads"));
    }
    // 应用工作区
    let ws = crate::workspace::workspace_dir(&app);
    add("💾 成果工作区", &ws);
    let notes = crate::workspace::notes_dir(&app);
    add("📝 笔记目录", &notes);
    dirs
}

// ====== 文件管理：新建 / 删除 / 重命名 ======

/// 新建文件（content 为空则创建空文件）。已存在则报错。
#[tauri::command]
pub fn create_file(
    app: AppHandle, webview: tauri::Webview,
    path: String, content: Option<String>,
) -> Result<(), String> {
    use crate::security_policy as sp;
    crate::shared::native_legacy_policy::require_main(&webview, "create_file")?;
    let p = std::path::PathBuf::from(&path);
    let name = p.file_name().and_then(|n| n.to_str()).ok_or("文件名无效")?;
    sp::check_path_component(name).map_err(|e| e.to_string())?;
    let roots = workspace::allowed_roots(&app);
    let parent = p.parent().ok_or("父目录不存在")?;
    let authorized = sp::check_path_within_roots(&parent.to_string_lossy(), &roots)
        .map_err(|e| e.to_string())?;
    let target = authorized.join(name);
    let bytes = content.unwrap_or_default();
    sp::check_text_field("content", &bytes, sp::MAX_HTML_BYTES)
        .map_err(|e| e.to_string())?;
    crate::capabilities::workspace::safe_paths::create_new_file(
        &target, &roots, bytes.as_bytes()
    ).map_err(|e| format!("安全创建文件失败（父目录必须已存在）: {e}"))
}

/// 新建目录（递归创建父目录）。
#[tauri::command]
pub fn create_dir(
    app: AppHandle, webview: tauri::Webview, path: String,
) -> Result<(), String> {
    use crate::security_policy as sp;
    crate::shared::native_legacy_policy::require_main(&webview, "create_dir")?;
    let p = std::path::PathBuf::from(&path);
    let name = p.file_name().and_then(|n| n.to_str()).ok_or("目录名无效")?;
    sp::check_path_component(name).map_err(|e| e.to_string())?;
    let roots = workspace::allowed_roots(&app);
    if !p.is_absolute() || p.components().any(|part|
        matches!(part, std::path::Component::ParentDir)
    ) { return Err("目录路径必须是绝对路径且不可包含 ..".into()); }
    let mut ancestor = p.parent().ok_or("父目录不存在")?;
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("目录路径无效")?;
    }
    let authorized = sp::check_path_within_roots(&ancestor.to_string_lossy(), &roots)
        .map_err(|e| e.to_string())?;
    let relative = p.strip_prefix(ancestor).map_err(|_| "目录路径无效")?;
    crate::capabilities::workspace::safe_paths::create_directory_all(
        &authorized.join(relative), &roots
    ).map_err(|e| format!("安全创建目录失败: {e}"))
}

/// 删除文件或目录（递归删除）。
#[tauri::command]
pub fn delete_path(
    app: AppHandle, webview: tauri::Webview, path: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "delete_path")?;
    use crate::security_policy as sp;
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err("路径不存在".into());
    }
    // 递归删除最危险：必须在允许根目录内，且不允许删根目录本身。
    let canonical = sp::check_delete_target(&path, &crate::workspace::allowed_roots(&app))
        .map_err(|e| e.to_string())?;
    if canonical.is_dir() {
        // Rust's recursive remover does not follow child symlinks/junctions.
        // Root authorization still needs the original directory to remain
        // at the path selected by the user during this operation.
        std::fs::remove_dir_all(&canonical).map_err(|e| e.to_string())?;
    } else {
        // The previous std::fs::remove_file could race a symlink/junction
        // swap. Only remove a regular file relative to an authorized handle.
        crate::capabilities::workspace::safe_paths::remove_regular(&canonical, &crate::workspace::allowed_roots(&app))
            .map_err(|e| format!("安全删除失败: {e}"))?;
    }
    Ok(())
}

/// 重命名 / 移动文件或目录。
#[tauri::command]
pub fn rename_path(
    app: AppHandle, webview: tauri::Webview,
    path: String, new_name: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "rename_path")?;
    use crate::security_policy as sp;
    let p = std::path::PathBuf::from(&path);
    if !p.exists() {
        return Err("路径不存在".into());
    }
    // 新名称必须是不含分隔符的单一分量，否则 `parent.join(new_name)` 会变成跨目录移动。
    sp::check_path_component(&new_name).map_err(|e| e.to_string())?;
    let roots = crate::workspace::allowed_roots(&app);
    let canonical = sp::check_path_within_roots(&path, &roots).map_err(|e| e.to_string())?;
    let parent = canonical.parent().ok_or("无法取得父目录")?;
    let dst = parent.join(new_name.trim());
    if dst.exists() {
        return Err("目标名称已存在".into());
    }
    crate::capabilities::workspace::safe_paths::rename_no_replace(
        &canonical, parent, std::ffi::OsStr::new(new_name.trim()), &roots,
    ).map_err(|e| format!("安全重命名失败: {e}"))
}

// ====== 剪贴板 ======

// ====== 宫格浏览器：多网页并排对比 =====
//
// 多进程架构（Phase 2 起）：每个宫格 grid-N 由独立子进程承载（崩溃隔离），
// 本组命令不再走本地 TabManagerState（add_child），而是经 GridProcessManager
// 转发 UDS 命令到对应子进程。页签 tab-N 不受影响（仍主进程本地）。
// 前端契约不变：create_grid/grid_open/grid_position/grid_set_zoom/close_grid/grid_close_one。
