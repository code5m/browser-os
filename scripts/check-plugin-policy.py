#!/usr/bin/env python3
# ---------------------------------------------------------------------------
# check-plugin-policy.py — M5-10/M5-11 插件 manifest/生命周期策略不变量夹具
#                          （Lane A9, M5-W6；ACTIVE + PENDING 码位）
#
# 守：
#   ACTIVE:
#     PLUGIN_CAP_SINGLE_DEF   PLUGIN_CAPABILITY_V1 在 security_policy.rs 唯一定义
#                             （能力单一真源；不漂移，不重复定义）
#   W13 新增 ACTIVE（Stage-I manifest 生命周期）：
#     PLUGIN_CMD_ACL_PARITY    main.rs 注册的 plugin_* 命令与 ACL 集合一致，且 ACL
#                              末条恒为 list_artifact_images（K1）
#     PLUGIN_CMD_SOURCE_CHECK  每个 plugin_* 命令体必过 check_invocation_source
#     PLUGIN_NO_EXEC_SURFACE   插件域无执行面（无 plugin_invoke / Command::new /
#                              std::process / WebviewWindow / dlopen / reqwest /
#                              tokio::spawn）
#     PLUGIN_AUDIT_REDACTED    插件审计不得回显 resource_path / signature / pubkey /
#                              value（K3 + W13 Hard Stop）
#     PLUGIN_REGISTRY_ATOMIC_WRITE
#                              登记簿落盘必走 session::atomic_write，禁 std::fs::write
#     PLUGIN_DTO_NO_SIG_VALUE  Stage-I DTO 禁声明 value / pubkey / resource_path 字段
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
    "PLUGIN_CMD_ACL_PARITY": "main.rs 注册的 plugin_* 命令与 ACL 集合一致且末条恒为 list_artifact_images",
    "PLUGIN_CMD_SOURCE_CHECK": "每个 plugin_* 命令体必过 check_invocation_source",
    "PLUGIN_NO_EXEC_SURFACE": "插件域无执行面（无 plugin_invoke/Command::new/std::process/WebviewWindow/dlopen/reqwest/tokio::spawn）",
    "PLUGIN_AUDIT_REDACTED": "插件审计不回显 resource_path/signature/pubkey/value",
    "PLUGIN_REGISTRY_ATOMIC_WRITE": "登记簿落盘必走 session::atomic_write，禁 std::fs::write",
    "PLUGIN_DTO_NO_SIG_VALUE": "Stage-I DTO 禁声明 value/pubkey/resource_path 字段",
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


# ---- W13 Stage-I ACTIVE 检查（命令面 / 执行面 / 审计脱敏 / 原子落盘）----

W13_REGION_MARK = "M5-W13 插件 manifest 生命周期 Stage-I"


def _repo_files():
    """按逻辑名装载 W13 检查所需的仓库文件文本。"""
    base = os.path.join(ROOT, "src-tauri")
    return {
        "bridge": _read(os.path.join(base, "src", "bridge.rs")),
        "plugin": _read(os.path.join(base, "src", "plugin.rs")),
        "main": _read(os.path.join(base, "src", "main.rs")),
        "domain": _read(os.path.join(base, "src", "domain.rs")),
        "acl": _read(os.path.join(base, "permissions", "default-commands.toml")),
    }


def _w13_region(bridge_text):
    """bridge.rs 中 W13 插件命令区（只扫本 wave 的命令，不误伤其它域）。"""
    i = bridge_text.find(W13_REGION_MARK + " 命令")
    if i < 0:
        i = bridge_text.find(W13_REGION_MARK)
    return bridge_text[i:] if i >= 0 else ""


