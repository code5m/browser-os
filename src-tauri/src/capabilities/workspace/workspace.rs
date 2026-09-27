use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use tauri::{AppHandle, Manager};

use crate::domain::{
    Artifact, AuditEntry, Bookmark, CommandSnippet, ImageRef, RepoConfig, ScriptMeta,
};

/// 应用数据根目录（`.../<app_data_dir>/mvp-browser-os`）。
///
/// M4-6 起对 `tasks.rs` 开放：任务持久化 `tasks.json` / `task-runs.json` 与既有
/// `scripts.json` / `snippets.json` 必须落在**同一**数据目录（A1 实测项 11：全仓
/// 持久化均为该目录下的 JSON，不得另起第二条路径）。
pub fn data_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("app data dir")
        .join("mvp-browser-os")
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
//   - 文件不存在 / 读不到 → 空列表（正常首次启动，不是数据丢失）；
//   - **解析失败绝不静默清空**：备份为 `<file>.corrupt` + 告警后返回空
//     （对齐 `tasks.rs` O-A6-7 范式，保证原字节可人工取回）。
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

/// 列表读取：缺失 → 空；**损坏 → `.corrupt` 备份 + 告警 → 空**（不静默丢弃）。
pub fn load_json_list_at<T: serde::de::DeserializeOwned>(path: &Path, tag: &str) -> Vec<T> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    match serde_json::from_str::<Vec<T>>(&content) {
        Ok(list) => list,
        Err(e) => {
            eprintln!(
                "[workspace] {tag} 文件损坏，已备份为 .corrupt（不会静默清空后被覆盖写回）：{}（{e}）",
                path.display()
            );
            let mut backup = path.as_os_str().to_os_string();
            backup.push(".corrupt");
            let _ = fs::rename(path, PathBuf::from(backup));
            Vec::new()
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

/// 脚本库读取。B4-2：损坏 → `.corrupt` 备份 + 告警（不再 `unwrap_or_default()` 静默清空）。
pub fn load_scripts_at(path: &Path) -> Vec<ScriptMeta> {
    load_json_list_at(path, "脚本库")
}

pub fn save_scripts(app: &AppHandle, list: &[ScriptMeta]) -> Result<(), String> {
    save_scripts_at(&scripts_file(app), list)
}

pub fn load_scripts(app: &AppHandle) -> Vec<ScriptMeta> {
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

/// 命令片段读取。B4-2：损坏 → `.corrupt` 备份 + 告警。
pub fn load_snippets_at(path: &Path) -> Vec<CommandSnippet> {
    load_json_list_at(path, "命令片段库")
}

pub fn save_snippets(app: &AppHandle, list: &[CommandSnippet]) -> Result<(), String> {
    save_snippets_at(&snippets_file(app), list)
}

pub fn load_snippets(app: &AppHandle) -> Vec<CommandSnippet> {
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
    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("sh")
        .to_string();
    let tmp = target.with_extension(format!("{ext}.tmp"));
    if let Err(e) = fs::write(&tmp, body) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("写脚本临时文件失败: {e}"));
    }
    if let Err(e) = fs::rename(&tmp, target) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("脚本落盘 rename 失败: {e}"));
    }
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

/// 仓库配置（不含 token）持久化
pub fn repos_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("repos.json")
}
/// 仓库配置读取（`_at` 便于无 AppHandle 单测）。B4-2：损坏 → `.corrupt` 备份 + 告警。
pub fn load_repos_at(path: &Path) -> Vec<RepoConfig> {
    load_json_list_at(path, "仓库配置")
}
/// 仓库配置落盘（`_at` 便于无 AppHandle 单测）。B4-1：原子写，不再 `fs::write` 直接覆盖。
pub fn save_repos_at(path: &Path, repos: &[RepoConfig]) -> Result<(), String> {
    save_json_list_at(path, repos, "仓库配置")
}
pub fn load_repos(app: &AppHandle) -> Vec<RepoConfig> {
    load_repos_at(&repos_file(app))
}
pub fn save_repos(app: &AppHandle, repos: &[RepoConfig]) -> Result<(), String> {
    save_repos_at(&repos_file(app), repos)
}

