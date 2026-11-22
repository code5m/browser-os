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
- 它对齐 A2 的 `scripts/check-core-boundary.py` 范式。W1 曾以 PENDING 码位「产物存在才判」
  （`--expect-pending` 验证 MCP 产物尚未落地）；W7 A3 以**只读 Tauri 命令桥**落地 MCP
  （mcp_policy_get / mcp_registry_list / mcp_capability_preview），W1 的「独立 rmcp server + mcp_tools」
  架构被 W7 Hard Stop 永久否决。故 W8 收口 W1 语义：PENDING 码位全部退役，rmcp/server/listener
  由单一 **ACTIVE** 守门 `MCP_NO_RMCP_SERVER` 约束 MCP Owner，并新增 `--expect-current-gaps` 当前相位断言。

为什么需要这个夹具（编译器与人工都守不住）：
- 纯 Rust rmcp 会把 `tokio` 作为 **normal 非 optional** 依赖带进来（A3 主篇 §3.1 实测）；
  阶段一同 package 双 target 下 `[dependencies]` 对 lib/bin 同时生效（A2 §1 第 2 条实测）——
  当前 BFC 与 Agent Foundation 已有独立的 tokio 使用；因此本脚本拒绝桌面直接 rmcp 依赖，
   并在 MCP 注册表、stdio 模块及实际 mcp_* 命令体内禁止网络与异步运行时，不能把其它 Owner
   的 tokio 依赖或 loopback 测试误判为 MCP server。
- 能力白名单若被多处各自定义（Skill / A2A / MCP / Plugin），会静默漂移（A11 D46）。
- `read_file`/`list_dir` 在仓库里**无路径根策略**（A3 补篇 §3 / `check-security-policy.py` 的
  `READ_ONLY_BROWSE_WITHOUT_PATH_POLICY` 已知缺口）；经 MCP 暴露后从「本地可读性缺口」升级为
  「远程任意文件读」——必须机器守门。

用法：
  python3 scripts/check-mcp-policy.py                 默认扫描（无违规 → EXIT 0；有违规 → EXIT 2）
  python3 scripts/check-mcp-policy.py --self-test     好样本 + 坏样本双向自检（含变异防呆）
  python3 scripts/check-mcp-policy.py --expect-current-gaps  当前相位断言（W10：只读桥在 + gated stdio 骨架已落地、无未门控 rmcp/server/listener）
