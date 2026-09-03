use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use tauri::{AppHandle, Manager};

use crate::domain::{Artifact, AuditEntry, Bookmark, ImageRef, RepoConfig};

fn data_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("app data dir")
        .join("mvp-browser-os")
}

fn ensure(dir: &PathBuf) {
    fs::create_dir_all(dir).ok();
}

/// 本地成果库目录
pub fn workspace_dir(app: &AppHandle) -> PathBuf {
    let d = data_dir(app).join("workspace");
    ensure(&d);
    d
}

/// 默认笔记目录（网页选区一键存 Markdown）。
/// 固定为 ~/Documents/极智笔记（用户可见的特定目录），取不到主目录时退回应用数据目录。
pub fn notes_dir(app: &AppHandle) -> PathBuf {
    let d = app
        .path()
        .home_dir()
        .map(|h| h.join("Documents").join("极智笔记"))
        .unwrap_or_else(|_| data_dir(app).join("notes"));
    ensure(&d);
    d
}

pub fn save_artifact(app: &AppHandle, art: &Artifact) -> Result<PathBuf, String> {
    let dir = workspace_dir(app);
    let file = dir.join(format!("{}.json", art.id));
    fs::write(
        &file,
        serde_json::to_string_pretty(art).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(file)
}

pub fn load_artifacts(app: &AppHandle) -> Vec<Artifact> {
    let dir = workspace_dir(app);
    let mut out = vec![];
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            // M2-1：只解析 `.json`。图片等附件一律在子目录（images/），
            // 且这里显式过滤，避免任何非成果文件被当成 JSON 试解析（R4）。
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if let Ok(c) = fs::read_to_string(path) {
                if let Ok(a) = serde_json::from_str::<Artifact>(&c) {
                    out.push(a);
                }
            }
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}

pub fn delete_artifact(app: &AppHandle, id: &str) -> Result<(), String> {
    let file = workspace_dir(app).join(format!("{}.json", id));
    if file.exists() {
        fs::remove_file(&file).map_err(|e| e.to_string())?;
    }
    // M2-1：删除联动——成果删了，其图片目录不得变成孤儿（R5）。
    // 删图失败不阻断删 JSON，但要留审计条目，便于人工兜底。
    match delete_artifact_images(app, id) {
        Ok(0) => {}
        Ok(n) => log_audit(app, "image.prune", format!("artifact_id={id} removed={n}")),
        Err(e) => log_audit(
            app,
            "image.prune_failed",
            format!("artifact_id={id} error={e}"),
        ),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// M2-1 图片存储（契约见 `logs/assist/M2-1.a-prework-20260902-1055.md` §4.5）
//
// 目录布局（图片必须放子目录，避免被 load_artifacts 误当成果解析）：
//   workspace_dir/
//   ├── <artifact_id>.json
//   └── images/<artifact_id>/<image_id>.<ext>      # ext 由 MIME 反查
// ---------------------------------------------------------------------------

/// 保存一张图片到成果（**先校验后落盘**：超限/逃逸用例不会留下任何字节）。
///
/// 顺序即契约：id 形态 → 字节数预检 → 成果存在性 → MIME/魔法字节/尺寸
/// → 去重（同 sha256 复用，不重复写盘）→ 配额 → 路径白名单 + 前缀校验
/// → 原子写（tmp + rename）→ 回写成果 JSON。
pub fn save_image(
    app: &AppHandle,
    artifact_id: &str,
    bytes: &[u8],
    mime: &str,
    source_url: Option<&str>,
    caption: Option<&str>,
) -> Result<ImageRef, String> {
    // 1) 拼路径前先卡 id 形态（防 ../ 与绝对路径）
    crate::images::validate_id(artifact_id).map_err(|e| e.to_string())?;
    // 2) 字节数预检（不分配、不写盘）
    crate::images::check_size(bytes.len()).map_err(|e| e.to_string())?;

    let mut arts = load_artifacts(app);
    let art = arts
        .iter_mut()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| "成果不存在".to_string())?;

    // 3) 内容校验（MIME 白名单 + 魔法字节 + 最长边）
    let prepared = crate::images::prepare(bytes, mime).map_err(|e| e.to_string())?;
    // 4) 去重：内容相同直接复用引用，磁盘不新增文件
    if let Some(hit) = crate::images::find_duplicate(&art.images, &prepared.sha256) {
        return Ok(hit.clone());
    }
    // 5) 配额（数量 + 总字节），在写盘前判定
    crate::images::check_quota(&art.images, prepared.bytes).map_err(|e| e.to_string())?;

    // 6) 原子落盘（纯函数：canonicalize + 前缀校验 + tmp/rename 在 images.rs 内，
    //    有真实磁盘单测覆盖）
    let image_id = uuid::Uuid::new_v4().to_string();
    let rel = crate::images::write_image_file(
        &workspace_dir(app),
        artifact_id,
        &image_id,
        prepared.ext,
        bytes,
    )
    .map_err(|e| e.to_string())?;

    // 7) 回写成果 JSON（只存引用，不存字节）
    let reference = crate::images::build_file_ref(&prepared, &rel, source_url, caption)
        .map_err(|e| e.to_string())?;
    art.images.push(reference.clone());
    save_artifact(app, art)?;
    Ok(reference)
}

/// 删除某成果的全部图片目录（幂等：不存在返回 0）。
/// 删除前同样做 canonicalize + 前缀校验，防止误删 images_root 之外的目录。
pub fn delete_artifact_images(app: &AppHandle, artifact_id: &str) -> Result<usize, String> {
    crate::images::remove_artifact_images(&workspace_dir(app), artifact_id)
        .map_err(|e| e.to_string())
}

/// 仓库配置（不含 token）持久化
pub fn repos_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("repos.json")
}
pub fn load_repos(app: &AppHandle) -> Vec<RepoConfig> {
    fs::read_to_string(repos_file(app))
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}
pub fn save_repos(app: &AppHandle, repos: &[RepoConfig]) -> Result<(), String> {
    fs::write(
        repos_file(app),
        serde_json::to_string_pretty(repos).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// 审计日志
pub fn audit_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("audit.json")
}
pub fn log_audit(app: &AppHandle, action: &str, detail: String) {
    let mut list = load_audit(app);
    list.push(AuditEntry {
        at: Utc::now(),
        action: action.into(),
        detail,
    });
    if list.len() > 1000 {
        list.drain(0..list.len() - 1000);
    }
    let _ = fs::write(
        audit_file(app),
        serde_json::to_string_pretty(&list).unwrap_or_default(),
    );
}
pub fn load_audit(app: &AppHandle) -> Vec<AuditEntry> {
    fs::read_to_string(audit_file(app))
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// 收藏持久化文件
pub fn bookmarks_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("bookmarks.json")
}

/// M1-9：浏览器会话存档目录（`data_dir/sessions/<id>.json`，原子写）。
/// 与成果库/收藏分开：会话是「浏览痕迹」，生命周期与隐私边界都不同。
pub fn sessions_dir(app: &AppHandle) -> PathBuf {
    let d = data_dir(app).join("sessions");
    ensure(&d);
    d
}
pub fn load_bookmarks(app: &AppHandle) -> Vec<Bookmark> {
    fs::read_to_string(bookmarks_file(app))
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}
pub fn save_bookmarks(app: &AppHandle, list: &[Bookmark]) -> Result<(), String> {
    fs::write(
        bookmarks_file(app),
        serde_json::to_string_pretty(list).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// 新增收藏：若 url 已存在则视为更新（保留原 id/created_at，仅覆盖 title/category）。
pub fn add_bookmark(
    app: &AppHandle,
    url: String,
    title: String,
    category: String,
) -> Result<Bookmark, String> {
    let mut list = load_bookmarks(app);
    if let Some(existing) = list.iter_mut().find(|b| b.url == url) {
        existing.title = title;
        existing.category = category;
        let bm = existing.clone();
        save_bookmarks(app, &list)?;
        Ok(bm)
    } else {
        let bm = Bookmark::new(url, title, category);
        list.push(bm.clone());
        save_bookmarks(app, &list)?;
        Ok(bm)
    }
}

/// 按 id 删除收藏；不存在不报错（幂等）。
pub fn remove_bookmark(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut list = load_bookmarks(app);
    let before = list.len();
    list.retain(|b| b.id != id);
    if list.len() != before {
        save_bookmarks(app, &list)?;
    }
    Ok(())
}
