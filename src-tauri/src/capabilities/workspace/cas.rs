//! Workspace text-save boundary. Every Linux operation after authorization is
//! rooted in a pinned parent directory FD. A stable sidecar lock serializes
//! cooperating BrowserOS writers; same-directory rename is atomic.
//!
//! This is not a kernel-wide compare-and-swap against arbitrary writers that
//! ignore the sidecar lock. The final check detects ordinary external changes,
//! but a non-cooperating writer can still race the final rename syscall.
use crate::security_policy as sp;
use crate::shared::invocation::check_invocation_source;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const MAX_EDITOR_BYTES: usize = 8 * 1024 * 1024;

#[tauri::command]
pub async fn write_file_cas(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
    expected_content: String,
    content: String,
) -> Result<(), String> {
    check_invocation_source(&webview, "write_file_cas", None, &app)?;
    // XP-011R1: Windows has no handle-anchored CAS backend yet. Do not allow
    // the former path-only fallback to write after checking a canonical path.
    #[cfg(not(target_os = "linux"))]
    return Err("FILE_SAVE_PLATFORM_UNSUPPORTED".into());
    sp::check_text_field("content", &content, MAX_EDITOR_BYTES).map_err(|e| e.to_string())?;
    sp::check_text_field("expected_content", &expected_content, MAX_EDITOR_BYTES)
        .map_err(|e| e.to_string())?;
    let roots = crate::workspace::allowed_roots(&app);
    let canonical = sp::check_path_within_roots(&path, &roots).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        save_if_unchanged(
            &canonical,
            &roots,
            expected_content.as_bytes(),
            content.as_bytes(),
        )
    })
    .await
    .map_err(|_| "FILE_SAVE_WORKER_FAILED".to_string())?
}

fn save_if_unchanged(
    path: &Path,
    roots: &[PathBuf],
    expected: &[u8],
    content: &[u8],
) -> Result<(), String> {
    if expected.len() > MAX_EDITOR_BYTES || content.len() > MAX_EDITOR_BYTES {
        return Err("FILE_SAVE_TOO_LARGE".into());
    }
    #[cfg(target_os = "linux")]
    {
        super::safe_paths::save_if_unchanged(path, roots, expected, content, MAX_EDITOR_BYTES)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (path, roots, expected, content);
        Err("FILE_SAVE_PLATFORM_UNSUPPORTED".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(target_os = "linux")]
    use std::sync::{Arc, Barrier};

    fn fixture() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("browseros-cas-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("entry.txt");
        fs::write(&file, "before").unwrap();
        (dir, file)
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn replace_is_atomic_and_conflicts_preserve_external_content() {
        let (dir, file) = fixture();
        let roots = [dir.clone()];
        save_if_unchanged(&file, &roots, b"before", b"saved").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "saved");
        fs::write(&file, "someone else's update").unwrap();
        assert_eq!(
            save_if_unchanged(&file, &roots, b"saved", b"stale"),
            Err("FILE_CONFLICT".into())
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), "someone else's update");
        assert!(!fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().ends_with(".tmp")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn concurrent_cooperating_writers_from_distinct_file_descriptors_have_one_winner() {
        let (dir, file) = fixture();
        let roots = [dir.clone()];
        let gate = Arc::new(Barrier::new(3));
        let writers: Vec<_> = (0..2)
            .map(|i| {
                let file = file.clone();
                let root = dir.clone();
                let gate = Arc::clone(&gate);
                std::thread::spawn(move || {
                    gate.wait();
                    save_if_unchanged(&file, &[root], b"before", format!("edit{i}").as_bytes())
                })
            })
            .collect();
        gate.wait();
        let results: Vec<_> = writers
            .into_iter()
            .map(|writer| writer.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| result
                    .as_ref()
                    .err()
                    .is_some_and(|error| error == "FILE_CONFLICT"))
                .count(),
            1
        );
        assert!(roots[0].join("entry.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn target_symlink_and_oversized_draft_are_denied() {
        let (dir, file) = fixture();
        let roots = [dir.clone()];
        assert_eq!(
            save_if_unchanged(&file, &roots, b"before", &vec![b'a'; MAX_EDITOR_BYTES + 1]),
            Err("FILE_SAVE_TOO_LARGE".into())
        );
        #[cfg(unix)]
        {
            let link = dir.join("link.txt");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert_eq!(
                save_if_unchanged(&link, &roots, b"before", b"new"),
                Err("FILE_TARGET_NOT_REGULAR".into())
            );
            assert_eq!(fs::read_to_string(&file).unwrap(), "before");
        }
        fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn non_linux_save_is_denied_without_mutating_the_file() {
        let (dir, file) = fixture();
        let roots = [dir.clone()];
        assert_eq!(
            save_if_unchanged(&file, &roots, b"before", b"unexpected"),
            Err("FILE_SAVE_PLATFORM_UNSUPPORTED".into())
        );
        assert_eq!(fs::read(&file).unwrap(), b"before");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
