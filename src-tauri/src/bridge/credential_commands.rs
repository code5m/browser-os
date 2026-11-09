//! Browser credential import, keyring metadata and autofill commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

#[derive(Clone, serde::Deserialize)]
pub struct BrowserCredentialRow {
    pub url: String,
    pub username: String,
    pub password: String,
}

/// keyring account key：同 url + username 稳定映射到同一条目（重复导入即覆盖更新）。
pub fn browser_credential_key(url: &str, username: &str) -> String {
    format!("browser:{}:{}", normalize_url(url.trim()), username.trim())
}

/// 行级校验（纯函数，便于单测）：合法行返回 keyring account key，非法行返回 None。
fn credential_row_key(row: &BrowserCredentialRow) -> Option<String> {
    if row.url.trim().is_empty() || row.username.trim().is_empty() || row.password.is_empty() {
        return None;
    }
    Some(browser_credential_key(&row.url, &row.username))
}

// ---------------- 非敏感索引（「已导入账号」列表用）----------------
//
// 为什么必须有索引：`keyring` crate 只提供「按 key 精确读写」，不提供枚举；且 browser
// 条目的 key 含 username（`browser:<url>:<username>`），无法仅凭 url 反查。
// 因此由应用层维护一份**编译期即无 password 字段**的元数据索引，只用于展示
// 「哪些账号已保存」，不承担任何取密码职责。
//
// 索引内容白名单：**只有** key / url / username / imported_at。
// 任何 password / token / secret 一律禁止入内（落盘即违规）。

/// 非敏感凭据索引条目。该结构**没有 password 字段**，编译期即无法写入密码。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowserCredentialMeta {
    /// keyring account key（与系统密钥库条目 1:1）
    pub key: String,
    /// 归一化后的 url（与 key 中一致，避免二次推导出现偏差）
    pub url: String,
    pub username: String,
    /// 导入时间（RFC3339），仅用于展示/排序
    pub imported_at: String,
}

/// 账号列表返回给前端的 DTO。
///
/// 只含 url / username / has_password：不返回 password、不返回 credential key
/// （前端不需要知道内部 keyring 命名空间）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct BrowserCredentialItem {
    pub url: String,
    pub username: String,
    pub has_password: bool,
    /// 不透明句柄（会话级，每次 list 重新生成）：前端只能用它指认「用户选了哪一条」，
    /// 既拿不到 password，也拿不到真正的 keyring account key。
    pub credential_id: String,
    /// 精确 origin（`scheme://host[:port]`，默认端口已规范化），供前端判断是否与
    /// 当前网页同源；非 http/https 一律为空串（永不匹配）。
    pub origin: String,
}

/// 索引文件：`data_dir()/browser-credentials.json`（与 bookmarks.json 同目录、同原子写范式）。
fn browser_credential_index_file(app: &AppHandle) -> std::path::PathBuf {
    workspace::data_dir(app).join("browser-credentials.json")
}

/// 纯函数：把新导入的元数据并入现有列表，**按 key 去重**（重复导入只更新展示字段与
/// imported_at，绝不产生重复账号记录）。
fn merge_credential_metas(list: &mut Vec<BrowserCredentialMeta>, metas: &[BrowserCredentialMeta]) {
    for meta in metas {
        match list.iter_mut().find(|m| m.key == meta.key) {
            Some(existing) => {
                existing.url = meta.url.clone();
                existing.username = meta.username.clone();
                existing.imported_at = meta.imported_at.clone();
            }
            None => list.push(meta.clone()),
        }
    }
    list.sort_by(|a, b| a.key.cmp(&b.key));
}

/// 合并索引并原子写到指定路径（`_at` 便于无 AppHandle 单测）。
/// 只在 keyring 写成功后调用；失败时返回**不含任何输入字段**的安全错误。
fn merge_credential_index_at(
    path: &std::path::Path,
    metas: &[BrowserCredentialMeta],
) -> Result<(), String> {
    if metas.is_empty() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let mut list: Vec<BrowserCredentialMeta> =
        workspace::load_json_list_at(path, "browser-credentials");
    merge_credential_metas(&mut list, metas);
    workspace::save_json_list_at(path, &list, "browser-credentials")
}

fn merge_credential_index(app: &AppHandle, metas: &[BrowserCredentialMeta]) -> Result<(), String> {
    merge_credential_index_at(&browser_credential_index_file(app), metas)
}

