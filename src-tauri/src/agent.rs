//! M5-4/5 Agent 解析与校验（Lane A5, W4）。
//!
//! 纯逻辑：**不执行、不联网、不引桥**。方言仅作数据标记，endpoint 配置在 M5-4.b 加。

#![allow(dead_code)]

use crate::domain::{AgentDef, PermissionPreview};
use crate::security_policy::{self, PolicyError};

impl AgentDef {
    /// 解析外部 JSON 为 `AgentDef`。
    pub fn parse(text: &str) -> Result<AgentDef, String> {
        serde_json::from_str::<AgentDef>(text).map_err(|e| format!("Agent 解析失败：{e}"))
    }

    /// 语义校验：id/version 非空、能力单源、`system_prompt`/`description` 不含凭据（K3 + 隐私）。
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.id.trim().is_empty() {
            return Err(PolicyError::EmptyRequiredField("agent.id".into()));
        }
        if self.version.trim().is_empty() {
            return Err(PolicyError::EmptyRequiredField("agent.version".into()));
        }
        let ids: Vec<String> = self
            .default_capabilities
            .iter()
            .map(|c| c.id.clone())
            .collect();
        security_policy::check_agent_capabilities(&ids)?;
        if security_policy::contains_credential_leak(&self.system_prompt) {
            return Err(PolicyError::CredentialLeak(self.system_prompt.clone()));
        }
        if security_policy::contains_credential_leak(&self.description) {
            return Err(PolicyError::CredentialLeak(self.description.clone()));
        }
        Ok(())
    }

    /// 权限预览（M5-6 UI 用）：Agent 运行默认需确认（A2A 委派场景），并列出所需能力。
    pub fn permission_preview(&self) -> PermissionPreview {
        PermissionPreview {
            gate: crate::domain::AclLevel::Confirm,
            capabilities: self
                .default_capabilities
                .iter()
                .map(|c| c.id.clone())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AclLevel, AgentDialect, CapabilityRef};
    use serde_json::json;

    fn minimal_agent() -> AgentDef {
        AgentDef {
            id: "assistant".into(),
            version: "1.0.0".into(),
            display_name: "Assistant".into(),
            description: "general assistant".into(),
            dialect: AgentDialect::OpenAiCompatible,
            system_prompt: "You are a helpful assistant.".into(),
            default_capabilities: vec![],
            a2a: crate::domain::A2aConfig {
                delegate_to: false,
                delegated_from: false,
            },
            metadata: json!({}),
        }
    }

    #[test]
    fn safe_agent_validates() {
        assert!(minimal_agent().validate().is_ok());
    }

    #[test]
    fn credential_in_system_prompt_rejected() {
        let mut a = minimal_agent();
        a.system_prompt = "use sk-abc123XYZ".into();
        assert!(matches!(a.validate(), Err(PolicyError::CredentialLeak(_))));
    }

    #[test]
    fn unknown_capability_rejected() {
        let mut a = minimal_agent();
        a.default_capabilities = vec![CapabilityRef {
            id: "file_read".into(),
        }];
        assert!(matches!(
            a.validate(),
            Err(PolicyError::UnknownCapability(_))
        ));
    }

    #[test]
    fn permission_preview_defaults_confirm() {
        let p = minimal_agent().permission_preview();
        assert_eq!(p.gate, AclLevel::Confirm);
    }
}
