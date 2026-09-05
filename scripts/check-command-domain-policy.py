#!/usr/bin/env python3
"""Expose the M2-6.a command snippet contract invariants as a reproducible fixture.

契约来源：`logs/checkpoints/M2-6-20260905-1700.md` §7（冻结条款 F1~F10）与
`logs/checkpoints/B-M2-6.a-command-snippet-contract-20260905-1710.md`。

M2-6.a 只做「命令片段**契约冻结**」——**不做持久化（b）、不接执行（c）、不做前端（d）**。
本脚本守住三条底线：

  形态（与脚本库的结构性差异，见展开卡 §7 F10）
  - 命令以 `argv: Vec<String>` 存储，**不得**退化为 `line: String` 字符串行
    （字符串行需自实现 shell 词法解析，逼近 `M2-4.a §8.1` 的 P0 红线）
  - **无正文文件**：`CommandSnippet` 不得出现 `path` 字段（脚本库才有 `<id>.sh`）

  安全（沿用 `M2-4` 四条 P0 红线）
  - 命令片段定义层（`domain.rs` / `snippets.rs`）不得出现
    `sh -c` / `bash -c` / `Command::new` / `std::process::Command`
  - argv 值不得被单引号包裹（`M2-4.b-VERDICT §3.1`：加引号会把单引号作字面量）
  - `interpreter` 必须是 `ScriptInterpreter` 枚举白名单，不得退化为自由字符串
  - `params` 必须复用 `ScriptParam`（不得自造 `ScriptArg`）

  字段与隐私
  - `dangerous` 为**显式字段**（黑名单推断易被绕过，见 F4）
  - `timeout_secs` 与脚本库同口径（0 = 默认 60，上限 600，见 F8）
  - `cmd.*` 审计 detail 不得含 argv / 参数值明文（命令行可能含 secret 参数）
  - 零新增 npm 依赖

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。

关于「若不存在则跳过」：`snippets.rs` / `snippet_*` / `run_command` / 前端组件在
a 卡阶段尚不存在（分别归 b / c / d 卡），相关检测采用「存在才判」，
故本夹具在 b/c/d 落地后仍长期有效，无需改码位。
"""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

# b/c 卡才实现的命令；a 卡阶段不存在 → 「存在才判」
SNIPPET_COMMANDS = ("snippet_list", "snippet_add", "snippet_update", "snippet_remove")
RUN_COMMANDS = ("run_command",)

# 写/删命令必须做 id 形态校验（读命令无 id 入参）
ID_COMMANDS = ("snippet_update", "snippet_remove", "run_command")

BASELINE_SHA256 = {
    # 与 check-script-domain-policy.py 同源：M2-4.e 刷新后的指纹，
    # 本门禁守住 M2-6 全周期不得新增前端依赖。
    "package.json": "9c567f34a78d5f38a9d347b215282edd0fce81359cb2d77361680cb135599ec4",
    "package-lock.json": "16103828ece143a50ac0203348e1e021de898c7c8cde80d15accdc9f7c13d7c6",
}

# 审计格式串中禁止出现的片段（argv / 参数值 / 命令行明文）
AUDIT_FORBIDDEN = ("argv", "value", "values", "line", "content", "param")

# 凭据**字面量赋值**（不拦 `param.secret` 这类标识符）
CREDENTIAL_ASSIGN = re.compile(
    r"\b(token|cookie|authorization|passwd|password|secret|api_?key)\b[^;\n]{0,40}=\s*[\"']",
    re.I,
)

# 执行期能力：命令片段**定义层**禁止出现（执行接入归 c 卡，且只能在 bridge/script_runner）
RUN_FORBIDDEN = (
    "std::process::Command",
    "Command::new",
    "run_command",
)

SHELL_FORBIDDEN = ('sh -c', 'bash -c', "sh\", \"-c", "bash\", \"-c", "spawn(")

# argv 值被单引号包裹的典型形态（M2-4.b-VERDICT §3.1）
QUOTE_WRAP_SUBSTRINGS = (
    "\"'{}'\"",    # 字面量 "'{}'" —— 单引号包裹占位
    "format!(\"'", # format!("'...'") 形态
)

# 尚未实现的码位（a 卡阶段为空；b/c/d 实现后由对应卡转入默认判定）
PENDING_CODES: frozenset[str] = frozenset()


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def strip_comments(source: str) -> str:
    without_blocks = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
    return "\n".join(line.split("//", 1)[0] for line in without_blocks.splitlines())


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


