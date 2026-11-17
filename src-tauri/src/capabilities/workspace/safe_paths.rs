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
        let filename = src.file_name().ok_or_else(bad_path)?;
        rename_no_replace(src, dest_dir, filename, roots)
    }
    pub(super) fn rename_no_replace(
        src: &Path,
        dest_dir: &Path,
        new_name: &OsStr,
        roots: &[PathBuf],
    ) -> io::Result<()> {
        if Path::new(new_name).components().count() != 1
            || new_name == OsStr::new(".") || new_name == OsStr::new("..")
        {
            return Err(bad_path());
        }
        let parent = src.parent().ok_or_else(bad_path)?;
        let filename = src.file_name().ok_or_else(bad_path)?;
        let source_dir = open_allowed_dir(parent, roots)?;
        let target_dir = open_allowed_dir(dest_dir, roots)?;
        rename_bound(&source_dir, filename, &target_dir, new_name)
    }

    /// Recursively remove only objects reached through pinned, authorized
    /// directory descriptors. Unlike remove_dir_all(path), every descent and
    /// unlink is relative to the directory FD and refuses symlinks/specials.
    fn directory_names(dir: &File) -> io::Result<Vec<std::ffi::OsString>> {
        use std::os::unix::ffi::OsStringExt;
        let dot = CString::new(".").expect("static dot");
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(), dot.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 { return Err(io::Error::last_os_error()); }
        let raw = unsafe { libc::fdopendir(fd) };
        if raw.is_null() {
            let error = io::Error::last_os_error();
            unsafe { libc::close(fd); }
            return Err(error);
        }
        struct Directory(*mut libc::DIR);
        impl Drop for Directory {
            fn drop(&mut self) { unsafe { libc::closedir(self.0); } }
        }
        let directory = Directory(raw);
        let mut names = Vec::new();
        loop {
            unsafe { *libc::__errno_location() = 0; }
            let entry = unsafe { libc::readdir(directory.0) };
            if entry.is_null() {
                let errno = unsafe { *libc::__errno_location() };
                if errno != 0 { return Err(io::Error::from_raw_os_error(errno)); }
                break;
            }
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name == b"." || name == b".." { continue; }
            if names.len() >= 50_000 { return Err(bad_path()); }
            names.push(std::ffi::OsString::from_vec(name.to_vec()));
        }
        Ok(names)
    }

    /// Directory entry names and sizes must be read relative to a pinned
    /// authorized dirfd. AT_SYMLINK_NOFOLLOW prevents external size leaks.
    pub(super) fn list_directory(
        path: &Path, roots: &[PathBuf],
    ) -> io::Result<Vec<(std::ffi::OsString, bool, u64)>> {
        let directory = open_allowed_dir(path, roots)?;
        let mut entries = Vec::new();
        for filename in directory_names(&directory)? {
            let child = name(&filename)?;
            let mut stat: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe {
                libc::fstatat(
                    directory.as_raw_fd(), child.as_ptr(), &mut stat,
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } != 0 {
                continue;
            }
            let kind = stat.st_mode & libc::S_IFMT;
            let size = if kind == libc::S_IFREG && stat.st_size > 0 {
                stat.st_size as u64
            } else { 0 };
            entries.push((filename, kind == libc::S_IFDIR, size));
        }
        Ok(entries)
    }

    fn remove_directory_contents(dir: &File, depth: usize, remaining: &mut usize) -> io::Result<()> {
        if depth > 64 { return Err(bad_path()); }
        for child_name in directory_names(dir)? {
            if *remaining == 0 { return Err(bad_path()); }
            *remaining -= 1;
            let child = name(&child_name)?;
            // fstatat with NOFOLLOW is a hint for the expected object kind;
            // the subsequent openat pins the object and rejects a swap to
            // a symlink. All mutations remain relative to the pinned dir FD.
            let mut stat: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe { libc::fstatat(dir.as_raw_fd(), child.as_ptr(), &mut stat, libc::AT_SYMLINK_NOFOLLOW) } != 0 {
                return Err(io::Error::last_os_error());
            }
            match stat.st_mode & libc::S_IFMT {
                libc::S_IFDIR => {
                    let nested = open_child_dir(dir, &child_name)?;
                    remove_directory_contents(&nested, depth + 1, remaining)?;
                    drop(nested);
                    if unsafe { libc::unlinkat(dir.as_raw_fd(), child.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
                        return Err(io::Error::last_os_error());
                    }
                }
                libc::S_IFREG => {
                    let regular = open_regular_at(dir, &child_name)?;
                    drop(regular);
                    if unsafe { libc::unlinkat(dir.as_raw_fd(), child.as_ptr(), 0) } != 0 {
                        return Err(io::Error::last_os_error());
                    }
                }
                // Do not follow or silently delete symlink/junction-like
                // entries, FIFOs, sockets, devices or other special files.
                _ => return Err(bad_path()),
            }
        }
        sync_directory(dir)
    }

    pub(super) fn remove_tree(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
        if !path.is_absolute() { return Err(bad_path()); }
        let parent_path = path.parent().ok_or_else(bad_path)?;
        let basename = path.file_name().ok_or_else(bad_path)?;
        let parent = open_allowed_dir(parent_path, roots)?;
        let directory = open_child_dir(&parent, basename)?;
        let mut remaining = 50_000usize;
        remove_directory_contents(&directory, 0, &mut remaining)?;
        drop(directory);
        let child = name(basename)?;
        if unsafe { libc::unlinkat(parent.as_raw_fd(), child.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
            return Err(io::Error::last_os_error());
        }
        sync_directory(&parent)
    }

    pub(super) fn remove_regular(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
        if !path.is_absolute() { return Err(bad_path()); }
        let parent_path = path.parent().ok_or_else(bad_path)?;
        let filename = path.file_name().ok_or_else(bad_path)?;
        let parent = open_allowed_dir(parent_path, roots)?;
        // Never unlink a directory, special file or symlink. The parent is
        // opened by dirfd and refuses symlinks at every ancestor.
        let file = open_regular_at(&parent, filename)?;
        if !file.metadata()?.is_file() { return Err(bad_path()); }
        let filename = name(filename)?;
        if unsafe { libc::unlinkat(parent.as_raw_fd(), filename.as_ptr(), 0) } != 0 {
            return Err(io::Error::last_os_error());
        }
        sync_directory(&parent)
    }

    pub(super) fn create_new_file(
        path: &Path, roots: &[PathBuf], content: &[u8],
    ) -> io::Result<()> {
        if !path.is_absolute() { return Err(bad_path()); }
        let parent_path = path.parent().ok_or_else(bad_path)?;
        let filename = path.file_name().ok_or_else(bad_path)?;
        let parent = open_allowed_dir(parent_path, roots)?;
        let temp = std::ffi::OsString::from(format!(
            ".browseros-create-{}.tmp", uuid::Uuid::new_v4()
        ));
        let result = (|| {
            let mut staged = create_temp_at(&parent, &temp)?;
            staged.write_all(content)?;
            staged.sync_all()?;
            drop(staged);
            rename_bound(&parent, &temp, &parent, filename)?;
            sync_directory(&parent)
        })();
        if result.is_err() { unlink_at(&parent, &temp); }
        result
    }

    pub(super) fn create_new_directory(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
        if !path.is_absolute() { return Err(bad_path()); }
        let parent_path = path.parent().ok_or_else(bad_path)?;
        let filename = path.file_name().ok_or_else(bad_path)?;
        let parent = open_allowed_dir(parent_path, roots)?;
        let segment = name(filename)?;
        if filename == OsStr::new(".") || filename == OsStr::new("..") {
            return Err(bad_path());
        }
        if unsafe { libc::mkdirat(parent.as_raw_fd(), segment.as_ptr(), 0o700) } != 0 {
            return Err(io::Error::last_os_error());
        }
        sync_directory(&parent)
    }

    /// mkdir -p inside a verified allowed root, with one pinned directory
    /// descriptor per ancestor. The final directory itself must be new.
    pub(super) fn create_directory_all(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
        if !path.is_absolute() { return Err(bad_path()); }
        let mut cursor = path.parent().ok_or_else(bad_path)?;
        let mut missing = Vec::<std::ffi::OsString>::new();
        while !cursor.exists() {
            missing.push(cursor.file_name().ok_or_else(bad_path)?.to_os_string());
            cursor = cursor.parent().ok_or_else(bad_path)?;
        }
        let mut parent = open_allowed_dir(cursor, roots)?;
        for name_part in missing.iter().rev() {
            if name_part == OsStr::new(".") || name_part == OsStr::new("..") {
                return Err(bad_path());
            }
            let part = name(name_part)?;
            if unsafe { libc::mkdirat(parent.as_raw_fd(), part.as_ptr(), 0o700) } != 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::AlreadyExists { return Err(error); }
            }
            parent = open_child_dir(&parent, name_part)?;
        }
        let target = path.file_name().ok_or_else(bad_path)?;
        let target_name = name(target)?;
        if unsafe { libc::mkdirat(parent.as_raw_fd(), target_name.as_ptr(), 0o700) } != 0 {
            return Err(io::Error::last_os_error());
        }
        sync_directory(&parent)
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
#[path = "safe_paths_windows.rs"]
mod windows;

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

/// Rename without overwriting another file. Both source and destination
/// directories are pinned by the OS before renameat2 / NtSetInformationFile.
pub fn rename_no_replace(
    src: &Path,
    dest_dir: &Path,
    new_name: &std::ffi::OsStr,
    roots: &[PathBuf],
) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    { linux::rename_no_replace(src, dest_dir, new_name, roots) }
    #[cfg(target_os = "windows")]
    { windows::rename_no_replace(src, dest_dir, new_name, roots) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (src, dest_dir, new_name, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported, "secure rename requires Linux/Windows"
        ))
    }
}

/// Create a new regular file atomically without overwriting an existing
/// entry. The destination parent must already exist and be an authorized root
/// descendant; callers must create missing directories as separate actions.
pub fn create_new_file(
    path: &Path, roots: &[PathBuf], bytes: &[u8],
) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    { linux::create_new_file(path, roots, bytes) }
    #[cfg(target_os = "windows")]
    { windows::create_new_file(path, roots, bytes) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots, bytes);
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "secure create unsupported"))
    }
}

