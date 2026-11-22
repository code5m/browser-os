use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::Utc;
#[path = "audit_lock.rs"]
mod audit_lock;
use tauri::{AppHandle, Manager};

use crate::domain::{
    Artifact, AuditEntry, Bookmark, CommandSnippet, ImageRef, RepoConfig, ScriptMeta,
};

/// 应用数据根目录（`.../<app_data_dir>/mvp-browser-os`）。
///
/// M4-6 起对 `tasks.rs` 开放：任务持久化 `tasks.json` / `task-runs.json` 与既有
/// `scripts.json` / `snippets.json` 必须落在**同一**数据目录（A1 实测项 11：全仓
/// 持久化均为该目录下的 JSON，不得另起第二条路径）。
struct PreparedDataDirectory {
    path: PathBuf,
    #[cfg(target_os = "windows")]
    _preparation: browseros_platform_security::windows::PreparedPrivateDirectory,
}

/// Complete the fallible private-directory migration before starting any
/// workspace reader, scheduler, browser window or IPC command handler.
/// A failed preparation never publishes the path to application state.
pub fn prepare_data_dir(app: &AppHandle) -> std::io::Result<()> {
    if app.try_state::<PreparedDataDirectory>().is_some() {
        return Ok(());
    }
    let base = app.path()
        .app_data_dir()
        .map_err(|_| std::io::Error::other("PRIVATE_DATA_DIR_LOCATION_UNAVAILABLE"))?;
    #[cfg(target_os = "windows")]
    browseros_platform_security::windows::ensure_private_directory_root(
        &base,
        &browseros_platform_security::windows::PrivateTreeOptions::default(),
    ).map_err(|error| std::io::Error::new(
        error.kind(),
        format!("PRIVATE_DATA_DIR_PARENT_PREPARATION_FAILED: {error}"),
    ))?;
    #[cfg(target_os = "windows")]
    let preparation = browseros_platform_security::windows::PreparedPrivateDirectory::prepare(
        &base.join("mvp-browser-os"),
        &browseros_platform_security::windows::PrivateTreeOptions::default(),
    ).map_err(|error| std::io::Error::new(
        error.kind(), format!("PRIVATE_DATA_DIR_PREPARATION_FAILED: {error}"),
    ))?;
    #[cfg(target_os = "windows")]
    let dir = {
        let report = preparation.report();
        eprintln!("PRIVATE_DATA_DIR_READY reused={} checked={} repaired={} elapsed_ms={}",
            preparation.reused_preparation(), report.checked_objects, report.repaired_objects, report.elapsed_ms);
        preparation.path().to_path_buf()
    };
    #[cfg(not(target_os = "windows"))]
    let dir = crate::capabilities::workspace::safe_paths::prepare_application_data_directory(&base)
        .map_err(|error| std::io::Error::new(
            error.kind(), format!("PRIVATE_DATA_DIR_PREPARATION_FAILED: {error}"),
        ))?;
    if !app.manage(PreparedDataDirectory {
        path: dir,
        #[cfg(target_os = "windows")]
        _preparation: preparation,
    }) {
        return Err(std::io::Error::other("PRIVATE_DATA_DIR_ALREADY_INITIALIZED"));
    }
    Ok(())
}

/// Existing internal callers retain their path API. The only initialization
/// entry is prepare_data_dir in both main and Grid-child setup, before windows
/// or background workers exist. This getter performs no filesystem mutation.
pub fn data_dir(app: &AppHandle) -> PathBuf {
    app.state::<PreparedDataDirectory>().path.clone()
}

fn ensure(dir: &PathBuf) {
    fs::create_dir_all(dir).ok();
}

// ---------------------------------------------------------------------------
// BUG-HUNT B4-1/B4-2：workspace 系**共享落盘 / 读取原语**
//
// B4-1（破坏性覆盖写）：改前 artifacts / repos / bookmarks / audit 用 `fs::write`
//   直接覆盖目标文件，崩溃会留下半截内容；
// B4-2（静默清空）：加载侧 `unwrap_or_default()` 把「解析失败」当成空列表返回，
//   于是「崩溃损坏 → 下次保存把空列表覆盖写回 → 用户数据无痕蒸发」（audit 是热路径，最致命）。
//
// 契约：
//   - 落盘一律走**唯一**原子写原语 `crate::session::atomic_write`（tmp + rename），
//     不新造第二条写路径、不 `fs::write` 直接覆盖目标文件；
//   - 仅文件不存在时返回空列表；无法读取或超过容量时返回错误；
//   - 解析失败保留原文件，并按内容哈希保存私有备份后返回错误；
//     错误绝不成为后续读改写的空基线，重复读取也不会重复占用备份空间。
// ---------------------------------------------------------------------------

/// 列表落盘（原子写：tmp + rename）。
pub fn save_json_list_at<T: serde::Serialize>(
    path: &Path,
    list: &[T],
    tag: &str,
) -> Result<(), String> {
    let content =
        serde_json::to_string_pretty(list).map_err(|e| format!("序列化{tag}失败: {e}"))?;
    crate::session::atomic_write(path, &content)
}

/// A missing store is empty; an unreadable or corrupt store is never a write baseline.
const MAX_LIST_STORE_BYTES: u64 = 32 * 1024 * 1024;

