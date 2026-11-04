//! Root-anchored filesystem operations. No path-based I/O after authorization.
//! For Linux, each directory component is opened without following symlinks;
//! rename uses those pinned directory descriptors. If unavailable, fail closed.
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::ffi::{CString, OsStr};
    use std::fs::File;
    use std::io::{self, Read};
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

pub fn move_no_replace(src: &Path, dest_dir: &Path, roots: &[PathBuf]) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux::move_no_replace(src, dest_dir, roots)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (src, dest_dir, roots);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "descriptor-anchored move requires Linux",
        ))
    }
}

pub fn read_regular(canonical: &Path, roots: &[PathBuf], limit: usize) -> std::io::Result<Vec<u8>> {
    #[cfg(target_os = "linux")]
    {
        linux::read_regular(canonical, roots, limit)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (canonical, roots, limit);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "descriptor-anchored reads require Linux",
        ))
    }
}