/// Create one directory with a pinned authorized parent; never follow a
/// replaceable path during mkdirat / NtCreateFile.
pub fn create_new_directory(path: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    { linux::create_new_directory(path, roots) }
    #[cfg(target_os = "windows")]
    { windows::create_new_directory(path, roots) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots);
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "secure mkdir unsupported"))
    }
}

/// Create missing subdirectories one component at a time using anchored
/// directory handles, then atomically create the requested final directory.
pub fn create_directory_all(path: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    { linux::create_directory_all(path, roots) }
    #[cfg(target_os = "windows")]
    { windows::create_directory_all(path, roots) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure recursive mkdir requires Linux/Windows"
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

/// Read directory entries from an authorized OS directory handle.
pub fn list_directory(
    path: &Path, roots: &[PathBuf],
) -> std::io::Result<Vec<(std::ffi::OsString, bool, u64)>> {
    #[cfg(target_os = "linux")]
    { linux::list_directory(path, roots) }
    #[cfg(target_os = "windows")]
    { windows::list_directory(path, roots) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported, "secure directory listing unsupported"
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

#[cfg(target_os = "windows")]
pub fn atomic_write(path: &Path, bytes: &[u8], limit: usize) -> Result<(), String> {
    windows::atomic_write(path, bytes, limit)
}

/// Remove a regular file without following a symlink or reparse-point
/// target. Both platforms pin the authorized destination parent.
pub fn remove_regular(canonical: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    { linux::remove_regular(canonical, roots) }
    #[cfg(target_os = "windows")]
    { windows::remove_regular(canonical, roots) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (canonical, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported, "secure delete unsupported"
        ))
    }
}

/// Remove a directory tree using pinned directory handles. No symlinks,
/// reparse points, UNC roots or special files may be traversed; both native
/// implementations cap depth and entry count to limit adversarial recursion.
pub fn remove_tree(path: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    { linux::remove_tree(path, roots) }
    #[cfg(target_os = "windows")]
    { windows::remove_tree(path, roots) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported, "secure recursive delete unsupported"
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


#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod cross_platform_file_manager_tests {
    use super::*;

    #[test]
    fn native_directory_listing_denies_external_paths() {
        let base = std::env::temp_dir().join(format!(
            "browseros-native-list-{}", uuid::Uuid::new_v4()
        ));
        let allowed_dir = base.join("allowed");
        let outside = base.join("outside");
        std::fs::create_dir_all(&allowed_dir).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(allowed_dir.join("hello.txt"), b"1234").unwrap();
        let roots = [allowed_dir.canonicalize().unwrap()];
        let entries = list_directory(&roots[0], &roots).unwrap();
        assert!(entries.iter().any(|(name, is_dir, size)|
            name == std::ffi::OsStr::new("hello.txt") && !is_dir && *size == 4
        ));
        assert!(list_directory(&outside, &roots).is_err());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn native_create_rename_delete_use_pinned_parents_and_no_overwrite() {
        let base = std::env::temp_dir().join(format!(
            "browseros-native-fs-{}", uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let roots = [base.canonicalize().unwrap()];
        let directory = roots[0].join("nested").join("child");
        create_directory_all(&directory, &roots).unwrap();
        assert!(directory.is_dir());
        assert!(create_directory_all(&directory, &roots).is_err());

        let original = directory.join("one.txt");
        create_new_file(&original, &roots, b"original").unwrap();
        assert!(create_new_file(&original, &roots, b"overwritten").is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"original");

        let occupied = directory.join("occupied.txt");
        create_new_file(&occupied, &roots, b"occupied").unwrap();
        assert!(rename_no_replace(
            &original, &directory, std::ffi::OsStr::new("occupied.txt"), &roots,
        ).is_err());
        assert_eq!(std::fs::read(&occupied).unwrap(), b"occupied");

        rename_no_replace(
            &original, &directory, std::ffi::OsStr::new("renamed.txt"), &roots
        ).unwrap();
        assert!(!original.exists());
        let renamed = directory.join("renamed.txt");
        remove_regular(&renamed, &roots).unwrap();
        assert!(!renamed.exists());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn recursive_delete_stays_beneath_authorized_root_and_refuses_links() {
        let outer = std::env::temp_dir().join(format!(
            "browseros-safe-tree-{}", uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&outer).unwrap();
        let root = outer.join("workspace");
        let outside = outer.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("must-survive.txt"), b"host-owned").unwrap();
        let roots = [root.canonicalize().unwrap()];
        let tree = roots[0].join("tree");
        create_directory_all(&tree.join("a").join("b"), &roots).unwrap();
        create_new_file(&tree.join("a").join("b").join("delete.txt"), &roots, b"remove").unwrap();
        remove_tree(&tree, &roots).unwrap();
        assert!(!tree.exists());
        assert!(outside.join("must-survive.txt").is_file());
        assert!(remove_tree(&roots[0], &roots).is_err());

        let guarded = roots[0].join("guarded");
        create_directory_all(&guarded, &roots).unwrap();
        #[cfg(target_os = "linux")]
        let linked = std::os::unix::fs::symlink(&outside, guarded.join("escape")).is_ok();
        #[cfg(target_os = "windows")]
        let linked = std::os::windows::fs::symlink_dir(&outside, guarded.join("escape")).is_ok();
        if linked {
            assert!(remove_tree(&guarded, &roots).is_err());
            assert!(outside.join("must-survive.txt").is_file());
        }
        std::fs::remove_dir_all(&outer).unwrap();
    }
}
