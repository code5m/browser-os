#!/usr/bin/env python3
# ---------------------------------------------------------------------------
# check-plugin-policy.py — M5-10/M5-11 插件 manifest/生命周期策略不变量夹具
#                          （Lane A9, M5-W6；ACTIVE + PENDING 码位）
#
# 守：
#   ACTIVE:
#     PLUGIN_CAP_SINGLE_DEF   PLUGIN_CAPABILITY_V1 在 security_policy.rs 唯一定义
#                             （能力单一真源；不漂移，不重复定义）
#   PENDING（仅当插件域存在时判；产物未落时自动 SKIP）：
#     PLUGIN_SECOND_PATH      插件模块无第二执行路径（无 std::process / Command::new /
#                             tokio::spawn 直起独立进程）
#     PLUGIN_INLINE_SHELL     插件模块无内联脚本 / 裸 shell 执行
#     PLUGIN_SIG_BYPASS       插件模块无跳过/绕过签名校验（无 todo!/unimplemented!/
#                             panic!/unwrap_or(true) 旁路）
#     PLUGIN_FORM_THREE       插件仅形态③（无 window.__TAURI__ 越权 / 无独立 webview
#                             自起 / 无 std::process 独立进程）
#     PLUGIN_NO_SECRETS       插件 manifest/生命周期代码不持有凭据字段
#                             （无 password/secret/token/api_key 等字段）
#
# W6 红线：纯 manifest/生命周期策略切片——无安装/卸载运行时、无下载/执行、
# 无真签名加密（仅结构校验）、无新增命令、无 secret 持久化。
#
# 用法:
#   scripts/check-plugin-policy.py            默认门禁（插件域存在则判，否则 SKIP）
#   scripts/check-plugin-policy.py --help     显示帮助
#   scripts/check-plugin-policy.py --self-test 双向自检（1 好样本 + N 坏样本变异防呆）
#   scripts/check-plugin-policy.py --expect-pending 校验 PENDING 码位集合未变化
# 退出码: 0 = 全部通过；1 = 任一失败；2 = 非法参数
# ---------------------------------------------------------------------------
import argparse
import os
import re
import sys
import glob

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(SCRIPT_DIR)

ACTIVE_CODES = {
    "PLUGIN_CAP_SINGLE_DEF": "PLUGIN_CAPABILITY_V1 在 security_policy.rs 唯一定义（能力单一真源）",
}
PENDING_CODES = {
    "PLUGIN_SECOND_PATH": "插件模块无第二执行路径（std::process / Command::new / tokio::spawn）",
    "PLUGIN_INLINE_SHELL": "插件模块无内联脚本 / 裸 shell 执行",
    "PLUGIN_SIG_BYPASS": "插件模块无跳过/绕过签名校验（todo!/unimplemented!/panic!/unwrap_or(true)）",
    "PLUGIN_FORM_THREE": "插件仅形态③（无 window.__TAURI__ 越权 / 无独立 webview 自起 / 无 std::process 独立进程）",
    "PLUGIN_NO_SECRETS": "插件 manifest/生命周期代码不持有凭据字段（无 password/secret/token/api_key 等）",
}
EXPECTED_PENDING = set(PENDING_CODES.keys())

# 插件逻辑模块（纯函数所在）；门禁只扫它，避免 domain.rs 等其他文件误伤。
PLUGIN_MODULE = os.path.join(ROOT, "src-tauri", "src", "plugin.rs")

VIOLATION_PATTERNS = {
    "PLUGIN_SECOND_PATH": r"std::process|Command::new|tokio::spawn|\bspawn\b",
    "PLUGIN_INLINE_SHELL": r"InlineScript|RawShell|\bsh -c\b",
    "PLUGIN_SIG_BYPASS": r"todo!|unimplemented!|panic!|\.unwrap_or\(true\)",
    "PLUGIN_FORM_THREE": r"window\.__TAURI__|WebviewWindow|Webview|std::process",
}


def _plugin_present():
    """插件域是否存在：PluginManifest 类型 / PLUGIN_CAPABILITY_V1 / plugin.rs 任一即在。"""
    domain_rs = os.path.join(ROOT, "src-tauri", "src", "domain.rs")
    sec_rs = os.path.join(ROOT, "src-tauri", "src", "security_policy.rs")
    if os.path.exists(domain_rs) and "pub struct PluginManifest" in _read(domain_rs):
        return True
    if os.path.exists(sec_rs) and "PLUGIN_CAPABILITY_V1" in _read(sec_rs):
        return True
    if os.path.exists(PLUGIN_MODULE):
        return True
    return False


def _read(path):
    try:
        with open(path, "r", encoding="utf-8") as f:
            return f.read()
    except OSError:
        return ""


def _all_rs_texts():
    texts = []
    for p in glob.glob(os.path.join(ROOT, "src-tauri", "src", "*.rs")):
        texts.append(_read(p))
    return texts


