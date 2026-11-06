//! WeChat Official Account draft-only integration. No publish, mass-send or
//! scheduled-send endpoints are defined anywhere in this module.
//! App secret remains in the OS keyring; token responses never reach the WebView.
use crate::keyring_store::KeyringStore;
use crate::security_policy as sp;
use crate::shared::invocation::check_invocation_source;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;

// One OS keyring item avoids partially applied account rotation.
const ACCOUNT_KEY: &str = "wechat:official:account-v1";
const WORKING_DRAFT_KEY: &str = "wechat:official:working-draft-v1";
const MAX_WORKING_DRAFT_BYTES: usize = 64 * 1024;
const API_BASE: &str = "https://api.weixin.qq.com/cgi-bin";
const MAX_HTML: usize = 1_000_000;

struct CachedToken {
    app_id: String,
    token: String,
    expires_at: Instant,
}
static TOKEN: OnceLock<Mutex<Option<CachedToken>>> = OnceLock::new();
fn cache() -> &'static Mutex<Option<CachedToken>> {
    TOKEN.get_or_init(|| Mutex::new(None))
}

fn account() -> Result<(String, String), String> {
    let json = KeyringStore::get_token(ACCOUNT_KEY)
        .map_err(|_| "WECHAT_ACCOUNT_NOT_CONFIGURED".to_string())?;
    let data: Value =
        serde_json::from_str(&json).map_err(|_| "WECHAT_ACCOUNT_INVALID".to_string())?;
    let app_id = data
        .get("app_id")
        .and_then(Value::as_str)
        .ok_or("WECHAT_ACCOUNT_INVALID")?;
    let secret = data
        .get("secret")
        .and_then(Value::as_str)
        .ok_or("WECHAT_ACCOUNT_INVALID")?;
    if !valid_app_id(app_id) || secret.len() < 8 {
        return Err("WECHAT_ACCOUNT_INVALID".into());
    }
    Ok((app_id.to_owned(), secret.to_owned()))
}