fn corrupt_store_backup_path(path: &Path, bytes: &[u8]) -> PathBuf {
    use sha2::{Digest, Sha256};
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".corrupt-{:x}", Sha256::digest(bytes)));
    PathBuf::from(name)
}

fn preserve_corrupt_store(path: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    use std::io::Read;
    let backup = corrupt_store_backup_path(path, bytes);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    match options.open(&backup) {
        Ok(mut file) => {
            if file.write_all(bytes).and_then(|_| file.sync_all()).is_err() {
                let _ = fs::remove_file(&backup);
                return Err("WORKSPACE_STORE_BACKUP_FAILED".into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(&backup)
                .map_err(|_| "WORKSPACE_STORE_BACKUP_FAILED")?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("WORKSPACE_STORE_BACKUP_FAILED".into());
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err("WORKSPACE_STORE_BACKUP_FAILED".into());
                }
            }
            let mut existing = Vec::new();
            fs::File::open(&backup).map_err(|_| "WORKSPACE_STORE_BACKUP_FAILED")?
                .take(MAX_LIST_STORE_BYTES + 1).read_to_end(&mut existing)
                .map_err(|_| "WORKSPACE_STORE_BACKUP_FAILED")?;
            if existing != bytes { return Err("WORKSPACE_STORE_BACKUP_FAILED".into()); }
        }
        Err(_) => return Err("WORKSPACE_STORE_BACKUP_FAILED".into()),
    }
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        fs::File::open(parent).and_then(|dir| dir.sync_all())
            .map_err(|_| "WORKSPACE_STORE_BACKUP_FAILED")?;
    }
    Ok(backup)
}

pub fn load_json_list_at<T: serde::de::DeserializeOwned>(
    path: &Path,
    tag: &str,
) -> Result<Vec<T>, String> {
    use std::io::Read;
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(format!("WORKSPACE_STORE_READ_FAILED: {tag}")),
    };
    let mut bytes = Vec::new();
    file.take(MAX_LIST_STORE_BYTES + 1).read_to_end(&mut bytes)
        .map_err(|_| format!("WORKSPACE_STORE_READ_FAILED: {tag}"))?;
    if bytes.len() as u64 > MAX_LIST_STORE_BYTES {
        return Err(format!("WORKSPACE_STORE_SIZE_LIMIT: {tag}"));
    }
    match serde_json::from_slice::<Vec<T>>(&bytes) {
        Ok(list) => Ok(list),
        Err(_) => {
            // Copy the original bytes without moving or replacing the live store.
            // A content-addressed, create-new backup preserves distinct failures
            // while repeated reads of the same bad file do not consume more disk.
            let backup = preserve_corrupt_store(path, &bytes)
                .map_err(|_| format!("WORKSPACE_STORE_CORRUPT_BACKUP_FAILED: {tag}"))?;
            Err(format!("WORKSPACE_STORE_CORRUPT: {tag}; backup={}", backup.display()))
        }
    }
}

/// 本地成果库目录
pub fn workspace_dir(app: &AppHandle) -> PathBuf {
    let d = data_dir(app).join("workspace");
    ensure(&d);
    d
}

/// 图片目录基准（M2-2.b 预览通道）。
///
/// 与 `images::write_image_file` 写出的相对路径 `images/<artifact_id>/<image_id>.<ext>`
/// 同源：前端拿本函数的返回值与 `ImageRef.rel_path` 拼接，再经 `convertFileSrc`
/// 转成 `asset://`。与落盘同一处推导，避免前后端各拼一套导致路径漂移。
pub fn images_dir(app: &AppHandle) -> PathBuf {
    let d = workspace_dir(app).join(crate::images::IMAGES_DIR_NAME);
    ensure(&d);
    d
}

/// 默认笔记目录（网页选区一键存 Markdown）。
/// 固定为 ~/Documents/极智笔记（用户可见的特定目录），取不到主目录时退回应用数据目录。
pub fn notes_dir(app: &AppHandle) -> PathBuf {
    let d = app
        .path()
        .home_dir()
        .map(|h| h.join("Documents").join("极智笔记"))
        .unwrap_or_else(|_| data_dir(app).join("notes"));
    ensure(&d);
    d
}

/// M0-3.c：写/删类命令的允许根目录。
///
/// 取值与 `get_start_dirs` 对外承诺的入口保持一致（主目录 / 桌面 / 文档 / 下载 /
/// 成果工作区 / 笔记目录），否则文件管理器会出现「能列出来却写不进去」的不一致。
/// 效果是：仍可在这些用户目录内正常增删改名，但 `../` 逃逸、符号链接逃逸、
/// 以及写到 `/etc`、`/usr`、其他用户目录等均被拒绝。
///
/// 从 `bridge.rs` 迁入本 workspace 能力模块（Native Physical Boundary 分解）：
/// 本函数依赖 `workspace_dir` / `notes_dir`（同模块），故归属 workspace 而非共享层，
/// 避免 shared → workspace 反向依赖。
pub fn allowed_roots(app: &AppHandle) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(home) = app.path().home_dir() {
        roots.push(home.clone());
        for sub in ["Desktop", "Documents", "Downloads"] {
            roots.push(home.join(sub));
        }
    }
    roots.push(workspace_dir(app));
    roots.push(notes_dir(app));
    roots.sort();
    roots.dedup();
    roots
}