/// 导入结果（纯函数层，便于无 AppHandle 单测）。
pub struct CredentialImportOutcome {
    pub saved: usize,
    pub failed: usize,
    /// 成功写入 keyring 的条目对应的**非敏感**元数据（用于合并索引，绝不含密码）。
    pub metas: Vec<BrowserCredentialMeta>,
}

fn import_browser_credentials_inner(rows: &[BrowserCredentialRow]) -> CredentialImportOutcome {
    let mut saved = 0usize;
    let mut failed = 0usize;
    let mut metas: Vec<BrowserCredentialMeta> = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();
    for row in rows {
        let Some(key) = credential_row_key(row) else {
            failed += 1;
            continue;
        };
        // 只吞掉 keyring 错误本身，不回显任何输入（密码绝不能出现在错误信息里）
        match KeyringStore::save_token(&key, &row.password) {
            Ok(()) => {
                saved += 1;
                metas.push(BrowserCredentialMeta {
                    key,
                    url: normalize_url(row.url.trim()),
                    username: row.username.trim().to_string(),
                    imported_at: now.clone(),
                });
            }
            Err(_) => failed += 1,
        }
    }
    CredentialImportOutcome {
        saved,
        failed,
        metas,
    }
}

#[tauri::command]
pub fn import_browser_credentials(
    app: AppHandle,
    rows: Vec<BrowserCredentialRow>,
) -> Result<usize, String> {
    let out = import_browser_credentials_inner(&rows);
    // 写入顺序（不可颠倒）：keyring 写成功 → merge metadata → 原子写索引。
    // 严禁「先写 metadata 再写 keyring」，否则会显示「已保存」但实际没有密码。
    if let Err(e) = merge_credential_index(&app, &out.metas) {
        // 不回滚已成功写入的 keyring 密码（回滚才是真丢数据）。
        // 日志只打条数，不打 key / url / username，更不打 password。
        eprintln!(
            "[credential] 账号索引更新失败：{} 条（密码已安全保存，列表暂不可见，重新导入可覆盖）（{e}）",
            out.metas.len()
        );
    }
    if out.saved == 0 && out.failed > 0 {
        return Err(format!(
            "导入失败：{} 条未写入系统密钥库（请确认钥匙环已解锁）",
            out.failed
        ));
    }
    Ok(out.saved)
}

/// 已导入账号列表（**只读**）：只返回 url / username / has_password。
///
/// 安全约束：
///   - 列表路径**不调用** `get_password`：既避免一次列表触发 N 次钥匙环解锁弹窗，
///     也避免把 secret 无意义地读进进程内存；
///   - `has_password` 由非敏感索引推导（索引条目只在 keyring 写成功后追加）；
///   - 不返回 password / secret / token / credential key。
#[tauri::command]
pub fn list_browser_credentials(app: AppHandle) -> Result<Vec<BrowserCredentialItem>, String> {
    let path = browser_credential_index_file(&app);
    let list: Vec<BrowserCredentialMeta> =
        workspace::load_json_list_at(&path, "browser-credentials");
    // 句柄每次列班重建：一次性、会话级、不落盘，旧句柄自动失效。
    let state = app.state::<AppState>();
    let mut handles = state
        .credential_handles
        .lock()
        .map_err(|_| "凭据句柄表不可用".to_string())?;
    handles.clear();
    let items = list
        .into_iter()
        .map(|m| {
            let credential_id = uuid::Uuid::new_v4().to_string();
            handles.insert(credential_id.clone(), m.clone());
            BrowserCredentialItem {
                url: m.url.clone(),
                username: m.username,
                has_password: !m.key.is_empty(),
                credential_id,
                origin: credential_origin(&m.url).unwrap_or_default(),
            }
        })
        .collect();
    Ok(items)
}

// ---------------- 自动填充（用户主动触发，绝不自动提交）----------------

