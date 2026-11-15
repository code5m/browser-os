use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub struct SafeWriteError {
    kind: io::ErrorKind,
    code: &'static str,
}

impl SafeWriteError {
    pub fn new(kind: io::ErrorKind, code: &'static str) -> Self { Self { kind, code } }
    pub fn kind(&self) -> io::ErrorKind { self.kind }
    pub fn code(&self) -> &'static str { self.code }
}
impl std::fmt::Display for SafeWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.code) }
}
impl std::error::Error for SafeWriteError {}
impl From<SafeWriteError> for io::Error {
    fn from(value: SafeWriteError) -> Self { io::Error::new(value.kind, value.code) }
}

pub struct SafeWriteOutcome {
    pub saved: bool,
    pub current_hash: Option<String>,
}

fn validate_relative(path: &Path) -> Result<(), SafeWriteError> {
    if path.as_os_str().is_empty() || path.is_absolute() || path.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "PATH_OUTSIDE_WORKSPACE"));
    }
    Ok(())
}

#[cfg(unix)]
fn open_beneath(root: &Path, relative: &Path, create_dirs: bool) -> Result<(std::os::fd::OwnedFd, PathBuf), SafeWriteError> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::FromRawFd;
    validate_relative(relative)?;
    let root = CString::new(root.as_os_str().as_bytes()).map_err(|_| SafeWriteError::new(io::ErrorKind::InvalidInput, "PATH_INVALID"))?;
    let root_fd = unsafe { libc::open(root.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if root_fd < 0 { return Err(SafeWriteError::new(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID")); }
    let mut fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(root_fd) };
    use std::os::fd::AsRawFd;
    let components: Vec<_> = relative.components().filter_map(|c| match c { Component::Normal(v) => Some(v), _ => None }).collect();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        let name = CString::new(component.as_bytes()).map_err(|_| SafeWriteError::new(io::ErrorKind::InvalidInput, "PATH_INVALID"))?;
        let next = unsafe { libc::openat(fd.as_raw_fd(), name.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
        if next < 0 {
            let error = io::Error::last_os_error();
            if create_dirs && error.kind() == io::ErrorKind::NotFound {
                if unsafe { libc::mkdirat(fd.as_raw_fd(), name.as_ptr(), 0o700) } != 0 { return Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "SAFE_DIRECTORY_CREATE_FAILED")); }
                let opened = unsafe { libc::openat(fd.as_raw_fd(), name.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
                if opened < 0 { return Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "SAFE_DIRECTORY_OPEN_FAILED")); }
                fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(opened) };
            } else { return Err(SafeWriteError::new(error.kind(), "SAFE_PATH_ESCAPE")); }
        } else { fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(next) }; }
    }
    let name = components.last().ok_or_else(|| SafeWriteError::new(io::ErrorKind::InvalidInput, "PATH_INVALID"))?;
    let mut name_buf = name.as_bytes().to_vec();
    if name_buf.len() > 255 { return Err(SafeWriteError::new(io::ErrorKind::InvalidInput, "PATH_INVALID")); }
    let mut name = PathBuf::from(relative);
    name.pop();
    Ok((fd, PathBuf::from(components.last().unwrap())))
}

#[cfg(unix)]
fn read_at(root: &Path, relative: &Path, max_bytes: usize) -> Result<Vec<u8>, SafeWriteError> {
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStrExt;
    use std::ffi::CString;
    use std::io::Read;
    let (parent, name) = open_beneath(root, relative, false)?;
    let name = CString::new(name.as_os_str().as_bytes()).map_err(|_| SafeWriteError::new(io::ErrorKind::InvalidInput, "PATH_INVALID"))?;
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK) };
    if fd < 0 { let e = io::Error::last_os_error(); return Err(SafeWriteError::new(e.kind(), if e.kind() == io::ErrorKind::NotFound { "FILE_NOT_FOUND" } else { "SAFE_PATH_ESCAPE" })); }
    let owned = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    if unsafe { libc::fstat(owned.as_raw_fd(), &mut stat) } != 0 { return Err(SafeWriteError::new(io::ErrorKind::Other, "FILE_STAT_FAILED")); }
    if stat.st_mode & libc::S_IFMT != libc::S_IFREG { return Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "NOT_REGULAR_FILE")); }
    if stat.st_size < 0 || stat.st_size as usize > max_bytes { return Err(SafeWriteError::new(io::ErrorKind::InvalidData, "FILE_TOO_LARGE")); }
    let mut file = fs::File::from(owned);
    let mut bytes = Vec::with_capacity(stat.st_size as usize);
    file.take(max_bytes as u64 + 1).read_to_end(&mut bytes).map_err(|_| SafeWriteError::new(io::ErrorKind::Other, "FILE_READ_FAILED"))?;
    if bytes.len() > max_bytes { return Err(SafeWriteError::new(io::ErrorKind::InvalidData, "FILE_TOO_LARGE")); }
    Ok(bytes)
}

