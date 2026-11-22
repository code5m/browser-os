//! Versioned Keyring publication with a bounded, non-secret cleanup journal.
//! Callers serialize their config owner and capture credentials before releasing it.
//! The journal is durable BEFORE a new key is created. Reconcile only against a
//! freshly read persisted config, including after an ambiguous save/fsync error.
use crate::keyring_store::KeyringStore;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::{Path, PathBuf}};

const LIMIT: usize = 256;
fn redirected(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() { return true; }
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        return metadata.file_attributes() & 0x400 != 0; // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))] { false }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record { references: Vec<String> }

fn valid_reference(namespace: &str, reference: &str) -> bool {
    let git_id = |id: &str| !id.is_empty() && id.len() <= 128
        && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    if namespace == "git" && git_id(reference) { return true; }
    let Some(rest) = reference.strip_prefix(&format!("{namespace}:")) else { return false; };
    let mut parts = rest.split(':');
    let id = parts.next().unwrap_or("");
    let owner_valid = match namespace {
        "ai" | "ai-plugin" => id.len() == 24 && id.bytes().all(|b| b.is_ascii_hexdigit()),
        "db" => uuid::Uuid::parse_str(id).is_ok(),
        "git" => git_id(id),
        _ => false,
    };
    owner_valid && match (parts.next(), parts.next()) {
        (None, None) => true,
        (Some(version), None) => uuid::Uuid::parse_str(version).is_ok(),
        _ => false,
    }
}
fn directory(config: &Path, namespace: &str) -> Result<PathBuf, String> {
    if !matches!(namespace, "ai" | "ai-plugin" | "db" | "git") { return Err("CREDENTIAL_OWNER_INVALID".into()); }
    let parent = config.parent().ok_or("CREDENTIAL_CONFIG_PATH_INVALID")?;
    fs::create_dir_all(parent).map_err(|_| "CREDENTIAL_JOURNAL_CREATE_FAILED")?;
    let metadata = fs::symlink_metadata(parent).map_err(|_| "CREDENTIAL_JOURNAL_UNAVAILABLE")?;
    if !metadata.is_dir() || redirected(&metadata) { return Err("CREDENTIAL_JOURNAL_UNSAFE".into()); }
    let root = parent.join("credential-updates");
    let dir = root.join(namespace);
    for path in [&root, &dir] {
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.is_dir() && !redirected(&meta) => {}
            Ok(_) => return Err("CREDENTIAL_JOURNAL_UNSAFE".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match fs::create_dir(path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        let meta = fs::symlink_metadata(path).map_err(|_| "CREDENTIAL_JOURNAL_UNAVAILABLE")?;
                        if !meta.is_dir() || redirected(&meta) { return Err("CREDENTIAL_JOURNAL_UNSAFE".into()); }
                    }
                    Err(_) => return Err("CREDENTIAL_JOURNAL_CREATE_FAILED".into()),
                }
                #[cfg(unix)] {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                        .map_err(|_| "CREDENTIAL_JOURNAL_PERMISSIONS_FAILED")?;
                    // Make each new directory entry durable before an intent can
                    // publish a Keyring value. Syncing only the leaf is not enough.
                    fs::File::open(path.parent().ok_or("CREDENTIAL_CONFIG_PATH_INVALID")?)
                        .and_then(|parent| parent.sync_all())
                        .map_err(|_| "CREDENTIAL_JOURNAL_SYNC_FAILED")?;
                }
            }
            Err(_) => return Err("CREDENTIAL_JOURNAL_UNAVAILABLE".into()),
        }
    }
    Ok(dir)
}
fn records(config: &Path, namespace: &str) -> Result<Vec<PathBuf>, String> {
    let dir = directory(config, namespace)?;
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(|_| "CREDENTIAL_JOURNAL_READ_FAILED")? {
        let entry = entry.map_err(|_| "CREDENTIAL_JOURNAL_READ_FAILED")?;
        // Atomic-write residue contains only reference names, never secret values.
        if entry.file_name().to_string_lossy().ends_with(".tmp") {
            let meta = fs::symlink_metadata(entry.path()).map_err(|_| "CREDENTIAL_JOURNAL_READ_FAILED")?;
            if !meta.is_file() || redirected(&meta) || meta.len() > 4096 {
                return Err("CREDENTIAL_JOURNAL_UNSAFE".into());
            }
            fs::remove_file(entry.path()).map_err(|_| "CREDENTIAL_JOURNAL_REMOVE_FAILED")?;
            continue;
        }
        if files.len() >= LIMIT { return Err("CREDENTIAL_CLEANUP_LIMIT".into()); }
        let path = entry.path();
        let name = path.file_stem().and_then(|name| name.to_str())
            .ok_or("CREDENTIAL_JOURNAL_INVALID")?;
        if path.extension().and_then(|e| e.to_str()) != Some("json")
            || uuid::Uuid::parse_str(name).is_err() { return Err("CREDENTIAL_JOURNAL_INVALID".into()); }
        let meta = fs::symlink_metadata(&path).map_err(|_| "CREDENTIAL_JOURNAL_READ_FAILED")?;
        if !meta.is_file() || redirected(&meta) || meta.len() > 4096 {
            return Err("CREDENTIAL_JOURNAL_UNSAFE".into());
        }
        files.push(path);
    }
    Ok(files)
}
pub(crate) fn record_retirement(config: &Path, namespace: &str, references: &[String]) -> Result<(), String> {
    if references.is_empty() { return Ok(()); }
    if references.len() > 4 || references.iter().any(|r| !valid_reference(namespace, r)) {
        return Err("CREDENTIAL_REFERENCE_INVALID".into());
    }
    if records(config, namespace)?.len() >= LIMIT { return Err("CREDENTIAL_CLEANUP_LIMIT".into()); }
    let path = directory(config, namespace)?.join(format!("{}.json", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_string(&Record { references: references.to_vec() })
        .map_err(|_| "CREDENTIAL_JOURNAL_ENCODE_FAILED")?;
    crate::session::atomic_write(&path, &bytes)
        .map_err(|_| "CREDENTIAL_JOURNAL_WRITE_FAILED".to_string())
}
pub(crate) fn stage(
    config: &Path, namespace: &str, id: &str, secret: &str, retired: &[String],
) -> Result<String, String> {
    let reference = format!("{namespace}:{id}:{}", uuid::Uuid::new_v4());
    if !valid_reference(namespace, &reference) { return Err("CREDENTIAL_REFERENCE_INVALID".into()); }
    let mut references = retired.to_vec();
    references.push(reference.clone());
    record_retirement(config, namespace, &references)?;
    KeyringStore::save_token(&reference, secret).map_err(|_| "CREDENTIAL_SAVE_FAILED")?;
    Ok(reference)
}
pub(crate) fn reconcile(config: &Path, namespace: &str, live: &HashSet<String>) -> Result<(), String> {
    for path in records(config, namespace)? {
        let record: Record = serde_json::from_slice(
            &fs::read(&path).map_err(|_| "CREDENTIAL_JOURNAL_READ_FAILED")?
        ).map_err(|_| "CREDENTIAL_JOURNAL_INVALID")?;
        if record.references.is_empty() || record.references.len() > 4
            || record.references.iter().any(|r| !valid_reference(namespace, r)) {
            return Err("CREDENTIAL_JOURNAL_INVALID".into());
        }
        for reference in record.references {
            if live.contains(&reference) { continue; }
            match KeyringStore::get_token_result(&reference) {
                Err(keyring::Error::NoEntry) => continue,
                Err(_) => return Err("CREDENTIAL_CLEANUP_PENDING".into()),
                Ok(_) => KeyringStore::delete_token(&reference)
                    .map_err(|_| "CREDENTIAL_CLEANUP_PENDING")?,
            }
        }
        fs::remove_file(&path).map_err(|_| "CREDENTIAL_JOURNAL_REMOVE_FAILED")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_references_are_scoped_and_versioned() {
        let id = "a".repeat(24);
        assert!(valid_reference("ai", &format!("ai:{id}")));
        assert!(valid_reference("ai", &format!("ai:{id}:{}", uuid::Uuid::new_v4())));
        assert!(valid_reference("ai-plugin", &format!("ai-plugin:{id}")));
        assert!(valid_reference("ai-plugin", &format!("ai-plugin:{id}:{}", uuid::Uuid::new_v4())));
        assert!(!valid_reference("ai-plugin", &format!("ai:{id}")));
        assert!(!valid_reference("ai", &format!("ai-plugin:{id}")));
        assert!(!valid_reference("ai-plugin", "ai-plugin:short"));
        assert!(!valid_reference("ai-plugin", &format!("ai-plugin:{id}:not-a-version")));
        assert!(!valid_reference("ai", &format!("db:{}", uuid::Uuid::new_v4())));
        assert!(!valid_reference("ai", &format!("ai:{id}:../other")));
        assert!(!valid_reference("db", "db:repo-id"));
        assert!(valid_reference("git", "legacy_repo-1"));
        assert!(valid_reference("git", &format!("git:legacy_repo-1:{}", uuid::Uuid::new_v4())));
        for reference in ["ai:0123456789abcdef01234567", "db:legacy", "browser:secret", "../repo", "git:repo:not-a-version"] {
            assert!(!valid_reference("git", reference));
        }
    }
}
