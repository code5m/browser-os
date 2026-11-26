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

const LEGACY_INVENTORY: &str = "legacy-checkpoint-inventory.v1";
const LEGACY_INVENTORY_SCOPE: &str = "legacy-inventory:v1";
const LEGACY_INVENTORY_LIMIT: usize = 1024 * 1024;
type LegacyInventory = std::collections::BTreeMap<String, String>;

fn is_cas_sidecar(name: &str) -> bool {
    if name == ".browseros-cas-directory.lock" { return true; }
    name.strip_prefix(".browseros-cas-").and_then(|s| s.strip_suffix(".lock"))
        .is_some_and(|digest| digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn canonical_id(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}

/// Capture the exact legacy plaintext bytes before the first encrypted write.
/// No previously sealed store is permitted to create a new legacy inventory.
fn collect_legacy_inventory(storage: &Path) -> Result<LegacyInventory, String> {
    // These namespaces have never had plaintext records. A missing key
    // initialization marker beside existing encrypted production runs is
    // not a first-run migration, and must never generate a replacement key.
    for name in ["agent-run-receipts", "agent-run-archive"] {
        let directory = private_directory(storage, name)?;
        for (index, entry) in std::fs::read_dir(&directory)
            .map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?.enumerate() {
            if index >= 32768 { return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into()); }
            let entry = entry.map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
            let filename = entry.file_name();
            let filename = filename.to_str().ok_or("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
            if !is_cas_sidecar(filename)
                || safe_paths::read_regular(&entry.path(), &[directory.clone()], 1)
                    .map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?.len() != 0
            { return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into()); }
        }
    }
    let mut records = LegacyInventory::new();
    let mut count = 0usize;
    let mut bytes_read = 0usize;
    let mut add = |scope: Option<String>, path: &Path, directory: &Path| -> Result<(), String> {
        count += 1;
        if count > 1024 { return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into()); }
        let limit = if scope.is_some() { 4 * 1024 * 1024 + OVERHEAD } else { 1 };
        let data = safe_paths::read_regular(path, &[directory.to_path_buf()], limit)
            .map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
        bytes_read = bytes_read.saturating_add(data.len());
        if bytes_read > 32 * 1024 * 1024 || data.starts_with(PREFIX) {
            return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
        }
        if let Some(scope) = scope {
            records.insert(scope, format!("{:x}", Sha256::digest(&data)));
        } else if !data.is_empty() {
            return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
        }
        Ok(())
    };
    let sessions = private_directory(storage, "coding-sessions")?;
    for entry in std::fs::read_dir(&sessions).map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")? {
        let entry = entry.map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
        let name = entry.file_name();
        let name = name.to_str().ok_or("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
        if let Some(id) = name.strip_suffix(".json").filter(|id| canonical_id(id)) {
            add(Some(id.to_owned()), &entry.path(), &sessions)?;
        } else if is_cas_sidecar(name) {
            add(None, &entry.path(), &sessions)?;
        } else {
            return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
        }
    }
    let backups = private_directory(storage, "coding-backups")?;
    for session in std::fs::read_dir(&backups).map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")? {
        let session = session.map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
        let name = session.file_name();
        let id = name.to_str().ok_or("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
        if !canonical_id(id) || !session.file_type().map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?.is_dir() {
            return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
        }
        let directory = private_directory(&backups, id)?;
        for entry in std::fs::read_dir(&directory).map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")? {
            let entry = entry.map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
            let name = entry.file_name();
            let name = name.to_str().ok_or("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?;
            if is_cas_sidecar(name) {
                add(None, &entry.path(), &directory)?;
            } else if matches!(name, "manifest.json" | "apply-transaction.json") || canonical_id(name) {
                add(Some(format!("backup:{id}:{name}")), &entry.path(), &directory)?;
            } else {
                return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
            }
        }
    }
    Ok(records)
}

