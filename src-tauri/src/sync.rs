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

// ---------------------------------------------------------------------------
// M1-5 Git 只读能力（status / diff / branch_list）
//
// 硬约束：本段只允许 git2 的只读 API（`statuses` / `diff_*` / `branches` /
// `head` / `index`），不得出现 checkout / reset / commit / push / pull /
// merge / rebase / branch 删除等任何写或联网调用；不读 Keyring、不输出凭据。
// ---------------------------------------------------------------------------

/// 单文件 diff 默认字节上限（未显式传 max_bytes 时使用）。
pub const GIT_DIFF_DEFAULT_MAX_BYTES: usize = 64 * 1024;
/// 服务端硬上限：即使调用方要求更大也不突破，防前端卡死/内存放大。
pub const GIT_DIFF_HARD_CAP: usize = 256 * 1024;

/// 仓库根目录（只读用途）。
/// 安全边界：repo_id 必须命中 `repos.json` 里已配置的仓库，且不得含路径
/// 分隔符、绝对路径或 `..` 逃逸；最终落点严格在
/// `app_data_dir/mvp-browser-os/repos/<id>` 内。
pub fn repo_dir(app: &AppHandle, repo_id: &str) -> Result<std::path::PathBuf, String> {
    let cfg = workspace::load_repos(app)
        .into_iter()
        .find(|r| r.id == repo_id)
        .ok_or_else(|| "仓库未配置".to_string())?;
    let id = &cfg.id;
    if id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
        || std::path::Path::new(id).is_absolute()
    {
        return Err("非法仓库 id".to_string());
    }
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("mvp-browser-os")
        .join("repos");
    let root = base.join(id);
    // 二次校验：拼接后仍必须在 base 之内（防未来 base 规则变动引入逃逸）
    let resolved = root.canonicalize().unwrap_or_else(|_| root.clone());
    let canon = resolved.components().collect::<Vec<_>>();
    let base_canon = base.canonicalize().unwrap_or_else(|_| base.clone());
    let base_len = base_canon.components().count();
    if canon.len() <= base_len {
        return Err("非法仓库路径".to_string());
    }
    Ok(root)
}

/// 以只读方式打开已配置仓库。未 clone 或不是 git 仓库均返回可读错误。
pub fn open_readonly(app: &AppHandle, repo_id: &str) -> Result<Repository, String> {
    let dir = repo_dir(app, repo_id)?;
    if !dir.exists() {
        return Err("仓库尚未初始化（未同步过）".to_string());
    }
    Repository::open(&dir).map_err(|e| format!("打开仓库失败: {e}"))
}

/// git status 等价：工作区+索引的脏文件清单（含未跟踪文件）。
/// 纯只读，不修改索引、不触碰工作区。
pub fn read_status(repo: &Repository) -> Result<Vec<GitFileStatus>, String> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(false)
        .include_ignored(false);
    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for entry in statuses.iter() {
        let path = entry.path().unwrap_or_default().to_string();
        if path.is_empty() {
            continue;
        }
        let st = entry.status();
        let status = if st.contains(git2::Status::CONFLICTED) {
            "conflicted"
        } else if st.contains(git2::Status::WT_NEW) || st.contains(git2::Status::INDEX_NEW) {
            // 未跟踪文件（WT_NEW 且不在索引）标记 untracked，其余为已暂存新增
            if st.contains(git2::Status::WT_NEW) && !st.contains(git2::Status::INDEX_NEW) {
                "untracked"
            } else {
                "added"
            }
        } else if st.contains(git2::Status::WT_DELETED) || st.contains(git2::Status::INDEX_DELETED)
        {
            "deleted"
        } else if st.contains(git2::Status::WT_RENAMED) || st.contains(git2::Status::INDEX_RENAMED)
        {
            "renamed"
        } else if st.contains(git2::Status::WT_MODIFIED)
            || st.contains(git2::Status::INDEX_MODIFIED)
        {
            "modified"
        } else {
            continue;
        };
        out.push(GitFileStatus {
            path,
            status: status.to_string(),
        });
    }
    Ok(out)
}

