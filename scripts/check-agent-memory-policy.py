#!/usr/bin/env python3
"""把 M5-3 agent memory KV 的容量 / 隐私 / 审计不变量暴露成可复现的准入夹具。

契约来源（均来自本仓库既有交付，非本脚本发明）：
- `logs/checkpoints/M5-20260906/M5-3-a2a-bidir-agent-kv.md`（A1 M5-3 卡 §4）
- `logs/assist/A4-M5-a2a-memory-20260906-1410-w3-delta.md`（A4 W3 delta，含 C-5/C-6 修正）
- `src-tauri/src/capabilities/agent/agent_memory.rs`（本切片实现，落点在 W4 Lane A4）

本脚本的定位（对齐 A2 的 `check-core-boundary.py` / A3 的 `check-mcp-policy.py`）：
- 它是**准入门禁脚本**，不是 KV 运行时代码；只读取既有文件做静态断言。
- 各码位只在对应文件存在时才判定（`agent_memory.rs` / `domain.rs`），产物缺失自动 no-op。

守什么（ACTIVE 码位，全部与 A1 卡 C-5/C-6 修正直接对应）：
- `AGENT_KV_CONSTANTS_PRESENT`：容量常量在 `domain.rs` 单一真源定义。
- `AGENT_KV_PRIVACY_DOUBLE_SCAN`：隐私三重闸必须**同时扫字段名 + 字符串值**（修正 C-5）。
- `AGENT_KV_PER_AGENT_BYTES_NOT_COUNT`：per-agent 软配额 = 字节（1 MiB），agent 数上限 = 32 为独立不变量（修正 C-6）。
- `AGENT_KV_BOUNDED_TOTAL`：store 必须接总容量 / 总条目上限淘汰逻辑。
- `AGENT_KV_AUDIT_NO_VALUE`：审计条目只记 key 哈希，绝不序列化完整 record.value。

用法：
  python3 scripts/check-agent-memory-policy.py                 默认扫描（无违规 → EXIT 0；有 → EXIT 2）
  python3 scripts/check-agent-memory-policy.py --self-test     好/坏样本双向自检（含变异防呆）
  python3 scripts/check-agent-memory-policy.py --expect-pending 验证 agent_memory.rs 是否已落地（W4 后应翻为默认）
"""

from __future__ import annotations

import argparse
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

AGENT_MEMORY = "src-tauri/src/capabilities/agent/agent_memory.rs"
DOMAIN = "src-tauri/src/domain.rs"


def _read(path: str) -> str | None:
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            return fh.read()
    except OSError:
        return None


def _rel(path: str) -> str:
    return os.path.relpath(path, ROOT)


def _scan_real_repo() -> dict[str, str]:
    repo: dict[str, str] = {}
    for rel in (DOMAIN, AGENT_MEMORY):
        t = _read(os.path.join(ROOT, rel))
        if t is not None:
            repo[rel] = t
    return repo


# ---------------------------------------------------------------------------
# 码位
# ---------------------------------------------------------------------------


def c_constants_present(rel, text, _repo):
    if rel != DOMAIN:
        return None
    need = (
        "AGENT_KV_MAX_TOTAL_BYTES",
        "AGENT_KV_MAX_TOTAL_RECORDS",
        "AGENT_KV_MAX_PER_NAMESPACE_RECORDS",
        "AGENT_KV_MAX_PER_AGENT_BYTES",
        "AGENT_KV_MAX_AGENTS",
        "AGENT_KV_MAX_KEY_BYTES",
        "AGENT_KV_MAX_VALUE_BYTES",
    )
    missing = [c for c in need if c not in text]
    if missing:
        return [f"domain.rs 缺 AGENT_KV 容量常量：{missing}"]
    return None


def c_privacy_double_scan(rel, text, _repo):
    if rel != AGENT_MEMORY:
        return None
    # C-5 修正：必须同时扫字段名 + 字符串值；值模式需覆盖 sk-/AKIA/Bearer /eyJ/-----BEGIN
    need_key_names = ('"token"', '"password"', '"secret"', '"api_key"')
    need_value_pat = ("sk-", "AKIA", "Bearer ", "eyJ", "-----BEGIN")
    problems = []
    if not all(tok in text for tok in need_key_names):
        problems.append("隐私闸缺字段名黑名单（token/password/secret/api_key）")
    if not all(p in text for p in need_value_pat):
        problems.append("隐私闸缺字符串值敏感模式（sk-/AKIA/Bearer /eyJ/-----BEGIN）")
    if "scan_json_value" not in text:
        problems.append("缺 scan_json_value（应递归扫字段名 + 字符串值）")
    return problems or None


