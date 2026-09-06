//! M5-4/5 Skill 解析与校验（Lane A5, W4）。
//!
//! 纯逻辑：**不执行、不安装、不联网、不引桥（crate::bridge）**。
//! 执行层（M5-4.b）才允许调用 `script_runner`；此处只负责把外部 JSON 解析为
//! `SkillDef` 并做静态校验（K6 禁内联、能力单源、凭据防泄露）。

#![allow(dead_code)]

use crate::domain::{PermissionPreview, SkillDef, SkillExec};
use crate::security_policy::{self, PolicyError};

impl SkillDef {
    /// 解析外部 JSON 为 `SkillDef`（serde 已保证类型合法；这里补语义校验）。
    ///
    /// 注：YAML 解析需 `serde_yaml` 依赖，W4 不引入新依赖，故仅 JSON。
    /// YAML 支持在 M5-4.b 视需要补（同一 `SkillDef` 类型）。
    pub fn parse(text: &str) -> Result<SkillDef, String> {
        serde_json::from_str::<SkillDef>(text).map_err(|e| format!("Skill 解析失败：{e}"))
    }

    /// 语义校验：id/version 非空、能力单源、描述/输入不含凭据泄露。
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.id.trim().is_empty() {
            return Err(PolicyError::EmptyRequiredField("skill.id".into()));
        }
        if self.version.trim().is_empty() {
            return Err(PolicyError::EmptyRequiredField("skill.version".into()));
        }
        let ids: Vec<String> = self.capabilities.iter().map(|c| c.id.clone()).collect();
        security_policy::check_skill_capabilities(&ids)?;
        if security_policy::contains_credential_leak(&self.description) {
            return Err(PolicyError::CredentialLeak(self.description.clone()));
        }
        for input in &self.inputs {
            if security_policy::contains_credential_leak(&input.description)
                || security_policy::contains_credential_leak(&input.name)
            {
                return Err(PolicyError::CredentialLeak(input.name.clone()));
            }
        }
        Ok(())
    }

    /// 权限预览（M5-6 UI 用）：闸门档 + 所需能力列表。
    pub fn permission_preview(&self) -> PermissionPreview {
        PermissionPreview {
            gate: self.acl,
            capabilities: self.capabilities.iter().map(|c| c.id.clone()).collect(),
        }
    }
}

impl SkillExec {
    /// 递归校验执行体不含内联 shell（K6）。类型已排除 Inline/RawShell，
    /// 此处仅为显式守卫，便于未来扩展时 fail-closed。
    pub fn validate(&self) -> Result<(), PolicyError> {
        match self {
            SkillExec::ScriptRef { .. } | SkillExec::CommandRef { .. } => Ok(()),
            SkillExec::Sequence { steps } => {
                for s in steps {
                    s.validate()?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AclLevel, CapabilityRef, SkillExec};
    use serde_json::json;

    fn minimal_skill() -> SkillDef {
        SkillDef {
            id: "demo".into(),
            version: "1.0.0".into(),
            display_name: "Demo".into(),
            description: "a safe demo".into(),
            acl: AclLevel::Safe,
            exec: SkillExec::ScriptRef {
                script_id: "s1".into(),
                params: json!({}),
            },
            inputs: vec![],
            capabilities: vec![],
            tests: vec![],
            metadata: json!({}),
        }
    }

    #[test]
    fn safe_skill_validates() {
        assert!(minimal_skill().validate().is_ok());
    }

    #[test]
    fn unknown_capability_rejected() {
        let mut d = minimal_skill();
        d.capabilities = vec![CapabilityRef {
            id: "file_read".into(),
        }];
        assert!(matches!(
            d.validate(),
            Err(PolicyError::UnknownCapability(_))
        ));
    }

    #[test]
    fn credential_in_description_rejected() {
        let mut d = minimal_skill();
        d.description = "token sk-abc123".into();
        assert!(matches!(d.validate(), Err(PolicyError::CredentialLeak(_))));
    }

    #[test]
    fn empty_id_rejected() {
        let mut d = minimal_skill();
        d.id = "  ".into();
        assert!(matches!(
            d.validate(),
            Err(PolicyError::EmptyRequiredField(_))
        ));
    }

    #[test]
    fn sequence_exec_validates() {
        let mut d = minimal_skill();
        d.exec = SkillExec::Sequence {
            steps: vec![SkillExec::ScriptRef {
                script_id: "s1".into(),
                params: json!({}),
            }],
        };
        assert!(d.validate().is_ok());
        assert!(d.exec.validate().is_ok());
    }

    #[test]
    fn parse_json_roundtrip() {
        let d = minimal_skill();
        let txt = serde_json::to_string(&d).unwrap();
        let back = SkillDef::parse(&txt).unwrap();
        assert_eq!(back.id, "demo");
    }
}