fn valid_app_id(value: &str) -> bool {
    value.len() >= 6 && value.len() <= 64 && value.bytes().all(|b| b.is_ascii_alphanumeric())
}
fn safe_media_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.contains(['\r', '\n', '\0'])
}
fn checked_payload(response: Value) -> Result<Value, String> {
    let err = response.get("errcode").and_then(Value::as_i64).unwrap_or(0);
    if err != 0 {
        // Never propagate the provider's errmsg: it may echo token or secret.
        return Err(format!("WECHAT_API_{err}"));
    }
    Ok(response)
}
fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "WECHAT_HTTP_CONFIG_FAILED".to_string())
}
async fn json_response(response: reqwest::Response) -> Result<Value, String> {
    if !response.status().is_success() {
        return Err(format!("WECHAT_HTTP_{}", response.status().as_u16()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| "WECHAT_RESPONSE_READ_FAILED")?;
    if bytes.len() > 262_144 {
        return Err("WECHAT_RESPONSE_TOO_LARGE".into());
    }
    let parsed = serde_json::from_slice::<Value>(&bytes)
        .map_err(|_| "WECHAT_RESPONSE_INVALID".to_string())?;
    checked_payload(parsed)
}
async fn token(client: &reqwest::Client) -> Result<String, String> {
    let (app_id, secret) = account()?;
    if let Ok(locked) = cache().lock() {
        if let Some(cached) = locked.as_ref() {
            if cached.app_id == app_id && cached.expires_at > Instant::now() {
                return Ok(cached.token.clone());
            }
        }
    }
    let response = client
        .post(format!("{API_BASE}/stable_token"))
        .json(&json!({
            "grant_type": "client_credential",
            "appid": app_id,
            "secret": secret,
            "force_refresh": false
        }))
        .send()
        .await
        .map_err(|_| "WECHAT_TOKEN_NETWORK_FAILED".to_string())?;
    let data = json_response(response).await?;
    let access = data
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() < 4096)
        .ok_or("WECHAT_TOKEN_MISSING")?;
    let ttl = data
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(7200)
        .clamp(60, 7200);
    let access = access.to_owned();
    if let Ok(mut locked) = cache().lock() {
        *locked = Some(CachedToken {
            app_id,
            token: access.clone(),
            expires_at: Instant::now() + Duration::from_secs(ttl.saturating_sub(300)),
        });
    }
    Ok(access)
}

#[tauri::command]
pub async fn wechat_account_save(
    app: AppHandle,
    webview: tauri::Webview,
    app_id: String,
    app_secret: String,
) -> Result<String, String> {
    check_invocation_source(&webview, "wechat_account_save", None, &app)?;
    if !valid_app_id(&app_id)
        || app_secret.len() < 8
        || app_secret.len() > 512
        || app_secret.contains(['\r', '\n', '\0'])
    {
        return Err("WECHAT_ACCOUNT_INVALID".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let json = json!({"app_id":app_id, "secret":app_secret}).to_string();
        KeyringStore::save_token(ACCOUNT_KEY, &json)
            .map_err(|_| "WECHAT_KEYRING_UNAVAILABLE".to_string())?;
        if let Ok(mut locked) = cache().lock() {
            *locked = None;
        }
        Ok(format!("***{}", &app_id[app_id.len() - 4..]))
    })
    .await
    .map_err(|_| "WECHAT_KEYRING_WORKER_FAILED".to_string())?
}

#[tauri::command]
pub async fn wechat_account_status(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<String, String> {
    check_invocation_source(&webview, "wechat_account_status", None, &app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let (id, _) = account()?;
        Ok(format!("***{}", &id[id.len() - 4..]))
    })
    .await
    .map_err(|_| "WECHAT_KEYRING_WORKER_FAILED".to_string())?
}

#[tauri::command]
pub async fn wechat_draft_permission_check(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Value, String> {
    check_invocation_source(&webview, "wechat_draft_permission_check", None, &app)?;
    let client = http()?;
    let access = token(&client).await?;
    // Read-only API, does not create or publish any draft.
    let response = client
        .get(format!("{API_BASE}/draft/count"))
        .query(&[("access_token", access)])
        .send()
        .await
        .map_err(|_| "WECHAT_PERMISSION_NETWORK_FAILED".to_string())?;
    let result = json_response(response).await?;
    Ok(
        json!({"draft_count": result.get("total_count").and_then(Value::as_u64), "draft_read_access": true}),
    )
}

async fn upload(app: AppHandle, path: String, cover: bool) -> Result<Value, String> {
    let roots = crate::workspace::allowed_roots(&app);
    let canonical = sp::check_path_within_roots(&path, &roots)
        .map_err(|_| "WECHAT_IMAGE_PATH_DENIED".to_string())?;
    let size_limit = if cover { 64 * 1024 } else { 1024 * 1024 };
    // Read through authorized, non-following directory FDs: the path may be
    // replaced after canonicalize and must never redirect to an outside file.
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        super::safe_paths::read_regular(&canonical, &roots, size_limit)
    })
    .await
    .map_err(|_| "WECHAT_IMAGE_READ_FAILED")?
    .map_err(|_| "WECHAT_IMAGE_READ_FAILED")?;
    let jpeg = bytes.starts_with(&[0xff, 0xd8, 0xff]);
    let png = bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]);
    if !jpeg && !(png && !cover) {
        return Err("WECHAT_IMAGE_TYPE_INVALID".into());
    }
    let mime = if jpeg { "image/jpeg" } else { "image/png" };
    let client = http()?;
    let access = token(&client).await?;
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(if jpeg { "image.jpg" } else { "image.png" })
        .mime_str(mime)
        .map_err(|_| "WECHAT_MULTIPART_INVALID")?;
    let form = reqwest::multipart::Form::new().part("media", part);
    let endpoint = if cover {
        "material/add_material"
    } else {
        "media/uploadimg"
    };
    let mut query = vec![("access_token", access.as_str())];
    if cover {
        query.push(("type", "thumb"));
    }
    let response = client
        .post(format!("{API_BASE}/{endpoint}"))
        .query(&query)
        .multipart(form)
        .send()
        .await
        .map_err(|_| "WECHAT_IMAGE_UPLOAD_NETWORK_FAILED")?;
    let result = json_response(response).await?;
    if cover {
        let media = result
            .get("media_id")
            .and_then(Value::as_str)
            .filter(|s| safe_media_id(s))
            .ok_or("WECHAT_COVER_MEDIA_ID_MISSING")?;
        Ok(json!({"media_id": media}))
    } else {
        let url = result
            .get("url")
            .and_then(Value::as_str)
            .filter(|s| s.starts_with("https://") && s.len() <= 4096)
            .ok_or("WECHAT_BODY_IMAGE_URL_MISSING")?;
        Ok(json!({"url": url}))
    }
}

