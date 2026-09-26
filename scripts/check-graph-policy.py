#!/usr/bin/env python3
"""把 M5-7/M5-8 知识图谱 core 切片的容量 / 隐私 / bounded / 完整性不变量暴露成可复现的准入夹具。

契约来源（均来自本仓库既有交付，非本脚本发明）：
- `logs/assist/A7-M5-W4-graph-core-delta-20260906-1455.md`（A7 W4 delta，对齐 A4/A5 契约）
- `src-tauri/src/domain.rs`（本切片 DTO + 容量常量单一真源）
- `src-tauri/src/graph.rs`（本切片实现，落点在 W5 Lane A7）

本脚本的定位（对齐 A2 `check-core-boundary.py` / A3 `check-mcp-policy.py` / A4 `check-agent-memory-policy.py`）：
- 它是**准入门禁脚本**，不是图谱运行时代码；只读取既有文件做静态断言。
- 各码位只在对应文件存在时才判定（`graph.rs` / `domain.rs`），产物缺失自动 no-op。

守什么（ACTIVE 码位，全部与 A7 W5 实现 + W4 delta 直接对应）：
- `GRAPH_CONSTANTS_PRESENT`：容量常量在 `domain.rs` 单一真源定义。
- `GRAPH_PROPS_EQ_MAX_TEXT_FIELD`：`GRAPH_PROPS_MAX_BYTES` 必须等于 `MAX_TEXT_FIELD_BYTES`（与
  A4 `AGENT_KV_MAX_VALUE_BYTES == DB_MAX_TEXT_FIELD_BYTES` 同款不变量）。
- `GRAPH_PRIVACY_DOUBLE_SCAN`：隐私双重扫描必须同时扫字段名 + 字符串值（镜像 A4 agent_kv C-5）。
- `GRAPH_BOUNDED_STORE`：store 必须接 `GRAPH_MAX_NODES` / `GRAPH_MAX_EDGES` 容量拒绝逻辑。
- `GRAPH_TRAVERSAL_BOUNDED`：遍历必须接 `GRAPH_MAX_DEPTH` / `GRAPH_QUERY_LIMIT` 上限。
- `GRAPH_REF_NODE_INTEGRITY`：Skill/Agent 节点 id 必须是 `GRAPH_NODE_ID_HEX_LEN` 位十六进制
  （AGRAPH-10 完整性；实时反查 A5 SkillDef/AgentDef 留待 agent store 接入后）。
- `GRAPH_NO_SECOND_PATH`：graph.rs 不得出现 `std::process` / `Command::new` / `tokio` /
  `use tauri` / `crate::bridge`（无第二执行路径 / 无网络 / 不反向依赖命令层）。
- `GRAPH_OUTPUT_NO_PROPS`：只读出参必须删除 `props`（K7 双闸）：`domain.rs` 的
  `GraphNodeView` / `GraphEdgeView` 不得含 `props` 字段；`graph.rs` 命令内核须经
  `GraphNodeView::from` / `GraphEdgeView::from` 转换，且 `GraphError` 须有稳定 `code()`。

用法：
  python3 scripts/check-graph-policy.py                 默认扫描（无违规 → EXIT 0；有 → EXIT 2）
  python3 scripts/check-graph-policy.py --self-test     好/坏样本双向自检（含变异防呆）
  python3 scripts/check-graph-policy.py --expect-pending 验证图谱产物是否已落地（W5 后应翻为默认）
"""

from __future__ import annotations

import argparse
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

GRAPH = "src-tauri/src/capabilities/graph/graph.rs"
DOMAIN = "src-tauri/src/domain.rs"


def _read(path: str) -> str | None:
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            return fh.read()
    except OSError:
        return None


def _rel(path: str) -> str:
    return os.path.relpath(path, ROOT)


def _strip_comments(text: str) -> str:
    """剥离行/块注释与字符串字面量，避免注释里的 `crate::bridge` 等字样误报。

    与 `check-core-boundary.py` 同源思路：守门脚本只应判定真实代码路径。
    """
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    text = re.sub(r'"(\\.|[^"\\])*"', '""', text)
    return text


def _scan_real_repo() -> dict[str, str]:
    repo: dict[str, str] = {}
    for rel in (DOMAIN, GRAPH):
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
        "GRAPH_PROPS_MAX_BYTES",
        "GRAPH_LABEL_MAX_BYTES",
        "GRAPH_NODE_ID_HEX_LEN",
        "GRAPH_MAX_DEPTH",
        "GRAPH_QUERY_LIMIT",
        "GRAPH_MAX_NODES",
        "GRAPH_MAX_EDGES",
    )
    missing = [c for c in need if c not in text]
    if missing:
        return [f"domain.rs 缺图谱容量常量：{missing}"]
    return None


