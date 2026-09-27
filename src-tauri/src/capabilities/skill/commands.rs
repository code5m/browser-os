//! Skill 能力原生命令层（CAPABILITY_NATIVE_ADAPTER）。
//!
//! 只承载 `skill_parse` / `skill_validate` / `skill_permission_preview` 三个只读命令，
//! 以及其私有解析/校验 helper。不执行、不安装、不联网、不持久化写（与同目录
//! `skills.rs` 纯逻辑层一致）。命令体委托 `domain::SkillDef::parse/validate/
//! permission_preview` 与 `shared::validation` 的通用体积守卫 + 报告类型。
//!
//! 迁移自 `src-tauri/src/bridge.rs`（Native Physical Boundary Matrix Pilot /
//! native-physical-batch-skill）。

use tauri::{AppHandle, Webview};

use crate::domain::SkillDef;
use crate::shared::invocation::check_invocation_source;
use crate::shared::validation::{reject_oversized_def, ValidationReport, SKILL_DEF_MAX_BYTES};

fn skill_validate_inner(text: &str) -> ValidationReport {
    if text.len() > SKILL_DEF_MAX_BYTES {
        return ValidationReport::with_errors(vec![format!(
            "Skill 定义超出大小上限（{} 字节），已拒绝解析",
            SKILL_DEF_MAX_BYTES
        )]);
    }
    match SkillDef::parse(text) {
        Ok(def) => match def.validate() {
            Ok(()) => ValidationReport::ok(),
            Err(e) => ValidationReport::with_errors(vec![format!("{e}")]),
        },
        Err(e) => ValidationReport::with_errors(vec![e]),
    }
}

fn skill_parse_inner(text: &str) -> Result<SkillDef, String> {
    reject_oversized_def(text, SKILL_DEF_MAX_BYTES, "Skill")?;
    SkillDef::parse(text)
}

fn skill_permission_preview_inner(text: &str) -> Result<crate::domain::PermissionPreview, String> {
    reject_oversized_def(text, SKILL_DEF_MAX_BYTES, "Skill")?;
    let def = SkillDef::parse(text)?;
    Ok(def.permission_preview())
}

#[tauri::command]
pub fn skill_parse(
    app: AppHandle,
    webview: Webview,
    text: String,
) -> Result<SkillDef, String> {
    check_invocation_source(&webview, "skill_parse", None, &app)?;
    skill_parse_inner(&text)
}

#[tauri::command]
pub fn skill_validate(
    app: AppHandle,
    webview: Webview,
    text: String,
) -> Result<ValidationReport, String> {
    check_invocation_source(&webview, "skill_validate", None, &app)?;
    Ok(skill_validate_inner(&text))
}

#[tauri::command]
pub fn skill_permission_preview(
    app: AppHandle,
    webview: Webview,
    text: String,
) -> Result<crate::domain::PermissionPreview, String> {
    check_invocation_source(&webview, "skill_permission_preview", None, &app)?;
    skill_permission_preview_inner(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AclLevel, SkillExec};
    use serde_json;

    fn good_skill_json() -> String {
        let s = crate::domain::SkillDef {
            id: "demo".into(),
            version: "1.0.0".into(),
            display_name: "Demo".into(),
            description: "a safe demo".into(),
            acl: AclLevel::Safe,
            exec: SkillExec::ScriptRef {
                script_id: "s1".into(),
                params: serde_json::json!({}),
            },
            inputs: vec![],
            capabilities: vec![],
            tests: vec![],
            metadata: serde_json::json!({}),
        };
        serde_json::to_string(&s).unwrap()
    }

    #[test]
    fn validate_inner_accepts_good_skill() {
        assert!(skill_validate_inner(&good_skill_json()).valid);
    }

    #[test]
    fn validate_inner_rejects_skill_credential() {
        let mut s: crate::domain::SkillDef = serde_json::from_str(&good_skill_json()).unwrap();
        s.description = "token sk-abc123".into();
        let r = skill_validate_inner(&serde_json::to_string(&s).unwrap());
        assert!(!r.valid);
    }

    #[test]
    fn skill_validate_inner_rejects_oversized() {
        let huge = "x".repeat(SKILL_DEF_MAX_BYTES + 1);
        let r = skill_validate_inner(&huge);
        assert!(!r.valid);
    }

    #[test]
    fn skill_parse_inner_rejects_oversized() {
        let huge = "x".repeat(SKILL_DEF_MAX_BYTES + 1);
        assert!(skill_parse_inner(&huge).is_err());
    }

    #[test]
    fn skill_validate_rejects_invalid_acl() {
        let mut v: serde_json::Value = serde_json::from_str(&good_skill_json()).unwrap();
        v["acl"] = serde_json::json!(123); // 非枚举字符串 → 解析失败
        let r = skill_validate_inner(&v.to_string());
        assert!(
            !r.valid,
            "invalid acl type must invalidate, got {:?}",
            r.errors
        );
    }

    #[test]
    fn skill_permission_preview_inner_shape() {
        let p = skill_permission_preview_inner(&good_skill_json()).unwrap();
        assert!(matches!(
            p.gate,
            AclLevel::Safe | AclLevel::Confirm | AclLevel::Dangerous
        ));
        assert!(p.capabilities.is_empty());
    }

    #[test]
    fn skill_validate_redacts_credential_leak() {
        let mut v: serde_json::Value = serde_json::from_str(&good_skill_json()).unwrap();
        v["description"] = serde_json::json!("desc with sk-abc123XYZsecret in it");
        let r = skill_validate_inner(&v.to_string());
        assert!(!r.valid, "credential leak 必须使校验失败");
        let msg = r.errors.concat();
        assert!(!msg.contains("sk-abc123XYZ"), "校验错误不得回显密文: {msg}");
        assert!(
            msg.contains("<redacted>") || msg.contains("凭据") || msg.contains("密钥"),
            "必须指示泄露但不含密文: {msg}"
        );
    }

    #[test]
    fn skill_permission_preview_gate_maps_acl() {
        for (acl, expected) in [
            ("safe", AclLevel::Safe),
            ("confirm", AclLevel::Confirm),
            ("dangerous", AclLevel::Dangerous),
        ] {
            let mut v: serde_json::Value = serde_json::from_str(&good_skill_json()).unwrap();
            v["acl"] = serde_json::json!(acl);
            let p = skill_permission_preview_inner(&v.to_string()).unwrap();
            assert_eq!(p.gate, expected, "acl={acl} 映射错误");
        }
    }

    #[test]
    fn skill_permission_preview_reflects_declared_capabilities() {
        let mut v: serde_json::Value = serde_json::from_str(&good_skill_json()).unwrap();
        v["capabilities"] = serde_json::json!([{"id": "cap_a"}, {"id": "cap_b"}]);
        let p = skill_permission_preview_inner(&v.to_string()).unwrap();
        assert_eq!(
            p.capabilities,
            vec!["cap_a".to_string(), "cap_b".to_string()]
        );
    }

    #[test]
    fn skill_parse_inner_rejects_empty() {
        assert!(skill_parse_inner("").is_err());
        assert!(skill_parse_inner("   ").is_err());
    }
}
