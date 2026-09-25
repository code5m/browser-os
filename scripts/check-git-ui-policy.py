#!/usr/bin/env python3
"""Expose the M1-7 Git UI safety invariants as a reproducible source fixture.

M1-7 把 M1-5 只读能力与 M1-6 写能力接到前端。后端已有 `check-git-write-policy.py`
守住 Rust 侧，本脚本守住**前端侧**，防止后续改动悄悄绕过闸门：

  - 写操作只能经 `bridge.requestGitWrite` → `bridge.confirmGitWrite`，
    除 `src/bridge.ts` 外任何前端文件不得直接 invoke Git 写/读命令
  - confirm 必须持 preview、必须先过本地二次确认（dangerous 未勾选不发命令）
  - dangerous（discard / push）必须有显式二次确认勾选项，且未勾选时按钮禁用
  - commit message / 分支名 / 路径列表必须先本地校验，且校验在 request 之前
  - 错误态必须经 `redactSecrets` 脱敏；Git UI 不得读写/持久化任何凭据
  - 不得自动触发写操作（store 内禁止 `watch(`）
  - 不得出现黑名单操作（pull/fetch/merge/rebase/reset/stash）与 force 推送字面量
  - 必须订阅 `git-write-completed` 并全局挂载确认弹窗

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# 只允许 bridge.ts 直接 invoke 的 Git 命令（前端其它位置一律视为绕过闸门）
GIT_COMMANDS = (
    "git_status",
    "git_diff",
    "git_branch_list",
    "request_git_write",
    "confirm_git_write",
)
GIT_WRITE_COMMANDS = ("request_git_write", "confirm_git_write")

# 白名单之外的 Git 操作字面量（M1-6 冻结黑名单）
FORBIDDEN_OP_LITERALS = (
    "pull",
    "fetch",
    "merge",
    "rebase",
    "reset",
    "stash",
    "cherry-pick",
    "cherry_pick",
    "clean",
)
# 危险推送字面量（force / 删除远端分支 / mirror）
FORCE_PUSH_LITERALS = (
    r"--force",
    r"force[-_]with[-_]lease",
    r"\+refs/heads",
    r":refs/heads",
    r"--mirror",
    r"--delete",
)

# 参与「凭据隔离」检查的 Git UI 文件（redact.ts 是脱敏层本身，不参与）
GIT_UI_KEYS = ("store", "panel", "diff", "dialog")


def strip_comments(source: str) -> str:
    """去掉 // 与 /* */ 注释：逐字符跟踪字符串字面量，避免误伤代码里的引号内容。"""
    out: list[str] = []
    i = 0
    n = len(source)
    quote: str | None = None
    while i < n:
        ch = source[i]
        if quote:
            if ch == "\\":
                out.append(source[i : i + 2])
                i += 2
                continue
            if ch == quote:
                quote = None
            out.append(ch)
            i += 1
            continue
        if ch in "'\"":
            quote = ch
            out.append(ch)
            i += 1
            continue
        if ch == "/" and i + 1 < n and source[i + 1] == "/":
            while i < n and source[i] != "\n":
                i += 1
            continue
        if ch == "/" and i + 1 < n and source[i + 1] == "*":
            i += 2
            while i + 1 < n and not (source[i] == "*" and source[i + 1] == "/"):
                if source[i] == "\n":
                    out.append("\n")
                i += 1
            i += 2
            continue
        out.append(ch)
        i += 1
    return "".join(out)


def ts_function_body(source: str, name: str) -> str:
    """Return the body of a TS/Vue `function <name>(...) { ... }` block.

    先跳到参数列表的右括号再找 body 的 `{`，否则 `opts = {}` 之类默认值会被误判成 body。
    """
    match = re.search(rf"\bfunction\s+{re.escape(name)}\s*\(", source)
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
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[start : index + 1]
    return source[start:]


