//! Authorized local Git workbench. Unlike managed artifact Git operations,
//! this module works on the ORIGINAL worktree selected by the user.
//! Every mutation uses a short-lived, one-time, server-side confirmation ticket.
//! No shell interpolation, credential logging, force checkout, force push or reset.
use crate::capabilities::git::local_readonly::{discover_allowed, LocalGitContext};
use crate::shared::invocation::check_invocation_source;
use crate::workspace;
use git2::{
    BranchType, Cred, FetchOptions, RemoteCallbacks, Repository, StashFlags, Status, StatusOptions,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Webview};

const MAX_CHANGED: usize = 500;
const MAX_TEXT: usize = 128 * 1024;
const JOB_TTL: Duration = Duration::from_secs(300);

#[derive(Default)]
pub struct LocalGitWriteQueue(Mutex<HashMap<String, LocalWriteTicket>>);

#[derive(Clone)]
struct LocalWriteTicket {
    path: String,
    action: String,
    branch: Option<String>,
    message: Option<String>,
    resolved: Option<String>,
    pr: Option<u32>,
    pr_head: Option<String>,
    expected_head: Option<String>,
    expected_index: String,
    expires: Instant,
}

#[derive(Serialize)]
pub struct LocalWritePreview {
    pub job_id: String,
    pub action: String,
    pub scope: String,
    pub summary: String,
    pub dangerous: bool,
    pub expires_in_seconds: u32,
}

#[derive(Serialize)]
pub struct LocalGitStatus {
    pub root: String,
    pub scope: String,
    pub branch: String,
    pub head: Option<String>,
    pub files: Vec<LocalStatusEntry>,
    pub local_branches: Vec<String>,
    pub has_conflicts: bool,
    pub truncated: bool,
}

#[derive(Serialize)]
pub struct LocalStatusEntry {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub conflicted: bool,
}

#[derive(Serialize)]
pub struct LocalConflict {
    pub path: String,
    pub ancestor: String,
    pub ours: String,
    pub theirs: String,
}

fn errors(_err: git2::Error, code: &str) -> String {
    // git2 errors can embed URLs, usernames and remote data. Never expose them.
    code.into()
}

fn selected_file(ctx: &LocalGitContext) -> Result<&Path, String> {
    if ctx.is_dir || ctx.relative_path.is_empty() {
        return Err("GIT_LOCAL_FILE_REQUIRED".into());
    }
    Ok(Path::new(&ctx.relative_path))
}

fn selected_scope<'a>(ctx: &LocalGitContext, p: &'a Path) -> bool {
    ctx.relative_path.is_empty()
        || if ctx.is_dir {
            p.starts_with(&ctx.relative_path)
        } else {
            p == Path::new(&ctx.relative_path)
        }
}

fn status_options() -> StatusOptions {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);
    opts
}

fn clean_worktree(repo: &Repository) -> Result<(), String> {
    if repo.state() != git2::RepositoryState::Clean {
        return Err("GIT_LOCAL_UNFINISHED_OPERATION".into());
    }
    let statuses = repo
        .statuses(Some(&mut status_options()))
        .map_err(|e| errors(e, "GIT_LOCAL_STATUS_FAILED"))?;
    if !statuses.is_empty() {
        return Err("GIT_LOCAL_REQUIRES_CLEAN_WORKTREE".into());
    }
    Ok(())
}

