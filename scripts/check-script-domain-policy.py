#!/usr/bin/env python3
"""Expose the M2-3 script domain invariants as a reproducible fixture.

契约来源：`logs/checkpoints/M2-3.a-20260903-1604.md`（冻结裁定书）与
`logs/checkpoints/M2-3-20260903-1556.md` §6（违规码清单）。

M2-3 只做「脚本领域与持久化」，**不做执行**。本脚本守住三条底线：

  边界
  - 不得出现 `run_script` / `script_cancel` / `std::process::Command` / shell spawn
    （执行归 M2-4）
  - 不得出现 `ScriptRunRecord` / `RunStatus` / 运行历史落盘（M2-4 才有权定义）
  - `interpreter` 必须是枚举白名单，不得退化为自由字符串
    （自由字符串 = 重新开放任意程序执行，与
    `security_policy::BLOCKED_LAUNCH_PROGRAMS` 冲突）

  路径与落盘
  - 正文读写必须 canonicalize + 前缀校验；元数据必须原子写
  - 写/删命令必须先做 id 形态校验

  命令与隐私
  - 四条命令过 `check_invocation_source`、进 ACL 与 `invoke_handler`
  - 审计 detail 不得含脚本正文、参数默认值、参数值或绝对路径
  - 前端组件不得直接 invoke（唯一出口 `bridge.ts`）
  - 零新增 npm 依赖

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。

关于 `SCR_CREDENTIAL_TOKEN`：本项目必然存在 `secret` / `token` 等**标识符**
（`ScriptParam.secret`、脱敏规则表），因此该码只拦「凭据字面量赋值」
（`token = "xxx"` 形态），不拦标识符本身。
"""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

SCRIPT_COMMANDS = ("script_list", "script_add", "script_update", "script_remove")

# 写/删命令必须做 id 形态校验（读命令无 id 入参）
ID_COMMANDS = ("script_update", "script_remove")

BASELINE_SHA256 = {
    "package.json": "58596ae7f74ad05895f5bff2d83951ebb7c03accf49b15bee9efa232659b5c62",
    "package-lock.json": "024f51edd0f1de2047a55f09ac20b54c059baa9072ae88f01325129582d67ee6",
}

# 审计格式串中禁止出现的片段（脚本正文 / 参数默认值 / 参数值 / 绝对路径）
AUDIT_FORBIDDEN = ("body", "default", "value", "path", "content", "script_body")

# 凭据**字面量赋值**（不拦 `param.secret` 这类标识符）。
# 允许标识符与 `= "..."` 之间夹类型声明（如 `const TOKEN: &str = "x"`），
# 但限定在 40 字符内，避免把整段代码误判进来。
CREDENTIAL_ASSIGN = re.compile(
    r"\b(token|cookie|authorization|passwd|password|secret|api_?key)\b[^;\n]{0,40}=\s*[\"']",
    re.I,
)

# 执行期能力（M2-4 专属，本卡禁止出现）
RUN_FORBIDDEN = (
    "std::process::Command",
    "run_script",
    "script_cancel",
    "Command::new",
)

SHELL_FORBIDDEN = ('sh -c', 'bash -c', "sh\", \"-c", "bash\", \"-c", "spawn(")

# 「运行记录落盘」的越界关键词。
#
# **配套修订（M2-4.b，2026-09-04）**：原列表含 `RunStatus`，但 `M2-4.b-VERDICT §3.5`
# 裁定 `RunStatus` 只是**状态枚举**（非落盘形态），归属 b 卡、合法落 `domain.rs`；
# 真正的落盘形态是 `ScriptRunRecord`（落盘记录结构）与 `script-runs[.json]`
# （落盘文件），归 **M2-4.d**。故从本列表移除 `RunStatus`，保留真正的落盘关键词，
# 以放行 b 卡的合法状态枚举、同时保持对 d 卡落盘的监管。
RUN_RECORD_FORBIDDEN = ("ScriptRunRecord", "script-runs", "script_runs")


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def strip_comments(source: str) -> str:
    without_blocks = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
    return "\n".join(line.split("//", 1)[0] for line in without_blocks.splitlines())