pub fn save_artifact(app: &AppHandle, art: &Artifact) -> Result<PathBuf, String> {
    let dir = workspace_dir(app);
    let file = dir.join(format!("{}.json", art.id));
    // B4-1：原子写（tmp + rename），不再 `fs::write` 直接覆盖（崩溃会留半截文件）。
    let content = serde_json::to_string_pretty(art).map_err(|e| e.to_string())?;
    crate::session::atomic_write(&file, &content)?;
    Ok(file)
}

pub fn load_artifacts(app: &AppHandle) -> Vec<Artifact> {
    let dir = workspace_dir(app);
    let mut out = vec![];
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            // M2-1：只解析 `.json`。图片等附件一律在子目录（images/），
            // 且这里显式过滤，避免任何非成果文件被当成 JSON 试解析（R4）。
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if let Ok(c) = fs::read_to_string(&path) {
                match serde_json::from_str::<Artifact>(&c) {
                    Ok(a) => out.push(a),
                    // B4-2：损坏的成果文件**不静默丢弃**——备份为 .corrupt 并告警，
                    // 便于人工取回（成果逐文件保存，不会因空列表覆盖而整体蒸发）。
                    Err(e) => {
                        eprintln!(
                            "[workspace] 成果文件损坏，已备份为 .corrupt：{}（{e}）",
                            path.display()
                        );
                        let mut backup = path.as_os_str().to_os_string();
                        backup.push(".corrupt");
                        let _ = fs::rename(&path, PathBuf::from(backup));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}

pub fn delete_artifact(app: &AppHandle, id: &str) -> Result<(), String> {
    let file = workspace_dir(app).join(format!("{}.json", id));
    if file.exists() {
        fs::remove_file(&file).map_err(|e| e.to_string())?;
    }
    // M2-1：删除联动——成果删了，其图片目录不得变成孤儿（R5）。
    // 删图失败不阻断删 JSON，但要留审计条目，便于人工兜底。
    match delete_artifact_images(app, id) {
        Ok(0) => {}
        Ok(n) => log_audit(app, "image.prune", format!("artifact_id={id} removed={n}")),
        Err(e) => log_audit(
            app,
            "image.prune_failed",
            format!("artifact_id={id} error={e}"),
        ),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// M2-1 图片存储（契约见 `logs/assist/M2-1.a-prework-20260902-1055.md` §4.5）
//
// 目录布局（图片必须放子目录，避免被 load_artifacts 误当成果解析）：
//   workspace_dir/
//   ├── <artifact_id>.json
//   └── images/<artifact_id>/<image_id>.<ext>      # ext 由 MIME 反查
// ---------------------------------------------------------------------------

/// 保存一张图片到成果（**先校验后落盘**：超限/逃逸用例不会留下任何字节）。
///
/// 顺序即契约：id 形态 → 字节数预检 → 成果存在性 → MIME/魔法字节/尺寸
/// → 去重（同 sha256 复用，不重复写盘）→ 配额 → 路径白名单 + 前缀校验
/// → 原子写（tmp + rename）→ 回写成果 JSON。
pub fn save_image(
    app: &AppHandle,
    artifact_id: &str,
    bytes: &[u8],
    mime: &str,
    source_url: Option<&str>,
    caption: Option<&str>,
) -> Result<ImageRef, String> {
    // 1) 拼路径前先卡 id 形态（防 ../ 与绝对路径）
    crate::images::validate_id(artifact_id).map_err(|e| e.to_string())?;
    // 2) 字节数预检（不分配、不写盘）
    crate::images::check_size(bytes.len()).map_err(|e| e.to_string())?;

    let mut arts = load_artifacts(app);
    let art = arts
        .iter_mut()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| "成果不存在".to_string())?;

    // 3) 内容校验（MIME 白名单 + 魔法字节 + 最长边）
    let prepared = crate::images::prepare(bytes, mime).map_err(|e| e.to_string())?;
    // 4) 去重：内容相同直接复用引用，磁盘不新增文件
    if let Some(hit) = crate::images::find_duplicate(&art.images, &prepared.sha256) {
        return Ok(hit.clone());
    }
    // 5) 配额（数量 + 总字节），在写盘前判定
    crate::images::check_quota(&art.images, prepared.bytes).map_err(|e| e.to_string())?;

    // 6) 原子落盘（纯函数：canonicalize + 前缀校验 + tmp/rename 在 images.rs 内，
    //    有真实磁盘单测覆盖）
    let image_id = uuid::Uuid::new_v4().to_string();
    let rel = crate::images::write_image_file(
        &workspace_dir(app),
        artifact_id,
        &image_id,
        prepared.ext,
        bytes,
    )
    .map_err(|e| e.to_string())?;

    // 7) 回写成果 JSON（只存引用，不存字节）
    let reference = crate::images::build_file_ref(&prepared, &rel, source_url, caption)
        .map_err(|e| e.to_string())?;
    art.images.push(reference.clone());
    save_artifact(app, art)?;
    Ok(reference)
}

/// 删除某成果的全部图片目录（幂等：不存在返回 0）。
/// 删除前同样做 canonicalize + 前缀校验，防止误删 images_root 之外的目录。
pub fn delete_artifact_images(app: &AppHandle, artifact_id: &str) -> Result<usize, String> {
    crate::images::remove_artifact_images(&workspace_dir(app), artifact_id)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// M2-3 脚本库持久化（契约见 logs/checkpoints/M2-3.a-20260903-1604.md §2）
//
//   元数据：data_dir/scripts.json（**原子写**）
//   正文：  data_dir/scripts/<id>.<ext>（原子写，独立文件，不内嵌 JSON）
//
// 与 M2-1 `images::write_image_file` 同口径：核心实现**显式收路径参数**
// （不依赖 AppHandle），便于在临时目录里做**真实读写**单测；带 `app` 的版本
// 只是薄包装。既有的 `save_repos` / `save_bookmarks` 是非原子写（既有技术债，
// 本卡不倒改），新增的脚本持久化一律用原子写。
// ---------------------------------------------------------------------------

/// 脚本正文目录
pub fn scripts_dir(app: &AppHandle) -> PathBuf {
    let d = data_dir(app).join("scripts");
    ensure(&d);
    d
}

/// 脚本元数据文件
pub fn scripts_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("scripts.json")
}

/// 脚本运行历史文件（M2-4.d）：只存已截断尾存，不存完整输出。
pub fn script_runs_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("script-runs.json")
}

/// 元数据落盘（原子写：tmp + rename）。B4-1：走共享原语，不新造写路径。
pub fn save_scripts_at(path: &Path, list: &[ScriptMeta]) -> Result<(), String> {
    save_json_list_at(path, list, "脚本库")
}

/// 脚本库读取。读取失败返回错误，损坏原件与按内容命名的备份均保留。
pub fn load_scripts_at(path: &Path) -> Result<Vec<ScriptMeta>, String> {
    load_json_list_at(path, "脚本库")
}

pub fn save_scripts(app: &AppHandle, list: &[ScriptMeta]) -> Result<(), String> {
    save_scripts_at(&scripts_file(app), list)
}

pub fn load_scripts(app: &AppHandle) -> Result<Vec<ScriptMeta>, String> {
    load_scripts_at(&scripts_file(app))
}

// ---------------------------------------------------------------------------
// M2-6.b 命令片段持久化
//
// 与脚本库同目录、同原子写范式，但**没有正文文件**——命令以 `argv` 数组直接存在
// `snippets.json` 内（`CommandSnippet` 无 `path` 字段，见 M2-6.a 冻结条款 F6）。
// 因此不存在「先写正文、失败回滚」的两阶段，保存是单阶段原子写。
// ---------------------------------------------------------------------------

/// 命令片段库文件（M2-6.b）。
pub fn snippets_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("snippets.json")
}

/// 命令片段落盘（原子写：tmp + rename）。B4-1：走共享原语。
pub fn save_snippets_at(path: &Path, list: &[CommandSnippet]) -> Result<(), String> {
    save_json_list_at(path, list, "命令片段库")
}

/// 命令片段读取。读取失败返回错误，禁止用空列表覆盖原库。
pub fn load_snippets_at(path: &Path) -> Result<Vec<CommandSnippet>, String> {
    load_json_list_at(path, "命令片段库")
}

pub fn save_snippets(app: &AppHandle, list: &[CommandSnippet]) -> Result<(), String> {
    save_snippets_at(&snippets_file(app), list)
}

pub fn load_snippets(app: &AppHandle) -> Result<Vec<CommandSnippet>, String> {
    load_snippets_at(&snippets_file(app))
}

/// 正文文件名形态：`<id>.<ext>`，不含任何路径分隔符。
fn validate_script_file_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 128 {
        return Err("SCRIPT_FILE_NAME_INVALID".to_string());
    }
    if name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return Err("SCRIPT_FILE_NAME_INVALID".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err("SCRIPT_FILE_NAME_INVALID".to_string());
    }
    Ok(())
}

/// 解析正文绝对路径：**先形态校验，再 canonicalize + 前缀校验**。
///
/// 根目录与父目录**都要** canonicalize：只校验最终路径的话，中间目录被替换成
/// 软链接的场景会漏判（M2-1 图片落盘同款处理）。任何试图读写 `root` 之外
/// 路径的请求都在这里被拦下。
pub fn body_path_in(root: &Path, file_name: &str) -> Result<PathBuf, String> {
    validate_script_file_name(file_name)?;
    let root_canon = fs::canonicalize(root).map_err(|e| format!("脚本目录不可用: {e}"))?;
    let target = root.join(file_name);
    let parent = target.parent().ok_or_else(|| "PATH_ESCAPE".to_string())?;
    let parent_canon = fs::canonicalize(parent).map_err(|e| format!("脚本目录不可用: {e}"))?;
    if !parent_canon.starts_with(&root_canon) {
        return Err("PATH_ESCAPE".to_string());
    }
    Ok(target)
}

/// 契约预留：`script_body_path` 由 M2-5 的读正文命令消费（本卡命令只写不读）。
#[allow(dead_code)]
pub fn script_body_path(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    body_path_in(&scripts_dir(app), file_name)
}

/// 正文原子写（tmp + rename；失败清理临时文件，不留残留）
fn atomic_write_body(target: &Path, body: &str) -> Result<(), String> {
    let tmp = target.with_file_name(format!(
        ".browseros-script-{}.tmp",
        uuid::Uuid::new_v4()
    ));
    if let Err(e) = fs::write(&tmp, body) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("写脚本临时文件失败: {e}"));
    }
    crate::session::atomic_replace_file(&tmp, target)
        .map_err(|e| format!("脚本落盘 replace 失败: {e}"))?;
    Ok(())
}

