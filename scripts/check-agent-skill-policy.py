#!/usr/bin/env python3
"""把 M5-4/5 Agent/Skill 的静态安全不变量暴露成可复现的准入夹具。

契约来源（均来自本仓库既有交付，非本脚本发明）：
- `logs/assist/A5-M5-agent-skill-*.md` 系列（A5 M5-4/5 预研 + W1/W3/W4 切片）：
  AGSK_1 执行复用 M2-4（禁第二执行路径）/ AGSK_3 禁内联 shell（K6）/
  AGSK_7 能力白名单单一真源 / K1 ACL 末条恒 `list_artifact_images`。
- `logs/checkpoints/M5-20260906/M5-4-agent-skill-runtime.md` / `M5-5-agent-skill-commands.md`
  （A1 卡）：SkillExec 仅 ScriptRef/CommandRef/Sequence；命令必进 ACL 且末条 K1。
- `src-tauri/src/security_policy.rs`：能力白名单单一真源（SKILL_CAPABILITY_V1 / AGENT_CAPABILITY_V1）。
- `PARALLEL_COMMAND_BOARD.md` §M5-W4 Hard Stops：
  「no second execution path, no installer/network/download」「any new command must be atomic
   with source check, ACL, frontend bridge/types, policy coverage, and tests」。

本脚本定位（对齐 A2 `check-core-boundary.py` / A3 `check-mcp-policy.py` 范式）：
- 它是**准入门禁脚本**，不是 Agent/Skill 运行时代码；只读取既有文件做静态断言。
- 「产物存在才判」：Agent/Skill 域尚未落地时，相关 PENDING 码位自动 no-op，默认扫描
  必然 EXIT 0；`--self-test` 转 DEFAULT 并进 pre-merge.sh。
- 一旦 M5-4.b 执行层 / M5-5 命令开始，把对应 PENDING 码位 gate 去掉、转 ACTIVE 即可。

用法：
  python3 scripts/check-agent-skill-policy.py                 默认扫描（无违规 → EXIT 0；有违规 → EXIT 2）
  python3 scripts/check-agent-skill-policy.py --self-test     好样本 + 坏样本双向自检（含变异防呆）
"""

from __future__ import annotations

import argparse
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _read(path: str) -> str | None:
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            return fh.read()
    except OSError:
        return None


def _rel(path: str) -> str:
    return os.path.relpath(path, ROOT)


def _strip_line_comments(text: str) -> str:
    """去掉 // 行注释，避免 doc 注释里的 InlineScript/RawShell/Command::new 误触发静态码位。"""
    out = []
    for line in text.splitlines():
        idx = line.find("//")
        out.append(line[:idx] if idx >= 0 else line)
    return "\n".join(out)


# ---------------------------------------------------------------------------
# 产物探针：Agent/Skill 域是否已落地（决定 PENDING 码位是否守门）
# ---------------------------------------------------------------------------


def _agent_skill_present(repo: dict[str, str]) -> bool:
    """Agent/Skill 域已开建的依据（任一即真）：
    - `src-tauri/src/domain.rs` 出现 `pub enum SkillDef` / `pub struct SkillDef`
    - `src-tauri/src/security_policy.rs` 出现 `SKILL_CAPABILITY_V1` 常量定义
    - 存在 `src-tauri/src/skills.rs` 或 `src-tauri/src/agent.rs`
    """
    for rel, text in repo.items():
        if rel == "src-tauri/src/domain.rs" and re.search(r"\b(?:pub enum|pub struct)\s+SkillDef\b", text):
            return True
        if rel == "src-tauri/src/security_policy.rs" and re.search(r"\bSKILL_CAPABILITY_V1\s*[=:]", text):
            return True
        if rel in ("src-tauri/src/skills.rs", "src-tauri/src/agent.rs"):
            return True
    return False


# ---------------------------------------------------------------------------
# 码位定义
# 每个码位: (rel, text, repo) -> Optional[list[str]]
#   None          := 不适用 / 干净
#   [detail, ...] := 命中违规
# kind: "ACTIVE"（始终守门） | "PENDING"（仅当 _agent_skill_present 为真时守门）
# ---------------------------------------------------------------------------

