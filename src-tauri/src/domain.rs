use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 本地成果：网页采集而来，带溯源信息（来源 URL + 内容哈希）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub title: String,
    pub source_url: String,
    pub text: String,
    pub html: String,
    pub hash: String,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl Artifact {
    pub fn new(title: String, source_url: String, text: String, html: String) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hasher.update(source_url.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Artifact {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            source_url,
            text,
            html,
            hash,
            created_at: Utc::now(),
            tags: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoProvider {
    Git,
    Gitee,
}

/// 仓库配置（注意：不含 token，凭据隔离）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub id: String,
    pub provider: RepoProvider,
    pub name: String,
    pub remote_url: String,
    pub branch: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Pending,
    Confirmed,
    Running,
    Success,
    Failed,
}

/// 同步任务：核心聚合。推送前必须处于 Pending 并经确认
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub id: String,
    pub repo_id: String,
    pub status: SyncStatus,
    pub artifact_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// 推送到前端前的预览（确认闸门用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPreview {
    pub job_id: String,
    pub repo_id: String,
    pub repo_name: String,
    pub artifact_count: usize,
    pub artifact_titles: Vec<String>,
    pub remote_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub at: DateTime<Utc>,
    pub action: String,
    pub detail: String,
}

/// 收藏项（M1-2）。一条记录 = 一个 URL + 标题 + 分类。
/// 持久化为 data_dir/bookmarks.json，重启不丢。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub category: String,
    pub created_at: DateTime<Utc>,
}

impl Bookmark {
    pub fn new(url: String, title: String, category: String) -> Self {
        Bookmark {
            id: uuid::Uuid::new_v4().to_string(),
            url,
            title,
            category,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmark_roundtrip_serde() {
        let bm = Bookmark::new(
            "https://example.com/a".into(),
            "Example".into(),
            "tech".into(),
        );
        let json = serde_json::to_string(&bm).expect("serialize");
        let back: Bookmark = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, bm.id);
        assert_eq!(back.url, "https://example.com/a");
        assert_eq!(back.title, "Example");
        assert_eq!(back.category, "tech");
        // created_at 应可被 serde 反序列化
        assert!(back.created_at.timestamp() > 0);
    }

    #[test]
    fn bookmark_id_is_uuid_v4() {
        let bm = Bookmark::new("u".into(), "t".into(), "c".into());
        let parsed = uuid::Uuid::parse_str(&bm.id).expect("uuid parse");
        assert_eq!(parsed.get_version_num(), 4);
    }
}
