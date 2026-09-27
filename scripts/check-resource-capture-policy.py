#!/usr/bin/env python3
"""Expose the M1-8 resource capture privacy/capacity invariants as a reproducible fixture.

M1-8 资源瀑布把 WebKitGTK 原生资源信号接入前端瀑布视图。本脚本守住以下不变量，
防止后续改动悄悄破坏隐私红线或容量上限：

  后端（Rust / 插件）
  - DTO（domain.rs::ResourceReceived / 插件 models.rs::ResourceReceived 变体）
    不得含 cookie / authorization / set-cookie / headers / body 任何字段
  - URL 必须经 `security_policy::redact_sensitive_url` 脱敏（敏感键清单含
    token/password/secret/signature/cookie）后才入库
  - ResourceBuffer 必须同时执行 per-tab 与全局两级 FIFO 容量上限
  - close_tab 必须 remove_tab、ShutdownCoordinator close-tabs 必须 clear_all
  - 4 个命令必须过 check_invocation_source；审计不得含 URL
  - 4 个命令必须注册进 invoke_handler 与 permissions/default-commands.toml（ACL 同步）
  - 插件资源信号块不得 eprintln/println/log 任何 URL；原始事件只能经
    bridge::on_resource_received 处理（main.rs 不得直接转发原始 payload）

  前端（TS / Vue）
  - 除 bridge.ts 外不得直接 invoke 4 个资源命令
  - store / 组件不得出现 token/cookie/authorization 标识符（展示层零凭据）
  - browser 能力 index.ts 必须注册 ResourceWaterfall（BROWSER_DOCK view=net）；App.vue 必须订阅 resource-received

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

RESOURCE_COMMANDS = (
    "list_tab_resources",
    "clear_tab_resources",
    "get_resource_capture_settings",
    "set_resource_capture_settings",
)

SENSITIVE_KEY_MIN = ("token", "password", "secret", "signature", "cookie")

# DTO 禁区字段（结构级隐私红线）
FORBIDDEN_DTO_FIELDS = (
    "cookie",
    "authorization",
    "set_cookie",
    "headers",
    "body",
)


def strip_line_comments(source: str) -> str:
    """去掉 // 行注释（Rust/TS 通用，够用且不追踪字符串，避免复杂化）。"""
    return "\n".join(line.split("//", 1)[0] for line in source.splitlines())


