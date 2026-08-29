use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use tauri::{AppHandle, Manager};

use crate::domain::{Artifact, AuditEntry, RepoConfig};

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
            if let Ok(c) = fs::read_to_string(e.path()) {
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
    Ok(())
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
