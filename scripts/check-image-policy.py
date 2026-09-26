#!/usr/bin/env python3
"""Expose the M2-1 image domain/persistence invariants as a reproducible fixture.

M2-1 只做「图片领域与持久化」：`ImageRef` / `Artifact.images` / `save_image`
（MIME 白名单、大小与路径限制）。画廊/灯箱/缩放属 M2-2，本卡不实现、也不提前
扩张安全边界（如 `assetProtocol.scope`）。本脚本守住以下不变量：

  后端（Rust）
  - `ImageRef` 结构性不含 cookie / authorization / set-cookie / headers / body
  - `Artifact.images` 必须 `#[serde(default)]`（否则历史成果会被静默丢弃）
  - MIME 白名单为常量表，含 png/jpeg/webp/gif 且**不含 svg**
  - 落盘必须：id 形态校验 + rel_path 白名单 + canonicalize + 前缀校验 + 原子写
  - 溯源 URL 必须经 `redact_sensitive_url`
  - 配额校验必须发生在落盘之前（先验收后写，不留临时文件）
  - 两条命令必须过 `check_invocation_source`，且同步进 ACL 与 invoke_handler
  - 图片审计格式串不得含 URL/token
  - M1-ACCEPT 挂账加固：会话命令（get/delete/export/restore）必须先校验 id 形态

  前端（TS）
  - 除 `bridge.ts` 外不得直接 invoke 图片命令
  - `src/utils/image.ts` 不得出现凭据标识符（展示层零凭据）

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

IMAGE_COMMANDS = ("save_image", "list_artifact_images")

# 需要 id 形态校验的会话命令（M1-ACCEPT NON-BLOCKER 加固项）
SESSION_ID_COMMANDS = (
    "session_get",
    "session_delete",
    "session_export",
    "session_restore",
)

FORBIDDEN_DTO_FIELDS = ("cookie", "authorization", "set_cookie", "headers", "body")

REQUIRED_MIMES = ("image/png", "image/jpeg", "image/webp", "image/gif")


def strip_line_comments(source: str) -> str:
    """去掉 // 行注释（Rust/TS 通用，够用且不追踪字符串，避免复杂化）。"""
    return "\n".join(line.split("//", 1)[0] for line in source.splitlines())


def rust_fn_body(source: str, name: str) -> str:
    """提取 `fn <name>(...) { ... }` 的 body（含花括号）。"""
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


