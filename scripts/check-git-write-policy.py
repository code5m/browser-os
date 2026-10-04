#!/usr/bin/env python3
"""Expose the M1-6 Git write safety invariants as a reproducible source fixture.

M1-6.b 落地 Git 写后端核心（双阶段闸门 + 六个白名单写操作），M1-6.c 复核固化
静态夹具，M1-6.d 增加 push 后按任务书 REQUIRE #11 扩展 push 不变量，防止后续
改动悄悄绕过：

  - 写原语必须各自带校验器调用（先校验后执行）
  - 写段不得出现黑名单 API / 联网 / shell 调 git
  - push 敏感 API（.push(/find_remote(/cred_cb/token）只允许出现在 write_push 体内；
    KeyringStore 在写段全面禁止（凭据只能在 bridge 层推送瞬间读取）
  - push 禁止 force refspec / 前导冒号（删除远端分支）/ 任意 refspec 参数注入，
    且只允许推送 origin
  - 白名单恰好七项且 `from_op_str` 与枚举一致；push 必须 dangerous
  - 闸门两阶段命令必须做来源校验、不得有绕过闸门的直写路径
  - dangerous 操作必须走二次确认、任务必须带过期判定
  - 审计构造器签名/实现不得接收路径清单、diff 或凭据；push 审计必须带
    branch / remote_name 字段
  - 新命令必须同时注册到 main.rs 与 ACL
  - 既有 `push_artifacts` 行为不得被改变（行为锚点检查）

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
    "write_push": "build_push_refspec",
}

# Git 写白名单：枚举变体 → 序列化名（M1-6.b 六项 + M1-6.d push）
EXPECTED_OPS: dict[str, str] = {
    "Stage": "stage",
    "Unstage": "unstage",
    "Discard": "discard",
    "Commit": "commit",
    "CreateBranch": "create_branch",
    "CheckoutBranch": "checkout_branch",
    "Push": "push",
}

# 写段全面禁止的 API / 语义（(正则, 违规码)）——push 敏感 API 另行按位置检查
FORBIDDEN_IN_WRITE_SECTION: tuple[tuple[str, str], ...] = (
    (r"\.fetch\(", "GIT_WRITE_FETCH_API"),
    (r"\.merge\(", "GIT_WRITE_MERGE_API"),
    (r"reset\(", "GIT_WRITE_RESET_API"),
    (r"cleanup_state\(", "GIT_WRITE_MERGE_API"),
    (r"\bstash\b|rebase|cherry[-_]pick", "GIT_WRITE_BLACKLISTED_OP"),
    (r"userpass_plaintext", "GIT_WRITE_CREDENTIAL_ACCESS"),
    (r"Command::new\(\s*\"git\"", "GIT_WRITE_SHELL_GIT"),
    (r"Command::new\(\s*\"sh\"\s*\)\s*[\s\S]{0,120}?\.arg\(\s*\"-c\"", "GIT_WRITE_SHELL_GIT"),
)

# push 敏感 API：只允许出现在 write_push 函数体内
SENSITIVE_PUSH_APIS: tuple[str, ...] = (
    r"\.push\(",
    r"find_remote\(",
    r"cred_cb",
    r"\btoken\b",
)

WRITE_SECTION_MARKER = "// M1-6.b Git 写原语"
GATE_COMMANDS = ("request_git_write", "confirm_git_write")
AUDIT_FORBIDDEN_PARAMS = ("paths", "diff", "content", "credential", "token", "secret")

# 既有 push_artifacts 行为锚点（M1-6.d 不得改变其链路）
PUSH_ARTIFACTS_ANCHORS = ("pull(", "commit(", "push(", "KeyringStore::get_token")


def function_span(source: str, name: str) -> tuple[int, int] | None:
    """Return (start, end) offsets of a Rust function including its body."""
    match = re.search(rf"\bfn\s+{re.escape(name)}\s*\(", source)
    if not match:
        return None
    start = source.find("{", match.end())
    if start < 0:
        return None
    depth = 0
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return (match.start(), index + 1)
    return None


def function_body(source: str, name: str) -> str:
    span = function_span(source, name)
    if span is None:
        return ""
    return source[span[0] : span[1]]


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


def check_push_primitive(section: str) -> list[str]:
    """push 不变量：敏感 API 只在 write_push/scrub_sensitive_error 体内；
    禁 force/删除 refspec 字面量；禁任意 refspec 参数注入；只允许 origin；
    KeyringStore 在写段全面禁止。"""
    code = strip_comments(section)
    push_span = function_span(code, "write_push")
    if push_span is None:
        return ["GIT_WRITE_PUSH_PRIMITIVE_MISSING"]
    # token 处理合法存在于 write_push（凭据回调）与 scrub_sensitive_error（错误脱敏）
    allowed_spans = [
        span
        for span in (push_span, function_span(code, "scrub_sensitive_error"))
        if span is not None
    ]
    violations: list[str] = []
    for pattern in SENSITIVE_PUSH_APIS:
        for match in re.finditer(pattern, code):
            if not any(start <= match.start() < end for start, end in allowed_spans):
                violations.append("GIT_WRITE_SENSITIVE_API_OUTSIDE_PUSH_PRIMITIVE")
                break
    if "KeyringStore" in code:
        violations.append("GIT_WRITE_KEYRING_IN_WRITE_SECTION")
    body = code[push_span[0] : push_span[1]]
    signature = body[: body.find("{")]
    if "refspec" in signature:
        violations.append("GIT_WRITE_PUSH_REFSPEC_INJECTABLE")
    if "GIT_PUSH_REMOTE" not in body:
        violations.append("GIT_WRITE_PUSH_UNSAFE_REMOTE")
    # force（"+refs/..."）/删除（":refs/..."）refspec 字面量：整个写段都不允许出现
    if re.search(r'"\+\s*refs/', code) or re.search(r'":\s*refs/', code):
        violations.append("GIT_WRITE_PUSH_FORCE_OR_DELETE")
    if re.search(r"\bforce\b", body):
        violations.append("GIT_WRITE_PUSH_FORCE_OR_DELETE")
    return violations


def check_op_whitelist(domain_source: str) -> list[str]:
    violations: list[str] = []
    enum_match = re.search(r"pub enum GitWriteOp \{(.*?)\n\}", domain_source, re.S)
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
    dangerous = function_body(domain_source, "is_dangerous")
    if "Push" not in dangerous:
        violations.append("GIT_WRITE_PUSH_NOT_DANGEROUS")
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
    # 必须把关卡的 confirmed_dangerous 传给闸门（仅出现在签名里不算）
    if not re.search(
        r"take_confirmable_git_job\([\s\S]{0,240}?confirmed_dangerous", confirm
    ):
        violations.append("GIT_WRITE_DANGEROUS_WITHOUT_DOUBLE_CONFIRM")
    if "pending_git_jobs" not in confirm:
        violations.append("GIT_WRITE_CONFIRM_WITHOUT_JOB_TABLE")

    if "dangerous" not in gate or "confirmed_dangerous" not in gate:
        violations.append("GIT_WRITE_DANGEROUS_WITHOUT_DOUBLE_CONFIRM")
    if "expires_at" not in gate:
        violations.append("GIT_WRITE_GATE_WITHOUT_EXPIRY")

    execute = function_body(bridge_source, "execute_git_write")
    if "write_push" not in execute:
        violations.append("GIT_WRITE_PUSH_GATE_MISSING")

    # 只看函数签名（到第一个 `{` 为止），函数体内的字段名不算
    signature_start = bridge_source.find("fn git_write_audit_detail")
    signature_end = bridge_source.find("{", signature_start)
    signature = bridge_source[signature_start:signature_end]
    if any(param in signature for param in AUDIT_FORBIDDEN_PARAMS):
        violations.append("GIT_WRITE_AUDIT_SIGNATURE_LEAKS_DATA")
    if "branch" not in signature or "remote_name" not in signature:
        violations.append("GIT_WRITE_PUSH_AUDIT_FIELDS_MISSING")
    if "job.paths" in audit:
        violations.append("GIT_WRITE_AUDIT_LEAKS_PATHS")
    return violations


def check_registration(main_source: str, acl_source: str) -> list[str]:
    violations: list[str] = []
    for command in GATE_COMMANDS:
        if not any(f"{owner}::{command}" in main_source for owner in ("bridge", "crate::capabilities::git::commands")):
            violations.append("GIT_WRITE_COMMAND_NOT_REGISTERED")
        if f'"{command}"' not in acl_source:
            violations.append("GIT_WRITE_COMMAND_NOT_IN_ACL")
    return violations


def check_push_artifacts_untouched(sync_source: str) -> list[str]:
    body = function_body(sync_source, "push_artifacts")
    if not all(anchor in body for anchor in PUSH_ARTIFACTS_ANCHORS):
        return ["GIT_WRITE_PUSH_ARTIFACTS_BROKEN"]
    return []


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
            + check_push_primitive(section)
            + check_op_whitelist(domain_source)
            + check_gate(bridge_source)
            + check_registration(main_source, acl_source)
            + check_push_artifacts_untouched(sync_source)
        )
    )


def scan_repository(root: Path) -> list[str]:
    src = root / "src-tauri" / "src"
    permissions = sorted((root / "src-tauri" / "permissions").glob("*.toml"))
    acl_source = "\n".join(p.read_text(encoding="utf-8") for p in permissions)
    return detect_violations(
        (src / "capabilities" / "git" / "sync.rs").read_text(encoding="utf-8"),
        (src / "domain.rs").read_text(encoding="utf-8"),
        (src / "capabilities/git/commands.rs").read_text(encoding="utf-8"),
        (src / "main.rs").read_text(encoding="utf-8"),
        acl_source,
    )


# ---------------------------------------------------------------------------
# 自检夹具：好样本必须零违规；每个坏样本必须命中对应违规码
# ---------------------------------------------------------------------------

GOOD_SYNC = """
pub fn push_artifacts(app: &AppHandle, job: &SyncJob) -> Result<(), String> {
    let token = KeyringStore::get_token(&repo.id)?;
    pull(&repository, &repo.branch, &repo.username, &token)?;
    commit(&repository, "msg")?;
    push(&repository, &repo.branch, &repo.username, &token)?;
    Ok(())
}

