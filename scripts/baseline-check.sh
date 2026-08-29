#!/usr/bin/env bash
#
# baseline-check.sh — M0-1.a 质量门禁：把 m0-baseline-contract-v1.md 的
# 质量、构建、来源/环境指纹与证据目录初始化编码成可复跑脚本。
#
# 范围（M0-1.a，严格限定）：
#   1. 质量：主工程 + browser-tabs 插件 cargo fmt --check；
#            一次结构化 clippy（--message-format=json），按 package_id 分组、
#            按 (lint, 文件, 行, 列) 去重后计数，绝不把人类可读 warning 行直接相加。
#   2. 构建：清空 dist/ 后 npm run build（1 次预热 + N 次正式，N 可经
#            BS_FRONTEND_SAMPLES 覆盖，契约默认 3）；dist 总字节、最大 JS 原始/ gzip 字节。
#            release 构建：cargo build --manifest-path src-tauri/Cargo.toml --release --locked，
#            记录二进制字节与 SHA-256。
#   3. 来源/环境指纹：契约 §4 的 environment.json（source_fingerprint +
#            environment_fingerprint），并创建正式采集所需的隔离 XDG 目录。
#   4. 证据目录初始化：logs/m0-baseline/<run_id>/ 下 commands/ raw/ measurements/
#            screenshots/ + environment.json + scenario.json + summary.json +
#            summary.md + SHA256SUMS（契约 §8）。
#
# 边界（禁止）：
#   - 不实现 verify-resources.sh（M0-1.b）：不采集启动/RSS/FD/资源循环/终端测量。
#   - 不修改 Rust/Vue 产品代码，不补 ready/终端测量钩子（M0-0.b）。
#   - 不宣称正式 release 性能基线（M0-0.b/c）。
#   - 不自动安装依赖、不访问公网、不删除用户数据；生成物只进入本 run 隔离目录。
#
# 正式基线要求：工作树必须干净（git status --porcelain=v1 为空）；否则拒绝并
# 非零退出（只能归档为 EXPLORATORY）。
#
# BLOCKED/DEFERRED 语义：需要产品 ready/终端钩子的指标（startup_ready_ms、
# idle_*、*_cycle_*、terminal_*、orphan_process_count）在 M0-1.a 中不采集，
# 仅在 summary 中明确标记 BLOCKED（负责人=相应 WBS、解除条件=钩子落地）；
# DEFERRED(M2-4/M4-3) 指标同样不计入本检查点 PASS 判定。
# 本检查点退出码只由 M0-1.a 覆盖的指标决定。
#
# 证据保留策略（.gitignore 已对 logs/m0-baseline/ 开例外）：
#   summary / environment / scenario / measurements / SHA256SUMS / raw 原始输出
#   全部入库可追溯；若未来要求调整原始 .log 的入库口径，先提契约版本变更。
#
# 用法：
#   scripts/baseline-check.sh            # 正式模式（工作树必须干净）
#   scripts/baseline-check.sh --help
#   scripts/baseline-check.sh --self-test   # fixture 验证脚本自身，不生成正式证据
#
# 环境变量（可选）：
#   BS_FRONTEND_SAMPLES   frontend_build_ms 正式样本数，默认 3（另加 1 次预热）
#   BS_SELF_TEST          内部使用：1 = fixture 模式（--self-test 自动设置）
#
set -euo pipefail

# ---------------------------------------------------------------------------
# 常量
# ---------------------------------------------------------------------------
SCRIPT_VERSION="M0-1.a-1"
CONTRACT_VERSION="V1.0"
SCENARIO_VERSION="M0-1.a-v1"
DEFAULT_FRONTEND_SAMPLES=3

# 命令超时（秒），scenario.json 会记录
T_FMT=300
T_CLIPPY=900
T_FRONTEND=600
T_RELEASE=5400

SCRIPT_SRC="${BASH_SOURCE[0]}"
SCRIPT_PATH="$(readlink -f "$SCRIPT_SRC")"
SCRIPT_DIR="$(dirname "$SCRIPT_PATH")"

# ---------------------------------------------------------------------------
# 工具与根目录解析
# ---------------------------------------------------------------------------
REQUIRED_TOOLS=(git cargo rustc node npm python3 gzip sha256sum timeout stat date)

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
baseline-check.sh — M0-1.a 质量/构建/指纹/证据目录门禁

用法:
  scripts/baseline-check.sh            正式模式（工作树必须干净，生成 logs/m0-baseline/<run_id>/ 证据）
  scripts/baseline-check.sh --help     显示本帮助
  scripts/baseline-check.sh --self-test 用固定夹具验证脚本自身逻辑（fixture，不生成正式证据）

