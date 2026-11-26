//! Windows coding context read-only discovery. No spawned MCP servers or
//! untrusted code execution before a Windows restricted-token/AppContainer
//! sandbox passes the required security review.
#![cfg(windows)]
use serde_json::{json, Value};
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};

const MAX_READ: usize = 128 * 1024;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

use super::mcp_protocol::{sensitive, safe_text};
fn read(path: &Path, allowed: &Path) -> Result<String, String> {
    let data = crate::capabilities::workspace::safe_paths::read_regular(
        path,
        &[allowed.to_path_buf()],
        MAX_READ,
    )
    .map_err(|_| "CONTEXT_READ_DENIED")?;
    let text = String::from_utf8(data).map_err(|_| "CONTEXT_NOT_TEXT")?;
    Ok(safe_text(&text))
}
fn no_reparse_path(path: &Path) -> bool {
    use std::path::{Component, Prefix};
    if !path.is_absolute()
        || !matches!(
            path.components().next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
    {
        return false;
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::Prefix(_)) || !current.has_root() {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 => {
                return false;
            }
            Ok(_) => {}
            _ => return false,
        }
    }
    true
}

fn roots(workspace: &Path) -> Vec<PathBuf> {
    let mut roots = vec![
        workspace.join(".agents/skills"),
        workspace.join(".opencode/skills"),
    ];
    if let Some(home) = std::env::var_os("USERPROFILE") {
        for suffix in [
            ".agents/skills",
            ".codex/skills",
            ".codebuddy/skills",
            ".workbuddy/skills",
        ] {
            roots.push(PathBuf::from(&home).join(suffix));
        }
    }
    roots
        .into_iter()
        .filter(|path| no_reparse_path(path))
        .filter_map(|path| path.canonicalize().ok())
        .filter(|path| path.is_dir())
        .collect()
}
fn scan(
    dir: &Path,
    base: &Path,
    depth: usize,
    budget: &mut usize,
    files: &mut Vec<PathBuf>,
    only_skills: bool,
) -> bool {
    if depth > 6 || *budget == 0 {
        return true;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return true;
    };
    let mut limited = false;
    for entry in entries {
        if *budget == 0 {
            limited = true;
            break;
        }
        *budget -= 1;
        let Ok(entry) = entry else {
            limited = true;
            continue;
        };
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.')
            || ["node_modules", "target", "dist", "vendor"].contains(&name.as_str())
        {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&path) else {
            limited = true;
            continue;
        };
        // On Windows a directory junction is a reparse point, not necessarily
        // FileType::is_symlink(). Never recurse through junction/mount points.
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            continue;
        }
        if meta.is_dir() {
            limited |= scan(&path, base, depth + 1, budget, files, only_skills);
        } else if meta.is_file()
            && path.starts_with(base)
            && ((only_skills && name == "SKILL.md")
                || (!only_skills
                    && ["md", "txt", "rst"]
                        .contains(&path.extension().and_then(|e| e.to_str()).unwrap_or(""))))
        {
            files.push(path);
        }
    }
    limited
}
fn skill_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut budget = 4000;
    for dir in roots(root) {
        scan(&dir, &dir, 0, &mut budget, &mut files, true);
    }
    files.sort();
    files.dedup();
    files
}
pub fn execute(root: &Path, storage: &Path, request: Value) -> Result<Value, String> {
    execute_cancelable(root, storage, request, Arc::new(AtomicBool::new(false)))
}
pub(crate) fn execute_cancelable(root: &Path, storage: &Path, request: Value, cancellation: Arc<AtomicBool>) -> Result<Value, String> {
    execute_cancelable_with_admission(root, storage, request, cancellation, |_| Ok(()))
}