#[tauri::command]
pub async fn wechat_upload_body_image(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
) -> Result<Value, String> {
    check_invocation_source(&webview, "wechat_upload_body_image", None, &app)?;
    upload(app, path, false).await
}
#[tauri::command]
pub async fn wechat_upload_cover(
    app: AppHandle,
    webview: tauri::Webview,
    path: String,
) -> Result<Value, String> {
    check_invocation_source(&webview, "wechat_upload_cover", None, &app)?;
    upload(app, path, true).await
}

fn validate_news_draft(
    title: &str,
    author: &str,
    digest: &str,
    html: &str,
    cover_media_id: &str,
) -> Result<(), String> {
    // Documented news-draft limits: 32 title characters, 16 author characters,
    // fewer than 20,000 content characters and less than 1 MB of UTF-8.
    if title.trim().is_empty()
        || title.chars().count() > 32
        || author.chars().count() > 16
        || digest.chars().count() > 120
        || html.trim().is_empty()
        || html.chars().count() >= 20_000
        || html.len() >= MAX_HTML
        || !safe_media_id(cover_media_id)
    {
        return Err("WECHAT_DRAFT_INPUT_INVALID".into());
    }
    Ok(())
}

// The provider returns news_item[] from draft/get. A 200 response alone is
// not proof that an article has the expected title, body and image binding.
fn returned_article(details: &Value) -> Option<&Value> {
    details.get("news_item")?.as_array()?.first()
}
fn draft_has_valid_content(details: &Value) -> bool {
    let Some(article) = returned_article(details) else {
        return false;
    };
    let title = article.get("title").and_then(Value::as_str).unwrap_or("");
    let content = article.get("content").and_then(Value::as_str).unwrap_or("");
    let cover = article
        .get("thumb_media_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    !title.trim().is_empty() && !content.trim().is_empty() && safe_media_id(cover)
}
fn draft_matches_request(
    details: &Value,
    title: &str,
    author: &str,
    digest: &str,
    html: &str,
    cover_id: &str,
) -> bool {
    let Some(article) = returned_article(details) else {
        return false;
    };
    draft_has_valid_content(details)
        && article.get("title").and_then(Value::as_str) == Some(title)
        && article.get("author").and_then(Value::as_str).unwrap_or("") == author
        && article.get("digest").and_then(Value::as_str).unwrap_or("") == digest
        && article.get("content").and_then(Value::as_str) == Some(html)
        && article.get("thumb_media_id").and_then(Value::as_str) == Some(cover_id)
}

#[tauri::command]
pub async fn wechat_create_draft(
    app: AppHandle,
    webview: tauri::Webview,
    title: String,
    author: String,
    digest: String,
    content_html: String,
    cover_media_id: String,
) -> Result<Value, String> {
    check_invocation_source(&webview, "wechat_create_draft", None, &app)?;
    validate_news_draft(&title, &author, &digest, &content_html, &cover_media_id)?;
    let client = http()?;
    let access = token(&client).await?;
    let payload = json!({"articles":[{
        "article_type": "news",
        "title": title,
        "author": author,
        "digest": digest,
        "content": content_html,
        "thumb_media_id": cover_media_id,
        "need_open_comment": 0,
        "only_fans_can_comment": 0
    }]});
    // Not automatically retried: a timeout may mean the draft already exists.
    let response = client
        .post(format!("{API_BASE}/draft/add"))
        .query(&[("access_token", access.as_str())])
        .json(&payload)
        .send()
        .await
        .map_err(|_| "WECHAT_DRAFT_OUTCOME_UNKNOWN_NO_AUTO_RETRY")?;
    let result = json_response(response).await?;
    let media = result
        .get("media_id")
        .and_then(Value::as_str)
        .filter(|s| safe_media_id(s))
        .ok_or("WECHAT_DRAFT_MEDIA_ID_MISSING")?
        .to_string();
    // Verify the draft exists. Verification failure must not discard returned ID
    // or trigger a duplicate draft/add call.
    let verified = match client
        .post(format!("{API_BASE}/draft/get"))
        .query(&[("access_token", access.as_str())])
        .json(&json!({"media_id": media}))
        .send()
        .await
    {
        Ok(response) => match json_response(response).await {
            Ok(details) => draft_matches_request(
                &details,
                &title,
                &author,
                &digest,
                &content_html,
                &cover_media_id,
            ),
            Err(_) => false,
        },
        Err(_) => false,
    };
    Ok(
        json!({"media_id": media, "verified": verified, "status": if verified {"DRAFT_CONFIRMED"} else {"DRAFT_CREATED_VERIFICATION_PENDING"}}),
    )
}

#[tauri::command]
pub async fn wechat_get_draft_status(
    app: AppHandle,
    webview: tauri::Webview,
    media_id: String,
) -> Result<Value, String> {
    check_invocation_source(&webview, "wechat_get_draft_status", None, &app)?;
    if !safe_media_id(&media_id) {
        return Err("WECHAT_MEDIA_ID_INVALID".into());
    }
    let client = http()?;
    let access = token(&client).await?;
    let response = client
        .post(format!("{API_BASE}/draft/get"))
        .query(&[("access_token", access.as_str())])
        .json(&json!({"media_id": media_id}))
        .send()
        .await
        .map_err(|_| "WECHAT_DRAFT_VERIFY_NETWORK_FAILED")?;
    let details = json_response(response).await?;
    let verified = draft_has_valid_content(&details);
    Ok(
        json!({"media_id": media_id, "verified": verified, "status": if verified {"DRAFT_CONTENT_PRESENT_NEEDS_BACKEND_REVIEW"} else {"DRAFT_DETAILS_INCOMPLETE"}}),
    )
}

/// User-authored WeChat draft state is kept in a dedicated OS-keyring item.
/// It is never copied to localStorage, browser cache or the provider API until
/// the user explicitly presses Create Draft.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WechatWorkingDraft {
    pub version: u32,
    pub title: String,
    pub author: String,
    pub digest: String,
    pub content_html: String,
    pub cover_path: String,
    pub body_image_path: String,
    pub cover_media_id: String,
    pub body_image_url: String,
    pub draft_media_id: String,
    pub verified: bool,
    pub outcome_unknown: bool,
    pub updated_at_ms: u64,
}

