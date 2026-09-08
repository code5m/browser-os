//! Bounded, explicit local reads and non-overwriting Markdown exports.
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const MAX_NOTE_BYTES: u64 = 512 * 1024;
pub const MAX_VAULT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Serialize)]
pub struct VaultNote {
    pub path: String,
    pub text: String,
}
#[derive(Serialize)]
pub struct VaultSnapshot {
    pub root: String,
    pub notes: Vec<VaultNote>,
    pub skipped: usize,
    pub truncated: bool,
}

pub fn read_vault(root: &Path) -> Result<VaultSnapshot, String> {
    if !root.is_dir() {
        return Err("VAULT_NOT_DIRECTORY".into());
    }
    let mut out = VaultSnapshot {
        root: root.to_string_lossy().into_owned(),
        notes: vec![],
        skipped: 0,
        truncated: false,
    };
    let mut pending = vec![root.to_path_buf()];
    let mut bytes = 0;
    let mut visited = 0;
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(dir).map_err(|_| "VAULT_READ_FAILED")?;
        for entry in entries {
            visited += 1;
            if visited > 20_000 {
                out.truncated = true;
                return Ok(out);
            }
            let Ok(entry) = entry else {
                out.skipped += 1;
                continue;
            };
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') || name == "node_modules" {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                out.skipped += 1;
                continue;
            };
            if kind.is_symlink() {
                out.skipped += 1;
                continue;
            }
            if kind.is_dir() {
                pending.push(entry.path());
                continue;
            }
            if !kind.is_file() || !name.to_string_lossy().to_lowercase().ends_with(".md") {
                continue;
            }
            if out.notes.len() >= 1000 || bytes >= MAX_VAULT_BYTES {
                out.truncated = true;
                return Ok(out);
            }
            let mut text = String::new();
            // Recheck after directory enumeration; reject symlinks on the final open too.
            let path = entry
                .path()
                .canonicalize()
                .map_err(|_| "VAULT_READ_FAILED")?;
            if !path.starts_with(root) {
                out.skipped += 1;
                continue;
            }
            let file = std::fs::File::open(&path).map_err(|_| "VAULT_READ_FAILED")?;
            if file
                .take(MAX_NOTE_BYTES + 1)
                .read_to_string(&mut text)
                .is_err()
                || text.len() as u64 > MAX_NOTE_BYTES
            {
                out.skipped += 1;
                continue;
            }
            if bytes + text.len() > MAX_VAULT_BYTES {
                out.truncated = true;
                return Ok(out);
            }
            bytes += text.len();
            out.notes.push(VaultNote {
                path: path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                text,
            });
        }
    }
    out.notes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

#[derive(Deserialize)]
pub struct ArchiveItem {
    pub label: String,
    pub markdown: String,
}
#[derive(Serialize)]
pub struct ArchiveResult {
    pub label: String,
    pub path: Option<String>,
    pub error: Option<String>,
}

pub fn filename_part(value: &str) -> String {
    let mut name = String::new();
    for c in value
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
    {
        if name.len() + c.len_utf8() > 64 {
            break;
        }
        name.push(c);
    }
    if name.is_empty() {
        "reply".into()
    } else {
        name
    }
}

pub fn save_archives(
    root: &Path,
    items: Vec<ArchiveItem>,
    tags: &[String],
) -> Result<Vec<ArchiveResult>, String> {
    if items.is_empty() || items.len() > 12 || tags.len() > 12 || tags.iter().any(|t| t.len() > 128)
    {
        return Err("ARCHIVE_LIMIT".into());
    }
    if !root.is_dir() {
        return Err("ARCHIVE_NOT_DIRECTORY".into());
    }
    let safe_tags: Vec<String> = tags.iter().map(|t| filename_part(t)).collect();
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    Ok(items
        .into_iter()
        .map(|item| {
            let result = (|| -> Result<PathBuf, String> {
                if item.markdown.trim().is_empty() || item.markdown.len() > MAX_NOTE_BYTES as usize
                {
                    return Err("ARCHIVE_CONTENT_LIMIT".into());
                }
                let label = filename_part(&item.label);
                let tag = safe_tags.first().map(String::as_str).unwrap_or("chat");
                let id = uuid::Uuid::new_v4().to_string();
                let path = root.join(format!("{stamp}-{tag}-{label}-{id}.md"));
                let temp = root.join(format!(".archive-{id}.tmp"));
                let text = format!(
                    "---\ntags: {}\nsource: {}\ncreated: {}\n---\n\n{}\n",
                    serde_json::to_string(&safe_tags).unwrap(),
                    serde_json::to_string(&item.label).unwrap(),
                    stamp,
                    item.markdown
                );
                let write = (|| -> std::io::Result<()> {
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&temp)?;
                    file.write_all(text.as_bytes())?;
                    file.sync_all()?;
                    // hard_link is atomic and refuses to replace an existing destination.
                    std::fs::hard_link(&temp, &path)?;
                    std::fs::File::open(root)?.sync_all()?;
                    Ok(())
                })();
                let _ = std::fs::remove_file(temp);
                write.map_err(|_| "ARCHIVE_WRITE_FAILED".to_string())?;
                Ok(path)
            })();
            match result {
                Ok(path) => ArchiveResult {
                    label: item.label,
                    path: Some(path.to_string_lossy().into_owned()),
                    error: None,
                },
                Err(error) => ArchiveResult {
                    label: item.label,
                    path: None,
                    error: Some(error),
                },
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workbench_vault_and_archive_boundaries() {
        let root = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(root.join(".obsidian")).unwrap();
        std::fs::write(root.join("note.md"), "# Note\n[[Other]]").unwrap();
        std::fs::write(root.join(".obsidian/secret.md"), "hidden").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/passwd", root.join("escape.md")).unwrap();
        let snapshot = read_vault(&root).unwrap();
        assert_eq!(snapshot.notes.len(), 1);
        assert_eq!(filename_part("../../hello:世界"), "hello世界");
        let make = || {
            vec![
                ArchiveItem {
                    label: "A1".into(),
                    markdown: "answer".into(),
                },
                ArchiveItem {
                    label: "A2".into(),
                    markdown: "".into(),
                },
            ]
        };
        let a = save_archives(&root, make(), &["tag".into()]).unwrap();
        let b = save_archives(&root, make(), &["tag".into()]).unwrap();
        assert!(a[0].path.is_some());
        assert!(a[1].error.is_some());
        assert_ne!(a[0].path, b[0].path);
        assert!(std::fs::read_to_string(a[0].path.as_ref().unwrap())
            .unwrap()
            .contains("tags: [\"tag\"]"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