可选环境变量:
  BS_FRONTEND_SAMPLES=N  frontend_build_ms 正式样本数（默认 3；另加 1 次预热）

退出码: 0 = 本检查点覆盖指标全部 PASS；1 = 任一 FAIL / 工作树非干净拒绝；2 = 非法参数
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
# 工具函数：命令包装（保存 raw 输出 + 捕获退出码）
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

# ---------------------------------------------------------------------------
# 指标：fmt
# ---------------------------------------------------------------------------
run_fmt_main() {
  if [ "${BS_SELF_TEST:-0}" = "1" ]; then
    FMT_MAIN_EXIT=0
    echo "fixture: fmt main ok" >"$RAW/fmt_main_r01.stdout.log"
    return
  fi
  run_capture fmt_main r01 -- timeout "$T_FMT" cargo fmt --manifest-path "$ROOT/src-tauri/Cargo.toml" --check
  FMT_MAIN_EXIT=$LAST_RC
}

run_fmt_plugin() {
  if [ "${BS_SELF_TEST:-0}" = "1" ]; then
    FMT_PLUGIN_EXIT=0
    echo "fixture: fmt plugin ok" >"$RAW/fmt_plugin_r01.stdout.log"
    return
  fi
  run_capture fmt_plugin r01 -- timeout "$T_FMT" cargo fmt --manifest-path "$ROOT/tauri-browser-tabs/Cargo.toml" --check
  FMT_PLUGIN_EXIT=$LAST_RC
}

# ---------------------------------------------------------------------------
# 指标：clippy（结构化 JSON 去重）
# ---------------------------------------------------------------------------
run_clippy() {
  if [ "${BS_SELF_TEST:-0}" = "1" ]; then
    cp "$SCRIPT_DIR/fixtures/clippy-sample.json" "$RAW/clippy_r01.stdout.log"
    : >"$RAW/clippy_r01.stderr.log"
  else
    run_capture clippy r01 -- timeout "$T_CLIPPY" cargo clippy --manifest-path "$ROOT/src-tauri/Cargo.toml" --message-format=json
    if [ $LAST_RC -ne 0 ]; then
      CLIPPY_MAIN_COUNT=0
      CLIPPY_PLUGIN_COUNT=0
      CLIPPY_CMD_RC=$LAST_RC
      CLIPPY_STATUS="FAIL"
      return
    fi
    CLIPPY_CMD_RC=0
  fi
  local out
  out="$(python3 - "$RAW/clippy_r01.stdout.log" <<'PY'
import json, sys
path = sys.argv[1]
main_keys, plugin_keys = set(), set()
with open(path, encoding="utf-8", errors="replace") as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except ValueError:
            continue
        if obj.get("reason") != "compiler-message":
            continue
        msg = obj.get("message") or {}
        if msg.get("level") != "warning":
            continue
        pid = obj.get("package_id") or ""
        if "mvp-browser-os" in pid:
            bucket = main_keys
        elif "tauri-plugin-browser-tabs" in pid:
            bucket = plugin_keys
        else:
            continue
        code = (msg.get("code") or {}).get("code") or msg.get("message") or ""
        spans = msg.get("spans") or []
        if spans:
            s = spans[0]
            key = (code, s.get("file_name"), s.get("line_start"), s.get("column_start"))
        else:
            key = (code, None, None, None)
        bucket.add(key)
print(json.dumps({"main": len(main_keys), "plugin": len(plugin_keys)}))
PY
)"
  CLIPPY_MAIN_COUNT="$(printf '%s' "$out" | python3 -c 'import json,sys; print(json.load(sys.stdin)["main"])')"
  CLIPPY_PLUGIN_COUNT="$(printf '%s' "$out" | python3 -c 'import json,sys; print(json.load(sys.stdin)["plugin"])')"
  CLIPPY_STATUS="PASS"
}

# ---------------------------------------------------------------------------
# 指标：前端构建 + 体积
# ---------------------------------------------------------------------------
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