_CODE_REX = {
    # 第二执行路径（与 check-core-boundary.py 同源）：skills/agent/domain 不得直起进程
    "AGSK_SECOND_PATH": re.compile(r"std::process|Command::new|\"-\s*c\"|tokio::spawn|tokio::net"),
    # 内联 shell 形态（K6）：SkillExec 枚举不得含 Inline/RawShell 变体
    "AGSK_INLINE_SHELL": re.compile(r"InlineScript|RawShell|InlineShell|inline_shell|raw_shell|ShellInline"),
}


def _is_agent_skill_file(rel: str) -> bool:
    return rel in (
        "src-tauri/src/skills.rs",
        "src-tauri/src/agent.rs",
        "src-tauri/src/domain.rs",
    )


def _is_src_rs(rel: str) -> bool:
    return rel.startswith("src-tauri/src/") and rel.endswith(".rs")


def _acl_commands(toml_text: str) -> list[str]:
    """提取 [commands] 段下的命令名列表（去注释/空行）。"""
    cmds: list[str] = []
    in_commands = False
    for line in toml_text.splitlines():
        s = line.strip()
        if s.startswith("["):
            in_commands = s == "[commands]"
            continue
        if not in_commands:
            continue
        if not s or s.startswith("#"):
            continue
        cmds.append(s)
    return cmds


# ---- ALWAYS-ACTIVE（硬红线，与 Agent/Skill 是否开建无关）----


def c_acl_tail(_rel, text, repo):
    """K1：ACL 末条恒为 `list_artifact_images`（default-commands.toml 最后一条命令）。"""
    if _rel != "src-tauri/permissions/default-commands.toml":
        return None
    cmds = _acl_commands(text)
    if not cmds:
        return None
    if cmds[-1] != "list_artifact_images":
        return [f"ACL 末条不是 list_artifact_images（实际：{cmds[-1]}），违反 K1"]
    return None


def c_capability_drift(rel, text, repo):
    """AGSK_7：SKILL_CAPABILITY_V1 / AGENT_CAPABILITY_V1 各恰好定义一次（单一真源）。"""
    problems = []
    for const in ("SKILL_CAPABILITY_V1", "AGENT_CAPABILITY_V1"):
        defs = [r for r, t in repo.items()
                if r.startswith("src-tauri/src/") and r.endswith(".rs")
                and re.search(r"\b" + const + r"\s*[=:]", t)]
        if len(defs) > 1:
            problems.append(f"{const} 在多处定义（漂移）：{sorted(_rel(d) for d in defs)}")
        # 被引用但零定义
        refs = [r for r, t in repo.items()
                if r.startswith("src-tauri/src/") and r.endswith(".rs") and const in t]
        if refs and not defs:
            problems.append(f"{const} 被引用但无定义（单一真源缺失）")
    return problems or None


# ---- PENDING（仅当 Agent/Skill 域已落地时守门）----


def c_second_path(rel, text, repo):
    if not _agent_skill_present(repo):
        return None
    # 第二执行路径只查 Agent/Skill 解析/运行时模块（skills.rs/agent.rs）；
    # domain.rs 是共享 DTO 容器，含历史 Command::new 注释，不在此扫。
    if rel in ("src-tauri/src/skills.rs", "src-tauri/src/agent.rs"):
        if _CODE_REX["AGSK_SECOND_PATH"].search(_strip_line_comments(text)):
            return ["Agent/Skill 模块出现第二执行路径形态（std::process/Command::new/\"-c\"/tokio::spawn），违反 AGSK_1"]
    return None


def c_inline_shell(rel, text, repo):
    if not _agent_skill_present(repo):
        return None
    if rel == "src-tauri/src/domain.rs" and "pub enum SkillExec" in text:
        if _CODE_REX["AGSK_INLINE_SHELL"].search(_strip_line_comments(text)):
            return ["SkillExec 枚举含内联 shell 变体（InlineScript/RawShell 等），违反 K6"]
    return None