// M1-6.b Git 写原语（stage / unstage / ...）
// 硬约束：严禁出现 cred_cb / push / fetch / clone / pull / token（注释不算违规）
pub const GIT_PUSH_REMOTE: &str = "origin";

pub(crate) fn build_push_refspec(branch: &str) -> Result<String, String> {
    validate_branch_name(branch)?;
    Ok(format!("refs/heads/{branch}:refs/heads/{branch}"))
}

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
pub fn write_push(repo: &Repository, username: &str, token: &str) -> Result<String, String> {
    let branch = head.shorthand().unwrap().to_string();
    let refspec = build_push_refspec(&branch)?;
    let mut remote = repo.find_remote(GIT_PUSH_REMOTE).map_err(|e| e.to_string())?;
    let mut po = PushOptions::new();
    po.remote_callbacks(cred_cb(username.to_string(), token.to_string()));
    remote.push(&[refspec.as_str()], Some(&mut po)).map_err(|e| e.to_string())?;
    Ok(branch)
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
    Push,
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
            "push" => Some(GitWriteOp::Push),
            _ => None,
        }
    }

    pub fn is_dangerous(&self) -> bool {
        matches!(self, GitWriteOp::Discard | GitWriteOp::Push)
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
fn execute_git_write(app: &AppHandle, job: &GitWriteJob) -> Result<usize, String> {
    let token = KeyringStore::get_token(&cfg.id)?;
    sync::write_push(&repo, &cfg.username, &token).map(|_| 0)
}
pub(crate) fn git_write_audit_detail(op: GitWriteOp, repo_id: &str, job_id: &str, path_count: usize, confirmed: bool, branch: Option<&str>, remote_name: Option<&str>, extra: &str) -> String {
    let payload = serde_json::json!({ "op": op.as_str(), "path_count": path_count, "branch": branch, "remote_name": remote_name });
    payload.to_string()
}
"""

GOOD_MAIN = "bridge::request_git_write,\n bridge::confirm_git_write,\n"
GOOD_ACL = '"request_git_write",\n "confirm_git_write"\n'


def run_self_test() -> int:
    failures: list[str] = []

    good = detect_violations(GOOD_SYNC, GOOD_DOMAIN, GOOD_BRIDGE, GOOD_MAIN, GOOD_ACL)
    if good:
        failures.append(f"good fixture should be clean, got {good}")
    migrated_main = GOOD_MAIN.replace("bridge::", "crate::capabilities::git::commands::")
    if detect_violations(GOOD_SYNC, GOOD_DOMAIN, GOOD_BRIDGE, migrated_main, GOOD_ACL):
        failures.append("capability-owned command registration should remain protected and valid")
    if not check_registration(migrated_main.replace("::git::", "::unknown::"), GOOD_ACL):
        failures.append("unrecognized command owner must be rejected")

    cases: list[tuple[str, dict[str, str], str]] = [
        (
            "无校验直写",
            {"sync": GOOD_SYNC.replace("validate_repo_paths(paths)?;", "", 1)},
            "GIT_WRITE_PRIMITIVE_WITHOUT_VALIDATION",
        ),
        (
            "write_push 缺失",
            {"sync": GOOD_SYNC.replace("fn write_push", "fn write_push_removed")},
            "GIT_WRITE_PUSH_PRIMITIVE_MISSING",
        ),
        (
            "push API 出现在 write_push 之外",
            {"sync": GOOD_SYNC.replace("index.write()?;", "remote.push(&[\"x\"], None)?;", 1)},
            "GIT_WRITE_SENSITIVE_API_OUTSIDE_PUSH_PRIMITIVE",
        ),
        (
            "写段读 Keyring",
            {"sync": GOOD_SYNC.replace("index.write()?;", "let t = KeyringStore::get_token(\"r\")?;", 1)},
            "GIT_WRITE_KEYRING_IN_WRITE_SECTION",
        ),
        (
            "写段出现 reset",
            {"sync": GOOD_SYNC.replace("index.write()?;", "repo.reset(&obj, Hard, None)?;", 1)},
            "GIT_WRITE_RESET_API",
        ),
        (
            "shell 调 git",
            {"sync": GOOD_SYNC.replace("index.write()?;", "Command::new(\"git\").arg(\"reset\").output()?;", 1)},
            "GIT_WRITE_SHELL_GIT",
        ),
        (
            "force refspec",
            {"sync": GOOD_SYNC.replace('format!("refs/heads/{branch}:refs/heads/{branch}")', 'format!("+refs/heads/{branch}:refs/heads/{branch}")')},
            "GIT_WRITE_PUSH_FORCE_OR_DELETE",
        ),
        (
            "非 origin 远端",
            {"sync": GOOD_SYNC.replace('repo.find_remote(GIT_PUSH_REMOTE)', 'repo.find_remote("upstream")')},
            "GIT_WRITE_PUSH_UNSAFE_REMOTE",
        ),
        (
            "refspec 参数注入",
            {"sync": GOOD_SYNC.replace("pub fn write_push(repo: &Repository, username: &str, token: &str)", "pub fn write_push(repo: &Repository, username: &str, token: &str, refspec: &str)")},
            "GIT_WRITE_PUSH_REFSPEC_INJECTABLE",
        ),
        (
            "白名单扩权",
            {"domain": GOOD_DOMAIN.replace("    Push,\n", "    Push,\n    ForcePush,\n")},
            "GIT_WRITE_OP_WHITELIST_DRIFT",
        ),
        (
            "解析与枚举不一致",
            {"domain": GOOD_DOMAIN.replace('"stage" => Some(GitWriteOp::Stage),', '"stage" => Some(GitWriteOp::Commit),')},
            "GIT_WRITE_OP_PARSER_DRIFT",
        ),
        (
            "push 未标 dangerous",
            {"domain": GOOD_DOMAIN.replace("GitWriteOp::Discard | GitWriteOp::Push", "GitWriteOp::Discard")},
            "GIT_WRITE_PUSH_NOT_DANGEROUS",
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
            "闸门缺 push 映射",
            {"bridge": GOOD_BRIDGE.replace("sync::write_push(&repo, &cfg.username, &token)", "sync::write_stage(&repo, &job.paths)")},
            "GIT_WRITE_PUSH_GATE_MISSING",
        ),
        (
            "审计签名带路径清单",
            {"bridge": GOOD_BRIDGE.replace("path_count: usize, confirmed: bool, branch: Option<&str>, remote_name: Option<&str>, extra: &str", "paths: &[String], diff: &str")},
            "GIT_WRITE_AUDIT_SIGNATURE_LEAKS_DATA",
        ),
        (
            "审计缺 push 字段",
            {"bridge": GOOD_BRIDGE.replace("branch: Option<&str>, remote_name: Option<&str>, ", "")},
            "GIT_WRITE_PUSH_AUDIT_FIELDS_MISSING",
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
        (
            "push_artifacts 行为被破坏",
            {"sync": GOOD_SYNC.replace("pull(&repository, &repo.branch, &repo.username, &token)?;", "")},
            "GIT_WRITE_PUSH_ARTIFACTS_BROKEN",
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
