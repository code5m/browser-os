use std::fs;
use std::path::Path;

use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository, Signature,
};
use tauri::{AppHandle, Manager};

use crate::domain::*;
use crate::keyring_store::KeyringStore;
use crate::workspace;

/// 把待确认任务真正推送到 git / gitee。
/// 凭据仅在此时从系统密钥库读取，用于 HTTPS 鉴权（token 作为密码）。
pub fn push_artifacts(app: &AppHandle, job: &SyncJob) -> Result<(), String> {
    let repos = workspace::load_repos(app);
    let repo = repos
        .iter()
        .find(|r| r.id == job.repo_id)
        .ok_or("仓库未配置".to_string())?;
    let token = KeyringStore::get_token(&repo.id)?;

    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("mvp-browser-os")
        .join("repos")
        .join(&repo.id);
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;

    let repository = if base.join(".git").exists() {
        Repository::open(&base).map_err(|e| e.to_string())?
    } else {
        clone_repo(&repo.remote_url, &base, &repo.username, &token)?
    };

    pull(&repository, &repo.branch, &repo.username, &token)?;

    let arts: Vec<Artifact> = workspace::load_artifacts(app)
        .into_iter()
        .filter(|a| job.artifact_ids.contains(&a.id))
        .collect();
    if arts.is_empty() {
        return Err("没有可同步的成果".into());
    }

    for art in &arts {
        let dir = base.join("artifacts");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        // 正文 Markdown（带溯源），便于阅读/检索
        let md_path = dir.join(format!("{}.md", art.id));
        fs::write(&md_path, render_markdown(art)).map_err(|e| e.to_string())?;
        // 富文本保真 HTML（保留选区排版/图片/链接），带溯源头信息
        if !art.html.trim().is_empty() {
            let html_path = dir.join(format!("{}.html", art.id));
            fs::write(&html_path, render_html(art)).map_err(|e| e.to_string())?;
        }
    }

    commit(
        &repository,
        &format!("sync {} artifact(s) via 极智简单", arts.len()),
    )?;
    push(&repository, &repo.branch, &repo.username, &token)?;
    Ok(())
}

/// 凭据回调：复用同一 (username, token)。token 作为密码用于 HTTPS。
fn cred_cb(username: String, token: String) -> RemoteCallbacks<'static> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(move |_url, _user, _allowed| Cred::userpass_plaintext(&username, &token));
    cb
}

fn clone_repo(url: &str, dest: &Path, username: &str, token: &str) -> Result<Repository, String> {
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cred_cb(username.to_string(), token.to_string()));
    RepoBuilder::new()
        .fetch_options(fo)
        .clone(url, dest)
        .map_err(|e| format!("clone 失败: {e}"))
}

fn pull(repo: &Repository, branch: &str, username: &str, token: &str) -> Result<(), String> {
    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cred_cb(username.to_string(), token.to_string()));
    // 远端可能是空仓库（首次推送），fetch 失败不视为致命错误
    if let Err(e) = remote.fetch(&[branch], Some(&mut fo), None) {
        // 空仓库 / 分支不存在：无可合并，直接返回让后续 commit+push 建立分支
        if e.code() == git2::ErrorCode::NotFound {
            return Ok(());
        }
        return Err(format!("fetch 失败: {e}"));
    }

    // FETCH_HEAD 可能不存在（远端空仓库）
    let fetch_head = match repo.find_reference("FETCH_HEAD") {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    let fetch_commit = repo
        .reference_to_annotated_commit(&fetch_head)
        .map_err(|e| e.to_string())?;
    let (analysis, _) = repo
        .merge_analysis(&[&fetch_commit])
        .map_err(|e| e.to_string())?;

    if analysis.is_up_to_date() {
        return Ok(());
    }

    if analysis.is_fast_forward() {
        // 快进：本地无独立提交，直接把分支指针前移到远端
        return fast_forward(repo, branch, &fetch_commit);
    }

    if analysis.is_normal() {
        // 三方合并：本地与远端各有提交，尝试真正合并
        return three_way_merge(repo, &fetch_commit, username);
    }

    Err("无法确定合并策略（unborn/未知），请人工处理".into())
}