/// 精确 origin：`scheme://host[:port]`，默认端口规范化后省略。
///
/// 只接受 http/https；其余（`file://` / `about:` / 非法串）返回 `None` → 一律拒绝匹配。
/// 不做父域共享、子域共享、通配符、相似域名匹配：
/// `sql.ainfinit.com` 与 `sql.ainfinit.com.evil.com`、`evil-sql.ainfinit.com`
/// 必须是**不同** origin。
pub fn credential_origin(raw: &str) -> Option<String> {
    let u = url::Url::parse(raw.trim()).ok()?;
    match u.scheme() {
        "http" | "https" => {}
        _ => return None,
    }
    let host = u.host_str()?;
    // url crate 已做小写化与默认端口剥离；显式非默认端口才带 :port
    match u.port() {
        Some(p) => Some(format!("{}://{}:{}", u.scheme(), host, p)),
        None => Some(format!("{}://{}", u.scheme(), host)),
    }
}

/// 允许的填充结果码（全部非敏感，可安全回传前端与写入日志）。
const AUTOFILL_CODES: [&str; 10] = [
    "FILLED",
    "ORIGIN_MISMATCH",
    "TAB_NOT_FOUND",
    "WEBVIEW_NOT_FOUND",
    "NO_PASSWORD_FIELD",
    "NO_USERNAME_FIELD",
    "AMBIGUOUS_FORM",
    "CROSS_ORIGIN_IFRAME_UNSUPPORTED",
    "KEYRING_READ_FAILED",
    "CREDENTIAL_NOT_FOUND",
];

/// 构造**一次性**自动填充脚本。
///
/// - 页面内用 `location.origin` 做**权威**同源校验（`EXPECT` 由 Rust 传入），
///   可挡住「打开账号列表 → 页面跳到别的 origin → 再点填充」。
/// - 用户名/密码经 JSON 字符串转义嵌入，密码中的引号/反斜杠/换行不会破坏字面量。
/// - **绝不**提交表单：不 `submit()`、不 `requestSubmit()`、不点按钮、不派发 Enter。
/// - 用原生 value setter + input/change 事件，兼容 Vue/React/Angular 受控输入。
fn build_autofill_script(expected_origin: &str, username: &str, password: &str) -> String {
    // U+2028/U+2029 在部分旧 JS 解析器里是行终止符，JSON 不转义 → 额外处理
    let j = |s: &str| {
        serde_json::to_string(s)
            .unwrap_or_else(|_| "\"\"".to_string())
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029")
    };
    const SCRIPT: &str = r#"(function () {
  var EXPECT = __ORIGIN__;
  var U = __USERNAME__;
  var P = __PASSWORD__;
  function visible(el) {
    if (!el || el.disabled || el.readOnly) return false;
    var s = window.getComputedStyle(el);
    if (!s || s.display === 'none' || s.visibility === 'hidden') return false;
    var r = el.getBoundingClientRect();
    return r.width > 0 || r.height > 0;
  }
  function passwordInputs(doc) {
    return Array.prototype.slice.call(doc.querySelectorAll('input[type="password"]')).filter(visible);
  }
  try {
    if (!window.location || window.location.origin !== EXPECT) return 'ORIGIN_MISMATCH';
    var scope = document;
    var pw = passwordInputs(document);
    if (pw.length === 0) {
      var frames = Array.prototype.slice.call(document.querySelectorAll('iframe'));
      for (var i = 0; i < frames.length; i += 1) {
        var d = null;
        try { d = frames[i].contentDocument; } catch (e) { d = null; }
        if (!d) continue;
        var f = passwordInputs(d);
        if (f.length) { pw = f; scope = d; break; }
      }
      if (pw.length === 0) {
        return frames.length ? 'CROSS_ORIGIN_IFRAME_UNSUPPORTED' : 'NO_PASSWORD_FIELD';
      }
    }
    if (pw.length > 1) return 'AMBIGUOUS_FORM';
    var pwEl = pw[0];
    var pool = pwEl.form
      ? Array.prototype.slice.call(pwEl.form.querySelectorAll('input'))
      : Array.prototype.slice.call(scope.querySelectorAll('input'));
    var cands = pool.filter(function (el) {
      if (el === pwEl) return false;
      var t = (el.type || 'text').toLowerCase();
      if (['password','hidden','submit','button','checkbox','radio','file','image','reset'].indexOf(t) >= 0) return false;
      return visible(el);
    });
    function score(el) {
      var ac = (el.getAttribute('autocomplete') || '').toLowerCase();
      var nm = ((el.name || '') + ' ' + (el.id || '')).toLowerCase();
      var t = (el.type || '').toLowerCase();
      if (ac === 'username') return 100;
      if (t === 'email') return 90;
      if (/user|login|email|account/.test(nm) || /user|login|email|account/.test(ac)) return 70;
      return 10;
    }
    var best = null, bestScore = 0, ties = 0;
    cands.forEach(function (el) {
      var s = score(el);
      if (s > bestScore) { best = el; bestScore = s; ties = 1; }
      else if (s === bestScore) { ties += 1; }
    });
    // 宁可失败，不要填错字段：无关键词命中且候选不止一个 → 不猜
    if (!best || (bestScore < 70 && ties > 1)) return 'NO_USERNAME_FIELD';
    var setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    function fill(el, v) {
      if (setter) setter.call(el, v); else el.value = v;
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
    }
    fill(best, U);
    fill(pwEl, P);
    return 'FILLED';
  } catch (e) {
    return 'FILL_FAILED';
  }
})()"#;
    SCRIPT
        .replace("__ORIGIN__", &j(expected_origin))
        .replace("__USERNAME__", &j(username))
        .replace("__PASSWORD__", &j(password))
}