#[cfg(unix)]
fn save_at(root: &Path, relative: &Path, content: &[u8], expected: &str, max_bytes: usize) -> Result<SafeWriteOutcome, SafeWriteError> {
    use std::ffi::CString;
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStrExt;
    use std::io::{Read, Write};
    validate_relative(relative)?;
    let current = match read_at(root, relative, max_bytes) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let actual = current.as_deref().map(crate::capabilities::workspace::cas::hash_bytes);
    if actual.as_deref() != Some(expected) { return Ok(SafeWriteOutcome { saved: false, current_hash: actual }); }
    let lock_relative = PathBuf::from(format!(".{}.browseros-cas.lock", relative.file_name().unwrap().to_string_lossy()));
    let (parent, name) = open_beneath(root, relative, false)?;
    let (_, lock_name) = open_beneath(root, &lock_relative, false).or_else(|e| if e.kind() == io::ErrorKind::NotFound { open_beneath(root, &lock_relative, true) } else { Err(e) })?;
    let lock_name = CString::new(lock_name.as_os_str().as_bytes()).map_err(|_| SafeWriteError::new(io::ErrorKind::InvalidInput, "PATH_INVALID"))?;
    let lock_fd = unsafe { libc::openat(parent.as_raw_fd(), lock_name.as_ptr(), libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o600) };
    if lock_fd < 0 { return Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "CAS_LOCK_FAILED")); }
    let lock = unsafe { std::os::fd::OwnedFd::from_raw_fd(lock_fd) };
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 { return Err(SafeWriteError::new(io::ErrorKind::Other, "CAS_LOCK_FAILED")); }
    let current = match read_at(root, relative, max_bytes) { Ok(bytes) => Some(bytes), Err(e) if e.kind() == io::ErrorKind::NotFound => None, Err(e) => return Err(e) };
    let actual = current.as_deref().map(crate::capabilities::workspace::cas::hash_bytes);
    if actual.as_deref() != Some(expected) { return Ok(SafeWriteOutcome { saved: false, current_hash: actual }); }
    let old = current.ok_or_else(|| SafeWriteError::new(io::ErrorKind::NotFound, "FILE_NOT_FOUND"))?;
    let name = CString::new(name.as_os_str().as_bytes()).map_err(|_| SafeWriteError::new(io::ErrorKind::InvalidInput, "PATH_INVALID"))?;
    let old_fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if old_fd < 0 { return Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "SAFE_PATH_ESCAPE")); }
    let old_owned = unsafe { std::os::fd::OwnedFd::from_raw_fd(old_fd) };
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    if unsafe { libc::fstat(old_owned.as_raw_fd(), &mut stat) } != 0 { return Err(SafeWriteError::new(io::ErrorKind::Other, "FILE_STAT_FAILED")); }
    if stat.st_mode & libc::S_IFMT != libc::S_IFREG { return Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "NOT_REGULAR_FILE")); }
    let mut verify = fs::File::from(old_owned);
    let mut bytes = Vec::new();
    verify.read_to_end(&mut bytes).map_err(|_| SafeWriteError::new(io::ErrorKind::Other, "FILE_READ_FAILED"))?;
    if bytes != old { return Ok(SafeWriteOutcome { saved: false, current_hash: Some(crate::capabilities::workspace::cas::hash_bytes(&bytes)) }); }
    let temp_name = format!(".{}.browseros-cas.{}.tmp", relative.file_name().unwrap().to_string_lossy(), uuid::Uuid::new_v4());
    let temp_c = CString::new(temp_name.clone()).map_err(|_| SafeWriteError::new(io::ErrorKind::InvalidInput, "PATH_INVALID"))?;
    let temp_fd = unsafe { libc::openat(parent.as_raw_fd(), temp_c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, stat.st_mode & 0o777) };
    if temp_fd < 0 { return Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "CAS_TEMP_CREATE_FAILED")); }
    let temp_owned = unsafe { std::os::fd::OwnedFd::from_raw_fd(temp_fd) };
    let mut temp = fs::File::from(temp_owned);
    temp.write_all(content).and_then(|_| temp.sync_all()).map_err(|_| SafeWriteError::new(io::ErrorKind::Other, "CAS_TEMP_WRITE_FAILED"))?;
    drop(temp);
    let renamed = unsafe { libc::renameat(parent.as_raw_fd(), temp_c.as_ptr(), parent.as_raw_fd(), name.as_ptr()) };
    if renamed != 0 {
        unsafe { libc::unlinkat(parent.as_raw_fd(), temp_c.as_ptr(), 0); }
        return Err(SafeWriteError::new(io::ErrorKind::Other, "CAS_RENAME_FAILED"));
    }
    unsafe { libc::fsync(parent.as_raw_fd()); }
    Ok(SafeWriteOutcome { saved: true, current_hash: actual })
}

#[cfg(not(any(unix, target_os = "windows")))]
fn save_if_unchanged_path(path: &Path, roots: &[PathBuf], content: &[u8], expected: &str, max_bytes: usize) -> Result<SafeWriteOutcome, SafeWriteError> {
    let (root, relative) = find_root(path, roots)?;
    let _ = (root, relative);
    let current = read_regular(path, roots, max_bytes).ok();
    let actual = current.as_deref().map(crate::capabilities::workspace::cas::hash_bytes);
    if actual.as_deref() != Some(expected) { return Ok(SafeWriteOutcome { saved: false, current_hash: actual }); }
    let old = current.ok_or_else(|| SafeWriteError::new(io::ErrorKind::NotFound, "FILE_NOT_FOUND"))?;
    let parent = path.parent().ok_or_else(|| SafeWriteError::new(io::ErrorKind::PermissionDenied, "SAFE_PATH_ESCAPE"))?;
    let temp = parent.join(format!(".{}.browseros-cas.{}.tmp", path.file_name().unwrap().to_string_lossy(), uuid::Uuid::new_v4()));
    {
        use std::io::Write;
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temp).map_err(|_| SafeWriteError::new(io::ErrorKind::PermissionDenied, "CAS_TEMP_CREATE_FAILED"))?;
        file.write_all(content).and_then(|_| file.sync_all()).map_err(|_| SafeWriteError::new(io::ErrorKind::Other, "CAS_TEMP_WRITE_FAILED"))?;
    }
    let verify = read_regular(path, roots, max_bytes)?;
    if verify != old {
        let _ = fs::remove_file(&temp);
        return Ok(SafeWriteOutcome { saved: false, current_hash: Some(crate::capabilities::workspace::cas::hash_bytes(&verify)) });
    }
    fs::rename(&temp, path).map_err(|_| SafeWriteError::new(io::ErrorKind::Other, "CAS_RENAME_FAILED"))?;
    Ok(SafeWriteOutcome { saved: true, current_hash: actual })
}

