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
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const KEY_ID: &str = "browseros:workspace:draft-aes256-gcm:v1";
const MAX_FILE: usize = 8 * 1024 * 1024;
const MAX_JOURNAL: u64 = 17 * 1024 * 1024;
const PREFIX: &[u8] = b"BOSD1";
const MAX_JOURNALS_PER_FILE: usize = 25;
const CATALOG_PAGE_SIZE: usize = 16;
static DRAFT_IO: crate::shared::blocking::BlockingLane =
    crate::shared::blocking::BlockingLane::new(4, "DRAFT_IO_BUSY", "DRAFT_WORKER_FAILED");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSnapshot {
    pub session_id: String,
    pub path: String,
    #[serde(default)]
    pub source_path: String,
    pub baseline: String,
    pub content: String,
    pub updated_at_ms: u64,
}

fn valid_session(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn path_identity(path: &Path) -> String {
    let value = path.to_string_lossy().replace('/', "\\");
    #[cfg(windows)]
    {
        value.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        value
    }
}
fn path_hash(path: &Path) -> String {
    format!("{:x}", Sha256::digest(path_identity(path).as_bytes()))
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
    #[cfg(windows)]
    crate::capabilities::workspace::safe_paths::ensure_private_directory(&dir)
        .map_err(|_| "DRAFT_DIR_PERMISSIONS_FAILED")?;
    #[cfg(all(not(unix), not(windows)))]
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
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        b.custom_flags(0x0020_0000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    // A stable process-shared lock prevents concurrent first-run key rotation.
    let guard = b
        .open(dir.join(".keyinit.lock"))
        .map_err(|_| "DRAFT_KEY_LOCK_FAILED")?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if !guard
            .metadata()
            .map_err(|_| "DRAFT_KEY_LOCK_FAILED")?
            .is_file()
            || guard
                .metadata()
                .map_err(|_| "DRAFT_KEY_LOCK_FAILED")?
                .file_attributes()
                & 0x0000_0400
                != 0
        {
            return Err("DRAFT_KEY_LOCK_FAILED".into());
        }
    }
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
    // A journal must be recoverable by the exact identity the read path checks.
    if !valid_session(&snapshot.session_id) || snapshot.path != path_identity(target) {
        return Err("DRAFT_JOURNAL_INVALID".into());
    }
    let final_path = journal_path(dir, target, &snapshot.session_id);
    let plaintext = serde_json::to_vec(snapshot).map_err(|_| "DRAFT_SERIALIZE_FAILED")?;
    if plaintext.len() + PREFIX.len() + 12 + 16 > MAX_JOURNAL as usize {
        return Err("DRAFT_TOO_LARGE".into());
    }
    let encrypted = encrypt(key, &plaintext)?;
    // Autosave, recovered edits and explicit discard all publish through the
    // same native owner. A separate rename engine could bypass the CAS lock
    // and replace a journal between recovery's version check and deletion.
    let roots = [dir.to_path_buf()];
    let expected = crate::capabilities::workspace::safe_paths::read_versioned(
        &final_path,
        &roots,
        MAX_JOURNAL as usize,
    )
    .map_err(|_| "DRAFT_READ_FAILED")?;
    crate::capabilities::workspace::safe_paths::write_versioned(
        &final_path,
        &roots,
        expected.as_ref(),
        &encrypted,
        MAX_JOURNAL as usize,
        |_| Ok(()),
    )
    .map_err(|error| match error.as_str() {
        "FILE_CONFLICT" => "DRAFT_JOURNAL_CONFLICT".to_string(),
        "FILE_DURABILITY_UNCERTAIN" => "DRAFT_DURABILITY_UNCERTAIN".to_string(),
        _ => "DRAFT_WRITE_FAILED".to_string(),
    })
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
        let bytes = crate::capabilities::workspace::safe_paths::read_regular(
            &entry.path(),
            &[dir.to_path_buf()],
            MAX_JOURNAL as usize,
        )
        .map_err(|_| "DRAFT_READ_FAILED")?;
        let text = decrypt(key, &bytes)?;
        let snap: DraftSnapshot =
            serde_json::from_slice(&text).map_err(|_| "DRAFT_JOURNAL_INVALID")?;
        if !valid_session(&snap.session_id)
            || snap.path != path_identity(target)
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
    let target = PathBuf::from(path);
    if path.len() > 16 * 1024
        || !target.is_absolute()
        || target
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("DRAFT_PATH_DENIED".into());
    }
    let roots = crate::workspace::allowed_roots(app);
    if let Ok(canonical) = sp::check_path_within_roots(path, &roots) {
        return Ok(canonical);
    }
    // A journal is private metadata, not authority to access the source file.
    // Keep journaling after external deletion without creating the missing path.
    let mut ancestor = target.as_path();
    let mut suffix = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = ancestor.file_name().ok_or("DRAFT_PATH_DENIED")?;
                sp::check_path_component(name.to_str().ok_or("DRAFT_PATH_DENIED")?)
                    .map_err(|_| "DRAFT_PATH_DENIED")?;
                suffix.push(name.to_os_string());
                ancestor = ancestor.parent().ok_or("DRAFT_PATH_DENIED")?;
            }
            Err(_) => return Err("DRAFT_PATH_DENIED".into()),
        }
    }
    if suffix.is_empty() {
        return Err("DRAFT_PATH_DENIED".into());
    }
    let mut canonical =
        sp::check_path_within_roots(ancestor.to_str().ok_or("DRAFT_PATH_DENIED")?, &roots)
            .map_err(|_| "DRAFT_PATH_DENIED")?;
    crate::capabilities::workspace::safe_paths::directory_identity(&canonical)
        .map_err(|_| "DRAFT_PATH_DENIED")?;
    for name in suffix.into_iter().rev() {
        canonical.push(name);
    }
    Ok(canonical)
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
    if webview.label() != "main" {
        return Err("DRAFT_SOURCE_DENIED".into());
    }
    check_invocation_source(&webview, "editor_draft_save", None, &app)?;
    if !valid_session(&session_id) || content.len() > MAX_FILE || baseline.len() > MAX_FILE {
        return Err("DRAFT_INPUT_INVALID".into());
    }
    DRAFT_IO
        .run(move || {
            let target = resolve(&app, &path)?;
            let dir = private_dir(&app)?;
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
                    path: path_identity(&target),
                    source_path: target.to_string_lossy().into_owned(),
                    baseline,
                    content,
                    updated_at_ms,
                },
                &key,
            )
        })
        .await
}
#[tauri::command]
pub async fn editor_draft_list(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
) -> Result<Vec<DraftSnapshot>, String> {
    if webview.label() != "main" {
        return Err("DRAFT_SOURCE_DENIED".into());
    }
    check_invocation_source(&webview, "editor_draft_list", None, &app)?;
    DRAFT_IO
        .run(move || {
            let target = resolve(&app, &path)?;
            let dir = private_dir(&app)?;
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
}
#[tauri::command]
pub async fn editor_draft_clear(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
    session_id: String,
) -> Result<(), String> {
    if webview.label() != "main" {
        return Err("DRAFT_SOURCE_DENIED".into());
    }
    check_invocation_source(&webview, "editor_draft_clear", None, &app)?;
    if !valid_session(&session_id) {
        return Err("DRAFT_INPUT_INVALID".into());
    }
    DRAFT_IO
        .run(move || {
            let target = resolve(&app, &path)?;
            let dir = private_dir(&app)?;
            let journal = journal_path(&dir, &target, &session_id);
            let roots = [dir];
            let Some(expected) = crate::capabilities::workspace::safe_paths::read_versioned(
                &journal,
                &roots,
                MAX_JOURNAL as usize,
            )
            .map_err(|_| "DRAFT_CLEAR_FAILED")?
            else {
                return Ok(());
            };
            crate::capabilities::workspace::safe_paths::remove_if_unchanged(
                &journal,
                &roots,
                &expected,
                MAX_JOURNAL as usize,
            )
            .map_err(|error| match error.as_str() {
                "FILE_CONFLICT" => "DRAFT_CLEAR_CONFLICT".to_string(),
                "FILE_DURABILITY_UNCERTAIN" => "DRAFT_CLEAR_DURABILITY_UNCERTAIN".to_string(),
                _ => "DRAFT_CLEAR_FAILED".to_string(),
            })
        })
        .await
}

#[derive(Serialize)]
pub struct DraftCatalogItem {
    pub recovery_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    pub path: String,
    pub session_id: String,
    pub updated_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
#[derive(Serialize)]
pub struct DraftCatalog {
    pub items: Vec<DraftCatalogItem>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize)]
pub struct RecoveredDraft {
    pub recovery_id: String,
    pub revision: String,
    pub path: String,
    pub session_id: String,
    pub baseline: String,
    pub content: String,
    pub updated_at_ms: u64,
}
#[derive(Serialize)]
pub struct DraftRevision {
    pub revision: String,
}
#[derive(Serialize)]
pub struct DraftCopy {
    pub path: String,
    pub content: String,
}

fn valid_recovery_id(id: &str) -> bool {
    id.len() == 101
        && id.as_bytes()[64] == b'-'
        && id.ends_with(".bin")
        && id[..64].bytes().all(|byte| byte.is_ascii_hexdigit())
        && valid_session(&id[65..97])
}
fn native_source_path(snapshot: &DraftSnapshot) -> Result<PathBuf, String> {
    let value = if !snapshot.source_path.is_empty() {
        snapshot.source_path.clone()
    } else {
        #[cfg(windows)]
        {
            snapshot.path.clone()
        }
        #[cfg(not(windows))]
        {
            snapshot.path.replace('\\', "/")
        }
    };
    let path = PathBuf::from(value);
    if !path.is_absolute()
        || path_identity(&path) != snapshot.path
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("DRAFT_JOURNAL_INVALID".into());
    }
    Ok(path)
}
fn record_id(snapshot: &DraftSnapshot) -> String {
    format!(
        "{:x}-{}.bin",
        Sha256::digest(snapshot.path.as_bytes()),
        snapshot.session_id
    )
}
fn record_revision(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read_record(
    dir: &Path,
    recovery_id: &str,
    key: &[u8; 32],
) -> Result<
    (
        crate::capabilities::workspace::safe_paths::FileSnapshot,
        DraftSnapshot,
    ),
    String,
> {
    if !valid_recovery_id(recovery_id) {
        return Err("DRAFT_RECOVERY_ID_INVALID".into());
    }
    let file = crate::capabilities::workspace::safe_paths::read_versioned(
        &dir.join(recovery_id),
        &[dir.to_path_buf()],
        MAX_JOURNAL as usize,
    )?
    .ok_or("DRAFT_RECOVERY_NOT_FOUND")?;
    let snapshot: DraftSnapshot =
        serde_json::from_slice(&decrypt(key, &file.bytes)?).map_err(|_| "DRAFT_JOURNAL_INVALID")?;
    if !valid_session(&snapshot.session_id)
        || record_id(&snapshot) != recovery_id
        || snapshot.content.len() > MAX_FILE
        || snapshot.baseline.len() > MAX_FILE
        || snapshot.path.len() > 16 * 1024
        || snapshot.source_path.len() > 16 * 1024
    {
        return Err("DRAFT_JOURNAL_INVALID".into());
    }
    native_source_path(&snapshot)?;
    Ok((file, snapshot))
}
fn recovered_view(
    recovery_id: String,
    file: &crate::capabilities::workspace::safe_paths::FileSnapshot,
    snapshot: DraftSnapshot,
) -> Result<RecoveredDraft, String> {
    let path = native_source_path(&snapshot)?
        .to_string_lossy()
        .into_owned();
    Ok(RecoveredDraft {
        recovery_id,
        revision: record_revision(&file.bytes),
        path,
        session_id: snapshot.session_id,
        baseline: snapshot.baseline,
        content: snapshot.content,
        updated_at_ms: snapshot.updated_at_ms,
    })
}
fn recovery_key(dir: &Path) -> Result<[u8; 32], String> {
    raw_key(false, dir)?.ok_or_else(|| "DRAFT_KEY_MISSING_RECOVERY_BLOCKED".into())
}
fn require_record_revision(
    file: &crate::capabilities::workspace::safe_paths::FileSnapshot,
    revision: &str,
) -> Result<(), String> {
    if revision.len() != 64 || record_revision(&file.bytes) != revision {
        return Err("DRAFT_RECOVERY_CONFLICT".into());
    }
    Ok(())
}
fn catalog_names(dir: &Path, after: Option<&str>) -> Result<Vec<String>, String> {
    let mut names = std::collections::BTreeSet::new();
    for entry in fs::read_dir(dir).map_err(|_| "DRAFT_READ_FAILED")? {
        let entry = entry.map_err(|_| "DRAFT_READ_FAILED")?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !valid_recovery_id(&name) || after.is_some_and(|cursor| name.as_str() <= cursor) {
            continue;
        }
        names.insert(name);
        if names.len() > CATALOG_PAGE_SIZE + 1 {
            names.pop_last();
        }
    }
    Ok(names.into_iter().collect())
}

/// Enumerate authenticated private records independently of source-file existence.
#[tauri::command]
pub async fn editor_draft_catalog(
    app: AppHandle,
    webview: tauri::Webview,
    cursor: Option<String>,
) -> Result<DraftCatalog, String> {
    if webview.label() != "main" {
        return Err("DRAFT_SOURCE_DENIED".into());
    }
    check_invocation_source(&webview, "editor_draft_catalog", None, &app)?;
    if cursor.as_deref().is_some_and(|id| !valid_recovery_id(id)) {
        return Err("DRAFT_CATALOG_CURSOR_INVALID".into());
    }
    DRAFT_IO
        .run(move || {
            let dir = private_dir(&app)?;
            let names = catalog_names(&dir, cursor.as_deref())?;
            if names.is_empty() {
                return Ok(DraftCatalog {
                    items: Vec::new(),
                    next_cursor: None,
                });
            }
            let key = raw_key(false, &dir);
            let next_cursor = if names.len() > CATALOG_PAGE_SIZE {
                names.get(CATALOG_PAGE_SIZE - 1).cloned()
            } else {
                None
            };
            let mut items = Vec::new();
            for id in names.into_iter().take(CATALOG_PAGE_SIZE) {
                let decoded = match &key {
                    Ok(Some(key)) => read_record(&dir, &id, key),
                    Ok(None) => Err("DRAFT_KEY_MISSING_RECOVERY_BLOCKED".into()),
                    Err(error) => Err(error.clone()),
                };
                let item = match decoded {
                    Ok((file, snapshot)) => DraftCatalogItem {
                        revision: Some(record_revision(&file.bytes)),
                        recovery_id: id,
                        path: native_source_path(&snapshot)?
                            .to_string_lossy()
                            .into_owned(),
                        session_id: snapshot.session_id,
                        updated_at_ms: snapshot.updated_at_ms,
                        error: None,
                    },
                    Err(error) => {
                        let revision = crate::capabilities::workspace::safe_paths::read_versioned(
                            &dir.join(&id),
                            &[dir.clone()],
                            MAX_JOURNAL as usize,
                        )
                        .ok()
                        .flatten()
                        .map(|file| record_revision(&file.bytes));
                        DraftCatalogItem {
                            recovery_id: id,
                            revision,
                            path: String::new(),
                            session_id: String::new(),
                            updated_at_ms: 0,
                            error: Some(error),
                        }
                    }
                };
                items.push(item);
            }
            Ok(DraftCatalog { items, next_cursor })
        })
        .await
}

/// Opening a recovery editor forks a unique journal; another live process keeps
/// its original journal and cannot overwrite the recovered editor's autosaves.
#[tauri::command]
pub async fn editor_draft_read(
    app: AppHandle,
    webview: tauri::Webview,
    recovery_id: String,
    fork: Option<bool>,
) -> Result<RecoveredDraft, String> {
    if webview.label() != "main" {
        return Err("DRAFT_SOURCE_DENIED".into());
    }
    check_invocation_source(&webview, "editor_draft_read", None, &app)?;
    DRAFT_IO
        .run(move || {
            let dir = private_dir(&app)?;
            let key = recovery_key(&dir)?;
            let (file, mut snapshot) = read_record(&dir, &recovery_id, &key)?;
            if !fork.unwrap_or(false) {
                return recovered_view(recovery_id, &file, snapshot);
            }
            let source = native_source_path(&snapshot)?;
            snapshot.source_path = source.to_string_lossy().into_owned();
            snapshot.session_id = uuid::Uuid::new_v4().simple().to_string();
            let id = record_id(&snapshot);
            let encoded = serde_json::to_vec(&snapshot).map_err(|_| "DRAFT_SERIALIZE_FAILED")?;
            let encrypted = encrypt(&key, &encoded)?;
            if encrypted.len() > MAX_JOURNAL as usize {
                return Err("DRAFT_TOO_LARGE".into());
            }
            crate::capabilities::workspace::safe_paths::write_versioned(
                &dir.join(&id),
                &[dir.clone()],
                None,
                &encrypted,
                MAX_JOURNAL as usize,
                |_| Ok(()),
            )?;
            let (file, snapshot) = read_record(&dir, &id, &key)?;
            recovered_view(id, &file, snapshot)
        })
        .await
}

#[tauri::command]
pub async fn editor_draft_update_recovered(
    app: AppHandle,
    webview: tauri::Webview,
    recovery_id: String,
    revision: String,
    content: String,
) -> Result<DraftRevision, String> {
    if webview.label() != "main" {
        return Err("DRAFT_SOURCE_DENIED".into());
    }
    check_invocation_source(&webview, "editor_draft_update_recovered", None, &app)?;
    if content.len() > MAX_FILE {
        return Err("DRAFT_TOO_LARGE".into());
    }
    DRAFT_IO
        .run(move || {
            let dir = private_dir(&app)?;
            let key = recovery_key(&dir)?;
            let (file, mut snapshot) = read_record(&dir, &recovery_id, &key)?;
            require_record_revision(&file, &revision)?;
            snapshot.content = content;
            snapshot.updated_at_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u64::MAX as u128) as u64;
            let encoded = serde_json::to_vec(&snapshot).map_err(|_| "DRAFT_SERIALIZE_FAILED")?;
            let encrypted = encrypt(&key, &encoded)?;
            if encrypted.len() > MAX_JOURNAL as usize {
                return Err("DRAFT_TOO_LARGE".into());
            }
            crate::capabilities::workspace::safe_paths::write_versioned(
                &dir.join(&recovery_id),
                &[dir.clone()],
                Some(&file),
                &encrypted,
                MAX_JOURNAL as usize,
                |_| Ok(()),
            )
            .map_err(|error| {
                if error == "FILE_CONFLICT" {
                    "DRAFT_RECOVERY_CONFLICT".to_string()
                } else {
                    error
                }
            })?;
            Ok(DraftRevision {
                revision: record_revision(&encrypted),
            })
        })
        .await
}

#[tauri::command]
pub async fn editor_draft_remove(
    app: AppHandle,
    webview: tauri::Webview,
    recovery_id: String,
    revision: String,
) -> Result<(), String> {
    if webview.label() != "main" {
        return Err("DRAFT_SOURCE_DENIED".into());
    }
    check_invocation_source(&webview, "editor_draft_remove", None, &app)?;
    DRAFT_IO
        .run(move || {
            let dir = private_dir(&app)?;
            if !valid_recovery_id(&recovery_id) {
                return Err("DRAFT_RECOVERY_ID_INVALID".into());
            }
            // Explicit discard may remove a corrupt encrypted record, but only the
            // exact private object/version the user selected, never an arbitrary path.
            let file = crate::capabilities::workspace::safe_paths::read_versioned(
                &dir.join(&recovery_id),
                &[dir.clone()],
                MAX_JOURNAL as usize,
            )?
            .ok_or("DRAFT_RECOVERY_NOT_FOUND")?;
            require_record_revision(&file, &revision)?;
            crate::capabilities::workspace::safe_paths::remove_if_unchanged(
                &dir.join(&recovery_id),
                &[dir.clone()],
                &file,
                MAX_JOURNAL as usize,
            )
            .map_err(|error| {
                if error == "FILE_CONFLICT" {
                    "DRAFT_RECOVERY_CONFLICT".to_string()
                } else {
                    error
                }
            })
        })
        .await
}

fn copy_recovered_record(
    dir: &Path,
    recovery_id: &str,
    key: &[u8; 32],
    target_path: String,
    roots: &[PathBuf],
    content: String,
) -> Result<DraftCopy, String> {
    if content.len() > MAX_FILE || target_path.len() > 16 * 1024 {
        return Err("DRAFT_INPUT_INVALID".into());
    }
    read_record(dir, recovery_id, key)?;
    let target = PathBuf::from(&target_path);
    if !target.is_absolute()
        || target.file_name().is_none()
        || target
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("DRAFT_PATH_DENIED".into());
    }
    // Explicit Save As creates a new file only. Native pinned-root and
    // no-replace CAS remain authoritative; the source journal is retained.
    crate::capabilities::workspace::safe_paths::write_versioned(
        &target,
        roots,
        None,
        content.as_bytes(),
        MAX_FILE,
        |_| Ok(()),
    )?;
    Ok(DraftCopy {
        path: target_path,
        content,
    })
}

