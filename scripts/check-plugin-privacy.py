#!/usr/bin/env python3
"""Expose the M5-W13 plugin privacy invariants as a reproducible fixture (Lane A4).

契约来源：
- `PARALLEL_COMMAND_BOARD.md` § M5-W13 Hard Stops
  （审计/检查点不得存凭据、冻结 DTO 之外的裸签名、请求/响应体、插件 stdout/stderr、
    未脱敏路径）
- `src-tauri/src/plugin.rs`（M5-10 / W6 纯策略切片，当前无调用方；W13 接命令层后生效）
- `src-tauri/src/security_policy.rs`（`contains_credential_leak` / `PolicyError::CredentialLeak`）

本夹具守住的底线（默认模式 2 个 ACTIVE 码，5 个 pending 码）：

  错误不回显（W13 接命令层后即为真实泄露面）
  - 自由文本 manifest 字段（`id` / `entry_url` / `metadata` / `signature.algorithm`）
    不得原样进错误串——`id` 可以是 `sk-...`，`entry_url` 可带 `?token=` / `user:pass@`
  - `metadata` 必须过 `contains_credential_leak`（当前只做体量边界，注释声称已守但未实现）
  - `CredentialLeak` 载荷不得承载原始密文（`PolicyError` 派生 `Debug`，`{:?}` 可回显）

  审计只用 ids / 计数 / 哈希前缀
  - 插件审计 detail 不得出现 entry_url / metadata / 清单正文 / 裸签名 / 未脱敏路径

  已闭环（ACTIVE，防回归）
  - `CredentialLeak` 的 Display 必须输出 `<redacted>`（M5-W9 修复，A4 W8 F-W8-1）

用法：
  --self-test        好样本 + 坏样本双向自检（含变异防呆）
  --expect-pending   pending 码位一旦被检出即 FAIL（提示应转入默认判定 / 需先修复）
  默认               只判 ACTIVE 码位
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ACTIVE_CODES: tuple[str, ...] = (
    # 已闭环，防回归
    "PLUGIN_CRED_LEAK_DISPLAY_REDACTED",
    # 审计禁裸签名（产物存在才判）
    "PLUGIN_AUDIT_NO_RAW_SIGNATURE",
)

PENDING_CODES: tuple[str, ...] = (
    # W13 接线前必须闭环（当前 3 项已在 plugin.rs 违约，属潜伏缺陷）
    "PLUGIN_ERR_NO_RAW_FIELD_ECHO",
    "PLUGIN_METADATA_CRED_CHECKED",
    "PLUGIN_CRED_LEAK_PAYLOAD_NOT_RAW",
    # W13 新增面（store / audit 尚未落地，产物存在才判）
    "PLUGIN_AUDIT_IDS_ONLY",
    "PLUGIN_STORE_NO_SECRET",
)

ALL_CODES: tuple[str, ...] = ACTIVE_CODES + PENDING_CODES

# 自由文本 manifest 字段：值可以是任意串（密钥 / 带凭据 URL），原样进错误即泄露面
RAW_ECHO_FIELDS = ("m.id", "m.entry.entry_url", "m.metadata", "sig.algorithm")

# 审计 detail 禁含片段（W13：ids / 计数 / 哈希前缀以外一律禁止）
AUDIT_FORBIDDEN = (
    "entry_url",
    "metadata",
    "manifest",
    "description",
    "display_name",
    "path=",
    "http",
)

# 审计 detail 禁含裸签名
AUDIT_SIG_FORBIDDEN = ("sig.value", "signature.value", "sig_value", "private_key")

# store 落盘标记（存在才判 store 脱敏）
STORE_MARKERS = ("plugins.json", "plugin_store", "save_plugins", "load_plugins", "PLUGINS_FILE")


# ----------------------------- 通用工具 -----------------------------

def read_repo(root: Path) -> dict[str, str]:
    def rd(rel: str) -> str:
        p = root / rel
        return p.read_text(encoding="utf-8", errors="replace") if p.is_file() else ""

    return {
        "plugin": rd("src-tauri/src/capabilities/plugin/plugin.rs"),
        "sp": rd("src-tauri/src/security_policy.rs"),
        "bridge": rd("src-tauri/src/bridge.rs"),
        "main": rd("src-tauri/src/main.rs"),
        "domain": rd("src-tauri/src/domain.rs"),
    }


def strip_tests(src: str) -> str:
    """截掉 `#[cfg(test)]` 之后的测试代码，避免测试样本造成误报。"""
    idx = src.find("#[cfg(test)]")
    return src if idx < 0 else src[:idx]


def plugin_audit_events(bridge: str) -> list[str]:
    return sorted(set(re.findall(r'"(plugin\.[a-z_.]+)"', bridge)))


def audit_detail_literals(bridge: str, event: str) -> list[str]:
    """取该事件**每次 log_audit 调用内的字符串字面量**（即真正会进审计 detail 的内容）。

    刻意不看调用点前后固定窗口：那会把同作用域里的变量名（如 `&manifest.id`）
    误当成审计内容。
    """
    out: list[str] = []
    pos = 0
    while True:
        i = bridge.find(f'"{event}"', pos)
        if i < 0:
            return out
        pos = i + len(event) + 2
        j = bridge.rfind("log_audit(", 0, i)
        if j < 0 or i - j > 400:
            continue
        span = _paren_arg(bridge, j + len("log_audit"))
        out.extend(m.lower() for m in re.findall(r'"([^"\\]*)"', span))
    return out


def _paren_arg(src: str, open_idx: int) -> str:
    """`open_idx` 指向 '('，返回配对的实参文本（不含外层括号）。"""
    depth = 0
    for i in range(open_idx, len(src)):
        if src[i] == "(":
            depth += 1
        elif src[i] == ")":
            depth -= 1
            if depth == 0:
                return src[open_idx + 1: i]
    return ""


# ----------------------------- 判定 -----------------------------

def detect_hits(f: dict[str, str]) -> dict[str, list[str]]:
    hits: dict[str, list[str]] = {}

    def hit(code: str, detail: str) -> None:
        hits.setdefault(code, []).append(detail)

    plugin = strip_tests(f.get("plugin", ""))
    plugin_all = f.get("plugin", "")
    sp = f.get("sp", "")
    bridge = f.get("bridge", "")

    # ---- A1) CredentialLeak 的 Display 必须脱敏（M5-W9 已闭环，防回归）----
    m = re.search(r"PolicyError::CredentialLeak\((\w*)\)\s*=>", sp)
    if m is None:
        hit("PLUGIN_CRED_LEAK_DISPLAY_REDACTED", "security_policy.rs 缺 CredentialLeak 的 Display 分支")
    else:
        window = sp[m.start(): m.start() + 400]
        binding = m.group(1)
        if binding and not binding.startswith("_") and "{" + binding + "}" in window:
            hit("PLUGIN_CRED_LEAK_DISPLAY_REDACTED", f"Display 回显凭据载荷（{{{binding}}}）")
        if "<redacted>" not in window:
            hit("PLUGIN_CRED_LEAK_DISPLAY_REDACTED", "Display 未输出 <redacted>")

    events = plugin_audit_events(bridge)

    # ---- A2) 审计不得出现裸签名（产物存在才判）----
    for ev in events:
        for literal in audit_detail_literals(bridge, ev):
            for bad in AUDIT_SIG_FORBIDDEN:
                if bad in literal:
                    hit("PLUGIN_AUDIT_NO_RAW_SIGNATURE", f"{ev}:{bad}")

    # ---- P1) 自由文本字段不得原样进错误串 ----
    # 按 **每个 format!(...) 调用体**判定（到 `;` 为止），不用固定回溯窗口：
    # 固定窗口会横跨上一条已脱敏语句，既漏报真违规又误报无辜字段。
    for fm in re.finditer(r"format!\(", plugin):
        span = plugin[fm.start(): fm.start() + 400]
        end = span.find(";")
        if end > 0:
            span = span[:end]
        low = span.lower()
        if any(k in low for k in ("redact", "sanitize", "<redacted>", "prefix")):
            continue
        for field in RAW_ECHO_FIELDS:
            if field in span:
                hit("PLUGIN_ERR_NO_RAW_FIELD_ECHO", f"{field} 原样进错误信息")

    # ---- P2) metadata 必须过凭据检测（当前注释声称已守但未实现）----
    if "serde_json::to_string(&m.metadata)" in plugin:
        if not re.search(r"contains_credential_leak\s*\([^)]*metadata", plugin):
            hit("PLUGIN_METADATA_CRED_CHECKED", "metadata 只做体量边界，未过 contains_credential_leak")

    # ---- P3) CredentialLeak 载荷不得是原始密文（PolicyError 派生 Debug，{:?} 可回显）----
    # 括号配对提取实参：`CredentialLeak(s.to_string())` 内含括号，`[^()]*` 匹配不到。
    pos = 0
    while True:
        idx = plugin.find("CredentialLeak(", pos)
        if idx < 0:
            break
        expr = _paren_arg(plugin, idx + len("CredentialLeak"))
        pos = idx + len("CredentialLeak(")
        if expr and re.search(r"\bm\.|&?s\.to_string\(\)|\.clone\(\)", expr):
            hit("PLUGIN_CRED_LEAK_PAYLOAD_NOT_RAW", f"原始字段进 CredentialLeak 载荷: {expr.strip()}")

    # ---- P4) 审计 detail 只用 ids / 计数 / 哈希前缀（产物存在才判）----
    for ev in events:
        for literal in audit_detail_literals(bridge, ev):
            for bad in AUDIT_FORBIDDEN:
                if bad in literal:
                    hit("PLUGIN_AUDIT_IDS_ONLY", f"{ev}:{bad}")

    # ---- P5) store 落盘不得带未脱敏 manifest（产物存在才判）----
    # 判据刻意不看「代码里有没有 redact 这个词」（会被测试函数名之类的文本骗过），
    # 而是看**落盘结构**：登记簿若整份持久化 `PluginManifest`，则未过凭据检测的
    # `metadata`（以及 signature.value 原文）会一起落盘。
    domain = f.get("domain", "")
    if any(k in plugin_all for k in STORE_MARKERS):
        record_persists_manifest = bool(
            re.search(r"pub struct PluginRecord\b[\s\S]{0,400}?pub manifest:\s*PluginManifest", domain)
        )
        metadata_cred_checked = bool(
            re.search(r"contains_credential_leak\s*\([^)]*metadata", plugin)
        )
        if record_persists_manifest and not metadata_cred_checked:
            hit(
                "PLUGIN_STORE_NO_SECRET",
                "登记簿整份落盘 PluginManifest：metadata 未过凭据检测（密钥落盘）+ signature.value 原文入库",
            )

    return hits


# ----------------------------- 自检 -----------------------------

REFERENCE_PLUGIN = '''
pub fn validate_plugin_manifest(m: &PluginManifest) -> Result<(), PolicyError> {
    if !is_reverse_domain(&m.id) {
        return Err(PolicyError::InvalidPluginManifest(format!(
            "plugin.id 非反向域名规范：{}",
            redact_preview(&m.id)
        )));
    }
    if !is_allowed_entry_url(&m.entry.entry_url) {
        return Err(PolicyError::InvalidPluginManifest(format!(
            "plugin.entry.entry_url 非形态③同源入口：{}",
            redact_preview(&m.entry.entry_url)
        )));
    }
    let metadata_json = serde_json::to_string(&m.metadata).unwrap_or_default();
    if security_policy::contains_credential_leak(&metadata_json) {
        return Err(PolicyError::CredentialLeak("plugin.metadata".to_string()));
    }
    if metadata_json.len() > crate::domain::MAX_TEXT_FIELD_BYTES {
        return Err(PolicyError::InvalidPluginManifest("plugin.metadata 超 64KiB".into()));
    }
    if sig.algorithm != "Ed25519" {
        return Err(PolicyError::InvalidPluginSignature(format!(
            "不支持的签名算法（仅 Ed25519）：{}",
            redact_preview(&sig.algorithm)
        )));
    }
    Ok(())
}

const PLUGINS_FILE: &str = "plugins.json";

fn redact_manifest_for_store(m: &PluginManifest) -> PluginManifest {
    let mut out = m.clone();
    out.metadata = json!({});
    out
}
'''

REFERENCE_SP = '''
impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::CredentialLeak(_s) => {
                write!(f, "凭据/密钥泄露（禁止进入插件 manifest）：<redacted>")
            }
        }
    }
}
'''

REFERENCE_BRIDGE = '''
#[tauri::command]
pub fn plugin_install(app: AppHandle, webview: tauri::Webview) -> Result<(), String> {
    workspace::log_audit(
        &app,
        "plugin.install",
        format!("id={} state={:?} hash_prefix={}", m.id, state, &m.hash[..8]),
    );
    Ok(())
}
'''


REFERENCE_DOMAIN = '''
pub struct PluginRecord {
    pub manifest: PluginManifest,
    pub state: PluginState,
}
'''

# 参考实现的落盘形态：只留派生视图 + 哈希前缀，不整份落 manifest
REFERENCE_DOMAIN_FIXED = '''
pub struct PluginRecord {
    pub id: String,
    pub state: PluginState,
    pub hash_prefix: String,
    pub signature_key_id: String,
}
'''


def build_good() -> dict[str, str]:
    return {
        "plugin": REFERENCE_PLUGIN,
        "sp": REFERENCE_SP,
        "bridge": REFERENCE_BRIDGE,
        "main": "",
        "domain": REFERENCE_DOMAIN_FIXED,
    }


def mutate(**over: str) -> dict[str, str]:
    good = build_good()
    good.update(over)
    return good


def run_self_test(root: Path) -> int:
    real = read_repo(root)
    good = build_good()

    # 好样本 A：真实仓库 —— 只允许 ACTIVE 零违规（pending 为已知开放缺口）
    real_active = [c for c in detect_hits(real) if c in ACTIVE_CODES]
    if real_active:
        print("PLUGIN_PRIVACY_SELF_TEST_RESULT=FAIL")
        print(f"  x 真实仓库 ACTIVE 违规（应为空）：{sorted(real_active)}")
        return 1

    # 好样本 B：合成参考实现 —— 全部码位零违规
    good_all = sorted(detect_hits(good))
    if good_all:
        print("PLUGIN_PRIVACY_SELF_TEST_RESULT=FAIL")
        print(f"  x 合成参考实现存在违规（应为空）：{good_all}")
        return 1

    bad_samples = [
        (
            "CredentialLeak Display 回显密文",
            mutate(sp=REFERENCE_SP.replace("_s", "s").replace("<redacted>", "{s}")),
            "PLUGIN_CRED_LEAK_DISPLAY_REDACTED",
        ),
        (
            "id 原样进错误信息（id 可为 sk-...）",
            mutate(plugin=REFERENCE_PLUGIN.replace("redact_preview(&m.id)", "m.id")),
            "PLUGIN_ERR_NO_RAW_FIELD_ECHO",
        ),
        (
            "entry_url 原样进错误信息（可带 ?token= / user:pass@）",
            mutate(plugin=REFERENCE_PLUGIN.replace("redact_preview(&m.entry.entry_url)", "m.entry.entry_url")),
            "PLUGIN_ERR_NO_RAW_FIELD_ECHO",
        ),
        (
            "metadata 未过凭据检测（只做体量边界）",
            mutate(plugin=REFERENCE_PLUGIN.replace(
                'if security_policy::contains_credential_leak(&metadata_json) {\n        return Err(PolicyError::CredentialLeak("plugin.metadata".to_string()));\n    }\n',
                "",
            )),
            "PLUGIN_METADATA_CRED_CHECKED",
        ),
        (
            "原始密文进 CredentialLeak 载荷（Debug 可回显）",
            mutate(plugin=REFERENCE_PLUGIN.replace(
                'PolicyError::CredentialLeak("plugin.metadata".to_string())',
                "PolicyError::CredentialLeak(s.to_string())",
            )),
            "PLUGIN_CRED_LEAK_PAYLOAD_NOT_RAW",
        ),
        (
            "审计 detail 出现裸签名",
            mutate(bridge=REFERENCE_BRIDGE.replace(
                'format!("id={} state={:?} hash_prefix={}", m.id, state, &m.hash[..8])',
                'format!("id={} sig.value={}", m.id, m.signature.value)',
            )),
            "PLUGIN_AUDIT_NO_RAW_SIGNATURE",
        ),
        (
            "审计 detail 出现 entry_url / metadata",
            mutate(bridge=REFERENCE_BRIDGE.replace(
                'format!("id={} state={:?} hash_prefix={}", m.id, state, &m.hash[..8])',
                'format!("id={} entry_url={} metadata={}", m.id, m.entry.entry_url, m.metadata)',
            )),
            "PLUGIN_AUDIT_IDS_ONLY",
        ),
        (
            "登记簿整份落盘 manifest 且 metadata 未过凭据检测",
            mutate(
                domain=REFERENCE_DOMAIN,
                plugin=REFERENCE_PLUGIN.replace(
                    'if security_policy::contains_credential_leak(&metadata_json) {\n        return Err(PolicyError::CredentialLeak("plugin.metadata".to_string()));\n    }\n',
                    "",
                ),
            ),
            "PLUGIN_STORE_NO_SECRET",
        ),
    ]

    failures: list[str] = []
    for name, files, expected in bad_samples:
        found = detect_hits(files)
        if expected not in found:
            failures.append(f"坏样本未检出：{name}（期望 {expected}，实得 {sorted(found)}）")

    if failures:
        print("PLUGIN_PRIVACY_SELF_TEST_RESULT=FAIL")
        for line in failures:
            print(f"  x {line}")
        return 1

    print(
        "PLUGIN_PRIVACY_SELF_TEST_RESULT=PASS: 2 好样本零违规（真实仓库 ACTIVE + 合成参考实现全码位）"
        f" + {len(bad_samples)} 个坏样本全部检出（含变异防呆）；"
        f"ACTIVE={len(ACTIVE_CODES)} PENDING={len(PENDING_CODES)}"
    )
    return 0


# ----------------------------- 入口 -----------------------------

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--self-test", action="store_true", help="好样本 + 坏样本双向自检（含变异防呆）")
    ap.add_argument(
        "--expect-pending",
        action="store_true",
        help="验证 pending 码位未被检出；一旦被检出即应转入默认判定或先修复",
    )
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[1]

    if args.self_test:
        return run_self_test(root)

    hits = detect_hits(read_repo(root))

    if args.expect_pending:
        pending_hits = [c for c in PENDING_CODES if c in hits]
        if pending_hits:
            print("PLUGIN_PRIVACY_PENDING_RESULT=FAIL")
            for code in pending_hits:
                for detail in hits[code]:
                    print(f"  {code}:{detail}")
            return 1
        print(f"PLUGIN_PRIVACY_PENDING_RESULT=NONE（{len(PENDING_CODES)} 个 pending 码位均未检出）")
        return 0

    active_hits = sorted(c for c in hits if c in ACTIVE_CODES)
    if active_hits:
        print("PLUGIN_PRIVACY_POLICY_RESULT=FAIL")
        for code in active_hits:
            for detail in hits[code]:
                print(f"  {code}:{detail}")
        return 1
    print(f"plugin privacy policy: all invariants hold（ACTIVE={len(ACTIVE_CODES)}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
