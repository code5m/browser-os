//! Read-only history for a user-selected local file/directory.
//!
//! This NEVER uses the BrowserOS-managed clone, starts a Git process,
//! reads Git credentials, or changes the index/worktree. The selected path
//! and discovered repository root must both be within allowed workspace roots.
use std::path::{Path, PathBuf};
use git2::{DiffOptions, Repository, Sort};
use serde::Serialize;
use tauri::{AppHandle, Webview};
use crate::security_policy as sp;
use crate::shared::invocation::check_invocation_source;
use crate::workspace;
use super::commands::GitLogEntry;

#[derive(Serialize)]
pub struct LocalGitHistory {
    pub repository_root: String,
    pub relative_path: String,
    pub branch: String,
    pub entries: Vec<GitLogEntry>,
    pub has_more: bool,
    pub scan_limited: bool,
}

fn authorized_repo(path: &str, roots: &[PathBuf]) -> Result<(Repository, PathBuf, PathBuf), String> {
    if path.len() > 4096 {
        return Err("GIT_LOCAL_PATH_TOO_LONG".into());
    }
    let canonical = sp::check_path_within_roots(path, roots)
        .map_err(|_| "GIT_LOCAL_PATH_NOT_ALLOWED".to_string())?;
    let start = if canonical.is_file() {
        canonical.parent().ok_or("GIT_LOCAL_PATH_INVALID")?
    } else {
        canonical.as_path()
    };
    let repo = Repository::discover(start).map_err(|_| "GIT_LOCAL_REPO_NOT_FOUND".to_string())?;
    let workdir = repo.workdir().ok_or("GIT_LOCAL_BARE_REPO_NOT_SUPPORTED")?;
    let root = workdir.canonicalize().map_err(|_| "GIT_LOCAL_ROOT_MISSING")?;
    if !sp::is_within_roots(&root, roots) || !canonical.starts_with(&root) {
        return Err("GIT_LOCAL_REPO_OUTSIDE_ALLOWED_ROOTS".to_string());
    }
    let relative = canonical.strip_prefix(&root)
        .map_err(|_| "GIT_LOCAL_PATH_OUTSIDE_REPO")?.to_path_buf();
    if relative.components().any(|c| c.as_os_str() == ".git") {
        return Err("GIT_LOCAL_INTERNAL_PATH_REFUSED".into());
    }
    Ok((repo, root, relative))
}

fn affects(selected: &Path, changed: &Path) -> bool {
    selected.as_os_str().is_empty()
        || changed == selected
        || changed.starts_with(selected)
}

fn touches_path(repo: &Repository, commit: &git2::Commit<'_>, selected: &Path) -> Result<bool, String> {
    if selected.as_os_str().is_empty() { return Ok(true); }
    let tree = commit.tree().map_err(|_| "GIT_LOCAL_TREE_FAILED")?;
    let prior = if commit.parent_count() > 0 {
        Some(commit.parent(0).and_then(|p| p.tree()).map_err(|_| "GIT_LOCAL_PARENT_FAILED")?)
    } else { None };
    let diff = repo.diff_tree_to_tree(prior.as_ref(), Some(&tree), None)
        .map_err(|_| "GIT_LOCAL_DIFF_FAILED")?;
    Ok(diff.deltas().any(|d| {
        d.old_file().path().is_some_and(|p| affects(selected, p))
            || d.new_file().path().is_some_and(|p| affects(selected, p))
    }))
}

