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

// ---------------------------------------------------------------------------
// M1-5 Git 只读能力 DTO（status / diff / branch_list）
// 全部字段只描述「仓库当前状态」，不含任何凭据，也不承载任何写操作语义。
// ---------------------------------------------------------------------------

/// 单文件状态。`status` 取值：
/// `modified` / `added` / `deleted` / `renamed` / `untracked` / `conflicted`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

/// 单文件 diff。内容超限时 `truncated=true`，`new_content` 为截断后的片段；
/// 二进制文件无法生成文本补丁时 `binary=true` 且两侧内容为 None。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffHunk {
    pub file: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub truncated: bool,
    pub binary: bool,
}

/// 分支信息（本地 + 远程跟踪），`is_head` 标当前检出分支。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub is_remote: bool,
    pub is_head: bool,
}

/// diff 结果：`more=true` 表示因总大小触顶提前停止收集（还有未返回的文件）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffResult {
    pub hunks: Vec<GitDiffHunk>,
    pub more: bool,
}

// ---------------------------------------------------------------------------
// M1-6.b Git 写能力（双阶段确认闸门）
//
// 白名单只有六个操作；任何其他操作串（reset/push/merge/rebase/stash/clean…）
// 在 request 阶段即被拒绝（fail-closed，零写入）。
// 结构体只承载操作语义与计数，绝不包含 token/凭据/完整 diff。
// ---------------------------------------------------------------------------

/// Git 写操作白名单（任务书 M1-6.b ALLOW_IMPLEMENT 冻结六项）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitWriteOp {
    Stage,
    Unstage,
    Discard,
    Commit,
    CreateBranch,
    CheckoutBranch,
}

impl GitWriteOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            GitWriteOp::Stage => "stage",
            GitWriteOp::Unstage => "unstage",
            GitWriteOp::Discard => "discard",
            GitWriteOp::Commit => "commit",
            GitWriteOp::CreateBranch => "create_branch",
            GitWriteOp::CheckoutBranch => "checkout_branch",
        }
    }

    /// 白名单解析：非白名单操作一律 None（调用方据此返回「操作禁止」）。
    pub fn from_op_str(s: &str) -> Option<GitWriteOp> {
        match s {
            "stage" => Some(GitWriteOp::Stage),
            "unstage" => Some(GitWriteOp::Unstage),
            "discard" => Some(GitWriteOp::Discard),
            "commit" => Some(GitWriteOp::Commit),
            "create_branch" => Some(GitWriteOp::CreateBranch),
            "checkout_branch" => Some(GitWriteOp::CheckoutBranch),
            _ => None,
        }
    }

    /// 危险操作需二次确认：discard 不可逆地用索引内容覆盖工作区文件。
    pub fn is_dangerous(&self) -> bool {
        matches!(self, GitWriteOp::Discard)
    }
}

/// Git 写任务状态机：Pending → Running → Success/Failed。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GitWriteStatus {
    Pending,
    Running,
    Success,
    Failed,
}

/// Git 写任务。与 `SyncJob` 完全独立：SyncJob 语义属成果推送，
/// 混用会污染既有 request_sync/confirm_sync 流程，因此单独建模。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitWriteJob {
    pub id: String,
    pub repo_id: String,
    pub op: GitWriteOp,
    /// 已校验的仓库内相对路径（stage/unstage/discard 必填；commit 空 = 全量）
    pub paths: Vec<String>,
    /// commit 的提交信息（已通过校验：非空/限长/无控制字符）
    pub message: Option<String>,
    /// create_branch / checkout_branch 的目标分支名（已通过校验）
    pub branch: Option<String>,
    /// create_branch 是否同时检出
    pub checkout: bool,
    pub status: GitWriteStatus,
    pub dangerous: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// 阶段一返回给前端的预览：只含摘要、计数与截断后的路径列表（≤20 条），
/// 不含 diff 内容、不含任何凭据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitWritePreview {
    pub job_id: String,
    pub repo_id: String,
    pub op: GitWriteOp,
    pub summary: String,
    pub affected_paths: Vec<String>,
    pub path_count: usize,
    pub dangerous: bool,
    pub expires_at: DateTime<Utc>,
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

    // T-gw-1（b 卡）：白名单外操作（reset/push/merge/rebase/stash/clean 等任意形式）
    // 必须在 request 阶段即被判定为 None → Err("操作禁止")，零写入。
    #[test]
    fn git_write_op_whitelist_only_six_ops() {
        for ok in [
            "stage",
            "unstage",
            "discard",
            "commit",
            "create_branch",
            "checkout_branch",
        ] {
            assert!(GitWriteOp::from_op_str(ok).is_some(), "{ok} 应在白名单内");
        }
        for bad in [
            "reset",
            "reset --hard",
            "push",
            "push --force",
            "pull",
            "fetch",
            "merge",
            "rebase",
            "stash",
            "clean",
            "revert",
            "cherry-pick",
            "branch -D",
            "checkout .",
            "",
            "STAGE",
            " stage",
        ] {
            assert!(
                GitWriteOp::from_op_str(bad).is_none(),
                "白名单外操作必须被拒绝: {bad:?}"
            );
        }
        // 危险标记：仅 discard 需要二次确认
        assert!(GitWriteOp::Discard.is_dangerous());
        assert!(!GitWriteOp::Commit.is_dangerous());
        assert!(!GitWriteOp::CheckoutBranch.is_dangerous());
    }
}
