#!/usr/bin/env bash
# 在你自己的图形终端（gnome-terminal）里运行本脚本，
# 应用会从当前 Wayland/X11 桌面会话弹出窗口。
#
# 用法：
#   bash /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/run-gui.sh
#
# 如需观察日志（排查白屏/崩溃），末尾加 2>&1 | tee /tmp/mvp-gui.log

set -e
APP_DIR="/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3"
BIN="$APP_DIR/src-tauri/target/debug/mvp-browser-os"

if [ ! -x "$BIN" ]; then
  echo "[run-gui] 未找到二进制，先编译……"
  (cd "$APP_DIR/src-tauri" && cargo build)
fi

# 继承当前图形会话的环境（DISPLAY / WAYLAND_DISPLAY / XDG_RUNTIME_DIR 等）
#
# 关键 workaround：WebKitGTK 在 Wayland 下默认使用 DMA-BUF 渲染器会崩溃
# （进程静默退出、白屏）。禁用后即可稳定运行。同时用 X11 后端规避
# Tauri v2 在 Wayland 下子 webview 定位错位的已知 bug。
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export GDK_BACKEND=x11

echo "[run-gui] 启动中…… WEBKIT_DISABLE_DMABUF_RENDERER=1 GDK_BACKEND=x11"
exec "$BIN" "$@"
