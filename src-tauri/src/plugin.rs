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

use crate::domain::{AclLevel, PermissionPreview, PluginManifest, PluginSignature, PluginState};
use crate::security_policy::{self, PolicyError};

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
        return Err(PolicyError::InvalidPluginManifest(format!(
            "plugin.id 非反向域名规范：{}",
            m.id
        )));
    }

    // 2) version / display_name / description / min_app_version 边界 + 凭据泄露
    bound_text(&m.version, "plugin.version")?;
    bound_text(&m.display_name, "plugin.display_name")?;
    bound_text(&m.description, "plugin.description")?;
    bound_text(&m.min_app_version, "plugin.min_app_version")?;

    // 3) 形态③入口：entry_url 必须同源 webview，禁 javascript:/data:/file:/其他 scheme
    if !is_allowed_entry_url(&m.entry.entry_url) {
        return Err(PolicyError::InvalidPluginManifest(format!(
            "plugin.entry.entry_url 非形态③同源入口：{}",
            m.entry.entry_url
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

    // 7) metadata 体量边界（禁止正文/凭据由 contains_credential_leak 守）
    if let Ok(s) = serde_json::to_string(&m.metadata) {
        if s.len() > crate::domain::MAX_TEXT_FIELD_BYTES {
            return Err(PolicyError::InvalidPluginManifest(
                "plugin.metadata 超 64KiB".into(),
            ));
        }
    }

    Ok(())
}

/// 纯签名结构校验（W6 仅结构；真 Ed25519 验签由运行时 lane 在本地完成，不引加密 crate）。
pub fn verify_plugin_signature_structure(sig: &PluginSignature) -> Result<(), PolicyError> {
    if sig.algorithm != "Ed25519" {
        return Err(PolicyError::InvalidPluginSignature(format!(
            "不支持的签名算法（仅 Ed25519）：{}",
            sig.algorithm
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
        return Err(PolicyError::CredentialLeak(s.to_string()));
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
}