def c_props_eq_max_text_field(rel, text, _repo):
    if rel != DOMAIN:
        return None
    if "GRAPH_PROPS_MAX_BYTES" not in text:
        return ["domain.rs 缺 GRAPH_PROPS_MAX_BYTES"]
    if "MAX_TEXT_FIELD_BYTES" not in text:
        return ["domain.rs 缺 MAX_TEXT_FIELD_BYTES（GRAPH_PROPS_MAX_BYTES 必须与之相等）"]
    return None


def c_privacy_double_scan(rel, text, _repo):
    if rel != GRAPH:
        return None
    # 必须同时扫字段名黑名单 + 字符串值模式（镜像 A4 agent_kv C-5 修正）。
    need_key_names = ('"token"', '"password"', '"secret"', '"api_key"')
    need_value_pat = ("sk-", "AKIA", "Bearer ", "eyJ", "-----BEGIN")
    problems = []
    if not all(tok in text for tok in need_key_names):
        problems.append("隐私闸缺字段名黑名单（token/password/secret/api_key）")
    if not all(p in text for p in need_value_pat):
        problems.append("隐私闸缺字符串值敏感模式（sk-/AKIA/Bearer /eyJ/-----BEGIN）")
    if "scan_graph_value" not in text or "graph_props_contain_secret" not in text:
        problems.append("缺 scan_graph_value / graph_props_contain_secret（应递归扫字段名 + 字符串值）")
    return problems or None


def c_bounded_store(rel, text, _repo):
    if rel != GRAPH:
        return None
    problems = []
    if "GRAPH_MAX_NODES" not in text:
        problems.append("store 未引用 GRAPH_MAX_NODES（节点容量拒绝逻辑缺失）")
    if "GRAPH_MAX_EDGES" not in text:
        problems.append("store 未引用 GRAPH_MAX_EDGES（边容量拒绝逻辑缺失）")
    if "NodeCapacityExceeded" not in text or "EdgeCapacityExceeded" not in text:
        problems.append("缺 NodeCapacityExceeded / EdgeCapacityExceeded 拒绝分支")
    return problems or None


def c_traversal_bounded(rel, text, _repo):
    if rel != GRAPH:
        return None
    problems = []
    if "GRAPH_MAX_DEPTH" not in text:
        problems.append("遍历未引用 GRAPH_MAX_DEPTH（深度上限缺失，可能爆栈）")
    if "GRAPH_QUERY_LIMIT" not in text:
        problems.append("遍历未引用 GRAPH_QUERY_LIMIT（返回上限缺失，可能响应爆）")
    return problems or None


def c_ref_node_integrity(rel, text, _repo):
    if rel == DOMAIN:
        if "GRAPH_NODE_ID_HEX_LEN" not in text:
            return ["domain.rs 缺 GRAPH_NODE_ID_HEX_LEN（Skill/Agent 节点 id 完整性基准）"]
        return None
    if rel == GRAPH:
        # graph.rs 必须把 GRAPH_NODE_ID_HEX_LEN 用于 Skill/Agent id 校验。
        if "GRAPH_NODE_ID_HEX_LEN" not in text or "RefIdNotHex" not in text or "is_hex" not in text:
            return ["graph.rs 未将 GRAPH_NODE_ID_HEX_LEN 用于 Skill/Agent 节点 id 校验（AGRAPH-10）"]
        return None
    return None


def c_no_second_path(rel, text, _repo):
    if rel != GRAPH:
        return None
    code = _strip_comments(text)
    forbidden = (
        "std::process",
        "Command::new",
        "tokio",
        "use tauri",
        "crate::bridge",
    )
    hits = [f for f in forbidden if f in code]
    if hits:
        return [f"graph.rs 出现第二执行路径/网络/反向依赖标志：{hits}"]
    return None


def c_output_no_props(rel, text, _repo):
    if rel == DOMAIN:
        # 只读 View 类型必须存在且结构体体内不得含 props 字段（K7 双闸：出参即删）。
        if "GraphNodeView" not in text or "GraphEdgeView" not in text:
            return ["domain.rs 缺只读 View 类型 GraphNodeView/GraphEdgeView（出参须删 props）"]
        if re.search(r"struct GraphNodeView\s*\{[^}]*\bprops\b", text, flags=re.S):
            return ["domain.rs GraphNodeView 含 props 字段（出参泄露脱敏前原文）"]
        if re.search(r"struct GraphEdgeView\s*\{[^}]*\bprops\b", text, flags=re.S):
            return ["domain.rs GraphEdgeView 含 props 字段（出参泄露脱敏前原文）"]
        return None
    if rel == GRAPH:
        # 命令内核必须经 View 转换（删 props）；错误必须走稳定 code()（零 secret echo）。
        if "GraphNodeView::from" not in text or "GraphEdgeView::from" not in text:
            return ["graph.rs 图谱出参未经 GraphNodeView/GraphEdgeView 转换（props 可能外泄）"]
        if "fn code(" not in text:
            return ["graph.rs 缺 GraphError::code（稳定错误码缺失，错误会 echo 内部串）"]
        return None
    return None


