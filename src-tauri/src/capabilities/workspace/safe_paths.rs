//! Root-anchored filesystem operations. No path-based I/O after authorization.
//! For Linux, each directory component is opened without following symlinks;
//! rename uses those pinned directory descriptors. If unavailable, fail closed.
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use sha2::Digest;
    use std::ffi::{CString, OsStr};
    use std::fs::File;
    use std::io::{self, Read, Write};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Component;

    fn bad_path() -> io::Error {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "path outside allowed roots",
        )
    }
    fn name(value: &OsStr) -> io::Result<CString> {
        CString::new(value.as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in path"))
    }
    fn open_child_dir(parent: &File, child: &OsStr) -> io::Result<File> {
        let segment = name(child)?;
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                segment.as_ptr(),
                libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    fn walk_beneath(mut dir: File, relative: &Path) -> io::Result<File> {
        for component in relative.components() {
            match component {
                Component::Normal(part) => dir = open_child_dir(&dir, part)?,
                Component::CurDir => {}
                _ => return Err(bad_path()),
            }
        }
        Ok(dir)
    }
    fn open_abs_dir_no_symlinks(path: &Path) -> io::Result<File> {
        if !path.is_absolute() {
            return Err(bad_path());
        }
        let root = File::open("/")?;
        walk_beneath(root, path.strip_prefix("/").map_err(|_| bad_path())?)
    }
    // Canonical path is a preflight only: the final access is always rooted in
    // a verified root directory FD, with no following symlinks during lookup.
    pub(super) fn open_allowed_dir(canonical: &Path, roots: &[PathBuf]) -> io::Result<File> {
        for root in roots {
            let Ok(root_path) = root.canonicalize() else {
                continue;
            };
            let Ok(relative) = canonical.strip_prefix(&root_path) else {
                continue;
            };
            // The root itself must not be reached by following a swapped symlink.
            let base = open_abs_dir_no_symlinks(&root_path)?;
            return walk_beneath(base, relative);
        }
        Err(bad_path())
    }
    fn open_regular_at(parent: &File, filename: &OsStr) -> io::Result<File> {
        let filename = name(filename)?;
        // NONBLOCK avoids a FIFO/block-device stall before fstat rejects it.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                filename.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a regular file",
            ));
        }
        Ok(file)
    }
    pub(super) fn read_regular(
        canonical: &Path,
        roots: &[PathBuf],
        limit: usize,
    ) -> io::Result<Vec<u8>> {
        let parent = canonical.parent().ok_or_else(bad_path)?;
        let filename = canonical.file_name().ok_or_else(bad_path)?;
        let dir = open_allowed_dir(parent, roots)?;
        let file = open_regular_at(&dir, filename)?;
        if file.metadata()?.len() > limit as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file size limit exceeded",
            ));
        }
        let mut bytes = Vec::new();
        file.take((limit as u64) + 1).read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file size limit exceeded",
            ));
        }
        Ok(bytes)
    }
    fn rename_bound(
        src_parent: &File,
        source: &OsStr,
        dst_parent: &File,
        dest: &OsStr,
    ) -> io::Result<()> {
        let source = name(source)?;
        let dest = name(dest)?;
        let status = unsafe {
            libc::renameat2(
                src_parent.as_raw_fd(),
                source.as_ptr(),
                dst_parent.as_raw_fd(),
                dest.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if status == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
    pub(super) fn move_no_replace(
        src: &Path,
        dest_dir: &Path,
        roots: &[PathBuf],
    ) -> io::Result<()> {
        let parent = src.parent().ok_or_else(bad_path)?;
        let filename = src.file_name().ok_or_else(bad_path)?;
        let source_dir = open_allowed_dir(parent, roots)?;
        let target_dir = open_allowed_dir(dest_dir, roots)?;
        rename_bound(&source_dir, filename, &target_dir, filename)
    }

    fn open_lock_at(parent: &File, lock_name: &OsStr) -> io::Result<File> {
        let lock_name = name(lock_name)?;
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                lock_name.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "lock is not regular",
            ));
        }
        Ok(file)
    }

    fn create_temp_at(parent: &File, temp_name: &OsStr) -> io::Result<File> {
        let temp_name = name(temp_name)?;
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                temp_name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    fn unlink_at(parent: &File, filename: &OsStr) {
        if let Ok(filename) = name(filename) {
            unsafe {
                libc::unlinkat(parent.as_raw_fd(), filename.as_ptr(), 0);
            }
        }
    }

    fn replace_at(parent: &File, source: &OsStr, target: &OsStr) -> io::Result<()> {
        let source = name(source)?;
        let target = name(target)?;
        let result = unsafe {
            libc::renameat(
                parent.as_raw_fd(),
                source.as_ptr(),
                parent.as_raw_fd(),
                target.as_ptr(),
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn sync_directory(parent: &File) -> io::Result<()> {
        let dot = CString::new(".").expect("static path");
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                dot.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let directory = unsafe { File::from_raw_fd(fd) };
        directory.sync_all()
    }

    fn same_file_identity(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
        use std::os::unix::fs::MetadataExt;
        (
            left.dev(),
            left.ino(),
            left.mtime(),
            left.mtime_nsec(),
            left.len(),
        ) == (
            right.dev(),
            right.ino(),
            right.mtime(),
            right.mtime_nsec(),
            right.len(),
        )
    }

    /// Compare and replace an existing regular file using only a pinned parent
    /// directory FD. The advisory sidecar lock serializes cooperating BrowserOS
    /// writers; the second content/identity check detects ordinary external
    /// edits before rename. A non-cooperating writer can still race the final
    /// rename syscall, which is not a kernel CAS primitive.
    fn save_if_unchanged_at(
        parent: &File,
        filename: &OsStr,
        expected: &[u8],
        content: &[u8],
        limit: usize,
    ) -> Result<(), String> {
        if expected.len() > limit || content.len() > limit {
            return Err("FILE_SAVE_TOO_LARGE".into());
        }
        let digest = format!("{:x}", sha2::Sha256::digest(filename.as_bytes()));
        let lock_name = std::ffi::OsString::from(format!(".browseros-cas-{digest}.lock"));
        let lock = open_lock_at(parent, &lock_name).map_err(|_| "FILE_LOCK_UNAVAILABLE")?;
        lock.lock().map_err(|_| "FILE_LOCK_UNAVAILABLE")?;

        let result = (|| {
            let current_file = open_regular_at(parent, filename).map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    "FILE_TARGET_MISSING"
                } else {
                    "FILE_TARGET_NOT_REGULAR"
                }
            })?;
            let initial_meta = current_file.metadata().map_err(|_| "FILE_READ_FAILED")?;
            if initial_meta.len() > limit as u64 {
                return Err("FILE_SAVE_TOO_LARGE".into());
            }
            let mut current = Vec::with_capacity(initial_meta.len() as usize);
            current_file
                .take(limit as u64 + 1)
                .read_to_end(&mut current)
                .map_err(|_| "FILE_READ_FAILED")?;
            if current.len() > limit {
                return Err("FILE_SAVE_TOO_LARGE".into());
            }
            if current != expected {
                return Err("FILE_CONFLICT".into());
            }

            let temp_name =
                std::ffi::OsString::from(format!(".browseros-cas-{}.tmp", uuid::Uuid::new_v4()));
            let stage_result = (|| {
                let mut staged =
                    create_temp_at(parent, &temp_name).map_err(|_| "FILE_STAGE_FAILED")?;
                staged.write_all(content).map_err(|_| "FILE_STAGE_FAILED")?;
                use std::os::unix::fs::PermissionsExt;
                staged
                    .set_permissions(std::fs::Permissions::from_mode(
                        initial_meta.permissions().mode() & 0o777,
                    ))
                    .map_err(|_| "FILE_STAGE_FAILED")?;
                staged.sync_all().map_err(|_| "FILE_STAGE_FAILED")?;

                let latest_file = open_regular_at(parent, filename).map_err(|_| "FILE_CONFLICT")?;
                let latest_meta = latest_file.metadata().map_err(|_| "FILE_CONFLICT")?;
                if !same_file_identity(&initial_meta, &latest_meta) {
                    return Err("FILE_CONFLICT".into());
                }
                let mut latest = Vec::with_capacity(latest_meta.len() as usize);
                latest_file
                    .take(limit as u64 + 1)
                    .read_to_end(&mut latest)
                    .map_err(|_| "FILE_CONFLICT")?;
                if latest != expected {
                    return Err("FILE_CONFLICT".into());
                }

                replace_at(parent, &temp_name, filename).map_err(|_| "FILE_REPLACE_FAILED")?;
                sync_directory(parent).map_err(|_| "FILE_DURABILITY_UNCERTAIN")?;
                Ok(())
            })();
            if stage_result.is_err() {
                unlink_at(parent, &temp_name);
            }
            stage_result
        })();
        drop(lock);
        result
    }

    pub(super) fn save_if_unchanged(
        canonical: &Path,
        roots: &[PathBuf],
        expected: &[u8],
        content: &[u8],
        limit: usize,
    ) -> Result<(), String> {
        let parent_path = canonical.parent().ok_or("FILE_PARENT_INVALID")?;
        let filename = canonical.file_name().ok_or("FILE_NAME_INVALID")?;
        let parent =
            open_allowed_dir(parent_path, roots).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
        save_if_unchanged_at(&parent, filename, expected, content, limit)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::os::unix::fs::symlink;
        fn fixture() -> PathBuf {
            let root = std::env::temp_dir().join(format!(
                "browseros_descriptor_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&root).unwrap();
            root
        }
        #[test]
        fn destination_symlink_swap_cannot_escape_authorized_root() {
            let base = fixture();
            let trusted = base.join("trusted");
            let outside = base.join("outside");
            let dest = trusted.join("dest");
            std::fs::create_dir(&trusted).unwrap();
            std::fs::create_dir(&dest).unwrap();
            std::fs::create_dir(&outside).unwrap();
            std::fs::write(trusted.join("source"), b"safe").unwrap();
            let roots = [trusted.clone()];
            let source_fd = open_allowed_dir(&trusted, &roots).unwrap();
            let destination_fd = open_allowed_dir(&dest, &roots).unwrap();
            std::fs::rename(&dest, trusted.join("original-dest")).unwrap();
            symlink(&outside, &dest).unwrap();
            rename_bound(
                &source_fd,
                OsStr::new("source"),
                &destination_fd,
                OsStr::new("source"),
            )
            .unwrap();
            assert!(!outside.join("source").exists());
            assert_eq!(
                std::fs::read(trusted.join("original-dest/source")).unwrap(),
                b"safe"
            );
            assert!(
                open_allowed_dir(&dest, &roots).is_err(),
                "swapped symlink denied"
            );
            std::fs::remove_dir_all(base).unwrap();
        }
        #[test]
        fn image_path_swapped_for_external_symlink_is_denied() {
            let base = fixture();
            let trusted = base.join("trusted");
            let outside = base.join("outside");
            std::fs::create_dir(&trusted).unwrap();
            std::fs::create_dir(&outside).unwrap();
            let image = trusted.join("image.jpg");
            std::fs::write(&image, b"safe").unwrap();
            let canonical = image.canonicalize().unwrap();
            let roots = [trusted.clone()];
            assert_eq!(read_regular(&canonical, &roots, 1024).unwrap(), b"safe");
            std::fs::rename(&image, trusted.join("original.jpg")).unwrap();
            let external = outside.join("external.jpg");
            std::fs::write(&external, b"secret").unwrap();
            symlink(external, image).unwrap();
            assert!(read_regular(&canonical, &roots, 1024).is_err());
            assert!(read_regular(&trusted.join("original.jpg"), &roots, 3).is_err());
            std::fs::remove_dir_all(base).unwrap();
        }
        #[test]
        fn parent_directory_replacement_cannot_redirect_pinned_cas_write() {
            let base = fixture();
            let trusted = base.join("trusted");
            let parent = trusted.join("workspace");
            let outside = base.join("outside");
            std::fs::create_dir_all(&parent).unwrap();
            std::fs::create_dir(&outside).unwrap();
            let target = parent.join("entry.txt");
            std::fs::write(&target, b"before").unwrap();
            let outside_target = outside.join("entry.txt");
            std::fs::write(&outside_target, b"outside").unwrap();
            let roots = [trusted.clone()];
            let pinned_parent = open_allowed_dir(&parent, &roots).unwrap();

            std::fs::rename(&parent, trusted.join("workspace-original")).unwrap();
            symlink(&outside, &parent).unwrap();

            save_if_unchanged_at(
                &pinned_parent,
                OsStr::new("entry.txt"),
                b"before",
                b"saved",
                1024,
            )
            .unwrap();

            assert_eq!(
                std::fs::read(trusted.join("workspace-original/entry.txt")).unwrap(),
                b"saved"
            );
            assert_eq!(std::fs::read(&outside_target).unwrap(), b"outside");
            assert!(open_allowed_dir(&parent, &roots).is_err());
            std::fs::remove_dir_all(base).unwrap();
        }

        #[test]
        fn ordinary_sibling_move_works_and_conflicts_fail_closed() {
            let base = fixture();
            let trusted = base.join("trusted");
            let a = trusted.join("a");
            let b = trusted.join("b");
            std::fs::create_dir_all(&a).unwrap();
            std::fs::create_dir(&b).unwrap();
            std::fs::write(a.join("file"), b"first").unwrap();
            let roots = [trusted.clone()];
            move_no_replace(&a.join("file"), &b, &roots).unwrap();
            assert_eq!(std::fs::read(b.join("file")).unwrap(), b"first");
            std::fs::write(a.join("file"), b"second").unwrap();
            assert!(move_no_replace(&a.join("file"), &b, &roots).is_err());
            assert_eq!(std::fs::read(b.join("file")).unwrap(), b"first");
            std::fs::remove_dir_all(base).unwrap();
        }
    }
}

// Windows read authorization and handle-relative mutation. Existing OS
// handles and kernel-resolved paths are checked against pinned allowed roots.
// CAS creates lock/stage files relative to the pinned parent and never falls
// back to path-only writes after authorization.
#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::{self, Read};
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const FILE_SHARE_READ_WRITE_DELETE: u32 = 0x0000_0007;

    fn denied() -> io::Error {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "WINDOWS_FILE_OUTSIDE_ALLOWED_ROOT",
        )
    }

    fn invalid_input() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, "WINDOWS_FILE_PATH_INVALID")
    }

    fn reject_reparse_ancestors(path: &Path) -> io::Result<()> {
        if !path.is_absolute() || path.to_string_lossy().starts_with(r"\\") {
            return Err(denied());
        }
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component.as_os_str());
            match std::fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 => {
                    return Err(denied());
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn system_tool(name: &str) -> io::Result<PathBuf> {
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
        let mut buffer = [0u16; 512];
        let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
        if length == 0 || length as usize >= buffer.len() {
            return Err(io::Error::last_os_error());
        }
        Ok(PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize])).join(name))
    }

    fn current_user_identity() -> io::Result<(String, String)> {
        let whoami = system_tool("whoami.exe")?;
        let output = std::process::Command::new(whoami)
            .args(["/user", "/fo", "csv", "/nh"])
            .output()?;
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "WINDOWS_USER_SID_UNAVAILABLE",
            ));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let line = text
            .lines()
            .find(|line| line.contains("S-1-"))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "WINDOWS_USER_SID_UNAVAILABLE",
                )
            })?;
        let fields: Vec<_> = line.trim_start_matches('\u{feff}').split(',').collect();
        let account = fields
            .first()
            .map(|value| value.trim().trim_matches('"'))
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "WINDOWS_USER_ACCOUNT_UNAVAILABLE",
                )
            })?;
        let sid = fields
            .get(1)
            .map(|value| value.trim().trim_matches('"'))
            .filter(|value| {
                value.starts_with("S-1-")
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || byte == b'-')
            })
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::PermissionDenied, "WINDOWS_USER_SID_INVALID")
            })?;
        Ok((account.to_string(), sid.to_string()))
    }

    /// Protect a private app-owned directory and every existing descendant.
    /// The root DACL is owner/SYSTEM-only; children are reset to inherit only
    /// that DACL. Reparse points, UNC paths and special files fail closed.
    pub(super) fn ensure_private_directory(path: &Path) -> io::Result<()> {
        reject_reparse_ancestors(path)?;
        std::fs::create_dir_all(path)?;
        reject_reparse_ancestors(path)?;
        let root = open(path, true)?;
        let expected = final_path(&root)?;
        let (account, sid) = current_user_identity()?;
        let icacls = system_tool("icacls.exe")?;
        let mut pending = vec![path.to_path_buf()];
        let mut objects = Vec::<PathBuf>::new();
        const MAX_OBJECTS: usize = 50_000;
        const MAX_DEPTH: usize = 64;
        while let Some(directory) = pending.pop() {
            let depth = directory
                .strip_prefix(path)
                .map(|relative| relative.components().count())
                .unwrap_or(MAX_DEPTH);
            if depth > MAX_DEPTH {
                return Err(denied());
            }
            for entry in std::fs::read_dir(&directory)? {
                let entry = entry?;
                let child = entry.path();
                let metadata = std::fs::symlink_metadata(&child)?;
                if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
                    || !(metadata.is_file() || metadata.is_dir())
                {
                    return Err(denied());
                }
                if final_path(&open(&child, metadata.is_dir())?)?
                    .to_lowercase()
                    .starts_with(&(expected.trim_end_matches('\\').to_lowercase() + "\\"))
                    == false
                {
                    return Err(denied());
                }
                objects.push(child.clone());
                if objects.len() > MAX_OBJECTS {
                    return Err(denied());
                }
                if metadata.is_dir() {
                    pending.push(child);
                }
            }
        }

        let owner_sid = format!("*{sid}");
        let set_root_owner = std::process::Command::new(&icacls)
            .arg(path)
            .arg("/setowner")
            .arg(&owner_sid)
            .output()?;
        if !set_root_owner.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "WINDOWS_PRIVATE_DIR_OWNER_SET",
            ));
        }
        let reset_root = std::process::Command::new(&icacls)
            .arg(path)
            .arg("/reset")
            .output()?;
        if !reset_root.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "WINDOWS_PRIVATE_DIR_ACL_RESET",
            ));
        }
        let suffix = ":(OI)(CI)F";
        let grant_user = format!("*{sid}{suffix}");
        let grant_system = format!("*S-1-5-18{suffix}");
        let secure_root = std::process::Command::new(&icacls)
            .arg(path)
            .args(["/inheritance:r", "/grant:r"])
            .arg(&grant_user)
            .arg(&grant_system)
            .output()?;
        if !secure_root.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "WINDOWS_PRIVATE_DIR_ACL_SET",
            ));
        }
        for child in &objects {
            let owner = std::process::Command::new(&icacls)
                .arg(child)
                .arg("/setowner")
                .arg(&owner_sid)
                .output()?;
            if !owner.status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "WINDOWS_PRIVATE_CHILD_OWNER_SET",
                ));
            }
            let reset = std::process::Command::new(&icacls)
                .arg(child)
                .arg("/reset")
                .output()?;
            if !reset.status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "WINDOWS_PRIVATE_CHILD_ACL_RESET",
                ));
            }
        }
        let verify = std::process::Command::new(&icacls)
            .arg(path)
            .arg("/T")
            .output()?;
        if !verify.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "WINDOWS_PRIVATE_DIR_ACL_VERIFY",
            ));
        }
        let listing = String::from_utf8_lossy(&verify.stdout);
        let entries: Vec<_> = listing.lines().filter(|line| line.contains(":(")).collect();
        let object_count = objects.len() + 1;
        let user_entries = entries
            .iter()
            .filter(|line| line.contains(&sid) || line.contains(&account))
            .count();
        let system_entries = entries
            .iter()
            .filter(|line| {
                line.contains("S-1-5-18") || line.to_ascii_uppercase().contains("SYSTEM")
            })
            .count();
        if entries.len() != object_count * 2
            || user_entries != object_count
            || system_entries != object_count
            || listing.to_ascii_lowercase().contains("everyone")
            || listing.to_ascii_lowercase().contains("authenticated users")
            || listing.contains("S-1-1-0")
            || listing.contains("S-1-5-32-545")
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "WINDOWS_PRIVATE_DIR_ACL_NOT_EXCLUSIVE",
            ));
        }
        Ok(())
    }
    fn normalized_path(value: &str) -> String {
        let value = value.replace('/', "\\");
        let value = value.strip_prefix("\\\\?\\").unwrap_or(&value);
        value.trim_end_matches('\\').to_lowercase()
    }

    fn verify_path_handle(path: &Path, file: &File) -> io::Result<()> {
        // A final-component OPEN_REPARSE_POINT flag does not prevent an
        // intermediate junction from being followed. Check every component,
        // then compare the kernel-resolved handle path with the current
        // canonical path. The second check detects a parent swapped between
        // the preflight and CreateFileW.
        reject_reparse_ancestors(path)?;
        let actual = final_path(file)?;
        let canonical = path.canonicalize()?.to_string_lossy().into_owned();
        if normalized_path(&actual) != normalized_path(&canonical) {
            return Err(denied());
        }
        reject_reparse_ancestors(path)?;
        Ok(())
    }

    fn open(path: &Path, directory: bool) -> io::Result<File> {
        reject_reparse_ancestors(path)?;
        let mut options = OpenOptions::new();
        options
            .read(true)
            .share_mode(FILE_SHARE_READ_WRITE_DELETE)
            .custom_flags(
                FILE_FLAG_OPEN_REPARSE_POINT
                    | if directory {
                        FILE_FLAG_BACKUP_SEMANTICS
                    } else {
                        0
                    },
            );
        let file = options.open(path)?;
        let meta = file.metadata()?;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(denied());
        }
        if directory && !meta.is_dir() || !directory && !meta.is_file() {
            return Err(denied());
        }
        verify_path_handle(path, &file)?;
        Ok(file)
    }
    fn open_destination_directory(path: &Path) -> io::Result<File> {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
        const FILE_ADD_FILE: u32 = 0x0000_0002;
        const FILE_ADD_SUBDIRECTORY: u32 = 0x0000_0004;
        const SYNCHRONIZE: u32 = 0x0010_0000;
        let mut options = OpenOptions::new();
        options
            .access_mode(FILE_READ_ATTRIBUTES | FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ_WRITE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
        reject_reparse_ancestors(path)?;
        let directory = options.open(path)?;
        let metadata = directory.metadata()?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 || !metadata.is_dir() {
            return Err(denied());
        }
        verify_path_handle(path, &directory)?;
        Ok(directory)
    }

    fn final_path(file: &File) -> io::Result<String> {
        let handle = file.as_raw_handle() as _;
        let n = unsafe { GetFinalPathNameByHandleW(handle, std::ptr::null_mut(), 0, 0) };
        if n == 0 || n > 32768 {
            return Err(io::Error::last_os_error());
        }
        let mut buf = vec![0u16; n as usize + 1];
        let used =
            unsafe { GetFinalPathNameByHandleW(handle, buf.as_mut_ptr(), buf.len() as u32, 0) };
        if used == 0 || used as usize >= buf.len() {
            return Err(io::Error::last_os_error());
        }
        // Windows paths are case-insensitive. The trailing path separator
        // enforces a whole-directory boundary rather than a prefix match.
        Ok(String::from_utf16_lossy(&buf[..used as usize]).to_lowercase())
    }
    fn open_authorized_root(path: &Path, roots: &[PathBuf]) -> io::Result<File> {
        let path_key = normalized_path(&path.canonicalize()?.to_string_lossy());
        for root in roots {
            let Ok(root_canonical) = root.canonicalize() else {
                continue;
            };
            let root_key = normalized_path(&root_canonical.to_string_lossy());
            let prefix = format!("{}\\", root_key.trim_end_matches('\\'));
            if path_key != root_key && !path_key.starts_with(&prefix) {
                continue;
            }
            // Pin the allowed root before opening the requested child. A
            // later rename/junction swap cannot redirect the child outside
            // this already-open directory identity.
            return open(root, true);
        }
        Err(denied())
    }

    fn authorize_with_root(file: &File, root: &File) -> io::Result<String> {
        let actual = normalized_path(&final_path(file)?);
        let root_path = normalized_path(&final_path(root)?);
        if actual == root_path
            || actual.starts_with(&format!("{}\\", root_path.trim_end_matches('\\')))
        {
            return Ok(actual);
        }
        Err(denied())
    }
    /// No-overwrite rename by existing source/destination OS handles.
    /// FILE_RENAME_INFO RootDirectory pins the actual destination directory,
    /// and ReplaceIfExists=false avoids path-only check-then-rename races.
    pub(super) fn move_no_replace(
        src: &Path,
        dest_dir: &Path,
        roots: &[PathBuf],
    ) -> io::Result<()> {
        use std::ffi::c_void;
        use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::{FileRenameInfo, SetFileInformationByHandle};
        const DELETE: u32 = 0x0001_0000;
        const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;

        if !src.is_absolute() || !dest_dir.is_absolute() {
            return Err(denied());
        }
        reject_reparse_ancestors(src)?;
        reject_reparse_ancestors(dest_dir)?;
        let filename = src.file_name().ok_or_else(denied)?;
        let source_parent = src.parent().ok_or_else(denied)?;
        let source_root = open_authorized_root(source_parent, roots)?;
        let destination_root = open_authorized_root(dest_dir, roots)?;
        let text = filename.to_string_lossy();
        if text.is_empty() || text.contains(':') || text.ends_with('.') || text.ends_with(' ') {
            // Reject alternate data streams, path aliases and Win32 trailing
            // dot/space normalization before constructing an OS rename.
            return Err(denied());
        }

        let mut source_opts = OpenOptions::new();
        source_opts
            .access_mode(DELETE | FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ_WRITE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
        let source = source_opts.open(src)?;
        let meta = source.metadata()?;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || !(meta.is_file() || meta.is_dir())
        {
            return Err(denied());
        }
        verify_path_handle(src, &source)?;
        let source_path = authorize_with_root(&source, &source_root)?;
        // Rename requires FILE_ADD_FILE / FILE_ADD_SUBDIRECTORY rights on
        // the pinned destination directory handle. A read-only directory
        // handle passes path checks but fails every real FileRenameInfo call.
        let destination = open_destination_directory(dest_dir)?;
        let destination_path = authorize_with_root(&destination, &destination_root)?;
        if destination_path == source_path
            || meta.is_dir() && destination_path.starts_with(&(source_path + "\\"))
        {
            return Err(denied());
        }

        #[repr(C)]
        struct RenameHeader {
            flags: u32,
            root_directory: *mut c_void,
            filename_length: u32,
            filename: [u16; 1],
        }
        let chars: Vec<u16> = filename.encode_wide().collect();
        if chars.is_empty() || chars.len() > 32000 {
            return Err(denied());
        }
        let offset = std::mem::offset_of!(RenameHeader, filename);
        let size = offset + (chars.len() + 1) * std::mem::size_of::<u16>();
        let mut aligned = vec![0u64; (size + 7) / 8];
        let header = aligned.as_mut_ptr() as *mut RenameHeader;
        unsafe {
            (*header).flags = 0; // FileRenameInfo: FALSE = NO REPLACE
            (*header).root_directory = destination.as_raw_handle() as *mut c_void;
            (*header).filename_length = (chars.len() * 2) as u32;
            std::ptr::copy_nonoverlapping(
                chars.as_ptr(),
                (aligned.as_mut_ptr() as *mut u8).add(offset) as *mut u16,
                chars.len(),
            );
            if SetFileInformationByHandle(
                source.as_raw_handle() as _,
                FileRenameInfo,
                aligned.as_ptr() as *const c_void,
                size as u32,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub(super) fn read_regular(
        path: &Path,
        roots: &[PathBuf],
        limit: usize,
    ) -> io::Result<Vec<u8>> {
        if !path.is_absolute() || limit == 0 || limit > 64 * 1024 * 1024 {
            return Err(denied());
        }
        // Pin the authorized parent directory before opening the file. A
        // parent path replaced after this point cannot redefine the allowed
        // root for the file handle.
        let parent = path.parent().ok_or_else(denied)?;
        let root = open_authorized_root(parent, roots)?;
        let file = open(path, false)?;
        authorize_with_root(&file, &root)?;
        let length = file.metadata()?.len();
        if length > limit as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "FILE_READ_SIZE_LIMIT",
            ));
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "FILE_READ_SIZE_LIMIT",
            ));
        }
        Ok(bytes)
    }

    /// Open a single file name relative to an already-authorized directory
    /// handle. Unlike CreateFileW(parent.join(name)), NtCreateFile cannot be
    /// redirected by replacing the parent path after it has been pinned.
    fn open_object_at(
        parent: &File,
        filename: &std::ffi::OsStr,
        desired_access: u32,
        share_access: u32,
        create_disposition: u32,
        directory: bool,
    ) -> io::Result<File> {
        use std::os::windows::{ffi::OsStrExt, io::FromRawHandle};
        use windows_sys::{
            Wdk::{
                Foundation::OBJECT_ATTRIBUTES,
                Storage::FileSystem::{
                    FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN_REPARSE_POINT,
                    FILE_SYNCHRONOUS_IO_NONALERT, NtCreateFile,
                },
            },
            Win32::{
                Foundation::UNICODE_STRING, Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
                System::IO::IO_STATUS_BLOCK,
            },
        };
        let object_options = FILE_SYNCHRONOUS_IO_NONALERT
            | FILE_OPEN_REPARSE_POINT
            | if directory {
                FILE_DIRECTORY_FILE
            } else {
                FILE_NON_DIRECTORY_FILE
            };
        const SYNCHRONIZE: u32 = 0x0010_0000;
        if filename.is_empty()
            || filename.to_string_lossy().contains(['/', '\\', ':', '\0'])
            || filename.to_string_lossy().ends_with(['.', ' '])
        {
            return Err(denied());
        }
        let mut wide: Vec<u16> = filename.encode_wide().collect();
        let byte_length = wide
            .len()
            .checked_mul(2)
            .filter(|length| *length <= u16::MAX as usize)
            .ok_or_else(denied)?;
        let mut object_name = UNICODE_STRING {
            Length: byte_length as u16,
            MaximumLength: byte_length as u16,
            Buffer: wide.as_mut_ptr(),
        };
        let mut attributes: OBJECT_ATTRIBUTES = unsafe { std::mem::zeroed() };
        attributes.Length = std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32;
        attributes.RootDirectory = parent.as_raw_handle() as _;
        attributes.ObjectName = &mut object_name;
        attributes.Attributes = 0x0000_0040; // OBJ_CASE_INSENSITIVE
        let mut raw = std::ptr::null_mut();
        let mut status_block = IO_STATUS_BLOCK::default();
        let status = unsafe {
            NtCreateFile(
                &mut raw,
                desired_access | SYNCHRONIZE,
                &attributes,
                &mut status_block,
                std::ptr::null(),
                FILE_ATTRIBUTE_NORMAL,
                share_access,
                create_disposition,
                object_options,
                std::ptr::null(),
                0,
            )
        };
        if status < 0 || raw.is_null() {
            let kind = match status as u32 {
                0xc000_0034 | 0xc000_003a => io::ErrorKind::NotFound,
                0xc000_0022 | 0xc000_0043 => io::ErrorKind::PermissionDenied,
                _ => io::ErrorKind::Other,
            };
            return Err(io::Error::new(
                kind,
                format!("WINDOWS_HANDLE_RELATIVE_OPEN_FAILED_{:08x}", status as u32),
            ));
        }
        let file = unsafe { File::from_raw_handle(raw) };
        let metadata = file.metadata()?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || directory && !metadata.is_dir()
            || !directory && !metadata.is_file()
        {
            return Err(denied());
        }
        authorize_named_child(&file, parent, filename)?;
        Ok(file)
    }

    fn open_child_at(
        parent: &File,
        filename: &std::ffi::OsStr,
        desired_access: u32,
        share_access: u32,
        create_disposition: u32,
    ) -> io::Result<File> {
        open_object_at(
            parent,
            filename,
            desired_access,
            share_access,
            create_disposition,
            false,
        )
    }

    fn open_parent_tree(path: &Path) -> io::Result<File> {
        use windows_sys::Wdk::Storage::FileSystem::FILE_OPEN_IF;
        const FILE_ADD_FILE: u32 = 0x0000_0002;
        const FILE_ADD_SUBDIRECTORY: u32 = 0x0000_0004;
        const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
        const FILE_TRAVERSE: u32 = 0x0000_0020;
        if !path.is_absolute() {
            return Err(denied());
        }
        let mut cursor = path;
        let mut missing = Vec::new();
        while !cursor.exists() {
            missing.push(cursor.file_name().ok_or_else(denied)?.to_os_string());
            cursor = cursor.parent().ok_or_else(denied)?;
        }
        let mut directory = open_destination_directory(cursor)?;
        for name in missing.iter().rev() {
            directory = open_object_at(
                &directory,
                name,
                FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY | FILE_READ_ATTRIBUTES | FILE_TRAVERSE,
                FILE_SHARE_READ_WRITE_DELETE,
                FILE_OPEN_IF,
                true,
            )?;
        }
        Ok(directory)
    }

    fn authorize_named_child(
        file: &File,
        parent: &File,
        filename: &std::ffi::OsStr,
    ) -> io::Result<()> {
        let actual = normalized_path(&final_path(file)?);
        let parent = normalized_path(&final_path(parent)?);
        let expected = format!(
            "{}\\{}",
            parent.trim_end_matches('\\'),
            filename.to_string_lossy().to_lowercase()
        );
        if actual == expected {
            Ok(())
        } else {
            Err(denied())
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct FileIdentity {
        volume: u32,
        index: u64,
        size: u64,
        modified: (u32, u32),
    }

    fn identity(file: &File) -> io::Result<FileIdentity> {
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };
        let info = unsafe {
            let mut info = BY_HANDLE_FILE_INFORMATION::default();
            if GetFileInformationByHandle(file.as_raw_handle() as _, &mut info) == 0 {
                return Err(io::Error::last_os_error());
            }
            info
        };
        Ok(FileIdentity {
            volume: info.dwVolumeSerialNumber,
            index: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
            size: ((info.nFileSizeHigh as u64) << 32) | info.nFileSizeLow as u64,
            modified: (
                info.ftLastWriteTime.dwLowDateTime,
                info.ftLastWriteTime.dwHighDateTime,
            ),
        })
    }

    fn read_snapshot(file: &mut File, limit: usize) -> Result<(FileIdentity, Vec<u8>), String> {
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0))
            .map_err(|_| "FILE_READ_FAILED")?;
        let metadata = file.metadata().map_err(|_| "FILE_READ_FAILED")?;
        if metadata.len() > limit as u64 {
            return Err("FILE_SAVE_TOO_LARGE".into());
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "FILE_READ_FAILED")?;
        if bytes.len() > limit {
            return Err("FILE_SAVE_TOO_LARGE".into());
        }
        let identity = identity(file).map_err(|_| "FILE_IDENTITY_UNAVAILABLE")?;
        Ok((identity, bytes))
    }

    fn copy_dacl(source: &File, destination: &File) -> Result<(), String> {
        use windows_sys::Win32::{
            Foundation::LocalFree,
            Security::{
                ACL,
                Authorization::{GetSecurityInfo, SE_FILE_OBJECT, SetSecurityInfo},
                DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
            },
        };
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let get_status = unsafe {
            GetSecurityInfo(
                source.as_raw_handle() as _,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        if get_status != 0 || descriptor.is_null() {
            return Err("FILE_ACL_READ_FAILED".into());
        }
        let set_status = unsafe {
            SetSecurityInfo(
                destination.as_raw_handle() as _,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                dacl,
                std::ptr::null_mut(),
            )
        };
        unsafe {
            LocalFree(descriptor as _);
        }
        if set_status != 0 {
            return Err("FILE_ACL_PRESERVE_FAILED".into());
        }
        Ok(())
    }

    fn rename_replace_at(source: &File, parent: &File, target: &std::ffi::OsStr) -> io::Result<()> {
        use std::ffi::c_void;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{FileRenameInfo, SetFileInformationByHandle};
        #[repr(C)]
        struct RenameHeader {
            flags: u32,
            root_directory: *mut c_void,
            filename_length: u32,
            filename: [u16; 1],
        }
        let chars: Vec<u16> = target.encode_wide().collect();
        if chars.is_empty() || chars.len() > 32000 {
            return Err(denied());
        }
        let offset = std::mem::offset_of!(RenameHeader, filename);
        let size = offset + (chars.len() + 1) * std::mem::size_of::<u16>();
        let mut aligned = vec![0u64; (size + 7) / 8];
        let header = aligned.as_mut_ptr() as *mut RenameHeader;
        unsafe {
            (*header).flags = 1; // FileRenameInfo: TRUE = atomic replace in same directory.
            (*header).root_directory = parent.as_raw_handle() as *mut c_void;
            (*header).filename_length = (chars.len() * 2) as u32;
            std::ptr::copy_nonoverlapping(
                chars.as_ptr(),
                (aligned.as_mut_ptr() as *mut u8).add(offset) as *mut u16,
                chars.len(),
            );
            if SetFileInformationByHandle(
                source.as_raw_handle() as _,
                FileRenameInfo,
                aligned.as_ptr() as *const c_void,
                size as u32,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    fn delete_on_close(file: &File) {
        use std::ffi::c_void;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_DISPOSITION_INFO, FileDispositionInfo, SetFileInformationByHandle,
        };
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle() as _,
                FileDispositionInfo,
                &disposition as *const _ as *const c_void,
                std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            );
        }
    }

    fn remove_regular_at(parent: &File, filename: &std::ffi::OsStr) -> io::Result<()> {
        use std::ffi::c_void;
        use windows_sys::Wdk::Storage::FileSystem::FILE_OPEN;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_DISPOSITION_INFO, FileDispositionInfo, SetFileInformationByHandle,
        };
        const DELETE: u32 = 0x0001_0000;
        const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
        let file = open_child_at(
            parent,
            filename,
            DELETE | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ_WRITE_DELETE,
            FILE_OPEN,
        )?;
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        if unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle() as _,
                FileDispositionInfo,
                &disposition as *const _ as *const c_void,
                std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn remove_regular(canonical: &Path, roots: &[PathBuf]) -> io::Result<()> {
        if !canonical.is_absolute() {
            return Err(denied());
        }
        let name = canonical.file_name().ok_or_else(invalid_input)?;
        let parent_path = canonical.parent().ok_or_else(invalid_input)?;
        let root = open_authorized_root(parent_path, roots)?;
        let parent = open_destination_directory(parent_path)?;
        authorize_with_root(&parent, &root)?;
        remove_regular_at(&parent, name)
    }

    /// Compare and replace through a pinned parent directory. Both the lock and
    /// the unique stage file are created relative to that handle, so a renamed
    /// parent or replacement junction cannot redirect writes outside the root.
    fn save_if_unchanged_at(
        parent: &File,
        filename: &std::ffi::OsStr,
        expected: &[u8],
        content: &[u8],
        limit: usize,
    ) -> Result<(), String> {
        use sha2::Digest;
        use windows_sys::Wdk::Storage::FileSystem::{FILE_CREATE, FILE_OPEN, FILE_OPEN_IF};
        const DELETE: u32 = 0x0001_0000;
        const FILE_READ_DATA: u32 = 0x0000_0001;
        const FILE_WRITE_DATA: u32 = 0x0000_0002;
        const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
        const READ_CONTROL: u32 = 0x0002_0000;
        const WRITE_DAC: u32 = 0x0004_0000;
        if expected.len() > limit || content.len() > limit || limit == 0 {
            return Err("FILE_SAVE_TOO_LARGE".into());
        }
        let digest = format!("{:x}", sha2::Sha256::digest(filename.as_encoded_bytes()));
        let lock_name = std::ffi::OsString::from(format!(".browseros-cas-{digest}.lock"));
        let lock = open_child_at(
            parent,
            &lock_name,
            FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ_WRITE_DELETE,
            FILE_OPEN_IF,
        )
        .map_err(|_| "FILE_LOCK_UNAVAILABLE")?;
        lock.lock().map_err(|_| "FILE_LOCK_UNAVAILABLE")?;

        let result = (|| {
            let mut current = open_child_at(
                parent,
                filename,
                FILE_READ_DATA | FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ_WRITE_DELETE,
                FILE_OPEN,
            )
            .map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    "FILE_TARGET_MISSING"
                } else {
                    "FILE_TARGET_NOT_REGULAR"
                }
            })?;
            let (original_identity, actual) = read_snapshot(&mut current, limit)?;
            if actual != expected {
                return Err("FILE_CONFLICT".into());
            }
            drop(current);

            let mut stage = None;
            for _ in 0..8 {
                let candidate = std::ffi::OsString::from(format!(
                    ".browseros-cas-{}.tmp",
                    uuid::Uuid::new_v4()
                ));
                match open_child_at(
                    parent,
                    &candidate,
                    DELETE | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES | READ_CONTROL | WRITE_DAC,
                    0,
                    FILE_CREATE,
                ) {
                    Ok(file) => {
                        stage = Some(file);
                        break;
                    }
                    Err(_) => continue,
                }
            }
            let mut stage = stage.ok_or("FILE_STAGE_CREATE_FAILED")?;
            let stage_result = (|| {
                use std::io::Write;
                stage
                    .write_all(content)
                    .map_err(|_| "FILE_STAGE_WRITE_FAILED")?;
                stage.sync_all().map_err(|_| "FILE_STAGE_SYNC_FAILED")?;

                let mut latest = open_child_at(
                    parent,
                    filename,
                    FILE_READ_DATA | FILE_READ_ATTRIBUTES | READ_CONTROL,
                    FILE_SHARE_READ_WRITE_DELETE,
                    FILE_OPEN,
                )
                .map_err(|_| "FILE_CONFLICT")?;
                let (latest_identity, latest_bytes) = read_snapshot(&mut latest, limit)?;
                if latest_identity != original_identity || latest_bytes != expected {
                    return Err("FILE_CONFLICT".into());
                }
                copy_dacl(&latest, &stage)?;
                drop(latest);
                rename_replace_at(&stage, parent, filename).map_err(|_| "FILE_REPLACE_FAILED")?;
                // FlushFileBuffers on a directory is not supported by every
                // Windows filesystem. The stage itself is always flushed; a
                // directory flush is best-effort rather than a false failure.
                let _ = parent.sync_all();
                Ok(())
            })();
            if stage_result.is_err() {
                delete_on_close(&stage);
            }
            stage_result
        })();
        drop(lock);
        result
    }

    pub(super) fn save_if_unchanged(
        path: &Path,
        roots: &[PathBuf],
        expected: &[u8],
        content: &[u8],
        limit: usize,
    ) -> Result<(), String> {
        if !path.is_absolute() {
            return Err("FILE_PARENT_NOT_AUTHORIZED".into());
        }
        let parent_path = path.parent().ok_or("FILE_PARENT_INVALID")?;
        let filename = path.file_name().ok_or("FILE_NAME_INVALID")?;
        let root =
            open_authorized_root(parent_path, roots).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
        let parent =
            open_destination_directory(parent_path).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
        authorize_with_root(&parent, &root).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
        save_if_unchanged_at(&parent, filename, expected, content, limit)
    }

    /// Atomic create/replace for Agent-owned drafts, new files and recovery
    /// backups. Missing parent components are created relative to the nearest
    /// verified existing ancestor, never through an unchecked junction path.
    pub(super) fn atomic_write(path: &Path, bytes: &[u8], limit: usize) -> Result<(), String> {
        use sha2::Digest;
        use std::io::Write;
        use windows_sys::Wdk::Storage::FileSystem::{FILE_CREATE, FILE_OPEN, FILE_OPEN_IF};
        const DELETE: u32 = 0x0001_0000;
        const FILE_READ_DATA: u32 = 0x0000_0001;
        const FILE_WRITE_DATA: u32 = 0x0000_0002;
        const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
        const READ_CONTROL: u32 = 0x0002_0000;
        const WRITE_DAC: u32 = 0x0004_0000;
        if !path.is_absolute() || bytes.len() > limit || limit == 0 {
            return Err("AGENT_WRITE_LIMIT".into());
        }
        reject_reparse_ancestors(path).map_err(|_| "AGENT_SYMLINK_DENIED")?;
        let filename = path.file_name().ok_or("AGENT_PATH_DENIED")?;
        let parent_path = path.parent().ok_or("AGENT_PATH_DENIED")?;
        let parent = open_parent_tree(parent_path).map_err(|_| "AGENT_WRITE_FAILED")?;
        let digest = format!("{:x}", sha2::Sha256::digest(filename.as_encoded_bytes()));
        let lock_name = std::ffi::OsString::from(format!(".browseros-cas-{digest}.lock"));
        let lock = open_child_at(
            &parent,
            &lock_name,
            FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ_WRITE_DELETE,
            FILE_OPEN_IF,
        )
        .map_err(|_| "AGENT_LOCK_UNAVAILABLE")?;
        lock.lock().map_err(|_| "AGENT_LOCK_UNAVAILABLE")?;
        let result = (|| {
            let mut stage = None;
            for _ in 0..8 {
                let candidate = std::ffi::OsString::from(format!(
                    ".browseros-agent-{}.tmp",
                    uuid::Uuid::new_v4()
                ));
                if let Ok(file) = open_child_at(
                    &parent,
                    &candidate,
                    DELETE | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES | READ_CONTROL | WRITE_DAC,
                    0,
                    FILE_CREATE,
                ) {
                    stage = Some(file);
                    break;
                }
            }
            let mut stage = stage.ok_or("AGENT_WRITE_FAILED")?;
            let write_result = (|| {
                stage.write_all(bytes).map_err(|_| "AGENT_WRITE_FAILED")?;
                stage.sync_all().map_err(|_| "AGENT_WRITE_FAILED")?;
                match open_child_at(
                    &parent,
                    filename,
                    FILE_READ_DATA | FILE_READ_ATTRIBUTES | READ_CONTROL,
                    FILE_SHARE_READ_WRITE_DELETE,
                    FILE_OPEN,
                ) {
                    Ok(target) => copy_dacl(&target, &stage)?,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(_) => return Err("AGENT_SYMLINK_DENIED".into()),
                }
                rename_replace_at(&stage, &parent, filename).map_err(|_| "AGENT_WRITE_FAILED")?;
                let _ = parent.sync_all();
                Ok(())
            })();
            if write_result.is_err() {
                delete_on_close(&stage);
            }
            write_result
        })();
        drop(lock);
        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn authorized_windows_read_returns_bytes_and_denies_external_files() {
            let base = std::env::temp_dir()
                .join(format!("browseros-windows-read-{}", uuid::Uuid::new_v4()));
            let safe = base.join("safe");
            let external = base.join("external");
            std::fs::create_dir_all(&safe).unwrap();
            std::fs::create_dir_all(&external).unwrap();
            let file = safe.join("one.txt");
            let outside = external.join("two.txt");
            std::fs::write(&file,b"inside").unwrap();
            std::fs::write(&outside,b"external").unwrap();
            assert_eq!(read_regular(&file, &[safe.clone()], 128).unwrap(), b"inside");
            assert!(read_regular(&outside, &[safe.clone()], 128).is_err());
            assert!(read_regular(&file, &[safe.clone()], 2).is_err());
            std::fs::remove_dir_all(base).unwrap();
        }
        #[test]
        fn pinned_handle_move_is_no_replace_and_does_not_touch_external_root() {
            let base = std::env::temp_dir()
                .join(format!("browseros-winmove-{}", uuid::Uuid::new_v4()));
            let allowed = base.join("allowed");
            let a = allowed.join("a");
            let b = allowed.join("b");
            let outside = base.join("outside");
            std::fs::create_dir_all(&a).unwrap();
            std::fs::create_dir_all(&b).unwrap();
            std::fs::create_dir_all(&outside).unwrap();
            let src = a.join("one.txt");
            std::fs::write(&src, b"first").unwrap();
            move_no_replace(&src, &b, &[allowed.clone()]).unwrap();
            assert!(!src.exists());
            assert_eq!(std::fs::read(b.join("one.txt")).unwrap(), b"first");
            std::fs::write(&src, b"second").unwrap();
            assert!(move_no_replace(&src, &b, &[allowed.clone()]).is_err());
            assert_eq!(std::fs::read(b.join("one.txt")).unwrap(), b"first");
            assert!(move_no_replace(&src, &outside, &[allowed.clone()]).is_err());
            std::fs::remove_dir_all(base).unwrap();
        }

        #[test]
        fn windows_cas_replaces_unchanged_file_and_preserves_external_edit() {
            let root =
                std::env::temp_dir().join(format!("browseros-wincas-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            let target = root.join("entry.txt");
            std::fs::write(&target, b"before").unwrap();
            save_if_unchanged(&target, &[root.clone()], b"before", b"saved", 1024).unwrap();
            assert_eq!(std::fs::read(&target).unwrap(), b"saved");
            std::fs::write(&target, b"external").unwrap();
            assert_eq!(
                save_if_unchanged(&target, &[root.clone()], b"saved", b"stale", 1024),
                Err("FILE_CONFLICT".into())
            );
            assert_eq!(std::fs::read(&target).unwrap(), b"external");
            assert_eq!(
                save_if_unchanged(
                    &root.join("missing.txt"),
                    &[root.clone()],
                    b"",
                    b"new",
                    1024
                ),
                Err("FILE_TARGET_MISSING".into())
            );
            assert!(
                !std::fs::read_dir(&root)
                    .unwrap()
                    .flatten()
                    .any(|entry| { entry.file_name().to_string_lossy().ends_with(".tmp") })
            );
            std::fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn windows_cas_pinned_parent_survives_junction_replacement() {
            let base = std::env::temp_dir()
                .join(format!("browseros-wincas-pivot-{}", uuid::Uuid::new_v4()));
            let trusted = base.join("trusted");
            let parent = trusted.join("workspace");
            let outside = base.join("outside");
            std::fs::create_dir_all(&parent).unwrap();
            std::fs::create_dir_all(&outside).unwrap();
            let target = parent.join("entry.txt");
            let external_target = outside.join("entry.txt");
            std::fs::write(&target, b"before").unwrap();
            std::fs::write(&external_target, b"outside").unwrap();
            let roots = [trusted.clone()];
            let root = open_authorized_root(&parent, &roots).unwrap();
            let pinned = open_destination_directory(&parent).unwrap();
            authorize_with_root(&pinned, &root).unwrap();
            let original = trusted.join("workspace-original");
            std::fs::rename(&parent, &original).unwrap();
            let junction = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&parent)
                .arg(&outside)
                .output()
                .unwrap();
            assert!(
                junction.status.success(),
                "test junction creation must work"
            );

            save_if_unchanged_at(
                &pinned,
                std::ffi::OsStr::new("entry.txt"),
                b"before",
                b"saved",
                1024,
            )
                .unwrap();
            assert_eq!(std::fs::read(original.join("entry.txt")).unwrap(), b"saved");
            assert_eq!(std::fs::read(&external_target).unwrap(), b"outside");
            std::fs::remove_dir(&parent).unwrap();
            std::fs::remove_dir_all(base).unwrap();
        }

        #[test]
        fn windows_agent_atomic_write_creates_nested_files_without_touching_junction_target() {
            let base = std::env::temp_dir()
                .join(format!("browseros-winagent-write-{}", uuid::Uuid::new_v4()));
            let root = base.join("workspace");
            let outside = base.join("outside");
            std::fs::create_dir_all(&root).unwrap();
            std::fs::create_dir_all(&outside).unwrap();
            let nested = root.join("drafts").join("entry.txt");
            atomic_write(&nested, b"approved", 1024).unwrap();
            assert_eq!(std::fs::read(&nested).unwrap(), b"approved");
            atomic_write(&nested, b"replacement", 1024).unwrap();
            assert_eq!(std::fs::read(&nested).unwrap(), b"replacement");

            let moved = root.join("drafts-original");
            std::fs::rename(root.join("drafts"), &moved).unwrap();
            let junction = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(root.join("drafts"))
                .arg(&outside)
                .output()
                .unwrap();
            assert!(junction.status.success());
            assert!(atomic_write(&nested, b"must-not-escape", 1024).is_err());
            assert!(!outside.join("entry.txt").exists());
            std::fs::remove_dir(root.join("drafts")).unwrap();
            std::fs::remove_dir_all(base).unwrap();
        }
    }
}

pub fn move_no_replace(src: &Path, dest_dir: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux::move_no_replace(src, dest_dir, roots)
    }
    #[cfg(target_os = "windows")]
    {
        windows::move_no_replace(src, dest_dir, roots)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (src, dest_dir, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure move requires a supported platform",
        ))
    }
}

