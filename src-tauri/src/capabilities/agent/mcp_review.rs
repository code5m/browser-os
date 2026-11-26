//! Native review of the exact MCP configuration and per-call credential scope.
//! Only references and environment names leave native memory; values never do.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let sorted: BTreeMap<_, _> = object
                .iter()
                .map(|(key, value)| (key.clone(), canonical(value)))
                .collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(array) => Value::Array(array.iter().map(canonical).collect()),
        _ => value.clone(),
    }
}
fn fingerprint(value: &Value) -> Result<String, String> {
    // Reject oversize before recursively cloning/sorting the descriptor,
    // then enforce the same bound during canonical serialization.
    let encode = |value: &Value| {
        super::mcp_protocol::encode_bounded(value, 1024 * 1024).map_err(|error| {
            if error == "MCP_REQUEST_LIMIT" { "MCP_REVIEW_LIMIT".to_string() }
            else { "MCP_REVIEW_ENCODE_FAILED".to_string() }
        })
    };
    encode(value)?;
    let bytes = encode(&canonical(value))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
pub(crate) struct Prepared {
    pub config: Value,
    pub review: Value,
}

pub(crate) fn prepare(
    root: &Path,
    storage: &Path,
    request: &Value,
    mut config: Value,
) -> Result<Prepared, String> {
    let action = if request["action"] == "mcp_review" {
        request["review_action"]
            .as_str()
            .ok_or("MCP_REVIEW_ACTION_REQUIRED")?
    } else {
        request["action"].as_str().unwrap_or("")
    };
    if !matches!(action, "mcp_tools" | "mcp_call") {
        return Err("MCP_ACTION_UNKNOWN".into());
    }
    let command = config["command"].as_str().ok_or("MCP_COMMAND_REQUIRED")?;
    let executable = std::path::PathBuf::from(command);
    if !executable.is_absolute() || !executable.is_file() {
        return Err("MCP_ABSOLUTE_EXECUTABLE_REQUIRED".into());
    }
    let executable = executable
        .canonicalize()
        .map_err(|_| "MCP_EXECUTABLE_NOT_FOUND")?;
    let stem = executable
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if [
        "sh",
        "bash",
        "dash",
        "zsh",
        "fish",
        "cmd",
        "powershell",
        "pwsh",
    ]
    .contains(&stem.as_str())
    {
        return Err("MCP_SHELL_COMMAND_REJECTED".into());
    }
    config["command"] = json!(executable);
    let args = match config.get("args") {
        None | Some(Value::Null) => &[][..],
        Some(Value::Array(args)) => args.as_slice(),
        _ => return Err("MCP_ARGS_INVALID".into()),
    };
    if args.len() > 64
        || args.iter().any(|arg| {
            arg.as_str().is_none_or(|arg| {
                arg.len() > 4096 || crate::security_policy::contains_credential_leak(arg)
            })
        })
    {
        return Err("MCP_ARGS_INVALID".into());
    }
    let args = args.to_vec();
    let mut environment_keys = BTreeSet::new();
    let normalize_env = |name: &str| {
        if cfg!(windows) { name.to_ascii_uppercase() } else { name.to_owned() }
    };
    let mut grants = BTreeSet::new();
    let mut credentials = Vec::new();
    if let Some(refs) = config.get_mut("env_refs") {
        let refs = refs.as_object_mut().ok_or("MCP_ENV_INVALID")?;
        if refs.len() > 64 {
            return Err("MCP_ENV_LIMIT".into());
        }
        for (name, reference) in refs {
            if name.is_empty()
                || name.len() > 100
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Err("MCP_ENV_INVALID".into());
            }
            if !environment_keys.insert(normalize_env(name)) {
                return Err("MCP_ENV_DUPLICATE".into());
            }
            let mut info = crate::framework::ai::mcp_credential_info(
                storage,
                reference.as_str().ok_or("MCP_CREDENTIAL_REF_INVALID")?,
            )?;
            let resolved = info["reference"]
                .as_str()
                .ok_or("MCP_CREDENTIAL_REF_INVALID")?
                .to_owned();
            grants.insert(resolved.clone());
            info["environment"] = json!(name);
            credentials.push(info);
            *reference = json!(resolved);
        }
    }
    let env_names = match config.get("env") {
        None => Vec::new(),
        Some(Value::Object(env)) if env.len() <= 64 => {
            for (name, value) in env {
                if name.is_empty() || name.len() > 100
                    || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    || value.as_str().is_none_or(|value| value.len() > 16384 || value.contains('\0'))
                {
                    return Err("MCP_ENV_INVALID".into());
                }
                if !environment_keys.insert(normalize_env(name)) {
                    return Err("MCP_ENV_DUPLICATE".into());
                }
            }
            env.keys().cloned().collect::<Vec<_>>()
        },
        _ => return Err("MCP_ENV_INVALID".into()),
    };
    let arguments = if action == "mcp_call" {
        super::mcp_protocol::checked_arguments(request)?
    } else {
        Value::Null
    };
    let tool = if action == "mcp_call" {
        let tool = request["tool"]
            .as_str()
            .filter(|tool| !tool.is_empty() && tool.len() <= 200)
            .ok_or("MCP_TOOL_REQUIRED")?;
        Some(tool)
    } else {
        None
    };
    let timeout = request["timeout_seconds"]
        .as_u64()
        .unwrap_or(20)
        .clamp(1, 60);
    let authority = super::mcp_authority::Authority::from_config(storage, &config)?;
    let mut descriptor = json!({
        "action": action, "command": config["command"], "args": args, "workspace": root,
        "cwd": if cfg!(windows) { json!("AppContainer 私有运行目录") } else { json!(root) },
        "isolation": if cfg!(windows) { "appcontainer_no_network" } else { "user_process" },
        "source": config["_browseros_source"], "environment_names": env_names, "credentials": credentials,
        "credential_refs": grants.iter().collect::<Vec<_>>(), "tool": tool,
        "arguments": if action == "mcp_call" { arguments } else { Value::Null },
        "timeout_seconds": timeout
    });
    if let Some(authority) = authority.filter(|_| action == "mcp_call") {
        descriptor["effect_authority"] = json!(authority);
        descriptor["effect_status_inbox"] = json!(storage.join("mcp-effect-status"));
    }
    let revision = fingerprint(&json!({"config":config,"call":descriptor}))?;
    let mut review = descriptor;
    review["revision"] = json!(revision);
    Ok(Prepared { config, review })
}
pub(crate) fn authorize(prepared: &Prepared, request: &Value) -> Result<(), String> {
    if request["approved"] != true {
        return Err("MCP_EXPLICIT_APPROVAL_REQUIRED".into());
    }
    if request["review_revision"].as_str() != prepared.review["revision"].as_str() {
        return Err("MCP_REVIEW_CHANGED".into());
    }
    let approved = request["approved_credential_refs"]
        .as_array()
        .ok_or("MCP_CREDENTIAL_SCOPE_REQUIRED")?;
    if approved.len() > 64 || approved.iter().any(|value| !value.is_string()) {
        return Err("MCP_CREDENTIAL_SCOPE_INVALID".into());
    }
    let approved = approved
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let expected = prepared.review["credential_refs"]
        .as_array()
        .ok_or("MCP_REVIEW_INVALID")?
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if approved != expected {
        return Err("MCP_CREDENTIAL_SCOPE_CHANGED".into());
    }
    let approved_env = request["approved_environment_names"]
        .as_array()
        .ok_or("MCP_ENV_SCOPE_REQUIRED")?;
    if approved_env.len() > 64 || approved_env.iter().any(|value| !value.is_string()) {
        return Err("MCP_ENV_SCOPE_INVALID".into());
    }
    let approved_env = approved_env
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let expected_env = prepared.review["environment_names"]
        .as_array()
        .ok_or("MCP_REVIEW_INVALID")?
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if approved_env != expected_env {
        return Err("MCP_ENV_SCOPE_CHANGED".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approval_binds_configuration_and_credential_scope() {
        let make = |revision: &str| Prepared {
            config: json!({}),
            review: json!({
                "revision":revision, "credential_refs":["ai:profile:version"], "environment_names":[]
            }),
        };
        let request = json!({"approved":true,"review_revision":"one","approved_credential_refs":["ai:profile:version"],"approved_environment_names":[]});
        assert!(authorize(&make("one"), &request).is_ok());
        assert!(authorize(&make("two"), &request).is_err());
        let mut no_grant = request.clone();
        no_grant["approved_credential_refs"] = json!([]);
        assert!(authorize(&make("one"), &no_grant).is_err());
    }
    #[test]
    fn digest_canonicalizes_object_order_but_binds_arguments_and_environment() {
        assert_eq!(
            fingerprint(&json!({"a":1,"b":2})),
            fingerprint(&json!({"b":2,"a":1}))
        );
        assert_ne!(
            fingerprint(&json!({"args":["old"]})),
            fingerprint(&json!({"args":["new"]}))
        );
        assert_ne!(
            fingerprint(&json!({"env_refs":{"TOKEN":"old"}})),
            fingerprint(&json!({"env_refs":{"TOKEN":"new"}}))
        );
    }
}
