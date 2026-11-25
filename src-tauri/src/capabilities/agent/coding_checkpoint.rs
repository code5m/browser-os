//! Agent-owned encrypted Coding session checkpoints. The existing session
//! record remains the only source of truth; no parallel run engine or SQLite
//! database is enabled. Lock ownership is kernel-scoped, not time-scoped.
use crate::capabilities::workspace::safe_paths;
use crate::workspace::AuditLock;
use base64::Engine;
use ring::aead::{self, Aad, LessSafeKey, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use std::{path::{Path, PathBuf}, time::Duration};

const PREFIX: &[u8] = b"BOSC1";
const OVERHEAD: usize = 5 + 12 + 16;
#[cfg(not(test))]
const LEGACY_KEY_ID: &str = "browseros:agent:coding-checkpoint:aes256:v1";

fn private_directory(storage: &Path, name: &str) -> Result<PathBuf, String> {
    let path = storage.join(name);
    safe_paths::ensure_private_directory(&path).map_err(|_| "AGENT_PRIVATE_STORAGE_UNAVAILABLE")?;
    Ok(path)
}

pub(super) fn session_owner(storage: &Path, id: &str) -> Result<AuditLock, String> {
    let id = uuid::Uuid::parse_str(id).map_err(|_| "AGENT_SESSION_INVALID")?;
    let directory = private_directory(storage, "coding-owner-locks")?;
    AuditLock::acquire_named(&directory.join(format!("session-{id}.lock")), Duration::ZERO)
        .map_err(|_| "AGENT_SESSION_OWNER_BUSY".into())
}

pub(super) fn workspace_owner(storage: &Path, root: &Path) -> Result<AuditLock, String> {
    // The native directory identity avoids path aliases. A reused path naming
    // a new native workspace is intentionally a different ownership scope.
    let identity = safe_paths::directory_identity(root)
        .map_err(|_| "AGENT_WORKSPACE_IDENTITY_UNAVAILABLE")?;
    let digest = format!("{:x}", Sha256::digest(identity.as_bytes()));
    let directory = private_directory(storage, "coding-owner-locks")?;
    AuditLock::acquire_named(&directory.join(format!("workspace-{digest}.lock")), Duration::ZERO)
        .map_err(|_| "AGENT_WORKSPACE_OWNER_BUSY".into())
}

#[cfg(test)]
fn key(_storage: &Path, _create: bool) -> Result<[u8; 32], String> {
    // Unit fixtures never touch a real OS credential service.
    Ok([0x42; 32])
}

fn is_cas_sidecar(name: &str) -> bool {
    if name == ".browseros-cas-directory.lock" { return true; }
    name.strip_prefix(".browseros-cas-").and_then(|s| s.strip_suffix(".lock"))
        .is_some_and(|digest| digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(not(test))]
fn key(storage: &Path, create: bool) -> Result<[u8; 32], String> {
    use crate::keyring_store::KeyringStore;
    let directory = private_directory(storage, "coding-owner-locks")?;
    let _owner = AuditLock::acquire_named(&directory.join("key-init.lock"), Duration::from_secs(2))
        .map_err(|_| "AGENT_CHECKPOINT_KEY_BUSY")?;
    let marker = directory.join("key-initialized");
    let marker_value = match std::fs::symlink_metadata(&marker) {
        Ok(_) => {
            let data = safe_paths::read_regular(&marker, &[directory.clone()], 16)
                .map_err(|_| "AGENT_CHECKPOINT_KEY_MARKER_INVALID")?;
            if data != b"BOSC1" && data != b"BOSC2" {
                return Err("AGENT_CHECKPOINT_KEY_MARKER_INVALID".into());
            }
            Some(data)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("AGENT_CHECKPOINT_KEY_MARKER_INVALID".into()),
    };
    let initialized = marker_value.is_some();
    let identity = safe_paths::directory_identity(&directory)
        .map_err(|_| "AGENT_CHECKPOINT_KEY_UNAVAILABLE")?;
    let scoped_key = format!("browseros:agent:coding-checkpoint:aes256:v2:{:x}", Sha256::digest(identity.as_bytes()));
    let key_id = if marker_value.as_deref() == Some(b"BOSC1".as_slice()) {
        LEGACY_KEY_ID
    } else {
        scoped_key.as_str()
    };
    if !initialized {
        // A removed marker must not select a fresh key for a sealed store.
        match std::fs::read_dir(storage.join("coding-sealed-records")) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into()),
        }
        // Permit a bounded legacy plaintext store to migrate, but never
        // generate a new key over a sealed backup whose key marker is lost.
        let backup_root = storage.join("coding-backups");
        match std::fs::read_dir(&backup_root) {
            Ok(sessions) => {
                let mut count = 0usize;
                let mut bytes_read = 0usize;
                for session in sessions {
                    let session = session.map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
                    count += 1;
                    if count > 1024 || !session.file_type().map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?.is_dir()
                        || session.file_name().to_str().is_none_or(|id| uuid::Uuid::parse_str(id).is_err()) {
                        return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
                    }
                    let directory = session.path();
                    for entry in std::fs::read_dir(&directory).map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")? {
                        let entry = entry.map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
                        count += 1;
                        if count > 1024 { return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into()); }
                        let name = entry.file_name();
                        let name = name.to_str().ok_or("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
                        // Historical CAS sidecars are zero-length lock
                        // objects, not user data. Never delete them or follow
                        // a symlink/reparse point when migrating.
                        if is_cas_sidecar(name) {
                            let bytes = safe_paths::read_regular(&entry.path(), &[directory.clone()], 1)
                                .map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
                            if !bytes.is_empty() { return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into()); }
                            continue;
                        }
                        if !matches!(name, "manifest.json" | "apply-transaction.json")
                            && uuid::Uuid::parse_str(name).is_err() {
                            return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
                        }
                        let data = safe_paths::read_regular(&entry.path(), &[directory.clone()], 4 * 1024 * 1024 + OVERHEAD)
                            .map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
                        bytes_read = bytes_read.saturating_add(data.len());
                        if bytes_read > 32 * 1024 * 1024 || data.starts_with(PREFIX) {
                            return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
                        }
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into()),
        }
    }
    let encoded = match KeyringStore::get_token_result(key_id) {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) if create && !initialized => {
            // Never rotate a missing key if encrypted checkpoints already
            // exist. A lost key is a recovery error, not a fresh empty store.
            let sessions = private_directory(storage, "coding-sessions")?;
            let entries = std::fs::read_dir(&sessions).map_err(|_| "AGENT_CHECKPOINT_KEY_UNAVAILABLE")?;
            for (index, entry) in entries.enumerate() {
                if index >= 1024 {
                    return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
                }
                let path = entry.map_err(|_| "AGENT_CHECKPOINT_KEY_UNAVAILABLE")?.path();
                if path.extension().and_then(|v| v.to_str()) != Some("json") {
                    continue;
                }
                let bytes = safe_paths::read_regular(&path, &[sessions.clone()], 4 * 1024 * 1024 + OVERHEAD)
                    .map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
                if bytes.starts_with(PREFIX) {
                    return Err("AGENT_CHECKPOINT_KEY_MISSING".into());
                }
            }
            let mut key = [0u8; 32];
            SystemRandom::new().fill(&mut key).map_err(|_| "AGENT_CHECKPOINT_RANDOM_FAILED")?;
            let value = base64::engine::general_purpose::STANDARD.encode(key);
            KeyringStore::save_token(key_id, &value).map_err(|_| "AGENT_CHECKPOINT_KEY_UNAVAILABLE")?;
            value
        }
        Err(_) => return Err("AGENT_CHECKPOINT_KEY_UNAVAILABLE".into()),
    };
    let key: [u8; 32] = base64::engine::general_purpose::STANDARD.decode(encoded)
        .map_err(|_| "AGENT_CHECKPOINT_KEY_INVALID")?.try_into()
        .map_err(|_| "AGENT_CHECKPOINT_KEY_INVALID")?;
    if create && !initialized {
        safe_paths::create_new_file(&marker, &[directory], b"BOSC2")
            .map_err(|_| "AGENT_CHECKPOINT_KEY_MARKER_FAILED")?;
    }
    Ok(key)
}