/// Secure a private app-owned directory on Windows. Other platforms keep
/// their existing directory ownership/mode policy.
pub fn ensure_private_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::ensure_private_directory(path)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::fs::create_dir_all(path)
    }
}

pub fn read_regular(canonical: &Path, roots: &[PathBuf], limit: usize) -> std::io::Result<Vec<u8>> {
    #[cfg(target_os = "linux")]
    {
        linux::read_regular(canonical, roots, limit)
    }
    #[cfg(target_os = "windows")]
    {
        windows::read_regular(canonical, roots, limit)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (canonical, roots, limit);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "safe read requires a supported platform",
        ))
    }
}

#[cfg(target_os = "windows")]
pub fn atomic_write(path: &Path, bytes: &[u8], limit: usize) -> Result<(), String> {
    windows::atomic_write(path, bytes, limit)
}

#[cfg(target_os = "windows")]
pub(crate) fn remove_regular(canonical: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    windows::remove_regular(canonical, roots)
}

pub fn save_if_unchanged(
    canonical: &Path,
    roots: &[PathBuf],
    expected: &[u8],
    content: &[u8],
    limit: usize,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        linux::save_if_unchanged(canonical, roots, expected, content, limit)
    }
    #[cfg(target_os = "windows")]
    {
        windows::save_if_unchanged(canonical, roots, expected, content, limit)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (canonical, roots, expected, content, limit);
        Err("FILE_DESCRIPTOR_CAS_UNSUPPORTED".into())
    }
}
