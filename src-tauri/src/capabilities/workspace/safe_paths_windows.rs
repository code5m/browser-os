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

#[cfg(test)]
mod windows_identity_contract_tests {
    use super::{acl_listing_is_exclusive, valid_windows_sid};

    const OWNER_SID: &str = "S-1-5-21-1697530992-2138327965-217515802-1001";
    const OWNER_ACCOUNT: &str = r"yanghuiqian\yhq";

    #[test]
    fn sid_validation_preserves_literal_s_prefix_and_rejects_malformed_segments() {
        assert!(valid_windows_sid("S-1-5-21-100-200-300-1001"));
        assert!(!valid_windows_sid("1-5-21-100"));
        assert!(!valid_windows_sid("S-1--21"));
        assert!(!valid_windows_sid("S-1-5-21-abc"));
        assert!(!valid_windows_sid("S-1-"));
    }

    /// Verbatim shape captured from `icacls <appdata> /T` on the machine that
    /// hit the startup panic: lowercase `whoami` account vs mixed-case
    /// `YANGHUIQIAN\Yhq` display, inherited `(I)` flags on children, and the
    /// localized footer. Two objects -> four ACEs.
    #[test]
    fn acl_exclusivity_accepts_real_icacls_output_mixed_case_and_localized_footer() {
        let listing = concat!(
            "C:/Users/10146/AppData/Roaming/app NT AUTHORITY\\SYSTEM:(OI)(CI)(F)\n",
            "                                          YANGHUIQIAN\\Yhq:(OI)(CI)(F)\n",
            "\n",
            "C:/Users/10146/AppData/Roaming/app\\sessions NT AUTHORITY\\SYSTEM:(I)(OI)(CI)(F)\n",
            "                                                     YANGHUIQIAN\\Yhq:(I)(OI)(CI)(F)\n",
            "\n已成功处理 2 个文件; 处理 0 个文件时失败\n",
        );
        assert!(
            acl_listing_is_exclusive(listing, 2, OWNER_SID, OWNER_ACCOUNT),
            "real mixed-case / localized output must be accepted as exclusive"
        );
    }

    /// P1: account names may contain non-ASCII characters. Folding must use
    /// Unicode-aware `to_lowercase()`, not `to_ascii_lowercase()`.
    #[test]
    fn acl_exclusivity_folds_non_ascii_account_names() {
        let listing = concat!(
            "C:\\data\\app NT AUTHORITY\\SYSTEM:(OI)(CI)(F)\n",
            "            CONTOSO\\JÖRG:(OI)(CI)(F)\n",
        );
        assert!(
            acl_listing_is_exclusive(listing, 1, OWNER_SID, "contoso\\jörg"),
            "non-ASCII account case differences must fold to the same key"
        );
    }

    /// P2: a trustee whose name merely contains "SYSTEM" must never be counted
    /// as the SYSTEM trustee (the old code used `contains("SYSTEM")`).
    #[test]
    fn acl_exclusivity_rejects_system_name_substring_impostor() {
        let impostor = concat!(
            "C:\\data\\app CONTOSO\\SystemAdmins:(OI)(CI)(F)\n",
            "            YANGHUIQIAN\\Yhq:(OI)(CI)(F)\n",
        );
        assert!(
            !acl_listing_is_exclusive(impostor, 1, OWNER_SID, OWNER_ACCOUNT),
            "a lookalike trustee must not satisfy the SYSTEM requirement"
        );
    }