fn scoped_status(
    repo: &Repository,
    ctx: &LocalGitContext,
) -> Result<(Vec<LocalStatusEntry>, bool), String> {
    let all = repo
        .statuses(Some(&mut status_options()))
        .map_err(|e| errors(e, "GIT_LOCAL_STATUS_FAILED"))?;
    let mut files = Vec::new();
    let mut truncated = false;
    for entry in all.iter() {
        let Some(path) = entry.path() else { continue };
        if !selected_scope(ctx, Path::new(path)) {
            continue;
        }
        if files.len() == MAX_CHANGED {
            truncated = true;
            break;
        }
        let s = entry.status();
        files.push(LocalStatusEntry {
            path: path.into(),
            status: if s.contains(Status::CONFLICTED) {
                "conflicted"
            } else if s.is_wt_new() {
                "untracked"
            } else if s.is_wt_deleted() || s.is_index_deleted() {
                "deleted"
            } else if s.is_index_new() {
                "added"
            } else {
                "modified"
            }
            .into(),
            staged: s.intersects(
                Status::INDEX_NEW
                    | Status::INDEX_MODIFIED
                    | Status::INDEX_DELETED
                    | Status::INDEX_RENAMED
                    | Status::INDEX_TYPECHANGE,
            ),
            conflicted: s.contains(Status::CONFLICTED),
        });
    }
    Ok((files, truncated))
}
fn local_status(repo: &Repository, ctx: &LocalGitContext) -> Result<LocalGitStatus, String> {
    let (files, truncated) = scoped_status(repo, ctx)?;
    let mut branches = Vec::new();
    for result in repo
        .branches(Some(BranchType::Local))
        .map_err(|e| errors(e, "GIT_LOCAL_BRANCH_READ_FAILED"))?
    {
        let (branch, _) = result.map_err(|e| errors(e, "GIT_LOCAL_BRANCH_READ_FAILED"))?;
        if let Some(name) = branch
            .name()
            .map_err(|e| errors(e, "GIT_LOCAL_BRANCH_READ_FAILED"))?
        {
            if branches.len() < 250 {
                branches.push(name.to_owned());
            }
        }
    }
    branches.sort();
    Ok(LocalGitStatus {
        root: ctx.root.clone(),
        scope: ctx.relative_path.clone(),
        branch: ctx.branch.clone(),
        head: repo
            .head()
            .ok()
            .and_then(|x| x.target())
            .map(|x| x.to_string()),
        has_conflicts: repo
            .index()
            .map_err(|e| errors(e, "GIT_LOCAL_INDEX_FAILED"))?
            .has_conflicts(),
        files,
        local_branches: branches,
        truncated,
    })
}

