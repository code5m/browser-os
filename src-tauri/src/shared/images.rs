//! M2-1 图片领域校验（纯函数层）
//!
//! 设计原则（与 `session.rs` 同口径）：
//!   1. **不依赖 `AppHandle`**：只吃字节与字符串，便于无环境单测。
//!   2. **fail-closed**：MIME 不在白名单一律拒绝；扩展名由 MIME **反查**，
//!      绝不接受外部传入（杜绝 `a.png.html`）。
//!   3. **先校验后落盘**：超限/逃逸用例在写任何字节之前失败，不留临时文件。
//!   4. **不伪造**：尺寸解析失败（未知容器/变体）返回 `None`，不猜、不填 0。
//!
//! 反向用例编号对应 `logs/assist/M2-1.a-prework-20260902-1055.md` §6。

use std::fmt;
use std::fs;
use std::path::Path;

use crate::domain::{
    ImageRef, ImageSource, IMAGE_ID_MAX_LEN, IMAGE_MAX_BYTES, IMAGE_MAX_COUNT, IMAGE_MAX_DIMENSION,
    IMAGE_MAX_TOTAL_BYTES, IMAGE_MIME_EXT,
};
use chrono::Utc;

/// 图片校验失败原因（错误码与冻结契约一致，前端可直接展示）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    /// MIME 不在白名单（含 SVG：可携带 `<script>`）
    MimeNotAllowed,
    /// 声明的 MIME 与文件头（magic bytes）不符
    MimeMagicMismatch,
    /// 路径逃逸（`..`、绝对路径、非法字符）
    PathEscape,
    /// 超过单张字节上限
    ImageTooLarge,
    /// 0 字节
    ImageEmpty,
    /// 最长边超过上限（防解码 OOM）
    ImageDimensionExceeded,
    /// 单个成果图片数量超限
    ImageCountExceeded,
    /// 单个成果图片总字节超限
    ImageBudgetExceeded,
    /// 超过允许内联为 dataURL 的上限（必须落盘）。
    /// 契约预留：`collect.js` 已把图片内联进 `Artifact.html`，M2-2 接入该通道后消费。
    /// 这里刻意保留（含 `check_inline_size`/`build_inline_ref`），避免 M2-2 重开契约。
    #[allow(dead_code)]
    InlineTooLarge,
    /// id 形态非法（用于拼接路径前拦截）
    InvalidId,
    /// 文件系统失败（目录不可建、写入失败、canonicalize 失败等）
    Io(String),
}

impl ImageError {
    /// 契约错误码（稳定字符串，前端据此选择降级文案）
    pub fn code(&self) -> &'static str {
        match self {
            ImageError::MimeNotAllowed => "MIME_NOT_ALLOWED",
            ImageError::MimeMagicMismatch => "MIME_MAGIC_MISMATCH",
            ImageError::PathEscape => "PATH_ESCAPE",
            ImageError::ImageTooLarge => "IMAGE_TOO_LARGE",
            ImageError::ImageEmpty => "IMAGE_EMPTY",
            ImageError::ImageDimensionExceeded => "IMAGE_DIMENSION_EXCEEDED",
            ImageError::ImageCountExceeded => "IMAGE_COUNT_EXCEEDED",
            ImageError::ImageBudgetExceeded => "IMAGE_BUDGET_EXCEEDED",
            ImageError::InlineTooLarge => "INLINE_TOO_LARGE",
            ImageError::InvalidId => "INVALID_ID",
            ImageError::Io(_) => "IO_FAILED",
        }
    }
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// MIME → 扩展名（白名单反查；非白名单返回 None）。
pub fn ext_for_mime(mime: &str) -> Option<&'static str> {
    IMAGE_MIME_EXT
        .iter()
        .find(|(m, _)| *m == mime.trim().to_ascii_lowercase())
        .map(|(_, ext)| *ext)
}

/// 是否在 MIME 白名单内（fail-closed）
pub fn is_allowed_mime(mime: &str) -> bool {
    ext_for_mime(mime).is_some()
}

/// 按文件头（magic bytes）识别 MIME；无法识别返回 `None`。
pub fn detect_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 8 && bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some("image/jpeg");
    }
    if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

fn be_u32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn le_u16(b: &[u8], at: usize) -> u32 {
    u32::from(b[at]) | (u32::from(b[at + 1]) << 8)
}

/// 解析图片尺寸（宽, 高）。解析不出返回 `None`——**不猜、不填 0**。
pub fn image_dimensions(bytes: &[u8], mime: &str) -> Option<(u32, u32)> {
    match mime {
        "image/png" => {
            // IHDR：宽 16..20、高 20..24（大端）
            if bytes.len() >= 24 {
                Some((be_u32(bytes, 16), be_u32(bytes, 20)))
            } else {
                None
            }
        }
        "image/gif" => {
            // 逻辑屏幕描述符：宽 6..8、高 8..10（小端）
            if bytes.len() >= 10 {
                Some((le_u16(bytes, 6), le_u16(bytes, 8)))
            } else {
                None
            }
        }
        "image/jpeg" => jpeg_dimensions(bytes),
        "image/webp" => webp_dimensions(bytes),
        _ => None,
    }
}

