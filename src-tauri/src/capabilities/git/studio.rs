//! Unified Git execution for managed repositories and authorized worktrees.
//! Reads never fetch. Writes are one-shot previews pinned to HEAD/index/worktree.
use super::local_readonly::discover_allowed;
use crate::shared::invocation::check_invocation_source;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, Webview};

const OUTPUT_CAP: usize = 1024 * 1024;
const TEXT_CAP: usize = 128 * 1024;
const FILE_CAP: usize = 500;
#[derive(Clone, Deserialize, Serialize)]
pub struct StudioSource {
    pub path: Option<String>,
    pub repo_id: Option<String>,
}
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct StudioInput {
    pub action: String,
    #[serde(default)]
    pub paths: Vec<String>,
    pub target: Option<String>,
    pub message: Option<String>,
    pub content: Option<String>,
    pub revision: Option<String>,
    pub other: Option<String>,
    pub search: Option<String>,
    pub author: Option<String>,
    pub offset: Option<u32>,
    pub staged: Option<bool>,
    pub pr: Option<u32>,
    pub expected_fingerprint: Option<String>,
}
struct Ticket {
    source: StudioSource,
    input: StudioInput,
    fingerprint: String,
    root: PathBuf,
    expires: Instant,
    remote_pin: Option<String>,
}
#[derive(Default)]
pub struct GitStudioRuntime {
    tickets: Mutex<HashMap<String, Ticket>>,
    active: Mutex<HashMap<PathBuf, Arc<AtomicBool>>>,
}
struct ActiveGuard<'a> {
    runtime: &'a GitStudioRuntime,
    root: PathBuf,
}
impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.runtime.active.lock() {
            active.remove(&self.root);
        }
    }
}