fn allowed_action(action: &str) -> bool {
    matches!(
        action,
        "stage" | "commit" | "checkout" | "fetch" | "pull" | "stash" | "resolve" | "github_merge"
    )
}
fn safe_branch(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 200
        && !s.starts_with('-')
        && !s.contains("..")
        && !s.contains("@{")
        && !s.bytes().any(|b| b < 32 || b == 127 || b == b'\\')
        && !s.ends_with('/')
        && !s.ends_with('.')
        && !s.contains(':')
}
fn approved_remote(repo: &Repository) -> Result<git2::Remote<'_>, String> {
    let remote = repo
        .find_remote("origin")
        .map_err(|_| "GIT_LOCAL_ORIGIN_MISSING")?;
    let raw = remote.url().ok_or("GIT_LOCAL_ORIGIN_URL_MISSING")?;
    if raw.starts_with("https://") {
        let url = url::Url::parse(raw).map_err(|_| "GIT_LOCAL_ORIGIN_PROTOCOL_DENIED")?;
        if url.username() != "" || url.password().is_some() || url.fragment().is_some() {
            return Err("GIT_LOCAL_REMOTE_CREDENTIALS_IN_URL_DENIED".into());
        }
    } else if !(raw.starts_with("git@") && raw.contains(':')) && !raw.starts_with("ssh://git@") {
        return Err("GIT_LOCAL_ORIGIN_PROTOCOL_DENIED".into());
    }
    Ok(remote)
}
fn index_snapshot(repo: &Repository) -> Result<String, String> {
    // Pure read: never call Index::write_tree_to during a PREVIEW. That API
    // writes Git objects even if it does not modify the index file.
    let path = repo.path().join("index");
    let bytes = match std::fs::read(path) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(_) => return Err("GIT_LOCAL_INDEX_READ_FAILED".into()),
    };
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("GIT_LOCAL_INDEX_TOO_LARGE".into());
    }
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn check_preconditions(
    repo: &Repository,
    ctx: &LocalGitContext,
    action: &str,
    branch: Option<&str>,
    message: Option<&str>,
    resolved: Option<&str>,
    pr: Option<u32>,
) -> Result<String, String> {
    if !allowed_action(action) {
        return Err("GIT_LOCAL_ACTION_DENIED".into());
    }
    match action {
        "stage" => {
            let (files, truncated) = scoped_status(repo, ctx)?;
            if truncated || files.len() > MAX_CHANGED {
                return Err("GIT_LOCAL_TOO_MANY_FILES".into());
            }
            if files.is_empty() {
                return Err("GIT_LOCAL_NO_CHANGES".into());
            }
            if files.iter().any(|x| x.conflicted) {
                return Err("GIT_LOCAL_CONFLICT_REQUIRES_RESOLVE".into());
            }
            Ok(format!("仅暂存所选路径内的 {} 项改动", files.len()))
        }
        "commit" => {
            let msg = message.unwrap_or("");
            if msg.trim().is_empty() || msg.len() > 2048 || msg.chars().any(|x| x.is_control()) {
                return Err("GIT_LOCAL_INVALID_COMMIT_MESSAGE".into());
            }
            if repo
                .index()
                .map_err(|e| errors(e, "GIT_LOCAL_INDEX_FAILED"))?
                .has_conflicts()
            {
                return Err("GIT_LOCAL_INDEX_CONFLICTED".into());
            }
            // No silent commits of paths outside the right-click scope.
            let all = repo
                .statuses(Some(&mut status_options()))
                .map_err(|e| errors(e, "GIT_LOCAL_STATUS_FAILED"))?;
            let staged = all
                .iter()
                .filter(|x| {
                    x.status().intersects(
                        Status::INDEX_NEW
                            | Status::INDEX_MODIFIED
                            | Status::INDEX_DELETED
                            | Status::INDEX_RENAMED
                            | Status::INDEX_TYPECHANGE,
                    )
                })
                .collect::<Vec<_>>();
            if staged.is_empty() {
                return Err("GIT_LOCAL_NOTHING_STAGED".into());
            }
            if staged.len() > MAX_CHANGED
                || staged
                    .iter()
                    .any(|x| !x.path().is_some_and(|p| selected_scope(ctx, Path::new(p))))
            {
                return Err("GIT_LOCAL_STAGED_OUTSIDE_SCOPE".into());
            }
            repo.signature()
                .map_err(|_| "GIT_LOCAL_CONFIGURE_AUTHOR_NAME_EMAIL")?;
            Ok(format!(
                "只提交 {} 项已暂存文件；不会把工作区其它改动加入提交",
                staged.len()
            ))
        }
        "checkout" => {
            clean_worktree(repo)?;
            let name = branch.ok_or("GIT_LOCAL_BRANCH_REQUIRED")?;
            if !safe_branch(name) {
                return Err("GIT_LOCAL_BAD_BRANCH".into());
            }
            repo.find_branch(name, BranchType::Local)
                .map_err(|_| "GIT_LOCAL_BRANCH_MISSING")?;
            Ok(format!(
                "干净工作区中安全切换到本地分支 {name}；不使用强制覆盖"
            ))
        }
        "fetch" => {
            approved_remote(repo)?;
            Ok("从 origin 抓取远端引用；不更新当前文件、不执行合并".into())
        }
        "pull" => {
            clean_worktree(repo)?;
            approved_remote(repo)?;
            if repo
                .head()
                .ok()
                .and_then(|h| h.shorthand().map(str::to_owned))
                .is_none()
            {
                return Err("GIT_LOCAL_DETACHED_HEAD_DENIED".into());
            }
            Ok("Fetch 后仅允许 fast-forward；分支分叉时拒绝自动合并或改写历史".into())
        }
        "stash" => {
            clean_state(repo)?;
            let statuses = repo
                .statuses(Some(&mut status_options()))
                .map_err(|_| "GIT_LOCAL_STATUS_FAILED")?;
            if statuses.is_empty() {
                return Err("GIT_LOCAL_NO_CHANGES".into());
            }
            Ok("整个仓库的已跟踪和未跟踪改动都会收入 stash；此操作不局限右键文件".into())
        }
        "resolve" => {
            let relative = selected_file(ctx)?;
            let target = repo.index().map_err(|_| "GIT_LOCAL_INDEX_FAILED")?;
            if !target.has_conflicts()
                || target.get_path(relative, 2).is_none() && target.get_path(relative, 3).is_none()
            {
                return Err("GIT_LOCAL_NO_FILE_CONFLICT".into());
            }
            let result = resolved.ok_or("GIT_LOCAL_RESOLUTION_MISSING")?;
            if result.len() > MAX_TEXT || result.contains('\0') {
                return Err("GIT_LOCAL_RESOLUTION_TOO_LARGE".into());
            }
            Ok("将明确编辑的解决结果写入当前冲突文件并暂存；覆盖冲突标记内容".into())
        }
        "github_merge" => {
            let number = pr.ok_or("GIT_LOCAL_PR_REQUIRED")?;
            if number == 0 {
                return Err("GIT_LOCAL_BAD_PR".into());
            }
            let slug = github_slug(repo)?;
            Ok(format!(
                "将 GitHub {slug} 的 PR #{number} 使用 merge commit 合并；不删除分支"
            ))
        }
        _ => Err("GIT_LOCAL_ACTION_DENIED".into()),
    }
}
fn clean_state(repo: &Repository) -> Result<(), String> {
    if repo.state() != git2::RepositoryState::Clean {
        return Err("GIT_LOCAL_UNFINISHED_OPERATION".into());
    }
    Ok(())
}

