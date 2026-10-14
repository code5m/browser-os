//! Durable connection METADATA, never credentials.
//! Passwords are held exclusively in the OS keyring under db:<conn_id>.
//! File is user-only (0700 parent / 0600 file) and atomically replaced.
use crate::domain::DbConnectionConfig;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};
const MAX_BYTES: u64 = 256 * 1024;
const MAX_COUNT: usize = 64;

pub fn path_for(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "DB_PROFILE_LOCATION_FAILED")?
        .join("mvp-browser-os");
    Ok(dir.join("database-profiles.json"))
}
fn assert_regular_or_missing(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
            Err("DB_PROFILE_PATH_UNSAFE".into())
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("DB_PROFILE_METADATA_FAILED".into()),
    }
}
pub fn read_file(path: &Path) -> Result<Vec<DbConnectionConfig>, String> {
    assert_regular_or_missing(path)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let size = fs::metadata(path)
        .map_err(|_| "DB_PROFILE_READ_FAILED")?
        .len();
    if size > MAX_BYTES {
        return Err("DB_PROFILE_OVERSIZE".into());
    }
    let bytes = fs::read(path).map_err(|_| "DB_PROFILE_READ_FAILED")?;
    let configs: Vec<DbConnectionConfig> =
        serde_json::from_slice(&bytes).map_err(|_| "DB_PROFILE_INVALID_FORMAT")?;
    if configs.len() > MAX_COUNT {
        return Err("DB_PROFILE_OVER_LIMIT".into());
    }
    Ok(configs)
}
pub fn write_file(path: &Path, configs: &[DbConnectionConfig]) -> Result<(), String> {
    if configs.len() > MAX_COUNT {
        return Err("DB_PROFILE_OVER_LIMIT".into());
    }
    let bytes = serde_json::to_vec_pretty(configs).map_err(|_| "DB_PROFILE_ENCODE_FAILED")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("DB_PROFILE_OVERSIZE".into());
    }
    let dir = path.parent().ok_or("DB_PROFILE_PATH_UNSAFE")?;
    fs::create_dir_all(dir).map_err(|_| "DB_PROFILE_CREATE_DIR_FAILED")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
            .map_err(|_| "DB_PROFILE_PERMISSIONS_FAILED")?;
    }
    assert_regular_or_missing(path)?;
    let temp = dir.join(format!("database-profiles.{}.tmp", uuid::Uuid::new_v4()));
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let result = (|| {
        let mut f = opts.open(&temp).map_err(|_| "DB_PROFILE_TMP_FAILED")?;
        f.write_all(&bytes).map_err(|_| "DB_PROFILE_WRITE_FAILED")?;
        f.sync_all().map_err(|_| "DB_PROFILE_SYNC_FAILED")?;
        fs::rename(&temp, path).map_err(|_| "DB_PROFILE_RENAME_FAILED")?;
        Ok::<(), String>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_roundtrip_empty_without_credentials() {
        let root =
            std::env::temp_dir().join(format!("browseros-db-profile-{}", uuid::Uuid::new_v4()));
        let file = root.join("database-profiles.json");
        write_file(&file, &[]).unwrap();
        assert!(read_file(&file).unwrap().is_empty());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let _ = fs::remove_dir_all(&root);
    }
    #[test]
    fn profiles_roundtrip_actual_non_secret_data_source() {
        use crate::domain::{DbSslMode, SupportedDb};
        let root = std::env::temp_dir().join(format!(
            "browseros-db-profile-test-{}",
            uuid::Uuid::new_v4()
        ));
        let file = root.join("database-profiles.json");
        let now = chrono::Utc::now();
        let cfg = DbConnectionConfig {
            id: uuid::Uuid::new_v4().to_string(),
            name: "my IDE import".to_string(),
            kind: SupportedDb::MySql,
            host: Some("127.0.0.1".to_string()),
            port: Some(3306),
            database: "orders".to_string(),
            username: Some("reader".to_string()),
            ssl_mode: DbSslMode::Prefer,
            allow_write: false,
            production_hint: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        write_file(&file, &[cfg.clone()]).unwrap();
        let restored = read_file(&file).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].name, cfg.name);
        assert_eq!(restored[0].database, "orders");
        assert!(!restored[0].allow_write);
        let raw = fs::read_to_string(&file).unwrap();
        assert!(!raw.contains("password"));
        assert!(!raw.contains("connection_string"));
        assert!(!raw.contains("dsn"));
        let _ = fs::remove_dir_all(&root);
    }
    #[cfg(unix)]
    #[test]
    fn profile_symlink_to_other_file_rejected() {
        let root = std::env::temp_dir().join(format!(
            "browseros-db-profile-link-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let actual = root.join("actual.json");
        fs::write(&actual, "[]").unwrap();
        let target = root.join("database-profiles.json");
        std::os::unix::fs::symlink(&actual, &target).unwrap();
        assert!(write_file(&target, &[]).is_err());
        assert!(read_file(&target).is_err());
        assert_eq!(fs::read_to_string(&actual).unwrap(), "[]");
        let _ = fs::remove_dir_all(&root);
    }
}
