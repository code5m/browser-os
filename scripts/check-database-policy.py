#!/usr/bin/env python3
"""Expose the M4 database invariants as a reproducible fixture.

契约来源：
- `logs/checkpoints/M4-1.b-20260905-2250.md`（`SupportedDb` / `DbConnectionConfig` / `DbErrorCode`）
- `logs/checkpoints/M4-1.c-20260905-2255.md`（结果上限 / 取消 / 截断 / 多语句通道）
- `logs/checkpoints/M4-1.d-20260905-2300.md` §3（生产判定信号契约）与 §5（码位表）
- `logs/assist/A10-M4-security-review-20260905-2240.md` G-1~G-6、G-10、G-11

本夹具守住的底线（默认模式 14 个 ACTIVE 码位，1 个 PENDING）：

  凭据结构（F2）
  - `DbConnectionConfig` **结构性不含** password 字段——不是「写前清空」，是类型上不存在

  写默认拒绝（F1）
  - `allow_write` 必须存在且缺省 `false`；禁止自定义默认函数返回 `true`

  依赖形态（M4-1.a §6.1 / G-11）
  - 禁 JDBC 侧车
  - 禁把 `tokio` / `async-std` / `sqlx` / `diesel` 提升为直接依赖（同步栈裁定）

  错误码闭合（M4-1.b §4）
  - `DbErrorCode` 变体、`as_str` 映射、`DB_` 前缀三者一致（18 码）

  安全闸门纯度（F4）
  - `security_policy.rs` 零 db 依赖：分类器与生产判定是纯函数，才能先于数据库实现合入

  分类器 fail-closed（G-4 / G-5）
  - 不可解析即拒、多语句整批拒、注释与字符串混淆不得绕过——
    这条不变量的行为本体在 `cargo test`（M4-2.s 测试模块），静态层只能守住
    「必需测试仍在」：删掉任一关键用例即 FAIL（见下文的诚实声明）

PENDING 码位（1 个）：见下方 `PENDING_CODES` 注释。

**2026-09-06（Lane A2）提升记录**：A4 命令层（`db_connect` / `db_query` /
`db_disconnect` 已落地，ACL 顺序 + 审计脱敏就位）与 A3 连接池（`database.rs`
已落地）均已实现，故原 8 个 pending 中 7 个提升为 ACTIVE：

  - A4：`DB_ACL_ORDER` / `DB_CRED_IN_AUDIT` / `DB_MULTI_STATEMENT_FORBIDDEN`
  - A3：`DB_CRED_NAMESPACE` / `DB_RESULT_LIMIT_MISSING` / `DB_TIMEOUT_NOT_LAYERED` / `DB_PLATFORM_DEGRADE`

真实仓库复跑 15 码位全部零违规，产物已存在故不再是「存在才判」的空转项。
其中 `DB_CRED_NAMESPACE` 为「弱守」：其 Keyring 调用实际落在 `bridge.rs`，
本码位只扫 `database.rs`（仅含 `credential_key` / `DB_CRED_PREFIX` 的 `db:`
字面量）。待 A4 把凭据落库逻辑收口后，建议把扫描目标扩到 `bridge.rs`（见 §5 备注）。

唯一保留的 `DB_PERSIST_NOT_ATOMIC`：连接配置落盘（F10，
`data_dir/db_connections.json` 原子写）尚未实现——当前 `bridge.rs` 仅维护内存态
`DbConnectionRegistry`，无文件持久化。一旦 A3 落 `save_connections` + `atomic_write`，
应从 `PENDING_CODES` 移除并补坏样本。

`--expect-pending` 会在「pending 码位已经可被检出」时 FAIL —— 那意味着它已被
实现，应立即转入默认判定。当前仅 `DB_PERSIST_NOT_ATOMIC` 保留，其制品缺失故
返回 NONE（符合预期）。

关于「产物存在才判」：仅剩 `DB_PERSIST_NOT_ATOMIC` 受此约束——`database*.rs`
存在但无 `save_connections` 系列函数时该码位自动降级为 no-op；其余码位因制品已
落地而常驻生效。

**诚实声明**：`DB_SQL_PARSE_FAIL_CLOSED` 是**行为**不变量，真正的判定在
`cargo test security_policy`（36 条用例）。本脚本只能静态守住「这些用例还在、
分类器 API 还在」，不能替代运行时断言；两者都必须跑。

默认模式：ACTIVE 不变量全部成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
--self-test：真实仓库零违规 + 一份**合成的合规参考实现**零违规 + 每个码位
至少一个变异坏样本被检出（含变异防呆：坏样本必须真的改动内容，否则按漏检计）。
"""