fn validate_working_draft(draft: &WechatWorkingDraft) -> Result<(), String> {
    if draft.version != 1
        || draft.title.chars().count() > 32
        || draft.author.chars().count() > 16
        || draft.digest.chars().count() > 120
        || draft.content_html.len() > 19_999
        || draft.cover_path.len() > 4096
        || draft.body_image_path.len() > 4096
        || draft.cover_media_id.len() > 512
        || draft.body_image_url.len() > 2048
        || draft.draft_media_id.len() > 512
        || draft.title.contains(['\r', '\n', '\0'])
        || draft.author.contains(['\r', '\n', '\0'])
        || draft.digest.contains('\0')
        || draft.content_html.contains('\0')
        || draft.cover_path.contains('\0')
        || draft.body_image_path.contains('\0')
        || (!draft.cover_media_id.is_empty() && !safe_media_id(&draft.cover_media_id))
        || (!draft.draft_media_id.is_empty() && !safe_media_id(&draft.draft_media_id))
        || (!draft.body_image_url.is_empty()
            && !(draft.body_image_url.starts_with("https://mmbiz.qpic.cn/")
                || draft.body_image_url.starts_with("https://mmbiz.qlogo.cn/")))
    {
        return Err("WECHAT_WORKING_DRAFT_INVALID".into());
    }
    for (name, value, max) in [
        ("title", draft.title.as_str(), 512),
        ("author", draft.author.as_str(), 256),
        ("digest", draft.digest.as_str(), 1024),
        ("content_html", draft.content_html.as_str(), 19_999),
        ("cover_path", draft.cover_path.as_str(), 4096),
        ("body_image_path", draft.body_image_path.as_str(), 4096),
        ("cover_media_id", draft.cover_media_id.as_str(), 512),
        ("body_image_url", draft.body_image_url.as_str(), 2048),
        ("draft_media_id", draft.draft_media_id.as_str(), 512),
    ] {
        sp::check_text_field(name, value, max).map_err(|_| "WECHAT_WORKING_DRAFT_INVALID")?;
    }
    Ok(())
}

