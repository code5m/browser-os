//! Bounded blame and rename-follow over the originally selected Git worktree.
//! No cloning, network access, pathspec globbing, or secret retrieval.
use super::local_readonly::{discover_allowed, LocalGitContext};
use crate::shared::invocation::check_invocation_source;
use crate::workspace;
use git2::{Delta, DiffFindOptions, Repository, Sort};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Webview};

const MAX_BLAME_LINES: usize=4000;
const MAX_FOLLOW_COMMITS: usize=4000;

#[derive(Serialize)]
pub struct LocalBlameLine {
    pub start: u32,
    pub lines: u32,
    pub commit: String,
    pub author: String,
}
#[derive(Serialize)]
pub struct LocalBlameResult {
    pub path: String,
    pub hunks: Vec<LocalBlameLine>,
    pub truncated: bool,
}
#[derive(Serialize)]
pub struct LocalFollowEntry {
    pub oid: String,
    pub summary: String,
    pub author: String,
    pub time: i64,
    pub file_at_commit: String,
    pub renamed_from: Option<String>,
}
#[derive(Serialize)]
pub struct LocalFollowResult {
    pub entries: Vec<LocalFollowEntry>,
    pub scan_limited: bool,
}

fn selected_path(ctx:&LocalGitContext)->Result<&Path,String>{
    if ctx.is_dir || ctx.relative_path.is_empty(){return Err("GIT_LOCAL_FILE_REQUIRED".into())}
    Ok(Path::new(&ctx.relative_path))
}
fn blame(repo:&Repository,ctx:&LocalGitContext)->Result<LocalBlameResult,String>{
    let path=selected_path(ctx)?;
    let file=Path::new(&ctx.root).join(path);
    if file.metadata().map_err(|_|"GIT_LOCAL_BLAME_FILE_MISSING")?.len()>1024*1024 {
        return Err("GIT_LOCAL_BLAME_FILE_TOO_LARGE".into())
    }
    let result=repo.blame_file(path,None).map_err(|_|"GIT_LOCAL_BLAME_FAILED")?;
    let mut hunks=Vec::new();let mut truncated=false;let mut seen=0usize;
    for h in result.iter(){
        let lines=h.lines_in_hunk();
        if seen+lines>MAX_BLAME_LINES {truncated=true;break}
        hunks.push(LocalBlameLine{
            start:h.final_start_line() as u32,lines:lines as u32,
            commit:h.final_commit_id().to_string(),
            author:h.final_signature().name().unwrap_or("").chars().take(80).collect(),
        });
        seen+=lines;
    }
    Ok(LocalBlameResult{path:ctx.relative_path.clone(),hunks,truncated})
}
fn follow(repo:&Repository,ctx:&LocalGitContext)->Result<LocalFollowResult,String>{
    let mut tracked=selected_path(ctx)?.to_path_buf();
    let mut walk=repo.revwalk().map_err(|_|"GIT_LOCAL_FOLLOW_FAILED")?;
    walk.set_sorting(Sort::TIME|Sort::TOPOLOGICAL).map_err(|_|"GIT_LOCAL_FOLLOW_FAILED")?;
    walk.push_head().map_err(|_|"GIT_LOCAL_FOLLOW_FAILED")?;
    let mut entries=Vec::new();let mut scan_limited=false;
    for (i,id) in walk.enumerate(){
        if i>=MAX_FOLLOW_COMMITS {scan_limited=true;break}
        let commit=repo.find_commit(id.map_err(|_|"GIT_LOCAL_FOLLOW_FAILED")?).map_err(|_|"GIT_LOCAL_FOLLOW_FAILED")?;
        let tree=commit.tree().map_err(|_|"GIT_LOCAL_FOLLOW_FAILED")?;
        let prior=if commit.parent_count()>0 {
            Some(commit.parent(0).and_then(|p|p.tree()).map_err(|_|"GIT_LOCAL_FOLLOW_FAILED")?)
        }else{None};
        let mut diff=repo.diff_tree_to_tree(prior.as_ref(),Some(&tree),None).map_err(|_|"GIT_LOCAL_FOLLOW_FAILED")?;
        // Detect rename deltas from the actual tree pair, never from a string-prefix heuristic.
        let mut find=DiffFindOptions::new();
        find.renames(true);
        diff.find_similar(Some(&mut find)).map_err(|_|"GIT_LOCAL_FOLLOW_FAILED")?;
        let mut touched=false;let mut rename=None;
        for delta in diff.deltas(){
            let old=delta.old_file().path(); let new=delta.new_file().path();
            if new==Some(tracked.as_path()) || old==Some(tracked.as_path()){
                touched=true;
                if delta.status()==Delta::Renamed && new==Some(tracked.as_path()) {
                    rename=old.map(|p|p.to_path_buf());
                }
            }
        }
        if touched {
            entries.push(LocalFollowEntry{
                oid:commit.id().to_string(),
                summary:commit.summary().unwrap_or("").chars().take(180).collect(),
                author:commit.author().name().unwrap_or("").chars().take(80).collect(),
                time:commit.time().seconds(),
                file_at_commit:tracked.to_string_lossy().into_owned(),
                renamed_from:rename.as_ref().map(|p|p.to_string_lossy().into_owned()),
            });
            if entries.len()>=200 {scan_limited=true;break}
        }
        if let Some(older)=rename { tracked=older; }
    }
    Ok(LocalFollowResult{entries,scan_limited})
}

#[tauri::command]
pub async fn git_local_blame(app:AppHandle,webview:Webview,path:String)->Result<LocalBlameResult,String>{
    check_invocation_source(&webview,"git_local_blame",None,&app)?;
    let roots=workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move||{
        let (repo,ctx)=discover_allowed(&path,&roots)?;
        blame(&repo,&ctx)
    }).await.map_err(|_|"GIT_LOCAL_WORKER_FAILED".to_string())?
}
#[tauri::command]
pub async fn git_local_follow(app:AppHandle,webview:Webview,path:String)->Result<LocalFollowResult,String>{
    check_invocation_source(&webview,"git_local_follow",None,&app)?;
    let roots=workspace::allowed_roots(&app);
    tauri::async_runtime::spawn_blocking(move||{
        let (repo,ctx)=discover_allowed(&path,&roots)?;
        follow(&repo,&ctx)
    }).await.map_err(|_|"GIT_LOCAL_WORKER_FAILED".to_string())?
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn refuse_directory_for_line_attribution(){
        let ctx=LocalGitContext{root:"/tmp/safe".into(),relative_path:"src".into(),is_dir:true,branch:"master".into()};
        assert!(selected_path(&ctx).is_err());
    }
}
