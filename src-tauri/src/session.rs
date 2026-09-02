//! M1-9 浏览器会话存档：落盘、读取、容量与脱敏（纯函数层）
//!
//! 设计原则：
//!   1. **不依赖 `AppHandle`**：所有函数显式接收会话目录 `dir`，便于在临时目录里做
//!      真实的读写单测（而不是只测内存结构）。
//!   2. **原子写**：`write(.tmp)` → `rename`，保证异常退出（崩溃/SIGKILL）不会留下
//!      半截 JSON；启动时的 `prune_tmp_files` 负责清理残留 `.tmp`。
//!   3. **隐私红线**：入参 URL 一律经 `security_policy::redact_sensitive_url` 脱敏；
//!      预览文本中的 http(s) URL 同样逐个脱敏；结构上不承载任何
//!      headers / Cookie / Authorization / body 字段。
//!   4. **容量上限**：单会话资源 ≤ `SESSION_MAX_RESOURCES`，会话总数 ≤ `SESSION_MAX_COUNT`
//!      （超出按 `updated_at` 从旧到新删除）。

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::domain::{
    BrowserSession, ResourceReceived, SessionSummary, SESSION_MAX_RESOURCES,
    SESSION_PREVIEW_MAX_BYTES, SESSION_TITLE_MAX_BYTES,
};