from __future__ import annotations

import argparse
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
)

PENDING_CODES: tuple[str, ...] = (
    # A3（M4-2 连接池与查询核心）：连接配置落盘（F10，`data_dir/db_connections.json`
    # 原子写）尚未实现——当前 `bridge.rs` 仅维护内存态 `DbConnectionRegistry`，无文件
    # 持久化。一旦 A3 落 `save_connections` + `atomic_write`，应转入 ACTIVE 并补坏样本。
    "DB_PERSIST_NOT_ATOMIC",
)

# ----------------------------- 检测常量 -----------------------------

# 连接配置中**结构性禁止**出现的字段名（F2）
CFG_FORBIDDEN_FIELD_RE = re.compile(
    r"^\s*(?:pub\s+)?(password|passwd|pwd|secret|token|dsn|conn_str|connection_string)\s*:",
    re.M,
)

# 允许出现的凭据相关字段（`username` 不是凭据，且 Keyring 键不落在配置里）
# —— 命中即违规的只有上表的字段名。

# 禁用的运行时 / ORM（M4-1.a §3 裁定：同步栈，应用层不自建 runtime）
FORBIDDEN_RUNTIME_RE = re.compile(r"^\s*(tokio|async-std|sqlx|diesel|sea-orm)\s*=", re.M)

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
    main_rs = files.get("main_rs", "")
    acl = files.get("acl_toml", "")
    cargo = files.get("cargo_toml", "")
    build_rs = files.get("build_rs", "")
    scripts_txt = files.get("scripts_txt", "")

    domain_code = strip_comments(domain)
    security_code = strip_comments(security)
    database_code = strip_comments(database)

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
        ("scripts", scripts_txt),
    ):
        if not blob.strip():
            continue
        low = blob.lower()
        for token in JDBC_TOKENS:
            if token in low:
                hit("DB_JDBC_SIDECAR", f"{label}:{token}")

    # ---- 4) 依赖形态：不得提升 tokio / async ORM ----
    deps = cargo_dependencies(cargo)
    if cargo.strip():
        for m in FORBIDDEN_RUNTIME_RE.finditer(deps):
            hit(
                "DB_UNDECLARED_RUNTIME",
                f"Cargo.toml [dependencies] 引入 {m.group(1)}（M4-1.a §3：同步栈，应用层不自建 runtime）",
            )

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

    # ================= PENDING =================

    # ---- P1) ACL 顺序：db_* 必须插在 list_artifact_images 之前（A4）----
    anchor = acl.find(f'"{ACL_ANCHOR}"')
    if anchor >= 0:
        for cmd in DB_COMMANDS:
            idx = acl.find(f'"{cmd}"')
            if idx > anchor:
                hit("DB_ACL_ORDER", f"{cmd} 排在 {ACL_ANCHOR} 之后")

    # ---- P2) db 审计 detail 不得含 SQL 原文与凭据（A4，G-3）----
    for event in DB_AUDIT_EVENTS:
        index = bridge.find(f'"{event}"')
        if index < 0:
            continue
        window = bridge[index : index + 260].lower()
        for bad in AUDIT_FORBIDDEN:
            if bad in window:
                hit("DB_CRED_IN_AUDIT", f"{event}:{bad}")

    if not database_code.strip():
        return hits

    # ---- P3) 多语句通道禁用于处理用户 SQL（A4 命令层，M4-1.c §5）----
    # 扫 **bridge.rs 的 db_query**（命令层是唯一处理用户 SQL 的入口）；database.rs
    # 仅在测试 fixture 用 execute_batch 建表，属合法，不在此判。
    if "fn db_query" in bridge:
        for bad in MULTI_STATEMENT_APIS:
            if bad in bridge:
                hit("DB_MULTI_STATEMENT_FORBIDDEN", f"bridge.rs 命令层用 {bad} 处理用户 SQL")

    # ---- P4) Keyring 键必须是 db:<conn_id>（A3，G-1）----
    if re.search(r"KeyringStore::(save_token|get_token|delete_token)\s*\(", database_code):
        # 只认 `db:` 前缀字面量（`format!("db:{}", id)` 亦命中），不认裸 `conn_id`
        if not re.search(r'"db:', database_code):
            hit(
                "DB_CRED_NAMESPACE",
                "database.rs 使用 KeyringStore 但无 `db:` 键前缀（与 git 的 repo_id 会静默互串）",
            )

    # ---- P5) 结果结构必须有截断标记（A3，M4-1.c §2）----
    if re.search(r"\bstruct\s+\w*QueryResult\b", database_code) or "row_count" in database_code:
        for field in ("truncated", "field_truncated"):
            if not re.search(rf"\bpub\s+{field}\s*:", database_code):
                hit("DB_RESULT_LIMIT_MISSING", f"结果 DTO 缺 {field} 字段（禁止静默截断）")

    # ---- P6) 连接配置落盘必须原子写（A3，F10）----
    for fn in ("save_connections", "save_connections_at", "persist_config"):
        body = rust_fn_body(database_code, fn)
        if body and "atomic_write" not in body:
            hit("DB_PERSIST_NOT_ATOMIC", f"database.rs::{fn} 未走 atomic_write")

    # ---- P7) 超时分层必须复用 30/600/5（A3，M4-1.c §4）----
    if "timeout" in database_code.lower():
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


