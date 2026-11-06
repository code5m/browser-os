//! Workspace text-save boundary. Every Linux operation after authorization is
//! rooted in a pinned parent directory FD. A stable sidecar lock serializes
//! cooperating BrowserOS writers; same-directory rename is atomic.
//!
//! This is not a kernel-wide compare-and-swap against arbitrary writers that
//! ignore the sidecar lock. The final check detects ordinary external changes,
//! but a non-cooperating writer can still race the final rename syscall.
use crate::security_policy as sp;
use crate::shared::invocation::check_invocation_source;
#[cfg(not(target_os = "linux"))]
use std::fs::{self, File, OpenOptions};
#[cfg(not(target_os = "linux"))]
use std::io::{Read, Write};
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
        save_if_unchanged_path(path, expected, content)
    }
}

#[cfg(not(target_os = "linux"))]
fn save_if_unchanged_path(path: &Path, expected: &[u8], content: &[u8]) -> Result<(), String> {
    if expected.len() > MAX_EDITOR_BYTES || content.len() > MAX_EDITOR_BYTES {
        return Err("FILE_SAVE_TOO_LARGE".into());
    }
    let parent = path.parent().ok_or("FILE_PARENT_INVALID")?;
    let name = path
        .file_name()
        .ok_or("FILE_NAME_INVALID")?
        .to_string_lossy();
    let metadata = fs::symlink_metadata(path).map_err(|_| "FILE_TARGET_MISSING")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("FILE_TARGET_NOT_REGULAR".into());
    }
    let lock_path = parent.join(format!(".{name}.browseros.lock"));
    let mut lock_options = OpenOptions::new();
    lock_options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        lock_options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let lock = lock_options
        .open(&lock_path)
        .map_err(|_| "FILE_LOCK_UNAVAILABLE")?;
    lock.lock().map_err(|_| "FILE_LOCK_UNAVAILABLE")?;

    let result = (|| {
        let live_meta = fs::symlink_metadata(path).map_err(|_| "FILE_TARGET_MISSING")?;
        if live_meta.file_type().is_symlink() || !live_meta.is_file() {
            return Err("FILE_TARGET_NOT_REGULAR".into());
        }
        if live_meta.len() > MAX_EDITOR_BYTES as u64 {
            return Err("FILE_SAVE_TOO_LARGE".into());
        }
        let mut current = Vec::with_capacity(live_meta.len() as usize);
        let mut reader = OpenOptions::new();
        reader.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            reader.custom_flags(libc::O_NOFOLLOW);
        }
        reader
            .open(path)
            .and_then(|file| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    let opened = file.metadata()?;
                    if (opened.dev(), opened.ino()) != (live_meta.dev(), live_meta.ino()) {
                        return Err(std::io::Error::other("FILE_CHANGED"));
                    }
                }
                file.take(MAX_EDITOR_BYTES as u64 + 1)
                    .read_to_end(&mut current)
            })
            .map_err(|_| "FILE_READ_FAILED")?;
        if current != expected {
            return Err("FILE_CONFLICT".into());
        }

        let temp = parent.join(format!(".{name}.browseros-{}.tmp", uuid::Uuid::new_v4()));
        let staged_result = (|| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
            }
            let mut staged = options.open(&temp).map_err(|_| "FILE_STAGE_FAILED")?;
            staged.write_all(content).map_err(|_| "FILE_STAGE_FAILED")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                staged
                    .set_permissions(fs::Permissions::from_mode(
                        live_meta.permissions().mode() & 0o777,
                    ))
                    .map_err(|_| "FILE_STAGE_FAILED")?;
            }
            #[cfg(not(unix))]
            staged
                .set_permissions(live_meta.permissions())
                .map_err(|_| "FILE_STAGE_FAILED")?;
            staged.sync_all().map_err(|_| "FILE_STAGE_FAILED")?;

            let latest = fs::symlink_metadata(path).map_err(|_| "FILE_TARGET_MISSING")?;
            if latest.file_type().is_symlink() || !latest.is_file() {
                return Err("FILE_TARGET_NOT_REGULAR".into());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if (
                    latest.dev(),
                    latest.ino(),
                    latest.mtime(),
                    latest.mtime_nsec(),
                    latest.len(),
                ) != (
                    live_meta.dev(),
                    live_meta.ino(),
                    live_meta.mtime(),
                    live_meta.mtime_nsec(),
                    live_meta.len(),
                ) {
                    return Err("FILE_CONFLICT".into());
                }
            }
            fs::rename(&temp, path).map_err(|_| "FILE_REPLACE_FAILED")?;
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| "FILE_DURABILITY_UNCERTAIN".to_string())
        })();
        if staged_result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        staged_result
    })();
    drop(lock);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Barrier};

    fn fixture() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("browseros-cas-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("entry.txt");
        fs::write(&file, "before").unwrap();
        (dir, file)
    }

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
}