def rust_fn_body(source: str, name: str) -> str:
    match = re.search(rf"\bfn\s+{re.escape(name)}\s*\(", source)
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


def rust_enum_body(source: str, name: str) -> str:
    m = re.search(rf"\benum\s+{re.escape(name)}\b[^{{]*\{{", source)
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
    scripts = files.get("scripts", "")
    bridge = files.get("bridge", "")
    workspace = files.get("workspace", "")
    main_rs = files.get("main_rs", "")
    acl = files.get("acl", "")
    bridge_ts = files.get("bridge_ts", "")
    components = files.get("components", {})
    scripts_code = strip_comments(scripts)

    # ---- 1) DTO 结构性红线：不得承载凭据类字段 ----
    for struct in ("ScriptMeta", "ScriptParam"):
        body = rust_struct_body(domain, struct).lower()
        if not body:
            v.append(f"SCR_DTO_MISSING:{struct}")
            continue
        for bad in ("cookie", "authorization", "set_cookie", "headers", "body"):
            if re.search(rf"\b{bad}\b", body):
                v.append(f"SCR_DTO_SENSITIVE_FIELD:{struct}:{bad}")

    # ---- 2) interpreter 必须是枚举白名单 ----
    if not rust_enum_body(domain, "ScriptInterpreter"):
        v.append("SCR_INTERPRETER_NOT_WHITELISTED:ScriptInterpreter 枚举缺失")
    elif re.search(r"interpreter\s*:\s*String", domain):
        v.append("SCR_INTERPRETER_NOT_WHITELISTED:interpreter 退化为 String")

    # ---- 3) 不得越界定义运行记录（M2-4 专属） ----
    # 配套修订（M2-4.b，2026-09-04）：haystack 改为 strip_comments 后文本。
    # 理由：本检测判定「是否**定义**运行记录」，但 `domain.rs` 的 `RunStatus` 注释
    # 会**提及** d 卡的落盘形态名（`ScriptRunRecord`/`script-runs`），属「提及」而非
    # 「定义」；strip 注释后只保留真实代码，坏样本自检（注入真实
    # `pub struct ScriptRunRecord`）仍命中，好样本（仅注释提及）放行。
    haystack = (
        f"{strip_comments(domain)}\n"
        f"{strip_comments(scripts)}\n"
        f"{strip_comments(workspace)}\n"
        f"{strip_comments(bridge)}"
    )
    for bad in RUN_RECORD_FORBIDDEN:
        if re.search(rf"\b{re.escape(bad)}\b", haystack):
            v.append(f"SCR_RUN_RECORD_PRESENT:{bad}")

    # ---- 4) 纯函数层与脚本命令不得含执行能力 ----
    for bad in RUN_FORBIDDEN:
        if re.search(re.escape(bad), scripts_code):
            v.append(f"SCR_RUN_COMMAND_PRESENT:scripts.rs:{bad}")
    for cmd in SCRIPT_COMMANDS:
        body = rust_fn_body(bridge, cmd)
        if not body:
            v.append(f"SCR_COMMAND_MISSING:{cmd}")
            continue
        for bad in RUN_FORBIDDEN:
            if re.search(re.escape(bad), body):
                v.append(f"SCR_RUN_COMMAND_PRESENT:{cmd}:{bad}")
        for bad in SHELL_FORBIDDEN:
            if bad in body:
                v.append(f"SCR_SHELL_SPAWN_PRESENT:{cmd}:{bad}")

    # ---- 5) 来源校验 + id 形态校验 ----
    for cmd in SCRIPT_COMMANDS:
        body = rust_fn_body(bridge, cmd)
        if body and "check_invocation_source" not in body:
            v.append(f"SCR_SOURCE_CHECK_MISSING:{cmd}")
    for cmd in ID_COMMANDS:
        body = rust_fn_body(bridge, cmd)
        if body and "check_id(" not in body:
            v.append(f"SCR_ID_VALIDATION_MISSING:{cmd}")

    # ---- 6) 命令三处同步：ACL + invoke_handler ----
    for cmd in SCRIPT_COMMANDS:
        if f'"{cmd}"' not in acl:
            v.append(f"SCR_ACL_MISSING:{cmd}")
        if f"bridge::{cmd}" not in main_rs:
            v.append(f"SCR_HANDLER_NOT_REGISTERED:{cmd}")

    # ---- 7) 路径安全：正文读写必须 canonicalize + 前缀校验 ----
    body_path = rust_fn_body(workspace, "body_path_in")
    if not body_path:
        v.append("SCR_PATH_ESCAPE_CHECK_MISSING:body_path_in 缺失")
    else:
        # 根目录与父目录都要 canonicalize：只校验一处会在软链接场景下漏判
        if body_path.count("canonicalize") < 2:
            v.append("SCR_PATH_ESCAPE_CHECK_MISSING:canonicalize")
        if "starts_with" not in body_path:
            v.append("SCR_PATH_ESCAPE_CHECK_MISSING:starts_with")

    # ---- 8) 元数据必须原子写（不倒查既有 repos/bookmarks） ----
    save_body = rust_fn_body(workspace, "save_scripts_at")
    if not save_body:
        v.append("SCR_ATOMIC_WRITE_MISSING:save_scripts_at 缺失")
    elif "atomic_write" not in save_body:
        v.append("SCR_ATOMIC_WRITE_MISSING:save_scripts 未走原子写")

    # ---- 9) 审计不得泄露正文/默认值/参数值/绝对路径 ----
    audit_blocks = re.findall(r'"(script\.[a-z_]+)",\s*format!\((.*?)\),', bridge, re.S)
    if not audit_blocks:
        v.append("SCR_AUDIT_MISSING")
    for name, fmt in audit_blocks:
        low = fmt.lower()
        for bad in AUDIT_FORBIDDEN:
            if bad in low:
                v.append(f"SCR_AUDIT_LEAKS_ARG_VALUE:{name}:{bad}")

    # ---- 10) 前端：组件不得直接 invoke 脚本命令 ----
    for path, src in components.items():
        code_only = strip_comments(src)
        for cmd in SCRIPT_COMMANDS:
            if re.search(rf'invoke[<(]\s*"{cmd}"', code_only):
                v.append(f"SCR_FRONTEND_DIRECT_INVOKE:{path}:{cmd}")
    # bridge.ts 必须封装全部四条
    for cmd in SCRIPT_COMMANDS:
        if f'"{cmd}"' not in bridge_ts:
            v.append(f"SCR_BRIDGE_TS_MISSING:{cmd}")

    # ---- 11) 零新增 npm 依赖 ----
    for rel in ("package.json", "package-lock.json"):
        got = files.get(rel)
        if got is None:
            v.append(f"SCR_NPM_DEP_ADDED:{rel} 缺失")
        elif sha256_text(got) != BASELINE_SHA256[rel]:
            v.append(f"SCR_NPM_DEP_ADDED:{rel}")

    # ---- 12) 凭据字面量赋值（不拦标识符） ----
    for label, src in (("scripts.rs", scripts_code), ("bridge.ts", strip_comments(bridge_ts))):
        for m in CREDENTIAL_ASSIGN.finditer(src):
            v.append(f"SCR_CREDENTIAL_TOKEN:{label}:{m.group(1)}")

    return v


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


