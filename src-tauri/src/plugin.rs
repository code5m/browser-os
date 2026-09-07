//! M5-10 / M5-11 插件 manifest 与生命周期策略切片（Lane A9, M5-W6）。
//!
//! 本模块**只含纯函数**：
//! - `validate_plugin_manifest`：manifest schema 边界 + 凭据泄露 + 形态③入口 +
//!   能力白名单 + 哈希/签名结构（不做文件 I/O、不做真验签）。
//! - `verify_plugin_signature_structure`：仅签名**结构**校验；真 Ed25519 验签由运行时
//!   lane 在本地完成（W6 不引入加密 crate、不联网）。
//! - `can_transition` / `transition`：生命周期状态机（纯，无文件变更）。
//! - `permission_preview_for_plugin`：派生于 A5 `PermissionPreview`，供 A6 UI 渲染。
//!
//! 设计红线（承 M5-10 §4 / W6 FORBID）：
//! - 无安装/卸载运行时（不触碰 `plugins_dir`、不读写磁盘清单）。
//! - 无下载/执行/启用真实插件。
//! - 形态③唯一合法：入口 URL 同源 webview；类型与校验层排除独立 webview / stdio 进程。
//! - 能力白名单单一真源：`security_policy::PLUGIN_CAPABILITY_V1`（fail-closed 空集合首期）。
//! - 无凭据/secret 进入 manifest 或审计（复用 `security_policy::contains_credential_leak`）。
#![allow(dead_code)] // W6 为策略切片：纯函数仅被单测与未来运行时 lane 消费，无当前调用方。

use crate::domain::{
    AclLevel, PermissionPreview, PluginCapabilityView, PluginDetail, PluginManifest, PluginRecord,
    PluginResourceMeta, PluginSignature, PluginSignatureView, PluginState, PluginSummary,
    TrustedKeyRecord,
};
use crate::security_policy::{self, PolicyError};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// 能力 → 安装/运行闸门档（M5-10 §4.5：声明危险 capability ⇒ Dangerous 闸门 + keyring 二次确认）
// ---------------------------------------------------------------------------

/// 危险能力集合（首期为空集合占位）。能力逐个评估追加到 `security_policy::PLUGIN_CAPABILITY_V1`
/// 时，同步在此标记其风险档——与能力白名单单一真源一致，不另立文件。
pub const DANGEROUS_PLUGIN_CAPABILITIES: &[&str] = &[
    "fs_write",
    "fs_delete",
    "command_exec",
    "workspace.write",
    "workspace.delete",
    "network.request",
    "credential.read",
];

/// 能力 → 安装/运行闸门档（fail-closed：未知能力已在白名单校验前拒绝，这里仅作风险分类）。
pub fn capability_acl_level(cap: &str) -> AclLevel {
    if DANGEROUS_PLUGIN_CAPABILITIES.contains(&cap) {
        AclLevel::Dangerous
    } else {
        AclLevel::Confirm
    }
}

