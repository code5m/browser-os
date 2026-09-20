#!/usr/bin/env python3
"""Expose the M1-9 session persistence & close-protocol invariants as a fixture.

M1-9 把会话存档（BrowserSession）与关闭协议接入既有 M1-8 资源瀑布。本脚本守住：

  后端（Rust）—— 与会话持久化安全相关，不受前端关闭语义改动影响：
  - BrowserSession/SessionDraft 持久化白名单：不得含 token/cookie/authorization/
    headers/body/raw 等字段（黑名单字段结构性不存在）
  - 落盘前 URL 必须经 `redact_sensitive_url`；预览必须经 `scrub_preview` 脱敏+截断
  - 原子写（.tmp + rename）；单会话资源上限与全局会话容量上限存在
  - 10 个会话命令全部过 check_invocation_source；审计格式串不含 URL/预览内容
  - ACL（permissions/default-commands.toml）与 invoke_handler 同步
  - 生命周期：create_tab 建草稿、close_tab 清草稿、flush-sessions 注册在
    close-tabs 之前、启动时 prune_tmp_files（异常退出恢复边界）
  - 默认策略：close_prompt=true（后端契约保留）、auto_save_on_exit=false

  前端（TS/Vue）—— Owner 最终裁决（2026-09-12）后：
  - 普通 Tab 关闭 = 不弹确认框 + 不持久化 + 直接关闭。
  - 关闭入口只把 {url,title} 写入 recentlyClosed 内存栈，再调 closeTabNow
    完成 WebView 生命周期关闭。
  - 原关闭协议（SessionCloseDialog / bindCloseInterceptor / requestClose /
    resolveClose / pendingCloseTabId / closeDialogOpen）必须已撤销、不得残留
    （防止普通关闭再次触发 prompt 或 sessionSave 的确定性门禁）。
  - 手动"保存会话"（saveTab）仍独立存在、与关闭解耦。
  - recentlyClosed 内存栈 + restoreRecent（Ctrl+Shift+T）必须存在。

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

SESSION_COMMANDS = (
    "session_save",
    "session_discard",
    "session_list",
    "session_get",
    "session_delete",
    "session_export",
    "session_restore",
    "flush_sessions",
    "get_session_policy",
    "set_session_policy",
)

FORBIDDEN_DTO_FIELDS = (
    "token",
    "cookie",
    "authorization",
    "set_cookie",
    "headers",
    "body",
    "raw",
)


def strip_line_comments(source: str) -> str:
    return "\n".join(line.split("//", 1)[0] for line in source.splitlines())


def rust_fn_body(source: str, name: str) -> str:
    # 同时支持 Rust（`fn name(`）与 TS（`[async] function name(`）的函数体提取
    match = re.search(rf"\b(?:fn|function)\s+{re.escape(name)}\s*\(", source)
    if not match:
        return ""
    depth = 0
    index = match.end() - 1
    while index < len(source):
        if source[index] == "(":
            depth += 1
        elif source[index] == ")":
            depth -= 1
            if depth == 0:
                break
        index += 1
    start = source.find("{", index)
    if start < 0:
        return ""
    depth = 0
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[start : index + 1]
    return source[start:]


def rust_struct_body(source: str, name: str) -> str:
    match = re.search(rf"\bstruct\s+{re.escape(name)}\b[^{{]*\{{", source)
    if not match:
        return ""
    start = source.find("{", match.start())
    depth = 0
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[start : index + 1]
    return source[start:]


def detect_violations(files: dict[str, str]) -> list[str]:
    v: list[str] = []
    domain = files.get("domain", "")
    session_rs = files.get("session_rs", "")
    bridge = files.get("bridge", "")
    main_rs = files.get("main_rs", "")
    acl = files.get("acl", "")
    browser_store = files.get("browser_store", "")
    session_store = files.get("session_store", "")
    panel = files.get("panel", "")
    app_vue = files.get("app_vue", "")
    bridge_ts = files.get("bridge_ts", "")
    types_ts = files.get("types_ts", "")

    # ---- 1) 持久化白名单：DTO 不得含黑名单字段 ----
    for struct_name in ("BrowserSession", "SessionDraft"):
        body = strip_line_comments(rust_struct_body(domain, struct_name))
        if not body:
            v.append(f"SP_DTO_MISSING: {struct_name}")
            continue
        for field in FORBIDDEN_DTO_FIELDS:
            if re.search(rf"\b{field}\b", body, re.IGNORECASE):
                v.append(f"SP_DTO_BLACKLIST_FIELD: {struct_name}.{field}")

    # ---- 2) 落盘前脱敏 + 预览 scrub ----
    build_body = rust_fn_body(session_rs, "build_session")
    if not build_body:
        v.append("SP_BUILD_MISSING")
    else:
        if "redact_sensitive_url" not in build_body:
            v.append("SP_URL_NOT_REDACTED")
        if "SESSION_MAX_RESOURCES" not in build_body:
            v.append("SP_RESOURCE_CAP_MISSING")
    scrub_body = rust_fn_body(session_rs, "scrub_preview")
    if not scrub_body or "truncate_bytes" not in scrub_body:
        v.append("SP_PREVIEW_SCRUB_MISSING")

    # ---- 3) 原子写 + 容量 ----
    atomic_body = rust_fn_body(session_rs, "atomic_write")
    if not atomic_body:
        v.append("SP_ATOMIC_WRITE_MISSING")
    else:
        if ".tmp" not in atomic_body or "rename" not in atomic_body:
            v.append("SP_ATOMIC_WRITE_NOT_ATOMIC")
    if "fn prune_sessions" not in session_rs or "fn prune_tmp_files" not in session_rs:
        v.append("SP_PRUNE_MISSING")

    # ---- 4) 命令来源校验 + 审计不含 URL ----
    for cmd in SESSION_COMMANDS:
        body = rust_fn_body(bridge, cmd)
        if not body:
            v.append(f"SP_COMMAND_MISSING: {cmd}")
            continue
        if "check_invocation_source" not in body:
            v.append(f"SP_SOURCE_CHECK_MISSING: {cmd}")
        for line in body.splitlines():
            if "log_audit" in line or "format!" in line:
                code = line.split("//", 1)[0]
                if re.search(r"\burl\b", code, re.IGNORECASE):
                    v.append(f"SP_AUDIT_LEAKS_URL: {cmd}")
                if re.search(r"preview\s*=\s*\{?preview", code):
                    v.append(f"SP_AUDIT_LEAKS_PREVIEW: {cmd}")

    # ---- 5) ACL 与 handler 同步 ----
    for cmd in SESSION_COMMANDS:
        if f'"{cmd}"' not in acl:
            v.append(f"SP_ACL_MISSING: {cmd}")
        if f"bridge::{cmd}" not in main_rs:
            v.append(f"SP_HANDLER_NOT_REGISTERED: {cmd}")

    # ---- 6) 生命周期接线 ----
    create_body = rust_fn_body(bridge, "create_tab")
    if "upsert_session_draft" not in create_body:
        v.append("SP_DRAFT_ON_CREATE_MISSING")
    close_body = rust_fn_body(bridge, "close_tab")
    if "session_drafts" not in close_body or "remove" not in close_body:
        v.append("SP_DRAFT_ON_CLOSE_MISSING")
    shutdown_body = rust_fn_body(bridge, "register_shutdown_tasks")
    flush_idx = shutdown_body.find('"flush-sessions"')
    close_idx = shutdown_body.find('"close-tabs"')
    if flush_idx < 0:
        v.append("SP_FLUSH_TASK_MISSING")
    elif close_idx < 0 or flush_idx > close_idx:
        v.append("SP_FLUSH_AFTER_CLOSE_TABS")
    if "flush_sessions_inner" not in shutdown_body:
        v.append("SP_FLUSH_NOT_WIRED")
    if "prune_tmp_files" not in main_rs:
        v.append("SP_STARTUP_PRUNE_MISSING")

    # ---- 7) 后端默认策略（契约保留：close_prompt=true / auto_save_on_exit=false） ----
    policy_body = rust_struct_body(domain, "SessionPolicy")
    default_body = re.search(r"impl Default for SessionPolicy \{[^}]*\}", domain, re.DOTALL)
    if not default_body:
        v.append("SP_POLICY_DEFAULT_MISSING")
    else:
        text = default_body.group(0)
        if not re.search(r"close_prompt:\s*true", text):
            v.append("SP_CLOSE_PROMPT_DEFAULT_OFF")
        if not re.search(r"auto_save_on_exit:\s*false", text):
            v.append("SP_AUTO_SAVE_DEFAULT_ON")
    if not re.search(r"session_close_prompt\s*\n?\s*\.store\(true", main_rs):
        v.append("SP_CLOSE_PROMPT_SETUP_MISSING")

    # ---- 8) 普通 Tab 关闭：直接关闭，不得挂接任何关闭拦截器 / 确认框 ----
    # Owner 最终裁决（2026-09-12）：普通关闭不得走 prompt / sessionSave。
    # 任何残留的关闭协议符号都必须判违规（确定性门禁）。
    if "bindCloseInterceptor" in browser_store:
        v.append("SP_INTERCEPTOR_STILL_PRESENT")
    if "closeInterceptor" in browser_store:
        v.append("SP_INTERCEPTOR_STATE_STILL_PRESENT")
    tab_close_body = rust_fn_body(browser_store, "tabClose")
    if not tab_close_body:
        v.append("SP_TAB_CLOSE_MISSING")
    else:
        for sym in ("closeInterceptor", "requestClose", "resolveClose",
                    "pendingCloseTabId", "closeDialogOpen"):
            if sym in tab_close_body:
                v.append(f"SP_TAB_CLOSE_REFERENCES_PROTOCOL: {sym}")
        if "closeTabNow" not in tab_close_body:
            v.append("SP_TAB_CLOSE_NO_CLOSE_NOW")
    if "SessionCloseDialog" in app_vue:
        v.append("SP_DIALOG_STILL_MOUNTED")
    if "bindCloseInterceptor" in app_vue:
        v.append("SP_INTERCEPTOR_STILL_BOUND")

    # ---- 9) 原关闭协议 resolveClose 必须已撤销；手动保存仍独立存在 ----
    resolve_body = rust_fn_body(session_store, "resolveClose")
    if resolve_body:
        v.append("SP_RESOLVE_CLOSE_STILL_PRESENT")
    save_tab_body = rust_fn_body(session_store, "saveTab")
    if not save_tab_body:
        v.append("SP_SAVE_TAB_MISSING")
    elif "sessionSave" not in save_tab_body:
        v.append("SP_SAVE_TAB_NO_PERSIST")

    # ---- 10) 前端不得绕过 bridge.ts 直连会话命令 ----
    direct = re.compile(r"invoke[<(]\s*[\"'](?:%s)[\"']" % "|".join(SESSION_COMMANDS))
    for extra_path, extra_text in files.get("src_files", []):  # type: ignore[union-attr]
        if extra_path.endswith("bridge.ts"):
            continue
        if direct.search(extra_text):
            v.append(f"SP_FRONTEND_DIRECT_INVOKE: {extra_path}")

    # ---- 11) 会话 UI 零凭据标识符 ----
    for key, text in (("session_store", session_store), ("panel", panel)):
        code = strip_line_comments(text)
        for bad in ("token", "cookie", "authorization"):
            if re.search(rf"\b{bad}\b", code, re.IGNORECASE):
                v.append(f"SP_FRONTEND_CREDENTIAL_TOKEN: {key}.{bad}")

    # ---- 12) 前端接线完整性（保留手动保存能力 + recentlyClosed 内存栈） ----
    for method in (
        "sessionSave",
        "sessionDiscard",
        "sessionList",
        "sessionGet",
        "sessionDelete",
        "sessionExport",
        "sessionRestore",
        "flushSessions",
        "getSessionPolicy",
        "setSessionPolicy",
    ):
        if method not in bridge_ts:
            v.append(f"SP_BRIDGE_TS_MISSING: {method}")
    if "flushSessions" not in app_vue:
        v.append("SP_FLUSH_ON_UNLOAD_MISSING")
    # recentlyClosed 内存栈 + 恢复入口必须存在（Ctrl+Shift+T）
    if "recentlyClosed" not in browser_store:
        v.append("SP_RECENTLY_CLOSED_MISSING")
    if "restoreRecent" not in browser_store:
        v.append("SP_RESTORE_RECENT_MISSING")
    # 前端类型契约不再要求已撤销的 SessionCloseChoice

    return v


def scan_repository(root: Path) -> list[str]:
    mapping = {
        "domain": root / "src-tauri/src/domain.rs",
        "session_rs": root / "src-tauri/src/session.rs",
        "bridge": root / "src-tauri/src/bridge.rs",
        "main_rs": root / "src-tauri/src/main.rs",
        "acl": root / "src-tauri/permissions/default-commands.toml",
        "browser_store": root / "src/capabilities/browser/state/useBrowserStore.ts",
        "session_store": root / "src/stores/useSessionStore.ts",
        "panel": root / "src/components/browser/SessionPanel.vue",
        "app_vue": root / "src/App.vue",
        "bridge_ts": root / "src/bridge.ts",
        "types_ts": root / "src/types.ts",
    }
    files: dict[str, str] = {}
    for key, path in mapping.items():
        if not path.exists():
            return [f"SP_FILE_MISSING: {path.relative_to(root)}"]
        files[key] = path.read_text(encoding="utf-8")
    src_files: list[tuple[str, str]] = []
    for path in sorted((root / "src").rglob("*")):
        if path.is_file() and path.suffix in {".ts", ".vue"}:
            src_files.append((str(path.relative_to(root)), path.read_text(encoding="utf-8")))
    files["src_files"] = src_files  # type: ignore[assignment]
    return detect_violations(files)


# --------------------------------------------------------------------------
# 自检夹具
# --------------------------------------------------------------------------

GOOD_DOMAIN = """
pub struct BrowserSession {
    pub id: String,
    pub tab_id: String,
    pub url: String,
    pub resources: Vec<ResourceReceived>,
}
pub struct SessionDraft {
    pub tab_id: String,
    pub url: String,
    pub title: String,
}
pub struct SessionPolicy {
    pub close_prompt: bool,
    pub auto_save_on_exit: bool,
}
impl Default for SessionPolicy {
    fn default() -> Self {
        SessionPolicy {
            close_prompt: true,
            auto_save_on_exit: false,
        }
    }
}
"""

GOOD_SESSION_RS = """
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
pub fn scrub_preview(text: &str) -> (String, bool) {
    let redacted = redact_urls_in_text(text);
    truncate_bytes(&redacted, SESSION_PREVIEW_MAX_BYTES)
}
pub fn build_session(tab_id: &str, url: &str, title: &str, preview: &str, resources: &[ResourceReceived], close_reason: &str) -> BrowserSession {
    let url = crate::security_policy::redact_sensitive_url(url);
    let kept = if resources.len() > SESSION_MAX_RESOURCES { resources[..SESSION_MAX_RESOURCES].to_vec() } else { resources.to_vec() };
    make()
}
pub fn prune_sessions(dir: &Path, keep: usize) -> usize { 0 }
pub fn prune_tmp_files(dir: &Path) -> usize { 0 }
"""

GOOD_BRIDGE = """
fn create_tab(app: AppHandle, url: &str) -> Result<TabInfo, String> {
    upsert_session_draft(&app, &id, &target, &title0);
    Ok(info)
}
fn close_tab(app: &AppHandle, id: &str) -> Result<(), String> {
    state.session_drafts.lock().unwrap().remove(id);
    Ok(())
}
pub fn register_shutdown_tasks(app: &AppHandle) -> Result<(), String> {
    coordinator.register("flush-sessions", move || {
        let report = flush_sessions_inner(&app);
        Ok(())
    })?;
    coordinator.register("close-tabs", move || { Ok(()) })?;
    Ok(())
}
pub fn session_save(app: AppHandle, webview: tauri::Webview, tab_id: String, preview: Option<String>) -> Result<SessionSummary, String> {
    check_invocation_source(&webview, "session_save", None, &app)?;
    workspace::log_audit(&app, "session_save", format!("tab_id={} resources={} preview_bytes={}", tab_id, 1, 2));
    Ok(x)
}
pub fn session_discard(app: AppHandle, webview: tauri::Webview, tab_id: String) -> Result<(), String> {
    check_invocation_source(&webview, "session_discard", None, &app)?;
    workspace::log_audit(&app, "session_discard", format!("tab_id={tab_id} draft_existed={existed}"));
    Ok(())
}
pub fn session_list(app: AppHandle, webview: tauri::Webview) -> Result<Vec<SessionSummary>, String> {
    check_invocation_source(&webview, "session_list", None, &app)?;
    Ok(vec![])
}
pub fn session_get(app: AppHandle, webview: tauri::Webview, id: String) -> Result<BrowserSession, String> {
    check_invocation_source(&webview, "session_get", None, &app)?;
    Ok(x)
}
pub fn session_delete(app: AppHandle, webview: tauri::Webview, id: String) -> Result<bool, String> {
    check_invocation_source(&webview, "session_delete", None, &app)?;
    workspace::log_audit(&app, "session_delete", format!("id={id} removed={removed}"));
    Ok(true)
}
pub fn session_export(app: AppHandle, webview: tauri::Webview, id: String) -> Result<String, String> {
    check_invocation_source(&webview, "session_export", None, &app)?;
    workspace::log_audit(&app, "session_export", format!("id={id} bytes={}", 1));
    Ok(String::new())
}
pub fn session_restore(app: AppHandle, webview: tauri::Webview, id: String) -> Result<TabInfo, String> {
    check_invocation_source(&webview, "session_restore", None, &app)?;
    workspace::log_audit(&app, "session_restore", format!("id={id} tab_id={} resources={}", t.id, 1));
    Ok(x)
}
pub fn flush_sessions(app: AppHandle, webview: tauri::Webview) -> Result<SessionFlushReport, String> {
    check_invocation_source(&webview, "flush_sessions", None, &app)?;
    workspace::log_audit(&app, "session_flush", format!("auto_save={a} persisted={p} drafts_dropped={d} tmp_removed={t}"));
    Ok(x)
}
pub fn get_session_policy(app: AppHandle, webview: tauri::Webview) -> Result<SessionPolicy, String> {
    check_invocation_source(&webview, "get_session_policy", None, &app)?;
    Ok(x)
}
pub fn set_session_policy(app: AppHandle, webview: tauri::Webview, close_prompt: Option<bool>, auto_save_on_exit: Option<bool>) -> Result<SessionPolicy, String> {
    check_invocation_source(&webview, "set_session_policy", None, &app)?;
    workspace::log_audit(&app, "session_policy", format!("close_prompt={} auto_save_on_exit={}", a, b));
    Ok(x)
}
"""

GOOD_MAIN_RS = """
bridge::session_save,
bridge::session_discard,
bridge::session_list,
bridge::session_get,
bridge::session_delete,
bridge::session_export,
bridge::session_restore,
bridge::flush_sessions,
bridge::get_session_policy,
bridge::set_session_policy,
app.state::<AppState>()
    .session_close_prompt
    .store(true, Ordering::SeqCst);