/// 用户**主动点击**才触发的一次性填充：读单条密码 → 注入当前 WebView → 结束。
///
/// 严格不做：自动提交 / 自动点击登录 / `form.submit()` / `requestSubmit()` /
/// 自动 Enter / 自动登录 / 后台静默填充。填充后由用户自己点「登录」。
///
/// 返回值只允许是 `AUTOFILL_CODES` 中的非敏感状态码（或 `FILL_FAILED`）。
#[tauri::command]
pub async fn fill_browser_credential(
    app: AppHandle,
    webview: tauri::Webview,
    credential_id: String,
    tab_id: String,
) -> Result<String, String> {
    check_invocation_source(&webview, "fill_browser_credential", None, &app)?;

    // 1) opaque handle → 非敏感元数据（会话级，不落盘）
    let cred = {
        let state = app.state::<AppState>();
        let handles = state
            .credential_handles
            .lock()
            .map_err(|_| "凭据句柄表不可用".to_string())?;
        handles.get(&credential_id).cloned()
    };
    let Some(cred) = cred else {
        return Ok("CREDENTIAL_NOT_FOUND".to_string());
    };

    // 2) 填充**瞬间**重新校验：页签仍存在 / 仍是当前 active tab / webview 未休眠
    let st = app.state::<AppState>();
    if st
        .hibernated_tabs
        .lock()
        .map_err(|_| "页签状态不可用".to_string())?
        .contains(&tab_id)
    {
        return Ok("WEBVIEW_NOT_FOUND".to_string());
    }
    let active = st
        .active_tab
        .lock()
        .map_err(|_| "页签状态不可用".to_string())?
        .clone();
    if active.as_deref() != Some(tab_id.as_str()) {
        return Ok("TAB_NOT_FOUND".to_string());
    }
    let stored_url = match st
        .tabs
        .lock()
        .map_err(|_| "页签状态不可用".to_string())?
        .get(&tab_id)
    {
        Some(t) => t.url.clone(),
        None => return Ok("TAB_NOT_FOUND".to_string()),
    };

    // 3) origin 精确匹配（这里只是粗筛；权威校验在页面内用 location.origin 再做一次）
    let cred_origin = match credential_origin(&cred.url) {
        Some(o) => o,
        None => return Ok("ORIGIN_MISMATCH".to_string()),
    };
    // 存储 URL 可解析且 origin 不同 → 立即拒绝；不可解析（首次创建/会话恢复等边界）
    // 时不硬拒，交由**页面内** location.origin 的权威校验兜底（更强，因为它就是真实页面）。
    if let Some(page) = credential_origin(&stored_url) {
        if page != cred_origin {
            return Ok("ORIGIN_MISMATCH".to_string());
        }
    }

    // 4) 只读这一条；keyring 报错可能带 account key，故不回显具体错误文本，
    // 只根据错误类型映射到非敏感结果码。
    let password = match KeyringStore::get_token_result(&cred.key) {
        Ok(p) => p,
        Err(keyring::Error::NoEntry) => return Ok("CREDENTIAL_NOT_FOUND".to_string()),
        Err(_) => return Ok("KEYRING_READ_FAILED".to_string()),
    };

    // 5) 构造一次性脚本（password 只活在这一段调用栈里，不写日志、不回传前端）
    let js = build_autofill_script(&cred_origin, &cred.username, &password);
    drop(password);

    // 6) 执行并取回状态码：走 Linux 原生返回通道，不授予页面任何 IPC 权限
    let (tx, rx) = std::sync::mpsc::channel();
    let manager = app.state::<tauri_plugin_browser_tabs::TabManagerState>();
    if manager
        .eval_result(&tab_id, js, move |value| {
            let _ = tx.send(value);
        })
        .is_err()
    {
        return Ok("WEBVIEW_NOT_FOUND".to_string());
    }
    match rx.recv_timeout(std::time::Duration::from_millis(3000)) {
        Ok(Ok(code)) if AUTOFILL_CODES.contains(&code.as_str()) => Ok(code),
        _ => Ok("FILL_FAILED".to_string()),
    }
}