/// JPEG：扫描 SOF 段取高/宽（大端，位于段内偏移 +5 / +7）
fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        // SOF0~SOF3 / SOF5~SOF7 / SOF9~SOF11 / SOF13~SOF15 承载帧尺寸
        let is_sof = matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF);
        if is_sof {
            let height = u32::from(bytes[i + 5]) << 8 | u32::from(bytes[i + 6]);
            let width = u32::from(bytes[i + 7]) << 8 | u32::from(bytes[i + 8]);
            return Some((width, height));
        }
        // 无长度字段的独立标记
        if marker == 0xD8 || marker == 0xD9 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        let seg_len = (u32::from(bytes[i + 2]) << 8 | u32::from(bytes[i + 3])) as usize;
        if seg_len < 2 {
            return None;
        }
        i += 2 + seg_len;
    }
    None
}

/// WebP：VP8X（扩展）/ VP8（有损）/ VP8L（无损）三种容器
fn webp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 16 || !bytes.starts_with(b"RIFF") || &bytes[8..12] != b"WEBP" {
        return None;
    }
    match &bytes[12..16] {
        b"VP8X" => {
            if bytes.len() < 30 {
                return None;
            }
            // 画布宽/高各 24 位小端，存的是 -1 后的值
            let w =
                u32::from(bytes[24]) | (u32::from(bytes[25]) << 8) | (u32::from(bytes[26]) << 16);
            let h =
                u32::from(bytes[27]) | (u32::from(bytes[28]) << 8) | (u32::from(bytes[29]) << 16);
            Some((w + 1, h + 1))
        }
        b"VP8 " => {
            // 关键帧：3 字节帧头 + 0x9d 0x01 0x2a 同步码，其后 2 字节宽、2 字节高（低 14 位）
            if bytes.len() < 30 || &bytes[23..26] != [0x9d, 0x01, 0x2a] {
                return None;
            }
            Some((le_u16(bytes, 26) & 0x3FFF, le_u16(bytes, 28) & 0x3FFF))
        }
        b"VP8L" => {
            // 无损：0x2f 签名后 14 位宽-1、14 位高-1
            if bytes.len() < 25 || bytes[20] != 0x2f {
                return None;
            }
            let b0 = u32::from(bytes[21]);
            let b1 = u32::from(bytes[22]);
            let b2 = u32::from(bytes[23]);
            let b3 = u32::from(bytes[24]);
            let w = (b0 | ((b1 & 0x3F) << 8)) + 1;
            let h = ((b1 >> 6) | (b2 << 2) | ((b3 & 0x0F) << 10)) + 1;
            Some((w, h))
        }
        _ => None,
    }
}

/// SHA-256（内容寻址，用于去重）
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// 字节数预检（在分配/落盘之前拒绝，避免先写完再删）。
pub fn check_size(len: usize) -> Result<(), ImageError> {
    if len == 0 {
        return Err(ImageError::ImageEmpty);
    }
    if len > IMAGE_MAX_BYTES {
        return Err(ImageError::ImageTooLarge);
    }
    Ok(())
}

/// 内联上限：超过 `IMAGE_INLINE_MAX_BYTES` 必须落盘为文件，不得内联进 HTML。
/// （契约预留，M2-2 接入 `collect.js` 内联通道后消费；已有单测守护）
#[allow(dead_code)]
pub fn check_inline_size(len: usize) -> Result<(), ImageError> {
    if len == 0 {
        return Err(ImageError::ImageEmpty);
    }
    if len > crate::domain::IMAGE_INLINE_MAX_BYTES {
        return Err(ImageError::InlineTooLarge);
    }
    Ok(())
}

/// id（artifact / session / image）形态校验：只允许 `[A-Za-z0-9_-]`，
/// 拒绝 `/`、`\`、`.`、NUL、空值与超长值——所有用 id 拼路径的地方都必须先过这里。
pub fn validate_id(id: &str) -> Result<(), ImageError> {
    if id.is_empty() || id.len() > IMAGE_ID_MAX_LEN {
        return Err(ImageError::InvalidId);
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ImageError::InvalidId);
    }
    Ok(())
}

/// id 形态校验的薄封装：所有「拿 id 拼路径」的入口（会话/成果/图片/脚本/片段/任务/运行）
/// 都先过这里，缺形态校验时理论上可借 `../` 逃逸出目标目录（纵深防御）。
///
/// 实际校验委托 [`validate_id`]，这里只把 `ImageError` 包成中文错误文案，便于命令体
/// 直接向用户/前端返回可读信息。跨能力复用，与 [`validate_id`] 同置本模块
/// （属 `SHARED_NATIVE_INFRASTRUCTURE`）。
pub fn check_id(id: &str, what: &str) -> Result<(), String> {
    validate_id(id).map_err(|_| format!("非法 {what}"))
}

/// `rel_path` 校验：只允许 `[a-zA-Z0-9_/-]`，不含 `..`、不以 `/` 开头、
/// 不含单独的 `.` 段、不含 `\` 与 NUL、不以 `/` 结尾。
pub fn validate_rel_path(rel: &str) -> Result<(), ImageError> {
    if rel.is_empty() || rel.len() > 256 {
        return Err(ImageError::PathEscape);
    }
    if rel.starts_with('/') || rel.ends_with('/') || rel.contains('\\') || rel.contains('\0') {
        return Err(ImageError::PathEscape);
    }
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(ImageError::PathEscape);
        }
        if !seg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return Err(ImageError::PathEscape);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// M2-2.b 预览通道（只做「路径拼接」，不读字节）
