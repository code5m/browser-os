#!/usr/bin/env bash
#
# verify-resources.sh — M0-1.b 资源验证驱动：把 m0-baseline-contract-v1.md §5/§6/§7
# 中资源类指标（启动、进程树 RSS/FD、资源循环、终端吞吐、孤儿进程）的采集与统计
# 流程编码成可复跑驱动。
#
# 范围（M0-1.b，严格限定）：
#   1. release 启动驱动：契约 §6.1 固定场景（冷启动新隔离 XDG、spawn release、
#      ready 信号带 run_id 后记录 startup_ready_ms）。
#   2. 进程树枚举与 RSS/FD 采样：/proc/<pid>/stat 的 starttime 防 PID 复用；
#      VmRSS（/proc/<pid>/status）；FD 数（/proc/<pid>/fd 可访问条目）。
#      契约 §5 idle_process_tree_rss_kib / idle_process_tree_fd_count：
#      ready 后每 5 秒采 1 次、持续 60 秒；读取失败的 PID 单列，不得默认为 0。
#   3. 资源循环驱动：tab / grid / terminal 三类固定场景（契约 §6.2），
#      每类 5 次预热 + 20 次正式，独立进程、独立统计；输出
#      *_cycle_rss_slope_kib（OLS 斜率，契约 §7）、resource_cycle_fd_delta、
#      orphan_process_count（关闭前快照后代 PID+starttime，等待 2 秒后复核存活数）。
#   4. 终端吞吐驱动：契约 §6.3 固定 10 MiB ASCII 负载，begin/end 唯一标记，
#      记录 terminal_10mib_elapsed_ms / terminal_frame_gap_p95_ms / max_ms。
#   5. 统计计算：median/min/max/波动率、p95（nearest-rank，<20 样本不伪报）、
#      OLS 斜率（单位为 KiB/cycle），全部与契约 §7 公式一致。
#
# 边界（禁止）：
#   - 不实现 M0-1.c（固定参数/退出码/JSON schema/完整性哈希/日志格式统一接入 pre-merge）。
#   - 不修改冻结契约、不修改 Rust/Vue 产品代码、不补 ready/终端测量钩子（M0-0.b）。
#   - 不采集或宣称 M0-0.b/c 正式性能基线：产品 ready/终端钩子未落地时，本脚本
#     正式模式只输出明确 BLOCKED（证据目录完整、退出非 0），绝不伪造 PASS 数值。
#   - 不自动安装依赖、不访问公网、不删除用户数据；生成物只进入本 run 隔离目录。
#
# BLOCKED/DEFERRED 语义（与 baseline-check.sh 一致）：
#   - 需要产品 ready 钩子的指标（startup_ready_ms、idle_*、*_cycle_*、
#     resource_cycle_fd_delta、orphan_process_count）在钩子缺失时标 BLOCKED，
#     owner=M0-0.b（补 ready 钩子的 WBS）。
#   - 需要终端 begin/end 标记的指标（terminal_*）标 BLOCKED，owner=M0-0.b。
#   - DEFERRED(M2-4/M4-3) 指标同样不计入本检查点 PASS 判定。
#   - 正式模式钩子缺失 → summary status=BLOCKED → 退出 1（门禁失败语义，
#     「明确返回 BLOCKED 而非伪造成功」）；self-test 用固定夹具验证驱动能力。
#
# 证据保留策略（与 baseline-check.sh 相同，.gitignore 已对 logs/m0-baseline/ 开例外）：
#   summary / environment / scenario / measurements / SHA256SUMS / raw 原始输出
#   全部入库可追溯。
#
# 用法：
#   scripts/verify-resources.sh            # 正式模式（工作树必须干净）
#   scripts/verify-resources.sh --help
#   scripts/verify-resources.sh --self-test   # fixture 验证脚本自身，不生成正式证据
#
# 环境变量（可选）：
#   M0_RUN_MODE          formal（默认，固定完整样本）或 smoke（仅探索，不得 PASS）
#   M0_EVIDENCE_ROOT     证据暂存根目录；默认 <repo>/logs/m0-baseline
#   M0_EVIDENCE_LABEL_ROOT  summary 中记录的最终逻辑根；默认跟随物理根
#   M0_EXPECTED_BINARY_SHA256  baseline-check.sh 生成的 release 二进制哈希
#   VR_CYCLE_SAMPLES     每类资源循环正式样本数，默认 20（另加 5 次预热）
#   VR_IDLE_SECONDS      idle 采样总时长，默认 60（每 5 秒 1 点）
#   VR_SELF_TEST         内部使用：1 = fixture 模式（--self-test 自动设置）
#
set -euo pipefail

# ---------------------------------------------------------------------------
# 常量
# ---------------------------------------------------------------------------
SCRIPT_VERSION="M0-1.b-3"
CONTRACT_VERSION="V1.1"
SCENARIO_VERSION="M0-1.b-v2"
DEFAULT_CYCLE_SAMPLES=20
DEFAULT_WARMUP_SAMPLES=5
DEFAULT_IDLE_SECONDS=60
IDLE_INTERVAL_S=5
ORPHAN_WAIT_S=2
TERM_LOAD_MIB=10
DEFAULT_TERM_SAMPLES=3

# 命令超时（秒），scenario.json 会记录
T_READY=120
T_TERM=300

SCRIPT_SRC="${BASH_SOURCE[0]}"
SCRIPT_PATH="$(readlink -f "$SCRIPT_SRC")"
SCRIPT_DIR="$(dirname "$SCRIPT_PATH")"
# M0-1.c：summary.json 固定 schema 与共享校验器（见 scripts/GATE-CONTRACT.md）
SCHEMA_FILE="$SCRIPT_DIR/schema/m0-summary.schema.json"
VALIDATE_SUMMARY="$SCRIPT_DIR/validate-summary.py"

# ---------------------------------------------------------------------------
# 工具与根目录解析
# ---------------------------------------------------------------------------
REQUIRED_TOOLS=(git python3 sha256sum timeout stat date ps awk sort head tail)
# 注：进程树/RSS/FD 采集直接读 /proc，不依赖 ps；ps 仅用于 fixture 对照。

require_tools() {
  local t
  for t in "${REQUIRED_TOOLS[@]}"; do
    if ! command -v "$t" >/dev/null 2>&1; then
      echo "error: required tool missing: $t" >&2
      exit 1
    fi
  done
}

resolve_root() {
  git rev-parse --show-toplevel 2>/dev/null || {
    echo "error: not inside a git repository" >&2
    exit 1
  }
}

# ---------------------------------------------------------------------------
# 用法与参数
# ---------------------------------------------------------------------------
usage() {
  cat <<'EOF'
verify-resources.sh — M0-1.b 资源验证驱动

用法:
  scripts/verify-resources.sh            正式模式（工作树必须干净；产品 ready/终端钩子
                                         未落地时输出 BLOCKED 证据并退出 1）
  scripts/verify-resources.sh --help     显示本帮助
  scripts/verify-resources.sh --self-test 用固定夹具验证驱动能力（fixture，不生成正式证据）

覆盖指标（契约 §5，钩子缺失时标 BLOCKED）:
  startup_ready_ms / idle_process_tree_rss_kib / idle_process_tree_fd_count
  tab|grid|terminal_cycle_rss_slope_kib / resource_cycle_fd_delta / orphan_process_count
  terminal_10mib_elapsed_ms / terminal_frame_gap_p95_ms / terminal_frame_gap_max_ms
  （DEFERRED: script_first_response_ms=M2-4, database_first_row_ms=M4-3）

可选环境变量:
  M0_RUN_MODE=formal|smoke  默认 formal；smoke 结果固定为 EXPLORATORY
  M0_EVIDENCE_ROOT=PATH     证据根目录（多批次采集应指向仓库外暂存目录）
  M0_EVIDENCE_LABEL_ROOT=PATH summary 中记录的最终逻辑根（总控归档时使用）
  M0_EXPECTED_BINARY_SHA256 baseline-check.sh 生成的 release 二进制 SHA-256；formal 必填
  VR_CYCLE_SAMPLES=N  每类资源循环正式样本数（默认 20；另加 5 次预热）
  VR_IDLE_SECONDS=N   idle 采样总时长（默认 60，每 5 秒 1 点）

输出契约（M0-1.c）:
  summary.json 满足 scripts/schema/m0-summary.schema.json（机器判定源）
  SHA256SUMS 覆盖本 run 内除自身外全部证据文件
  日志行前缀 [M0-1.b]，见 scripts/GATE-CONTRACT.md
  合并前门禁: scripts/pre-merge.sh（M0-1 脚本集合检查入口）

退出码: 0 = 驱动自检/无 BLOCKED 阻断；1 = BLOCKED（钩子缺失）或驱动 FAIL；2 = 非法参数
EOF
}

parse_args() {
  case "${1:-}" in
    --help|-h)
      usage
      exit 0
      ;;
    --self-test)
      run_self_test
      exit $?
      ;;
    --)
      ;;
    -*)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      if [ $# -gt 0 ]; then
        echo "error: unexpected argument: $1" >&2
        usage >&2
        exit 2
      fi
      ;;
  esac
}

# ---------------------------------------------------------------------------
# 工具函数
# ---------------------------------------------------------------------------
# run_capture <name> <out_prefix> -- <cmd...>
#  stdout -> $RAW/<name>_<out_prefix>.stdout.log  stderr -> 同名 .stderr.log
#  返回码通过全局 LAST_RC 返回
run_capture() {
  local name="$1" prefix="$2"
  shift 2
  [ "$1" = "--" ] && shift
  set +e
  "$@" >"$RAW/${name}_${prefix}.stdout.log" 2>"$RAW/${name}_${prefix}.stderr.log"
  LAST_RC=$?
  set -e
}

json_get() { # python 单值转义，供 bash 拼 JSON
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
}

monotonic_ms() {
  python3 -c 'import time; print(time.monotonic_ns() // 1_000_000)'
}

