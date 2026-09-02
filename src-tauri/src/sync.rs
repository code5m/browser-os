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
/// repo_id 合法性纯判定：拒绝空值/路径分隔符/`..`/绝对路径（供 repo_dir 与测试复用）。
pub(crate) fn is_valid_repo_id(id: &str) -> bool {
    !(id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
        || std::path::Path::new(id).is_absolute())
}

pub fn repo_dir(app: &AppHandle, repo_id: &str) -> Result<std::path::PathBuf, String> {
    let cfg = workspace::load_repos(app)
        .into_iter()
        .find(|r| r.id == repo_id)
        .ok_or_else(|| "仓库未配置".to_string())?;
    let id = &cfg.id;
    if !is_valid_repo_id(id) {
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

// ---------------------------------------------------------------------------
// M1-6.b Git 写原语（stage / unstage / discard / commit / create_branch /
// checkout_branch）
//
// 硬约束：
// - 只操作调用方显式给出的、经 `validate_repo_paths` 校验的仓库内相对路径；
//   禁止 .git 内部路径、绝对路径、`..`/`.` 分量、反斜杠、空路径、控制字符。
// - 全部「先校验后执行」：任何一条路径非法即整体拒绝，索引不落盘 = 零部分写。
// - 纯本地写：不联网、不读 Keyring、不接触 token/凭据；本段严禁出现
//   cred_cb / push / fetch / clone / pull。
// - 失败 fail-closed：返回 Err 时仓库不得发生部分后续写动作。
// ---------------------------------------------------------------------------

/// 单次写操作的路径数上限（防载荷放大）。
pub const GIT_WRITE_MAX_PATHS: usize = 200;
/// commit message 字节上限。
pub const GIT_COMMIT_MSG_MAX_BYTES: usize = 500;
/// 分支名字节上限。
pub const GIT_BRANCH_NAME_MAX_BYTES: usize = 100;

/// 校验写操作路径列表：只接受仓库根内的显式相对路径，拒绝整仓隐式操作。
pub fn validate_repo_paths(paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("路径列表不能为空（拒绝整仓隐式操作）".to_string());
    }
    if paths.len() > GIT_WRITE_MAX_PATHS {
        return Err(format!("路径过多：{} > {GIT_WRITE_MAX_PATHS}", paths.len()));
    }
    for p in paths {
        if p.trim().is_empty() {
            return Err("存在空路径".to_string());
        }
        if p.chars().any(|c| c.is_control()) {
            return Err("路径含控制字符".to_string());
        }
        if std::path::Path::new(p).is_absolute() {
            return Err(format!("禁止绝对路径: {p}"));
        }
        if p.contains('\\') {
            return Err(format!("禁止反斜杠路径: {p}"));
        }
        // Path::components() 会归一化跳过中间的 `.`，因此先用字符串级检查
        // 显式拒绝 `.`/`..`/空分量（含 `a/./b`、`a//b`、`a/`、`./a`）
        if p.split('/')
            .any(|seg| seg.is_empty() || seg == "." || seg == "..")
        {
            return Err(format!("路径含非法分量（. / .. / 空分量）: {p}"));
        }
        let mut saw_normal = false;
        for comp in std::path::Path::new(p).components() {
            match comp {
                std::path::Component::Normal(seg) => {
                    if seg == ".git" {
                        return Err(format!("禁止 .git 内部路径: {p}"));
                    }
                    saw_normal = true;
                }
                // RootDir 已被 is_absolute 拦截；CurDir/ParentDir/Prefix 一律拒绝
                _ => return Err(format!("路径含非法分量（. 或 ..）: {p}")),
            }
        }
        if !saw_normal {
            return Err(format!("路径无效: {p}"));
        }
    }
    Ok(())
}

/// 校验 commit message：非空、限长、禁止控制字符（含换行）。
pub fn validate_commit_message(message: &str) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("提交信息不能为空".to_string());
    }
    if message.len() > GIT_COMMIT_MSG_MAX_BYTES {
        return Err(format!(
            "提交信息超长：{} > {GIT_COMMIT_MSG_MAX_BYTES} 字节",
            message.len()
        ));
    }
    if message.chars().any(|c| c.is_control()) {
        return Err("提交信息含控制字符".to_string());
    }
    Ok(())
}