def c_no_inline_exec_variant(rel, text, repo):
    """AGSK_3 补充：skills.rs/agent.rs 不得出现 `InlineScript`/`RawShell` 标识符。"""
    if not _agent_skill_present(repo):
        return None
    if rel in ("src-tauri/src/skills.rs", "src-tauri/src/agent.rs"):
        if _CODE_REX["AGSK_INLINE_SHELL"].search(_strip_line_comments(text)):
            return ["Agent/Skill 解析层引用内联 shell 标识符，违反 K6"]
    return None


def c_command_parity(rel, text, repo):
    """AGSK_2（命令奇偶）：若 handler 暴露 skill_*/agent_* 命令，则 ACL 与 bridge.ts 必须同时出现。"""
    if not _agent_skill_present(repo):
        return None
    if rel not in ("src-tauri/src/bridge.rs", "src-tauri/src/main.rs"):
        return None
    # 仅匹配 M5-5 规定的 15 条 Agent/Skill 命令（避免误伤 A4 的 agent_memory 等其它 agent_* 命令）
    _M5_5_CMDS = (
        "skill_install", "skill_remove", "skill_list", "skill_get", "skill_run",
        "skill_cancel", "skill_runs", "agent_install", "agent_remove", "agent_list",
        "agent_get", "agent_chat", "agent_chat_cancel", "agent_chat_history", "agent_runs",
    )
    skill_cmds = set(re.findall(r"\b(?:" + "|".join(_M5_5_CMDS) + r")\b", text))
    if not skill_cmds:
        return None
    acl = repo.get("src-tauri/permissions/default-commands.toml", "")
    bts = repo.get("src/bridge.ts", "")
    missing = [c for c in skill_cmds if c not in acl or c not in bts]
    if missing:
        return [f"Agent/Skill 命令缺 ACL/bridge.ts 奇偶：{sorted(missing)}"]
    return None


_READONLY_CMDS = (
    "agent_parse", "agent_validate", "agent_permission_preview",
    "skill_parse", "skill_validate", "skill_permission_preview",
)

def c_readonly_command_parity(rel, text, repo):
    """AGSK_RO_COMMAND_PARITY：W7 只读 Agent/Skill 桥命令须在 bridge.rs handler + main.rs 注册 + ACL + bridge.ts 四件套齐全。

    与 c_command_parity（M5-5 运行时命令）互补：覆盖 W7 新增的「只读 parse/validate/permission_preview」子集。
    仅在任一只读命令已出现时才守门（presence-gated），故 Agent/Skill 域未落地或 W7 未开工时自动 no-op。
    """
    if rel != "src-tauri/src/main.rs":
        return None
    present = any(
        c in repo.get("src-tauri/src/bridge.rs", "")
        or c in repo.get("src-tauri/src/main.rs", "")
        or c in repo.get("src/bridge.ts", "")
        or c in repo.get("src-tauri/permissions/default-commands.toml", "")
        for c in _READONLY_CMDS
    )
    if not present:
        return None
    bridge_rs = repo.get("src-tauri/src/bridge.rs", "")
    main_rs = repo.get("src-tauri/src/main.rs", "")
    acl = repo.get("src-tauri/permissions/default-commands.toml", "")
    bts = repo.get("src/bridge.ts", "")
    missing = []
    for c in _READONLY_CMDS:
        in_handler = (f"bridge::{c}" in bridge_rs) or (f"pub fn {c}" in bridge_rs)
        has_source_check = f'check_invocation_source(&webview, "{c}"' in bridge_rs
        parity_ok = in_handler and c in main_rs and c in acl and c in bts
        if not (parity_ok and has_source_check):
            missing.append(c)
    if missing:
        return [f"W7 只读 Agent/Skill 命令缺四件套奇偶或 source check（bridge.rs handler/main.rs/ACL/bridge.ts/source-check）：{sorted(missing)}"]
    return None