pub fn write_body_at(root: &Path, file_name: &str, body: &str) -> Result<(), String> {
    let target = body_path_in(root, file_name)?;
    atomic_write_body(&target, body)
}

/// 契约预留：正文读取由 **M2-5 脚本库 UI**（编辑/查看脚本）消费。
#[allow(dead_code)]
pub fn read_body_at(root: &Path, file_name: &str) -> Result<String, String> {
    let target = body_path_in(root, file_name)?;
    fs::read_to_string(&target).map_err(|e| format!("读取脚本正文失败: {e}"))
}

/// 删除正文文件：**幂等**（不存在视为成功，与 `remove_artifact_images` 同口径）
pub fn delete_body_at(root: &Path, file_name: &str) -> Result<(), String> {
    let target = body_path_in(root, file_name)?;
    match fs::remove_file(&target) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("删除脚本正文失败: {e}")),
    }
}

pub fn write_script_body(app: &AppHandle, file_name: &str, body: &str) -> Result<(), String> {
    write_body_at(&scripts_dir(app), file_name, body)
}

/// 契约预留：同 `read_body_at`，由 M2-5 消费。
#[allow(dead_code)]
pub fn read_script_body(app: &AppHandle, file_name: &str) -> Result<String, String> {
    read_body_at(&scripts_dir(app), file_name)
}