fn resolve(app: &AppHandle, source: &StudioSource) -> Result<PathBuf, String> {
    match (&source.path, &source.repo_id) {
        (Some(path), None) => {
            let (_, context) = discover_allowed(path, &crate::workspace::allowed_roots(app))?;
            Ok(PathBuf::from(context.root))
        }
        (None, Some(id)) => {
            let repo = crate::sync::open_readonly(app, id)?;
            repo.workdir()
                .ok_or_else(|| "GIT_STUDIO_BARE_DENIED".into())
                .and_then(|p| {
                    p.canonicalize()
                        .map_err(|_| "GIT_STUDIO_ROOT_MISSING".into())
                })
        }
        _ => Err("GIT_STUDIO_EXACTLY_ONE_SOURCE".into()),
    }
}
fn safe_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.len() > 4096
        || path.chars().any(|c| c.is_control())
        || path.contains('\\')
        || Path::new(path).is_absolute()
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || Path::new(path)
            .components()
            .any(|c| c.as_os_str() == ".git")
    {
        return Err("GIT_STUDIO_PATH_INVALID".into());
    }
    Ok(())
}
fn paths(input: &StudioInput) -> Result<Vec<String>, String> {
    if input.paths.is_empty() || input.paths.len() > FILE_CAP {
        return Err("GIT_STUDIO_SELECT_PATHS".into());
    }
    let mut result = input.paths.clone();
    result.sort();
    result.dedup();
    for p in &result {
        safe_path(p)?;
    }
    Ok(result)
}
fn check_file(root: &Path, relative: &str, allow_missing: bool) -> Result<PathBuf, String> {
    safe_path(relative)?;
    let path = root.join(relative);
    let mut parent = path.as_path();
    while !parent.exists() {
        parent = parent.parent().ok_or("GIT_STUDIO_PATH_INVALID")?;
    }
    if !parent
        .canonicalize()
        .map_err(|_| "GIT_STUDIO_PATH_INVALID")?
        .starts_with(root)
    {
        return Err("GIT_STUDIO_PATH_ESCAPE".into());
    }
    match path.symlink_metadata() {
        Ok(meta) if meta.file_type().is_symlink() => Err("GIT_STUDIO_SYMLINK_DENIED".into()),
        Ok(_) => Ok(path),
        Err(e) if allow_missing && e.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(_) => Err("GIT_STUDIO_FILE_MISSING".into()),
    }
}
struct Output {
    code: i32,
    text: String,
}
fn process(
    root: &Path,
    program: &str,
    args: &[String],
    input: Option<&[u8]>,
    index: Option<&Path>,
    cancel: &AtomicBool,
) -> Result<Output, String> {
    if program != "git" && program != "gh" {
        return Err("GIT_STUDIO_PROGRAM_DENIED".into());
    }
    let mut command = Command::new(program);
    if program == "git" {
        command.args(["-c", "core.fsmonitor=false"]);
    }
    command
        .current_dir(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GH_PROMPT_DISABLED", "1")
        .env("GIT_PAGER", "cat")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", "true")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_EXTERNAL_DIFF")
        .env_remove("GIT_CONFIG_PARAMETERS");
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|_| format!("GIT_STUDIO_{}_UNAVAILABLE", program.to_uppercase()))?;
    let stdout = child.stdout.take().ok_or("GIT_STUDIO_PIPE_FAILED")?;
    let overflow = Arc::new(AtomicBool::new(false));
    let of = overflow.clone();
    let reader = thread::spawn(move || {
        let mut stdout = stdout;
        let mut kept = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let count = n.min(OUTPUT_CAP.saturating_sub(kept.len()));
                    kept.extend_from_slice(&buffer[..count]);
                    if count < n {
                        of.store(true, Ordering::Relaxed);
                    }
                }
                Err(_) => break,
            }
        }
        kept
    });
    let bytes = input.unwrap_or_default().to_vec();
    let writer = child.stdin.take().map(|mut stdin| {
        thread::spawn(move || {
            let _ = stdin.write_all(&bytes);
        })
    });
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut interrupted = false;
    let status = loop {
        if cancel.load(Ordering::Relaxed)
            || overflow.load(Ordering::Relaxed)
            || Instant::now() > deadline
        {
            interrupted = true;
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            break child.wait().map_err(|_| "GIT_STUDIO_WAIT_FAILED")?;
        }
        if let Some(status) = child.try_wait().map_err(|_| "GIT_STUDIO_WAIT_FAILED")? {
            break status;
        }
        thread::sleep(Duration::from_millis(20));
    };
    if let Some(writer) = writer {
        let _ = writer.join();
    }
    let bytes = reader.join().map_err(|_| "GIT_STUDIO_READER_FAILED")?;
    if overflow.load(Ordering::Relaxed) {
        return Err("GIT_STUDIO_OUTPUT_LIMIT".into());
    }
    if interrupted {
        return Err("GIT_STUDIO_INTERRUPTED_CHECK_REPOSITORY_STATE".into());
    }
    let text = String::from_utf8(bytes).map_err(|_| "GIT_STUDIO_NON_UTF8_OUTPUT".to_string())?;
    Ok(Output {
        code: status.code().unwrap_or(-1),
        text,
    })
}
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let args = args.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let result = process(root, "git", &args, None, None, &AtomicBool::new(false))?;
    if result.code != 0 {
        return Err("GIT_STUDIO_READ_FAILED".into());
    }
    Ok(result.text)
}
fn oid(root: &Path, revision: &str) -> Result<String, String> {
    if revision.is_empty()
        || revision.len() > 240
        || revision.starts_with('-')
        || revision.chars().any(|c| c.is_control())
    {
        return Err("GIT_STUDIO_REVISION_INVALID".into());
    }
    let resolved = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{revision}^{{commit}}"),
        ],
    )?;
    let resolved = resolved.trim();
    if ![40, 64].contains(&resolved.len()) || !resolved.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("GIT_STUDIO_REVISION_INVALID".into());
    }
    Ok(resolved.into())
}
fn status(root: &Path) -> Result<Vec<Value>, String> {
    let raw = git(
        root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    let mut records = raw.split('\0').filter(|s| !s.is_empty());
    let mut result = Vec::new();
    while let Some(record) = records.next() {
        if record.len() < 4 || !record.is_char_boundary(3) {
            return Err("GIT_STUDIO_STATUS_INVALID".into());
        }
        let x = &record[0..1];
        let y = &record[1..2];
        let path = &record[3..];
        let original = if x == "R" || x == "C" || y == "R" || y == "C" {
            records.next()
        } else {
            None
        };
        result.push(json!({"path":path,"original":original,"index":x,"worktree":y,"staged":x!=" " && x!="?","unstaged":y!=" ","conflicted":x=="U"||y=="U"||["AA","DD"].contains(&&record[..2])}));
        if result.len() > FILE_CAP {
            return Err("GIT_STUDIO_FILE_LIMIT".into());
        }
    }
    Ok(result)
}
fn snapshot(root: &Path) -> Result<String, String> {
    let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
    let mut hash = Sha256::new();
    hash.update(
        repo.head()
            .ok()
            .and_then(|h| h.target())
            .map(|h| h.to_string())
            .unwrap_or_default(),
    );
    hash.update(format!("{:?}", repo.state()));
    hash.update(
        repo.head()
            .ok()
            .and_then(|h| h.name().map(str::to_owned))
            .unwrap_or_default(),
    );
    hash.update(git(root, &["config", "--local", "--null", "--list"])?);
    let index_path = repo.path().join("index");
    if index_path.exists() {
        let meta = fs::metadata(&index_path).map_err(|_| "GIT_STUDIO_INDEX_READ")?;
        if meta.len() > 32 * 1024 * 1024 {
            return Err("GIT_STUDIO_INDEX_LIMIT".into());
        }
        hash.update(fs::read(index_path).map_err(|_| "GIT_STUDIO_INDEX_READ")?);
    }
    let changes = status(root)?;
    let mut total = 0u64;
    for change in changes {
        let relative = change["path"].as_str().ok_or("GIT_STUDIO_STATUS_INVALID")?;
        hash.update(relative.as_bytes());
        hash.update(change.to_string());
        let path = check_file(root, relative, true)?;
        if path.exists() {
            let meta = path.metadata().map_err(|_| "GIT_STUDIO_FILE_READ")?;
            if !meta.is_file() {
                return Err("GIT_STUDIO_SUBMODULE_OR_DIRECTORY_CHANGED".into());
            }
            total += meta.len();
            if meta.len() > 8 * 1024 * 1024 || total > 32 * 1024 * 1024 {
                return Err("GIT_STUDIO_PREVIEW_CONTENT_LIMIT".into());
            }
            hash.update(meta.len().to_be_bytes());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                hash.update(meta.permissions().mode().to_be_bytes());
            }
            hash.update(fs::read(path).map_err(|_| "GIT_STUDIO_FILE_READ")?);
        } else {
            hash.update(b"missing");
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn is_clean(root: &Path) -> Result<(), String> {
    let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
    if repo.state() != git2::RepositoryState::Clean || !status(root)?.is_empty() {
        return Err("GIT_STUDIO_CLEAN_WORKTREE_REQUIRED".into());
    }
    Ok(())
}
fn branch(root: &Path, name: &str) -> Result<(), String> {
    if name == "HEAD" || name.starts_with('-') || name.len() > 200 {
        return Err("GIT_STUDIO_BRANCH_INVALID".into());
    }
    git(root, &["check-ref-format", "--branch", name]).map(|_| ())
}
fn origin(root: &Path) -> Result<String, String> {
    let remote = git(root, &["remote", "get-url", "origin"])?;
    let raw = remote.trim();
    if raw.starts_with("https://") {
        let url = url::Url::parse(raw).map_err(|_| "GIT_STUDIO_ORIGIN_INVALID")?;
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("GIT_STUDIO_ORIGIN_CREDENTIALS_DENIED".into());
        }
    } else if !(raw.starts_with("git@") && raw.contains(':')) && !raw.starts_with("ssh://git@") {
        return Err("GIT_STUDIO_ORIGIN_PROTOCOL_DENIED".into());
    }
    Ok(raw.into())
}
fn slug(root: &Path) -> Result<String, String> {
    let remote = origin(root)?;
    let tail = remote
        .strip_prefix("https://github.com/")
        .or_else(|| remote.strip_prefix("git@github.com:"))
        .or_else(|| remote.strip_prefix("ssh://git@github.com/"))
        .ok_or("GIT_STUDIO_GITHUB_ONLY")?;
    let tail = tail.trim_end_matches('/').trim_end_matches(".git");
    let parts: Vec<_> = tail.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return Err("GIT_STUDIO_GITHUB_REPO_INVALID".into());
    }
    Ok(tail.into())
}
fn gh(root: &Path, args: &[&str]) -> Result<Value, String> {
    let args = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let out = process(root, "gh", &args, None, None, &AtomicBool::new(false))?;
    if out.code != 0 {
        return Err("GIT_STUDIO_GITHUB_AUTH_OR_API_FAILED".into());
    }
    serde_json::from_str(&out.text).map_err(|_| "GIT_STUDIO_GITHUB_JSON_FAILED".into())
}
/// GraphQL CheckRun and legacy StatusContext have different success fields.
/// Missing fields, neutral and skipped checks are never treated as success.
pub(crate) fn studio_check_passed(check: &Value) -> bool {
    if check.get("status").is_some() || check.get("conclusion").is_some() {
        check["status"] == "COMPLETED" && check["conclusion"] == "SUCCESS"
    } else {
        check["state"] == "SUCCESS"
    }
}
fn pr_pin(root: &Path, number: u32) -> Result<String, String> {
    if number == 0 {
        return Err("GIT_STUDIO_PR_INVALID".into());
    }
    let repo = slug(root)?;
    let n = number.to_string();
    let pr = gh(
        root,
        &[
            "pr",
            "view",
            &n,
            "--repo",
            &repo,
            "--json",
            "state,isDraft,mergeStateStatus,reviewDecision,statusCheckRollup,headRefOid",
        ],
    )?;
    if pr["state"] != "OPEN"
        || pr["isDraft"] != false
        || pr["mergeStateStatus"] != "CLEAN"
        || pr["reviewDecision"] == "CHANGES_REQUESTED"
        || pr["reviewDecision"] == "REVIEW_REQUIRED"
    {
        return Err("GIT_STUDIO_PR_NOT_READY".into());
    }
    let checks = pr["statusCheckRollup"]
        .as_array()
        .ok_or("GIT_STUDIO_CI_MISSING")?;
    if checks.is_empty() || checks.iter().any(|c| !studio_check_passed(c)) {
        return Err("GIT_STUDIO_CI_NOT_PASSED".into());
    }
    let head = pr["headRefOid"]
        .as_str()
        .ok_or("GIT_STUDIO_PR_HEAD_MISSING")?;
    if head.len() != 40 || !head.bytes().all(|x| x.is_ascii_hexdigit()) {
        return Err("GIT_STUDIO_PR_HEAD_INVALID".into());
    }
    if repo == "code5m/browser-os" {
        let runs = gh(
            root,
            &[
                "api",
                &format!(
                    "repos/{repo}/actions/runs?head_sha={head}&event=pull_request&per_page=100"
                ),
            ],
        )?;
        let runs = runs["workflow_runs"]
            .as_array()
            .ok_or("GIT_STUDIO_FULL_VALIDATION_MISSING")?;
        let run = runs
            .iter()
            .find(|r| r["name"] == "BrowserOS Full Validation" && r["head_sha"] == head)
            .ok_or("GIT_STUDIO_FULL_VALIDATION_MISSING")?;
        if run["status"] != "completed" || run["conclusion"] != "success" {
            return Err("GIT_STUDIO_FULL_VALIDATION_NOT_PASSED".into());
        }
        let id = run["id"].as_u64().ok_or("GIT_STUDIO_CI_ID_INVALID")?;
        let jobs = gh(
            root,
            &[
                "api",
                &format!("repos/{repo}/actions/runs/{id}/jobs?per_page=100"),
            ],
        )?;
        let jobs = jobs["jobs"]
            .as_array()
            .ok_or("GIT_STUDIO_PREMERGE_MISSING")?;
        let validate = jobs
            .iter()
            .find(|j| j["name"] == "validate" && j["conclusion"] == "success")
            .ok_or("GIT_STUDIO_PREMERGE_MISSING")?;
        let steps = validate["steps"]
            .as_array()
            .ok_or("GIT_STUDIO_PREMERGE_MISSING")?;
        for name in [
            "Full pre-merge gate",
            "Rust check",
            "Rust tests",
            "Packaged GUI cold-start acceptance",
            "Full Tauri GUI regression",
        ] {
            if !steps.iter().any(|s| {
                s["name"] == name && s["status"] == "completed" && s["conclusion"] == "success"
            }) {
                return Err("GIT_STUDIO_PREMERGE_NOT_EXECUTED".into());
            }
        }
    }
    Ok(head.into())
}
fn read(root: &Path, input: &StudioInput) -> Result<Value, String> {
    let selected = input.paths.first().map(String::as_str);
    if let Some(p) = selected {
        check_file(root, p, true)?;
    }
    match input.action.as_str() {
        "status" => {
            let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
            let head = repo.head().ok();
            let name = head
                .as_ref()
                .and_then(|h| h.shorthand())
                .unwrap_or("(unborn)");
            let branches = git(
                root,
                &[
                    "for-each-ref",
                    "--format=%(refname:short)%09%(upstream:short)%09%(upstream:track)%09%(refname)",
                    "refs/heads",
                    "refs/remotes",
                ],
            )?;
            Ok(
                json!({"root":root,"branch":name,"head":head.as_ref().and_then(|h|h.target()).map(|h|h.to_string()),"state":format!("{:?}",repo.state()),"files":status(root)?,"branches":branches.lines().map(|s|s.split('\t').collect::<Vec<_>>()).collect::<Vec<_>>() }),
            )
        }
        "diff" => {
            let mut args = vec!["diff", "--no-ext-diff", "--no-textconv", "--unified=3"];
            if input.staged.unwrap_or(false) {
                args.push("--cached");
            }
            args.push("--");
            if let Some(p) = selected {
                args.push(p);
            }
            Ok(json!({"patch":git(root,&args)?}))
        }
        "file" => {
            let basis = snapshot(root)?;
            let p = selected.ok_or("GIT_STUDIO_SELECT_FILE")?;
            let file = check_file(root, p, true)?;
            let working = if file.exists() {
                if file.metadata().map_err(|_| "GIT_STUDIO_FILE_READ")?.len() > TEXT_CAP as u64 {
                    return Err("GIT_STUDIO_TEXT_LIMIT".into());
                }
                fs::read_to_string(file).map_err(|_| "GIT_STUDIO_BINARY_FILE")?
            } else {
                String::new()
            };
            let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
            let index = repo.index().map_err(|_| "GIT_STUDIO_INDEX_READ")?;
            let blob_text = |id: git2::Oid| -> Result<String, String> {
                let blob = repo.find_blob(id).map_err(|_| "GIT_STUDIO_BLOB_READ")?;
                if blob.size() > TEXT_CAP || blob.is_binary() {
                    return Err("GIT_STUDIO_TEXT_LIMIT_OR_BINARY".into());
                }
                String::from_utf8(blob.content().to_vec())
                    .map_err(|_| "GIT_STUDIO_BINARY_FILE".into())
            };
            let staged = index
                .get_path(Path::new(p), 0)
                .map(|e| blob_text(e.id))
                .transpose()?
                .unwrap_or_default();
            let head = repo
                .head()
                .ok()
                .and_then(|h| h.peel_to_tree().ok())
                .and_then(|t| t.get_path(Path::new(p)).ok())
                .map(|e| blob_text(e.id()))
                .transpose()?
                .unwrap_or_default();
            Ok(
                json!({"path":p,"head":head,"index":staged,"worktree":working,"fingerprint":if snapshot(root)? == basis { basis } else { return Err("GIT_STUDIO_READ_BASIS_CHANGED".into()); }}),
            )
        }
        "log" | "follow" => {
            let skip = input.offset.unwrap_or(0);
            if skip > 50000 {
                return Err("GIT_STUDIO_HISTORY_LIMIT".into());
            }
            let mut args = vec![
                "log".into(),
                "--topo-order".into(),
                "--max-count=51".into(),
                format!("--skip={skip}"),
                "--format=%H%x1f%P%x1f%an%x1f%at%x1f%s%x1e".into(),
            ];
            if input.action == "follow" {
                if selected.is_none() {
                    return Err("GIT_STUDIO_SELECT_FILE".into());
                }
                args.push("--follow".into());
            } else if input.revision.is_none() {
                // An explicit branch must restrict git log to that ref.
                // --all combined with a revision unions other branch histories.
                args.push("--all".into());
            }
            for (flag, value) in [("--grep", &input.search), ("--author", &input.author)] {
                if let Some(value) = value {
                    if value.len() > 200 || value.chars().any(|c| c.is_control()) {
                        return Err("GIT_STUDIO_FILTER_INVALID".into());
                    }
                    args.push("--fixed-strings".into());
                    args.push(format!("{flag}={value}"));
                }
            }
            if let Some(revision) = &input.revision {
                args.push(oid(root, revision)?);
            }
            args.push("--".into());
            if let Some(p) = selected {
                args.push(p.into());
            }
            let out = process(root, "git", &args, None, None, &AtomicBool::new(false))?;
            if out.code != 0 {
                return Err("GIT_STUDIO_LOG_FAILED".into());
            }
            let mut entries=out.text.split('\x1e').filter(|r|!r.trim().is_empty()).map(|r|{let parts=r.trim_start_matches('\n').splitn(5,'\x1f').collect::<Vec<_>>();if parts.len()!=5{return Err("GIT_STUDIO_LOG_FORMAT".to_string());}Ok(json!({"oid":parts[0],"parents":parts[1].split_whitespace().collect::<Vec<_>>(),"author":parts[2],"time":parts[3].parse::<i64>().unwrap_or(0),"summary":parts[4]}))}).collect::<Result<Vec<_>,String>>()?;
            let more = entries.len() > 50;
            entries.truncate(50);
            Ok(json!({"commits":entries,"has_more":more}))
        }
        "compare" => {
            let before = oid(root, input.revision.as_deref().unwrap_or("HEAD"))?;
            let after = oid(
                root,
                input.other.as_deref().ok_or("GIT_STUDIO_COMPARE_TARGET")?,
            )?;
            Ok(
                json!({"before":before,"after":after,"patch":git(root,&["diff","--no-ext-diff","--no-textconv",&before,&after,"--"])?}),
            )
        }
        "commit_diff" => {
            let revision = oid(
                root,
                input
                    .revision
                    .as_deref()
                    .ok_or("GIT_STUDIO_REVISION_REQUIRED")?,
            )?;
            Ok(
                json!({"patch":git(root,&["show","--format=fuller","--no-ext-diff","--no-textconv",&revision,"--"])?}),
            )
        }
        "commit_files" | "commit_file_diff" | "commit_file_worktree_diff" => {
            let revision = oid(
                root,
                input.revision.as_deref().ok_or("GIT_STUDIO_REVISION_REQUIRED")?,
            )?;
            let lineage = git(root, &["rev-list", "--parents", "-n", "1", &revision])?;
            let parent = lineage.split_whitespace().nth(1);
            if input.action == "commit_files" {
                // Names are NUL-delimited: filenames may contain tabs/newlines.
                // Disable rename coalescing so every record is status + one path.
                let records = if let Some(parent) = parent {
                    git(root, &["diff", "--name-status", "--no-renames", "-z", parent, &revision, "--"])?
                } else {
                    git(root, &["diff-tree", "--root", "--no-commit-id", "--name-status", "-r", "--no-renames", "-z", &revision, "--"])?
                };
                let parts = records.split('\0').filter(|value| !value.is_empty()).collect::<Vec<_>>();
                if parts.len() % 2 != 0 || parts.len() / 2 > FILE_CAP {
                    return Err("GIT_STUDIO_COMMIT_FILES_FORMAT".into());
                }
                let mut files = Vec::new();
                for entry in parts.chunks_exact(2) {
                    safe_path(entry[1])?;
                    files.push(json!({"status":entry[0],"path":entry[1]}));
                }
                Ok(json!({"revision":revision,"files":files,"first_parent":parent}))
            } else {
                let file = selected.ok_or("GIT_STUDIO_SELECT_FILE")?;
                safe_path(file)?;
                let patch = if input.action == "commit_file_worktree_diff" {
                    git(root, &["diff", "--no-ext-diff", "--no-textconv", "--unified=3", &revision, "--", file])?
                } else if let Some(parent) = parent {
                    git(root, &["diff", "--no-ext-diff", "--no-textconv", "--unified=3", parent, &revision, "--", file])?
                } else {
                    git(root, &["show", "--format=", "--no-ext-diff", "--no-textconv", &revision, "--", file])?
                };
                Ok(json!({"revision":revision,"path":file,"patch":patch,"mode":input.action}))
            }
        }

        "conflict" => {
            let path = selected.ok_or("GIT_STUDIO_SELECT_FILE")?;
            let basis = snapshot(root)?;
            let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
            let index = repo.index().map_err(|_| "GIT_STUDIO_INDEX_READ")?;
            let mut versions = Vec::new();
            let mut found = false;
            for stage in [1, 2, 3] {
                if let Some(entry) = index.get_path(Path::new(path), stage) {
                    found = true;
                    let blob = repo
                        .find_blob(entry.id)
                        .map_err(|_| "GIT_STUDIO_BLOB_READ")?;
                    if blob.size() > TEXT_CAP || blob.is_binary() {
                        return Err("GIT_STUDIO_BINARY_FILE".into());
                    }
                    versions.push(
                        String::from_utf8(blob.content().to_vec())
                            .map_err(|_| "GIT_STUDIO_BINARY_FILE")?,
                    );
                } else {
                    versions.push(String::new());
                }
            }
            if !found {
                return Err("GIT_STUDIO_NO_CONFLICT".into());
            }
            if snapshot(root)? != basis {
                return Err("GIT_STUDIO_READ_BASIS_CHANGED".into());
            }
            Ok(
                json!({"path":path,"ancestor":versions[0],"ours":versions[1],"theirs":versions[2],"fingerprint":basis}),
            )
        }
        "blame" => Ok(
            json!({"text":git(root,&["blame","--line-porcelain","--",selected.ok_or("GIT_STUDIO_SELECT_FILE")?])?}),
        ),
        "stash" => Ok(json!({"text":git(root,&["stash","list","--format=%gd%x09%H%x09%s"])?})),
        "stash_diff" => {
            let name = input.target.as_deref().ok_or("GIT_STUDIO_STASH_REQUIRED")?;
            let pinned = stash_id(root, name)?;
            Ok(
                json!({"patch":git(root,&["stash","show","-p","--no-ext-diff","--no-textconv",&pinned,"--"])?}),
            )
        }
        "worktrees" => Ok(json!({"text":git(root,&["worktree","list","--porcelain"])?})),
        "github" => {
            let repo = slug(root)?;
            Ok(
                json!({"repository":repo,"prs":gh(root,&["pr","list","--repo",&repo,"--limit","50","--json","number,title,url,headRefName,baseRefName,isDraft,reviewDecision,mergeStateStatus,statusCheckRollup"] )?,"runs":gh(root,&["run","list","--repo",&repo,"--limit","30","--json","databaseId,workflowName,status,conclusion,url,headSha"])?}),
            )
        }
        "pr_detail" => {
            let repo = slug(root)?;
            let n = input
                .pr
                .filter(|n| *n > 0)
                .ok_or("GIT_STUDIO_PR_INVALID")?
                .to_string();
            gh(root,&["pr","view",&n,"--repo",&repo,"--json","number,title,body,url,files,comments,reviews,statusCheckRollup,headRefOid,baseRefName"] )
        }
        _ => Err("GIT_STUDIO_READ_ACTION_DENIED".into()),
    }
}
fn stash_id(root: &Path, name: &str) -> Result<String, String> {
    if !name.starts_with("stash@{")
        || !name.ends_with('}')
        || !name[7..name.len() - 1].bytes().all(|b| b.is_ascii_digit())
    {
        return Err("GIT_STUDIO_STASH_INVALID".into());
    }
    oid(root, name)
}
fn validate(root: &Path, input: &StudioInput) -> Result<Option<String>, String> {
    let action = input.action.as_str();
    if matches!(action, "index_edit" | "resolve") {
        let expected = input
            .expected_fingerprint
            .as_deref()
            .ok_or("GIT_STUDIO_READ_BASIS_REQUIRED")?;
        if snapshot(root)? != expected {
            return Err("GIT_STUDIO_READ_BASIS_CHANGED".into());
        }
    }
    let target = input.target.as_deref().unwrap_or("");
    if input
        .message
        .as_ref()
        .is_some_and(|x| x.len() > 8000 || x.contains('\0'))
        || input
            .content
            .as_ref()
            .is_some_and(|x| x.len() > TEXT_CAP || x.contains('\0'))
    {
        return Err("GIT_STUDIO_INPUT_LIMIT".into());
    }
    match action {
        "stage" | "unstage" | "discard" | "commit" | "amend" | "index_edit" => {
            for p in paths(input)? {
                check_file(root, &p, true)?;
            }
            if action == "commit" || action == "amend" {
                ensure_scoped_commit_allowed(root)?;
                if input.message.as_deref().unwrap_or("").trim().is_empty() {
                    return Err("GIT_STUDIO_COMMIT_MESSAGE_REQUIRED".into());
                }
                if status(root)?.iter().any(|s| s["conflicted"] == true) {
                    return Err("GIT_STUDIO_UNRESOLVED_CONFLICTS".into());
                }
            }
            if action == "amend"
                && !git(root, &["branch", "-r", "--contains", "HEAD"])?
                    .trim()
                    .is_empty()
            {
                return Err("GIT_STUDIO_PUBLISHED_AMEND_DENIED".into());
            }
            if action == "index_edit" && (input.paths.len() != 1 || input.content.is_none()) {
                return Err("GIT_STUDIO_INDEX_CONTENT_REQUIRED".into());
            }
            if action == "index_edit"
                && status(root)?
                    .iter()
                    .any(|s| s["conflicted"] == true && input.paths.iter().any(|p| s["path"] == *p))
            {
                return Err("GIT_STUDIO_USE_CONFLICT_RESOLVER".into());
            }
            Ok(None)
        }
        "branch_create" | "checkout" | "branch_delete" => {
            is_clean(root)?;
            branch(root, target)?;
            Ok(None)
        }
        "fetch" | "pull" | "push" => {
            origin(root)?;
            if action == "pull" {
                is_clean(root)?;
            }
            if action == "push" {
                let repo =
                    git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
                if repo.head_detached().unwrap_or(true) {
                    return Err("GIT_STUDIO_DETACHED_HEAD".into());
                }
            }
            Ok(None)
        }
        "merge" | "rebase" | "cherry_pick" => {
            is_clean(root)?;
            if action == "rebase"
                && !git(root, &["branch", "-r", "--contains", "HEAD"])?
                    .trim()
                    .is_empty()
            {
                return Err("GIT_STUDIO_PUBLISHED_REBASE_DENIED".into());
            }
            Ok(Some(oid(root, target)?))
        }
        "merge_continue"
        | "merge_abort"
        | "rebase_continue"
        | "rebase_abort"
        | "cherry_pick_continue"
        | "cherry_pick_abort" => {
            let state = git2::Repository::open(root)
                .map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?
                .state();
            if state == git2::RepositoryState::Clean {
                return Err("GIT_STUDIO_NO_OPERATION_TO_CONTINUE".into());
            }
            Ok(None)
        }
        "stash_save" => {
            if status(root)?.is_empty() {
                return Err("GIT_STUDIO_NO_CHANGES".into());
            }
            Ok(None)
        }
        "stash_apply" | "stash_pop" | "stash_drop" => Ok(Some(stash_id(root, target)?)),
        "resolve" => {
            let p = paths(input)?;
            if p.len() != 1 || input.content.is_none() {
                return Err("GIT_STUDIO_RESOLUTION_REQUIRED".into());
            }
            check_file(root, &p[0], false)?;
            if !status(root)?
                .iter()
                .any(|x| x["path"] == p[0] && x["conflicted"] == true)
            {
                return Err("GIT_STUDIO_NO_CONFLICT".into());
            }
            Ok(None)
        }
        "worktree_add" => {
            is_clean(root)?;
            safe_path(target)?;
            if target.contains('/') || root.join(target).exists() {
                return Err("GIT_STUDIO_NEW_CHILD_DIRECTORY_REQUIRED".into());
            }
            Ok(Some(oid(
                root,
                input.revision.as_deref().unwrap_or("HEAD"),
            )?))
        }
        "worktree_remove" => {
            safe_path(target)?;
            let path = check_file(root, target, false)?
                .canonicalize()
                .map_err(|_| "GIT_STUDIO_WORKTREE_MISSING")?;
            if path == root || !path.starts_with(root) {
                return Err("GIT_STUDIO_WORKTREE_REMOVE_DENIED".into());
            }
            is_clean(&path)?;
            let repo = git2::Repository::open(&path).map_err(|_| "GIT_STUDIO_WORKTREE_MISSING")?;
            if !repo.is_worktree() {
                return Err("GIT_STUDIO_NOT_LINKED_WORKTREE".into());
            }
            Ok(None)
        }
        "pr_merge" => Ok(Some(pr_pin(
            root,
            input.pr.ok_or("GIT_STUDIO_PR_INVALID")?,
        )?)),
        "pr_create" | "pr_comment" | "pr_approve" | "pr_request_changes" => {
            slug(root)?;
            if input.message.as_deref().unwrap_or("").trim().is_empty() {
                return Err("GIT_STUDIO_PR_TEXT_REQUIRED".into());
            }
            if action == "pr_create" {
                branch(root, target)?;
            } else if input.pr.unwrap_or(0) == 0 {
                return Err("GIT_STUDIO_PR_INVALID".into());
            }
            Ok(None)
        }
        _ => Err("GIT_STUDIO_WRITE_ACTION_DENIED".into()),
    }
}
fn execute(
    root: &Path,
    input: &StudioInput,
    pin: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Value, String> {
    let target = input.target.as_deref().unwrap_or("");
    let message = input.message.as_deref().unwrap_or("");
    let selected = input.paths.clone();
    let (program, mut args) = match input.action.as_str() {
        "stage" => ("git", vec!["add".into(), "--".into()]),
        "unstage" => (
            "git",
            vec!["restore".into(), "--staged".into(), "--".into()],
        ),
        "discard" => (
            "git",
            vec!["restore".into(), "--worktree".into(), "--".into()],
        ),
        "commit" | "amend" => return commit_selected(root, input, cancel),
        "index_edit" => {
            return write_index(root, &selected[0], input.content.as_deref().unwrap_or(""))
        }
        "branch_create" => ("git", vec!["branch".into(), target.into()]),
        "branch_delete" => ("git", vec!["branch".into(), "-d".into(), target.into()]),
        "checkout" => ("git", vec!["switch".into(), target.into()]),
        "fetch" => ("git", vec!["fetch".into(), "origin".into()]),
        "pull" => ("git", vec!["pull".into(), "--ff-only".into()]),
        "push" => ("git", vec!["push".into(), "origin".into(), "HEAD".into()]),
        "merge" => (
            "git",
            vec![
                "merge".into(),
                "--no-edit".into(),
                pin.ok_or("GIT_STUDIO_PIN_MISSING")?.into(),
            ],
        ),
        "rebase" => (
            "git",
            vec!["rebase".into(), pin.ok_or("GIT_STUDIO_PIN_MISSING")?.into()],
        ),
        "cherry_pick" => (
            "git",
            vec![
                "cherry-pick".into(),
                pin.ok_or("GIT_STUDIO_PIN_MISSING")?.into(),
            ],
        ),
        "merge_continue" => ("git", vec!["merge".into(), "--continue".into()]),
        "merge_abort" => ("git", vec!["merge".into(), "--abort".into()]),
        "rebase_continue" => ("git", vec!["rebase".into(), "--continue".into()]),
        "rebase_abort" => ("git", vec!["rebase".into(), "--abort".into()]),
        "cherry_pick_continue" => ("git", vec!["cherry-pick".into(), "--continue".into()]),
        "cherry_pick_abort" => ("git", vec!["cherry-pick".into(), "--abort".into()]),
        "stash_save" => (
            "git",
            vec![
                "stash".into(),
                "push".into(),
                "--include-untracked".into(),
                "-m".into(),
                if message.is_empty() {
                    "BrowserOS stash".into()
                } else {
                    message.into()
                },
            ],
        ),
        "stash_apply" => (
            "git",
            vec![
                "stash".into(),
                "apply".into(),
                pin.ok_or("GIT_STUDIO_PIN_MISSING")?.into(),
            ],
        ),
        "stash_pop" => ("git", vec!["stash".into(), "pop".into(), target.into()]),
        "stash_drop" => ("git", vec!["stash".into(), "drop".into(), target.into()]),
        "resolve" => {
            let path = check_file(root, &selected[0], false)?;
            let temp =
                path.with_file_name(format!(".browseros-resolution-{}", uuid::Uuid::new_v4()));
            let permissions = path
                .metadata()
                .map_err(|_| "GIT_STUDIO_FILE_READ")?
                .permissions();
            fs::write(&temp, input.content.as_deref().unwrap_or(""))
                .map_err(|_| "GIT_STUDIO_RESOLUTION_WRITE")?;
            fs::set_permissions(&temp, permissions).map_err(|_| "GIT_STUDIO_RESOLUTION_WRITE")?;
            if fs::rename(&temp, &path).is_err() {
                let _ = fs::remove_file(&temp);
                return Err("GIT_STUDIO_RESOLUTION_WRITE".into());
            }
            ("git", vec!["add".into(), "--".into()])
        }
        "worktree_add" => (
            "git",
            vec![
                "worktree".into(),
                "add".into(),
                "--detach".into(),
                root.join(target).to_string_lossy().into_owned(),
                pin.ok_or("GIT_STUDIO_PIN_MISSING")?.into(),
            ],
        ),
        "worktree_remove" => (
            "git",
            vec![
                "worktree".into(),
                "remove".into(),
                root.join(target).to_string_lossy().into_owned(),
            ],
        ),
        "pr_merge" => {
            let current = pr_pin(root, input.pr.unwrap_or(0))?;
            if Some(current.as_str()) != pin {
                return Err("GIT_STUDIO_PR_CHANGED".into());
            }
            (
                "gh",
                vec![
                    "pr".into(),
                    "merge".into(),
                    input.pr.unwrap_or(0).to_string(),
                    "--repo".into(),
                    slug(root)?,
                    "--merge".into(),
                    "--match-head-commit".into(),
                    current,
                ],
            )
        }
        "pr_create" => (
            "gh",
            vec![
                "pr".into(),
                "create".into(),
                "--repo".into(),
                slug(root)?,
                "--base".into(),
                target.into(),
                "--title".into(),
                message
                    .lines()
                    .next()
                    .unwrap_or("BrowserOS change")
                    .chars()
                    .take(200)
                    .collect(),
                "--body".into(),
                message.into(),
            ],
        ),
        "pr_comment" => (
            "gh",
            vec![
                "pr".into(),
                "comment".into(),
                input.pr.unwrap_or(0).to_string(),
                "--repo".into(),
                slug(root)?,
                "--body".into(),
                message.into(),
            ],
        ),
        "pr_approve" | "pr_request_changes" => (
            "gh",
            vec![
                "pr".into(),
                "review".into(),
                input.pr.unwrap_or(0).to_string(),
                "--repo".into(),
                slug(root)?,
                if input.action == "pr_approve" {
                    "--approve".into()
                } else {
                    "--request-changes".into()
                },
                "--body".into(),
                message.into(),
            ],
        ),
        _ => return Err("GIT_STUDIO_WRITE_ACTION_DENIED".into()),
    };
    if ["stage", "unstage", "discard", "resolve"].contains(&input.action.as_str()) {
        args.extend(selected);
    }
    let output = process(root, program, &args, None, None, cancel)?;
    let state = git2::Repository::open(root)
        .map(|r| format!("{:?}", r.state()))
        .unwrap_or_else(|_| "Unknown".into());
    Ok(
        json!({"success":output.code==0,"exit_code":output.code,"state":state,"action":input.action,"recovery_required":output.code!=0,"message":if output.code==0{"GIT_STUDIO_COMPLETED"}else{"GIT_STUDIO_FAILED_INSPECT_STATE"}}),
    )
}
fn write_index(root: &Path, path: &str, contents: &str) -> Result<Value, String> {
    let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
    let mut index = repo.index().map_err(|_| "GIT_STUDIO_INDEX_READ")?;
    if index.get_path(Path::new(path), 0).is_none() {
        index
            .add_path(Path::new(path))
            .map_err(|_| "GIT_STUDIO_INDEX_FILE_REQUIRED")?;
    }
    let mut entry = index
        .get_path(Path::new(path), 0)
        .ok_or("GIT_STUDIO_INDEX_FILE_REQUIRED")?;
    entry.id = repo
        .blob(contents.as_bytes())
        .map_err(|_| "GIT_STUDIO_BLOB_WRITE")?;
    entry.file_size = contents.len() as u32;
    index.add(&entry).map_err(|_| "GIT_STUDIO_INDEX_WRITE")?;
    index.write().map_err(|_| "GIT_STUDIO_INDEX_WRITE")?;
    Ok(json!({"success":true,"action":"index_edit","message":"GIT_STUDIO_INDEX_UPDATED"}))
}
fn ensure_scoped_commit_allowed(root: &Path) -> Result<(), String> {
    let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
    if repo.state() != git2::RepositoryState::Clean {
        return Err("GIT_STUDIO_OPERATION_IN_PROGRESS_USE_CONTINUE".into());
    }
    Ok(())
}
fn commit_selected(root: &Path, input: &StudioInput, cancel: &AtomicBool) -> Result<Value, String> {
    // Recheck at the actual write boundary, not only at preview time. A merge
    // must commit its complete real index, never a HEAD-based partial index.
    ensure_scoped_commit_allowed(root)?;
    let mut selected = paths(input)?;
    let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
    let source = repo.index().map_err(|_| "GIT_STUDIO_INDEX_READ")?;
    let temp_path = repo
        .path()
        .join(format!("browseros-index-{}", uuid::Uuid::new_v4()));
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
            let _ = fs::remove_file(self.0.with_extension("lock"));
        }
    }
    let _cleanup = Cleanup(temp_path.clone());
    let mut temp = git2::Index::open(&temp_path).map_err(|_| "GIT_STUDIO_TEMP_INDEX")?;
    if let Ok(head) = repo.head() {
        let tree = head.peel_to_tree().map_err(|_| "GIT_STUDIO_HEAD_TREE")?;
        temp.read_tree(&tree).map_err(|_| "GIT_STUDIO_TEMP_INDEX")?;
    }
    let changes = status(root)?;
    for change in &changes {
        if selected.iter().any(|p| change["path"] == *p) {
            if let Some(original) = change["original"].as_str() {
                safe_path(original)?;
                if !selected.iter().any(|p| p == original) {
                    selected.push(original.into());
                }
            }
        }
    }
    let staged = changes
        .into_iter()
        .filter(|s| s["staged"] == true)
        .flat_map(|s| {
            [
                s["path"].as_str().map(str::to_owned),
                s["original"].as_str().map(str::to_owned),
            ]
            .into_iter()
            .flatten()
        })
        .collect::<HashSet<_>>();
    for path in &selected {
        if !staged.contains(path) {
            return Err("GIT_STUDIO_SELECT_STAGED_FILES".into());
        }
        if let Some(entry) = source.get_path(Path::new(path), 0) {
            temp.add(&entry).map_err(|_| "GIT_STUDIO_TEMP_INDEX")?;
        } else {
            let _ = temp.remove_path(Path::new(path));
        }
    }
    temp.write().map_err(|_| "GIT_STUDIO_TEMP_INDEX")?;
    let mut args = vec![
        "commit".into(),
        "-m".into(),
        input.message.clone().unwrap_or_default(),
    ];
    if input.action == "amend" {
        args.push("--amend".into());
    }
    let result = process(root, "git", &args, None, Some(&temp_path), cancel)?;
    if result.code != 0 {
        return Ok(
            json!({"success":false,"action":input.action,"message":"GIT_STUDIO_COMMIT_OR_HOOK_REJECTED"}),
        );
    }
    // Preserve unrelated staged changes; never reset the working tree.
    let mut args = vec![
        "restore".into(),
        "--staged".into(),
        "--source=HEAD".into(),
        "--".into(),
    ];
    args.extend(selected);
    let reconciled = process(root, "git", &args, None, None, &AtomicBool::new(false))?;
    Ok(
        json!({"success":reconciled.code==0,"commit_created":true,"head":oid(root,"HEAD")?,"recovery_required":reconciled.code!=0,"message":if reconciled.code==0{"GIT_STUDIO_COMMITTED"}else{"GIT_STUDIO_COMMIT_CREATED_INDEX_REPAIR_REQUIRED"}}),
    )
}
#[tauri::command]
pub async fn git_studio_read(
    app: AppHandle,
    webview: Webview,
    source: StudioSource,
    input: StudioInput,
) -> Result<Value, String> {
    check_invocation_source(&webview, "git_studio_read", None, &app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let root = resolve(&app, &source)?;
        read(&root, &input)
    })
    .await
    .map_err(|_| "GIT_STUDIO_WORKER_FAILED".to_string())?
}
#[tauri::command]
pub async fn git_studio_preview(
    app: AppHandle,
    webview: Webview,
    source: StudioSource,
    input: StudioInput,
) -> Result<Value, String> {
    check_invocation_source(&webview, "git_studio_preview", None, &app)?;
    tauri::async_runtime::spawn_blocking(move||{
        let root=resolve(&app,&source)?;let runtime=app.state::<GitStudioRuntime>();if runtime.active.lock().map_err(|_|"GIT_STUDIO_LOCK_FAILED")?.contains_key(&root){return Err("GIT_STUDIO_REPOSITORY_BUSY".into());}
        let remote_pin=validate(&root,&input)?;let fingerprint=snapshot(&root)?;let id=uuid::Uuid::new_v4().to_string();let mut tickets=runtime.tickets.lock().map_err(|_|"GIT_STUDIO_LOCK_FAILED")?;tickets.retain(|_,t|t.expires>Instant::now());if tickets.len()>=32{return Err("GIT_STUDIO_PREVIEW_LIMIT".into());}
        let reply=json!({"job_id":id,"root":root,"action":input.action,"paths":input.paths,"target":input.target,"message":input.message,"expires_in_seconds":300,"requires_confirmation":true,"scope":if input.action=="stash_save"{"whole_worktree_including_untracked"}else{"explicit_operation"}});
        tickets.insert(id,Ticket{source,input,fingerprint,root,expires:Instant::now()+Duration::from_secs(300),remote_pin});Ok(reply)
    }).await.map_err(|_|"GIT_STUDIO_WORKER_FAILED".to_string())?
}
#[tauri::command]
pub async fn git_studio_confirm(
    app: AppHandle,
    webview: Webview,
    job_id: String,
    confirmed: bool,
) -> Result<Value, String> {
    check_invocation_source(&webview, "git_studio_confirm", None, &app)?;
    if !confirmed {
        return Err("GIT_STUDIO_CONFIRMATION_REQUIRED".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let runtime = app.state::<GitStudioRuntime>();
        let ticket = runtime
            .tickets
            .lock()
            .map_err(|_| "GIT_STUDIO_LOCK_FAILED")?
            .remove(&job_id)
            .ok_or("GIT_STUDIO_PREVIEW_MISSING")?;
        if ticket.expires <= Instant::now() {
            return Err("GIT_STUDIO_PREVIEW_EXPIRED".into());
        }
        let root = resolve(&app, &ticket.source)?;
        if root != ticket.root {
            return Err("GIT_STUDIO_SOURCE_CHANGED".into());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut active = runtime
                .active
                .lock()
                .map_err(|_| "GIT_STUDIO_LOCK_FAILED")?;
            if active.contains_key(&root) {
                return Err("GIT_STUDIO_REPOSITORY_BUSY".into());
            }
            active.insert(root.clone(), cancel.clone());
        }
        let _active = ActiveGuard {
            runtime: &runtime,
            root: root.clone(),
        };
        if snapshot(&root)? != ticket.fingerprint {
            return Err("GIT_STUDIO_CHANGED_SINCE_PREVIEW".into());
        }
        let current_pin = validate(&root, &ticket.input)?;
        if current_pin != ticket.remote_pin {
            return Err("GIT_STUDIO_TARGET_CHANGED".into());
        }
        let result = execute(&root, &ticket.input, ticket.remote_pin.as_deref(), &cancel);
        crate::workspace::log_audit(
            &app,
            "git.studio.write",
            format!(
                "action={} success={}",
                ticket.input.action,
                result.as_ref().is_ok_and(|v| v["success"] == true)
            ),
        );
        result
    })
    .await
    .map_err(|_| "GIT_STUDIO_WORKER_FAILED".to_string())?
}
#[tauri::command]
pub fn git_studio_cancel(app: AppHandle, webview: Webview, job_id: String) -> Result<bool, String> {
    check_invocation_source(&webview, "git_studio_cancel", None, &app)?;
    let runtime = app.state::<GitStudioRuntime>();
    let removed = runtime
        .tickets
        .lock()
        .map_err(|_| "GIT_STUDIO_LOCK_FAILED")?
        .remove(&job_id)
        .is_some();
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn studio_check_states_fail_closed() {
        for x in [
            json!({}),
            json!({"state":"FAILURE"}),
            json!({"conclusion":"SUCCESS"}),
            json!({"status":"COMPLETED","conclusion":"SKIPPED"}),
            json!({"status":"IN_PROGRESS","conclusion":"SUCCESS"}),
        ] {
            assert!(!studio_check_passed(&x));
        }
        assert!(studio_check_passed(&json!({"state":"SUCCESS"})));
        assert!(studio_check_passed(
            &json!({"status":"COMPLETED","conclusion":"SUCCESS"})
        ));
    }
    #[test]
    fn studio_path_rejects_escape_and_git_internal() {
        for p in [
            "",
            "/tmp/file",
            "../file",
            "src/../file",
            ".git/config",
            "src/.git/config",
            "src\\x",
            "x\nfile",
        ] {
            assert!(safe_path(p).is_err(), "{p}");
        }
        assert!(safe_path("src/a b.txt").is_ok());
    }
    fn fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!("browseros-studio-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        git(&path, &["init"]).unwrap();
        git(&path, &["config", "user.name", "Fixture"]).unwrap();
        git(&path, &["config", "user.email", "fixture@example.invalid"]).unwrap();
        fs::write(path.join("a.txt"), "one\n").unwrap();
        fs::write(path.join("b.txt"), "two\n").unwrap();
        git(&path, &["add", "--", "a.txt", "b.txt"]).unwrap();
        git(&path, &["commit", "-m", "initial"]).unwrap();
        path
    }
    #[test]
    fn studio_snapshot_observes_unstaged_bytes() {
        let root = fixture();
        let a = snapshot(&root).unwrap();
        fs::write(root.join("a.txt"), "changed\n").unwrap();
        let b = snapshot(&root).unwrap();
        assert_ne!(a, b);
        fs::write(root.join("a.txt"), "changed again\n").unwrap();
        assert_ne!(b, snapshot(&root).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
    fn assert_in_progress_commit_is_safe(operation: &str, continuation: &str, parents: usize) {
        let root = fixture();
        git(&root, &["branch", "-M", "base"]).unwrap();
        git(&root, &["switch", "-c", "incoming"]).unwrap();
        fs::write(root.join("a.txt"), "incoming a\n").unwrap();
        fs::write(root.join("b.txt"), "incoming b\n").unwrap();
        git(&root, &["add", "--", "a.txt", "b.txt"]).unwrap();
        git(&root, &["commit", "-m", "incoming changes"]).unwrap();
        git(&root, &["switch", "base"]).unwrap();
        fs::write(root.join("a.txt"), "base a\n").unwrap();
        git(&root, &["add", "--", "a.txt"]).unwrap();
        git(&root, &["commit", "-m", "base changes"]).unwrap();
        assert!(git(&root, &[operation, "incoming"]).is_err());
        fs::write(root.join("a.txt"), "resolved a\n").unwrap();
        git(&root, &["add", "--", "a.txt"]).unwrap();
        let before = snapshot(&root).unwrap();
        for action in ["commit", "amend"] {
            let input = StudioInput {
                action: action.into(),
                paths: vec!["a.txt".into()],
                message: Some("partial denied".into()),
                ..Default::default()
            };
            assert_eq!(
                validate(&root, &input).unwrap_err(),
                "GIT_STUDIO_OPERATION_IN_PROGRESS_USE_CONTINUE"
            );
            assert_eq!(
                commit_selected(&root, &input, &AtomicBool::new(false)).unwrap_err(),
                "GIT_STUDIO_OPERATION_IN_PROGRESS_USE_CONTINUE"
            );
            assert_eq!(snapshot(&root).unwrap(), before);
        }
        let input = StudioInput {
            action: continuation.into(),
            ..Default::default()
        };
        validate(&root, &input).unwrap();
        assert_eq!(
            execute(&root, &input, None, &AtomicBool::new(false)).unwrap()["success"],
            true
        );
        assert_eq!(git(&root, &["show", "HEAD:b.txt"]).unwrap(), "incoming b\n");
        assert_eq!(git(&root, &["show", "HEAD:a.txt"]).unwrap(), "resolved a\n");
        assert_eq!(
            git(&root, &["show", "-s", "--format=%P", "HEAD"])
                .unwrap()
                .split_whitespace()
                .count(),
            parents
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn studio_merge_rejects_partial_commit_and_keeps_complete_tree() {
        assert_in_progress_commit_is_safe("merge", "merge_continue", 2);
    }
    #[test]
    fn studio_rebase_rejects_partial_commit_and_keeps_complete_tree() {
        assert_in_progress_commit_is_safe("rebase", "rebase_continue", 1);
    }
    #[test]
    fn studio_cherry_pick_rejects_partial_commit_and_keeps_complete_tree() {
        assert_in_progress_commit_is_safe("cherry-pick", "cherry_pick_continue", 1);
    }
    #[test]
    fn studio_scoped_commit_preserves_unselected_index() {
        let root = fixture();
        fs::write(root.join("a.txt"), "new a\n").unwrap();
        fs::write(root.join("b.txt"), "new b\n").unwrap();
        git(&root, &["add", "--", "a.txt", "b.txt"]).unwrap();
        let input = StudioInput {
            action: "commit".into(),
            paths: vec!["a.txt".into()],
            message: Some("only a".into()),
            ..Default::default()
        };
        let out = commit_selected(&root, &input, &AtomicBool::new(false)).unwrap();
        assert_eq!(out["success"], true);
        assert_eq!(git(&root, &["show", "HEAD:b.txt"]).unwrap(), "two\n");
        assert_eq!(git(&root, &["show", ":b.txt"]).unwrap(), "new b\n");
        assert_eq!(status(&root).unwrap().len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