/// 审计日志
pub fn audit_file(app: &AppHandle) -> PathBuf {
    data_dir(app).join("audit.json")
}
pub fn log_audit(app: &AppHandle, action: &str, detail: String) {
    let mut list = load_audit(app);
    list.push(AuditEntry {
        at: Utc::now(),
        action: action.into(),
        detail,
    });
    if list.len() > 1000 {
        list.drain(0..list.len() - 1000);
    }
    // B4-1：原子写；且**序列化失败绝不写空串**——改前 `unwrap_or_default()` 会把整份
    // 审计抹成空文件（`let _ =` 还吞掉错误）。现在失败只告警，原文件保持不被覆盖。
    if let Err(e) = save_audit_at(&audit_file(app), &list) {
        eprintln!("[workspace] 审计落盘失败（保留原文件，不覆盖写空）：{e}");
    }
}
/// 审计读取（`_at` 便于无 AppHandle 单测）。B4-2：损坏 → `.corrupt` 备份 + 告警。
pub fn load_audit_at(path: &Path) -> Vec<AuditEntry> {
    load_json_list_at(path, "审计日志")
}
/// 审计落盘（`_at` 便于无 AppHandle 单测）。B4-1：原子写。
pub fn save_audit_at(path: &Path, list: &[AuditEntry]) -> Result<(), String> {
    save_json_list_at(path, list, "审计日志")
}
pub fn load_audit(app: &AppHandle) -> Vec<AuditEntry> {
    load_audit_at(&audit_file(app))
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
/// 收藏读取（`_at` 便于无 AppHandle 单测）。B4-2：损坏 → `.corrupt` 备份 + 告警。
pub fn load_bookmarks_at(path: &Path) -> Vec<Bookmark> {
    load_json_list_at(path, "收藏")
}
/// 收藏落盘（`_at` 便于无 AppHandle 单测）。B4-1：原子写。
pub fn save_bookmarks_at(path: &Path, list: &[Bookmark]) -> Result<(), String> {
    save_json_list_at(path, list, "收藏")
}
pub fn load_bookmarks(app: &AppHandle) -> Vec<Bookmark> {
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
    let mut list = load_bookmarks(app);
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
    let mut list = load_bookmarks(app);
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
        let loaded = load_snippets_at(&path);

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].argv, vec!["grep", "-r", "{PATTERN}"]);
        assert_eq!(loaded[0].params[0].name, "PATTERN");
        assert!(!path.with_extension("sh").exists());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_or_missing_snippets_file_is_empty() {
        let missing = temp_path("snippets-missing");
        assert!(load_snippets_at(&missing).is_empty());

        let corrupt = temp_path("snippets-corrupt");
        fs::write(&corrupt, "{not json").expect("write corrupt snippets");
        assert!(load_snippets_at(&corrupt).is_empty());
        // B4-2：损坏文件改备份为 `.corrupt`（原字节可人工取回），不再无痕蒸发。
        let mut backup = corrupt.as_os_str().to_os_string();
        backup.push(".corrupt");
        let backup = PathBuf::from(backup);
        assert!(backup.exists(), "损坏文件应备份为 .corrupt");
        let _ = fs::remove_file(&backup);
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
        let back = load_repos_at(&path);
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
        let back = load_bookmarks_at(&path);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].title, "Example");
        assert!(!tmp_of(&path).exists(), "atomic_write 后不得残留 .tmp");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn audit_roundtrip_is_atomic_without_tmp_leftover() {
        let path = temp_path("audit");
        save_audit_at(&path, &[audit_entry("a.one"), audit_entry("a.two")]).expect("save audit");
        let back = load_audit_at(&path);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].action, "a.one");
        assert!(!tmp_of(&path).exists(), "atomic_write 后不得残留 .tmp");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn corrupt_file_is_backed_up_not_silently_dropped() {
        let path = temp_path("corrupt-repos");
        // 先写有效数据，再模拟「崩溃把文件写坏」。
        save_repos_at(&path, &[repo()]).expect("seed repos");
        fs::write(&path, "{not json at all").expect("simulate corruption");

        assert!(
            load_repos_at(&path).is_empty(),
            "损坏时返回空，避免把坏数据当真的用"
        );

        // B4-2 核心：原字节必须仍在（.corrupt 备份），不会被下次保存的空列表覆盖蒸发。
        let backup = corrupt_backup_of(&path);
        assert!(backup.exists(), "损坏文件必须备份为 .corrupt");
        assert!(
            !path.exists(),
            "损坏文件已被 rename 走，不会被空列表覆盖写回"
        );
        let raw = fs::read_to_string(&backup).unwrap_or_default();
        assert!(
            raw.contains("not json at all"),
            "备份保留原始字节，可人工取回"
        );

        let _ = fs::remove_file(&backup);
    }

    #[test]
    fn corrupt_audit_is_backed_up_and_survives_next_save() {
        let path = temp_path("corrupt-audit");
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
    }

    #[test]
    fn missing_file_stays_empty_without_backup() {
        let path = temp_path("missing");
        assert!(load_repos_at(&path).is_empty());
        assert!(
            !corrupt_backup_of(&path).exists(),
            "文件不存在属正常首次启动，不应产生 .corrupt"
        );
    }
}
