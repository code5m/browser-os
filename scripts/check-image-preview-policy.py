#!/usr/bin/env python3
"""Expose the M2-2.b image preview invariants as a reproducible fixture.

契约来源：`logs/checkpoints/M2-2.a-20260903-1352.md` §4（七项冻结契约）与
`logs/checkpoints/M2-2-20260903-1340.md` §6（违规码清单）。守住的是「预览层
不得反向扩张安全边界」：

  通道与边界
  - `assetProtocol.scope` 相对基线不得有任何变化（图片目录已在
    `$HOME/.local/share/**` 内，b 卡严禁扩 scope）
  - 依赖零新增：`package.json` / `package-lock.json` 内容哈希不变
  - 收集侧安全边界不动：`remote-collect.toml` / `injected/collect.js` 哈希不变
  - `save_image` 的来源校验不得被放宽

  命令三处同步
  - `workspace_images_dir` 必须在 `bridge.ts` 封装、进 ACL、进 `invoke_handler`

  前端红线
  - 预览组件不得直接 invoke（唯一出口 `bridge.ts`）
  - 预览组件不得渲染 `source_url` 原文（溯源只出 host）
  - 键盘监听必须经 `createKeyBinder` 且成对解绑
  - 新增预览文件剥离注释后不得含凭据标识符

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

# ---------------- 基线指纹（M2-2.a 冻结时刻，b 卡不得改动这些文件） ----------------
# package.json 依赖集合采用「结构化比对」（见 BASELINE_DEPS / extract_deps），
# 不再用整文件 SHA256，避免 scripts/description 等良性变更误报 IMG_PREV_NPM_DEP_ADDED。
BASELINE_SHA256 = {
    # ea62872：核对 f0733c2→5b4738f→ea62872，仅增加两个本地能力包及 TypeScript 开发依赖。
    # 仍校验整文件，不能以本轮工作树内容自动更新基线。
    "package-lock.json": "097d63bddf24347654d9bfb194714d848590e9d7da7d8f3c26c61d63189c4003",
    "src-tauri/permissions/remote-collect.toml": (
        "cc35e0edc7933c2a43fd6e0c9271667db483aa5aa206d7e08d98a21b832f05da"
    ),
    "src-tauri/injected/collect.js": (
        "6299bdd7108553daded3bb964f6dd14cb01da7e39eb00537a60241c58c0ad40b"
    ),
}

# npm 依赖冻结基线（M2-2.b：之后不得新增前端依赖）。
# 历史：M2-4.e 移除 @tauri-apps/plugin-shell；M5-W20 纳入官方异步 dialog 插件
# (@tauri-apps/plugin-dialog)。仅比较受保护依赖集合，scripts 等变更不再误报。
BASELINE_DEPS = {
    "dependencies": {
        "@lucide/vue": "^1.42.0",
        "@tauri-apps/api": "^2.0.0",
        "@tauri-apps/plugin-dialog": "^2.7.3",
        "@xterm/addon-fit": "^0.11.0",
        "@xterm/xterm": "^6.0.0",
        "dompurify": "^3.4.15",
        "marked": "^18.0.12",
        "pinia": "^2.3.1",
        "turndown": "^7.2.4",
        "vue": "^3.4.0",
    },
    "devDependencies": {
        "@tauri-apps/cli": "^2.0.0",
        "@vitejs/plugin-vue": "^5.0.0",
        "vite": "^5.2.0",
    },
    "optionalDependencies": {},
    "peerDependencies": {},
}

# 受追踪的 npm 清单文件（read_repo 读取用）。
NPM_MANIFESTS = ("package.json", "package-lock.json")
PROTECTED_DEP_FIELDS = ("dependencies", "devDependencies", "optionalDependencies", "peerDependencies")


def extract_deps(pkg_text: str) -> dict:
    """从 package.json 文本结构化提取受保护依赖集合（顺序无关，缺省为空 dict）。"""
    try:
        data = json.loads(pkg_text)
    except Exception:
        return {"__parse_error__": True}
    out: dict = {}
    for field in PROTECTED_DEP_FIELDS:
        deps = data.get(field) or {}
        out[field] = {k: str(v) for k, v in deps.items()}
    return out


def _mutate_pkg(pkg_text: str, **ops) -> str:
    """对 package.json 文本做受控变异，返回新 JSON 文本。"""
    d = json.loads(pkg_text)
    if "add_dep" in ops:
        field, name, ver = ops["add_dep"]
        d.setdefault(field, {})[name] = ver
    if "bump_dep" in ops:
        field, name, ver = ops["bump_dep"]
        d[field][name] = ver
    if "drop_dep" in ops:
        field, name = ops["drop_dep"]
        d[field].pop(name, None)
    if "move_dep" in ops:
        name, src, dst = ops["move_dep"]
        d.setdefault(dst, {})[name] = d[src].pop(name)
    if "add_script" in ops:
        name, cmd = ops["add_script"]
        d.setdefault("scripts", {})[name] = cmd
    if "add_field" in ops:
        name, val = ops["add_field"]
        d[name] = val
    return json.dumps(d, indent=2)

# `tauri.conf.json` 的 assetProtocol.scope 基线（顺序敏感：重排也视为变化）
BASELINE_ASSET_SCOPE = [
    "/usr/share/**",
    "/usr/local/share/**",
    "$HOME/.local/share/**",
    "$HOME/.icons/**",
]

# 本卡新增/改动的预览相关文件（凭据与原文渲染检查范围）
PREVIEW_FILES = (
    "src/utils/imagePreview.ts",
    "src/stores/useImagePreviewStore.ts",
    "src/capabilities/browser/ui/ImageGallery.vue",
    "src/capabilities/browser/ui/ImageLightbox.vue",
)

CREDENTIAL_TOKENS = ("token", "cookie", "authorization", "password", "secret")


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def strip_comments(source: str) -> str:
    """去掉 // 行注释与 /* */ 块注释（前端/Rust 通用近似，够用）。"""
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


