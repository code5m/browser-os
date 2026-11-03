//! Crash-safe encrypted workspace draft journals (OS keyring + AES-256-GCM).
//! Unlike browser localStorage, draft bytes never enter an unencrypted on-disk
//! cache. Each editor process/session writes its own journal file so concurrent
//! instances cannot silently replace each other's unsaved text.
use crate::keyring_store::KeyringStore;
use crate::security_policy as sp;
use crate::shared::invocation::check_invocation_source;
use base64::Engine;
use ring::aead::{self, Aad, LessSafeKey, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const KEY_ID: &str = "browseros:workspace:draft-aes256-gcm:v1";
const MAX_FILE: usize = 8 * 1024 * 1024;
const MAX_JOURNAL: u64 = 17 * 1024 * 1024;
const PREFIX: &[u8] = b"BOSD1";
const MAX_JOURNALS_PER_FILE: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSnapshot {
    pub session_id: String,
    pub path: String,
    pub baseline: String,
    pub content: String,
    pub updated_at_ms: u64,
}

fn valid_session(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn path_hash(path: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(path.as_os_str().to_string_lossy().as_bytes())
    )
}
fn private_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::workspace::data_dir(app).join("encrypted-drafts-v1");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let mut b = fs::DirBuilder::new();
        b.recursive(true).mode(0o700);
        b.create(&dir).map_err(|_| "DRAFT_DIR_UNAVAILABLE")?;
        let m = fs::symlink_metadata(&dir).map_err(|_| "DRAFT_DIR_UNAVAILABLE")?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err("DRAFT_DIR_INVALID".into());
        }
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
            .map_err(|_| "DRAFT_DIR_PERMISSIONS_FAILED")?;
    }
    #[cfg(not(unix))]
    fs::create_dir_all(&dir).map_err(|_| "DRAFT_DIR_UNAVAILABLE")?;
    Ok(dir)
}
fn journal_path(dir: &Path, target: &Path, session_id: &str) -> PathBuf {
    dir.join(format!("{}-{}.bin", path_hash(target), session_id))
}

