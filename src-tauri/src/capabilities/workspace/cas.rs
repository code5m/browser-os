use super::safe_paths::{read_regular, save_if_unchanged, SafeWriteError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CasWriteResult {
    Saved,
    Conflict,
}

#[derive(Debug, Serialize)]
pub struct CasReadResult {
    pub path: String,
    pub content: String,
    pub content_hash: String,
    pub bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct CasWriteResponse {
    pub path: String,
    pub result: CasWriteResult,
    pub content_hash: String,
    pub current_hash: Option<String>,
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn read_file_cas(path: &Path, roots: &[PathBuf], max_bytes: usize) -> Result<CasReadResult, SafeWriteError> {
    let bytes = read_regular(path, roots, max_bytes)?;
    let content = String::from_utf8(bytes).map_err(|_| SafeWriteError::new(io::ErrorKind::InvalidData, "FILE_NOT_UTF8"))?;
    Ok(CasReadResult { path: path.to_string_lossy().into_owned(), content_hash: hash_bytes(content.as_bytes()), bytes: content.len(), content })
}

pub fn write_file_cas(path: &Path, roots: &[PathBuf], content: &str, expected_hash: &str, max_bytes: usize) -> Result<CasWriteResponse, SafeWriteError> {
    if content.len() > max_bytes { return Err(SafeWriteError::new(io::ErrorKind::InvalidData, "FILE_TOO_LARGE")); }
    if expected_hash.len() != 64 || !expected_hash.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(SafeWriteError::new(io::ErrorKind::InvalidInput, "EXPECTED_HASH_INVALID")); }
    let outcome = save_if_unchanged(path, roots, content.as_bytes(), expected_hash, max_bytes)?;
    Ok(CasWriteResponse { path: path.to_string_lossy().into_owned(), result: if outcome.saved { CasWriteResult::Saved } else { CasWriteResult::Conflict }, content_hash: hash_bytes(content.as_bytes()), current_hash: outcome.current_hash })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_is_stable() { assert_eq!(hash_bytes(b"hello"), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"); }
    #[test]
    fn hash_syntax_is_checked_before_writing() {
        let root = std::env::temp_dir().join(format!("cas-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        let result = write_file_cas(&root.join("a.txt"), &[root.clone()], "new", "bad", 100);
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