#[tauri::command]
pub async fn editor_draft_save_as(
    app: AppHandle,
    webview: tauri::Webview,
    recovery_id: String,
    target_path: String,
    content: String,
) -> Result<DraftCopy, String> {
    if webview.label() != "main" {
        return Err("DRAFT_SOURCE_DENIED".into());
    }
    check_invocation_source(&webview, "editor_draft_save_as", None, &app)?;
    if content.len() > MAX_FILE || target_path.len() > 16 * 1024 {
        return Err("DRAFT_INPUT_INVALID".into());
    }
    DRAFT_IO
        .run(move || {
            let dir = private_dir(&app)?;
            let key = recovery_key(&dir)?;
            copy_recovered_record(
                &dir,
                &recovery_id,
                &key,
                target_path,
                &crate::workspace::allowed_roots(&app),
                content,
            )
        })
        .await
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
    fn journal_rejects_wrong_workspace_identity_before_writing() {
        let dir = std::env::temp_dir().join(format!(
            "browseros-journal-wrong-root-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target.md");
        let snapshot = DraftSnapshot {
            session_id: "0123456789abcdef0123456789abcdef".into(),
            path: "unrelated/workspace.md".into(),
            source_path: String::new(),
            baseline: String::new(),
            content: "private draft".into(),
            updated_at_ms: 1,
        };
        assert_eq!(
            write_journal(&dir, &target, &snapshot, &[7u8; 32]).unwrap_err(),
            "DRAFT_JOURNAL_INVALID"
        );
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
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
                    path: path_identity(&path),
                    source_path: path.to_string_lossy().into_owned(),
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
    #[test]
    fn orphan_recovery_preserves_source_and_save_as_never_replaces() {
        let directory = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "browseros-orphan-recovery-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&directory).unwrap();
        let source = directory.join("gone.md");
        fs::write(&source, "disk baseline").unwrap();
        let snapshot = DraftSnapshot {
            session_id: uuid::Uuid::new_v4().simple().to_string(),
            path: path_identity(&source),
            source_path: source.to_string_lossy().into_owned(),
            baseline: "disk baseline".into(),
            content: "unsaved draft".into(),
            updated_at_ms: 1,
        };
        let key = [23u8; 32];
        write_journal(&directory, &source, &snapshot, &key).unwrap();
        let id = record_id(&snapshot);
        fs::remove_file(&source).unwrap();
        let (file, recovered) = read_record(&directory, &id, &key).unwrap();
        assert_eq!(recovered.content, "unsaved draft");
        assert!(require_record_revision(&file, &record_revision(&file.bytes)).is_ok());
        assert!(require_record_revision(&file, &"0".repeat(64)).is_err());
        let destination = directory.join("copy.md");
        fs::write(&destination, "keep this file").unwrap();
        assert!(copy_recovered_record(
            &directory,
            &id,
            &key,
            destination.to_string_lossy().into_owned(),
            &[directory.clone()],
            "edited recovery".into(),
        )
        .is_err());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "keep this file");
        fs::remove_file(&destination).unwrap();
        copy_recovered_record(
            &directory,
            &id,
            &key,
            destination.to_string_lossy().into_owned(),
            &[directory.clone()],
            "edited recovery".into(),
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&destination).unwrap(), "edited recovery");
        assert!(directory.join(&id).exists());
        assert!(!source.exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recovery_delete_rejects_a_newer_encrypted_record() {
        let directory = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "browseros-recovery-conflict-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&directory).unwrap();
        let source = directory.join("missing.md");
        let mut snapshot = DraftSnapshot {
            session_id: uuid::Uuid::new_v4().simple().to_string(),
            path: path_identity(&source),
            source_path: source.to_string_lossy().into_owned(),
            baseline: String::new(),
            content: "first".into(),
            updated_at_ms: 1,
        };
        let key = [25u8; 32];
        write_journal(&directory, &source, &snapshot, &key).unwrap();
        let id = record_id(&snapshot);
        let (old, _) = read_record(&directory, &id, &key).unwrap();
        snapshot.content = "newer edits".into();
        write_journal(&directory, &source, &snapshot, &key).unwrap();
        assert!(
            crate::capabilities::workspace::safe_paths::remove_if_unchanged(
                &directory.join(&id),
                &[directory.clone()],
                &old,
                MAX_JOURNAL as usize,
            )
            .is_err()
        );
        assert_eq!(
            read_record(&directory, &id, &key).unwrap().1.content,
            "newer edits"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recovery_catalog_pages_without_omitting_remaining_records() {
        let directory =
            std::env::temp_dir().join(format!("browseros-catalog-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        for number in 0..40 {
            fs::write(
                directory.join(format!("{number:064x}-{:032x}.bin", 1)),
                b"encrypted",
            )
            .unwrap();
        }
        fs::write(directory.join("not-a-record.bin"), b"ignored").unwrap();
        let first = catalog_names(&directory, None).unwrap();
        assert_eq!(first.len(), CATALOG_PAGE_SIZE + 1);
        let second = catalog_names(&directory, Some(&first[CATALOG_PAGE_SIZE - 1])).unwrap();
        assert_eq!(second[0], first[CATALOG_PAGE_SIZE]);
        assert!(!valid_recovery_id("../outside.bin"));
        assert!(!valid_recovery_id(&"界".repeat(34)));
        fs::remove_dir_all(directory).unwrap();
    }
}