def c_credential_not_echoed(rel, text, repo):
    """AGSK_CREDENTIAL_NOT_ECHOED（W8 S-W8-1，A10 加）：PolicyError::CredentialLeak
    的 Display 不得回显 secret 原文（{s}）。validate 拒后错误只给分类信息，不回显密文。

    设计来源：A4 W8 隐私评审 F-W8-1 —— agent_validate/skill_validate 在 CredentialLeak
    错误里明文回显 system_prompt/description/name（security_policy.rs Display 写 "{s}"；
    agent.rs/skills.rs clone secret 进错误），前端日志/剪贴/缓存即捕获 → 击穿 W8 脱敏镜头 +
    「no prompt-secret logging or persistence」精神。修复后 Display 用 <redacted> 或不插值。
    """
    if not _agent_skill_present(repo):
        return None
    if rel != "src-tauri/src/security_policy.rs":
        return None
    if re.search(r"CredentialLeak\(\w+\)\s*=>\s*\{[^}]{0,400}?\{s\}", text):
        return ["PolicyError::CredentialLeak 的 Display 回显 secret 原文（{s}），违反 W8 脱敏镜头；应改为 <redacted> 或不插值捕获变量"]
    return None


# Agent/Skill **执行类**命令（M5-5 运行时命令中真正带副作用/执行语义的子集）。
# 注意：不含 `skill_list`/`agent_list`（只读列举），执行面判据只看这些。
_EXEC_CMDS = (
    "skill_run", "skill_cancel", "skill_runs_list",
    "agent_chat", "agent_chat_cancel", "agent_runs",
)


def c_exec_locked(rel, text, repo):
    """AGSK_EXEC_LOCKED（W11 · A5 加）：Agent/Skill **执行面解锁**的前置门禁。

    背景（本轮实测发现，升级 A10 的 S-W8-3）：
      `src/bridge.ts` 已存在 `skillRun`/`agentChat`/`agentRunCancel`/`skillRunsList` 等
      **执行类**前端 wrapper（invoke `skill_run` / `agent_chat` / `agent_chat_cancel` /
      `skill_runs_list`），但 `main.rs` 只注册了 W7 只读六件套，`default-commands.toml`
      中 `skill_*`/`agent_*` 条目为 0 —— 即「前端已声明执行能力、后端无 handler」。
      当前不致命（invoke 会 command-not-found 失败），但 A10 W10 记录的 S-W8-3
      只写了 `skill_list`/`agent_list`（只读），**未覆盖执行类 wrapper**，属描述不全。

    本码位把「执行解锁」变成机器可判事件：一旦 `main.rs` 注册了任一执行类命令，
    即视为执行面已开，则该命令**必须**同时出现在 ACL 与 bridge.ts（四件套最小集），
    否则判违规 —— 防止「执行先上线、ACL/前端门禁后补」的窗口期裸奔，
    并落实 W11 Hard Stop「no real agent/skill execution」。

    门控：仅当 main.rs 出现执行类命令时才守门；当前执行未解锁 → no-op，
    默认扫描保持 PASS（不引入新的红灯、不影响既有 ACTIVE/PENDING 计数语义）。
    """
    if rel != "src-tauri/src/main.rs":
        return None
    acl = repo.get("src-tauri/permissions/default-commands.toml", "")
    bts = repo.get("src/bridge.ts", "")
    missing = [c for c in _EXEC_CMDS if c in text and (c not in acl or c not in bts)]
    if missing:
        return [f"Agent/Skill 执行类命令已在 main.rs 注册但缺 ACL/bridge.ts（执行解锁须先补齐门禁，违反 AGSK 执行锁定）：{sorted(missing)}"]
    return None