fn stage_scoped(repo: &Repository, ctx: &LocalGitContext) -> Result<usize, String> {
    let (files, truncated) = scoped_status(repo, ctx)?;
    if truncated || files.len() > MAX_CHANGED {
        return Err("GIT_LOCAL_TOO_MANY_FILES".into());
    }
    let mut index = repo.index().map_err(|_| "GIT_LOCAL_INDEX_FAILED")?;
    for f in &files {
        let p = Path::new(&f.path);
        if p.is_absolute()
            || p.components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("GIT_LOCAL_UNSAFE_PATH".into());
        }
        if f.status == "deleted" {
            index
                .remove_path(p)
                .map_err(|_| "GIT_LOCAL_INDEX_REMOVE_FAILED")?;
        } else {
            index
                .add_path(p)
                .map_err(|_| "GIT_LOCAL_INDEX_ADD_FAILED")?;
        }
    }
    index.write().map_err(|_| "GIT_LOCAL_INDEX_WRITE_FAILED")?;
    Ok(files.len())
}
fn commit_scoped(repo: &Repository, ctx: &LocalGitContext, msg: &str) -> Result<usize, String> {
    check_preconditions(repo, ctx, "commit", None, Some(msg), None, None)?;
    let index = repo.index().map_err(|_| "GIT_LOCAL_INDEX_FAILED")?;
    let tree_id = index
        .write_tree_to(repo)
        .map_err(|_| "GIT_LOCAL_TREE_FAILED")?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|_| "GIT_LOCAL_TREE_FAILED")?;
    let sig = repo
        .signature()
        .map_err(|_| "GIT_LOCAL_CONFIGURE_AUTHOR_NAME_EMAIL")?;
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
        .map_err(|_| "GIT_LOCAL_COMMIT_FAILED")?;
    Ok(1)
}
fn checkout_safe(repo: &Repository, name: &str) -> Result<usize, String> {
    clean_worktree(repo)?;
    let branch = repo
        .find_branch(name, BranchType::Local)
        .map_err(|_| "GIT_LOCAL_BRANCH_MISSING")?;
    let reference = branch.into_reference();
    let refname = reference
        .name()
        .ok_or("GIT_LOCAL_BRANCH_INVALID")?
        .to_string();
    let target = reference
        .peel_to_commit()
        .map_err(|_| "GIT_LOCAL_BRANCH_INVALID")?;
    // Git2 SAFE checkout fails instead of clobbering modified or untracked paths.
    repo.checkout_tree(
        target.as_object(),
        Some(git2::build::CheckoutBuilder::new().safe()),
    )
    .map_err(|_| "GIT_LOCAL_CHECKOUT_BLOCKED")?;
    repo.set_head(&refname)
        .map_err(|_| "GIT_LOCAL_CHECKOUT_HEAD_FAILED")?;
    Ok(1)
}
fn fetch_origin(repo: &Repository) -> Result<(), String> {
    let mut remote = approved_remote(repo)?;
    let config = repo.config().map_err(|_| "GIT_LOCAL_CONFIG_FAILED")?;
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, user, allowed| {
        if allowed.contains(git2::CredentialType::SSH_KEY) {
            let name = user.unwrap_or("git");
            Cred::ssh_key_from_agent(name)
        } else if allowed.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            Cred::credential_helper(&config, url, user)
        } else {
            Cred::default()
        }
    });
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callbacks);
    // Only origin's normal configured fetch refspecs. Never force push or rewrite worktree.
    remote
        .fetch(&[] as &[&str], Some(&mut opts), None)
        .map_err(|_| "GIT_LOCAL_FETCH_FAILED")?;
    Ok(())
}
fn pull_ff_only(repo: &Repository) -> Result<usize, String> {
    clean_worktree(repo)?;
    let branch = repo
        .head()
        .map_err(|_| "GIT_LOCAL_UNBORN_HEAD")?
        .shorthand()
        .ok_or("GIT_LOCAL_DETACHED_HEAD_DENIED")?
        .to_string();
    fetch_origin(repo)?;
    let remote = repo
        .find_reference(&format!("refs/remotes/origin/{branch}"))
        .map_err(|_| "GIT_LOCAL_UPSTREAM_MISSING")?;
    let oid = remote.target().ok_or("GIT_LOCAL_REMOTE_HEAD_INVALID")?;
    let local = repo
        .head()
        .map_err(|_| "GIT_LOCAL_HEAD_FAILED")?
        .target()
        .ok_or("GIT_LOCAL_HEAD_INVALID")?;
    if oid == local {
        return Ok(0);
    }
    if !repo
        .graph_descendant_of(oid, local)
        .map_err(|_| "GIT_LOCAL_GRAPH_FAILED")?
    {
        return Err("GIT_LOCAL_DIVERGED_USE_THREE_WAY_MERGE".into());
    }
    let target = repo
        .find_object(oid, None)
        .map_err(|_| "GIT_LOCAL_TARGET_FAILED")?;
    repo.checkout_tree(&target, Some(git2::build::CheckoutBuilder::new().safe()))
        .map_err(|_| "GIT_LOCAL_PULL_CHECKOUT_BLOCKED")?;
    let mut local_ref = repo
        .find_reference(&format!("refs/heads/{branch}"))
        .map_err(|_| "GIT_LOCAL_BRANCH_MISSING")?;
    local_ref
        .set_target(oid, "BrowserOS approved fast-forward pull")
        .map_err(|_| "GIT_LOCAL_PULL_REF_FAILED")?;
    Ok(1)
}
fn github_slug(repo: &Repository) -> Result<String, String> {
    let remote = approved_remote(repo)?;
    let url = remote.url().ok_or("GITHUB_ORIGIN_MISSING")?;
    let tail = if let Some(p) = url.strip_prefix("https://github.com/") {
        p
    } else if let Some(p) = url.strip_prefix("git@github.com:") {
        p
    } else if let Some(p) = url.strip_prefix("ssh://git@github.com/") {
        p
    } else {
        return Err("GITHUB_ORIGIN_NOT_GITHUB".into());
    };
    let slug = tail.trim_end_matches(".git").trim_end_matches('/');
    let parts: Vec<&str> = slug.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 100
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        })
    {
        return Err("GITHUB_ORIGIN_INVALID".into());
    }
    Ok(slug.into())
}
fn gh_output(args: &[&str]) -> Result<serde_json::Value, String> {
    let result = std::process::Command::new("gh")
        .args(args)
        .env("GH_PROMPT_DISABLED", "1")
        .output()
        .map_err(|_| "GITHUB_CLI_NOT_INSTALLED_OR_UNAVAILABLE")?;
    if !result.status.success() {
        return Err("GITHUB_AUTH_OR_API_FAILED".into());
    }
    if result.stdout.len() > 512 * 1024 {
        return Err("GITHUB_RESULT_TOO_LARGE".into());
    }
    serde_json::from_slice(&result.stdout).map_err(|_| "GITHUB_JSON_FAILED".into())
}
fn gh_merge_preview(repo: &Repository, number: u32) -> Result<String, String> {
    let slug = github_slug(repo)?;
    let num = number.to_string();
    let data = gh_output(&[
        "pr",
        "view",
        &num,
        "--repo",
        &slug,
        "--json",
        "state,isDraft,mergeStateStatus,reviewDecision,statusCheckRollup,headRefOid",
    ])?;
    if data["state"] != "OPEN" || data["isDraft"] != false {
        return Err("GITHUB_PR_NOT_OPEN".into());
    }
    let state = data["mergeStateStatus"].as_str().unwrap_or("");
    if state != "CLEAN" && state != "UNSTABLE" {
        return Err("GITHUB_PR_NOT_MERGEABLE".into());
    }
    if data["reviewDecision"] == "CHANGES_REQUESTED" {
        return Err("GITHUB_REVIEW_CHANGES_REQUESTED".into());
    }
    let checks = data["statusCheckRollup"]
        .as_array()
        .ok_or("GITHUB_CI_MISSING")?;
    if checks.is_empty()
        || checks.iter().any(|c| {
            let status = c["status"].as_str().unwrap_or("COMPLETED");
            let conclusion = c["conclusion"].as_str().unwrap_or("SUCCESS");
            status != "COMPLETED" || !matches!(conclusion, "SUCCESS" | "NEUTRAL" | "SKIPPED")
        })
    {
        return Err("GITHUB_CI_NOT_ALL_GREEN".into());
    }
    let head = data["headRefOid"]
        .as_str()
        .ok_or("GITHUB_PR_HEAD_MISSING")?;
    if head.len() != 40 || !head.bytes().all(|x| x.is_ascii_hexdigit()) {
        return Err("GITHUB_PR_HEAD_INVALID".into());
    }
    Ok(head.into())
}