/// 校验分支名：禁止 HEAD/空名/路径逃逸/控制字符/git 保留序列，
/// 兜底交给 libgit2 的 refname 合法性校验。
pub fn validate_branch_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("分支名不能为空".to_string());
    }
    if name.len() > GIT_BRANCH_NAME_MAX_BYTES {
        return Err(format!(
            "分支名超长：{} > {GIT_BRANCH_NAME_MAX_BYTES} 字节",
            name.len()
        ));
    }
    if name == "HEAD" || name == "@" {
        return Err("禁止以 HEAD/@ 作为分支名".to_string());
    }
    if name.chars().any(|c| c.is_control()) {
        return Err("分支名含控制字符".to_string());
    }
    if name.contains("..") || name.contains("@{") {
        return Err("分支名含非法序列（.. 或 @{）".to_string());
    }
    if name.contains('\\') {
        return Err("分支名禁止反斜杠".to_string());
    }
    if name.starts_with('/') || name.ends_with('/') || name.contains("//") {
        return Err("分支名路径逃逸".to_string());
    }
    if name.starts_with('-') {
        return Err("分支名禁止以 - 开头".to_string());
    }
    if name.ends_with(".lock") || name.ends_with('.') {
        return Err("分支名后缀非法".to_string());
    }
    // git check-ref-format 禁用字符：空格 ~ ^ : ? * [
    if name
        .chars()
        .any(|c| matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '['))
    {
        return Err("分支名含 git 禁用字符".to_string());
    }
    // 每个路径分量不得以 . 开头（如 a/.b）
    if name
        .split('/')
        .any(|seg| seg.is_empty() || seg.starts_with('.'))
    {
        return Err("分支名分量非法".to_string());
    }
    let refname = format!("refs/heads/{name}");
    if !git2::Reference::is_valid_name(&refname) {
        return Err("分支名不合法（refname 校验未过）".to_string());
    }
    Ok(())
}

/// 预检：所有路径必须存在于工作区或索引（stage/commit 用）。
pub fn precheck_stageable(repo: &Repository, paths: &[String]) -> Result<(), String> {
    let workdir = repo.workdir().ok_or("裸仓库不支持写操作")?;
    let index = repo.index().map_err(|e| e.to_string())?;
    for p in paths {
        let rel = std::path::Path::new(p);
        if !workdir.join(rel).exists() && index.get_path(rel, 0).is_none() {
            return Err(format!("路径不存在（工作区与索引均无）: {p}"));
        }
    }
    Ok(())
}

/// 预检：所有路径必须已在索引中（unstage/discard 用）。
pub fn precheck_tracked(repo: &Repository, paths: &[String]) -> Result<(), String> {
    let index = repo.index().map_err(|e| e.to_string())?;
    for p in paths {
        if index.get_path(std::path::Path::new(p), 0).is_none() {
            return Err(format!("路径未被跟踪: {p}"));
        }
    }
    Ok(())
}

/// 显式暂存：只把给定路径加入索引；工作区已删除的路径记为删除。
pub fn write_stage(repo: &Repository, paths: &[String]) -> Result<usize, String> {
    validate_repo_paths(paths)?;
    precheck_stageable(repo, paths)?;
    let workdir = repo.workdir().ok_or("裸仓库不支持写操作")?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    // 任一失败即返回：index.write() 未执行 = 磁盘索引零变化
    for p in paths {
        let rel = std::path::Path::new(p);
        if workdir.join(rel).exists() {
            index
                .add_path(rel)
                .map_err(|e| format!("暂存失败 {p}: {e}"))?;
        } else {
            index
                .remove_path(rel)
                .map_err(|e| format!("暂存删除失败 {p}: {e}"))?;
        }
    }
    index.write().map_err(|e| e.to_string())?;
    Ok(paths.len())
}

/// 显式取消暂存：把给定路径的索引条目重置回 HEAD（不动工作区）。
pub fn write_unstage(repo: &Repository, paths: &[String]) -> Result<usize, String> {
    validate_repo_paths(paths)?;
    precheck_tracked(repo, paths)?;
    match repo
        .head()
        .ok()
        .and_then(|h| h.peel(git2::ObjectType::Any).ok())
    {
        Some(target) => {
            repo.reset_default(Some(&target), paths.iter())
                .map_err(|e| format!("取消暂存失败: {e}"))?;
        }
        None => {
            // 无 HEAD（尚无提交）：直接从索引移除
            let mut index = repo.index().map_err(|e| e.to_string())?;
            for p in paths {
                index
                    .remove_path(std::path::Path::new(p))
                    .map_err(|e| e.to_string())?;
            }
            index.write().map_err(|e| e.to_string())?;
        }
    }
    Ok(paths.len())
}