def detect_violations(files: dict[str, object]) -> list[str]:
    violations: list[str] = []
    store = str(files.get("store", ""))
    panel = str(files.get("panel", ""))
    diff = str(files.get("diff", ""))
    dialog = str(files.get("dialog", ""))
    redact = str(files.get("redact", ""))
    repo = str(files.get("repo", ""))
    app = str(files.get("app", ""))
    src_files = files.get("src_files", [])
    assert isinstance(src_files, list)
    src_files = [(str(p), str(t)) for p, t in src_files]

    # ---- 1) 闸门不可绕过：除 bridge.ts 外不得直接 invoke Git 命令 ----
    direct = re.compile(
        r"invoke[<(]\s*[\"'](?:%s)[\"']" % "|".join(GIT_COMMANDS)
    )
    for path, text in src_files:
        if path.endswith(("src/bridge.ts", "bridge.ts")):
            continue
        if direct.search(text):
            violations.append(f"GIT_UI_GATE_BYPASSED: {path}")

    # ---- 2) store：request / confirm 双阶段 ----
    if "bridge.requestGitWrite" not in store:
        violations.append("GIT_UI_REQUEST_MISSING")
    confirm_body = ts_function_body(store, "confirmWrite")
    if not confirm_body:
        violations.append("GIT_UI_CONFIRM_GATE_MISSING")
    else:
        if "bridge.confirmGitWrite" not in confirm_body:
            violations.append("GIT_UI_CONFIRM_OUTSIDE_GATE")
        if not re.search(r"if\s*\(\s*!pv\s*\)", confirm_body):
            violations.append("GIT_UI_CONFIRM_WITHOUT_PREVIEW_GUARD")
        if not re.search(r"dangerous\s*&&\s*!dangerousAck", confirm_body):
            violations.append("GIT_UI_DANGEROUS_WITHOUT_DOUBLE_CONFIRM")

    # ---- 3) 本地校验必须在 request 之前，且覆盖 commit message ----
    request_body = ts_function_body(store, "requestWrite")
    if not request_body:
        violations.append("GIT_UI_REQUEST_GATE_MISSING")
    else:
        pre_idx = request_body.find("precheck(")
        req_idx = request_body.find("bridge.requestGitWrite")
        if pre_idx < 0 or req_idx < 0 or pre_idx > req_idx:
            violations.append("GIT_UI_PRECHECK_AFTER_REQUEST")
    if 'case "commit":' not in store:
        violations.append("GIT_UI_COMMIT_CHECK_MISSING")
    if 'trim() === ""' not in store:
        violations.append("GIT_UI_COMMIT_WITHOUT_MESSAGE_CHECK")

    # ---- 4) 错误态脱敏 ----
    if store.count("redactSecrets(") < 4:
        violations.append("GIT_UI_ERROR_NOT_REDACTED")
    if not re.search(r"URL_USERINFO", redact):
        violations.append("GIT_UI_REDACTION_INCOMPLETE")
    if not re.search(r"Bearer", redact):
        violations.append("GIT_UI_REDACTION_INCOMPLETE")

    # ---- 5) 不得自动触发写操作 ----
    if re.search(r"\bwatch\(", store):
        violations.append("GIT_UI_AUTO_WRITE_TRIGGER")

    # ---- 6) 凭据隔离：Git UI 不得读写/持久化凭据 ----
    for key, text in (("store", store), ("panel", panel), ("diff", diff), ("dialog", dialog)):
        code = strip_comments(text)
        if re.search(r"\btoken\b", code, re.IGNORECASE):
            violations.append(f"GIT_UI_TOKEN_IN_GIT_UI: {key}")
        if re.search(r"localStorage|sessionStorage|indexedDB", code):
            violations.append(f"GIT_UI_CREDENTIAL_PERSISTED: {key}")

    # ---- 7) 确认弹窗：dangerous 二次确认 + 按钮禁用 + preview 字段展示 ----
    if "dangerousAck" not in dialog:
        violations.append("GIT_UI_DANGEROUS_ACK_MISSING")
    if not re.search(r"type=\"checkbox\"", dialog):
        violations.append("GIT_UI_DANGEROUS_ACK_MISSING")
    if "!git.canConfirm" not in dialog:
        violations.append("GIT_UI_CONFIRM_BUTTON_NOT_GATED")
    for field in ("pv.summary", "affected_paths", "pv.dangerous", "PUSH_NOTICE"):
        if field not in dialog:
            violations.append("GIT_UI_PREVIEW_FIELDS_MISSING")

    # ---- 8) 面板：复用既有仓库数据 + 七个写操作入口齐全 ----
    if "ws.repos" not in panel:
        violations.append("GIT_UI_REPO_SOURCE_NOT_REUSED")
    for entry in (
        "git.stage(",
        "git.unstage(",
        "git.discard(",
        "git.commit(",
        "git.createBranch(",
        "git.checkoutBranch(",
        "git.push(",
    ):
        if entry not in panel:
            violations.append("GIT_UI_WRITE_ENTRY_MISSING")
    if "GitDiffViewer" not in panel:
        violations.append("GIT_UI_DIFF_NOT_MOUNTED")

    # ---- 9) 挂载与事件订阅 ----
    if not re.search(r"<GitPanel\b", repo):
        violations.append("GIT_UI_NOT_MOUNTED")
    if "onGitWriteCompleted" not in app:
        violations.append("GIT_UI_EVENT_NOT_SUBSCRIBED")
    if "GitWriteConfirmDialog" not in app:
        violations.append("GIT_UI_DIALOG_NOT_MOUNTED")

    # ---- 10) 黑名单操作 / force 推送字面量（全前端） ----
    for path, text in src_files:
        for op in FORBIDDEN_OP_LITERALS:
            if re.search(rf"[\"']{re.escape(op)}[\"']", text):
                violations.append(f"GIT_UI_FORBIDDEN_OP: {path}")
                break
        for literal in FORCE_PUSH_LITERALS:
            if re.search(literal, text):
                violations.append(f"GIT_UI_FORCE_PUSH: {path}")
                break

    return violations


