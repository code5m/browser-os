#!/usr/bin/env bash
# M5-W17 · Lane A2 —— run-gui.sh / scripts/dev-server.sh 启动路径的 ownership-safe 冒烟测试。
#
# 不启动真实 Vite、不启动 GUI：用 `python3 -m http.server` 作 stub server 在临时端口上
# 验证所有权语义：
#   1) 端口已有服务 → 判定"他人所有"：不启动，cleanup 也不杀它（stub 存活）
#   2) 无服务      → 自己拉起并等待就绪，cleanup 只回收自己那个（stub 消失、pidfile 清理）
#   3) 启动超时    → 明确失败（非零退出）且不留下孤儿进程
#   4) run-gui.sh 静态契约（可执行、引用助手、注册 cleanup trap、保留图形环境 workaround）
#   5) main.rs debug/release 资产选择与 Tauri devUrl/ACL 来源身份回归

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HELPER="$ROOT/scripts/dev-server.sh"
RUN_GUI="$ROOT/run-gui.sh"
MAIN_RS="$ROOT/src-tauri/src/main.rs"
TAURI_CONF="$ROOT/src-tauri/tauri.conf.json"
DEFAULT_CAP="$ROOT/src-tauri/capabilities/default.json"
DEV_CAP="$ROOT/src-tauri/dev-capabilities/main.json"

PASS=0
FAIL=0
ok() { printf '  ✓ %s\n' "$*"; PASS=$((PASS + 1)); }
bad() { printf '  ✗ %s\n' "$*"; FAIL=$((FAIL + 1)); }
section() { printf '\n[%s]\n' "$*"; }

for f in "$HELPER" "$RUN_GUI" "$MAIN_RS" "$TAURI_CONF" "$DEFAULT_CAP" "$DEV_CAP"; do
  if [[ ! -f "$f" ]]; then printf '缺少文件：%s\n' "$f" >&2; exit 1; fi
done

STATE_ROOT="$(mktemp -d)"
trap 'rm -rf "$STATE_ROOT"' EXIT

free_port() {
  python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()'
}

port_open() {
  local port="$1"
  (exec 3<>"/dev/tcp/127.0.0.1/${port}") >/dev/null 2>&1
}

wait_port() {
  local port="$1" i
  for i in $(seq 1 60); do
    if port_open "$port"; then return 0; fi
    sleep 0.25
  done
  return 1
}

# 切到一份干净的 state，并按给定端口/命令重新加载助手（重算 pidfile 等）
reset_state() {
  local name="$1" port="$2" cmd="$3"
  export MVP_DEV_SERVER_STATE_DIR="$STATE_ROOT/$name"
  export MVP_DEV_SERVER_PORT="$port"
  export MVP_DEV_SERVER_HOST="127.0.0.1"
  export MVP_DEV_SERVER_URL="http://127.0.0.1:${port}/"
  export MVP_DEV_SERVER_CMD="$cmd"
  export MVP_DEV_SERVER_TIMEOUT_SECS="${MVP_DEV_SERVER_TIMEOUT_SECS:-60}"
  rm -rf "$MVP_DEV_SERVER_STATE_DIR"
  mkdir -p "$MVP_DEV_SERVER_STATE_DIR"
  # shellcheck disable=SC1090
  source "$HELPER"
  unset DS_CLEANED 2>/dev/null || true
}

echo "== M5-W17 A2 dev-startup smoke =="

# ---------------------------------------------------------------
section "1) 端口已有服务 → 不接管、不清理"
STUB1_PORT="$(free_port)"
STUB1_DIR="$STATE_ROOT/stub1"
mkdir -p "$STUB1_DIR"
(cd "$STUB1_DIR" && exec python3 -m http.server "$STUB1_PORT" --bind 127.0.0.1 >/dev/null 2>&1) &
STUB1_PID=$!
if wait_port "$STUB1_PORT"; then ok "stub（他人服务）已就绪 :${STUB1_PORT}"; else bad "stub 未能就绪"; fi
reset_state t1 "$STUB1_PORT" "true"
if dev_server_ensure >/dev/null 2>&1; then ok "ensure 识别到已有服务并直接返回"; else bad "ensure 对已有服务报错"; fi
if [[ -f "$MVP_DEV_SERVER_STATE_DIR/dev-server.pid" ]]; then
  bad "不应写入 pidfile（未自己启动 → 不持有）"