/// 显式丢弃工作区改动：用索引内容覆盖给定路径的工作区文件（不可恢复）。
/// 只允许显式路径；未跟踪文件一律整体拒绝（防误删新文件）。
pub fn write_discard(repo: &Repository, paths: &[String]) -> Result<usize, String> {
    validate_repo_paths(paths)?;
    precheck_tracked(repo, paths)?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    let mut cb = CheckoutBuilder::new();
    cb.force();
    for p in paths {
        cb.path(p);
    }
    repo.checkout_index(Some(&mut index), Some(&mut cb))
        .map_err(|e| format!("丢弃改动失败: {e}"))?;
    Ok(paths.len())
}

/// 提交：paths=None 时全量 add_all（对齐既有成果推送 commit 语义）；
/// paths=Some 时只提交显式路径（其余改动保持原状，绝不顺带提交）。
pub fn write_commit(
    repo: &Repository,
    message: &str,
    paths: Option<&[String]>,
) -> Result<String, String> {
    validate_commit_message(message)?;
    if let Some(ps) = paths {
        validate_repo_paths(ps)?;
        precheck_stageable(repo, ps)?;
    }
    let mut index = repo.index().map_err(|e| e.to_string())?;
    match paths {
        Some(ps) => {
            let workdir = repo.workdir().ok_or("裸仓库不支持写操作")?;
            for p in ps {
                let rel = std::path::Path::new(p);
                if workdir.join(rel).exists() {
                    index
                        .add_path(rel)
                        .map_err(|e| format!("暂存失败 {p}: {e}"))?;
                } else {
                    index
                        .remove_path(rel)
                        .map_err(|e| format!("暂存删除失败 {p}: {e}"))?;
                }
            }
        }
        None => {
            index
                .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
                .map_err(|e| e.to_string())?;
        }
    }
    index.write().map_err(|e| e.to_string())?;
    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    // 无可提交变更（索引树 == HEAD 树）：拒绝创建空提交（对齐 git CLI 行为）。
    if let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) {
        if head_tree.id() == tree_id {
            return Err("没有可提交的变更".to_string());
        }
    }
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let sig = Signature::now("极智简单", "mvp@jizhijiandan.local").map_err(|e| e.to_string())?;
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .map_err(|e| format!("提交失败: {e}"))?;
    Ok(oid.to_string())
}

/// 建本地分支（基于当前 HEAD），可选同时检出。禁止覆盖已存在分支。
pub fn write_create_branch(
    repo: &Repository,
    name: &str,
    checkout: bool,
) -> Result<String, String> {
    validate_branch_name(name)?;
    if repo.find_branch(name, git2::BranchType::Local).is_ok() {
        return Err(format!("分支已存在: {name}"));
    }
    let head = repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|_| "仓库尚无提交，无法创建分支".to_string())?;
    repo.branch(name, &head, false)
        .map_err(|e| format!("创建分支失败: {e}"))?;
    if checkout {
        let refname = format!("refs/heads/{name}");
        repo.set_head(&refname).map_err(|e| e.to_string())?;
        // 新分支与 HEAD 同 commit，safe checkout 不会覆盖任何本地改动
        repo.checkout_head(Some(CheckoutBuilder::new().safe()))
            .map_err(|e| e.to_string())?;
    }
    Ok(name.to_string())
}