ACTIVE_CODES = [
    ("AGSK_ACL_TAIL", "ACTIVE", c_acl_tail),
    ("AGSK_CAPABILITY_DRIFT", "ACTIVE", c_capability_drift),
    ("AGSK_RO_COMMAND_PARITY", "ACTIVE", c_readonly_command_parity),
]
PENDING_CODES = [
    ("AGSK_SECOND_PATH", "PENDING", c_second_path),
    ("AGSK_INLINE_SHELL_ENUM", "PENDING", c_inline_shell),
    ("AGSK_INLINE_SHELL_REF", "PENDING", c_no_inline_exec_variant),
    ("AGSK_COMMAND_PARITY", "PENDING", c_command_parity),
    ("AGSK_CREDENTIAL_NOT_ECHOED", "PENDING", c_credential_not_echoed),
    ("AGSK_EXEC_LOCKED", "PENDING", c_exec_locked),
]
ALL_CODES = ACTIVE_CODES + PENDING_CODES


# ---------------------------------------------------------------------------
# 扫描
# ---------------------------------------------------------------------------


def _scan_real_repo() -> dict[str, str]:
    repo: dict[str, str] = {}
    want = [
        "src-tauri/Cargo.toml",
        "src-tauri/permissions/default-commands.toml",
        "src/bridge.ts",
        "src/types.ts",
    ]
    for w in want:
        t = _read(os.path.join(ROOT, w))
        if t is not None:
            repo[w] = t
    base = os.path.join(ROOT, "src-tauri/src")
    if os.path.isdir(base):
        for root, _dirs, files in os.walk(base):
            for fn in files:
                if fn.endswith(".rs"):
                    full = os.path.join(root, fn)
                    t = _read(full)
                    if t is not None:
                        repo[os.path.relpath(full, ROOT)] = t
    return repo


def detect_hits(repo: dict[str, str]) -> dict[str, list[str]]:
    hits: dict[str, list[str]] = {}
    for name, _kind, fn in ALL_CODES:
        for rel, text in repo.items():
            res = fn(rel, text, repo)
            if res:
                hits.setdefault(name, []).extend(res)
    return hits


# ---------------------------------------------------------------------------
# 自测
# ---------------------------------------------------------------------------


def _baseline_repo() -> dict[str, str]:
    # 基线含 Agent/Skill 域（W4 已落地）：SkillDef/SkillExec 类型 + 能力单源常量，
    # 且无第二执行路径 / 无内联 / ACL 末条 K1 / 无命令奇偶缺口。
    return {
        "src-tauri/Cargo.toml": "[package]\nname = \"mvp-browser-os\"\ntauri = \"2\"\n",
        "src-tauri/src/domain.rs": (
            "pub enum AclLevel { Safe, Confirm, Dangerous }\n"
            "pub enum SkillExec {\n"
            "  ScriptRef { script_id: String, params: serde_json::Value },\n"
            "  CommandRef { command_id: String, params: serde_json::Value },\n"
            "  Sequence { steps: Vec<SkillExec> },\n"
            "}\n"
            "pub struct SkillDef { id: String, acl: AclLevel, exec: SkillExec }\n"
            "pub struct AgentDef { id: String, system_prompt: String }\n"
        ),
        "src-tauri/src/skills.rs": (
            "use crate::domain::SkillDef;\n"
            "impl SkillDef { pub fn validate(&self) -> Result<(), PolicyError> { Ok(()) } }\n"
        ),
        "src-tauri/src/agent.rs": (
            "use crate::domain::AgentDef;\n"
            "impl AgentDef { pub fn validate(&self) -> Result<(), PolicyError> { Ok(()) } }\n"
        ),
        "src-tauri/src/security_policy.rs": (
            "pub const SKILL_CAPABILITY_V1: &[&str] = &[];\n"
            "pub const AGENT_CAPABILITY_V1: &[&str] = &[];\n"
            "enum PolicyError { CredentialLeak(String) }\n"
            "impl std::fmt::Display for PolicyError {\n"
            "  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {\n"
            "    match self {\n"
            "      PolicyError::CredentialLeak(_s) => {\n"
            "        write!(f, \"凭据/密钥泄露（禁止进入 Agent/Skill 定义）：<redacted>\")\n"
            "      }\n"
            "      _ => Ok(()),\n"
            "    }\n"
            "  }\n"
            "}\n"
        ),
        "src-tauri/permissions/default-commands.toml": "[commands]\nlist_artifacts\nlist_artifact_images\n",
        "src/bridge.ts": "export function invoke(name: string) {}\n",
        "package.json": '{"dependencies":{"react":"^18"}}\n',
    }