def _glob_concat(root: Path, pattern: str) -> str:
    parts = [_read(p) for p in sorted((root / "src-tauri/src").glob(pattern)) if p.is_file()]
    return "\n".join(parts)


def _scripts_concat(root: Path) -> str:
    """拼接 `scripts/` 下脚本用于 JDBC 侧车扫描。

    **排除本文件自身**：本夹具的代码里必然出现 `jdbc` 等检测 token，
    把自己纳入扫描会得到恒真告警。
    """
    self_name = Path(__file__).name
    parts: list[str] = []
    for pattern in ("*.py", "*.sh", "*.mjs"):
        for p in sorted((root / "scripts").glob(pattern)):
            if p.is_file() and p.name != self_name:
                parts.append(_read(p))
    return "\n".join(parts)


def read_repo(root: Path) -> dict:
    return {
        "domain_rs": _read(root / "src-tauri/src/domain.rs"),
        "security_rs": _read(root / "src-tauri/src/security_policy.rs"),
        # 实现归 A3：文件可能不存在（→ 空串 → 存在才判）
        "database_rs": _glob_concat(root, "database*.rs"),
        "bridge_rs": _read(root / "src-tauri/src/bridge.rs"),
        "main_rs": _read(root / "src-tauri/src/main.rs"),
        "acl_toml": _read(root / "src-tauri/permissions/default-commands.toml"),
        "cargo_toml": _read(root / "src-tauri/Cargo.toml"),
        "build_rs": _read(root / "src-tauri/build.rs"),
        "scripts_txt": _scripts_concat(root),
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
    let rows = crate::database::query(&app, &conn_id, &sql)?;
    workspace::log_audit(&app, "db.query", format!("conn_id={} rows={} truncated={}",
        conn_id, rows.row_count, rows.truncated));
    Ok(rows)
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

    # 1) 好样本 A：真实仓库（db 命令与 database.rs 尚未落地 → 必须零违规）
    real = detect_hits(read_repo(root))
    if real:
        failures.append(f"真实仓库存在违规（应为空）：{sorted(real)}")

    # 2) 好样本 B：合成参考实现（产物齐备 → 仍必须零违规，证明无误报）
    good = reference_impl()
    ref = detect_hits(good)
    if ref:
        failures.append(f"合成参考实现存在违规（应为空，说明码位误报）：{sorted(ref)}")

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
        "把 tokio 提升为直接依赖",
        mutate(cargo_toml=good["cargo_toml"].replace(
            'rusqlite = { version = "0.32", features = ["bundled"] }',
            'tokio = { version = "1", features = ["full"] }')),
        "cargo_toml",
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

    # ---- PENDING 坏样本（8）----

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
        f"DB_SELF_TEST_RESULT=PASS: 2 好样本零违规（真实仓库 + 合成参考实现）"
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
