use keyring::{Entry, Error};

/// 凭据隔离：token 只存系统密钥库（macOS Keychain / Windows Credential Manager /
/// Linux secret-service），绝不以任何形式进入网页 JS 的内存/网络/日志。
pub struct KeyringStore;

const SERVICE: &str = "com.jizhijiandan.mvp";

impl KeyringStore {
    pub fn save_token(repo_id: &str, token: &str) -> Result<(), String> {
        let entry = Entry::new(SERVICE, repo_id).map_err(|e: Error| e.to_string())?;
        entry.set_password(token).map_err(|e| e.to_string())
    }

    pub fn get_token(repo_id: &str) -> Result<String, String> {
        let entry = Entry::new(SERVICE, repo_id).map_err(|e: Error| e.to_string())?;
        entry
            .get_password()
            .map_err(|e| format!("凭据缺失: {e}（请重新配置仓库）"))
    }

    /// 原始 keyring 结果（调用方自行决定错误映射）。
    /// 用于需要区分 `NoEntry`（条目被外部删除）与密钥库不可读的场景。
    pub fn get_token_result(repo_id: &str) -> Result<String, keyring::Error> {
        let entry = Entry::new(SERVICE, repo_id)?;
        entry.get_password()
    }

    #[allow(dead_code)]
    pub fn delete_token(repo_id: &str) -> Result<(), String> {
        let entry = Entry::new(SERVICE, repo_id).map_err(|e: Error| e.to_string())?;
        entry.delete_credential().map_err(|e| e.to_string())
    }
}