//
// 字节读取由前端 `convertFileSrc` + `asset://` 承担（图片目录已在既有
// `assetProtocol.scope` 的 `$HOME/.local/share/**` 之内，不扩 scope）。
// 后端只提供目录基准与「基准 + 相对路径」的安全拼接，避免前端自行推导
// data_dir 或拼出可越界的绝对路径。
// ---------------------------------------------------------------------------

/// 图片子目录名（与 `write_image_file` 拼出的 rel 前缀 `images/...` 一致）。
pub const IMAGES_DIR_NAME: &str = "images";

/// 把「目录基准 + 相对路径」拼成绝对路径字符串（纯函数，有单测）。
///
/// - `rel` 必须过 `validate_rel_path`（`..`、绝对路径、反斜杠、空段一律拒绝）
/// - `dir` 为空视为不可用（宁可不显示，也不退化成相对路径读取）
///
/// 契约锚点：前端 `src/utils/imagePreview.ts` 的 `buildAssetSrc` 是本函数的同构
/// 实现（同一套白名单）。生产路径暂时只由前端调用（后端不读字节），这里保留
/// 后端实现 + 单测，确保两侧白名单不会各自漂移；与 `InlineTooLarge` 同为契约
/// 预留，故标注 `allow(dead_code)`。
#[allow(dead_code)]
pub fn join_image_path(dir: &str, rel: &str) -> Result<String, ImageError> {
    validate_rel_path(rel)?;
    let base = dir.trim_end_matches('/');
    if base.is_empty() {
        return Err(ImageError::PathEscape);
    }
    Ok(format!("{base}/{rel}"))
}

/// 溯源 URL 脱敏（复用 M1-8 既有策略：`?token=` 等敏感查询值 → `***`）。
/// 这是图片路径上唯一的 URL 出入点，禁止绕过。
pub fn redact_source_url(url: Option<&str>) -> Option<String> {
    url.map(|u| crate::security_policy::redact_sensitive_url(u))
}

/// 通过校验后的图片元数据（不含字节本体）
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedImage {
    pub mime: String,
    pub ext: &'static str,
    pub bytes: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub sha256: String,
}

/// 入参校验流水线：尺寸预检 → MIME 白名单 → 魔法字节 → 最长边。
/// **顺序即契约**：超大小写必须在任何落盘动作之前被拦下。
pub fn prepare(bytes: &[u8], declared_mime: &str) -> Result<PreparedImage, ImageError> {
    check_size(bytes.len())?;
    let mime = declared_mime.trim().to_ascii_lowercase();
    if !is_allowed_mime(&mime) {
        return Err(ImageError::MimeNotAllowed);
    }
    if detect_mime(bytes) != Some(mime.as_str()) {
        return Err(ImageError::MimeMagicMismatch);
    }
    let ext = ext_for_mime(&mime).ok_or(ImageError::MimeNotAllowed)?;
    let (width, height) = match image_dimensions(bytes, &mime) {
        Some((w, h)) => {
            if w > IMAGE_MAX_DIMENSION || h > IMAGE_MAX_DIMENSION {
                return Err(ImageError::ImageDimensionExceeded);
            }
            (Some(w), Some(h))
        }
        // 容器认识但尺寸解析不出：不阻塞保存，如实留空（不伪造）
        None => (None, None),
    };
    Ok(PreparedImage {
        mime,
        ext,
        bytes: bytes.len() as u64,
        width,
        height,
        sha256: sha256_hex(bytes),
    })
}

/// 去重：同 sha256 命中已有引用则复用（不重复写盘）。
pub fn find_duplicate<'a>(existing: &'a [ImageRef], sha256: &str) -> Option<&'a ImageRef> {
    existing.iter().find(|r| r.sha256 == sha256)
}

/// 配额校验：数量上限 + 总字节上限（在写入前判定，避免写一半超预算）。
pub fn check_quota(existing: &[ImageRef], incoming_bytes: u64) -> Result<(), ImageError> {
    if existing.len() >= IMAGE_MAX_COUNT {
        return Err(ImageError::ImageCountExceeded);
    }
    let used: u64 = existing.iter().map(|r| r.bytes).sum();
    if used.saturating_add(incoming_bytes) > IMAGE_MAX_TOTAL_BYTES {
        return Err(ImageError::ImageBudgetExceeded);
    }
    Ok(())
}

/// 构造落盘型 `ImageRef`（id 与时间戳在此生成，rel_path 由调用方按白名单拼好并先校验）
pub fn build_file_ref(
    prepared: &PreparedImage,
    rel_path: &str,
    source_url: Option<&str>,
    caption: Option<&str>,
) -> Result<ImageRef, ImageError> {
    validate_rel_path(rel_path)?;
    Ok(ImageRef {
        id: uuid::Uuid::new_v4().to_string(),
        source: ImageSource::File,
        rel_path: Some(rel_path.to_string()),
        mime: prepared.mime.clone(),
        bytes: prepared.bytes,
        width: prepared.width,
        height: prepared.height,
        sha256: prepared.sha256.clone(),
        source_url: redact_source_url(source_url),
        caption: caption.map(|c| c.to_string()),
        created_at: Utc::now(),
    })
}