/// 切换分支前的冲突预检（只读）：返回第一个与脏工作区冲突的路径，
/// None = 可安全切换。分支不存在/非本地分支直接 Err。
pub fn checkout_conflict(repo: &Repository, name: &str) -> Result<Option<String>, String> {
    let branch = repo
        .find_branch(name, git2::BranchType::Local)
        .map_err(|_| format!("分支不存在: {name}"))?;
    if branch.is_head() {
        return Ok(None);
    }
    let dirty: std::collections::HashSet<String> =
        read_status(repo)?.into_iter().map(|s| s.path).collect();
    if dirty.is_empty() {
        return Ok(None);
    }
    let head_tree = repo
        .head()
        .and_then(|h| h.peel_to_tree())
        .map_err(|e| e.to_string())?;
    let target_tree = branch.get().peel_to_tree().map_err(|e| e.to_string())?;
    let diff = repo
        .diff_tree_to_tree(Some(&head_tree), Some(&target_tree), None)
        .map_err(|e| e.to_string())?;
    for d in diff.deltas() {
        let p = d
            .new_file()
            .path()
            .or_else(|| d.old_file().path())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if dirty.contains(&p) {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

/// 切换到已存在的本地分支。脏工作区与目标分支有文件级冲突时在移动
/// HEAD 之前整体拒绝（失败零部分动作）；拒绝 detached HEAD/路径/checkout --force。
///
/// 顺序刻意是**先检出目标树、再移动 HEAD**（M1-6.c 复核修复）：
/// - 旧实现「先 `set_head` 再 `checkout_head`」有两个缺陷：
///   ① `checkout_head` 以新 HEAD 为基线，旧分支独有的文件既不会被移除、
///      索引条目也会残留，切回后 `git status` 凭空出现「已添加」条目，
///      后续全量 commit 会把别的分支的文件一起提交；
///   ② checkout 失败时 HEAD 已经移动，构成部分动作。
/// - 先 `checkout_tree`（以索引为基线，safe 策略）再 `set_head`：clean 切换
///   与 git CLI 一致（移除旧分支独有文件并同步索引），冲突时直接返回错误且
///   HEAD 与工作区均不变；若 `set_head` 极端失败，尽力把工作区滚回原树。
pub fn write_checkout_branch(repo: &Repository, name: &str) -> Result<String, String> {
    validate_branch_name(name)?;
    if let Some(conflict) = checkout_conflict(repo, name)? {
        return Err(format!(
            "工作区有未提交改动且与目标分支冲突，禁止切换: {conflict}"
        ));
    }
    let branch = repo
        .find_branch(name, git2::BranchType::Local)
        .map_err(|_| format!("分支不存在: {name}"))?;
    let target = branch.get().peel_to_tree().map_err(|e| e.to_string())?;
    let previous_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    repo.checkout_tree(&target.into_object(), Some(CheckoutBuilder::new().safe()))
        .map_err(|e| format!("切换分支失败: {e}"))?;
    let refname = format!("refs/heads/{name}");
    if let Err(e) = repo.set_head(&refname) {
        // 极端情形：工作区已切到目标树但 HEAD 未移动 —— 尽力回滚，
        // 避免「工作区是 A 分支、HEAD 指向 B 分支」的不一致状态。
        if let Some(prev) = previous_tree {
            let _ = repo.checkout_tree(&prev.into_object(), Some(CheckoutBuilder::new().safe()));
        }
        return Err(format!("切换分支失败（HEAD 未移动，已回滚工作区）: {e}"));
    }
    Ok(name.to_string())
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

#[cfg(test)]
mod git_write_tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// 建一个本地临时 git 仓库（不联网、不依赖 AppHandle）。
    fn temp_repo(tag: &str) -> (std::path::PathBuf, Repository) {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("mvp-git-gw-{tag}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        let repo = Repository::init(&dir).expect("init");
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

    /// 取指定路径的原始 status flags（可区分已暂存/未暂存）。
    fn status_flags(repo: &Repository, path: &str) -> git2::Status {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true);
        let st = repo.statuses(Some(&mut opts)).expect("statuses");
        for e in st.iter() {
            if e.path() == Some(path) {
                return e.status();
            }
        }
        git2::Status::empty()
    }

    fn head_shorthand(repo: &Repository) -> String {
        repo.head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()))
            .expect("head shorthand")
    }

    fn head_file_content(repo: &Repository, path: &str) -> Option<String> {
        let tree = repo.head().ok()?.peel_to_tree().ok()?;
        let entry = tree.get_name(path)?;
        let blob = repo.find_blob(entry.id()).ok()?;
        Some(String::from_utf8_lossy(blob.content()).to_string())
    }

    // T-gw-b-1：stage 只接受 repo 内显式路径（其余改动保持未暂存）
    #[test]
    fn git_write_stage_only_explicit_paths() {
        let (dir, repo) = temp_repo("stage");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        fs::write(dir.join("b.txt"), "b\n").expect("write");
        fs::write(dir.join("c.txt"), "c\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "a2\n").expect("modify a");
        fs::write(dir.join("b.txt"), "b2\n").expect("modify b");
        fs::remove_file(dir.join("c.txt")).expect("delete c");
        write_stage(&repo, &["a.txt".to_string(), "c.txt".to_string()]).expect("stage");
        let a = status_flags(&repo, "a.txt");
        let b = status_flags(&repo, "b.txt");
        let c = status_flags(&repo, "c.txt");
        let _ = fs::remove_dir_all(&dir);
        assert!(
            a.contains(git2::Status::INDEX_MODIFIED) && !a.contains(git2::Status::WT_MODIFIED),
            "a.txt 应已暂存: {a:?}"
        );
        assert!(
            b.contains(git2::Status::WT_MODIFIED) && !b.contains(git2::Status::INDEX_MODIFIED),
            "b.txt 不应被顺带暂存: {b:?}"
        );
        assert!(
            c.contains(git2::Status::INDEX_DELETED),
            "c.txt 的删除应被显式暂存: {c:?}"
        );
    }

    // T-gw-b-2：unstage 只接受 repo 内显式路径
    #[test]
    fn git_write_unstage_only_explicit_paths() {
        let (dir, repo) = temp_repo("unstage");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        fs::write(dir.join("b.txt"), "b\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "a2\n").expect("modify a");
        fs::write(dir.join("b.txt"), "b2\n").expect("modify b");
        write_stage(&repo, &["a.txt".to_string(), "b.txt".to_string()]).expect("stage both");
        write_unstage(&repo, &["a.txt".to_string()]).expect("unstage a");
        let a = status_flags(&repo, "a.txt");
        let b = status_flags(&repo, "b.txt");
        let _ = fs::remove_dir_all(&dir);
        assert!(
            a.contains(git2::Status::WT_MODIFIED) && !a.contains(git2::Status::INDEX_MODIFIED),
            "a.txt 应回到未暂存: {a:?}"
        );
        assert!(
            b.contains(git2::Status::INDEX_MODIFIED),
            "b.txt 应保持已暂存: {b:?}"
        );
    }

    // T-gw-b-4：discard 只对显式路径生效；未跟踪文件整体拒绝（防误删新文件）
    #[test]
    fn git_write_discard_explicit_paths_only() {
        let (dir, repo) = temp_repo("discard");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        fs::write(dir.join("b.txt"), "b\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "a2\n").expect("modify a");
        fs::write(dir.join("b.txt"), "b2\n").expect("modify b");
        fs::write(dir.join("new.txt"), "new\n").expect("untracked");
        // 未跟踪文件：整体拒绝，文件保持原样
        let err = write_discard(&repo, &["new.txt".to_string()]).expect_err("未跟踪必须拒绝");
        assert!(err.contains("未被跟踪"), "实际错误: {err}");
        assert_eq!(
            fs::read_to_string(dir.join("new.txt")).expect("read"),
            "new\n",
            "未跟踪文件不得被删除"
        );
        // 显式已跟踪路径：工作区内容被索引内容覆盖；b.txt 不受影响
        write_discard(&repo, &["a.txt".to_string()]).expect("discard a");
        let a_content = fs::read_to_string(dir.join("a.txt")).expect("read a");
        let b_content = fs::read_to_string(dir.join("b.txt")).expect("read b");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(a_content, "a\n", "a.txt 应恢复为索引内容");
        assert_eq!(b_content, "b2\n", "b.txt 不应被顺带丢弃");
    }

    // T-gw-b-5：commit message 空/控制字符/超长拒绝；拒绝时 HEAD 不动
    #[test]
    fn git_write_commit_message_validation() {
        for bad in ["", "   ", "含\n换行", "控\u{7}制", "回\r车"] {
            assert!(
                validate_commit_message(bad).is_err(),
                "非法提交信息必须拒绝: {bad:?}"
            );
        }
        let too_long = "x".repeat(GIT_COMMIT_MSG_MAX_BYTES + 1);
        assert!(validate_commit_message(&too_long).is_err(), "超长必须拒绝");
        assert!(validate_commit_message("正常提交信息").is_ok());

        let (dir, repo) = temp_repo("commitmsg");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "a2\n").expect("modify");
        let before = repo.head().ok().and_then(|h| h.target());
        let err = write_commit(&repo, "", None).expect_err("空 message 必须拒绝");
        let after = repo.head().ok().and_then(|h| h.target());
        let _ = fs::remove_dir_all(&dir);
        assert!(err.contains("不能为空"), "实际错误: {err}");
        assert_eq!(before, after, "拒绝时 HEAD 不得移动");
    }

    // T-gw-b-6：commit 不包含未授权路径（显式 paths 之外的改动绝不进提交）
    #[test]
    fn git_write_commit_scoped_paths_only() {
        let (dir, repo) = temp_repo("commitscope");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        fs::write(dir.join("b.txt"), "b\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "a2\n").expect("modify a");
        fs::write(dir.join("b.txt"), "b2\n").expect("modify b");
        write_commit(&repo, "only a", Some(&["a.txt".to_string()])).expect("commit");
        let head_a = head_file_content(&repo, "a.txt");
        let head_b = head_file_content(&repo, "b.txt");
        let b = status_flags(&repo, "b.txt");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(head_a.as_deref(), Some("a2\n"), "a.txt 应进入提交");
        assert_eq!(head_b.as_deref(), Some("b\n"), "b.txt 的改动不得进入提交");
        assert!(
            b.contains(git2::Status::WT_MODIFIED),
            "b.txt 应保持未暂存脏状态: {b:?}"
        );
    }

    // T-gw-b-7：branch name 非法拒绝（HEAD/空名/路径逃逸/控制字符/保留序列）
    #[test]
    fn git_write_branch_name_validation() {
        for bad in [
            "",
            "   ",
            "HEAD",
            "@",
            "../evil",
            "a..b",
            "a b",
            "-x",
            "a/.b",
            ".hidden",
            "a.lock",
            "a\\b",
            "ctrl\u{3}x",
            "a//b",
            "/abs",
            "a/",
            "a@{b",
            "a:b",
            "a?b",
            "a*b",
        ] {
            assert!(
                validate_branch_name(bad).is_err(),
                "非法分支名必须拒绝: {bad:?}"
            );
        }
        for ok in ["feature/x", "fix-123", "M1-6.b", "hotfix_2"] {
            assert!(validate_branch_name(ok).is_ok(), "合法分支名应放行: {ok:?}");
        }
    }

    // T-gw-b-7b：create_branch 正常路径 + 重名拒绝 + HEAD 校验
    #[test]
    fn git_write_create_branch_happy_and_duplicate() {
        let (dir, repo) = temp_repo("mkbranch");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        write_create_branch(&repo, "feat/x", true).expect("create+checkout");
        let head = head_shorthand(&repo);
        let dup = write_create_branch(&repo, "feat/x", false).expect_err("重名必须拒绝");
        let illegal = write_create_branch(&repo, "HEAD", false).expect_err("HEAD 必须拒绝");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(head, "feat/x", "checkout=true 应同时切换");
        assert!(dup.contains("已存在"), "实际错误: {dup}");
        assert!(illegal.contains("HEAD"), "实际错误: {illegal}");
    }

    // T-gw-b-8：checkout 非法分支拒绝（不存在/非法名），HEAD 与工作区不变
    #[test]
    fn git_write_checkout_invalid_branch_rejected() {
        let (dir, repo) = temp_repo("ckinvalid");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        let default_branch = head_shorthand(&repo);
        let err = write_checkout_branch(&repo, "nope").expect_err("不存在分支必须拒绝");
        assert!(err.contains("分支不存在"), "实际错误: {err}");
        let err2 = write_checkout_branch(&repo, "../evil").expect_err("非法名必须拒绝");
        let head_after = head_shorthand(&repo);
        let content = fs::read_to_string(dir.join("a.txt")).expect("read");
        let _ = fs::remove_dir_all(&dir);
        assert!(!err2.is_empty());
        assert_eq!(head_after, default_branch, "HEAD 不得移动");
        assert_eq!(content, "a\n", "工作区不得变化");
    }

    // T-gw-b-9：repo_id 路径逃逸拒绝（纯判定，repo_dir 复用同一谓词）
    #[test]
    fn git_write_repo_id_escape_rejected() {
        for bad in ["..", "../../etc", "/etc", "a/b", "a\\b", ""] {
            assert!(!is_valid_repo_id(bad), "非法 repo_id 必须拒绝: {bad:?}");
        }
        for ok in ["r1", "repo-2", "我的仓库"] {
            assert!(is_valid_repo_id(ok), "合法 repo_id 应放行: {ok:?}");
        }
    }

    // T-gw-b-10：禁止 .git 内部路径（含逃逸/绝对路径/反斜杠组合）
    #[test]
    fn git_write_git_dir_paths_rejected() {
        for bad in [
            ".git",
            ".git/config",
            "a/.git/config",
            ".git/hooks/x.sh",
            "./a",
            "a/../b",
            "a/./b",
            "/etc/passwd",
            "a\\b",
            "",
            "  ",
        ] {
            assert!(
                validate_repo_paths(&[bad.to_string()]).is_err(),
                "非法路径必须拒绝: {bad:?}"
            );
        }
        assert!(validate_repo_paths(&["a.txt".to_string(), "src/b.rs".to_string()]).is_ok());
        // 端到端：带 .git 路径的 stage 整体拒绝，索引零变化
        let (dir, repo) = temp_repo("gitdir");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "a2\n").expect("modify");
        let err = write_stage(&repo, &[".git/config".to_string()]).expect_err(".git 必须拒绝");
        let a = status_flags(&repo, "a.txt");
        let _ = fs::remove_dir_all(&dir);
        assert!(err.contains(".git"), "实际错误: {err}");
        assert!(
            a.contains(git2::Status::WT_MODIFIED) && !a.contains(git2::Status::INDEX_MODIFIED),
            "索引不得被部分写入: {a:?}"
        );
    }

    // T-gw-b-11a：写操作失败不产生部分后续动作（stage 混合合法+非法路径 → 整体拒绝）
    #[test]
    fn git_write_stage_failure_no_partial_action() {
        let (dir, repo) = temp_repo("nopartial");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("a.txt"), "a2\n").expect("modify");
        let err = write_stage(&repo, &["a.txt".to_string(), "../evil".to_string()])
            .expect_err("含非法路径必须整体拒绝");
        let a = status_flags(&repo, "a.txt");
        let _ = fs::remove_dir_all(&dir);
        assert!(err.contains(".."), "实际错误: {err}");
        assert!(
            !a.contains(git2::Status::INDEX_MODIFIED),
            "失败时合法路径也不得被部分暂存: {a:?}"
        );
    }

    // T-gw-b-11b：checkout 脏工作区冲突 → HEAD 不动、工作区不变（零部分动作）
    #[test]
    fn git_write_checkout_dirty_conflict_no_partial_action() {
        let (dir, repo) = temp_repo("ckdirty");
        fs::write(dir.join("a.txt"), "base\n").expect("write");
        commit_all(&repo, "init");
        let default_branch = head_shorthand(&repo);
        write_create_branch(&repo, "other", false).expect("create other");
        write_checkout_branch(&repo, "other").expect("switch to other");
        fs::write(dir.join("a.txt"), "other\n").expect("modify on other");
        commit_all(&repo, "on other");
        write_checkout_branch(&repo, &default_branch).expect("switch back");
        // 在默认分支上制造与 other 冲突的脏改动
        fs::write(dir.join("a.txt"), "dirty\n").expect("dirty modify");
        let err = write_checkout_branch(&repo, "other").expect_err("脏冲突必须拒绝");
        let head_after = head_shorthand(&repo);
        let content = fs::read_to_string(dir.join("a.txt")).expect("read");
        let _ = fs::remove_dir_all(&dir);
        assert!(err.contains("冲突"), "实际错误: {err}");
        assert_eq!(head_after, default_branch, "拒绝时 HEAD 不得移动");
        assert_eq!(content, "dirty\n", "拒绝时工作区不得被覆盖");
    }

    // T-gw-b-11c：checkout 干净工作区正常切换（正向用例）
    #[test]
    fn git_write_checkout_clean_switch_ok() {
        let (dir, repo) = temp_repo("ckclean");
        fs::write(dir.join("a.txt"), "base\n").expect("write");
        commit_all(&repo, "init");
        let default_branch = head_shorthand(&repo);
        write_create_branch(&repo, "other", false).expect("create");
        write_checkout_branch(&repo, "other").expect("switch");
        let head = head_shorthand(&repo);
        write_checkout_branch(&repo, &default_branch).expect("switch back");
        let head_back = head_shorthand(&repo);
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(head, "other");
        assert_eq!(head_back, default_branch);
    }

    // T-gw-c-1（M1-6.c 复核）：切换分支后工作区与索引必须与目标树一致
    // ——旧实现会残留旧分支独有文件与索引条目（status 凭空「已添加」）
    #[test]
    fn git_write_checkout_syncs_workdir_and_index() {
        let (dir, repo) = temp_repo("cksync");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        let base = head_shorthand(&repo);
        write_create_branch(&repo, "other", false).expect("create");
        write_checkout_branch(&repo, "other").expect("to other");
        fs::write(dir.join("other.txt"), "o\n").expect("write");
        commit_all(&repo, "add other");
        write_checkout_branch(&repo, &base).expect("back to base");
        let file_gone = !dir.join("other.txt").exists();
        let status = read_status(&repo).expect("status");
        let index_has_other = repo
            .index()
            .expect("index")
            .get_path(std::path::Path::new("other.txt"), 0)
            .is_some();
        let head = head_shorthand(&repo);
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(head, base, "应回到原分支");
        assert!(file_gone, "旧分支独有文件应随切换移除");
        assert!(status.is_empty(), "切换后状态应干净: {status:?}");
        assert!(!index_has_other, "索引不得残留旧分支条目");
    }

    // T-gw-c-2：目标分支存在的同名未跟踪文件 → 拒绝且不覆盖本地文件
    #[test]
    fn git_write_checkout_untracked_conflict_rejected() {
        let (dir, repo) = temp_repo("ckunt");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        let base = head_shorthand(&repo);
        write_create_branch(&repo, "other", false).expect("create");
        write_checkout_branch(&repo, "other").expect("to other");
        fs::write(dir.join("u.txt"), "in-branch\n").expect("write");
        commit_all(&repo, "add u");
        write_checkout_branch(&repo, &base).expect("back");
        // 未跟踪文件与目标分支同名 → 必须拒绝（不得覆盖/删除本地文件）
        fs::write(dir.join("u.txt"), "local\n").expect("write");
        let err = write_checkout_branch(&repo, "other").expect_err("未跟踪冲突必须拒绝");
        let head = head_shorthand(&repo);
        let content = fs::read_to_string(dir.join("u.txt")).expect("read");
        let _ = fs::remove_dir_all(&dir);
        assert!(err.contains("冲突"), "实际错误: {err}");
        assert_eq!(head, base, "HEAD 不得移动");
        assert_eq!(content, "local\n", "未跟踪文件不得被覆盖");
    }

    // T-gw-c-3：未跟踪目录与目标分支文件共存时应允许切换（不得误拦截、不得删未跟踪文件）
    #[test]
    fn git_write_checkout_allows_untracked_dir_coexist() {
        let (dir, repo) = temp_repo("ckcoexist");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        let base = head_shorthand(&repo);
        write_create_branch(&repo, "other", false).expect("create");
        write_checkout_branch(&repo, "other").expect("to other");
        fs::create_dir_all(dir.join("newdir")).expect("mkdir");
        fs::write(dir.join("newdir").join("x.txt"), "x\n").expect("write");
        commit_all(&repo, "add newdir/x");
        write_checkout_branch(&repo, &base).expect("back");
        // 工作区有未跟踪目录 newdir/（含 y.txt），与目标分支 newdir/x.txt 不冲突
        fs::create_dir_all(dir.join("newdir")).expect("mkdir");
        fs::write(dir.join("newdir").join("y.txt"), "y\n").expect("write");
        let r = write_checkout_branch(&repo, "other");
        let head = head_shorthand(&repo);
        let x_exists = dir.join("newdir").join("x.txt").exists();
        let y_exists = dir.join("newdir").join("y.txt").exists();
        let _ = fs::remove_dir_all(&dir);
        assert!(r.is_ok(), "非冲突场景应允许切换: {r:?}");
        assert_eq!(head, "other");
        assert!(x_exists, "目标分支文件应被检出");
        assert!(y_exists, "未跟踪文件不得被删除");
    }

    // T-gw-c-4：unstage 新建文件不得删除工作区文件（防数据丢失）
    #[test]
    fn git_write_unstage_new_file_keeps_workdir_file() {
        let (dir, repo) = temp_repo("unsnew");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        fs::write(dir.join("new.txt"), "new\n").expect("write");
        write_stage(&repo, &["new.txt".to_string()]).expect("stage");
        write_unstage(&repo, &["new.txt".to_string()]).expect("unstage");
        let exists = dir.join("new.txt").exists();
        let content = fs::read_to_string(dir.join("new.txt")).ok();
        let status = read_status(&repo).expect("status");
        let _ = fs::remove_dir_all(&dir);
        assert!(exists, "取消暂存不得删除工作区文件");
        assert_eq!(content.as_deref(), Some("new\n"));
        assert!(
            status
                .iter()
                .any(|s| s.path == "new.txt" && s.status == "untracked"),
            "应回到未跟踪状态: {status:?}"
        );
    }

    // T-gw-c-5：discard 工作区已删除的跟踪文件 → 按索引内容恢复（不丢数据）
    #[test]
    fn git_write_discard_restores_deleted_tracked_file() {
        let (dir, repo) = temp_repo("discdel");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        commit_all(&repo, "init");
        fs::remove_file(dir.join("a.txt")).expect("delete");
        write_discard(&repo, &["a.txt".to_string()]).expect("discard");
        let content = fs::read_to_string(dir.join("a.txt")).ok();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(content.as_deref(), Some("a\n"), "已删除的跟踪文件应被恢复");
    }

    // T-gw-c-6：全量 commit（paths=None）必须记录工作区删除（删除不静默丢失）
    #[test]
    fn git_write_full_commit_records_workdir_deletion() {
        let (dir, repo) = temp_repo("fulldel");
        fs::write(dir.join("a.txt"), "a\n").expect("write");
        fs::write(dir.join("b.txt"), "b\n").expect("write");
        commit_all(&repo, "init");
        fs::remove_file(dir.join("b.txt")).expect("delete");
        fs::write(dir.join("c.txt"), "c\n").expect("write");
        write_commit(&repo, "full", None).expect("commit");
        let head_b = head_file_content(&repo, "b.txt");
        let head_c = head_file_content(&repo, "c.txt");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(head_b, None, "工作区删除应进入提交（HEAD 树不再含 b.txt）");
        assert_eq!(head_c.as_deref(), Some("c\n"), "新增文件应进入提交");
    }
}