fn read_inventory(directory: &Path, secret: &[u8; 32]) -> Result<LegacyInventory, String> {
    let bytes = safe_paths::read_regular(&directory.join(LEGACY_INVENTORY), &[directory.to_path_buf()],
        LEGACY_INVENTORY_LIMIT + OVERHEAD).map_err(|_| "AGENT_CHECKPOINT_LEGACY_REVIEW_REQUIRED")?;
    let plaintext = open(secret, LEGACY_INVENTORY_SCOPE, &bytes)?;
    serde_json::from_slice(&plaintext).map_err(|_| "AGENT_CHECKPOINT_LEGACY_REVIEW_REQUIRED".into())
}
fn verify_legacy_record(storage: &Path, secret: &[u8; 32], scope: &str, bytes: &[u8]) -> Result<(), String> {
    if sealed_marker(storage, scope, false)? {
        return Err("AGENT_CHECKPOINT_PLAINTEXT_DOWNGRADE".into());
    }
    let directory = private_directory(storage, "coding-owner-locks")?;
    let records = read_inventory(&directory, secret)?;
    let digest = format!("{:x}", Sha256::digest(bytes));
    if records.get(scope) != Some(&digest) {
        return Err("AGENT_CHECKPOINT_LEGACY_REVIEW_REQUIRED".into());
    }
    Ok(())
}

fn initialization_marker(directory: &Path) -> Result<Option<Vec<u8>>, String> {
    let marker = directory.join("key-initialized");
    match std::fs::symlink_metadata(&marker) {
        Ok(_) => {
            let data = safe_paths::read_regular(&marker, &[directory.to_path_buf()], 16)
                .map_err(|_| "AGENT_CHECKPOINT_KEY_MARKER_INVALID")?;
            if data != b"BOSC1" && data != b"BOSC2" {
                return Err("AGENT_CHECKPOINT_KEY_MARKER_INVALID".into());
            }
            Ok(Some(data))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("AGENT_CHECKPOINT_KEY_MARKER_INVALID".into()),
    }
}
fn inventory_exists(directory: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(directory.join(LEGACY_INVENTORY)) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("AGENT_CHECKPOINT_LEGACY_REVIEW_REQUIRED".into()),
    }
}
fn prepare_legacy_inventory(storage: &Path, directory: &Path) -> Result<Option<LegacyInventory>, String> {
    let sealed = private_directory(storage, "coding-sealed-records")?;
    if std::fs::read_dir(sealed).map_err(|_| "AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED")?.next().is_some() {
        return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into());
    }
    if inventory_exists(directory)? { Ok(None) } else { Ok(Some(collect_legacy_inventory(storage)?)) }
}
fn publish_inventory(directory: &Path, secret: &[u8; 32], legacy: Option<LegacyInventory>) -> Result<(), String> {
    if let Some(records) = legacy {
        let data = serde_json::to_vec(&records).map_err(|_| "AGENT_CHECKPOINT_LEGACY_REVIEW_REQUIRED")?;
        if data.len() > LEGACY_INVENTORY_LIMIT { return Err("AGENT_CHECKPOINT_LEGACY_REVIEW_REQUIRED".into()); }
        let encrypted = seal(secret, LEGACY_INVENTORY_SCOPE, &data)?;
        safe_paths::create_new_file(&directory.join(LEGACY_INVENTORY), &[directory.to_path_buf()], &encrypted)
            .map_err(|_| "AGENT_CHECKPOINT_LEGACY_REVIEW_REQUIRED")?;
    } else {
        // A crash between inventory and key marker must authenticate the
        // original inventory, never enroll whatever bytes are present now.
        read_inventory(directory, secret)?;
    }
    safe_paths::create_new_file(&directory.join("key-initialized"), &[directory.to_path_buf()], b"BOSC2")
        .map_err(|_| "AGENT_CHECKPOINT_KEY_MARKER_FAILED")?;
    Ok(())
}

#[cfg(test)]
fn key(storage: &Path, create: bool) -> Result<[u8; 32], String> {
    // Fixtures never touch the real OS credential service.
    let secret = [0x42; 32];
    let directory = private_directory(storage, "coding-owner-locks")?;
    let _owner = AuditLock::acquire_named(&directory.join("key-init.lock"), Duration::from_secs(2))
        .map_err(|_| "AGENT_CHECKPOINT_KEY_BUSY")?;
    if initialization_marker(&directory)?.is_none() && create {
        let legacy = prepare_legacy_inventory(storage, &directory)?;
        publish_inventory(&directory, &secret, legacy)?;
    }
    Ok(secret)
}

