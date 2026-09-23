#!/usr/bin/env python3
"""Expose the M5-W14 Plugin Manager UI privacy invariants as a reproducible fixture (Lane A4).

评审口径（PARALLEL_COMMAND_BOARD.md § M5-W14 Hard Stops + Lane A4）：
- UI 只能经冻结的 `src/bridge.ts` 方法调用，禁止裸 `invoke("plugin_*")`。
- 禁止显示/持久化：裸签名、公钥原文（key material）、资源路径、manifest metadata、
  凭据、请求/响应体、stdout/stderr。
- 错误渲染只能展示稳定码（`error_code` 产出），禁止展示原始消息体 / manifest 字段。

判定对象（A6 产物，gated：文件不存在时全部为 no-op，不阻塞批次）：
- `src/capabilities/plugin/state/usePluginStore.ts`
- `src/capabilities/plugin/ui/**`（任意前端源文件）

与既有夹具不重叠：
- `check-plugin-policy.py`（A9）：查 Rust DTO 是否有 `password/secret/token` 字段名。
- `check-plugin-privacy.py`（A4/W13）：查 Rust 后端错误回显 / 审计 detail / 落盘脱敏。
- 本夹具：查**前端渲染/存储层**对冻结红脱敏 DTO 的遵守情况。

用法：
  --self-test        好样本 + 坏样本双向自检（含变异防呆）
  --expect-pending   pending 码位一旦被检出即 FAIL（提示应转默认判定 / 需先修复）
  默认               只判 ACTIVE 码位
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# 已闭环，防回归（A6 落地后即时生效）
ACTIVE_CODES: tuple[str, ...] = (
    # 只能经 bridge.ts 调用，禁止裸 invoke
    "PLUGIN_UI_NO_RAW_INVOKE",
    # 只渲染冻结红脱敏 DTO 字段，禁止渲染密钥/签名/路径字段
    "PLUGIN_UI_NO_SECRET_RENDER",
)

# W14 验收项（A6 落地后应闭环；在 A6 文件存在前为 no-op）
PENDING_CODES: tuple[str, ...] = (
    # 禁止把插件记录/密钥/清单原文落浏览器存储
    "PLUGIN_UI_NO_BROWSER_STORAGE",
    # 错误渲染只用稳定码，禁止原始消息体 / manifest 字段
    "PLUGIN_UI_ERROR_CODE_ONLY",
)

ALL_CODES: tuple[str, ...] = ACTIVE_CODES + PENDING_CODES

# 禁止渲染的字段（来自冻结红脱敏 DTO 已剔除的项 + 通用密钥词）
# 注意：刻意不含 `metadata` —— 安装表单收集用户自有 metadata 输入是允许的，
#       风险在于「渲染/持久化」metadata，由 PLUGIN_UI_NO_BROWSER_STORAGE 兜底。
SECRET_RENDER_TOKENS = (
    r"signature\.value",
    r"resource_path",
    r"rawSignature",
    r"raw_signature",
    r"privateKey",
    r"private_key",
    r"apiKey",
    r"api_key",
    r"secretKey",
    r"secret_key",
)

UI_GLOB_DIRS = (
    ("src/capabilities/plugin/state/usePluginStore.ts", False),
    ("src/capabilities/plugin/ui", True),
)


# ----------------------------- 读取（gated）-----------------------------

def read_ui(root: Path) -> dict[str, str]:
    """返回 A6 前端文件内容；若均不存在，返回空 dict（触发 gated no-op）。"""
    out: dict[str, str] = {}
    for rel, is_dir in UI_GLOB_DIRS:
        p = root / rel
        if is_dir:
            if p.is_dir():
                for f in sorted(p.rglob("*")):
                    if f.is_file() and f.suffix in (".ts", ".tsx", ".vue", ".js", ".jsx"):
                        out[str(f)] = f.read_text(encoding="utf-8", errors="replace")
        else:
            if p.is_file():
                out[rel] = p.read_text(encoding="utf-8", errors="replace")
    return out


# ----------------------------- 判定 -----------------------------

def detect_hits(files: dict[str, str]) -> dict[str, list[str]]:
    if not files:  # gated：A6 文件未落地
        return {}
    hits: dict[str, list[str]] = {}
    src = "\n".join(files.values())
    # 文档注释和瞬时表单输入不等于渲染。只扫描 Vue 模板，避免把登记用
    # pubkey 入参误判为回显。
    render_src = "\n".join(
        re.findall(
            r"<template[\s\S]*?</template>|template\s*:\s*`[^`]*`",
            src,
            flags=re.IGNORECASE,
        )
    )

    def hit(code: str, detail: str) -> None:
        hits.setdefault(code, []).append(detail)

    # ---- A1) 禁止裸 invoke（只能经 bridge.ts 方法）----
    for m in re.finditer(r'invoke\s*\(\s*["\']plugin_', src):
        hit("PLUGIN_UI_NO_RAW_INVOKE", f"裸 invoke 直达 plugin 命令：{m.group(0)[:40]}")
    for m in re.finditer(r'@tauri-apps/api/tauri["\']', src):
        hit("PLUGIN_UI_NO_RAW_INVOKE", "直接 import @tauri-apps/api/tauri（应经 bridge.ts）")

    # ---- A2) 禁止渲染密钥/签名/路径字段 ----
    for tok in SECRET_RENDER_TOKENS:
        for m in re.finditer(tok, render_src, flags=re.IGNORECASE):
            hit("PLUGIN_UI_NO_SECRET_RENDER", f"渲染了禁止字段：{tok}")
    for m in re.finditer(r"{{[^}]*\bpubkey\b[^}]*}}", render_src, flags=re.IGNORECASE):
        hit("PLUGIN_UI_NO_SECRET_RENDER", "模板插值回显 pubkey")

    # ---- P1) 禁止浏览器持久化插件/密钥/清单原文 ----
    for m in re.finditer(r'(localStorage|sessionStorage)\s*\.\s*setItem', src):
        hit("PLUGIN_UI_NO_BROWSER_STORAGE", f"浏览器存储写入：{m.group(0)}")
    for m in re.finditer(r'(indexedDB|localForage|openDatabase)', src):
        hit("PLUGIN_UI_NO_BROWSER_STORAGE", f"浏览器数据库：{m.group(0)}")

    # ---- P2) 错误渲染只能用稳定码，禁止原始消息体 ----
    for m in re.finditer(r'(err|error)\s*\.\s*message', src):
        hit("PLUGIN_UI_ERROR_CODE_ONLY", "渲染了 err.message（原始消息体）")
    for m in re.finditer(r'String\(\s*(err|error)\s*\)', src):
        hit("PLUGIN_UI_ERROR_CODE_ONLY", "渲染了 String(err)（原始消息体）")
    for m in re.finditer(r'(err|error)\s*\.\s*toString\(\s*\)', src):
        hit("PLUGIN_UI_ERROR_CODE_ONLY", "渲染了 err.toString()（原始消息体）")

    return hits


# ----------------------------- 自检 -----------------------------

GOOD_STORE = '''
import { ref } from "vue";
import { bridge } from "../../bridge";
import type { PluginSummary, PluginDetail } from "../../types";

export function usePluginStore() {
  const list = ref<PluginSummary[]>([]);
  const detail = ref<PluginDetail | null>(null);
  const errorCode = ref<string | null>(null);

  async function load() {
    list.value = await bridge.pluginList(null);
  }
  async function inspect(id: string) {
    detail.value = await bridge.pluginGet(id);
  }
  function showError(code: string) {
    errorCode.value = code;
  }
  return { list, detail, errorCode, load, inspect, showError };
}
'''

GOOD_CARD = '''
import { defineComponent } from "vue";
import type { PluginSummary } from "../../types";

export default defineComponent({
  props: { plugin: { type: Object as () => PluginSummary, required: true } },
  template: `<div>
    <span>{{ plugin.id }}</span>
    <span>{{ plugin.display_name }}</span>
    <span>{{ plugin.hash_prefix }}</span>
    <span>{{ plugin.state }}</span>
    <span>{{ plugin.error_code }}</span>
  </div>`,
});
'''


def build_good() -> dict[str, str]:
    return {
        "src/stores/usePluginStore.ts": GOOD_STORE,
        "src/components/plugin/PluginCard.ts": GOOD_CARD,
    }


def mutate(**over: str) -> dict[str, str]:
    good = build_good()
    good.update(over)
    return good


def run_self_test(root: Path) -> int:
    real = read_ui(root)
    good = build_good()

    # 好样本 A：真实仓库（gated → ACTIVE 应为空）
    real_active = [c for c in detect_hits(real) if c in ACTIVE_CODES]
    if real_active:
        print("PLUGIN_UI_PRIVACY_SELF_TEST_RESULT=FAIL")
        print(f"  x 真实仓库 ACTIVE 违规：{sorted(real_active)}")
        return 1

    # 好样本 B：合成参考实现（全部码位零违规）
    good_all = sorted(detect_hits(good))
    if good_all:
        print("PLUGIN_UI_PRIVACY_SELF_TEST_RESULT=FAIL")
        print(f"  x 合成参考实现违规：{good_all}")
        return 1

    bad_samples = [
        (
            "裸 invoke 直达 plugin 命令",
            mutate(**{"src/components/plugin/PluginCard.ts":
                      'import { invoke } from "@tauri-apps/api/tauri";\n'
                      'await invoke("plugin_list", { state: null });'}),
            "PLUGIN_UI_NO_RAW_INVOKE",
        ),
        (
            "直接 import 裸 tauri invoke",
            mutate(**{"src/stores/usePluginStore.ts":
                      'import { invoke } from "@tauri-apps/api/tauri";'}),
            "PLUGIN_UI_NO_RAW_INVOKE",
        ),
        (
            "渲染 signature.value",
            mutate(**{"src/components/plugin/PluginCard.ts":
                      'template: `<span>{{ plugin.signature.value }}</span>`'}),
            "PLUGIN_UI_NO_SECRET_RENDER",
        ),
        (
            "渲染 pubkey 原文",
            mutate(**{"src/components/plugin/PluginCard.ts":
                      'template: `<span>{{ detail.pubkey }}</span>`'}),
            "PLUGIN_UI_NO_SECRET_RENDER",
        ),
        (
            "localStorage 持久化插件详情",
            mutate(**{"src/stores/usePluginStore.ts":
                      'localStorage.setItem("plugin:" + id, JSON.stringify(detail.value));'}),
            "PLUGIN_UI_NO_BROWSER_STORAGE",
        ),
        (
            "indexedDB 持久化密钥",
            mutate(**{"src/stores/usePluginStore.ts":
                      'const db = indexedDB.open("keys");'}),
            "PLUGIN_UI_NO_BROWSER_STORAGE",
        ),
        (
            "渲染 err.message 原始消息体",
            mutate(**{"src/components/plugin/PluginCard.ts":
                      'template: `<span>{{ err.message }}</span>`'}),
            "PLUGIN_UI_ERROR_CODE_ONLY",
        ),
        (
            "渲染 String(err) 原始消息体",
            mutate(**{"src/components/plugin/PluginCard.ts":
                      'const msg = String(err);'}),
            "PLUGIN_UI_ERROR_CODE_ONLY",
        ),
    ]

    failures: list[str] = []
    for name, files, expected in bad_samples:
        found = detect_hits(files)
        if expected not in found:
            failures.append(f"坏样本未检出：{name}（期望 {expected}，实得 {sorted(found)}）")

    if failures:
        print("PLUGIN_UI_PRIVACY_SELF_TEST_RESULT=FAIL")
        for line in failures:
            print(f"  x {line}")
        return 1

    print(
        "PLUGIN_UI_PRIVACY_SELF_TEST_RESULT=PASS: 2 好样本零违规（真实仓库 gated + 合成参考实现）"
        f" + {len(bad_samples)} 个坏样本全部检出（含变异防呆）；"
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
        help="验证 pending 码位未被检出；一旦被检出即应转入默认判定或先修复",
    )
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[1]

    if args.self_test:
        return run_self_test(root)

    hits = detect_hits(read_ui(root))

    if args.expect_pending:
        pending_hits = [c for c in PENDING_CODES if c in hits]
        if pending_hits:
            print("PLUGIN_UI_PRIVACY_PENDING_RESULT=FAIL")
            for code in pending_hits:
                for detail in hits[code]:
                    print(f"  {code}:{detail}")
            return 1
        print(f"PLUGIN_UI_PRIVACY_PENDING_RESULT=NONE（{len(PENDING_CODES)} 个 pending 码位均未检出）")
        return 0

    active_hits = sorted(c for c in hits if c in ACTIVE_CODES)
    if active_hits:
        print("PLUGIN_UI_PRIVACY_POLICY_RESULT=FAIL")
        for code in active_hits:
            for detail in hits[code]:
                print(f"  {code}:{detail}")
        return 1
    print(f"plugin UI privacy policy: all invariants hold（ACTIVE={len(ACTIVE_CODES)}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