let tmp = crate::session::prune_tmp_files(&dir);
"""

GOOD_ACL = """
commands.allow = [
    "session_save",
    "session_discard",
    "session_list",
    "session_get",
    "session_delete",
    "session_export",
    "session_restore",
    "flush_sessions",
    "get_session_policy",
    "set_session_policy"
]
"""

GOOD_BROWSER_STORE = """
const recentlyClosed = [];
async function tabClose(id) {
  await closeTabNow(id);
}
async function closeTabNow(id) {
  recordClose(id);
  await bridge.tabClose(id);
}
async function restoreRecent() {
  const item = recentlyClosed.shift();
  if (item) await tabNew(item.url);
}
"""

GOOD_SESSION_STORE = """
async function saveTab(tabId, preview) {
  await bridge.sessionSave(tabId, preview);
}
"""

GOOD_PANEL = "<button @click=\"session.saveTab('tab-1')\">保存</button>"

GOOD_APP_VUE = """
window.addEventListener("beforeunload", () => bridge.flushSessions().catch(() => {}));
"""

GOOD_TYPES_TS = "export interface RecentlyClosedEntry { url: string; title: string; }"

GOOD_BRIDGE_TS = """
sessionSave: (tabId: string, preview?: string) => invoke<SessionSummary>("session_save", { tabId, preview: preview ?? null }),
sessionDiscard: (tabId: string) => invoke("session_discard", { tabId }),
sessionList: () => invoke<SessionSummary[]>("session_list"),
sessionGet: (id: string) => invoke<BrowserSession>("session_get", { id }),
sessionDelete: (id: string) => invoke<boolean>("session_delete", { id }),
sessionExport: (id: string) => invoke<string>("session_export", { id }),
sessionRestore: (id: string) => invoke<TabInfo>("session_restore", { id }),
flushSessions: () => invoke<SessionFlushReport>("flush_sessions"),
getSessionPolicy: () => invoke<SessionPolicy>("get_session_policy"),
setSessionPolicy: (closePrompt?: boolean) => invoke<SessionPolicy>("set_session_policy", { closePrompt }),
"""

GOOD_SRC_FILES = [
    ("src/bridge.ts", GOOD_BRIDGE_TS),
    ("src/stores/useSessionStore.ts", GOOD_SESSION_STORE),
]


def run_self_test() -> int:
    good = {
        "domain": GOOD_DOMAIN,
        "session_rs": GOOD_SESSION_RS,
        "bridge": GOOD_BRIDGE,
        "main_rs": GOOD_MAIN_RS,
        "acl": GOOD_ACL,
        "browser_store": GOOD_BROWSER_STORE,
        "session_store": GOOD_SESSION_STORE,
        "panel": GOOD_PANEL,
        "app_vue": GOOD_APP_VUE,
        "bridge_ts": GOOD_BRIDGE_TS,
        "types_ts": GOOD_TYPES_TS,
        "src_files": GOOD_SRC_FILES,
    }
    failures: list[str] = []
    baseline = detect_violations(good)
    if baseline:
        failures.append(f"good fixture should be clean, got {baseline}")

    cases: list[tuple[str, dict, str]] = [
        ("DTO 含 cookie 字段",
         {"domain": GOOD_DOMAIN.replace("pub url: String,\n    pub resources", "pub cookie: String,\n    pub resources")},
         "SP_DTO_BLACKLIST_FIELD"),
        ("落盘前未脱敏",
         {"session_rs": GOOD_SESSION_RS.replace("crate::security_policy::redact_sensitive_url(url)", "url.to_string()")},
         "SP_URL_NOT_REDACTED"),
        ("原子写退化为直接写",
         {"session_rs": GOOD_SESSION_RS.replace('let tmp = path.with_extension("json.tmp");\n    fs::write(&tmp, content)?;\n    fs::rename(&tmp, path)?;', "fs::write(path, content)?;")},
         "SP_ATOMIC_WRITE_NOT_ATOMIC"),
        ("命令缺来源校验",
         {"bridge": GOOD_BRIDGE.replace('check_invocation_source(&webview, "session_save", None, &app)?;\n    ', "")},
         "SP_SOURCE_CHECK_MISSING"),
        ("审计含 URL",
         {"bridge": GOOD_BRIDGE.replace('format!("tab_id={tab_id} draft_existed={existed}")', 'format!("url={url}")')},
         "SP_AUDIT_LEAKS_URL"),
        ("ACL 缺命令",
         {"acl": GOOD_ACL.replace('"session_save",\n    ', "")},
         "SP_ACL_MISSING"),
        ("handler 未注册",
         {"main_rs": GOOD_MAIN_RS.replace("bridge::session_save,\n", "")},
         "SP_HANDLER_NOT_REGISTERED"),
        ("flush 注册在 close-tabs 之后",
         {"bridge": GOOD_BRIDGE.replace('coordinator.register("flush-sessions", move || {\n        let report = flush_sessions_inner(&app);\n        Ok(())\n    })?;\n    coordinator.register("close-tabs"', 'coordinator.register("close-tabs", move || { Ok(()) })?;\n    coordinator.register("flush-sessions", move || {\n        let report = flush_sessions_inner(&app);\n        Ok(())\n    })?;\n    coordinator.register("close-tabs-x"')},
         "SP_FLUSH_AFTER_CLOSE_TABS"),
        ("create_tab 不建草稿",
         {"bridge": GOOD_BRIDGE.replace("upsert_session_draft(&app, &id, &target, &title0);", "")},
         "SP_DRAFT_ON_CREATE_MISSING"),
        ("关闭弹窗默认关（后端契约）",
         {"domain": GOOD_DOMAIN.replace("close_prompt: true", "close_prompt: false")},
         "SP_CLOSE_PROMPT_DEFAULT_OFF"),
        ("退出自动保存默认开（后端契约）",
         {"domain": GOOD_DOMAIN.replace("auto_save_on_exit: false", "auto_save_on_exit: true")},
         "SP_AUTO_SAVE_DEFAULT_ON"),
        # ===== 新裁决门禁：不得残留关闭协议 =====
        ("tabClose 仍挂接 bindCloseInterceptor",
         {"browser_store": GOOD_BROWSER_STORE + "\nfunction bindCloseInterceptor(fn) {}\n"},
         "SP_INTERCEPTOR_STILL_PRESENT"),
        ("tabClose 仍引用关闭协议符号",
         {"browser_store": GOOD_BROWSER_STORE.replace(
             "async function tabClose(id) {\n  await closeTabNow(id);\n}",
             "async function tabClose(id) {\n  if (requestClose(id)) return;\n  await closeTabNow(id);\n}")},
         "SP_TAB_CLOSE_REFERENCES_PROTOCOL"),
        ("App 仍挂载 SessionCloseDialog",
         {"app_vue": GOOD_APP_VUE + "\n<SessionCloseDialog />\n"},
         "SP_DIALOG_STILL_MOUNTED"),
        ("resolveClose 仍存在",
         {"session_store": GOOD_SESSION_STORE + '\nasync function resolveClose(choice) {\n  await browser.closeTabNow("x");\n}'},
         "SP_RESOLVE_CLOSE_STILL_PRESENT"),
        ("前端绕过 bridge 直连",
         {"src_files": GOOD_SRC_FILES + [("src/components/X.vue", 'invoke("session_save", { tabId })')]},
         "SP_FRONTEND_DIRECT_INVOKE"),
        ("会话 UI 出现凭据标识符",
         {"panel": GOOD_PANEL + "\nconst token = 'x';"},
         "SP_FRONTEND_CREDENTIAL_TOKEN"),
        ("beforeunload 未 flush",
         {"app_vue": GOOD_APP_VUE.replace("bridge.flushSessions()", "bridge.closeBrowser()")},
         "SP_FLUSH_ON_UNLOAD_MISSING"),
        ("saveTab 缺失（手动保存能力被误删）",
         {"session_store": "async function loadSessions() {}\n"},
         "SP_SAVE_TAB_MISSING"),
        ("recentlyClosed 内存栈缺失",
         {"browser_store": "async function tabClose(id) {\n  await closeTabNow(id);\n}\nasync function closeTabNow(id) {\n  await bridge.tabClose(id);\n}\n"},
         "SP_RECENTLY_CLOSED_MISSING"),
    ]

    for label, overrides, expected in cases:
        merged = dict(good)
        merged.update(overrides)
        found = detect_violations(merged)
        if not any(item == expected or item.startswith(expected + ":") for item in found):
            failures.append(f"{label}: expected {expected}, got {found}")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print("self-test: ok")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Check M1-9 session persistence & close-protocol invariants."
    )
    parser.add_argument("--self-test", action="store_true", help="run built-in tests")
    args = parser.parse_args(argv)
    if args.self_test:
        return run_self_test()

    violations = scan_repository(Path(__file__).resolve().parent.parent)
    if violations:
        print(f"check-session-persistence-policy: failed ({len(violations)} violation(s))")
        for violation in violations:
            print(violation)
        return 1
    print("check-session-persistence-policy: ok (session persistence invariants hold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