#[tauri::command]
pub async fn wechat_working_draft_load(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<Option<WechatWorkingDraft>, String> {
    check_invocation_source(&webview, "wechat_working_draft_load", None, &app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let raw = match KeyringStore::get_token_result(WORKING_DRAFT_KEY) {
            Ok(value) => value,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(_) => return Err("WECHAT_DRAFT_STORE_UNAVAILABLE".into()),
        };
        if raw.len() > MAX_WORKING_DRAFT_BYTES {
            return Err("WECHAT_WORKING_DRAFT_INVALID".into());
        }
        let draft: WechatWorkingDraft =
            serde_json::from_str(&raw).map_err(|_| "WECHAT_WORKING_DRAFT_INVALID")?;
        validate_working_draft(&draft)?;
        Ok(Some(draft))
    })
    .await
    .map_err(|_| "WECHAT_DRAFT_STORE_WORKER_FAILED".to_string())?
}

#[tauri::command]
pub async fn wechat_working_draft_save(
    app: AppHandle,
    webview: tauri::Webview,
    mut draft: WechatWorkingDraft,
) -> Result<u64, String> {
    check_invocation_source(&webview, "wechat_working_draft_save", None, &app)?;
    draft.version = 1;
    draft.updated_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64;
    validate_working_draft(&draft)?;
    let encoded = serde_json::to_string(&draft).map_err(|_| "WECHAT_WORKING_DRAFT_INVALID")?;
    if encoded.len() > MAX_WORKING_DRAFT_BYTES {
        return Err("WECHAT_WORKING_DRAFT_TOO_LARGE".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        KeyringStore::save_token(WORKING_DRAFT_KEY, &encoded)
            .map_err(|_| "WECHAT_DRAFT_STORE_UNAVAILABLE".to_string())?;
        Ok(draft.updated_at_ms)
    })
    .await
    .map_err(|_| "WECHAT_DRAFT_STORE_WORKER_FAILED".to_string())?
}

