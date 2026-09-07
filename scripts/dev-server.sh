#!/usr/bin/env bash
# M5-W17 · Lane A2 —— 桌面 debug 客户端的 ownership-safe Vite dev-server 助手。
#
# 背景（W17 AC-1）：debug 构建的主窗口在 src-tauri/src/main.rs:1210-1214 强制指向
#   WebviewUrl::External("http://localhost:1421")（条件是 cfg!(debug_assertions) 且
#   未设 MVP_FORCE_DIST），release 仍走 WebviewUrl::App("index.html") 打包资源。
#   所以 debug 启动前必须保证 1421 上已有 Vite dev server，否则 webview 直接
#   "connection refused"（此前 run-gui.sh 从不启动 Vite，正是该问题）。
#
# 所有权契约（ownership-safe）：
#   - 启动前若端口已在服务 → 判定"他人所有"：本助手不启动，退出时也绝不杀它；
#   - 仅当本助手自己拉起 dev server 时，才写入 pidfile（pid + pgid），
#     cleanup 时按进程组回收（Vite 会派生子进程，故杀整个进程组）；
#   - cleanup 由 pidfile 驱动且幂等，无 pidfile 则不动任何已有服务。
#
# 可测试性：端口/主机/启动命令/状态目录/超时均可由环境变量覆盖，便于
#   scripts/check-dev-startup.sh 用 stub server（python3 -m http.server）在临时
#   端口上验证所有权语义，无需真的启动 Vite 或 GUI。
#
# 用法：
#   source scripts/dev-server.sh     # 作为库使用（dev_server_ensure / ds_cleanup ...）
#   scripts/dev-server.sh ensure     # 确保 dev server 就绪（必要时才启动并等待）
#   scripts/dev-server.sh is-up      # 仅探测（up/down，退出码 0/1）
#   scripts/dev-server.sh stop       # 只停本助手记录的那个（无 pidfile 则不动）
#   scripts/dev-server.sh url        # 打印 dev server URL

set -euo pipefail

MVP_DEV_SERVER_PORT="${MVP_DEV_SERVER_PORT:-1421}"
MVP_DEV_SERVER_HOST="${MVP_DEV_SERVER_HOST:-127.0.0.1}"
MVP_DEV_SERVER_URL="${MVP_DEV_SERVER_URL:-http://${MVP_DEV_SERVER_HOST}:${MVP_DEV_SERVER_PORT}/}"
# 启动命令（测试时可替换为 stub）
MVP_DEV_SERVER_CMD="${MVP_DEV_SERVER_CMD:-npm run dev}"
MVP_DEV_SERVER_TIMEOUT_SECS="${MVP_DEV_SERVER_TIMEOUT_SECS:-60}"
MVP_DEV_SERVER_STATE_DIR="${MVP_DEV_SERVER_STATE_DIR:-${TMPDIR:-/tmp}/mvp-dev-server-${UID:-$(id -u)}}"

DS_PIDFILE="${MVP_DEV_SERVER_STATE_DIR}/dev-server.pid"
DS_LOGFILE="${MVP_DEV_SERVER_STATE_DIR}/dev-server.log"

ds_log() { printf '[dev-server] %s\n' "$*" >&2; }
ds_err() { printf '[dev-server][ERROR] %s\n' "$*" >&2; }

# 探测 dev server 是否就绪：优先 HTTP GET（python3），回退 TCP 连通性（/dev/tcp）
ds_is_up() {
  local url="${1:-$MVP_DEV_SERVER_URL}" host="${2:-$MVP_DEV_SERVER_HOST}" port="${3:-$MVP_DEV_SERVER_PORT}"
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$url" <<'PY' >/dev/null 2>&1
import sys, urllib.request
try:
    with urllib.request.urlopen(sys.argv[1], timeout=2) as r:
        sys.exit(0 if r.status < 500 else 1)
except Exception:
    sys.exit(1)
PY
    return $?
  fi
  (exec 3<>"/dev/tcp/${host}/${port}") >/dev/null 2>&1
}

# 有界等待就绪（默认 60s），超时返回非零
ds_wait_ready() {
  local deadline
  deadline=$(( $(date +%s) + MVP_DEV_SERVER_TIMEOUT_SECS ))
  while (($(date +%s) < deadline)); do
    if ds_is_up; then return 0; fi
    sleep 0.5
  done
  return 1
}