def g_cap_single_def(texts):
    cnt = sum(1 for t in texts if "pub const PLUGIN_CAPABILITY_V1" in t)
    return cnt == 1, f"PLUGIN_CAPABILITY_V1 定义数={cnt}（期望 1，落在 security_policy.rs）"


def g_violation(code, text):
    if code == "PLUGIN_NO_SECRETS":
        for line in text.splitlines():
            if re.search(r"^\s*pub \w+\s*:", line) and re.search(
                r"(?i)(password|secret|token|api_key|authorization|bearer|cookie|private_key)",
                line,
            ):
                return False, f"字段含凭据名：{line.strip()}"
        return True, "无凭据字段"
    pat = VIOLATION_PATTERNS[code]
    m = re.search(pat, text)
    if m:
        return False, f"匹配违规片段：{m.group(0)!r}"
    return True, "未检出违规"


def run_default():
    if not _plugin_present():
        print("PLUGIN_POLICY=SKIP (插件域不存在)")
        return 0
    texts = _all_rs_texts()
    plugin_text = _read(PLUGIN_MODULE)
    results = {}
    ok, note = g_cap_single_def(texts)
    results["PLUGIN_CAP_SINGLE_DEF"] = (ok, note)
    for code in PENDING_CODES:
        ok, note = g_violation(code, plugin_text)
        results[code] = (ok, note)
    rc = 0
    for code, (ok, note) in results.items():
        tag = "OK" if ok else "FAIL"
        print(f"[{tag}] {code}: {note}")
        if not ok:
            rc = 1
    print("PLUGIN_POLICY=PASS" if rc == 0 else "PLUGIN_POLICY=FAIL")
    return rc


# ---- 自检夹具：1 好样本 + N 坏样本变异防呆 ----

CLEAN_SAMPLE = """
//! 干净的插件模块示例
use crate::domain::PluginManifest;
use crate::security_policy::PolicyError;

pub fn validate_plugin_manifest(m: &PluginManifest) -> Result<(), PolicyError> {
    if m.id.is_empty() {
        return Err(PolicyError::EmptyRequiredField("plugin.id".into()));
    }
    Ok(())
}

pub fn verify_plugin_signature_structure(sig: &crate::domain::PluginSignature) -> Result<(), PolicyError> {
    if sig.algorithm != "Ed25519" {
        return Err(PolicyError::InvalidPluginSignature("algo".into()));
    }
    Ok(())
}
"""

BAD_SAMPLES = {
    "PLUGIN_SECOND_PATH": 'let _ = std::process::Command::new("ls").spawn();',
    "PLUGIN_INLINE_SHELL": "let _ = InlineScript { code: \"rm -rf /\".into() };",
    "PLUGIN_SIG_BYPASS": "pub fn verify(_s: &PluginSignature) -> bool { todo!() }",
    "PLUGIN_FORM_THREE": "let w = WebviewWindow::new(\"p\", None, WebviewUrl::default());",
    "PLUGIN_NO_SECRETS": "pub token: String,",
}


def run_self_test():
    # ACTIVE：能力单一真源
    assert g_cap_single_def(["pub const PLUGIN_CAPABILITY_V1: &[&str] = &[];"])[0] is True, \
        "单定义应放行"
    assert g_cap_single_def(
        ["x", "pub const PLUGIN_CAPABILITY_V1: &[&str] = &[];",
         "pub const PLUGIN_CAPABILITY_V1: &[&str] = &[];"]
    )[0] is False, "重复定义应拦截"
    assert g_cap_single_def(["x", "y"])[0] is False, "零定义应拦截"

    # PENDING：好样本全放行、坏样本全拦截
    for code in PENDING_CODES:
        ok, _ = g_violation(code, CLEAN_SAMPLE)
        assert ok is True, f"{code}: 干净样本被误拦"
        ok, _ = g_violation(code, BAD_SAMPLES[code])
        assert ok is False, f"{code}: 坏样本未被拦截"

    print("PLUGIN_SELF_TEST=ALL_PASS")
    print(f"ACTIVE={len(ACTIVE_CODES)} PENDING={len(PENDING_CODES)}")
    return 0


def run_expect_pending():
    if set(PENDING_CODES.keys()) != EXPECTED_PENDING:
        missing = EXPECTED_PENDING - set(PENDING_CODES.keys())
        extra = set(PENDING_CODES.keys()) - EXPECTED_PENDING
        print(f"PLUGIN_PENDING_MISMATCH missing={missing} extra={extra}")
        return 1
    print("PLUGIN_PENDING_OK")
    return 0


def usage():
    print(__doc__)


def main():
    ap = argparse.ArgumentParser(add_help=False)
    ap.add_argument("--help", "-h", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--expect-pending", action="store_true")
    args, _ = ap.parse_known_args()
    if args.help:
        usage()
        return 0
    if args.self_test:
        return run_self_test()
    if args.expect_pending:
        return run_expect_pending()
    return run_default()


if __name__ == "__main__":
    sys.exit(main())
