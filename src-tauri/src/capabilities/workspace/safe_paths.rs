//! Root-anchored filesystem operations. No path-based I/O after authorization.
//! For Linux, each directory component is opened without following symlinks;
//! rename uses those pinned directory descriptors. If unavailable, fail closed.
use std::path::{Path, PathBuf};

/// A bounded snapshot of one native file object. Identity is opaque and only
/// meaningful together with its authorized root and exact content. This is not
/// a durable lock against arbitrary external writers.
#[derive(Clone, Debug)]
pub struct FileSnapshot {
    pub identity: String,
    pub bytes: Vec<u8>,
}

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
            || new_name == OsStr::new(".")
            || new_name == OsStr::new("..")
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
                dir.as_raw_fd(),
                dot.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let raw = unsafe { libc::fdopendir(fd) };
        if raw.is_null() {
            let error = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(error);
        }
        struct Directory(*mut libc::DIR);
        impl Drop for Directory {
            fn drop(&mut self) {
                unsafe {
                    libc::closedir(self.0);
                }
            }
        }
        let directory = Directory(raw);
        let mut names = Vec::new();
        loop {
            unsafe {
                *libc::__errno_location() = 0;
            }
            let entry = unsafe { libc::readdir(directory.0) };
            if entry.is_null() {
                let errno = unsafe { *libc::__errno_location() };
                if errno != 0 {
                    return Err(io::Error::from_raw_os_error(errno));
                }
                break;
            }
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name == b"." || name == b".." {
                continue;
            }
            if names.len() >= 50_000 {
                return Err(bad_path());
            }
            names.push(std::ffi::OsString::from_vec(name.to_vec()));
        }
        Ok(names)
    }

    /// Directory entry names and sizes must be read relative to a pinned
    /// authorized dirfd. AT_SYMLINK_NOFOLLOW prevents external size leaks.
    pub(super) fn list_directory(
        path: &Path,
        roots: &[PathBuf],
    ) -> io::Result<Vec<(std::ffi::OsString, bool, u64)>> {
        let directory = open_allowed_dir(path, roots)?;
        let mut entries = Vec::new();
        for filename in directory_names(&directory)? {
            let child = name(&filename)?;
            let mut stat: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe {
                libc::fstatat(
                    directory.as_raw_fd(),
                    child.as_ptr(),
                    &mut stat,
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } != 0
            {
                continue;
            }
            let kind = stat.st_mode & libc::S_IFMT;
            let size = if kind == libc::S_IFREG && stat.st_size > 0 {
                stat.st_size as u64
            } else {
                0
            };
            entries.push((filename, kind == libc::S_IFDIR, size));
        }
        Ok(entries)
    }

    // Recheck the pinned inode before name-based unlinkat; this narrows a
    // concurrent rename race, but it cannot make unlinkat itself atomic.
    fn entry_still_matches(parent: &File, basename: &OsStr, pinned: &File) -> io::Result<()> {
        use std::os::unix::fs::MetadataExt;
        let name = name(basename)?;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe {
            libc::fstatat(
                parent.as_raw_fd(),
                name.as_ptr(),
                &mut stat,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let opened = pinned.metadata()?;
        if stat.st_dev != opened.dev() || stat.st_ino != opened.ino() {
            return Err(bad_path());
        }
        Ok(())
    }

    fn remove_directory_contents(
        dir: &File,
        depth: usize,
        remaining: &mut usize,
    ) -> io::Result<()> {
        if depth > 64 {
            return Err(bad_path());
        }
        for child_name in directory_names(dir)? {
            if *remaining == 0 {
                return Err(bad_path());
            }
            *remaining -= 1;
            let child = name(&child_name)?;
            // fstatat with NOFOLLOW is a hint for the expected object kind;
            // the subsequent openat pins the object and rejects a swap to
            // a symlink. All mutations remain relative to the pinned dir FD.
            let mut stat: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe {
                libc::fstatat(
                    dir.as_raw_fd(),
                    child.as_ptr(),
                    &mut stat,
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } != 0
            {
                return Err(io::Error::last_os_error());
            }
            match stat.st_mode & libc::S_IFMT {
                libc::S_IFDIR => {
                    let nested = open_child_dir(dir, &child_name)?;
                    remove_directory_contents(&nested, depth + 1, remaining)?;
                    entry_still_matches(dir, &child_name, &nested)?;
                    drop(nested);
                    if unsafe {
                        libc::unlinkat(dir.as_raw_fd(), child.as_ptr(), libc::AT_REMOVEDIR)
                    } != 0
                    {
                        return Err(io::Error::last_os_error());
                    }
                }
                libc::S_IFREG => {
                    let regular = open_regular_at(dir, &child_name)?;
                    entry_still_matches(dir, &child_name, &regular)?;
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
        if !path.is_absolute() {
            return Err(bad_path());
        }
        let parent_path = path.parent().ok_or_else(bad_path)?;
        let basename = path.file_name().ok_or_else(bad_path)?;
        let parent = open_allowed_dir(parent_path, roots)?;
        let directory = open_child_dir(&parent, basename)?;
        let mut remaining = 50_000usize;
        remove_directory_contents(&directory, 0, &mut remaining)?;
        entry_still_matches(&parent, basename, &directory)?;
        drop(directory);
        let child = name(basename)?;
        if unsafe { libc::unlinkat(parent.as_raw_fd(), child.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
            return Err(io::Error::last_os_error());
        }
        sync_directory(&parent)
    }

    pub(super) fn remove_regular(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
        if !path.is_absolute() {
            return Err(bad_path());
        }
        let parent_path = path.parent().ok_or_else(bad_path)?;
        let filename = path.file_name().ok_or_else(bad_path)?;
        let parent = open_allowed_dir(parent_path, roots)?;
        // Never unlink a directory, special file or symlink. The parent is
        // opened by dirfd and refuses symlinks at every ancestor.
        let file = open_regular_at(&parent, filename)?;
        if !file.metadata()?.is_file() {
            return Err(bad_path());
        }
        entry_still_matches(&parent, filename, &file)?;
        let filename = name(filename)?;
        if unsafe { libc::unlinkat(parent.as_raw_fd(), filename.as_ptr(), 0) } != 0 {
            return Err(io::Error::last_os_error());
        }
        sync_directory(&parent)
    }

    pub(super) fn create_new_file(
        path: &Path,
        roots: &[PathBuf],
        content: &[u8],
    ) -> io::Result<()> {
        if !path.is_absolute() {
            return Err(bad_path());
        }
        let parent_path = path.parent().ok_or_else(bad_path)?;
        let filename = path.file_name().ok_or_else(bad_path)?;
        let parent = open_allowed_dir(parent_path, roots)?;
        let temp =
            std::ffi::OsString::from(format!(".browseros-create-{}.tmp", uuid::Uuid::new_v4()));
        let mut staged = create_temp_at(&parent, &temp)?;
        let mut published = false;
        let result = (|| {
            staged.write_all(content)?;
            staged.sync_all()?;
            entry_still_matches(&parent, &temp, &staged)?;
            rename_bound(&parent, &temp, &parent, filename)?;
            published = true;
            sync_directory(&parent)
        })();
        if result.is_err() && !published && entry_still_matches(&parent, &temp, &staged).is_ok() {
            unlink_at(&parent, &temp);
        }
        result
    }

    pub(super) fn create_new_directory(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
        if !path.is_absolute() {
            return Err(bad_path());
        }
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
        if !path.is_absolute() {
            return Err(bad_path());
        }
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
                if error.kind() != io::ErrorKind::AlreadyExists {
                    return Err(error);
                }
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

    fn snapshot_at(
        parent: &File,
        filename: &OsStr,
        limit: usize,
    ) -> Result<Option<(std::fs::Metadata, Vec<u8>)>, String> {
        let mut file = match open_regular_at(parent, filename) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("FILE_TARGET_NOT_REGULAR".into()),
        };
        let before = file.metadata().map_err(|_| "FILE_READ_FAILED")?;
        if before.len() > limit as u64 {
            return Err("FILE_SAVE_TOO_LARGE".into());
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "FILE_READ_FAILED")?;
        if bytes.len() > limit {
            return Err("FILE_SAVE_TOO_LARGE".into());
        }
        let after = file.metadata().map_err(|_| "FILE_READ_FAILED")?;
        if !same_file_identity(&before, &after) {
            return Err("FILE_CONFLICT".into());
        }
        entry_still_matches(parent, filename, &file).map_err(|_| "FILE_CONFLICT")?;
        Ok(Some((after, bytes)))
    }

    fn object_identity(metadata: &std::fs::Metadata) -> String {
        use std::os::unix::fs::MetadataExt;
        format!("linux:{:x}:{:x}", metadata.dev(), metadata.ino())
    }

    pub(super) fn directory_identity(path: &Path) -> io::Result<String> {
        let directory = open_abs_dir_no_symlinks(path)?;
        Ok(object_identity(&directory.metadata()?))
    }

    fn cas_lock(parent: &File, filename: &OsStr) -> Result<File, String> {
        let digest = format!("{:x}", sha2::Sha256::digest(filename.as_bytes()));
        let lock_name = std::ffi::OsString::from(format!(".browseros-cas-{digest}.lock"));
        let lock = open_lock_at(parent, &lock_name).map_err(|_| "FILE_LOCK_UNAVAILABLE")?;
        lock.lock().map_err(|_| "FILE_LOCK_UNAVAILABLE")?;
        Ok(lock)
    }

    /// Persist the receipt before taking the commit lock, so owner metadata can
    /// be written without recursively acquiring a target lock. The parent stays
    /// pinned; after locking, recheck the original snapshot before publication.
    fn write_versioned_at(
        parent: &File,
        filename: &OsStr,
        expected: Option<&[u8]>,
        expected_identity: Option<&str>,
        content: &[u8],
        limit: usize,
        prepare: impl FnOnce(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        if limit == 0 || expected.is_some_and(|b| b.len() > limit) || content.len() > limit {
            return Err("FILE_SAVE_TOO_LARGE".into());
        }
        let initial = snapshot_at(parent, filename, limit)?;
        // Preserve the existing editor error for a removed target. Versioned
        // recovery still treats disappearance of an expected object as conflict.
        if initial.is_none() && expected.is_some() && expected_identity.is_none() {
            return Err("FILE_TARGET_MISSING".into());
        }
        if initial.as_ref().map(|(_, bytes)| bytes.as_slice()) != expected
            || expected_identity.is_some_and(|id| {
                initial
                    .as_ref()
                    .is_none_or(|(meta, _)| object_identity(meta) != id)
            })
        {
            return Err("FILE_CONFLICT".into());
        }
        let temp_name =
            std::ffi::OsString::from(format!(".browseros-cas-{}.tmp", uuid::Uuid::new_v4()));
        // Do not clean up a name when O_EXCL creation failed: it may belong
        // to someone else. Retain our actual file descriptor for error cleanup.
        let mut stage = create_temp_at(parent, &temp_name).map_err(|_| "FILE_STAGE_FAILED")?;
        let mut published = false;
        let result = (|| {
            stage.write_all(content).map_err(|_| "FILE_STAGE_FAILED")?;
            if let Some((metadata, _)) = &initial {
                use std::os::unix::fs::PermissionsExt;
                stage
                    .set_permissions(std::fs::Permissions::from_mode(
                        metadata.permissions().mode() & 0o777,
                    ))
                    .map_err(|_| "FILE_STAGE_FAILED")?;
            }
            stage.sync_all().map_err(|_| "FILE_STAGE_FAILED")?;
            sync_directory(parent).map_err(|_| "FILE_DURABILITY_UNCERTAIN")?;
            let staged_identity =
                object_identity(&stage.metadata().map_err(|_| "FILE_STAGE_FAILED")?);
            prepare(&staged_identity)?;
            let _lock = cas_lock(parent, filename)?;
            // Journal persistence and lock acquisition may take time. Recheck
            // after both, so a stale snapshot never bypasses the commit lock.
            let latest = snapshot_at(parent, filename, limit)?;
            match (&initial, &latest) {
                (None, None) => {}
                (Some((before, bytes)), Some((after, current)))
                    if same_file_identity(before, after) && bytes == current => {}
                _ => return Err("FILE_CONFLICT".into()),
            }
            entry_still_matches(parent, &temp_name, &stage).map_err(|_| "FILE_STAGE_CHANGED")?;
            if initial.is_some() {
                replace_at(parent, &temp_name, filename).map_err(|_| "FILE_REPLACE_FAILED")?;
            } else {
                // Atomic no-replace; a newly appearing file is never overwritten.
                rename_bound(parent, &temp_name, parent, filename).map_err(|e| {
                    if e.kind() == io::ErrorKind::AlreadyExists {
                        "FILE_CONFLICT"
                    } else {
                        "FILE_REPLACE_FAILED"
                    }
                })?;
            }
            published = true;
            sync_directory(parent).map_err(|_| "FILE_DURABILITY_UNCERTAIN")?;
            Ok(())
        })();
        if result.is_err() && !published && entry_still_matches(parent, &temp_name, &stage).is_ok() {
            // A replaced stage name is not our object. Leave it untouched.
            // As with the existing unlinkat boundary, this identity recheck
            // narrows but cannot eliminate arbitrary namespace races.
            unlink_at(parent, &temp_name);
        }
        result
    }

    fn save_if_unchanged_at(
        parent: &File,
        filename: &OsStr,
        expected: &[u8],
        content: &[u8],
        limit: usize,
    ) -> Result<(), String> {
        write_versioned_at(
            parent,
            filename,
            Some(expected),
            None,
            content,
            limit,
            |_| Ok(()),
        )
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

    pub(super) fn read_versioned(
        path: &Path,
        roots: &[PathBuf],
        limit: usize,
    ) -> Result<Option<FileSnapshot>, String> {
        let parent_path = path.parent().ok_or("FILE_PARENT_INVALID")?;
        let filename = path.file_name().ok_or("FILE_NAME_INVALID")?;
        let parent = match open_allowed_dir(parent_path, roots) {
            Ok(parent) => parent,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("FILE_PARENT_NOT_AUTHORIZED".into()),
        };
        Ok(
            snapshot_at(&parent, filename, limit)?.map(|(meta, bytes)| FileSnapshot {
                identity: object_identity(&meta),
                bytes,
            }),
        )
    }

    pub(super) fn write_versioned_approved(
        path: &Path, roots: &[PathBuf], ancestor: &Path, ancestor_identity: &str,
        expected: Option<&FileSnapshot>, content: &[u8], limit: usize,
        prepare: impl FnOnce(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        let relative = path.strip_prefix(ancestor).map_err(|_| "FILE_APPROVED_ANCESTOR_CHANGED")?;
        if relative.components().any(|part| !matches!(part, Component::Normal(_))) {
            return Err("FILE_APPROVED_ANCESTOR_CHANGED".into());
        }
        let filename = relative.file_name().ok_or("FILE_NAME_INVALID")?;
        let mut parent = open_allowed_dir(ancestor, roots).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
        if object_identity(&parent.metadata().map_err(|_| "FILE_IDENTITY_UNAVAILABLE")?) != ancestor_identity {
            return Err("FILE_APPROVED_ANCESTOR_CHANGED".into());
        }
        // The approved native directory stays pinned. Resolve each remaining
        // segment relative to the current handle, never reopen an absolute path.
        for part in relative.parent().ok_or("FILE_PARENT_INVALID")?.components() {
            let Component::Normal(segment) = part else { return Err("FILE_PARENT_INVALID".into()); };
            parent = match open_child_dir(&parent, segment) {
                Ok(next) => next,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    let native = name(segment).map_err(|_| "FILE_PARENT_INVALID")?;
                    if unsafe { libc::mkdirat(parent.as_raw_fd(), native.as_ptr(), 0o700) } != 0
                        && io::Error::last_os_error().kind() != io::ErrorKind::AlreadyExists {
                        return Err("FILE_PARENT_CREATE_FAILED".into());
                    }
                    sync_directory(&parent).map_err(|_| "FILE_DURABILITY_UNCERTAIN")?;
                    open_child_dir(&parent, segment).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?
                }
                Err(_) => return Err("FILE_PARENT_NOT_AUTHORIZED".into()),
            };
        }
        write_versioned_at(&parent, filename, expected.map(|s| s.bytes.as_slice()),
            expected.map(|s| s.identity.as_str()), content, limit, prepare)
    }

    pub(super) fn write_versioned(
        path: &Path,
        roots: &[PathBuf],
        expected: Option<&FileSnapshot>,
        content: &[u8],
        limit: usize,
        prepare: impl FnOnce(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        let parent_path = path.parent().ok_or("FILE_PARENT_INVALID")?;
        let filename = path.file_name().ok_or("FILE_NAME_INVALID")?;
        let parent =
            open_allowed_dir(parent_path, roots).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
        write_versioned_at(
            &parent,
            filename,
            expected.map(|s| s.bytes.as_slice()),
            expected.map(|s| s.identity.as_str()),
            content,
            limit,
            prepare,
        )
    }

    pub(super) fn remove_if_unchanged(
        path: &Path,
        roots: &[PathBuf],
        expected: &FileSnapshot,
        limit: usize,
    ) -> Result<(), String> {
        let parent_path = path.parent().ok_or("FILE_PARENT_INVALID")?;
        let filename = path.file_name().ok_or("FILE_NAME_INVALID")?;
        let parent =
            open_allowed_dir(parent_path, roots).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
        let _lock = cas_lock(&parent, filename)?;
        let (metadata, bytes) = snapshot_at(&parent, filename, limit)?.ok_or("FILE_CONFLICT")?;
        if object_identity(&metadata) != expected.identity || bytes != expected.bytes {
            return Err("FILE_CONFLICT".into());
        }
        let name = name(filename).map_err(|_| "FILE_NAME_INVALID")?;
        if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err("FILE_DELETE_FAILED".into());
        }
        sync_directory(&parent).map_err(|_| "FILE_DURABILITY_UNCERTAIN".into())
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
        fn pinned_file_identity_recheck_detects_replaced_name() {
            let base = fixture();
            let trusted = base.join("trusted");
            std::fs::create_dir_all(&trusted).unwrap();
            let roots = [trusted.canonicalize().unwrap()];
            let original = trusted.join("original.txt");
            let moved = trusted.join("moved.txt");
            std::fs::write(&original, b"first").unwrap();
            let parent = open_allowed_dir(&trusted, &roots).unwrap();
            let pinned = open_regular_at(&parent, OsStr::new("original.txt")).unwrap();
            std::fs::rename(&original, &moved).unwrap();
            std::fs::write(&original, b"second").unwrap();
            assert!(entry_still_matches(&parent, OsStr::new("original.txt"), &pinned).is_err());
            drop(pinned);
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

#[cfg(unix)]
#[path = "private_directory_unix.rs"]
mod private_directory_unix;

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
    {
        linux::rename_no_replace(src, dest_dir, new_name, roots)
    }
    #[cfg(target_os = "windows")]
    {
        windows::rename_no_replace(src, dest_dir, new_name, roots)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (src, dest_dir, new_name, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure rename requires Linux/Windows",
        ))
    }
}

/// Create a new regular file atomically without overwriting an existing
/// entry. The destination parent must already exist and be an authorized root
/// descendant; callers must create missing directories as separate actions.
pub fn create_new_file(path: &Path, roots: &[PathBuf], bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux::create_new_file(path, roots, bytes)
    }
    #[cfg(target_os = "windows")]
    {
        windows::create_new_file(path, roots, bytes)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots, bytes);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure create unsupported",
        ))
    }
}

/// Create one directory with a pinned authorized parent; never follow a
/// replaceable path during mkdirat / NtCreateFile.
pub fn create_new_directory(path: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux::create_new_directory(path, roots)
    }
    #[cfg(target_os = "windows")]
    {
        windows::create_new_directory(path, roots)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure mkdir unsupported",
        ))
    }
}

/// Create missing subdirectories one component at a time using anchored
/// directory handles, then atomically create the requested final directory.
pub fn create_directory_all(path: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux::create_directory_all(path, roots)
    }
    #[cfg(target_os = "windows")]
    {
        windows::create_directory_all(path, roots)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure recursive mkdir requires Linux/Windows",
        ))
    }
}