def read_repo(root: Path) -> dict:
    out: dict = {}
    for rel in list(BASELINE_SHA256) + [
        "src-tauri/src/domain.rs",
        "src-tauri/src/scripts.rs",
        "src-tauri/src/bridge.rs",
        "src-tauri/src/workspace.rs",
        "src-tauri/src/main.rs",
        "src-tauri/permissions/default-commands.toml",
        "src/bridge.ts",
    ]:
        p = root / rel
        out[rel] = p.read_text(encoding="utf-8") if p.exists() else ""

    comps: dict[str, str] = {}
    for p in sorted((root / "src/components").rglob("*")):
        if p.is_file() and p.suffix in (".vue", ".ts"):
            comps[str(p.relative_to(root))] = p.read_text(encoding="utf-8")

    return {
        "domain": out["src-tauri/src/domain.rs"],
        "scripts": out["src-tauri/src/scripts.rs"],
        "bridge": out["src-tauri/src/bridge.rs"],
        "workspace": out["src-tauri/src/workspace.rs"],
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

    # 1. DTO 塞进凭据字段
    samples.append((
        "ScriptParam 增加 cookie 字段",
        mutate(domain=good["domain"].replace(
            "    pub name: String,\n    /// 前端展示名",
            "    pub cookie: String,\n    pub name: String,\n    /// 前端展示名")),
        "SCR_DTO_SENSITIVE_FIELD"))
    # 2. interpreter 退化为 String
    samples.append((
        "interpreter 退化为自由字符串",
        mutate(domain=good["domain"].replace(
            "    pub interpreter: ScriptInterpreter,", "    pub interpreter: String,")),
        "SCR_INTERPRETER_NOT_WHITELISTED"))
    # 3. ScriptInterpreter 枚举被删
    samples.append((
        "ScriptInterpreter 枚举被删除",
        mutate(domain=re.sub(r"pub enum ScriptInterpreter \{[^}]*\}",
                             "pub struct ScriptInterpreterDummy {}", good["domain"])),
        "SCR_INTERPRETER_NOT_WHITELISTED"))
    # 4. 越界定义运行记录
    samples.append((
        "M2-3 定义 ScriptRunRecord",
        mutate(scripts=good["scripts"] + "\npub struct ScriptRunRecord { pub run_id: String }\n"),
        "SCR_RUN_RECORD_PRESENT"))
    # 5. 纯函数层出现执行能力
    samples.append((
        "scripts.rs 出现 std::process::Command",
        mutate(scripts=good["scripts"] + "\nfn x() { let _ = std::process::Command::new(\"ls\"); }\n"),
        "SCR_RUN_COMMAND_PRESENT"))
    # 6. 脚本命令里出现 shell 拼接
    samples.append((
        "script_add 内出现 sh -c",
        mutate(bridge=good["bridge"].replace(
            "    check_invocation_source(&webview, \"script_add\", None, &app)?;",
            "    check_invocation_source(&webview, \"script_add\", None, &app)?;\n"
            "    let _ = \"sh -c\";")),
        "SCR_SHELL_SPAWN_PRESENT"))
    # 7. 去掉来源校验
    samples.append((
        "script_remove 去掉来源校验",
        mutate(bridge=good["bridge"].replace(
            'check_invocation_source(&webview, "script_remove", None, &app)?;', "")),
        "SCR_SOURCE_CHECK_MISSING"))
    # 8. 去掉 id 形态校验
    samples.append((
        "script_remove 去掉 id 形态校验",
        mutate(bridge=good["bridge"].replace(
            'check_invocation_source(&webview, "script_remove", None, &app)?;\n'
            '    check_id(&id, "脚本 id")?;',
            'check_invocation_source(&webview, "script_remove", None, &app)?;')),
        "SCR_ID_VALIDATION_MISSING"))
    # 9. ACL 漏登记
    samples.append((
        "ACL 漏掉 script_add",
        mutate(acl=good["acl"].replace(',\n    "script_add"', "")),
        "SCR_ACL_MISSING"))
    # 10. handler 漏注册
    samples.append((
        "handler 漏注册 script_list",
        mutate(main_rs=good["main_rs"].replace("            bridge::script_list,\n", "")),
        "SCR_HANDLER_NOT_REGISTERED"))
    # 11. 正文路径去掉前缀校验
    samples.append((
        "body_path_in 去掉前缀校验",
        mutate(workspace=good["workspace"].replace(
            "    if !parent_canon.starts_with(&root_canon) {",
            "    if false {")),
        "SCR_PATH_ESCAPE_CHECK_MISSING"))
    # 12. 正文路径去掉 canonicalize
    samples.append((
        "body_path_in 去掉 canonicalize",
        mutate(workspace=good["workspace"].replace(
            "    let parent_canon = fs::canonicalize(parent)",
            "    let parent_canon = parent.to_path_buf()")),
        "SCR_PATH_ESCAPE_CHECK_MISSING"))
    # 13. 元数据改成非原子写
    samples.append((
        "save_scripts_at 改为非原子写",
        mutate(workspace=good["workspace"].replace(
            "    crate::session::atomic_write(path, &content)",
            "    fs::write(path, &content).map_err(|e| e.to_string())")),
        "SCR_ATOMIC_WRITE_MISSING"))
    # 14. 审计泄露脚本正文
    samples.append((
        "script.add 审计写入正文",
        mutate(bridge=good["bridge"].replace(
            'format!(\n            "id={id} name={} interpreter={:?} param_count={}",',
            'format!(\n            "id={id} name={} interpreter={:?} param_count={} body={body}",')),
        "SCR_AUDIT_LEAKS_ARG_VALUE"))
    # 15. 组件直接 invoke
    gal = "src/components/shared/ImageGallery.vue"
    samples.append((
        "组件直接 invoke script_add",
        mutate(components={gal: good["components"][gal]
                           + '\nconst s = await invoke("script_add", {});\n'}),
        "SCR_FRONTEND_DIRECT_INVOKE"))
    # 16. 新增 npm 依赖
    samples.append((
        "package.json 被改动",
        mutate(**{"package.json": good["package.json"] + "\n"}),
        "SCR_NPM_DEP_ADDED"))
    # 17. 凭据字面量赋值
    samples.append((
        "scripts.rs 出现 token 字面量赋值",
        mutate(scripts=good["scripts"] + '\npub const TOKEN: &str = "abc123";\n'),
        "SCR_CREDENTIAL_TOKEN"))

    def changed(files: dict) -> bool:
        """坏样本必须与好样本**真的不同**。

        变异字符串一旦因重构而失配，`str.replace` 会静默返回原文，
        于是「什么都没测却显示通过」——这是夹具最危险的失效模式，必须拦下。
        """
        if files.keys() != good.keys():
            return True
        for k in good:
            if isinstance(good[k], dict):
                if files[k] != good[k]:
                    return True
            elif files[k] != good[k]:
                return True
        return False

    failures = 0
    for label, files, expected in samples:
        if not changed(files):
            failures += 1
            print(f"  X   {label} 坏样本无效（变异未生效，等于什么都没测）")
            continue
        found = detect_violations(files)
        if any(f.startswith(expected) for f in found):
            print(f"  ok  {label} -> {expected}")
        else:
            failures += 1
            print(f"  X   {label} 未检出（期望 {expected}，实际 {found}）")

    if failures:
        print(f"\nself-test: FAIL（{failures}/{len(samples)} 个坏样本漏检）")
        return 1
    print(f"\nself-test: ok（1 好样本 + {len(samples)} 坏样本全部检出）")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="M2-3 脚本领域不变量夹具")
    parser.add_argument("--self-test", action="store_true", help="自检检测能力（好/坏样本）")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    if args.self_test:
        return run_self_test(root)

    violations = detect_violations(read_repo(root))
    if violations:
        print("script domain policy violations detected:")
        for x in violations:
            print(f"  x {x}")
        return 1
    print("ok (script domain invariants hold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
