//! Durable connection METADATA, never credentials.
//! Passwords are held exclusively in the OS keyring under immutable db:<conn_id>:<version> references; legacy keys remain readable.
//! File is user-only (0700 parent / 0600 file) and atomically replaced.
use crate::domain::DbConnectionConfig;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};
const MAX_BYTES: u64 = 256 * 1024;
const MAX_COUNT: usize = 64;
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StoredProfile {
    #[serde(flatten)]
    pub config: DbConnectionConfig,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub credential_ref: String,
}
impl StoredProfile {
    pub fn reference(&self) -> Result<String, String> {
        let legacy = crate::database::credential_key(&self.config.id);
        if self.credential_ref.is_empty() || self.credential_ref == legacy { return Ok(legacy); }
        let version = self.credential_ref.strip_prefix(&format!("{legacy}:"))
            .ok_or("DB_CREDENTIAL_REF_INVALID")?;
        uuid::Uuid::parse_str(version).map_err(|_| "DB_CREDENTIAL_REF_INVALID")?;
        Ok(self.credential_ref.clone())
    }
}

pub fn path_for(app: &AppHandle) -> Result<PathBuf, String> {
    // The shared workspace data_dir enforces the Windows owner/SYSTEM ACL
    // before any profile metadata is read or replaced.
    Ok(crate::workspace::data_dir(app).join("database-profiles.json"))
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
pub(crate) fn read_records(path: &Path) -> Result<Vec<StoredProfile>, String> {
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
    let configs: Vec<StoredProfile> =
        serde_json::from_slice(&bytes).map_err(|_| "DB_PROFILE_INVALID_FORMAT")?;
    if configs.len() > MAX_COUNT {
        return Err("DB_PROFILE_OVER_LIMIT".into());
    }
    for profile in &configs { profile.reference()?; }
    Ok(configs)
}
pub(crate) fn write_records(path: &Path, configs: &[StoredProfile]) -> Result<(), String> {
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
        crate::session::atomic_replace_file(&temp, path)
            .map_err(|_| "DB_PROFILE_RENAME_FAILED")?;
        #[cfg(unix)]
        fs::File::open(dir).and_then(|directory| directory.sync_all())
            .map_err(|_| "DB_PROFILE_SYNC_FAILED")?;
        Ok::<(), String>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
#[cfg(test)]
pub(crate) fn read_file(path: &Path) -> Result<Vec<DbConnectionConfig>, String> {
    Ok(read_records(path)?.into_iter().map(|record| record.config).collect())
}
#[cfg(test)]
pub(crate) fn write_file(path: &Path, configs: &[DbConnectionConfig]) -> Result<(), String> {
    let previous = read_records(path)?;
    let records = configs.iter().map(|config| StoredProfile {
        config: config.clone(),
        credential_ref: previous.iter().find(|p| p.config.id == config.id)
            .map(|p| p.credential_ref.clone()).unwrap_or_default(),
    }).collect::<Vec<_>>();
    write_records(path, &records)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_metadata_and_immutable_credential_versions_roundtrip() {
        let id = uuid::Uuid::new_v4().to_string();
        let value = serde_json::json!({"id":id,"name":"fixture","kind":"mysql","host":"example.invalid",
            "port":3306,"database":"fixture","username":"reader","ssl_mode":"require","allow_write":false,
            "production_hint":true,"enabled":true,"created_at":"2026-11-22T00:00:00Z","updated_at":"2026-11-22T00:00:00Z"});
        let mut record: StoredProfile = serde_json::from_value(value).unwrap();
        assert_eq!(record.reference().unwrap(), format!("db:{id}"));
        let root = std::env::temp_dir().join(format!("browseros-db-version-{}", uuid::Uuid::new_v4()));
        let path = root.join("profiles.json");
        record.credential_ref = format!("db:{id}:{}", uuid::Uuid::new_v4());
        write_records(&path, &[record.clone()]).unwrap();
        let restored = read_records(&path).unwrap();
        assert_eq!(restored[0].reference().unwrap(), record.credential_ref);
        assert!(!fs::read_to_string(&path).unwrap().contains("password"));
        record.credential_ref = format!("db:{}:{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        assert!(record.reference().is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn profiles_roundtrip_empty_without_credentials() {
        let root =
            std::env::temp_dir().join(format!("browseros-db-profile-{}", uuid::Uuid::new_v4()));
        let file = root.join("database-profiles.json");
        write_file(&file, &[]).unwrap();
        assert!(read_file(&file).unwrap().is_empty());
        // Windows std::fs::rename cannot overwrite an existing destination;
        // the second save must still replace atomically.
        write_file(&file, &[]).expect("repeat profile write must replace");
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
