#!/usr/bin/env python3
"""Expose the M5-1 core boundary invariants as a reproducible fixture.

契约来源：
- `logs/assist/A2-M5-core-20260906-0749.md`（v3）§2.1（四边界与依赖方向硬规则 R-B1~R-B6）、
  §3.2（同 package 双 target + re-export shim 的实测结论）、§3.3（断言规则草案）
- `logs/checkpoints/M5-20260906/M5-1-core-workspace-split.md`（A1 权威卡）§4 FORBID
- `logs/checkpoints/A0-M5-W1-dispatch-20260906-0835.md`（A0 的 W1 指派与硬停止）
- `PARALLEL_COMMAND_BOARD.md` §M5-W1 Implementation Dispatch → W1 Hard Stops

为什么要这个夹具（**编译器守不住，只能靠脚本**）：
M5-1 阶段一采用「同 package 双 target」——`[lib]` 与 `[[bin]]` 共处一个 Cargo package。
Cargo 的 package 级 `[dependencies]` 对 **lib 与 bin 同时生效**（A2 已在 /tmp 最小工程实测：
core 内 `use serde::Serialize` 可直接编译通过）。因此 `tauri` 一旦留在 `[dependencies]`，
core **在编译上完全有能力**引用它——`cargo tree` 与 rustc 都不会报错。
core 边界只能由本夹具在 pre-merge 阶段守住，属**准入前置**而非事后检查。

本夹具守住的底线（默认模式 7 个 ACTIVE 码位，2 个 PENDING）：

  core 不得沾 Tauri（R-B1 / A0 W1 硬停止）
  - `CORE_TAURI_IMPORT`：core 内不得 `use tauri` / `tauri::`
  - `CORE_APP_HANDLE`：core 内不得引用 `AppHandle` / `AppState`

  core 不得反向依赖二进制（R-B2）
  - `CORE_BRIDGE_REF`：core 内不得出现 `crate::bridge` / 任何 bin-only 模块的 `crate::X::` 路径

  不得出现第二执行路径（M2-4 P0 红线 / R-B5）
  - `CORE_SECOND_EXEC_PATH`：core 内不得出现 `std::process::Command` / `Command::new` /
    `sh -c` / `bash -c`

  边界形态自洽（防机械搬运事故）
  - `CORE_LIB_UNDECLARED`：core 目录存在时，`Cargo.toml` 必须声明 `[lib] name = "mvp_core"`
  - `CORE_SHIM_CONFLICT`：模块已搬进 core 后，`main.rs` 不得再保留同名 `mod X;`
    （否则重复定义，且与 re-export shim 冲突）
  - `CORE_EMPTY_PLACEHOLDER`：core 目录不得出现空文件；占位必须含 `STATUS=BLOCKED` 与解锁条件

**「产物存在才判」**：`src-tauri/src/core/` 与 `[lib]` 在 M5-1.a 之前尚不存在，
相关码位自动降级为 no-op，故本夹具在边界建立前后都长期有效，无需改码位。

PENDING 码位（2 个，见 `PENDING_CODES` 注释）：
- `CORE_TREE_TAURI`：`cargo tree -p mvp-core` 不含 tauri。阶段一为同 package，
  `-p` 指向 package 而非 lib target，且 package 必然依赖 tauri → **该判据在阶段一不可能成立**，
  阶段二提升为独立 crate 后才应转 ACTIVE。
- `CORE_DEP_NOT_ALLOWLISTED`：core 依赖白名单。阶段一 lib 与 bin 共享 `[dependencies]`，
  tauri 必须留在其中供 bin 使用 → 同理阶段二才可判。

**诚实声明**：本夹具是**静态**层守门，只能断言「core 源码里不出现这些形态」。
它守不住「core 通过 build.rs / 宏 / 过程宏间接引入 tauri」，也守不住
「行为在搬运中被静默丢弃」——后者由 `cargo test` 的测试总数对齐（T-3/T-13）兜底。
二者缺一不可，不得互相替代。

用法:
  python3 scripts/check-core-boundary.py                 默认扫描（ACTIVE 码位全过 → EXIT 0）
  python3 scripts/check-core-boundary.py --self-test     好样本 + 坏样本双向自检（含变异防呆）
  python3 scripts/check-core-boundary.py --expect-pending 验证 pending 码位仍未实现
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# core 内禁止引用的二进制专属模块（R-B2）。用 `crate::X` 路径形态匹配，避免裸词误报。
BIN_ONLY_MODULES = (
    "bridge",
    "main",
    "terminal",
    "grid_process",
    "tools",
    "shutdown",
)

# 第二执行路径形态（M2-4 P0 红线）
EXEC_PATTERNS = (
    r"std::process::Command",
    r"\bCommand::new\s*\(",
    r"Command::new",
    r"""\bsh\s+-c\b""",
    r"""\bbash\s+-c\b""",
)

# core 依赖白名单（**阶段二独立 crate 后启用**；阶段一因共享 [dependencies] 不可判）
CORE_DEP_ALLOWLIST = (
    "serde",
    "serde_json",
    "keyring",
    "git2",
    "uuid",
    "chrono",
    "sha2",
    "url",
    "urlencoding",
    "open",
    "arboard",
    "portable-pty",
    "freedesktop-icons",
    "strip-ansi-escapes",
    "libc",
    "rusqlite",
    "mysql",
    "postgres",
)

ACTIVE_CODES: tuple[str, ...] = (
    "CORE_TAURI_IMPORT",
    "CORE_APP_HANDLE",
    "CORE_BRIDGE_REF",
    "CORE_SECOND_EXEC_PATH",
    "CORE_LIB_UNDECLARED",
    "CORE_SHIM_CONFLICT",
    "CORE_EMPTY_PLACEHOLDER",
)

# PENDING：阶段一（同 package 双 target）在物理上不可能成立的判据，
# 阶段二（core 提升为独立 crate）后必须转 ACTIVE。
PENDING_CODES: tuple[str, ...] = (
    "CORE_TREE_TAURI",
    "CORE_DEP_NOT_ALLOWLISTED",
)


# ----------------------------- 词法辅助 -----------------------------

_STRING_RE = re.compile(r'"(?:\\.|[^"\\])*"', re.S)
_BLOCK_COMMENT_RE = re.compile(r"/\*.*?\*/", re.S)


def strip_strings_and_comments(source: str) -> str:
    """剥离字符串字面量与注释，避免 `use tauri` 出现在 URL / 文档注释里造成误报。

    `domain.rs:47`、`database.rs:11`、`images.rs:4/404/895`、`session.rs:4`、`scripts.rs:797`
    等处都在注释里解释「不依赖 AppHandle」——不做剥离会直接误报。
    """
    without_strings = _STRING_RE.sub('""', source)
    without_blocks = _BLOCK_COMMENT_RE.sub("", without_strings)
    return "\n".join(line.split("//", 1)[0] for line in without_blocks.splitlines())


def core_module_names(files: dict) -> list[str]:
    """从 core 文件清单推导已搬入 core 的模块名（用于 shim 冲突判定）。"""
    names: list[str] = []
    for rel in files.get("core_files", {}):
        stem = Path(rel).stem
        if stem and stem != "mod":
            names.append(stem)
    # core/mod.rs 里的 `pub mod X;` 也算
    mod = files.get("core_files", {}).get("core/mod.rs", "")
    if mod:
        names.extend(re.findall(r"pub\s+mod\s+([a-z_]+)\s*;", mod))
    return sorted(set(names))


# ----------------------------- 检测 -----------------------------


def detect_hits(files: dict) -> dict[str, list[str]]:
    hits: dict[str, list[str]] = {}

    def add(code: str, detail: str) -> None:
        hits.setdefault(code, []).append(detail)

    core_files: dict[str, str] = files.get("core_files", {}) or {}
    cargo: str = files.get("cargo_toml", "") or ""
    main_rs: str = files.get("main_rs", "") or ""

    # ---- 产物存在才判：core 尚未建立时，core 相关码位自动 no-op ----
    if not core_files:
        return hits

    for rel, raw in sorted(core_files.items()):
        src = strip_strings_and_comments(raw)

        # CORE_EMPTY_PLACEHOLDER：空文件 / 无实质内容，且未声明 STATUS=BLOCKED
        if not raw.strip():
            add("CORE_EMPTY_PLACEHOLDER", f"{rel}: 空文件（board 禁止；占位须含 STATUS=BLOCKED）")
        elif "STATUS=BLOCKED" in raw and "unblock" not in raw.lower():
            add(
                "CORE_EMPTY_PLACEHOLDER",
                f"{rel}: 占位文件缺少 unblock 条件（必须写明解锁条件）",
            )

        # CORE_TAURI_IMPORT
        for m in re.finditer(r"use\s+tauri\b|tauri::", src):
            line = src[: m.start()].count("\n") + 1
            add("CORE_TAURI_IMPORT", f"{rel}:{line}: {m.group(0)!r}")
            break

        # CORE_APP_HANDLE
        for m in re.finditer(r"\bAppHandle\b|\bAppState\b", src):
            line = src[: m.start()].count("\n") + 1
            add("CORE_APP_HANDLE", f"{rel}:{line}: {m.group(0)!r}")
            break

        # CORE_BRIDGE_REF
        for m in re.finditer(r"crate::bridge\b", src):
            line = src[: m.start()].count("\n") + 1
            add("CORE_BRIDGE_REF", f"{rel}:{line}: crate::bridge（逻辑层反向依赖命令层）")
            break
        for name in BIN_ONLY_MODULES:
            if name == "bridge":
                continue  # 已单独处理
            for m in re.finditer(rf"crate::{re.escape(name)}::", src):
                line = src[: m.start()].count("\n") + 1
                add("CORE_BRIDGE_REF", f"{rel}:{line}: crate::{name}::（引用二进制专属模块）")
                break

        # CORE_SECOND_EXEC_PATH
        for pat in EXEC_PATTERNS:
            m = re.search(pat, src)
            if m:
                line = src[: m.start()].count("\n") + 1
                add("CORE_SECOND_EXEC_PATH", f"{rel}:{line}: {m.group(0)!r}")
                break

    # ---- Cargo.toml 形态 ----
    lib_declared = bool(re.search(r"\[lib\]", cargo)) and bool(
        re.search(r'name\s*=\s*"mvp_core"', cargo)
    )
    if not lib_declared:
        add(
            "CORE_LIB_UNDECLARED",
            'Cargo.toml: core 目录已存在，但缺少 [lib] name = "mvp_core"',
        )

    # ---- main.rs shim 冲突：已搬进 core 的模块不得再在 main.rs 里 mod 声明 ----
    main_src = strip_strings_and_comments(main_rs)
    for name in core_module_names(files):
        if re.search(rf"^\s*(?:pub\s+)?mod\s+{re.escape(name)}\s*;", main_src, re.M):
            add(
                "CORE_SHIM_CONFLICT",
                f"main.rs: `mod {name};` 与 core 内同名模块重复（应删 mod 声明，改 re-export shim）",
            )

    return hits


# ----------------------------- 读取仓库 -----------------------------


def read_repo(root: Path) -> dict:
    core_dir = root / "src-tauri" / "src" / "core"
    core_files: dict[str, str] = {}
    if core_dir.is_dir():
        for p in sorted(core_dir.rglob("*.rs")):
            try:
                core_files[str(p.relative_to(core_dir))] = p.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):  # pragma: no cover - 读取失败按空处理
                continue

    def _read(rel: str) -> str:
        p = root / rel
        if not p.is_file():
            return ""
        try:
            return p.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):  # pragma: no cover
            return ""

    return {
        "core_files": core_files,
        "cargo_toml": _read("src-tauri/Cargo.toml"),
        "main_rs": _read("src-tauri/src/main.rs"),
        "lib_rs": _read("src-tauri/src/lib.rs"),
    }


# ----------------------------- 合成样本 -----------------------------

CARGO_GOOD = '''[package]
name = "mvp-browser-os"
version = "0.1.0"
edition = "2021"

[lib]
name = "mvp_core"
path = "src/lib.rs"

[dependencies]
tauri = { version = "2", features = ["unstable", "protocol-asset"] }
serde = { version = "1", features = ["derive"] }
keyring = "3"
rusqlite = { version = "0.40.2", features = ["bundled"] }
mysql = "28.0.2"
postgres = "0.19.14"
'''

MAIN_GOOD = '''mod bridge;
mod domain;
mod scheduler;
mod script_runner;
mod sync;
mod tasks;
mod workspace;

// M5-1 re-export shim：core 内模块对二进制侧仍以 `crate::X::` 可见
pub use mvp_core::keyring_store;

fn main() {}
'''

LIB_GOOD = '''pub mod core;
pub use crate::core::*;
'''

CORE_MOD_GOOD = '''pub mod keyring_store;
'''

# 真实 `keyring_store.rs` 的等价最小实现（零内部依赖、零 Tauri 耦合）
KEYRING_GOOD = '''use keyring::{Entry, Error};

/// 凭据隔离：token 只存系统密钥库，绝不进入日志 / 前端 / 普通文件。
pub struct KeyringStore;

const SERVICE: &str = "com.jizhijiandan.mvp";

impl KeyringStore {
    pub fn save_token(repo_id: &str, token: &str) -> Result<(), String> {
        let entry = Entry::new(SERVICE, repo_id).map_err(|e: Error| e.to_string())?;
        entry.set_password(token).map_err(|e: Error| e.to_string())
    }

    pub fn get_token(repo_id: &str) -> Result<String, String> {
        let entry = Entry::new(SERVICE, repo_id).map_err(|e: Error| e.to_string())?;
        entry
            .get_password()
            .map_err(|e: Error| format!("凭据缺失: {e}（请重新配置仓库）"))
    }
}
'''


def reference_impl() -> dict:
    return {
        "core_files": {
            "mod.rs": CORE_MOD_GOOD,
            "keyring_store.rs": KEYRING_GOOD,
        },
        "cargo_toml": CARGO_GOOD,
        "main_rs": MAIN_GOOD,
        "lib_rs": LIB_GOOD,
    }


# ----------------------------- 自检 -----------------------------


def run_self_test(root: Path) -> int:
    failures: list[str] = []

    # 0) 码位集合自身一致
    overlap = set(ACTIVE_CODES) & set(PENDING_CODES)
    if overlap:
        failures.append(f"码位重复定义（ACTIVE ∩ PENDING）：{sorted(overlap)}")

    good = reference_impl()

    # 1) 好样本 A：合成参考实现（产物齐备 → 必须零违规，证明**无误报**）
    ref = detect_hits(good)
    if ref:
        failures.append(f"合成参考实现存在违规（应为空，说明码位误报）：{sorted(ref)}")

    # 2) 好样本 B：真实仓库（M5-1.a 落地后 core 存在 → 仍必须零违规）
    real = detect_hits(read_repo(root))
    real_active = sorted(c for c in real if c in ACTIVE_CODES)
    if real_active:
        failures.append(f"真实仓库存在违规（应为空）：{real_active}")
        for code in real_active:
            for detail in real[code]:
                failures.append(f"    {code}:{detail}")

    # 3) 注释/字符串不得误报（A2 v3 §1.1 口径：domain.rs:47 等注释命中必须被剥离）
    comment_only = {
        "core_files": {
            "mod.rs": "pub mod demo;\n",
            "demo.rs": (
                "//! 本模块**不依赖 AppHandle**：只吃字节与字符串，便于无 AppHandle 单测。\n"
                'pub const URL: &str = "https://example.com/tauri::scope";\n'
                "// 与 `crate::bridge::allowed_roots` 同口径（仅注释，非引用）\n"
                "pub fn demo() -> u32 {\n    1\n}\n"
            ),
        },
        "cargo_toml": CARGO_GOOD,
        "main_rs": MAIN_GOOD,
        "lib_rs": LIB_GOOD,
    }
    comment_hits = sorted(c for c in detect_hits(comment_only) if c in ACTIVE_CODES)
    if comment_hits:
        failures.append(f"注释/字符串样本误报（应为空，说明词法剥离失效）：{comment_hits}")

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

    # ---- ACTIVE 坏样本（每个码位至少一个）----

    add(
        "core 内 use tauri",
        mutate(
            core_files={
                "mod.rs": CORE_MOD_GOOD,
                "keyring_store.rs": "use tauri::AppHandle;\n" + KEYRING_GOOD,
            }
        ),
        "core_files",
        "CORE_TAURI_IMPORT",
    )

    add(
        "core 内引用 AppState（不经 use tauri）",
        mutate(
            core_files={
                "mod.rs": CORE_MOD_GOOD,
                "keyring_store.rs": KEYRING_GOOD
                + "\npub fn peek(app: &impl std::fmt::Debug) -> String {\n"
                '    let _ = app.state::<AppState>();\n    String::new()\n}\n',
            }
        ),
        "core_files",
        "CORE_APP_HANDLE",
    )

    add(
        "core 内 crate::bridge 反向引用",
        mutate(
            core_files={
                "mod.rs": CORE_MOD_GOOD,
                "keyring_store.rs": KEYRING_GOOD
                + "\npub fn roots(app: &impl std::fmt::Debug) -> Vec<String> {\n"
                "    crate::bridge::allowed_roots(app);\n    Vec::new()\n}\n",
            }
        ),
        "core_files",
        "CORE_BRIDGE_REF",
    )

    add(
        "core 内引用二进制专属模块 crate::tools::",
        mutate(
            core_files={
                "mod.rs": CORE_MOD_GOOD,
                "keyring_store.rs": KEYRING_GOOD
                + "\npub fn list() -> Vec<String> {\n"
                "    crate::tools::manifest()\n}\n",
            }
        ),
        "core_files",
        "CORE_BRIDGE_REF",
    )

    add(
        "core 内出现第二执行路径",
        mutate(
            core_files={
                "mod.rs": CORE_MOD_GOOD,
                "keyring_store.rs": KEYRING_GOOD
                + "\npub fn run() {\n"
                '    std::process::Command::new("sh").arg("-c").arg("echo hi").output();\n'
                "}\n",
            }
        ),
        "core_files",
        "CORE_SECOND_EXEC_PATH",
    )

    add(
        "Cargo.toml 缺少 [lib] name = \"mvp_core\"",
        mutate(cargo_toml=CARGO_GOOD.replace(
            '[lib]\nname = "mvp_core"\npath = "src/lib.rs"\n\n', "")),
        "cargo_toml",
        "CORE_LIB_UNDECLARED",
    )

    add(
        "main.rs 对已搬入 core 的模块仍保留 mod 声明",
        mutate(main_rs=MAIN_GOOD.replace(
            "// M5-1 re-export shim", "mod keyring_store;\n\n// M5-1 re-export shim")),
        "main_rs",
        "CORE_SHIM_CONFLICT",
    )

    add(
        "core 目录出现空文件",
        mutate(core_files={"mod.rs": CORE_MOD_GOOD, "empty.rs": ""}),
        "core_files",
        "CORE_EMPTY_PLACEHOLDER",
    )

    add(
        "占位文件缺少 unblock 条件",
        mutate(
            core_files={
                "mod.rs": CORE_MOD_GOOD,
                "todo.rs": "// STATUS=BLOCKED\n// 待 A0 裁决\n",
            }
        ),
        "core_files",
        "CORE_EMPTY_PLACEHOLDER",
    )

    # ---- 逐样本判定 ----
    for desc, mutated, _key, expect in samples:
        got = detect_hits(mutated)
        if expect not in got:
            failures.append(f"坏样本「{desc}」未检出 {expect}（实际：{sorted(got)}）")

    # ---- PENDING 码位：好样本下不应被检出（它们尚未实现）----
    pending_hits = [c for c in PENDING_CODES if c in ref]
    if pending_hits:
        failures.append(f"pending 码位在好样本下被检出（应未实现）：{pending_hits}")

    if failures:
        print("CORE_POLICY_SELF_TEST=FAIL")
        for f in failures:
            print(f"  {f}")
        return 1

    print(
        f"CORE_POLICY_SELF_TEST=PASS（ACTIVE={len(ACTIVE_CODES)}，"
        f"坏样本={len(samples)}，含注释/字符串阴性样本）"
    )
    return 0


# ----------------------------- 入口 -----------------------------


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument(
        "--self-test",
        action="store_true",
        help="好样本 + 坏样本双向自检（含变异防呆与注释阴性样本）",
    )
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
            print("CORE_PENDING_RESULT=FAIL")
            for code in pending_hits:
                for detail in hits[code]:
                    print(f"  {code}:{detail}")
            return 1
        print(
            f"CORE_PENDING_RESULT=NONE（{len(PENDING_CODES)} 个 pending 码位均未实现，"
            "阶段一同 package 双 target 下预期如此）"
        )
        return 0

    active_hits = sorted(c for c in hits if c in ACTIVE_CODES)
    if active_hits:
        print("CORE_POLICY_RESULT=FAIL")
        for code in active_hits:
            for detail in hits[code]:
                print(f"  {code}:{detail}")
        return 1

    core_count = len(read_repo(root).get("core_files", {}))
    print(
        f"core boundary policy: all invariants hold"
        f"（ACTIVE={len(ACTIVE_CODES)}，core 文件={core_count}）"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