fn cipher(key: &[u8; 32]) -> Result<LessSafeKey, String> {
    Ok(LessSafeKey::new(UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| "AGENT_CHECKPOINT_CIPHER_FAILED")?))
}
fn associated_data(id: &str) -> String {
    format!("browseros:coding-session:v1:{id}")
}
fn seal(key: &[u8; 32], id: &str, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; 12];
    SystemRandom::new().fill(&mut nonce).map_err(|_| "AGENT_CHECKPOINT_RANDOM_FAILED")?;
    let mut sealed = plaintext.to_vec();
    cipher(key)?.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce), Aad::from(associated_data(id).as_bytes()), &mut sealed,
    ).map_err(|_| "AGENT_CHECKPOINT_ENCRYPT_FAILED")?;
    let mut bytes = Vec::with_capacity(PREFIX.len() + nonce.len() + sealed.len());
    bytes.extend_from_slice(PREFIX);
    bytes.extend_from_slice(&nonce);
    bytes.extend_from_slice(&sealed);
    Ok(bytes)
}
fn open(key: &[u8; 32], id: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < OVERHEAD || !bytes.starts_with(PREFIX) {
        return Err("AGENT_CHECKPOINT_INVALID".into());
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&bytes[PREFIX.len()..PREFIX.len() + 12]);
    let mut ciphertext = bytes[PREFIX.len() + 12..].to_vec();
    let plain = cipher(key)?.open_in_place(
        Nonce::assume_unique_for_key(nonce), Aad::from(associated_data(id).as_bytes()), &mut ciphertext,
    ).map_err(|_| "AGENT_CHECKPOINT_AUTH_FAILED")?;
    Ok(plain.to_vec())
}