#[tauri::command]
pub async fn wechat_working_draft_clear(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<(), String> {
    check_invocation_source(&webview, "wechat_working_draft_clear", None, &app)?;
    tauri::async_runtime::spawn_blocking(move || {
        match KeyringStore::get_token_result(WORKING_DRAFT_KEY) {
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("WECHAT_DRAFT_STORE_UNAVAILABLE".into()),
            Ok(_) => KeyringStore::delete_token(WORKING_DRAFT_KEY)
                .map_err(|_| "WECHAT_DRAFT_STORE_UNAVAILABLE".to_string()),
        }
    })
    .await
    .map_err(|_| "WECHAT_DRAFT_STORE_WORKER_FAILED".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn account_and_media_ids_reject_unsafe_strings() {
        assert!(valid_app_id("wx1234567890abcdef"));
        assert!(!valid_app_id("a/b"));
        assert!(!safe_media_id("abc\nxyz"));
        assert!(safe_media_id("abc-DEF_123/+"));
    }
    #[test]
    fn news_draft_payload_has_documented_bounds() {
        assert!(validate_news_draft("标题", "作者", "摘要", "<p>正文</p>", "cover-id").is_ok());
        assert_eq!(
            validate_news_draft(&"中".repeat(33), "作者", "摘要", "<p>正文</p>", "cover-id"),
            Err("WECHAT_DRAFT_INPUT_INVALID".into())
        );
        assert_eq!(
            validate_news_draft("标题", &"中".repeat(17), "摘要", "<p>正文</p>", "cover-id"),
            Err("WECHAT_DRAFT_INPUT_INVALID".into())
        );
        assert_eq!(
            validate_news_draft("标题", "作者", "摘要", &"中".repeat(20_000), "cover-id"),
            Err("WECHAT_DRAFT_INPUT_INVALID".into())
        );
    }
    #[test]
    fn draft_details_are_checked_not_just_http_status() {
        let good = json!({"news_item":[{"title":"标题","author":"作者","digest":"摘要","content":"<p><img src=\"https://mmbiz.qpic.cn/test.jpg\"/></p>","thumb_media_id":"cover-id"}]});
        let html = "<p><img src=\"https://mmbiz.qpic.cn/test.jpg\"/></p>";
        assert!(draft_matches_request(
            &good, "标题", "作者", "摘要", html, "cover-id"
        ));
        assert!(draft_has_valid_content(&good));
        assert!(!draft_matches_request(
            &good,
            "错误标题",
            "作者",
            "摘要",
            html,
            "cover-id"
        ));
        assert!(!draft_matches_request(
            &good,
            "标题",
            "作者",
            "摘要",
            "<p>图片丢失</p>",
            "cover-id"
        ));
        assert!(!draft_matches_request(
            &good,
            "标题",
            "作者",
            "摘要",
            html,
            "错误封面"
        ));
        assert!(!draft_has_valid_content(&json!({"errcode":0})));
        assert!(!draft_has_valid_content(&json!({"news_item":[]})));
    }
    #[test]
    fn malformed_draft_details_cannot_be_verified() {
        assert!(!draft_has_valid_content(
            &json!({"news_item":[{"title":"标题","content":"正文"}]})
        ));
        assert!(!draft_has_valid_content(
            &json!({"news_item":[{"title":"标题","content":"", "thumb_media_id":"x"}]})
        ));
    }
    #[test]
    fn api_error_hides_provider_message_and_secrets() {
        assert_eq!(
            checked_payload(json!({"errcode":48001,"errmsg":"secret=do-not-show"})).unwrap_err(),
            "WECHAT_API_48001"
        );
    }

    #[test]
    fn working_draft_roundtrip_validates_bounds_and_never_contains_appsecret() {
        let sample = WechatWorkingDraft {
            version: 1,
            title: "未完成选题".into(),
            author: "作者".into(),
            content_html: "<p>用户自己的正文</p>".into(),
            outcome_unknown: true,
            ..Default::default()
        };
        validate_working_draft(&sample).expect("working draft is within bounds");
        let serialized = serde_json::to_string(&sample).unwrap();
        assert!(!serialized.contains("app_secret"));
        assert!(!serialized.contains("access_token"));
        let restored: WechatWorkingDraft = serde_json::from_str(&serialized).unwrap();
        assert!(
            restored.outcome_unknown,
            "ambiguous submit state must survive roundtrip"
        );
        assert_eq!(restored.content_html, sample.content_html);

        let oversized = WechatWorkingDraft {
            title: "字".repeat(33),
            ..sample.clone()
        };
        assert_eq!(
            validate_working_draft(&oversized),
            Err("WECHAT_WORKING_DRAFT_INVALID".into())
        );
        let foreign_image = WechatWorkingDraft {
            body_image_url: "https://example.org/image.png".into(),
            ..sample
        };
        assert_eq!(
            validate_working_draft(&foreign_image),
            Err("WECHAT_WORKING_DRAFT_INVALID".into())
        );
    }
}