pub fn delete_script_body(app: &AppHandle, file_name: &str) -> Result<(), String> {
    delete_body_at(&scripts_dir(app), file_name)
}

/// Repository metadata keeps versioned credential references native-only.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct StoredRepoConfig {
    #[serde(flatten)]
    config: RepoConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    credential_ref: Option<String>,
}
static REPO_CONFIG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn repos_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("repos.json")
}
fn repo_key(record: &StoredRepoConfig) -> Result<String, String> {
    let id = &record.config.id;
    if id.is_empty() || id.len() > 128
        || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    { return Err("REPO_ID_INVALID".into()); }
    match &record.credential_ref {
        None => Ok(id.clone()),
        Some(reference) if reference == id => Ok(reference.clone()),
        Some(reference) => {
            let prefix = format!("git:{id}:");
            if reference.strip_prefix(&prefix).is_some_and(|version| uuid::Uuid::parse_str(version).is_ok()) {
                Ok(reference.clone())
            } else { Err("REPO_CREDENTIAL_REF_INVALID".into()) }
        }
    }
}
fn repo_live_keys(records: &[StoredRepoConfig]) -> Result<std::collections::HashSet<String>, String> {
    records.iter().map(repo_key).collect()
}
fn load_stored_repos(path: &Path) -> Result<Vec<StoredRepoConfig>, String> {
    load_json_list_at(path, "仓库配置")
}
pub fn load_repos_at(path: &Path) -> Result<Vec<RepoConfig>, String> {
    Ok(load_stored_repos(path)?.into_iter().map(|record| record.config).collect())
}
/// Metadata-only updates retain each existing reference, including legacy records.
pub fn save_repos_at(path: &Path, repos: &[RepoConfig]) -> Result<(), String> {
    let existing = load_stored_repos(path)?;
    let records: Vec<_> = repos.iter().map(|config| StoredRepoConfig {
        config: config.clone(),
        credential_ref: existing.iter().find(|record| record.config.id == config.id)
            .and_then(|record| record.credential_ref.clone()),
    }).collect();
    save_json_list_at(path, &records, "仓库配置")
}
pub fn load_repos(app: &AppHandle) -> Result<Vec<RepoConfig>, String> {
    let _guard = REPO_CONFIG_LOCK.lock().map_err(|_| "REPO_CONFIG_LOCK_FAILED")?;
    load_repos_at(&repos_file(app))
}
pub fn save_repos(app: &AppHandle, repos: &[RepoConfig]) -> Result<(), String> {
    let _guard = REPO_CONFIG_LOCK.lock().map_err(|_| "REPO_CONFIG_LOCK_FAILED")?;
    save_repos_at(&repos_file(app), repos)
}

/// Capture the endpoint and secret in one short native-owner critical section.
/// No lock or mutable Keyring name is carried into a queued/network operation.
pub fn repo_execution_snapshot(app: &AppHandle, id: &str) -> Result<(RepoConfig, String), String> {
    let _guard = REPO_CONFIG_LOCK.lock().map_err(|_| "REPO_CONFIG_LOCK_FAILED")?;
    let records = load_stored_repos(&repos_file(app))?;
    let record = records.into_iter().find(|record| record.config.id == id)
        .ok_or("仓库未配置")?;
    let reference = repo_key(&record)?;
    let token = crate::keyring_store::KeyringStore::get_token(&reference)
        .map_err(|_| "REPO_CREDENTIAL_UNAVAILABLE")?;
    Ok((record.config, token))
}