#[cfg(test)]
mod browser_credential_tests {
    use super::*;

    #[test]
    fn key_is_stable_per_url_and_username() {
        let a = browser_credential_key("https://example.com", "u1");
        let b = browser_credential_key("https://example.com", "u1");
        assert_eq!(a, b, "同一 url+username 必须稳定映射到同一 keyring 条目");
        assert_ne!(a, browser_credential_key("https://example.com", "u2"));
        assert_ne!(a, browser_credential_key("https://other.com", "u1"));
        assert!(a.starts_with("browser:"), "须与 db:/git 凭据命名空间隔离");
    }

    #[test]
    fn key_normalizes_url_and_trims_fields() {
        assert_eq!(
            browser_credential_key("example.com", " u1 "),
            browser_credential_key("https://example.com", "u1")
        );
    }

    #[test]
    fn invalid_rows_rejected_without_touching_keyring() {
        let rows = vec![
            BrowserCredentialRow {
                url: "".into(),
                username: "u".into(),
                password: "p".into(),
            },
            BrowserCredentialRow {
                url: "https://e.com".into(),
                username: "".into(),
                password: "p".into(),
            },
            BrowserCredentialRow {
                url: "https://e.com".into(),
                username: "u".into(),
                password: "".into(),
            },
        ];
        // 全部非法 → 在访问 keyring 前即被拒（无钥匙环环境下也不会误报）
        let out = import_browser_credentials_inner(&rows);
        assert_eq!(out.saved, 0, "非法行不得写入 keyring");
        assert_eq!(out.failed, 3, "三条非法行都要计入失败");
        assert!(out.metas.is_empty(), "失败行不得进入索引");
    }

    fn meta(key: &str, imported_at: &str) -> BrowserCredentialMeta {
        BrowserCredentialMeta {
            key: key.into(),
            url: "https://example.com".into(),
            username: "u1".into(),
            imported_at: imported_at.into(),
        }
    }

