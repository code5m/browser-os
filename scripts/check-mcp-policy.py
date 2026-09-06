#!/usr/bin/env python3
"""把 M5-2 MCP/rmcp 的静态安全不变量暴露成可复现的准入夹具。

契约来源（均来自本仓库既有交付，非本脚本发明）：
- `logs/assist/A3-M5-mcp-20260906-0757.md`（A3 M5-2 主篇）§3~§8：rmcp/tokio 与 F-1 的冲突、
  stdio-only 禁 TCP、能力白名单单一真源、McpGlobalPolicy fail-closed、复用 security_policy。
- `logs/assist/A3-M5-mcp-20260906-0815-reconcile.md`（A3 补篇）：B1/B7/B8/B3 四类阻塞项与
  `check-mcp-policy.py` 的 10 个码位草案。
- `logs/assist/A3-M5-mcp-20260906-0827-W1-delta.md`（A3 W1 delta）：阶段一「同 package 双 target」
  下 `rmcp`+`tokio` 不能靠独立 bin 隔离，须改 `optional=true` + `[[bin]] required-features=["mcp"]`；
  并新增 `MCP_OPTIONAL_DEP` / `MCP_BIN_GATED`，把 `MCP_TREE_TAURI` 降级 PENDING。
- `logs/checkpoints/M5-20260906/M5-2-rmcp-mcp-policy.md`（A1 M5-2 卡，注意：**截至本脚本落盘仍
  未修订**——仍含 `mcp_server_start/stop` 命令、未处理 B1/B7/B8；本脚本的 PENDING 码位正是为
  将来修订后的实现守门）。
- `logs/checkpoints/A0-M5-W1-dispatch-20260906-0835.md`（A0 W1 指派与硬停止）。
- `logs/assist/A11-M5-verification-matrix-20260906-0820.md` §3.5.0 / D53：M5 策略脚本未落地，
  `check-mcp-policy.py` 归属 A3/A14（M5-2），须 `--self-test` 转 DEFAULT 并进 pre-merge.sh。
- `PARALLEL_COMMAND_BOARD.md` §M5-W1 Implementation Dispatch → W1 Hard Stops：
  「No rmcp, tokio, npm package, MCP server, Agent runtime, graph runtime, plugin runtime,
  new Tauri command, or ACL entry in W1」。

本脚本的定位（**关键，避免与 W1 硬停止冲突**）：
- 它是**准入门禁脚本**，不是 MCP 运行时代码。它**不引入** rmcp / tokio / npm / MCP server / 新命令 /
  ACL 条目；只读取既有文件做静态断言。
- 它对齐 A2 的 `scripts/check-core-boundary.py` 范式：「**产物存在才判**」——M5-2 代码尚未落地时，
  PENDING 码位自动 no-op，默认扫描必然 EXIT 0；`--expect-pending` 验证「MCP 产物尚未出现」。
- 一旦 M5-2 实现开始（A0 把板子翻到 W2），把相关 PENDING 码位的 gate 去掉、转 ACTIVE 即可，
  不需要重写脚本。

为什么需要这个夹具（编译器与人工都守不住）：
- 纯 Rust rmcp 会把 `tokio` 作为 **normal 非 optional** 依赖带进来（A3 主篇 §3.1 实测）；
  阶段一同 package 双 target 下 `[dependencies]` 对 lib/bin 同时生效（A2 §1 第 2 条实测）——
  因此「MCP 不能污染主二进制依赖图」只能由本脚本在 pre-merge 阶段断言。
- 能力白名单若被多处各自定义（Skill / A2A / MCP / Plugin），会静默漂移（A11 D46）。
- `read_file`/`list_dir` 在仓库里**无路径根策略**（A3 补篇 §3 / `check-security-policy.py` 的
  `READ_ONLY_BROWSE_WITHOUT_PATH_POLICY` 已知缺口）；经 MCP 暴露后从「本地可读性缺口」升级为
  「远程任意文件读」——必须机器守门。

用法：
  python3 scripts/check-mcp-policy.py                 默认扫描（无违规 → EXIT 0；有违规 → EXIT 2）
  python3 scripts/check-mcp-policy.py --self-test     好样本 + 坏样本双向自检（含变异防呆）
  python3 scripts/check-mcp-policy.py --expect-pending 验证 MCP 产物尚未落地（W1 守门）
"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import sys
from typing import Callable, Optional

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# ---------------------------------------------------------------------------
# 工具
# ---------------------------------------------------------------------------

def _read(path: str) -> Optional[str]:
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            return fh.read()
    except OSError:
        return None


def _rel(path: str) -> str:
    return os.path.relpath(path, ROOT)


def _mcp_present(repo: dict[str, str]) -> bool:
    """PENDING 码位的统一 gate：仓库里是否已有任何 M5-2 产物。

    判定依据（任一即视为 MCP 已开建）：
      - `src-tauri/Cargo.toml` 出现 `rmcp`
      - 存在目录 `src-tauri/src/mcp_tools/`
      - 存在文件 `src-tauri/src/bin/mcp_server.rs`
      - `src-tauri/src/**` 出现 `MCP_CAPABILITY_V1` 常量定义
    """
    for rel, text in repo.items():
        if rel == "src-tauri/Cargo.toml" and "rmcp" in text:
            return True
        if rel.startswith("src-tauri/src/mcp_tools/"):
            return True
        if rel == "src-tauri/src/bin/mcp_server.rs":
            return True
        if rel.startswith("src-tauri/src/") and rel.endswith(".rs"):
            if re.search(r"\bMCP_CAPABILITY_V1\s*[=:]", text):
                return True
    return False


# ---------------------------------------------------------------------------
# 码位定义
# 每个码位是一个函数 (rel, text, repo) -> Optional[list[str]]
#   返回 None          := 不适用 / 干净（不计入违规）
#   返回 [detail, ...] := 命中违规，detail 为可读说明
# kind: "ACTIVE"（始终守门） | "PENDING"（仅当 _mcp_present 为真时守门）
# ---------------------------------------------------------------------------

_CODE_REX = {
    "MCP_NPM_SDK_PRESENT": re.compile(r'"@modelcontextprotocol/'),
    "MCP_NPM_IN_CARGO": re.compile(r"\bnpm\b"),
    "MCP_NODE_RUNTIME_PRESENT": re.compile(r'Command::new\s*\(\s*["\'](?:node|npx)["\']'),
    "MCP_LISTEN_PORT": re.compile(r"TcpListener|\.bind\s*\(|axum|hyper::Server|tokio::net::TcpListener|std::net::TcpListener"),
    "MCP_RUNTIME_LEAK": re.compile(r"tokio::|#\[tokio::main"),
    "MCP_TOOL_CALLS_COMMAND": re.compile(r"crate::bridge|bridge::|crate::terminal|::db_query\("),
}

# 仅出现在 MCP 专属文件（mcp_server / mcp_tools）里的形态才允许
_MCP_FILE_RE = re.compile(r"mcp_server|mcp_tools")


def _is_mcp_file(rel: str) -> bool:
    return bool(_MCP_FILE_RE.search(rel)) and rel.endswith(".rs")


def _is_src_rs(rel: str) -> bool:
    return rel.startswith("src-tauri/src/") and rel.endswith(".rs")


def _is_cargo(rel: str) -> bool:
    return rel.endswith("Cargo.toml")


def _is_pkg(rel: str) -> bool:
    return rel.endswith("package.json") or rel.endswith("package-lock.json")


# ---- ALWAYS-ACTIVE（硬红线，与 M5-2 是否开建无关）----


def c_npm_sdk(rel, text, repo):
    if _is_pkg(rel) and _CODE_REX["MCP_NPM_SDK_PRESENT"].search(text):
        return ["package.json 出现 @modelcontextprotocol npm SDK（禁 npm 分包）"]
    return None


def c_npm_cargo(rel, text, repo):
    if _is_cargo(rel) and _CODE_REX["MCP_NPM_IN_CARGO"].search(text):
        return ["Cargo.toml 出现 `npm`（禁 npm 分包，M5-2 必须是纯 Rust rmcp）"]
    return None


def c_node_runtime(rel, text, repo):
    if _is_src_rs(rel) and _CODE_REX["MCP_NODE_RUNTIME_PRESENT"].search(text):
        return ["src 内 Command::new(\"node\"|\"npx\")（禁 Node 运行时，必须纯 Rust）"]
    return None


# ---- PENDING（仅当 MCP 产物已存在时守门）----


def c_listen_port(rel, text, repo):
    if not _mcp_present(repo):
        return None
    if _is_mcp_file(rel) and _CODE_REX["MCP_LISTEN_PORT"].search(text):
        return ["MCP 文件出现 TCP 监听形态（首期仅 stdio，禁端口）"]
    return None


def c_runtime_leak(rel, text, repo):
    if not _mcp_present(repo):
        return None
    # 允许出现在 MCP 专属 bin / 工具目录；其余 src 出现 tokio 即视为漏进主构建
    if _is_src_rs(rel) and not _is_mcp_file(rel) and _CODE_REX["MCP_RUNTIME_LEAK"].search(text):
        return ["非 MCP 专属文件出现 tokio::（tokio 应通过 optional+required-features 隔离到 mcp_server bin）"]
    return None


def c_optional_dep(rel, text, repo):
    if not _mcp_present(repo):
        return None
    if not _is_cargo(rel):
        return None
    problems = []
    # rmcp / tokio 若存在，必须 optional=true，且由 [features] mcp 启用，且不在 default
    for dep in ("rmcp", "tokio"):
        m = re.search(re.escape(dep) + r"\s*=\s*\{[^}]*\}", text)
        if m and "optional" not in m.group(0):
            problems.append(f"{dep} 未声明 optional=true（会污染主二进制依赖图，违反 F-1 实质）")
    # default features 不得含 mcp
    dm = re.search(r"\[features\][^\[]*?default\s*=\s*\[(.*?)\]", text, re.S)
    if dm and re.search(r"\bmcp\b", dm.group(1)):
        problems.append("[features] default 含 mcp（默认构建会引入 tokio，违反 F-1 实质）")
    # 存在 [[bin]] mcp_server 必须有 required-features
    return problems or None


def c_bin_gated(rel, text, repo):
    if not _mcp_present(repo):
        return None
    if not _is_cargo(rel):
        return None
    for bm in re.finditer(r"\[\[bin\]\][^\[]*?name\s*=\s*[\"']mcp_server[\"']", text, re.S):
        block = bm.group(0)
        if "required-features" not in block or "mcp" not in re.search(r"required-features\s*=\s*\[(.*?)\]", block, re.S).group(1):
            return ["[[bin]] mcp_server 缺少 required-features=[\"mcp\"]（未隔离会导致默认构建编译 MCP）"]
    return None


def c_tool_calls_command(rel, text, repo):
    if not _mcp_present(repo):
        return None
    if rel.startswith("src-tauri/src/mcp_tools/") and _CODE_REX["MCP_TOOL_CALLS_COMMAND"].search(text):
        return ["mcp_tools 调用 bridge::/terminal:: 命令层（MCP 工具必须调 core 内部 API，不得绕过来源校验）"]
    return None


def c_path_policy_missing(rel, text, repo):
    if not _mcp_present(repo):
        return None
    if rel.startswith("src-tauri/src/mcp_tools/") and re.search(r"\b(?:read_file|list_dir)\b", text):
        if "check_path_within_roots" not in text:
            return ["mcp_tools 暴露 read_file/list_dir 但缺 check_path_within_roots（远程任意文件读风险，B8）"]
    return None


def c_url_not_redacted(rel, text, repo):
    if not _mcp_present(repo):
        return None
    if rel.startswith("src-tauri/src/mcp_tools/") and re.search(r"tab_list|redact_sensitive_url|\.url\b", text):
        if "redact_sensitive_url" not in text:
            return ["mcp_tools 回传 URL 但缺 redact_sensitive_url（URL 可能含敏感 query，B9）"]
    return None


# ---- GLOBAL（需跨文件/跨 Cargo 上下文）----


def c_capability_drift(rel, text, repo):
    # 单一真源：MCP_CAPABILITY_V1 常量定义必须恰好出现在一个文件
    defs = [r for r, t in repo.items()
            if r.startswith("src-tauri/src/") and r.endswith(".rs")
            and re.search(r"\bMCP_CAPABILITY_V1\s*[=:]", t)]
    if len(defs) > 1:
        return [f"MCP_CAPABILITY_V1 在多处定义（漂移）：{sorted(_rel(d) for d in defs)}"]
    # 被引用但零定义
    refs = [r for r, t in repo.items()
            if r.startswith("src-tauri/src/") and r.endswith(".rs")
            and "MCP_CAPABILITY_V1" in t]
    if refs and not defs:
        return ["MCP_CAPABILITY_V1 被引用但无定义（单一真源缺失）"]
    return None


def c_parity(rel, text, repo):
    # 若 handler 暴露 mcp_* 命令，则 ACL 与 bridge.ts 必须同时出现（防止 ACL 缺口，B5/§2.2 F-1）
    mcp_cmds = set()
    handler = repo.get("src-tauri/src/main.rs", "") + repo.get("src-tauri/src/bridge.rs", "")
    for m in re.finditer(r"\b(mcp_server_start|mcp_server_stop|mcp_policy_get|mcp_policy_set)\b", handler):
        mcp_cmds.add(m.group(1))
    if not mcp_cmds:
        return None
    acl = repo.get("src-tauri/permissions/default-commands.toml", "")
    bts = repo.get("src/bridge.ts", "")
    missing = [c for c in mcp_cmds if c not in acl or c not in bts]
    if missing:
        return [f"mcp 命令缺 ACL/bridge.ts 奇偶：{sorted(missing)}"]
    return None


def c_tree_tauri(_rel, _text, _repo):
    # PENDING：阶段一同 package 双 target 下 cargo tree -p mvp-core 必含 tauri（见 A3 W1 delta §3）。
    # 本脚本不在默认模式跑 cargo tree（慢且阶段一必然 FAIL）；阶段二独立 crate 后再接 ACTIVE。
    return None


# 码位登记表
ACTIVE_CODES = [
    ("MCP_NPM_SDK_PRESENT", "ACTIVE", c_npm_sdk),
    ("MCP_NPM_IN_CARGO", "ACTIVE", c_npm_cargo),
    ("MCP_NODE_RUNTIME_PRESENT", "ACTIVE", c_node_runtime),
    ("MCP_CAPABILITY_DRIFT", "ACTIVE", c_capability_drift),
]
PENDING_CODES = [
    ("MCP_LISTEN_PORT", "PENDING", c_listen_port),
    ("MCP_RUNTIME_LEAK", "PENDING", c_runtime_leak),
    ("MCP_OPTIONAL_DEP", "PENDING", c_optional_dep),
    ("MCP_BIN_GATED", "PENDING", c_bin_gated),
    ("MCP_TOOL_CALLS_COMMAND", "PENDING", c_tool_calls_command),
    ("MCP_PATH_POLICY_MISSING", "PENDING", c_path_policy_missing),
    ("MCP_URL_NOT_REDACTED", "PENDING", c_url_not_redacted),
    ("MCP_PARITY", "PENDING", c_parity),
    ("MCP_TREE_TAURI", "PENDING", c_tree_tauri),
]
ALL_CODES = ACTIVE_CODES + PENDING_CODES


# ---------------------------------------------------------------------------
# 扫描
# ---------------------------------------------------------------------------


def _scan_real_repo() -> dict[str, str]:
    repo: dict[str, str] = {}
    want = [
        "src-tauri/Cargo.toml",
        "src-tauri/permissions/default-commands.toml",
        "src/bridge.ts",
        "src/types.ts",
        "package.json",
        "package-lock.json",
    ]
    for w in want:
        t = _read(os.path.join(ROOT, w))
        if t is not None:
            repo[w] = t
    # src-tauri/src/**/*.rs
    base = os.path.join(ROOT, "src-tauri/src")
    if os.path.isdir(base):
        for root, _dirs, files in os.walk(base):
            for fn in files:
                if fn.endswith(".rs"):
                    full = os.path.join(root, fn)
                    t = _read(full)
                    if t is not None:
                        repo[os.path.relpath(full, ROOT)] = t
    return repo


def detect_hits(repo: dict[str, str]) -> dict[str, list[str]]:
    hits: dict[str, list[str]] = {}
    for name, _kind, fn in ALL_CODES:
        for rel, text in repo.items():
            res = fn(rel, text, repo)
            if res:
                hits.setdefault(name, []).extend(res)
    return hits


# ---------------------------------------------------------------------------
# 自测
# ---------------------------------------------------------------------------


def _baseline_repo() -> dict[str, str]:
    # 注意：基线**不含**任何 M5-2 产物（无 mcp_tools 目录 / 无 mcp_server bin / 无 rmcp /
    # 无 MCP_CAPABILITY_V1），否则 _mcp_present 会被提前置真、PENDING gate 失效。
    return {
        "src-tauri/Cargo.toml": (
            "[package]\nname = \"mvp-browser-os\"\n"
            "[dependencies]\ntauri = { version = \"2\" }\nrusqlite = { version = \"0.31\" }\n"
        ),
        "src-tauri/src/bridge.rs": (
            "pub fn list_artifacts(app: AppHandle) -> Vec<Artifact> {\n"
            "    workspace::load_artifacts(&app)\n}\n"
        ),
        "src-tauri/src/main.rs": "fn main() { tauri::Builder::default(); }\n",
        "src-tauri/src/core/mod.rs": "pub mod keyring_store;\n",
        "src-tauri/permissions/default-commands.toml": "[commands]\nlist_artifact_images\n",
        "src/bridge.ts": "export function invoke(name: string) {}\n",
        "package.json": "{\"dependencies\":{\"react\":\"^18\"}}\n",
        "package-lock.json": "{}\n",
    }


# PENDING 坏样本需要「M5-2 已开建」才能让 gate 生效；最小产物 = 一个空的 mcp_server bin。
_MCP_ARTIFACT = {"src-tauri/src/bin/mcp_server.rs": "fn main() {}\n"}


def _with_artifact(**kw) -> dict[str, str]:
    d = dict(_MCP_ARTIFACT)
    d.update(kw)
    return d


def _run_self_test() -> int:
    failures: list[str] = []

    # 0) 码位登记表自洽：ACTIVE ∩ PENDING 必须为空
    overlap = set(n for n, _, _ in ACTIVE_CODES) & set(n for n, _, _ in PENDING_CODES)
    if overlap:
        failures.append(f"码位重复定义（ACTIVE ∩ PENDING）：{sorted(overlap)}")

    good = _baseline_repo()

    # 1) 好样本：必须零违规（无误报）
    good_hits = detect_hits(good)
    if good_hits:
        failures.append(f"好样本误报：{good_hits}")

    # 2) 坏样本：每个码位至少一个，且必须真的改动内容（变异防呆），且必须被检出
    def mutate(**kw):
        d = dict(good)
        d.update(kw)
        return d

    bad_cases: list[tuple[str, str, dict[str, str]]] = []

    def add(code: str, desc: str, mutated: dict[str, str], key: str) -> None:
        if mutated.get(key, "") == good.get(key, ""):
            print(f"self-test FAIL: 坏样本「{desc}」未改动任何内容（变异失配，按漏检计）")
            sys.exit(1)
        bad_cases.append((code, desc, mutated))

    # ACTIVE 坏样本（无需 MCP 产物）
    add("MCP_NPM_SDK_PRESENT", "package.json 含 @modelcontextprotocol/sdk",
        mutate(**{"package.json": '{"dependencies":{"@modelcontextprotocol/sdk":"^1"}}'}), "package.json")
    add("MCP_NPM_IN_CARGO", "Cargo.toml 含 npm",
        mutate(**{"src-tauri/Cargo.toml": "[dependencies]\nmcp_js = { path = \"crates/mcp\", npm = true }\n"}), "src-tauri/Cargo.toml")
    add("MCP_NODE_RUNTIME_PRESENT", "src 内 Command::new(\"node\")",
        mutate(**{"src-tauri/src/bridge.rs": 'fn run() { std::process::Command::new("node").spawn(); }\n'}), "src-tauri/src/bridge.rs")

    # PENDING 坏样本：必须带最小 MCP 产物（_MCP_ARTIFACT）才能让 gate 生效
    add("MCP_LISTEN_PORT", "mcp_server 出现 TcpListener",
        _with_artifact(**{"src-tauri/src/bin/mcp_server.rs": "let _ = tokio::net::TcpListener::bind(\"127.0.0.1:9999\");\n"}),
        "src-tauri/src/bin/mcp_server.rs")
    add("MCP_RUNTIME_LEAK", "非 MCP 专属文件出现 tokio::",
        _with_artifact(**{"src-tauri/src/foo.rs": "pub fn f() { let _ = tokio::spawn(async {}); }\n"}),
        "src-tauri/src/foo.rs")
    add("MCP_OPTIONAL_DEP", "rmcp 非 optional",
        _with_artifact(**{"src-tauri/Cargo.toml": "[dependencies]\nrmcp = { version = \"3\" }\n"}),
        "src-tauri/Cargo.toml")
    add("MCP_BIN_GATED", "mcp_server bin 缺 required-features",
        _with_artifact(**{"src-tauri/Cargo.toml": "[dependencies]\nrmcp = { version = \"3\", optional = true }\n[[bin]]\nname = \"mcp_server\"\npath = \"src/bin/mcp_server.rs\"\n"}),
        "src-tauri/Cargo.toml")
    add("MCP_TOOL_CALLS_COMMAND", "mcp_tools 调 bridge::db_query",
        _with_artifact(**{"src-tauri/src/mcp_tools/db.rs": "pub fn run() { crate::bridge::db_query(Default::default(), \"x\".into()); }\n"}),
        "src-tauri/src/mcp_tools/db.rs")
    add("MCP_PATH_POLICY_MISSING", "mcp_tools 暴露 read_file 缺根策略",
        _with_artifact(**{"src-tauri/src/mcp_tools/fs.rs": "pub fn read() { let _ = bridge::read_file(\"x\"); }\n"}),
        "src-tauri/src/mcp_tools/fs.rs")
    add("MCP_URL_NOT_REDACTED", "mcp_tools 回传 url 缺脱敏",
        _with_artifact(**{"src-tauri/src/mcp_tools/tabs.rs": "pub fn tabs() -> Vec<String> { tab_list() }\n"}),
        "src-tauri/src/mcp_tools/tabs.rs")
    add("MCP_CAPABILITY_DRIFT", "MCP_CAPABILITY_V1 定义两处",
        mutate(**{"src-tauri/src/core/mod.rs": "const MCP_CAPABILITY_V1: &[&str] = &[];\n",
                  "src-tauri/src/extra.rs": "const MCP_CAPABILITY_V1: &[&str] = &[];\n"}),
        "src-tauri/src/core/mod.rs")
    add("MCP_PARITY", "mcp_server_start 只在 handler 不在 ACL/bridge.ts",
        _with_artifact(**{"src-tauri/src/bridge.rs": "pub fn mcp_server_start(app: AppHandle) {}\n"}),
        "src-tauri/src/bridge.rs")
    # MCP_TREE_TAURI 是 PENDING 主动 no-op，不要求坏样本检出（其本身是「期望尚未实现」）

    for code, _desc, mutated in bad_cases:
        h = detect_hits(mutated)
        if code not in h:
            failures.append(f"坏样本未检出码位 {code}（漏检）；命中={h}")

    # 3) 坏样本中 PENDING 码位必须依赖 mcp 产物存在：单独构造「无 mcp 产物」的坏内容，
    #    断言 PENDING 码位不误触发（gate 生效）
    no_mcp = mutate(**{"src-tauri/src/bridge.rs": "pub fn f() { let _ = tokio::spawn(async {}); }\n"})
    no_mcp_hits = detect_hits(no_mcp)
    leaked = [c for c in no_mcp_hits if c in ("MCP_RUNTIME_LEAK", "MCP_OPTIONAL_DEP", "MCP_BIN_GATED")]
    if leaked:
        failures.append(f"无 MCP 产物时 PENDING 码位误触发（gate 失效）：{leaked}")

    if failures:
        for f in failures:
            print(f"  - {f}")
        print("MCP_POLICY_SELF_TEST=FAIL")
        return 1
    print(f"MCP_POLICY_SELF_TEST=PASS（ACTIVE={len(ACTIVE_CODES)}，PENDING={len(PENDING_CODES)}）")
    return 0


# ---------------------------------------------------------------------------
# 模式
# ---------------------------------------------------------------------------


def _mode_expect_pending() -> int:
    repo = _scan_real_repo()
    if _mcp_present(repo):
        print("MCP_PENDING_RESULT=FAIL：已检测到 M5-2 产物（rmcp / mcp_tools / mcp_server bin / "
              "MCP_CAPABILITY_V1），应把本脚本 PENDING 码位翻为 ACTIVE 并由 W2 实现接管")
        return 1
    print(f"MCP_PENDING_RESULT=NONE（{len(PENDING_CODES)} 个 pending 码位均未实现，W1 守门通过）")
    return 0


def _mode_default() -> int:
    repo = _scan_real_repo()
    hits = detect_hits(repo)
    if not hits:
        print("MCP_POLICY=PASS（无违规）")
        return 0
    for code, details in sorted(hits.items()):
        for d in details:
            print(f"  {code}: {d}")
    print("MCP_POLICY=FAIL")
    return 2


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--self-test", action="store_true", help="好/坏样本双向自检")
    ap.add_argument("--expect-pending", action="store_true", help="验证 MCP 产物尚未落地（W1 守门）")
    args = ap.parse_args()

    if args.self_test:
        return _run_self_test()
    if args.expect_pending:
        return _mode_expect_pending()

    # 默认：只跑默认扫描。self-test 由 pre-merge.sh 单独调用（与 check-core-boundary.py 同范式），
    # 不在默认模式内嵌，避免单次 pre-merge 重复执行。
    return _mode_default()


if __name__ == "__main__":
    sys.exit(main())