pub fn configure_repo_credentials(app: &AppHandle, config: RepoConfig, token: &str) -> Result<(), String> {
    use crate::shared::credential_versions;
    let _guard = REPO_CONFIG_LOCK.lock().map_err(|_| "REPO_CONFIG_LOCK_FAILED")?;
    let path = repos_file(app);
    let mut records = load_stored_repos(&path)?;
    let proposed = StoredRepoConfig { config: config.clone(), credential_ref: None };
    repo_key(&proposed)?;
    let endpoint = url::Url::parse(&config.remote_url).map_err(|_| "REPO_ENDPOINT_INVALID")?;
    if !matches!(endpoint.scheme(), "https" | "http") || endpoint.host_str().is_none()
        || endpoint.password().is_some()
    { return Err("REPO_ENDPOINT_INVALID".into()); }
    if token.trim().is_empty() || token.len() > 16 * 1024 || token.contains(['\r', '\n', '\0']) {
        return Err("REPO_CREDENTIAL_INVALID".into());
    }
    if records.len() >= 256 && !records.iter().any(|record| record.config.id == config.id) {
        return Err("REPO_CONFIG_LIMIT".into());
    }
    let live = repo_live_keys(&records)?;
    let _ = credential_versions::reconcile(&path, "git", &live);
    let retired: Vec<String> = records.iter().filter(|record| record.config.id == config.id)
        .map(repo_key).collect::<Result<_, _>>()?;
    let reference = credential_versions::stage(&path, "git", &config.id, token, &retired)?;
    records.retain(|record| record.config.id != config.id);
    records.push(StoredRepoConfig { config, credential_ref: Some(reference) });
    let saved = save_json_list_at(&path, &records, "仓库配置");
    // An atomic replacement can publish before a durability error. Reconcile
    // against the persisted file even on Err; never guess which key is live.
    if let Ok(persisted) = load_stored_repos(&path) {
        if let Ok(live) = repo_live_keys(&persisted) {
            if credential_versions::reconcile(&path, "git", &live).is_err() {
                eprintln!("[workspace] repository credential cleanup pending");
            }
        }
    }
    saved
}

/// 审计日志
pub fn audit_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("audit.json")
}
pub fn log_audit(app: &AppHandle, action: &str, detail: String) {
    if let Err(error) = append_audit_at(&audit_file(app), AuditEntry {
        at: Utc::now(), action: action.into(), detail,
    }) {
        // Do not echo an audit detail or filesystem error containing user data.
        eprintln!("[workspace] audit append failed: {error}");
    }
}

/// The lock spans read, corruption recovery, append, trim and atomic replace.
/// Locking only the final rename would still lose concurrent writers.
pub fn append_audit_at(path: &Path, entry: AuditEntry) -> Result<(), String> {
    let _guard = audit_lock::AuditLock::acquire(path)
        .map_err(|error| format!("AUDIT_LOCK_FAILED:{:?}", error.kind()))?;
    let mut list = load_audit_unlocked(path)?;
    list.push(entry);
    if list.len() > 1000 { list.drain(0..list.len() - 1000); }
    save_json_list_at(path, &list, "审计日志")
        .map_err(|_| "AUDIT_ATOMIC_WRITE_FAILED".to_string())
}

fn load_audit_unlocked(path: &Path) -> Result<Vec<AuditEntry>, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err("AUDIT_READ_FAILED".into()),
    };
    match serde_json::from_str(&content) {
        Ok(entries) => Ok(entries),
        Err(_) => {
            let mut name = path.as_os_str().to_os_string();
            name.push(".corrupt");
            let mut backup = PathBuf::from(name);
            if backup.exists() {
                let mut name = path.as_os_str().to_os_string();
                name.push(format!(".corrupt-{}", uuid::Uuid::new_v4()));
                backup = PathBuf::from(name);
            }
            fs::rename(path, backup).map_err(|_| "AUDIT_CORRUPT_BACKUP_FAILED".to_string())?;
            eprintln!("[workspace] audit corruption preserved in private backup");
            Ok(Vec::new())
        }
    }
}

/// Reading can move a corrupt file to a backup, so it uses the same lock.
pub fn load_audit_at(path: &Path) -> Vec<AuditEntry> {
    match audit_lock::AuditLock::acquire(path) {
        Ok(_guard) => match load_audit_unlocked(path) {
            Ok(entries) => entries,
            Err(code) => { eprintln!("[workspace] audit read unavailable: {code}"); Vec::new() }
        },
        Err(error) => {
            eprintln!("[workspace] audit read unavailable: {:?}", error.kind());
            Vec::new()
        }
    }
}
pub fn save_audit_at(path: &Path, list: &[AuditEntry]) -> Result<(), String> {
    let _guard = audit_lock::AuditLock::acquire(path)
        .map_err(|error| format!("AUDIT_LOCK_FAILED:{:?}", error.kind()))?;
    save_json_list_at(path, list, "审计日志")
        .map_err(|_| "AUDIT_ATOMIC_WRITE_FAILED".to_string())
}
pub fn load_audit(app: &AppHandle) -> Vec<AuditEntry> {
    load_audit_at(&audit_file(app))
}

/// Use the bounded private audit channel for lifecycle evidence. Never create
/// a shared temporary marker or an unbounded series of per-launch log files.
pub fn record_lifecycle(app: &AppHandle, event: &'static str) {
    if event == "main-window-created-and-shown" {
        log_audit(app, "lifecycle", event.into());
    }
}

