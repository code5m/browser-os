//! Git 能力原生命令层（CAPABILITY_NATIVE(git)）。
//!
//! 承载 git 只读命令（status / diff / branch_list / log / commit_diff）、
//! Git 写双阶段确认闸门（request_git_write / confirm_git_write）与成果推送
//! 同步（request_sync / confirm_sync）。命令体委托 `crate::sync`（Git 同步内核）
//! 与 `crate::keyring_store`（凭据）/ `crate::workspace`（repo dir）。
//!
//! 迁移自 `src-tauri/src/bridge.rs`（Native Physical Boundary Matrix /
//! native-physical-batch-git）。`pending_git_jobs` / `pending_jobs` 仍定义在
//! `AppState`，本层经 `app.state::<crate::AppState>()` 访问（矩阵 §2.5 归属 git）。

use std::collections::HashMap;

use tauri::{AppHandle, Emitter, Manager};

use crate::domain::*;
use crate::keyring_store::KeyringStore;
use crate::shared::invocation::check_invocation_source;
use crate::sync;
use crate::workspace;

/// Git 提交日志条目（git log 等价），仅含可序列化元数据，绝不含凭据/diff。
/// 迁移自 `bridge.rs`（native-physical-batch-git），归 git 能力所有。
#[derive(serde::Serialize)]
pub struct GitLogEntry {
    pub oid: String,
    pub parents: Vec<String>,
    pub summary: String,
    pub author: String,
    pub time: i64,
}

// ---------------------------------------------------------------------------
// M1-5 Git 只读命令（status / diff / branch_list）
//
// 硬约束：只允许读仓库状态，不得实现 commit / checkout / push / pull /
// merge / rebase / 分支删除等任何写操作或联网调用；不读 Keyring、不回传凭据。
// 三个命令均做调用来源校验（子 webview / 远程页面不得调用），仓库路径锁定在
// app_data_dir/mvp-browser-os/repos/<id>（见 sync::repo_dir）。
// ---------------------------------------------------------------------------

/// 只读：工作区/索引脏文件清单（git status 等价）。
#[tauri::command]
pub fn git_status(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
) -> Result<Vec<GitFileStatus>, String> {
    check_invocation_source(&webview, "git_status", None, &app)?;
    let repo = sync::open_readonly(&app, &repo_id)?;
    sync::read_status(&repo)
}

/// 只读：HEAD → 工作区的改动（git diff 等价）。
/// `path` 为可选路径过滤；`max_bytes` 为单文件补丁上限（缺省 64KB，
/// 服务端硬上限 256KB），超限截断并标 `truncated`，总量触顶标 `more`。
#[tauri::command]
pub fn git_diff(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
    path: Option<String>,
    max_bytes: Option<u32>,
) -> Result<GitDiffResult, String> {
    check_invocation_source(&webview, "git_diff", None, &app)?;
    let repo = sync::open_readonly(&app, &repo_id)?;
    let cap = max_bytes
        .map(|v| v as usize)
        .unwrap_or(sync::GIT_DIFF_DEFAULT_MAX_BYTES);
    sync::read_diff(&repo, path.as_deref(), cap)
}

/// 只读：本地 + 远程跟踪分支清单（git branch -a 等价）。
#[tauri::command]
pub fn git_branch_list(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
) -> Result<Vec<GitBranch>, String> {
    check_invocation_source(&webview, "git_branch_list", None, &app)?;
    let repo = sync::open_readonly(&app, &repo_id)?;
    sync::read_branches(&repo)
}

