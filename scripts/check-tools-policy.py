#!/usr/bin/env python3
"""Expose the M2-7/M2-8 tool contract invariants (清单/打包 + 打开/隔离) as a reproducible fixture.

契约来源：logs/checkpoints/M2-7-20260905-1701.md（a 卡 F1~F9）、
logs/checkpoints/B-M2-7.a-tool-manifest-contract-20260905-1800.md、
logs/checkpoints/M2-8-20260905-1900.md（M2-8 a 卡 F1~F9）。

M2-7 覆盖「清单与打包」；M2-8 覆盖「工具箱 UI + 子 webview 打开 + Web 隔离（F1~F9）」。

本脚本守住几条底线（M2-7 形态/打包/ACL + M2-8 打开/隔离）：

  形态（与脚本/命令库的结构性差异）
  - `ToolMeta` 必须存在，且含 id/name/category/source/entry 字段
  - `ToolMeta` **不得**出现 `dangerous` 字段（工具是静态 HTML，无执行风险）
  - `source` 必须是 `ToolSource` 枚举（snake_case：builtin / user），不得退化为自由字符串

  打包（F8）
  - `build.rs` 保持极简：**不得**出现 `include_dir` / `IncludeDir`（禁用）
  - 内置种子必须经 `include_str!` 嵌入（tools.rs 内 `include_str!` 计数 ≥ 5）
  - 5 个种子 HTML 必须存在于 src-tauri/src/tools/

  命令与 ACL（F5）
  - `list_tools` 命令必须存在（tools.rs），且为只读：不得含 `log_audit`
  - `list_tools` 必须在 main.rs 的 generate_handler 中注册（tools::list_tools）
  - `list_tools` 必须在 default-commands.toml 的 commands.allow 中允许，
    且**在 `list_artifact_images` 之前**（末位逗号坑）

  零新增 npm 依赖（M2-7 仅 Rust + 静态 HTML + py 夹具，不改前端依赖）

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# 尚未实现的码位（a 卡阶段为空；M2-8/M2-9 实现后由对应卡转入默认判定）
PENDING_CODES: frozenset[str] = frozenset()

SEED_HTML = (
    "json-tool.html",
    "base64-tool.html",
    "timestamp-tool.html",
    "regex-tool.html",
    "cron-tool.html",
)


def strip_comments(source: str) -> str:
    without_blocks = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
    return "\n".join(line.split("//", 1)[0] for line in without_blocks.splitlines())


def rust_struct_body(source: str, name: str) -> str:
    # 同时匹配 struct / enum（ToolSource 是枚举，ToolMeta 是结构体）
    m = re.search(rf"\b(?:struct|enum)\s+{re.escape(name)}\b[^{{]*\{{", source)
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


def detect_violations(files: dict) -> list[str]:
    v: list[str] = []
    domain = files.get("domain", "")
    tools = files.get("tools", "")
    main_rs = files.get("main_rs", "")
    acl = files.get("acl", "")
    build_rs = files.get("build_rs", "")

    domain_code = strip_comments(domain)

    # ---- 1) ToolMeta 必须存在 ----
    body = rust_struct_body(domain, "ToolMeta")
    if not body:
        v.append("TOOL_META_MISSING:ToolMeta 结构体缺失")
        return v

    # ---- 2) 必备字段 ----
    for field, code in (
        ("id: String", "TOOL_ID_MISSING"),
        ("name: String", "TOOL_NAME_MISSING"),
        ("category: String", "TOOL_CATEGORY_MISSING"),
        ("source: ToolSource", "TOOL_SOURCE_FIELD_MISSING"),
        ("entry: String", "TOOL_ENTRY_MISSING"),
    ):
        if field not in body:
            v.append(f"{code}:ToolMeta 缺字段 {field}")

    # ---- 3) 不得有 dangerous 字段（静态 HTML，无执行风险）----
    if re.search(r"\bdangerous\s*:\s*bool", body):
        v.append("TOOL_DANGEROUS_PRESENT:ToolMeta 含 dangerous 字段（工具无执行风险）")

    # ---- 4) source 必须是 ToolSource 枚举 ----
    if not re.search(r"\bsource\s*:\s*ToolSource", body):
        v.append("TOOL_SOURCE_FREEFORM:source 未使用 ToolSource 枚举")

    # ---- 5) ToolSource 枚举必须存在 + snake_case 变体 ----
    src_body = rust_struct_body(domain, "ToolSource")
    if not src_body:
        v.append("TOOL_SOURCE_ENUM_MISSING:ToolSource 枚举缺失")
    elif "Builtin" not in src_body or "User" not in src_body:
        v.append("TOOL_SOURCE_VARIANTS_MISSING:ToolSource 缺 Builtin/User 变体")

    # ---- 6) build.rs 禁用 include_dir（F8）----
    if "include_dir" in build_rs or "IncludeDir" in build_rs:
        v.append("TOOL_BUILD_INCLUDE_DIR:build.rs 出现 include_dir（F8 禁用）")

    # ---- 7) 内置种子必须经 include_str! 嵌入（tools.rs）----
    n_embed = tools.count("include_str!")
    if n_embed < len(SEED_HTML):
        v.append(f"TOOL_EMBED_MISSING:tools.rs include_str! 计数 {n_embed} < {len(SEED_HTML)}")

    # ---- 8) 5 个种子 HTML 必须存在 ----
    tools_dir = files.get("tools_dir")
    if tools_dir is None:
        v.append("TOOL_SEED_DIR_MISSING:src-tauri/src/tools 目录不可用")
    else:
        for name in SEED_HTML:
            if not (tools_dir / name).is_file():
                v.append(f"TOOL_SEED_HTML_MISSING:种子缺失 {name}")

    # ---- 9) list_tools 命令：存在 + 只读 ----
    cmd_body = rust_fn_body(tools, "list_tools")
    if not cmd_body:
        v.append("TOOL_LIST_CMD_MISSING:tools.rs 缺 list_tools 命令")
    else:
        if "log_audit" in strip_comments(cmd_body):
            v.append("TOOL_LIST_AUDIT_PRESENT:list_tools 不应写审计（只读枚举）")
        # 签名含 AppHandle（函数体内不含，故查全模块源码）
        if "list_tools(app: AppHandle)" not in tools:
            v.append("TOOL_LIST_SIG_INVALID:list_tools 签名异常")

    # ---- 10) ACL + 注册 ----
    if '"list_tools"' not in acl:
        v.append("TOOL_ACL_MISSING:default-commands.toml 未允许 list_tools")
    elif '"list_artifact_images"' in acl:
        # 末位逗号坑：list_tools 必须在 list_artifact_images 之前
        if acl.index('"list_tools"') > acl.index('"list_artifact_images"'):
            v.append("TOOL_ACL_ORDER_WRONG:list_tools 必须在 list_artifact_images 之前")
    if "tools::list_tools" not in main_rs:
        v.append("TOOL_HANDLER_NOT_REGISTERED:main.rs 未注册 tools::list_tools")

    # ===== M2-8 打开与隔离（F1~F9） =====
    # ---- 11) open_tool 命令存在 + 已注册 ----
    if "pub fn open_tool(" not in tools:
        v.append("TOOL_OPEN_CMD_MISSING:tools.rs 缺 open_tool 命令")
    if "tools::open_tool" not in main_rs:
        v.append("TOOL_OPEN_REGISTERED_MISSING:main.rs 未注册 tools::open_tool")

    # ---- 12) tool:// 协议在 Builder 上注册 ----
    if 'register_uri_scheme_protocol("tool"' not in main_rs:
        v.append("TOOL_PROTOCOL_MISSING:main.rs 未注册 tool:// 协议")

    # ---- 13) 用户工具路径越权防御（canonicalize + starts_with）----
    if "canonicalize" not in tools or "starts_with" not in tools:
        v.append("TOOL_PATH_DEFENSE_MISSING:tools.rs 缺 canonicalize/starts_with 越权防御")

    # ---- 14) 安全错误页（锚定函数定义 fn error_page(，避免与调用点混淆）----
    if "fn error_page(" not in tools:
        v.append("TOOL_ERROR_PAGE_MISSING:tools.rs 缺安全错误页")

    # ---- 15) 隔离（F4）：工具窗口（label tool-*）不得出现在任何 capability ----
    caps = files.get("caps", "")
    if "tool-" in caps:
        v.append("TOOL_CAPABILITY_LEAK:capabilities 出现 tool-* 窗口（工具窗口须零能力隔离）")

    return v


def read_repo(root: Path) -> dict:
    paths = {
        "domain": "src-tauri/src/domain.rs",
        "tools": "src-tauri/src/tools.rs",
        "main_rs": "src-tauri/src/main.rs",
        "build_rs": "src-tauri/build.rs",
        "acl": "src-tauri/permissions/default-commands.toml",
    }
    out = {
        k: (root / p).read_text(encoding="utf-8") if (root / p).exists() else ""
        for k, p in paths.items()
    }
    out["tools_dir"] = root / "src-tauri/src/tools"
    # M2-8 隔离：扫描全部 capability，拼接其文本（用于检测 tool-* 窗口是否被授予权限）
    caps_text = ""
    caps_dir = root / "src-tauri/capabilities"
    if caps_dir.is_dir():
        for f in sorted(caps_dir.iterdir()):
            if f.suffix == ".json":
                try:
                    caps_text += f.read_text(encoding="utf-8") + "\n"
                except OSError:
                    pass
    out["caps"] = caps_text
    return out


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
        d = {k: v for k, v in good.items()}
        for k, val in kw.items():
            d[k] = val
        return d

    def add(desc: str, mutated: dict, key_before: str, expect_code: str) -> None:
        before = good.get(key_before, "")
        after = mutated.get(key_before, "")
        if after == before:
            print(f"self-test FAIL: 坏样本「{desc}」未改动任何内容（变异失配，按漏检计）")
            sys.exit(1)
        samples.append((desc, mutated, expect_code))

    # 1. ToolMeta 缺失
    add(
        "ToolMeta 结构体被删除",
        mutate(
            domain=re.sub(
                r"pub struct ToolMeta \{[^}]*\}",
                "pub struct ToolMetaDummy {}",
                good["domain"],
                count=1,
            )
        ),
        "domain",
        "TOOL_META_MISSING",
    )

    # 2. 增加 dangerous 字段
    add(
        "ToolMeta 增加 dangerous 字段",
        mutate(
            domain=good["domain"].replace(
                "    pub entry: String,\n}",
                "    pub entry: String,\n    pub dangerous: bool,\n}",
                1,
            )
        ),
        "domain",
        "TOOL_DANGEROUS_PRESENT",
    )

    # 3. source 退化为自由字符串
    add(
        "source 退化为 String",
        mutate(
            domain=good["domain"].replace(
                "    pub source: ToolSource,", "    pub source: String,", 1
            )
        ),
        "domain",
        "TOOL_SOURCE_FREEFORM",
    )

    # 4. 去掉 entry 字段
    add(
        "去掉 entry 字段",
        mutate(domain=good["domain"].replace("    pub entry: String,\n", "", 1)),
        "domain",
        "TOOL_ENTRY_MISSING",
    )

    # 5. build.rs 引入 include_dir（F8 违规）
    add(
        "build.rs 引入 include_dir",
        mutate(
            build_rs=good["build_rs"]
            + '\nfn _x() { let _ = include_dir!("src/tools"); }\n'
        ),
        "build_rs",
        "TOOL_BUILD_INCLUDE_DIR",
    )

    # 6. 减少 include_str! 嵌入数量（< 5）
    add(
        "tools.rs 减少 include_str! 到 1",
        mutate(
            tools=good["tools"]
            .replace('include_str!("tools/base64-tool.html")', '""', 1)
            .replace('include_str!("tools/timestamp-tool.html")', '""', 1)
            .replace('include_str!("tools/regex-tool.html")', '""', 1)
            .replace('include_str!("tools/cron-tool.html")', '""', 1)
        ),
        "tools",
        "TOOL_EMBED_MISSING",
    )

    # 7. list_tools 写审计（只读被破坏）
    add(
        "list_tools 写入审计",
        mutate(
            tools=good["tools"].replace(
                "pub fn list_tools(app: AppHandle) -> Vec<ToolMeta> {\n    build_tool_list(&app)\n}",
                'pub fn list_tools(app: AppHandle) -> Vec<ToolMeta> {\n'
                '    crate::workspace::log_audit(&app, "tool.list", "x".into());\n'
                "    build_tool_list(&app)\n}",
                1,
            )
        ),
        "tools",
        "TOOL_LIST_AUDIT_PRESENT",
    )

    # 8. ACL 漏配 list_tools
    add(
        "ACL 漏配 list_tools",
        mutate(acl=good["acl"].replace('    "list_tools",\n', "", 1)),
        "acl",
        "TOOL_ACL_MISSING",
    )

    # 9. 注册漏配
    add(
        "main.rs 漏注册 tools::list_tools",
        mutate(main_rs=good["main_rs"].replace("            tools::list_tools,\n", "", 1)),
        "main_rs",
        "TOOL_HANDLER_NOT_REGISTERED",
    )

    # 10. ACL 顺序错误（list_tools 在 list_artifact_images 之后）
    #
    # 变异写法**必须与两者之间的内容无关**：M4 已把 task_* ×5 与 db_* ×3 插在
    # `list_tools` 与 `list_artifact_images` 之间，原「交换相邻两行」的 replace
    # 因此失配为空操作，被变异防呆按漏检判 FAIL（2026-09-06 实测红灯 IF-3）。
    # 改为「摘出 list_tools 条目 → 追加到锚点之后」，中间插多少条命令都不影响。
    anchor = "list_artifact_images"
    acl_reordered = good["acl"]
    tools_entry = '    "list_tools",\n'
    if tools_entry in acl_reordered and anchor in acl_reordered:
        acl_reordered = acl_reordered.replace(tools_entry, "", 1).replace(
            f'    "{anchor}"',
            f'    "{anchor}",\n    "list_tools"',
            1,
        )
    add(
        "ACL 顺序错误：list_tools 晚于 list_artifact_images",
        mutate(acl=acl_reordered),
        "acl",
        "TOOL_ACL_ORDER_WRONG",
    )

    # 11. open_tool 命令被删除
    add(
        "open_tool 命令被删除",
        mutate(
            tools=good["tools"].replace(
                "pub fn open_tool(id: String, app: AppHandle) -> Result<(), String> {",
                "pub fn open_tool_disabled() {",
                1,
            )
        ),
        "tools",
        "TOOL_OPEN_CMD_MISSING",
    )

    # 12. 注册漏配
    add(
        "main.rs 漏注册 tools::open_tool",
        mutate(main_rs=good["main_rs"].replace("            tools::open_tool,\n", "", 1)),
        "main_rs",
        "TOOL_OPEN_REGISTERED_MISSING",
    )

    # 13. tool:// 协议未注册
    add(
        "tool:// 协议未注册",
        mutate(
            main_rs=good["main_rs"].replace(
                'register_uri_scheme_protocol("tool",',
                'register_uri_scheme_protocol_disabled("tool",',
                1,
            )
        ),
        "main_rs",
        "TOOL_PROTOCOL_MISSING",
    )

    # 14. 路径越权防御被破坏（全量替换 + 不含原串子串的 token，避免残留子串漏检）
    add(
        "路径越权防御被破坏",
        mutate(
            tools=good["tools"]
            .replace("canonicalize", "zz_canonical_zz")
            .replace("starts_with", "zz_starts_zz")
        ),
        "tools",
        "TOOL_PATH_DEFENSE_MISSING",
    )

    # 15. 错误页被破坏（锚定函数定义）
    add(
        "错误页被破坏",
        mutate(
            tools=good["tools"].replace(
                "fn error_page(msg: &str) -> String {",
                "fn error_page_renamed(msg: &str) -> String {",
                1,
            )
        ),
        "tools",
        "TOOL_ERROR_PAGE_MISSING",
    )

    # 16. 隔离被破坏：capability 授予 tool-* 窗口
    add(
        "隔离被破坏：capability 出现 tool-* 窗口",
        mutate(
            caps='permissions = [{ "identifier": "default-commands", "windows": ["main", "tool-json"] }]'
        ),
        "caps",
        "TOOL_CAPABILITY_LEAK",
    )

    failures = 0
    for desc, mutated, expect in samples:
        got = detect_violations(mutated)
        if not any(x.startswith(expect) for x in got):
            print(f"self-test FAIL: 坏样本「{desc}」未检出 {expect}（实得 {got}）")
            failures += 1

    if failures:
        return 1

    print(
        f"self-test OK: 好样本零违规 + {len(samples)} 个坏样本全部检出（含变异防呆）"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--self-test", action="store_true", help="跑好样本 + 坏样本自检（含变异防呆）")
    ap.add_argument("--expect-pending", action="store_true", help="打印仍处于 pending 的码位（为空则 NONE）")
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[1]

    if args.self_test:
        return run_self_test(root)

    if args.expect_pending:
        print("NONE" if not PENDING_CODES else "\n".join(sorted(PENDING_CODES)))
        return 0

    violations = detect_violations(read_repo(root))
    if violations:
        for x in violations:
            print(x)
        return 1
    print("tool manifest policy: all invariants hold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