fn perform(
    repo: &mut Repository,
    ctx: &LocalGitContext,
    job: &LocalWriteTicket,
) -> Result<usize, String> {
    match job.action.as_str() {
        "stage" => stage_scoped(repo, ctx),
        "commit" => commit_scoped(repo, ctx, job.message.as_deref().unwrap_or("")),
        "checkout" => checkout_safe(
            repo,
            job.branch.as_deref().ok_or("GIT_LOCAL_BRANCH_REQUIRED")?,
        ),
        "fetch" => {
            fetch_origin(repo)?;
            Ok(1)
        }
        "pull" => pull_ff_only(repo),
        "stash" => {
            let sig = repo
                .signature()
                .map_err(|_| "GIT_LOCAL_CONFIGURE_AUTHOR_NAME_EMAIL")?;
            repo.stash_save(
                &sig,
                "BrowserOS approved stash",
                Some(StashFlags::INCLUDE_UNTRACKED),
            )
            .map_err(|_| "GIT_LOCAL_STASH_FAILED")?;
            Ok(1)
        }
        "resolve" => {
            let rel = selected_file(ctx)?;
            let contents = job
                .resolved
                .as_deref()
                .ok_or("GIT_LOCAL_RESOLUTION_MISSING")?;
            let path = Path::new(&ctx.root).join(rel);
            if path
                .symlink_metadata()
                .map_err(|_| "GIT_LOCAL_FILE_MISSING")?
                .file_type()
                .is_symlink()
            {
                return Err("GIT_LOCAL_SYMLINK_DENIED".into());
            }
            let temp =
                path.with_file_name(format!(".browseros-resolve-{}.tmp", uuid::Uuid::new_v4()));
            std::fs::write(&temp, contents).map_err(|_| "GIT_LOCAL_RESOLVE_WRITE_FAILED")?;
            if std::fs::rename(&temp, &path).is_err() {
                let _ = std::fs::remove_file(&temp);
                return Err("GIT_LOCAL_RESOLVE_RENAME_FAILED".into());
            }
            let mut index = repo.index().map_err(|_| "GIT_LOCAL_INDEX_FAILED")?;
            index
                .add_path(rel)
                .map_err(|_| "GIT_LOCAL_RESOLVE_INDEX_FAILED")?;
            index
                .write()
                .map_err(|_| "GIT_LOCAL_RESOLVE_INDEX_FAILED")?;
            Ok(1)
        }
        "github_merge" => {
            let pr = job.pr.ok_or("GIT_LOCAL_PR_REQUIRED")?;
            let slug = github_slug(repo)?;
            let head = gh_merge_preview(repo, pr)?;
            if Some(head.as_str()) != job.pr_head.as_deref() {
                return Err("GITHUB_PR_HEAD_CHANGED".into());
            }
            let num = pr.to_string();
            let result = std::process::Command::new("gh")
                .args([
                    "pr",
                    "merge",
                    &num,
                    "--repo",
                    &slug,
                    "--merge",
                    "--match-head-commit",
                    &head,
                ])
                .env("GH_PROMPT_DISABLED", "1")
                .output()
                .map_err(|_| "GITHUB_CLI_NOT_INSTALLED_OR_UNAVAILABLE")?;
            if !result.status.success() {
                return Err("GITHUB_PR_MERGE_REJECTED".into());
            }
            Ok(1)
        }
        _ => Err("GIT_LOCAL_ACTION_DENIED".into()),
    }
}