/// Secure the caller-selected private application root using native ownership
/// and access controls. This does not recursively normalize existing contents.
pub fn ensure_private_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows::ensure_private_directory(path)
    }
    #[cfg(unix)]
    {
        private_directory_unix::ensure_private_directory(path)
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "PRIVATE_DIRECTORY_PLATFORM_UNSUPPORTED",
        ))
    }
}

/// Read directory entries from an authorized OS directory handle.
pub fn list_directory(
    path: &Path,
    roots: &[PathBuf],
) -> std::io::Result<Vec<(std::ffi::OsString, bool, u64)>> {
    #[cfg(target_os = "linux")]
    {
        linux::list_directory(path, roots)
    }
    #[cfg(target_os = "windows")]
    {
        windows::list_directory(path, roots)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure directory listing unsupported",
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
    {
        linux::remove_regular(canonical, roots)
    }
    #[cfg(target_os = "windows")]
    {
        windows::remove_regular(canonical, roots)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (canonical, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure delete unsupported",
        ))
    }
}

/// Remove a directory tree using pinned directory handles. No symlinks,
/// reparse points, UNC roots or special files may be traversed; both native
/// implementations cap depth and entry count to limit adversarial recursion.
pub fn remove_tree(path: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux::remove_tree(path, roots)
    }
    #[cfg(target_os = "windows")]
    {
        windows::remove_tree(path, roots)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "secure recursive delete unsupported",
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

/// A read-only object identity for duplicate/storage preflight. This does not
/// authorize later access: callers still use the pinned-root write primitives.
pub fn directory_identity(path: &Path) -> std::io::Result<String> {
    #[cfg(target_os = "linux")]
    {
        linux::directory_identity(path)
    }
    #[cfg(target_os = "windows")]
    {
        windows::directory_identity(path)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "native directory identity unavailable",
        ))
    }
}