    /// P3: broad groups must be rejected whether they appear as English names,
    /// localized names, or SIDs — without a locale-dependent name blacklist.
    #[test]
    fn acl_exclusivity_rejects_broad_groups_by_name_and_sid() {
        let broad_trustees = [
            "Everyone",
            "每个人",                              // localized Everyone
            "NT AUTHORITY\\Authenticated Users",
            "BUILTIN\\Users",
            "BUILTIN\\Administrators",
            "*S-1-1-0",                            // Everyone SID
            "S-1-5-11",                            // Authenticated Users SID
            "S-1-5-32-544",                        // Administrators SID
            "S-1-5-32-545",                        // Users SID
        ];
        for trustee in broad_trustees {
            let listing = format!(
                "C:\\data\\app NT AUTHORITY\\SYSTEM:(OI)(CI)(F)\n  {trustee}:(OI)(CI)(F)\n  YANGHUIQIAN\\Yhq:(OI)(CI)(F)\n"
            );
            assert!(
                !acl_listing_is_exclusive(&listing, 1, OWNER_SID, OWNER_ACCOUNT),
                "broad trustee must be rejected: {trustee}"
            );
        }
    }

    #[test]
    fn acl_exclusivity_accepts_sid_form_owner_and_system() {
        // When name resolution fails, icacls prints raw SIDs instead of names.
        let listing = format!(
            "C:\\data\\app *S-1-5-18:(OI)(CI)(F)\n  *{OWNER_SID}:(OI)(CI)(F)\n"
        );
        assert!(acl_listing_is_exclusive(&listing, 1, OWNER_SID, OWNER_ACCOUNT));
    }

    #[test]
    fn acl_exclusivity_rejects_wrong_ace_count() {
        let one_object = concat!(
            "C:\\data\\app NT AUTHORITY\\SYSTEM:(OI)(CI)(F)\n",
            "            YANGHUIQIAN\\Yhq:(OI)(CI)(F)\n",
        );
        // Claim two objects but supply one object's worth of ACEs.
        assert!(!acl_listing_is_exclusive(one_object, 2, OWNER_SID, OWNER_ACCOUNT));
    }
}

fn valid_windows_sid(value: &str) -> bool {
    value.strip_prefix("S-1-").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.split('-').all(|part| {
            !part.is_empty() && part.bytes().all(|ch| ch.is_ascii_digit())
        })
    })
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
    // The Windows account display name may contain quoted commas. Parse
    // from the final CSV separator, never tokenize the account field as SID.
    let (account_raw, sid_raw) = line.trim_start_matches('\u{feff}')
        .rsplit_once(',')
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::PermissionDenied, "WINDOWS_USER_SID_UNAVAILABLE"
        ))?;
    let account = account_raw.trim().trim_matches('"');
    if account.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied, "WINDOWS_USER_ACCOUNT_UNAVAILABLE"
        ));
    }
    let sid = sid_raw.trim().trim_matches('"');
    if !valid_windows_sid(sid) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied, "WINDOWS_USER_SID_INVALID"
        ));
    }
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
    let object_count = objects.len() + 1;
    if !acl_listing_is_exclusive(&listing, object_count, &sid, &account) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "WINDOWS_PRIVATE_DIR_ACL_NOT_EXCLUSIVE",
        ));
    }
    Ok(())
}