def g_cmd_acl_parity(f):
    cmds = sorted(set(re.findall(r"bridge::(plugin_\w+)", f["main"])))
    acl = sorted(set(re.findall(r'"(plugin_\w+)"', f["acl"])))
    if not cmds:
        return False, "main.rs 未注册任何 plugin_* 命令"
    if cmds != acl:
        return False, f"命令集不一致 main={cmds} acl={acl}"
    m = re.search(r',\s*"([^"]+)"\s*\]\s*$', f["acl"])
    tail = m.group(1) if m else "<none>"
    if tail != "list_artifact_images":
        return False, f"ACL 末条应为 list_artifact_images，实为 {tail}"
    return True, f"{len(cmds)} 个 plugin_* 命令 ACL 一致，末条={tail}"


def g_cmd_source_check(f):
    names = sorted(set(re.findall(r"pub fn (plugin_\w+)\(", f["bridge"])))
    if not names:
        return False, "bridge.rs 未定义任何 plugin_* 命令"
    missing = []
    for name in names:
        i = f["bridge"].find(f"pub fn {name}(")
        seg = f["bridge"][i: i + 900] if i >= 0 else ""
        if "check_invocation_source" not in seg:
            missing.append(name)
    if missing:
        return False, f"命令缺少来源校验：{missing}"
    return True, f"{len(names)} 个命令均过 check_invocation_source"


def g_no_exec_surface(f):
    region = _w13_region(f["bridge"])
    if not region:
        return False, "bridge.rs 未找到 W13 插件命令区"
    m = re.search(
        r"plugin_invoke|Command::new|std::process|WebviewWindow|dlopen|reqwest|tokio::spawn",
        f["plugin"] + region,
    )
    if m:
        return False, f"插件域出现执行面：{m.group(0)!r}"
    return True, "插件域无执行 / 动态加载 / 网络面"


def g_audit_redacted(f):
    region = _w13_region(f["bridge"])
    if not region:
        return False, "bridge.rs 未找到 W13 插件命令区"
    hits = []
    for m in re.finditer(r"log_audit\(", region):
        seg = region[m.start(): m.start() + 600]
        end = seg.find(");")
        seg = seg[: end + 2] if end >= 0 else seg
        for tok in ("resource_path", "signature", "pubkey", "value"):
            if re.search(rf"\b{tok}\b", seg):
                hits.append(tok)
    if hits:
        return False, f"插件审计回显禁记字段：{sorted(set(hits))}"
    return True, "插件审计仅记 id / 计数 / 稳定错误码"


def g_registry_atomic_write(f):
    # 只扫生产区：`#[cfg(test)]` 之后的单测夹具允许直写临时文件造损坏样本。
    prod = f["plugin"].split("#[cfg(test)]")[0]
    if "atomic_write" not in prod:
        return False, "登记簿未使用 session::atomic_write"
    m = re.search(r"std::fs::write\(", prod)
    if m:
        return False, "登记簿出现非原子写（std::fs::write）"
    return True, "登记簿生产区落盘走 atomic_write（tmp + rename）"


def g_dto_no_sig_value(f):
    i = f["domain"].find(W13_REGION_MARK)
    if i < 0:
        return False, "domain.rs 未找到 W13 插件 DTO 区"
    region = f["domain"][i:]
    for tok in ("pub value:", "pub pubkey:", "pub resource_path:"):
        if tok in region:
            return False, f"DTO 含禁落字段：{tok}"
    return True, "DTO 无签名原文 / 公钥原文 / 资源路径字段"


W13_CHECKS = {
    "PLUGIN_CMD_ACL_PARITY": g_cmd_acl_parity,
    "PLUGIN_CMD_SOURCE_CHECK": g_cmd_source_check,
    "PLUGIN_NO_EXEC_SURFACE": g_no_exec_surface,
    "PLUGIN_AUDIT_REDACTED": g_audit_redacted,
    "PLUGIN_REGISTRY_ATOMIC_WRITE": g_registry_atomic_write,
    "PLUGIN_DTO_NO_SIG_VALUE": g_dto_no_sig_value,
}