#[cfg(not(unix))]
fn find_root(path: &Path, roots: &[PathBuf]) -> Result<(PathBuf, PathBuf), SafeWriteError> {
    let canonical_path = path.canonicalize().map_err(|e| SafeWriteError::new(e.kind(), "PATH_NOT_FOUND"))?;
    for root in roots {
        let root = root.canonicalize().map_err(|_| SafeWriteError::new(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID"))?;
        if let Ok(relative) = canonical_path.strip_prefix(&root) { validate_relative(relative)?; return Ok((root, relative.to_path_buf())); }
    }
    Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "PATH_OUTSIDE_WORKSPACE"))
}

#[cfg(target_os = "windows")]
mod windows {
    use super::{SafeWriteError, SafeWriteOutcome, validate_relative};
    use std::ffi::{CString, c_void};
    use std::fs;
    use std::io;
    use std::mem::{size_of, zeroed};
    use std::path::{Path, PathBuf};
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, HANDLE,
        INVALID_HANDLE_VALUE, STATUS_SUCCESS,
    };
    use windows_sys::Win32::Security::{
        DACL_SECURITY_INFORMATION, GetSecurityInfo, OWNER_SECURITY_INFORMATION,
        PROTECTED_DACL_SECURITY_INFORMATION, SE_FILE_OBJECT, SetSecurityInfo,
        UNPROTECTED_DACL_SECURITY_INFORMATION,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CREATE_ALWAYS, CreateFileW, FILE_ACCESS_FLAGS, FILE_ATTRIBUTE_NORMAL, FILE_CREATE,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_WRITE_THROUGH,
        FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_LIST_DIRECTORY, FILE_OPEN, FILE_OPEN_IF,
        FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FILE_WRITE_ATTRIBUTES, FILE_WRITE_DATA, FILE_WRITE_EA, FindClose, FindFirstFileW,
        FindNextFileW, FlushFileBuffers, OPEN_EXISTING, SYNCHRONIZE, SetEndOfFile, WIN32_FIND_DATAW,
        WRITE_DAC, WRITE_OWNER,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::FSCTL_SET_REPARSE_POINT;
    use windows_sys::Win32::System::WindowsProgramming::{
        FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS, FILE_DISPOSITION_INFO,
        FILE_RENAME_FLAG_POSIX_SEMANTICS, FILE_RENAME_FLAG_REPLACE_IF_EXISTS, FILE_RENAME_INFO,
        NtCreateFile,
    };
    use windows_sys::Win32::System::Kernel::{
        FILE_ATTRIBUTE_TAG_INFO, FILE_ID_BOTH_DIR_INFO, IO_STATUS_BLOCK, OBJ_CASE_INSENSITIVE,
        OBJECT_ATTRIBUTES, NT_SUCCESS, RtlInitUnicodeString, UNICODE_STRING,
    };
    use windows_sys::Win32::System::Threading::{
        AcquireSRWLockExclusive, AcquireSRWLockShared, InitializeSRWLock, ReleaseSRWLockExclusive,
        ReleaseSRWLockShared, SRWLOCK,
    };
    use windows_sys::Win32::System::SystemServices::{
        FileAttributeTagInfo, FileDispositionInfo, FileDispositionInfoEx, FileIdBothDirectoryInfo,
        FileRenameInfoEx,
    };
    use windows_sys::Win32::System::IO::GetFileInformationByHandleEx;
    use windows_sys::Win32::System::SystemInformation::{GetSystemDirectoryW, GetTickCount64};

    const FILE_DIRECTORY_FILE: u32 = 0x0000_0001;
    const FILE_NON_DIRECTORY_FILE: u32 = 0x0000_0040;
    const FILE_OPEN_REPARSE_POINT_FLAG: u32 = 0x0020_0000;
    const FILE_DELETE_ON_CLOSE: u32 = 0x0000_1000;
    const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x0000_0020;
    const FILE_OPEN_FOR_BACKUP_INTENT: u32 = 0x0000_4000;
    const FILE_VALID_SET_FLAGS: u32 = 0x0000_0002;
    const FILE_RENAME_FLAGS: u32 = FILE_RENAME_FLAG_REPLACE_IF_EXISTS | FILE_RENAME_FLAG_POSIX_SEMANTICS;
    const MAX_LOCK_WAIT_MS: u64 = 10_000;
    const MAX_TREE_ENTRIES: usize = 100_000;
    const MAX_TREE_DEPTH: usize = 128;

    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) { unsafe { CloseHandle(self.0); } }
    }
    unsafe impl Send for Handle {}
    unsafe impl Sync for Handle {}

    struct FileLock {
        handle: Handle,
        name: Vec<u16>,
        parent: Handle,
    }

    fn err(kind: io::ErrorKind, code: &'static str) -> SafeWriteError { SafeWriteError::new(kind, code) }
    fn winerr(code: &'static str) -> SafeWriteError { SafeWriteError::new(io::Error::last_os_error().kind(), code) }
    fn wide(value: &std::ffi::OsStr) -> Result<Vec<u16>, SafeWriteError> {
        use std::os::windows::ffi::OsStrExt;
        let value: Vec<u16> = value.encode_wide().collect();
        if value.contains(&0) { return Err(err(io::ErrorKind::InvalidInput, "PATH_INVALID")); }
        Ok(value)
    }
    fn name_wide(value: &std::ffi::OsStr) -> Result<Vec<u16>, SafeWriteError> {
        let value = wide(value)?;
        if value.is_empty() || value.len() > 255 || value.contains(&b'\\' as u16) || value.contains(&b'/' as u16) {
            return Err(err(io::ErrorKind::InvalidInput, "PATH_INVALID"));
        }
        Ok(value)
    }
    fn nt_path(path: &Path) -> Result<Vec<u16>, SafeWriteError> {
        let path = path.canonicalize().map_err(|_| err(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID"))?;
        let mut value = wide(path.as_os_str())?;
        if value.starts_with(&[b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16]) {
            value.drain(..4);
        } else if value.starts_with(&[b'\\' as u16, b'\\' as u16]) {
            value.drain(..2);
            value.splice(..0, [b'U' as u16, b'N' as u16, b'C' as u16, b'\\' as u16]);
        }
        let mut result = vec![b'\\' as u16, b'?' as u16, b'?' as u16, b'\\' as u16];
        result.extend(value);
        Ok(result)
    }
    fn unicode(value: &mut [u16]) -> UNICODE_STRING {
        UNICODE_STRING { Length: (value.len() * 2) as u16, MaximumLength: (value.len() * 2) as u16, Buffer: value.as_mut_ptr() }
    }
    fn object_attributes(name: &mut UNICODE_STRING, root: HANDLE) -> OBJECT_ATTRIBUTES {
        OBJECT_ATTRIBUTES {
            Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
            RootDirectory: root,
            ObjectName: name,
            Attributes: OBJ_CASE_INSENSITIVE,
            SecurityDescriptor: null_mut(),
            SecurityQualityOfService: null_mut(),
        }
    }
    fn nt_open(
        root: HANDLE,
        name: &mut [u16],
        access: FILE_ACCESS_FLAGS,
        attributes: u32,
        share: u32,
        disposition: u32,
        options: u32,
    ) -> Result<Handle, SafeWriteError> {
        unsafe {
            let mut name = unicode(name);
            let mut object = object_attributes(&mut name, root);
            let mut handle = INVALID_HANDLE_VALUE;
            let mut status: IO_STATUS_BLOCK = zeroed();
            let result = NtCreateFile(
                &mut handle, access, &mut object, &mut status, null_mut(), attributes,
                share, disposition, options, null_mut(), 0,
            );
            if result != STATUS_SUCCESS || handle == INVALID_HANDLE_VALUE {
                return Err(err(io::ErrorKind::PermissionDenied, "SAFE_PATH_OPEN_FAILED"));
            }
            Ok(Handle(handle))
        }
    }
    fn open_root(path: &Path) -> Result<(PathBuf, Handle), SafeWriteError> {
        let canonical = path.canonicalize().map_err(|_| err(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID"))?;
        let mut name = nt_path(&canonical)?;
        let handle = nt_open(
            null_mut(), &mut name,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | FILE_READ_DATA | FILE_TRAVERSE | SYNCHRONIZE,
            0, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, FILE_OPEN,
            FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT,
        )?;
        Ok((canonical, handle))
    }
    // Imported here to keep all Win32 flag use in one module.
    const FILE_TRAVERSE: u32 = 0x0020;
    fn open_child_directory(parent: HANDLE, name: &std::ffi::OsStr, create: bool) -> Result<Handle, SafeWriteError> {
        let mut name = name_wide(name)?;
        match nt_open(
            parent, &mut name,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | FILE_READ_DATA | FILE_TRAVERSE | SYNCHRONIZE,
            0, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, FILE_OPEN,
            FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT | FILE_OPEN_REPARSE_POINT_FLAG,
        ) {
            Ok(handle) => {
                if is_reparse(handle.0)? { return Err(err(io::ErrorKind::PermissionDenied, "SAFE_PATH_ESCAPE")); }
                Ok(handle)
            }
            Err(error) if create => {
                let created = nt_open(
                    parent, &mut name,
                    FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | FILE_READ_DATA | FILE_TRAVERSE | SYNCHRONIZE,
                    0, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, FILE_CREATE,
                    FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT,
                ).map_err(|_| err(io::ErrorKind::PermissionDenied, "SAFE_DIRECTORY_CREATE_FAILED"))?;
                if is_reparse(created.0)? { return Err(err(io::ErrorKind::PermissionDenied, "SAFE_PATH_ESCAPE")); }
                let _ = error;
                Ok(created)
            }
            Err(_) => Err(err(io::ErrorKind::NotFound, "FILE_NOT_FOUND")),
        }
    }
    fn open_parent(root: HANDLE, relative: &Path, create: bool) -> Result<(Handle, Vec<u16>), SafeWriteError> {
        validate_relative(relative)?;
        let mut parent = Handle(root);
        // Ownership of the root is retained by the caller; duplicate it for a uniform guard.
        let mut initial = Vec::new();
        let components: Vec<_> = relative.components().filter_map(|part| match part { std::path::Component::Normal(value) => Some(value), _ => None }).collect();
        let name = name_wide(components.last().ok_or_else(|| err(io::ErrorKind::InvalidInput, "PATH_INVALID"))?)?;
        if components.len() > 1 {
            initial = nt_open(
                root, &mut name_wide(components[0])?,
                FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | FILE_READ_DATA | FILE_TRAVERSE | SYNCHRONIZE,
                0, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, FILE_OPEN,
                FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT | FILE_OPEN_REPARSE_POINT_FLAG,
            ).map(|h| vec![h]).unwrap_or_default();
        }
        let mut current = if components.len() > 1 {
            if initial.is_empty() {
                open_child_directory(root, components[0], create)?
            } else {
                let handle = initial.pop().unwrap();
                if is_reparse(handle.0)? { return Err(err(io::ErrorKind::PermissionDenied, "SAFE_PATH_ESCAPE")); }
                handle
            }
        } else {
            // A null handle is not dereferenced: this branch is replaced below.
            Handle(null_mut())
        };
        if components.len() == 1 {
            // Open the root again instead of duplicating a borrowed HANDLE.
            let root_name = final_path(root)?;
            let mut root_name = root_name;
            current = nt_open(
                null_mut(), &mut root_name,
                FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | FILE_READ_DATA | FILE_TRAVERSE | SYNCHRONIZE,
                0, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, FILE_OPEN,
                FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT,
            )?;
        }
        for component in components.iter().skip(1).take(components.len().saturating_sub(2)) {
            current = open_child_directory(current.0, component, create)?;
        }
        let _ = parent;
        Ok((current, name))
    }
    fn is_reparse(handle: HANDLE) -> Result<bool, SafeWriteError> {
        unsafe {
            let mut info: FILE_ATTRIBUTE_TAG_INFO = zeroed();
            if GetFileInformationByHandleEx(handle, FileAttributeTagInfo, &mut info as *mut _ as *mut c_void, size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32) == 0 {
                return Err(winerr("FILE_STAT_FAILED"));
            }
            Ok(info.FileAttributes & 0x400 != 0)
        }
    }
    fn final_path(handle: HANDLE) -> Result<Vec<u16>, SafeWriteError> {
        unsafe {
            let mut length = 512usize;
            loop {
                let mut buffer = vec![0u16; length];
                let written = windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW(handle, buffer.as_mut_ptr(), buffer.len() as u32, 0);
                if written == 0 { return Err(winerr("FINAL_PATH_FAILED")); }
                if written as usize >= buffer.len() { length = written as usize + 1; continue; }
                buffer.truncate(written as usize);
                return Ok(buffer);
            }
        }
    }
    fn read_handle(handle: HANDLE, max_bytes: usize) -> Result<Vec<u8>, SafeWriteError> {
        use std::os::windows::io::FromRawHandle;
        use std::io::Read;
        unsafe {
            let owned = Handle(handle);
            let metadata = fs::File::from_raw_handle(handle).metadata().map_err(|_| err(io::ErrorKind::Other, "FILE_STAT_FAILED"))?;
            std::mem::forget(owned);
            if !metadata.is_file() || metadata.len() > max_bytes as u64 {
                return Err(err(io::ErrorKind::InvalidData, "FILE_TOO_LARGE"));
            }
            let mut file = fs::File::from_raw_handle(handle);
            let mut bytes = Vec::with_capacity(metadata.len() as usize);
            file.take(max_bytes as u64 + 1).read_to_end(&mut bytes).map_err(|_| err(io::ErrorKind::Other, "FILE_READ_FAILED"))?;
            std::mem::forget(file);
            if bytes.len() > max_bytes { return Err(err(io::ErrorKind::InvalidData, "FILE_TOO_LARGE")); }
            Ok(bytes)
        }
    }
    fn open_regular(parent: HANDLE, name: &mut [u16], write: bool) -> Result<Handle, SafeWriteError> {
        let access = FILE_GENERIC_READ | FILE_READ_ATTRIBUTES | SYNCHRONIZE | if write { FILE_WRITE_ATTRIBUTES } else { 0 };
        let handle = nt_open(
            parent, name, access, FILE_ATTRIBUTE_NORMAL,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, FILE_OPEN,
            FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT_FLAG | FILE_SYNCHRONOUS_IO_NONALERT,
        ).map_err(|_| err(io::ErrorKind::NotFound, "FILE_NOT_FOUND"))?;
        if is_reparse(handle.0)? { return Err(err(io::ErrorKind::PermissionDenied, "SAFE_PATH_ESCAPE")); }
        Ok(handle)
    }
    fn read_beneath(root: &Path, relative: &Path, max_bytes: usize) -> Result<Vec<u8>, SafeWriteError> {
        let (_, root_handle) = open_root(root)?;
        let (parent, mut name) = open_parent(root_handle.0, relative, false)?;
        let handle = open_regular(parent.0, &mut name, false)?;
        read_handle(handle.0, max_bytes)
    }
    fn read_hash(root: &Path, relative: &Path, max_bytes: usize) -> Result<Option<String>, SafeWriteError> {
        match read_beneath(root, relative, max_bytes) {
            Ok(bytes) => Ok(Some(crate::capabilities::workspace::cas::hash_bytes(&bytes))),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
    fn lock_name(relative: &Path) -> Result<Vec<u16>, SafeWriteError> {
        let name = relative.file_name().ok_or_else(|| err(io::ErrorKind::InvalidInput, "PATH_INVALID"))?;
        let digest = crate::capabilities::workspace::cas::hash_bytes(name.to_string_lossy().as_bytes());
        name_wide(std::ffi::OsStr::new(&format!(".{digest}.browseros-cas.lock")))
    }
    fn acquire_lock(root: &Path, relative: &Path) -> Result<FileLock, SafeWriteError> {
        let (_, root_handle) = open_root(root)?;
        let (parent, _) = open_parent(root_handle.0, relative, false)?;
        let mut name = lock_name(relative)?;
        let handle = nt_open(
            parent.0, &mut name,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | SYNCHRONIZE,
            FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_OPEN_IF, FILE_NON_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT,
        ).map_err(|_| err(io::ErrorKind::PermissionDenied, "CAS_LOCK_FAILED"))?;
        if is_reparse(handle.0)? { return Err(err(io::ErrorKind::PermissionDenied, "CAS_LOCK_FAILED")); }
        let start = unsafe { GetTickCount64() };
        let mut offset: windows_sys::Win32::System::IO::OVERLAPPED = unsafe { zeroed() };
        loop {
            let result = unsafe { windows_sys::Win32::Storage::FileSystem::LockFileEx(handle.0, 0x0000_0002 | 0x0000_0001, 0, u32::MAX, u32::MAX, &mut offset) };
            if result != 0 { return Ok(FileLock { handle, name, parent }); }
            if unsafe { GetTickCount64() } - start >= MAX_LOCK_WAIT_MS {
                return Err(err(io::ErrorKind::TimedOut, "CAS_LOCK_TIMEOUT"));
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }
    impl Drop for FileLock {
        fn drop(&mut self) {
            unsafe {
                let mut offset: windows_sys::Win32::System::IO::OVERLAPPED = zeroed();
                windows_sys::Win32::Storage::FileSystem::UnlockFileEx(self.handle.0, 0, u32::MAX, u32::MAX, &mut offset);
                let mut unicode = unicode(&mut self.name);
                let mut object = object_attributes(&mut unicode, self.parent.0);
                let mut status: IO_STATUS_BLOCK = zeroed();
                let mut handle = INVALID_HANDLE_VALUE;
                if NtCreateFile(&mut handle, DELETE | SYNCHRONIZE, &mut object, &mut status, null_mut(), FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, FILE_OPEN, FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT_FLAG | FILE_DELETE_ON_CLOSE | FILE_SYNCHRONOUS_IO_NONALERT, null_mut(), 0) == STATUS_SUCCESS {
                    CloseHandle(handle);
                }
            }
        }
    }
    const DELETE: u32 = 0x0001_0000;
    fn preserve_security(source: HANDLE, destination: HANDLE) -> Result<(), SafeWriteError> {
        unsafe {
            let mut owner = null_mut();
            let mut group = null_mut();
            let mut dacl = null_mut();
            let mut sacl = null_mut();
            let mut descriptor = null_mut();
            let result = GetSecurityInfo(
                source, SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | windows_sys::Win32::Security::GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner, &mut group, &mut dacl, &mut sacl, &mut descriptor,
            );
            if result != 0 { return Err(err(io::ErrorKind::PermissionDenied, "CAS_SECURITY_READ_FAILED")); }
            let set = SetSecurityInfo(
                destination, SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(), null_mut(), dacl, null_mut(),
            );
            windows_sys::Win32::Foundation::LocalFree(descriptor as _);
            if set != 0 { return Err(err(io::ErrorKind::PermissionDenied, "CAS_SECURITY_WRITE_FAILED")); }
            Ok(())
        }
    }
    fn rename_relative(parent: HANDLE, source: HANDLE, target: &[u16]) -> Result<(), SafeWriteError> {
        unsafe {
            let bytes = size_of::<FILE_RENAME_INFO>() + target.len() * 2;
            let mut buffer = vec![0u8; bytes];
            let info = &mut *(buffer.as_mut_ptr() as *mut FILE_RENAME_INFO);
            info.Anonymous.Flags = FILE_RENAME_FLAGS;
            info.RootDirectory = parent;
            info.FileNameLength = (target.len() * 2) as u32;
            std::ptr::copy_nonoverlapping(target.as_ptr(), info.FileName.as_mut_ptr(), target.len());
            if !windows_sys::Win32::System::IO::SetFileInformationByHandle(
                source, FileRenameInfoEx, buffer.as_mut_ptr() as *mut c_void, buffer.len() as u32,
            ) {
                return Err(winerr("CAS_RENAME_FAILED"));
            }
            Ok(())
        }
    }
    fn save_beneath(root: &Path, relative: &Path, content: &[u8], expected: &str, max_bytes: usize) -> Result<SafeWriteOutcome, SafeWriteError> {
        let (_, root_handle) = open_root(root)?;
        let (parent, mut target_name) = open_parent(root_handle.0, relative, false)?;
        let before = read_hash(root, relative, max_bytes)?;
        if before.as_deref() != Some(expected) { return Ok(SafeWriteOutcome { saved: false, current_hash: before }); }
        let _lock = acquire_lock(root, relative)?;
        let before = read_hash(root, relative, max_bytes)?;
        if before.as_deref() != Some(expected) { return Ok(SafeWriteOutcome { saved: false, current_hash: before }); }
        let target = open_regular(parent.0, &mut target_name, true)?;
        let original = read_handle(target.0, max_bytes)?;
        if crate::capabilities::workspace::cas::hash_bytes(&original) != expected {
            return Ok(SafeWriteOutcome { saved: false, current_hash: Some(crate::capabilities::workspace::cas::hash_bytes(&original)) });
        }
        let mut temp_name = name_wide(std::ffi::OsStr::new(&format!(".{}.browseros-cas.{}.tmp", relative.file_name().unwrap().to_string_lossy(), uuid::Uuid::new_v4())))?;
        let temp = nt_open(
            parent.0, &mut temp_name,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_WRITE_ATTRIBUTES | DELETE | SYNCHRONIZE,
            FILE_ATTRIBUTE_NORMAL, 0, FILE_CREATE,
            FILE_NON_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_REPARSE_POINT_FLAG,
        ).map_err(|_| err(io::ErrorKind::PermissionDenied, "CAS_TEMP_CREATE_FAILED"))?;
        if is_reparse(temp.0)? { return Err(err(io::ErrorKind::PermissionDenied, "CAS_TEMP_CREATE_FAILED")); }
        preserve_security(target.0, temp.0)?;
        {
            use std::io::Write;
            use std::os::windows::io::FromRawHandle;
            let mut file = fs::File::from_raw_handle(temp.0);
            let result = file.write_all(content).and_then(|_| file.flush());
            std::mem::forget(file);
            result.map_err(|_| err(io::ErrorKind::Other, "CAS_TEMP_WRITE_FAILED"))?;
        }
        if unsafe { SetEndOfFile(temp.0) } == 0 || unsafe { FlushFileBuffers(temp.0) } == 0 {
            return Err(winerr("CAS_TEMP_WRITE_FAILED"));
        }
        rename_relative(parent.0, temp.0, &target_name)?;
        unsafe { FlushFileBuffers(parent.0); }
        Ok(SafeWriteOutcome { saved: true, current_hash: before })
    }
    fn create_tree_beneath(root: &Path, relative: &Path) -> Result<(), SafeWriteError> {
        validate_relative(relative)?;
        let (_, root_handle) = open_root(root)?;
        let components: Vec<_> = relative.components().filter_map(|part| match part { std::path::Component::Normal(value) => Some(value), _ => None }).collect();
        let mut current = root_handle;
        for component in components {
            current = open_child_directory(current.0, component, true)?;
        }
        Ok(())
    }
    pub fn atomic_write_beneath(root: &Path, relative: &Path, content: &[u8], max_bytes: usize) -> Result<(), SafeWriteError> {
        if content.len() > max_bytes { return Err(err(io::ErrorKind::InvalidData, "FILE_TOO_LARGE")); }
        validate_relative(relative)?;
        let (_, root_handle) = open_root(root)?;
        let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
        if !parent_relative.as_os_str().is_empty() { create_tree_beneath(root, parent_relative)?; }
        let (parent, mut target_name) = open_parent(root_handle.0, relative, true)?;
        let target = match open_regular(parent.0, &mut target_name, true) {
            Ok(handle) => Some(handle),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        let mut temp_name = name_wide(std::ffi::OsStr::new(&format!(".{}.browseros-agent.{}.tmp", relative.file_name().unwrap().to_string_lossy(), uuid::Uuid::new_v4())))?;
        let temp = nt_open(
            parent.0, &mut temp_name,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_WRITE_ATTRIBUTES | DELETE | SYNCHRONIZE,
            FILE_ATTRIBUTE_NORMAL, 0, FILE_CREATE,
            FILE_NON_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_REPARSE_POINT_FLAG,
        ).map_err(|_| err(io::ErrorKind::PermissionDenied, "AGENT_WRITE_FAILED"))?;
        if is_reparse(temp.0)? { return Err(err(io::ErrorKind::PermissionDenied, "AGENT_WRITE_FAILED")); }
        if let Some(target) = &target { preserve_security(target.0, temp.0)?; }
        {
            use std::io::Write;
            use std::os::windows::io::FromRawHandle;
            let mut file = fs::File::from_raw_handle(temp.0);
            let result = file.write_all(content).and_then(|_| file.flush());
            std::mem::forget(file);
            result.map_err(|_| err(io::ErrorKind::Other, "AGENT_WRITE_FAILED"))?;
        }
        if unsafe { SetEndOfFile(temp.0) } == 0 || unsafe { FlushFileBuffers(temp.0) } == 0 {
            return Err(winerr("AGENT_WRITE_FAILED"));
        }
        rename_relative(parent.0, temp.0, &target_name).map_err(|_| err(io::ErrorKind::Other, "AGENT_WRITE_FAILED"))?;
        unsafe { FlushFileBuffers(parent.0); }
        Ok(())
    }
    pub fn remove_beneath(root: &Path, relative: &Path) -> Result<(), SafeWriteError> {
        let (_, root_handle) = open_root(root)?;
        let (parent, mut name) = open_parent(root_handle.0, relative, false)?;
        let target = nt_open(
            parent.0, &mut name, DELETE | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_OPEN, FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT_FLAG | FILE_SYNCHRONOUS_IO_NONALERT,
        ).map_err(|_| err(io::ErrorKind::PermissionDenied, "AGENT_RECOVERY_DELETE_FAILED"))?;
        if is_reparse(target.0)? { return Err(err(io::ErrorKind::PermissionDenied, "SAFE_PATH_ESCAPE")); }
        unsafe {
            let info = FILE_DISPOSITION_INFO { Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS };
            if !windows_sys::Win32::System::IO::SetFileInformationByHandle(
                target.0, FileDispositionInfoEx, &info as *const _ as *mut c_void, size_of::<FILE_DISPOSITION_INFO>() as u32,
            ) {
                return Err(winerr("AGENT_RECOVERY_DELETE_FAILED"));
            }
            FlushFileBuffers(parent.0);
        }
        Ok(())
    }
    pub fn read(path: &Path, roots: &[PathBuf], max_bytes: usize) -> Result<Vec<u8>, SafeWriteError> {
        for root in roots {
            let canonical = root.canonicalize().map_err(|_| err(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID"))?;
            let candidate = path.canonicalize().map_err(|e| err(e.kind(), "FILE_NOT_FOUND"))?;
            if let Ok(relative) = candidate.strip_prefix(&canonical) {
                return read_beneath(&canonical, relative, max_bytes);
            }
        }
        Err(err(io::ErrorKind::PermissionDenied, "PATH_OUTSIDE_WORKSPACE"))
    }
    pub fn save(path: &Path, roots: &[PathBuf], content: &[u8], expected: &str, max_bytes: usize) -> Result<SafeWriteOutcome, SafeWriteError> {
        for root in roots {
            let canonical = root.canonicalize().map_err(|_| err(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID"))?;
            let candidate = path.canonicalize().map_err(|e| err(e.kind(), "FILE_NOT_FOUND"))?;
            if let Ok(relative) = candidate.strip_prefix(&canonical) {
                return save_beneath(&canonical, relative, content, expected, max_bytes);
            }
        }
        Err(err(io::ErrorKind::PermissionDenied, "PATH_OUTSIDE_WORKSPACE"))
    }
    pub fn create_tree(path: &Path, roots: &[PathBuf]) -> Result<(), SafeWriteError> {
        for root in roots {
            let canonical = root.canonicalize().map_err(|_| err(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID"))?;
            if let Ok(relative) = path.strip_prefix(&canonical) {
                return create_tree_beneath(&canonical, relative);
            }
        }
        Err(err(io::ErrorKind::PermissionDenied, "PATH_OUTSIDE_WORKSPACE"))
    }
    pub fn atomic_write(path: &Path, content: &[u8], max_bytes: usize) -> Result<(), SafeWriteError> {
        let parent = path.parent().ok_or_else(|| err(io::ErrorKind::PermissionDenied, "SAFE_PATH_ESCAPE"))?;
        let relative = path.file_name().ok_or_else(|| err(io::ErrorKind::InvalidInput, "PATH_INVALID"))?;
        atomic_write_beneath(parent, Path::new(relative), content, max_bytes)
    }
    pub fn remove(path: &Path, roots: &[PathBuf]) -> Result<(), SafeWriteError> {
        for root in roots {
            let canonical = root.canonicalize().map_err(|_| err(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID"))?;
            let candidate = path.canonicalize().map_err(|e| err(e.kind(), "FILE_NOT_FOUND"))?;
            if let Ok(relative) = candidate.strip_prefix(&canonical) {
                return remove_beneath(&canonical, relative);
            }
        }
        Err(err(io::ErrorKind::PermissionDenied, "PATH_OUTSIDE_WORKSPACE"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn windows_cas_writes_under_pinned_parent_and_preserves_dacl() {
            let root = std::env::temp_dir().join(format!("cas-win-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(root.join("nested")).unwrap();
            let target = root.join("nested/file.txt");
            fs::write(&target, "old").unwrap();
            let expected = crate::capabilities::workspace::cas::hash_bytes(b"old");
            let saved = save(&target, &[root.clone()], b"new", &expected, 1024).unwrap();
            assert!(saved.saved);
            assert_eq!(fs::read(&target).unwrap(), b"new");
            let conflict = save(&target, &[root.clone()], b"bad", &expected, 1024).unwrap();
            assert!(!conflict.saved);
            assert_eq!(fs::read(&target).unwrap(), b"new");
            fs::remove_dir_all(root).unwrap();
        }
        #[test]
        fn windows_agent_atomic_write_creates_nested_file() {
            let root = std::env::temp_dir().join(format!("agent-win-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&root).unwrap();
            atomic_write(&root.join("new/deep/file.txt"), b"approved", 1024).unwrap();
            assert_eq!(fs::read(root.join("new/deep/file.txt")).unwrap(), b"approved");
            fs::remove_dir_all(root).unwrap();
        }
        #[test]
        fn windows_recovery_remove_does_not_follow_junction() {
            let root = std::env::temp_dir().join(format!("remove-win-{}", uuid::Uuid::new_v4()));
            let outside = std::env::temp_dir().join(format!("remove-outside-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&root).unwrap();
            fs::create_dir_all(&outside).unwrap();
            fs::write(outside.join("keep.txt"), "keep").unwrap();
            let link = root.join("link");
            let command = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&link).arg(&outside).status().unwrap();
            if command.success() {
                assert!(remove(&link.join("keep.txt"), &[root.clone()]).is_err());
                assert!(outside.join("keep.txt").exists());
            }
            let _ = fs::remove_dir_all(root);
            let _ = fs::remove_dir_all(outside);
        }
    }
}

pub fn read_regular(path: &Path, roots: &[PathBuf], max_bytes: usize) -> Result<Vec<u8>, SafeWriteError> {
    #[cfg(unix)]
    {
        let canonical_path = path.canonicalize().map_err(|e| SafeWriteError::new(e.kind(), "PATH_NOT_FOUND"))?;
        for root in roots {
            let root = root.canonicalize().map_err(|_| SafeWriteError::new(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID"))?;
            if let Ok(relative) = canonical_path.strip_prefix(&root) { return read_at(&root, relative, max_bytes); }
        }
        Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "PATH_OUTSIDE_WORKSPACE"))
    }
    #[cfg(target_os = "windows")]
    { windows::read(path, roots, max_bytes) }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = (path, roots, max_bytes);
        Err(SafeWriteError::new(io::ErrorKind::Unsupported, "SAFE_FILESYSTEM_PLATFORM_UNSUPPORTED"))
    }
}

pub fn save_if_unchanged(path: &Path, roots: &[PathBuf], content: &[u8], expected: &str, max_bytes: usize) -> Result<SafeWriteOutcome, SafeWriteError> {
    if content.len() > max_bytes { return Err(SafeWriteError::new(io::ErrorKind::InvalidData, "FILE_TOO_LARGE")); }
    #[cfg(unix)]
    {
        let canonical_path = path.canonicalize().map_err(|e| SafeWriteError::new(e.kind(), "PATH_NOT_FOUND"))?;
        for root in roots {
            let root = root.canonicalize().map_err(|_| SafeWriteError::new(io::ErrorKind::NotFound, "WORKSPACE_ROOT_INVALID"))?;
            if let Ok(relative) = canonical_path.strip_prefix(&root) { return save_at(&root, relative, content, expected, max_bytes); }
        }
        Err(SafeWriteError::new(io::ErrorKind::PermissionDenied, "PATH_OUTSIDE_WORKSPACE"))
    }
    #[cfg(target_os = "windows")]
    { windows::save(path, roots, content, expected, max_bytes) }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = (path, roots, content, expected);
        Err(SafeWriteError::new(io::ErrorKind::Unsupported, "SAFE_FILESYSTEM_PLATFORM_UNSUPPORTED"))
    }
}

pub fn create_directory_tree(path: &Path, roots: &[PathBuf]) -> Result<(), SafeWriteError> {
    #[cfg(target_os = "windows")]
    { return windows::create_tree(path, roots); }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (path, roots);
        Err(SafeWriteError::new(io::ErrorKind::Unsupported, "SAFE_FILESYSTEM_PLATFORM_UNSUPPORTED"))
    }
}

pub fn atomic_write(path: &Path, content: &[u8], max_bytes: usize) -> Result<(), SafeWriteError> {
    #[cfg(target_os = "windows")]
    { return windows::atomic_write(path, content, max_bytes); }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (path, content, max_bytes);
        Err(SafeWriteError::new(io::ErrorKind::Unsupported, "SAFE_FILESYSTEM_PLATFORM_UNSUPPORTED"))
    }
}

pub fn remove_regular(path: &Path, roots: &[PathBuf]) -> Result<(), SafeWriteError> {
    #[cfg(target_os = "windows")]
    { return windows::remove(path, roots); }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (path, roots);
        Err(SafeWriteError::new(io::ErrorKind::Unsupported, "SAFE_FILESYSTEM_PLATFORM_UNSUPPORTED"))
    }
}