def rust_struct_body(source: str, name: str) -> str:
    match = re.search(rf"\bstruct\s+{re.escape(name)}\b[^{{]*\{{", source)
    if not match:
        return ""
    start = source.find("{", match.start())
    depth = 0
    for i in range(start, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[start : i + 1]
    return source[start:]


def detect_violations(files: dict[str, str]) -> list[str]:
    v: list[str] = []
    domain = files.get("domain", "")
    images = files.get("images", "")
    bridge = files.get("bridge", "")
    main_rs = files.get("main_rs", "")
    acl = files.get("acl", "")
    workspace = files.get("workspace", "")
    types_ts = files.get("types_ts", "")
    bridge_ts = files.get("bridge_ts", "")
    image_ts = files.get("image_ts", "")
    stores = files.get("stores", "")
    conf = files.get("conf", "")

    # ---- 1) ImageRef 结构红线 ----
    dto_body = strip_line_comments(rust_struct_body(domain, "ImageRef")).lower()
    if not dto_body:
        v.append("IMG_DTO_MISSING")
    else:
        for bad in FORBIDDEN_DTO_FIELDS:
            if bad in dto_body:
                v.append(f"IMG_DTO_SENSITIVE_FIELD:{bad}")

    # ---- 2) Artifact.images 必须 #[serde(default)]（历史成果兼容） ----
    # 注释里也会提到 `#[serde(default)]`（说明为何需要它），必须先剥注释再判定
    artifact_body = strip_line_comments(rust_struct_body(domain, "Artifact"))
    has_images_field = re.search(r"pub\s+images\s*:", artifact_body or "") is not None
    if not has_images_field:
        v.append("IMG_ARTIFACT_IMAGES_MISSING")
    elif "serde(default)" not in artifact_body:
        v.append("IMG_SERDE_DEFAULT_MISSING")

    # ---- 3) MIME 白名单常量表（含四类型、不含 SVG） ----
    table = re.search(r"IMAGE_MIME_EXT[^=]*=\s*\[(.*?)\];", domain, re.S)
    if not table:
        v.append("IMG_MIME_TABLE_MISSING")
    else:
        body = table.group(1)
        for mime in REQUIRED_MIMES:
            if mime not in body:
                v.append(f"IMG_MIME_TABLE_INCOMPLETE:{mime}")
        if "svg" in body.lower():
            v.append("IMG_SVG_ALLOWED")

    # ---- 4) 落盘路径安全：id 校验 + rel_path 白名单 + canonicalize + 前缀 + 原子写 ----
    write_body = rust_fn_body(images, "write_image_file")
    if not write_body:
        v.append("IMG_WRITE_FN_MISSING")
    else:
        if "validate_id(" not in write_body:
            v.append("IMG_ID_VALIDATION_MISSING")
        if "validate_rel_path(" not in write_body:
            v.append("IMG_RELPATH_VALIDATION_MISSING")
        if "canonicalize" not in write_body:
            v.append("IMG_CANONICALIZE_MISSING")
        if "starts_with" not in write_body:
            v.append("IMG_PREFIX_CHECK_MISSING")
        if "rename" not in write_body or ".tmp" not in write_body:
            v.append("IMG_ATOMIC_WRITE_MISSING")

    # ---- 5) 溯源 URL 脱敏 ----
    images_code = strip_line_comments(images)
    if "redact_sensitive_url" not in images_code:
        v.append("IMG_URL_REDACTION_MISSING")

    # ---- 6) 配额必须在落盘之前（先验收后写） ----
    save_body = rust_fn_body(workspace, "save_image")
    if not save_body:
        v.append("IMG_SAVE_FN_MISSING")
    else:
        q = save_body.find("check_quota(")
        w = save_body.find("write_image_file(")
        if q < 0:
            v.append("IMG_QUOTA_MISSING")
        elif w >= 0 and q > w:
            v.append("IMG_QUOTA_AFTER_WRITE")

    # ---- 7) 命令：来源校验 + ACL + handler 三处同步 ----
    for cmd in IMAGE_COMMANDS:
        body = rust_fn_body(bridge, cmd)
        if not body:
            v.append(f"IMG_COMMAND_MISSING:{cmd}")
            continue
        if "check_invocation_source" not in body:
            v.append(f"IMG_SOURCE_CHECK_MISSING:{cmd}")
        if f'"{cmd}"' not in acl:
            v.append(f"IMG_ACL_MISSING:{cmd}")
        if f"bridge::{cmd}" not in main_rs:
            v.append(f"IMG_HANDLER_NOT_REGISTERED:{cmd}")

    # ---- 8) 图片审计不得含 URL/凭据 ----
    for m in re.finditer(
        r'log_audit\(\s*&app,\s*"([^"]+)"', bridge
    ):
        pass
    audit_blocks = re.findall(
        r'"(image\.[a-z_]+)",\s*format!\((.*?)\),\s*\);', bridge, re.S
    )
    if not audit_blocks:
        v.append("IMG_AUDIT_MISSING")
    for name, fmt in audit_blocks:
        low = fmt.lower()
        for bad in ("url", "token", "cookie", "authorization", "secret"):
            if bad in low:
                v.append(f"IMG_AUDIT_LEAKS_URL:{name}:{bad}")

    # ---- 9) M1-ACCEPT 挂账加固：会话命令 id 形态校验 ----
    for cmd in SESSION_ID_COMMANDS:
        body = rust_fn_body(bridge, cmd)
        if not body:
            v.append(f"IMG_SESSION_CMD_MISSING:{cmd}")
        elif "check_id(" not in body:
            v.append(f"IMG_SESSION_ID_CHECK_MISSING:{cmd}")

    # ---- 10) 安全边界不得被提前扩张（asset scope 属 M2-2 决策） ----
    scope = re.search(r'"assetProtocol"\s*:\s*\{(.*?)\}', conf, re.S)
    if scope:
        scope_body = scope.group(1)
        # 只禁止放宽。M2-1 不需要 asset:// 读取（字节通道属 M2-2），
        # 因此「未扩 APPDATA」是合法现状；一旦出现宽泛通配即为越界。
        for bad in ('"/**"', '"$HOME/**"', '"$APPCONFIG/**"', '"$APPDATA/**"'):
            if bad in scope_body:
                v.append(f"IMG_ASSET_SCOPE_TOO_BROAD:{bad}")

    # ---- 11) 前端：除 bridge.ts 外不得直接 invoke 图片命令 ----
    for cmd in IMAGE_COMMANDS:
        for name, src in (("types.ts", types_ts), ("stores", stores), ("image.ts", image_ts)):
            if re.search(rf'invoke[<(]\s*"{cmd}"', src):
                v.append(f"IMG_FRONTEND_DIRECT_INVOKE:{name}:{cmd}")
    if image_ts and IMAGE_COMMANDS[0] not in bridge_ts:
        v.append("IMG_BRIDGE_TS_MISSING")

    # ---- 12) 前端展示层零凭据标识符 ----
    code_only = strip_line_comments(image_ts)
    for bad in ("token", "cookie", "authorization", "password", "secret"):
        if re.search(rf"\b{bad}\b", code_only, re.I):
            v.append(f"IMG_FRONTEND_CREDENTIAL_TOKEN:{bad}")

    return v


def read_repo(root: Path) -> dict[str, str]:
    def read(rel: str) -> str:
        p = root / rel
        return p.read_text(encoding="utf-8") if p.exists() else ""

    stores = "\n".join(
        p.read_text(encoding="utf-8")
        for p in (root / "src/stores").glob("*.ts")
        if p.exists()
    )
    return {
        "domain": read("src-tauri/src/domain.rs"),
        "images": read("src-tauri/src/shared/images.rs"),
        "bridge": read("src-tauri/src/bridge.rs"),
        "main_rs": read("src-tauri/src/main.rs"),
        "acl": read("src-tauri/permissions/default-commands.toml"),
        "workspace": read("src-tauri/src/capabilities/workspace/workspace.rs"),
        "types_ts": read("src/types.ts"),
        "bridge_ts": read("src/bridge.ts"),
        "image_ts": read("src/utils/image.ts"),
        "stores": stores,
        "conf": read("src-tauri/tauri.conf.json"),
    }


def run_self_test(root: Path) -> int:
    """1 个好样本（当前仓库）+ 若干坏样本（逐条变异），自检检测能力。"""
    good = read_repo(root)
    violations = detect_violations(good)
    if violations:
        print("self-test FAIL: 当前仓库自身存在违规（应为空）")
        for x in violations:
            print(f"  ✗ {x}")
        return 1

    samples: list[tuple[str, dict[str, str], str]] = []

    def mutate(**kw) -> dict[str, str]:
        files = dict(good)
        files.update(kw)
        return files

    samples.append((
        "ImageRef 增加 cookie 字段",
        mutate(domain=good["domain"].replace(
            "pub mime: String,", "pub cookie: String,\n    pub mime: String,")),
        "IMG_DTO_SENSITIVE_FIELD",
    ))
    samples.append((
        "去掉 Artifact.images 的 serde(default)",
        mutate(domain=good["domain"].replace("    #[serde(default)]\n    pub images", "    pub images")),
        "IMG_SERDE_DEFAULT_MISSING",
    ))
    samples.append((
        "白名单放进 SVG",
        mutate(domain=good["domain"].replace(
            '("image/gif", "gif"),', '("image/gif", "gif"),\n    ("image/svg+xml", "svg"),')),
        "IMG_SVG_ALLOWED",
    ))
    samples.append((
        "落盘去掉 canonicalize",
        mutate(images=good["images"].replace("fs::canonicalize", "identity_path")),
        "IMG_CANONICALIZE_MISSING",
    ))
    samples.append((
        "落盘去掉前缀校验",
        mutate(images=good["images"].replace("starts_with", "eq_ignore_prefix")),
        "IMG_PREFIX_CHECK_MISSING",
    ))
    samples.append((
        "落盘改非原子写（rename 换成 copy）",
        mutate(images=good["images"].replace("fs::rename(&tmp, &target)", "fs::copy(&tmp, &target)")),
        "IMG_ATOMIC_WRITE_MISSING",
    ))
    samples.append((
        "落盘去掉 id 校验（artifact_id 与 image_id 都去掉）",
        mutate(images=good["images"]
               .replace("validate_id(artifact_id)?;", "")
               .replace("validate_id(image_id)?;", "")),
        "IMG_ID_VALIDATION_MISSING",
    ))
    samples.append((
        "去掉 URL 脱敏",
        mutate(images=good["images"].replace("redact_sensitive_url", "passthrough_url")),
        "IMG_URL_REDACTION_MISSING",
    ))
    quota_line = (
        "    crate::images::check_quota(&art.images, prepared.bytes).map_err(|e| e.to_string())?;\n"
    )
    moved_quota = good["workspace"].replace(quota_line, "", 1).replace(
        "    // 7) 回写成果 JSON（只存引用，不存字节）",
        "    // 7) 回写成果 JSON（只存引用，不存字节）\n"
        "    let _ = crate::images::check_quota(&art.images, prepared.bytes);",
        1,
    )
    samples.append((
        "配额挪到落盘之后（先写后验）",
        mutate(workspace=moved_quota),
        "IMG_QUOTA_AFTER_WRITE",
    ))
    samples.append((
        "save_image 去掉来源校验",
        mutate(bridge=good["bridge"].replace(
            'check_invocation_source(&webview, "save_image", None, &app)?;', "")),
        "IMG_SOURCE_CHECK_MISSING:save_image",
    ))
    samples.append((
        "ACL 漏掉 list_artifact_images",
        mutate(acl=good["acl"].replace('    "list_artifact_images"\n', "")),
        "IMG_ACL_MISSING:list_artifact_images",
    ))
    samples.append((
        "handler 漏注册 save_image",
        mutate(main_rs=good["main_rs"].replace("            bridge::save_image,\n", "")),
        "IMG_HANDLER_NOT_REGISTERED:save_image",
    ))
    samples.append((
        "图片审计带 URL",
        mutate(bridge=good["bridge"].replace(
            '"artifact_id={artifact_id} image_id={} bytes={} mime={}",',
            '"artifact_id={artifact_id} image_id={} bytes={} mime={} url={}",')),
        "IMG_AUDIT_LEAKS_URL",
    ))
    samples.append((
        "session_get 去掉 id 校验",
        mutate(bridge=good["bridge"].replace(
            'check_invocation_source(&webview, "session_get", None, &app)?;\n    check_id(&id, "会话 id")?;',
            'check_invocation_source(&webview, "session_get", None, &app)?;')),
        "IMG_SESSION_ID_CHECK_MISSING:session_get",
    ))
    samples.append((
        "asset scope 放宽到 /**",
        mutate(conf=good["conf"].replace('"scope": [', '"scope": [\n          "/**",')),
        "IMG_ASSET_SCOPE_TOO_BROAD",
    ))
    samples.append((
        "前端组件直接 invoke 图片命令",
        mutate(stores=good["stores"] + '\nconst x = invoke("save_image", {});\n'),
        "IMG_FRONTEND_DIRECT_INVOKE",
    ))
    samples.append((
        "展示层出现 token 标识符",
        mutate(image_ts=good["image_ts"] + "\nexport const TOKEN = 1;\n"),
        "IMG_FRONTEND_CREDENTIAL_TOKEN",
    ))

    failures = 0
    for label, files, expected in samples:
        found = detect_violations(files)
        if any(f.startswith(expected) for f in found):
            print(f"  ok  {label} → {expected}")
        else:
            failures += 1
            print(f"  ✗   {label} 未检出（期望 {expected}，实际 {found}）")

    if failures:
        print(f"\nself-test: FAIL（{failures}/{len(samples)} 个坏样本漏检）")
        return 1
    print(f"\nself-test: ok（1 好样本 + {len(samples)} 坏样本全部检出）")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="M2-1 图片领域不变量夹具")
    parser.add_argument("--self-test", action="store_true", help="自检检测能力（好/坏样本）")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    if args.self_test:
        return run_self_test(root)

    files = read_repo(root)
    violations = detect_violations(files)
    if violations:
        print("image policy violations detected:")
        for x in violations:
            print(f"  ✗ {x}")
        return 1
    print("ok (image domain invariants hold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