    #[test]
    fn index_meta_schema_has_no_password_field() {
        let value = serde_json::to_value(meta("browser:https://example.com:u1", "t1")).unwrap();
        let obj = value.as_object().expect("索引条目必须是对象");
        let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec!["imported_at", "key", "url", "username"],
            "索引 schema 只允许 key/url/username/imported_at，严禁 password"
        );
        let json = serde_json::to_string(&value).unwrap();
        assert!(
            !json.contains("password"),
            "序列化结果不得出现 password 字段"
        );
    }

    #[test]
    fn list_dto_schema_has_no_password_field() {
        let item = BrowserCredentialItem {
            url: "https://example.com".into(),
            username: "u1".into(),
            has_password: true,
            credential_id: "opaque-handle".into(),
            origin: "https://example.com".into(),
        };
        let value = serde_json::to_value(item).unwrap();
        let obj = value.as_object().expect("列表 DTO 必须是对象");
        let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec!["credential_id", "has_password", "origin", "url", "username"],
            "列表 DTO 只允许非敏感字段，严禁 password/secret/credential key"
        );
        let json = serde_json::to_string(&value).unwrap();
        assert!(!json.contains("password\": \""), "DTO 不得携带密码值字段");
    }

    // ---- origin 精确匹配（攻击场景 1/2/3）----

    fn origin_of(u: &str) -> Option<String> {
        credential_origin(u)
    }

    #[test]
    fn origin_matches_same_site_paths_and_hash() {
        let cred = origin_of("https://sql.ainfinit.com").unwrap();
        // 场景 1：同站点的 hash / path 差异视为同一 origin → ALLOW
        assert_eq!(origin_of("https://sql.ainfinit.com/#/login").unwrap(), cred);
        assert_eq!(
            origin_of("https://sql.ainfinit.com/user/login").unwrap(),
            cred
        );
        assert_eq!(origin_of("https://sql.ainfinit.com:443/x").unwrap(), cred);
        assert_eq!(
            origin_of("https://SQL.AINFINIT.COM/x").unwrap(),
            cred,
            "host 大小写不敏感"
        );
    }

    #[test]
    fn origin_rejects_lookalike_domains() {
        let cred = origin_of("https://sql.ainfinit.com").unwrap();
        // 场景 2：后缀伪装
        assert_ne!(
            origin_of("https://sql.ainfinit.com.evil.com").unwrap(),
            cred
        );
        // 场景 3：前缀伪装 / 子域
        assert_ne!(origin_of("https://evil-sql.ainfinit.com").unwrap(), cred);
        assert_ne!(
            origin_of("https://sub.sql.ainfinit.com").unwrap(),
            cred,
            "不做子域共享"
        );
        assert_ne!(
            origin_of("https://ainfinit.com").unwrap(),
            cred,
            "不做父域共享"
        );
        assert_ne!(
            origin_of("http://sql.ainfinit.com").unwrap(),
            cred,
            "scheme 必须一致"
        );
        assert_ne!(
            origin_of("https://sql.ainfinit.com:8443").unwrap(),
            cred,
            "端口必须一致"
        );
        assert_ne!(origin_of("https://ainfinit.com.evil.org").unwrap(), cred);
    }

    #[test]
    fn origin_keeps_explicit_non_default_port_and_rejects_non_http() {
        assert_eq!(
            origin_of("http://example.com:8080").unwrap(),
            "http://example.com:8080"
        );
        assert_eq!(
            origin_of("http://example.com:80").unwrap(),
            "http://example.com"
        );
        assert_eq!(
            origin_of("https://example.com:443").unwrap(),
            "https://example.com"
        );
        // 非 http/https 一律拒绝（None 永不匹配）
        assert_eq!(origin_of("file:///etc/passwd"), None);
        assert_eq!(origin_of("about:blank"), None);
        assert_eq!(origin_of("not a url"), None);
        assert_eq!(origin_of(""), None);
    }

    #[test]
    fn autofill_script_escapes_secrets_and_never_submits() {
        let pw = "Fake\"\\Pass\nword";
        let js = build_autofill_script("https://sql.ainfinit.com", "test_user_001", pw);
        // 密码被 JSON 转义，不会破坏 JS 字面量（原文含引号/反斜杠/换行）
        assert!(!js.contains("Fake\"\\Pass\nword"), "密码必须以转义形式嵌入");
        assert!(
            js.contains("Fake\\\"\\\\Pass\\nword"),
            "引号/反斜杠/换行须转义"
        );
        // origin 硬编码进脚本，页面内会再次校验
        assert!(js.contains("https://sql.ainfinit.com"));
        // 绝不自动提交
        assert!(!js.contains(".submit("), "脚本不得提交表单");
        assert!(!js.contains("requestSubmit"), "脚本不得请求提交");
        assert!(!js.contains("click()"), "脚本不得点击按钮");
        assert!(!js.contains("Enter"), "脚本不得派发回车");
        assert!(
            !js.contains("dispatchEvent(new KeyboardEvent"),
            "脚本不得模拟键盘"
        );
        // 兼容受控输入：native setter + input/change
        assert!(
            js.contains("getOwnPropertyDescriptor"),
            "须走原生 value setter"
        );
        assert!(js.contains("'input'") && js.contains("'change'"));
    }

    #[test]
    fn merge_dedups_by_key_and_refreshes_timestamp() {
        let mut list = vec![meta("k1", "old"), meta("k2", "old")];
        // 重复导入同一 key：不得新增记录，只更新 imported_at
        merge_credential_metas(&mut list, &[meta("k1", "new")]);
        assert_eq!(list.len(), 2, "按 key 去重，重复导入不得产生重复账号记录");
        let first = list.iter().find(|m| m.key == "k1").unwrap();
        assert_eq!(first.imported_at, "new", "重复导入应刷新 imported_at");
        // 新 key：追加
        merge_credential_metas(&mut list, &[meta("k3", "new")]);
        assert_eq!(list.len(), 3, "新账号应追加");
        // 顺序稳定（按 key 排序），便于展示
        assert_eq!(
            list.iter().map(|m| m.key.as_str()).collect::<Vec<_>>(),
            vec!["k1", "k2", "k3"]
        );
    }

    #[test]
    fn index_file_persists_without_password() {
        let uniq = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("mvp-cred-index-test-{uniq}"));
        let path = dir.join("browser-credentials.json");
        let metas = vec![
            meta("browser:https://example.com:test_user_001", "t1"),
            meta("browser:https://example.org:test_user_002", "t1"),
        ];
        merge_credential_index_at(&path, &metas).expect("索引首次落盘应成功");
        // 重复导入同一 key：文件内容仍只有 2 条
        merge_credential_index_at(
            &path,
            &[meta("browser:https://example.com:test_user_001", "t2")],
        )
        .expect("索引去重叠加应成功");

        let raw = std::fs::read_to_string(&path).expect("索引文件应可读");
        assert!(
            !raw.to_lowercase().contains("password"),
            "索引文件不得出现 password 字样（当前内容结构：{raw}）"
        );
        assert!(!raw.contains("FakePassword"), "索引文件不得出现任何密码值");
        let list: Vec<BrowserCredentialMeta> =
            workspace::load_json_list_at(&path, "browser-credentials");
        assert_eq!(list.len(), 2, "按 key 去重后仍应为 2 条");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 虚构凭据端到端（**默认 ignore**，需显式运行）：
    /// `cargo test fictional_keyring_roundtrip_cleans_up -- --ignored`
    ///
    /// 只触碰 `browser:https://example.com:test_user_001` 与
    /// `browser:https://example.org:test_user_002` 两个明确的测试 key，
    /// 写完后立即删除，**绝不读取/修改/删除任何真实凭据**。
    #[test]
    #[ignore = "需要真实 keyring 与解锁状态；显式运行并自带清理"]
    fn fictional_keyring_roundtrip_cleans_up() {
        let rows = vec![
            BrowserCredentialRow {
                url: "https://example.com".into(),
                username: "test_user_001".into(),
                password: "FakePassword_001".into(),
            },
            BrowserCredentialRow {
                url: "https://example.org".into(),
                username: "test_user_002".into(),
                password: "FakePassword_002".into(),
            },
        ];
        let out = import_browser_credentials_inner(&rows);
        if out.saved == 0 {
            // 先清理再跳过
            for m in &out.metas {
                let _ = KeyringStore::delete_token(&m.key);
            }
            eprintln!("[credential] 跳过 keyring 往返断言：当前环境无可用密钥库");
            return;
        }
        assert_eq!(out.saved, 2, "两条虚构凭据都应写入 keyring");
        assert_eq!(out.metas.len(), 2, "每条成功凭据产生一条非敏感元数据");
        for m in &out.metas {
            assert!(m.key.starts_with("browser:"), "须与 db:/git 命名空间隔离");
            assert!(!m.key.contains("FakePassword"), "元数据绝不含密码");
            assert!(m.username.starts_with("test_user_"), "元数据只含用户名");
            // 关键：必须能跨 Entry 实例读回，否则是 mock/密钥库未启用
            let pwd = KeyringStore::get_token_result(&m.key).expect("刚写入的虚构凭据必须能读回");
            assert!(pwd.starts_with("FakePassword_"), "读回内容应与写入一致");
        }
        // 断言通过后再清理，确保失败时还能人工核查
        for m in &out.metas {
            let _ = KeyringStore::delete_token(&m.key);
        }
    }

    /// 回归：中文用户名 + https 尾斜杠 key 在真实 secret-service 上可往返。
    #[test]
    #[ignore = "需要真实 keyring 与解锁状态；显式运行并自带清理"]
    fn fictional_keyring_chinese_username_roundtrip() {
        let url = "https://sql.example.com/";
        let username = "测试员";
        let key = browser_credential_key(url, username);
        assert_eq!(key, "browser:https://sql.example.com/:测试员");

        KeyringStore::save_token(&key, "FakePassword_中文").expect("save 应成功");
        let got = KeyringStore::get_token_result(&key);
        let _ = KeyringStore::delete_token(&key);
        match got {
            Ok(p) => assert_eq!(p, "FakePassword_中文"),
            Err(keyring::Error::NoEntry) => panic!("save 成功但 get 返回 NoEntry"),
            Err(e) => panic!("get 失败: {e}"),
        }
    }
}