/// 落盘：把字节写到 `workspace/images/<artifact_id>/<image_id>.<ext>`，
/// 返回相对 `workspace` 的路径。
///
/// 与 `session::atomic_write` 同口径：显式收目录参数（不依赖 AppHandle），
/// 便于在临时目录里做**真实读写**单测。流程：
/// id 形态 → 扩展名白名单 → rel_path 白名单 → 建目录 → canonicalize+前缀校验
/// → 原子写（tmp + rename）→ 落盘后二次前缀校验（软链接替换场景）。
/// 任一环节失败都不会留下临时文件。
pub fn write_image_file(
    workspace: &Path,
    artifact_id: &str,
    image_id: &str,
    ext: &str,
    bytes: &[u8],
) -> Result<String, ImageError> {
    validate_id(artifact_id)?;
    validate_id(image_id)?;
    if !IMAGE_MIME_EXT.iter().any(|(_, e)| *e == ext) {
        return Err(ImageError::MimeNotAllowed);
    }
    let rel = format!("images/{artifact_id}/{image_id}.{ext}");
    validate_rel_path(&rel)?;

    let root = workspace.join("images");
    let target = workspace.join(&rel);
    let parent = target.parent().ok_or(ImageError::PathEscape)?;
    fs::create_dir_all(parent).map_err(|e| ImageError::Io(format!("创建图片目录失败: {e}")))?;

    let root_canon =
        fs::canonicalize(&root).map_err(|e| ImageError::Io(format!("图片根目录不可用: {e}")))?;
    let parent_canon =
        fs::canonicalize(parent).map_err(|e| ImageError::Io(format!("图片目录不可用: {e}")))?;
    if !parent_canon.starts_with(&root_canon) {
        return Err(ImageError::PathEscape);
    }

    let tmp = target.with_extension(format!("{ext}.tmp"));
    if let Err(e) = fs::write(&tmp, bytes) {
        return Err(ImageError::Io(format!("写临时文件失败: {e}")));
    }
    if let Err(e) = fs::rename(&tmp, &target) {
        let _ = fs::remove_file(&tmp);
        return Err(ImageError::Io(format!("图片落盘失败: {e}")));
    }
    if let Ok(final_canon) = fs::canonicalize(&target) {
        if !final_canon.starts_with(&root_canon) {
            let _ = fs::remove_file(&target);
            return Err(ImageError::PathEscape);
        }
    }
    Ok(rel)
}

/// 删除某成果的图片目录（幂等：不存在返回 0）。
/// 删除前同样 canonicalize + 前缀校验，且**拒绝删除 images 根目录本身**。
pub fn remove_artifact_images(workspace: &Path, artifact_id: &str) -> Result<usize, ImageError> {
    validate_id(artifact_id)?;
    let root = workspace.join("images");
    let dir = root.join(artifact_id);
    if !dir.exists() {
        return Ok(0);
    }
    let root_canon =
        fs::canonicalize(&root).map_err(|e| ImageError::Io(format!("图片根目录不可用: {e}")))?;
    let dir_canon =
        fs::canonicalize(&dir).map_err(|e| ImageError::Io(format!("图片目录不可用: {e}")))?;
    if !dir_canon.starts_with(&root_canon) || dir_canon == root_canon {
        return Err(ImageError::PathEscape);
    }
    let count = fs::read_dir(&dir).map(|e| e.flatten().count()).unwrap_or(0);
    fs::remove_dir_all(&dir).map_err(|e| ImageError::Io(format!("删除图片目录失败: {e}")))?;
    Ok(count)
}