fn inspect(path: &str, roots: &[PathBuf]) -> Result<LocalGitStatus, String> {
    let (repo, ctx) = discover_allowed(path, roots)?;
    local_status(&repo, &ctx)
}
#[tauri::command]
pub async fn git_local_inspect(
    app: AppHandle,
    webview: Webview,
    path: String,
) -> Result<LocalGitStatus, String> {
    check_invocation_source(&webview, "git_local_inspect", None, &app)?;
    let roots = workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move || inspect(&path, &roots))
        .await
        .map_err(|_| "GIT_LOCAL_WORKER_FAILED".to_string())?
}

#[tauri::command]
pub async fn git_local_write_request(
    app: AppHandle,
    webview: Webview,
    path: String,
    action: String,
    message: Option<String>,
    branch: Option<String>,
    resolved: Option<String>,
    pr: Option<u32>,
) -> Result<LocalWritePreview, String> {
    check_invocation_source(&webview, "git_local_write_request", None, &app)?;
    let roots = workspace::allowed_roots(&app);
    let (summary, ticket) = tauri::async_runtime::spawn_blocking(move || {
        let (repo, ctx) = discover_allowed(&path, &roots)?;
        let summary = check_preconditions(
            &repo,
            &ctx,
            &action,
            branch.as_deref(),
            message.as_deref(),
            resolved.as_deref(),
            pr,
        )?;
        let pr_head = if action == "github_merge" {
            Some(gh_merge_preview(&repo, pr.ok_or("GIT_LOCAL_PR_REQUIRED")?)?)
        } else {
            None
        };
        let expected_head = repo
            .head()
            .ok()
            .and_then(|h| h.target())
            .map(|x| x.to_string());
        let expected_index = index_snapshot(&repo)?;
        Ok::<_, String>((
            summary,
            LocalWriteTicket {
                path,
                action,
                branch,
                message,
                resolved,
                pr,
                pr_head,
                expected_head,
                expected_index,
                expires: Instant::now() + JOB_TTL,
            },
        ))
    })
    .await
    .map_err(|_| "GIT_LOCAL_WORKER_FAILED".to_string())??;
    let job_id = uuid::Uuid::new_v4().to_string();
    let mut queue = app
        .state::<LocalGitWriteQueue>()
        .0
        .lock()
        .map_err(|_| "GIT_LOCAL_QUEUE_LOCK_FAILED")?;
    queue.retain(|_, item| item.expires > Instant::now());
    if queue.len() >= 32 {
        return Err("GIT_LOCAL_TOO_MANY_PENDING_ACTIONS".into());
    }
    let dangerous = matches!(
        ticket.action.as_str(),
        "checkout" | "pull" | "stash" | "resolve" | "github_merge"
    );
    let preview = LocalWritePreview {
        job_id: job_id.clone(),
        action: ticket.action.clone(),
        scope: ticket.path.clone(),
        summary,
        dangerous,
        expires_in_seconds: 300,
    };
    queue.insert(job_id, ticket);
    Ok(preview)
}
#[tauri::command]
pub async fn git_local_write_confirm(
    app: AppHandle,
    webview: Webview,
    job_id: String,
    confirmed_dangerous: bool,
) -> Result<String, String> {
    check_invocation_source(&webview, "git_local_write_confirm", None, &app)?;
    let job = {
        let mut guard = app
            .state::<LocalGitWriteQueue>()
            .0
            .lock()
            .map_err(|_| "GIT_LOCAL_QUEUE_LOCK_FAILED")?;
        let ticket = guard.get(&job_id).ok_or("GIT_LOCAL_UNKNOWN_JOB")?;
        if ticket.expires <= Instant::now() {
            guard.remove(&job_id);
            return Err("GIT_LOCAL_JOB_EXPIRED".into());
        }
        if matches!(
            ticket.action.as_str(),
            "checkout" | "pull" | "stash" | "resolve" | "github_merge"
        ) && !confirmed_dangerous
        {
            return Err("GIT_LOCAL_SECOND_CONFIRMATION_REQUIRED".into());
        }
        guard.remove(&job_id).ok_or("GIT_LOCAL_UNKNOWN_JOB")?
    };
    let roots = workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let (mut repo, ctx) = discover_allowed(&job.path, &roots)?;
        let current_head = repo
            .head()
            .ok()
            .and_then(|h| h.target())
            .map(|x| x.to_string());
        if current_head != job.expected_head || index_snapshot(&repo)? != job.expected_index {
            return Err("GIT_LOCAL_CHANGED_SINCE_PREVIEW".into());
        }
        check_preconditions(
            &repo,
            &ctx,
            &job.action,
            job.branch.as_deref(),
            job.message.as_deref(),
            job.resolved.as_deref(),
            job.pr,
        )?;
        perform(&mut repo, &ctx, &job)?;
        Ok::<_, String>(format!("{} 完成", job.action))
    })
    .await
    .map_err(|_| "GIT_LOCAL_WORKER_FAILED".to_string())?
}

