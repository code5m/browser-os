#!/usr/bin/env python3
"""Expose the database Owner invariants as a reproducible fixture.

The M4 contracts remain active: metadata never contains credentials, writes
default to denied, SQL classification fails closed, and native synchronous
drivers preserve bounded results, cancellation, timeouts and platform refusal.
The source of these checks is the physical capabilities/database Owner, including
its command, studio and durable profile paths; legacy root re-exports are not
implementation evidence.

BFC/Agent Foundation may depend on tokio. This does not authorize the Database
Owner to import an async driver/runtime, introduce an ORM or launch a JDBC sidecar.
Tauri's existing spawn_blocking bridge delegates to the synchronous driver.

DB_PERSIST_NOT_ATOMIC is ACTIVE: profiles::write_records now exists and must keep
its exclusive temporary file, sync, atomic replacement and failure cleanup.
--expect-pending remains compatible with existing callers; no pending exception
remains. Static checks do not replace the real SQL classifier, transaction,
credential, TLS and driver integration tests.
"""

from __future__ import annotations

import argparse
import ast
import re
import sys
from pathlib import Path

# ----------------------------- 码位清单 -----------------------------

ACTIVE_CODES: tuple[str, ...] = (
    # ---- 基线（M4-1 冻结即生效）----
    "DB_CFG_HAS_PASSWORD_FIELD",
    "DB_WRITE_DEFAULT_DENY",
    "DB_JDBC_SIDECAR",
    "DB_UNDECLARED_RUNTIME",
    "DB_ERROR_CODE_CLOSED",
    "DB_SAFETY_HAS_DB_DEP",
    "DB_SQL_PARSE_FAIL_CLOSED",
    # ---- 2026-09-06 Lane A2 提升：A4 命令层 + A3 连接池均已落地，产物存在故不再是「存在才判」----
    # A4（M4-3 命令层）
    "DB_ACL_ORDER",
    "DB_CRED_IN_AUDIT",
    "DB_MULTI_STATEMENT_FORBIDDEN",
    # A3（M4-2 连接池与查询核心）
    "DB_CRED_NAMESPACE",
    "DB_RESULT_LIMIT_MISSING",
    "DB_TIMEOUT_NOT_LAYERED",
    "DB_PLATFORM_DEGRADE",
    "DB_PERSIST_NOT_ATOMIC",
)

PENDING_CODES: tuple[str, ...] = ()


# ----------------------------- 检测常量 -----------------------------

# 连接配置中**结构性禁止**出现的字段名（F2）
CFG_FORBIDDEN_FIELD_RE = re.compile(
    r"^\s*(?:pub\s+)?(password|passwd|pwd|secret|token|dsn|conn_str|connection_string)\s*:",
    re.M,
)

# 允许出现的凭据相关字段（`username` 不是凭据，且 Keyring 键不落在配置里）
# —— 命中即违规的只有上表的字段名。

# 禁用的运行时 / ORM（M4-1.a §3 裁定：同步栈，应用层不自建 runtime）
FORBIDDEN_RUNTIME_RE = re.compile(r"^\s*(async-std|sqlx|diesel|sea-orm)\s*=", re.M)
DB_RUNTIME_USE_RE = re.compile(
    r"\b(?:tokio|async_std|sqlx|diesel|sea_orm)\s*::|"
    r"\buse\s+(?:tokio|async_std|sqlx|diesel|sea_orm)\b"
)

# JDBC 侧车（WBS 明令禁止）
JDBC_TOKENS = ("jdbc", "jre", "jdk", "maven", "gradle", "ojdbc")

# 安全闸门零 db 依赖（F4）——只认**代码引用**形态，避免把注释里的说明文字误判
DB_DEP_REFS = (
    "use sqlx",
    "use rusqlite",
    "use mysql",
    "use postgres",
    "use diesel",
    "sqlx::",
    "rusqlite::",
    "mysql::",
    "postgres::",
    "crate::database",
    "crate::capabilities::database",
)

# 分类器必备 API（`DB_SQL_PARSE_FAIL_CLOSED` 的组成之一）
REQUIRED_SQL_APIS = (
    "fn classify_sql_risk",
    "fn is_write_statement",
    "fn is_production_database",
    "fn require_write_confirmation",
    "enum ProductionVerdict",
)

# 分类器必备用例：删除任一即 FAIL（N-sql 失败用例 + 防 fail-closed 过头的反向用例）
REQUIRED_SQL_TESTS = (
    "empty_sql_is_rejected",
    "oversized_sql_is_rejected",
    "unterminated_block_comment_is_unparsable",
    "unterminated_string_literal_is_unparsable",
    "stacked_drop_is_rejected_as_multiple_statements",
    "comment_wrapped_delete_is_rejected",
    "cte_delete_is_rejected",
    "mixed_case_ddl_is_rejected",
    "explain_analyze_is_not_treated_as_read",
    "select_into_outfile_is_treated_as_write",
    "unknown_verb_is_rejected",
    "truncate_is_ddl_class",
    # 反向用例（正常只读查询不得被误伤）
    "trailing_semicolon_is_single_statement",
    "semicolon_inside_literal_does_not_split",
    "chinese_and_comment_read_query_is_allowed",
    # 写闸门与生产判定
    "write_is_denied_by_default",
    "write_on_unknown_production_is_denied",
    "write_without_second_confirmation_is_denied",
    "ddl_is_never_allowed_even_with_all_gates",
    "unresolvable_host_verdict_is_unknown",
    "empty_database_name_verdict_is_unknown",
    "prod_name_only_verdict_is_unknown",
    "explicit_nonprod_hint_with_loopback_is_nonproduction",
)

# db 命令（A1 展开卡 F-3：占位名，最终以 M4-3.a 冻结为准）。
# `db_cancel` 见 M4-1.c §3（未冻结）、`db_forget_connection` 见 D28 / O-A2-1（未裁决）——
# 二者在冻结前均不纳入本表。
DB_COMMANDS = ("db_connect", "db_query", "db_disconnect")

# 多语句通道（M4-1.c §5：命令层禁止用这些 API 处理用户 SQL）。
# 注意：扫描目标是 **bridge.rs 命令层**（db_query），不是 database.rs——
# A3 的 database.rs 仅在 `#[cfg(test)]` 建表 fixture 里用 `execute_batch`，属合法，
# 扫 database.rs 会恒真误报。
MULTI_STATEMENT_APIS = ("execute_batch", "simple_query", "batch_execute")