def rust_fn_body(source: str, name: str) -> str:
    """提取 `fn <name>(...) { ... }` 的 body（含花括号）。"""
    match = re.search(rf"\bfn\s+{re.escape(name)}\s*\(", source)
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
    security = files.get("security", "")
    bridge = files.get("bridge", "")
    browser_commands = files.get("browser_commands", "")
    main_rs = files.get("main_rs", "")
    acl = files.get("acl", "")
    plugin_models = files.get("plugin_models", "")
    plugin_commands = files.get("plugin_commands", "")
    types_ts = files.get("types_ts", "")
    bridge_ts = files.get("bridge_ts", "")
    store = files.get("store", "")
    waterfall = files.get("waterfall", "")
    mainarea = files.get("mainarea", "")
    app_vue = files.get("app_vue", "")

    # ---- 1) DTO 结构红线：不含 cookie/authorization/headers/body 字段 ----
    dto_body = strip_line_comments(rust_struct_body(domain, "ResourceReceived"))
    if not dto_body:
        v.append("RC_DTO_MISSING")
    else:
        for field in FORBIDDEN_DTO_FIELDS:
            if re.search(rf"\b{field}\b", dto_body, re.IGNORECASE):
                v.append(f"RC_DTO_SENSITIVE_FIELD: domain.ResourceReceived.{field}")
    variant = re.search(
        r"ResourceReceived\s*\{[^}]*\}", plugin_models, re.DOTALL
    )
    if not variant:
        v.append("RC_PLUGIN_EVENT_MISSING")
    else:
        body = strip_line_comments(variant.group(0))
        for field in FORBIDDEN_DTO_FIELDS:
            if re.search(rf"\b{field}\b", body, re.IGNORECASE):
                v.append(f"RC_DTO_SENSITIVE_FIELD: plugin.ResourceReceived.{field}")

    # ---- 2) 脱敏函数存在且敏感键清单齐全 ----
    if "fn redact_sensitive_url" not in security:
        v.append("RC_REDACT_FN_MISSING")
    for key in SENSITIVE_KEY_MIN:
        if f'"{key}"' not in security:
            v.append(f"RC_SENSITIVE_KEY_MISSING: {key}")

    # ---- 3) 入库前必须脱敏 ----
    build_body = rust_fn_body(bridge, "build_resource_received")
    if not build_body:
        v.append("RC_BUILD_FN_MISSING")
    elif "redact_sensitive_url" not in build_body:
        v.append("RC_REDACT_NOT_APPLIED")

    # ---- 4) 容量上限：per-tab + 全局两级 FIFO ----
    push_body = rust_fn_body(bridge, "push")
    if not push_body:
        v.append("RC_BUFFER_PUSH_MISSING")
    else:
        if "max_per_tab" not in push_body:
            v.append("RC_PER_TAB_CAP_MISSING")
        if "max_total" not in push_body:
            v.append("RC_GLOBAL_CAP_MISSING")
        if push_body.count("pop_front") < 2:
            v.append("RC_FIFO_EVICT_MISSING")

    # ---- 5) 生命周期清理 ----
    close_tab_body = rust_fn_body(bridge, "close_tab")
    if "remove_tab" not in close_tab_body or "resource_buffer" not in close_tab_body:
        v.append("RC_TAB_CLOSE_CLEANUP_MISSING")
    shutdown_body = rust_fn_body(bridge, "register_shutdown_tasks")
    if "clear_all" not in shutdown_body or "resource_buffer" not in shutdown_body:
        v.append("RC_SHUTDOWN_CLEANUP_MISSING")

    # ---- 6) 命令来源校验 + 审计不含 URL ----
    # 注：4 个资源命令已于 PHASE A 从 bridge.rs 迁移至 capabilities/browser/commands.rs，
    # 此处扫描 browser_commands（真实物理位置），不再误扫 bridge.rs。
    for cmd in RESOURCE_COMMANDS:
        body = rust_fn_body(browser_commands, cmd)
        if not body:
            v.append(f"RC_COMMAND_MISSING: {cmd}")
            continue
        if "check_invocation_source" not in body:
            v.append(f"RC_SOURCE_CHECK_MISSING: {cmd}")
        if "log_audit" in body:
            for line in body.splitlines():
                if "log_audit" in line or "format!" in line:
                    code = line.split("//", 1)[0]
                    if re.search(r"\burl\b", code, re.IGNORECASE):
                        v.append(f"RC_AUDIT_LEAKS_URL: {cmd}")

    # ---- 7) ACL 同步 ----
    for cmd in RESOURCE_COMMANDS:
        if f'"{cmd}"' not in acl:
            v.append(f"RC_ACL_MISSING: {cmd}")

    # ---- 8) invoke_handler 注册 + main.rs 不直接转发原始 payload ----
    for cmd in RESOURCE_COMMANDS:
        # 注册路径可能为 legacy `bridge::{cmd}` 或 PHASE A 后
        # `crate::capabilities::browser::commands::{cmd}`，二者任一即视为已注册。
        if (
            f"bridge::{cmd}" not in main_rs
            and f"crate::capabilities::browser::commands::{cmd}" not in main_rs
        ):
            v.append(f"RC_HANDLER_NOT_REGISTERED: {cmd}")
    if '"resourceReceived"' not in main_rs:
        v.append("RC_EVENT_ARM_MISSING")
    if "on_resource_received" not in main_rs:
        v.append("RC_EVENT_NOT_ROUTED_TO_BRIDGE")
    if re.search(r'emit\(\s*"resource-received"', main_rs):
        v.append("RC_RAW_PAYLOAD_FORWARDED")

    # ---- 9) 插件资源信号块不得打印 URL ----
    sig_idx = plugin_commands.find("connect_resource_load_started")
    if sig_idx < 0:
        v.append("RC_PLUGIN_SIGNAL_MISSING")
    else:
        block = plugin_commands[sig_idx:]
        # 信号块内禁止任何打印宏（URL 可能带 token）
        for macro in ("eprintln!", "println!", "log::", "info!", "debug!"):
            if macro in block:
                v.append(f"RC_PLUGIN_LOGS_URL: {macro}")

    # ---- 10) 前端不得绕过 bridge.ts 直连命令 ----
    direct = re.compile(r"invoke[<(]\s*[\"'](?:%s)[\"']" % "|".join(RESOURCE_COMMANDS))
    for extra_path, extra_text in files.get("src_files", []):  # type: ignore[union-attr]
        if extra_path.endswith("bridge.ts"):
            continue
        if direct.search(extra_text):
            v.append(f"RC_FRONTEND_DIRECT_INVOKE: {extra_path}")

    # ---- 11) 前端展示层零凭据标识符 ----
    for key, text in (("store", store), ("waterfall", waterfall)):
        code = strip_line_comments(text)
        for bad in ("token", "cookie", "authorization"):
            if re.search(rf"\b{bad}\b", code, re.IGNORECASE):
                v.append(f"RC_FRONTEND_CREDENTIAL_TOKEN: {key}.{bad}")

    # ---- 12) 前端接线完整 ----
    if "ResourceReceived" not in types_ts:
        v.append("RC_TYPES_MISSING")
    for method in (
        "listTabResources",
        "clearTabResources",
        "getResourceCaptureSettings",
        "setResourceCaptureSettings",
        "onResourceReceived",
    ):
        if method not in bridge_ts:
            v.append(f"RC_BRIDGE_TS_MISSING: {method}")
    if "applyReceived" not in store or "clearTab" not in store or "setEnabled" not in store:
        v.append("RC_STORE_INCOMPLETE")
    if "ResourceWaterfall" not in mainarea:
        v.append("RC_WATERFALL_NOT_MOUNTED")
    if "onResourceReceived" not in app_vue:
        v.append("RC_EVENT_NOT_SUBSCRIBED")

    return v