def c_per_agent_bytes_not_count(rel, text, _repo):
    if rel != AGENT_MEMORY:
        return None
    # C-6 修正：per-agent 软配额 = 字节（AGENT_KV_MAX_PER_AGENT_BYTES）；agent 数上限 = 32 独立
    if "AGENT_KV_MAX_PER_AGENT_BYTES" not in text:
        return [
            "per-agent 软配额未用字节常量 AGENT_KV_MAX_PER_AGENT_BYTES"
            "（误用 32 条会导致总上限形同虚设）"
        ]
    if "AGENT_KV_MAX_AGENTS" not in text:
        return [
            "未引用 agent 总数上限 AGENT_KV_MAX_AGENTS（32 是 agent 数上限，非单 agent 条目数）"
        ]
    return None


def c_bounded_total(rel, text, _repo):
    if rel != AGENT_MEMORY:
        return None
    if "AGENT_KV_MAX_TOTAL" not in text:
        return ["store 未引用 AGENT_KV_MAX_TOTAL_*（总容量/总条目上限未接入淘汰逻辑）"]
    return None


def c_audit_no_value(rel, text, _repo):
    if rel != AGENT_MEMORY:
        return None
    problems = []
    if "key_hash" not in text:
        problems.append("审计条目缺 key_hash（应只记 key 哈希，不记明文）")
    if "redact_for_audit" not in text and "AgentKvAuditEntry" not in text:
        problems.append("缺审计脱敏函数/结构（redact_for_audit 或 AgentKvAuditEntry）")
    if "serde_json::to_string(&rec)" in text or "serde_json::to_string(&record)" in text:
        problems.append("审计路径序列化了完整 record（可能泄露 value）")
    return problems or None


ALL_CODES = [
    ("AGENT_KV_CONSTANTS_PRESENT", c_constants_present),
    ("AGENT_KV_PRIVACY_DOUBLE_SCAN", c_privacy_double_scan),
    ("AGENT_KV_PER_AGENT_BYTES_NOT_COUNT", c_per_agent_bytes_not_count),
    ("AGENT_KV_BOUNDED_TOTAL", c_bounded_total),
    ("AGENT_KV_AUDIT_NO_VALUE", c_audit_no_value),
]


def detect_hits(repo: dict[str, str]) -> dict[str, list[str]]:
    hits: dict[str, list[str]] = {}
    for name, fn in ALL_CODES:
        for rel, text in repo.items():
            res = fn(rel, text, repo)
            if res:
                hits.setdefault(name, []).extend(res)
    return hits


# ---------------------------------------------------------------------------
# 自测
# ---------------------------------------------------------------------------

GOOD_DOMAIN = """
pub const AGENT_KV_MAX_TOTAL_BYTES: usize = 5 * 1024 * 1024;
pub const AGENT_KV_MAX_TOTAL_RECORDS: usize = 5000;
pub const AGENT_KV_MAX_PER_NAMESPACE_RECORDS: usize = 1000;
pub const AGENT_KV_MAX_PER_AGENT_BYTES: usize = 1 * 1024 * 1024;
pub const AGENT_KV_MAX_AGENTS: usize = 32;
pub const AGENT_KV_MAX_KEY_BYTES: usize = 256;
pub const AGENT_KV_MAX_VALUE_BYTES: usize = 64 * 1024;
"""

GOOD_AGENT_MEMORY = """
use crate::domain::*;
const SENSITIVE_KEY_NAMES: &[&str] = &["token","password","secret","api_key"];
const SENSITIVE_VALUE_PATTERNS: &[&str] = &["sk-","AKIA","Bearer ","eyJ","-----BEGIN"];
fn scan_json_value(_v: &()) {}
fn agent_kv_privacy_ok() {}
fn put() {
    let _ = AGENT_KV_MAX_PER_AGENT_BYTES;
    let _ = AGENT_KV_MAX_AGENTS;
    let _ = AGENT_KV_MAX_TOTAL_BYTES;
    let _ = AGENT_KV_MAX_TOTAL_RECORDS;
    let _ = AGENT_KV_MAX_PER_NAMESPACE_RECORDS;
}
struct AgentKvAuditEntry { key_hash: String }
fn redact_for_audit() {}
fn audit() { let _ = serde_json::to_string(&summary); }
"""


def _baseline_repo() -> dict[str, str]:
    return {DOMAIN: GOOD_DOMAIN, AGENT_MEMORY: GOOD_AGENT_MEMORY}