median_of() { # stdin 每行一个数字
  python3 -c '
import sys
vals = sorted(float(x) for x in sys.stdin if x.strip())
n = len(vals)
if n == 0:
    print(0); raise SystemExit
m = vals[n//2] if n % 2 else (vals[n//2-1] + vals[n//2]) / 2
print("%.0f" % m)
'
}

# bash 普通变量不会自动进入 python 子进程环境；需要显式 export。
export_for_python() {
  local v
  for v in "$@"; do
    if declare -p "$v" >/dev/null 2>&1; then
      export "$v"
    fi
  done
}

# ---------------------------------------------------------------------------
# 产品钩子探测（契约 §6.1/§6.3 的关键标记；M0-0.b 补齐，M0-1.b 仅标记 BLOCKED）
# ---------------------------------------------------------------------------
detect_product_hooks() {
  if grep -rqE '__M0_READY|startup_ready' "$ROOT/src" "$ROOT/src-tauri/src" 2>/dev/null; then
    READY_HOOK="READY"
  else
    READY_HOOK="BLOCKED"
  fi
  if grep -rq '__M0_TERM' "$ROOT/src" "$ROOT/src-tauri/src" 2>/dev/null; then
    TERM_HOOK="READY"
  else
    TERM_HOOK="BLOCKED"
  fi
}

# ---------------------------------------------------------------------------
# 环境 / 来源指纹收集（契约 §4，字段与 baseline-check.sh 一致）
# ---------------------------------------------------------------------------
collect_environment() {
  local val
  # --- 系统 ---
  ENV_OS="$(sed -n 's/^PRETTY_NAME="\?\([^"]*\)"\?/\1/p' /etc/os-release 2>/dev/null || uname -s)"
  ENV_KERNEL="$(uname -r)"
  ENV_ARCH="$(uname -m)"
  ENV_CPU_MODEL="$(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2 | sed 's/^ *//' || true)"
  [ -n "$ENV_CPU_MODEL" ] || ENV_CPU_MODEL="$(grep -m1 'Hardware' /proc/cpuinfo 2>/dev/null | cut -d: -f2 | sed 's/^ *//' || true)"
  [ -n "$ENV_CPU_MODEL" ] || ENV_CPU_MODEL="UNAVAILABLE:cpuinfo-model-unreadable"
  ENV_NPROC="$(nproc 2>/dev/null || echo UNAVAILABLE:nproc)"
  ENV_MEM_BYTES="$(awk '/MemTotal/ {print int($2)*1024}' /proc/meminfo 2>/dev/null || echo 0)"
  [ -n "$ENV_MEM_BYTES" ] && [ "$ENV_MEM_BYTES" != "0" ] || ENV_MEM_BYTES="UNAVAILABLE:meminfo"

  # --- 图形 ---
  ENV_XDG_SESSION="${XDG_SESSION_TYPE:-UNAVAILABLE:no-xdg-session-type}"
  ENV_DISPLAY="${DISPLAY:-}"
  ENV_WAYLAND="${WAYLAND_DISPLAY:-}"
  val="$(xdpyinfo 2>/dev/null | grep -m1 dimensions || true)"
  ENV_RESOLUTION="${val:-UNAVAILABLE:xdpyinfo}"
  val="$(gsettings get org.gnome.desktop.interface text-scaling-factor 2>/dev/null || true)"
  ENV_SCALE="${val:-UNAVAILABLE:gsettings}"
  ENV_HOST_GDK_BACKEND="${GDK_BACKEND:-}"
  ENV_HOST_WEBKIT_DMABUF="${WEBKIT_DISABLE_DMABUF_RENDERER:-}"
  ENV_GDK_BACKEND="x11"
  ENV_WEBKIT_DMABUF="1"

  # --- WebKit / GTK ---
  val="$(pkg-config --modversion webkit2gtk-4.1 2>/dev/null || pkg-config --modversion webkit2gtk-4.0 2>/dev/null || true)"
  ENV_WEBKITGTK="${val:-UNAVAILABLE:pkg-config-webkit2gtk}"
  val="$(pkg-config --modversion gtk+-3.0 2>/dev/null || true)"
  ENV_GTK="${val:-UNAVAILABLE:pkg-config-gtk3}"

  # --- 工具链 ---
  ENV_RUSTC="$(rustc --version 2>/dev/null || echo UNAVAILABLE:rustc)"
  ENV_CARGO="$(cargo --version 2>/dev/null || echo UNAVAILABLE:cargo)"
  ENV_NODE="$(node --version 2>/dev/null || echo UNAVAILABLE:node)"
  ENV_NPM="$(npm --version 2>/dev/null || echo UNAVAILABLE:npm)"

  # --- 运行条件 ---
  ENV_LOCALE="${LANG:-UNAVAILABLE:no-lang}"
  ENV_TIMEZONE="$(readlink -f /etc/localtime 2>/dev/null | sed 's#^.*/zoneinfo/##' || echo UNAVAILABLE:localtime)"
  ENV_LOADAVG="$(awk '{print $1","$2","$3}' /proc/loadavg 2>/dev/null || echo UNAVAILABLE:loadavg)"
  ENV_MEMAVAIL="$(awk '/MemAvailable/ {print int($2)*1024}' /proc/meminfo 2>/dev/null || echo UNAVAILABLE:meminfo)"
  val="$(ls /sys/class/power_supply/ 2>/dev/null | grep -E '^(AC|BAT)' | head -1 || true)"
  ENV_POWER="${val:-UNAVAILABLE:no-ac-interface}"

  # --- 构建物 / 锁文件 ---
  ENV_CARGO_LOCK_SHA="$(sha256sum "$ROOT/src-tauri/Cargo.lock" 2>/dev/null | awk '{print $1}' || echo UNAVAILABLE:Cargo.lock)"
  ENV_NPM_LOCK_SHA="$(sha256sum "$ROOT/package-lock.json" 2>/dev/null | awk '{print $1}' || echo UNAVAILABLE:package-lock.json)"

  ENV_ISOLATED_XDG_DATA="$XDG_BASE/<scenario>/data"
  ENV_ISOLATED_XDG_CACHE="$XDG_BASE/<scenario>/cache"
  ENV_ISOLATED_XDG_CONFIG="$XDG_BASE/<scenario>/config"
}

# ---------------------------------------------------------------------------
# 驱动能力：进程树 / RSS / FD 采样（直接读 /proc）
# ---------------------------------------------------------------------------
# snapshot_process_tree.py <root_pid>：枚举 root 的全部后代（含 root），输出 JSON：
#   {"pid":..., "ppid":..., "starttime":..., "comm":..., "cmdline":..., "rss_kib":..., "fd_count":..., "unreadable": [pid,...]}
# 契约要求：读取失败的 PID 单列，不得默认为 0；starttime 用于防 PID 复用。
snapshot_process_tree() {
  local root_pid="$1"
  python3 - "$root_pid" <<'PY'
import json, os, sys
root = int(sys.argv[1])

def read_proc(pid, field):
    # /proc/<pid>/stat 第 22 字段为 starttime（tick），ppid 为第 4 字段
    try:
        with open("/proc/%d/stat" % pid, "r", encoding="utf-8", errors="replace") as f:
            data = f.read()
        name_end = data.rfind(")")
        rest = data[name_end + 2:].split()
        ppid = int(rest[1])
        starttime = int(rest[19])
        return ppid, starttime
    except (OSError, ValueError, IndexError):
        return None, None

def proc_comm(pid):
    try:
        with open("/proc/%d/comm" % pid, "r", encoding="utf-8", errors="replace") as f:
            return f.read().strip()
    except OSError:
        return None

def proc_cmdline(pid):
    try:
        with open("/proc/%d/cmdline" % pid, "rb") as f:
            raw = f.read()
        if not raw:
            return ""
        text = raw.replace(b"\0", b" ").decode("utf-8", errors="replace").strip()
        return text[:512]
    except OSError:
        return None

def rss_kib(pid):
    try:
        with open("/proc/%d/status" % pid, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    return int(line.split()[1])
    except (OSError, ValueError, IndexError):
        pass
    return None

def fd_count(pid):
    try:
        n = 0
        for entry in os.listdir("/proc/%d/fd" % pid):
            n += 1
        return n
    except OSError:
        return None

# 先收集所有 /proc 下的 (pid, ppid, starttime)，避免依赖 ps 的输出解析
children = {}
ppids = {}
all_pids = []
for entry in os.listdir("/proc"):
    if not entry.isdigit():
        continue
    pid = int(entry)
    ppid, starttime = read_proc(pid, 22)
    if ppid is None:
        continue
    all_pids.append(pid)
    ppids[pid] = ppid
    children.setdefault(ppid, []).append((pid, starttime))

# BFS 从 root 出发，得到后代集合（含 root）
desc = {}
stack = [root]
seen = set()
while stack:
    pid = stack.pop()
    if pid in seen:
        continue
    seen.add(pid)
    for child, st in children.get(pid, []):
        stack.append(child)
        desc[child] = st
desc[root] = None  # root 自身 starttime 单独读
_, root_st = read_proc(root, 22)
desc[root] = root_st

out = {"root_pid": root, "members": [], "unreadable": []}
for pid in sorted(desc):
    starttime = desc[pid]
    ppid = ppids.get(pid)
    rss = rss_kib(pid)
    fd = fd_count(pid)
    if starttime is None or rss is None or fd is None:
        out["unreadable"].append(pid)
    out["members"].append({
        "pid": pid,
        "ppid": ppid,
        "starttime": starttime,
        "comm": proc_comm(pid),
        "cmdline": proc_cmdline(pid),
        "rss_kib": rss,
        "fd_count": fd,
    })
out["total_rss_kib"] = sum(m["rss_kib"] for m in out["members"] if m["rss_kib"] is not None)
out["total_fd_count"] = sum(m["fd_count"] for m in out["members"] if m["fd_count"] is not None)
print(json.dumps(out))
PY
}

# ---------------------------------------------------------------------------
# 驱动能力：统计计算（契约 §7）
# ---------------------------------------------------------------------------
# stats.py：median/min/max/volatility/OLS slope/p95（nearest-rank）
compute_stats() {
  python3 - "$@" <<'PY'
import json, sys
def median(vals):
    n = len(vals)
    if n == 0:
        return 0.0
    s = sorted(vals)
    if n % 2:
        return s[n // 2]
    return (s[n // 2 - 1] + s[n // 2]) / 2.0

def p95(vals):
    # 契约 §7：仅对至少 20 个样本使用 nearest-rank；少于 20 只报告全部值和 max，不伪报 p95
    s = sorted(vals)
    n = len(s)
    if n == 0:
        return None
    if n < 20:
        return None
    idx = int(0.95 * (n - 1))
    return s[idx]

def ols_slope(vals):
    # 对 x = 0..n-1（循环序号）与 y = vals 做普通最小二乘斜率
    n = len(vals)
    if n == 0:
        return None
    xs = list(range(n))
    mean_x = sum(xs) / n
    mean_y = sum(vals) / n
    num = sum((xs[i] - mean_x) * (vals[i] - mean_y) for i in range(n))
    den = sum((x - mean_x) ** 2 for x in xs)
    if den == 0:
        return 0.0
    return num / den

vals = [float(x) for x in sys.argv[1:] if x != ""]
if not vals:
    print(json.dumps({"median": 0, "min": 0, "max": 0, "volatility": "n/a",
                      "p95": None, "ols_slope": None, "last_minus_first": 0, "pct": 0}))
    raise SystemExit
med = median(vals)
mn = min(vals)
mx = max(vals)
vol = "abs-diff:%s" % (mx - mn) if med == 0 else "%.1f%%" % ((mx - mn) / med * 100.0)
out = {
    "median": med,
    "min": mn,
    "max": mx,
    "volatility": vol,
    "p95": p95(vals),
    "ols_slope": ols_slope(vals),
    "last_minus_first": vals[-1] - vals[0],
    "pct": (vals[-1] - vals[0]) / vals[0] * 100.0 if vals[0] != 0 else 0.0,
}
print(json.dumps(out))
PY
}

# ---------------------------------------------------------------------------
# 驱动能力：资源进程差分 / 孤儿检测（契约 §5 orphan_process_count）
# ---------------------------------------------------------------------------
# compare_cycle_snapshots <baseline> <opened> <closed> [closed-fixture]：
#   candidates = opened - baseline（按 pid+starttime，排除应用根进程）
#   orphans    = 关闭后仍可从 /proc 读到相同 starttime 的 candidates（PPID 变化不影响）
# closed 快照由产品驱动在资源关闭并稳定 ORPHAN_WAIT_S 秒后才允许采集。
compare_cycle_snapshots() {
  local baseline_file="$1" opened_file="$2" closed_file="$3" mode="${4:-proc}"
  python3 - "$baseline_file" "$opened_file" "$closed_file" "$mode" <<'PY'
import json, sys

with open(sys.argv[1], encoding="utf-8") as f:
    baseline = json.load(f)
with open(sys.argv[2], encoding="utf-8") as f:
    opened = json.load(f)
with open(sys.argv[3], encoding="utf-8") as f:
    closed = json.load(f)
mode = sys.argv[4]

root = baseline.get("root_pid")
if opened.get("root_pid") != root or closed.get("root_pid") != root:
    raise SystemExit("snapshot root_pid mismatch")

def identities(snapshot):
    return {
        (member.get("pid"), member.get("starttime"))
        for member in snapshot.get("members", [])
        if member.get("pid") != root and member.get("starttime") is not None
    }

baseline_ids = identities(baseline)
opened_ids = identities(opened)
closed_ids = identities(closed)
candidates = sorted(opened_ids - baseline_ids)

def proc_starttime(pid):
    try:
        with open("/proc/%d/stat" % pid, encoding="utf-8", errors="replace") as f:
            data = f.read()
        return int(data[data.rfind(")") + 2:].split()[19])
    except (OSError, ValueError, IndexError):
        return None

if mode == "closed-fixture":
    orphans = sorted(set(candidates) & closed_ids)
else:
    orphans = sorted((pid, starttime) for pid, starttime in candidates if proc_starttime(pid) == starttime)
root_alive = any(
    member.get("pid") == root and member.get("starttime") is not None
    for member in closed.get("members", [])
)

print(json.dumps({
    "baseline_count": len(baseline.get("members", [])),
    "opened_count": len(opened.get("members", [])),
    "closed_count": len(closed.get("members", [])),
    "candidate_count": len(candidates),
    "candidate_pids": [pid for pid, _ in candidates],
    "orphan_count": len(orphans),
    "orphan_pids": [pid for pid, _ in orphans],
    "orphan_detection": "closed-fixture" if mode == "closed-fixture" else "/proc/<pid>/stat starttime",
    "root_alive": root_alive,
    "closed_total_rss_kib": closed.get("total_rss_kib", 0),
    "closed_total_fd_count": closed.get("total_fd_count", 0),
    "unreadable": {
        "baseline": baseline.get("unreadable", []),
        "opened": opened.get("unreadable", []),
        "closed": closed.get("unreadable", []),
    },
}))
PY
}

# validate_terminal_report <report.json> <run_id>：后端报告与当前批次契约必须完全一致。
# stdout 始终输出结构化校验结果；有效返回 0，无效返回 1。
validate_terminal_report() {
  local report_file="$1" expected_run_id="$2"
  python3 - "$report_file" "$expected_run_id" <<'PY'
import json, sys

path, expected_run_id = sys.argv[1:]
errors = []
try:
    with open(path, encoding="utf-8") as f:
        report = json.load(f)
except (OSError, json.JSONDecodeError) as exc:
    report = {}
    errors.append("invalid JSON: %s" % exc)

expected_bytes = 10 * 1024 * 1024
checks = {
    "run_id": report.get("run_id") == expected_run_id,
    "backend_valid": report.get("valid") is True,
    "backend_errors_empty": report.get("errors") == [],
    "begin_seen_once": report.get("begin_seen") == 1,
    "end_seen_once": report.get("end_seen") == 1,
    "consumed_exactly_10mib": report.get("consumed_bytes") == expected_bytes,
    "expected_bytes_exactly_10mib": report.get("expected_bytes") == expected_bytes,
    "elapsed_positive": isinstance(report.get("elapsed_ms"), (int, float)) and report.get("elapsed_ms", 0) > 0,
    "timestamps_ordered": isinstance(report.get("start_ts_ms"), (int, float))
        and isinstance(report.get("end_ts_ms"), (int, float))
        and report.get("end_ts_ms", 0) >= report.get("start_ts_ms", 0) > 0,
    "frame_gaps_nonempty": isinstance(report.get("frame_gaps_ms"), list)
        and len(report.get("frame_gaps_ms", [])) > 0,
    "frame_gaps_numeric": isinstance(report.get("frame_gaps_ms"), list)
        and all(isinstance(value, (int, float)) and value >= 0 for value in report.get("frame_gaps_ms", [])),
}
errors.extend(name for name, passed in checks.items() if not passed)
print(json.dumps({"valid": not errors, "checks": checks, "errors": errors}, ensure_ascii=False))
raise SystemExit(0 if not errors else 1)
PY
}


# ---------------------------------------------------------------------------
# 驱动能力：idle 采样编排（契约 §6.1 第 5 条 / §5 idle_*）
# ---------------------------------------------------------------------------
# run_idle_sampling <root_pid> <seconds>：每 IDLE_INTERVAL_S 秒对进程树采 1 点，
# 共 <seconds> 秒；输出 measurements/idle_process_tree_r0<NN>.json。
# 正式模式在 ready 后调用；fixture 模式用固定样本验证编排与统计。
run_idle_sampling() {
  local root_pid="$1" seconds="$2" points
  points=$((seconds / IDLE_INTERVAL_S))
  [ "$points" -ge 1 ] || points=1
  local i=0 now snap rss fd
  local -a rss_vals=() fd_vals=()
  while [ $i -lt "$points" ]; do
    sleep "$IDLE_INTERVAL_S"
    now="$(monotonic_ms)"
    snap="$(snapshot_process_tree "$root_pid")"
    rss="$(printf '%s' "$snap" | python3 -c 'import json,sys; print(json.load(sys.stdin)["total_rss_kib"])')"
    fd="$(printf '%s' "$snap" | python3 -c 'import json,sys; print(json.load(sys.stdin)["total_fd_count"])')"
    rss_vals+=("$rss")
    fd_vals+=("$fd")
    printf '{"metric":"idle_process_tree","run_id":"%s","sample":%d,"clock":"CLOCK_MONOTONIC","ts_ms":%s,"data":%s}\n' \
      "$RUN_ID" "$((i + 1))" "$now" "$snap" \
      >"$RUN_DIR/measurements/idle_process_tree_r$(printf '%02d' "$((i + 1))").json"
    if [ "$rss" -le 0 ] || [ "$fd" -le 0 ]; then
      echo "[M0-0.b] idle sample $((i + 1)) invalid: rss=$rss fd=$fd" >&2
      MEASUREMENTS_OK="FAIL"
    fi
    i=$((i + 1))
  done
  local rss_stats fd_stats rss_samples fd_samples
  rss_stats="$(compute_stats "${rss_vals[@]}")"
  fd_stats="$(compute_stats "${fd_vals[@]}")"
  rss_samples="$(python3 -c 'import json,sys; print(json.dumps([int(value) for value in sys.argv[1:]]))' "${rss_vals[@]}")"
  fd_samples="$(python3 -c 'import json,sys; print(json.dumps([int(value) for value in sys.argv[1:]]))' "${fd_vals[@]}")"
  printf '{"metric":"idle_process_tree","run_id":"%s","interval_s":%d,"duration_s":%d,"sample_count":%d,"rss_kib":{"samples":%s,"stats":%s},"fd_count":{"samples":%s,"stats":%s}}\n' \
    "$RUN_ID" "$IDLE_INTERVAL_S" "$seconds" "${#rss_vals[@]}" "$rss_samples" "$rss_stats" "$fd_samples" "$fd_stats" \
    >"$RUN_DIR/measurements/idle_process_tree.json"
}

# ---------------------------------------------------------------------------
# 驱动能力：资源循环编排（契约 §6.2）
# ---------------------------------------------------------------------------
# run_resource_cycle <kind> <root_pid>：kind ∈ tab|grid|terminal。
# 预热 DEFAULT_WARMUP_SAMPLES 次 + 正式 VR_CYCLE_SAMPLES 次；每次循环固定：
#   创建场景 -> 稳定 2 秒 -> 关闭 -> 稳定 2 秒 -> 采样进程树。
# 收集正式样本 RSS（进程树总 KiB），输出 OLS 斜率、FD delta、orphan 检测与原始样本。
# 当前产品钩子缺失，正式模式不会调用本函数；fixture 模式用合成样本验证统计路径。
run_resource_cycle() {
  local kind="$1" root_pid="$2"
  local warmup="$DEFAULT_WARMUP_SAMPLES" formal="${VR_CYCLE_SAMPLES:-$DEFAULT_CYCLE_SAMPLES}"
  local i rss fd snap j
  local -a rss_vals=() fd_delta_vals=()
  local formal_json='[]'

  for j in $(seq 1 $((warmup + formal))); do
    # 场景操作（tab/grid/terminal 创建与关闭）由 M0-0.b 与产品 ready 协议接通后填充；
    # 本检查点仅编码时序骨架（创建 -> 稳定 2s -> 关闭 -> 稳定 2s -> 采样）。
    # 正式基线采集（M0-0.b/c）会在此调用真实场景驱动。
    sleep 0
    if [ "$j" -gt "$warmup" ]; then
      snap="$(snapshot_process_tree "$root_pid")"
      rss="$(printf '%s' "$snap" | python3 -c 'import json,sys; print(json.load(sys.stdin)["total_rss_kib"])')"
      fd="$(printf '%s' "$snap" | python3 -c 'import json,sys; print(json.load(sys.stdin)["total_fd_count"])')"
      rss_vals+=("$rss")
      fd_delta_vals+=("$fd")
      # 每循环追加一条样本；本函数仅由 fixture/未来正式路径调用，开销可接受
      local rec
      rec="$(python3 -c 'import json,sys; print(json.dumps({"cycle": int(sys.argv[1]), "rss_kib": int(sys.argv[2]), "fd_count": int(sys.argv[3])}))' "$((j - warmup))" "$rss" "$fd")"
      formal_json="$(python3 -c 'import json,sys; a=json.loads(sys.argv[1]); a.append(json.loads(sys.argv[2])); print(json.dumps(a))' "$formal_json" "$rec")"
    fi
  done

  # 正式样本统计（契约 §7）
  local stats
  stats="$(compute_stats "${rss_vals[@]}")"
  local fd_stats
  fd_stats="$(compute_stats "${fd_delta_vals[@]}")"

  # FD delta = 末值 - 初值（契约 §5 resource_cycle_fd_delta）
  local fd_first fd_last
  fd_first="${fd_delta_vals[0]:-0}"
  fd_last="${fd_delta_vals[${#fd_delta_vals[@]}-1]:-0}"

  printf '{"metric":"%s_cycle_rss_slope_kib","run_id":"%s","kind":"%s","warmup":%d,"formal":%d,"stats":%s,"fd_delta":%d,"fd_stats":%s,"samples":%s}\n' \
    "$kind" "$RUN_ID" "$kind" "$warmup" "$formal" "$stats" "$((fd_last - fd_first))" "$fd_stats" "$formal_json" \
    >"$RUN_DIR/measurements/${kind}_cycle.json"
}

# ---------------------------------------------------------------------------
# 驱动能力：终端吞吐（契约 §6.3）
# ---------------------------------------------------------------------------
# run_terminal_throughput <root_pid>：提交固定 10 MiB ASCII 负载（begin/end 标记），
# 计时终点为前端收到 end 标记并完成下一次 animation frame（M0-0.b 协议接通后填充）。
# 当前终端 begin/end 钩子缺失，正式模式不调用；fixture 模式用合成样本验证统计路径。
run_terminal_throughput() {
  local root_pid="$1"
  local warmup=1 formal="${VR_TERM_SAMPLES:-$DEFAULT_TERM_SAMPLES}"
  local i j
  local -a elapsed_vals=() frame_gap_vals=()
  local elapsed frame_gap_max frame_gap_p95

  for j in $(seq 1 $((warmup + formal))); do
    # M0-0.b 接通后：printf '__M0_TERM_BEGIN__\n'; head -c $((TERM_LOAD_MIB*1024*1024)) /dev/zero | tr '\0' 'x'; printf '\n__M0_TERM_END__\n'
    # 前端收到 end 标记并完成下一次 animation frame 时记录 elapsed；帧间隔同步采样。
    sleep 0
    if [ "$j" -gt "$warmup" ]; then
      elapsed_vals+=("1000")
      frame_gap_vals+=("16")
    fi
  done
  compute_stats "${elapsed_vals[@]}" >/dev/null 2>&1 || true

  local estats fstats
  estats="$(compute_stats "${elapsed_vals[@]}")"
  fstats="$(compute_stats "${frame_gap_vals[@]}")"
  frame_gap_p95="$(printf '%s' "$fstats" | python3 -c 'import json,sys; d=json.load(sys.stdin); print("null" if d["p95"] is None else d["p95"])')"
  frame_gap_max="$(printf '%s' "$fstats" | python3 -c 'import json,sys; print(json.load(sys.stdin)["max"])')"

  printf '{"metric":"terminal_10mib_elapsed_ms","run_id":"%s","load_mib":%d,"begin_marker":"__M0_TERM_BEGIN__","end_marker":"__M0_TERM_END__","warmup":%d,"formal":%d,"stats":%s,"frame_gap_p95_ms":%s,"frame_gap_max_ms":%s}\n' \
    "$RUN_ID" "$TERM_LOAD_MIB" "$warmup" "$formal" "$estats" "$frame_gap_p95" "$frame_gap_max" \
    >"$RUN_DIR/measurements/terminal_throughput.json"
}

# ---------------------------------------------------------------------------
# 输出：environment.json（契约 §4，字段与 baseline-check.sh 一致）
# ---------------------------------------------------------------------------
write_environment_json() {
  python3 - "$RUN_DIR/environment.json" <<'PY'
import json, os, sys
out = sys.argv[1]
def g(k, default=""):
    return os.environ.get(k, default)

data = {
    "run_id": g("RUN_ID"),
    "timestamp": g("RUN_TS"),
    "contract_version": g("CONTRACT_VERSION"),
    "script_version": g("SCRIPT_VERSION"),
    "run_mode": g("M0_RUN_MODE", "formal"),
    "git": {
        "commit_sha": g("GIT_FULL_SHA"),
        "short_sha": g("GIT_SHORT_SHA"),
        "branch": g("GIT_BRANCH"),
        "git_status_porcelain": g("GIT_PORCELAIN"),
        "repo_root": g("ROOT"),
    },
    "artifacts": {
        "profile": "release",
        "binary_path": g("BIN_PATH"),
        "binary_sha256": g("RELEASE_BIN_SHA256"),
        "binary_bytes": int(g("RELEASE_BIN_BYTES", "0")),
        "expected_binary_sha256": g("M0_EXPECTED_BINARY_SHA256"),
        "matches_expected": g("BINARY_MATCH", "false") == "true",
        "build_cmd": "cargo build --manifest-path {root}/src-tauri/Cargo.toml --release --locked".format(root=g("ROOT")),
        "start_cmd": "WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11 {bin}".format(bin=g("BIN_PATH")),
    },
    "locks": {
        "cargo_lock_sha256": g("ENV_CARGO_LOCK_SHA"),
        "npm_lock_sha256": g("ENV_NPM_LOCK_SHA"),
    },
    "system": {
        "os": g("ENV_OS"),
        "kernel": g("ENV_KERNEL"),
        "arch": g("ENV_ARCH"),
        "cpu_model": g("ENV_CPU_MODEL"),
        "logical_cpus": g("ENV_NPROC"),
        "mem_bytes": g("ENV_MEM_BYTES"),
    },
    "graphics": {
        "xdg_session_type": g("ENV_XDG_SESSION"),
        "display": g("ENV_DISPLAY"),
        "wayland": g("ENV_WAYLAND"),
        "resolution": g("ENV_RESOLUTION"),
        "scale": g("ENV_SCALE"),
        "gdk_backend": g("ENV_GDK_BACKEND"),
        "host_gdk_backend": g("ENV_HOST_GDK_BACKEND"),
    },
    "webkit": {
        "webkit_disable_dmabuf_renderer": g("ENV_WEBKIT_DMABUF"),
        "host_webkit_disable_dmabuf_renderer": g("ENV_HOST_WEBKIT_DMABUF"),
        "webkitgtk_version": g("ENV_WEBKITGTK"),
        "gtk_version": g("ENV_GTK"),
    },
    "toolchain": {
        "rustc": g("ENV_RUSTC"),
        "cargo": g("ENV_CARGO"),
        "node": g("ENV_NODE"),
        "npm": g("ENV_NPM"),
    },
    "runtime_conditions": {
        "locale": g("ENV_LOCALE"),
        "timezone": g("ENV_TIMEZONE"),
        "load_average": g("ENV_LOADAVG"),
        "avail_mem_bytes": g("ENV_MEMAVAIL"),
        "power_mode": g("ENV_POWER"),
        "isolated_xdg_data": g("ENV_ISOLATED_XDG_DATA"),
        "isolated_xdg_cache": g("ENV_ISOLATED_XDG_CACHE"),
        "isolated_xdg_config": g("ENV_ISOLATED_XDG_CONFIG"),
    },
    "source_fingerprint": {
        "commit_sha": g("GIT_FULL_SHA"),
        "cargo_lock_sha256": g("ENV_CARGO_LOCK_SHA"),
        "npm_lock_sha256": g("ENV_NPM_LOCK_SHA"),
        "release_binary_sha256": g("RELEASE_BIN_SHA256"),
    },
    "environment_fingerprint": {
        "os": g("ENV_OS"),
        "kernel": g("ENV_KERNEL"),
        "arch": g("ENV_ARCH"),
        "cpu_model": g("ENV_CPU_MODEL"),
        "logical_cpus": g("ENV_NPROC"),
        "mem_bytes": g("ENV_MEM_BYTES"),
        "xdg_session_type": g("ENV_XDG_SESSION"),
        "resolution": g("ENV_RESOLUTION"),
        "scale": g("ENV_SCALE"),
        "gdk_backend": g("ENV_GDK_BACKEND"),
        "host_gdk_backend": g("ENV_HOST_GDK_BACKEND"),
        "webkit_disable_dmabuf_renderer": g("ENV_WEBKIT_DMABUF"),
        "host_webkit_disable_dmabuf_renderer": g("ENV_HOST_WEBKIT_DMABUF"),
        "webkitgtk_version": g("ENV_WEBKITGTK"),
        "gtk_version": g("ENV_GTK"),
        "rustc": g("ENV_RUSTC"),
        "cargo": g("ENV_CARGO"),
        "node": g("ENV_NODE"),
        "npm": g("ENV_NPM"),
        "locale": g("ENV_LOCALE"),
        "timezone": g("ENV_TIMEZONE"),
        "power_mode": g("ENV_POWER"),
    },
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, indent=2)
    f.write("\n")
print("environment.json written")
PY
}

write_scenario_json() {
  python3 - "$RUN_DIR/scenario.json" <<'PY'
import json, os, sys
out = sys.argv[1]
def g(k, d=""):
    return os.environ.get(k, d)
data = {
    "contract_version": g("CONTRACT_VERSION"),
    "scenario_version": g("SCENARIO_VERSION"),
    "checkpoint": "M0-1.b",
    "run_mode": g("M0_RUN_MODE", "formal"),
    "startup": {
        "spawn": "WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11 {bin}".format(bin=g("BIN_PATH")),
        "ready_signal": "带 run_id 的 ready 信号（契约 §6.1：mount + 2x rAF + 1x IPC 往返）",
        "ready_timeout_s": int(g("T_READY", "120")),
    },
    "idle": {
        "interval_s": int(g("IDLE_INTERVAL_S", "5")),
        "duration_s": int(g("VR_IDLE_SECONDS", "60")),
        "metrics": ["idle_process_tree_rss_kib", "idle_process_tree_fd_count"],
    },
    "resource_cycle": {
        "kinds": ["tab", "grid", "terminal"],
        "warmup_samples": int(g("DEFAULT_WARMUP_SAMPLES", "5")),
        "formal_samples": int(g("VR_CYCLE_SAMPLES", "20")),
        "settle_s": 2,
        "sampling_handshake": ["prepare", "opened", "done", "sampled"],
        "orphan_detection": "opened-baseline candidates rechecked by /proc/<pid>/stat starttime after close",
        "metrics": ["<kind>_cycle_rss_slope_kib", "resource_cycle_fd_delta", "orphan_process_count"],
    },
    "terminal_throughput": {
        "load_mib": int(g("TERM_LOAD_MIB", "10")),
        "warmup_samples": 1,
        "formal_samples": int(g("DEFAULT_TERM_SAMPLES", "3")),
        "begin_marker": "__M0_TERM_BEGIN__",
        "end_marker": "__M0_TERM_END__",
        "metrics": ["terminal_10mib_elapsed_ms", "terminal_frame_gap_p95_ms", "terminal_frame_gap_max_ms"],
        "timeout_s": int(g("T_TERM", "300")),
    },
    "stats": {
        "median": "sorted median",
        "volatility": "(max-min)/median*100% (median=0 -> abs-diff)",
        "p95": "nearest-rank, 仅 >=20 样本",
        "slope": "OLS, 单位 KiB/cycle; 同时报告 last-first 与末/初百分比",
    },
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, indent=2)
    f.write("\n")
print("scenario.json written")
PY
}

# ---------------------------------------------------------------------------
# 输出：summary.json / summary.md / SHA256SUMS / measurements
# ---------------------------------------------------------------------------
write_summary_json() {
  export_for_python \
    READY_HOOK TERM_HOOK RUN_ID RUN_TS CONTRACT_VERSION SCRIPT_VERSION SCENARIO_VERSION \
    ROOT GIT_FULL_SHA GIT_SHORT_SHA GIT_BRANCH GIT_PORCELAIN BIN_PATH RELEASE_BIN_SHA256 RELEASE_BIN_BYTES \
    M0_EXPECTED_BINARY_SHA256 BINARY_MATCH M0_RUN_MODE WORKTREE_CLEAN_AT_START EVIDENCE_RUN_DIR_LABEL \
    VR_IDLE_SECONDS VR_CYCLE_SAMPLES DEFAULT_TERM_SAMPLES DRIVER_SELFCHECK DRIVER_SELFCHECK_DETAIL
  python3 - "$RUN_DIR/summary.json" <<'PY'
import json, os, sys
out = sys.argv[1]
def g(k, d=""):
    v = os.environ.get(k)
    return d if v is None else v

ready_blocked = (g("READY_HOOK") == "BLOCKED")
term_blocked = (g("TERM_HOOK") == "BLOCKED")

blocked = []
if ready_blocked:
    blocked.append({"metric": "startup_ready_ms", "status": "BLOCKED",
                    "owner": "M0-0.b", "unblock": "实现产品 ready 钩子（契约 §6.1：mount+2×rAF+IPC 往返后写带 run_id 的 ready 信号）"})
    for m in ["idle_process_tree_rss_kib", "idle_process_tree_fd_count",
              "tab_cycle_rss_slope_kib", "grid_cycle_rss_slope_kib", "terminal_cycle_rss_slope_kib",
              "resource_cycle_fd_delta", "orphan_process_count"]:
        blocked.append({"metric": m, "status": "BLOCKED", "owner": "M0-0.b",
                        "unblock": "ready 钩子落地后由 verify-resources.sh 采集（契约 §6.1/§6.2）"})
if term_blocked:
    for m in ["terminal_10mib_elapsed_ms", "terminal_frame_gap_p95_ms", "terminal_frame_gap_max_ms"]:
        blocked.append({"metric": m, "status": "BLOCKED", "owner": "M0-0.b",
                        "unblock": "实现终端 begin/end 标记（契约 §6.3：__M0_TERM_BEGIN__/__M0_TERM_END__）"})

deferred = [
    {"metric": "script_first_response_ms", "status": "DEFERRED(M2-4)"},
    {"metric": "database_first_row_ms", "status": "DEFERRED(M4-3)"},
]

# M0-1.b/M0-0.b 检查点判定：驱动能力自检必须 PASS；若钩子缺失则 status=BLOCKED
# （门禁未通过，正式模式退出 1，符合「明确返回 BLOCKED 而非伪造成功」）；
# 钩子就绪但正式测量不完整（MEASUREMENTS_OK=FAIL）则整批 FAIL（契约 §9 M0-0.b）。
driver_ok = (g("DRIVER_SELFCHECK", "FAIL") == "PASS")
measurements_ok = (g("MEASUREMENTS_OK", "FAIL") == "PASS")
if not driver_ok:
    overall = "FAIL"
elif ready_blocked or term_blocked:
    overall = "BLOCKED"
elif not measurements_ok:
    overall = "FAIL"
elif g("M0_RUN_MODE", "formal") == "smoke":
    overall = "EXPLORATORY"
else:
    overall = "PASS"

data = {
    "run_id": g("RUN_ID"),
    "timestamp": g("RUN_TS"),
    "checkpoint": "M0-1.b",
    "contract_version": g("CONTRACT_VERSION"),
    "script_version": g("SCRIPT_VERSION"),
    "scenario_version": g("SCENARIO_VERSION"),
    "run_mode": g("M0_RUN_MODE", "formal"),
    "sample_profile": {
        "frontend_formal_samples": 0,
        "idle_seconds": int(g("VR_IDLE_SECONDS", "0")),
        "resource_cycle_formal_samples": int(g("VR_CYCLE_SAMPLES", "0")),
        "terminal_formal_samples": int(g("DEFAULT_TERM_SAMPLES", "0")),
    },
    "artifact": {
        "profile": "release",
        "binary_sha256": g("RELEASE_BIN_SHA256"),
        "binary_bytes": int(g("RELEASE_BIN_BYTES", "0")),
        "expected_binary_sha256": g("M0_EXPECTED_BINARY_SHA256"),
        "matches_expected": g("BINARY_MATCH", "false") == "true",
    },
    "repo_root": g("ROOT"),
    "worktree_clean_at_start": g("WORKTREE_CLEAN_AT_START", "false") == "true",
    "git": {"commit_sha": g("GIT_FULL_SHA"), "short_sha": g("GIT_SHORT_SHA"), "branch": g("GIT_BRANCH")},
    "status": overall,
    "driver_selfcheck": {"status": g("DRIVER_SELFCHECK", "FAIL"), "detail": g("DRIVER_SELFCHECK_DETAIL")},
    "measurements": {"ok": measurements_ok},
    "blocked": blocked,
    "deferred": deferred,
    "evidence": {
        "run_dir": g("EVIDENCE_RUN_DIR_LABEL"),
        "environment": "environment.json",
        "scenario": "scenario.json",
        "summary_json": "summary.json",
        "summary_md": "summary.md",
        "sha256sums": "SHA256SUMS",
    },
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, indent=2)
    f.write("\n")
print("summary.json written with status=%s" % overall)
PY
  SUMMARY_STATUS="$(python3 -c 'import json; print(json.load(open("'"$RUN_DIR"'/summary.json"))["status"])')"
}

write_summary_md() {
  local f="$RUN_DIR/summary.md"
  {
    echo "# M0-1.b resource verification driver — $RUN_ID"
    echo ""
    echo "> 检查点：M0-1.b；契约：$CONTRACT_VERSION；脚本：$SCRIPT_VERSION；场景：$SCENARIO_VERSION"
    echo "> 生成时间：$(date '+%Y-%m-%d %H:%M:%S %z')"
    echo "> 运行模式：$M0_RUN_MODE（smoke 只能生成 EXPLORATORY）"
    echo "> 机器判定源：summary.json（本文件仅人类阅读；两者不一致时整批 FAIL）"
    echo ""
    echo "## 总体状态：**$SUMMARY_STATUS**"
    echo ""
    echo "驱动能力自检：$DRIVER_SELFCHECK（$DRIVER_SELFCHECK_DETAIL）"
    echo ""
    echo "产品钩子：ready=$READY_HOOK term=$TERM_HOOK"
    echo ""
    echo "| 指标 | 状态 | 说明 |"
    echo "|------|------|------|"
    if [ "$READY_HOOK" = "READY" ] && [ "$TERM_HOOK" = "READY" ] && [ "${MEASUREMENTS_OK:-}" = "PASS" ]; then
      echo "| startup_ready_ms | MEASURED | measurements/startup_ready.json |"
      echo "| idle_process_tree_rss_kib / fd_count | MEASURED | measurements/idle_process_tree_r01.json |"
      echo "| tab/grid/terminal_cycle_rss_slope_kib | MEASURED | measurements/{tab,grid,terminal}_cycle.json |"
      echo "| resource_cycle_fd_delta / orphan_process_count | MEASURED | measurements/*_cycle.json + orphan_*.json |"
      echo "| terminal_10mib_elapsed_ms / frame_gap_* | MEASURED | measurements/terminal_throughput.json |"
    elif [ "$READY_HOOK" = "BLOCKED" ] || [ "$TERM_HOOK" = "BLOCKED" ]; then
      echo "| startup_ready_ms | BLOCKED | 需 ready 钩子（M0-0.b 补齐） |"
      echo "| idle_process_tree_rss_kib / fd_count | BLOCKED | 需 ready 钩子（M0-0.b 补齐） |"
      echo "| tab/grid/terminal_cycle_rss_slope_kib | BLOCKED | 需 ready 钩子（M0-0.b 补齐） |"
      echo "| resource_cycle_fd_delta / orphan_process_count | BLOCKED | 需 ready 钩子（M0-0.b 补齐） |"
      echo "| terminal_10mib_elapsed_ms / frame_gap_* | BLOCKED | 需终端 begin/end 标记（M0-0.b 补齐） |"
    else
      echo "| startup/idle/resource/terminal metrics | FAIL | 详见 raw/ 与 measurements/；任一无效样本整批失败 |"
    fi
    echo ""
    echo "## 证据"
    echo ""
    echo "- run 目录：\`$EVIDENCE_RUN_DIR_LABEL\`"
    echo "- environment.json / scenario.json / summary.json / summary.md / SHA256SUMS"
    echo "- raw/：命令原始 stdout/stderr；measurements/：结构化测量；commands/：完整命令行"
    echo ""
  } >"$f"
}

write_commands_txt() {
  local f="$RUN_DIR/commands/commands.txt"
  {
    echo "M0-1.b commands — run_id=$RUN_ID"
    echo "generated=$(date '+%Y-%m-%d %H:%M:%S %z')"
    echo ""
    echo "# 启动（契约 §6.1；ready 钩子由 M0-0.b 补齐）"
    echo "WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11 $BIN_PATH"
    echo "# idle 采样（契约 §6.1；每 5 秒 1 点，持续 ${VR_IDLE_SECONDS:-60} 秒）"
    echo "snapshot_process_tree <root_pid> -> VmRSS / FD / starttime"
    echo "# 资源循环（契约 §6.2；tab/grid/terminal，5 预热 + ${VR_CYCLE_SAMPLES:-20} 正式）"
    echo "# 终端吞吐（契约 §6.3；${TERM_LOAD_MIB} MiB，__M0_TERM_BEGIN__/__M0_TERM_END__）"
    echo "# 指纹"
    echo "environment.json / scenario.json（本 run）"
  } >"$f"
}

write_measurements() {
  # 驱动能力自检结果归档（正式模式 BLOCKED 时亦保留，证明驱动初始化完成）
  # 用 python json.dump 写：detail 内含未转义双引号，printf 拼会产生非法 JSON。
  DRIVER_SELFCHECK="$DRIVER_SELFCHECK" DRIVER_SELFCHECK_DETAIL="$DRIVER_SELFCHECK_DETAIL" \
  READY_HOOK="$READY_HOOK" TERM_HOOK="$TERM_HOOK" RUN_ID="$RUN_ID" \
  OUT_DIR="$RUN_DIR/measurements" python3 - <<'PY'
import json, os
d = os.environ["OUT_DIR"]
with open(os.path.join(d, "driver_selfcheck.json"), "w", encoding="utf-8") as f:
    json.dump({"metric": "driver_selfcheck", "status": os.environ["DRIVER_SELFCHECK"],
               "detail": os.environ["DRIVER_SELFCHECK_DETAIL"],
               "run_id": os.environ["RUN_ID"]}, f, ensure_ascii=False)
    f.write("\n")
with open(os.path.join(d, "product_hooks.json"), "w", encoding="utf-8") as f:
    json.dump({"metric": "product_hooks", "ready_hook": os.environ["READY_HOOK"],
               "term_hook": os.environ["TERM_HOOK"],
               "run_id": os.environ["RUN_ID"]}, f, ensure_ascii=False)
    f.write("\n")
PY
}

write_sha256sums() {
  (cd "$RUN_DIR" && find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 -n1 sha256sum > SHA256SUMS)
}

# ---------------------------------------------------------------------------
# 驱动能力冒烟（正式模式执行一次，证明驱动在真实 /proc 环境可用；
# 不采集任何产品基线，只 spawn 一个临时 sleep 子进程做进程树 fixture）
# ---------------------------------------------------------------------------
driver_smoke() {
  local child snap stats ok detail
  sleep 3 &
  child=$!
  snap="$(snapshot_process_tree "$child")"
  stats="$(compute_stats 10 20 30 || true)"
  ok="$(python3 -c 'import json,sys; d=json.loads(sys.argv[1]); print("1" if d.get("root_pid")==int(sys.argv[2]) and d.get("unreadable")==[] else "0")' "$snap" "$child")"
  wait "$child" 2>/dev/null || true
  detail="proc-snapshot(root_pid=$child,members=$(printf '%s' "$snap" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["members"]))'),rss/fd readable=$ok); stats(10,20,30)->$stats"
  if [ "$ok" = "1" ]; then
    DRIVER_SELFCHECK="PASS"
    DRIVER_SELFCHECK_DETAIL="$detail"
  else
    DRIVER_SELFCHECK="FAIL"
    DRIVER_SELFCHECK_DETAIL="$detail"
  fi
}

# ---------------------------------------------------------------------------
# M0-0.b 正式测量：产品 ready/终端钩子落地后接通契约 §6.1/§6.2/§6.3 真实采集。
# 与产品钩子的标记文件协议（M0_DRIVER 驱动，见 src-tauri/src/main.rs run_m0_driver）：
#   - ready 信号：<report_dir>/startup-*/ready.signal（前端 mount+2×rAF+IPC 后写入）
#   - 循环完成标记：<report_dir>/cycle-<kind>/<kind>.cycle-<NN>.done
#   - 驱动结果：<report_dir>/cycle-<kind>/<kind>.driver.result = PASS|FAIL
#   - 吞吐报告：<report_dir>/term-<n>/term-throughput-report.json
# ---------------------------------------------------------------------------

# 用隔离 XDG + M0 环境变量 spawn release 应用；输出 root_pid。
# <driver> 可为空（startup/idle 场景不驱动）。输出 pid 到 stdout。
spawn_m0_app() {
  local driver="$1" run_id="$2" ready_file="$3" report_dir="$4" xdg="$5"
  mkdir -p "$xdg"/data "$xdg"/cache "$xdg"/config
  local env_args=(
    XDG_DATA_HOME="$xdg/data" XDG_CACHE_HOME="$xdg/cache" XDG_CONFIG_HOME="$xdg/config"
    M0_RUN_ID="$run_id" M0_READY_FILE="$ready_file" M0_REPORT_DIR="$report_dir"
    WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11
  )
  if [ -n "$driver" ]; then
    # term-throughput 由脚本逐样本独立 spawn（每次启动前端只驱动一轮 10 MiB 负载），
    # 驱动必须只跑 1 轮；否则第 2 轮会删除报告文件空等，与脚本 wait_file 形成竞态。
    local cycles="$((DEFAULT_WARMUP_SAMPLES + ${VR_CYCLE_SAMPLES:-$DEFAULT_CYCLE_SAMPLES}))"
    if [ "$driver" = "term-throughput" ]; then
      cycles=1
    fi
    env_args+=(M0_DRIVER="$driver" M0_CYCLES="$cycles")
  fi
  env "${env_args[@]}" "$BIN_PATH" >"$report_dir/app.log" 2>&1 &
  echo $!
}

# 轮询 ready 信号：首行 == run_id 即成功。
wait_ready_signal() {
  local file="$1" run_id="$2" max_s="$3"
  local i=0
  while [ "$i" -lt "$((max_s * 2))" ]; do
    if [ -f "$file" ] && [ "$(head -1 "$file" 2>/dev/null)" = "$run_id" ]; then
      return 0
    fi
    sleep 0.5
    i=$((i + 1))
  done
  return 1
}

# 轮询文件出现（最多 max_s 秒）。返回 0=出现。
wait_file() {
  local file="$1" max_s="$2"
  local i=0
  while [ "$i" -lt "$((max_s * 2))" ]; do
    [ -f "$file" ] && return 0
    sleep 0.5
    i=$((i + 1))
  done
  return 1
}

# 契约 §6.1：startup_ready_ms（1 预热 + 3 正式独立进程）+ idle 采样（第 3 个正式进程
# ready 后不操作 30 秒，再按 VR_IDLE_SECONDS 采 60 秒进程树 RSS/FD）。
run_startup_ready() {
  local i xdg rd ready pid start_ms ready_ms elapsed
  local -a vals=()
  for i in 1 2 3 4; do
    xdg="$XDG_BASE/startup-$i"
    rd="$REPORT_DIR/startup-$i"
    mkdir -p "$xdg" "$rd"
    ready="$rd/ready.signal"
    rm -f "$ready"
    start_ms="$(monotonic_ms)"
    pid="$(spawn_m0_app "" "$RUN_ID" "$ready" "$rd" "$xdg")"
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "[M0-0.b] startup-$i: app failed to spawn" >&2
      MEASUREMENTS_OK="FAIL"
      cp "$rd/app.log" "$RUN_DIR/raw/startup_ready_r$(printf '%02d' "$i")_app.log" 2>/dev/null || true
      return 1
    fi
    if ! wait_ready_signal "$ready" "$RUN_ID" "$T_READY"; then
      echo "[M0-0.b] startup-$i: ready timeout" >&2
      MEASUREMENTS_OK="FAIL"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      cp "$rd/app.log" "$RUN_DIR/raw/startup_ready_r$(printf '%02d' "$i")_app.log" 2>/dev/null || true
      return 1
    fi
    ready_ms="$(monotonic_ms)"
    elapsed=$((ready_ms - start_ms))
    if [ "$i" -gt 1 ]; then
      vals+=("$elapsed")
      printf '{"metric":"startup_ready_ms","run_id":"%s","sample":%d,"clock":"CLOCK_MONOTONIC","elapsed_ms":%d,"pid":%d}\n' \
        "$RUN_ID" "$((i - 1))" "$elapsed" "$pid" \
        >"$RUN_DIR/measurements/startup_ready_r$(printf '%02d' "$((i - 1))").json"
    fi
    if [ "$i" -lt 4 ]; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    else
      # ready 后不操作 30 秒，再按 5 秒间隔采样 60 秒（契约 §6.1 第 5 条）
      sleep 30
      run_idle_sampling "$pid" "$VR_IDLE_SECONDS"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
    cp "$rd/app.log" "$RUN_DIR/raw/startup_ready_r$(printf '%02d' "$i")_app.log" 2>/dev/null || true
  done
  if [ "${#vals[@]}" -gt 0 ]; then
    local stats samples
    samples="$(python3 -c 'import json,sys; print(json.dumps([int(x) for x in sys.argv[1:]]))' "${vals[@]}")"
    stats="$(compute_stats "${vals[@]}")"
    printf '{"metric":"startup_ready_ms","run_id":"%s","clock":"CLOCK_MONOTONIC","warmup":1,"formal":3,"samples":%s,"stats":%s}\n' \
      "$RUN_ID" "$samples" "$stats" >"$RUN_DIR/measurements/startup_ready.json"
  fi
}

# 契约 §6.2：单类资源循环（kind ∈ tab|grid|terminal）。独立启动应用（M0_DRIVER 驱动），
# 5 预热 + VR_CYCLE_SAMPLES 正式；每轮与产品驱动做四阶段握手：
# prepare（创建前快照）-> opened（打开态快照）-> done（关闭态快照）-> sampled（放行）。
run_resource_cycle_real() {
  local kind="$1"
  local warmup="$DEFAULT_WARMUP_SAMPLES" formal="${VR_CYCLE_SAMPLES:-$DEFAULT_CYCLE_SAMPLES}"
  local total=$((warmup + formal))
  local xdg="$XDG_BASE/cycle-$kind" rd="$REPORT_DIR/cycle-$kind"
  local ready pid n cycle_ok=1
  mkdir -p "$xdg" "$rd"
  ready="$rd/ready.signal"
  rm -f "$ready"
  pid="$(spawn_m0_app "$kind" "$RUN_ID" "$ready" "$rd" "$xdg")"
  if ! wait_file "$rd/$kind.driver.start" "$T_READY"; then
    echo "[M0-0.b] cycle-$kind: driver start timeout" >&2
    MEASUREMENTS_OK="FAIL"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    cp "$rd/app.log" "$RUN_DIR/raw/${kind}_cycle_app.log" 2>/dev/null || true
    return 1
  fi
  local -a rss_vals=() fd_vals=() orphan_vals=()
  local formal_json='[]'
  for n in $(seq 1 "$total"); do
    local tag prefix baseline_file opened_file closed_file
    tag="$(printf '%02d' "$n")"
    prefix="$rd/$kind.cycle-$tag"
    baseline_file="$prefix.baseline.json"
    opened_file="$prefix.opened.json"
    closed_file="$prefix.closed.json"

    if ! wait_file "$prefix.prepare" 90; then
      echo "[M0-0.b] cycle-$kind: #$n prepare timeout" >&2
      MEASUREMENTS_OK="FAIL"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      cp "$rd/app.log" "$RUN_DIR/raw/${kind}_cycle_app.log" 2>/dev/null || true
      return 1
    fi
    snapshot_process_tree "$pid" >"$baseline_file"
    printf 'ok\n' >"$prefix.prepare.ack"

    if ! wait_file "$prefix.opened" 90; then
      echo "[M0-0.b] cycle-$kind: #$n opened timeout" >&2
      MEASUREMENTS_OK="FAIL"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      cp "$rd/app.log" "$RUN_DIR/raw/${kind}_cycle_app.log" 2>/dev/null || true
      return 1
    fi
    snapshot_process_tree "$pid" >"$opened_file"
    printf 'ok\n' >"$prefix.opened.ack"

    if ! wait_file "$prefix.done" 90; then
      echo "[M0-0.b] cycle-$kind: #$n done timeout" >&2
      MEASUREMENTS_OK="FAIL"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      cp "$rd/app.log" "$RUN_DIR/raw/${kind}_cycle_app.log" 2>/dev/null || true
      return 1
    fi
    snapshot_process_tree "$pid" >"$closed_file"
    local comparison opened_status done_status root_alive
    comparison="$(compare_cycle_snapshots "$baseline_file" "$opened_file" "$closed_file")"
    printf 'ok\n' >"$prefix.sampled"
    opened_status="$(head -1 "$prefix.opened" 2>/dev/null || true)"
    done_status="$(head -1 "$prefix.done" 2>/dev/null || true)"
    root_alive="$(printf '%s' "$comparison" | python3 -c 'import json,sys; print("1" if json.load(sys.stdin)["root_alive"] else "0")')"
    if [ "$opened_status" != "ok" ] || [ "$done_status" != "ok" ] || [ "$root_alive" != "1" ]; then
      echo "[M0-0.b] cycle-$kind: #$n invalid state: opened=$opened_status done=$done_status root_alive=$root_alive" >&2
      MEASUREMENTS_OK="FAIL"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      cp "$rd/app.log" "$RUN_DIR/raw/${kind}_cycle_app.log" 2>/dev/null || true
      return 1
    fi

    if [ "$n" -gt "$warmup" ]; then
      local rss fd cno rec orphan_count
      cno=$((n - warmup))
      cp "$baseline_file" "$RUN_DIR/raw/${kind}_cycle_c$(printf '%02d' "$cno")_baseline.json"
      cp "$opened_file" "$RUN_DIR/raw/${kind}_cycle_c$(printf '%02d' "$cno")_opened.json"
      cp "$closed_file" "$RUN_DIR/raw/${kind}_cycle_c$(printf '%02d' "$cno")_closed.json"
      rss="$(printf '%s' "$comparison" | python3 -c 'import json,sys; print(json.load(sys.stdin)["closed_total_rss_kib"])')"
      fd="$(printf '%s' "$comparison" | python3 -c 'import json,sys; print(json.load(sys.stdin)["closed_total_fd_count"])')"
      orphan_count="$(printf '%s' "$comparison" | python3 -c 'import json,sys; print(json.load(sys.stdin)["orphan_count"])')"
      rss_vals+=("$rss")
      fd_vals+=("$fd")
      orphan_vals+=("$orphan_count")
      rec="$(python3 -c 'import json,sys; d=json.loads(sys.argv[4]); print(json.dumps({"cycle": int(sys.argv[1]), "rss_kib": int(sys.argv[2]), "fd_count": int(sys.argv[3]), "candidate_count": d["candidate_count"], "orphan_count": d["orphan_count"]}))' "$cno" "$rss" "$fd" "$comparison")"
      formal_json="$(python3 -c 'import json,sys; a=json.loads(sys.argv[1]); a.append(json.loads(sys.argv[2])); print(json.dumps(a))' "$formal_json" "$rec")"
      printf '{"metric":"orphan_process_count","run_id":"%s","kind":"%s","cycle":%d,"data":%s}\n' \
        "$RUN_ID" "$kind" "$cno" "$comparison" \
        >"$RUN_DIR/measurements/orphan_${kind}_c$(printf '%02d' "$cno").json"
      if [ "$rss" -le 0 ] || [ "$fd" -le 0 ] || [ "$orphan_count" -ne 0 ]; then
        echo "[M0-0.b] cycle-$kind: formal #$cno invalid measurement: rss=$rss fd=$fd orphan_count=$orphan_count" >&2
        cycle_ok=0
      fi
    fi
  done
  if ! wait_file "$rd/$kind.driver.result" 60; then
    echo "[M0-0.b] cycle-$kind: driver result timeout" >&2
    MEASUREMENTS_OK="FAIL"
    cycle_ok=0
  else
    local result
    result="$(head -1 "$rd/$kind.driver.result")"
    if [ "$result" != "PASS" ]; then
      echo "[M0-0.b] cycle-$kind: driver result=$result" >&2
      MEASUREMENTS_OK="FAIL"
      cycle_ok=0
    fi
  fi
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  cp "$rd/app.log" "$RUN_DIR/raw/${kind}_cycle_app.log" 2>/dev/null || true
  if [ "${#rss_vals[@]}" -ne "$formal" ] || [ "${#fd_vals[@]}" -ne "$formal" ]; then
    echo "[M0-0.b] cycle-$kind: expected $formal formal samples, got rss=${#rss_vals[@]} fd=${#fd_vals[@]}" >&2
    cycle_ok=0
  fi
  local stats fd_stats orphan_stats fd_first fd_last
  stats="$(compute_stats "${rss_vals[@]}")"
  fd_stats="$(compute_stats "${fd_vals[@]}")"
  orphan_stats="$(compute_stats "${orphan_vals[@]}")"
  fd_first="${fd_vals[0]:-0}"
  fd_last="${fd_vals[${#fd_vals[@]}-1]:-0}"
  printf '{"metric":"%s_cycle_rss_slope_kib","run_id":"%s","kind":"%s","warmup":%d,"formal":%d,"orphan_wait_s":%d,"stats":%s,"fd_delta":%d,"fd_stats":%s,"orphan_stats":%s,"samples":%s}\n' \
    "$kind" "$RUN_ID" "$kind" "$warmup" "$formal" "$ORPHAN_WAIT_S" "$stats" "$((fd_last - fd_first))" "$fd_stats" "$orphan_stats" "$formal_json" \
    >"$RUN_DIR/measurements/${kind}_cycle.json"
  if [ "$cycle_ok" != "1" ]; then
    MEASUREMENTS_OK="FAIL"
    return 1
  fi
}

# 契约 §6.3：终端吞吐（1 预热 + DEFAULT_TERM_SAMPLES 正式）。前端自动挂载终端并驱动
# 10 MiB 负载，计时终点为前端收到 end 标记并完成下一次 animation frame。
run_terminal_throughput_real() {
  local warmup=1 formal="${DEFAULT_TERM_SAMPLES:-3}"
  local total=$((warmup + formal))
  local -a elapsed_vals=()
  local all_gaps='[]'
  for n in $(seq 1 "$total"); do
    local xdg="$XDG_BASE/term-$n" rd="$REPORT_DIR/term-$n" ready pid
    mkdir -p "$xdg" "$rd"
    ready="$rd/ready.signal"
    rm -f "$ready"
    pid="$(spawn_m0_app "term-throughput" "$RUN_ID" "$ready" "$rd" "$xdg")"
    if ! wait_file "$rd/term-throughput.driver.start" "$T_READY"; then
      echo "[M0-0.b] term-$n: driver start timeout" >&2
      MEASUREMENTS_OK="FAIL"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      cp "$rd/app.log" "$RUN_DIR/raw/terminal_throughput_r$(printf '%02d' "$n")_app.log" 2>/dev/null || true
      continue
    fi
    local rfile="$rd/term-throughput-report.json"
    if ! wait_file "$rfile" "$T_TERM"; then
      echo "[M0-0.b] term-$n: report timeout" >&2
      MEASUREMENTS_OK="FAIL"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      cp "$rd/app.log" "$RUN_DIR/raw/terminal_throughput_r$(printf '%02d' "$n")_app.log" 2>/dev/null || true
      continue
    fi
    wait_file "$rd/term-throughput.driver.result" 60 || true
    local dresult
    dresult="$(head -1 "$rd/term-throughput.driver.result" 2>/dev/null || echo unknown)"
    if [ "$dresult" != "PASS" ]; then
      echo "[M0-0.b] term-$n: driver result=$dresult" >&2
      MEASUREMENTS_OK="FAIL"
    fi
    local raw_report="$RUN_DIR/raw/terminal_throughput_r$(printf '%02d' "$n").json"
    local validation="$RUN_DIR/raw/terminal_throughput_r$(printf '%02d' "$n")_validation.json"
    cp "$rfile" "$raw_report"
    local report_ok=1
    if ! validate_terminal_report "$rfile" "$RUN_ID" >"$validation"; then
      report_ok=0
      MEASUREMENTS_OK="FAIL"
      echo "[M0-0.b] term-$n: invalid throughput report: $(cat "$validation")" >&2
    fi
    if [ "$n" -gt "$warmup" ] && [ "$report_ok" = "1" ]; then
      local elapsed gaps
      elapsed="$(python3 -c 'import json; print(json.load(open("'$rfile'"))["elapsed_ms"])')"
      elapsed_vals+=("$elapsed")
      gaps="$(python3 -c 'import json; d=json.load(open("'$rfile'")); print(json.dumps(d.get("frame_gaps_ms", [])))')"
      all_gaps="$(python3 -c 'import json,sys; a=json.loads(sys.argv[1]); a += json.loads(sys.argv[2]); print(json.dumps(a))' "$all_gaps" "$gaps")"
    fi
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    cp "$rd/app.log" "$RUN_DIR/raw/terminal_throughput_r$(printf '%02d' "$n")_app.log" 2>/dev/null || true
  done
  if [ "${#elapsed_vals[@]}" -ne "$formal" ]; then
    echo "[M0-0.b] terminal throughput: expected $formal valid formal samples, got ${#elapsed_vals[@]}" >&2
    MEASUREMENTS_OK="FAIL"
  fi
  local estats fstats fgap_p95 fgap_max
  estats="$(compute_stats "${elapsed_vals[@]}")"
  fstats="$(printf '%s' "$all_gaps" | python3 -c 'import json,sys; a=json.load(sys.stdin); n=len(a); s=sorted(a); print(json.dumps({"count": n, "p95": (s[int(0.95*(n-1))] if n >= 20 else None), "max": (s[-1] if n else None)}))')"
  fgap_p95="$(printf '%s' "$fstats" | python3 -c 'import json,sys; print(json.dumps(json.load(sys.stdin)["p95"]))')"
  fgap_max="$(printf '%s' "$fstats" | python3 -c 'import json,sys; print(json.dumps(json.load(sys.stdin)["max"]))')"
  printf '{"metric":"terminal_10mib_elapsed_ms","run_id":"%s","load_mib":%d,"begin_marker":"__M0_TERM_BEGIN__","end_marker":"__M0_TERM_END__","warmup":%d,"formal":%d,"stats":%s,"frame_gap_p95_ms":%s,"frame_gap_max_ms":%s}\n' \
    "$RUN_ID" "$TERM_LOAD_MIB" "$warmup" "$formal" "$estats" "$fgap_p95" "$fgap_max" \
    >"$RUN_DIR/measurements/terminal_throughput.json"
}

# M0-0.b 编排：产品钩子 READY 时执行全部 REQUIRED_NOW 真实采集。
run_formal_measurements() {
  MEASUREMENTS_OK="PASS"
  mkdir -p "$XDG_BASE" "$REPORT_DIR"
  run_startup_ready || MEASUREMENTS_OK="FAIL"
  for kind in tab grid terminal; do
    run_resource_cycle_real "$kind" || MEASUREMENTS_OK="FAIL"
  done
  run_terminal_throughput_real || MEASUREMENTS_OK="FAIL"
  echo "[M0-0.b] measurements ok=$MEASUREMENTS_OK"
}

# ---------------------------------------------------------------------------
# 正式模式
# ---------------------------------------------------------------------------
run_formal() {
  ROOT="$(resolve_root)"
  M0_RUN_MODE="${M0_RUN_MODE:-formal}"
  VR_IDLE_SECONDS="${VR_IDLE_SECONDS:-$DEFAULT_IDLE_SECONDS}"
  VR_CYCLE_SAMPLES="${VR_CYCLE_SAMPLES:-$DEFAULT_CYCLE_SAMPLES}"
  case "$M0_RUN_MODE" in
    formal|smoke) ;;
    *) echo "error: M0_RUN_MODE must be formal or smoke" >&2; exit 2 ;;
  esac
  case "$VR_IDLE_SECONDS:$VR_CYCLE_SAMPLES" in
    *[!0-9:]*|:*|*:) echo "error: VR_IDLE_SECONDS and VR_CYCLE_SAMPLES must be positive integers" >&2; exit 2 ;;
  esac
  if [ "$VR_IDLE_SECONDS" -lt "$IDLE_INTERVAL_S" ] || [ $((VR_IDLE_SECONDS % IDLE_INTERVAL_S)) -ne 0 ] || [ "$VR_CYCLE_SAMPLES" -lt 1 ]; then
    echo "error: VR_IDLE_SECONDS must be >= $IDLE_INTERVAL_S and divisible by it; VR_CYCLE_SAMPLES must be >= 1" >&2
    exit 2
  fi
  if [ "$M0_RUN_MODE" = "formal" ] && { [ "$VR_IDLE_SECONDS" -ne "$DEFAULT_IDLE_SECONDS" ] || [ "$VR_CYCLE_SAMPLES" -ne "$DEFAULT_CYCLE_SAMPLES" ]; }; then
    echo "error: formal mode requires VR_IDLE_SECONDS=$DEFAULT_IDLE_SECONDS and VR_CYCLE_SAMPLES=$DEFAULT_CYCLE_SAMPLES; use M0_RUN_MODE=smoke for shortened runs" >&2
    exit 2
  fi

  GIT_PORCELAIN="$(git -C "$ROOT" status --porcelain=v1)"
  WORKTREE_CLEAN_AT_START="true"
  if [ -n "$GIT_PORCELAIN" ]; then
    WORKTREE_CLEAN_AT_START="false"
  fi
  if [ "$WORKTREE_CLEAN_AT_START" != "true" ] && [ "$M0_RUN_MODE" = "formal" ]; then
    echo "error: worktree not clean; formal run requires a clean commit" >&2
    echo "git status --porcelain=v1:" >&2
    echo "$GIT_PORCELAIN" >&2
    echo "note: with a dirty tree results could only be archived as EXPLORATORY" >&2
    exit 1
  fi

  GIT_FULL_SHA="$(git -C "$ROOT" rev-parse HEAD)"
  GIT_SHORT_SHA="$(git -C "$ROOT" rev-parse --short=7 HEAD)"
  GIT_BRANCH="$(git -C "$ROOT" symbolic-ref -q --short HEAD || echo "detached:$(git -C "$ROOT" rev-parse --short HEAD)")"
  RUN_TS="$(date +%Y%m%dT%H%M%S%z)"
  BACKEND="x11"
  RUN_ID="${RUN_TS}_${GIT_SHORT_SHA}_release_${BACKEND}"
  EVIDENCE_ROOT="${M0_EVIDENCE_ROOT:-$ROOT/logs/m0-baseline}"
  RUN_DIR="$EVIDENCE_ROOT/$RUN_ID"
  if [ -n "${M0_EVIDENCE_LABEL_ROOT:-}" ]; then
    EVIDENCE_RUN_DIR_LABEL="${M0_EVIDENCE_LABEL_ROOT%/}/$RUN_ID"
  elif [ "$EVIDENCE_ROOT" = "$ROOT/logs/m0-baseline" ]; then
    EVIDENCE_RUN_DIR_LABEL="logs/m0-baseline/$RUN_ID"
  else
    EVIDENCE_RUN_DIR_LABEL="$RUN_DIR"
  fi
  # XDG_BASE 必须短：grid 的 Unix socket 路径受 SUN_LEN(~108) 限制。
  XDG_BASE="/tmp/m0m/$(printf '%s' "$RUN_ID" | sha256sum | cut -c1-12)"
  REPORT_DIR="$XDG_BASE/reports"

  mkdir -p "$RUN_DIR"/{commands,raw,measurements,screenshots}
  mkdir -p "$XDG_BASE" "$REPORT_DIR"
  RAW="$RUN_DIR/raw"

  BIN_PATH="$ROOT/src-tauri/target/release/mvp-browser-os"
  RELEASE_BIN_BYTES=0
  RELEASE_BIN_SHA256=""
  if [ -f "$BIN_PATH" ]; then
    RELEASE_BIN_BYTES="$(stat -c %s "$BIN_PATH")"
    RELEASE_BIN_SHA256="$(sha256sum "$BIN_PATH" | awk '{print $1}')"
  fi
  M0_EXPECTED_BINARY_SHA256="${M0_EXPECTED_BINARY_SHA256:-}"
  BINARY_MATCH="false"
  if [ -n "$M0_EXPECTED_BINARY_SHA256" ] && [ "$RELEASE_BIN_SHA256" = "$M0_EXPECTED_BINARY_SHA256" ]; then
    BINARY_MATCH="true"
  fi
  DRIVER_SELFCHECK="FAIL"
  DRIVER_SELFCHECK_DETAIL="not-run"
  MEASUREMENTS_OK="FAIL"

  echo "[M0-1.b] run_id=$RUN_ID"
  echo "[M0-1.b] mode=$M0_RUN_MODE evidence_root=$EVIDENCE_ROOT"
  echo "[M0-1.b] repo=$ROOT branch=$GIT_BRANCH commit=$GIT_SHORT_SHA"

  detect_product_hooks
  echo "[M0-1.b] hooks: ready=$READY_HOOK term=$TERM_HOOK"

  echo "[M0-1.b] collecting environment fingerprint..."
  collect_environment

  echo "[M0-1.b] driver smoke (proc snapshot + stats, no product baseline)..."
  driver_smoke
  echo "[M0-1.b] driver selfcheck: $DRIVER_SELFCHECK"

  # M0-0.b：产品钩子落地后执行正式测量（契约 §6.1/§6.2/§6.3）
  if [ "$READY_HOOK" = "READY" ] && [ "$TERM_HOOK" = "READY" ]; then
    local artifact_ready="true"
    if [ "$RELEASE_BIN_BYTES" -le 0 ] || [ -z "$RELEASE_BIN_SHA256" ]; then
      echo "[M0-0.b] release binary missing: $BIN_PATH" >&2
      artifact_ready="false"
    elif [ -n "$M0_EXPECTED_BINARY_SHA256" ] && [ "$BINARY_MATCH" != "true" ]; then
      echo "[M0-0.b] release binary hash mismatch: expected=$M0_EXPECTED_BINARY_SHA256 actual=$RELEASE_BIN_SHA256" >&2
      artifact_ready="false"
    elif [ "$M0_RUN_MODE" = "formal" ] && [ -z "$M0_EXPECTED_BINARY_SHA256" ]; then
      echo "[M0-0.b] formal mode requires M0_EXPECTED_BINARY_SHA256 from baseline-check.sh" >&2
      artifact_ready="false"
    fi
    if [ "$artifact_ready" = "true" ]; then
      echo "[M0-0.b] product hooks and artifact ready, running $M0_RUN_MODE measurements..."
      run_formal_measurements
    else
      MEASUREMENTS_OK="FAIL"
    fi
  else
    echo "[M0-1.b] product hooks missing (ready=$READY_HOOK term=$TERM_HOOK), emitting BLOCKED evidence"
  fi

  echo "[M0-1.b] writing evidence..."
  export_for_python \
    RUN_ID RUN_TS CONTRACT_VERSION SCRIPT_VERSION SCENARIO_VERSION M0_RUN_MODE \
    ROOT GIT_FULL_SHA GIT_SHORT_SHA GIT_BRANCH GIT_PORCELAIN BIN_PATH RELEASE_BIN_SHA256 RELEASE_BIN_BYTES \
    M0_EXPECTED_BINARY_SHA256 BINARY_MATCH WORKTREE_CLEAN_AT_START EVIDENCE_RUN_DIR_LABEL \
    T_READY T_TERM IDLE_INTERVAL_S VR_IDLE_SECONDS VR_CYCLE_SAMPLES DEFAULT_WARMUP_SAMPLES \
    DEFAULT_TERM_SAMPLES TERM_LOAD_MIB MEASUREMENTS_OK \
    ENV_OS ENV_KERNEL ENV_ARCH ENV_CPU_MODEL ENV_NPROC ENV_MEM_BYTES \
    ENV_XDG_SESSION ENV_DISPLAY ENV_WAYLAND ENV_RESOLUTION ENV_SCALE ENV_GDK_BACKEND ENV_WEBKIT_DMABUF \
    ENV_HOST_GDK_BACKEND ENV_HOST_WEBKIT_DMABUF \
    ENV_WEBKITGTK ENV_GTK ENV_RUSTC ENV_CARGO ENV_NODE ENV_NPM \
    ENV_LOCALE ENV_TIMEZONE ENV_LOADAVG ENV_MEMAVAIL ENV_POWER \
    ENV_CARGO_LOCK_SHA ENV_NPM_LOCK_SHA ENV_ISOLATED_XDG_DATA ENV_ISOLATED_XDG_CACHE ENV_ISOLATED_XDG_CONFIG
  write_environment_json
  write_scenario_json
  write_measurements
  write_commands_txt
  write_summary_json
  write_summary_md
  write_sha256sums

  echo ""
  echo "=== M0-1.b summary: status=$SUMMARY_STATUS ==="
  echo "evidence dir: $RUN_DIR"
  if [ "$SUMMARY_STATUS" = "PASS" ] || [ "$SUMMARY_STATUS" = "EXPLORATORY" ]; then
    exit 0
  else
    echo "one or more metrics BLOCKED/FAILED: product hooks ready=$READY_HOOK term=$TERM_HOOK" >&2
    exit 1
  fi
}

# ---------------------------------------------------------------------------
# self-test：固定夹具验证驱动能力与 BLOCKED 语义（fixture，不生成正式证据）
# ---------------------------------------------------------------------------
run_self_test() {
  echo "[self-test] fixture mode: validating driver logic, NOT generating formal evidence"
  require_tools
  local rc=0
  tmp="$(mktemp -d /tmp/vr-self-test.XXXXXX)" # 脚本级变量：EXIT trap 需要它在函数返回后仍可引用
  trap 'rm -rf "$tmp"' EXIT

  # 用例 1：--help 退出 0
  if ! bash "$SCRIPT_PATH" --help >/dev/null 2>&1; then
    echo "FAIL: --help should exit 0"; rc=1
  else
    echo "PASS: --help exits 0"
  fi

  # 用例 2：非法参数非零退出
  if bash "$SCRIPT_PATH" --definitely-invalid >/dev/null 2>&1; then
    echo "FAIL: invalid argument should exit nonzero"; rc=1
  else
    echo "PASS: invalid argument exits nonzero"
  fi

  # 用例 3：统计计算 fixture（契约 §7：median/波动率/OLS 斜率/p95）
  local stats_ok=1
  local m1 m2 m3 m4
  m1="$(compute_stats 10 20 30)"
  [ "$(printf '%s' "$m1" | python3 -c 'import json,sys; print(json.load(sys.stdin)["median"])')" = "20.0" ] || stats_ok=0
  [ "$(printf '%s' "$m1" | python3 -c 'import json,sys; print(json.load(sys.stdin)["ols_slope"])')" = "10.0" ] || stats_ok=0
  # 20 个样本 1..20：p95 nearest-rank idx=int(0.95*19)=18 -> 19
  m2="$(compute_stats $(seq 1 20))"
  [ "$(printf '%s' "$m2" | python3 -c 'import json,sys; print(json.load(sys.stdin)["p95"])')" = "19.0" ] || stats_ok=0
  # <20 样本不伪报 p95
  m3="$(compute_stats 1 2 3)"
  [ "$(printf '%s' "$m3" | python3 -c 'import json,sys; d=json.load(sys.stdin); print("none" if d["p95"] is None else d["p95"])')" = "none" ] || stats_ok=0
  # OLS 斜率：y=2x（0..8）-> 2.0
  m4="$(compute_stats 0 2 4 6 8)"
  [ "$(printf '%s' "$m4" | python3 -c 'import json,sys; print(json.load(sys.stdin)["ols_slope"])')" = "2.0" ] || stats_ok=0
  # 波动率：(30-10)/20*100=100.0%
  [ "$(printf '%s' "$m1" | python3 -c 'import json,sys; print(json.load(sys.stdin)["volatility"])')" = "100.0%" ] || stats_ok=0
  if [ "$stats_ok" = "1" ]; then
    echo "PASS: stats fixture (median/volatility/OLS slope/p95/nearest-rank)"
  else
    echo "FAIL: stats fixture"; rc=1
  fi

  # 用例 4：进程树/RSS/FD 采样 fixture（真实 /proc）
  local child snap p4_ok
  sleep 2 &
  child=$!
  snap="$(snapshot_process_tree "$child")"
  p4_ok="$(python3 -c 'import json,sys; d=json.loads(sys.argv[1]); m=[x for x in d["members"] if x["pid"]==int(sys.argv[2])]; print("1" if len(m)==1 and m[0]["rss_kib"] is not None and m[0]["fd_count"] is not None and d["unreadable"]==[] else "0")' "$snap" "$child")"
  wait "$child" 2>/dev/null || true
  if [ "$p4_ok" = "1" ]; then
    echo "PASS: process tree snapshot (root_pid in members, rss/fd readable, no unreadable)"
  else
    echo "FAIL: process tree snapshot"; rc=1
  fi

  # 用例 5：三阶段快照差分；根进程不计孤儿，只有 opened 新增且 closed 仍存活才计数
  local orph_ok=1
  local orphan1 orphan2 ob="$tmp/orphan-baseline.json" oo="$tmp/orphan-opened.json" oc="$tmp/orphan-closed.json"
  printf '%s\n' '{"root_pid":100,"members":[{"pid":100,"starttime":10}],"unreadable":[],"total_rss_kib":10,"total_fd_count":2}' >"$ob"
  printf '%s\n' '{"root_pid":100,"members":[{"pid":100,"starttime":10},{"pid":200,"starttime":20}],"unreadable":[],"total_rss_kib":20,"total_fd_count":4}' >"$oo"
  printf '%s\n' '{"root_pid":100,"members":[{"pid":100,"starttime":10},{"pid":200,"starttime":20}],"unreadable":[],"total_rss_kib":20,"total_fd_count":4}' >"$oc"
  orphan1="$(compare_cycle_snapshots "$ob" "$oo" "$oc" closed-fixture)"
  [ "$(printf '%s' "$orphan1" | python3 -c 'import json,sys; d=json.load(sys.stdin); print("1" if d["candidate_count"] == 1 and d["orphan_count"] == 1 and d["root_alive"] else "0")')" = "1" ] || orph_ok=0
  printf '%s\n' '{"root_pid":100,"members":[{"pid":100,"starttime":10}],"unreadable":[],"total_rss_kib":10,"total_fd_count":2}' >"$oc"
  orphan2="$(compare_cycle_snapshots "$ob" "$oo" "$oc" closed-fixture)"
  [ "$(printf '%s' "$orphan2" | python3 -c 'import json,sys; print(json.load(sys.stdin)["orphan_count"])')" = "0" ] || orph_ok=0
  if [ "$orph_ok" = "1" ]; then
    echo "PASS: orphan detection (root excluded, opened candidate tracked by pid+starttime)"
  else
    echo "FAIL: orphan detection"; rc=1
  fi

  # 用例 6：终端报告必须精确匹配 run_id、10 MiB、唯一标记与非空帧样本
  local term_ok=1 term_report="$tmp/term-report.json"
  printf '%s\n' '{"run_id":"fixture-run","start_ts_ms":1000,"end_ts_ms":1100,"elapsed_ms":100,"begin_seen":1,"end_seen":1,"consumed_bytes":10485760,"expected_bytes":10485760,"frame_gaps_ms":[16.0],"valid":true,"errors":[]}' >"$term_report"
  validate_terminal_report "$term_report" fixture-run >/dev/null || term_ok=0
  printf '%s\n' '{"run_id":"fixture-run","start_ts_ms":1000,"end_ts_ms":1100,"elapsed_ms":100,"begin_seen":1,"end_seen":1,"consumed_bytes":1,"expected_bytes":10485760,"frame_gaps_ms":[16.0],"valid":true,"errors":[]}' >"$term_report"
  if validate_terminal_report "$term_report" fixture-run >/dev/null; then
    term_ok=0
  fi
  if [ "$term_ok" = "1" ]; then
    echo "PASS: terminal report validation (exact payload and provenance enforced)"
  else
    echo "FAIL: terminal report validation"; rc=1
  fi

  # 用例 7：干净 fixture 仓库正式流程 -> BLOCKED（钩子缺失）+ 证据完整 + 退出非 0
  local repo="$tmp/repo-clean"
  mkdir -p "$repo"
  git -C "$repo" init -q
  git -C "$repo" config user.name fixture
  git -C "$repo" config user.email fixture@example.invalid
  echo "fixture" >"$repo/README.md"
  git -C "$repo" add -A
  git -C "$repo" commit -qm "fixture baseline"
  local out code=0
  out="$(cd "$repo" && VR_SELF_TEST=1 bash "$SCRIPT_PATH" 2>&1)" || code=$?
  echo "$out" | sed 's/^/    [fixture] /'
  if [ $code -eq 0 ]; then
    echo "FAIL: BLOCKED formal run should exit nonzero (got 0)"; rc=1
  else
    local runid summary
    runid="$(cd "$repo" && ls logs/m0-baseline | tail -1)"
    summary="$repo/logs/m0-baseline/$runid/summary.json"
    if [ ! -f "$summary" ]; then
      echo "FAIL: summary.json not generated"; rc=1
    else
      local status b_ok ds_ok
      status="$(python3 -c 'import json;print(json.load(open("'$summary'"))["status"])')"
      b_ok="$(python3 -c 'import json;d=json.load(open("'$summary'"));print(any(b["metric"]=="startup_ready_ms" and b["status"]=="BLOCKED" and b["owner"]=="M0-0.b" for b in d["blocked"]))')"
      ds_ok="$(python3 -c 'import json;print(json.load(open("'$summary'"))["driver_selfcheck"]["status"])')"
      if [ "$status" = "BLOCKED" ] && [ "$b_ok" = "True" ] && [ "$ds_ok" = "PASS" ] \
         && [ -f "$repo/logs/m0-baseline/$runid/summary.md" ] && [ -f "$repo/logs/m0-baseline/$runid/environment.json" ]; then
        echo "PASS: BLOCKED formal run -> status=BLOCKED, startup_ready_ms owner=M0-0.b, driver selfcheck=PASS, evidence complete"
      else
        echo "FAIL: BLOCKED fixture summary mismatch status=$status blocked_ok=$b_ok driver_selfcheck=$ds_ok"; rc=1
      fi
    fi
  fi

  # 用例 8：脏工作树正式模式 -> 非零退出
  local repo2="$tmp/repo-dirty"
  mkdir -p "$repo2"
  git -C "$repo2" init -q
  git -C "$repo2" config user.name fixture
  git -C "$repo2" config user.email fixture@example.invalid
  echo "fixture" >"$repo2/README.md"
  git -C "$repo2" add -A
  git -C "$repo2" commit -qm "fixture dirty"
  echo "dirty" >"$repo2/untracked.txt"
  if (cd "$repo2" && VR_SELF_TEST=1 bash "$SCRIPT_PATH" >/dev/null 2>&1); then
    echo "FAIL: dirty worktree run should exit nonzero"; rc=1
  else
    echo "PASS: dirty worktree rejected (nonzero exit)"
  fi

  # 用例 9：SHA256SUMS 与 run 目录完整性
  local runid2
  runid2="$(ls "$tmp/repo-clean/logs/m0-baseline" | tail -1)"
  if (cd "$tmp/repo-clean/logs/m0-baseline/$runid2" && sha256sum -c SHA256SUMS >/dev/null 2>&1); then
    echo "PASS: SHA256SUMS verifies all evidence files"
  else
    echo "FAIL: SHA256SUMS verification"; rc=1
  fi

  # 用例 10：summary.json 满足固定 schema（M0-1.c）
  if [ ! -f "$SCHEMA_FILE" ]; then
    echo "FAIL: schema file missing: $SCHEMA_FILE"; rc=1
  elif [ ! -f "$VALIDATE_SUMMARY" ]; then
    echo "FAIL: validate-summary.py missing: $VALIDATE_SUMMARY"; rc=1
  else
    local runid9 summary9
    runid9="$(ls "$tmp/repo-clean/logs/m0-baseline" | tail -1)"
    summary9="$tmp/repo-clean/logs/m0-baseline/$runid9/summary.json"
    if python3 "$VALIDATE_SUMMARY" "$SCHEMA_FILE" "$summary9" >/dev/null 2>&1; then
      echo "PASS: summary.json conforms to m0-summary.schema.json"
    else
      echo "FAIL: summary.json schema validation"; rc=1
    fi
  fi

  if [ $rc -eq 0 ]; then
    echo ""
    echo "SELF_TEST_RESULT=ALL_PASS"
  else
    echo ""
    echo "SELF_TEST_RESULT=FAIL"
  fi
  return $rc
}

# ---------------------------------------------------------------------------
main() {
  require_tools
  parse_args "$@"
  run_formal
}

main "$@"