def detect_violations(files: dict) -> list[str]:
    v: list[str] = []
    domain = files.get("domain", "")
    snippets = files.get("snippets", "")
    bridge = files.get("bridge", "")
    main_rs = files.get("main_rs", "")
    acl = files.get("acl", "")
    bridge_ts = files.get("bridge_ts", "")
    components = files.get("components", {})

    domain_code = strip_comments(domain)
    snippets_code = strip_comments(snippets)

    # ---- 1) 契约类型必须存在 ----
    body = rust_struct_body(domain, "CommandSnippet")
    if not body:
        v.append("CMD_SNIPPET_MISSING:CommandSnippet 结构体缺失")
        return v

    # ---- 2) 形态：不得退化为字符串行（F1）----
    if re.search(r"\bline\s*:\s*String", body):
        v.append("CMD_LINE_STRING_PRESENT:退化为 line: String 字符串行")

    # ---- 3) 形态：argv 必须是 Vec<String>（F1）----
    if not re.search(r"\bargv\s*:\s*Vec<\s*String\s*>", body):
        v.append("CMD_ARGV_TYPE_INVALID:argv 不是 Vec<String>")

    # ---- 4) 形态：无正文文件（F6）----
    if re.search(r"\bpath\s*:", body):
        v.append("CMD_BODY_FILE_PRESENT:CommandSnippet 含 path 字段（无正文文件）")

    # ---- 5) interpreter 必须是枚举白名单 ----
    if not re.search(r"\binterpreter\s*:\s*ScriptInterpreter", body):
        v.append("CMD_INTERPRETER_FREEFORM:interpreter 未使用 ScriptInterpreter 白名单")

    # ---- 6) dangerous 显式字段（F4）----
    if not re.search(r"\bdangerous\s*:\s*bool", body):
        v.append("CMD_DANGEROUS_MISSING:dangerous 显式字段缺失")

    # ---- 7) timeout_secs 与脚本库同口径（F8）----
    if not re.search(r"\btimeout_secs\s*:\s*u32", body):
        v.append("CMD_TIMEOUT_MISSING:timeout_secs 字段缺失")

    # ---- 8) params 复用 ScriptParam（F3）----
    if not re.search(r"\bparams\s*:\s*Vec<\s*ScriptParam\s*>", body):
        v.append("CMD_PARAMS_TYPE_MISUSED:params 未复用 Vec<ScriptParam>")

    # ---- 9) DTO 结构性红线：不得承载凭据类字段 ----
    lowered = body.lower()
    for bad in ("cookie", "authorization", "set_cookie", "headers"):
        if re.search(rf"\b{bad}\b", lowered):
            v.append(f"CMD_DTO_SENSITIVE_FIELD:{bad}")

    # ---- 10) 定义层（domain.rs / snippets.rs）不得含执行能力 ----
    for label, code in (("domain.rs", domain_code), ("snippets.rs", snippets_code)):
        if not code.strip():
            continue
        for bad in RUN_FORBIDDEN:
            if re.search(re.escape(bad), code):
                v.append(f"CMD_RUN_COMMAND_PRESENT:{label}:{bad}")
        for bad in SHELL_FORBIDDEN:
            if bad in code:
                v.append(f"CMD_SHELL_CONCAT_PRESENT:{label}:{bad}")

    # ---- 11) argv 值不得被单引号包裹（§3.1）----
    for label, code in (("domain.rs", domain_code), ("snippets.rs", snippets_code)):
        if not code.strip():
            continue
        for s in QUOTE_WRAP_SUBSTRINGS:
            if s in code:
                v.append(f"CMD_QUOTE_WRAP_PRESENT:{label}")
                break

    # ---- 12) 命令层（b/c 卡）：存在才判 ----
    all_cmds = list(SNIPPET_COMMANDS) + list(RUN_COMMANDS)
    for cmd in all_cmds:
        cmd_body = rust_fn_body(bridge, cmd)
        if not cmd_body:
            continue  # 尚未实现（归 b/c 卡），跳过
        for bad in SHELL_FORBIDDEN:
            if bad in cmd_body:
                v.append(f"CMD_SHELL_CONCAT_PRESENT:{cmd}:{bad}")
        if "check_invocation_source" not in cmd_body:
            v.append(f"CMD_SOURCE_CHECK_MISSING:{cmd}")
        if cmd in ID_COMMANDS and "check_id(" not in cmd_body:
            v.append(f"CMD_ID_VALIDATION_MISSING:{cmd}")
        if f'"{cmd}"' not in acl:
            v.append(f"CMD_ACL_MISSING:{cmd}")
        if f"bridge::{cmd}" not in main_rs:
            v.append(f"CMD_HANDLER_NOT_REGISTERED:{cmd}")

    # ---- 13) cmd.* 审计不得泄露 argv / 参数值（存在才判）----
    for name in ("cmd.run.start", "cmd.validate.reject", "cmd.run.cancel"):
        index = bridge.find(f'"{name}"')
        if index < 0:
            continue
        window = bridge[index : index + 260].lower()
        for bad in AUDIT_FORBIDDEN:
            if bad in window:
                v.append(f"CMD_AUDIT_LEAKS_ARGV:{name}:{bad}")

    # ---- 14) 前端：组件不得直接 invoke（存在才判）----
    for path, src in components.items():
        code_only = strip_comments(src)
        for cmd in all_cmds:
            if re.search(rf'invoke[<(]\s*"{cmd}"', code_only):
                v.append(f"CMD_FRONTEND_DIRECT_INVOKE:{path}:{cmd}")

    # ---- 15) 零新增 npm 依赖 ----
    for rel in ("package.json", "package-lock.json"):
        got = files.get(rel)
        if got is None:
            v.append(f"CMD_NPM_DEP_ADDED:{rel} 缺失")
        elif sha256_text(got) != BASELINE_SHA256[rel]:
            v.append(f"CMD_NPM_DEP_ADDED:{rel}")

    # ---- 16) 凭据字面量赋值（不拦标识符）----
    for label, src in (
        ("domain.rs", domain_code),
        ("snippets.rs", snippets_code),
        ("bridge.ts", strip_comments(bridge_ts)),
    ):
        if not src.strip():
            continue
        for m in CREDENTIAL_ASSIGN.finditer(src):
            v.append(f"CMD_CREDENTIAL_TOKEN:{label}:{m.group(1)}")

    return v


