#!/usr/bin/env bash
# 在你自己的图形终端（gnome-terminal）里运行本脚本，
# 应用会从当前 Wayland/X11 桌面会话弹出窗口。
#
# 用法：
#   bash /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/run-gui.sh
#
# 如需观察日志（排查白屏/崩溃），末尾加 2>&1 | tee /tmp/mvp-gui.log
#
# M5-W17（Lane A2）：debug 启动改为"先确保 Vite dev server 就绪，再拉起客户端"。
#   背景：debug 构建的主窗口在 src-tauri/src/main.rs:1210-1214 强制指向
#   WebviewUrl::External("http://localhost:1421")（条件 cfg!(debug_assertions) 且
#   未设 MVP_FORCE_DIST），release 仍走 WebviewUrl::App("index.html") 打包资源。
#   此前本脚本从不启动 Vite，于是 webview 直接 "connection refused"。
#
#   现在：scripts/dev-server.sh 负责检测/启动/等待 1421；本脚本拉起客户端；退出时
#   **只回收本脚本自己拉起的那个** dev server（此前已在运行的不会被杀）。
#
#   本脚本不改动任何命令 / bridge / ACL / 生产运行时权限；release 行为不变。

set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$APP_DIR/src-tauri/target/debug/mvp-browser-os"
DEV_SERVER_HELPER="$APP_DIR/scripts/dev-server.sh"

FORCE_DIST="${MVP_FORCE_DIST:-}"
NO_BUILD=0
APP_ARGS=()

print_usage() {
  cat <<'USAGE'
usage: run-gui.sh [--force-dist] [--no-build] [-h|--help] [-- <客户端参数...>]

在图形终端里启动桌面 debug 客户端。debug 启动前会自动确保 Vite dev server
（默认 http://127.0.0.1:1421）就绪，并在本脚本退出时只回收自己拉起的那个。

  --force-dist   跳过 dev server，强制加载内嵌 dist（会导出 MVP_FORCE_DIST=1，
                 与 main.rs 的同名分支一致）
  --no-build     跳过本次 cargo build（仅用于确认二进制已是最新时）
  -h, --help     打印本帮助并退出（不做任何构建/启动）
  --             其后参数原样透传给客户端二进制
USAGE
}

while (($#)); do
  case "$1" in
    --force-dist | --no-dev-server) FORCE_DIST=1; shift ;;
    --no-build) NO_BUILD=1; shift ;;
    -h | --help) print_usage; exit 0 ;;
    --)
      shift
      APP_ARGS+=("$@")
      break
      ;;
    *)
      APP_ARGS+=("$1")
      shift
      ;;
  esac
done

DEV_STATE_DIR=""
CLEANED=""

cleanup() {
  [[ -n "$CLEANED" ]] && return 0
  CLEANED=1
  if [[ -n "$DEV_STATE_DIR" && -f "$DEV_STATE_DIR/dev-server.pid" ]]; then
    MVP_DEV_SERVER_STATE_DIR="$DEV_STATE_DIR" bash "$DEV_SERVER_HELPER" stop >/dev/null 2>&1 || true
  fi
  if [[ -n "$DEV_STATE_DIR" ]]; then rm -rf "$DEV_STATE_DIR" || true; fi
  return 0
}
trap 'cleanup' EXIT
trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM

if [[ "$NO_BUILD" == "1" ]]; then
  if [[ ! -x "$BIN" ]]; then
    echo "[run-gui][ERROR] --no-build 但未找到二进制：$BIN" >&2
    exit 1
  fi
else
  # Tauri 的 capability 在 Rust 构建期嵌入。始终构建可避免 Vite 已更新、
  # 原生二进制仍携带旧 ACL 时，页面把所有 invoke 报成 "not allowed"。
  echo "[run-gui] 校验并构建当前客户端……" >&2
  (cd "$APP_DIR/src-tauri" && cargo build)
fi

if [[ -n "$FORCE_DIST" ]]; then
  echo "[run-gui] --force-dist：跳过 dev server，强制加载内嵌 dist" >&2
  # 必须导出：main.rs 依据该变量决定走 App("index.html") 而不是 External(1421)
  export MVP_FORCE_DIST=1
else
  if [[ ! -f "$DEV_SERVER_HELPER" ]]; then
    echo "[run-gui][ERROR] 缺少 dev-server 助手：$DEV_SERVER_HELPER" >&2
    exit 1
  fi
  # 私有 state dir：pidfile 只属于本次运行，避免跨运行误杀别人启动的 server
  DEV_STATE_DIR="$(mktemp -d)"
  export MVP_DEV_SERVER_STATE_DIR="$DEV_STATE_DIR"
  # shellcheck source=scripts/dev-server.sh disable=SC1090
  source "$DEV_SERVER_HELPER"
  dev_server_ensure
fi

# 继承/补全当前图形会话环境（DISPLAY / WAYLAND_DISPLAY / XDG_RUNTIME_DIR 等）
#
# 关键 workaround：WebKitGTK 在 Wayland 下默认使用 DMA-BUF 渲染器会崩溃
# （进程静默退出、白屏）。禁用后即可稳定运行。同时用 X11 后端规避
# Tauri v2 在 Wayland 下子 webview 定位错位的已知 bug。
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export GDK_BACKEND=x11

echo "[run-gui] 启动中…… WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11" >&2
# 不用 exec：保留本 shell，客户端退出后由 EXIT trap 回收自己拉起的 dev server
"$BIN" ${APP_ARGS[@]+"${APP_ARGS[@]}"}
exit $?
