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
    use std::path::{Component, Prefix};
    // Rust canonicalize() on Windows returns a verbatim local disk path
    // (\\?\C:\...), which is safe to inspect. Only UNC/device prefixes
    // are rejected; never treat a verbatim DISK path as an unsafe UNC share.
    let local_disk = matches!(
        path.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    );
    if !path.is_absolute() || !local_disk {
        return Err(denied());
    }
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::Prefix(_)) || !current.has_root() { continue; } // Skip drive-only C: prefix.
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

/// Private app data uses the same native policy as WebView2. AppContainer
/// profiles live outside app_data and have a separate package-SID policy.
pub(super) fn ensure_private_directory(path: &Path) -> io::Result<()> {
    let report = browseros_platform_security::windows::ensure_private_directory(
        path,
        &browseros_platform_security::windows::PrivateTreeOptions::default(),
    )?;
    eprintln!(
        "WINDOWS_PRIVATE_STORAGE_READY checked={} repaired={} elapsed_ms={}",
        report.checked_objects, report.repaired_objects, report.elapsed_ms,
    );
    Ok(())
}

fn verify_path_handle(path: &Path, file: &File) -> io::Result<()> {
    reject_reparse_ancestors(path)?;
    if !browseros_platform_security::windows::path_matches_handle(path, file)? {
        return Err(denied());
    }
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
    const FILE_TRAVERSE: u32 = 0x0000_0020;
    const FILE_LIST_DIRECTORY: u32 = 0x0000_0001;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    let mut options = OpenOptions::new();
    options
        .access_mode(
            FILE_READ_ATTRIBUTES
                | FILE_ADD_FILE
                | FILE_ADD_SUBDIRECTORY
                | FILE_TRAVERSE
                | FILE_LIST_DIRECTORY
                | SYNCHRONIZE,
        )
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
    let used = unsafe { GetFinalPathNameByHandleW(handle, buf.as_mut_ptr(), buf.len() as u32, 0) };
    if used == 0 || used as usize >= buf.len() {
        return Err(io::Error::last_os_error());
    }
    // Do not fold names or merge undecodable UTF-16 spellings. Authorization
    // uses native file identities; text here is only a bounded path transport.
    String::from_utf16(&buf[..used as usize]).map_err(|_| invalid_input())
}
fn open_authorized_root(path: &Path, roots: &[PathBuf]) -> io::Result<File> {
    let candidate = open(path, true)?;
    for root in roots {
        let Ok(trusted) = open(root, true) else { continue; };
        if browseros_platform_security::windows::handle_is_within_root(&candidate, &trusted)? {
            return Ok(trusted);
        }
    }
    Err(denied())
}