/// Read an object-bound snapshot; a missing target is distinct from a bad type,
/// link, authorization failure, or a concurrently changing file.
pub fn read_versioned(
    path: &Path,
    roots: &[PathBuf],
    limit: usize,
) -> Result<Option<FileSnapshot>, String> {
    #[cfg(target_os = "linux")]
    {
        linux::read_versioned(path, roots, limit)
    }
    #[cfg(target_os = "windows")]
    {
        windows::read_versioned(path, roots, limit)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots, limit);
        Err("FILE_DESCRIPTOR_CAS_UNSUPPORTED".into())
    }
}

/// Uses the same native parent and sidecar lock as editor CAS. The owner must
/// persist the staged object's receipt in prepare BEFORE publication; if that
/// fails, no target is changed. None means atomic create without replacement.
/// Non-cooperating writers can still race the final replacement on Linux and
/// Windows, or Linux unlink; this is not a kernel-wide transaction or hostile
/// same-SID isolation. Windows serializes cooperating writes per native parent.
/// prepare runs before acquiring the commit lock; the snapshot is rechecked
/// afterwards while holding that lock.
/// Bind the commit to the native ancestor approved by the user, not a later
/// path lookup. Shared external files still do not provide arbitrary-writer CAS.
pub fn write_versioned_approved(
    path: &Path, roots: &[PathBuf], ancestor: &Path, ancestor_identity: &str,
    expected: Option<&FileSnapshot>, content: &[u8], limit: usize,
    prepare: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    { linux::write_versioned_approved(path, roots, ancestor, ancestor_identity, expected, content, limit, prepare) }
    #[cfg(target_os = "windows")]
    { windows::write_versioned_approved(path, roots, ancestor, ancestor_identity, expected, content, limit, prepare) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    { let _ = (path, roots, ancestor, ancestor_identity, expected, content, limit, prepare);
      Err("FILE_DESCRIPTOR_CAS_UNSUPPORTED".into()) }
}

pub fn write_versioned(
    path: &Path,
    roots: &[PathBuf],
    expected: Option<&FileSnapshot>,
    content: &[u8],
    limit: usize,
    prepare: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        linux::write_versioned(path, roots, expected, content, limit, prepare)
    }
    #[cfg(target_os = "windows")]
    {
        windows::write_versioned(path, roots, expected, content, limit, prepare)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots, expected, content, limit, prepare);
        Err("FILE_DESCRIPTOR_CAS_UNSUPPORTED".into())
    }
}