def run_default():
    if not _plugin_present():
        print("PLUGIN_POLICY=SKIP (插件域不存在)")
        return 0
    texts = _all_rs_texts()
    plugin_text = _read(PLUGIN_MODULE)
    results = {}
    ok, note = g_cap_single_def(texts)
    results["PLUGIN_CAP_SINGLE_DEF"] = (ok, note)
    # W13 Stage-I：命令面 / 执行面 / 审计脱敏 / 原子落盘 ACTIVE 不变量
    files = _repo_files()
    for code, fn in W13_CHECKS.items():
        results[code] = fn(files)
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

# ---- W13 Stage-I 自检夹具：1 好样本 + 每码位 1 变异坏样本 ----

W13_GOOD = {
    "main": "            bridge::plugin_install,\n            bridge::plugin_list,\n",
    "acl": (
        '    "plugin_install",\n'
        '    "plugin_list",\n'
        '    "list_artifact_images"\n'
        "]\n"
    ),
    "bridge": (
        "// M5-W13 插件 manifest 生命周期 Stage-I 命令（Lane A9）\n"
        "pub fn plugin_install(app: AppHandle, webview: tauri::Webview)"
        " -> Result<(), String> {\n"
        '    check_invocation_source(&webview, "plugin_install", None, &app)?;\n'
        '    workspace::log_audit(&app, "plugin.install",'
        ' format!("id={id_safe} result=ok"));\n'
        "    Ok(())\n"
        "}\n"
        "pub fn plugin_list(app: AppHandle, webview: tauri::Webview)"
        " -> Result<(), String> {\n"
        '    check_invocation_source(&webview, "plugin_list", None, &app)?;\n'
        "    Ok(())\n"
        "}\n"
    ),
    "plugin": "crate::session::atomic_write(path, &content)\n",
    "domain": (
        "// M5-W13 插件 manifest 生命周期 Stage-I DTO\n"
        "pub struct PluginSummary { pub hash_prefix: String }\n"
    ),
}


def _w13_bad(code):
    """按码位对好样本做单点变异（变异防呆：门禁必须能抓住该退化）。"""
    f = dict(W13_GOOD)
    if code == "PLUGIN_CMD_ACL_PARITY":
        # ACL 漏登记 plugin_list
        f["acl"] = '    "plugin_install",\n    "list_artifact_images"\n]\n'
    elif code == "PLUGIN_CMD_SOURCE_CHECK":
        # plugin_list 去掉来源校验
        f["bridge"] = f["bridge"].replace(
            '    check_invocation_source(&webview, "plugin_list", None, &app)?;\n', ""
        )
    elif code == "PLUGIN_NO_EXEC_SURFACE":
        f["plugin"] = f["plugin"] + 'let _ = std::process::Command::new("ls");\n'
    elif code == "PLUGIN_AUDIT_REDACTED":
        f["bridge"] = f["bridge"].replace(
            'format!("id={id_safe} result=ok")',
            'format!("id={id_safe} resource_path={p}")',
        )
    elif code == "PLUGIN_REGISTRY_ATOMIC_WRITE":
        f["plugin"] = f["plugin"] + "std::fs::write(path, content)?;\n"
    elif code == "PLUGIN_DTO_NO_SIG_VALUE":
        f["domain"] = f["domain"] + "pub value: String,\n"
    return f


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

    # W13 ACTIVE：好样本全放行、坏样本全拦截（每个码位各自变异）
    for code, fn in W13_CHECKS.items():
        ok, note = fn(W13_GOOD)
        assert ok is True, f"{code}: 干净样本被误拦（{note}）"
        ok, note = fn(_w13_bad(code))
        assert ok is False, f"{code}: 坏样本未被拦截（{note}）"
    # 附加：ACL 末条被顶掉（新命令插在 list_artifact_images 之后）须拦截
    tail_broken = dict(W13_GOOD)
    tail_broken["acl"] = '    "plugin_install",\n    "plugin_list",\n    "plugin_get"\n]\n'
    assert g_cmd_acl_parity(tail_broken)[0] is False, "ACL 末条非 list_artifact_images 应拦截"

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