// ---------------------------------------------------------------------------
// M1-6.b Git 写能力：双阶段确认闸门（request_git_write / confirm_git_write）
//
// 对齐 request_sync/confirm_sync 的双阶段范式，但使用独立的
// GitWriteJob/pending_git_jobs（SyncJob 语义属成果推送，混用会污染既有流程）。
//
// 安全边界（fail-closed，任一环节失败零写入）：
// - 操作白名单仅六项（GitWriteOp），其余一律 Err("操作禁止")；
// - repo_id 必须命中 repos.json 且锁定在 app_data_dir/mvp-browser-os/repos/<id>
//   （复用 M1-5 sync::repo_dir/open_readonly）；
// - paths 仅允许仓库内显式相对路径，禁止 .git 内部/绝对路径/空路径；
// - commit message 非空限长禁控制字符；branch name 禁 HEAD/空名/逃逸/控制字符；
// - discard 为 dangerous：confirm 必须带 confirmed_dangerous=true 二次确认；
// - 任务一次性、5 分钟过期；确认后后台线程执行，完成发 git-write-completed；
// - 审计 git_write_request / git_write / git_write_failed / git_write_rejected，
//   detail 只含操作语义与计数，绝无 token/凭据/完整路径清单/diff。
// ---------------------------------------------------------------------------

/// Git 写任务确认有效期（冻结：5 分钟）。
pub const GIT_WRITE_JOB_TTL_SECS: i64 = 300;
/// 预览返回给前端的路径列表截断条数（完整列表不落前端，防大仓刷屏）。
pub const GIT_WRITE_PREVIEW_MAX_PATHS: usize = 20;

/// 闸门纯函数：校验并取出一个待确认任务（一次性；任何失败都不产生写）。
pub(crate) fn take_confirmable_git_job(
    jobs: &mut HashMap<String, GitWriteJob>,
    job_id: &str,
    now: chrono::DateTime<chrono::Utc>,
    confirmed_dangerous: bool,
) -> Result<GitWriteJob, String> {
    let job = jobs.get(job_id).cloned().ok_or("未知任务")?;
    if job.status != GitWriteStatus::Pending {
        jobs.remove(job_id);
        return Err("任务状态异常".to_string());
    }
    if now >= job.expires_at {
        jobs.remove(job_id);
        return Err("任务已过期".to_string());
    }
    if job.dangerous && !confirmed_dangerous {
        // 保留任务：用户可在前端补二次确认后在有效期内重试；绝不执行
        return Err("需二次确认".to_string());
    }
    jobs.remove(job_id);
    let mut job = job;
    job.status = GitWriteStatus::Running;
    Ok(job)
}

/// 清理已过期的待确认任务（纯函数）：request 阶段插入新任务前调用，
/// 防止未被 confirm 的任务在表里无限累积。只清理「即使 confirm 也会被拒」的
/// 过期任务，不影响任何有效期内任务。
pub(crate) fn purge_expired_git_jobs(
    jobs: &mut HashMap<String, GitWriteJob>,
    now: chrono::DateTime<chrono::Utc>,
) -> usize {
    let before = jobs.len();
    jobs.retain(|_, job| now < job.expires_at);
    before - jobs.len()
}

/// 审计 detail 构造（纯函数）：只含操作语义与计数。
/// 刻意不接收 paths/diff/凭据/远端 URL，从签名上杜绝敏感内容落审计。
/// `branch`/`remote_name` 仅 push/分支类操作使用（分支名与远端名非凭据）。
pub(crate) fn git_write_audit_detail(
    op: GitWriteOp,
    repo_id: &str,
    job_id: &str,
    path_count: usize,
    confirmed: bool,
    branch: Option<&str>,
    remote_name: Option<&str>,
    extra: &str,
) -> String {
    serde_json::json!({
        "op": op.as_str(),
        "repo_id": repo_id,
        "job_id": job_id,
        "path_count": path_count,
        "confirmed": confirmed,
        "branch": branch,
        "remote_name": remote_name,
        "extra": extra,
    })
    .to_string()
}

