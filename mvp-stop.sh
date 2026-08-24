#!/usr/bin/env bash
# ============================================================
#  mvp-stop.sh  ——  关闭 极智简单·浏览器OS融合 MVP 服务
# ============================================================
#  关闭占用 1421 端口的 Vite 进程, 以及可能的 tauri/cargo 相关进程。
#
#  用法:
#    bash mvp-stop.sh            # 关闭前端 Vite 服务
#    bash mvp-stop.sh --all      # 同时关闭 tauri 桌面进程与 cargo 构建残留
# ============================================================
set -uo pipefail

PORT=1421
APP_DIR="/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3"
STOP_ALL=0

for arg in "$@"; do
  case "$arg" in
    --all) STOP_ALL=1 ;;
    --help|-h) grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "!! 未知参数: $arg"; exit 1 ;;
  esac
done

KILLED=0

# ---- 1. 关闭占用端口的进程 ----
if lsof -ti:"$PORT" >/dev/null 2>&1; then
  PIDS=$(lsof -ti:"$PORT")
  echo "[stop] 关闭占用端口 $PORT 的进程: $PIDS"
  kill $PIDS 2>/dev/null || true
  KILLED=1
fi

# ---- 2. 关闭 npm/vite/node 主进程 (精确匹配本项目路径) ----
PIDS=$(pgrep -f "mvp-browser-os-v3.*vite" 2>/dev/null || true)
if [ -n "$PIDS" ]; then
  echo "[stop] 关闭 vite 主进程: $PIDS"
  kill $PIDS 2>/dev/null || true
  KILLED=1
fi

# ---- 3. (可选) 关闭 tauri / cargo 相关进程 ----
if [ "$STOP_ALL" = "1" ]; then
  TPIDS=$(pgrep -f "src-tauri|tauri dev|cargo" 2>/dev/null || true)
  if [ -n "$TPIDS" ]; then
    echo "[stop] 关闭 tauri/cargo 进程: $TPIDS"
    kill $TPIDS 2>/dev/null || true
    KILLED=1
  fi
fi

# ---- 结果 ----
sleep 1
if lsof -ti:"$PORT" >/dev/null 2>&1; then
  echo "⚠️ 端口 $PORT 仍被占用，强制 kill ..."
  kill -9 $(lsof -ti:"$PORT") 2>/dev/null || true
fi

if [ "$KILLED" = "1" ]; then
  echo "✅ 已关闭服务"
else
  echo "ℹ️ 未发现运行中的服务"
fi