run_frontend_build() {
  local samples="${BS_FRONTEND_SAMPLES:-$DEFAULT_FRONTEND_SAMPLES}"
  local t0 t1 rc i
  FRONTEND_FORMAL_MS=()

  if [ "${BS_SELF_TEST:-0}" = "1" ]; then
    FRONTEND_STATUS="PASS"
    FRONTEND_WARMUP_MS=123
    FRONTEND_FORMAL_MS=(456 457 456)
    DIST_TOTAL_BYTES=9999
    LARGEST_JS_REL="assets/index-fixture.js"
    LARGEST_JS_BYTES=3000
    LARGEST_JS_GZIP=1000
    return
  fi

  # 预热（1 次，清空 dist/）
  rm -rf "$ROOT/dist"
  t0=$(date +%s%3N)
  set +e
  (cd "$ROOT" && timeout "$T_FRONTEND" npm run build) >"$RAW/frontend_build_warmup.stdout.log" 2>"$RAW/frontend_build_warmup.stderr.log"
  rc=$?
  set -e
  t1=$(date +%s%3N)
  if [ $rc -ne 0 ]; then
    FRONTEND_STATUS="FAIL"
    FRONTEND_WARMUP_MS=$((t1 - t0))
    return
  fi
  FRONTEND_WARMUP_MS=$((t1 - t0))

  # 正式样本
  for i in $(seq 1 "$samples"); do
    rm -rf "$ROOT/dist"
    t0=$(date +%s%3N)
    set +e
    (cd "$ROOT" && timeout "$T_FRONTEND" npm run build) >"$RAW/frontend_build_r$(printf '%02d' "$i").stdout.log" 2>"$RAW/frontend_build_r$(printf '%02d' "$i").stderr.log"
    rc=$?
    set -e
    t1=$(date +%s%3N)
    FRONTEND_FORMAL_MS+=("$((t1 - t0))")
    if [ $rc -ne 0 ]; then
      FRONTEND_STATUS="FAIL"
      return
    fi
  done
  FRONTEND_STATUS="PASS"

  # 体积统计（以最后一次正式构建为准）
  DIST_TOTAL_BYTES="$(find "$ROOT/dist" -type f -printf '%s\n' 2>/dev/null | awk '{s+=$1} END {print s+0}')"
  local largest largest_bytes
  largest=""
  largest_bytes=0
  while IFS= read -r f; do
    local sz
    sz=$(stat -c %s "$f")
    if [ "$sz" -gt "$largest_bytes" ]; then
      largest="$f"
      largest_bytes="$sz"
    fi
  done < <(find "$ROOT/dist/assets" -type f -name '*.js' 2>/dev/null || true)
  LARGEST_JS_BYTES=$largest_bytes
  LARGEST_JS_REL=""
  if [ -n "$largest" ]; then
    LARGEST_JS_REL="${largest#"$ROOT/dist/"}"
    LARGEST_JS_GZIP="$(gzip -9 -c "$largest" | wc -c)"
  else
    LARGEST_JS_GZIP=0
  fi
}

# ---------------------------------------------------------------------------
# 指标：release 二进制
# ---------------------------------------------------------------------------
run_release_build() {
  if [ "${BS_SELF_TEST:-0}" = "1" ]; then
    RELEASE_BIN_BYTES=1234567
    RELEASE_BIN_SHA256="$(printf 'fixture-release-binary' | sha256sum | awk '{print $1}')"
    RELEASE_BUILD_RC=0
    return
  fi
  run_capture release_build r01 -- timeout "$T_RELEASE" cargo build --manifest-path "$ROOT/src-tauri/Cargo.toml" --release --locked
  RELEASE_BUILD_RC=$LAST_RC
  if [ $LAST_RC -ne 0 ]; then
    RELEASE_BIN_BYTES=0
    RELEASE_BIN_SHA256=""
    return
  fi
  local bin="$ROOT/src-tauri/target/release/mvp-browser-os"
  if [ ! -f "$bin" ]; then
    echo "error: release binary not found at $bin" >&2
    RELEASE_BIN_BYTES=0
    RELEASE_BIN_SHA256=""
    RELEASE_BUILD_RC=1
    return
  fi
  RELEASE_BIN_BYTES="$(stat -c %s "$bin")"
  RELEASE_BIN_SHA256="$(sha256sum "$bin" | awk '{print $1}')"
}

# ---------------------------------------------------------------------------
# 产品钩子探测（契约 §6.1/§6.3 的关键标记；M0-1.a 不采集，仅标记 BLOCKED）
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
# 环境 / 来源指纹收集（契约 §4）
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
  ENV_GDK_BACKEND="${GDK_BACKEND:-}"
  ENV_WEBKIT_DMABUF="${WEBKIT_DISABLE_DMABUF_RENDERER:-}"

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

  ENV_ISOLATED_XDG_DATA="/tmp/mvp-browser-os-m0/$RUN_ID/xdg-data"
  ENV_ISOLATED_XDG_CACHE="/tmp/mvp-browser-os-m0/$RUN_ID/xdg-cache"
  ENV_ISOLATED_XDG_CONFIG="/tmp/mvp-browser-os-m0/$RUN_ID/xdg-config"
}

