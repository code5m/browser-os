#!/usr/bin/env python3
"""Expose the known M0-3 security boundary gaps as a reproducible source fixture.

M0-3.a 只盘点与定契约（威胁矩阵 logs/m0-security-threat-matrix-v1.md），
收口分属 M0-3.b/c/d，因此默认模式当前预期 EXIT=1。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tempfile
from pathlib import Path
from native_bridge_owner_sources import read_native_bridge_owners

# M0-3.b 已收口：capability 删除了残留的 `browser` label，远程权限集不再向外部页面
# 开放写盘/开终端类副作用命令，且上报类命令带来源校验与载荷边界。
# 剩余缺口归 M0-3.c（路径策略）与 M0-3.d（launch_app 应用条目白名单）。
# M0-3.d 已收口 launch_app：直接启动经过校验的可执行目标。
# V2 已收口只读路径与动态 JS IPC 来源：下方链路/负例必须持续成立。
# 浏览器开放 http/https 导航的残余事实保留，由 remote report-only 权限补偿。
EXPECTED_GAPS = (
    "REMOTE_WILDCARD_IPC",
)

# Legacy raw filesystem sinks remain forbidden in an unprotected command or delegate.
FILE_COMMAND_MARKERS = (
    "std::fs::write(",
    "std::fs::remove_dir_all(",
    "std::fs::remove_file(",
    "std::fs::create_dir_all(",
)


def function_body(source: str, name: str) -> str:
    """Return a Rust function body using a small brace-balanced extractor."""
    match = re.search(rf"\bfn\s+{re.escape(name)}\s*\(", source)
    if not match:
        return ""
    start = source.find("{", match.end())
    if start < 0:
        return ""
    depth = 0
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[start + 1 : index]
    return ""


WRITE_COMMANDS = ("write_file", "create_file", "create_dir", "delete_path", "rename_path")


def command_path_policy_body(source: str, name: str) -> str:
    """Follow only the concrete command's invoked implementation chain.

    Physical async wrappers contain admission/source checks, while authorization
    remains inside the blocking owner. An uncalled helper or another command's
    check cannot prove this command safe. Missing links fail closed.
    """
    chain = [name, f"{name}_blocking"]
    if name == "write_file":
        chain.append("write_file_authorized")
    body = function_body(source, chain[0])
    for index, current in enumerate(chain):
        if not body:
            return ""
        if re.search(r"\b(?:check_path_within_roots|check_delete_target)\s*\(", body):
            return body
        if index + 1 == len(chain):
            return ""
        delegate = chain[index + 1]
        if not re.search(rf"\b{re.escape(delegate)}\s*\(", body):
            return ""
        if re.search(r"\b(?:std::fs|fs)::(?:read|read_dir|read_to_string|write|remove_file|remove_dir_all|create_dir_all|rename)\s*\(", body):
            return ""
        body = function_body(source, delegate)
    return ""


def write_commands_use_path_policy(bridge_source: str) -> bool:
    checked = 0
    for name in WRITE_COMMANDS:
        if not function_body(bridge_source, name):
            continue
        checked += 1
        if not command_path_policy_body(bridge_source, name):
            return False
    return checked > 0


def eval_command_is_main_only(source: str) -> bool:
    """The dynamic JS IPC entry must reject remote callers before dispatch."""
    body = function_body(source, "eval_in_tab")
    if not body:
        return False
    source_check = re.search(r"\brequire_main\s*\(\s*&webview\s*,[^;]+\)\s*\?", body)
    id_bound = re.search(r"\bbound_text\s*\([^;]*&id\s*\)\s*\?", body)
    js_bound = re.search(r"\bbound_text\s*\([^;]*&js\s*\)\s*\?", body)
    effect = re.search(r"\beval_in_tab_blocking\s*\(|\.eval\s*\(", body)
    if not all((source_check, id_bound, js_bound, effect)):
        return False
    if max(source_check.end(), id_bound.end(), js_bound.end()) > effect.start():
        return False
    if "eval_in_tab_blocking" in effect.group(0):
        return bool(function_body(source, "eval_in_tab_blocking"))
    return True


def detect_gaps(
    bridge_source: str, capability_sources: str, policy_source: str = ""
) -> list[str]:
    gaps: list[str] = []

    # SEC-01/SEC-07（M0-3.d 已收口）：launch_app 不得再用 sh -c，且必须过启动目标校验。
    if re.search(r'Command::new\(\s*"sh"\s*\)\s*[\s\S]{0,120}?\.arg\(\s*"-c"\s*\)', bridge_source):
        gaps.append("LAUNCH_APP_ARBITRARY_SHELL")
    elif "check_launch_target" not in function_body(bridge_source, "launch_app"):
        gaps.append("LAUNCH_APP_WITHOUT_TARGET_POLICY")
    else:
        # B2-1（BUG-HUNT）：启动目标校验的**拦截完整性**——门禁不能只看"是否调用了函数"。
        # 必须同时具备：执行包装器黑名单（env/sudo…）、解释器内联代码开关、符号链接解析后复查。
        # 三者缺一即视为可被 env / 解释器 `-c` / 软链接绕过。
        missing = [
            marker
            for marker in (
                "BLOCKED_LAUNCH_WRAPPERS",
                "BLOCKED_INTERPRETERS",
                "INTERPRETER_EXEC_FLAGS",
            )
            if marker not in policy_source
        ]
        if missing or "resolve_program_file" not in function_body(
            policy_source, "check_launch_target"
        ):
            gaps.append("LAUNCH_TARGET_BYPASS_NOT_HARDENED")

    # SEC-02（M0-3.c 已收口）：写/删类命令必须经 check_path_within_roots。
    # 判定方式：找到写/删命令的函数体，确认其中调用了路径策略。
    if not write_commands_use_path_policy(bridge_source):
        gaps.append("FILE_COMMANDS_WITHOUT_PATH_POLICY")

    # SEC-09: the actual path-taking read commands now use the same authorized
    # native owner chain. browse_workspace/get_start_dirs take no caller path.
    read_commands = ("list_dir", "read_file")
    if any(function_body(bridge_source, name) for name in read_commands) and not all(
        command_path_policy_body(bridge_source, name) for name in read_commands
    ):
        gaps.append("READ_ONLY_BROWSE_WITHOUT_PATH_POLICY")

    # SEC-03（M0-3.b 已收口）：capability 里不得再残留已不存在的 browser label。
    if re.search(r'"(windows|webviews)"\s*:\s*\[[^\]]*"browser"', capability_sources):
        gaps.append("CAPABILITY_STALE_BROWSER_LABEL")

    # SEC-08：远程权限集（remote-collect）不得向外部页面开放有副作用的命令。
    remote_block = re.search(
        r'identifier\s*=\s*"remote-collect"[\s\S]{0,400}?commands\.allow\s*=\s*\[([^\]]*)\]',
        capability_sources,
    )
    if remote_block:
        remote_allow = remote_block.group(1)
        # The remote capability is a return-channel contract, not a general
        # Native allowlist. Dynamic JS/files/process effects cannot be added.
        allowed_reports = {"report_resources", "report_title", "report_grid_load_failed"}
        leaked = [c for c in re.findall(r'"([^"]+)"', remote_allow) if c not in allowed_reports]
        if leaked:
            gaps.append("REMOTE_SIDE_EFFECT_COMMANDS_EXPOSED")

    # SEC-04：remote urls 全通配。
    if re.search(r'"remote"\s*:\s*\{[\s\S]{0,200}?"https?://\*"', capability_sources):
        gaps.append("REMOTE_WILDCARD_IPC")

    # SEC-05: follow the real dynamic-JS IPC entry, not unrelated occurrences
    # of a source-check name elsewhere in the facade. Native internal eval is
    # allowed only behind this main-only, bounded entry and the remote list.
    has_eval = re.search(r"\.eval\(\s*&", bridge_source) or function_body(bridge_source, "eval_in_tab")
    if has_eval and not eval_command_is_main_only(bridge_source):
        gaps.append("EVAL_WITHOUT_SOURCE_CHECK")

    return gaps


def load_capability_sources(root: Path) -> str:
    """capabilities/*.json + permissions/*.toml。

    远程权限集（remote-collect）的实际命令清单定义在 permissions/*.toml，
    只扫 capabilities 会漏掉真正的放行面，因此两类配置一起纳入。
    """
    parts = []
    for directory, pattern in (
        (root / "src-tauri" / "capabilities", "*.json"),
        (root / "src-tauri" / "permissions", "*.toml"),
    ):
        if not directory.is_dir():
            continue
        for path in sorted(directory.glob(pattern)):
            parts.append(path.read_text(encoding="utf-8"))
    return "\n".join(parts)


def load_policy_source(root: Path) -> str:
    """security_policy.rs：启动目标校验的拦截完整性在此定义（B2-1）。

    文件缺失时返回空串——此时 launch_app 若已接入 check_launch_target，
    detect_gaps 会据此判定完整性缺失（失败关闭）。
    """
    path = root / "src-tauri" / "src" / "security_policy.rs"
    if not path.is_file():
        return ""
    return path.read_text(encoding="utf-8")


def scan_repository(root: Path) -> list[str]:
    # Real repo owns these commands in physical bridge owner files; the self-test
    # fixture intentionally builds a minimal legacy bridge.rs without an owner
    # directory. Both paths are scanned using the SAME detect_gaps rules.
    owner_dir = root / "src-tauri/src/bridge"
    if owner_dir.is_dir():
        bridge_source = read_native_bridge_owners(root, "filesystem_commands", "resource_commands", "tab_runtime", "automation_commands", "workspace_commands", "integration_commands")
    else:
        bridge_source = (root / "src-tauri/src/bridge.rs").read_text(encoding="utf-8")
    # launch_app 已迁入 framework；继续校验真实实现，不把迁移误判成保护缺失。
    framework = root / "src-tauri/src/framework/commands.rs"
    if framework.is_file():
        bridge_source += "\n" + framework.read_text(encoding="utf-8")
    return detect_gaps(
        bridge_source,
        load_capability_sources(root),
        load_policy_source(root),
    )


def run_self_test() -> int:
    legacy_bridge = """
pub fn launch_app(exec: String) -> Result<(), String> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(exec)
        .spawn()?;
    Ok(())
}
pub fn launch_app_plain(exec: String) -> Result<(), String> {
    std::process::Command::new(&exec).spawn()?;
    Ok(())
}
pub fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}
pub fn delete_path(path: String) -> Result<(), String> {
    std::fs::remove_dir_all(&path).map_err(|e| e.to_string())
}
pub fn eval_in_tab(id: String, js: String) {
    let _ = win.eval(&format!("{}", js));
}
pub fn list_dir(path: String) -> Result<Vec<String>, String> { Ok(vec![]) }
pub fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}
"""
    legacy_capabilities = """
{
  "windows": ["main", "browser"],
  "webviews": ["browser", "tab-*"],
  "remote": {"urls": ["https://*", "http://*"]}
}
"""
    # M0-3.b 之前的真实配置：远程集放行写盘与开终端命令。
    legacy_permissions = """
[[permission]]
identifier = "remote-collect"
commands.allow = ["collect_selection", "save_note", "request_open_terminal", "report_resources"]
"""
    # M0-3.a 时的真实状态：写命令无路径策略、capability 残留 browser、远程集放行副作用命令。
    legacy_policy = """
pub const BLOCKED_LAUNCH_PROGRAMS: [&str; 6] = ["sh", "bash", "zsh", "fish", "powershell", "cmd"];
pub fn check_launch_target(line: &str) -> Result<(String, Vec<String>), PolicyError> {
    check_shell_command(line)?;
    let (program, args) = parse_command_line(line)?;
    if BLOCKED_LAUNCH_PROGRAMS.contains(&program.as_str()) {
        return Err(PolicyError::BlockedLaunchProgram(program));
    }
    Ok((program, args))
}
"""
    legacy_gaps = [
        "REMOTE_WILDCARD_IPC",
        "EVAL_WITHOUT_SOURCE_CHECK",
        "READ_ONLY_BROWSE_WITHOUT_PATH_POLICY",
        "LAUNCH_APP_ARBITRARY_SHELL",
        "FILE_COMMANDS_WITHOUT_PATH_POLICY",
        "CAPABILITY_STALE_BROWSER_LABEL",
        "REMOTE_SIDE_EFFECT_COMMANDS_EXPOSED",
    ]
    detected = detect_gaps(
        legacy_bridge,
        legacy_capabilities + "\n" + legacy_permissions,
        legacy_policy,
    )
    if sorted(detected) != sorted(legacy_gaps):
        print(f"self-test: legacy mismatch: {detected}", file=sys.stderr)
        return 1

    resolved_bridge = """
pub fn launch_app(entry: AppEntry) -> Result<(), String> {
    let (program, args) = security_policy::check_launch_target(&entry.exec)?;
    std::process::Command::new(program).args(args).spawn()?;
    Ok(())
}
pub fn write_file(path: String, content: String) -> Result<(), String> {
    let canonical = std::fs::canonicalize(&path)?;
    security_policy::check_path_within_roots(&canonical, allowed_roots())?;
    std::fs::write(&canonical, &content).map_err(|e| e.to_string())
}
pub fn eval_in_tab(webview: Webview, id: String, js: String) {
    native_legacy_policy::require_main(&webview, "eval_in_tab")?;
    native_legacy_policy::bound_text("eval_in_tab", "id", &id)?;
    native_legacy_policy::bound_text("eval_in_tab", "js", &js)?;
    let _ = win.eval(&format!("{}", js));
}
pub fn list_dir(path: String) -> Result<Vec<String>, String> {
    security_policy::check_path_within_roots(&path, roots())?;
    Ok(vec![])
}
pub fn read_file(path: String) -> Result<String, String> {
    let canonical = std::fs::canonicalize(&path)?;
    security_policy::check_path_within_roots(&canonical, roots())?;
    std::fs::read_to_string(&canonical).map_err(|e| e.to_string())
}
"""
    resolved_capabilities = """
{
  "windows": ["main"],
  "webviews": ["tab-*", "grid-*"],
  "remote": {"urls": ["https://trusted.example"]}
}
"""
    resolved_permissions = """
[[permission]]
identifier = "remote-collect"
commands.allow = ["report_resources", "report_title", "report_grid_load_failed"]
"""
    resolved_policy = """
pub const BLOCKED_LAUNCH_PROGRAMS: [&str; 12] = ["sh", "bash", "dash", "ash", "zsh", "ksh", "csh", "tcsh", "fish", "powershell", "pwsh", "cmd"];
pub const BLOCKED_LAUNCH_WRAPPERS: [&str; 10] = ["env", "busybox", "nohup", "sudo", "su", "doas", "pkexec", "xargs", "timeout", "setsid"];
pub const BLOCKED_INTERPRETERS: [&str; 12] = ["python", "perl", "ruby", "node", "nodejs", "deno", "bun", "lua", "luajit", "php", "tclsh", "osascript"];
pub const INTERPRETER_EXEC_FLAGS: [&str; 6] = ["-c", "-e", "--eval", "-E", "-r", "-p"];
pub fn check_launch_target(line: &str) -> Result<(String, Vec<String>), PolicyError> {
    check_shell_command(line)?;
    let (program, args) = parse_command_line(line)?;
    let resolved = resolve_program_file(&program).ok_or(PolicyError::LaunchProgramNotFound(program.clone()))?;
    Ok((program, args))
}
"""
    if detect_gaps(
        resolved_bridge,
        resolved_capabilities + "\n" + resolved_permissions,
        resolved_policy,
    ):
        print("self-test: resolved fixture still reports gaps", file=sys.stderr)
        return 1

    # Async physical owners: prove every edge rather than accepting an
    # unrelated protected helper. Each mutation must fail independently.
    async_writes = """
pub async fn write_file(path: String) {
    FILESYSTEM_IO.run(move || write_file_blocking(path)).await
}
fn write_file_blocking(path: String) { write_file_authorized(&path) }
fn write_file_authorized(path: &str) {
    check_path_within_roots(path, roots())?;
    native::save_if_unchanged(path, roots(), before, content)
}
"""
    if not write_commands_use_path_policy(async_writes):
        print("self-test: protected async write chain rejected", file=sys.stderr)
        return 1
    for changed in (
        async_writes.replace("move || write_file_blocking(path)", "move || unsafe_write(path)"),
        async_writes.replace("write_file_authorized(&path)", "unsafe_write(&path)"),
        async_writes.replace("check_path_within_roots(path, roots())?;", ""),
        async_writes.replace("FILESYSTEM_IO.run", "std::fs::write(path, data); FILESYSTEM_IO.run"),
    ):
        if write_commands_use_path_policy(changed):
            print("self-test: broken async write chain accepted", file=sys.stderr)
            return 1
    async_reads = """
pub async fn list_dir(path: String) { FILESYSTEM_IO.run(move || list_dir_blocking(path)).await }
fn list_dir_blocking(path: String) {
    check_path_within_roots(&path, roots())?;
    native::list_directory(path, roots())
}
pub async fn read_file(path: String) { FILESYSTEM_IO.run(move || read_file_blocking(path)).await }
fn read_file_blocking(path: String) {
    check_path_within_roots(&path, roots())?;
    native::read_regular(path, roots(), MAX_BYTES)
}
"""
    if not all(command_path_policy_body(async_reads, name) for name in ("list_dir", "read_file")):
        print("self-test: protected async read chain rejected", file=sys.stderr)
        return 1
    for name in ("list_dir", "read_file"):
        changed = async_reads.replace(f"move || {name}_blocking(path)", "move || unprotected_read(path)")
        if command_path_policy_body(changed, name):
            print(f"self-test: unprotected {name} wrapper accepted", file=sys.stderr)
            return 1
    async_eval = """
pub async fn eval_in_tab(webview: Webview, id: String, js: String) {
    require_main(&webview, "eval_in_tab")?;
    bound_text("eval_in_tab", "id", &id)?;
    bound_text("eval_in_tab", "js", &js)?;
    TAB_EVAL_WORKERS.run(move || eval_in_tab_blocking(app, id, js)).await
}
fn eval_in_tab_blocking(app: AppHandle, id: String, js: String) { eval_in_child(&id, &js) }
"""
    if not eval_command_is_main_only(async_eval):
        print("self-test: main-only bounded async eval rejected", file=sys.stderr)
        return 1
    for changed in (
        async_eval.replace('require_main(&webview, "eval_in_tab")?;', ""),
        async_eval.replace('bound_text("eval_in_tab", "js", &js)?;', ""),
        async_eval.replace('bound_text("eval_in_tab", "id", &id)?;', ""),
        async_eval.replace("fn eval_in_tab_blocking(", "fn unrelated_eval_helper("),
        async_eval.replace('require_main(&webview, "eval_in_tab")?;', "").replace(
            ".await", '.await; require_main(&webview, "eval_in_tab")?;'
        ),
    ):
        if eval_command_is_main_only(changed):
            print("self-test: unprotected async eval accepted", file=sys.stderr)
            return 1
    leaked_permissions = resolved_permissions.replace('"report_title"', '"report_title", "eval_in_tab"')
    if "REMOTE_SIDE_EFFECT_COMMANDS_EXPOSED" not in detect_gaps(
        resolved_bridge, resolved_capabilities + leaked_permissions, resolved_policy
    ):
        print("self-test: remote dynamic-JS exposure accepted", file=sys.stderr)
        return 1

    # 端到端：临时仓库扫描必须与 fixture 判定一致。
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / "src-tauri" / "src").mkdir(parents=True)
        (root / "src-tauri" / "capabilities").mkdir(parents=True)
        (root / "src-tauri" / "src" / "bridge.rs").write_text(legacy_bridge, encoding="utf-8")
        (root / "src-tauri" / "src" / "security_policy.rs").write_text(
            legacy_policy, encoding="utf-8"
        )
        (root / "src-tauri" / "capabilities" / "default.json").write_text(
            legacy_capabilities, encoding="utf-8"
        )
        (root / "src-tauri" / "permissions").mkdir(parents=True)
        (root / "src-tauri" / "permissions" / "remote-collect.toml").write_text(
            legacy_permissions, encoding="utf-8"
        )
        if sorted(scan_repository(root)) != sorted(legacy_gaps):
            print("self-test: repository scan mismatch", file=sys.stderr)
            return 1

        # 模块迁移后也须扫描实际 handler；删除其保护调用仍必须被检出。
        launch_definition = resolved_bridge.split("pub fn write_file", 1)[0]
        (root / "src-tauri/src/bridge.rs").write_text(resolved_bridge.replace(launch_definition, ""), encoding="utf-8")
        (root / "src-tauri/src/framework").mkdir()
        framework = root / "src-tauri/src/framework/commands.rs"
        framework.write_text(launch_definition, encoding="utf-8")
        (root / "src-tauri/src/security_policy.rs").write_text(resolved_policy, encoding="utf-8")
        (root / "src-tauri/capabilities/default.json").write_text(resolved_capabilities, encoding="utf-8")
        (root / "src-tauri/permissions/remote-collect.toml").write_text(resolved_permissions, encoding="utf-8")
        if scan_repository(root):
            print("self-test: migrated framework handler should be protected", file=sys.stderr)
            return 1
        framework.write_text(launch_definition.replace("check_launch_target", "missing_check"), encoding="utf-8")
        if "LAUNCH_APP_WITHOUT_TARGET_POLICY" not in scan_repository(root):
            print("self-test: migrated handler missing protection must fail", file=sys.stderr)
            return 1

    # capability JSON 必须可被解析，避免夹具只做字符串匹配而放过坏 JSON。
    try:
        json.loads(legacy_capabilities)
    except json.JSONDecodeError as error:
        print(f"self-test: capability fixture is not valid JSON: {error}", file=sys.stderr)
        return 1

    print("SELF_TEST_RESULT=ALL_PASS")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Report whether the M0-3 security boundary contract is enforced."
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--self-test", action="store_true", help="run built-in fixtures")
    mode.add_argument(
        "--expect-current-gaps",
        action="store_true",
        help="pass only while the documented M0-3.a gap set remains reproducible",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="repository root (defaults to this script's repository)",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        return run_self_test()

    try:
        gaps = scan_repository(args.root.resolve())
    except (OSError, UnicodeError) as error:
        print(f"security-policy: scan error: {error}", file=sys.stderr)
        return 1

    if args.expect_current_gaps:
        missing = sorted(set(EXPECTED_GAPS) - set(gaps))
        unexpected = sorted(set(gaps) - set(EXPECTED_GAPS))
        if missing or unexpected:
            print("security-current-gap-fixture: FAIL")
            for gap in missing:
                print(f"MISSING_EXPECTED_GAP={gap}")
            for gap in unexpected:
                print(f"UNEXPECTED_GAP={gap}")
            return 1
        for gap in gaps:
            print(f"GAP={gap}")
        print("CURRENT_GAP_FIXTURE_RESULT=PASS")
        return 0

    if gaps:
        print(f"security-policy: FAIL ({len(gaps)} known gap(s))")
        for gap in gaps:
            print(f"GAP={gap}")
        print("SECURITY_POLICY_RESULT=FAIL")
        return 1

    print("SECURITY_POLICY_RESULT=PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