fn read_history(path: &str, roots: &[PathBuf], page: u32) -> Result<LocalGitHistory, String> {
    if page > 20 { return Err("GIT_LOCAL_PAGE_LIMIT".into()); }
    let (repo, root, relative) = authorized_repo(path, roots)?;
    let branch = repo.head().ok().and_then(|h| h.shorthand().map(str::to_owned))
        .unwrap_or_else(|| "HEAD (detached)".to_string());
    let mut entries = Vec::new();
    let mut has_more = false;
    let mut scan_limited = false;
    if !repo.is_empty().unwrap_or(true) {
        let mut walk = repo.revwalk().map_err(|_| "GIT_LOCAL_LOG_FAILED")?;
        walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME).map_err(|_| "GIT_LOCAL_LOG_FAILED")?;
        walk.push_head().map_err(|_| "GIT_LOCAL_LOG_FAILED")?;
        let skip = (page as usize) * 50;
        let mut matching = 0;
        let mut visited = 0;
        // Bound synchronous disk/CPU work in large enterprise repositories.
        for id in walk {
            if visited >= 4000 { scan_limited = true; break; }
            visited += 1;
            let id = id.map_err(|_| "GIT_LOCAL_WALK_FAILED")?;
            let commit = repo.find_commit(id).map_err(|_| "GIT_LOCAL_COMMIT_FAILED")?;
            if !touches_path(&repo, &commit, &relative)? { continue; }
            if matching < skip { matching += 1; continue; }
            if entries.len() >= 50 { has_more = true; break; }
            entries.push(GitLogEntry {
                oid: commit.id().to_string(),
                parents: commit.parent_ids().map(|p| p.to_string()).collect(),
                summary: commit.summary().unwrap_or("").chars().take(1024).collect(),
                author: commit.author().name().unwrap_or("").chars().take(128).collect(),
                time: commit.time().seconds(),
            });
        }
    }
    Ok(LocalGitHistory {
        repository_root: root.to_string_lossy().to_string(),
        relative_path: relative.to_string_lossy().to_string(),
        branch, entries, has_more, scan_limited,
    })
}

fn read_commit_diff(path: &str, roots: &[PathBuf], oid: &str) -> Result<String, String> {
    let (repo, _, relative) = authorized_repo(path, roots)?;
    let id = git2::Oid::from_str(oid).map_err(|_| "GIT_LOCAL_INVALID_OID")?;
    let commit = repo.find_commit(id).map_err(|_| "GIT_LOCAL_COMMIT_NOT_FOUND")?;
    let tree = commit.tree().map_err(|_| "GIT_LOCAL_TREE_FAILED")?;
    let prior = if commit.parent_count() > 0 {
        Some(commit.parent(0).and_then(|p| p.tree()).map_err(|_| "GIT_LOCAL_PARENT_FAILED")?)
    } else { None };
    let mut opts = DiffOptions::new();
    // The native commit diff is further restricted by the real selected path.
    if !relative.as_os_str().is_empty() {
        opts.pathspec(&relative);
        opts.disable_pathspec_match(true);
    }
    let diff = repo.diff_tree_to_tree(prior.as_ref(), Some(&tree), Some(&mut opts))
        .map_err(|_| "GIT_LOCAL_DIFF_FAILED")?;
    let mut output = String::new();
    let mut truncated = false;
    let result = diff.print(git2::DiffFormat::Patch, |_, _, line| {
        let text = String::from_utf8_lossy(line.content());
        if output.len() + text.len() + 2 > 128 * 1024 {
            truncated = true;
            return false;
        }
        if matches!(line.origin(), '+' | '-' | ' ') {
            output.push(line.origin());
        }
        output.push_str(&text);
        true
    });
    if !truncated { result.map_err(|_| "GIT_LOCAL_DIFF_FAILED")?; }
    if truncated { output.push_str("\n[Diff truncated at 128 KiB]\n"); }
    Ok(output)
}

#[tauri::command]
pub async fn git_local_history(
    app: AppHandle, webview: Webview, path: String, page: u32,
) -> Result<LocalGitHistory, String> {
    check_invocation_source(&webview, "git_local_history", None, &app)?;
    let roots = workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move || read_history(&path, &roots, page))
        .await.map_err(|_| "GIT_LOCAL_WORKER_FAILED".to_string())?
}

#[tauri::command]
pub async fn git_local_commit_diff(
    app: AppHandle, webview: Webview, path: String, oid: String,
) -> Result<String, String> {
    check_invocation_source(&webview, "git_local_commit_diff", None, &app)?;
    let roots = workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move || read_commit_diff(&path, &roots, &oid))
        .await.map_err(|_| "GIT_LOCAL_WORKER_FAILED".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_scope_is_component_based_not_string_prefix() {
        assert!(affects(Path::new("src"), Path::new("src/lib.rs")));
        assert!(affects(Path::new("src/main.rs"), Path::new("src/main.rs")));
        assert!(!affects(Path::new("src"), Path::new("src2/file.rs")));
        assert!(!affects(Path::new("src/main.rs"), Path::new("src/main2.rs")));
    }
    #[test]
    fn refusing_paths_outside_authorized_roots() {
        let roots = vec![PathBuf::from("/unlikely-browseros-test-root-not-present")];
        assert!(authorized_repo("/etc/passwd", &roots).is_err());
    }
}