else
  ok "未接管已有服务（无 pidfile → 不持有）"
fi
ds_cleanup >/dev/null 2>&1 || true
if port_open "$STUB1_PORT"; then ok "cleanup 未杀掉他人服务（stub 仍存活）"; else bad "cleanup 误杀了他人服务"; fi
kill "$STUB1_PID" 2>/dev/null || true
wait "$STUB1_PID" 2>/dev/null || true

# ---------------------------------------------------------------
section "2) 无服务 → 自己拉起并等待就绪 → cleanup 只回收自己那个"
STUB2_PORT="$(free_port)"
STUB2_DIR="$STATE_ROOT/stub2"
mkdir -p "$STUB2_DIR"
reset_state t2 "$STUB2_PORT" "cd $STUB2_DIR && python3 -m http.server $STUB2_PORT --bind 127.0.0.1"
if dev_server_ensure >/dev/null 2>&1; then ok "ensure 成功（拉起并等待就绪）"; else bad "ensure 失败"; fi
if [[ -f "$MVP_DEV_SERVER_STATE_DIR/dev-server.pid" ]]; then ok "写入 pidfile（本助手持有）"; else bad "缺少 pidfile"; fi
if ds_is_up; then ok "dev server 可探测（HTTP 就绪）"; else bad "探测失败"; fi
ds_cleanup >/dev/null 2>&1 || true
sleep 0.5
if ds_is_up; then bad "cleanup 后服务仍在（未回收）"; else ok "cleanup 回收了自己拉起的 server"; fi
if [[ -f "$MVP_DEV_SERVER_STATE_DIR/dev-server.pid" ]]; then bad "pidfile 未清理"; else ok "pidfile 已清理"; fi

# ---------------------------------------------------------------
section "3) 启动超时 → 明确失败且不留下孤儿"
STUB3_PORT="$(free_port)"
reset_state t3 "$STUB3_PORT" "sleep 30"
export MVP_DEV_SERVER_TIMEOUT_SECS=2
if dev_server_ensure >/dev/null 2>&1; then
  bad "超时应失败，却返回成功"
else
  ok "超时明确失败（非零退出）"
fi
if [[ -f "$MVP_DEV_SERVER_STATE_DIR/dev-server.pid" ]]; then
  bad "超时后残留 pidfile（可能留下孤儿）"
else
  ok "超时后 pidfile 已清理（已回收刚启动的进程）"
fi
if command -v pgrep >/dev/null 2>&1; then
  if pgrep -f "sleep 30" >/dev/null 2>&1; then bad "残留孤儿进程（sleep 30）"; else ok "无孤儿进程残留"; fi
fi
export MVP_DEV_SERVER_TIMEOUT_SECS=60

# ---------------------------------------------------------------
section "4) run-gui.sh 静态契约"
if [[ -x "$RUN_GUI" ]]; then ok "run-gui.sh 可执行"; else bad "run-gui.sh 不可执行"; fi
if grep -q "scripts/dev-server.sh" "$RUN_GUI"; then ok "引用 dev-server 助手"; else bad "未引用 dev-server 助手"; fi
if grep -q "trap 'cleanup' EXIT" "$RUN_GUI"; then ok "注册 EXIT cleanup trap"; else bad "缺少 cleanup trap"; fi
if grep -qE '^exec "\$BIN"' "$RUN_GUI"; then bad "仍用 exec 启动（cleanup 将不会执行）"; else ok "未用 exec（客户端退出后 cleanup 可运行）"; fi
if grep -q "WEBKIT_DISABLE_DMABUF_RENDERER" "$RUN_GUI" && grep -q "GDK_BACKEND" "$RUN_GUI"; then
  ok "保留图形环境 workaround（WEBKIT/GDK）"