fn raw_key(create: bool, dir: &Path) -> Result<Option<[u8; 32]>, String> {
    let mut b = OpenOptions::new();
    b.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        b.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    // A stable process-shared lock prevents concurrent first-run key rotation.
    let guard = b
        .open(dir.join(".keyinit.lock"))
        .map_err(|_| "DRAFT_KEY_LOCK_FAILED")?;
    guard.lock().map_err(|_| "DRAFT_KEY_LOCK_FAILED")?;
    let value = match KeyringStore::get_token_result(KEY_ID) {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) if !create => return Ok(None),
        Err(keyring::Error::NoEntry) => {
            let mut key = [0u8; 32];
            SystemRandom::new()
                .fill(&mut key)
                .map_err(|_| "DRAFT_RANDOM_FAILED")?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(key);
            KeyringStore::save_token(KEY_ID, &encoded).map_err(|_| "DRAFT_KEYRING_UNAVAILABLE")?;
            encoded
        }
        Err(_) => return Err("DRAFT_KEYRING_UNAVAILABLE".into()),
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| "DRAFT_KEY_INVALID")?;
    let key: [u8; 32] = bytes.try_into().map_err(|_| "DRAFT_KEY_INVALID")?;
    drop(guard);
    Ok(Some(key))
}
fn cipher(key: &[u8; 32]) -> Result<LessSafeKey, String> {
    Ok(LessSafeKey::new(
        UnboundKey::new(&aead::AES_256_GCM, key).map_err(|_| "DRAFT_CIPHER_FAILED")?,
    ))
}
fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; 12];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| "DRAFT_RANDOM_FAILED")?;
    let c = cipher(key)?;
    let mut ciphertext = plaintext.to_vec();
    c.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::empty(),
        &mut ciphertext,
    )
    .map_err(|_| "DRAFT_ENCRYPT_FAILED")?;
    let mut bytes = Vec::with_capacity(PREFIX.len() + 12 + ciphertext.len());
    bytes.extend_from_slice(PREFIX);
    bytes.extend_from_slice(&nonce);
    bytes.extend_from_slice(&ciphertext);
    Ok(bytes)
}
fn decrypt(key: &[u8; 32], bytes: &[u8]) -> Result<Vec<u8>, String> {
    if !bytes.starts_with(PREFIX) || bytes.len() < PREFIX.len() + 12 + 16 {
        return Err("DRAFT_JOURNAL_INVALID".into());
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&bytes[PREFIX.len()..PREFIX.len() + 12]);
    let mut sealed = bytes[PREFIX.len() + 12..].to_vec();
    let plaintext = cipher(key)?
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::empty(),
            &mut sealed,
        )
        .map_err(|_| "DRAFT_JOURNAL_AUTH_FAILED")?;
    Ok(plaintext.to_vec())
}
fn write_journal(
    dir: &Path,
    target: &Path,
    snapshot: &DraftSnapshot,
    key: &[u8; 32],
) -> Result<(), String> {
    let final_path = journal_path(dir, target, &snapshot.session_id);
    let plaintext = serde_json::to_vec(snapshot).map_err(|_| "DRAFT_SERIALIZE_FAILED")?;
    if plaintext.len() > MAX_JOURNAL as usize {
        return Err("DRAFT_TOO_LARGE".into());
    }
    let encrypted = encrypt(key, &plaintext)?;
    let tmp = dir.join(format!(".draft-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let mut file = opts.open(&tmp).map_err(|_| "DRAFT_WRITE_FAILED")?;
        file.write_all(&encrypted)
            .map_err(|_| "DRAFT_WRITE_FAILED")?;
        file.sync_all().map_err(|_| "DRAFT_WRITE_FAILED")?;
        fs::rename(&tmp, final_path).map_err(|_| "DRAFT_REPLACE_FAILED")?;
        File::open(dir)
            .and_then(|f| f.sync_all())
            .map_err(|_| "DRAFT_DURABILITY_UNCERTAIN".into())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
fn read_journals(dir: &Path, target: &Path, key: &[u8; 32]) -> Result<Vec<DraftSnapshot>, String> {
    let prefix = format!("{}-", path_hash(target));
    let mut result = Vec::new();
    for entry in fs::read_dir(dir).map_err(|_| "DRAFT_READ_FAILED")? {
        let entry = entry.map_err(|_| "DRAFT_READ_FAILED")?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(&prefix) || !name.ends_with(".bin") {
            continue;
        }
        if result.len() >= MAX_JOURNALS_PER_FILE {
            return Err("DRAFT_TOO_MANY_SNAPSHOTS".into());
        }
        let meta = fs::symlink_metadata(entry.path()).map_err(|_| "DRAFT_READ_FAILED")?;
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_JOURNAL {
            return Err("DRAFT_JOURNAL_INVALID".into());
        }
        let bytes = fs::read(entry.path()).map_err(|_| "DRAFT_READ_FAILED")?;
        let text = decrypt(key, &bytes)?;
        let snap: DraftSnapshot =
            serde_json::from_slice(&text).map_err(|_| "DRAFT_JOURNAL_INVALID")?;
        if !valid_session(&snap.session_id)
            || snap.path != target.to_string_lossy()
            || name != format!("{prefix}{}.bin", snap.session_id)
        {
            return Err("DRAFT_JOURNAL_INVALID".into());
        }
        result.push(snap);
    }
    result.sort_by(|a, b| b.updated_at_ms.cmp(&a.updated_at_ms));
    Ok(result)
}
fn resolve(app: &AppHandle, path: &str) -> Result<PathBuf, String> {
    sp::check_path_within_roots(path, &crate::workspace::allowed_roots(app))
        .map_err(|_| "DRAFT_PATH_DENIED".into())
}

#[tauri::command]
pub async fn editor_draft_save(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
    session_id: String,
    baseline: String,
    content: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "editor_draft_save", None, &app)?;
    if !valid_session(&session_id) || content.len() > MAX_FILE || baseline.len() > MAX_FILE {
        return Err("DRAFT_INPUT_INVALID".into());
    }
    let target = resolve(&app, &path)?;
    let dir = private_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let key = raw_key(true, &dir)?.ok_or("DRAFT_KEY_UNAVAILABLE")?;
        let updated_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        write_journal(
            &dir,
            &target,
            &DraftSnapshot {
                session_id,
                path: target.to_string_lossy().into_owned(),
                baseline,
                content,
                updated_at_ms,
            },
            &key,
        )
    })
    .await
    .map_err(|_| "DRAFT_WORKER_FAILED".to_string())?
}
#[tauri::command]
pub async fn editor_draft_list(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
) -> Result<Vec<DraftSnapshot>, String> {
    check_invocation_source(&webview, "editor_draft_list", None, &app)?;
    let target = resolve(&app, &path)?;
    let dir = private_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let key = raw_key(false, &dir)?;
        match key {
            Some(key) => read_journals(&dir, &target, &key),
            None => {
                let prefix = format!("{}-", path_hash(&target));
                let existing = fs::read_dir(&dir)
                    .map_err(|_| "DRAFT_READ_FAILED")?
                    .filter_map(Result::ok)
                    .any(|entry| entry.file_name().to_string_lossy().starts_with(&prefix));
                if existing {
                    Err("DRAFT_KEY_MISSING_RECOVERY_BLOCKED".into())
                } else {
                    Ok(Vec::new())
                }
            }
        }
    })
    .await
    .map_err(|_| "DRAFT_WORKER_FAILED".to_string())?
}
#[tauri::command]
pub async fn editor_draft_clear(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
    session_id: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "editor_draft_clear", None, &app)?;
    if !valid_session(&session_id) {
        return Err("DRAFT_INPUT_INVALID".into());
    }
    let target = resolve(&app, &path)?;
    let dir = private_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let journal = journal_path(&dir, &target, &session_id);
        match fs::remove_file(journal) {
            Ok(()) => File::open(dir)
                .and_then(|f| f.sync_all())
                .map_err(|_| "DRAFT_CLEAR_DURABILITY_UNCERTAIN".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("DRAFT_CLEAR_FAILED".into()),
        }
    })
    .await
    .map_err(|_| "DRAFT_WORKER_FAILED".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticated_journal_detects_tampering_and_wrong_keys() {
        let key = [0x8a; 32];
        let msg = b"private saved draft with local key\0 and unicode";
        let sealed = encrypt(&key, msg).unwrap();
        assert_eq!(decrypt(&key, &sealed).unwrap(), msg);
        let mut tampered = sealed.clone();
        *tampered.last_mut().unwrap() ^= 1;
        assert_eq!(
            decrypt(&key, &tampered),
            Err("DRAFT_JOURNAL_AUTH_FAILED".into())
        );
        assert_eq!(
            decrypt(&[0x17; 32], &sealed),
            Err("DRAFT_JOURNAL_AUTH_FAILED".into())
        );
    }
    #[test]
    fn session_identity_is_validated() {
        assert!(valid_session("0123456789abcdef0123456789abcdef"));
        assert!(!valid_session("../other-file"));
        assert!(!valid_session("0123456789abcdef0123456789abcdef/"));
    }
    #[test]
    fn independent_sessions_are_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("browseros-drafts-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("example.md");
        let key = [7u8; 32];
        for (id, text) in [
            ("0123456789abcdef0123456789abcdef", "first edits"),
            ("fedcba9876543210fedcba9876543210", "second edits"),
        ] {
            write_journal(
                &dir,
                &path,
                &DraftSnapshot {
                    session_id: id.into(),
                    path: path.to_string_lossy().into(),
                    baseline: "base".into(),
                    content: text.into(),
                    updated_at_ms: 1712345678000,
                },
                &key,
            )
            .unwrap();
        }
        let items = read_journals(&dir, &path, &key).unwrap();
        assert_eq!(items.len(), 2);
        fs::remove_dir_all(dir).unwrap();
    }
}