def scan_repository(root: Path) -> list[str]:
    src = root / "src"
    mapping = {
        "store": src / "stores" / "useGitStore.ts",
        "panel": src / "components" / "workspace" / "GitPanel.vue",
        "diff": src / "components" / "workspace" / "GitDiffViewer.vue",
        "dialog": src / "components" / "workspace" / "GitWriteConfirmDialog.vue",
        "redact": src / "utils" / "redact.ts",
        "repo": src / "components" / "workspace" / "RepoPanel.vue",
        "app": src / "App.vue",
    }
    files: dict[str, object] = {}
    for key, path in mapping.items():
        if not path.exists():
            print(f"check-git-ui-policy: missing file {path.relative_to(root)}", file=sys.stderr)
            return [f"GIT_UI_FILE_MISSING: {path.relative_to(root)}"]
        files[key] = path.read_text(encoding="utf-8")
    src_files: list[tuple[str, str]] = []
    for path in sorted(src.rglob("*")):
        if path.is_file() and path.suffix in {".ts", ".vue"}:
            src_files.append((str(path.relative_to(root)), path.read_text(encoding="utf-8")))
    files["src_files"] = src_files
    return detect_violations(files)


# --------------------------------------------------------------------------
# 自检夹具
# --------------------------------------------------------------------------

GOOD_STORE = """
async function requestWrite(op, opts = {}) {
  const invalid = precheck(op, opts);
  if (invalid) { writeError.value = invalid; return; }
  preview.value = await bridge.requestGitWrite({ repoId: repoId.value, op });
}
async function confirmWrite() {
  const pv = preview.value;
  if (!pv) return;
  if (pv.dangerous && !dangerousAck.value) { writeError.value = "x"; return; }
  lastJob.value = await bridge.confirmGitWrite({ jobId: pv.job_id });
}
function precheck(op, opts) {
  switch (op) {
    case "commit":
      if (!opts.message || opts.message.trim() === "") return "empty";
      return null;
  }
}
statusError.value = redactSecrets(e);
branchError.value = redactSecrets(e);
diffError.value = redactSecrets(e);
writeError.value = redactSecrets(e);
"""

GOOD_PANEL = """
const ws = useWorkspaceStore();
<option v-for="r in ws.repos" :key="r.id" :value="r.id">
<button @click="git.stage()">暂存</button>
<button @click="git.unstage()">取消暂存</button>
<button @click="git.discard()">丢弃</button>
<button @click="git.commit()">提交</button>
<button @click="git.createBranch()">新建分支</button>
<button @click="git.checkoutBranch(targetBranch)">切换</button>
<button @click="git.push()">推送</button>
<GitDiffViewer />
"""

GOOD_DIFF = """
<div v-if="git.diff?.more">已截断</div>
<span v-if="h.binary">二进制</span>
"""

GOOD_DIALOG = """
<span>{{ pv.summary }}</span>
<li v-for="p in pv.affected_paths">{{ p }}</li>
<span v-if="pv.dangerous">需二次确认</span>
<div>{{ PUSH_NOTICE }}</div>
<input v-model="git.dangerousAck" type="checkbox" />
<button :disabled="git.busy || !git.canConfirm" @click="git.confirmWrite()">确认执行</button>
"""

GOOD_REDACT = """
const URL_USERINFO = /https:\\/\\/u:p@/;
const AUTH = /Bearer [A-Za-z0-9]+/;
export function redactSecrets(e) { return ""; }
"""

GOOD_REPO = """
import GitPanel from "./GitPanel.vue";
<GitPanel v-if="tab === 'git'" />
"""

GOOD_APP = """
bridge.onGitWriteCompleted((j) => git.onWriteCompleted(j));
<GitWriteConfirmDialog />
"""

GOOD_SRC_FILES = [
    ("src/bridge.ts", 'invoke<GitWritePreview>("request_git_write", p)'),
    ("src/capabilities/git/state/useGitStore.ts", "bridge.requestGitWrite"),
    ("src/App.vue", "bridge.onGitWriteCompleted"),
]