def parse_asset_scope(conf_text: str) -> list[str] | None:
    block = re.search(r'"assetProtocol"\s*:\s*\{(.*?)\}', conf_text, re.S)
    if not block:
        return None
    arr = re.search(r'"scope"\s*:\s*\[(.*?)\]', block.group(1), re.S)
    if not arr:
        return None
    return re.findall(r'"([^"]+)"', arr.group(1))


def detect_violations(files: dict[str, str]) -> list[str]:
    v: list[str] = []

    # ---- 1) asset scope 相对基线不得变化（含重排/增删） ----
    scope = parse_asset_scope(files.get("conf", ""))
    if scope is None:
        v.append("IMG_PREV_SCOPE_EXPANDED:assetProtocol.scope 解析失败")
    elif scope != BASELINE_ASSET_SCOPE:
        v.append(f"IMG_PREV_SCOPE_EXPANDED:{scope}")

    # ---- 2) 零新增 npm 依赖（结构化依赖集合比对；scripts 等良性变更不再误报） ----
    for rel in NPM_MANIFESTS:
        got = files.get(rel)
        if got is None:
            v.append(f"IMG_PREV_NPM_DEP_ADDED:{rel} 缺失")
            continue
        if rel == "package.json":
            if extract_deps(got) != BASELINE_DEPS:
                v.append(f"IMG_PREV_NPM_DEP_ADDED:{rel}")
        else:
            if sha256_text(got) != BASELINE_SHA256[rel]:
                v.append(f"IMG_PREV_NPM_DEP_ADDED:{rel}")

    # ---- 3) 收集侧安全边界不得被本卡触碰 ----
    for rel in (
        "src-tauri/permissions/remote-collect.toml",
        "src-tauri/injected/collect.js",
    ):
        got = files.get(rel)
        if got is None:
            v.append(f"IMG_PREV_COLLECT_TOUCHED:{rel} 缺失")
            continue
        if sha256_text(got) != BASELINE_SHA256[rel]:
            # 两个文件分属不同违规码，避免误报混淆
            code = (
                "IMG_PREV_REMOTE_COLLECT_TOUCHED"
                if rel.endswith("remote-collect.toml")
                else "IMG_PREV_COLLECT_TOUCHED"
            )
            v.append(f"{code}:{rel}")

    # ---- 4) save_image 来源校验不得被放宽 ----
    save_body = rust_fn_body(files.get("bridge", ""), "save_image")
    if not save_body:
        v.append("IMG_PREV_SAVE_IMAGE_WIDENED:save_image 缺失")
    elif "check_invocation_source" not in save_body:
        v.append("IMG_PREV_SAVE_IMAGE_WIDENED:check_invocation_source 被删/降级")

    # ---- 5) 新命令三处同步：bridge.ts 封装 / ACL / invoke_handler ----
    cmd = "workspace_images_dir"
    if cmd not in files.get("bridge_ts", ""):
        v.append("IMG_PREV_BRIDGE_WRAP_MISSING:workspace_images_dir")
    if f'"{cmd}"' not in files.get("acl", ""):
        v.append("IMG_PREV_ACL_MISSING:workspace_images_dir")
    if f"bridge::{cmd}" not in files.get("main_rs", ""):
        v.append("IMG_PREV_HANDLER_NOT_REGISTERED:workspace_images_dir")

    # ---- 6) 预览组件不得直接 invoke（唯一出口是 bridge.ts） ----
    # 说明：`src/components/**` 全量扫描；`bridge.ts` 是唯一的 invoke 出口且不在该目录。
    for path, src in files.get("components", {}).items():
        code_only = strip_comments(src)
        if re.search(r"\binvoke\s*[<(]", code_only) and "from \"@tauri-apps/api" in code_only:
            v.append(f"IMG_PREV_DIRECT_INVOKE:{path}")
        if re.search(r"@tauri-apps/api[^\"]*\"[^)]*\binvoke\b", code_only):
            v.append(f"IMG_PREV_DIRECT_INVOKE:{path}")

    # ---- 7) 预览组件不得渲染 source_url 原文（溯源只出 host） ----
    # 范围说明：只查本卡新增的预览文件。`ArtifactPanel.vue` 编辑区原本就展示
    # 成果来源链接（M1 既有行为，非本卡引入），改它属于范围外变更，不在此卡裁决。
    for path, src in files.get("preview_files", {}).items():
        if src is None:
            v.append(f"IMG_PREV_PREVIEW_FILE_MISSING:{path}")
            continue
        for pattern in (
            r"\{\{[^}]*source_url[^}]*\}\}",
            r":src\s*=\s*\"[^\"]*source_url[^\"]*\"",
            r"v-html\s*=\s*\"[^\"]*source_url[^\"]*\"",
        ):
            if re.search(pattern, src):
                v.append(f"IMG_PREV_RAW_URL_RENDERED:{path}")
                break

    # ---- 8) 键盘监听必须经 createKeyBinder 且成对解绑 ----
    for path, src in files.get("preview_files", {}).items():
        if not src or "keydown" not in src:
            continue
        code_only = strip_comments(src)
        if "createKeyBinder" not in code_only:
            v.append(f"IMG_PREV_LISTENER_LEAK:{path}:未使用 createKeyBinder")
            continue
        if "removeEventListener" not in code_only:
            v.append(f"IMG_PREV_LISTENER_LEAK:{path}:缺少解绑调用")
        if "unbind()" not in code_only:
            v.append(f"IMG_PREV_LISTENER_LEAK:{path}:未调用 binder.unbind()")
        if "onBeforeUnmount" not in code_only:
            v.append(f"IMG_PREV_LISTENER_LEAK:{path}:卸载时未解绑")

    # ---- 9) 新增预览文件剥离注释后零凭据标识符 ----
    for path, src in files.get("preview_files", {}).items():
        if not src:
            continue
        code_only = strip_comments(src).lower()
        for bad in CREDENTIAL_TOKENS:
            if re.search(rf"\b{bad}\b", code_only):
                v.append(f"IMG_PREV_CREDENTIAL_TOKEN:{path}:{bad}")

    return v