#[cfg(test)]
mod audit_concurrency_tests {
    use super::*;
    #[test]
    fn concurrent_audit_appends_keep_every_entry() {
        let root = std::env::temp_dir().canonicalize().unwrap()
            .join(format!("browseros-audit-{}", uuid::Uuid::new_v4()));
        crate::capabilities::workspace::safe_paths::ensure_private_directory(&root).unwrap();
        let path = root.join("audit.json");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers: Vec<_> = (0..8).map(|writer| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                for index in 0..8 {
                    append_audit_at(&path, AuditEntry {
                        at: Utc::now(), action: "concurrent".into(),
                        detail: format!("{writer}:{index}"),
                    }).unwrap();
                }
            })
        }).collect();
        for worker in workers { worker.join().unwrap(); }
        let entries = load_audit_at(&path);
        assert_eq!(entries.len(), 64);
        let details: std::collections::HashSet<_> = entries.iter().map(|entry| &entry.detail).collect();
        assert_eq!(details.len(), 64);
        fs::remove_dir_all(root).unwrap();
    }
}

/// 收藏持久化文件
pub fn bookmarks_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("bookmarks.json")
}

/// M1-9：浏览器会话存档目录（`data_dir/sessions/<id>.json`，原子写）。
/// 与成果库/收藏分开：会话是「浏览痕迹」，生命周期与隐私边界都不同。
pub fn sessions_dir(app: &AppHandle) -> PathBuf {
    let d = data_dir(app).join("sessions");
    ensure(&d);
    d
}
/// 收藏读取（`_at` 便于无 AppHandle 单测）。读取失败返回错误，禁止用空列表覆盖原库。
pub fn load_bookmarks_at(path: &Path) -> Result<Vec<Bookmark>, String> {
    load_json_list_at(path, "收藏")
}
/// 收藏落盘（`_at` 便于无 AppHandle 单测）。B4-1：原子写。
pub fn save_bookmarks_at(path: &Path, list: &[Bookmark]) -> Result<(), String> {
    save_json_list_at(path, list, "收藏")
}
pub fn load_bookmarks(app: &AppHandle) -> Result<Vec<Bookmark>, String> {
    load_bookmarks_at(&bookmarks_file(app))
}
pub fn save_bookmarks(app: &AppHandle, list: &[Bookmark]) -> Result<(), String> {
    save_bookmarks_at(&bookmarks_file(app), list)
}

/// 新增收藏：若 url 已存在则视为更新（保留原 id/created_at，仅覆盖 title/category）。
pub fn add_bookmark(
    app: &AppHandle,
    url: String,
    title: String,
    category: String,
) -> Result<Bookmark, String> {
    let mut list = load_bookmarks(app)?;
    if let Some(existing) = list.iter_mut().find(|b| b.url == url) {
        existing.title = title;
        existing.category = category;
        let bm = existing.clone();
        save_bookmarks(app, &list)?;
        Ok(bm)
    } else {
        let bm = Bookmark::new(url, title, category);
        list.push(bm.clone());
        save_bookmarks(app, &list)?;
        Ok(bm)
    }
}

/// 按 id 删除收藏；不存在不报错（幂等）。
pub fn remove_bookmark(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut list = load_bookmarks(app)?;
    let before = list.len();
    list.retain(|b| b.id != id);
    if list.len() != before {
        save_bookmarks(app, &list)?;
    }
    Ok(())
}