/// The native owner publishes intent only after local validation and
/// immediately before starting an external MCP executable. The callback
/// is host-only, not supplied by the request, model or server.
pub(super) fn execute_cancelable_with_admission(root: &Path, storage: &Path, request: Value,
    cancellation: Arc<AtomicBool>, before_spawn: impl FnOnce(&Value) -> Result<(), String>) -> Result<Value, String> {
    if !no_reparse_path(root) {
        return Err("CONTEXT_ROOT_INVALID".into());
    }
    let root = root.canonicalize().map_err(|_| "CONTEXT_ROOT_INVALID")?;
    if !root.is_dir() {
        return Err("CONTEXT_ROOT_INVALID".into());
    }
    match request["action"].as_str().unwrap_or("") {
        "skills_list" => {
            let skills = skill_files(&root)
                .into_iter()
                .take(256)
                .map(|file| {
                    let allowed = file.parent().and_then(Path::parent).unwrap_or(&root);
                    let description = read(&file, allowed)
                        .unwrap_or_default()
                        .lines()
                        .find(|line| line.starts_with("description:"))
                        .unwrap_or("")
                        .chars()
                        .take(240)
                        .collect::<String>();
                    json!({"path":file,"name":file.parent()
                        .and_then(Path::file_name).map(|n|n.to_string_lossy()),
                        "description":description})
                })
                .collect::<Vec<_>>();
            Ok(json!({"skills":skills,"platform":"windows","read_only":true}))
        }
        "skill_read" => {
            let raw = request["path"].as_str().ok_or("SKILL_PATH_REQUIRED")?;
            let raw_path = PathBuf::from(raw);
            if !no_reparse_path(&raw_path) {
                return Err("SKILL_OUTSIDE_ALLOWED_ROOTS".into());
            }
            let path = raw_path.canonicalize().map_err(|_| "SKILL_NOT_FOUND")?;
            if !skill_files(&root).contains(&path) {
                return Err("SKILL_OUTSIDE_ALLOWED_ROOTS".into());
            }
            let allowed = roots(&root)
                .into_iter()
                .find(|allowed| path.starts_with(allowed))
                .ok_or("SKILL_OUTSIDE_ALLOWED_ROOTS")?;
            Ok(json!({"path":path,"text":read(&path,&allowed)?}))
        }
        "knowledge_search" => {
            let query = request["query"]
                .as_str()
                .filter(|query| (2..=200).contains(&query.len()))
                .ok_or("KNOWLEDGE_QUERY_REQUIRED")?
                .to_lowercase();
            let docs_path = root.join("docs");
            if !no_reparse_path(&docs_path) {
                return Err("KNOWLEDGE_ROOT_UNSAFE".into());
            }
            let base = docs_path
                .canonicalize()
                .map_err(|_| "KNOWLEDGE_ROOT_MISSING")?;
            if !base.starts_with(&root) {
                return Err("KNOWLEDGE_ROOT_UNSAFE".into());
            }
            let mut files = Vec::new();
            let mut budget = 3000;
            let mut incomplete = scan(&base, &base, 0, &mut budget, &mut files, false);
            let mut hits = Vec::new();
            for file in files {
                match read(&file, &base) {
                    Ok(text) => {
                        for (line, content) in text.lines().enumerate() {
                            if !sensitive(content) && content.to_lowercase().contains(&query) {
                                hits.push(json!({"path":file,"line":line+1,
                                "text":content.chars().take(400).collect::<String>()}));
                                if hits.len() >= 40 {
                                    return Ok(
                                        json!({"hits":hits,"limited":true,"read_only":true}),
                                    );
                                }
                            }
                        }
                    }
                    Err(_) => {
                        incomplete = true;
                    }
                }
            }
            Ok(json!({"hits":hits,"limited":incomplete,"read_only":true}))
        }
        "mcp_list" => Ok(super::coding_mcp_windows::discover(&root, storage)),
        "mcp_review" | "mcp_tools" | "mcp_call" => {
            super::coding_mcp_windows::execute_cancelable_with_admission(&root, storage, request, cancellation, before_spawn)
        }
        _ => Err("CONTEXT_ACTION_UNKNOWN".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_execution_requires_explicit_approval_and_rejects_shell_actions() {
        let base = std::env::temp_dir().join(format!("ctx-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        for action in ["mcp_call", "mcp_tools"] {
            assert_eq!(
                execute(&base, &base, json!({"action":action})),
                Err("MCP_EXPLICIT_APPROVAL_REQUIRED".into())
            );
        }
        assert_eq!(
            execute(&base, &base, json!({"action":"shell_exec","approved":true})),
            Err("CONTEXT_ACTION_UNKNOWN".into())
        );
        std::fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn knowledge_search_redacts_sensitive_lines() {
        let base = std::env::temp_dir().join(format!("ctx-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(base.join("docs")).unwrap();
        std::fs::write(
            base.join("docs/note.md"),
            "a public architecture note\npassword: should-not-leak\n",
        )
        .unwrap();
        let result = execute(
            &base,
            &base,
            json!({"action":"knowledge_search","query":"architecture"}),
        )
        .unwrap();
        assert_eq!(result["hits"].as_array().unwrap().len(), 1);
        let sensitive = execute(
            &base,
            &base,
            json!({"action":"knowledge_search","query":"should-not-leak"}),
        )
        .unwrap();
        assert_eq!(sensitive["hits"].as_array().unwrap().len(), 0);
        std::fs::remove_dir_all(base).unwrap();
    }
}