def _run_self_test() -> int:
    failures: list[str] = []
    code_names = {n for n, _ in ALL_CODES}

    good = _baseline_repo()
    good_hits = detect_hits(good)
    if good_hits:
        failures.append(f"好样本误报：{good_hits}")

    def mutate(**kw):
        d = dict(good)
        d.update(kw)
        return d

    bad_cases: list[tuple[str, str, dict[str, str], str]] = []
    # (code, desc, mutated_repo, key_that_must_change)

    bad_cases.append((
        "AGENT_KV_CONSTANTS_PRESENT",
        "domain.rs 缺 AGENT_KV_MAX_PER_AGENT_BYTES",
        mutate(**{DOMAIN: GOOD_DOMAIN.replace("AGENT_KV_MAX_PER_AGENT_BYTES: usize = 1 * 1024 * 1024;\n", "")}),
        DOMAIN,
    ))
    bad_cases.append((
        "AGENT_KV_PRIVACY_DOUBLE_SCAN",
        "agent_memory.rs 只扫字段名不扫值",
        mutate(**{AGENT_MEMORY: GOOD_AGENT_MEMORY.replace(
            'const SENSITIVE_VALUE_PATTERNS: &[&str] = &["sk-","AKIA","Bearer ","eyJ","-----BEGIN"];',
            'const SENSITIVE_VALUE_PATTERNS: &[&str] = &[];')}),
        AGENT_MEMORY,
    ))
    bad_cases.append((
        "AGENT_KV_PER_AGENT_BYTES_NOT_COUNT",
        "agent_memory.rs 未用字节软配额常量",
        mutate(**{AGENT_MEMORY: GOOD_AGENT_MEMORY.replace("let _ = AGENT_KV_MAX_PER_AGENT_BYTES;\n", "")}),
        AGENT_MEMORY,
    ))
    bad_cases.append((
        "AGENT_KV_BOUNDED_TOTAL",
        "agent_memory.rs 未接总上限",
        mutate(**{AGENT_MEMORY: GOOD_AGENT_MEMORY.replace("let _ = AGENT_KV_MAX_TOTAL_BYTES;\n", "")
                  .replace("let _ = AGENT_KV_MAX_TOTAL_RECORDS;\n", "")}),
        AGENT_MEMORY,
    ))
    bad_cases.append((
        "AGENT_KV_AUDIT_NO_VALUE",
        "agent_memory.rs 审计序列化完整 record",
        mutate(**{AGENT_MEMORY: GOOD_AGENT_MEMORY.replace(
            "fn audit() { let _ = serde_json::to_string(&summary); }",
            "fn audit() { let _ = serde_json::to_string(&rec); }")}),
        AGENT_MEMORY,
    ))

    for code, _desc, mutated, key in bad_cases:
        if mutated.get(key, "") == good.get(key, ""):
            print(f"self-test FAIL: 坏样本「{code}」未改动任何内容（变异失配，按漏检计）")
            sys.exit(1)
        h = detect_hits(mutated)
        if code not in h:
            failures.append(f"坏样本未检出码位 {code}（漏检）；命中={h}")

    if failures:
        for f in failures:
            print(f"  - {f}")
        print("AGENT_KV_POLICY_SELF_TEST=FAIL")
        return 1
    print(f"AGENT_KV_POLICY_SELF_TEST=PASS（ACTIVE={len(code_names)}）")
    return 0


# ---------------------------------------------------------------------------
# 模式
# ---------------------------------------------------------------------------


def _mode_expect_pending() -> int:
    repo = _scan_real_repo()
    if AGENT_MEMORY in repo:
        print("AGENT_KV_PENDING_RESULT=FAIL：已检测到 M5-3 产物（agent_memory.rs 已落地），"
              "应把本脚本作为默认门禁接入 pre-merge.sh")
        return 1
    print("AGENT_KV_PENDING_RESULT=NONE（agent_memory.rs 尚未落地，W4 前应处此态）")
    return 0


def _mode_default() -> int:
    repo = _scan_real_repo()
    hits = detect_hits(repo)
    if not hits:
        print("AGENT_KV_POLICY=PASS（无违规）")
        return 0
    for code, details in sorted(hits.items()):
        for d in details:
            print(f"  {code}: {d}")
    print("AGENT_KV_POLICY=FAIL")
    return 2


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--expect-pending", action="store_true")
    args = ap.parse_args()
    if args.self_test:
        return _run_self_test()
    if args.expect_pending:
        return _mode_expect_pending()
    return _mode_default()


if __name__ == "__main__":
    sys.exit(main())