fn authorize_with_root(file: &File, root: &File) -> io::Result<String> {
    if browseros_platform_security::windows::handle_is_within_root(file, root)? {
        return final_path(file);
    }
    Err(denied())
}
/// No-overwrite rename by existing source/destination OS handles.
/// FILE_RENAME_INFO RootDirectory pins the actual destination directory,
/// and ReplaceIfExists=false avoids path-only check-then-rename races.
pub(super) fn move_no_replace(src: &Path, dest_dir: &Path, roots: &[PathBuf]) -> io::Result<()> {
    let filename = src.file_name().ok_or_else(denied)?;
    rename_no_replace(src, dest_dir, filename, roots)
}
pub(super) fn rename_no_replace(
    src: &Path, dest_dir: &Path, new_name: &std::ffi::OsStr, roots: &[PathBuf],
) -> io::Result<()> {
    use std::ffi::c_void;
    use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
    use windows_sys::Wdk::Storage::FileSystem::{
        FileRenameInformation, NtSetInformationFile, FILE_RENAME_INFORMATION,
        FILE_RENAME_INFORMATION_0,
    };
    use windows_sys::Win32::Foundation::RtlNtStatusToDosError;
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;
    const DELETE: u32 = 0x0001_0000;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const SYNCHRONIZE: u32 = 0x0010_0000;

    if !src.is_absolute() || !dest_dir.is_absolute() {
        return Err(denied());
    }
    reject_reparse_ancestors(src)?;
    reject_reparse_ancestors(dest_dir)?;
    let filename = new_name;
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
        .access_mode(DELETE | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
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
    authorize_with_root(&source, &source_root)?;
    // Rename requires FILE_ADD_FILE / FILE_ADD_SUBDIRECTORY rights on
    // the pinned destination directory handle. A read-only directory
    // handle passes path checks but fails every real FileRenameInfo call.
    let destination = open_destination_directory(dest_dir)?;
    authorize_with_root(&destination, &destination_root)?;
    if browseros_platform_security::windows::file_identity(&destination)?
        == browseros_platform_security::windows::file_identity(&source)?
        || meta.is_dir()
            && browseros_platform_security::windows::handle_is_within_root(&destination, &source)?
    {
        return Err(denied());
    }

    let chars: Vec<u16> = filename.encode_wide().collect();
    if chars.is_empty() || chars.len() > 32000 {
        return Err(denied());
    }
    let offset = std::mem::offset_of!(FILE_RENAME_INFORMATION, FileName);
    let size = (offset + chars.len() * std::mem::size_of::<u16>())
        .max(std::mem::size_of::<FILE_RENAME_INFORMATION>());
    let mut buffer = vec![0u64; (size + 7) / 8];
    let info = buffer.as_mut_ptr() as *mut FILE_RENAME_INFORMATION;
    unsafe {
        (*info).Anonymous = FILE_RENAME_INFORMATION_0 {
            ReplaceIfExists: false,
        };
        (*info).RootDirectory = destination.as_raw_handle() as _;
        (*info).FileNameLength = (chars.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            chars.as_ptr(),
            (buffer.as_mut_ptr() as *mut u8).add(offset) as *mut u16,
            chars.len(),
        );
        let mut status_block = IO_STATUS_BLOCK::default();
        // Win32 SetFileInformationByHandle has a documented-in-practice
        // ERROR_INVALID_PARAMETER path for relative names with a
        // RootDirectory handle. NtSetInformationFile is the native form
        // that actually resolves the target against the pinned directory.
        let status = NtSetInformationFile(
            source.as_raw_handle() as _,
            &mut status_block,
            buffer.as_ptr() as *const c_void,
            size as u32,
            FileRenameInformation,
        );
        if status < 0 {
            let dos = RtlNtStatusToDosError(status) as i32;
            return Err(io::Error::new(
                io::Error::from_raw_os_error(dos).kind(),
                format!(
                    "WINDOWS_RENAME_FAILED_NTSTATUS_{:08x}_WIN32_{}",
                    status as u32, dos
                ),
            ));
        }
    }
    Ok(())
}

/// New-file creation uses NtCreateFile(FILE_CREATE) under a pinned parent,
/// then renames the fully flushed stage without replacing existing entries.
/// Never fall back to a raw path-based std::fs::write after authorization.
pub(super) fn create_new_file(
    path: &Path, roots: &[PathBuf], content: &[u8],
) -> io::Result<()> {
    use std::io::Write;
    use windows_sys::Wdk::Storage::FileSystem::FILE_CREATE;
    const DELETE: u32 = 0x0001_0000;
    const FILE_WRITE_DATA: u32 = 0x0000_0002;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    if !path.is_absolute() { return Err(denied()); }
    let parent_path = path.parent().ok_or_else(denied)?;
    let filename = path.file_name().ok_or_else(denied)?;
    let root = open_authorized_root(parent_path, roots)?;
    let parent = open_destination_directory(parent_path)?;
    authorize_with_root(&parent, &root)?;
    let temp = std::ffi::OsString::from(format!(
        ".browseros-create-{}.tmp", uuid::Uuid::new_v4()
    ));
    let result = (|| {
        let mut stage = open_child_at(
            &parent, &temp,
            DELETE | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ_WRITE_DELETE,
            FILE_CREATE,
        )?;
        stage.write_all(content)?;
        stage.sync_all()?;
        drop(stage);
        rename_no_replace(&parent_path.join(&temp), parent_path, filename, roots)
    })();
    if result.is_err() { let _ = remove_regular_at(&parent, &temp); }
    result
}

/// Single-component create only: never call create_dir_all on user-supplied
/// paths, which could follow a swapped Windows junction.
pub(super) fn create_new_directory(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
    use windows_sys::Wdk::Storage::FileSystem::FILE_CREATE;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const FILE_LIST_DIRECTORY: u32 = 0x0000_0001;
    if !path.is_absolute() { return Err(denied()); }
    let parent_path = path.parent().ok_or_else(denied)?;
    let filename = path.file_name().ok_or_else(denied)?;
    let root = open_authorized_root(parent_path, roots)?;
    let parent = open_destination_directory(parent_path)?;
    authorize_with_root(&parent, &root)?;
    let _created = open_object_at(
        &parent, filename,
        FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY,
        FILE_SHARE_READ_WRITE_DELETE,
        FILE_CREATE, true,
    )?;
    Ok(())
}

/// Recursively create missing directories using the nearest pinned,
/// already-authorized ancestor. Never call create_dir_all on a user-provided
/// path after a prior, replaceable canonicalize check.
pub(super) fn create_directory_all(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
    use windows_sys::Wdk::Storage::FileSystem::{FILE_CREATE, FILE_OPEN_IF};
    const FILE_ADD_FILE: u32 = 0x0000_0002;
    const FILE_ADD_SUBDIRECTORY: u32 = 0x0000_0004;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const FILE_LIST_DIRECTORY: u32 = 0x0000_0001;
    const FILE_TRAVERSE: u32 = 0x0000_0020;
    if !path.is_absolute() { return Err(denied()); }
    let mut ancestor = path.parent().ok_or_else(denied)?;
    let mut missing = Vec::<std::ffi::OsString>::new();
    while !ancestor.exists() {
        missing.push(ancestor.file_name().ok_or_else(denied)?.to_os_string());
        ancestor = ancestor.parent().ok_or_else(denied)?;
    }
    let root = open_authorized_root(ancestor, roots)?;
    let mut parent = open_destination_directory(ancestor)?;
    authorize_with_root(&parent, &root)?;
    for name in missing.iter().rev() {
        parent = open_object_at(
            &parent, name,
            FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY
                | FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY | FILE_TRAVERSE,
            FILE_SHARE_READ_WRITE_DELETE,
            FILE_OPEN_IF, true,
        )?;
        authorize_with_root(&parent, &root)?;
    }
    let filename = path.file_name().ok_or_else(denied)?;
    let _created = open_object_at(
        &parent, filename,
        FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY,
        FILE_SHARE_READ_WRITE_DELETE,
        FILE_CREATE, true,
    )?;
    Ok(())
}

pub(super) fn read_regular(path: &Path, roots: &[PathBuf], limit: usize) -> io::Result<Vec<u8>> {
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
                NtCreateFile, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE,
                FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
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
    attributes.Attributes = 0; // Preserve exact names in case-sensitive directories.
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
    // NtCreateFile resolves this single validated component against parent.
    // Confirm object ancestry by identity, not by lowercased path strings.
    if !browseros_platform_security::windows::handle_is_within_root(&file, parent)? {
        return Err(denied());
    }
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

#[derive(Debug, PartialEq, Eq)]
struct FileIdentity {
    volume: u32,
    index: u64,
    size: u64,
    modified: (u32, u32),
}

fn identity(file: &File) -> io::Result<FileIdentity> {
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    let info = unsafe {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        if GetFileInformationByHandle(file.as_raw_handle() as _, &mut info) == 0 {
            return Err(io::Error::last_os_error());
        }
        info
    };
    let index = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    if index == 0 {
        return Err(io::Error::new(io::ErrorKind::Unsupported, "WINDOWS_FILE_IDENTITY_UNAVAILABLE"));
    }
    Ok(FileIdentity {
        volume: info.dwVolumeSerialNumber,
        index,
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
    let before = identity(file).map_err(|_| "FILE_IDENTITY_UNAVAILABLE")?;
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
    if before != identity { return Err("FILE_CONFLICT".into()); }
    Ok((identity, bytes))
}

fn copy_dacl(source: &File, destination: &File) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            Authorization::{GetSecurityInfo, SetSecurityInfo, SE_FILE_OBJECT},
            ACL, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
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
    rename_at(source, parent, target, true)
}

fn rename_at(source: &File, parent: &File, target: &std::ffi::OsStr, replace: bool) -> io::Result<()> {
    use std::ffi::c_void;
    use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
    use windows_sys::Wdk::Storage::FileSystem::{
        FileRenameInformation, NtSetInformationFile, FILE_RENAME_INFORMATION,
        FILE_RENAME_INFORMATION_0,
    };
    use windows_sys::Win32::Foundation::RtlNtStatusToDosError;
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;
    let chars: Vec<u16> = target.encode_wide().collect();
    if chars.is_empty() || chars.len() > 32000 {
        return Err(denied());
    }
    // SetFileInformationByHandle(FileRenameInfo) with a NULL RootDirectory
    // expects a full path; with a bare name Win32 returns ERROR_INVALID_PARAMETER.
    // The native form resolves the bare name against the pinned parent handle,
    // so the existing file is atomically replaced even if its path was swapped.
    let offset = std::mem::offset_of!(FILE_RENAME_INFORMATION, FileName);
    let size = (offset + chars.len() * std::mem::size_of::<u16>())
        .max(std::mem::size_of::<FILE_RENAME_INFORMATION>());
    let mut buffer = vec![0u64; (size + 7) / 8];
    let info = buffer.as_mut_ptr() as *mut FILE_RENAME_INFORMATION;
    unsafe {
        (*info).Anonymous = FILE_RENAME_INFORMATION_0 {
            ReplaceIfExists: replace,
        };
        (*info).RootDirectory = parent.as_raw_handle() as _;
        (*info).FileNameLength = (chars.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            chars.as_ptr(),
            (buffer.as_mut_ptr() as *mut u8).add(offset) as *mut u16,
            chars.len(),
        );
        let mut status_block = IO_STATUS_BLOCK::default();
        let status = NtSetInformationFile(
            source.as_raw_handle() as _,
            &mut status_block,
            buffer.as_ptr() as *const c_void,
            size as u32,
            FileRenameInformation,
        );
        if status < 0 {
            let dos = RtlNtStatusToDosError(status) as i32;
            return Err(io::Error::new(
                io::Error::from_raw_os_error(dos).kind(),
                format!(
                    "WINDOWS_REPLACE_FAILED_NTSTATUS_{:08x}_WIN32_{}",
                    status as u32, dos
                ),
            ));
        }
    }
    Ok(())
}

fn delete_on_close(file: &File) {
    use std::ffi::c_void;
    use windows_sys::Win32::Storage::FileSystem::{
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
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
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
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

/// Delete a directory by its open kernel handle. A directory path is used
/// only to ENUMERATE candidate names; all child opens and mutations are
/// resolved against the already-authorized parent handle via NtCreateFile.
fn mark_object_deleted(object: &File) -> io::Result<()> {
    use std::ffi::c_void;
    use windows_sys::Win32::Storage::FileSystem::{
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
    };
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    if unsafe {
        SetFileInformationByHandle(
            object.as_raw_handle() as _,
            FileDispositionInfo,
            &disposition as *const _ as *const c_void,
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// A moved or replaced path can change the directory visible by name. Use
/// handle identity to verify that the read_dir path is still the pinned
/// directory; never trust an enumerated entry path for mutation.
fn names_of_pinned_directory(dir: &File) -> io::Result<Vec<(std::ffi::OsString, bool)>> {
    let named = PathBuf::from(final_path(dir)?);
    let path_handle = open_destination_directory(&named)?;
    let a = identity(dir)?;
    let b = identity(&path_handle)?;
    if a.volume != b.volume || a.index != b.index {
        return Err(denied());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&named)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if !kind.is_dir() && !kind.is_file() { return Err(denied()); }
        if entries.len() >= 50_000 { return Err(denied()); }
        entries.push((entry.file_name(), kind.is_dir()));
    }
    // A rename while enumerating cannot redirect any of the eventual NT
    // child opens. Still fail closed when the name no longer represents the
    // pinned directory.
    let after = open_destination_directory(&named)?;
    let current = identity(&after)?;
    if a.volume != current.volume || a.index != current.index {
        return Err(denied());
    }
    Ok(entries)
}

/// Read-only NT-handle directory listing. Only a path belonging to the
/// pinned handle may be enumerated; child metadata uses NtCreateFile relative
/// to that handle, never a second mutable host path.
pub(super) fn list_directory(
    path: &Path, roots: &[PathBuf],
) -> io::Result<Vec<(std::ffi::OsString, bool, u64)>> {
    use windows_sys::Wdk::Storage::FileSystem::FILE_OPEN;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    if !path.is_absolute() { return Err(denied()); }
    let root = open_authorized_root(path, roots)?;
    let directory = open(path, true)?;
    authorize_with_root(&directory, &root)?;
    let named = PathBuf::from(final_path(&directory)?);
    let pinned = identity(&directory)?;
    let check_pinned = || -> io::Result<()> {
        let currently_named = open(&named, true)?;
        let current = identity(&currently_named)?;
        if pinned.volume != current.volume || pinned.index != current.index {
            return Err(denied());
        }
        Ok(())
    };
    check_pinned()?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&named)? {
        let Ok(entry) = entry else { continue; };
        if entries.len() >= 50_000 { return Err(denied()); }
        let filename = entry.file_name();
        let Ok(kind) = entry.file_type() else { continue; };
        if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) {
            entries.push((filename, false, 0));
            continue;
        }
        let Ok(child) = open_object_at(
            &directory, &filename, FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ_WRITE_DELETE, FILE_OPEN, kind.is_dir(),
        ) else { continue; };
        let Ok(metadata) = child.metadata() else { continue; };
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            entries.push((filename, false, 0));
            continue;
        }
        authorize_with_root(&child, &root)?;
        entries.push((
            filename, metadata.is_dir(),
            if metadata.is_file() { metadata.len() } else { 0 },
        ));
    }
    check_pinned()?;
    Ok(entries)
}

fn remove_directory_contents(
    directory: &File, authorized_root: &File, depth: usize, remaining: &mut usize,
) -> io::Result<()> {
    use windows_sys::Wdk::Storage::FileSystem::FILE_OPEN;
    const DELETE: u32 = 0x0001_0000;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const FILE_LIST_DIRECTORY: u32 = 0x0000_0001;
    const FILE_TRAVERSE: u32 = 0x0000_0020;
    if depth > 64 { return Err(denied()); }
    authorize_with_root(directory, authorized_root)?;
    for (name, is_dir) in names_of_pinned_directory(directory)? {
        if *remaining == 0 { return Err(denied()); }
        *remaining -= 1;
        if is_dir {
            let nested = open_object_at(
                directory, &name,
                DELETE | FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY | FILE_TRAVERSE,
                FILE_SHARE_READ_WRITE_DELETE, FILE_OPEN, true,
            )?;
            authorize_with_root(&nested, authorized_root)?;
            remove_directory_contents(&nested, authorized_root, depth + 1, remaining)?;
            mark_object_deleted(&nested)?;
            drop(nested);
        } else {
            let child = open_child_at(
                directory, &name,
                DELETE | FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ_WRITE_DELETE, FILE_OPEN,
            )?;
            authorize_with_root(&child, authorized_root)?;
            mark_object_deleted(&child)?;
            drop(child);
        }
    }
    Ok(())
}

pub(super) fn remove_tree(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
    use windows_sys::Wdk::Storage::FileSystem::FILE_OPEN;
    const DELETE: u32 = 0x0001_0000;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const FILE_LIST_DIRECTORY: u32 = 0x0000_0001;
    const FILE_TRAVERSE: u32 = 0x0000_0020;
    if !path.is_absolute() { return Err(denied()); }
    reject_reparse_ancestors(path)?;
    let parent_path = path.parent().ok_or_else(denied)?;
    let target_name = path.file_name().ok_or_else(denied)?;
    let root = open_authorized_root(parent_path, roots)?;
    let parent = open_destination_directory(parent_path)?;
    authorize_with_root(&parent, &root)?;
    let target = open_object_at(
        &parent, target_name,
        DELETE | FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY | FILE_TRAVERSE,
        FILE_SHARE_READ_WRITE_DELETE, FILE_OPEN, true,
    )?;
    authorize_with_root(&target, &root)?;
    let mut remaining = 50_000usize;
    remove_directory_contents(&target, &root, 0, &mut remaining)?;
    mark_object_deleted(&target)?;
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
fn object_identity(value: &FileIdentity) -> String {
    format!("windows:{:x}:{:x}", value.volume, value.index)
}

pub(super) fn directory_identity(path: &Path) -> io::Result<String> {
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    reject_reparse_ancestors(path)?;
    let directory = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ_WRITE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    if !directory.metadata()?.is_dir() { return Err(denied()); }
    verify_path_handle(path, &directory)?;
    let id = browseros_platform_security::windows::file_identity(&directory)?;
    Ok(format!("windows:{:x}:{:x}", id.volume_serial, id.file_index))
}

fn cas_lock(parent: &File, filename: &std::ffi::OsStr) -> Result<(File, File), String> {
    use sha2::Digest;
    use windows_sys::Wdk::Storage::FileSystem::FILE_OPEN_IF;
    const FILE_READ_DATA: u32 = 0x0000_0001;
    const FILE_WRITE_DATA: u32 = 0x0000_0002;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const FILE_SHARE_READ_WRITE: u32 = 0x0000_0003;
    // One native parent object owns the lock, independent of a target's case,
    // Unicode spelling or 8.3 alias. Do not invent Windows casing rules here.
    // Deny delete sharing so a held sidecar cannot be removed and recreated.
    let directory_lock = open_child_at(
        parent, std::ffi::OsStr::new(".browseros-cas-directory.lock"),
        FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ_WRITE, FILE_OPEN_IF,
    ).map_err(|_| "FILE_LOCK_UNAVAILABLE")?;
    directory_lock.lock().map_err(|_| "FILE_LOCK_UNAVAILABLE")?;
    // Keep the original exact-name sidecar as well, for older cooperating
    // callers using this spelling. Only upgraded callers share alias safety.
    let digest = format!("{:x}", sha2::Sha256::digest(filename.as_encoded_bytes()));
    let lock_name = std::ffi::OsString::from(format!(".browseros-cas-{digest}.lock"));
    let legacy_lock = open_child_at(
        parent, &lock_name, FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ_WRITE, FILE_OPEN_IF,
    ).map_err(|_| "FILE_LOCK_UNAVAILABLE")?;
    legacy_lock.lock().map_err(|_| "FILE_LOCK_UNAVAILABLE")?;
    Ok((directory_lock, legacy_lock))
}

fn snapshot_at(
    parent: &File, filename: &std::ffi::OsStr, limit: usize,
) -> Result<Option<(File, FileIdentity, Vec<u8>)>, String> {
    use windows_sys::Wdk::Storage::FileSystem::FILE_OPEN;
    const FILE_READ_DATA: u32 = 0x0000_0001;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const READ_CONTROL: u32 = 0x0002_0000;
    let mut file = match open_child_at(
        parent, filename, FILE_READ_DATA | FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ_WRITE_DELETE, FILE_OPEN,
    ) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("FILE_TARGET_NOT_REGULAR".into()),
    };
    let (identity, bytes) = read_snapshot(&mut file, limit)?;
    Ok(Some((file, identity, bytes)))
}

fn write_versioned_at(
    parent: &File, filename: &std::ffi::OsStr, expected: Option<&[u8]>,
    expected_identity: Option<&str>, content: &[u8], limit: usize,
    prepare: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    use windows_sys::Wdk::Storage::FileSystem::FILE_CREATE;
    const DELETE: u32 = 0x0001_0000;
    const FILE_WRITE_DATA: u32 = 0x0000_0002;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const READ_CONTROL: u32 = 0x0002_0000;
    const WRITE_DAC: u32 = 0x0004_0000;
    if limit == 0 || expected.is_some_and(|b| b.len() > limit) || content.len() > limit {
        return Err("FILE_SAVE_TOO_LARGE".into());
    }
    let initial = snapshot_at(parent, filename, limit)?;
    // Preserve the existing editor error for a removed target. Versioned
    // recovery still treats disappearance of an expected object as conflict.
    if initial.is_none() && expected.is_some() && expected_identity.is_none() {
        return Err("FILE_TARGET_MISSING".into());
    }
    if initial.as_ref().map(|(_, _, bytes)| bytes.as_slice()) != expected
        || expected_identity.is_some_and(|id| {
            initial.as_ref().is_none_or(|(_, identity, _)| object_identity(identity) != id)
        })
    {
        return Err("FILE_CONFLICT".into());
    }
    let original_identity = initial.as_ref().map(|(_, id, _)| FileIdentity {
        volume: id.volume, index: id.index, size: id.size, modified: id.modified,
    });
    drop(initial);
    let candidate = std::ffi::OsString::from(format!(".browseros-cas-{}.tmp", uuid::Uuid::new_v4()));
    let mut stage = open_child_at(
        parent, &candidate,
        DELETE | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES | READ_CONTROL | WRITE_DAC,
        0, FILE_CREATE,
    ).map_err(|_| "FILE_STAGE_CREATE_FAILED")?;
    let mut published = false;
    let result = (|| {
        use std::io::Write;
        stage.write_all(content).map_err(|_| "FILE_STAGE_WRITE_FAILED")?;
        stage.sync_all().map_err(|_| "FILE_STAGE_SYNC_FAILED")?;
        let receipt = object_identity(&identity(&stage).map_err(|_| "FILE_IDENTITY_UNAVAILABLE")?);
        prepare(&receipt)?;
        // Receipt persistence may itself use the same directory. Acquire the
        // commit lock afterwards, then revalidate under it before replacement.
        let _lock = cas_lock(parent, filename)?;
        let latest = snapshot_at(parent, filename, limit)?;
        if latest.as_ref().map(|(_, id, _)| id) != original_identity.as_ref()
            || latest.as_ref().map(|(_, _, bytes)| bytes.as_slice()) != expected
        {
            return Err("FILE_CONFLICT".into());
        }
        if let Some((file, _, _)) = &latest { copy_dacl(file, &stage)?; }
        drop(latest);
        rename_at(&stage, parent, filename, expected.is_some())
            .map_err(|e| if e.kind() == io::ErrorKind::AlreadyExists {
                "FILE_CONFLICT"
            } else { "FILE_REPLACE_FAILED" })?;
        published = true;
        let _ = parent.sync_all();
        Ok(())
    })();
    if result.is_err() && !published { delete_on_close(&stage); }
    result
}

fn save_if_unchanged_at(
    parent: &File, filename: &std::ffi::OsStr, expected: &[u8], content: &[u8], limit: usize,
) -> Result<(), String> {
    write_versioned_at(parent, filename, Some(expected), None, content, limit, |_| Ok(()))
}

fn authorized_parent(path: &Path, roots: &[PathBuf]) -> Result<File, String> {
    if !path.is_absolute() { return Err("FILE_PARENT_NOT_AUTHORIZED".into()); }
    let parent_path = path.parent().ok_or("FILE_PARENT_INVALID")?;
    let root = open_authorized_root(parent_path, roots).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
    let parent = open_destination_directory(parent_path).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
    authorize_with_root(&parent, &root).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
    Ok(parent)
}

pub(super) fn save_if_unchanged(
    path: &Path, roots: &[PathBuf], expected: &[u8], content: &[u8], limit: usize,
) -> Result<(), String> {
    let parent = authorized_parent(path, roots)?;
    let filename = path.file_name().ok_or("FILE_NAME_INVALID")?;
    save_if_unchanged_at(&parent, filename, expected, content, limit)
}

pub(super) fn read_versioned(
    path: &Path, roots: &[PathBuf], limit: usize,
) -> Result<Option<FileSnapshot>, String> {
    if !path.is_absolute() { return Err("FILE_PARENT_NOT_AUTHORIZED".into()); }
    let parent_path = path.parent().ok_or("FILE_PARENT_INVALID")?;
    let filename = path.file_name().ok_or("FILE_NAME_INVALID")?;
    let root = match open_authorized_root(parent_path, roots) {
        Ok(root) => root,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // Missing parents are not a reason to skip root authorization.
            let mut ancestor = parent_path;
            loop {
                match open_authorized_root(ancestor, roots) {
                    Ok(_) => return Ok(None),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        ancestor = ancestor.parent().ok_or("FILE_PARENT_NOT_AUTHORIZED")?;
                    }
                    Err(_) => return Err("FILE_PARENT_NOT_AUTHORIZED".into()),
                }
            }
        }
        Err(_) => return Err("FILE_PARENT_NOT_AUTHORIZED".into()),
    };
    let parent = match open_destination_directory(parent_path) {
        Ok(parent) => parent,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("FILE_PARENT_NOT_AUTHORIZED".into()),
    };
    authorize_with_root(&parent, &root).map_err(|_| "FILE_PARENT_NOT_AUTHORIZED")?;
    Ok(snapshot_at(&parent, filename, limit)?.map(|(_, identity, bytes)| FileSnapshot {
        identity: object_identity(&identity), bytes,
    }))
}

pub(super) fn write_versioned(
    path: &Path, roots: &[PathBuf], expected: Option<&FileSnapshot>,
    content: &[u8], limit: usize, prepare: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    let parent = authorized_parent(path, roots)?;
    let filename = path.file_name().ok_or("FILE_NAME_INVALID")?;
    write_versioned_at(
        &parent, filename, expected.map(|s| s.bytes.as_slice()),
        expected.map(|s| s.identity.as_str()), content, limit, prepare,
    )
}

pub(super) fn remove_if_unchanged(
    path: &Path, roots: &[PathBuf], expected: &FileSnapshot, limit: usize,
) -> Result<(), String> {
    use windows_sys::Wdk::Storage::FileSystem::FILE_OPEN;
    const DELETE: u32 = 0x0001_0000;
    const FILE_READ_DATA: u32 = 0x0000_0001;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    let parent = authorized_parent(path, roots)?;
    let filename = path.file_name().ok_or("FILE_NAME_INVALID")?;
    let _lock = cas_lock(&parent, filename)?;
    // No write/delete sharing: the same handle is checked and marked deleted.
    // Existing incompatible external writers make this operation fail closed.
    let mut target = open_child_at(
        &parent, filename, DELETE | FILE_READ_DATA | FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ, FILE_OPEN,
    ).map_err(|_| "FILE_CONFLICT")?;
    let (identity, bytes) = read_snapshot(&mut target, limit)?;
    if object_identity(&identity) != expected.identity || bytes != expected.bytes {
        return Err("FILE_CONFLICT".into());
    }
    mark_object_deleted(&target).map_err(|_| "FILE_DELETE_FAILED")
}

/// Atomic create/replace for Agent-owned drafts, new files and recovery
/// backups. Missing parent components are created relative to the nearest
/// verified existing ancestor, never through an unchecked junction path.
pub(super) fn atomic_write(path: &Path, bytes: &[u8], limit: usize) -> Result<(), String> {
    use std::io::Write;
    use windows_sys::Wdk::Storage::FileSystem::{FILE_CREATE, FILE_OPEN};
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
    let lock = cas_lock(&parent, filename).map_err(|_| "AGENT_LOCK_UNAVAILABLE")?;
    let result = (|| {
        let mut stage = None;
        for _ in 0..8 {
            let candidate =
                std::ffi::OsString::from(format!(".browseros-agent-{}.tmp", uuid::Uuid::new_v4()));
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
        let base =
            std::env::temp_dir().join(format!("browseros-windows-read-{}", uuid::Uuid::new_v4()));
        let safe = base.join("safe");
        let external = base.join("external");
        std::fs::create_dir_all(&safe).unwrap();
        std::fs::create_dir_all(&external).unwrap();
        let file = safe.join("one.txt");
        let outside = external.join("two.txt");
        std::fs::write(&file, b"inside").unwrap();
        std::fs::write(&outside, b"external").unwrap();
        assert_eq!(
            read_regular(&file, &[safe.clone()], 128).unwrap(),
            b"inside"
        );
        assert!(read_regular(&outside, &[safe.clone()], 128).is_err());
        assert!(read_regular(&file, &[safe.clone()], 2).is_err());
        std::fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn pinned_handle_move_is_no_replace_and_does_not_touch_external_root() {
        let base = std::env::temp_dir().join(format!("browseros-winmove-{}", uuid::Uuid::new_v4()));
        let allowed = base.join("allowed");
        let a = allowed.join("a");
        let b = allowed.join("b");
        let outside = base.join("outside");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let src = a.join("one.txt");
        std::fs::write(&src, b"first").unwrap();
        move_no_replace(&src, &b, &[allowed.clone()])
            .unwrap_or_else(|error| panic!("WINDOWS_PINNED_MOVE_FAILED: {error:?}"));
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
        let root = std::env::temp_dir().join(format!("browseros-wincas-{}", uuid::Uuid::new_v4()));
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
        assert!(!std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .any(|entry| { entry.file_name().to_string_lossy().ends_with(".tmp") }));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn windows_case_alias_writers_share_the_same_commit_lock() {
        use std::sync::{Arc, Barrier};
        let root = std::env::temp_dir().join(format!("browseros-wincas-alias-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        std::fs::write(root.join("Entry.txt"), b"before").unwrap();
        let gate = Arc::new(Barrier::new(3));
        let workers: Vec<_> = ["Entry.txt", "entry.txt"].into_iter().enumerate().map(|(index, name)| {
            let root = root.clone();
            let gate = Arc::clone(&gate);
            std::thread::spawn(move || {
                gate.wait();
                save_if_unchanged(&root.join(name), &[root.clone()], b"before", format!("writer-{index}").as_bytes(), 1024)
            })
        }).collect();
        gate.wait();
        let results: Vec<_> = workers.into_iter().map(|worker| worker.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.as_ref().err().is_some_and(|error| error == "FILE_CONFLICT")).count(), 1);
        let content = std::fs::read(root.join("Entry.txt")).unwrap();
        assert!(content == b"writer-0" || content == b"writer-1");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn receipt_persistence_in_the_same_parent_does_not_reenter_the_commit_lock() {
        let root = std::env::temp_dir().join(format!("browseros-wincas-receipt-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let target = root.join("entry.txt");
        std::fs::write(&target, b"before").unwrap();
        let expected = read_versioned(&target, &[root.clone()], 1024).unwrap().unwrap();
        let receipt = root.join("owner-receipt.json");
        write_versioned(&target, &[root.clone()], Some(&expected), b"after", 1024, |identity| {
            atomic_write(&receipt, identity.as_bytes(), 1024)
        }).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"after");
        assert_eq!(
            std::fs::read_to_string(&receipt).unwrap(),
            read_versioned(&target, &[root.clone()], 1024).unwrap().unwrap().identity,
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn windows_cas_pinned_parent_survives_junction_replacement() {
        let base =
            std::env::temp_dir().join(format!("browseros-wincas-pivot-{}", uuid::Uuid::new_v4()));
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
        let base =
            std::env::temp_dir().join(format!("browseros-winagent-write-{}", uuid::Uuid::new_v4()));
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
