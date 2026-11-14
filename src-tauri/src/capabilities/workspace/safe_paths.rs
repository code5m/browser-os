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

// Windows read-only authorization: open an actual OS handle first and
// validate its kernel-resolved final path against pinned allowed-root handles.
// Nothing is read until authorization passes; a swapped junction cannot make
// BrowserOS read bytes from an external directory through a stale path check.
// Mutating operations remain unavailable until handle-relative rename/CAS.
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
        io::Error::new(io::ErrorKind::PermissionDenied, "WINDOWS_FILE_OUTSIDE_ALLOWED_ROOT")
    }
    fn open(path: &Path, directory: bool) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.read(true)
            .share_mode(FILE_SHARE_READ_WRITE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT |
                if directory { FILE_FLAG_BACKUP_SEMANTICS } else { 0 });
        let file = options.open(path)?;
        let meta = file.metadata()?;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(denied());
        }
        if directory && !meta.is_dir() || !directory && !meta.is_file() {
            return Err(denied());
        }
        Ok(file)
    }
    fn final_path(file: &File) -> io::Result<String> {
        let handle = file.as_raw_handle() as _;
        let n = unsafe { GetFinalPathNameByHandleW(handle, std::ptr::null_mut(), 0, 0) };
        if n == 0 || n > 32768 {
            return Err(io::Error::last_os_error());
        }
        let mut buf = vec![0u16; n as usize + 1];
        let used = unsafe { GetFinalPathNameByHandleW(handle, buf.as_mut_ptr(), buf.len() as u32, 0) };
        if used == 0 || used as usize >= buf.len() {
            return Err(io::Error::last_os_error());
        }
        // Windows paths are case-insensitive. The trailing path separator
        // enforces a whole-directory boundary rather than a prefix match.
        Ok(String::from_utf16_lossy(&buf[..used as usize]).to_lowercase())
    }
    fn authorize(file: &File, roots: &[PathBuf]) -> io::Result<String> {
        let actual = final_path(file)?;
        for root in roots {
            let Ok(root_handle) = open(root, true) else { continue };
            let Ok(mut root_path) = final_path(&root_handle) else { continue };
            if !root_path.ends_with('\\') { root_path.push('\\'); }
            if actual.starts_with(&root_path) {
                return Ok(actual);
            }
        }
        Err(denied())
    }
    /// No-overwrite rename by existing source/destination OS handles.
    /// FILE_RENAME_INFO RootDirectory pins the actual destination directory,
    /// and ReplaceIfExists=false avoids path-only check-then-rename races.
    pub(super) fn move_no_replace(
        src: &Path, dest_dir: &Path, roots: &[PathBuf]
    ) -> io::Result<()> {
        use std::ffi::c_void;
        use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::{
            SetFileInformationByHandle, FileRenameInfo,
        };
        const DELETE: u32 = 0x0001_0000;
        const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;

        if !src.is_absolute() || !dest_dir.is_absolute() {
            return Err(denied());
        }
        let filename = src.file_name().ok_or_else(denied)?;
        let text = filename.to_string_lossy();
        if text.is_empty() || text.contains(':') || text.ends_with('.') || text.ends_with(' ') {
            // Reject alternate data streams, path aliases and Win32 trailing
            // dot/space normalization before constructing an OS rename.
            return Err(denied());
        }

        let mut source_opts = OpenOptions::new();
        source_opts.access_mode(DELETE | FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ_WRITE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
        let source = source_opts.open(src)?;
        let meta = source.metadata()?;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || !(meta.is_file() || meta.is_dir()) {
            return Err(denied());
        }
        let source_path = authorize(&source, roots)?;
        let destination = open(dest_dir, true)?;
        let destination_path = authorize(&destination, roots)?;
        if destination_path == source_path
            || meta.is_dir() && destination_path.starts_with(&(source_path + "\\")) {
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
                source.as_raw_handle() as _, FileRenameInfo,
                aligned.as_ptr() as *const c_void, size as u32,
            ) == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub(super) fn read_regular(path: &Path, roots: &[PathBuf], limit: usize) -> io::Result<Vec<u8>> {
        if !path.is_absolute() || limit == 0 || limit > 64 * 1024 * 1024 {
            return Err(denied());
        }
        // A final-file reparse point is denied even when it points inside the
        // allowed root; intermediate junctions are checked by final handle.
        let file = open(path, false)?;
        authorize(&file, roots)?;
        let length = file.metadata()?.len();
        if length > limit as u64 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "FILE_READ_SIZE_LIMIT"));
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "FILE_READ_SIZE_LIMIT"));
        }
        Ok(bytes)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn authorized_windows_read_returns_bytes_and_denies_external_files() {
            let base = std::env::temp_dir().join(format!("browseros-windows-read-{}", uuid::Uuid::new_v4()));
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
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (canonical, roots, expected, content, limit);
        Err("FILE_DESCRIPTOR_CAS_UNSUPPORTED".into())
    }
}