/// 单会话存档文件名。
pub fn session_file(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

/// 原子写：先写同目录 `.tmp`，再 rename（同分区 rename 是原子的）。
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建会话目录失败: {e}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content).map_err(|e| format!("写临时文件失败: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("会话落盘 rename 失败: {e}"))?;
    Ok(())
}

/// 按字节截断（回退 UTF-8 字符边界），返回 (文本, 是否被截断)。
pub fn truncate_bytes(s: &str, max: usize) -> (String, bool) {
    if s.len() <= max {
        return (s.to_string(), false);
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    (s[..end].to_string(), true)
}

/// 把文本中出现的 http(s) URL 逐个脱敏（凭据查询参数 → `***`）。
/// 手写扫描而非引入正则依赖：从 `http` 起，到空白/引号/结尾为止视为一个 URL。
pub fn redact_urls_in_text(text: &str) -> String {
    const STOP: [char; 6] = [' ', '\t', '\n', '\r', '"', '\''];
    let mut out = String::with_capacity(text.len());
    // 按**字节**下标推进、每次前进一个完整字符：多字节前缀（中文正文）下
    // 用 char 下标去切 &str 会错位甚至 panic，历史上正是这类 bug 让 URL 漏检。
    let mut i = 0usize;
    while i < text.len() {
        let mut chars = text[i..].chars();
        let ch = match chars.next() {
            Some(c) => c,
            None => break,
        };
        if (ch == 'h' || ch == 'H')
            && (text[i..].starts_with("http://") || text[i..].starts_with("https://"))
        {
            let mut j = i;
            while j < text.len() {
                let c = text[j..].chars().next().unwrap();
                if STOP.contains(&c) {
                    break;
                }
                j += c.len_utf8();
            }
            out.push_str(&crate::security_policy::redact_sensitive_url(&text[i..j]));
            i = j;
        } else {
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// 预览文本处理：URL 脱敏 → 截断到 `SESSION_PREVIEW_MAX_BYTES`。
pub fn scrub_preview(text: &str) -> (String, bool) {
    let redacted = redact_urls_in_text(text);
    truncate_bytes(&redacted, SESSION_PREVIEW_MAX_BYTES)
}

/// 构建会话：URL 脱敏 + 标题截断 + 预览脱敏截断 + 资源裁剪（保留最新 N 条）。
/// `resource_count` 记录裁剪前的真实总数（不伪造，也不因裁剪而丢失计数语义）。
pub fn build_session(
    tab_id: &str,
    url: &str,
    title: &str,
    preview: &str,
    resources: &[ResourceReceived],
    close_reason: &str,
) -> BrowserSession {
    let now = Utc::now();
    let (title_text, _) = truncate_bytes(title, SESSION_TITLE_MAX_BYTES);
    let (preview_text, preview_truncated) = scrub_preview(preview);
    let total = resources.len();
    let kept: Vec<ResourceReceived> = if total > SESSION_MAX_RESOURCES {
        resources[total - SESSION_MAX_RESOURCES..].to_vec()
    } else {
        resources.to_vec()
    };
    BrowserSession {
        id: uuid::Uuid::new_v4().to_string(),
        tab_id: tab_id.to_string(),
        url: crate::security_policy::redact_sensitive_url(url),
        title: title_text,
        preview: preview_text,
        preview_truncated,
        resource_count: total,
        resources: kept,
        saved: true,
        close_reason: close_reason.to_string(),
        created_at: now,
        updated_at: now,
    }
}

/// 会话 → 列表项（丢弃 resources 全量，避免列表 payload 膨胀）。
pub fn summarize(s: &BrowserSession) -> SessionSummary {
    SessionSummary {
        id: s.id.clone(),
        tab_id: s.tab_id.clone(),
        url: s.url.clone(),
        title: s.title.clone(),
        preview: s.preview.clone(),
        preview_truncated: s.preview_truncated,
        resource_count: s.resource_count,
        saved: s.saved,
        close_reason: s.close_reason.clone(),
        created_at: s.created_at,
        updated_at: s.updated_at,
    }
}

/// 落盘一个会话（原子写）。返回文件路径。
pub fn save_session(dir: &Path, session: &BrowserSession) -> Result<PathBuf, String> {
    let content = serde_json::to_string_pretty(session).map_err(|e| format!("序列化失败: {e}"))?;
    let path = session_file(dir, &session.id);
    atomic_write(&path, &content)?;
    Ok(path)
}

/// 读取单个会话。文件不存在 / JSON 损坏 / 结构不符 → Err（调用方决定是否审计）。
pub fn load_session(dir: &Path, id: &str) -> Result<BrowserSession, String> {
    let path = session_file(dir, id);
    let content = fs::read_to_string(&path).map_err(|e| format!("会话不存在或不可读: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("会话解析失败: {e}"))
}

/// 列出全部会话（按 updated_at 倒序）。损坏文件**跳过**而非整体失败：
/// 单个坏文件不得让「历史会话」面板整体不可用。
pub fn list_sessions(dir: &Path) -> Vec<SessionSummary> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut sessions: Vec<BrowserSession> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .filter_map(|p| fs::read_to_string(&p).ok())
        .filter_map(|c| serde_json::from_str::<BrowserSession>(&c).ok())
        .collect();
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    sessions.iter().map(summarize).collect()
}

/// 删除会话。幂等：不存在时返回 Ok(false)。
pub fn delete_session(dir: &Path, id: &str) -> Result<bool, String> {
    let path = session_file(dir, id);
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path).map_err(|e| format!("删除会话失败: {e}"))?;
    Ok(true)
}

/// 容量上限：超出 `keep` 时按 updated_at 从旧到新删除，返回删除条数。
pub fn prune_sessions(dir: &Path, keep: usize) -> usize {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    let mut items: Vec<(PathBuf, chrono::DateTime<Utc>)> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .filter_map(|p| {
            let content = fs::read_to_string(&p).ok()?;
            let s = serde_json::from_str::<BrowserSession>(&content).ok()?;
            Some((p, s.updated_at))
        })
        .collect();
    if items.len() <= keep {
        return 0;
    }
    items.sort_by(|a, b| a.1.cmp(&b.1)); // 旧 → 新
    let remove_count = items.len() - keep;
    let mut removed = 0;
    for (path, _) in items.into_iter().take(remove_count) {
        if fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// 异常退出（崩溃/SIGKILL）后清理残留 `.tmp`：原子写的中间产物。
/// 返回清理数量。JSON 半文件不存在（rename 原子），因此这里只处理 tmp。
pub fn prune_tmp_files(dir: &Path) -> usize {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    let mut removed = 0;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.ends_with(".json.tmp") && fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// 会话目录内已存会话数量（诊断/统计用）。
pub fn count_sessions(dir: &Path) -> usize {
    list_sessions(dir).len()
}

#[cfg(test)]
mod session_persistence_tests {
    use super::*;
    use crate::domain::{
        ResourceKind, CLOSE_REASON_SAVED, CLOSE_REASON_SHUTDOWN, SESSION_MAX_COUNT,
        SESSION_MAX_RESOURCES,
    };

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "m1-9-{}-{}-{}",
            tag,
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建临时会话目录");
        dir
    }

    fn rec(id: &str) -> ResourceReceived {
        ResourceReceived {
            id: id.to_string(),
            tab_id: "tab-1".to_string(),
            url: format!("https://example.com/{id}"),
            method: "GET".to_string(),
            status: Some(200),
            mime: Some("text/html".to_string()),
            size_bytes: Some(10),
            started_at: 1,
            finished_at: Some(2),
            duration_ms: Some(1),
            resource_type: ResourceKind::Document,
        }
    }

    // T-sp-1：持久化字段白/黑名单——落盘 JSON 不得含任何凭据/body/headers
    #[test]
    fn persisted_json_has_no_sensitive_fields() {
        let dir = temp_dir("whitelist");
        let s = build_session(
            "tab-1",
            "https://ex.com/a?token=abc123",
            "标题",
            "预览文本",
            &[rec("r1")],
            CLOSE_REASON_SAVED,
        );
        let path = save_session(&dir, &s).expect("落盘");
        let text = fs::read_to_string(path).expect("读回");
        for bad in [
            "cookie",
            "Cookie",
            "authorization",
            "Authorization",
            "set-cookie",
            "Set-Cookie",
            "headers",
            "body",
            "abc123",
        ] {
            assert!(!text.contains(bad), "落盘不得包含 {bad}: {text}");
        }
        for key in [
            "\"id\"",
            "\"tab_id\"",
            "\"url\"",
            "\"title\"",
            "\"preview\"",
            "\"resource_count\"",
            "\"resources\"",
            "\"saved\"",
            "\"close_reason\"",
        ] {
            assert!(text.contains(key), "落盘应包含字段 {key}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    // T-sp-2：URL 落盘前脱敏（敏感查询参数值 → ***）
    #[test]
    fn url_is_redacted_before_persist() {
        let s = build_session(
            "tab-1",
            "https://ex.com/cb?access_token=tok-9&password=pw&ok=1",
            "t",
            "",
            &[],
            CLOSE_REASON_SAVED,
        );
        assert!(!s.url.contains("tok-9"), "token 原值不得落盘: {}", s.url);
        assert!(!s.url.contains("pw"), "password 原值不得落盘: {}", s.url);
        assert!(s.url.contains("ok=1"), "非敏感参数应保留: {}", s.url);
    }

    // T-sp-3：预览文本中的 URL 脱敏 + 512B 截断 + truncated 标记
    #[test]
    fn preview_is_scrubbed_and_truncated() {
        let long = format!(
            "正文 https://ex.com/x?token=secret-tok {}",
            "字".repeat(800)
        );
        let (text, truncated) = scrub_preview(&long);
        assert!(!text.contains("secret-tok"), "预览中的 token 必须脱敏");
        assert!(
            text.contains("token=%2A%2A%2A") || text.contains("token=***"),
            "应保留脱敏形态: {text}"
        );
        assert!(truncated, "超长预览必须标记 truncated");
        assert!(text.len() <= SESSION_PREVIEW_MAX_BYTES + 4, "预览必须限长");
        let (short, not_truncated) = scrub_preview("短文本");
        assert!(!not_truncated && short == "短文本");
    }

    // T-sp-4：单会话资源上限（保留最新 N 条，resource_count 记真实总数）
    #[test]
    fn resource_list_is_capped_but_count_is_honest() {
        let mut recs = Vec::new();
        for i in 0..(SESSION_MAX_RESOURCES + 20) {
            recs.push(rec(&format!("r{i}")));
        }
        let s = build_session(
            "tab-1",
            "https://ex.com/",
            "t",
            "",
            &recs,
            CLOSE_REASON_SAVED,
        );
        assert_eq!(s.resources.len(), SESSION_MAX_RESOURCES, "资源列表必须裁剪");
        assert_eq!(
            s.resource_count,
            SESSION_MAX_RESOURCES + 20,
            "resource_count 记真实总数（不因裁剪伪造）"
        );
        assert_eq!(
            s.resources.last().unwrap().id,
            format!("r{}", SESSION_MAX_RESOURCES + 19),
            "裁剪应保留最新记录"
        );
    }

    // T-sp-5：原子写——落盘后目录内无 .tmp 残留，且内容可读回
    #[test]
    fn atomic_write_leaves_no_tmp() {
        let dir = temp_dir("atomic");
        let s = build_session(
            "tab-1",
            "https://ex.com/",
            "t",
            "",
            &[rec("r1")],
            CLOSE_REASON_SAVED,
        );
        save_session(&dir, &s).expect("落盘");
        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            !names.iter().any(|n| n.ends_with(".tmp")),
            "原子写后不得残留 tmp: {names:?}"
        );
        let back = load_session(&dir, &s.id).expect("读回");
        assert_eq!(back.title, "t");
        assert_eq!(back.resources.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    // T-sp-6：损坏文件容错——单个坏 JSON 不得让列表整体失败
    #[test]
    fn corrupt_file_does_not_break_listing() {
        let dir = temp_dir("corrupt");
        let good = build_session(
            "tab-1",
            "https://ex.com/g",
            "good",
            "",
            &[],
            CLOSE_REASON_SAVED,
        );
        save_session(&dir, &good).expect("落盘");
        fs::write(dir.join("broken.json"), "{not-json").expect("写坏文件");
        let list = list_sessions(&dir);
        assert_eq!(list.len(), 1, "损坏文件应被跳过: {list:?}");
        assert_eq!(list[0].id, good.id);
        assert!(
            load_session(&dir, "broken").is_err(),
            "读取损坏会话应返回 Err"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // T-sp-7：删除幂等 + 容量上限（FIFO 删最旧）
    #[test]
    fn delete_is_idempotent_and_capacity_is_enforced() {
        let dir = temp_dir("delete");
        let s = build_session("tab-1", "https://ex.com/", "t", "", &[], CLOSE_REASON_SAVED);
        save_session(&dir, &s).expect("落盘");
        assert!(delete_session(&dir, &s.id).expect("首次删除"));
        assert!(!delete_session(&dir, &s.id).expect("重复删除应幂等返回 false"));

        for i in 0..(SESSION_MAX_COUNT + 5) {
            let mut one = build_session(
                "tab-1",
                "https://ex.com/",
                &format!("s{i}"),
                "",
                &[],
                CLOSE_REASON_SHUTDOWN,
            );
            one.updated_at = Utc::now() + chrono::Duration::seconds(i as i64);
            save_session(&dir, &one).expect("落盘");
        }
        assert_eq!(count_sessions(&dir), SESSION_MAX_COUNT + 5);
        let removed = prune_sessions(&dir, SESSION_MAX_COUNT);
        assert_eq!(removed, 5, "超出容量应删除最旧的 5 条");
        assert_eq!(count_sessions(&dir), SESSION_MAX_COUNT);
        let list = list_sessions(&dir);
        assert_eq!(
            list[0].title,
            format!("s{}", SESSION_MAX_COUNT + 4),
            "最新会话应保留在首位"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // T-sp-8：异常退出残留 .tmp 清理（rename 原子 ⇒ 不会有半截 JSON）
    #[test]
    fn tmp_leftovers_are_pruned_on_restart() {
        let dir = temp_dir("tmpclean");
        fs::write(dir.join("orphan.json.tmp"), "{half").expect("模拟残留");
        let s = build_session("tab-1", "https://ex.com/", "t", "", &[], CLOSE_REASON_SAVED);
        save_session(&dir, &s).expect("落盘");
        assert_eq!(prune_tmp_files(&dir), 1, "孤儿 tmp 必须被清理");
        assert_eq!(count_sessions(&dir), 1, "正常会话不得被误删");
        let _ = fs::remove_dir_all(&dir);
    }

    // T-sp-8b：多字节前缀回归——中文/emoji 正文中的 URL 也必须被识别脱敏。
    // 逐用例只校验「该用例的密钥原值」是否消失（`password` 这类词本身含 "ss"，
    // 用全局子串判断会造成假阳性）。
    #[test]
    fn urls_after_multibyte_text_are_redacted() {
        let cases: [(&str, &str); 3] = [
            ("中文正文 https://ex.com/a?token=zzz 结束", "zzz"),
            ("🚀🚀 https://ex.com/b?password=pptail 尾巴", "pptail"),
            ("混合abc https://ex.com/c?secret=ssval", "ssval"),
        ];
        for (raw, secret) in cases {
            let out = redact_urls_in_text(raw);
            assert!(
                !out.contains(secret),
                "多字节前缀下 URL 漏检（{secret} 未脱敏）: {out}"
            );
            assert!(
                out.contains("%2A%2A%2A") || out.contains("***"),
                "应保留脱敏形态: {out}"
            );
        }
    }

    // T-sp-9：空目录/不存在目录的边界（不 panic、返回空）
    #[test]
    fn empty_or_missing_dir_is_safe() {
        let dir = temp_dir("empty");
        assert_eq!(list_sessions(&dir).len(), 0);
        assert_eq!(prune_tmp_files(&dir), 0);
        assert_eq!(prune_sessions(&dir, 5), 0);
        let missing = dir.join("nope");
        assert_eq!(list_sessions(&missing).len(), 0);
        assert!(!delete_session(&missing, "x").expect("不存在目录删除应幂等"));
        let _ = fs::remove_dir_all(&dir);
    }
}