/// 快进合并：直接把本地分支引用指向远端提交并 checkout
fn fast_forward(
    repo: &Repository,
    branch: &str,
    fetch_commit: &git2::AnnotatedCommit,
) -> Result<(), String> {
    let refname = format!("refs/heads/{branch}");
    let target = fetch_commit.id();
    match repo.find_reference(&refname) {
        Ok(mut r) => {
            r.set_target(target, "fast-forward")
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            // 本地分支尚不存在（例如首次），直接创建
            repo.reference(&refname, target, true, "create branch")
                .map_err(|e| e.to_string())?;
        }
    }
    repo.set_head(&refname).map_err(|e| e.to_string())?;
    repo.checkout_head(Some(CheckoutBuilder::default().force()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 三方合并：本地与远端分叉时，做索引级三方合并。
/// - 无冲突：自动生成一个 merge commit（两个父提交），保留双方改动。
/// - 有冲突：中止合并（`merge_cleanup`），返回冲突文件清单，交由用户处理，
///   绝不丢弃任何一方的数据（对齐“成果不可丢失”红线）。
fn three_way_merge(
    repo: &Repository,
    fetch_commit: &git2::AnnotatedCommit,
    username: &str,
) -> Result<(), String> {
    // 执行合并（把远端改动合入工作区与索引）
    repo.merge(&[fetch_commit], None, None)
        .map_err(|e| format!("merge 失败: {e}"))?;

    let mut index = repo.index().map_err(|e| e.to_string())?;
    if index.has_conflicts() {
        // 收集冲突文件
        let mut conflicts = vec![];
        if let Ok(iter) = index.conflicts() {
            for c in iter.flatten() {
                if let Some(entry) = c.our.or(c.their).or(c.ancestor) {
                    if let Ok(p) = std::str::from_utf8(&entry.path) {
                        conflicts.push(p.to_string());
                    }
                }
            }
        }
        // 中止合并，恢复到合并前状态，不破坏本地数据
        repo.cleanup_state().ok();
        let _ = repo.checkout_head(Some(CheckoutBuilder::default().force()));
        return Err(format!(
            "检测到与远端冲突，已中止自动合并以保护数据。冲突文件: {}。请拉取仓库人工合并后重试。",
            conflicts.join(", ")
        ));
    }

    // 无冲突：生成 merge commit（两个父：本地 HEAD + 远端）
    let sig = Signature::now(username_or_default(username), "mvp@jizhijiandan.local")
        .map_err(|e| e.to_string())?;
    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let local_commit = repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|e| e.to_string())?;
    let remote_commit = repo
        .find_commit(fetch_commit.id())
        .map_err(|e| e.to_string())?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "merge remote into local via 极智简单（三方合并）",
        &tree,
        &[&local_commit, &remote_commit],
    )
    .map_err(|e| e.to_string())?;

    // 清理合并状态并 checkout 结果
    repo.cleanup_state().ok();
    repo.checkout_head(Some(CheckoutBuilder::default().force()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn username_or_default(u: &str) -> &str {
    if u.trim().is_empty() {
        "极智简单"
    } else {
        u
    }
}

fn commit(repo: &Repository, message: &str) -> Result<(), String> {
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index
        .add_all(&["*"], git2::IndexAddOption::DEFAULT, None)
        .map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let sig = Signature::now("极智简单", "mvp@jizhijiandan.local").map_err(|e| e.to_string())?;
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn push(repo: &Repository, branch: &str, username: &str, token: &str) -> Result<(), String> {
    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
    let mut po = PushOptions::new();
    po.remote_callbacks(cred_cb(username.to_string(), token.to_string()));
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    remote
        .push(&[refspec.as_str()], Some(&mut po))
        .map_err(|e| format!("push 失败: {e}（检查 token 权限/分支保护）"))
}

/// 把成果渲染成带溯源的 Markdown
fn render_markdown(a: &Artifact) -> String {
    let mut s = String::new();
    s.push_str(&format!("# {}\n\n", a.title));
    s.push_str(&format!("- 来源: {}\n", a.source_url));
    s.push_str(&format!("- 溯源哈希: {}\n", a.hash));
    s.push_str(&format!("- 采集时间: {}\n\n", a.created_at));
    s.push_str("---\n\n");
    s.push_str(&a.text);
    s.push_str("\n\n<!-- provenance: 此文件由极智简单自动同步，哈希用于溯源 -->\n");
    s
}

/// 富文本保真：把采集的选区 HTML 包成独立可打开的文档，带溯源头信息。
fn render_html(a: &Artifact) -> String {
    format!(
        "<!doctype html>\n<html lang=\"zh\">\n<head>\n<meta charset=\"utf-8\">\n\
<title>{title}</title>\n\
<style>body{{font-family:system-ui,\"PingFang SC\",sans-serif;max-width:820px;margin:32px auto;padding:0 16px;line-height:1.7;}}\
.jzjd-src{{font-size:12px;color:#888;border-bottom:1px solid #eee;padding-bottom:12px;margin-bottom:20px;}}\
.jzjd-src a{{color:#2b6;}} img{{max-width:100%;height:auto;}}</style>\n</head>\n<body>\n\
<div class=\"jzjd-src\">来源：<a href=\"{url}\">{url}</a> · 溯源哈希：{hash} · 采集时间：{at}</div>\n\
{body}\n\
<!-- provenance: 由极智简单自动同步，此文件为选区富文本保真副本 -->\n</body>\n</html>\n",
        title = html_escape(&a.title),
        url = html_escape(&a.source_url),
        hash = a.hash,
        at = a.created_at,
        body = a.html,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