def scan_repository(root: Path) -> list[str]:
    mapping = {
        "domain": root / "src-tauri/src/domain.rs",
        "security": root / "src-tauri/src/security_policy.rs",
        "bridge": root / "src-tauri/src/bridge.rs",
        "browser_commands": root / "src-tauri/src/capabilities/browser/commands.rs",
        "main_rs": root / "src-tauri/src/main.rs",
        "acl": root / "src-tauri/permissions/default-commands.toml",
        "plugin_models": root
        / "tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/models.rs",
        "plugin_commands": root
        / "tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs",
        "types_ts": root / "src/types.ts",
        "bridge_ts": root / "src/bridge.ts",
        "store": root / "src/capabilities/browser/state/useResourceStore.ts",
        "waterfall": root / "src/capabilities/browser/ui/ResourceWaterfall.vue",
        "mainarea": root / "src/capabilities/browser/index.ts",
        "app_vue": root / "src/App.vue",
    }
    files: dict[str, str] = {}
    for key, path in mapping.items():
        if not path.exists():
            return [f"RC_FILE_MISSING: {path.relative_to(root)}"]
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
pub struct ResourceReceived {
    pub id: String,
    pub tab_id: String,
    pub url: String,
    pub method: String,
    pub status: Option<u32>,
}
"""

GOOD_SECURITY = """
pub const SENSITIVE_QUERY_KEYS: [&str; 5] = ["token", "password", "secret", "signature", "cookie"];
pub fn redact_sensitive_url(raw: &str) -> String { raw.to_string() }
"""

GOOD_BRIDGE = """
pub fn build_resource_received(tab_id: &str, raw: &RawResourceEvent, max_url_bytes: usize) -> Option<ResourceReceived> {
    let url = crate::security_policy::redact_sensitive_url(&raw.url);
    None
}
impl ResourceBuffer {
    pub fn push(&mut self, rec: ResourceReceived, max_per_tab: usize, max_total: usize) {
        while q.len() > max_per_tab { q.pop_front(); }
        while self.total > max_total { self.order.pop_front(); }
    }
}
fn close_tab(app: &AppHandle, id: &str) -> Result<(), String> {
    state.resource_buffer.lock().unwrap().remove_tab(id);
    Ok(())
}
pub fn register_shutdown_tasks(app: &AppHandle) -> Result<(), String> {
    state.resource_buffer.lock().unwrap().clear_all();
    Ok(())
}
"""

# 4 个资源命令已于 PHASE A 从 bridge.rs 迁移至 capabilities/browser/commands.rs
GOOD_BROWSER_COMMANDS = """
pub fn list_tab_resources(app: AppHandle, webview: tauri::Webview, tab_id: String) -> Result<TabResourceList, String> {
    check_invocation_source(&webview, "list_tab_resources", None, &app)?;
    Ok(x)
}
pub fn clear_tab_resources(app: AppHandle, webview: tauri::Webview, tab_id: String) -> Result<(), String> {
    check_invocation_source(&webview, "clear_tab_resources", None, &app)?;
    workspace::log_audit(&app, "resource_clear", format!("tab_id={tab_id} removed={removed}"));
    Ok(())
}
pub fn get_resource_capture_settings(app: AppHandle, webview: tauri::Webview) -> Result<ResourceCaptureSettings, String> {
    check_invocation_source(&webview, "get_resource_capture_settings", None, &app)?;
    Ok(x)
}
pub fn set_resource_capture_settings(app: AppHandle, webview: tauri::Webview, enabled: bool, max_per_tab: Option<usize>) -> Result<ResourceCaptureSettings, String> {
    check_invocation_source(&webview, "set_resource_capture_settings", None, &app)?;
    workspace::log_audit(&app, "resource_capture", format!("enabled={enabled}"));
    Ok(x)
}
"""

GOOD_MAIN_RS = """
Some("resourceReceived") => {
    match serde_json::from_value::<bridge::RawResourceEvent>(payload.clone()) {
        Ok(raw) => bridge::on_resource_received(&forward, id, raw),
        Err(_) => {}
    }
}
crate::capabilities::browser::commands::list_tab_resources,
crate::capabilities::browser::commands::clear_tab_resources,
crate::capabilities::browser::commands::get_resource_capture_settings,
crate::capabilities::browser::commands::set_resource_capture_settings,
"""

GOOD_ACL = """
commands.allow = [
    "list_tab_resources",
    "clear_tab_resources",
    "get_resource_capture_settings",
    "set_resource_capture_settings"
]
"""

GOOD_PLUGIN_MODELS = """
pub enum BrowserTabEvent {
    ResourceReceived {
        id: TabId,
        url: String,
        method: Option<String>,
        status: Option<u32>,
    },
}
"""

GOOD_PLUGIN_COMMANDS = """
gtk_wv.connect_resource_load_started(move |_wv, resource, request| {
    let url = request.uri().map(|u| u.to_string()).unwrap_or_default();
    resource.connect_finished(move |r| {
        let _ = app.emit("browser-tabs://event", BrowserTabEvent::ResourceReceived {
            id: id.clone(), url: url.clone(), method: None, status: None,
        });
    });
});
"""

GOOD_TYPES_TS = "export interface ResourceReceived { id: string; }"

GOOD_BRIDGE_TS = """
listTabResources: (tabId: string) => invoke<TabResourceList>("list_tab_resources", { tabId }),
clearTabResources: (tabId: string) => invoke("clear_tab_resources", { tabId }),
getResourceCaptureSettings: () => invoke<ResourceCaptureSettings>("get_resource_capture_settings"),
setResourceCaptureSettings: (enabled: boolean) => invoke<ResourceCaptureSettings>("set_resource_capture_settings", { enabled }),
onResourceReceived: (cb) => listen<ResourceReceived>("resource-received", (e) => cb(e.payload)),
"""

GOOD_STORE = """
function applyReceived(rec) { byTab[rec.tab_id].push(rec); }
async function clearTab(tabId) { await bridge.clearTabResources(tabId); }
async function setEnabled(on) { await bridge.setResourceCaptureSettings(on); }
"""

GOOD_WATERFALL = """
<span>{{ r.status ?? "-" }}</span>
<span>{{ displayUrl(r.url) }}</span>
"""

GOOD_MAINAREA = '<ResourceWaterfall v-else-if="layout.browserDockTab === \'net\'" />'

GOOD_APP_VUE = "bridge.onResourceReceived((r) => resources.applyReceived(r));"

GOOD_SRC_FILES = [
    ("src/bridge.ts", GOOD_BRIDGE_TS),
    ("src/stores/useResourceStore.ts", GOOD_STORE),
]


def run_self_test() -> int:
    good = {
        "domain": GOOD_DOMAIN,
        "security": GOOD_SECURITY,
        "bridge": GOOD_BRIDGE,
        "browser_commands": GOOD_BROWSER_COMMANDS,
        "main_rs": GOOD_MAIN_RS,
        "acl": GOOD_ACL,
        "plugin_models": GOOD_PLUGIN_MODELS,
        "plugin_commands": GOOD_PLUGIN_COMMANDS,
        "types_ts": GOOD_TYPES_TS,
        "bridge_ts": GOOD_BRIDGE_TS,
        "store": GOOD_STORE,
        "waterfall": GOOD_WATERFALL,
        "mainarea": GOOD_MAINAREA,
        "app_vue": GOOD_APP_VUE,
        "src_files": GOOD_SRC_FILES,
    }
    failures: list[str] = []
    baseline = detect_violations(good)
    if baseline:
        failures.append(f"good fixture should be clean, got {baseline}")

    cases: list[tuple[str, dict, str]] = [
        ("DTO 含 cookie 字段",
         {"domain": GOOD_DOMAIN.replace(
             "    pub status: Option<u32>,\n}",
             "    pub status: Option<u32>,\n    pub cookie: String,\n}")},
         "RC_DTO_SENSITIVE_FIELD"),
        ("插件事件含 headers 字段",
         {"plugin_models": GOOD_PLUGIN_MODELS.replace("status: Option<u32>,", "status: Option<u32>,\n        headers: Vec<String>,")},
         "RC_DTO_SENSITIVE_FIELD"),
        ("敏感键清单缺 token",
         {"security": GOOD_SECURITY.replace('"token", ', "")},
         "RC_SENSITIVE_KEY_MISSING"),
        ("入库前未脱敏",
         {"bridge": GOOD_BRIDGE.replace("crate::security_policy::redact_sensitive_url(&raw.url)", "raw.url.clone()")},
         "RC_REDACT_NOT_APPLIED"),
        ("缺全局容量上限",
         {"bridge": GOOD_BRIDGE.replace("while self.total > max_total { self.order.pop_front(); }", "")},
         "RC_GLOBAL_CAP_MISSING"),
        ("tab 关闭未清理",
         {"bridge": GOOD_BRIDGE.replace("state.resource_buffer.lock().unwrap().remove_tab(id);", "")},
         "RC_TAB_CLOSE_CLEANUP_MISSING"),
        ("退出路径未清空",
         {"bridge": GOOD_BRIDGE.replace("state.resource_buffer.lock().unwrap().clear_all();", "")},
         "RC_SHUTDOWN_CLEANUP_MISSING"),
        ("命令缺来源校验",
         {"browser_commands": GOOD_BROWSER_COMMANDS.replace('check_invocation_source(&webview, "list_tab_resources", None, &app)?;\n    ', "")},
         "RC_SOURCE_CHECK_MISSING"),
        ("审计含 URL",
         {"browser_commands": GOOD_BROWSER_COMMANDS.replace('format!("tab_id={tab_id} removed={removed}")', 'format!("url={url}")')},
         "RC_AUDIT_LEAKS_URL"),
        ("ACL 缺命令",
         {"acl": GOOD_ACL.replace('"list_tab_resources",\n    ', "")},
         "RC_ACL_MISSING"),
        ("handler 未注册",
         {"main_rs": GOOD_MAIN_RS.replace("crate::capabilities::browser::commands::list_tab_resources,\n", "")},
         "RC_HANDLER_NOT_REGISTERED"),
        ("main 直接转发原始事件",
         {"main_rs": GOOD_MAIN_RS + 'let _ = forward.emit("resource-received", payload);'},
         "RC_RAW_PAYLOAD_FORWARDED"),
        ("插件打印 URL",
         {"plugin_commands": GOOD_PLUGIN_COMMANDS.replace("let _ = app.emit(", 'eprintln!("url={}", url);\n        let _ = app.emit(')},
         "RC_PLUGIN_LOGS_URL"),
        ("前端绕过 bridge 直连",
         {"src_files": GOOD_SRC_FILES + [("src/components/X.vue", 'invoke("list_tab_resources", { tabId })')]},
         "RC_FRONTEND_DIRECT_INVOKE"),
        ("前端组件出现凭据标识符",
         {"waterfall": GOOD_WATERFALL + "\nconst token = readCookie();"},
         "RC_FRONTEND_CREDENTIAL_TOKEN"),
        ("面板未挂载",
         {"mainarea": GOOD_MAINAREA.replace("ResourceWaterfall", "div")},
         "RC_WATERFALL_NOT_MOUNTED"),
        ("事件未订阅",
         {"app_vue": GOOD_APP_VUE.replace("onResourceReceived", "onSyncCompleted")},
         "RC_EVENT_NOT_SUBSCRIBED"),
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
        description="Check M1-8 resource capture privacy/capacity invariants."
    )
    parser.add_argument("--self-test", action="store_true", help="run built-in tests")
    args = parser.parse_args(argv)
    if args.self_test:
        return run_self_test()

    violations = scan_repository(Path(__file__).resolve().parent.parent)
    if violations:
        print(f"check-resource-capture-policy: failed ({len(violations)} violation(s))")
        for violation in violations:
            print(violation)
        return 1
    print("check-resource-capture-policy: ok (resource capture invariants hold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