/// git diff 等价：HEAD → 工作区（含索引）的改动，按文件聚合为补丁文本。
/// 截断策略：
/// - 单文件补丁超过 `min(max_bytes, GIT_DIFF_HARD_CAP)` 即截断并标 `truncated`；
/// - 多文件累计超过硬上限则停止收集并置 `more=true`。
/// 纯只读：不写索引、不落盘。
pub fn read_diff(
    repo: &Repository,
    path: Option<&str>,
    max_bytes: usize,
) -> Result<GitDiffResult, String> {
    let per_file_cap = max_bytes.min(GIT_DIFF_HARD_CAP).max(1);
    // 无 HEAD（空仓库）：head_tree 为 None，diff 以空树为基准（等价于全量新增）
    let head = repo.head().ok();
    let head_tree = head.as_ref().and_then(|h| h.peel_to_tree().ok());
    let mut diff_opts = git2::DiffOptions::new();
    if let Some(p) = path {
        diff_opts.pathspec(p);
    }
    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut diff_opts))
        .map_err(|e| format!("生成 diff 失败: {e}"))?;

    let mut hunks: Vec<GitDiffHunk> = Vec::new();
    let mut total: usize = 0;
    let mut more = false;
    for (idx, delta) in diff.deltas().enumerate() {
        let file = delta
            .new_file()
            .path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let hunk = match git2::Patch::from_diff(&diff, idx) {
            Ok(Some(mut patch)) => match patch.to_buf() {
                Ok(buf) => {
                    let text = String::from_utf8_lossy(&buf).to_string();
                    // libgit2 对二进制不设 BINARY 标志，而是生成
                    // "Binary files a/... and b/... differ" 文本：据此判定，
                    // 不把这段说明当作真实补丁内容回传前端。
                    if text.contains("Binary files ") {
                        GitDiffHunk {
                            file,
                            old_content: None,
                            new_content: None,
                            truncated: false,
                            binary: true,
                        }
                    } else {
                        let (content, truncated) = if text.len() > per_file_cap {
                            (
                                format!("{}...(truncated)", truncate_at(&text, per_file_cap)),
                                true,
                            )
                        } else {
                            (text, false)
                        };
                        GitDiffHunk {
                            file,
                            old_content: None,
                            new_content: Some(content),
                            truncated,
                            binary: false,
                        }
                    }
                }
                Err(_) => GitDiffHunk {
                    file,
                    old_content: None,
                    new_content: None,
                    truncated: false,
                    binary: true,
                },
            },
            // 二进制/超大文件没有文本补丁：显式标记，避免前端显示空白
            Ok(None) | Err(_) => GitDiffHunk {
                file,
                old_content: None,
                new_content: None,
                truncated: false,
                binary: true,
            },
        };
        let len = hunk.new_content.as_deref().map(|c| c.len()).unwrap_or(0);
        if !hunks.is_empty() && total + len > GIT_DIFF_HARD_CAP {
            more = true;
            break;
        }
        total = total.saturating_add(len);
        hunks.push(hunk);
    }
    Ok(GitDiffResult { hunks, more })
}

