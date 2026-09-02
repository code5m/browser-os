#!/usr/bin/env python3
"""Expose the M1-6 Git write safety invariants as a reproducible source fixture.

M1-6.b 落地了 Git 写后端核心（双阶段闸门 + 六个白名单写操作），M1-6.c 复核后
把「靠人眼评审」的安全不变量固化成可复跑的静态夹具，防止后续改动悄悄绕过：

  - 写原语必须各自带校验器调用（先校验后执行）
  - 写段不得出现黑名单 API / 联网 / 凭据 / shell 调 git
  - 白名单恰好六项且 `from_op_str` 与枚举一致
  - 闸门两阶段命令必须做来源校验、不得有绕过闸门的直写路径
  - dangerous 操作必须走二次确认、任务必须带过期判定
  - 审计构造器签名/实现不得接收路径清单、diff 或凭据
  - 新命令必须同时注册到 main.rs 与 ACL

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# 写原语 → 必须出现在其函数体内的校验器调用
WRITE_PRIMITIVES: dict[str, str] = {
    "write_stage": "validate_repo_paths",
    "write_unstage": "validate_repo_paths",
    "write_discard": "validate_repo_paths",
    "write_commit": "validate_commit_message",
    "write_create_branch": "validate_branch_name",
    "write_checkout_branch": "validate_branch_name",
}

# Git 写白名单：枚举变体 → 序列化名
EXPECTED_OPS: dict[str, str] = {
    "Stage": "stage",
    "Unstage": "unstage",
    "Discard": "discard",
    "Commit": "commit",
    "CreateBranch": "create_branch",
    "CheckoutBranch": "checkout_branch",
}

# 写段禁止出现的 API / 语义（(正则, 违规码)）
FORBIDDEN_IN_WRITE_SECTION: tuple[tuple[str, str], ...] = (
    (r"\.push\(", "GIT_WRITE_PUSH_API"),
    (r"find_remote\(", "GIT_WRITE_REMOTE_API"),
    (r"\.fetch\(", "GIT_WRITE_FETCH_API"),
    (r"\.merge\(", "GIT_WRITE_MERGE_API"),
    (r"reset\(", "GIT_WRITE_RESET_API"),
    (r"cleanup_state\(", "GIT_WRITE_MERGE_API"),
    (r"\bstash\b|rebase|cherry[-_]pick", "GIT_WRITE_BLACKLISTED_OP"),
    (r"cred_cb|userpass_plaintext|KeyringStore", "GIT_WRITE_CREDENTIAL_ACCESS"),
    (r"\btoken\b", "GIT_WRITE_CREDENTIAL_ACCESS"),
    (r"Command::new\(\s*\"git\"", "GIT_WRITE_SHELL_GIT"),
    (r"Command::new\(\s*\"sh\"\s*\)\s*[\s\S]{0,120}?\.arg\(\s*\"-c\"", "GIT_WRITE_SHELL_GIT"),
)

WRITE_SECTION_MARKER = "// M1-6.b Git 写原语"
GATE_COMMANDS = ("request_git_write", "confirm_git_write")
AUDIT_FORBIDDEN_PARAMS = ("paths", "diff", "content", "credential", "token", "secret")


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


def strip_comments(source: str) -> str:
    """去掉行注释：逐字符跟踪字符串字面量，避免误伤 `"refs/heads/x"` 之类内容。"""
    cleaned: list[str] = []
    for line in source.splitlines():
        in_string = False
        index = 0
        cut = None
        while index < len(line):
            char = line[index]
            if in_string:
                if char == "\\":
                    index += 2
                    continue
                if char == '"':
                    in_string = False
            elif char == '"':
                in_string = True
            elif char == "/" and index + 1 < len(line) and line[index + 1] == "/":
                cut = index
                break
            index += 1
        cleaned.append(line[:cut] if cut is not None else line)
    return "\n".join(cleaned)


def write_section(sync_source: str) -> str:
    """取 M1-6.b 写原语段（到下一个 `#[cfg(test)]` 为止）。"""
    start = sync_source.find(WRITE_SECTION_MARKER)
    if start < 0:
        return ""
    end = sync_source.find("#[cfg(test)]", start)
    return sync_source[start : end if end > 0 else len(sync_source)]


def check_write_primitives(section: str) -> list[str]:
    violations: list[str] = []
    code = strip_comments(section)
    for name, validator in WRITE_PRIMITIVES.items():
        if f"fn {name}(" not in code:
            violations.append("GIT_WRITE_PRIMITIVE_MISSING")
            continue
        if validator not in function_body(code, name):
            violations.append("GIT_WRITE_PRIMITIVE_WITHOUT_VALIDATION")
    return violations


def check_forbidden_api(section: str) -> list[str]:
    code = strip_comments(section)
    return [
        code_name
        for pattern, code_name in FORBIDDEN_IN_WRITE_SECTION
        if re.search(pattern, code)
    ]


def check_op_whitelist(domain_source: str) -> list[str]:
    violations: list[str] = []
    enum_match = re.search(
        r"pub enum GitWriteOp \{(.*?)\n\}", domain_source, re.S
    )
    if not enum_match:
        return ["GIT_WRITE_OP_ENUM_MISSING"]
    variants = re.findall(r"^\s{4}(\w+),", enum_match.group(1), re.M)
    if set(variants) != set(EXPECTED_OPS):
        return ["GIT_WRITE_OP_WHITELIST_DRIFT"]

    parser = function_body(domain_source, "from_op_str")
    if not parser:
        return ["GIT_WRITE_OP_PARSER_MISSING"]
    mapping = dict(
        re.findall(r"\"([a-z_]+)\"\s*=>\s*Some\(GitWriteOp::(\w+)\)", parser)
    )
    expected_mapping = {name: variant for variant, name in EXPECTED_OPS.items()}
    if mapping != expected_mapping:
        violations.append("GIT_WRITE_OP_PARSER_DRIFT")
    return violations


def check_gate(bridge_source: str) -> list[str]:
    violations: list[str] = []
    request = function_body(bridge_source, "request_git_write")
    confirm = function_body(bridge_source, "confirm_git_write")
    gate = function_body(bridge_source, "take_confirmable_git_job")
    audit = function_body(bridge_source, "git_write_audit_detail")
    if not request or not confirm:
        return ["GIT_WRITE_GATE_COMMANDS_MISSING"]

    if "check_invocation_source" not in request:
        violations.append("GIT_WRITE_REQUEST_WITHOUT_SOURCE_CHECK")
    if "sync::write_" in request:
        violations.append("GIT_WRITE_GATE_BYPASS_DIRECT_WRITE")
    if "GitWriteStatus::Pending" not in request:
        violations.append("GIT_WRITE_REQUEST_WITHOUT_PENDING_JOB")

    if "check_invocation_source" not in confirm:
        violations.append("GIT_WRITE_CONFIRM_WITHOUT_SOURCE_CHECK")
    if "take_confirmable_git_job" not in confirm:
        violations.append("GIT_WRITE_CONFIRM_WITHOUT_GATE")
    if "confirmed_dangerous" not in confirm:
        violations.append("GIT_WRITE_DANGEROUS_WITHOUT_DOUBLE_CONFIRM")
    if "pending_git_jobs" not in confirm:
        violations.append("GIT_WRITE_CONFIRM_WITHOUT_JOB_TABLE")

    if "dangerous" not in gate or "confirmed_dangerous" not in gate:
        violations.append("GIT_WRITE_DANGEROUS_WITHOUT_DOUBLE_CONFIRM")
    if "expires_at" not in gate:
        violations.append("GIT_WRITE_GATE_WITHOUT_EXPIRY")

    signature = bridge_source[
        bridge_source.find("fn git_write_audit_detail") : bridge_source.find(
            "fn git_write_audit_detail"
        )
        + 400
    ]
    if any(param in signature for param in AUDIT_FORBIDDEN_PARAMS):
        violations.append("GIT_WRITE_AUDIT_SIGNATURE_LEAKS_DATA")
    if "job.paths" in audit:
        violations.append("GIT_WRITE_AUDIT_LEAKS_PATHS")
    return violations


def check_registration(main_source: str, acl_source: str) -> list[str]:
    violations: list[str] = []
    for command in GATE_COMMANDS:
        if f"bridge::{command}" not in main_source:
            violations.append("GIT_WRITE_COMMAND_NOT_REGISTERED")
        if f'"{command}"' not in acl_source:
            violations.append("GIT_WRITE_COMMAND_NOT_IN_ACL")
    return violations


def detect_violations(
    sync_source: str,
    domain_source: str,
    bridge_source: str,
    main_source: str,
    acl_source: str,
) -> list[str]:
    section = write_section(sync_source)
    if not section:
        return ["GIT_WRITE_SECTION_MISSING"]
    return sorted(
        set(
            check_write_primitives(section)
            + check_forbidden_api(section)
            + check_op_whitelist(domain_source)
            + check_gate(bridge_source)
            + check_registration(main_source, acl_source)
        )
    )


def scan_repository(root: Path) -> list[str]:
    src = root / "src-tauri" / "src"
    permissions = sorted((root / "src-tauri" / "permissions").glob("*.toml"))
    acl_source = "\n".join(p.read_text(encoding="utf-8") for p in permissions)
    return detect_violations(
        (src / "sync.rs").read_text(encoding="utf-8"),
        (src / "domain.rs").read_text(encoding="utf-8"),
        (src / "bridge.rs").read_text(encoding="utf-8"),
        (src / "main.rs").read_text(encoding="utf-8"),
        acl_source,
    )


# ---------------------------------------------------------------------------
# 自检夹具：好样本必须零违规；每个坏样本必须命中对应违规码
# ---------------------------------------------------------------------------

GOOD_SYNC = """
// M1-6.b Git 写原语（stage / unstage / ...）
// 硬约束：严禁出现 cred_cb / push / fetch / clone / pull / token
pub fn write_stage(repo: &Repository, paths: &[String]) -> Result<usize, String> {
    validate_repo_paths(paths)?;
    index.write()?;
    Ok(paths.len())
}
pub fn write_unstage(repo: &Repository, paths: &[String]) -> Result<usize, String> {
    validate_repo_paths(paths)?;
    repo.reset_default(Some(&target), paths.iter())?;
    Ok(paths.len())
}
pub fn write_discard(repo: &Repository, paths: &[String]) -> Result<usize, String> {
    validate_repo_paths(paths)?;
    Ok(paths.len())
}
pub fn write_commit(repo: &Repository, message: &str, paths: Option<&[String]>) -> Result<String, String> {
    validate_commit_message(message)?;
    Ok("oid".into())
}
pub fn write_create_branch(repo: &Repository, name: &str, checkout: bool) -> Result<String, String> {
    validate_branch_name(name)?;
    Ok(name.into())
}
pub fn write_checkout_branch(repo: &Repository, name: &str) -> Result<String, String> {
    validate_branch_name(name)?;
    Ok(name.into())
}
#[cfg(test)]
mod git_write_tests {}
"""

GOOD_DOMAIN = """
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitWriteOp {
    Stage,
    Unstage,
    Discard,
    Commit,
    CreateBranch,
    CheckoutBranch,
}