# ---------------------------------------------------------------------------
# 输出：environment.json / scenario.json
# ---------------------------------------------------------------------------
# bash 普通变量不会自动进入 python 子进程环境；需要显式 export。
export_for_python() {
  local v
  for v in "$@"; do
    if declare -p "$v" >/dev/null 2>&1; then
      export "$v"
    fi
  done
}

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
    },
    "webkit": {
        "webkit_disable_dmabuf_renderer": g("ENV_WEBKIT_DMABUF"),
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
        "webkit_disable_dmabuf_renderer": g("ENV_WEBKIT_DMABUF"),
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
samples = int(os.environ.get("FRONTEND_SAMPLES", "3"))
data = {
    "contract_version": os.environ.get("CONTRACT_VERSION", ""),
    "scenario_version": os.environ.get("SCENARIO_VERSION", ""),
    "checkpoint": "M0-1.a",
    "frontend_build": {
        "warmup_samples": 1,
        "formal_samples": samples,
        "command": "rm -rf dist && npm run build",
        "timeout_s": int(os.environ.get("T_FRONTEND", "600")),
    },
    "clippy": {
        "command": "cargo clippy --manifest-path src-tauri/Cargo.toml --message-format=json",
        "timeout_s": int(os.environ.get("T_CLIPPY", "900")),
    },
    "fmt": {"command": "cargo fmt --manifest-path <crate>/Cargo.toml --check", "timeout_s": int(os.environ.get("T_FMT", "300"))},
    "release_build": {
        "command": "cargo build --manifest-path src-tauri/Cargo.toml --release --locked",
        "timeout_s": int(os.environ.get("T_RELEASE", "5400")),
    },
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, indent=2)
    f.write("\n")
print("scenario.json written")
PY
}

# ---------------------------------------------------------------------------
# 输出：summary.json / summary.md / SHA256SUMS
# ---------------------------------------------------------------------------
compute_frontend_stats() {
  # FRONTEND_FORMAL_MS 数组 -> FRONTEND_MEDIAN / _MIN / _MAX / _VOLATILITY
  if [ "${#FRONTEND_FORMAL_MS[@]}" -eq 0 ]; then
    FRONTEND_MEDIAN=0; FRONTEND_MIN=0; FRONTEND_MAX=0; FRONTEND_VOLATILITY="n/a"
    return
  fi
  local list
  list="$(printf '%s\n' "${FRONTEND_FORMAL_MS[@]}")"
  FRONTEND_MEDIAN="$(printf '%s\n' "$list" | median_of)"
  FRONTEND_MIN="$(printf '%s\n' "$list" | sort -n | head -1)"
  FRONTEND_MAX="$(printf '%s\n' "$list" | sort -n | tail -1)"
  if [ "$FRONTEND_MEDIAN" = "0" ]; then
    FRONTEND_VOLATILITY="abs-diff:$((FRONTEND_MAX - FRONTEND_MIN))ms"
  else
    FRONTEND_VOLATILITY="$(python3 -c "print('%.1f' % (($FRONTEND_MAX - $FRONTEND_MIN) / $FRONTEND_MEDIAN * 100.0))")%"
  fi
}

