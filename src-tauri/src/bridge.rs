use std::collections::HashMap;
use std::sync::Mutex;

use tauri::{
    AppHandle, Emitter, Listener, Manager, PhysicalPosition, PhysicalSize,
    WebviewWindowBuilder, WebviewUrl,
};
use url::Url;

use crate::domain::*;
use crate::keyring_store::KeyringStore;
use crate::sync;
use crate::workspace;

/// 桥的进程级状态：仅保存待确认任务（凭据/审计都在磁盘，避免内存泄漏）
#[derive(Default)]
pub struct AppState {
    pub pending_jobs: Mutex<HashMap<String, SyncJob>>,
}

/// 归一化用户输入的网址：支持以下写法
/// - 空 -> 默认引导页（百度）
/// - www.baidu.com / baidu.com / example.com -> 自动补 https://
/// - http://... https://... -> 原样
/// - 带路径 baidu.com/s?wd=x -> 自动补 https://
/// - 非 URL 的单词（如 "天气"）-> 走搜索引擎
fn normalize_url(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return "https://www.baidu.com".to_string();
    }
    // 已带协议
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("file://") {
        return s.to_string();
    }
    // 含协议分隔但非 http（ftp 等）原样
    if s.contains("://") {
        return s.to_string();
    }
    // 像搜索词（含空格、中文、或不是 域名.后缀 形态）-> 搜索引擎
    let looks_like_domain =
        s.contains('.') && !s.contains(' ') && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
    if !looks_like_domain {
        // 百度搜索
        return format!("https://www.baidu.com/s?wd={}", urlencoding::encode(s));
    }
    // 裸域名：补 https://
    format!("https://{}", s)
}

/// 打开浏览器窗口（紧贴主窗口右侧）。
#[tauri::command]
pub fn open_browser(app: AppHandle, url: String) -> Result<(), String> {
    let target = normalize_url(&url);
    let parsed = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;

    // 若已存在旧浏览器窗口则先关闭
    if let Some(old) = app.get_webview_window("browser") {
        let _ = old.destroy();
    }

    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口未找到".to_string())?;

    let main_size = main.inner_size().map_err(|e| e.to_string())?;
    let pos = main.outer_position().map_err(|e| e.to_string())?;
    let bw = (main_size.width as f64 * 0.45).max(400.0) as u32;
    let bh = main_size.height;
    let bx = pos.x + main_size.width as i32;
    let by = pos.y;

    // 诊断：打印窗口参数便于排查
    eprintln!(
        "[open_browser] main outer_pos=({},{}) inner_size={}x{} browser_pos=({},{}) size={}x{} url={}",
        pos.x, pos.y, main_size.width, main_size.height, bx, by, bw, bh, url
    );

    // 初始化脚本：在页面 JS 执行前注入（比 eval 更早更可靠），禁用原生右键 + 注入采集逻辑
    let init_script = include_str!("../injected/collect.js").to_string();
    let w = WebviewWindowBuilder::new(&app, "browser", WebviewUrl::External(parsed))
        .title("浏览器")
        .position(bx as f64, by as f64)
        .inner_size(bw as f64, bh as f64)
        .decorations(true)
        .resizable(true)
        .initialization_script(&init_script)
        .build()
        .map_err(|e| format!("创建浏览器窗口失败: {e}"))?;

    eprintln!("[open_browser] 窗口创建成功 label={}", w.label());

    // 让窗口聚焦到前台
    let _ = w.set_focus();
    Ok(())
}

/// 采集：网页选区 → 本地成果（带溯源）。渲染进程零特权，只能 invoke 本命令。
#[tauri::command]
pub fn collect_selection(
    app: AppHandle,
    url: String,
    title: String,
    text: String,
    html: String,
) -> Result<Artifact, String> {
    let art = Artifact::new(title, url.clone(), text, html);
    workspace::save_artifact(&app, &art)?;
    workspace::log_audit(&app, "collect", format!("{} <- {}", art.title, url));
    Ok(art)
}

#[tauri::command]
pub fn list_artifacts(app: AppHandle) -> Vec<Artifact> {
    workspace::load_artifacts(&app)
}

/// 配置仓库：token 仅写入系统密钥库，绝不回传前端。
#[tauri::command]
pub fn configure_repo(
    app: AppHandle,
    config: RepoConfig,
    token: String,
) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("token 不能为空".into());
    }
    KeyringStore::save_token(&config.id, &token)?;
    let mut repos = workspace::load_repos(&app);
    repos.retain(|r| r.id != config.id);
    repos.push(config);
    workspace::save_repos(&app, &repos)?;
    workspace::log_audit(
        &app,
        "configure_repo",
        format!("{} ({:?})", repos.last().unwrap().name, repos.last().unwrap().provider),
    );
    Ok(())
}

#[tauri::command]
pub fn list_repos(app: AppHandle) -> Vec<RepoConfig> {
    workspace::load_repos(&app)
}