#[cfg(not(test))]
fn key(storage: &Path, create: bool) -> Result<[u8; 32], String> {
    use crate::keyring_store::KeyringStore;
    let directory = private_directory(storage, "coding-owner-locks")?;
    let _owner = AuditLock::acquire_named(&directory.join("key-init.lock"), Duration::from_secs(2))
        .map_err(|_| "AGENT_CHECKPOINT_KEY_BUSY")?;
    let marker_value = initialization_marker(&directory)?;
    let initialized = marker_value.is_some();
    let identity = safe_paths::directory_identity(&directory)
        .map_err(|_| "AGENT_CHECKPOINT_KEY_UNAVAILABLE")?;
    let scoped_key = format!("browseros:agent:coding-checkpoint:aes256:v2:{:x}", Sha256::digest(identity.as_bytes()));
    let key_id = if marker_value.as_deref() == Some(b"BOSC1".as_slice()) {
        LEGACY_KEY_ID
    } else {
        scoped_key.as_str()
    };
    if !initialized && !create { return Err("AGENT_CHECKPOINT_KEY_REVIEW_REQUIRED".into()); }
    let legacy = if initialized { None } else { prepare_legacy_inventory(storage, &directory)? };
    let encoded = match KeyringStore::get_token_result(key_id) {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) if create && !initialized && legacy.is_some() => {
            let mut key = [0u8; 32];
            SystemRandom::new().fill(&mut key).map_err(|_| "AGENT_CHECKPOINT_RANDOM_FAILED")?;
            let value = base64::engine::general_purpose::STANDARD.encode(key);
            KeyringStore::save_token(key_id, &value).map_err(|_| "AGENT_CHECKPOINT_KEY_UNAVAILABLE")?;
            value
        }
        Err(_) => return Err("AGENT_CHECKPOINT_KEY_UNAVAILABLE".into()),
    };
    let secret: [u8; 32] = base64::engine::general_purpose::STANDARD.decode(encoded)
        .map_err(|_| "AGENT_CHECKPOINT_KEY_INVALID")?.try_into()
        .map_err(|_| "AGENT_CHECKPOINT_KEY_INVALID")?;
    if !initialized { publish_inventory(&directory, &secret, legacy)?; }
    Ok(secret)
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
        // Stores already encrypted before this inventory existed cannot
        // establish that an unmarked plaintext record was genuinely legacy.
        // Keep its bytes for recovery, never execute it automatically.
        let secret = key(storage, true)?;
        verify_legacy_record(storage, &secret, scope, &bytes)?;
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
        } else {
            verify_legacy_record(storage, &secret, scope, &previous.bytes)?;
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

// One-shot production runs have their own typed namespace, but reuse the
// same private directory, OS lock, authenticated encryption and versioned
// publication. Unlike legacy Coding sessions these records NEVER migrate
// plaintext. A missing record is distinct from an unreadable record.
// Keep the admission budget separate from the per-record output cap.
// Never evict a run merely to make room: its ID and unknown-effect evidence
// are part of the replay boundary. Existing owners may finish at the cap.
const RUN_LIMIT: usize = 1024 * 1024;
const MAX_RUN_RECORDS: usize = 4096;
const MAX_RUN_DIRECTORY_ENTRIES: usize = 32768;
// Explicit terminal/superseded payload compaction preserves replay IDs,
// Task/Session identity and lineage without model text. Other eligible
// owner receipts retain their bounded effect and approval evidence.
const ARCHIVE_LIMIT: usize = 64 * 1024;
const MAX_ARCHIVED_RUNS: usize = 16384;
const MAX_ARCHIVE_DIRECTORY_ENTRIES: usize = 65536;

fn run_catalog_ids(directory: &Path) -> Result<Vec<uuid::Uuid>, String> {
    bounded_run_ids(directory, MAX_RUN_RECORDS, MAX_RUN_DIRECTORY_ENTRIES)
}
fn bounded_run_ids(directory: &Path, max_records: usize, max_entries: usize) -> Result<Vec<uuid::Uuid>, String> {
    let mut ids = Vec::new();
    for (index, entry) in std::fs::read_dir(directory)
        .map_err(|_| "AGENT_RUN_CATALOG_UNAVAILABLE")?.enumerate()
    {
        if index >= max_entries {
            return Err("AGENT_RUN_CATALOG_REVIEW_REQUIRED".into());
        }
        let entry = entry.map_err(|_| "AGENT_RUN_CATALOG_UNAVAILABLE")?;
        let name = entry.file_name();
        let Some(id) = name.to_str().and_then(|name| name.strip_suffix(".run")) else {
            // Versioned-write lock/temp files are not run identities. They
            // still count toward the bounded directory traversal.
            continue;
        };
        if !canonical_id(id) || !entry.file_type().map_err(|_| "AGENT_RUN_CATALOG_UNAVAILABLE")?.is_file() {
            return Err("AGENT_RUN_CATALOG_REVIEW_REQUIRED".into());
        }
        ids.push(uuid::Uuid::parse_str(id).map_err(|_| "AGENT_RUN_CATALOG_REVIEW_REQUIRED")?);
        if ids.len() > max_records { return Err("AGENT_RUN_CATALOG_FULL".into()); }
    }
    ids.sort_unstable();
    Ok(ids)
}
pub(super) struct RunCheckpoint {
    storage: PathBuf,
    directory: PathBuf,
    id: uuid::Uuid,
    _owner: AuditLock,
}
impl RunCheckpoint {
    pub(super) fn catalog_page(storage: &Path, after: Option<uuid::Uuid>) -> Result<(Vec<uuid::Uuid>, Option<uuid::Uuid>), String> {
        let directory = private_directory(storage, "agent-run-receipts")?;
        let archive = private_directory(storage, "agent-run-archive")?;
        // Active-before-archive scanning cannot omit a record solely
        // because archival publication precedes active-record removal.
        let mut ids = run_catalog_ids(&directory)?;
        ids.extend(bounded_run_ids(&archive, MAX_ARCHIVED_RUNS, MAX_ARCHIVE_DIRECTORY_ENTRIES)?);
        ids.sort_unstable();
        ids.dedup(); // interrupted archival may leave two identical copies
        let mut page: Vec<_> = ids.into_iter()
            .filter(|id| after.map_or(true, |cursor| *id > cursor)).take(17).collect();
        let next = if page.len() > 16 { page.pop(); page.last().copied() } else { None };
        Ok((page, next))
    }
    /// Unknown IDs must not create an unbounded collection of run locks.
    pub(super) fn record_exists(storage: &Path, id: uuid::Uuid) -> Result<bool, String> {
        if id.is_nil() { return Err("AGENT_RUN_ID_INVALID".into()); }
        let archive = private_directory(storage, "agent-run-archive")?;
        let directory = private_directory(storage, "agent-run-receipts")?;
        let present = |parent: &Path| -> Result<bool, String> {
            match std::fs::symlink_metadata(parent.join(format!("{id}.run"))) {
                Ok(meta) if meta.is_file() => Ok(true),
                Ok(_) => Err("AGENT_RUN_RECEIPT_INVALID".into()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(_) => Err("AGENT_RUN_RECEIPT_INVALID".into()),
            }
        };
        if present(&archive)? || present(&directory)? || present(&archive)? { return Ok(true); }
        if sealed_marker(storage, &format!("production-run:v1:{id}"), false)? {
            Err("AGENT_RUN_RECEIPT_MISSING".into())
        } else { Ok(false) }
    }
    pub(super) fn acquire(storage: &Path, id: uuid::Uuid) -> Result<Self, String> {
        if id.is_nil() { return Err("AGENT_RUN_ID_INVALID".into()); }
        let directory = private_directory(storage, "agent-run-receipts")?;
        let locks = private_directory(storage, "coding-owner-locks")?;
        let owner = AuditLock::acquire_named(&locks.join(format!("run-{id}.lock")), Duration::ZERO)
            .map_err(|_| "AGENT_RUN_OWNER_BUSY")?;
        Ok(Self { storage: storage.to_path_buf(), directory, id, _owner: owner })
    }
    fn path(&self) -> PathBuf { self.directory.join(format!("{}.run", self.id)) }
    fn scope(&self) -> String { format!("production-run:v1:{}", self.id) }
    fn archive_directory(&self) -> Result<PathBuf, String> {
        private_directory(&self.storage, "agent-run-archive")
    }
    fn archive_snapshot(&self) -> Result<Option<safe_paths::FileSnapshot>, String> {
        let directory = self.archive_directory()?;
        safe_paths::read_versioned(&directory.join(format!("{}.run", self.id)), &[directory.clone()],
            ARCHIVE_LIMIT + OVERHEAD)
    }
    pub(super) fn archive_confirmed(&self) -> Result<bool, String> {
        Ok(self.archive_snapshot()?.is_some()
            && safe_paths::read_versioned(&self.path(), &[self.directory.clone()], RUN_LIMIT + OVERHEAD)?.is_none())
    }
    pub(super) fn read(&self) -> Result<Option<Vec<u8>>, String> {
        let roots = [self.directory.clone()];
        let active = safe_paths::read_versioned(&self.path(), &roots, RUN_LIMIT + OVERHEAD)?;
        if let Some(archived) = self.archive_snapshot()? {
            let bytes = open(&key(&self.storage, false)?, &self.scope(), &archived.bytes)?;
            if bytes.len() > ARCHIVE_LIMIT || active.as_ref().is_some_and(|record| record.bytes != archived.bytes) {
                return Err("AGENT_RUN_ARCHIVE_CONFLICT".into());
            }
            return Ok(Some(bytes));
        }
        let Some(record) = active else {
            // A previously initialized missing record is not a new request.
            if sealed_marker(&self.storage, &self.scope(), false)? {
                return Err("AGENT_RUN_RECEIPT_MISSING".into());
            }
            return Ok(None);
        };
        let bytes = open(&key(&self.storage, false)?, &self.scope(), &record.bytes)?;
        if bytes.len() > RUN_LIMIT { return Err("AGENT_RUN_RECEIPT_LIMIT".into()); }
        Ok(Some(bytes))
    }
    /// Caller holds this Run lock and has already authenticated a terminal
    /// or superseded record eligible for archival. Publish an identical ciphertext with
    /// no-replace before removing the active copy. Neither a crash between
    /// publication/removal nor a repeated archival can resurrect a Run ID.
    pub(super) fn archive_terminal(&self) -> Result<(), String> {
        let locks = private_directory(&self.storage, "coding-owner-locks")?;
        let _catalog = AuditLock::acquire_named(&locks.join("run-catalog.lock"), Duration::from_secs(2))
            .map_err(|_| "AGENT_RUN_CATALOG_BUSY")?;
        let roots = [self.directory.clone()];
        let active = safe_paths::read_versioned(&self.path(), &roots, RUN_LIMIT + OVERHEAD)?;
        let archive = self.archive_directory()?;
        let archive_path = archive.join(format!("{}.run", self.id));
        let mut published = self.archive_snapshot()?;
        let secret = key(&self.storage, false)?;
        if let Some(archived) = &published {
            open(&secret, &self.scope(), &archived.bytes)?;
            if active.as_ref().is_some_and(|record| record.bytes != archived.bytes) {
                return Err("AGENT_RUN_ARCHIVE_CONFLICT".into());
            }
        } else {
            let source = active.as_ref().ok_or("AGENT_RUN_RECEIPT_MISSING")?;
            if open(&secret, &self.scope(), &source.bytes)?.len() > ARCHIVE_LIMIT {
                return Err("AGENT_RUN_ARCHIVE_LIMIT".into());
            }
            if bounded_run_ids(&archive, MAX_ARCHIVED_RUNS, MAX_ARCHIVE_DIRECTORY_ENTRIES)?.len() >= MAX_ARCHIVED_RUNS {
                return Err("AGENT_RUN_ARCHIVE_FULL".into());
            }
            if !sealed_marker(&self.storage, &self.scope(), false)? {
                return Err("AGENT_RUN_RECEIPT_MISSING".into());
            }
            safe_paths::create_new_file(&archive_path, &[archive.clone()], &source.bytes)
                .map_err(|_| "AGENT_RUN_ARCHIVE_PUBLICATION_PENDING")?;
            published = self.archive_snapshot()?;
            if published.as_ref().map(|record| &record.bytes) != Some(&source.bytes) {
                return Err("AGENT_RUN_ARCHIVE_PUBLICATION_PENDING".into());
            }
        }
        // Retain the sealed marker and archive even if removal fails. A
        // later retry can remove the exact duplicate under the same lock.
        if published.is_none() { return Err("AGENT_RUN_ARCHIVE_PUBLICATION_PENDING".into()); }
        if let Some(source) = &active {
            safe_paths::remove_if_unchanged(&self.path(), &roots, source, RUN_LIMIT + OVERHEAD)?;
        }
        if !self.archive_confirmed()? { return Err("AGENT_RUN_ARCHIVE_PUBLICATION_PENDING".into()); }
        Ok(())
    }
    pub(super) fn write(&self, bytes: &[u8]) -> Result<(), String> {
        if self.archive_snapshot()?.is_some() { return Err("AGENT_RUN_ARCHIVED".into()); }
        if bytes.len() > RUN_LIMIT { return Err("AGENT_RUN_RECEIPT_LIMIT".into()); }
        let roots = [self.directory.clone()];
        let path = self.path();
        let scope = self.scope();
        let expected = safe_paths::read_versioned(&path, &roots, RUN_LIMIT + OVERHEAD)?;
        // Serialize only new-record admission across processes. Never hold
        // the catalog lock during a provider request, and do not require new
        // admission capacity to publish the terminal result of a live run.
        let _catalog = if expected.is_none() {
            let locks = private_directory(&self.storage, "coding-owner-locks")?;
            let lock = AuditLock::acquire_named(&locks.join("run-catalog.lock"), Duration::from_secs(2))
                .map_err(|_| "AGENT_RUN_CATALOG_BUSY")?;
            if run_catalog_ids(&self.directory)?.len() >= MAX_RUN_RECORDS {
                return Err("AGENT_RUN_CATALOG_FULL".into());
            }
            Some(lock)
        } else { None };
        let secret = key(&self.storage, true)?;
        if let Some(previous) = &expected {
            open(&secret, &scope, &previous.bytes)?;
        } else if sealed_marker(&self.storage, &scope, false)? {
            return Err("AGENT_RUN_RECEIPT_MISSING".into());
        }
        let encrypted = seal(&secret, &scope, bytes)?;
        sealed_marker(&self.storage, &scope, true)?;
        safe_paths::write_versioned(&path, &roots, expected.as_ref(), &encrypted, RUN_LIMIT + OVERHEAD, |_| Ok(()))
    }
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
    fn legacy_inventory_binds_bytes_and_rejects_unenrolled_plaintext() {
        let storage = std::env::temp_dir().join(format!("browseros-agent-legacy-{}", uuid::Uuid::new_v4()));
        let id = uuid::Uuid::new_v4().to_string();
        let directory = private_directory(&storage, "coding-sessions").unwrap();
        let path = directory.join(format!("{id}.json"));
        std::fs::write(&path, b"legacy session").unwrap();
        assert_eq!(read(&storage, &id, 128).unwrap(), b"legacy session");
        std::fs::write(&path, b"tampered session").unwrap();
        assert!(read(&storage, &id, 128).is_err());
        let other = uuid::Uuid::new_v4().to_string();
        std::fs::write(directory.join(format!("{other}.json")), b"unenrolled").unwrap();
        assert!(read(&storage, &other, 128).is_err());
        std::fs::write(&path, b"legacy session").unwrap();
        write(&storage, &id, b"upgraded", 128).unwrap();
        assert_eq!(read(&storage, &id, 128).unwrap(), b"upgraded");
        std::fs::remove_dir_all(storage).unwrap();
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
    fn catalog_lists_canonical_run_records_not_write_sidecars() {
        let storage = std::env::temp_dir().join(format!("browseros-run-catalog-{}", uuid::Uuid::new_v4()));
        let directory = private_directory(&storage, "agent-run-receipts").unwrap();
        let id = uuid::Uuid::new_v4();
        std::fs::write(directory.join(format!("{id}.run")), b"fixture").unwrap();
        std::fs::write(directory.join(".browseros-cas-directory.lock"), b"").unwrap();
        assert_eq!(run_catalog_ids(&directory).unwrap(), vec![id]);
        std::fs::write(directory.join("bad.run"), b"invalid identity").unwrap();
        assert!(run_catalog_ids(&directory).is_err());
        std::fs::remove_dir_all(storage).unwrap();
    }

    #[test]
    fn archive_preserves_identity_and_is_idempotent_after_duplicate_publication() {
        let storage = std::env::temp_dir().join(format!("browseros-run-archive-{}", uuid::Uuid::new_v4()));
        let id = uuid::Uuid::new_v4();
        let run = RunCheckpoint::acquire(&storage, id).unwrap();
        run.write(b"compact fixture").unwrap();
        let original = std::fs::read(run.path()).unwrap();
        let archive = run.archive_directory().unwrap();
        // Simulate publication followed by a crash before active removal.
        safe_paths::create_new_file(&archive.join(format!("{id}.run")), &[archive.clone()], &original).unwrap();
        assert!(!run.archive_confirmed().unwrap());
        assert_eq!(run.read().unwrap().unwrap(), b"compact fixture");
        assert!(run.write(b"replay").is_err());
        run.archive_terminal().unwrap();
        run.archive_terminal().unwrap();
        assert!(run.archive_confirmed().unwrap());
        assert_eq!(run.read().unwrap().unwrap(), b"compact fixture");
        assert!(RunCheckpoint::record_exists(&storage,id).unwrap());
        assert_eq!(RunCheckpoint::catalog_page(&storage,None).unwrap().0,vec![id]);
        drop(run);
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