def read_repo(root: Path) -> dict:
    out: dict = {}
    for rel in list(BASELINE_SHA256) + [
        "src-tauri/src/domain.rs",
        "src-tauri/src/snippets.rs",
        "src-tauri/src/bridge.rs",
        "src-tauri/src/main.rs",
        "src-tauri/permissions/default-commands.toml",
        "src/bridge.ts",
    ]:
        p = root / rel
        out[rel] = p.read_text(encoding="utf-8") if p.exists() else ""

    comps: dict[str, str] = {}
    comp_root = root / "src/components"
    if comp_root.exists():
        for p in sorted(comp_root.rglob("*")):
            if p.is_file() and p.suffix in (".vue", ".ts"):
                comps[str(p.relative_to(root))] = p.read_text(encoding="utf-8")

    return {
        "domain": out["src-tauri/src/domain.rs"],
        "snippets": out["src-tauri/src/snippets.rs"],
        "bridge": out["src-tauri/src/bridge.rs"],
        "main_rs": out["src-tauri/src/main.rs"],
        "acl": out["src-tauri/permissions/default-commands.toml"],
        "bridge_ts": out["src/bridge.ts"],
        "components": comps,
        "package.json": out["package.json"],
        "package-lock.json": out["package-lock.json"],
    }


def run_self_test(root: Path) -> int:
    good = read_repo(root)
    violations = detect_violations(good)
    if violations:
        print("self-test FAIL: 当前仓库自身存在违规（应为空）")
        for x in violations:
            print(f"  x {x}")
        return 1

    samples: list[tuple[str, dict, str]] = []

    def mutate(**kw) -> dict:
        d = {k: (dict(v) if isinstance(v, dict) else v) for k, v in good.items()}
        for k, val in kw.items():
            if k == "components":
                d["components"] = {**d["components"], **val}
            else:
                d[k] = val
        return d

    def add(desc: str, mutated: dict, key_before: str, expect_code: str) -> None:
        """登记坏样本，并做**变异防呆**：文本必须真的改动，否则按漏检计。"""
        after = mutated.get(key_before, "")
        before = good.get(key_before, "")
        if after == before:
            print(f"self-test FAIL: 坏样本「{desc}」未改动任何内容（变异失配，按漏检计）")
            sys.exit(1)
        samples.append((desc, mutated, expect_code))

    # 1. 退化为字符串行
    add("argv 退化为 line: String",
        mutate(domain=good["domain"].replace(
            "    pub argv: Vec<String>,", "    pub line: String,")),
        "domain", "CMD_LINE_STRING_PRESENT")

    # 2. 塞进正文文件字段
    add("CommandSnippet 增加 path 字段",
        mutate(domain=good["domain"].replace(
            "    pub category: String,\n    /// 执行 `argv` 的程序",
            "    pub path: String,\n    pub category: String,\n    /// 执行 `argv` 的程序")),
        "domain", "CMD_BODY_FILE_PRESENT")

    # 3. interpreter 退化为自由字符串
    add("interpreter 退化为 String",
        mutate(domain=good["domain"].replace(
            "    pub interpreter: ScriptInterpreter,\n    /// 命令 argv 模板",
            "    pub interpreter: String,\n    /// 命令 argv 模板")),
        "domain", "CMD_INTERPRETER_FREEFORM")

    # 4. 去掉 dangerous 显式字段
    add("去掉 dangerous 字段",
        mutate(domain=good["domain"].replace(
            "    #[serde(default)]\n    pub dangerous: bool,\n", "")),
        "domain", "CMD_DANGEROUS_MISSING")

    # 5. 去掉 timeout_secs
    add("去掉 timeout_secs 字段",
        mutate(domain=good["domain"].replace(
            "    #[serde(default)]\n    pub timeout_secs: u32,\n", "")),
        "domain", "CMD_TIMEOUT_MISSING")

    # 6. params 自造类型
    add("params 改用自定义 ScriptArg",
        mutate(domain=good["domain"].replace(
            "    pub params: Vec<ScriptParam>,", "    pub params: Vec<ScriptArg>,")),
        "domain", "CMD_PARAMS_TYPE_MISUSED")

    # 7. 定义层出现执行能力
    add("domain.rs 出现 std::process::Command",
        mutate(domain=good["domain"] + "\nfn x() { let _ = std::process::Command::new(\"ls\"); }\n"),
        "domain", "CMD_RUN_COMMAND_PRESENT")

    # 8. 定义层出现 shell 拼接
    add("domain.rs 出现 sh -c",
        mutate(domain=good["domain"] + "\nconst BAD: &str = \"sh -c\";\n"),
        "domain", "CMD_SHELL_CONCAT_PRESENT")

    # 9. 结构体被删
    add("CommandSnippet 结构体被删除",
        mutate(domain=re.sub(r"pub struct CommandSnippet \{[^}]*\}",
                             "pub struct CommandSnippetDummy {}", good["domain"])),
        "domain", "CMD_SNIPPET_MISSING")

    # 10. DTO 塞进凭据字段
    add("CommandSnippet 增加 cookie 字段",
        mutate(domain=good["domain"].replace(
            "    /// uuid v4\n    pub id: String,\n    /// 展示名\n    pub name: String,\n"
            "    /// 分类 / 标签（内置取值见 `SNIPPET_BUILTIN_CATEGORIES`，不做白名单约束）\n"
            "    pub category: String,\n    /// 执行 `argv` 的程序",
            "    /// uuid v4\n    pub id: String,\n    /// 展示名\n    pub name: String,\n"
            "    pub cookie: String,\n"
            "    /// 分类 / 标签（内置取值见 `SNIPPET_BUILTIN_CATEGORIES`，不做白名单约束）\n"
            "    pub category: String,\n    /// 执行 `argv` 的程序")),
        "domain", "CMD_DTO_SENSITIVE_FIELD")

    # 11. argv 值被单引号包裹
    add("argv 值被单引号包裹",
        mutate(domain=good["domain"] + "\nfn q(v: &str) -> String { format!(\"'{}'\", v) }\n"),
        "domain", "CMD_QUOTE_WRAP_PRESENT")

    failures = 0
    for desc, mutated, expect in samples:
        got = detect_violations(mutated)
        if not any(x.startswith(expect) for x in got):
            print(f"self-test FAIL: 坏样本「{desc}」未检出 {expect}（实得 {got}）")
            failures += 1

    if failures:
        return 1

    print(f"self-test OK: 好样本零违规 + {len(samples)} 个坏样本全部检出（含变异防呆）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--self-test", action="store_true",
                    help="跑好样本 + 坏样本自检（含变异防呆）")
    ap.add_argument("--expect-pending", action="store_true",
                    help="打印仍处于 pending 的码位（为空则 NONE）")
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[1]

    if args.self_test:
        return run_self_test(root)

    if args.expect_pending:
        if PENDING_CODES:
            for code in sorted(PENDING_CODES):
                print(code)
        else:
            print("NONE")
        return 0

    violations = detect_violations(read_repo(root))
    if violations:
        for x in violations:
            print(x)
        return 1
    print("command snippet domain policy: all invariants hold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