impl GitWriteOp {
    pub fn from_op_str(s: &str) -> Option<GitWriteOp> {
        match s {
            "stage" => Some(GitWriteOp::Stage),
            "unstage" => Some(GitWriteOp::Unstage),
            "discard" => Some(GitWriteOp::Discard),
            "commit" => Some(GitWriteOp::Commit),
            "create_branch" => Some(GitWriteOp::CreateBranch),
            "checkout_branch" => Some(GitWriteOp::CheckoutBranch),
            _ => None,
        }
    }
}
"""

GOOD_BRIDGE = """
pub fn request_git_write(app: AppHandle, webview: tauri::Webview) -> Result<GitWritePreview, String> {
    check_invocation_source(&webview, "request_git_write", None, &app)?;
    let job = GitWriteJob { status: GitWriteStatus::Pending, expires_at: now };
    Ok(preview)
}
pub fn confirm_git_write(app: AppHandle, webview: tauri::Webview, confirmed_dangerous: Option<bool>) -> Result<GitWriteJob, String> {
    check_invocation_source(&webview, "confirm_git_write", None, &app)?;
    let mut jobs = state.pending_git_jobs.lock().unwrap();
    take_confirmable_git_job(&mut jobs, &job_id, now, confirmed_dangerous.unwrap_or(false))?;
    Ok(job)
}
pub(crate) fn take_confirmable_git_job(jobs: &mut HashMap<String, GitWriteJob>, job_id: &str, now: i64, confirmed_dangerous: bool) -> Result<GitWriteJob, String> {
    if now >= job.expires_at { return Err("任务已过期".into()); }
    if job.dangerous && !confirmed_dangerous { return Err("需二次确认".into()); }
    Ok(job)
}
pub(crate) fn git_write_audit_detail(op: GitWriteOp, repo_id: &str, job_id: &str, path_count: usize, confirmed: bool, extra: &str) -> String {
    let payload = serde_json::json!({ "op": op.as_str(), "path_count": path_count });
    payload.to_string()
}
"""

GOOD_MAIN = "bridge::request_git_write,\n bridge::confirm_git_write,\n"
GOOD_ACL = '"request_git_write",\n "confirm_git_write"\n'


def run_self_test() -> int:
    failures: list[str] = []

    good = detect_violations(
        GOOD_SYNC, GOOD_DOMAIN, GOOD_BRIDGE, GOOD_MAIN, GOOD_ACL
    )
    if good:
        failures.append(f"good fixture should be clean, got {good}")

    cases: list[tuple[str, dict[str, str], str]] = [
        (
            "无校验直写",
            {"sync": GOOD_SYNC.replace("validate_repo_paths(paths)?;", "", 1)},
            "GIT_WRITE_PRIMITIVE_WITHOUT_VALIDATION",
        ),
        (
            "写段出现 push",
            {"sync": GOOD_SYNC.replace("index.write()?;", "remote.push(&[\"x\"])?;", 1)},
            "GIT_WRITE_PUSH_API",
        ),
        (
            "写段出现 reset",
            {"sync": GOOD_SYNC.replace("index.write()?;", "repo.reset(&obj, Hard, None)?;", 1)},
            "GIT_WRITE_RESET_API",
        ),
        (
            "写段读凭据",
            {"sync": GOOD_SYNC.replace("index.write()?;", "let t = KeyringStore::get_token(\"r\")?;", 1)},
            "GIT_WRITE_CREDENTIAL_ACCESS",
        ),
        (
            "shell 调 git",
            {"sync": GOOD_SYNC.replace("index.write()?;", "Command::new(\"git\").arg(\"reset\").output()?;", 1)},
            "GIT_WRITE_SHELL_GIT",
        ),
        (
            "白名单扩权",
            {
                "domain": GOOD_DOMAIN.replace(
                    "    CheckoutBranch,\n", "    CheckoutBranch,\n    Push,\n"
                )
            },
            "GIT_WRITE_OP_WHITELIST_DRIFT",
        ),
        (
            "解析与枚举不一致",
            {"domain": GOOD_DOMAIN.replace('"stage" => Some(GitWriteOp::Stage),', '"stage" => Some(GitWriteOp::Commit),')},
            "GIT_WRITE_OP_PARSER_DRIFT",
        ),
        (
            "request 缺来源校验",
            {"bridge": GOOD_BRIDGE.replace('check_invocation_source(&webview, "request_git_write", None, &app)?;', "")},
            "GIT_WRITE_REQUEST_WITHOUT_SOURCE_CHECK",
        ),
        (
            "request 绕过闸门直写",
            {"bridge": GOOD_BRIDGE.replace("Ok(preview)", "sync::write_commit(&repo, \"m\", None)?; Ok(preview)")},
            "GIT_WRITE_GATE_BYPASS_DIRECT_WRITE",
        ),
        (
            "confirm 缺二次确认",
            {"bridge": GOOD_BRIDGE.replace("confirmed_dangerous.unwrap_or(false)", "false")},
            "GIT_WRITE_DANGEROUS_WITHOUT_DOUBLE_CONFIRM",
        ),
        (
            "闸门缺过期判定",
            {"bridge": GOOD_BRIDGE.replace('if now >= job.expires_at { return Err("任务已过期".into()); }', "")},
            "GIT_WRITE_GATE_WITHOUT_EXPIRY",
        ),
        (
            "审计签名带路径清单",
            {"bridge": GOOD_BRIDGE.replace("path_count: usize, confirmed: bool, extra: &str", "paths: &[String], diff: &str")},
            "GIT_WRITE_AUDIT_SIGNATURE_LEAKS_DATA",
        ),
        (
            "命令未注册 main.rs",
            {"main": "bridge::git_status,\n"},
            "GIT_WRITE_COMMAND_NOT_REGISTERED",
        ),
        (
            "命令未进 ACL",
            {"acl": '"git_status"\n'},
            "GIT_WRITE_COMMAND_NOT_IN_ACL",
        ),
    ]

    for label, overrides, expected_code in cases:
        found = detect_violations(
            overrides.get("sync", GOOD_SYNC),
            overrides.get("domain", GOOD_DOMAIN),
            overrides.get("bridge", GOOD_BRIDGE),
            overrides.get("main", GOOD_MAIN),
            overrides.get("acl", GOOD_ACL),
        )
        if expected_code not in found:
            failures.append(f"{label}: expected {expected_code}, got {found}")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print("self-test: ok")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check M1-6 Git write safety invariants.")
    parser.add_argument("--self-test", action="store_true", help="run built-in tests")
    args = parser.parse_args(argv)
    if args.self_test:
        return run_self_test()

    violations = scan_repository(Path(__file__).resolve().parent.parent)
    if violations:
        print(f"check-git-write-policy: failed ({len(violations)} violation(s))")
        for violation in violations:
            print(violation)
        return 1
    print("check-git-write-policy: ok (Git write invariants hold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