// A private per-record migration marker prevents a previously sealed record
// from being reinterpreted as an unauthenticated legacy JSON/blob. This is an
// application-owner boundary, not protection against a hostile same-UID owner.
fn sealed_marker(storage: &Path, scope: &str, establish: bool) -> Result<bool, String> {
    let dir = private_directory(storage, "coding-sealed-records")?;
    let path = dir.join(format!("{:x}.sealed", Sha256::digest(scope.as_bytes())));
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {
            if safe_paths::read_regular(&path, &[dir], 5)
                .map_err(|_| "AGENT_CHECKPOINT_MARKER_INVALID")? != PREFIX {
                return Err("AGENT_CHECKPOINT_MARKER_INVALID".into());
            }
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if establish {
                match safe_paths::create_new_file(&path, &[dir.clone()], PREFIX) {
                    Ok(()) => {},
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        if safe_paths::read_regular(&path, &[dir], PREFIX.len())
                            .map_err(|_| "AGENT_CHECKPOINT_MARKER_INVALID")? != PREFIX {
                            return Err("AGENT_CHECKPOINT_MARKER_INVALID".into());
                        }
                    }
                    Err(_) => return Err("AGENT_CHECKPOINT_MARKER_FAILED".into()),
                }
            }
            Ok(establish)
        }
        Err(_) => Err("AGENT_CHECKPOINT_MARKER_INVALID".into()),
    }
}

fn read_record(storage: &Path, path: &Path, directory: PathBuf, scope: &str, limit: usize) -> Result<Vec<u8>, String> {
    let bytes = safe_paths::read_regular(path, &[directory], limit + OVERHEAD)
        .map_err(|_| "AGENT_SESSION_READ_FAILED")?;
    let plaintext = if bytes.starts_with(PREFIX) {
        let plaintext = open(&key(storage, false)?, scope, &bytes)?;
        // A successfully authenticated historical record also establishes
        // its migration marker, even when only read, not rewritten.
        sealed_marker(storage, scope, true)?;
        plaintext
    } else {
        if sealed_marker(storage, scope, false)? {
            return Err("AGENT_CHECKPOINT_PLAINTEXT_DOWNGRADE".into());
        }
        bytes
    };
    if plaintext.len() > limit {
        return Err("AGENT_SESSION_LIMIT".into());
    }
    Ok(plaintext)
}

fn write_record(storage: &Path, path: &Path, directory: PathBuf, scope: &str, plaintext: &[u8], limit: usize) -> Result<(), String> {
    if plaintext.len() > limit {
        return Err("AGENT_SESSION_LIMIT".into());
    }
    let roots = [directory];
    let expected = safe_paths::read_versioned(path, &roots, limit + OVERHEAD)?;
    let secret = key(storage, true)?;
    if let Some(previous) = &expected {
        if previous.bytes.starts_with(PREFIX) {
            open(&secret, scope, &previous.bytes)?;
        } else if sealed_marker(storage, scope, false)? {
            return Err("AGENT_CHECKPOINT_PLAINTEXT_DOWNGRADE".into());
        }
    }
    let encrypted = seal(&secret, scope, plaintext)?;
    // Crash between marking and publication preserves bytes but fails closed.
    // Never silently fall back to legacy after migration began.
    sealed_marker(storage, scope, true)?;
    safe_paths::write_versioned(path, &roots, expected.as_ref(), &encrypted, limit + OVERHEAD, |_| Ok(()))
}