/// 第一步：生成"待确认"SyncJob（不真正推送）。校验仓库与凭据存在。
#[tauri::command]
pub fn request_sync(
    app: AppHandle,
    artifact_ids: Vec<String>,
    repo_id: String,
) -> Result<SyncPreview, String> {
    let repos = workspace::load_repos(&app);
    let repo = repos
        .iter()
        .find(|r| r.id == repo_id)
        .ok_or("仓库未配置".to_string())?;
    // 提前校验凭据存在，避免确认后才失败
    KeyringStore::get_token(&repo.id)?;

    let arts: Vec<Artifact> = workspace::load_artifacts(&app)
        .into_iter()
        .filter(|a| artifact_ids.contains(&a.id))
        .collect();
    if arts.is_empty() {
        return Err("没有可同步的成果".into());
    }

    let job = SyncJob {
        id: uuid::Uuid::new_v4().to_string(),
        repo_id: repo.id.clone(),
        status: SyncStatus::Pending,
        artifact_ids: arts.iter().map(|a| a.id.clone()).collect(),
        created_at: chrono::Utc::now(),
        finished_at: None,
        error: None,
    };
    app.state::<AppState>()
        .pending_jobs
        .lock()
        .unwrap()
        .insert(job.id.clone(), job.clone());

    workspace::log_audit(
        &app,
        "request_sync",
        format!("生成待确认任务 -> {}", repo.name),
    );

    Ok(SyncPreview {
        job_id: job.id.clone(),
        repo_id: repo.id.clone(),
        repo_name: repo.name.clone(),
        artifact_count: arts.len(),
        artifact_titles: arts.iter().map(|a| a.title.clone()).collect(),
        remote_url: repo.remote_url.clone(),
    })
}

/// 第二步（闸门）：用户确认后才真正推送。凭据仅在此从密钥库读取。
/// 改为后台线程执行，推送完成（成功/失败）通过 `sync-completed` 事件通知前端，
/// 避免阻塞 UI。
#[tauri::command]
pub fn confirm_sync(app: AppHandle, job_id: String) -> Result<SyncJob, String> {
    let state = app.state::<AppState>();
    let running = {
        let mut jobs = state.pending_jobs.lock().unwrap();
        let job = jobs.get_mut(&job_id).ok_or("未知任务".to_string())?;
        if job.status != SyncStatus::Pending {
            return Err("任务状态异常".into());
        }
        job.status = SyncStatus::Running;
        jobs[&job_id].clone()
    };

    // 后台线程：从密钥库取凭据 → push → 更新任务状态 → 发事件
    let app_thread = app.clone();
    let running_clone = running.clone();
    std::thread::spawn(move || {
        let state = app_thread.state::<AppState>();
        let result = sync::push_artifacts(&app_thread, &running_clone);
        let mut jobs = state.pending_jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(&running_clone.id) {
            match result {
                Ok(_) => {
                    job.status = SyncStatus::Success;
                    job.finished_at = Some(chrono::Utc::now());
                    workspace::log_audit(
                        &app_thread,
                        "confirm_sync",
                        format!("SUCCESS 任务 {}", running_clone.id),
                    );
                }
                Err(e) => {
                    job.status = SyncStatus::Failed;
                    job.error = Some(e.clone());
                    job.finished_at = Some(chrono::Utc::now());
                    workspace::log_audit(
                        &app_thread,
                        "confirm_sync",
                        format!("FAILED 任务 {}: {}", running_clone.id, e),
                    );
                }
            }
            let finished = job.clone();
            drop(jobs);
            let _ = app_thread.emit("sync-completed", finished);
        }
    });

    Ok(running)
}

#[tauri::command]
pub fn audit_log(app: AppHandle) -> Vec<AuditEntry> {
    workspace::load_audit(&app)
}

/// 读取单个成果完整内容（预览/编辑用）
#[tauri::command]
pub fn read_artifact(app: AppHandle, id: String) -> Result<Artifact, String> {
    workspace::load_artifacts(&app)
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| "成果不存在".to_string())
}

/// 编辑成果：仅允许改标题/标签/正文（溯源字段 source_url/hash/created_at 不可变）
#[tauri::command]
pub fn update_artifact(
    app: AppHandle,
    id: String,
    title: String,
    text: String,
    tags: Vec<String>,
) -> Result<Artifact, String> {
    let mut arts = workspace::load_artifacts(&app);
    let art = arts
        .iter_mut()
        .find(|a| a.id == id)
        .ok_or_else(|| "成果不存在".to_string())?;
    if !title.trim().is_empty() {
        art.title = title.trim().to_string();
    }
    art.text = text;
    art.tags = tags;
    let updated = art.clone();
    workspace::save_artifact(&app, art)?;
    workspace::log_audit(&app, "update", format!("编辑成果 {}", id));
    Ok(updated)
}

#[tauri::command]
pub fn delete_artifact(app: AppHandle, id: String) -> Result<(), String> {
    workspace::delete_artifact(&app, &id)?;
    workspace::log_audit(&app, "delete", format!("删除成果 {}", id));
    Ok(())
}

/// 工作区目录树：按来源域名聚合成果，便于在本地成果浏览器里浏览
#[tauri::command]
pub fn browse_workspace(app: AppHandle) -> WorkspaceTree {
    let arts = workspace::load_artifacts(&app);
    let mut domains: std::collections::BTreeMap<String, Vec<Artifact>> = std::collections::BTreeMap::new();
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
pub fn list_dir(path: String) -> Result<Vec<DirEntry>, String> {
    let dir = std::path::PathBuf::from(&path);
    if !dir.exists() { return Err("路径不存在".into()); }
    if !dir.is_dir() { return Err("不是目录".into()); }
    let mut entries = vec![];
    for e in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let meta = e.metadata().map_err(|e| e.to_string())?;
        entries.push(DirEntry {
            name: e.file_name().to_string_lossy().to_string(),
            path: e.path().to_string_lossy().to_string(),
            is_dir: meta.is_dir(),
            size: meta.len(),
        });
    }
    // 目录在前、按名字排序
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });
    Ok(entries)
}

/// 读取文本文件内容
#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// 写入文本文件（创建或覆盖）
#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}

/// 获取常用起始目录列表（Home / Desktop / Documents / Downloads / 工作区）
#[tauri::command]
pub fn get_start_dirs(app: AppHandle) -> Vec<DirEntry> {
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
    dirs
}
