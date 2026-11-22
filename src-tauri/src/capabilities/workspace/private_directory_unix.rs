//! Private application-directory boundary for Unix hosts.
//!
//! Only the caller-selected application root is tightened. Existing parents and
//! descendants are not recursively chmod'ed or chown'ed. Paths are walked from
//! an open "/" descriptor, preserving raw OsStr bytes and refusing symlinks.
//! This startup precondition does not replace the descriptor-anchored file
//! operations used for subsequent reads and writes.

use std::io;
use std::path::Path;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod native {
    use super::*;
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::os::unix::ffi::OsStrExt;

    const MAX_COMPONENTS: usize = 128;
    const MAX_PATH_BYTES: usize = 4096;

    fn invalid(message: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, message)
    }

    fn denied(message: &'static str) -> io::Error {
        io::Error::new(io::ErrorKind::PermissionDenied, message)
    }

    fn os_error(stage: &'static str) -> io::Error {
        let error = io::Error::last_os_error();
        io::Error::new(error.kind(), format!("{stage}: {error}"))
    }

    fn components(path: &Path) -> io::Result<Vec<CString>> {
        let bytes = path.as_os_str().as_bytes();
        if !path.is_absolute() || bytes.len() > MAX_PATH_BYTES || bytes.contains(&0) {
            return Err(invalid("PRIVATE_DIRECTORY_PATH_INVALID"));
        }
        let mut names = Vec::new();
        for part in bytes.split(|byte| *byte == b'/').filter(|part| !part.is_empty()) {
            if part == b"." || part == b".." || names.len() == MAX_COMPONENTS {
                return Err(invalid("PRIVATE_DIRECTORY_COMPONENT_INVALID"));
            }
            names.push(CString::new(part).map_err(|_| invalid("PRIVATE_DIRECTORY_NUL"))?);
        }
        // Never chmod the filesystem root, even when the application runs as root.
        if names.is_empty() {
            return Err(invalid("PRIVATE_DIRECTORY_ROOT_REQUIRED"));
        }
        Ok(names)
    }

    fn descriptor(raw: libc::c_int, stage: &'static str) -> io::Result<OwnedFd> {
        if raw < 0 {
            Err(os_error(stage))
        } else {
            // SAFETY: a successful open/openat returned this uniquely owned fd.
            Ok(unsafe { OwnedFd::from_raw_fd(raw) })
        }
    }

    fn open_child(parent: RawFd, name: &CString) -> io::Result<OwnedFd> {
        let raw = unsafe {
            libc::openat(
                parent,
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if raw < 0 {
            // Preserve the native errno for the ENOENT-only mkdir decision.
            return Err(io::Error::last_os_error());
        }
        descriptor(raw, "PRIVATE_DIRECTORY_OPEN")
    }

    fn walk(names: &[CString], create: bool) -> io::Result<OwnedFd> {
        let root = unsafe {
            libc::open(
                b"/\0".as_ptr().cast(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        let mut current = descriptor(root, "PRIVATE_DIRECTORY_ROOT_OPEN")?;
        for name in names {
            let next = match open_child(current.as_raw_fd(), name) {
                Ok(next) => next,
                Err(error) if create && error.raw_os_error() == Some(libc::ENOENT) => {
                    let result = unsafe { libc::mkdirat(current.as_raw_fd(), name.as_ptr(), 0o700) };
                    if result != 0 {
                        let error = io::Error::last_os_error();
                        if error.raw_os_error() != Some(libc::EEXIST) {
                            return Err(error);
                        }
                    }
                    // A concurrent creator never bypasses O_NOFOLLOW or the
                    // final effective-UID check, even when mkdirat saw EEXIST.
                    open_child(current.as_raw_fd(), name)?
                }
                Err(error) => return Err(error),
            };
            current = next;
        }
        Ok(current)
    }

    fn metadata(fd: RawFd) -> io::Result<libc::stat> {
        let mut value = MaybeUninit::<libc::stat>::uninit();
        if unsafe { libc::fstat(fd, value.as_mut_ptr()) } != 0 {
            return Err(os_error("PRIVATE_DIRECTORY_STAT"));
        }
        // SAFETY: successful fstat initialized the complete native stat value.
        let value = unsafe { value.assume_init() };
        if value.st_mode & libc::S_IFMT != libc::S_IFDIR || value.st_nlink == 0 {
            return Err(denied("PRIVATE_DIRECTORY_NOT_LINKED_DIRECTORY"));
        }
        Ok(value)
    }

    fn require_owner(value: &libc::stat, uid: libc::uid_t) -> io::Result<()> {
        if value.st_uid != uid {
            return Err(denied("PRIVATE_DIRECTORY_OWNER_MISMATCH"));
        }
        Ok(())
    }

    fn require_private(value: &libc::stat, uid: libc::uid_t) -> io::Result<()> {
        require_owner(value, uid)?;
        if value.st_mode & 0o7777 != 0o700 {
            return Err(denied("PRIVATE_DIRECTORY_MODE_NOT_0700"));
        }
        Ok(())
    }

    fn same_directory(left: &libc::stat, right: &libc::stat) -> io::Result<()> {
        if left.st_dev != right.st_dev || left.st_ino != right.st_ino {
            return Err(denied("PRIVATE_DIRECTORY_IDENTITY_CHANGED"));
        }
        Ok(())
    }

    fn mode_0700(fd: RawFd) -> io::Result<()> {
        if unsafe { libc::fchmod(fd, 0o700) } != 0 {
            return Err(os_error("PRIVATE_DIRECTORY_CHMOD"));
        }
        Ok(())
    }

    pub(super) fn ensure(path: &Path) -> io::Result<()> {
        let names = components(path)?;
        let directory = walk(&names, true)?;
        let uid = unsafe { libc::geteuid() };
        let original = metadata(directory.as_raw_fd())?;
        // Never take ownership of a directory belonging to another principal.
        require_owner(&original, uid)?;

        // On Linux this also zeros the POSIX ACL group-class mask. Existing
        // parents, descendants and their ACLs remain untouched.
        mode_0700(directory.as_raw_fd())?;

        #[cfg(target_os = "macos")]
        {
            // Darwin extended allow ACEs can grant access beyond the mode bits.
            // The app-private root policy permits no extended ACEs.
            macos_acl::clear(directory.as_raw_fd())?;
            mode_0700(directory.as_raw_fd())?;
            macos_acl::require_empty(directory.as_raw_fd())?;
        }

        let secured = metadata(directory.as_raw_fd())?;
        same_directory(&original, &secured)?;
        require_private(&secured, uid)?;

        // Reopen the absolute namespace without creating or following anything.
        // A moved/replaced ancestor must not make us bless a different leaf.
        let visible = walk(&names, false)?;
        let visible_metadata = metadata(visible.as_raw_fd())?;
        same_directory(&secured, &visible_metadata)?;
        require_private(&visible_metadata, uid)?;
        #[cfg(target_os = "macos")]
        macos_acl::require_empty(visible.as_raw_fd())?;
        Ok(())
    }

    /// Resolve the OS/configuration-selected base once, before appending the
    /// application-owned leaf. This supports an intentional XDG base symlink
    /// and macOS system aliases without allowing symlinks inside our root.
    /// Publish the resolved path, never return to the mutable alias afterwards.
    pub(super) fn prepare_application_data_directory(base: &Path) -> io::Result<std::path::PathBuf> {
        components(base)?;
        let mut existing = base.to_path_buf();
        let mut missing = Vec::new();
        let mut resolved = loop {
            match std::fs::canonicalize(&existing) {
                Ok(path) => break path,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    let name = existing.file_name()
                        .ok_or_else(|| invalid("PRIVATE_DIRECTORY_BASE_UNRESOLVED"))?
                        .to_os_string();
                    missing.push(name);
                    if missing.len() > MAX_COMPONENTS || !existing.pop() {
                        return Err(invalid("PRIVATE_DIRECTORY_BASE_DEPTH"));
                    }
                }
                Err(error) => return Err(io::Error::new(error.kind(), "PRIVATE_DIRECTORY_BASE_UNAVAILABLE")),
            }
        };
        for name in missing.into_iter().rev() { resolved.push(name); }
        resolved.push("mvp-browser-os");
        // The app-owned leaf and every unresolved component still pass the
        // original O_NOFOLLOW/openat + effective UID + 0700/ACL policy. A
        // dangling configured alias is never followed by a create operation.
        ensure(&resolved)?;
        Ok(resolved)
    }

    #[cfg(target_os = "macos")]
    mod macos_acl {
        use super::*;
        use std::ffi::c_void;
        use std::ptr;

        // Native Darwin <sys/acl.h>, not Linux libacl return conventions.
        const ACL_TYPE_EXTENDED: libc::c_int = 0x0000_0100;
        const ACL_FIRST_ENTRY: libc::c_int = 0;
        type Acl = *mut c_void;

        extern "C" {
            fn acl_init(count: libc::c_int) -> Acl;
            fn acl_free(value: *mut c_void) -> libc::c_int;
            fn acl_valid(acl: Acl) -> libc::c_int;
            fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_int) -> Acl;
            fn acl_set_fd_np(fd: libc::c_int, acl: Acl, kind: libc::c_int) -> libc::c_int;
            fn acl_get_entry(acl: Acl, entry_id: libc::c_int, entry: *mut *mut c_void) -> libc::c_int;
        }

        struct OwnedAcl(Acl);
        impl Drop for OwnedAcl {
            fn drop(&mut self) {
                unsafe { acl_free(self.0); }
            }
        }

        pub(super) fn clear(fd: RawFd) -> io::Result<()> {
            let raw = unsafe { acl_init(0) };
            if raw.is_null() {
                return Err(os_error("PRIVATE_DIRECTORY_ACL_ALLOCATE"));
            }
            let acl = OwnedAcl(raw);
            if unsafe { acl_valid(acl.0) } != 0 {
                return Err(os_error("PRIVATE_DIRECTORY_ACL_INVALID"));
            }
            if unsafe { acl_set_fd_np(fd, acl.0, ACL_TYPE_EXTENDED) } != 0 {
                // ENOTSUP/EOPNOTSUPP and permission errors are real failures.
                return Err(os_error("PRIVATE_DIRECTORY_ACL_SET"));
            }
            Ok(())
        }

        pub(super) fn require_empty(fd: RawFd) -> io::Result<()> {
            // A null result without a fresh errno must never be accepted based
            // on an unrelated earlier ENOENT.
            unsafe { *libc::__error() = 0; }
            let raw = unsafe { acl_get_fd_np(fd, ACL_TYPE_EXTENDED) };
            if raw.is_null() {
                let error = io::Error::last_os_error();
                // Darwin filesec_get_property(FILESEC_ACL) uses ENOENT for an
                // absent ACL. The fd itself was opened and fstat'ed above.
                if error.raw_os_error() == Some(libc::ENOENT) {
                    return Ok(());
                }
                return Err(io::Error::new(
                    error.kind(),
                    format!("PRIVATE_DIRECTORY_ACL_READ: {error}"),
                ));
            }
            let acl = OwnedAcl(raw);
            if unsafe { acl_valid(acl.0) } != 0 {
                return Err(os_error("PRIVATE_DIRECTORY_ACL_INVALID"));
            }
            let mut entry = ptr::null_mut();
            let result = unsafe { acl_get_entry(acl.0, ACL_FIRST_ENTRY, &mut entry) };
            if result == 0 {
                return Err(denied("PRIVATE_DIRECTORY_EXTENDED_ACL_REMAINS"));
            }
            // Darwin returns -1/EINVAL when a valid ACL has no first entry.
            // Unlike Linux libacl, 0 means an entry WAS returned, not EOF.
            let error = io::Error::last_os_error();
            if result == -1 && error.raw_os_error() == Some(libc::EINVAL) {
                return Ok(());
            }
            Err(io::Error::new(
                error.kind(),
                format!("PRIVATE_DIRECTORY_ACL_ENTRY: {error}"),
            ))
        }

        #[cfg(test)]
        pub(super) fn install_test_allow_ace(fd: RawFd) -> io::Result<()> {
            extern "C" {
                fn acl_create_entry(acl: *mut Acl, entry: *mut *mut c_void) -> libc::c_int;
                fn acl_set_tag_type(entry: *mut c_void, tag: libc::c_int) -> libc::c_int;
                fn acl_set_qualifier(entry: *mut c_void, qualifier: *const c_void) -> libc::c_int;
                fn acl_get_permset(entry: *mut c_void, permissions: *mut *mut c_void) -> libc::c_int;
                fn acl_add_perm(permissions: *mut c_void, permission: libc::c_int) -> libc::c_int;
            }
            let raw = unsafe { acl_init(1) };
            if raw.is_null() { return Err(os_error("TEST_ACL_ALLOCATE")); }
            let mut acl = OwnedAcl(raw);
            let mut entry = ptr::null_mut();
            let mut permissions = ptr::null_mut();
            let principal = uuid::Uuid::new_v4();
            if unsafe { acl_create_entry(&mut acl.0, &mut entry) } != 0
                || unsafe { acl_set_tag_type(entry, 1) } != 0
                || unsafe { acl_set_qualifier(entry, principal.as_bytes().as_ptr().cast()) } != 0
                || unsafe { acl_get_permset(entry, &mut permissions) } != 0
                || unsafe { acl_add_perm(permissions, 1 << 1) } != 0
                || unsafe { acl_set_fd_np(fd, acl.0, ACL_TYPE_EXTENDED) } != 0
            {
                return Err(os_error("TEST_ACL_INSTALL"));
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs;
        use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
        use std::path::PathBuf;

        struct Fixture(PathBuf);
        impl Fixture {
            fn new() -> Self {
                // Resolve only the trusted test TEMP base (macOS /var may be a
                // system symlink). The general ensure input is never canonicalized.
                let base = std::env::temp_dir().canonicalize().expect("test temp base");
                let path = base.join(format!("browseros-private-dir-{}", uuid::Uuid::new_v4()));
                fs::create_dir(&path).expect("test directory");
                Self(path)
            }
        }
        impl Drop for Fixture {
            fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
        }

        #[test]
        fn tightens_only_application_root_and_preserves_existing_children() {
            let fixture = Fixture::new();
            let parent = fixture.0.join("parent");
            let private = parent.join("app");
            fs::create_dir_all(&private).unwrap();
            let child = private.join("legacy.txt");
            fs::write(&child, b"fixture").unwrap();
            fs::set_permissions(&parent, fs::Permissions::from_mode(0o750)).unwrap();
            fs::set_permissions(&private, fs::Permissions::from_mode(0o755)).unwrap();
            fs::set_permissions(&child, fs::Permissions::from_mode(0o640)).unwrap();

            ensure(&private).unwrap();
            let metadata = fs::metadata(&private).unwrap();
            assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
            assert_eq!(metadata.permissions().mode() & 0o7777, 0o700);
            assert_eq!(fs::metadata(&parent).unwrap().permissions().mode() & 0o7777, 0o750);
            assert_eq!(fs::metadata(&child).unwrap().permissions().mode() & 0o7777, 0o640);
        }

        #[test]
        fn creates_missing_components_and_keeps_private_root_owned() {
            let fixture = Fixture::new();
            let private = fixture.0.join("new-parent").join("app");
            ensure(&private).unwrap();
            let metadata = fs::metadata(&private).unwrap();
            assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
            assert_eq!(metadata.permissions().mode() & 0o7777, 0o700);
        }

        #[test]
        fn rejects_leaf_and_ancestor_symlinks_without_changing_target() {
            let fixture = Fixture::new();
            let target = Fixture::new();
            fs::set_permissions(&target.0, fs::Permissions::from_mode(0o755)).unwrap();
            let link = fixture.0.join("linked");
            symlink(&target.0, &link).unwrap();
            assert!(ensure(&link).is_err());
            assert!(ensure(&link.join("must-not-exist")).is_err());
            assert!(!target.0.join("must-not-exist").exists());
            assert_eq!(fs::metadata(&target.0).unwrap().permissions().mode() & 0o7777, 0o755);
        }

        #[test]
        fn rejects_non_directory_and_lexical_traversal() {
            let fixture = Fixture::new();
            let file = fixture.0.join("file");
            fs::write(&file, b"unchanged").unwrap();
            assert!(ensure(&file).is_err());
            for path in [Path::new("/"), Path::new("relative"), Path::new("/tmp/../app")] {
                assert!(components(path).is_err());
            }
            let too_deep = format!("/{}", vec!["segment"; MAX_COMPONENTS + 1].join("/"));
            assert!(components(Path::new(&too_deep)).is_err());
            assert_eq!(fs::read(&file).unwrap(), b"unchanged");
        }

        #[test]
        fn configured_base_alias_resolves_once_without_relaxing_the_owned_leaf() {
            let fixture = Fixture::new();
            let target = Fixture::new();
            let alias = fixture.0.join("xdg-data");
            symlink(&target.0, &alias).unwrap();
            let resolved = prepare_application_data_directory(&alias.join("browseros")).unwrap();
            assert_eq!(resolved, target.0.join("browseros/mvp-browser-os"));
            fs::remove_file(&alias).unwrap();
            let other = Fixture::new();
            symlink(&other.0, &alias).unwrap();
            assert!(resolved.is_dir());
            assert!(!other.0.join("browseros/mvp-browser-os").exists());
        }

        #[test]
        fn configured_alias_does_not_authorize_a_symlinked_app_leaf() {
            let fixture = Fixture::new();
            let outside = Fixture::new();
            symlink(&outside.0, fixture.0.join("mvp-browser-os")).unwrap();
            assert!(prepare_application_data_directory(&fixture.0).is_err());
            let dangling = fixture.0.join("dangling");
            symlink(fixture.0.join("absent"), &dangling).unwrap();
            assert!(prepare_application_data_directory(&dangling).is_err());
            assert!(!fixture.0.join("absent").exists());
        }

        #[cfg(target_os = "linux")]
        #[test]
        fn preserves_non_utf8_directory_names() {
            use std::ffi::OsString;
            use std::os::unix::ffi::OsStringExt;
            let fixture = Fixture::new();
            let private = fixture.0.join(OsString::from_vec(vec![b'a', 0xff]));
            ensure(&private).unwrap();
            assert_eq!(fs::metadata(private).unwrap().permissions().mode() & 0o7777, 0o700);
        }

        #[cfg(target_os = "macos")]
        #[test]
        fn clears_real_extended_acl_even_when_mode_is_already_0700() {
            let fixture = Fixture::new();
            let private = fixture.0.join("app");
            fs::create_dir(&private).unwrap();
            fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
            let names = components(&private).unwrap();
            let directory = walk(&names, false).unwrap();
            macos_acl::install_test_allow_ace(directory.as_raw_fd()).unwrap();
            assert!(macos_acl::require_empty(directory.as_raw_fd()).is_err());
            ensure(&private).unwrap();
            macos_acl::require_empty(directory.as_raw_fd()).unwrap();
        }
    }
}

pub(super) fn ensure_private_directory(path: &Path) -> io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        native::ensure(path)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "PRIVATE_DIRECTORY_PLATFORM_UNSUPPORTED",
        ))
    }
}

/// Application configuration is the only place where a preselected base alias
/// is resolved. General private-directory callers remain strictly no-symlink.
pub(super) fn prepare_application_data_directory(base: &Path) -> io::Result<std::path::PathBuf> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    { native::prepare_application_data_directory(base) }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = base;
        Err(io::Error::new(io::ErrorKind::Unsupported, "PRIVATE_DIRECTORY_PLATFORM_UNSUPPORTED"))
    }
}
