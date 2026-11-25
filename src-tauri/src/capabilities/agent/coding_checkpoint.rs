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
const KEY_ID: &str = "browseros:agent:coding-checkpoint:aes256:v1";

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

#[cfg(not(test))]
fn key(storage: &Path, create: bool) -> Result<[u8; 32], String> {
    use crate::keyring_store::KeyringStore;
    let directory = private_directory(storage, "coding-owner-locks")?;
    let _owner = AuditLock::acquire_named(&directory.join("key-init.lock"), Duration::from_secs(2))
        .map_err(|_| "AGENT_CHECKPOINT_KEY_BUSY")?;
    let marker = directory.join("key-initialized");
    let initialized = match std::fs::symlink_metadata(&marker) {
        Ok(_) => {
            let data = safe_paths::read_regular(&marker, &[directory.clone()], 16)
                .map_err(|_| "AGENT_CHECKPOINT_KEY_MARKER_INVALID")?;
            if data != b"BOSC1" {
                return Err("AGENT_CHECKPOINT_KEY_MARKER_INVALID".into());
            }
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err("AGENT_CHECKPOINT_KEY_MARKER_INVALID".into()),
    };
    let encoded = match KeyringStore::get_token_result(KEY_ID) {
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
            KeyringStore::save_token(KEY_ID, &value).map_err(|_| "AGENT_CHECKPOINT_KEY_UNAVAILABLE")?;
            value
        }
        Err(_) => return Err("AGENT_CHECKPOINT_KEY_UNAVAILABLE".into()),
    };
    let key: [u8; 32] = base64::engine::general_purpose::STANDARD.decode(encoded)
        .map_err(|_| "AGENT_CHECKPOINT_KEY_INVALID")?.try_into()
        .map_err(|_| "AGENT_CHECKPOINT_KEY_INVALID")?;
    if create && !initialized {
        safe_paths::create_new_file(&marker, &[directory], PREFIX)
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

pub(super) fn read(storage: &Path, id: &str, limit: usize) -> Result<Vec<u8>, String> {
    let directory = private_directory(storage, "coding-sessions")?;
    let path = directory.join(format!("{id}.json"));
    let bytes = safe_paths::read_regular(&path, &[directory], limit + OVERHEAD)
        .map_err(|_| "AGENT_SESSION_READ_FAILED")?;
    let plaintext = if bytes.starts_with(PREFIX) {
        open(&key(storage, false)?, id, &bytes)?
    } else {
        // Legacy sessions are read only through the pinned, private owner.
        // Successful subsequent save migrates this exact session; never
        // interpret a failed authenticated ciphertext as legacy JSON.
        bytes
    };
    if plaintext.len() > limit {
        return Err("AGENT_SESSION_LIMIT".into());
    }
    Ok(plaintext)
}

pub(super) fn write(storage: &Path, id: &str, plaintext: &[u8], limit: usize) -> Result<(), String> {
    if plaintext.len() > limit {
        return Err("AGENT_SESSION_LIMIT".into());
    }
    let directory = private_directory(storage, "coding-sessions")?;
    let path = directory.join(format!("{id}.json"));
    let roots = [directory];
    let expected = safe_paths::read_versioned(&path, &roots, limit + OVERHEAD)?;
    let secret = key(storage, true)?;
    if let Some(previous) = &expected {
        if previous.bytes.starts_with(PREFIX) {
            // Refuse to overwrite an unreadable or swapped encrypted record.
            open(&secret, id, &previous.bytes)?;
        }
    }
    let encrypted = seal(&secret, id, plaintext)?;
    safe_paths::write_versioned(&path, &roots, expected.as_ref(), &encrypted, limit + OVERHEAD, |_| Ok(()))
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