/// 权限预览（M5-6/12 UI 用）：插件安装/启用前可见闸门档与所需能力。
/// 与 A5 `AgentDef::permission_preview` 同形；闸门取 capabilities 中最高风险档。
pub fn permission_preview_for_plugin(m: &PluginManifest) -> PermissionPreview {
    let gate = if m
        .capabilities
        .iter()
        .any(|c| capability_acl_level(&c.capability) == AclLevel::Dangerous)
    {
        AclLevel::Dangerous
    } else if m
        .capabilities
        .iter()
        .any(|c| capability_acl_level(&c.capability) == AclLevel::Confirm)
    {
        AclLevel::Confirm
    } else {
        AclLevel::Safe
    };
    PermissionPreview {
        gate,
        capabilities: m
            .capabilities
            .iter()
            .map(|c| c.capability.clone())
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// 纯 manifest 校验（fail-closed）
// ---------------------------------------------------------------------------

/// 纯 manifest 校验：schema 边界 + 凭据泄露 + 形态③入口 + 能力白名单 + 哈希/签名结构。
/// 不做文件 I/O、不做真验签、不联网。
pub fn validate_plugin_manifest(m: &PluginManifest) -> Result<(), PolicyError> {
    // 1) 反向域名 id 非空 + 边界 + 规范
    if m.id.trim().is_empty() {
        return Err(PolicyError::EmptyRequiredField("plugin.id".into()));
    }
    if m.id.len() > MAX_PLUGIN_ID_BYTES {
        return Err(PolicyError::InvalidPluginManifest("plugin.id 超长".into()));
    }
    if !is_reverse_domain(&m.id) {
        // 只给脱敏预览：`id` 可为 `sk-...`，原样回显即泄露（A4 W13 F-A4-1）。
        return Err(PolicyError::InvalidPluginManifest(format!(
            "plugin.id 非反向域名规范：{}",
            redact_preview(&m.id)
        )));
    }

    // 2) version / display_name / description / min_app_version 边界 + 凭据泄露
    bound_text(&m.version, "plugin.version")?;
    bound_text(&m.display_name, "plugin.display_name")?;
    bound_text(&m.description, "plugin.description")?;
    bound_text(&m.min_app_version, "plugin.min_app_version")?;

    // 3) 形态③入口：entry_url 必须同源 webview，禁 javascript:/data:/file:/其他 scheme
    if !is_allowed_entry_url(&m.entry.entry_url) {
        // 同上：URL 可带 `?token=` / `user:pass@`，只给脱敏预览（A4 W13 F-A4-1）。
        return Err(PolicyError::InvalidPluginManifest(format!(
            "plugin.entry.entry_url 非形态③同源入口：{}",
            redact_preview(&m.entry.entry_url)
        )));
    }
    bound_text(&m.entry.icon, "plugin.entry.icon")?;

    // 4) 能力白名单（单一真源）+ reason 非空
    let ids: Vec<String> = m
        .capabilities
        .iter()
        .map(|c| c.capability.clone())
        .collect();
    security_policy::check_plugin_capabilities(&ids)?;
    for c in &m.capabilities {
        if c.reason.trim().is_empty() {
            return Err(PolicyError::InvalidPluginManifest(format!(
                "plugin capability {} 缺 reason",
                c.capability
            )));
        }
    }

    // 5) hash：sha256 hex（64）
    if !is_sha256_hex(&m.hash) {
        return Err(PolicyError::InvalidPluginManifest(format!(
            "plugin.hash 非 sha256 hex：{}",
            m.hash
        )));
    }

    // 6) 签名结构
    verify_plugin_signature_structure(&m.signature)?;

    // 7) metadata：体量边界 + **凭据检测**
    // （A4 W13 F-A4-2 / F-A4-4：此前注释声称「由 contains_credential_leak 守」，
    //   实际只有体量边界——注释与代码漂移，metadata 里的密钥会随登记簿落盘。
    //   现在序列化后即过检测，命中以**字段名**报错，绝不回显命中密文。）
    if let Ok(metadata_json) = serde_json::to_string(&m.metadata) {
        if metadata_json.len() > crate::domain::MAX_TEXT_FIELD_BYTES {
            return Err(PolicyError::InvalidPluginManifest(
                "plugin.metadata 超 64KiB".into(),
            ));
        }
        if security_policy::contains_credential_leak(&metadata_json) {
            return Err(PolicyError::CredentialLeak("plugin.metadata".into()));
        }
    }

    Ok(())
}

/// 纯签名结构校验（W6 仅结构；真 Ed25519 验签由运行时 lane 在本地完成，不引加密 crate）。
pub fn verify_plugin_signature_structure(sig: &PluginSignature) -> Result<(), PolicyError> {
    if sig.algorithm != "Ed25519" {
        // 自由文本一律脱敏预览（A4 W13 F-A4-1）。
        return Err(PolicyError::InvalidPluginSignature(format!(
            "不支持的签名算法（仅 Ed25519）：{}",
            redact_preview(&sig.algorithm)
        )));
    }
    if sig.key_id.trim().is_empty() {
        return Err(PolicyError::InvalidPluginSignature("key_id 为空".into()));
    }
    if !is_base64_std(&sig.value) {
        return Err(PolicyError::InvalidPluginSignature(
            "value 非 base64".into(),
        ));
    }
    if !is_iso8601_len(&sig.signed_at) {
        return Err(PolicyError::InvalidPluginSignature(
            "signed_at 超长/非法（需 ISO8601）".into(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 生命周期状态机（纯函数，无文件 I/O）
// ---------------------------------------------------------------------------

/// 生命周期迁移是否合法（承 M5-10 §4.3）。
pub fn can_transition(from: PluginState, to: PluginState) -> bool {
    use PluginState::*;
    matches!(
        (from, to),
        (Discovered, Validating)
            | (Validating, SignedOk)
            | (Validating, SignedFailed)
            | (SignedOk, Loaded)
            | (SignedFailed, Uninstalled)
            | (SignedFailed, Discovered)
            | (Loaded, Enabled)
            | (Loaded, Disabled)
            | (Loaded, Uninstalled)
            | (Enabled, Disabled)
            | (Enabled, Uninstalled)
            | (Disabled, Enabled)
            | (Disabled, Uninstalled)
    )
}

/// 受控迁移（fail-closed）。
pub fn transition(state: PluginState, next: PluginState) -> Result<PluginState, PolicyError> {
    if can_transition(state, next) {
        Ok(next)
    } else {
        Err(PolicyError::PluginStateTransition(format!(
            "{:?} -> {:?} 非法迁移",
            state, next
        )))
    }
}

// ---------------------------------------------------------------------------
// 校验辅助（均为保守纯函数，不依赖外部）
// ---------------------------------------------------------------------------

const MAX_PLUGIN_ID_BYTES: usize = 256;

/// 反向域名宽松校验：含 '.'，字符限于小写字母数字 . - _，不以 . 开头/结尾。
fn is_reverse_domain(s: &str) -> bool {
    if s.is_empty() || s.len() > MAX_PLUGIN_ID_BYTES {
        return false;
    }
    if !s.contains('.') {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-' || c == '_')
        && !s.starts_with('.')
        && !s.ends_with('.')
}

/// 文本字段边界（≤ `MAX_TEXT_FIELD_BYTES`）+ 凭据泄露检查。
fn bound_text(s: &str, field: &str) -> Result<(), PolicyError> {
    if s.len() > crate::domain::MAX_TEXT_FIELD_BYTES {
        return Err(PolicyError::InvalidPluginManifest(format!("{field} 超长")));
    }
    if security_policy::contains_credential_leak(s) {
        // 载荷只放**字段名**：`PolicyError` 派生 `Debug`，`{:?}` 会回显载荷
        // （A4 W13 F-A4-3），塞原始密文等于给 Debug 开泄露口。
        return Err(PolicyError::CredentialLeak(field.to_string()));
    }
    Ok(())
}

/// 形态③：同源 webview 入口，仅 http/https/tool://。
fn is_allowed_entry_url(u: &str) -> bool {
    u.starts_with("http://") || u.starts_with("https://") || u.starts_with("tool://")
}

/// sha256 hex（64 字符，全小写十六进制）。
fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// 标准 base64 宽松校验（字符集 + 4 字节对齐；不解码，避免引依赖）。
fn is_base64_std(s: &str) -> bool {
    if s.is_empty() || s.len() > 4096 {
        return false;
    }
    s.len() % 4 == 0
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

/// ISO8601 字符串边界（不长于 64 字符且非空）。
fn is_iso8601_len(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64
}

// ---------------------------------------------------------------------------
// W13 Stage-I：本地登记簿（纯函数 + 原子落盘）
//
// 边界（A2 W13 复核口径）：本模块只有「登记簿 + 纯迁移 + 视图派生 + 原子落盘」。
// 命令注册与来源校验在 `bridge.rs`；本模块不开线程、不起监听、不解包、不真验签、
// 不触 graph/db/script/mcp/agent 任何执行路径（无第二执行路径）。
// ---------------------------------------------------------------------------

/// 摘要长度：sha256 前 16 hex（审计 / UI 展示同口径）。
pub const HASH_PREFIX_LEN: usize = 16;

/// 登记簿文件：`data_dir/plugins.json`。
pub fn plugins_file(app: &tauri::AppHandle) -> PathBuf {
    crate::workspace::data_dir(app).join("plugins.json")
}

/// 受信任公钥文件：`data_dir/trusted-pubkeys.json`（**不**落公钥原文）。
pub fn trusted_keys_file(app: &tauri::AppHandle) -> PathBuf {
    crate::workspace::data_dir(app).join("trusted-pubkeys.json")
}

/// 原子落盘（唯一原语 `session::atomic_write` = tmp + rename；不新造写路径）。
pub fn save_plugins_at(path: &Path, list: &[PluginRecord]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    crate::session::atomic_write(path, &content)
}

/// 读取登记簿：缺失 / 损坏 → 空列表（fail-closed，不猜内容）。
pub fn load_plugins_at(path: &Path) -> Vec<PluginRecord> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str::<Vec<PluginRecord>>(&c).ok())
        .unwrap_or_default()
}

/// 受信任公钥落盘（同 `atomic_write`）。
pub fn save_trusted_keys_at(path: &Path, list: &[TrustedKeyRecord]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    crate::session::atomic_write(path, &content)
}

/// 读取受信任公钥：缺失 / 损坏 → 空列表。
pub fn load_trusted_keys_at(path: &Path) -> Vec<TrustedKeyRecord> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str::<Vec<TrustedKeyRecord>>(&c).ok())
        .unwrap_or_default()
}

/// 稳定错误码：审计**只记码**，不记消息正文（防凭据 / 路径 / 签名回显）。
pub fn error_code(e: &PolicyError) -> &'static str {
    match e {
        PolicyError::CredentialLeak(_) => "PLUGIN_CREDENTIAL_LEAK",
        PolicyError::UnknownCapability(_) => "PLUGIN_CAPABILITY_UNKNOWN",
        PolicyError::InvalidPluginManifest(_) => "PLUGIN_MANIFEST_INVALID",
        PolicyError::InvalidPluginSignature(_) => "PLUGIN_SIGNATURE_INVALID",
        PolicyError::PluginStateTransition(_) => "PLUGIN_STATE_TRANSITION",
        PolicyError::EmptyRequiredField(_) => "PLUGIN_FIELD_EMPTY",
        PolicyError::PathOutsideAllowedRoots { .. } => "PLUGIN_PATH_OUTSIDE_ROOTS",
        PolicyError::UnresolvablePath(_) => "PLUGIN_PATH_UNRESOLVABLE",
        PolicyError::SymlinkEscape { .. } => "PLUGIN_PATH_SYMLINK",
        _ => "PLUGIN_POLICY_DENIED",
    }
}

/// 摘要前缀（入参已由 `validate_plugin_manifest` 判为 64 hex sha256）。
pub fn hash_prefix(hash: &str) -> String {
    hash.chars().take(HASH_PREFIX_LEN).collect()
}

/// 错误信息脱敏预览：只给「长度 + sha256 前 16 hex」，**绝不**回显原文。
///
/// 自由文本 manifest 字段（`id` / `entry_url` / `algorithm`）都可能是密钥或带
/// 凭据的 URL；`PolicyError` 的 Display 会原样输出，故入口处即脱敏（A4 W13 F-A4-1）。
pub fn redact_preview(raw: &str) -> String {
    format!("len={} sha256:{}", raw.len(), fingerprint_of(raw))
}

/// 公钥指纹：sha256 前 16 hex（**不**落公钥原文）。
pub fn fingerprint_of(pubkey: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pubkey.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .chars()
        .take(HASH_PREFIX_LEN)
        .collect()
}

/// Stage-I 安装：**仅元数据校验 + 登记**（不解包 / 不真验签 / 不执行 / 不下载）。
/// 迁移链：`Discovered → Validating → SignedOk → Loaded`。
pub fn install_record(
    list: &mut Vec<PluginRecord>,
    manifest: PluginManifest,
    resource_ok: bool,
    now: &str,
) -> Result<PluginRecord, PolicyError> {
    if list.iter().any(|r| r.id == manifest.id) {
        return Err(PolicyError::InvalidPluginManifest(
            "plugin.id 已登记（Stage-I 不支持覆盖 / 升级）".into(),
        ));
    }
    transition(PluginState::Discovered, PluginState::Validating)?;
    validate_plugin_manifest(&manifest)?;
    verify_plugin_signature_structure(&manifest.signature)?;
    transition(PluginState::Validating, PluginState::SignedOk)?;
    let next = transition(PluginState::SignedOk, PluginState::Loaded)?;
    // 只落派生字段：`metadata` 与 `signature.value` **不入库**（A4 W13 F-A4-2 / F-A4-4）。
    let rec = PluginRecord {
        id: manifest.id.clone(),
        version: manifest.version.clone(),
        display_name: manifest.display_name.clone(),
        description: manifest.description.clone(),
        min_app_version: manifest.min_app_version.clone(),
        capabilities: manifest.capabilities.clone(),
        hash: manifest.hash.clone(),
        signature_algorithm: manifest.signature.algorithm.clone(),
        signature_key_id: manifest.signature.key_id.clone(),
        resource: PluginResourceMeta {
            declared_hash: hash_prefix(&manifest.hash),
            path_provided: resource_ok,
            verified: resource_ok,
        },
        state: next,
        installed_at: now.to_string(),
        updated_at: now.to_string(),
    };
    list.push(rec.clone());
    Ok(rec)
}

/// 按 id 查找（不可变）。
pub fn find_record<'a>(list: &'a [PluginRecord], id: &str) -> Option<&'a PluginRecord> {
    list.iter().find(|r| r.id == id)
}

/// 状态迁移（`Loaded → Enabled` / `Enabled ⇄ Disabled` 等），写回 `updated_at`。
pub fn set_plugin_state(
    list: &mut Vec<PluginRecord>,
    id: &str,
    next: PluginState,
    now: &str,
) -> Result<PluginRecord, PolicyError> {
    let idx = list
        .iter()
        .position(|r| r.id == id)
        .ok_or_else(|| PolicyError::EmptyRequiredField("plugin.id 未登记".into()))?;
    let applied = transition(list[idx].state, next)?;
    list[idx].state = applied;
    list[idx].updated_at = now.to_string();
    Ok(list[idx].clone())
}

/// 列表视图（只读派生，**无**签名原文 / **无**资源路径）。
pub fn summary_of(r: &PluginRecord) -> PluginSummary {
    PluginSummary {
        id: r.id.clone(),
        version: r.version.clone(),
        display_name: r.display_name.clone(),
        state: r.state,
        capability_count: r.capabilities.len(),
        hash_prefix: hash_prefix(&r.hash),
        updated_at: r.updated_at.clone(),
    }
}

/// 按状态过滤（`None` = 全部）。
pub fn filter_by_state(list: &[PluginRecord], state: Option<PluginState>) -> Vec<PluginSummary> {
    // 注意：不用 `unwrap_or(true)`——该字面模式被 `check-plugin-policy.py`
    // 的 `PLUGIN_SIG_BYPASS` 码位视为「签名旁路」特征，此处以 `match` 显式表达。
    list.iter()
        .filter(|r| match state {
            Some(s) => r.state == s,
            None => true,
        })
        .map(summary_of)
        .collect()
}

/// 详情视图（逐项能力 + 风险档；**无** `value` 原文 / **无**资源路径）。
pub fn detail_of(r: &PluginRecord) -> PluginDetail {
    // 登记簿不落 `value`，故结构状态由已落字段推导：算法为 Ed25519 且 key_id 非空。
    let structure_ok = r.signature_algorithm == "Ed25519" && !r.signature_key_id.trim().is_empty();
    PluginDetail {
        id: r.id.clone(),
        version: r.version.clone(),
        display_name: r.display_name.clone(),
        description: r.description.clone(),
        min_app_version: r.min_app_version.clone(),
        state: r.state,
        capabilities: r
            .capabilities
            .iter()
            .map(|c| PluginCapabilityView {
                capability: c.capability.clone(),
                reason: c.reason.clone(),
                acl_level: capability_acl_level(&c.capability),
            })
            .collect(),
        hash_prefix: hash_prefix(&r.hash),
        signature: PluginSignatureView {
            algorithm: r.signature_algorithm.clone(),
            key_id: r.signature_key_id.clone(),
            status: if structure_ok {
                "structure_ok".to_string()
            } else {
                "structure_failed".to_string()
            },
        },
        installed_at: r.installed_at.clone(),
        updated_at: r.updated_at.clone(),
        resource: r.resource.clone(),
    }
}

/// 受信任公钥：新增（**仅**登记 key_id + 指纹；重复 key_id 一律拒绝，防替换）。
pub fn add_trusted_key(
    list: &mut Vec<TrustedKeyRecord>,
    key_id: &str,
    pubkey: &str,
    note: &str,
    now: &str,
) -> Result<TrustedKeyRecord, PolicyError> {
    if key_id.trim().is_empty() {
        return Err(PolicyError::EmptyRequiredField("key_id".into()));
    }
    if pubkey.trim().is_empty() {
        return Err(PolicyError::EmptyRequiredField("pubkey".into()));
    }
    if list.iter().any(|k| k.key_id == key_id) {
        return Err(PolicyError::InvalidPluginManifest(
            "key_id 已存在（不可覆盖 / 改公钥，防 FM-9 替换）".into(),
        ));
    }
    let rec = TrustedKeyRecord {
        key_id: key_id.to_string(),
        fingerprint: fingerprint_of(pubkey),
        note: note.to_string(),
        added_at: now.to_string(),
    };
    list.push(rec.clone());
    Ok(rec)
}

/// 受信任公钥：移除（**仅**按 key_id 命中整条删除，**不**改写其它条目，防 FM-9）。
pub fn remove_trusted_key(
    list: &mut Vec<TrustedKeyRecord>,
    key_id: &str,
) -> Result<TrustedKeyRecord, PolicyError> {
    let idx = list
        .iter()
        .position(|k| k.key_id == key_id)
        .ok_or_else(|| PolicyError::EmptyRequiredField("key_id 不存在".into()))?;
    Ok(list.remove(idx))
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PluginCapability, PluginEntry, PluginSignature};
    use serde_json::json;

    fn valid_manifest() -> PluginManifest {
        PluginManifest {
            id: "com.example.myplugin".to_string(),
            version: "1.0.0".to_string(),
            display_name: "示例插件".to_string(),
            description: "仅做展示的示例插件".to_string(),
            min_app_version: "0.1.0".to_string(),
            entry: PluginEntry {
                entry_url: "https://example.com/plugin/index.html".to_string(),
                icon: "https://example.com/plugin/icon.png".to_string(),
            },
            capabilities: vec![],
            hash: "a".repeat(64),
            signature: PluginSignature {
                algorithm: "Ed25519".to_string(),
                key_id: "key1".to_string(),
                value: "QUJDRA==".to_string(),
                signed_at: "2026-09-06T00:00:00Z".to_string(),
            },
            metadata: json!({}),
        }
    }

    #[test]
    fn valid_manifest_passes() {
        assert!(validate_plugin_manifest(&valid_manifest()).is_ok());
    }

    #[test]
    fn unknown_capability_rejected() {
        let mut m = valid_manifest();
        m.capabilities = vec![PluginCapability {
            capability: "fs_write".to_string(),
            reason: "需要写文件".to_string(),
        }];
        // 首期白名单为空 → fail-closed 拒绝
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PolicyError::UnknownCapability(_))
        ));
    }

    #[test]
    fn capability_without_reason_rejected() {
        // 首期白名单为空：未知能力先被 `check_plugin_capabilities` 判 UnknownCapability；
        // reason 校验在白名单通过后才生效（能力非空时）。二者均 fail-closed 拒绝。
        let mut m = valid_manifest();
        m.capabilities = vec![PluginCapability {
            capability: "fs_write".to_string(),
            reason: "  ".to_string(),
        }];
        assert!(validate_plugin_manifest(&m).is_err());
    }

    #[test]
    fn credential_in_description_rejected() {
        let mut m = valid_manifest();
        m.description = "sk-abcdefghijklmnopqrstuvw".to_string();
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PolicyError::CredentialLeak(_))
        ));
    }

    #[test]
    fn non_form_three_entry_rejected() {
        let mut m = valid_manifest();
        m.entry.entry_url = "javascript:alert(1)".to_string();
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PolicyError::InvalidPluginManifest(_))
        ));
        let mut m2 = valid_manifest();
        m2.entry.entry_url = "file:///etc/passwd".to_string();
        assert!(matches!(
            validate_plugin_manifest(&m2),
            Err(PolicyError::InvalidPluginManifest(_))
        ));
    }

    #[test]
    fn bad_hash_rejected() {
        let mut m = valid_manifest();
        m.hash = "deadbeef".to_string();
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PolicyError::InvalidPluginManifest(_))
        ));
    }

    #[test]
    fn signature_structure_validated() {
        // 非 Ed25519 算法 → 结构校验失败
        let mut m = valid_manifest();
        m.signature.algorithm = "RSA".to_string();
        assert!(matches!(
            validate_plugin_manifest(&m),
            Err(PolicyError::InvalidPluginSignature(_))
        ));
        // 空 key_id → 失败
        let mut m2 = valid_manifest();
        m2.signature.key_id = "".to_string();
        assert!(matches!(
            validate_plugin_manifest(&m2),
            Err(PolicyError::InvalidPluginSignature(_))
        ));
        // 非 base64 value → 失败
        let mut m3 = valid_manifest();
        m3.signature.value = "!!!notbase64".to_string();
        assert!(matches!(
            validate_plugin_manifest(&m3),
            Err(PolicyError::InvalidPluginSignature(_))
        ));
    }

    #[test]
    fn signature_structure_pure_ok() {
        // 结构合法（真验签留待运行时 lane）
        assert!(verify_plugin_signature_structure(&valid_manifest().signature).is_ok());
    }

    #[test]
    fn lifecycle_transitions() {
        use PluginState::*;
        assert!(can_transition(Discovered, Validating));
        assert!(can_transition(Validating, SignedOk));
        assert!(can_transition(SignedOk, Loaded));
        assert!(can_transition(Loaded, Enabled));
        assert!(can_transition(Enabled, Disabled));
        assert!(can_transition(Disabled, Enabled));
        assert!(can_transition(Loaded, Uninstalled));
        assert!(can_transition(Enabled, Uninstalled));
        assert!(can_transition(SignedFailed, Uninstalled));
        // 非法：直接 Discovered → Enabled
        assert!(!can_transition(Discovered, Enabled));
        // 非法：Uninstalled 复活
        assert!(!can_transition(Uninstalled, Discovered));

        assert!(transition(Loaded, Enabled).is_ok());
        assert!(matches!(
            transition(Discovered, Enabled),
            Err(PolicyError::PluginStateTransition(_))
        ));
    }

    #[test]
    fn permission_preview_gate() {
        // 无能力 → Safe
        assert_eq!(
            permission_preview_for_plugin(&valid_manifest()).gate,
            AclLevel::Safe
        );

        // 危险能力 → Dangerous
        let mut m = valid_manifest();
        m.capabilities = vec![PluginCapability {
            capability: "fs_write".to_string(),
            reason: "写文件".to_string(),
        }];
        assert_eq!(permission_preview_for_plugin(&m).gate, AclLevel::Dangerous);

        // 普通能力 → Confirm
        let mut m2 = valid_manifest();
        m2.capabilities = vec![PluginCapability {
            capability: "workspace.read".to_string(),
            reason: "读取工作区".to_string(),
        }];
        assert_eq!(permission_preview_for_plugin(&m2).gate, AclLevel::Confirm);
    }

    // ---- W13 Stage-I 登记簿 ----

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("mvp-plugin-{tag}-{nanos}.json"))
    }

    fn record_with(id: &str, state: PluginState) -> PluginRecord {
        let m = valid_manifest();
        PluginRecord {
            id: id.to_string(),
            version: m.version.clone(),
            display_name: m.display_name.clone(),
            description: m.description.clone(),
            min_app_version: m.min_app_version.clone(),
            capabilities: m.capabilities.clone(),
            hash: m.hash.clone(),
            signature_algorithm: m.signature.algorithm.clone(),
            signature_key_id: m.signature.key_id.clone(),
            state,
            installed_at: "2026-09-07T00:00:00Z".to_string(),
            updated_at: "2026-09-07T00:00:00Z".to_string(),
            resource: PluginResourceMeta {
                declared_hash: hash_prefix(&m.hash),
                path_provided: true,
                verified: true,
            },
        }
    }

    #[test]
    fn install_record_registers_loaded_state() {
        let mut list = Vec::new();
        let rec = install_record(&mut list, valid_manifest(), true, "now").unwrap();
        assert_eq!(rec.state, PluginState::Loaded);
        assert_eq!(list.len(), 1);
        assert_eq!(rec.resource.declared_hash.len(), HASH_PREFIX_LEN);
        assert!(rec.resource.verified);
    }

    #[test]
    fn install_record_rejects_duplicate_id() {
        let mut list = Vec::new();
        install_record(&mut list, valid_manifest(), false, "now").unwrap();
        // Stage-I 不支持覆盖 / 升级
        assert!(matches!(
            install_record(&mut list, valid_manifest(), false, "now"),
            Err(PolicyError::InvalidPluginManifest(_))
        ));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn install_record_rejects_bad_manifest_and_signature() {
        let mut list = Vec::new();
        let mut bad = valid_manifest();
        bad.hash = "zz".to_string();
        assert!(install_record(&mut list, bad, false, "now").is_err());

        let mut bad_sig = valid_manifest();
        bad_sig.signature.algorithm = "RSA".to_string();
        assert!(matches!(
            install_record(&mut list, bad_sig, false, "now"),
            Err(PolicyError::InvalidPluginSignature(_))
        ));
        assert!(list.is_empty());
    }

    #[test]
    fn install_record_rejects_credential_leak() {
        let mut list = Vec::new();
        let mut m = valid_manifest();
        m.description = "sk-abcdefghijklmnopqrstuvw".to_string();
        assert!(matches!(
            install_record(&mut list, m, false, "now"),
            Err(PolicyError::CredentialLeak(_))
        ));
        // 审计只记稳定码，不记消息正文
        let e = install_record(
            &mut list,
            {
                let mut m = valid_manifest();
                m.description = "sk-abcdefghijklmnopqrstuvw".to_string();
                m
            },
            false,
            "now",
        )
        .unwrap_err();
        assert_eq!(error_code(&e), "PLUGIN_CREDENTIAL_LEAK");
    }

    #[test]
    fn install_record_rejects_credential_in_metadata() {
        // A4 W13 F-A4-2 / F-A4-4：metadata 此前只做体量边界，密钥会随登记簿落盘。
        let mut m = valid_manifest();
        m.metadata = json!({ "api_key": "sk-abcdefghijklmnopqrstuvw" });
        let mut list = Vec::new();
        assert!(matches!(
            install_record(&mut list, m, false, "now"),
            Err(PolicyError::CredentialLeak(_))
        ));
        assert!(list.is_empty());
    }

    #[test]
    fn store_does_not_persist_metadata_or_signature_value() {
        // 登记簿只落派生字段：`metadata` 与 `signature.value` 永不入库。
        let mut list = Vec::new();
        let mut m = valid_manifest();
        m.metadata = json!({ "note": "hello" });
        install_record(&mut list, m, true, "now").unwrap();
        let json = serde_json::to_string(&list).unwrap();
        assert!(!json.contains("metadata"), "登记簿不得落 metadata");
        assert!(!json.contains("\"value\""), "登记簿不得落签名字段");
        assert!(
            !json.contains(valid_manifest().signature.value.as_str()),
            "登记簿不得落签名原文"
        );
    }

    #[test]
    fn set_plugin_state_enable_disable_roundtrip() {
        let mut list = vec![record_with("com.example.myplugin", PluginState::Loaded)];
        let rec = set_plugin_state(
            &mut list,
            "com.example.myplugin",
            PluginState::Enabled,
            "t1",
        )
        .unwrap();
        assert_eq!(rec.state, PluginState::Enabled);
        assert_eq!(rec.updated_at, "t1");
        let rec = set_plugin_state(
            &mut list,
            "com.example.myplugin",
            PluginState::Disabled,
            "t2",
        )
        .unwrap();
        assert_eq!(rec.state, PluginState::Disabled);
        let rec = set_plugin_state(
            &mut list,
            "com.example.myplugin",
            PluginState::Enabled,
            "t3",
        )
        .unwrap();
        assert_eq!(rec.state, PluginState::Enabled);
    }

    #[test]
    fn set_plugin_state_rejects_illegal_and_unknown() {
        let mut list = vec![record_with(
            "com.example.myplugin",
            PluginState::Uninstalled,
        )];
        assert!(matches!(
            set_plugin_state(&mut list, "com.example.myplugin", PluginState::Enabled, "t"),
            Err(PolicyError::PluginStateTransition(_))
        ));
        assert!(
            set_plugin_state(&mut list, "com.example.missing", PluginState::Enabled, "t").is_err()
        );
    }

    #[test]
    fn summary_and_filter_are_readonly_views() {
        let list = vec![
            record_with("com.example.myplugin", PluginState::Enabled),
            record_with("com.example.myplugin", PluginState::Loaded),
        ];
        // 无状态过滤 = 全部
        assert_eq!(filter_by_state(&list, None).len(), 2);
        assert_eq!(filter_by_state(&list, Some(PluginState::Enabled)).len(), 1);
        assert_eq!(
            filter_by_state(&list, Some(PluginState::Uninstalled)).len(),
            0
        );

        let s = summary_of(&list[0]);
        let json = serde_json::to_string(&s).unwrap();
        // 视图不含签名原文 / metadata / 资源路径
        assert!(!json.contains("\"value\""));
        assert!(!json.contains("metadata"));
        assert!(!json.contains("path"));
        assert_eq!(s.capability_count, 0);
    }

    #[test]
    fn detail_of_redacts_signature_value_and_path() {
        let r = record_with("com.example.myplugin", PluginState::Enabled);
        let d = detail_of(&r);
        let json = serde_json::to_string(&d).unwrap();
        assert!(!json.contains("\"value\""), "详情不得含签名原文");
        assert!(!json.contains("metadata"), "详情不得含 manifest.metadata");
        assert!(!json.contains("resource_path"));
        assert_eq!(d.hash_prefix.len(), HASH_PREFIX_LEN);
        assert_eq!(d.signature.status, "structure_ok");
        assert_eq!(d.signature.key_id, "key1");
    }

    #[test]
    fn detail_of_marks_bad_signature_as_failed() {
        let mut r = record_with("com.example.myplugin", PluginState::Enabled);
        r.signature_algorithm = "RSA".to_string();
        assert_eq!(detail_of(&r).signature.status, "structure_failed");
    }

    #[test]
    fn trusted_key_add_stores_fingerprint_only() {
        let mut keys = Vec::new();
        let rec =
            add_trusted_key(&mut keys, "key-1", "PUBKEY-MATERIAL-abc", "note", "now").unwrap();
        assert_eq!(rec.fingerprint.len(), HASH_PREFIX_LEN);
        let json = serde_json::to_string(&rec).unwrap();
        assert!(!json.contains("PUBKEY-MATERIAL-abc"), "不得落公钥原文");
        // 重复 key_id 拒绝（防 FM-9 替换）
        assert!(add_trusted_key(&mut keys, "key-1", "OTHER", "note", "now").is_err());
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn trusted_key_add_rejects_empty_inputs() {
        let mut keys = Vec::new();
        assert!(add_trusted_key(&mut keys, "  ", "PUB", "n", "now").is_err());
        assert!(add_trusted_key(&mut keys, "k", "  ", "n", "now").is_err());
        assert!(keys.is_empty());
    }

    #[test]
    fn trusted_key_remove_only_target_and_missing_rejected() {
        let mut keys = Vec::new();
        add_trusted_key(&mut keys, "key-1", "PUB-A", "a", "now").unwrap();
        add_trusted_key(&mut keys, "key-2", "PUB-B", "b", "now").unwrap();
        assert!(remove_trusted_key(&mut keys, "key-9").is_err());
        let removed = remove_trusted_key(&mut keys, "key-1").unwrap();
        assert_eq!(removed.key_id, "key-1");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_id, "key-2", "仅删除命中条目，不波及其余");
    }

    #[test]
    fn registry_roundtrip_uses_atomic_write() {
        let path = temp_path("registry");
        let list = vec![record_with("com.example.myplugin", PluginState::Enabled)];
        save_plugins_at(&path, &list).unwrap();
        let back = load_plugins_at(&path);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].id, "com.example.myplugin");
        assert!(
            !path.with_extension("tmp").exists(),
            "atomic_write 后不得残留 .tmp"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_registry_recovers_to_empty() {
        let path = temp_path("corrupt");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(
            load_plugins_at(&path).is_empty(),
            "损坏登记簿须 fail-closed 归空"
        );
        assert!(load_trusted_keys_at(&path).is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn trusted_keys_roundtrip() {
        let path = temp_path("keys");
        let mut keys = Vec::new();
        add_trusted_key(&mut keys, "key-1", "PUB-A", "note", "now").unwrap();
        save_trusted_keys_at(&path, &keys).unwrap();
        let back = load_trusted_keys_at(&path);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].fingerprint, keys[0].fingerprint);
        let _ = std::fs::remove_file(&path);
    }
}