pub fn remove_if_unchanged(
    path: &Path,
    roots: &[PathBuf],
    expected: &FileSnapshot,
    limit: usize,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        linux::remove_if_unchanged(path, roots, expected, limit)
    }
    #[cfg(target_os = "windows")]
    {
        windows::remove_if_unchanged(path, roots, expected, limit)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (path, roots, expected, limit);
        Err("FILE_DESCRIPTOR_CAS_UNSUPPORTED".into())
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod cross_platform_file_manager_tests {
    use super::*;

    #[test]
    fn approved_directory_replacement_is_not_authorized() {
        let root = std::env::temp_dir().join(format!("browseros-approval-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("approved")).unwrap();
        let root = root.canonicalize().unwrap();
        let approved = root.join("approved");
        let identity = directory_identity(&approved).unwrap();
        std::fs::rename(&approved, root.join("previous")).unwrap();
        std::fs::create_dir(&approved).unwrap();
        assert_eq!(write_versioned_approved(&approved.join("new/file.txt"), &[root.clone()],
            &approved, &identity, None, b"reviewed", 1024, |_| Ok(())).unwrap_err(),
            "FILE_APPROVED_ANCESTOR_CHANGED");
        assert!(!approved.join("new").exists());
        assert!(!root.join("previous/new").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_directory_listing_denies_external_paths() {
        let base =
            std::env::temp_dir().join(format!("browseros-native-list-{}", uuid::Uuid::new_v4()));
        let allowed_dir = base.join("allowed");
        let outside = base.join("outside");
        std::fs::create_dir_all(&allowed_dir).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(allowed_dir.join("hello.txt"), b"1234").unwrap();
        let roots = [allowed_dir.canonicalize().unwrap()];
        let entries = list_directory(&roots[0], &roots).unwrap();
        assert!(entries.iter().any(|(name, is_dir, size)| name
            == std::ffi::OsStr::new("hello.txt")
            && !is_dir
            && *size == 4));
        assert!(list_directory(&outside, &roots).is_err());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn native_create_rename_delete_use_pinned_parents_and_no_overwrite() {
        let base =
            std::env::temp_dir().join(format!("browseros-native-fs-{}", uuid::Uuid::new_v4()));
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
            &original,
            &directory,
            std::ffi::OsStr::new("occupied.txt"),
            &roots,
        )
        .is_err());
        assert_eq!(std::fs::read(&occupied).unwrap(), b"occupied");

        rename_no_replace(
            &original,
            &directory,
            std::ffi::OsStr::new("renamed.txt"),
            &roots,
        )
        .unwrap();
        assert!(!original.exists());
        let renamed = directory.join("renamed.txt");
        remove_regular(&renamed, &roots).unwrap();
        assert!(!renamed.exists());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn recursive_delete_stays_beneath_authorized_root_and_refuses_links() {
        let outer =
            std::env::temp_dir().join(format!("browseros-safe-tree-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&outer).unwrap();
        let root = outer.join("workspace");
        let outside = outer.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("must-survive.txt"), b"host-owned").unwrap();
        let roots = [root.canonicalize().unwrap()];
        let tree = roots[0].join("tree");
        create_directory_all(&tree.join("a").join("b"), &roots).unwrap();
        create_new_file(
            &tree.join("a").join("b").join("delete.txt"),
            &roots,
            b"remove",
        )
        .unwrap();
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

/// Resolve a configured Unix data-base alias before publishing the strict
/// application-owned root. Does not relax file-manager path authorization.
#[cfg(not(target_os = "windows"))]
pub fn prepare_application_data_directory(base: &Path) -> std::io::Result<std::path::PathBuf> {
    private_directory_unix::prepare_application_data_directory(base)
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod versioned_write_tests {
    use super::*;
    use std::fs;
    fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!("browseros-receipt-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    #[test]
    fn staged_receipt_precedes_publication_and_a_failed_record_preserves_content() {
        let root = fixture();
        let target = root.join("target.txt");
        fs::write(&target, b"before").unwrap();
        let before = read_versioned(&target, &[root.clone()], 1024)
            .unwrap()
            .unwrap();
        let result = write_versioned(
            &target,
            &[root.clone()],
            Some(&before),
            b"after",
            1024,
            |receipt| {
                assert_ne!(receipt, before.identity);
                assert_eq!(fs::read(&target).unwrap(), b"before");
                Err("JOURNAL_NOT_DURABLE".into())
            },
        );
        assert_eq!(result.unwrap_err(), "JOURNAL_NOT_DURABLE");
        assert_eq!(fs::read(&target).unwrap(), b"before");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publication_rechecks_external_edits_after_persisting_the_receipt() {
        let root = fixture();
        let target = root.join("target.txt");
        fs::write(&target, b"before").unwrap();
        let before = read_versioned(&target, &[root.clone()], 1024)
            .unwrap()
            .unwrap();
        let result = write_versioned(
            &target,
            &[root.clone()],
            Some(&before),
            b"after",
            1024,
            |_| {
                fs::write(&target, b"external").unwrap();
                Ok(())
            },
        );
        assert_eq!(result.unwrap_err(), "FILE_CONFLICT");
        assert_eq!(fs::read(&target).unwrap(), b"external");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn swapped_stage_is_not_deleted_by_failed_publication_cleanup() {
        let root = fixture();
        let target = root.join("target.txt");
        fs::write(&target, b"before").unwrap();
        let before = read_versioned(&target, &[root.clone()], 1024).unwrap().unwrap();
        let mut replaced = None;
        let result = write_versioned(&target, &[root.clone()], Some(&before), b"ours", 1024, |_| {
            let stage = fs::read_dir(&root).unwrap().map(|entry| entry.unwrap().path())
                .find(|path| path.file_name().unwrap().to_string_lossy().starts_with(".browseros-cas-")
                    && path.extension().is_some_and(|ext| ext == "tmp")).unwrap();
            fs::rename(&stage, root.join("held-stage")).unwrap();
            fs::write(&stage, b"external stage replacement").unwrap();
            replaced = Some(stage);
            Ok(())
        });
        assert_eq!(result.unwrap_err(), "FILE_STAGE_CHANGED");
        assert_eq!(fs::read(&target).unwrap(), b"before");
        assert_eq!(fs::read(replaced.unwrap()).unwrap(), b"external stage replacement");
        assert_eq!(fs::read(root.join("held-stage")).unwrap(), b"ours");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn new_file_publication_never_overwrites_a_concurrent_creation() {
        let root = fixture();
        let target = root.join("new.txt");
        let result = write_versioned(&target, &[root.clone()], None, b"ours", 1024, |_| {
            fs::write(&target, b"someone else").unwrap();
            Ok(())
        });
        assert_eq!(result.unwrap_err(), "FILE_CONFLICT");
        assert_eq!(fs::read(&target).unwrap(), b"someone else");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn equal_bytes_in_a_different_file_object_cannot_be_removed_by_old_receipt() {
        let root = fixture();
        let target = root.join("new.txt");
        let mut receipt = None;
        write_versioned(&target, &[root.clone()], None, b"ours", 1024, |id| {
            receipt = Some(id.to_owned());
            Ok(())
        })
        .unwrap();
        let ours = read_versioned(&target, &[root.clone()], 1024)
            .unwrap()
            .unwrap();
        assert_eq!(Some(&ours.identity), receipt.as_ref());
        fs::rename(&target, root.join("held.txt")).unwrap();
        fs::write(&target, b"ours").unwrap();
        assert_eq!(
            remove_if_unchanged(&target, &[root.clone()], &ours, 1024).unwrap_err(),
            "FILE_CONFLICT"
        );
        assert_eq!(fs::read(&target).unwrap(), b"ours");
        fs::remove_dir_all(root).unwrap();
    }
}