pub(super) fn read(storage: &Path, id: &str, limit: usize) -> Result<Vec<u8>, String> {
    let directory = private_directory(storage, "coding-sessions")?;
    let path = directory.join(format!("{id}.json"));
    read_record(storage, &path, directory, id, limit)
}

pub(super) fn write(storage: &Path, id: &str, plaintext: &[u8], limit: usize) -> Result<(), String> {
    let directory = private_directory(storage, "coding-sessions")?;
    let path = directory.join(format!("{id}.json"));
    write_record(storage, &path, directory, id, plaintext, limit)
}

fn artifact(storage: &Path, session: &str, name: &str) -> Result<(PathBuf, String), String> {
    let parsed = uuid::Uuid::parse_str(session).map_err(|_| "AGENT_SESSION_INVALID")?;
    if parsed.to_string() != session || (!matches!(name, "apply-transaction.json" | "manifest.json")
        && uuid::Uuid::parse_str(name).map(|id| id.to_string() != name).unwrap_or(true)) {
        return Err("AGENT_BACKUP_PATH_INVALID".into());
    }
    let base = private_directory(storage, "coding-backups")?;
    let directory = private_directory(&base, session)?;
    Ok((directory, format!("backup:{session}:{name}")))
}

pub(super) fn read_artifact(storage: &Path, session: &str, name: &str, limit: usize) -> Result<Vec<u8>, String> {
    let (directory, scope) = artifact(storage, session, name)?;
    let path = directory.join(name);
    read_record(storage, &path, directory, &scope, limit)
}

pub(super) fn write_artifact(storage: &Path, session: &str, name: &str, bytes: &[u8], limit: usize) -> Result<(), String> {
    // Initialize the key before creating a new backup directory.
    key(storage, true)?;
    let (directory, scope) = artifact(storage, session, name)?;
    let path = directory.join(name);
    write_record(storage, &path, directory, &scope, bytes, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_checkpoint_is_authenticated_and_bound_to_session() {
        let key = [9; 32];
        let content = b"private coding checkpoint";
        let encrypted = seal(&key, "session-a", content).unwrap();
        assert!(!encrypted.windows(content.len()).any(|bytes| bytes == content));
        assert_eq!(open(&key, "session-a", &encrypted).unwrap(), content);
        assert!(open(&key, "session-b", &encrypted).is_err());
        let mut corrupt = encrypted;
        let last = corrupt.len() - 1;
        corrupt[last] ^= 1;
        assert!(open(&key, "session-a", &corrupt).is_err());
    }

    #[test]
    fn migration_accepts_only_known_cas_lock_names() {
        assert!(is_cas_sidecar(".browseros-cas-directory.lock"));
        assert!(is_cas_sidecar(&format!(".browseros-cas-{}.lock", "a".repeat(64))));
        assert!(!is_cas_sidecar(".browseros-cas-unknown.lock"));
        assert!(!is_cas_sidecar(".browseros-cas-directory.lock.json"));
    }

    #[test]
    fn sealed_artifacts_cannot_downgrade_or_move_between_records() {
        let storage = std::env::temp_dir().join(format!("browseros-agent-artifact-{}", uuid::Uuid::new_v4()));
        let id = uuid::Uuid::new_v4().to_string();
        write_artifact(&storage, &id, "manifest.json", b"private", 128).unwrap();
        let dir = storage.join("coding-backups").join(&id);
        let sealed = std::fs::read(dir.join("manifest.json")).unwrap();
        assert_eq!(read_artifact(&storage, &id, "manifest.json", 128).unwrap(), b"private");
        std::fs::write(dir.join("apply-transaction.json"), &sealed).unwrap();
        assert!(read_artifact(&storage, &id, "apply-transaction.json", 128).is_err());
        std::fs::write(dir.join("manifest.json"), b"legacy").unwrap();
        assert_eq!(read_artifact(&storage, &id, "manifest.json", 128).unwrap_err(), "AGENT_CHECKPOINT_PLAINTEXT_DOWNGRADE");
        std::fs::remove_dir_all(storage).unwrap();
    }

    #[test]
    fn process_owner_never_steals_a_live_lock() {
        let storage = std::env::temp_dir().join(format!("browseros-agent-owner-{}", uuid::Uuid::new_v4()));
        let id = uuid::Uuid::new_v4().to_string();
        let first = session_owner(&storage, &id).unwrap();
        assert!(session_owner(&storage, &id).is_err());
        drop(first);
        assert!(session_owner(&storage, &id).is_ok());
        std::fs::remove_dir_all(storage).unwrap();
    }
}