_ds_pgid_of() {
  local pid="$1" pgid=""
  pgid="$(ps -o pgid= -p "$pid" 2>/dev/null | tr -d ' ' || true)"
  if [[ -z "$pgid" ]]; then pgid="$pid"; fi
  printf '%s' "$pgid"
}

# 记录 pid + pgid（只有本助手启动时才会写 → 即"所有权"标记）
ds_record_pid() {
  local pid="$1" pgid
  pgid="$(_ds_pgid_of "$pid")"
  mkdir -p "$MVP_DEV_SERVER_STATE_DIR"
  printf '%s %s\n' "$pid" "$pgid" > "$DS_PIDFILE"
}

# 启动 dev server 并等待就绪；失败则回收刚启动的进程，不留孤儿
ds_start() {
  mkdir -p "$MVP_DEV_SERVER_STATE_DIR"
  ds_log "启动 dev server：${MVP_DEV_SERVER_CMD}（端口 ${MVP_DEV_SERVER_PORT}，上限 ${MVP_DEV_SERVER_TIMEOUT_SECS}s）"
  local pid=""
  if command -v setsid >/dev/null 2>&1; then
    setsid bash -c "$MVP_DEV_SERVER_CMD" >>"$DS_LOGFILE" 2>&1 &
  else
    bash -c "$MVP_DEV_SERVER_CMD" >>"$DS_LOGFILE" 2>&1 &
  fi
  pid=$!
  ds_record_pid "$pid"
  if ds_wait_ready; then
    ds_log "dev server 就绪：${MVP_DEV_SERVER_URL}"
    return 0
  fi
  ds_err "dev server 在 ${MVP_DEV_SERVER_TIMEOUT_SECS}s 内未就绪；日志：${DS_LOGFILE}"
  ds_cleanup
  return 1
}

# 确保 dev server 就绪：已有则接管为"他人所有"（不启动、不清理）
dev_server_ensure() {
  if ds_is_up; then
    ds_log "dev server 已在运行（${MVP_DEV_SERVER_URL}）→ 非本助手启动，不接管、退出时不清理"
    return 0
  fi
  ds_start
}

# 只回收本助手启动的（pidfile 驱动）；无 pidfile → 不动任何已有服务；幂等
ds_cleanup() {
  if [[ -n "${DS_CLEANED:-}" ]]; then return 0; fi
  DS_CLEANED=1
  if [[ ! -f "$DS_PIDFILE" ]]; then
    ds_log "未持有 dev server（无 pidfile）→ 不清理任何已有服务"
    return 0
  fi
  local pid="" pgid=""
  if ! read -r pid pgid < "$DS_PIDFILE"; then
    rm -f "$DS_PIDFILE"
    return 0
  fi
  ds_log "回收本助手启动的 dev server（pid=${pid} pgid=${pgid}）"
  kill -TERM "-${pgid}" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
  local deadline
  deadline=$(( $(date +%s) + 5 ))
  while (($(date +%s) < deadline)); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.2
  done
  kill -KILL "-${pgid}" 2>/dev/null || kill -KILL "$pid" 2>/dev/null || true
  rm -f "$DS_PIDFILE"
  return 0
}

ds_usage() {
  cat <<'USAGE'
usage: dev-server.sh {ensure|is-up|stop|url}

ownership-safe Vite dev-server 助手（M5-W17 A2）。
  ensure  确保 dev server 就绪（已有则不启动、不接管；否则启动并等待就绪）
  is-up   仅探测，打印 up/down（退出码 0/1）
  stop    只停本助手记录的那个（无 pidfile 则不动任何已有服务）
  url     打印 dev server URL

环境变量：MVP_DEV_SERVER_{PORT,HOST,URL,CMD,TIMEOUT_SECS,STATE_DIR}
USAGE
}

ds_main() {
  case "${1:-ensure}" in
    ensure) dev_server_ensure ;;
    is-up)
      if ds_is_up; then echo "up"; else echo "down"; return 1; fi
      ;;
    stop) ds_cleanup ;;
    url) echo "$MVP_DEV_SERVER_URL" ;;
    -h | --help) ds_usage ;;
    *)
      ds_usage >&2
      return 2
      ;;
  esac
}

# 仅直接执行时跑 CLI；被 source 时只提供函数
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  ds_main "$@"
fi