/// Verify the `icacls /T` listing grants exactly the current user and SYSTEM
/// on every object, with no other trustee.
///
/// Two properties of the producer make string matching non-trivial:
///   1. `icacls` prints trustees as display names (`DOMAIN\\User`,
///      `NT AUTHORITY\\SYSTEM`) and resolves SIDs when it can. Windows account
///      names are case-insensitive, but `whoami /user` and `icacls` disagree on
///      casing (`yanghuiqian\\yhq` vs `YANGHUIQIAN\\Yhq`). A case-sensitive match
///      reports zero owner entries and rejects an otherwise private directory.
///      Fold with Unicode-aware `to_lowercase()` (not `to_ascii_lowercase()`,
///      which silently misses non-ASCII account names).
///   2. The summary/footer line is localized ("已成功处理 N 个文件",
///      "N fichiers traités", ...), so group names in the listing are locale
///      dependent too.
///
/// Instead of blacklisting broad groups by (localized) name — which can never
/// be exhaustive — classify *every* ACE: its trustee must be the owner or
/// SYSTEM. Any other trustee (Everyone, Authenticated Users, BUILTIN\\Users,
/// or their localized equivalents) falls through and fails closed. This keeps
/// the check strict on any Windows locale without a name blacklist.
fn acl_listing_is_exclusive(listing: &str, object_count: usize, sid: &str, account: &str) -> bool {
    let owner_key = account.to_lowercase();
    let owner_sid_key = sid.to_lowercase();
    // SYSTEM's well-known SID, and the SDDL display name icacls resolves it to.
    // Both are locale-independent; the SID is the authoritative form.
    const SYSTEM_SID: &str = "s-1-5-18";
    const SYSTEM_NAME: &str = "nt authority\\system";

    let mut ace_lines = 0usize;
    let mut owner_entries = 0usize;
    let mut system_entries = 0usize;

    for line in listing.lines() {
        // ACE lines carry the rights marker ":(..."; blank lines and the
        // localized footer do not. The trustee is the text before that marker
        // (the first line of each object also carries its path, hence the
        // suffix match below rather than an exact equality).
        let Some(rights_at) = line.find(":(") else { continue; };
        ace_lines += 1;
        let trustee = line[..rights_at].trim().to_lowercase();
        if trustee.ends_with(owner_key.as_str()) || trustee.ends_with(owner_sid_key.as_str()) {
            owner_entries += 1;
        } else if trustee.ends_with(SYSTEM_SID) || trustee.ends_with(SYSTEM_NAME) {
            system_entries += 1;
        } else {
            // Unknown trustee: fail closed rather than maintain a
            // locale-dependent broad-group blacklist.
            return false;
        }
    }

    ace_lines == object_count * 2
        && owner_entries == object_count
        && system_entries == object_count
}

fn normalized_path(value: &str) -> String {
    // Keep UNC roots absolute: \\?\UNC\server\share is the same location
    // as \\server\share. Stripping \\?\ blindly produced a relative-looking
    // "UNC\server" path that could disagree with the opened NT handle.
    let folded = value.replace('/', "\\").to_ascii_lowercase();
    let path = if let Some(unc) = folded.strip_prefix(r"\\?\unc\") {
        format!(r"\\{unc}")
    } else if let Some(rest) = folded.strip_prefix(r"\\?\") {
        // Strip the Win32 verbatim prefix only from an actual drive path.
        if rest.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            && rest.as_bytes().get(1) == Some(&b':')
            && rest.as_bytes().get(2) == Some(&b'\\')
        {
            rest.to_owned()
        } else {
            folded
        }
    } else {
        folded
    };
    if path.len() == 3 && path.as_bytes().get(1) == Some(&b':') && path.ends_with('\\') {
        path
    } else {
        path.trim_end_matches('\\').to_owned()
    }
}

#[cfg(test)]
#[test]
fn windows_path_normalizer_preserves_drive_and_unc_root_semantics() {
    assert_eq!(normalized_path(r"\\?\C:\Workspace\FILE.rs"),
        normalized_path(r"c:\workspace\file.rs"));
    assert_eq!(normalized_path(r"\\?\UNC\SERVER\Share\Dir\file.rs"),
        normalized_path(r"\\server\share\dir\file.rs"));
    assert_eq!(normalized_path(r"C:\"), r"c:\");
    assert_eq!(normalized_path(r"\\?\Volume{1234}\dir"),
        r"\\?\volume{1234}\dir");
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
    if actual == root_path || actual.starts_with(&format!("{}\\", root_path.trim_end_matches('\\'))) {
        return Ok(actual);
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

fn authorize_named_child(file: &File, parent: &File, filename: &std::ffi::OsStr) -> io::Result<()> {
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
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
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
            ReplaceIfExists: true,
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
            let candidate =
                std::ffi::OsString::from(format!(".browseros-cas-{}.tmp", uuid::Uuid::new_v4()));
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