"""

from __future__ import annotations

import argparse
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


def _mcp_bridge_present(repo: dict[str, str]) -> bool:
    """当前相位（W8）断言：只读 MCP 桥是否已落地。

    A3 W7 以只读 Tauri 命令桥落地 MCP；本函数确认三条命令均已在 main.rs 注册、
    ACL（default-commands.toml）与前端封装（bridge.ts）齐备。供 `--expect-current-gaps` 使用。
    """
    main = repo.get("src-tauri/src/main.rs", "")
    acl = repo.get("src-tauri/permissions/default-commands.toml", "")
    bts = repo.get("src/bridge.ts", "")
    cmds = ("mcp_policy_get", "mcp_registry_list", "mcp_capability_preview")
    return (
        all(f"bridge::{c}" in main for c in cmds)
        and all(f'"{c}"' in acl for c in cmds)
        and all(f'"{c}"' in bts for c in cmds)
        and all(sum(name == c for r, t in repo.items() if _is_src_rs(r)
                    for name, _body in _mcp_command_bodies(t)) == 1 for c in cmds)
    )


# ---------------------------------------------------------------------------
# 码位定义
# 每个码位是一个函数 (rel, text, repo) -> Optional[list[str]]
#   返回 None          := 不适用 / 干净（不计入违规）
#   返回 [detail, ...] := 命中违规，detail 为可读说明
# kind: "ACTIVE"（始终守门）。W8 起不再有 PENDING 码位（W1 相位债已关闭）。
# ---------------------------------------------------------------------------

_CODE_REX = {
    "MCP_NPM_SDK_PRESENT": re.compile(r'"@modelcontextprotocol/'),
    "MCP_NPM_IN_CARGO": re.compile(r"\bnpm\b"),
    "MCP_NODE_RUNTIME_PRESENT": re.compile(r'Command::new\s*\(\s*["\'](?:node|npx)["\']'),
}

def _is_src_rs(rel: str) -> bool:
    return rel.startswith("src-tauri/src/") and rel.endswith(".rs")


def _is_cargo(rel: str) -> bool:
    return rel.endswith("Cargo.toml")


def _is_pkg(rel: str) -> bool:
    return rel.endswith("package.json") or rel.endswith("package-lock.json")


_MCP_STDIO_FILES = ("src-tauri/src/bin/mcp_server.rs", "src-tauri/src/mcp_server.rs")
_MCP_REGISTRY_FILES = ("src-tauri/src/mcp.rs",)
_MCP_NETWORK = re.compile(
    r"\b(?:axum|hyper::Server|TcpListener|TcpStream|UdpSocket)\b"
)


def _without_comment_lines(text: str) -> str:
    # Keep string literals and code intact; explanatory whole-line comments
    # (including the original no-TcpListener contract) are not executable imports.
    return re.sub(r"(?m)^\s*//[^\n]*", "", text)


def _mcp_command_bodies(text: str) -> list[tuple[str, str]]:
    text = _without_comment_lines(text)
    return [(match.group(1), _extract_fn_body(text, match.end()))
            for match in _RE_MCP_CMD.finditer(text)]


def _mcp_owner_sections(rel: str, text: str) -> list[str]:
    """Follow the MCP surface, including commands after physical bridge extraction.

    The integration bridge also owns plugin lifecycle commands; only mcp_*
    command bodies belong to this read-only surface. Other capability Owners
    and their test listeners are not MCP transports.
    """
    if not _is_src_rs(rel):
        return []
    if (rel in _MCP_REGISTRY_FILES + _MCP_STDIO_FILES
            or rel.startswith("src-tauri/src/mcp_tools/")):
        return [_without_comment_lines(text)]
    return [body for _name, body in _mcp_command_bodies(text)]


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


def c_fs_tool_policy(rel, text, repo):
    # 注册表文件：touches_fs / returns_url 必须与路径根 / URL 脱敏守门一致（B8/B9）。
    if "MCP_COMMAND_REGISTRY" not in text:
        return None
    problems = []
    if "touches_fs" in text and "check_path_within_roots" not in text:
        problems.append("注册表含 touches_fs 项但缺 check_path_within_roots（远程任意文件读风险，B8）")
    if "returns_url" in text and "redact_sensitive_url" not in text:
        problems.append("注册表含 returns_url 项但缺 redact_sensitive_url（URL 敏感信息泄露，B9）")
    return problems or None


def c_no_rmcp_server(rel, text, repo):
    # Desktop MCP remains a read-only bridge plus gated stdio skeleton.
    # Runtime dependencies used by BFC/Agent Foundation are governed by those
    # Owners; they do not grant MCP permission to use a runtime or listener.
    problems = []
    if _is_cargo(rel):
        cargo = re.sub(r"(?m)^\s*#.*$", "", text)
        if (re.search(r"(?m)^\s*rmcp\s*=", cargo)
                or re.search(r'package\s*=\s*["\']rmcp["\']', cargo)
                or re.search(r"(?m)^\s*\[[^\]\n]*dependencies\.rmcp\]", cargo)):
            problems.append("桌面 Cargo.toml 引入 rmcp 依赖（MCP 只读桥不得启用独立 rmcp server）")
        for block in re.split(r"(?m)^\s*\[\[bin\]\]\s*$", cargo)[1:]:
            # A new TOML table ends this bin, while required-features' array
            # belongs to the current key and must remain available below.
            block = re.split(r"(?m)^\s*\[(?![\s\"'])", block, maxsplit=1)[0]
            if not re.search(r'\bname\s*=\s*["\']mcp_server["\']', block):
                continue
            required = re.search(r"required-features\s*=\s*\[([^\]]*)\]", block, re.S)
            if not required or not re.search(r'["\']mcp["\']', required.group(1)):
                problems.append('mcp_server 二进制缺少 required-features = ["mcp"] 门控')
    for section in _mcp_owner_sections(rel, text):
        section = _without_comment_lines(section)
        if _MCP_NETWORK.search(section):
            problems.append(f"{rel} 的 MCP Owner 含网络监听/传输（MCP 只准 stdio）")
        if re.search(r"\b(?:rmcp|tokio)\s*::|\buse\s+(?:rmcp|tokio)\b", section):
            problems.append(f"{rel} 的 MCP Owner 引入 rmcp/tokio 运行时（只读 stdio 边界）")
    if rel.startswith("src-tauri/src/mcp_tools/") and rel.endswith(".rs"):
        if not re.search(r'#!\s*\[\s*cfg\(\s*feature\s*=\s*"mcp"\s*\)\s*\]', text):
            problems.append(f'{rel} 非自门控（mcp_tools/ 必须以 #![cfg(feature="mcp")] 门控）')
    return problems or None


def c_server_gated(rel, text, repo):
    # W10 A3：允许的 MCP stdio-prep 骨架（feature-gated、std-only、无网络监听、无执行副作用）。
    # 仅当仓库存在 MCP 运行时预备件（src-tauri/src/bin/mcp_server.rs 或 src-tauri/src/mcp_server.rs）
    # 时才守门；它必须：
    #   1) 自门控：文件以 #![cfg(feature = "mcp")] 开头，确保默认构建不编译；
    #   2) Cargo.toml 声明 [features] mcp（feature 门控存在）；
    #   3) 不得含网络/监听/进程派生/执行副作用（axum/hyper::Server/TcpListener/TcpStream/UdpSocket/
    #      Command::new/spawn/std::process）；
    #   4) 必须走 stdio（使用 std::io 读 stdin），而非网络传输。
    # 命中任一违规 ⇒ 视为「未正确门控的 MCP server」⇒ 报错（与 W7 Hard Stop 一致）。
    if rel not in _MCP_STDIO_FILES:
        return None
    text = _without_comment_lines(text)
    problems = []
    if not re.search(r'#!\s*\[\s*cfg\(\s*feature\s*=\s*"mcp"\s*\)\s*\]', text):
        problems.append('MCP 运行时预备件未以 #![cfg(feature="mcp")] 自门控（默认构建不得编译）')
    cargo = repo.get("src-tauri/Cargo.toml", "")
    feature_block = re.search(r"(?ms)^\[features\]\s*\n(.*?)(?=^\[|\Z)", cargo)
    if not feature_block or not re.search(r"(?m)^\s*mcp\s*=", feature_block.group(1)):
        problems.append("Cargo.toml 缺少 [features] mcp（feature 门控缺失）")
    if re.search(
        r"\baxum\b|\bhyper::Server\b|tokio::net::TcpListener|std::net::TcpListener|"
        r"std::net::TcpStream|TcpListener|TcpStream|UdpSocket",
        text,
    ):
        problems.append("MCP 骨架含网络监听/传输形态（禁 TCP/HTTP，只准 stdio）")
    if re.search(r"Command::new|\.spawn\(|std::process", text):
        problems.append("MCP 骨架含进程派生/执行副作用（MCP_NO_SECOND_PATH 红线）")
    if "std::io" not in text:
        problems.append("MCP 骨架未使用 std::io（stdio 传输缺失）")
    return problems or None


def c_stdio_no_arg_echo(rel, text, repo):
    # W11 Hard Stop（PARALLEL_COMMAND_BOARD.md 行 208）：stdio 响应 / 日志 / audit /
    # 检查点 / UI 状态 不得回显原始 argument/query/token/cookie/Authorization。
    # 本码位静态守护 stdio 传输层（mcp_server.rs）：其 JSON-RPC 响应构造不得把请求
    # 的 params/arguments 原始内容插值进 format! 文本、序列化进响应、或打印进日志。
    # 仅当仓库存在 MCP stdio 预备件时才守门（与 `MCP_SERVER_GATED` 同源文件判定）。
    if rel not in _MCP_STDIO_FILES:
        return None
    problems = []
    # 1) 不得直接序列化 params/arguments 进响应文本（可能回显原始 argument）。
    if re.search(r"to_string\s*\(\s*&?\s*params\s*\)", text) or \
       re.search(r"to_string\s*\(\s*&?\s*arguments\s*\)", text):
        problems.append("stdio 响应直接序列化 params/arguments（可能回显原始 argument）")
    # 2) format! 中不得插值 {params} / {arguments}（原始 argument 回显）。
    if re.search(r"format!\s*\([^)]*\{\s*params\s*\}", text) or \
       re.search(r"format!\s*\([^)]*\{\s*arguments\s*\}", text):
        problems.append("format! 把 params/arguments 插值进响应文本（原始 argument 回显）")
    # 3) 不得用 println/eprintln 输出 params（日志 / 审计泄漏原始 argument）。
    if re.search(r"(?:eprintln|println)\s*\([^)]*params", text):
        problems.append("stdio 用 println/eprintln 输出 params（日志泄漏原始 argument）")
    return problems or None


def c_stdio_bounded(rel, text, repo):
    # W11 A3：stdio dry-run 必须有界——输入行与响应序列化都要有字节上限，且不得退回
    # 无限行读取（`stdin.lock().lines()` 会为超大行无限分配）。守门要点：
    #   1) 必须声明 MAX_INPUT_BYTES / MAX_RESPONSE_BYTES 两个上限常量（单一真源）；
    #   2) 必须存在有界读取函数（read_line_bounded 之类）并被 stdio 环调用；
    #   3) 不得出现无界 `stdin.lock().lines()` 读取（越界请求可撑爆内存）。
    # 与 `MCP_SERVER_GATED` 同源文件判定（产物不存在则不守门）。
    if rel not in _MCP_STDIO_FILES:
        return None
    problems = []
    if not re.search(r"const\s+MAX_INPUT_BYTES\s*:", text):
        problems.append("stdio 缺少 MAX_INPUT_BYTES 输入上限常量（无界读取风险）")
    if not re.search(r"const\s+MAX_RESPONSE_BYTES\s*:", text):
        problems.append("stdio 缺少 MAX_RESPONSE_BYTES 响应上限常量（无界响应风险）")
    # 注：函数带泛型参数（`fn read_line_bounded<R: BufRead>(...)`），正则须放行 `<...>`。
    if not re.search(r"fn\s+read_line_bounded\s*(?:<[^>]*>)?\s*\(", text):
        problems.append("stdio 缺少有界行读取函数 read_line_bounded（超大行可无限分配）")
    if re.search(r"stdin\(\)\s*\.lock\(\)\s*\.lines\(\)|stdin\.lock\(\)\s*\.lines\(\)", text):
        problems.append("stdio 使用无界 .lines() 读取（应走 read_line_bounded 有界读）")
    return problems or None


def c_graph_not_exposed(rel, text, repo):
    # W12 A3（MCP REVIEW ONLY 的守门固化）：W12 的图谱只读命令（`graph_query` /
    # `graph_node_get` / `graph_stats`）是 **Tauri 侧命令**，不得经 MCP stdio 暴露成
    # 可执行工具，也不得进入 MCP 能力白名单 / 注册表 / stdio 骨架
    # （W12 Hard Stop 行 209：MCP full runtime 仍然 LOCKED）。
    # 守门文件：`mcp.rs`（命令注册表）、`domain.rs`（`MCP_CAPABILITY_V1` 白名单块）、
    # `mcp_server.rs`（stdio 骨架）。命中任一图谱能力标识即判
    # 「图谱能力被接进 MCP 面」；未解锁前必须显式改本码位并留 A0 裁决记录。
    if rel == "src-tauri/src/domain.rs":
        block = re.search(r"MCP_CAPABILITY_V1\s*:\s*&\[&str\]\s*=\s*&\[(.*?)\]", text, re.S)
        text = block.group(1) if block else ""
    else:
        text = "\n".join(_mcp_owner_sections(rel, text))
    problems = []
    for m in ("graph_query", "graph_node_get", "graph_stats", "GraphState",
              "crate::graph", "crate::capabilities::graph"):
        if m in text:
            problems.append(
                f"{rel} 引用图谱能力 `{m}`（图谱只读命令不得经 MCP 暴露；"
                "W12 Hard Stop：MCP full runtime LOCKED）"
            )
    return problems or None


def c_plugin_not_exposed(rel, text, repo):
    # W13 A3（MCP REVIEW ONLY 的守门固化）：W13 的插件生命周期命令（`plugin_install` /
    # `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_*`，以及
    # 受信密钥增删）是 **Tauri 侧命令**，由 A9 在 W13 实现；它们不得经 MCP stdio 暴露成
    # 可执行工具，也不得进入 MCP 能力白名单 / 注册表 / stdio 骨架
    # （W13 Hard Stop：plugin invoke / command execution / MCP full runtime 仍然 LOCKED）。
    # 守门文件：`mcp.rs`（命令注册表）、`domain.rs`（`MCP_CAPABILITY_V1` 白名单块）、
    # `mcp_server.rs`（stdio 骨架）。命中任一插件能力标识即判
    # 「插件能力被接进 MCP 面」；未解锁前必须显式改本码位并留 A0 裁决记录。
    # 与 W12 的 `MCP_GRAPH_NOT_EXPOSED` 同源范式。
    if rel == "src-tauri/src/domain.rs":
        block = re.search(r"MCP_CAPABILITY_V1\s*:\s*&\[&str\]\s*=\s*&\[(.*?)\]", text, re.S)
        text = block.group(1) if block else ""
    else:
        text = "\n".join(_mcp_owner_sections(rel, text))
    problems = []
    for m in ("plugin_install", "plugin_enable", "plugin_disable", "plugin_list",
             "plugin_get", "plugin_key", "plugin_uninstall", "crate::plugin",
              "crate::capabilities::plugin"):
        if m in text:
            problems.append(
                f"{rel} 引用插件能力 `{m}`（插件生命周期命令不得经 MCP 暴露；"
                "W13 Hard Stop：plugin invoke / MCP full runtime LOCKED）"
            )
    return problems or None


# ---- PENDING（W8 已退役）----
# W1 阶段曾以 7 个 PENDING 码位（`MCP_LISTEN_PORT` / `MCP_RUNTIME_LEAK` / `MCP_OPTIONAL_DEP` /
# `MCP_BIN_GATED` / `MCP_TOOL_CALLS_COMMAND` / `MCP_PATH_POLICY_MISSING` / `MCP_URL_NOT_REDACTED`）
# 守「独立 rmcp server」架构，约定「产物存在才判」。W7 A3 以只读 Tauri 命令桥落地 MCP，
# 该 server 架构被 W7 Hard Stop 永久否决，故 W8 将其收口为单一 ACTIVE 守门 `MCP_NO_RMCP_SERVER`
# （见上方 ALWAYS-ACTIVE 段）。`MCP_TREE_TAURI` 亦退役（core 纯洁性由 check-core-boundary.py 守）。
# 当前无任何 PENDING 码位；相位债已关闭。


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
    # 任何以 `bridge::mcp_` 注册的命令（main.rs generate_handler 中）必须同时具备
    # ACL（default-commands.toml）与 bridge.ts 前端封装，否则存在 ACL/前端缺口（B5/§2.2 F-1）。
    # 比首期「硬编码白名单」更稳：后续新增的 mcp_* 命令（如本次 W7 的
    # mcp_policy_get / mcp_registry_list / mcp_capability_preview）一律受同一奇偶守门。
    if rel != "src-tauri/src/main.rs":
        return None
    mcp_cmds = set()
    for m in re.finditer(r"bridge::(mcp_[a-z_]+)", text):
        mcp_cmds.add(m.group(1))
    if not mcp_cmds:
        return None
    acl = repo.get("src-tauri/permissions/default-commands.toml", "")
    bts = repo.get("src/bridge.ts", "")
    missing = [c for c in mcp_cmds if (f'"{c}"' not in acl) or (f'"{c}"' not in bts)]
    problems = []
    if missing:
        problems.append(f"mcp 命令缺 ACL/bridge.ts 奇偶：{sorted(missing)}")
    definitions = [name for r, t in repo.items() if _is_src_rs(r)
                   for name, _body in _mcp_command_bodies(t)]
    unresolved = [c for c in mcp_cmds if definitions.count(c) != 1]
    if unresolved:
        problems.append(f"已注册 mcp 命令缺少唯一实际 Owner 定义：{sorted(unresolved)}")
    return problems or None


_RE_MCP_CMD = re.compile(
    r"#\[tauri::command(?:\([^\]]*\))?\]\s*"
    r"(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(mcp_[a-z_]+)\s*\("
)
# 只读约束（W7 Hard Stop）：mcp_* 命令不得含写/执行类副作用。
_FORBIDDEN_SIDE_EFFECT = re.compile(
    r"\b(?:std::fs::write|fs::write|write_file|create_file|create_dir|delete_path|"
    r"rename_path|script_runner|run_command|db_connect|db_query|db_disconnect|"
    r"spawn|Command::new|\.store\(|insert\(|emit\()"
)


def _extract_fn_body(text: str, start: int) -> str:
    """从签名起点提取函数体（首个 `{` 起，括号配平到 0）。"""
    i = text.find("{", start)
    if i < 0:
        return ""
    depth = 0
    j = i
    while j < len(text):
        c = text[j]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return text[i + 1 : j]
        j += 1
    return text[i + 1 :]


def c_bridge_readonly(rel, text, repo):
    # mcp_* 只读桥命令不得引入写/执行类副作用（与 W7 Hard Stop「只读」一致）。
    if not _is_src_rs(rel):
        return None
    problems = []
    for name, body in _mcp_command_bodies(text):
        if _FORBIDDEN_SIDE_EFFECT.search(_without_comment_lines(body)):
            problems.append(f"mcp 命令 {name} 含写/执行类副作用（W7 只读约束）")
    return problems or None


# 码位登记表
ACTIVE_CODES = [
    ("MCP_NPM_SDK_PRESENT", "ACTIVE", c_npm_sdk),
    ("MCP_NPM_IN_CARGO", "ACTIVE", c_npm_cargo),
    ("MCP_NODE_RUNTIME_PRESENT", "ACTIVE", c_node_runtime),
    ("MCP_CAPABILITY_DRIFT", "ACTIVE", c_capability_drift),
    ("MCP_FS_TOOL_PATH_POLICY", "ACTIVE", c_fs_tool_policy),
    ("MCP_BRIDGE_READONLY", "ACTIVE", c_bridge_readonly),
    ("MCP_NO_RMCP_SERVER", "ACTIVE", c_no_rmcp_server),
    ("MCP_SERVER_GATED", "ACTIVE", c_server_gated),
    ("MCP_STDIO_NO_ARG_ECHO", "ACTIVE", c_stdio_no_arg_echo),
    ("MCP_STDIO_BOUNDED", "ACTIVE", c_stdio_bounded),
    ("MCP_GRAPH_NOT_EXPOSED", "ACTIVE", c_graph_not_exposed),
    ("MCP_PLUGIN_NOT_EXPOSED", "ACTIVE", c_plugin_not_exposed),
    ("MCP_PARITY", "ACTIVE", c_parity),
]
# W8：W1 的 7 个 PENDING 码位 + MCP_TREE_TAURI 全部退役，相位债已关闭。当前无 PENDING 码位。
PENDING_CODES: list[tuple[str, str, Callable]] = []
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
    # 注意：基线**不含**任何 MCP 只读桥产物（无 mcp_policy_get / mcp_registry_list /
    # mcp_capability_preview 注册、无 rmcp / 无 mcp_server bin / 无 mcp_tools 目录），
    # 否则 _mcp_bridge_present 会误判「桥已落地」、当前相位断言失真。
    return {
        "src-tauri/Cargo.toml": (
            "[package]\nname = \"mvp-browser-os\"\nversion = \"0.1.0\"\n"
            "[features]\nmcp = []\n"
            "[dependencies]\ntauri = { version = \"2\" }\nrusqlite = { version = \"0.31\" }\n"
        ),
        "src-tauri/src/mcp_server.rs": (
            "#![cfg(feature = \"mcp\")]\n"
            "use std::io::{self, BufRead, Write};\n"
            "const MAX_INPUT_BYTES: usize = 1048576;\n"
            "const MAX_RESPONSE_BYTES: usize = 4194304;\n"
            "fn read_line_bounded<R: BufRead>(r: &mut R, max: usize) -> Option<Vec<u8>> {\n"
            "    let _ = (r, max);\n    None\n}\n"
            "pub fn run_stdio() {\n"
            "    let stdin = io::stdin();\n"
            "    let mut stdout = io::stdout().lock();\n"
            "    let mut reader = stdin.lock();\n"
            "    while let Some(line) = read_line_bounded(&mut reader, MAX_INPUT_BYTES) {\n"
            "        let resp = handle_request(&line);\n"
            "        let s = serde_json::to_string(&resp).unwrap_or_default();\n"
            "        if s.len() <= MAX_RESPONSE_BYTES {\n"
            "            let _ = writeln!(stdout, \"{s}\");\n"
            "            let _ = stdout.flush();\n"
            "        }\n"
            "    }\n"
            "}\n"
            "fn handle_request(req: &[u8]) -> serde_json::Value {\n"
            "    let _ = req;\n"
            "    serde_json::json!({})\n"
            "}\n"
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

    # Distinct Owners may use tokio or loopback listeners. The current bridge
    # is physically split and shares a file with plugin lifecycle commands.
    owned_good = dict(good)
    owned_good["src-tauri/Cargo.toml"] += (
        'tokio = { version = "1", features = ["net", "rt"] }\n'
        '[[bin]]\nname = "mcp_server"\nrequired-features = ["mcp"]\n'
    )
    owned_good["src-tauri/src/capabilities/browser/bfc.rs"] = (
        "use tokio::net::TcpStream;\n"
        "async fn exchange() { let _ = TcpStream::connect(\"127.0.0.1:1\").await; }\n"
        "#[cfg(test)] mod tests { use std::net::TcpListener; }\n"
    )
    commands = ("mcp_policy_get", "mcp_registry_list", "mcp_capability_preview")
    owned_good["src-tauri/src/bridge.rs"] = (
        "mod integration_commands;\npub use self::integration_commands::*;\n"
    )
    owned_good["src-tauri/src/bridge/integration_commands.rs"] = "".join(
        f"#[tauri::command]\npub fn {command}() {{ current_policy_snapshot(); }}\n"
        for command in commands
    ) + (
        "#[tauri::command]\npub fn plugin_install() { fs::write(\"manifest\", \"{}\"); }\n"
    )
    owned_good["src-tauri/src/main.rs"] = (
        "fn main() { tauri::generate_handler!["
        + ", ".join(f"bridge::{command}" for command in commands) + "]; }\n"
    )
    owned_good["src-tauri/permissions/default-commands.toml"] = (
        "allow = [" + ", ".join(f'"{command}"' for command in commands) + "]\n"
    )
    owned_good["src/bridge.ts"] = "\n".join(f'invoke("{command}");' for command in commands)
    owner_hits = detect_hits(owned_good)
    if owner_hits or not _mcp_bridge_present(owned_good):
        failures.append(f"分离 Owner 的真实桥正例误报或未识别：{owner_hits}")

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
    add("MCP_FS_TOOL_PATH_POLICY", "注册表 touches_fs 缺 check_path_within_roots",
        mutate(**{"src-tauri/src/mcp_tools/registry.rs": "pub const MCP_COMMAND_REGISTRY: &[McpCommandDef] = &[ McpCommandDef { capability: \"file_read\", touches_fs: true, returns_url: true } ];\n"}),
        "src-tauri/src/mcp_tools/registry.rs")

    # W8：W1 的 7 个 PENDING 坏样本收口为单一 ACTIVE 守门 `MCP_NO_RMCP_SERVER` 的坏样本
    # （Cargo.toml 引入 rmcp 即触发；不再有「MCP 产物才判」的 pending 语义）。
    add("MCP_NO_RMCP_SERVER", "Cargo.toml 引入 rmcp 依赖",
        mutate(**{"src-tauri/Cargo.toml": "[dependencies]\nrmcp = { version = \"3\" }\ntauri = { version = \"2\" }\n"}),
        "src-tauri/Cargo.toml")
    # W10：gated stdio 骨架的自门控 + 无网络守门（MCP_SERVER_GATED）。坏样本：未自门控且含网络监听。
    add("MCP_SERVER_GATED", "mcp_server.rs 未自门控且含 TCP 监听",
        mutate(**{"src-tauri/src/bin/mcp_server.rs":
                  "use std::net::TcpListener;\nfn run() { let _ = TcpListener::bind(\"127.0.0.1:0\"); }\n"}),
        "src-tauri/src/bin/mcp_server.rs")
    # W11：stdio 响应不得回显原始 params/arguments（MCP_STDIO_NO_ARG_ECHO）。
    add("MCP_STDIO_NO_ARG_ECHO", "mcp_server.rs 把 params/arguments 原始内容插值进响应文本",
        mutate(**{"src-tauri/src/mcp_server.rs":
                  '#![cfg(feature = "mcp")]\nuse std::io::{self, BufRead, Write};\n'
                  'pub fn run_stdio() {\n'
                  '    let stdin = io::stdin();\n    let mut stdout = io::stdout().lock();\n'
                  '    for line in stdin.lock().lines() {\n'
                  '        let line = match line { Ok(l) => l, Err(_) => break };\n'
                  '        let params = "user_secret_arg";\n'
                  '        let err = format!("invalid request: {params}");\n'
                  '        let _ = serde_json::to_string(&params);\n'
                  '        let _ = writeln!(stdout, "{err}");\n    }\n}\n'}),
        "src-tauri/src/mcp_server.rs")
    # W11：stdio 必须是有界的（MCP_STDIO_BOUNDED）。坏样本：退回无界 .lines() 读取且无上限常量。
    add("MCP_STDIO_BOUNDED", "mcp_server.rs 退回无界 .lines() 读取、无输入/响应上限",
        mutate(**{"src-tauri/src/mcp_server.rs":
                  '#![cfg(feature = "mcp")]\nuse std::io::{self, BufRead, Write};\n'
                  'pub fn run_stdio() {\n'
                  '    let stdin = io::stdin();\n'
                  '    let mut stdout = io::stdout().lock();\n'
                  '    for line in stdin.lock().lines() {\n'
                  '        let line = match line { Ok(l) => l, Err(_) => break };\n'
                  '        let resp = handle_request(&line);\n'
                  '        if let Ok(s) = serde_json::to_string(&resp) {\n'
                  '            let _ = writeln!(stdout, "{s}");\n'
                  '        }\n'
                  '    }\n}\n'
                  'fn handle_request(req: &str) -> serde_json::Value { let _ = req; serde_json::json!({}) }\n'}),
        "src-tauri/src/mcp_server.rs")
    # W12：图谱只读命令不得经 MCP 暴露（MCP_GRAPH_NOT_EXPOSED）。
    add("MCP_GRAPH_NOT_EXPOSED", "mcp_server.rs 直接调 crate::graph::graph_query 暴露成 MCP 工具",
        mutate(**{"src-tauri/src/mcp_server.rs":
                  '#![cfg(feature = "mcp")]\nuse std::io::{self, BufRead, Write};\n'
                  'const MAX_INPUT_BYTES: usize = 1048576;\n'
                  'const MAX_RESPONSE_BYTES: usize = 4194304;\n'
                  'fn read_line_bounded<R: BufRead>(r: &mut R, max: usize) -> Option<Vec<u8>> {\n'
                  '    let _ = (r, max);\n    None\n}\n'
                  'pub fn run_stdio() { let _ = crate::graph::graph_query("n1", 2, 1000); }\n'}),
        "src-tauri/src/mcp_server.rs")
    # W13：插件生命周期命令不得经 MCP 暴露（MCP_PLUGIN_NOT_EXPOSED）。
    add("MCP_PLUGIN_NOT_EXPOSED", "mcp_server.rs 直接调 crate::plugin::plugin_install 暴露成 MCP 工具",
        mutate(**{"src-tauri/src/mcp_server.rs":
                  '#![cfg(feature = "mcp")]\nuse std::io::{self, BufRead, Write};\n'
                  'const MAX_INPUT_BYTES: usize = 1048576;\n'
                  'const MAX_RESPONSE_BYTES: usize = 4194304;\n'
                  'fn read_line_bounded<R: BufRead>(r: &mut R, max: usize) -> Option<Vec<u8>> {\n'
                  '    let _ = (r, max);\n    None\n}\n'
                  'pub fn run_stdio() { let _ = crate::plugin::plugin_install("x", "y"); }\n'}),
        "src-tauri/src/mcp_server.rs")
    add("MCP_CAPABILITY_DRIFT", "MCP_CAPABILITY_V1 定义两处",
        mutate(**{"src-tauri/src/core/mod.rs": "const MCP_CAPABILITY_V1: &[&str] = &[];\n",
                  "src-tauri/src/extra.rs": "const MCP_CAPABILITY_V1: &[&str] = &[];\n"}),
        "src-tauri/src/core/mod.rs")
    add("MCP_PARITY", "mcp 命令在 handler 缺 ACL/bridge.ts",
        mutate(**{
            "src-tauri/src/main.rs": "fn main() { tauri::generate_handler![bridge::mcp_server_start]; }\n",
            "src-tauri/src/bridge.rs": "pub fn mcp_server_start(app: AppHandle) {}\n",
        }),
        "src-tauri/src/main.rs")
    add("MCP_BRIDGE_READONLY", "mcp 命令含写副作用",
        mutate(**{"src-tauri/src/bridge.rs":
            "#[tauri::command]\npub fn mcp_policy_set(app: AppHandle, webview: tauri::Webview, v: String) { std::fs::write(\"/x\", v).unwrap(); }\n"}),
        "src-tauri/src/bridge.rs")
    # Owner scoping must not turn migrated commands or stdio helpers into gaps.
    add("MCP_NO_RMCP_SERVER", "rmcp 依赖改名仍拒绝",
        mutate(**{"src-tauri/Cargo.toml":
                  '[dependencies]\nserver_sdk = { package = "rmcp", version = "3" }\n'}),
        "src-tauri/Cargo.toml")
    add("MCP_NO_RMCP_SERVER", "MCP 注册表使用 tokio runtime",
        mutate(**{"src-tauri/src/mcp.rs":
                  "fn run() { let _ = tokio::runtime::Runtime::new(); }\n"}),
        "src-tauri/src/mcp.rs")
    add("MCP_NO_RMCP_SERVER", "MCP 注册表导入监听器",
        mutate(**{"src-tauri/src/mcp.rs":
                  'use std::net::TcpListener;\nfn run() { TcpListener::bind("127.0.0.1:0"); }\n'}),
        "src-tauri/src/mcp.rs")
    add("MCP_NO_RMCP_SERVER", "已门控 mcp_tools 仍不得网络监听",
        mutate(**{"src-tauri/src/mcp_tools/network.rs":
                  '#![cfg(feature = "mcp")]\nuse std::net::TcpListener;\n'}),
        "src-tauri/src/mcp_tools/network.rs")
    for features in ("", 'required-features = []\n', 'required-features = ["other"]\n'):
        add("MCP_NO_RMCP_SERVER", "mcp_server bin 必须门控实际 mcp feature",
            mutate(**{"src-tauri/Cargo.toml":
                      good["src-tauri/Cargo.toml"]
                      + '[[bin]]\nname = "mcp_server"\n' + features}),
            "src-tauri/Cargo.toml")
    owner = "src-tauri/src/bridge/integration_commands.rs"
    add("MCP_NO_RMCP_SERVER", "迁移后的 MCP 桥不得监听",
        mutate(**{owner: '#[tauri::command]\npub fn mcp_registry_list() {'
                         ' std::net::TcpListener::bind("127.0.0.1:0"); }\n'}), owner)
    add("MCP_BRIDGE_READONLY", "迁移后的 MCP 桥不得写文件",
        mutate(**{owner: '#[tauri::command]\npub fn mcp_policy_get() {'
                         ' std::fs::write("policy", "value"); }\n'}), owner)
    add("MCP_BRIDGE_READONLY", "异步/带属性 MCP 桥仍受只读守门",
        mutate(**{owner: '#[tauri::command(rename_all = "snake_case")]\n'
                         'pub async fn mcp_registry_list() { db_query(); }\n'}), owner)
    add("MCP_GRAPH_NOT_EXPOSED", "迁移后的 MCP 桥不得绑定 Graph Owner",
        mutate(**{owner: '#[tauri::command]\npub fn mcp_policy_get() {'
                         ' crate::capabilities::graph::commands::graph_stats(); }\n'}), owner)
    add("MCP_PLUGIN_NOT_EXPOSED", "迁移后的 MCP 桥不得绑定 Plugin Owner",
        mutate(**{owner: '#[tauri::command]\npub fn mcp_policy_get() {'
                         ' crate::capabilities::plugin::install(); }\n'}), owner)
    add("MCP_GRAPH_NOT_EXPOSED", "bin 位置的 stdio server 同样不得暴露图谱",
        mutate(**{"src-tauri/src/bin/mcp_server.rs":
                  good["src-tauri/src/mcp_server.rs"]
                  + "\nfn exposed() { crate::graph::graph_query(); }\n"}),
        "src-tauri/src/bin/mcp_server.rs")
    missing_owner = dict(owned_good)
    missing_owner[owner] = "// MCP Owner definitions removed.\n"
    add("MCP_PARITY", "handler/ACL 存在但真实 MCP Owner 被移除", missing_owner, owner)
    if _mcp_bridge_present(missing_owner):
        failures.append("移除实际 Owner 后，桥存在性检查仍误报成功")

    for code, _desc, mutated in bad_cases:
        h = detect_hits(mutated)
        if code not in h:
            failures.append(f"坏样本未检出码位 {code}（漏检）；命中={h}")

    # 3) 当前无 PENDING 码位（W8 相位债已关闭），无需再构造「无 MCP 产物」的 gate 测试。

    if failures:
        for f in failures:
            print(f"  - {f}")
        print("MCP_POLICY_SELF_TEST=FAIL")
        return 1
    print(f"MCP_POLICY_SELF_TEST=PASS（ACTIVE={len(ACTIVE_CODES)}，"
          f"PENDING={len(PENDING_CODES)}，BAD={len(bad_cases)}）")
    return 0


# ---------------------------------------------------------------------------
# 模式
# ---------------------------------------------------------------------------


def _mode_expect_current_gaps() -> int:
    # 当前 Owner 契约：只读 MCP 桥已落地；其内部不得引入 runtime/listener 或执行副作用。
    repo = _scan_real_repo()
    hits = detect_hits(repo)
    problems: list[str] = []
    if not _mcp_bridge_present(repo):
        problems.append("MCP 只读桥未落地（W8 期望 mcp_policy_get / mcp_registry_list / "
                        "mcp_capability_preview 已在 main.rs 注册且 ACL + bridge.ts 齐备）")
    # 任一 ACTIVE 码位命中即违反 W8 相位（红线 / 奇偶 / 只读 不应出现）
    blocked = tuple(name for name, _kind, _fn in ACTIVE_CODES)
    for code in blocked:
        if code in hits:
            problems.append(f"{code} 命中（W10 不应出现）：{hits[code]}")
    if problems:
        for p in problems:
            print(f"  - {p}")
        print("MCP_CURRENT_GAPS_RESULT=FAIL")
        return 1
    print("MCP_CURRENT_GAPS_RESULT=PASS（W10 相位：只读桥在 + gated stdio 骨架已落地，"
          "MCP Owner 无 runtime/listener，奇偶/只读/红线性门禁全绿）")
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
    ap.add_argument("--expect-current-gaps", action="store_true",
                    help="当前相位断言（W10：只读桥在 + gated stdio 骨架已落地、无未门控 rmcp/server/listener）")
    args = ap.parse_args()

    if args.self_test:
        return _run_self_test()
    if args.expect_current_gaps:
        return _mode_expect_current_gaps()

    # 默认：只跑默认扫描。self-test / 当前相位断言 由 pre-merge.sh 单独调用
    # （与 check-core-boundary.py 同范式），不在默认模式内嵌，避免单次 pre-merge 重复执行。
    return _mode_default()


if __name__ == "__main__":
    sys.exit(main())
