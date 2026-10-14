//! Explicit, read-only inspection of an existing local Git repository.
//! Unlike managed-sync repositories these functions never clone, fetch or write,
//! and never accept raw paths outside the filesystem roots granted to BrowserOS.
use git2::{DiffOptions, Repository, Sort};
use serde::Serialize;
use std::path::{Path, PathBuf};
use crate::security_policy::check_path_within_roots;

pub const MAX_LOCAL_PATCH: usize = 256 * 1024;
const MAX_LOG_SCAN: usize = 4000;
const MAX_LOG_PAGE: usize = 75;

#[derive(Serialize, Clone)]
pub struct LocalGitContext {
    pub root: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub branch: String,
}
#[derive(Serialize)]
pub struct LocalLogEntry {
    pub oid: String,
    pub parents: Vec<String>,
    pub summary: String,
    pub author: String,
    pub time: i64,
}
#[derive(Serialize)]
pub struct LocalLogPage {
    pub commits: Vec<LocalLogEntry>,
    pub has_more: bool,
    pub scan_limited: bool,
}
#[derive(Serialize)]
pub struct LocalPatch {
    pub patch: String,
    pub truncated: bool,
}

pub fn discover_allowed(
    path: &str, roots: &[PathBuf],
) -> Result<(Repository, LocalGitContext), String> {
    if path.is_empty() || path.len() > 4096 { return Err("GIT_LOCAL_PATH_INVALID".into()); }
    let canonical = check_path_within_roots(path, roots).map_err(|_| "GIT_LOCAL_PATH_DENIED".to_string())?;
    if canonical.components().any(|part| part.as_os_str() == ".git") {
        return Err("GIT_LOCAL_INTERNAL_PATH_DENIED".into());
    }
    let is_dir = canonical.is_dir();
    let directory = if is_dir { canonical.clone() }
        else { canonical.parent().ok_or("GIT_LOCAL_PATH_INVALID")?.to_path_buf() };
    let repo = Repository::discover(&directory).map_err(|_| "GIT_LOCAL_NO_REPOSITORY".to_string())?;
    let root = repo.workdir().ok_or("GIT_LOCAL_BARE_NOT_SUPPORTED")?
        .canonicalize().map_err(|_| "GIT_LOCAL_ROOT_INVALID")?;
    check_path_within_roots(&root.to_string_lossy(), roots)
        .map_err(|_| "GIT_LOCAL_ROOT_DENIED".to_string())?;
    let relative = canonical.strip_prefix(&root).map_err(|_| "GIT_LOCAL_PATH_OUTSIDE_WORKTREE")?;
    if relative.components().any(|part| part.as_os_str() == ".git") {
        return Err("GIT_LOCAL_INTERNAL_PATH_DENIED".into());
    }
    let branch = repo.head().ok().and_then(|h| h.shorthand().map(str::to_owned))
        .unwrap_or_else(|| "(detached / unborn HEAD)".to_string());
    let context = LocalGitContext {
        root: root.to_string_lossy().into_owned(),
        relative_path: relative.to_string_lossy().into_owned(),
        is_dir, branch,
    };
    Ok((repo, context))
}
fn in_scope(file: Option<&Path>, relative: &str, is_dir: bool) -> bool {
    if relative.is_empty() { return true; }
    match file {
        Some(path) if is_dir => path.starts_with(Path::new(relative)),
        Some(path) => path == Path::new(relative),
        None => false,
    }
}
fn commit_touches(repo: &Repository, commit: &git2::Commit<'_>, ctx: &LocalGitContext) -> Result<bool, String> {
    if ctx.relative_path.is_empty() { return Ok(true); }
    let current = commit.tree().map_err(|_| "GIT_LOCAL_LOG_FAILED")?;
    let previous = if commit.parent_count() > 0 {
        Some(commit.parent(0).and_then(|p| p.tree()).map_err(|_| "GIT_LOCAL_LOG_FAILED")?)
    } else { None };
    let mut opts=DiffOptions::new();
    opts.include_typechange(true);
    let diff=repo.diff_tree_to_tree(previous.as_ref(), Some(&current), Some(&mut opts))
        .map_err(|_| "GIT_LOCAL_LOG_FAILED")?;
    Ok(diff.deltas().any(|delta| {
        in_scope(delta.old_file().path(), &ctx.relative_path, ctx.is_dir)
        || in_scope(delta.new_file().path(), &ctx.relative_path, ctx.is_dir)
    }))
}
pub fn log_page(repo: &Repository, ctx: &LocalGitContext, offset: u32, limit: u32) -> Result<LocalLogPage,String> {
    if offset>1000 || limit==0 || limit as usize>MAX_LOG_PAGE { return Err("GIT_LOCAL_PAGE_INVALID".into()); }
    if repo.is_empty().unwrap_or(false) { return Ok(LocalLogPage{commits:vec![],has_more:false,scan_limited:false}); }
    let mut walk=repo.revwalk().map_err(|_| "GIT_LOCAL_LOG_FAILED")?;
    walk.set_sorting(Sort::TOPOLOGICAL|Sort::TIME).map_err(|_| "GIT_LOCAL_LOG_FAILED")?;
    walk.push_head().map_err(|_| "GIT_LOCAL_LOG_FAILED")?;
    let mut commits=Vec::with_capacity(limit as usize);
    let mut matched=0u32;
    let mut scanned=0usize;
    let mut has_more=false;
    let mut scan_limited=false;
    for oid in walk {
        if scanned >= MAX_LOG_SCAN { scan_limited=true; has_more=true; break; }
        scanned+=1;
        let commit=repo.find_commit(oid.map_err(|_| "GIT_LOCAL_LOG_FAILED")?)
            .map_err(|_| "GIT_LOCAL_LOG_FAILED")?;
        if !commit_touches(repo,&commit,ctx)? { continue; }
        if matched<offset { matched+=1; continue; }
        if commits.len() >= limit as usize { has_more=true; break; }
        commits.push(LocalLogEntry {
            oid:commit.id().to_string(),
            parents:commit.parent_ids().map(|id|id.to_string()).collect(),
            summary:commit.summary().unwrap_or("").chars().take(500).collect(),
            author:commit.author().name().unwrap_or("").chars().take(100).collect(),
            time:commit.time().seconds(),
        });
    }
    Ok(LocalLogPage{commits,has_more,scan_limited})
}
fn collect_patch(repo:&Repository, mut opts:DiffOptions<'_>,
    before:Option<&git2::Tree<'_>>,after:Option<&git2::Tree<'_>>,
    ctx:&LocalGitContext
) -> Result<LocalPatch,String> {
    let diff=repo.diff_tree_to_tree(before,after,Some(&mut opts))
        .map_err(|_| "GIT_LOCAL_DIFF_FAILED")?;
    let mut patch=String::new();
    let mut truncated=false;
    let outcome=diff.print(git2::DiffFormat::Patch,|delta,_,line|{
        if !in_scope(delta.old_file().path(),&ctx.relative_path,ctx.is_dir)
           && !in_scope(delta.new_file().path(),&ctx.relative_path,ctx.is_dir) { return true; }
        let content=String::from_utf8_lossy(line.content());
        let extra=if matches!(line.origin(),'+'|'-'|' ') { 1 } else { 0 };
        if patch.len()+content.len()+extra>MAX_LOCAL_PATCH { truncated=true;return false; }
        if extra>0 { patch.push(line.origin()); }
        patch.push_str(&content);
        true
    });
    if !truncated { outcome.map_err(|_| "GIT_LOCAL_DIFF_FAILED")?; }
    Ok(LocalPatch{patch,truncated})
}
pub fn commit_diff(repo:&Repository,ctx:&LocalGitContext,oid:&str)->Result<LocalPatch,String>{
    let oid=git2::Oid::from_str(oid).map_err(|_| "GIT_LOCAL_INVALID_OID")?;
    let commit=repo.find_commit(oid).map_err(|_| "GIT_LOCAL_COMMIT_MISSING")?;
    let tree=commit.tree().map_err(|_| "GIT_LOCAL_DIFF_FAILED")?;
    let before=if commit.parent_count()>0 {
        Some(commit.parent(0).and_then(|p|p.tree()).map_err(|_| "GIT_LOCAL_DIFF_FAILED")?)
    } else { None };
    let mut opts=DiffOptions::new();
    // Never interpret an untrusted file name as a Git wildcard pathspec.
    // Match canonical paths component-by-component in the callback instead.
    collect_patch(repo,opts,before.as_ref(),Some(&tree),ctx)
}
pub fn working_diff(repo:&Repository,ctx:&LocalGitContext)->Result<LocalPatch,String>{
    let tree=repo.head().ok().and_then(|h|h.peel_to_tree().ok());
    let mut opts=DiffOptions::new();
    // Only the original worktree is inspected. No index writes or fetch.
    let diff=repo.diff_tree_to_workdir_with_index(tree.as_ref(),Some(&mut opts))
        .map_err(|_| "GIT_LOCAL_DIFF_FAILED")?;
    let mut patch=String::new();
    let mut truncated=false;
    let outcome=diff.print(git2::DiffFormat::Patch,|delta,_,line|{
        if !in_scope(delta.old_file().path(),&ctx.relative_path,ctx.is_dir)
           && !in_scope(delta.new_file().path(),&ctx.relative_path,ctx.is_dir) { return true; }
        let content=String::from_utf8_lossy(line.content());
        let extra=if matches!(line.origin(),'+'|'-'|' ') {1} else {0};
        if patch.len()+content.len()+extra>MAX_LOCAL_PATCH {truncated=true;return false;}
        if extra>0 {patch.push(line.origin());}
        patch.push_str(&content);true
    });
    if !truncated {outcome.map_err(|_| "GIT_LOCAL_DIFF_FAILED")?;}
    Ok(LocalPatch{patch,truncated})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scope_matches_exact_file_and_subdirectories_without_prefix_collision(){
        assert!(in_scope(Some(Path::new("src/lib.rs")),"src",true));
        assert!(!in_scope(Some(Path::new("src_backup/lib.rs")),"src",true));
        assert!(in_scope(Some(Path::new("src/lib.rs")),"src/lib.rs",false));
        assert!(!in_scope(Some(Path::new("src/lib.rs.old")),"src/lib.rs",false));
    }
    #[test]
    fn local_git_log_filters_real_commits(){
        let root=std::env::temp_dir().join(format!("browseros-git-local-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let repo=Repository::init(&root).unwrap();
        let sig=git2::Signature::now("test","test@example.invalid").unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.txt"),"hello").unwrap();
        let mut index=repo.index().unwrap();index.add_path(Path::new("src/a.txt")).unwrap();
        let tree_id=index.write_tree().unwrap();let tree=repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"),&sig,&sig,"initial",&tree,&[]).unwrap();
        let ctx=LocalGitContext {root:root.display().to_string(),relative_path:"src".into(),is_dir:true,branch:"main".into()};
        let page=log_page(&repo,&ctx,0,20).unwrap();
        assert_eq!(page.commits.len(),1);
        assert_eq!(page.commits[0].summary,"initial");
        let patch=commit_diff(&repo,&ctx,&page.commits[0].oid).unwrap();
        assert!(patch.patch.contains("hello"));
        drop(tree);drop(index);drop(repo);let _=std::fs::remove_dir_all(&root);
    }
}