fn git_blob_text(repo: &Repository, oid: git2::Oid) -> Result<String, String> {
    if oid.is_zero() {
        return Ok(String::new());
    }
    let b = repo
        .find_blob(oid)
        .map_err(|_| "GIT_LOCAL_CONFLICT_BLOB_MISSING")?;
    if b.size() > MAX_TEXT || b.content().contains(&0) {
        return Err("GIT_LOCAL_CONFLICT_BINARY_OR_TOO_LARGE".into());
    }
    String::from_utf8(b.content().to_vec()).map_err(|_| "GIT_LOCAL_CONFLICT_NOT_UTF8".into())
}
#[tauri::command]
pub async fn git_local_conflict(
    app: AppHandle,
    webview: Webview,
    path: String,
) -> Result<LocalConflict, String> {
    check_invocation_source(&webview, "git_local_conflict", None, &app)?;
    let roots = workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let (repo, ctx) = discover_allowed(&path, &roots)?;
        let rel = selected_file(&ctx)?;
        let index = repo.index().map_err(|_| "GIT_LOCAL_INDEX_FAILED")?;
        if !index.has_conflicts() {
            return Err("GIT_LOCAL_NO_FILE_CONFLICT".into());
        }
        let ids = [1, 2, 3].map(|s| {
            index
                .get_path(rel, s)
                .map(|e| e.id)
                .unwrap_or_else(git2::Oid::zero)
        });
        if ids.iter().all(|o| o.is_zero()) {
            return Err("GIT_LOCAL_NO_FILE_CONFLICT".into());
        }
        Ok::<_, String>(LocalConflict {
            path: ctx.relative_path,
            ancestor: git_blob_text(&repo, ids[0])?,
            ours: git_blob_text(&repo, ids[1])?,
            theirs: git_blob_text(&repo, ids[2])?,
        })
    })
    .await
    .map_err(|_| "GIT_LOCAL_WORKER_FAILED".to_string())?
}