write_summary_json() {
  compute_frontend_stats
  export_for_python \
    FMT_MAIN_EXIT FMT_PLUGIN_EXIT CLIPPY_STATUS CLIPPY_MAIN_COUNT CLIPPY_PLUGIN_COUNT \
    FRONTEND_STATUS FRONTEND_WARMUP_MS FRONTEND_FORMAL_MS_CSV FRONTEND_MEDIAN FRONTEND_MIN FRONTEND_MAX FRONTEND_VOLATILITY \
    DIST_TOTAL_BYTES LARGEST_JS_REL LARGEST_JS_BYTES LARGEST_JS_GZIP \
    RELEASE_BUILD_RC RELEASE_BIN_BYTES RELEASE_BIN_SHA256 READY_HOOK TERM_HOOK \
    RUN_ID RUN_TS CONTRACT_VERSION SCRIPT_VERSION ROOT GIT_FULL_SHA GIT_SHORT_SHA GIT_BRANCH
  python3 - "$RUN_DIR/summary.json" <<'PY'
import json, os, sys
out = sys.argv[1]
def g(k, d=""):
    v = os.environ.get(k)
    return d if v is None else v

fmt_main_ok = (g("FMT_MAIN_EXIT", "1") == "0")
fmt_plugin_ok = (g("FMT_PLUGIN_EXIT", "1") == "0")
results = {}
results["rust_fmt_main_exit"] = {"status": "PASS" if fmt_main_ok else "FAIL", "exit_code": int(g("FMT_MAIN_EXIT", "1"))}
results["rust_fmt_plugin_exit"] = {"status": "PASS" if fmt_plugin_ok else "FAIL", "exit_code": int(g("FMT_PLUGIN_EXIT", "1"))}
results["clippy_main_unique_warnings"] = {"status": g("CLIPPY_STATUS", "FAIL"), "count": int(g("CLIPPY_MAIN_COUNT", "0"))}
results["clippy_plugin_unique_warnings"] = {"status": g("CLIPPY_STATUS", "FAIL"), "count": int(g("CLIPPY_PLUGIN_COUNT", "0"))}
results["frontend_build_ms"] = {
    "status": g("FRONTEND_STATUS", "FAIL"),
    "warmup_ms": int(g("FRONTEND_WARMUP_MS", "0")),
    "samples_ms": [int(x) for x in g("FRONTEND_FORMAL_MS_CSV", "").split(",") if x != ""],
    "median_ms": int(g("FRONTEND_MEDIAN", "0")),
    "min_ms": int(g("FRONTEND_MIN", "0")),
    "max_ms": int(g("FRONTEND_MAX", "0")),
    "volatility": g("FRONTEND_VOLATILITY", "n/a"),
}
results["dist_total_bytes"] = {"status": "PASS", "bytes": int(g("DIST_TOTAL_BYTES", "0"))}
results["largest_js_bytes"] = {"status": "PASS", "path": g("LARGEST_JS_REL"), "bytes": int(g("LARGEST_JS_BYTES", "0"))}
results["largest_js_gzip_bytes"] = {"status": "PASS", "path": g("LARGEST_JS_REL"), "bytes": int(g("LARGEST_JS_GZIP", "0"))}
results["release_binary_bytes"] = {
    "status": "PASS" if (g("RELEASE_BUILD_RC", "1") == "0" and int(g("RELEASE_BIN_BYTES", "0")) > 0) else "FAIL",
    "bytes": int(g("RELEASE_BIN_BYTES", "0")),
    "sha256": g("RELEASE_BIN_SHA256"),
}

blocked = []
ready_owner = "M0-1.b"
if g("READY_HOOK") == "BLOCKED":
    blocked.append({"metric": "startup_ready_ms", "status": "BLOCKED", "owner": ready_owner, "unblock": "实现产品 ready 钩子（契约 §6.1：mount+2×rAF+IPC 往返后写带 run_id 的 ready 信号）"})
    for m in ["idle_process_tree_rss_kib", "idle_process_tree_fd_count", "tab_cycle_rss_slope_kib", "grid_cycle_rss_slope_kib", "terminal_cycle_rss_slope_kib", "resource_cycle_fd_delta", "orphan_process_count"]:
        blocked.append({"metric": m, "status": "BLOCKED", "owner": ready_owner, "unblock": "ready 钩子落地后由 M0-1.b/verify-resources.sh 采集"})
if g("TERM_HOOK") == "BLOCKED":
    for m in ["terminal_10mib_elapsed_ms", "terminal_frame_gap_p95_ms", "terminal_frame_gap_max_ms"]:
        blocked.append({"metric": m, "status": "BLOCKED", "owner": ready_owner, "unblock": "实现终端 begin/end 标记（契约 §6.3：__M0_TERM_BEGIN__/__M0_TERM_END__）"})

deferred = [
    {"metric": "script_first_response_ms", "status": "DEFERRED(M2-4)"},
    {"metric": "database_first_row_ms", "status": "DEFERRED(M4-3)"},
]

overall = "PASS"
for k, v in results.items():
    if v["status"] == "FAIL":
        overall = "FAIL"
        break

data = {
    "run_id": g("RUN_ID"),
    "timestamp": g("RUN_TS"),
    "checkpoint": "M0-1.a",
    "contract_version": g("CONTRACT_VERSION"),
    "script_version": g("SCRIPT_VERSION"),
    "repo_root": g("ROOT"),
    "worktree_clean_at_start": True,
    "git": {"commit_sha": g("GIT_FULL_SHA"), "short_sha": g("GIT_SHORT_SHA"), "branch": g("GIT_BRANCH")},
    "status": overall,
    "results": results,
    "blocked": blocked,
    "deferred": deferred,
    "evidence": {
        "run_dir": "logs/m0-baseline/" + g("RUN_ID"),
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
    echo "# M0-1.a baseline quality gate — $RUN_ID"
    echo ""
    echo "> 检查点：M0-1.a；契约：$CONTRACT_VERSION；脚本：$SCRIPT_VERSION；场景：$SCENARIO_VERSION"
    echo "> 生成时间：$(date '+%Y-%m-%d %H:%M:%S %z')"
    echo "> 机器判定源：summary.json（本文件仅人类阅读；两者不一致时整批 FAIL）"
    echo ""
    echo "## 总体状态：**$SUMMARY_STATUS**"
    echo ""
    echo "| 指标 | 状态 | 值 |"
    echo "|------|------|----|"
    echo "| rust_fmt_main_exit | ${FMT_MAIN_EXIT:-?} == 0 ? PASS : FAIL | exit=$FMT_MAIN_EXIT |"
    echo "| rust_fmt_plugin_exit | ${FMT_PLUGIN_EXIT:-?} == 0 ? PASS : FAIL | exit=$FMT_PLUGIN_EXIT |"
    echo "| clippy_main_unique_warnings | $CLIPPY_STATUS | $CLIPPY_MAIN_COUNT |"
    echo "| clippy_plugin_unique_warnings | $CLIPPY_STATUS | $CLIPPY_PLUGIN_COUNT |"
    echo "| frontend_build_ms | $FRONTEND_STATUS | warmup=${FRONTEND_WARMUP_MS:-0}ms; samples=${#FRONTEND_FORMAL_MS[@]}; median=${FRONTEND_MEDIAN:-0}ms; min=${FRONTEND_MIN:-0}; max=${FRONTEND_MAX:-0}; vol=${FRONTEND_VOLATILITY:-n/a} |"
    echo "| dist_total_bytes | PASS | $DIST_TOTAL_BYTES |"
    echo "| largest_js_bytes | PASS | $LARGEST_JS_BYTES ($LARGEST_JS_REL) |"
    echo "| largest_js_gzip_bytes | PASS | $LARGEST_JS_GZIP |"
    echo "| release_binary_bytes | ${RELEASE_BUILD_RC:-1} == 0 ? PASS : FAIL | $RELEASE_BIN_BYTES; sha256=$RELEASE_BIN_SHA256 |"
    echo ""
    echo "## BLOCKED / DEFERRED（信息性，不计入 M0-1.a PASS 判定）"
    echo ""
    echo "产品 ready/终端钩子状态：ready=$READY_HOOK term=$TERM_HOOK"
    echo ""
    echo "## 证据"
    echo ""
    echo "- run 目录：\`logs/m0-baseline/$RUN_ID/\`"
    echo "- environment.json / scenario.json / summary.json / summary.md / SHA256SUMS"
    echo "- raw/：各指标命令原始 stdout/stderr；measurements/：结构化测量；commands/：完整命令行"
    echo ""
  } >"$f"
}

write_commands_txt() {
  local f="$RUN_DIR/commands/commands.txt"
  {
    echo "M0-1.a commands — run_id=$RUN_ID"
    echo "generated=$(date '+%Y-%m-%d %H:%M:%S %z')"
    echo ""
    echo "# 质量"
    echo "cargo fmt --manifest-path $ROOT/src-tauri/Cargo.toml --check"
    echo "cargo fmt --manifest-path $ROOT/tauri-browser-tabs/Cargo.toml --check"
    echo "cargo clippy --manifest-path $ROOT/src-tauri/Cargo.toml --message-format=json"
    echo "# 构建"
    echo "rm -rf $ROOT/dist && (cd $ROOT && npm run build)  # warmup + $FRONTEND_SAMPLES formal"
    echo "cargo build --manifest-path $ROOT/src-tauri/Cargo.toml --release --locked"
    echo "# 指纹"
    echo "environment.json / scenario.json（本 run）"
  } >"$f"
}

write_sha256sums() {
  (cd "$RUN_DIR" && find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 -n1 sha256sum > SHA256SUMS)
}

write_measurements_frontend() {
  local i=0 v
  for v in "${FRONTEND_FORMAL_MS[@]}"; do
    i=$((i + 1))
    printf '{"metric":"frontend_build_ms","run_id":"%s","sample":%d,"elapsed_ms":%d}\n' "$RUN_ID" "$i" "$v" \
      >"$RUN_DIR/measurements/frontend_build_ms_r$(printf '%02d' "$i").json"
  done
  printf '{"metric":"clippy_main_unique_warnings","count":%s,"plugin_count":%s,"dedup":"package,file,line,col,lint"}\n' \
    "$CLIPPY_MAIN_COUNT" "$CLIPPY_PLUGIN_COUNT" >"$RUN_DIR/measurements/clippy_warnings.json"
  printf '{"metric":"dist_total_bytes","bytes":%s,"largest_js_bytes":%s,"largest_js_gzip_bytes":%s,"largest_js_rel":"%s"}\n' \
    "$DIST_TOTAL_BYTES" "$LARGEST_JS_BYTES" "$LARGEST_JS_GZIP" "$LARGEST_JS_REL" >"$RUN_DIR/measurements/dist_size.json"
  printf '{"metric":"release_binary_bytes","bytes":%s,"sha256":"%s"}\n' \
    "$RELEASE_BIN_BYTES" "$RELEASE_BIN_SHA256" >"$RUN_DIR/measurements/release_binary.json"
}

# ---------------------------------------------------------------------------
# 正式模式
# ---------------------------------------------------------------------------
run_formal() {
  ROOT="$(resolve_root)"
  GIT_PORCELAIN="$(git -C "$ROOT" status --porcelain=v1)"
  if [ -n "$GIT_PORCELAIN" ]; then
    echo "error: worktree not clean; formal baseline requires a clean commit" >&2
    echo "git status --porcelain=v1:" >&2
    echo "$GIT_PORCELAIN" >&2
    echo "note: with a dirty tree results could only be archived as EXPLORATORY" >&2
    exit 1
  fi

  GIT_FULL_SHA="$(git -C "$ROOT" rev-parse HEAD)"
  GIT_SHORT_SHA="$(git -C "$ROOT" rev-parse --short=7 HEAD)"
  GIT_BRANCH="$(git -C "$ROOT" symbolic-ref -q --short HEAD || echo "detached:$(git -C "$ROOT" rev-parse --short HEAD)")"
  RUN_TS="$(date +%Y%m%dT%H%M%S%z)"
  BACKEND="${GDK_BACKEND:-x11}"
  RUN_ID="${RUN_TS}_${GIT_SHORT_SHA}_release_${BACKEND}"
  RUN_DIR="$ROOT/logs/m0-baseline/$RUN_ID"

  mkdir -p "$RUN_DIR"/{commands,raw,measurements,screenshots}
  mkdir -p "/tmp/mvp-browser-os-m0/$RUN_ID"/xdg-{data,cache,config}
  RAW="$RUN_DIR/raw"

  FRONTEND_SAMPLES="${BS_FRONTEND_SAMPLES:-$DEFAULT_FRONTEND_SAMPLES}"
  BIN_PATH="$ROOT/src-tauri/target/release/mvp-browser-os"

  echo "[M0-1.a] run_id=$RUN_ID"
  echo "[M0-1.a] repo=$ROOT branch=$GIT_BRANCH commit=$GIT_SHORT_SHA"

  detect_product_hooks
  echo "[M0-1.a] hooks: ready=$READY_HOOK term=$TERM_HOOK"

  echo "[M0-1.a] collecting environment fingerprint..."
  collect_environment

  echo "[M0-1.a] fmt main..."
  run_fmt_main
  echo "[M0-1.a] fmt plugin..."
  run_fmt_plugin

  echo "[M0-1.a] clippy (structured, dedup by package/file/line/col/lint)..."
  run_clippy

  echo "[M0-1.a] frontend build (1 warmup + ${FRONTEND_SAMPLES} formal)..."
  run_frontend_build

  echo "[M0-1.a] release build (--locked)..."
  run_release_build

  echo "[M0-1.a] writing evidence..."
  FRONTEND_FORMAL_MS_CSV="$(IFS=,; echo "${FRONTEND_FORMAL_MS[*]:-}")"
  export_for_python \
    RUN_ID RUN_TS CONTRACT_VERSION SCRIPT_VERSION SCENARIO_VERSION \
    ROOT GIT_FULL_SHA GIT_SHORT_SHA GIT_BRANCH GIT_PORCELAIN BIN_PATH RELEASE_BIN_SHA256 \
    FRONTEND_SAMPLES T_FRONTEND T_CLIPPY T_FMT T_RELEASE \
    ENV_OS ENV_KERNEL ENV_ARCH ENV_CPU_MODEL ENV_NPROC ENV_MEM_BYTES \
    ENV_XDG_SESSION ENV_DISPLAY ENV_WAYLAND ENV_RESOLUTION ENV_SCALE ENV_GDK_BACKEND ENV_WEBKIT_DMABUF \
    ENV_WEBKITGTK ENV_GTK ENV_RUSTC ENV_CARGO ENV_NODE ENV_NPM \
    ENV_LOCALE ENV_TIMEZONE ENV_LOADAVG ENV_MEMAVAIL ENV_POWER \
    ENV_CARGO_LOCK_SHA ENV_NPM_LOCK_SHA ENV_ISOLATED_XDG_DATA ENV_ISOLATED_XDG_CACHE ENV_ISOLATED_XDG_CONFIG
  write_environment_json
  write_scenario_json
  write_measurements_frontend
  write_commands_txt
  write_summary_json
  write_summary_md
  write_sha256sums

  echo ""
  echo "=== M0-1.a summary: status=$SUMMARY_STATUS ==="
  echo "evidence dir: $RUN_DIR"
  if [ "$SUMMARY_STATUS" = "PASS" ]; then
    exit 0
  else
    echo "one or more M0-1.a metrics FAILED" >&2
    exit 1
  fi
}

# ---------------------------------------------------------------------------
# self-test：固定夹具验证脚本自身（fixture，不生成正式证据）
# ---------------------------------------------------------------------------
run_self_test() {
  echo "[self-test] fixture mode: validating script logic, NOT generating formal evidence"
  require_tools
  local rc=0
  tmp="$(mktemp -d /tmp/bs-self-test.XXXXXX)" # 脚本级变量：EXIT trap 需要它在函数返回后仍可引用
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

  # 用例 3：干净 fixture 仓库正式流程 → PASS + 正确去重 + BLOCKED 标记
  local repo="$tmp/repo-clean"
  mkdir -p "$repo"
  git -C "$repo" init -q
  git -C "$repo" config user.name fixture
  git -C "$repo" config user.email fixture@example.invalid
  echo "fixture" >"$repo/README.md"
  git -C "$repo" add -A
  git -C "$repo" commit -qm "fixture baseline"
  local out
  out="$(cd "$repo" && BS_SELF_TEST=1 BS_FRONTEND_SAMPLES=1 bash "$SCRIPT_PATH" 2>&1)"
  local code=$?
  echo "$out" | sed 's/^/    [fixture] /'
  if [ $code -ne 0 ]; then
    echo "FAIL: clean fixture run should exit 0 (got $code)"; rc=1
  else
    local runid summary
    runid="$(cd "$repo" && ls logs/m0-baseline | tail -1)"
    summary="$repo/logs/m0-baseline/$runid/summary.json"
    if [ ! -f "$summary" ]; then
      echo "FAIL: summary.json not generated"; rc=1
    else
      local status cm cpp blocked_ok
      status="$(python3 -c 'import json;print(json.load(open("'$summary'"))["status"])')"
      cm="$(python3 -c 'import json;print(json.load(open("'$summary'"))["results"]["clippy_main_unique_warnings"]["count"])')"
      cpp="$(python3 -c 'import json;print(json.load(open("'$summary'"))["results"]["clippy_plugin_unique_warnings"]["count"])')"
      blocked_ok="$(python3 -c 'import json;d=json.load(open("'$summary'"));print(any(b["metric"]=="startup_ready_ms" and b["status"]=="BLOCKED" for b in d["blocked"]))')"
      if [ "$status" = "PASS" ] && [ "$cm" = "4" ] && [ "$cpp" = "1" ] && [ "$blocked_ok" = "True" ]; then
        echo "PASS: clean fixture run -> status=$status, clippy main(dedup)=$cm plugin=$cpp, startup_ready_ms=BLOCKED"
      else
        echo "FAIL: fixture summary mismatch status=$status clippy_main=$cm clippy_plugin=$cpp blocked_ok=$blocked_ok"; rc=1
      fi
    fi
  fi

  # 用例 4：脏工作树正式模式 → 非零退出
  local repo2="$tmp/repo-dirty"
  mkdir -p "$repo2"
  git -C "$repo2" init -q
  git -C "$repo2" config user.name fixture
  git -C "$repo2" config user.email fixture@example.invalid
  echo "fixture" >"$repo2/README.md"
  git -C "$repo2" add -A
  git -C "$repo2" commit -qm "fixture dirty"
  echo "dirty" >"$repo2/untracked.txt"
  if (cd "$repo2" && BS_SELF_TEST=1 bash "$SCRIPT_PATH" >/dev/null 2>&1); then
    echo "FAIL: dirty worktree run should exit nonzero"; rc=1
  else
    echo "PASS: dirty worktree rejected (nonzero exit)"
  fi

  # 用例 5：SHA256SUMS 与 run 目录完整性
  local runid2
  runid2="$(ls "$tmp/repo-clean/logs/m0-baseline" | tail -1)"
  if (cd "$tmp/repo-clean/logs/m0-baseline/$runid2" && sha256sum -c SHA256SUMS >/dev/null 2>&1); then
    echo "PASS: SHA256SUMS verifies all evidence files"
  else
    echo "FAIL: SHA256SUMS verification"; rc=1
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