def read_repo(root: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    for rel in list(BASELINE_SHA256) + list(NPM_MANIFESTS) + [
        "src-tauri/tauri.conf.json",
        "src-tauri/src/bridge.rs",
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
    out["components"] = comps  # type: ignore[assignment]

    preview: dict[str, str] = {}
    for rel in PREVIEW_FILES:
        p = root / rel
        preview[rel] = p.read_text(encoding="utf-8") if p.exists() else ""
    out["preview_files"] = preview  # type: ignore[assignment]
    return out


def prepare(files: dict) -> dict:
    """摊平键名：既保留嵌套的 components/preview_files，也给出简称。"""
    flat = dict(files)
    flat["conf"] = files.get("src-tauri/tauri.conf.json", "")
    flat["bridge"] = files.get("src-tauri/src/bridge.rs", "")
    flat["main_rs"] = files.get("src-tauri/src/main.rs", "")
    flat["acl"] = files.get("src-tauri/permissions/default-commands.toml", "")
    flat["bridge_ts"] = files.get("src/bridge.ts", "")
    return flat


def run_self_test(root: Path) -> int:
    good = read_repo(root)
    violations = detect_violations(prepare(good))
    if violations:
        print("self-test FAIL: 当前仓库自身存在违规（应为空）")
        for x in violations:
            print(f"  x {x}")
        return 1

    samples: list[tuple[str, dict, str]] = []

    def clone() -> dict:
        d = {k: (dict(v) if isinstance(v, dict) else v) for k, v in good.items()}
        return d

    def mutate(**kw) -> dict:
        d = clone()
        for k, val in kw.items():
            if k in ("components", "preview_files"):
                d[k] = {**d[k], **val}
            else:
                d[k] = val
        return d

    # 1. scope 被扩张
    conf = good["src-tauri/tauri.conf.json"].replace(
        '"$HOME/.icons/**"', '"$HOME/.icons/**",\n          "$HOME/**"'
    )
    samples.append(("asset scope 增加 $HOME/**", mutate(**{"src-tauri/tauri.conf.json": conf}),
                    "IMG_PREV_SCOPE_EXPANDED"))
    # 2. scope 条目被改名（等价于"先删后加"，必须被检出）
    renamed = good["src-tauri/tauri.conf.json"].replace(
        '"$HOME/.icons/**"', '"$HOME/.themes/**"'
    )
    samples.append(("asset scope 条目被改名", mutate(**{"src-tauri/tauri.conf.json": renamed}),
                    "IMG_PREV_SCOPE_EXPANDED"))

    # 3. package.json 新增依赖
    pkg = json.loads(good["package.json"])
    pkg["dependencies"]["some-gallery-lib"] = "^1.0.0"
    samples.append(("package.json 新增 npm 依赖",
                    mutate(**{"package.json": json.dumps(pkg, indent=2) + "\n"}),
                    "IMG_PREV_NPM_DEP_ADDED"))
    # 4. package-lock.json 变动
    samples.append(("package-lock.json 被改动",
                    mutate(**{"package-lock.json": good["package-lock.json"] + "\n"}),
                    "IMG_PREV_NPM_DEP_ADDED"))
    # 5. remote-collect.toml 变动
    samples.append(("remote-collect.toml 被改动",
                    mutate(**{"src-tauri/permissions/remote-collect.toml":
                              good["src-tauri/permissions/remote-collect.toml"] + "\n# x\n"}),
                    "IMG_PREV_REMOTE_COLLECT_TOUCHED"))
    # 6. collect.js 变动
    samples.append(("injected/collect.js 被改动",
                    mutate(**{"src-tauri/injected/collect.js":
                              good["src-tauri/injected/collect.js"] + "\n// x\n"}),
                    "IMG_PREV_COLLECT_TOUCHED"))
    # 7. save_image 来源校验被删
    samples.append(("save_image 去掉来源校验",
                    mutate(**{"src-tauri/src/bridge.rs":
                              good["src-tauri/src/bridge.rs"].replace(
                                  'check_invocation_source(&webview, "save_image", None, &app)?;', "")}),
                    "IMG_PREV_SAVE_IMAGE_WIDENED"))
    # 8. bridge.ts 未封装新命令
    samples.append(("bridge.ts 漏封装 workspace_images_dir",
                    mutate(**{"src/bridge.ts": good["src/bridge.ts"].replace(
                        'invoke<string>("workspace_images_dir")', 'invoke<string>("")')}),
                    "IMG_PREV_BRIDGE_WRAP_MISSING"))
    # 9. ACL 漏登记
    samples.append(("ACL 漏掉 workspace_images_dir",
                    mutate(**{"src-tauri/permissions/default-commands.toml":
                              good["src-tauri/permissions/default-commands.toml"].replace(
                                  ',\n    "workspace_images_dir"', "")}),
                    "IMG_PREV_ACL_MISSING"))
    # 10. handler 漏注册
    samples.append(("handler 漏注册 workspace_images_dir",
                    mutate(**{"src-tauri/src/main.rs":
                              good["src-tauri/src/main.rs"].replace(
                                  "            bridge::workspace_images_dir,\n", "")}),
                    "IMG_PREV_HANDLER_NOT_REGISTERED"))
    # 11. 组件直接 invoke
    samples.append(("画廊组件直接 invoke",
                    mutate(components={"src/capabilities/browser/ui/ImageLightbox.vue":
                                       good["preview_files"]["src/capabilities/browser/ui/ImageLightbox.vue"]
                                       + '\nconst x = await invoke("workspace_images_dir");\n'}),
                    "IMG_PREV_DIRECT_INVOKE"))
    # 12. 渲染 source_url 原文
    samples.append(("灯箱渲染 source_url 原文",
                    mutate(preview_files={"src/capabilities/browser/ui/ImageLightbox.vue":
                                          good["preview_files"]["src/capabilities/browser/ui/ImageLightbox.vue"]
                                          + '\n<div>{{ img.source_url }}</div>\n'}),
                    "IMG_PREV_RAW_URL_RENDERED"))
    # 13. keydown 无条件绑定（绕过 createKeyBinder）
    lb = good["preview_files"]["src/capabilities/browser/ui/ImageLightbox.vue"]
    samples.append(("keydown 绕过 createKeyBinder 直接绑定",
                    mutate(preview_files={"src/capabilities/browser/ui/ImageLightbox.vue":
                                          lb.replace("createKeyBinder", "rawBinderHelper")}),
                    "IMG_PREV_LISTENER_LEAK"))
    # 14. 卸载时未解绑
    samples.append(("卸载时未解绑键盘监听",
                    mutate(preview_files={"src/capabilities/browser/ui/ImageLightbox.vue":
                                          lb.replace("onBeforeUnmount", "onMountedTwice")}),
                    "IMG_PREV_LISTENER_LEAK"))
    # 15. 预览文件出现凭据标识符
    samples.append(("预览逻辑层出现 token 标识符",
                    mutate(preview_files={"src/utils/imagePreview.ts":
                                          good["preview_files"]["src/utils/imagePreview.ts"]
                                          + "\nexport const TOKEN = 1;\n"}),
                    "IMG_PREV_CREDENTIAL_TOKEN"))

    # 16. 修改依赖版本
    samples.append(("package.json 修改 vue 版本",
                    mutate(**{"package.json": _mutate_pkg(good["package.json"], bump_dep=("dependencies", "vue", "^3.5.0"))}),
                    "IMG_PREV_NPM_DEP_ADDED"))
    # 17. 删除依赖
    samples.append(("package.json 删除 pinia 依赖",
                    mutate(**{"package.json": _mutate_pkg(good["package.json"], drop_dep=("dependencies", "pinia"))}),
                    "IMG_PREV_NPM_DEP_ADDED"))
    # 18. 依赖在 dependencies/devDependencies 间移动
    samples.append(("package.json 将 vue 从 dependencies 移到 devDependencies",
                    mutate(**{"package.json": _mutate_pkg(good["package.json"], move_dep=("vue", "dependencies", "devDependencies"))}),
                    "IMG_PREV_NPM_DEP_ADDED"))

    # 新增：良性变更不得误报（positive tests）
    good_samples = [
        ("package.json 仅新增 script 不误报",
         mutate(**{"package.json": _mutate_pkg(good["package.json"], add_script=("phase04_probe", "echo ok"))}),
         "IMG_PREV_NPM_DEP_ADDED"),
    ]

    failures = 0
    for label, files, expected in samples:
        found = detect_violations(prepare(files))
        if any(f.startswith(expected) for f in found):
            print(f"  ok  {label} -> {expected}")
        else:
            failures += 1
            print(f"  X   {label} 未检出（期望 {expected}，实际 {found}）")

    for label, files, not_expected in good_samples:
        found = detect_violations(prepare(files))
        if any(f.startswith(not_expected) for f in found):
            failures += 1
            print(f"  X   {label} 误报（不应出现 {not_expected}，实际 {found}）")
        else:
            print(f"  ok  {label} 未误报 {not_expected}")

    if failures:
        print(f"\nself-test: FAIL（{failures} 项未通过）")
        return 1
    print(f"\nself-test: ok（1 好样本 + {len(samples)} 坏样本全部检出 + {len(good_samples)} 良性样本零误报）")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="M2-2.b 图片预览不变量夹具")
    parser.add_argument("--self-test", action="store_true", help="自检检测能力（好/坏样本）")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    if args.self_test:
        return run_self_test(root)

    violations = detect_violations(prepare(read_repo(root)))
    if violations:
        print("image preview policy violations detected:")
        for x in violations:
            print(f"  x {x}")
        return 1
    print("ok (image preview invariants hold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