else
  bad "丢失图形环境 workaround"
fi
if grep -q "MVP_FORCE_DIST" "$RUN_GUI"; then ok "支持 MVP_FORCE_DIST（与 main.rs 分支一致）"; else bad "缺少 MVP_FORCE_DIST 支持"; fi
if grep -q 'cargo build' "$RUN_GUI" && grep -q 'Tauri 的 capability 在 Rust 构建期嵌入' "$RUN_GUI"; then
  ok "默认启动会构建当前二进制，避免前端与 ACL 版本漂移"
else
  bad "默认启动未保证当前二进制，可能出现 invoke not allowed"
fi
if bash "$RUN_GUI" --help >/dev/null 2>&1; then ok "--help 退出 0（不构建、不启动）"; else bad "--help 失败"; fi

# ---------------------------------------------------------------
section "5) main.rs debug/release 资产选择与 ACL 来源身份回归"
if grep -q 'cfg!(debug_assertions)' "$MAIN_RS" && grep -q 'WebviewUrl::App("index.html".into())' "$MAIN_RS"; then
  ok "main.rs 仍为 debug→External(1421) / release→App(index.html)"
else
  bad "main.rs 资产选择分支异常"
fi
if grep -q 'localhost:1421' "$MAIN_RS" && grep -q 'MVP_FORCE_DIST' "$MAIN_RS"; then
  ok "main.rs 保留 MVP_FORCE_DIST 逃生阀"
else
  bad "main.rs 缺少 MVP_FORCE_DIST 逃生阀"
fi
if grep -q '#\[cfg(debug_assertions)\]' "$MAIN_RS" && grep -q 'add_capability(include_str!("../dev-capabilities/main.json"))' "$MAIN_RS"; then
  ok "开发 capability 仅在 debug 进程动态注册"
else
  bad "开发 capability 未受 debug_assertions 隔离"
fi
if python3 - "$TAURI_CONF" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    config = json.load(handle)
raise SystemExit(0 if "devUrl" not in config.get("build", {}) else 1)
PY
then
  ok "tauri.conf 不含 devUrl（release 不会被编译到开发服务器）"
else
  bad "tauri.conf 含 devUrl；可能令 release 加载开发服务器并白屏"
fi
if python3 - "$DEFAULT_CAP" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    capability = json.load(handle)
raise SystemExit(0 if "remote" not in capability else 1)
PY
then
  ok "default-commands 未向远程 URL 扩权"
else
  bad "default capability 含 remote URL；主窗口高权限命令不应向远程来源开放"
fi
if python3 - "$DEV_CAP" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    capability = json.load(handle)
valid = (
    capability.get("identifier") == "main-dev"
    and capability.get("local") is False
    and capability.get("windows") == ["main"]
    and capability.get("remote", {}).get("urls") == ["http://localhost:1421/*"]
    and capability.get("permissions") == [
        "core:default",
        "core:window:allow-create",
        "browser-tabs:default",
        "default-commands",
    ]
)
raise SystemExit(0 if valid else 1)
PY
then
  ok "开发 capability 仅授权 main + localhost:1421 精确来源"
else
  bad "开发 capability 范围漂移；不得扩大窗口、URL 或权限"
fi
if [[ ! -e "$ROOT/src-tauri/capabilities/main-dev.json" ]]; then
  ok "开发 capability 未进入 release 自动扫描目录"
else
  bad "开发 capability 位于 capabilities/；会被静态打进 release"
fi
if grep -q '"open_tool"' "$ROOT/src-tauri/permissions/default-commands.toml"; then
  ok "工具打开命令已在主窗口 ACL 授权（避免运行时 not allowed）"
else
  bad "工具打开命令缺少主窗口 ACL 授权"
fi

# ---------------------------------------------------------------
printf '\n==== 冒烟结果：PASS=%s FAIL=%s ====\n' "$PASS" "$FAIL"
if (("$FAIL" > 0)); then exit 1; fi
exit 0