#[cfg(test)]
mod snippet_persistence_tests {
    use super::*;
    use crate::domain::{ParamType, ScriptInterpreter, ScriptParam};

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "mvp-browser-os-{name}-{}-{}.json",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        path
    }

    fn snippet() -> CommandSnippet {
        let now = Utc::now();
        CommandSnippet {
            id: uuid::Uuid::new_v4().to_string(),
            name: "List logs".to_string(),
            category: "text".to_string(),
            interpreter: ScriptInterpreter::Bash,
            argv: vec![
                "grep".to_string(),
                "-r".to_string(),
                "{PATTERN}".to_string(),
            ],
            params: vec![ScriptParam {
                name: "PATTERN".to_string(),
                label: "Pattern".to_string(),
                param_type: ParamType::String,
                required: true,
                default: None,
                options: vec![],
                raw: false,
                secret: false,
            }],
            description: "Search text".to_string(),
            dangerous: false,
            builtin: false,
            enabled: true,
            timeout_secs: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn snippets_round_trip_without_body_file() {
        let path = temp_path("snippets-round-trip");
        let list = vec![snippet()];

        save_snippets_at(&path, &list).expect("save snippets");
        let loaded = load_snippets_at(&path).expect("load snippets");

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].argv, vec!["grep", "-r", "{PATTERN}"]);
        assert_eq!(loaded[0].params[0].name, "PATTERN");
        assert!(!path.with_extension("sh").exists());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_store_is_empty_but_corrupt_store_blocks_mutation() {
        let missing = temp_path("snippets-missing");
        assert!(load_snippets_at(&missing).unwrap().is_empty());
        let path = temp_path("snippets-corrupt");
        let bytes = b"[{\"name\":\"old\"},\xff]";
        fs::write(&path, bytes).unwrap();
        let add = || -> Result<(), String> {
            let mut records = load_snippets_at(&path)?;
            records.push(snippet());
            save_snippets_at(&path, &records)
        };
        assert!(add().unwrap_err().starts_with("WORKSPACE_STORE_CORRUPT"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        let backup = corrupt_store_backup_path(&path, bytes);
        assert_eq!(fs::read(&backup).unwrap(), bytes);
        assert!(load_snippets_at(&path).is_err());
        assert_eq!(fs::read(&backup).unwrap(), bytes);
        fs::remove_file(backup).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn backup_failure_never_returns_a_writable_empty_store() {
        let path = temp_path("snippets-backup-failure");
        let bytes = b"not valid JSON";
        fs::write(&path, bytes).unwrap();
        let backup = corrupt_store_backup_path(&path, bytes);
        fs::create_dir(&backup).unwrap();
        assert!(load_snippets_at(&path).unwrap_err().contains("BACKUP_FAILED"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_dir(backup).unwrap();
        fs::remove_file(path).unwrap();
    }

}

// ---------------------------------------------------------------------------
// BUG-HUNT B4-1 / B4-2 回归测试：workspace 落盘必须原子，损坏必须留证
// ---------------------------------------------------------------------------
#[cfg(test)]
mod b4_atomic_persistence_tests {
    use super::*;
    use crate::domain::RepoProvider;

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "mvp-browser-os-b4-{name}-{}-{}.json",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        path
    }

    /// `session::atomic_write` 的临时文件口径：`path.with_extension("json.tmp")`。
    fn tmp_of(path: &Path) -> PathBuf {
        path.with_extension("json.tmp")
    }

    fn corrupt_backup_of(path: &Path) -> PathBuf {
        let mut backup = path.as_os_str().to_os_string();
        backup.push(".corrupt");
        PathBuf::from(backup)
    }

    fn repo() -> RepoConfig {
        RepoConfig {
            id: uuid::Uuid::new_v4().to_string(),
            provider: RepoProvider::Git,
            name: "demo".to_string(),
            remote_url: "https://example.com/demo.git".to_string(),
            branch: "main".to_string(),
            username: "alice".to_string(),
        }
    }

    fn audit_entry(action: &str) -> AuditEntry {
        AuditEntry {
            at: Utc::now(),
            action: action.to_string(),
            detail: "detail".to_string(),
        }
    }

    #[test]
    fn repos_roundtrip_is_atomic_without_tmp_leftover() {
        let path = temp_path("repos");
        save_repos_at(&path, &[repo()]).expect("save repos");
        let back = load_repos_at(&path).expect("load repos");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].name, "demo");
        assert!(!tmp_of(&path).exists(), "atomic_write 后不得残留 .tmp");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn bookmarks_roundtrip_is_atomic_without_tmp_leftover() {
        let path = temp_path("bookmarks");
        let list = vec![Bookmark::new(
            "https://example.com".to_string(),
            "Example".to_string(),
            "work".to_string(),
        )];
        save_bookmarks_at(&path, &list).expect("save bookmarks");
        let back = load_bookmarks_at(&path).expect("load bookmarks");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].title, "Example");
        assert!(!tmp_of(&path).exists(), "atomic_write 后不得残留 .tmp");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn audit_roundtrip_is_atomic_without_tmp_leftover() {
        let directory = std::env::temp_dir().canonicalize().unwrap()
            .join(format!("browseros-audit-{}", uuid::Uuid::new_v4()));
        crate::capabilities::workspace::safe_paths::ensure_private_directory(&directory).unwrap();
        let path = directory.join("audit.json");
        save_audit_at(&path, &[audit_entry("a.one"), audit_entry("a.two")]).expect("save audit");
        let back = load_audit_at(&path);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].action, "a.one");
        assert!(!tmp_of(&path).exists(), "atomic_write 后不得残留 .tmp");
        let _ = fs::remove_file(&path);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn corrupt_file_is_backed_up_and_original_stays_blocked() {
        let path = temp_path("corrupt-repos");
        save_repos_at(&path, &[repo()]).unwrap();
        let bytes = b"{not json at all";
        fs::write(&path, bytes).unwrap();
        assert!(load_repos_at(&path).is_err());
        let backup = corrupt_store_backup_path(&path, bytes);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read(&backup).unwrap(), bytes);
        fs::remove_file(backup).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn corrupt_audit_is_backed_up_and_survives_next_save() {
        let directory = std::env::temp_dir().canonicalize().unwrap()
            .join(format!("browseros-corrupt-audit-{}", uuid::Uuid::new_v4()));
        crate::capabilities::workspace::safe_paths::ensure_private_directory(&directory).unwrap();
        let path = directory.join("audit.json");
        fs::write(&path, "{{{ broken").expect("simulate corruption");
        assert!(load_audit_at(&path).is_empty());

        let backup = corrupt_backup_of(&path);
        assert!(backup.exists(), "审计损坏必须留证（.corrupt）");

        // 即便随后再落盘一份新审计，证据仍在（审计是热路径，最需要留证）。
        save_audit_at(&path, &[audit_entry("after.recovery")]).expect("save after corrupt");
        assert!(backup.exists(), "新落盘不得破坏已备份的损坏证据");
        assert_eq!(load_audit_at(&path).len(), 1);

        let _ = fs::remove_file(&backup);
        let _ = fs::remove_file(&path);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn missing_file_stays_empty_without_backup() {
        let path = temp_path("missing");
        assert!(load_repos_at(&path).expect("missing store").is_empty());
        assert!(
            !corrupt_backup_of(&path).exists(),
            "文件不存在属正常首次启动，不应产生 .corrupt"
        );
    }
}