def run_self_test() -> int:
    good = {
        "store": GOOD_STORE,
        "panel": GOOD_PANEL,
        "diff": GOOD_DIFF,
        "dialog": GOOD_DIALOG,
        "redact": GOOD_REDACT,
        "repo": GOOD_REPO,
        "app": GOOD_APP,
        "src_files": GOOD_SRC_FILES,
    }
    failures: list[str] = []
    baseline = detect_violations(good)
    if baseline:
        failures.append(f"good fixture should be clean, got {baseline}")

    cases: list[tuple[str, dict, str]] = [
        ("绕过闸门直连 invoke", {"src_files": [("src/views/X.vue", 'invoke("confirm_git_write")')]},
         "GIT_UI_GATE_BYPASSED"),
        ("confirm 缺 preview 守卫", {"store": GOOD_STORE.replace("if (!pv) return;", "")},
         "GIT_UI_CONFIRM_WITHOUT_PREVIEW_GUARD"),
        ("dangerous 无二次确认", {"store": GOOD_STORE.replace("pv.dangerous && !dangerousAck.value", "false")},
         "GIT_UI_DANGEROUS_WITHOUT_DOUBLE_CONFIRM"),
        ("校验在 request 之后", {"store": GOOD_STORE.replace(
            "const invalid = precheck(op, opts);\n  if (invalid) { writeError.value = invalid; return; }\n", "")},
         "GIT_UI_PRECHECK_AFTER_REQUEST"),
        ("commit 无 message 校验", {"store": GOOD_STORE.replace('trim() === ""', "false")},
         "GIT_UI_COMMIT_WITHOUT_MESSAGE_CHECK"),
        ("错误态未脱敏", {"store": GOOD_STORE.replace("redactSecrets(e)", "String(e)")},
         "GIT_UI_ERROR_NOT_REDACTED"),
        ("自动触发写", {"store": GOOD_STORE + "\nwatch(() => repoId.value, requestWrite);"},
         "GIT_UI_AUTO_WRITE_TRIGGER"),
        ("UI 里出现 token", {"store": GOOD_STORE + "\nconst token = localStorage.getItem('t');"},
         "GIT_UI_TOKEN_IN_GIT_UI"),
        ("弹窗缺勾选框", {"dialog": GOOD_DIALOG.replace('type="checkbox"', 'type="text"')},
         "GIT_UI_DANGEROUS_ACK_MISSING"),
        ("确认按钮未禁用", {"dialog": GOOD_DIALOG.replace("!git.canConfirm", "")},
         "GIT_UI_CONFIRM_BUTTON_NOT_GATED"),
        ("弹窗未展示 preview", {"dialog": GOOD_DIALOG.replace("pv.summary", "op")},
         "GIT_UI_PREVIEW_FIELDS_MISSING"),
        ("未复用仓库数据", {"panel": GOOD_PANEL.replace("ws.repos", "ownRepos")},
         "GIT_UI_REPO_SOURCE_NOT_REUSED"),
        ("缺写操作入口", {"panel": GOOD_PANEL.replace("git.push()", "")},
         "GIT_UI_WRITE_ENTRY_MISSING"),
        ("未挂载 GitPanel", {"repo": GOOD_REPO.replace("<GitPanel v-if", "<div v-if")},
         "GIT_UI_NOT_MOUNTED"),
        ("未订阅完成事件", {"app": GOOD_APP.replace("bridge.onGitWriteCompleted", "bridge.onSyncCompleted")},
         "GIT_UI_EVENT_NOT_SUBSCRIBED"),
        ("未挂载确认弹窗", {"app": GOOD_APP.replace("<GitWriteConfirmDialog />", "")},
         "GIT_UI_DIALOG_NOT_MOUNTED"),
        ("黑名单操作", {"src_files": GOOD_SRC_FILES + [("src/views/Y.vue", 'const op = "rebase";')]},
         "GIT_UI_FORBIDDEN_OP"),
        ("force 推送", {"src_files": GOOD_SRC_FILES + [("src/views/Z.vue", 'const r = "--force";')]},
         "GIT_UI_FORCE_PUSH"),
        ("脱敏覆盖不全", {"redact": GOOD_REDACT.replace("URL_USERINFO", "PATTERN")},
         "GIT_UI_REDACTION_INCOMPLETE"),
    ]

    for label, overrides, expected in cases:
        merged = dict(good)
        merged.update(overrides)
        found = detect_violations(merged)
        # 违规码可能带 `: <位置>` 后缀，按前缀匹配
        if not any(v == expected or v.startswith(expected + ":") for v in found):
            failures.append(f"{label}: expected {expected}, got {found}")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print("self-test: ok")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check M1-7 Git UI safety invariants.")
    parser.add_argument("--self-test", action="store_true", help="run built-in tests")
    args = parser.parse_args(argv)
    if args.self_test:
        return run_self_test()

    violations = scan_repository(Path(__file__).resolve().parent.parent)
    if violations:
        print(f"check-git-ui-policy: failed ({len(violations)} violation(s))")
        for violation in violations:
            print(violation)
        return 1
    print("check-git-ui-policy: ok (Git UI invariants hold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