def _run_self_test() -> int:
    failures: list[str] = []
    overlap = set(n for n, _, _ in ACTIVE_CODES) & set(n for n, _, _ in PENDING_CODES)
    if overlap:
        failures.append(f"码位重复定义（ACTIVE ∩ PENDING）：{sorted(overlap)}")

    good = _baseline_repo()
    good_hits = detect_hits(good)
    if good_hits:
        failures.append(f"好样本误报：{good_hits}")

    def mutate(**kw):
        d = dict(good)
        d.update(kw)
        return d

    bad_cases: list[tuple[str, str, dict[str, str]]] = []

    def add(code, desc, mutated, key):
        if mutated.get(key, "") == good.get(key, ""):
            print(f"self-test FAIL: 坏样本「{desc}」未改动任何内容（变异失配，按漏检计）")
            sys.exit(1)
        bad_cases.append((code, desc, mutated))

    # PENDING 坏样本（Agent/Skill 域已存在，gate 生效）
    add("AGSK_SECOND_PATH", "skills.rs 出现 std::process::Command",
        mutate(**{"src-tauri/src/skills.rs": "fn run() { let _ = std::process::Command::new(\"sh\"); }\n"}),
        "src-tauri/src/skills.rs")
    add("AGSK_INLINE_SHELL_ENUM", "SkillExec 含 InlineScript 变体",
        mutate(**{"src-tauri/src/domain.rs":
                  good["src-tauri/src/domain.rs"] + "pub enum X { InlineScript }\n"}),
        "src-tauri/src/domain.rs")
    add("AGSK_INLINE_SHELL_REF", "agent.rs 引用 RawShell",
        mutate(**{"src-tauri/src/agent.rs": "fn f() { let _ = RawShell; }\n"}),
        "src-tauri/src/agent.rs")
    add("AGSK_COMMAND_PARITY", "bridge.rs 暴露 skill_install 但不在 ACL/bridge.ts",
        mutate(**{"src-tauri/src/bridge.rs": "pub fn skill_install(app: AppHandle) {}\n"}),
        "src-tauri/src/bridge.rs")
    add("AGSK_RO_COMMAND_PARITY", "bridge.rs 暴露 agent_parse 但 ACL 缺失 → 四件套不齐",
        mutate(**{
            "src-tauri/src/bridge.rs": "pub fn agent_parse(app: AppHandle) {}\n",
            "src-tauri/src/main.rs": "bridge::agent_parse,\n",
            "src/bridge.ts": "export function invoke() {}\nagent_parse\n",
            "src-tauri/permissions/default-commands.toml": "[commands]\nlist_artifact_images\n",
        }),
        "src-tauri/src/bridge.rs")
    add("AGSK_RO_COMMAND_PARITY", "6 只读命令四件套齐全但 handler 缺 check_invocation_source",
        mutate(**{
            "src-tauri/src/bridge.rs": (
                'pub fn agent_parse(app: AppHandle) {}\n'
                'pub fn agent_validate(app: AppHandle) {}\n'
                'pub fn agent_permission_preview(app: AppHandle) {}\n'
                'pub fn skill_parse(app: AppHandle) {}\n'
                'pub fn skill_validate(app: AppHandle) {}\n'
                'pub fn skill_permission_preview(app: AppHandle) {}\n'
            ),
            "src-tauri/src/main.rs": (
                "bridge::agent_parse,\nbridge::agent_validate,\nbridge::agent_permission_preview,\n"
                "bridge::skill_parse,\nbridge::skill_validate,\nbridge::skill_permission_preview,\n"
            ),
            "src/bridge.ts": (
                "export function invoke() {}\n"
                "agent_parse\nagent_validate\nagent_permission_preview\n"
                "skill_parse\nskill_validate\nskill_permission_preview\n"
            ),
            "src-tauri/permissions/default-commands.toml": (
                "[commands]\nlist_artifact_images\nagent_parse\nagent_validate\n"
                "agent_permission_preview\nskill_parse\nskill_validate\nskill_permission_preview\n"
            ),
        }),
        "src-tauri/src/bridge.rs")
    add("AGSK_CAPABILITY_DRIFT", "SKILL_CAPABILITY_V1 定义两处",
        mutate(**{"src-tauri/src/security_policy.rs":
                  good["src-tauri/src/security_policy.rs"] + "pub const SKILL_CAPABILITY_V1: &[&str] = &[\"x\"];\n",
                  "src-tauri/src/extra.rs": "pub const SKILL_CAPABILITY_V1: &[&str] = &[\"y\"];\n"}),
        "src-tauri/src/security_policy.rs")
    add("AGSK_CREDENTIAL_NOT_ECHOED", "CredentialLeak Display 回显 {s} 原文",
        mutate(**{"src-tauri/src/security_policy.rs":
                  good["src-tauri/src/security_policy.rs"].replace("<redacted>", "{s}")}),
        "src-tauri/src/security_policy.rs")
    add("AGSK_EXEC_LOCKED", "main.rs 注册 skill_run（执行解锁）但 ACL/bridge.ts 缺失",
        mutate(**{"src-tauri/src/main.rs": "bridge::skill_run,\n"}),
        "src-tauri/src/main.rs")

    for code, _desc, mutated in bad_cases:
        h = detect_hits(mutated)
        if code not in h:
            failures.append(f"坏样本未检出码位 {code}（漏检）；命中={h}")

    # ACTIVE 坏样本（无需 Agent/Skill 域）
    add("AGSK_ACL_TAIL", "ACL 末条改为 list_artifacts",
        mutate(**{"src-tauri/permissions/default-commands.toml": "[commands]\nlist_artifact_images\nlist_artifacts\n"}),
        "src-tauri/permissions/default-commands.toml")

    for code, _desc, mutated in bad_cases:
        h = detect_hits(mutated)
        if code not in h:
            failures.append(f"坏样本未检出码位 {code}（漏检）；命中={h}")

    # PENDING gate 生效验证：无 Agent/Skill 域时 PENDING 码位不误触发
    no_as = {
        "src-tauri/Cargo.toml": "[package]\nname = \"mvp-browser-os\"\n",
        "src-tauri/src/bridge.rs": "fn f() { let _ = std::process::Command::new(\"sh\"); }\n",
        "src-tauri/permissions/default-commands.toml": "[commands]\nlist_artifact_images\n",
        "src/bridge.ts": "export function invoke(name: string) {}\n",
    }
    no_as_hits = detect_hits(no_as)
    leaked = [c for c in no_as_hits if c in ("AGSK_SECOND_PATH", "AGSK_INLINE_SHELL_ENUM",
                                              "AGSK_INLINE_SHELL_REF", "AGSK_COMMAND_PARITY")]
    if leaked:
        failures.append(f"无 Agent/Skill 域时 PENDING 码位误触发（gate 失效）：{leaked}")

    if failures:
        for f in failures:
            print(f"  - {f}")
        print("AGENT_SKILL_POLICY_SELF_TEST=FAIL")
        return 1
    print(f"AGENT_SKILL_POLICY_SELF_TEST=PASS（ACTIVE={len(ACTIVE_CODES)}，PENDING={len(PENDING_CODES)}）")
    return 0


# ---------------------------------------------------------------------------
# 模式
# ---------------------------------------------------------------------------


def _mode_default() -> int:
    repo = _scan_real_repo()
    hits = detect_hits(repo)
    if not hits:
        print("AGENT_SKILL_POLICY=PASS（无违规）")
        return 0
    for code, details in sorted(hits.items()):
        for d in details:
            print(f"  {code}: {d}")
    print("AGENT_SKILL_POLICY=FAIL")
    return 2


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--self-test", action="store_true", help="好/坏样本双向自检")
    args = ap.parse_args()
    if args.self_test:
        return _run_self_test()
    return _mode_default()


if __name__ == "__main__":
    sys.exit(main())
