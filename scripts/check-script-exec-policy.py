#!/usr/bin/env python3
"""Expose the M2-4 script execution invariants as a reproducible fixture.

契约来源：
- `logs/checkpoints/M2-4.a-20260903-2233.md`（执行安全契约冻结裁定书）
- `logs/checkpoints/M2-4.b-20260904-0000.md` §5/§6（B1~B14 测试矩阵与违规码）
- `logs/checkpoints/M2-4.b-VERDICT-20260904-0705.md`（独立裁定书）

本夹具守住「进程组与生命周期内核」的三条红线：

  1. 命令构造（P0）：禁 `sh -c` / `bash -c` / 字符串拼接，必须 argv 数组。
  2. 进程回收：必须 `setsid` + `killpg` 杀**进程组**；
     禁止沿用 `grid_process.rs` 的 `child.kill()`（只杀直接子进程，留孤儿）。
  3. 不加引号（裁定书 §3.1）：argv 模式下给值加 `'...'` 会把单引号作字面量传进脚本。

模式：
  默认            判定 ACTIVE 码（b/c/d/e 卡职责内 + P0 红线）；零命中 → EXIT 0
  --expect-pending 验证 PENDING 码位集合；M2-4.d 后应为空 → EXIT 0
  --self-test     好坏样本双向自检（含**变异防呆**：坏样本必须真的改动内容）

关于「一次性定义码位」：沿用 M0-3.a 模式，23 个码位一次定义，
默认门禁判定已落地的 b/c/d/e 卡职责。M2-4.d 后 pending 码位为空：
`EXEC_SHUTDOWN_NOT_REGISTERED` / `EXEC_RUN_RECORD_MISSING` / `EXEC_NO_RING_CAP` /
`EXEC_NO_OUTPUT_CAP` 已转入默认判定；M2-4.e 追加 shell spawn 开放面移除检查。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "src-tauri" / "src" / "capabilities" / "script" / "script_runner.rs"
DOMAIN_RS = ROOT / "src-tauri" / "src" / "domain.rs"
MAIN_RS = ROOT / "src-tauri" / "src" / "main.rs"
BRIDGE_RS = ROOT / "src-tauri" / "src" / "bridge.rs"
ACL_TOML = ROOT / "src-tauri" / "permissions" / "default-commands.toml"
CAPABILITY_JSON = ROOT / "src-tauri" / "capabilities" / "default.json"
CARGO_TOML = ROOT / "src-tauri" / "Cargo.toml"
CARGO_LOCK = ROOT / "src-tauri" / "Cargo.lock"
PACKAGE_JSON = ROOT / "package.json"
PACKAGE_LOCK = ROOT / "package-lock.json"

# ----------------------------- 码位定义 -----------------------------

# P0：贯穿 M2-4 全部子卡的命令注入红线
P0_CODES = ("EXEC_SH_C_PRESENT", "EXEC_STRING_INTERPOLATION")

# b 卡职责内（进程组与生命周期内核）
B_CARD_CODES = (
    "EXEC_NO_SETSID",
    "EXEC_NO_KILLPG",
    "EXEC_TIMEOUT_NOT_LAYERED",
    "EXEC_NO_WAIT_REAP",
    "EXEC_ARG_QUOTED",
    "EXEC_PLATFORM_STUB_MISSING",
    "EXEC_ENV_NOT_CLEARED",
    "EXEC_CWD_NOT_LOCKED",
    "EXEC_INTERPRETER_NOT_WHITELISTED",
)

# c 卡职责内（命令层、参数校验接入、审计、ACL 与 handler）
C_CARD_CODES = (
    "EXEC_ACL_MISSING",
    "EXEC_HANDLER_NOT_REGISTERED",
    "EXEC_NO_SOURCE_CHECK",
    "EXEC_VALIDATION_BYPASSED",
    "EXEC_AUDIT_LEAKS_VALUE",
)

# d 卡职责内（输出背压、事件流、运行记录落盘与退出收口）
D_CARD_CODES = (
    "EXEC_SHUTDOWN_NOT_REGISTERED",
    "EXEC_RUN_RECORD_MISSING",
    "EXEC_NO_RING_CAP",
    "EXEC_NO_OUTPUT_CAP",
)

# e 卡职责内（移除无调用点的 Tauri shell spawn 开放面）
E_CARD_CODES = (
    "EXEC_SHELL_SPAWN_CAPABILITY_PRESENT",
    "EXEC_FRONTEND_SHELL_DEP_PRESENT",
    "EXEC_RUST_SHELL_PLUGIN_PRESENT",
)

ACTIVE_CODES = P0_CODES + B_CARD_CODES + C_CARD_CODES + D_CARD_CODES + E_CARD_CODES

PENDING_CODES: tuple[str, ...] = ()

# ----------------------------- 检测规则 -----------------------------

# `sh -c` / `bash -c` 形态（含 argv 数组写法 `"sh", "-c"`）
SH_C_PATTERN = re.compile(
    r"""(?:"(?:sh|bash|zsh|fish)"\s*,\s*"-c")"""
    r"""|(?:'(?:sh|bash|zsh|fish)'\s*,\s*'-c')"""
    r"""|(?:\b(?:sh|bash|zsh|fish)\s+-c\b)"""
)

# argv 元素被单引号包裹：`format!("'{}'"...)` 或 `' + x + '`
ARG_QUOTED_PATTERN = re.compile(
    r"""(?:format!\(\s*"')"""
    r"""|(?:format!\(\s*r#\s*"')"""
    r"""|(?:'\s*\+\s*&?\w+\s*\+\s*')"""
)

# 只杀直接子进程（留孤儿的范式，来自 grid_process.rs:677/743）
DIRECT_CHILD_KILL_PATTERN = re.compile(r"\bchild\.kill\s*\(\s*\)")


def strip_comments(source: str) -> str:
    """去掉 Rust 注释，避免注释里提到的反例被误判为真实代码。"""
    without_blocks = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
    return "\n".join(line.split("//", 1)[0] for line in without_blocks.splitlines())


def rust_fn_body(source: str, name: str) -> str:
    """提取同名函数的**第一个**实现体（用于区分 `#[cfg(unix)]` / `#[cfg(not(unix))]` 两版）。

    只做花括号配平，不解析 Rust 语法；字符串里的花括号可能导致截断，
    但本夹具关注的函数体（spawn/kill）不含字符串花括号。
    """
    match = re.search(rf"\b(?:pub\s+)?fn\s+{re.escape(name)}\s*\(", source)
    if not match:
        return ""
    paren = source.find("(", match.end() - 1)
    depth = 0
    index = paren
    while index < len(source):
        if source[index] == "(":
            depth += 1
        elif source[index] == ")":
            depth -= 1
            if depth == 0:
                break
        index += 1
    body_start = source.find("{", index)
    if body_start < 0:
        return ""
    depth = 0
    i = body_start
    while i < len(source):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[body_start : i + 1]
        i += 1
    return source[body_start:]


def detect(ctx: dict[str, str]) -> set[str]:
    """返回命中的违规码集合（ACTIVE 与 PENDING 一并检测，由调用方按模式过滤）。"""
    hits: set[str] = set()
    runner = ctx["runner"]
    main = ctx["main"]
    bridge = ctx["bridge"]
    acl = ctx["acl"]
    capability = ctx["capability"]
    cargo_toml = ctx["cargo_toml"]
    cargo_lock = ctx["cargo_lock"]
    package_json = ctx["package_json"]
    package_lock = ctx["package_lock"]

    # --- P0：命令注入红线 ---
    if SH_C_PATTERN.search(runner):
        hits.add("EXEC_SH_C_PRESENT")

    # Command 构造窗口内出现 format! → 字符串拼接命令行
    for match in re.finditer(r"Command::new\(", runner):
        window = runner[match.end() : match.end() + 600]
        if "format!(" in window:
            hits.add("EXEC_STRING_INTERPOLATION")
            break

    # --- 进程组与回收（取 cfg(unix) 版函数体，避免被 non-unix 桩误导） ---
    spawn_body = rust_fn_body(runner, "spawn_in_new_group")
    if not ("pre_exec" in spawn_body and "setsid" in spawn_body):
        hits.add("EXEC_NO_SETSID")

    kill_body = rust_fn_body(runner, "kill_group")
    if "killpg" not in kill_body or DIRECT_CHILD_KILL_PATTERN.search(runner):
        hits.add("EXEC_NO_KILLPG")

    # 必须存在 soft/hard 分层的**常量定义**：M5-1 切片 0b 后常量可收口到
    # `domain.rs`（pub const）或保留在 `script_runner.rs`（pub const / pub use re-export）。
    # 只查定义不查使用处，否则把常量改名后使用处残留字符串会让检测蒙混过关。
    domain_src = ctx.get("domain", "")
    hard_grace_present = (
        re.search(r"pub const HARD_GRACE_SECS\s*:", runner)
        or re.search(r"pub use crate::domain::HARD_GRACE_SECS", runner)
        or re.search(r"pub const HARD_GRACE_SECS\s*:", domain_src)
    )
    if not (hard_grace_present and re.search(r"pub const CANCEL_GRACE_SECS\s*:", runner)):
        hits.add("EXEC_TIMEOUT_NOT_LAYERED")

    # 回收必须发生在 supervisor 内，且必须是**非阻塞轮询** `try_wait`
    # （`child.wait()` 只是 cancel/timeout 分支的收尾，只查它会蒙混过关）。
    if "try_wait" not in rust_fn_body(runner, "supervise"):
        hits.add("EXEC_NO_WAIT_REAP")

    if ARG_QUOTED_PATTERN.search(runner):
        hits.add("EXEC_ARG_QUOTED")

    if "#[cfg(not(unix))]" not in runner:
        hits.add("EXEC_PLATFORM_STUB_MISSING")

    # --- env / cwd / 解释器白名单 ---
    if "env_clear()" not in runner:
        hits.add("EXEC_ENV_NOT_CLEARED")
    if "current_dir(" not in runner:
        hits.add("EXEC_CWD_NOT_LOCKED")
    if ".binary()" not in runner:
        hits.add("EXEC_INTERPRETER_NOT_WHITELISTED")

    # --- PENDING：c / d 卡职责 ---
    exec_commands = ("run_script", "cancel_script", "script_status")
    if not all(cmd in acl for cmd in exec_commands):
        hits.add("EXEC_ACL_MISSING")
    if not all(f"bridge::{cmd}" in main for cmd in exec_commands):
        hits.add("EXEC_HANDLER_NOT_REGISTERED")
    for cmd in exec_commands:
        body = rust_fn_body(bridge, cmd)
        if "check_invocation_source" not in body:
            hits.add("EXEC_NO_SOURCE_CHECK")
            break
    shutdown_body = rust_fn_body(bridge, "register_shutdown_tasks")
    if 'coordinator.register("kill-running-scripts"' not in shutdown_body or "kill_all_running" not in shutdown_body:
        hits.add("EXEC_SHUTDOWN_NOT_REGISTERED")
    if not re.search(r"pub\s+struct\s+ScriptRunRecord\b", runner):
        hits.add("EXEC_RUN_RECORD_MISSING")
    if not (
        re.search(r"pub\s+const\s+RING_MAX_BYTES\s*:", runner)
        and re.search(r"pub\s+const\s+RING_MAX_LINES\s*:", runner)
    ):
        hits.add("EXEC_NO_RING_CAP")
    if not re.search(r"pub\s+const\s+SCRIPT_RUN_TAIL_BYTES\s*:", runner):
        hits.add("EXEC_NO_OUTPUT_CAP")

    # --- e 卡：移除 Tauri shell 插件开放面 ---
    if "shell:allow-spawn" in capability:
        hits.add("EXEC_SHELL_SPAWN_CAPABILITY_PRESENT")
    if "@tauri-apps/plugin-shell" in package_json or "@tauri-apps/plugin-shell" in package_lock:
        hits.add("EXEC_FRONTEND_SHELL_DEP_PRESENT")
    if (
        "tauri-plugin-shell" in cargo_toml
        or "tauri-plugin-shell" in cargo_lock
        or "tauri_plugin_shell::init" in main
    ):
        hits.add("EXEC_RUST_SHELL_PLUGIN_PRESENT")
    run_body = rust_fn_body(bridge, "run_script")
    helper_body = rust_fn_body(bridge, "get_enabled_script")
    if "start_run(" not in run_body or "validate_meta" not in helper_body:
        hits.add("EXEC_VALIDATION_BYPASSED")
    if not all(marker in bridge for marker in ("script.run.start", "script.run.cancel", "script.run.status")):
        hits.add("EXEC_AUDIT_LEAKS_VALUE")
    for marker in ("script.run.start", "script.run.cancel", "script.run.status"):
        index = bridge.find(marker)
        window = bridge[index : index + 320] if index >= 0 else ""
        if "values" in window or "body" in window:
            hits.add("EXEC_AUDIT_LEAKS_VALUE")
            break

    return hits


def strip_tests(source: str) -> str:
    """切掉 `#[cfg(test)]` 之后的内容：策略夹具只检测生产代码。

    必须这么做：测试里的**断言消息字符串**会提到反例
    （如 B9 的「child.kill() 做不到这点」），不切掉会被误判为真实代码。
    """
    # 只匹配「测试模块」`#[cfg(test)] mod ...`：**不能**用 `find("#[cfg(test)]")`，
    # 因为单个函数也可能带该属性（如本文件的 `process_group_id_of`），
    # 那样会把其后的生产代码一并切掉。
    match = re.search(r"#\[cfg\(test\)\]\s*\n\s*mod\s", source)
    index = match.start() if match else -1
    return source if index < 0 else source[:index]


def read_ctx() -> dict[str, str]:
    def read(path: Path) -> str:
        return path.read_text(encoding="utf-8") if path.is_file() else ""

    return {
        "runner": strip_tests(strip_comments(read(RUNNER))),
        "domain": strip_tests(strip_comments(read(DOMAIN_RS))),
        "main": strip_comments(read(MAIN_RS)),
        # bridge.rs contains large embedded JS/CSS strings where naive block-comment stripping
        # can eat real Rust that follows. For c-card command checks, raw source is safer.
        "bridge": read(BRIDGE_RS),
        "acl": read(ACL_TOML),
        "capability": read(CAPABILITY_JSON),
        "cargo_toml": read(CARGO_TOML),
        "cargo_lock": read(CARGO_LOCK),
        "package_json": read(PACKAGE_JSON),
        "package_lock": read(PACKAGE_LOCK),
    }


# ----------------------------- 自检（含变异防呆） -----------------------------

# (期望命中的码, 原文片段, 替换为, 替换次数)。
#
# **两条硬约束**（都是本次实测踩出来的坑）：
# 1. 原文片段必须存在，否则按**漏检测**计（M2-3.b 教训）。
# 2. 替换后的文本**不得残留被检测的关键字**——否则变异自废、坏样本永远检不出
#    （如把 `libc::setsid();` 替换成 `// setsid removed`，"setsid" 仍在，检测照样通过）。
# 3. count=-1 表示全替换（用于 `#[cfg(not(unix))]` 这类多处出现的锚点）。
MUTATIONS: tuple[tuple[str, str, str, int], ...] = (
    ("EXEC_SH_C_PRESENT", "Command::new(program)", 'Command::new(program).args(["sh", "-c"])', 1),
    ("EXEC_STRING_INTERPOLATION", "Command::new(program)", 'Command::new(format!("{}", program))', 1),
    ("EXEC_NO_SETSID", "            libc::setsid();", "            // removed", 1),
    ("EXEC_NO_KILLPG", "        libc::killpg(pgid, sig);", "        let _ = sig;", 1),
    ("EXEC_NO_WAIT_REAP", "        match child.try_wait() {", "        match child.id() {", 1),
    ("EXEC_ARG_QUOTED", "argv.push(value.to_string());", 'argv.push(format!("\'{}\'", value));', 1),
    ("EXEC_PLATFORM_STUB_MISSING", "#[cfg(not(unix))]", "#[cfg(unix)]", -1),
    ("EXEC_ENV_NOT_CLEARED", "        .env_clear()", "        // cleared", 1),
    ("EXEC_CWD_NOT_LOCKED", "        .current_dir(&cwd)", "        // cwd", 1),
    ("EXEC_INTERPRETER_NOT_WHITELISTED", "    if let Some(bin) = interpreter.binary() {", '    if let Some(bin) = Some("bash") {', 1),
)

BRIDGE_MUTATIONS: tuple[tuple[str, str, str, int], ...] = (
    ("EXEC_ACL_MISSING", '"run_script",', '', 1),
    ("EXEC_HANDLER_NOT_REGISTERED", "            bridge::run_script,", "", 1),
    ("EXEC_NO_SOURCE_CHECK", '    check_invocation_source(&webview, "run_script", None, &app)?;', '', 1),
    ("EXEC_VALIDATION_BYPASSED", "    crate::scripts::validate_meta(&script).map_err(|e| e.to_string())?;", "", 1),
    ("EXEC_AUDIT_LEAKS_VALUE", '"script.run.start",', '"script.run.start values",', 1),
    ("EXEC_SHUTDOWN_NOT_REGISTERED", 'coordinator.register("kill-running-scripts"', 'coordinator.register("kill-scripts-missing"', 1),
)

D_MUTATIONS: tuple[tuple[str, str, str, int], ...] = (
    ("EXEC_RUN_RECORD_MISSING", "pub struct ScriptRunRecord", "pub struct ScriptRunGone", 1),
    ("EXEC_NO_RING_CAP", "pub const RING_MAX_BYTES", "pub const RING_BYTES_DISABLED", 1),
    ("EXEC_NO_OUTPUT_CAP", "pub const SCRIPT_RUN_TAIL_BYTES", "pub const SCRIPT_RUN_TAIL_DISABLED", 1),
)

E_MUTATIONS: tuple[tuple[str, str, str, int, str], ...] = (
    ("EXEC_SHELL_SPAWN_CAPABILITY_PRESENT", '"default-commands"', '"default-commands", "shell:allow-spawn"', 1, "capability"),
    ("EXEC_FRONTEND_SHELL_DEP_PRESENT", '"@tauri-apps/api": "^2.0.0"', '"@tauri-apps/api": "^2.0.0",\n    "@tauri-apps/plugin-shell": "^2.3.5"', 1, "package_json"),
    ("EXEC_RUST_SHELL_PLUGIN_PRESENT", 'tauri = { version = "2", features = ["unstable", "protocol-asset"] }', 'tauri = { version = "2", features = ["unstable", "protocol-asset"] }\ntauri-plugin-shell = "2"', 1, "cargo_toml"),
)

# M5-1 切片 0b 后 `HARD_GRACE_SECS` 收口到 `domain.rs`：该变异作用于 domain 文件，
# 仍验证「定义消失即检出」的变异防呆（runner 为 re-export 时亦能捕获 domain 定义被删）。
DOMAIN_MUTATIONS: tuple[tuple[str, str, str, int, str], ...] = (
    ("EXEC_TIMEOUT_NOT_LAYERED", "pub const HARD_GRACE_SECS: u32 = 5;", "pub const HARD_GRACE_UNUSED: u32 = 0;", 1, "domain"),
)


def self_test() -> int:
    ctx = read_ctx()
    failures: list[str] = []

    # 1) 好样本：真实源码在默认模式下必须零命中
    good_hits = {c for c in detect(ctx) if c in ACTIVE_CODES}
    if good_hits:
        failures.append(f"好样本本应零命中，实际命中：{sorted(good_hits)}")

    # 2) 坏样本：每个变异必须命中对应码
    for code, old, new, count in MUTATIONS:
        # 防呆：变异若未真正改动内容（原文片段不存在），按漏检测计
        if old not in ctx["runner"]:
            failures.append(
                f"[{code}] 变异原文片段不存在，夹具已失效（必须同步更新 MUTATIONS）：{old!r}"
            )
            continue
        mutated = dict(ctx)
        mutated["runner"] = ctx["runner"].replace(old, new, count)
        if mutated["runner"] == ctx["runner"]:
            failures.append(f"[{code}] 变异未改变内容（str.replace 空转），按漏检测计")
            continue
        hits = detect(mutated)
        if code not in hits:
            failures.append(f"[{code}] 坏样本未被检出（实际命中：{sorted(hits & set(ACTIVE_CODES))}）")

    for code, old, new, count in BRIDGE_MUTATIONS:
        target_key = "acl" if code == "EXEC_ACL_MISSING" else "main" if code == "EXEC_HANDLER_NOT_REGISTERED" else "bridge"
        if old not in ctx[target_key]:
            failures.append(f"[{code}] 变异原文片段不存在，夹具已失效：{old!r}")
            continue
        mutated = dict(ctx)
        mutated[target_key] = ctx[target_key].replace(old, new, count)
        if mutated[target_key] == ctx[target_key]:
            failures.append(f"[{code}] 变异未改变内容，按漏检测计")
            continue
        hits = detect(mutated)
        if code not in hits:
            failures.append(f"[{code}] 坏样本未被检出（实际命中：{sorted(hits & set(ACTIVE_CODES))}）")

    for code, old, new, count, key in E_MUTATIONS:
        if old not in ctx[key]:
            failures.append(f"[{code}] 变异原文片段不存在，夹具已失效：{old!r}")
            continue
        mutated = dict(ctx)
        mutated[key] = ctx[key].replace(old, new, count)
        if mutated[key] == ctx[key]:
            failures.append(f"[{code}] 变异未改变内容，按漏检测计")
            continue
        hits = detect(mutated)
        if code not in hits:
            failures.append(f"[{code}] 坏样本未被检出（实际命中：{sorted(hits & set(ACTIVE_CODES))}）")

    for code, old, new, count, key in DOMAIN_MUTATIONS:
        if old not in ctx[key]:
            failures.append(f"[{code}] 变异原文片段不存在，夹具已失效：{old!r}")
            continue
        mutated = dict(ctx)
        mutated[key] = ctx[key].replace(old, new, count)
        if mutated[key] == ctx[key]:
            failures.append(f"[{code}] 变异未改变内容，按漏检测计")
            continue
        hits = detect(mutated)
        if code not in hits:
            failures.append(f"[{code}] 坏样本未被检出（实际命中：{sorted(hits & set(ACTIVE_CODES))}）")

    for code, old, new, count in D_MUTATIONS:
        if old not in ctx["runner"]:
            failures.append(f"[{code}] 变异原文片段不存在，夹具已失效：{old!r}")
            continue
        mutated = dict(ctx)
        mutated["runner"] = ctx["runner"].replace(old, new, count)
        if mutated["runner"] == ctx["runner"]:
            failures.append(f"[{code}] 变异未改变内容，按漏检测计")
            continue
        hits = detect(mutated)
        if code not in hits:
            failures.append(f"[{code}] 坏样本未被检出（实际命中：{sorted(hits & set(ACTIVE_CODES))}）")

    # 3) pending 码位完整性：默认模式不得误判 pending 为 active
    overlap = set(ACTIVE_CODES) & set(PENDING_CODES)
    if overlap:
        failures.append(f"码位重复定义：{sorted(overlap)}")

    if failures:
        print("SELF_TEST_RESULT=FAIL")
        for f in failures:
            print(f"FAIL: {f}")
        return 1
    print(f"SELF_TEST_RESULT=ALL_PASS（{len(MUTATIONS) + len(BRIDGE_MUTATIONS) + len(D_MUTATIONS) + len(E_MUTATIONS) + len(DOMAIN_MUTATIONS)} 个坏样本 + 1 个好样本 + 码位完整性）")
    return 0


# ----------------------------- 主流程 -----------------------------

def main() -> int:
    parser = argparse.ArgumentParser(description="M2-4 执行通道安全不变量夹具")
    parser.add_argument("--self-test", action="store_true", help="好坏样本双向自检")
    parser.add_argument(
        "--expect-pending",
        action="store_true",
        help="验证 pending 码（c/d 卡职责）确实仍未实现",
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    ctx = read_ctx()
    hits = detect(ctx)

    if args.expect_pending:
        pending_hits = [c for c in PENDING_CODES if c in hits]
        if pending_hits:
            print("PENDING_RESULT=FAIL")
            for code in pending_hits:
                print(f"  {code}")
            return 1
        print(f"PENDING_RESULT=NONE（{len(PENDING_CODES)} 个 pending 码位）")
        return 0

    active_hits = sorted(c for c in hits if c in ACTIVE_CODES)
    if active_hits:
        print("EXEC_POLICY_RESULT=FAIL")
        for code in active_hits:
            print(f"  {code}")
        return 1
    print(f"EXEC_POLICY_RESULT=PASS（{len(ACTIVE_CODES)} 个默认码位零命中）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