/// 带内嵌占位数据的内联型 `ImageRef`（`source = InlineDataUrl`，无独立文件）。
/// 用于记录「采集时已内联进 HTML」的图片，避免与落盘图片重复占空间。
/// （契约预留，M2-2 接入 `collect.js` 内联通道后消费；已有单测守护）
#[allow(dead_code)]
pub fn build_inline_ref(
    prepared: &PreparedImage,
    source_url: Option<&str>,
    caption: Option<&str>,
) -> ImageRef {
    ImageRef {
        id: uuid::Uuid::new_v4().to_string(),
        source: ImageSource::InlineDataUrl,
        rel_path: None,
        mime: prepared.mime.clone(),
        bytes: prepared.bytes,
        width: prepared.width,
        height: prepared.height,
        sha256: prepared.sha256.clone(),
        source_url: redact_source_url(source_url),
        caption: caption.map(|c| c.to_string()),
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod image_contract_tests {
    use super::*;
    use crate::domain::{
        Artifact, IMAGE_INLINE_MAX_BYTES, IMAGE_MAX_BYTES, IMAGE_MAX_COUNT, IMAGE_MAX_TOTAL_BYTES,
    };

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut v = vec![0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        v.extend_from_slice(&[0; 8]); // 8..16 长度/IHDR 标记（尺寸解析只取 16..24）
        v.extend_from_slice(&width.to_be_bytes());
        v.extend_from_slice(&height.to_be_bytes());
        v.extend_from_slice(&[0; 8]);
        v
    }

    fn gif(width: u16, height: u16) -> Vec<u8> {
        let mut v = b"GIF89a".to_vec();
        v.extend_from_slice(&width.to_le_bytes());
        v.extend_from_slice(&height.to_le_bytes());
        v.extend_from_slice(&[0; 8]);
        v
    }

    fn jpeg(width: u16, height: u16) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0];
        v.extend_from_slice(&[0x00, 0x04]); // APP0 段长
        v.extend_from_slice(b"JFIF");
        v.extend_from_slice(&[0xFF, 0xC0]); // SOF0
        v.extend_from_slice(&[0x00, 0x0B]); // 段长 11
        v.push(0x08);
        v.extend_from_slice(&height.to_be_bytes());
        v.extend_from_slice(&width.to_be_bytes());
        v.extend_from_slice(&[0x01, 0x01, 0x11, 0x00]);
        v
    }

    fn webp_vp8x(width: u32, height: u32) -> Vec<u8> {
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        v.extend_from_slice(b"WEBP");
        v.extend_from_slice(b"VP8X");
        v.extend_from_slice(&[0x0a, 0x00, 0x00, 0x00]); // chunk size（16..20）
        v.extend_from_slice(&[0x00; 4]); // flags(1) + reserved(3) → 20..24
        v.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
        v.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
        v
    }

    fn webp_vp8l(width: u32, height: u32) -> Vec<u8> {
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        v.extend_from_slice(b"WEBP");
        v.extend_from_slice(b"VP8L");
        v.extend_from_slice(&[0x05, 0x00, 0x00, 0x00]); // chunk size
        v.push(0x2f); // 无损签名
                      // 14 位宽-1 + 14 位高-1（小端位流）
        let bits: u32 = ((width - 1) & 0x3FFF) | (((height - 1) & 0x3FFF) << 14);
        v.extend_from_slice(&bits.to_le_bytes()[..4]);
        v
    }

    fn img(id: &str, sha: &str, bytes: u64) -> ImageRef {
        ImageRef {
            id: id.to_string(),
            source: ImageSource::File,
            rel_path: Some(format!("images/a/{id}.png")),
            mime: "image/png".to_string(),
            bytes,
            width: Some(1),
            height: Some(1),
            sha256: sha.to_string(),
            source_url: None,
            caption: None,
            created_at: Utc::now(),
        }
    }

    // T-img-1：MIME 白名单与扩展名反查（SVG 与未知类型 fail-closed）
    #[test]
    fn mime_whitelist_is_fail_closed() {
        assert_eq!(ext_for_mime("image/png"), Some("png"));
        assert_eq!(ext_for_mime("image/jpeg"), Some("jpg"));
        assert_eq!(ext_for_mime("image/webp"), Some("webp"));
        assert_eq!(ext_for_mime("image/gif"), Some("gif"));
        assert_eq!(
            ext_for_mime("image/svg+xml"),
            None,
            "SVG 必须拒绝（XSS 面）"
        );
        assert_eq!(ext_for_mime("image/bmp"), None, "首期不开 BMP");
        assert_eq!(ext_for_mime("text/html"), None);
        assert!(!is_allowed_mime("image/svg+xml"));
    }

    // T-img-2：魔法字节识别 + 声明与实际不符（N2：a.png 内容是 html）
    #[test]
    fn magic_bytes_must_match_declared_mime() {
        assert_eq!(detect_mime(&png(4, 4)), Some("image/png"));
        assert_eq!(detect_mime(&gif(4, 4)), Some("image/gif"));
        assert_eq!(detect_mime(&jpeg(4, 4)), Some("image/jpeg"));
        assert_eq!(detect_mime(&webp_vp8x(4, 4)), Some("image/webp"));
        assert_eq!(detect_mime(b"<html><body>x</body></html>"), None);

        let err = prepare(b"<html>not an image</html>", "image/png").unwrap_err();
        assert_eq!(err, ImageError::MimeMagicMismatch, "N2 必须报魔法字节不符");
        let err = prepare(&png(2, 2), "image/svg+xml").unwrap_err();
        assert_eq!(err, ImageError::MimeNotAllowed, "N1 SVG 必须拒绝");
    }

    // T-img-3：尺寸解析（四种容器）；解析不出 → None（不伪造）
    #[test]
    fn dimensions_are_parsed_or_left_unknown() {
        assert_eq!(
            image_dimensions(&png(640, 480), "image/png"),
            Some((640, 480))
        );
        assert_eq!(image_dimensions(&gif(32, 24), "image/gif"), Some((32, 24)));
        assert_eq!(
            image_dimensions(&jpeg(800, 600), "image/jpeg"),
            Some((800, 600))
        );
        assert_eq!(
            image_dimensions(&webp_vp8x(1024, 768), "image/webp"),
            Some((1024, 768))
        );
        assert_eq!(
            image_dimensions(&webp_vp8l(300, 200), "image/webp"),
            Some((300, 200))
        );
        // 截断文件：尺寸解不出 → None，而不是 0
        assert_eq!(image_dimensions(&png(640, 480)[..12], "image/png"), None);
        let prepared = prepare(&png(640, 480)[..20], "image/png").expect("容器合法但尺寸不可解析");
        assert_eq!(prepared.width, None, "解不出尺寸必须留空，不得填 0");
    }

    // T-img-4：N6 最长边超限；N5 空文件；N4 超单张上限
    #[test]
    fn size_and_dimension_limits_are_enforced() {
        assert_eq!(check_size(0), Err(ImageError::ImageEmpty), "N5 空文件");
        assert_eq!(
            check_size(IMAGE_MAX_BYTES + 1),
            Err(ImageError::ImageTooLarge),
            "N4 超限且不得先落盘"
        );
        assert!(check_size(1024).is_ok());

        let err = prepare(&png(9000, 9000), "image/png").unwrap_err();
        assert_eq!(
            err,
            ImageError::ImageDimensionExceeded,
            "N6 9000px 必须拒绝"
        );
        assert!(prepare(&png(8000, 8000), "image/png").is_ok(), "边界内放行");
    }

    // T-img-5：内联上限（超出必须落盘）
    #[test]
    fn inline_limit_is_enforced() {
        assert!(check_inline_size(1024).is_ok());
        assert_eq!(
            check_inline_size(IMAGE_INLINE_MAX_BYTES + 1),
            Err(ImageError::InlineTooLarge)
        );
        assert_eq!(check_inline_size(0), Err(ImageError::ImageEmpty));
    }

    // T-img-6：N3 路径逃逸与 rel_path 白名单
    #[test]
    fn rel_path_must_not_escape() {
        for bad in [
            "../../../../etc/passwd",
            "/etc/passwd",
            "images/../../x.png",
            "images/./x.png",
            "images\\x.png",
            "images/",
            "",
            "images/a/b\0c.png",
        ] {
            assert!(validate_rel_path(bad).is_err(), "必须拒绝逃逸路径: {bad:?}");
        }
        assert_eq!(
            validate_rel_path("../../etc/passwd").unwrap_err(),
            ImageError::PathEscape
        );
        assert!(validate_rel_path("images/abc-def_1/a.b.png").is_ok());
    }

    // T-img-7：id 形态校验（拼接路径前拦截；M1-ACCEPT NON-BLOCKER 加固）
    #[test]
    fn id_must_be_path_safe() {
        let uuid = uuid::Uuid::new_v4().to_string();
        assert!(validate_id(&uuid).is_ok());
        for bad in [
            "../../etc/passwd",
            "a/b",
            "a\\b",
            "..",
            "",
            "a".repeat(IMAGE_ID_MAX_LEN + 1).as_str(),
            "with space",
            "a.b",
        ] {
            assert!(validate_id(bad).is_err(), "必须拒绝非法 id: {bad:?}");
        }
    }

    // T-img-8：N7 去重（同 sha256 复用，不重复写盘）
    #[test]
    fn duplicate_hash_is_reused() {
        let existing = vec![img("1", "abc123", 10)];
        let hit = find_duplicate(&existing, "abc123");
        assert!(hit.is_some(), "同 sha256 应命中");
        assert_eq!(hit.unwrap().id, "1");
        assert!(find_duplicate(&existing, "nope").is_none());
    }

    // T-img-9：数量与总字节配额（写入前判定）
    #[test]
    fn quota_is_checked_before_write() {
        let full: Vec<ImageRef> = (0..IMAGE_MAX_COUNT)
            .map(|i| img(&i.to_string(), &format!("sha{i}"), 10))
            .collect();
        assert_eq!(check_quota(&full, 10), Err(ImageError::ImageCountExceeded));
        let heavy = vec![img("big", "sha-big", IMAGE_MAX_TOTAL_BYTES)];
        assert_eq!(check_quota(&heavy, 1), Err(ImageError::ImageBudgetExceeded));
        assert!(check_quota(&[], IMAGE_MAX_TOTAL_BYTES).is_ok());
    }

    // T-img-10：溯源 URL 脱敏（敏感查询值 → ***）
    #[test]
    fn source_url_is_redacted() {
        let out = redact_source_url(Some("https://ex.com/a.png?token=sekrit&x=1"));
        let out = out.expect("有值");
        assert!(!out.contains("sekrit"), "token 原值不得入库: {out}");
        assert!(out.contains("***"), "应保留脱敏形态: {out}");
        assert!(redact_source_url(None).is_none());
    }

    // T-img-11：ImageRef 序列化不含 headers/cookie/authorization/body（结构性红线）
    #[test]
    fn image_ref_has_no_sensitive_fields() {
        let prepared = prepare(&png(2, 2), "image/png").expect("合法 PNG");
        let r = build_file_ref(
            &prepared,
            "images/aid/iid.png",
            Some("https://ex.com/a.png?token=zz"),
            Some("说明"),
        )
        .expect("rel_path 合法");
        let json = serde_json::to_string(&r).expect("serialize");
        for bad in [
            "cookie",
            "authorization",
            "set-cookie",
            "headers",
            "body",
            "zz",
        ] {
            assert!(
                !json.to_ascii_lowercase().contains(bad),
                "ImageRef 不得含 {bad}: {json}"
            );
        }
        for key in [
            "\"id\"",
            "\"source\"",
            "\"rel_path\"",
            "\"mime\"",
            "\"bytes\"",
            "\"sha256\"",
            "\"source_url\"",
            "\"caption\"",
        ] {
            assert!(json.contains(key), "ImageRef 缺字段 {key}");
        }
        // 内联型：无独立文件，rel_path 必须为 null
        let inline = build_inline_ref(&prepared, None, None);
        assert_eq!(inline.source, ImageSource::InlineDataUrl);
        assert_eq!(inline.rel_path, None);
    }

    // T-img-12：N10 历史成果 JSON 无 images 字段 → 正常加载且 images == []
    // （`#[serde(default)]` 缺失会让 load_artifacts 静默丢弃全部历史成果）
    #[test]
    fn legacy_artifact_without_images_still_loads() {
        let legacy = r#"{
            "id": "6f1d2c3e-0000-4000-8000-000000000000",
            "title": "旧成果",
            "source_url": "https://ex.com/p",
            "text": "正文",
            "html": "<p>正文</p>",
            "hash": "deadbeef",
            "created_at": "2026-09-01T00:00:00Z",
            "tags": ["x"]
        }"#;
        let art: Artifact = serde_json::from_str(legacy).expect("历史成果必须能加载");
        assert!(art.images.is_empty(), "缺字段应默认空: {:?}", art.images);

        let fresh = Artifact::new("t".into(), "u".into(), "text".into(), "html".into());
        assert!(fresh.images.is_empty());
    }

    // ---- 真实磁盘用例（临时目录，落盘/清理全程可验） ----

    fn temp_workspace(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "m2-1-{}-{}-{}",
            tag,
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建临时工作区");
        dir
    }

    // T-img-14：落盘成功——文件在 images/<artifact_id>/ 下、无 .tmp 残留、内容一致
    #[test]
    fn write_image_file_persists_atomically() {
        let ws = temp_workspace("write");
        let artifact_id = uuid::Uuid::new_v4().to_string();
        let image_id = uuid::Uuid::new_v4().to_string();
        let payload = png(8, 8);
        let rel = write_image_file(&ws, &artifact_id, &image_id, "png", &payload).expect("落盘");
        assert_eq!(rel, format!("images/{artifact_id}/{image_id}.png"));
        let file = ws.join(&rel);
        assert_eq!(fs::read(&file).expect("读回"), payload, "内容必须一致");
        let leftovers: Vec<String> = fs::read_dir(ws.join("images").join(&artifact_id))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "原子写后不得残留 tmp: {leftovers:?}");
        let _ = fs::remove_dir_all(&ws);
    }

    // T-img-15：非法 id / 非白名单扩展名 → 拒绝且**磁盘不产生任何文件**
    #[test]
    fn write_image_file_rejects_before_touching_disk() {
        let ws = temp_workspace("reject");
        let payload = png(4, 4);
        let image_id = uuid::Uuid::new_v4().to_string();
        for bad_id in ["../../etc", "a/b", "..", ""] {
            assert!(
                write_image_file(&ws, bad_id, &image_id, "png", &payload).is_err(),
                "必须拒绝非法 artifact_id: {bad_id:?}"
            );
        }
        assert_eq!(
            write_image_file(&ws, "ok-id", &image_id, "svg", &payload).unwrap_err(),
            ImageError::MimeNotAllowed,
            "扩展名必须来自白名单反查"
        );
        assert_eq!(
            write_image_file(&ws, "ok-id", "../evil", "png", &payload).unwrap_err(),
            ImageError::InvalidId
        );
        assert!(
            !ws.join("images").exists(),
            "被拒绝的写入不得创建任何目录/文件"
        );
        let _ = fs::remove_dir_all(&ws);
    }

    // T-img-16：N8 删除联动——目录随成果删除消失；重复删除幂等返回 0
    #[test]
    fn remove_artifact_images_is_idempotent() {
        let ws = temp_workspace("remove");
        let artifact_id = uuid::Uuid::new_v4().to_string();
        let image_id = uuid::Uuid::new_v4().to_string();
        write_image_file(&ws, &artifact_id, &image_id, "png", &png(2, 2)).expect("落盘");
        assert_eq!(
            remove_artifact_images(&ws, &artifact_id).expect("删除"),
            1,
            "应删除 1 个文件"
        );
        assert!(
            !ws.join("images").join(&artifact_id).exists(),
            "图片目录必须随成果删除"
        );
        assert_eq!(
            remove_artifact_images(&ws, &artifact_id).expect("重复删除"),
            0,
            "重复删除必须幂等"
        );
        // 拒绝删除 images 根目录本身
        assert_eq!(
            remove_artifact_images(&ws, "..").unwrap_err(),
            ImageError::InvalidId
        );
        let _ = fs::remove_dir_all(&ws);
    }

    // T-img-17：完整保存流水线（校验 → 去重 → 配额 → 落盘 → 引用）——不依赖 AppHandle
    #[test]
    fn save_pipeline_enforces_dedupe_and_quota() {
        let ws = temp_workspace("pipeline");
        let artifact_id = uuid::Uuid::new_v4().to_string();
        let payload = png(16, 16);
        let prepared = prepare(&payload, "image/png").expect("合法 PNG");

        // 首次落盘
        let image_id = uuid::Uuid::new_v4().to_string();
        let rel =
            write_image_file(&ws, &artifact_id, &image_id, prepared.ext, &payload).expect("落盘");
        let first = build_file_ref(&prepared, &rel, Some("https://ex.com/a.png?token=zz"), None)
            .expect("构造引用");
        let mut images = vec![first.clone()];

        // 同一张图再来一次：命中 sha256 去重，不再写盘
        let prepared2 = prepare(&payload, "image/png").expect("合法 PNG");
        assert!(
            find_duplicate(&images, &prepared2.sha256).is_some(),
            "应命中去重"
        );
        let files_after = fs::read_dir(ws.join("images").join(&artifact_id))
            .unwrap()
            .flatten()
            .count();
        assert_eq!(files_after, 1, "去重后磁盘不得新增文件");

        // 数量配额：填满 IMAGE_MAX_COUNT 后拒绝
        images = (0..IMAGE_MAX_COUNT)
            .map(|i| img(&format!("i{i}"), &format!("sha{i}"), 10))
            .collect();
        assert_eq!(
            check_quota(&images, 10),
            Err(ImageError::ImageCountExceeded)
        );
        let _ = fs::remove_dir_all(&ws);
    }

    // T-img-18：落盘目录逃逸——canonicalize 前缀校验（软链接指向工作区外）
    #[test]
    fn symlinked_image_dir_cannot_escape() {
        let ws = temp_workspace("symlink");
        let outside = std::env::temp_dir().join(format!("m2-1-outside-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&outside).expect("创建外部目录");
        let artifact_id = uuid::Uuid::new_v4().to_string();
        let link = ws.join("images").join(&artifact_id);
        fs::create_dir_all(ws.join("images")).expect("建 images");
        if std::os::unix::fs::symlink(&outside, &link).is_err() {
            let _ = fs::remove_dir_all(&ws);
            let _ = fs::remove_dir_all(&outside);
            return; // 平台不支持软链接：如实跳过，不伪造结论
        }
        let image_id = uuid::Uuid::new_v4().to_string();
        let err = write_image_file(&ws, &artifact_id, &image_id, "png", &png(2, 2));
        assert_eq!(err, Err(ImageError::PathEscape), "软链接逃逸必须被拦下");
        let _ = fs::remove_dir_all(&ws);
        let _ = fs::remove_dir_all(&outside);
    }

    // T-img-13：错误码稳定（前端据此选择降级文案，不得随意改名）
    #[test]
    fn error_codes_match_frozen_contract() {
        assert_eq!(ImageError::MimeNotAllowed.code(), "MIME_NOT_ALLOWED");
        assert_eq!(ImageError::MimeMagicMismatch.code(), "MIME_MAGIC_MISMATCH");
        assert_eq!(ImageError::PathEscape.code(), "PATH_ESCAPE");
        assert_eq!(ImageError::ImageTooLarge.code(), "IMAGE_TOO_LARGE");
        assert_eq!(ImageError::ImageEmpty.code(), "IMAGE_EMPTY");
        assert_eq!(
            ImageError::ImageDimensionExceeded.code(),
            "IMAGE_DIMENSION_EXCEEDED"
        );
        assert_eq!(
            ImageError::ImageCountExceeded.code(),
            "IMAGE_COUNT_EXCEEDED"
        );
        assert_eq!(
            ImageError::ImageBudgetExceeded.code(),
            "IMAGE_BUDGET_EXCEEDED"
        );
        assert_eq!(ImageError::InlineTooLarge.code(), "INLINE_TOO_LARGE");
    }

    // T-img-14（M2-2.b）：预览路径拼接只允许「基准 + 白名单相对路径」。
    // 前端拿到的是绝对路径字符串，一旦这里放行 `..` 或绝对路径，
    // `asset://` 就会读到图片目录之外的文件。
    #[test]
    fn join_image_path_only_allows_whitelisted_rel() {
        let dir = "/home/u/.local/share/com.jizhijiandan.mvp/mvp-browser-os/workspace/images";
        assert_eq!(
            join_image_path(dir, "images/a1/b2.png"),
            Ok(format!("{dir}/images/a1/b2.png"))
        );
        // 目录基准尾部斜杠不产生双斜杠
        assert_eq!(
            join_image_path(&format!("{dir}/"), "images/a1/b2.png"),
            Ok(format!("{dir}/images/a1/b2.png"))
        );
        for bad in [
            "../secret.png",
            "/etc/passwd",
            "images/../../escape.png",
            "images//double.png",
            "images\\win.png",
            "",
        ] {
            assert_eq!(
                join_image_path(dir, bad),
                Err(ImageError::PathEscape),
                "越界相对路径必须被拒绝: {bad}"
            );
        }
    }

    // T-img-15（M2-2.b）：目录基准缺失时不得退化成相对路径（fail-closed）。
    #[test]
    fn join_image_path_requires_dir_base() {
        assert_eq!(
            join_image_path("", "images/a1/b2.png"),
            Err(ImageError::PathEscape)
        );
        assert_eq!(
            join_image_path("/", "images/a1/b2.png"),
            Err(ImageError::PathEscape)
        );
    }
}