ALL_CODES = [
    ("GRAPH_CONSTANTS_PRESENT", c_constants_present),
    ("GRAPH_PROPS_EQ_MAX_TEXT_FIELD", c_props_eq_max_text_field),
    ("GRAPH_PRIVACY_DOUBLE_SCAN", c_privacy_double_scan),
    ("GRAPH_BOUNDED_STORE", c_bounded_store),
    ("GRAPH_TRAVERSAL_BOUNDED", c_traversal_bounded),
    ("GRAPH_REF_NODE_INTEGRITY", c_ref_node_integrity),
    ("GRAPH_NO_SECOND_PATH", c_no_second_path),
    ("GRAPH_OUTPUT_NO_PROPS", c_output_no_props),
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
pub const MAX_TEXT_FIELD_BYTES: usize = 64 * 1024;
pub const GRAPH_PROPS_MAX_BYTES: usize = MAX_TEXT_FIELD_BYTES;
pub const GRAPH_LABEL_MAX_BYTES: usize = 256;
pub const GRAPH_NODE_ID_HEX_LEN: usize = 64;
pub const GRAPH_MAX_DEPTH: usize = 4;
pub const GRAPH_QUERY_LIMIT: usize = 1_000;
pub const GRAPH_MAX_NODES: usize = 5_000;
pub const GRAPH_MAX_EDGES: usize = 20_000;
pub struct GraphNodeView { pub id: String, pub kind: GraphNodeKind, pub label: String }
pub struct GraphEdgeView { pub from: String, pub to: String, pub kind: GraphEdgeKind }
"""

GOOD_GRAPH = """
use crate::domain::*;
const SENSITIVE_KEY_NAMES: &[&str] = &["token","password","secret","api_key"];
const SENSITIVE_VALUE_PATTERNS: &[&str] = &["sk-","AKIA","Bearer ","eyJ","-----BEGIN"];
fn is_hex(s: &str, len: usize) -> bool { s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit()) }
fn scan_graph_value(s: &str) -> bool { SENSITIVE_VALUE_PATTERNS.iter().any(|p| s.contains(p)) }
fn graph_props_contain_secret(props: &GraphProps) -> bool {
    for key in props.keys() { if SENSITIVE_KEY_NAMES.iter().any(|n| key.contains(n)) { return true; } }
    for val in props.values() { if scan_graph_value(val) { return true; } }
    false
}
fn validate_id(id: &str, kind: GraphNodeKind) -> Result<(), GraphError> {
    if kind == GraphNodeKind::Skill || kind == GraphNodeKind::Agent {
        if !is_hex(id, GRAPH_NODE_ID_HEX_LEN) { return Err(GraphError::RefIdNotHex); }
    }
    Ok(())
}
pub fn insert_node(&mut self, node: GraphNode) -> Result<(), GraphError> {
    if self.nodes.len() >= GRAPH_MAX_NODES { return Err(GraphError::NodeCapacityExceeded); }
    Ok(())
}
pub fn insert_edge(&mut self, edge: GraphEdge) -> Result<(), GraphError> {
    if self.edges.len() >= GRAPH_MAX_EDGES { return Err(GraphError::EdgeCapacityExceeded); }
    Ok(())
}
pub fn bounded_neighbors(&self, start_id: &str, depth: usize, limit: usize) -> Vec<GraphNode> {
    let depth = depth.min(GRAPH_MAX_DEPTH);
    let limit = limit.min(GRAPH_QUERY_LIMIT);
    Vec::new()
}
pub fn graph_query_impl(s: &GraphStore) -> GraphQueryResult {
    let _ = s.nodes.iter().map(GraphNodeView::from).collect::<Vec<GraphNodeView>>();
    let _ = s.edges.iter().map(GraphEdgeView::from).collect::<Vec<GraphEdgeView>>();
    GraphQueryResult {}
}
impl GraphError { pub fn code(&self) -> &'static str { "GRAPH_INVALID_ID" } }
"""


def _baseline_repo() -> dict[str, str]:
    return {DOMAIN: GOOD_DOMAIN, GRAPH: GOOD_GRAPH}


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
        "GRAPH_CONSTANTS_PRESENT",
        "domain.rs 缺 GRAPH_MAX_EDGES",
        mutate(**{DOMAIN: GOOD_DOMAIN.replace(
            "pub const GRAPH_MAX_EDGES: usize = 20_000;\n", "")}),
        DOMAIN,
    ))
    bad_cases.append((
        "GRAPH_PROPS_EQ_MAX_TEXT_FIELD",
        "domain.rs 缺 GRAPH_PROPS_MAX_BYTES（须等于 MAX_TEXT_FIELD_BYTES）",
        mutate(**{DOMAIN: GOOD_DOMAIN.replace(
            "pub const GRAPH_PROPS_MAX_BYTES: usize = MAX_TEXT_FIELD_BYTES;\n", "")}),
        DOMAIN,
    ))
    bad_cases.append((
        "GRAPH_PRIVACY_DOUBLE_SCAN",
        "graph.rs 只扫字段名不扫值",
        mutate(**{GRAPH: GOOD_GRAPH.replace(
            'const SENSITIVE_VALUE_PATTERNS: &[&str] = &["sk-","AKIA","Bearer ","eyJ","-----BEGIN"];',
            'const SENSITIVE_VALUE_PATTERNS: &[&str] = &[];')}),
        GRAPH,
    ))
    bad_cases.append((
        "GRAPH_BOUNDED_STORE",
        "graph.rs 未接 GRAPH_MAX_EDGES",
        mutate(**{GRAPH: GOOD_GRAPH.replace(
            "if self.edges.len() >= GRAPH_MAX_EDGES { return Err(GraphError::EdgeCapacityExceeded); }\n", "")}),
        GRAPH,
    ))
    bad_cases.append((
        "GRAPH_TRAVERSAL_BOUNDED",
        "graph.rs 未接 GRAPH_MAX_DEPTH",
        mutate(**{GRAPH: GOOD_GRAPH.replace(
            "let depth = depth.min(GRAPH_MAX_DEPTH);\n", "")}),
        GRAPH,
    ))
    bad_cases.append((
        "GRAPH_REF_NODE_INTEGRITY",
        "graph.rs 未校验 Skill/Agent id 十六进制",
        mutate(**{GRAPH: GOOD_GRAPH.replace(
            "if !is_hex(id, GRAPH_NODE_ID_HEX_LEN) { return Err(GraphError::RefIdNotHex); }\n", "")}),
        GRAPH,
    ))
    bad_cases.append((
        "GRAPH_NO_SECOND_PATH",
        "graph.rs 出现第二执行路径",
        mutate(**{GRAPH: GOOD_GRAPH.replace(
            "use crate::domain::*;\n",
            "use crate::domain::*;\nuse std::process::Command;\n")}),
        GRAPH,
    ))
    bad_cases.append((
        "GRAPH_OUTPUT_NO_PROPS",
        "domain.rs GraphNodeView 含 props 字段（出参泄露）",
        mutate(**{DOMAIN: GOOD_DOMAIN.replace(
            "pub struct GraphNodeView { pub id: String, pub kind: GraphNodeKind, pub label: String }",
            "pub struct GraphNodeView { pub id: String, pub kind: GraphNodeKind, pub label: String, pub props: GraphProps }")}),
        DOMAIN,
    ))
    bad_cases.append((
        "GRAPH_OUTPUT_NO_PROPS",
        "graph.rs 出参未经 View 转换（props 可能外泄）",
        mutate(**{GRAPH: GOOD_GRAPH.replace(
            "GraphNodeView::from", "leak_raw_node")}),
        GRAPH,
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
        print("GRAPH_POLICY_SELF_TEST=FAIL")
        return 1
    print(f"GRAPH_POLICY_SELF_TEST=PASS（ACTIVE={len(code_names)}）")
    return 0


# ---------------------------------------------------------------------------
# 模式
# ---------------------------------------------------------------------------


def _mode_expect_pending() -> int:
    repo = _scan_real_repo()
    if GRAPH in repo:
        print("GRAPH_PENDING_RESULT=FAIL：已检测到 M5-7/8 产物（graph.rs 已落地），"
              "应把本脚本作为默认门禁接入 pre-merge.sh")
        return 1
    print("GRAPH_PENDING_RESULT=NONE（graph.rs 尚未落地，W5 前应处此态）")
    return 0


def _mode_default() -> int:
    repo = _scan_real_repo()
    hits = detect_hits(repo)
    if not hits:
        print("GRAPH_POLICY=PASS（无违规）")
        return 0
    for code, details in sorted(hits.items()):
        for d in details:
            print(f"  {code}: {d}")
    print("GRAPH_POLICY=FAIL")
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
