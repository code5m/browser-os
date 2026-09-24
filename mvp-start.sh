#!/usr/bin/env bash
# ============================================================
#  mvp-start.sh  ——  启动 极智简单·浏览器OS融合 MVP 前端服务
# ============================================================
#  默认启动 Vite 开发服务器(端口 1421, 与 tauri.conf.json 的 devUrl 一致)。
#  若传入参数 "tauri"，则尝试以完整桌面应用方式启动(需图形环境)。
#
#  用法:
#    bash mvp-start.sh            # 启动前端 Vite 预览 (无头/服务器环境适用)
#    bash mvp-start.sh tauri      # 启动完整 Tauri 桌面应用 (需本地图形会话)
#    bash mvp-start.sh --host     # 启动并监听 0.0.0.0 (暴露到局域网)
#    bash mvp-start.sh --help     # 查看帮助
# ============================================================
set -euo pipefail

APP_DIR="/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3"
LOG_FILE="/tmp/mvp-vite.log"
PORT=1421
MODE="vite"
HOST_FLAG=""

# ---- 解析参数 ----
for arg in "$@"; do
  case "$arg" in
    tauri)   MODE="tauri" ;;
    --host)  HOST_FLAG="--host" ;;
    --help|-h)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) echo "!! 未知参数: $arg  (用 --help 查看用法)"; exit 1 ;;
  esac
done

cd "$APP_DIR"

# ---- 端口占用检测 / 自动清理 ----
if lsof -ti:"$PORT" >/dev/null 2>&1; then
  OLD_PID=$(lsof -ti:"$PORT" | head -1)
  echo "[start] 端口 $PORT 被进程 $OLD_PID 占用，先关闭..."
  bash "$APP_DIR/mvp-stop.sh" >/dev/null 2>&1 || true
  sleep 1
fi

# ---- 前置检查 ----
if [ ! -d "$APP_DIR/node_modules" ]; then
  echo "[start] 未检测到 node_modules，先执行 npm install ..."
  npm install
fi

# ---- 启动 ----
if [ "$MODE" = "tauri" ]; then
  echo "[start] 以 Tauri 桌面模式启动（委托 run-gui.sh 统一入口，自动清理残留）..."
  # 单一 GUI 入口：run-gui.sh 已内置 WebKit 环境变量与"启动前清场"，
  # 不再走 npm run tauri dev 的另一套 vite 所有权模型，避免孤儿 vite / 双入口混乱。
  exec bash "$APP_DIR/run-gui.sh"
else
  echo "[start] 启动 Vite 开发服务器 -> http://localhost:$PORT/"
  # 后台启动, 日志写入 LOG_FILE
  nohup npm run dev -- $HOST_FLAG > "$LOG_FILE" 2>&1 &
  echo "[start] 已后台启动 (PID $!), 日志: $LOG_FILE"
  echo "[start] 等待服务就绪 ..."
  for i in $(seq 1 20); do
    if grep -q "ready in" "$LOG_FILE" 2>/dev/null; then
      echo ""
      echo "✅ 服务已就绪:"
      grep -E "Local:|Network:" "$LOG_FILE" || true
      exit 0
    fi
    sleep 1
  done
  echo "⚠️ 启动超时，请查看日志: $LOG_FILE"
  exit 1
fi
