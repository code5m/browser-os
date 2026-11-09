//! Artifact, repository and image persistence commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

#[tauri::command]
pub fn list_artifacts(app: AppHandle) -> Vec<Artifact> {
    workspace::load_artifacts(&app)
}

/// 配置仓库：token 仅写入系统密钥库，绝不回传前端。
#[tauri::command]
pub fn configure_repo(app: AppHandle, config: RepoConfig, token: String) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("token 不能为空".into());
    }
    KeyringStore::save_token(&config.id, &token)?;
    let mut repos = workspace::load_repos(&app);
    repos.retain(|r| r.id != config.id);
    repos.push(config);
    workspace::save_repos(&app, &repos)?;
    workspace::log_audit(
        &app,
        "configure_repo",
        format!(
            "{} ({:?})",
            repos.last().unwrap().name,
            repos.last().unwrap().provider
        ),
    );
    Ok(())
}

#[tauri::command]
pub fn list_repos(app: AppHandle) -> Vec<RepoConfig> {
    workspace::load_repos(&app)
}

/// 读取单个成果完整内容（预览/编辑用）
#[tauri::command]
pub fn read_artifact(app: AppHandle, id: String) -> Result<Artifact, String> {
    workspace::load_artifacts(&app)
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| "成果不存在".to_string())
}

/// 编辑成果：仅允许改标题/标签/正文（溯源字段 source_url/hash/created_at 不可变）
#[tauri::command]
pub fn update_artifact(
    app: AppHandle,
    id: String,
    title: String,
    text: String,
    tags: Vec<String>,
) -> Result<Artifact, String> {
    let mut arts = workspace::load_artifacts(&app);
    let art = arts
        .iter_mut()
        .find(|a| a.id == id)
        .ok_or_else(|| "成果不存在".to_string())?;
    if !title.trim().is_empty() {
        art.title = title.trim().to_string();
    }
    art.text = text;
    art.tags = tags;
    let updated = art.clone();
    workspace::save_artifact(&app, art)?;
    workspace::log_audit(&app, "update", format!("编辑成果 {}", id));
    Ok(updated)
}

#[tauri::command]
pub fn delete_artifact(app: AppHandle, id: String) -> Result<(), String> {
    workspace::delete_artifact(&app, &id)?;
    workspace::log_audit(&app, "delete", format!("删除成果 {}", id));
    Ok(())
}

// ---------------------------------------------------------------------------
// M2-1 图片领域与持久化（契约见 logs/assist/M2-1.a-prework-20260902-1055.md）
//
// 范围：只做「领域 + 持久化」。画廊/灯箱/缩放属 M2-2，字节读取与渲染通道
// （asset:// scope 或后端直读）随 M2-2 一起落地，本卡不提前扩张安全边界。
//
// 红线：
//   - 两条命令均过 `check_invocation_source`；artifact_id 先过形态校验再拼路径
//   - MIME 白名单 fail-closed（SVG 拒绝），扩展名由 MIME 反查，魔法字节比对
//   - 超限在写盘前拒绝（不留临时文件）；落盘原子写；去重按 sha256
//   - 审计只记 id/字节数/MIME，**绝不记 URL 与图片内容**
// ---------------------------------------------------------------------------

/// 保存图片到指定成果（校验顺序见 `workspace::save_image`）。
#[tauri::command]
pub fn save_image(
    app: AppHandle,
    webview: tauri::Webview,
    artifact_id: String,
    data: Vec<u8>,
    mime: String,
    source_url: Option<String>,
    caption: Option<String>,
) -> Result<ImageRef, String> {
    check_invocation_source(&webview, "save_image", None, &app)?;
    crate::images::check_id(&artifact_id, "成果 id")?;

    // IPC 载荷入口先卡一次上限，避免把超大内容带进内存后才判超限
    if data.is_empty() {
        return Err(crate::images::ImageError::ImageEmpty.to_string());
    }
    if data.len() > IMAGE_MAX_BYTES {
        return Err(crate::images::ImageError::ImageTooLarge.to_string());
    }

    // 说明文案走既有文本边界校验（M0-3 口径），空串视为不填
    let cap = caption.as_deref().map(str::trim).filter(|c| !c.is_empty());
    if let Some(c) = cap {
        crate::security_policy::check_text_field(
            "caption",
            c,
            crate::security_policy::MAX_TEXT_FIELD_BYTES,
        )
        .map_err(|e| e.to_string())?;
    }

    let reference =
        workspace::save_image(&app, &artifact_id, &data, &mime, source_url.as_deref(), cap)?;
    workspace::log_audit(
        &app,
        "image.save",
        format!(
            "artifact_id={artifact_id} image_id={} bytes={} mime={}",
            reference.id, reference.bytes, reference.mime
        ),
    );
    Ok(reference)
}

/// 列出某成果的图片引用（出参 source_url 二次脱敏兜底）。
#[tauri::command]
pub fn list_artifact_images(
    app: AppHandle,
    webview: tauri::Webview,
    artifact_id: String,
) -> Result<Vec<ImageRef>, String> {
    check_invocation_source(&webview, "list_artifact_images", None, &app)?;
    crate::images::check_id(&artifact_id, "成果 id")?;
    let art = workspace::load_artifacts(&app)
        .into_iter()
        .find(|a| a.id == artifact_id)
        .ok_or_else(|| "成果不存在".to_string())?;
    let mut images = art.images;
    for r in images.iter_mut() {
        // 落库时已脱敏，这里对历史/手改数据再兜一层
        r.source_url = crate::images::redact_source_url(r.source_url.as_deref());
    }
    Ok(images)
}

/// M2-2.b：图片目录基准（只读，供前端 `convertFileSrc` 拼 `asset://`）。
///
/// 只返回目录基准本身，**不含任何图片相对路径**，也不读取字节：
/// 字节通道由前端 `convertFileSrc` 走 `asset://`，图片目录在既有
/// `assetProtocol.scope` 的 `$HOME/.local/share/**` 之内，不扩张 scope。
/// 路径拼接仍复用后端白名单（`images::join_image_path`），前端不得自行推导。
#[tauri::command]
pub fn workspace_images_dir(app: AppHandle, webview: tauri::Webview) -> Result<String, String> {
    check_invocation_source(&webview, "workspace_images_dir", None, &app)?;
    let dir = workspace::images_dir(&app);
    workspace::log_audit(
        &app,
        "image.dir_read",
        "op=workspace_images_dir".to_string(),
    );
    Ok(dir.to_string_lossy().to_string())
}

// ---------------------------------------------------------------------------
// M2-3 脚本领域与持久化（契约见 logs/checkpoints/M2-3.a-20260903-1604.md）
//
// 范围：**只做元数据的增删改查**，不做执行。`run_script` / 取消 / 超时 /
// 输出上限 / 进程组 kill / 运行记录一律归 M2-4，前端面板归 M2-5。
//
// 红线：
//   - 四条命令均过 `check_invocation_source`
//   - 落盘前先过 `scripts::validate_meta` / `validate_body`（先验收后写）
//   - 审计 detail 只含 id/name/interpreter/param_count/builtin，
//     **绝不写脚本正文、参数默认值或参数值**
//   - 正文读写一律经 `workspace::script_body_path`（canonicalize + 前缀校验）
// ---------------------------------------------------------------------------