# 超时分层常量（M4-1.c §4：30 / 600 / 5）
TIMEOUT_CONSTANTS = (
    "DB_DEFAULT_QUERY_TIMEOUT_SECS",
    "DB_MAX_QUERY_TIMEOUT_SECS",
    "DB_SOFT_TO_HARD_GRACE_SECS",
)

# db 审计事件：detail 不得含 SQL 原文与凭据（A10 G-3）
DB_AUDIT_EVENTS = ("db.connect", "db.query", "db.disconnect", "db.write", "db.cancel")
# 审计 detail 禁含片段。用**精确串**而非裸词：`sql` 会误命中紧邻的 db_query 函数
# 签名 `sql: String`（参数名，非 SQL 内容）；真正泄漏形态是 `sql=`、`sql_body` 等。
AUDIT_FORBIDDEN = (
    "sql=",
    "sql_body",
    "raw_sql",
    "statement",
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "dsn",
    "conn_str",
)

ACL_ANCHOR = "list_artifact_images"

DB_ERROR_CODE_COUNT = 18


# ----------------------------- 通用工具 -----------------------------


def strip_comments(source: str) -> str:
    without_blocks = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
    return "\n".join(line.split("//", 1)[0] for line in without_blocks.splitlines())


def rust_struct_body(source: str, name: str) -> str:
    m = re.search(rf"\bstruct\s+{re.escape(name)}\b[^{{]*\{{", source)
    if not m:
        return ""
    start = source.find("{", m.start())
    depth = 0
    for i in range(start, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[start : i + 1]
    return source[start:]


def rust_fn_body(source: str, name: str) -> str:
    match = re.search(rf"\b(?:pub\s+)?fn\s+{re.escape(name)}\s*\(", source)
    if not match:
        return ""
    depth = 0
    index = match.end() - 1
    while index < len(source):
        if source[index] == "(":
            depth += 1
        elif source[index] == ")":
            depth -= 1
            if depth == 0:
                break
        index += 1
    start = source.find("{", index)
    if start < 0:
        return ""
    depth = 0
    for i in range(start, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[start : i + 1]
    return source[start:]


def cargo_dependencies(cargo: str) -> str:
    """只取 `[dependencies]` 段（**直接依赖**）。

    `Cargo.lock` 中本来就有 tokio（tauri 传递依赖），扫 lock 会得到恒真告警；
    `[dev-dependencies]` / `[build-dependencies]` 也不代表应用运行时栈。
    """
    lines: list[str] = []
    in_section = False
    for raw in cargo.splitlines():
        if re.match(r"^\[[A-Za-z0-9_.\-]+\]", raw):
            in_section = raw.strip() == "[dependencies]"
            continue
        if in_section:
            lines.append(raw)
    return "\n".join(lines)


def _jdbc_command(value) -> bool:
    if isinstance(value, (list, tuple)):
        command = " ".join(item for item in value if isinstance(item, str))
    elif isinstance(value, str):
        command = value
    else:
        return False
    return bool(re.search(r"\b(?:java(?:\.exe)?|mvn|gradle)\b", command, re.I)
                and re.search(r"jdbc|ojdbc|database[-_]bridge", command, re.I))


def script_starts_jdbc(name: str, source: str) -> bool:
    if name.endswith(".py"):
        try:
            tree = ast.parse(source)
        except SyntaxError as error:
            raise RuntimeError(f"DB_SCRIPT_PARSE_FAILED: {name}") from error
        modules = {"subprocess", "os"}
        functions = set()
        launches = {"run", "Popen", "call", "check_call", "check_output",
                    "system", "execv", "execve", "execvp", "execvpe"}
        constants = {}
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                modules.update(alias.asname or alias.name for alias in node.names
                               if alias.name in ("subprocess", "os"))
            elif isinstance(node, ast.ImportFrom) and node.module in ("subprocess", "os"):
                functions.update(alias.asname or alias.name for alias in node.names
                                 if alias.name in launches)
            elif isinstance(node, ast.Assign) and len(node.targets) == 1:
                target = node.targets[0]
                if isinstance(target, ast.Name):
                    try:
                        constants[target.id] = ast.literal_eval(node.value)
                    except (ValueError, TypeError):
                        constants.pop(target.id, None)
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            fn = node.func
            is_launch = (
                isinstance(fn, ast.Attribute) and isinstance(fn.value, ast.Name)
                and fn.value.id in modules and fn.attr in launches
            ) or (isinstance(fn, ast.Name) and fn.id in functions)
            if not is_launch:
                continue
            args = list(node.args[:1]) + [arg.value for arg in node.keywords if arg.arg == "args"]
            for arg in args:
                try:
                    value = constants[arg.id] if isinstance(arg, ast.Name) else ast.literal_eval(arg)
                    if _jdbc_command(value):
                        return True
                except (KeyError, ValueError, TypeError):
                    # A launcher with unresolved arguments and JDBC material
                    # needs explicit review; do not silently approve a sidecar.
                    if re.search(r"jdbc|ojdbc|database[-_]bridge", source, re.I):
                        return True
        return False
    if name.endswith(".sh"):
        return any(_jdbc_command(line) for line in source.splitlines()
                   if re.match(r"^\s*(?:exec\s+)?(?:java(?:\.exe)?|mvn|gradle)\s", line))
    if name.endswith(".mjs"):
        return any(_jdbc_command(line) for line in source.splitlines()
                   if re.match(
                       r"^\s*(?:(?:const|let)\s+\w+\s*=\s*)?(?:await\s+)?"
                       r"(?:\w+\.)?(?:spawn|spawnSync|execFile|execFileSync|exec|execSync)\s*\(",
                       line))
    return False


def rust_call_arguments(source: str, name: str) -> list[list[str]]:
    """Extract call arguments, without mistaking format strings for delimiters."""
    calls = []
    for match in re.finditer(rf"\b{re.escape(name)}\s*\(", source):
        stack = ["("]
        start = match.end()
        args = []
        quoted = False
        escaped = False
        for index in range(start, len(source)):
            char = source[index]
            if quoted:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    quoted = False
                continue
            if char == '"':
                quoted = True
            elif char in "([{":
                stack.append(char)
            elif char in ")]}":
                if not stack:
                    break
                stack.pop()
                if not stack:
                    tail = source[start:index].strip()
                    if tail:
                        args.append(tail)
                    calls.append(args)
                    break
            elif char == "," and len(stack) == 1:
                args.append(source[start:index].strip())
                start = index + 1
    return calls


def _ordered(source: str, patterns: tuple[str, ...]) -> bool:
    offset = 0
    for pattern in patterns:
        match = re.search(pattern, source[offset:], re.S)
        if not match:
            return False
        offset += match.end()
    return True


# ----------------------------- 检测规则 -----------------------------


def detect_hits(files: dict) -> dict[str, list[str]]:
    """返回 {码位: [说明]}；ACTIVE 与 PENDING 一并检测，由调用方按模式过滤。"""
    hits: dict[str, list[str]] = {}

    def hit(code: str, detail: str) -> None:
        hits.setdefault(code, []).append(detail)

    domain = files.get("domain_rs", "")
    security = files.get("security_rs", "")
    database = files.get("database_rs", "")
    bridge = files.get("bridge_rs", "")
    profiles = files.get("profiles_rs", "")
    studio = files.get("studio_rs", "")
    main_rs = files.get("main_rs", "")
    acl = files.get("acl_toml", "")
    cargo = files.get("cargo_toml", "")
    build_rs = files.get("build_rs", "")
    scripts_txt = files.get("scripts_txt", "")

    domain_code = strip_comments(domain)
    security_code = strip_comments(security)
    database_code = strip_comments(database)
    profile_code = strip_comments(profiles)
    studio_code = strip_comments(studio)
    owner_code = "\n".join((database_code, strip_comments(bridge), profile_code, studio_code))

    # ================= ACTIVE =================

    # ---- 1) F2：连接配置结构性不含凭据字段 ----
    cfg_body = rust_struct_body(domain_code, "DbConnectionConfig")
    if not cfg_body:
        hit("DB_CFG_HAS_PASSWORD_FIELD", "domain.rs 缺 DbConnectionConfig 定义")
    else:
        for m in CFG_FORBIDDEN_FIELD_RE.finditer(cfg_body):
            hit("DB_CFG_HAS_PASSWORD_FIELD", f"DbConnectionConfig.{m.group(1)}")

    # ---- 2) F1：写默认拒绝 ----
    if cfg_body:
        m = re.search(
            r"((?:#[^\n]*\n\s*)*)pub\s+allow_write\s*:\s*bool", cfg_body
        )
        if not m:
            hit("DB_WRITE_DEFAULT_DENY", "DbConnectionConfig 缺 allow_write 字段")
        else:
            attrs = m.group(1)
            if "serde(default" not in attrs:
                hit("DB_WRITE_DEFAULT_DENY", "allow_write 缺 #[serde(default)]")
            elif re.search(r'default\s*=\s*"', attrs):
                fn = re.search(r'default\s*=\s*"([A-Za-z_][A-Za-z0-9_]*)"', attrs)
                name = fn.group(1) if fn else "?"
                body = rust_fn_body(domain_code, name)
                if not body:
                    hit("DB_WRITE_DEFAULT_DENY", f"allow_write 默认函数 {name} 未定义")
                elif not re.search(r"\bfalse\b", body) or re.search(r"\btrue\b", body):
                    hit("DB_WRITE_DEFAULT_DENY", f"{name} 未返回 false（写缺省必须关闭）")

    # ---- 3) 禁 JDBC 侧车（WBS）----
    for label, blob in (
        ("Cargo.toml", cargo),
        ("build.rs", build_rs),
    ):
        if not blob.strip():
            continue
        low = blob.lower()
        for token in JDBC_TOKENS:
            if token in low:
                hit("DB_JDBC_SIDECAR", f"{label}:{token}")

    scripts = dict(files.get("scripts", {}))
    if scripts_txt:
        scripts["fixture.py"] = scripts_txt
    for name, source in scripts.items():
        if script_starts_jdbc(name, source):
            hit("DB_JDBC_SIDECAR", f"{name}: executable JDBC sidecar")
    if re.search(r'Command::new\s*\(\s*"(?:java|java\.exe)"', owner_code):
        hit("DB_JDBC_SIDECAR", "Database Owner spawns a Java sidecar")

    # ---- 4) Database Owner keeps synchronous drivers; shared tokio is not DB permission ----
    deps = cargo_dependencies(cargo)
    if cargo.strip():
        for m in FORBIDDEN_RUNTIME_RE.finditer(deps):
            hit(
                "DB_UNDECLARED_RUNTIME",
                f"Cargo.toml [dependencies] 引入 {m.group(1)}（同步驱动不引入第二套 ORM/runtime）",
            )

    for match in DB_RUNTIME_USE_RE.finditer(owner_code):
        hit("DB_UNDECLARED_RUNTIME", f"Database Owner 引用 {match.group(0)}")

    # ---- 5) 错误码闭合（18 码，变体↔映射一致）----
    enum_m = re.search(r"pub\s+enum\s+DbErrorCode\s*\{([^}]*)\}", domain_code)
    if not enum_m:
        hit("DB_ERROR_CODE_CLOSED", "domain.rs 缺 DbErrorCode 枚举")
    else:
        variants = re.findall(r"^\s*([A-Z][A-Za-z0-9_]*)\s*,", enum_m.group(1), re.M)
        pairs = re.findall(
            r"DbErrorCode::([A-Za-z0-9_]+)\s*=>\s*\"([A-Z0-9_]+)\"", domain_code
        )
        mapped = [v for v, _ in pairs]
        codes = [c for _, c in pairs]
        if len(variants) != DB_ERROR_CODE_COUNT:
            hit(
                "DB_ERROR_CODE_CLOSED",
                f"DbErrorCode 变体数 {len(variants)} != {DB_ERROR_CODE_COUNT}",
            )
        if sorted(variants) != sorted(mapped):
            missing = sorted(set(variants) - set(mapped))
            extra = sorted(set(mapped) - set(variants))
            hit("DB_ERROR_CODE_CLOSED", f"变体与 as_str 映射不一致 missing={missing} extra={extra}")
        for code in codes:
            if not code.startswith("DB_"):
                hit("DB_ERROR_CODE_CLOSED", f"错误码串未用 DB_ 前缀：{code}")
        if len(set(codes)) != len(codes):
            hit("DB_ERROR_CODE_CLOSED", "错误码串存在重复")

    # ---- 6) F4：安全闸门零 db 依赖 ----
    if security_code.strip():
        for bad in DB_DEP_REFS:
            if bad in security_code:
                hit(
                    "DB_SAFETY_HAS_DB_DEP",
                    f"security_policy.rs 引用 {bad}（F4：分类器与生产判定必须是纯函数）",
                )
        for api in REQUIRED_SQL_APIS:
            if api not in security_code:
                hit("DB_SAFETY_HAS_DB_DEP", f"security_policy.rs 缺 {api}")

    # ---- 7) G-4/G-5：分类器 fail-closed 用例仍在 ----
    if security.strip():
        missing = [t for t in REQUIRED_SQL_TESTS if t not in security]
        for name in missing:
            hit(
                "DB_SQL_PARSE_FAIL_CLOSED",
                f"缺失必需用例 {name}（不可解析即拒 / 多语句整批拒 / 生产判定不确定即拒的取证被删除）",
            )

    # ================= Implemented Owner paths (ACTIVE) =================

    # ---- P1) ACL 顺序：db_* 必须插在 list_artifact_images 之前（A4）----
    anchor = acl.find(f'"{ACL_ANCHOR}"')
    if anchor >= 0:
        for cmd in DB_COMMANDS:
            idx = acl.find(f'"{cmd}"')
            if idx > anchor:
                hit("DB_ACL_ORDER", f"{cmd} 排在 {ACL_ANCHOR} 之后")

    # ---- DB audit examines only the actual detail argument, across both Owners ----
    for source in (strip_comments(bridge), studio_code):
        for args in rust_call_arguments(source, "log_audit"):
            if len(args) < 3 or not re.match(r'^"db\.', args[1]):
                continue
            detail = args[2].lower()
            for bad in AUDIT_FORBIDDEN + ("sql",):
                if re.search(rf"\b{re.escape(bad)}", detail):
                    hit("DB_CRED_IN_AUDIT", f"{args[1]}:{bad}")

    if not database_code.strip():
        hit("DB_RESULT_LIMIT_MISSING", "Database driver Owner is missing")
        hit("DB_TIMEOUT_NOT_LAYERED", "Database driver Owner is missing")
        return hits

    # ---- User SQL must keep one-statement execution and the same fail-closed gate ----
    sql_entries = [
        ("commands", bridge, name)
        for name in re.findall(r"\bfn\s+(db_query[A-Za-z0-9_]*)\s*\(", bridge)
    ]
    if not any(name == "db_query" for _label, _source, name in sql_entries):
        hit("DB_SQL_PARSE_FAIL_CLOSED", "commands.rs 缺实际 db_query Owner")
    if studio_code:
        sql_entries.extend(("studio", studio_code, name) for name in ("run_sql", "transact"))
    for label, source, name in sql_entries:
        body = rust_fn_body(strip_comments(source), name)
        for bad in MULTI_STATEMENT_APIS:
            if re.search(rf"\b{bad}\s*\(", body):
                hit("DB_MULTI_STATEMENT_FORBIDDEN", f"{label}::{name} 使用 {bad}")
        if name in ("db_query", "run_sql"):
            gate = body.find("evaluate_db_query_gate")
            query = re.search(r"\.\s*query\s*\(|crate::database::query\s*\(", body)
            gate_end = body.find(";", gate) if gate >= 0 else -1
            if (gate < 0 or not query or gate >= query.start()
                    or gate_end < 0 or "?" not in body[gate:gate_end]):
                hit("DB_SQL_PARSE_FAIL_CLOSED", f"{label}::{name} 未在查询前传播安全闸门拒绝")

    # ---- Credential identity follows the canonical namespace and validated versions ----
    key = rust_fn_body(database_code, "credential_key")
    literal_key = re.search(r'format!\s*\(\s*"db:\{\}"\s*,\s*conn_id\s*\)', key)
    constant_key = (
        re.search(r'const\s+DB_CRED_PREFIX\s*:\s*&str\s*=\s*"db:"', database_code)
        and re.search(r'format!\s*\(\s*"\{DB_CRED_PREFIX\}\{conn_id\}"\s*\)', key)
    )
    if not literal_key and not constant_key:
        hit("DB_CRED_NAMESPACE", "credential_key 未绑定 db:<conn_id>")
    if profile_code:
        reference = rust_fn_body(profile_code, "reference")
        if not _ordered(reference, (
            r"credential_key\s*\(\s*&self\.config\.id\s*\)",
            r"\.strip_prefix\s*\(\s*&format!\s*\(\s*\"\{legacy\}:\"\s*\)\s*\)",
            r"Uuid::parse_str\s*\(\s*version\s*\)",
        )):
            hit("DB_CRED_NAMESPACE", "StoredProfile.reference 未验证所属连接及版本 UUID")
    for operation in ("stage", "record_retirement", "reconcile"):
        for args in rust_call_arguments(owner_code, f"credential_versions::{operation}"):
            if len(args) < 2 or args[1] != '"db"':
                hit("DB_CRED_NAMESPACE", f"credential_versions::{operation} 未绑定 db Owner")
    if re.search(r"KeyringStore::(?:save_token|get_token|delete_token)\s*\(\s*&?(?:conn_id|cfg\.id|id)\s*[,)]",
                 owner_code):
        hit("DB_CRED_NAMESPACE", "Keyring 使用裸连接 ID，绕过命名空间")

    # ---- P5) 结果结构必须有截断标记（A3，M4-1.c §2）----
    if re.search(r"\bstruct\s+\w*QueryResult\b", database_code) or "row_count" in database_code:
        for field in ("truncated", "field_truncated"):
            if not re.search(rf"\bpub\s+{field}\s*:", database_code):
                hit("DB_RESULT_LIMIT_MISSING", f"结果 DTO 缺 {field} 字段（禁止静默截断）")

    # ---- Durable profile persistence is implemented and ACTIVE ----
    for fn in ("save_connections", "save_connections_at", "persist_config"):
        body = rust_fn_body(database_code, fn)
        if body and "atomic_write" not in body:
            hit("DB_PERSIST_NOT_ATOMIC", f"database.rs::{fn} 未走 atomic_write")
    writer = rust_fn_body(profile_code, "write_records")
    if not writer or not _ordered(writer, (
        r"serde_json::to_vec_pretty\s*\(\s*configs\s*\)",
        r"assert_regular_or_missing\s*\(\s*path\s*\)\s*\?",
        r"\.write\s*\(\s*true\s*\)\s*\.create_new\s*\(\s*true\s*\)",
        r"\.open\s*\(\s*&temp\s*\)",
        r"\.write_all\s*\(\s*&bytes\s*\)",
        r"\.sync_all\s*\(\s*\)",
        r"crate::session::atomic_replace_file\s*\(\s*&temp\s*,\s*path\s*\)",
    )):
        hit("DB_PERSIST_NOT_ATOMIC", "profiles::write_records 缺独占临时文件/同步/原子替换")
    if (not re.search(r"if\s+result\.is_err\(\).*?fs::remove_file\s*\(\s*&temp\s*\)", writer, re.S)
            or "directory.sync_all()" not in writer
            or not re.search(r"opts\.mode\s*\(\s*0o600\s*\)", writer)
            or re.search(r"(?:fs::write|File::create)\s*\(\s*path\s*[,)]", writer)):
        hit("DB_PERSIST_NOT_ATOMIC", "profile 写入缺权限/父目录同步/失败清理或绕过 staging")
    snapshot = rust_fn_body(strip_comments(bridge), "persist_snapshot")
    if not re.search(r"profiles::write_records\s*\(\s*&profiles::path_for\s*\(", snapshot):
        hit("DB_PERSIST_NOT_ATOMIC", "commands::persist_snapshot 未委托真实 profile Owner")

    # ---- P7) 超时分层必须复用 30/600/5（A3，M4-1.c §4）----
    for name in TIMEOUT_CONSTANTS:
        if name not in database_code:
            hit("DB_TIMEOUT_NOT_LAYERED", f"database.rs 缺 {name}")

    # ---- P8) 非 Linux 必须显式降级（A3，D29）----
    if "target_os" in database_code:
        if "NotSupported" not in database_code:
            hit(
                "DB_PLATFORM_DEGRADE",
                "database.rs 出现 target_os 分支但未返回 DbErrorCode::NotSupported（禁止静默失败）",
            )

    return hits


# ----------------------------- 输入读取 -----------------------------


def _read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return ""


def _read_required(path: Path) -> str:
    content = _read(path)
    if not content.strip():
        raise RuntimeError(f"DB_OWNER_SOURCE_MISSING: {path}")
    return content


def _script_sources(root: Path) -> dict[str, str]:
    # Preserve language boundaries: policy fixtures containing launch examples
    # are data, whereas an actual subprocess call is an executable sidecar.
    self_name = Path(__file__).name
    return {p.name: _read(p)
            for pattern in ("*.py", "*.sh", "*.mjs")
            for p in sorted((root / "scripts").glob(pattern))
            if p.is_file() and p.name != self_name}


def read_repo(root: Path) -> dict:
    return {
        "domain_rs": _read(root / "src-tauri/src/domain.rs"),
        "security_rs": _read(root / "src-tauri/src/security_policy.rs"),
        "database_rs": _read_required(root / "src-tauri/src/capabilities/database/database.rs"),
        "bridge_rs": _read_required(root / "src-tauri/src/capabilities/database/commands.rs"),
        "profiles_rs": _read_required(root / "src-tauri/src/capabilities/database/profiles.rs"),
        "studio_rs": _read_required(root / "src-tauri/src/capabilities/database/studio.rs"),
        "main_rs": _read(root / "src-tauri/src/main.rs"),
        "acl_toml": _read(root / "src-tauri/permissions/default-commands.toml"),
        "cargo_toml": _read(root / "src-tauri/Cargo.toml"),
        "build_rs": _read(root / "src-tauri/build.rs"),
        "scripts": _script_sources(root),
    }


# ----------------------------- 合成参考实现 -----------------------------
# 用途：证明「产物一旦落地，码位不会误报，且能被变异检出」。
# 这些字符串**不写入仓库**，只存在于自检进程内。

DOMAIN_GOOD = '''
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConnectionConfig {
    pub id: String,
    pub name: String,
    pub kind: SupportedDb,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    pub database: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub ssl_mode: DbSslMode,
    #[serde(default)]
    pub allow_write: bool,
    #[serde(default)]
    pub production_hint: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum DbErrorCode {
    UnsupportedKind,
    InvalidConfig,
    PathOutsideRoots,
    CredentialMissing,
    ConnectFailed,
    NotConnected,
    PoolExhausted,
    SqlEmpty,
    SqlTooLarge,
    SqlParseFailed,
    MultipleStatements,
    WriteDenied,
    ProductionUnknown,
    QueryFailed,
    Timeout,
    Cancelled,
    LimitExceeded,
    NotSupported,
}

impl DbErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            DbErrorCode::UnsupportedKind => "DB_UNSUPPORTED_KIND",
            DbErrorCode::InvalidConfig => "DB_INVALID_CONFIG",
            DbErrorCode::PathOutsideRoots => "DB_PATH_OUTSIDE_ROOTS",
            DbErrorCode::CredentialMissing => "DB_CREDENTIAL_MISSING",
            DbErrorCode::ConnectFailed => "DB_CONNECT_FAILED",
            DbErrorCode::NotConnected => "DB_NOT_CONNECTED",
            DbErrorCode::PoolExhausted => "DB_POOL_EXHAUSTED",
            DbErrorCode::SqlEmpty => "DB_SQL_EMPTY",
            DbErrorCode::SqlTooLarge => "DB_SQL_TOO_LARGE",
            DbErrorCode::SqlParseFailed => "DB_SQL_PARSE_FAILED",
            DbErrorCode::MultipleStatements => "DB_MULTIPLE_STATEMENTS",
            DbErrorCode::WriteDenied => "DB_WRITE_DENIED",
            DbErrorCode::ProductionUnknown => "DB_PRODUCTION_UNKNOWN",
            DbErrorCode::QueryFailed => "DB_QUERY_FAILED",
            DbErrorCode::Timeout => "DB_TIMEOUT",
            DbErrorCode::Cancelled => "DB_CANCELLED",
            DbErrorCode::LimitExceeded => "DB_LIMIT_EXCEEDED",
            DbErrorCode::NotSupported => "DB_NOT_SUPPORTED",
        }
    }
}
'''

SECURITY_GOOD = '''
pub fn classify_sql_risk(sql: &str) -> SqlClassification { todo!() }
pub fn is_write_statement(sql: &str) -> bool { todo!() }
pub fn is_production_database(s: &ProductionSignals) -> ProductionVerdict { todo!() }
pub fn require_write_confirmation(
    c: &SqlClassification,
    v: ProductionVerdict,
    allow_write: bool,
    confirmed: bool,
) -> Result<(), DbErrorCode> { todo!() }

pub enum ProductionVerdict { Production, NonProduction, Unknown }
''' + "\n".join(
    f"fn {name}() {{}}" for name in REQUIRED_SQL_TESTS
)

DATABASE_GOOD = '''
pub const DB_DEFAULT_QUERY_TIMEOUT_SECS: u32 = 30;
pub const DB_MAX_QUERY_TIMEOUT_SECS: u32 = 600;
pub const DB_SOFT_TO_HARD_GRACE_SECS: u32 = 5;

pub struct DbQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<DbValue>>,
    pub row_count: usize,
    pub truncated: bool,
    pub field_truncated: bool,
    pub elapsed_ms: u64,
    pub query_id: String,
}

fn credential_key(conn_id: &str) -> String {
    format!("db:{}", conn_id)
}

pub fn connect(cfg: &DbConnectionConfig) -> Result<Pool, DbErrorCode> {
    let secret = KeyringStore::get_token(&credential_key(&cfg.id))?;
    let _ = secret;
    Ok(Pool::new())
}

pub fn save_connections(path: &Path, list: &[DbConnectionConfig]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    crate::session::atomic_write(path, &content)
}

#[cfg(not(target_os = "linux"))]
pub fn connect_native() -> Result<Pool, DbErrorCode> {
    Err(DbErrorCode::NotSupported)
}
'''

BRIDGE_GOOD = '''
#[tauri::command]
pub fn db_query(app: AppHandle, webview: tauri::Webview, conn_id: String, sql: String)
    -> Result<DbQueryResult, String> {
    check_invocation_source(&webview, "db_query", None, &app)?;
    sp::evaluate_db_query_gate(&sql, &cfg, encrypted, confirmed)?;
    let rows = crate::database::query(&app, &conn_id, &sql)?;
    workspace::log_audit(&app, "db.query", format!("conn_id={} rows={} truncated={}",
        conn_id, rows.row_count, rows.truncated));
    Ok(rows)
}
fn persist_snapshot(app: &AppHandle, records: &[StoredProfile]) -> Result<(), String> {
    profiles::write_records(&profiles::path_for(app)?, records)
}
'''

PROFILES_GOOD = '''
impl StoredProfile {
    pub fn reference(&self) -> Result<String, String> {
        let legacy = crate::database::credential_key(&self.config.id);
        if self.credential_ref.is_empty() || self.credential_ref == legacy {
            return Ok(legacy);
        }
        let version = self.credential_ref.strip_prefix(&format!("{legacy}:"))
            .ok_or("DB_CREDENTIAL_REF_INVALID")?;
        uuid::Uuid::parse_str(version).map_err(|_| "DB_CREDENTIAL_REF_INVALID")?;
        Ok(self.credential_ref.clone())
    }
}
pub(crate) fn write_records(path: &Path, configs: &[StoredProfile]) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(configs)?;
    assert_regular_or_missing(path)?;
    let temp = dir.join("profiles.tmp");
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    opts.mode(0o600);
    let result = (|| {
        let mut f = opts.open(&temp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        crate::session::atomic_replace_file(&temp, path)?;
        #[cfg(unix)]
        fs::File::open(dir).and_then(|directory| directory.sync_all())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
'''

STUDIO_GOOD = '''
fn run_sql(pool: &mut DbPool, sql: &str) -> Result<Value, String> {
    sp::evaluate_db_query_gate(sql, cfg, pool.encrypted(), confirmed)?;
    pool.query(sql, cancel, timeout, id)
}
'''

MAIN_GOOD = '''
mod database;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            bridge::db_connect,
            bridge::db_query,
            bridge::db_disconnect,
        ])
        .run(tauri::generate_context!())
        .expect("startup failed");
}
'''

ACL_GOOD = '''[
    "db_connect",
    "db_query",
    "db_disconnect",
    "list_artifact_images"
]
'''

CARGO_GOOD = '''[package]
name = "mvp-browser-os"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = ["unstable", "protocol-asset"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
mysql = "28"
postgres = "0.19"
chrono = { version = "0.4", features = ["serde"] }
libc = "0.2"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
'''

BUILD_GOOD = '''fn main() {
    tauri_build::build()
}
'''


def reference_impl() -> dict:
    return {
        "domain_rs": DOMAIN_GOOD,
        "security_rs": SECURITY_GOOD,
        "database_rs": DATABASE_GOOD,
        "bridge_rs": BRIDGE_GOOD,
        "profiles_rs": PROFILES_GOOD,
        "studio_rs": STUDIO_GOOD,
        "main_rs": MAIN_GOOD,
        "acl_toml": ACL_GOOD,
        "cargo_toml": CARGO_GOOD,
        "build_rs": BUILD_GOOD,
        "scripts_txt": "",
    }


# ----------------------------- 自检 -----------------------------


def run_self_test(root: Path) -> int:
    failures: list[str] = []

    # 0) 码位集合自身一致
    overlap = set(ACTIVE_CODES) & set(PENDING_CODES)
    if overlap:
        failures.append(f"码位重复定义（ACTIVE ∩ PENDING）：{sorted(overlap)}")

    # 1) 真实仓库：实际 driver/command/studio/profile Owner 均必须满足契约
    real = detect_hits(read_repo(root))
    if real:
        failures.append(f"真实仓库存在违规（应为空）：{real}")

    # 2) 好样本 B：合成参考实现（产物齐备 → 仍必须零违规，证明无误报）
    good = reference_impl()
    ref = detect_hits(good)
    if ref:
        failures.append(f"合成参考实现存在违规（应为空，说明码位误报）：{sorted(ref)}")

    scoped_good = dict(good)
    scoped_good["cargo_toml"] = good["cargo_toml"].replace(
        'postgres = "0.19"', 'postgres = "0.19"\ntokio = { version = "1", features = ["net"] }')
    scoped_good["bridge_rs"] += (
        '\nfn credential_worker(secret: &str) { let _ = secret; }\n')
    scoped_good["scripts"] = {
        "metadata.py": 'jdbc_url = "jdbc:mysql://example.invalid/database"\n',
        "policy_fixture.py": (
            'fixture = \'subprocess.run(["java", "-jar", "jdbc-bridge.jar"])\'\n'),
    }
    scoped_hits = detect_hits(scoped_good)
    if scoped_hits:
        failures.append(f"共享 runtime/独立凭据函数/侧车文本夹具正例误报：{scoped_hits}")

    samples: list[tuple[str, dict, str, str]] = []

    def mutate(**kw) -> dict:
        d = dict(good)
        d.update(kw)
        return d

    def add(desc: str, mutated: dict, key: str, expect: str) -> None:
        """登记坏样本，并做**变异防呆**：内容必须真的改动，否则按漏检计。"""
        if mutated.get(key, "") == good.get(key, ""):
            print(f"self-test FAIL: 坏样本「{desc}」未改动任何内容（变异失配，按漏检计）")
            sys.exit(1)
        samples.append((desc, mutated, key, expect))

    # ---- ACTIVE 坏样本（7）----

    add(
        "连接配置新增 password 字段",
        mutate(domain_rs=good["domain_rs"].replace(
            "    #[serde(default)]\n    pub username: Option<String>,\n",
            "    #[serde(default)]\n    pub username: Option<String>,\n"
            "    #[serde(default)]\n    pub password: Option<String>,\n")),
        "domain_rs",
        "DB_CFG_HAS_PASSWORD_FIELD",
    )

    add(
        "allow_write 缺省改为 true",
        mutate(domain_rs=good["domain_rs"].replace(
            '    #[serde(default = "db_allow_write_default")]\n', "")
            .replace(
                "    #[serde(default)]\n    pub allow_write: bool,\n",
                '    #[serde(default = "db_allow_write_default")]\n    pub allow_write: bool,\n')
            + '\nfn db_allow_write_default() -> bool {\n    true\n}\n'),
        "domain_rs",
        "DB_WRITE_DEFAULT_DENY",
    )

    add(
        "build.rs 引入 JDBC 侧车",
        mutate(build_rs='fn main() {\n    println!("cargo:rustc-env=JDBC_BRIDGE=1");\n    tauri_build::build()\n}\n'),
        "build_rs",
        "DB_JDBC_SIDECAR",
    )

    add(
        "Database Owner 自建 tokio runtime",
        mutate(database_rs=good["database_rs"]
               + "\nfn connect_async() { tokio::runtime::Runtime::new(); }\n"),
        "database_rs",
        "DB_UNDECLARED_RUNTIME",
    )

    add(
        "DbErrorCode 删去一个变体但映射未同步",
        mutate(domain_rs=good["domain_rs"].replace("    NotSupported,\n", "")),
        "domain_rs",
        "DB_ERROR_CODE_CLOSED",
    )

    add(
        "分类器引入 db crate 依赖",
        mutate(security_rs=good["security_rs"] + "\npub fn leak(p: &crate::database::Pool) { let _ = p; }\n"),
        "security_rs",
        "DB_SAFETY_HAS_DB_DEP",
    )

    add(
        "删掉多语句拒绝用例",
        mutate(security_rs=good["security_rs"].replace(
            "fn stacked_drop_is_rejected_as_multiple_statements() {}", "")),
        "security_rs",
        "DB_SQL_PARSE_FAIL_CLOSED",
    )

    # ---- 已实现 Owner 的坏样本（含已转 ACTIVE 的持久化）----

    add(
        "db_* 插在 list_artifact_images 之后",
        mutate(acl_toml='[\n    "list_artifact_images",\n    "db_query"\n]\n'),
        "acl_toml",
        "DB_ACL_ORDER",
    )

    add(
        "db 审计 detail 写入 SQL 原文",
        mutate(bridge_rs=good["bridge_rs"].replace(
            'format!("conn_id={} rows={} truncated={}",\n        conn_id, rows.row_count, rows.truncated)',
            'format!("conn_id={} sql={}", conn_id, sql)')),
        "bridge_rs",
        "DB_CRED_IN_AUDIT",
    )

    add(
        "命令层用 execute_batch 处理用户 SQL",
        mutate(bridge_rs=good["bridge_rs"] + '\npub fn db_query_raw(sql: &str) { let _ = conn.execute_batch(sql); }\n'),
        "bridge_rs",
        "DB_MULTI_STATEMENT_FORBIDDEN",
    )

    add(
        "Keyring 键缺 db: 前缀（与 git repo_id 碰撞）",
        mutate(database_rs=good["database_rs"].replace(
            'format!("db:{}", conn_id)', "conn_id.to_string()")),
        "database_rs",
        "DB_CRED_NAMESPACE",
    )

    add(
        "结果 DTO 去掉截断标记",
        mutate(database_rs=good["database_rs"].replace("    pub field_truncated: bool,\n", "")),
        "database_rs",
        "DB_RESULT_LIMIT_MISSING",
    )

    add(
        "连接配置落盘退回裸 fs::write",
        mutate(database_rs=good["database_rs"].replace(
            "crate::session::atomic_write(path, &content)",
            "std::fs::write(path, content).map_err(|e| e.to_string())")),
        "database_rs",
        "DB_PERSIST_NOT_ATOMIC",
    )

    add(
        "超时另起一套口径（缺分层常量）",
        mutate(database_rs=good["database_rs"].replace(
            "pub const DB_SOFT_TO_HARD_GRACE_SECS: u32 = 5;",
            "pub const MY_OWN_GRACE: u32 = 3;")),
        "database_rs",
        "DB_TIMEOUT_NOT_LAYERED",
    )

    add(
        "非 Linux 分支静默失败（不返回 NotSupported）",
        mutate(database_rs=good["database_rs"].replace(
            'pub fn connect_native() -> Result<Pool, DbErrorCode> {\n    Err(DbErrorCode::NotSupported)\n}',
            'pub fn connect_native() -> Result<Pool, DbErrorCode> {\n    Err(DbErrorCode::QueryFailed)\n}')),
        "database_rs",
        "DB_PLATFORM_DEGRADE",
    )

    add(
        "Database Owner 不能通过 import alias 引入 runtime",
        mutate(database_rs=good["database_rs"] + "\nuse tokio as async_driver;\n"),
        "database_rs", "DB_UNDECLARED_RUNTIME")
    add(
        "新增 ORM 直接依赖仍拒绝",
        mutate(cargo_toml=good["cargo_toml"].replace(
            'postgres = "0.19"', 'postgres = "0.19"\nsqlx = "0.8"')),
        "cargo_toml", "DB_UNDECLARED_RUNTIME")
    for name, source in (
        ("launch.py", 'import subprocess\nsubprocess.run(["java", "-jar", "jdbc-bridge.jar"])\n'),
        ("alias.py", 'from subprocess import Popen as launch\n'
                     'command = ["java", "-jar", "jdbc-bridge.jar"]\nlaunch(command)\n'),
        ("launch.sh", 'java -jar jdbc-bridge.jar\n'),
        ("launch.mjs", 'childProcess.spawn("java", ["-jar", "jdbc-bridge.jar"]);\n'),
    ):
        add(f"真实 JDBC 侧车启动 {name}", mutate(scripts={name: source}),
            "scripts", "DB_JDBC_SIDECAR")
    add(
        "迁移后的命令 Owner 安全闸门被移除",
        mutate(bridge_rs=good["bridge_rs"].replace(
            "    sp::evaluate_db_query_gate(&sql, &cfg, encrypted, confirmed)?;\n", "")),
        "bridge_rs", "DB_SQL_PARSE_FAIL_CLOSED")
    add(
        "Studio 安全闸门错误不再传播",
        mutate(studio_rs=good["studio_rs"].replace(
            "pool.encrypted(), confirmed)?;", "pool.encrypted(), confirmed);")),
        "studio_rs", "DB_SQL_PARSE_FAIL_CLOSED")
    add(
        "审计 format 捕获 SQL 变量仍是泄漏",
        mutate(bridge_rs=good["bridge_rs"].replace(
            'format!("conn_id={} rows={} truncated={}",\n        conn_id, rows.row_count, rows.truncated)',
            'format!("{sql}")')),
        "bridge_rs", "DB_CRED_IN_AUDIT")
    add(
        "Studio 审计不得回显凭据",
        mutate(studio_rs=good["studio_rs"]
               + '\nfn audit() { log_audit(&app, "db.session.operation", format!("{password}")); }\n'),
        "studio_rs", "DB_CRED_IN_AUDIT")
    add(
        "移除真实 driver Owner 不能由旧 shim 兜底通过",
        mutate(database_rs="pub use super::database::*;\n"),
        "database_rs", "DB_CRED_NAMESPACE")
    add(
        "持久化凭据版本必须验证 UUID",
        mutate(profiles_rs=good["profiles_rs"].replace(
            '        uuid::Uuid::parse_str(version).map_err(|_| "DB_CREDENTIAL_REF_INVALID")?;\n', "")),
        "profiles_rs", "DB_CRED_NAMESPACE")
    add(
        "凭据版本操作不得切换到 Git 命名空间",
        mutate(bridge_rs=good["bridge_rs"]
               + '\nfn stage() { credential_versions::stage(&path, "git", &id, secret, &retired); }\n'),
        "bridge_rs", "DB_CRED_NAMESPACE")
    for before, after, detail in (
        ("opts.write(true).create_new(true);", "opts.write(true).create(true);", "独占创建"),
        ("        f.sync_all()?;\n", "", "数据同步"),
        ("crate::session::atomic_replace_file(&temp, path)?;", "fs::write(path, bytes)?;", "原子替换"),
        ("directory.sync_all()", "Ok(())", "目录同步"),
        ("        let _ = fs::remove_file(&temp);", "", "失败清理"),
        ("opts.mode(0o600);", "opts.mode(0o666);", "私有权限"),
    ):
        add(f"profile writer 失去{detail}",
            mutate(profiles_rs=good["profiles_rs"].replace(before, after)),
            "profiles_rs", "DB_PERSIST_NOT_ATOMIC")
    add(
        "命令持久化绕过 Profile Owner",
        mutate(bridge_rs=good["bridge_rs"].replace(
            "profiles::write_records(&profiles::path_for(app)?, records)",
            'fs::write("profiles.json", "[]")')),
        "bridge_rs", "DB_PERSIST_NOT_ATOMIC")

    # ---- 执行坏样本 ----
    covered: set[str] = set()
    for desc, mutated, _key, expect in samples:
        got = detect_hits(mutated)
        if expect in got:
            covered.add(expect)
        else:
            failures.append(f"坏样本「{desc}」未检出 {expect}（实得 {sorted(got)}）")

    uncovered = sorted((set(ACTIVE_CODES) | set(PENDING_CODES)) - covered)
    if uncovered:
        failures.append(f"以下码位没有任何坏样本覆盖：{uncovered}")

    if failures:
        print("DB_SELF_TEST_RESULT=FAIL")
        for f in failures:
            print(f"  x {f}")
        return 1

    print(
        f"DB_SELF_TEST_RESULT=PASS: 3 好样本零违规（真实 Owner + 合成参考 + Owner 分离）"
        f" + {len(samples)} 个坏样本全部检出（含变异防呆）；"
        f"ACTIVE={len(ACTIVE_CODES)} PENDING={len(PENDING_CODES)}"
    )
    return 0


# ----------------------------- 入口 -----------------------------


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--self-test", action="store_true", help="好样本 + 坏样本双向自检（含变异防呆）")
    ap.add_argument(
        "--expect-pending",
        action="store_true",
        help="验证 pending 码位仍未实现；一旦可检出即应转入默认判定",
    )
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[1]

    if args.self_test:
        return run_self_test(root)

    hits = detect_hits(read_repo(root))

    if args.expect_pending:
        pending_hits = [c for c in PENDING_CODES if c in hits]
        if pending_hits:
            print("DB_PENDING_RESULT=FAIL")
            for code in pending_hits:
                for detail in hits[code]:
                    print(f"  {code}:{detail}")
            return 1
        print(f"DB_PENDING_RESULT=NONE（{len(PENDING_CODES)} 个 pending 码位均未实现）")
        return 0

    active_hits = sorted(c for c in hits if c in ACTIVE_CODES)
    if active_hits:
        print("DB_POLICY_RESULT=FAIL")
        for code in active_hits:
            for detail in hits[code]:
                print(f"  {code}:{detail}")
        return 1
    print(f"database policy: all invariants hold（ACTIVE={len(ACTIVE_CODES)}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