#[tauri::command]
pub async fn git_local_github(
    app: AppHandle,
    webview: Webview,
    path: String,
) -> Result<serde_json::Value, String> {
    check_invocation_source(&webview, "git_local_github", None, &app)?;
    let roots = workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move||{
        let (repo,_)=discover_allowed(&path,&roots)?;
        let slug=github_slug(&repo)?;
        // gh must already be installed and authorized by the user. No OAuth
        // tokens enter BrowserOS, no token or stderr content is ever returned.
        let prs=gh_output(&["pr","list","--repo",&slug,"--state","open","--limit","30","--json",
            "number,title,state,headRefName,baseRefName,reviewDecision,isDraft,mergeStateStatus,statusCheckRollup,url"])?;
        let runs=gh_output(&["run","list","--repo",&slug,"--limit","20","--json",
            "databaseId,displayTitle,workflowName,status,conclusion,url,headSha"])?;
        Ok::<_,String>(serde_json::json!({"repository":slug,"prs":prs,"runs":runs}))
    }).await.map_err(|_|"GIT_LOCAL_WORKER_FAILED".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_remote_protocols_and_branch_injection() {
        assert!(!safe_branch("../escape"));
        assert!(!safe_branch("-f"));
        assert!(!safe_branch("bad:ref"));
        assert!(safe_branch("feature/good-branch"));
        assert!(!allowed_action("reset"));
        assert!(!allowed_action("force_push"));
    }
    #[test]
    fn scope_does_not_leak_sibling_prefix() {
        let ctx = LocalGitContext {
            root: "/tmp/project".into(),
            relative_path: "src".into(),
            is_dir: true,
            branch: "main".into(),
        };
        assert!(selected_scope(&ctx, Path::new("src/file.rs")));
        assert!(!selected_scope(&ctx, Path::new("src_bak/file.rs")));
    }
}