/// 审计/错误串兜底截断（300 字节，回退 UTF-8 边界）：
/// 写链不接触 token，这里防止冗长错误（如整段 diff 文本）落审计。
pub(crate) fn sanitize_audit_text(s: &str) -> String {
    const MAX: usize = 300;
    if s.len() <= MAX {
        return s.to_string();
    }
    let mut end = MAX;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

/// 阶段一预检（只读）：参数校验 + 必然失败情形提前暴露，
/// 返回 (摘要, 影响路径, 推导出的分支名)。任何一步失败都直接 Err——
/// 此函数不做任何写，push 分支也绝不接触远端。
fn precheck_git_write(
    repo: &git2::Repository,
    op: GitWriteOp,
    paths: &[String],
    message: Option<&str>,
    branch: Option<&str>,
    checkout: bool,
) -> Result<(String, Vec<String>, Option<String>), String> {
    match op {
        GitWriteOp::Stage => {
            sync::validate_repo_paths(paths)?;
            sync::precheck_stageable(repo, paths)?;
            Ok((format!("暂存 {} 个路径", paths.len()), paths.to_vec(), None))
        }
        GitWriteOp::Unstage => {
            sync::validate_repo_paths(paths)?;
            sync::precheck_tracked(repo, paths)?;
            Ok((
                format!("取消暂存 {} 个路径", paths.len()),
                paths.to_vec(),
                None,
            ))
        }
        GitWriteOp::Discard => {
            sync::validate_repo_paths(paths)?;
            sync::precheck_tracked(repo, paths)?;
            Ok((
                format!("丢弃 {} 个路径的工作区改动（不可恢复）", paths.len()),
                paths.to_vec(),
                None,
            ))
        }
        GitWriteOp::Commit => {
            sync::validate_commit_message(message.unwrap_or_default())?;
            if paths.is_empty() {
                let dirty = sync::read_status(repo)?;
                if dirty.is_empty() {
                    return Err("没有可提交的变更".to_string());
                }
                let affected: Vec<String> = dirty.into_iter().map(|s| s.path).collect();
                Ok((
                    format!("提交全部 {} 个变更文件", affected.len()),
                    affected,
                    None,
                ))
            } else {
                sync::validate_repo_paths(paths)?;
                sync::precheck_stageable(repo, paths)?;
                Ok((
                    format!("提交 {} 个指定路径", paths.len()),
                    paths.to_vec(),
                    None,
                ))
            }
        }
        GitWriteOp::CreateBranch => {
            let name = branch.unwrap_or_default();
            sync::validate_branch_name(name)?;
            if repo.find_branch(name, git2::BranchType::Local).is_ok() {
                return Err(format!("分支已存在: {name}"));
            }
            let summary = if checkout {
                format!("基于当前 HEAD 创建并切换到新分支 {name}")
            } else {
                format!("基于当前 HEAD 创建新分支 {name}")
            };
            Ok((summary, vec![], Some(name.to_string())))
        }
        GitWriteOp::CheckoutBranch => {
            let name = branch.unwrap_or_default();
            sync::validate_branch_name(name)?;
            match sync::checkout_conflict(repo, name)? {
                Some(conflict) => Err(format!(
                    "工作区有未提交改动且与目标分支冲突，禁止切换: {conflict}"
                )),
                None => Ok((
                    format!("切换到本地分支 {name}"),
                    vec![],
                    Some(name.to_string()),
                )),
            }
        }
        GitWriteOp::Push => {
            // 只读预检：分支合法、非 detached HEAD、远端已配置、预览领先提交数。
            // 不读 KeyringStore、不构造任何网络调用。
            let head = repo.head().map_err(|e| e.to_string())?;
            if !head.is_branch() {
                return Err("当前为 detached HEAD，禁止推送".to_string());
            }
            let name = head
                .shorthand()
                .ok_or_else(|| "无法解析当前分支名".to_string())?
                .to_string();
            sync::validate_branch_name(&name)?;
            if repo.find_remote(sync::GIT_PUSH_REMOTE).is_err() {
                return Err(format!("远端未配置: {}", sync::GIT_PUSH_REMOTE));
            }
            let summary = match sync::push_ahead(repo, &name)? {
                Some(0) => {
                    format!("推送当前分支 {name} 到 origin（远端已是最新，非 force）")
                }
                Some(n) => {
                    format!("推送当前分支 {name} 到 origin（领先 {n} 个提交，非 force）")
                }
                None => format!("首次推送当前分支 {name} 到 origin（远端尚无此分支，非 force）"),
            };
            Ok((summary, vec![], Some(name)))
        }
    }
}

/// Git 写 阶段一：生成待确认 GitWriteJob 与预览（不执行任何写）。
#[tauri::command]
pub fn request_git_write(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
    op: String,
    paths: Option<Vec<String>>,
    message: Option<String>,
    branch: Option<String>,
    checkout: Option<bool>,
) -> Result<GitWritePreview, String> {
    check_invocation_source(&webview, "request_git_write", None, &app)?;
    // 1) 操作白名单：非白名单（reset/push/merge/rebase/stash/clean…）立即拒绝
    let op = match GitWriteOp::from_op_str(&op) {
        Some(op) => op,
        None => {
            workspace::log_audit(&app, "git_write_rejected", format!("操作禁止: {op}"));
            return Err("操作禁止".to_string());
        }
    };
    // 2) 路径锁定：repo_id 必须命中 repos.json 且禁止逃逸（复用 M1-5）
    let repo = sync::open_readonly(&app, &repo_id).map_err(|e| {
        workspace::log_audit(&app, "git_write_rejected", format!("仓库校验失败: {e}"));
        e
    })?;
    // 3) 参数校验 + 只读预检（任何失败都零写入，且落 git_write_rejected 审计）
    let paths = paths.unwrap_or_default();
    let checkout = checkout.unwrap_or(false);
    let (summary, affected, derived_branch) = match precheck_git_write(
        &repo,
        op,
        &paths,
        message.as_deref(),
        branch.as_deref(),
        checkout,
    ) {
        Ok(v) => v,
        Err(e) => {
            workspace::log_audit(
                &app,
                "git_write_rejected",
                git_write_audit_detail(
                    op,
                    &repo_id,
                    "",
                    paths.len(),
                    false,
                    None,
                    None,
                    &sanitize_audit_text(&e),
                ),
            );
            return Err(e);
        }
    };
    // 4) 生成一次性待确认任务（5 分钟有效），不执行任何写
    let now = chrono::Utc::now();
    let job = GitWriteJob {
        id: uuid::Uuid::new_v4().to_string(),
        repo_id: repo_id.clone(),
        op,
        paths,
        message: message.filter(|m| !m.is_empty()),
        // 分支名：优先预检推导值（push 从 HEAD 推导），否则取调用方参数
        branch: derived_branch.or_else(|| branch.filter(|b| !b.is_empty())),
        checkout,
        status: GitWriteStatus::Pending,
        dangerous: op.is_dangerous(),
        created_at: now,
        expires_at: now + chrono::Duration::seconds(GIT_WRITE_JOB_TTL_SECS),
        finished_at: None,
        error: None,
    };
    let path_count = affected.len();
    {
        let state = app.state::<crate::AppState>();
        let mut jobs = state.pending_git_jobs.lock().unwrap();
        // 顺带清理过期任务（否则未被 confirm 的任务会一直在表里累积）
        purge_expired_git_jobs(&mut jobs, now);
        jobs.insert(job.id.clone(), job.clone());
    }
    let audit_remote = matches!(op, GitWriteOp::Push).then_some(sync::GIT_PUSH_REMOTE);
    workspace::log_audit(
        &app,
        "git_write_request",
        git_write_audit_detail(
            op,
            &repo_id,
            &job.id,
            path_count,
            false,
            job.branch.as_deref(),
            audit_remote,
            &summary,
        ),
    );
    Ok(GitWritePreview {
        job_id: job.id,
        repo_id,
        op,
        summary,
        affected_paths: affected
            .into_iter()
            .take(GIT_WRITE_PREVIEW_MAX_PATHS)
            .collect(),
        path_count,
        dangerous: job.dangerous,
        expires_at: job.expires_at,
    })
}

/// Git 写 阶段二（闸门）：用户确认后才执行。dangerous 操作必须二次确认。
/// 后台线程执行，完成（成功/失败）通过 `git-write-completed` 事件通知前端。
#[tauri::command]
pub fn confirm_git_write(
    app: AppHandle,
    webview: tauri::Webview,
    job_id: String,
    confirmed_dangerous: Option<bool>,
) -> Result<GitWriteJob, String> {
    check_invocation_source(&webview, "confirm_git_write", None, &app)?;
    let running = {
        let state = app.state::<crate::AppState>();
        let mut jobs = state.pending_git_jobs.lock().unwrap();
        match take_confirmable_git_job(
            &mut jobs,
            &job_id,
            chrono::Utc::now(),
            confirmed_dangerous.unwrap_or(false),
        ) {
            Ok(job) => job,
            Err(e) => {
                workspace::log_audit(
                    &app,
                    "git_write_rejected",
                    format!("确认闸门拒绝 job={job_id}: {e}"),
                );
                return Err(e);
            }
        }
    };

    let app_thread = app.clone();
    let job_clone = running.clone();
    std::thread::spawn(move || {
        let result = execute_git_write(&app_thread, &job_clone);
        let mut finished = job_clone.clone();
        finished.finished_at = Some(chrono::Utc::now());
        let audit_remote =
            matches!(job_clone.op, GitWriteOp::Push).then_some(sync::GIT_PUSH_REMOTE);
        match result {
            Ok(path_count) => {
                finished.status = GitWriteStatus::Success;
                workspace::log_audit(
                    &app_thread,
                    "git_write",
                    git_write_audit_detail(
                        job_clone.op,
                        &job_clone.repo_id,
                        &job_clone.id,
                        path_count,
                        true,
                        job_clone.branch.as_deref(),
                        audit_remote,
                        "ok",
                    ),
                );
            }
            Err(e) => {
                let clean = sanitize_audit_text(&e);
                finished.status = GitWriteStatus::Failed;
                finished.error = Some(clean.clone());
                workspace::log_audit(
                    &app_thread,
                    "git_write_failed",
                    git_write_audit_detail(
                        job_clone.op,
                        &job_clone.repo_id,
                        &job_clone.id,
                        job_clone.paths.len(),
                        true,
                        job_clone.branch.as_deref(),
                        audit_remote,
                        &clean,
                    ),
                );
            }
        }
        let _ = app_thread.emit("git-write-completed", finished);
    });

    Ok(running)
}

/// 阶段二执行体：重新做路径锁定与参数校验后才执行写（fail-closed 双保险）。
/// 返回实际影响路径数（供审计 `path_count` 字段使用）；分支类操作恒为 0。
///
/// 注意：`sync::open_readonly` 只是「打开已锁定路径下的仓库」的既有命名
/// （内部复用 M1-5 的 `repo_dir` 路径锁定），写操作同样经它进入。
fn execute_git_write(app: &AppHandle, job: &GitWriteJob) -> Result<usize, String> {
    let repo = sync::open_readonly(app, &job.repo_id)?;
    match job.op {
        GitWriteOp::Stage => sync::write_stage(&repo, &job.paths),
        GitWriteOp::Unstage => sync::write_unstage(&repo, &job.paths),
        GitWriteOp::Discard => sync::write_discard(&repo, &job.paths),
        GitWriteOp::Commit => {
            let msg = job.message.clone().unwrap_or_default();
            let paths = if job.paths.is_empty() {
                None
            } else {
                Some(job.paths.as_slice())
            };
            // 全量提交（paths=None）用执行前的脏文件数作口径：
            // job.paths 此时为空，若直接取 len() 会让审计永远记为 0。
            let count = match paths {
                None => sync::read_status(&repo)?.len(),
                Some(ps) => ps.len(),
            };
            sync::write_commit(&repo, &msg, paths).map(|_| count)
        }
        GitWriteOp::CreateBranch => {
            let name = job.branch.clone().unwrap_or_default();
            sync::write_create_branch(&repo, &name, job.checkout).map(|_| 0)
        }
        GitWriteOp::CheckoutBranch => {
            let name = job.branch.clone().unwrap_or_default();
            sync::write_checkout_branch(&repo, &name).map(|_| 0)
        }
        GitWriteOp::Push => {
            // 凭据仅在推送瞬间从系统密钥库读取（与 push_artifacts 同一思路），
            // 不持久化、不复制到任务结构；错误串先脱敏再向上传递。
            let repos = workspace::load_repos(app);
            let cfg = repos
                .iter()
                .find(|r| r.id == job.repo_id)
                .ok_or_else(|| "仓库未配置".to_string())?;
            let token = KeyringStore::get_token(&cfg.id)?;
            sync::write_push(&repo, &cfg.username, &token)
                .map(|_| 0)
                .map_err(|e| sync::scrub_sensitive_error(&e, &token))
        }
    }
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
    app.state::<crate::AppState>()
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
    let state = app.state::<crate::AppState>();
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
        let state = app_thread.state::<crate::AppState>();
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
pub fn git_log(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
) -> Result<Vec<GitLogEntry>, String> {
    check_invocation_source(&webview, "git_log", None, &app)?;
    let repo = sync::open_readonly(&app, &repo_id)?;
    if repo.is_empty().unwrap_or(false) {
        return Ok(vec![]);
    }
    let mut walk = repo.revwalk().map_err(|_| "GIT_LOG_FAILED")?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
        .map_err(|_| "GIT_LOG_FAILED")?;
    walk.push_head().map_err(|_| "GIT_LOG_FAILED")?;
    walk.take(200)
        .map(|id| {
            let commit = repo
                .find_commit(id.map_err(|_| "GIT_LOG_FAILED")?)
                .map_err(|_| "GIT_LOG_FAILED")?;
            let author = commit
                .author()
                .name()
                .unwrap_or("")
                .chars()
                .take(128)
                .collect();
            Ok(GitLogEntry {
                oid: commit.id().to_string(),
                parents: commit.parent_ids().map(|id| id.to_string()).collect(),
                summary: commit.summary().unwrap_or("").chars().take(1024).collect(),
                author,
                time: commit.time().seconds(),
            })
        })
        .collect()
}

#[tauri::command]
pub fn git_commit_diff(
    app: AppHandle,
    webview: tauri::Webview,
    repo_id: String,
    oid: String,
) -> Result<String, String> {
    check_invocation_source(&webview, "git_commit_diff", None, &app)?;
    let repo = sync::open_readonly(&app, &repo_id)?;
    let oid = git2::Oid::from_str(&oid).map_err(|_| "GIT_INVALID_OID")?;
    let commit = repo.find_commit(oid).map_err(|_| "GIT_COMMIT_MISSING")?;
    let tree = commit.tree().map_err(|_| "GIT_DIFF_FAILED")?;
    let parent = if commit.parent_count() > 0 {
        Some(
            commit
                .parent(0)
                .and_then(|p| p.tree())
                .map_err(|_| "GIT_DIFF_FAILED")?,
        )
    } else {
        None
    };
    let diff = repo
        .diff_tree_to_tree(parent.as_ref(), Some(&tree), None)
        .map_err(|_| "GIT_DIFF_FAILED")?;
    let mut out = String::new();
    let mut truncated = false;
    let printed = diff.print(git2::DiffFormat::Patch, |_, _, line| {
        let text = String::from_utf8_lossy(line.content());
        if out.len() + text.len() > 256 * 1024 {
            truncated = true;
            return false;
        }
        if matches!(line.origin(), '+' | '-' | ' ') {
            out.push(line.origin());
        }
        out.push_str(&text);
        true
    });
    if !truncated {
        printed.map_err(|_| "GIT_DIFF_FAILED")?;
    }
    if truncated {
        out.push_str("\n[Diff truncated at 256 KiB]\n");
    }
    Ok(out)
}

#[cfg(test)]
mod git_write_gate_tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_job(op: GitWriteOp, created: chrono::DateTime<Utc>, ttl_secs: i64) -> GitWriteJob {
        GitWriteJob {
            id: uuid::Uuid::new_v4().to_string(),
            repo_id: "r1".into(),
            op,
            paths: vec!["a.txt".into()],
            message: None,
            branch: None,
            checkout: false,
            status: GitWriteStatus::Pending,
            dangerous: op.is_dangerous(),
            created_at: created,
            expires_at: created + Duration::seconds(ttl_secs),
            finished_at: None,
            error: None,
        }
    }

    // T-gw-2（b 卡）：未确认执行 → confirm 不存在的 job 必须 Err("未知任务")，零写入
    #[test]
    fn git_write_gate_unknown_job_rejected() {
        let mut jobs = HashMap::new();
        let err = take_confirmable_git_job(&mut jobs, "nope", Utc::now(), false)
            .expect_err("未知任务必须拒绝");
        assert_eq!(err, "未知任务");
    }

    // T-gw-3（b 卡）：过期 job 被拒且清除，零写入
    #[test]
    fn git_write_gate_expired_job_rejected_and_removed() {
        let mut jobs = HashMap::new();
        let job = make_job(GitWriteOp::Stage, Utc::now() - Duration::seconds(600), 300);
        let id = job.id.clone();
        jobs.insert(id.clone(), job);
        let err = take_confirmable_git_job(&mut jobs, &id, Utc::now(), false)
            .expect_err("过期任务必须拒绝");
        assert_eq!(err, "任务已过期");
        assert!(jobs.is_empty(), "过期任务应被清除");
    }

    // T-gw-b-3：discard（dangerous）未带二次确认必须拒绝，且任务不被执行/不取出
    #[test]
    fn git_write_gate_discard_requires_double_confirm() {
        let mut jobs = HashMap::new();
        let job = make_job(GitWriteOp::Discard, Utc::now(), 300);
        let id = job.id.clone();
        jobs.insert(id.clone(), job);
        let err = take_confirmable_git_job(&mut jobs, &id, Utc::now(), false)
            .expect_err("缺二次确认必须拒绝");
        assert_eq!(err, "需二次确认");
        assert!(
            jobs.contains_key(&id),
            "未确认的 dangerous 任务应保留在表内（未执行，用户可补确认）"
        );
        // 带上二次确认：放行并一次性取出（Running）
        let taken =
            take_confirmable_git_job(&mut jobs, &id, Utc::now(), true).expect("二次确认后应放行");
        assert_eq!(taken.status, GitWriteStatus::Running);
        assert!(jobs.is_empty(), "确认后任务必须一次性移除");
    }

    // T-gw-2b：任务一次性（confirm 后再次 confirm 必须 Err）+ 非 Pending 状态拒绝
    #[test]
    fn git_write_gate_one_shot_and_pending_only() {
        let mut jobs = HashMap::new();
        let job = make_job(GitWriteOp::Commit, Utc::now(), 300);
        let id = job.id.clone();
        jobs.insert(id.clone(), job);
        take_confirmable_git_job(&mut jobs, &id, Utc::now(), false).expect("首次确认");
        let err = take_confirmable_git_job(&mut jobs, &id, Utc::now(), false)
            .expect_err("重复确认必须拒绝");
        assert_eq!(err, "未知任务");

        let mut jobs2 = HashMap::new();
        let mut running_job = make_job(GitWriteOp::Stage, Utc::now(), 300);
        running_job.status = GitWriteStatus::Running;
        let id2 = running_job.id.clone();
        jobs2.insert(id2.clone(), running_job);
        let err2 = take_confirmable_git_job(&mut jobs2, &id2, Utc::now(), false)
            .expect_err("非 Pending 状态必须拒绝");
        assert_eq!(err2, "任务状态异常");
        assert!(jobs2.is_empty(), "异常状态任务应被清除");
    }

    // T-gw-c-7：过期任务清理只清过期项，有效期内任务必须保留
    #[test]
    fn git_write_gate_purge_expired_jobs() {
        let mut jobs = HashMap::new();
        let now = Utc::now();
        let fresh = make_job(GitWriteOp::Stage, now, 300);
        let fresh_id = fresh.id.clone();
        let stale = make_job(GitWriteOp::Discard, now - Duration::seconds(600), 300);
        jobs.insert(fresh.id.clone(), fresh);
        jobs.insert(stale.id.clone(), stale);
        let removed = purge_expired_git_jobs(&mut jobs, now);
        assert_eq!(removed, 1, "只应清理已过期任务");
        assert!(jobs.contains_key(&fresh_id), "有效期内任务必须保留");
        assert_eq!(jobs.len(), 1);
    }

    // T-gw-d-8：push（dangerous）未带二次确认必须拒绝，确认后一次性取出
    #[test]
    fn git_write_gate_push_requires_double_confirm() {
        let mut jobs = HashMap::new();
        let mut job = make_job(GitWriteOp::Push, Utc::now(), 300);
        job.branch = Some("master".into());
        let id = job.id.clone();
        assert!(job.dangerous, "push 必须标记为 dangerous");
        jobs.insert(id.clone(), job);
        let err = take_confirmable_git_job(&mut jobs, &id, Utc::now(), false)
            .expect_err("缺二次确认必须拒绝");
        assert_eq!(err, "需二次确认");
        assert!(jobs.contains_key(&id), "未确认的 push 任务不得被执行/取出");
        let taken =
            take_confirmable_git_job(&mut jobs, &id, Utc::now(), true).expect("二次确认后应放行");
        assert_eq!(taken.status, GitWriteStatus::Running);
        assert!(jobs.is_empty(), "确认后任务必须一次性移除");
    }

    // T-gw-b-12：审计 detail 只含操作语义与计数，绝无凭据/路径清单/diff 全量
    #[test]
    fn git_write_audit_detail_contains_no_credentials_or_path_lists() {
        let d = git_write_audit_detail(GitWriteOp::Commit, "r1", "j1", 3, true, None, None, "ok");
        assert!(d.contains("\"op\":\"commit\""), "detail: {d}");
        assert!(d.contains("\"repo_id\":\"r1\""), "detail: {d}");
        assert!(d.contains("\"path_count\":3"), "detail: {d}");
        assert!(d.contains("\"confirmed\":true"), "detail: {d}");
        for forbidden in ["userpass", "token", "secret", "password", "a.txt", ".git/"] {
            assert!(
                !d.contains(forbidden),
                "审计 detail 不得含 {forbidden}: {d}"
            );
        }
        // 长错误串必须被截断（防整段 diff/冗长文本落审计），且不切 UTF-8 字符
        let long = "你".repeat(500); // 每字 3 字节，共 1500 字节
        let s = sanitize_audit_text(&long);
        assert!(s.len() <= 300 + "…".len(), "截断后长度应受限: {}", s.len());
        assert!(
            std::str::from_utf8(s.as_bytes()).is_ok(),
            "必须是合法 UTF-8"
        );
        let short = "短错误";
        assert_eq!(sanitize_audit_text(short), short, "短文本不应被改动");
    }

    // T-gw-d-9：push 审计必须含 branch/remote_name，且不得含 token/远端 URL 凭据
    #[test]
    fn git_write_push_audit_fields_present_and_sanitized() {
        let d = git_write_audit_detail(
            GitWriteOp::Push,
            "r1",
            "j1",
            0,
            true,
            Some("master"),
            Some("origin"),
            "ok",
        );
        assert!(d.contains("\"op\":\"push\""), "detail: {d}");
        assert!(d.contains("\"branch\":\"master\""), "detail: {d}");
        assert!(d.contains("\"remote_name\":\"origin\""), "detail: {d}");
        assert!(d.contains("\"confirmed\":true"), "detail: {d}");
        for forbidden in [
            "userpass", "token", "secret", "password", "https://", "://", "@",
        ] {
            assert!(
                !d.contains(forbidden),
                "push 审计不得含凭据/远端 URL: {forbidden} in {d}"
            );
        }
        // 失败路径：错误串经 scrub + 截断后落审计，token 与 URL userinfo 不出现
        // （URL host 保留用于排障，userinfo 统一掩码为 ***，审计另有 remote_name 字段）
        let raw = "push 失败: cannot push to 'https://u:tok-123@example.com/r.git'（auth failed for tok-123）";
        let clean = sanitize_audit_text(&sync::scrub_sensitive_error(raw, "tok-123"));
        assert!(!clean.contains("tok-123"), "错误串必须脱敏: {clean}");
        assert!(!clean.contains("u:tok"), "URL userinfo 必须掩码: {clean}");
        assert!(
            clean.contains("https://***@example.com"),
            "userinfo 应为 ***: {clean}"
        );
        let failed = git_write_audit_detail(
            GitWriteOp::Push,
            "r1",
            "j1",
            0,
            true,
            Some("master"),
            Some("origin"),
            &clean,
        );
        assert!(!failed.contains("tok-123"), "失败审计必须脱敏: {failed}");
        assert!(
            !failed.contains("u:tok"),
            "失败审计不得含带凭据的 URL: {failed}"
        );
        assert!(
            !failed.contains("\"branch\":null"),
            "失败审计必须带分支名: {failed}"
        );
    }
}