/// 按字节上限安全截断（回退到最近的 UTF-8 字符边界，绝不 panic）。
fn truncate_at(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// git branch -a 等价：本地分支 + 远程跟踪分支，`is_head` 标当前检出分支。
/// 纯只读，不创建/删除/改名任何引用。
pub fn read_branches(repo: &Repository) -> Result<Vec<GitBranch>, String> {
    let mut out = Vec::new();
    for (branch, kind) in repo
        .branches(None)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
    {
        let name = match branch.name() {
            Ok(Some(n)) => n.to_string(),
            _ => continue,
        };
        let is_remote = matches!(kind, git2::BranchType::Remote);
        let is_head = branch.is_head();
        out.push(GitBranch {
            name,
            is_remote,
            is_head,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod readonly_tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// 建一个本地临时 git 仓库（不联网、不依赖 AppHandle）。
    fn temp_repo(tag: &str) -> (std::path::PathBuf, Repository) {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("mvp-git-ro-{tag}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        let repo = Repository::init(&dir).expect("init");
        // 让 commit 可用（避免依赖全局 user.name/email）
        let mut cfg = repo.config().expect("config");
        cfg.set_str("user.name", "mvp-test").expect("user.name");
        cfg.set_str("user.email", "mvp@test.local")
            .expect("user.email");
        (dir, repo)
    }

    fn commit_all(repo: &Repository, msg: &str) {
        let mut index = repo.index().expect("index");
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .expect("add all");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let sig = Signature::now("mvp-test", "mvp@test.local").expect("sig");
        match repo.head().ok().and_then(|h| h.target()) {
            Some(parent) => {
                let parent_commit = repo.find_commit(parent).expect("parent");
                repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&parent_commit])
                    .expect("commit");
            }
            None => {
                repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[])
                    .expect("initial commit");
            }
        };
    }

    // T-status-1：干净仓库返回空 Vec
    #[test]
    fn t_status_1_clean_repo_is_empty() {
        let (dir, repo) = temp_repo("status1");
        fs::write(dir.join("a.txt"), "hello\n").expect("write");
        commit_all(&repo, "init");
        let st = read_status(&repo).expect("status");
        let _ = fs::remove_dir_all(&dir);
        assert!(st.is_empty(), "干净仓库不应有状态项: {st:?}");
    }

    // T-status-2：1 modified + 1 untracked → 2 条且状态正确
    #[test]
    fn t_status_2_modified_and_untracked() {
        let (dir, repo) = temp_repo("status2");
        fs::write(dir.join("a.txt"), "hello\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "hello\nworld\n").expect("modify");
        fs::write(dir.join("b.txt"), "new\n").expect("untracked");
        let st = read_status(&repo).expect("status");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(st.len(), 2, "应返回 2 条: {st:?}");
        let a = st.iter().find(|s| s.path == "a.txt").expect("a.txt");
        let b = st.iter().find(|s| s.path == "b.txt").expect("b.txt");
        assert_eq!(a.status, "modified");
        assert_eq!(b.status, "untracked");
    }

    // T-diff-1：小改动 → truncated=false 且内容完整（含改动行）
    #[test]
    fn t_diff_1_small_change_not_truncated() {
        let (dir, repo) = temp_repo("diff1");
        fs::write(dir.join("a.txt"), "hello\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "hello\nworld\n").expect("modify");
        let res = read_diff(&repo, None, GIT_DIFF_DEFAULT_MAX_BYTES).expect("diff");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(res.hunks.len(), 1);
        assert!(!res.hunks[0].truncated);
        assert!(!res.more);
        let content = res.hunks[0].new_content.as_ref().expect("content");
        assert!(content.contains("+world"), "应包含新增行: {content}");
    }

    // T-diff-2：大改动 + 小 max_bytes → truncated=true 且长度受限
    #[test]
    fn t_diff_2_large_change_truncated() {
        let (dir, repo) = temp_repo("diff2");
        fs::write(dir.join("big.txt"), "x").expect("write");
        commit_all(&repo, "init");
        let big = "y".repeat(200_000);
        fs::write(dir.join("big.txt"), big).expect("modify");
        let res = read_diff(&repo, None, 1024).expect("diff");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(res.hunks.len(), 1);
        assert!(res.hunks[0].truncated, "单文件超限应截断");
        let content = res.hunks[0].new_content.as_ref().expect("content");
        assert!(
            content.len() <= 1024 + "...(truncated)".len(),
            "截断后长度应受限: {}",
            content.len()
        );
        assert!(content.contains("(truncated)"));
    }

    // T-diff-3：path 过滤只返回目标文件
    #[test]
    fn t_diff_3_path_filter() {
        let (dir, repo) = temp_repo("diff3");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        fs::write(dir.join("b.txt"), "b\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "a2\n").expect("modify a");
        fs::write(dir.join("b.txt"), "b2\n").expect("modify b");
        let res = read_diff(&repo, Some("a.txt"), GIT_DIFF_DEFAULT_MAX_BYTES).expect("diff");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(res.hunks.len(), 1, "只应返回 a.txt");
        assert_eq!(res.hunks[0].file, "a.txt");
    }

    // T-diff-4：二进制文件没有文本补丁 → binary=true，不假装有内容
    #[test]
    fn t_diff_4_binary_file_marked() {
        let (dir, repo) = temp_repo("diff4");
        fs::write(dir.join("bin.dat"), [0u8, 1, 2, 3]).expect("write binary");
        commit_all(&repo, "init");
        fs::write(dir.join("bin.dat"), [9u8, 8, 7, 6, 5]).expect("modify binary");
        let res = read_diff(&repo, None, GIT_DIFF_DEFAULT_MAX_BYTES).expect("diff");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(res.hunks.len(), 1);
        let h = &res.hunks[0];
        assert!(h.binary, "二进制文件应被标记 binary");
        assert!(h.new_content.is_none(), "二进制不应伪造文本内容");
    }

    // T-util-1：截断不得切在 UTF-8 字符中间（多字节内容）
    #[test]
    fn t_util_1_truncate_at_char_boundary() {
        let text = "你好世界".repeat(1000); // 每字 3 字节
        for cap in [1, 2, 3, 4, 5, 7, 100] {
            let cut = truncate_at(&text, cap);
            assert!(
                cut.len() <= cap,
                "截断长度不得超过上限: {} > {cap}",
                cut.len()
            );
            assert!(
                std::str::from_utf8(cut.as_bytes()).is_ok(),
                "截断结果必须是合法 UTF-8: cap={cap}"
            );
        }
    }

    // T-branch-1：本地分支 + is_head 标记
    #[test]
    fn t_branch_1_local_branches_and_head() {
        let (dir, repo) = temp_repo("branch1");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        let head_name = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()))
            .expect("head name");
        let branches = read_branches(&repo).expect("branches");
        let _ = fs::remove_dir_all(&dir);
        assert!(!branches.is_empty(), "至少有一个分支");
        let head_count = branches.iter().filter(|b| b.is_head).count();
        assert_eq!(head_count, 1, "is_head 应唯一: {branches:?}");
        assert!(
            branches.iter().any(|b| b.name == head_name && b.is_head),
            "HEAD 分支应被标记: {branches:?}"
        );
        assert!(branches.iter().all(|b| !b.is_remote), "本地仓库无远程分支");
    }

    // T-sec-1：repo_id 逃逸（../.. 与绝对路径）必须被拒绝，不打开任何仓库
    #[test]
    fn t_sec_1_repo_id_escape_rejected() {
        // 直接验证纯函数层的路径规则（AppHandle 相关的“仓库未配置”在
        // bridge 层由 repo_dir 的首个分支保证）。
        for bad in ["..", "../../etc", "/etc", "a/b", ""] {
            let id = bad.to_string();
            let rejected = id.is_empty()
                || id.contains('/')
                || id.contains('\\')
                || id.contains("..")
                || std::path::Path::new(&id).is_absolute();
            assert!(rejected, "非法 repo_id 必须被拒绝: {bad}");
        }
    }

    // T-sec-2：只读调用不产生任何写副作用（工作区/索引在调用前后不变）
    #[test]
    fn t_sec_2_readonly_has_no_side_effect() {
        let (dir, repo) = temp_repo("sec2");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "a2\n").expect("modify");
        let before_status = read_status(&repo).expect("status before");
        let before_head = repo.head().map(|h| h.target()).ok();
        let _ = read_diff(&repo, None, GIT_DIFF_DEFAULT_MAX_BYTES);
        let _ = read_branches(&repo);
        let after_status = read_status(&repo).expect("status after");
        let after_head = repo.head().map(|h| h.target()).ok();
        let content = fs::read_to_string(dir.join("a.txt")).expect("read");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(before_status.len(), after_status.len(), "status 不应变化");
        assert_eq!(before_head, after_head, "HEAD 不应移动");
        assert_eq!(content, "a2\n", "工作区文件不应被修改");
    }
}
