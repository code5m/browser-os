//! Opt-in provider-signed final effect receipts. This is a narrow status
//! inbox protocol, not an assertion that arbitrary MCP tools are idempotent.
//! The provider's independently pinned Ed25519 key is never passed to a
//! child as a signing secret. No status lookup launches an MCP executable,
//! opens a network connection, or replays the original operation.
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

const DOMAIN: &str = "browseros.mcp.effect.final.v1";
const INBOX: &str = "mcp-effect-status";
const MAX_PROOF: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Authority {
    pub version: u32,
    pub account: String,
    pub namespace: String,
    pub public_key: String,
}

fn label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b':')
        })
}
fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn canonical_bytes(value: &str, len: usize) -> Option<Vec<u8>> {
    if value.len() > 128 {
        return None;
    }
    let bytes = STANDARD.decode(value).ok()?;
    (bytes.len() == len && STANDARD.encode(&bytes) == value).then_some(bytes)
}

impl Authority {
    pub(super) fn valid(&self) -> bool {
        self.version == 1
            && label(&self.account)
            && label(&self.namespace)
            && canonical_bytes(&self.public_key, 32).is_some()
    }
    /// Workspace, imported and inline configurations cannot opt into
    /// automatic settlement. Only the user's native MCP configuration can
    /// pin an independently trusted provider key; exact review still applies.
    pub(super) fn from_config(storage: &Path, config: &Value) -> Result<Option<Self>, String> {
        let Some(value) = config.get("browseros_effect_authority") else {
            return Ok(None);
        };
        if config["_browseros_source"] != json!(storage.join("coding-mcp.json")) {
            return Err("MCP_EFFECT_AUTHORITY_SOURCE_DENIED".into());
        }
        let authority: Self =
            serde_json::from_value(value.clone()).map_err(|_| "MCP_EFFECT_AUTHORITY_INVALID")?;
        if !authority.valid() {
            return Err("MCP_EFFECT_AUTHORITY_INVALID".into());
        }
        Ok(Some(authority))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub authority: Authority,
    pub run_id: Uuid,
    pub effect_id: Uuid,
    pub review_revision: String,
}
impl Binding {
    pub(super) fn valid(&self) -> bool {
        self.authority.valid()
            && self.run_id.get_version_num() == 4
            && self.effect_id.get_version_num() == 4
            && self.effect_id != self.run_id
            && digest(&self.review_revision)
    }
    pub(super) fn metadata(&self) -> Value {
        json!({
            "version":1, "run_id":self.run_id,
            "idempotency_key":format!("browseros:{}", self.effect_id),
            "review_revision":self.review_revision,
            "account":self.authority.account, "namespace":self.authority.namespace
        })
    }
    pub(super) fn from_review(
        storage: &Path,
        prepared: &super::mcp_review::Prepared,
        run_id: Option<Uuid>,
    ) -> Result<Option<Self>, String> {
        if prepared.review["action"] != "mcp_call" {
            return Ok(None);
        }
        let Some(authority) = Authority::from_config(storage, &prepared.config)? else {
            return Ok(None);
        };
        let binding = Self {
            authority,
            run_id: run_id.ok_or("MCP_EFFECT_OWNER_REQUIRED")?,
            effect_id: Uuid::new_v4(),
            review_revision: prepared.review["revision"]
                .as_str()
                .ok_or("MCP_EFFECT_AUTHORITY_INVALID")?
                .to_owned(),
        };
        if !binding.valid() {
            return Err("MCP_EFFECT_AUTHORITY_INVALID".into());
        }
        // The native owner prepares the fixed private inbox before dispatch.
        crate::capabilities::workspace::safe_paths::ensure_private_directory(&storage.join(INBOX))
            .map_err(|_| "MCP_EFFECT_INBOX_UNAVAILABLE")?;
        Ok(Some(binding))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum FinalOutcome {
    AppliedFinal,
    NotAppliedFinal,
}
impl FinalOutcome {
    fn wire(self) -> &'static str {
        match self {
            Self::AppliedFinal => "applied_final",
            Self::NotAppliedFinal => "not_applied_final",
        }
    }
    pub(super) fn applied(self) -> bool {
        self == Self::AppliedFinal
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SignedStatus {
    pub version: u32,
    pub run_id: Uuid,
    pub idempotency_key: String,
    pub review_revision: String,
    pub account: String,
    pub namespace: String,
    pub outcome: FinalOutcome,
    pub signature: String,
}
impl SignedStatus {
    fn message(&self) -> String {
        // Exactly these ASCII fields, in order, newline-delimited; no final
        // newline. Never verify an ambiguous JSON serialization.
        format!(
            "{DOMAIN}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            self.version,
            self.run_id,
            self.idempotency_key,
            self.review_revision,
            self.account,
            self.namespace,
            self.outcome.wire()
        )
    }
    pub(super) fn verify(&self, binding: &Binding) -> bool {
        if !binding.valid()
            || self.version != 1
            || self.run_id != binding.run_id
            || self.idempotency_key != format!("browseros:{}", binding.effect_id)
            || self.review_revision != binding.review_revision
            || self.account != binding.authority.account
            || self.namespace != binding.authority.namespace
        {
            return false;
        }
        let Some(key) = canonical_bytes(&binding.authority.public_key, 32) else {
            return false;
        };
        let Some(signature) = canonical_bytes(&self.signature, 64) else {
            return false;
        };
        UnparsedPublicKey::new(&ED25519, key)
            .verify(self.message().as_bytes(), &signature)
            .is_ok()
    }
}

/// Query-only native delivery boundary. An independent provider/status relay
/// publishes an atomic signed file named by the native effect UUID. Missing,
/// malformed, pending, wrong-account and forged replies never settle a Run.
pub(super) fn lookup(storage: &Path, binding: &Binding) -> Result<Option<SignedStatus>, String> {
    if !binding.valid() {
        return Err("MCP_EFFECT_AUTHORITY_INVALID".into());
    }
    let directory = storage.join(INBOX);
    let path = directory.join(format!("{}.json", binding.effect_id));
    let Some(snapshot) = crate::capabilities::workspace::safe_paths::read_versioned(
        &path,
        &[directory.clone()],
        MAX_PROOF,
    )? else {
        return Ok(None);
    };
    let proof: SignedStatus =
        serde_json::from_slice(&snapshot.bytes).map_err(|_| "MCP_EFFECT_STATUS_INVALID")?;
    if !proof.verify(binding) {
        return Err("MCP_EFFECT_STATUS_UNVERIFIED".into());
    }
    Ok(Some(proof))
}

#[cfg(test)]
pub(super) fn fixture_binding() -> (Binding, ring::signature::Ed25519KeyPair) {
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let key = Ed25519KeyPair::from_seed_unchecked(&[17; 32]).unwrap();
    let binding = Binding {
        authority: Authority {
            version: 1,
            account: "fixture-account".into(),
            namespace: "fixture-namespace".into(),
            public_key: STANDARD.encode(key.public_key().as_ref()),
        },
        run_id: Uuid::new_v4(),
        effect_id: Uuid::new_v4(),
        review_revision: "a".repeat(64),
    };
    (binding, key)
}
#[cfg(test)]
pub(super) fn fixture_proof(
    binding: &Binding,
    key: &ring::signature::Ed25519KeyPair,
    outcome: FinalOutcome,
) -> SignedStatus {
    let mut proof = SignedStatus {
        version: 1,
        run_id: binding.run_id,
        idempotency_key: format!("browseros:{}", binding.effect_id),
        review_revision: binding.review_revision.clone(),
        account: binding.authority.account.clone(),
        namespace: binding.authority.namespace.clone(),
        outcome,
        signature: String::new(),
    };
    proof.signature = STANDARD.encode(key.sign(proof.message().as_bytes()).as_ref());
    proof
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_status_requires_exact_signed_final_binding() {
        let (binding, key) = fixture_binding();
        let applied = fixture_proof(&binding, &key, FinalOutcome::AppliedFinal);
        let not_applied = fixture_proof(&binding, &key, FinalOutcome::NotAppliedFinal);
        assert!(applied.verify(&binding));
        assert!(not_applied.verify(&binding));
        let mut forged = applied.clone();
        forged.outcome = FinalOutcome::NotAppliedFinal;
        assert!(!forged.verify(&binding));
        let mut another = binding.clone();
        another.run_id = Uuid::new_v4();
        assert!(!applied.verify(&another));
        another = binding.clone();
        another.effect_id = Uuid::new_v4();
        assert!(!applied.verify(&another));
        another = binding.clone();
        another.authority.account = "other".into();
        assert!(!applied.verify(&another));
        another = binding.clone();
        another.review_revision = "b".repeat(64);
        assert!(!applied.verify(&another));
        let mut pending = serde_json::to_value(&applied).unwrap();
        pending["outcome"] = json!("pending");
        assert!(serde_json::from_value::<SignedStatus>(pending).is_err());
        let mut extra = serde_json::to_value(&applied).unwrap();
        extra["final"] = json!(true);
        assert!(serde_json::from_value::<SignedStatus>(extra).is_err());
    }

    #[test]
    fn workspace_or_inline_configuration_cannot_pin_an_authority() {
        let (binding, _) = fixture_binding();
        let storage = Path::new("/fixture/private");
        let mut config = json!({"_browseros_source":"inline",
            "browseros_effect_authority":binding.authority});
        assert!(Authority::from_config(storage, &config).is_err());
        config["_browseros_source"] = json!(storage.join("coding-mcp.json"));
        assert!(Authority::from_config(storage, &config).unwrap().is_some());
    }
}
