#!/usr/bin/env bash
# 极智简单 · 浏览器OS融合 MVP —— Linux 一键环境搭建
# 适配: Ubuntu 24.04 (Noble) / Debian 系; 自动探测其他发行版并提示
set -euo pipefail

# ---------- 0. (可选) 注册为系统默认浏览器（显式子命令，先于主流程判定） ----------
# M1-4 硬约束：本脚本默认流程绝不触碰系统默认浏览器；只有用户显式传入
# --register-default-browser 才执行。首选路径是应用内「设置 → 默认浏览器」按钮，
# 本函数仅作命令行等价入口。desktop 文件名实测发现，禁止凭猜。
register_default_browser() {
  local desktop=""
  for c in \
    /usr/share/applications/mvp-browser-os.desktop \
    /usr/share/applications/com.jizhijiandan.mvp.desktop \
    "$HOME/.local/share/applications/mvp-browser-os.desktop"; do
    if [ -f "$c" ]; then desktop="$(basename "$c")"; break; fi
  done
  if [ -z "$desktop" ]; then
    echo "!! 未找到本应用 .desktop 条目（/usr/share/applications 或 ~/.local/share/applications）"
    echo "   请先安装应用，或在应用内「设置 → 默认浏览器」一键注册后重试"; exit 1
  fi
  echo "==> 注册默认浏览器: $desktop（用户显式触发）"
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
  fi
  xdg-settings set default-web-browser "$desktop"
  echo "    当前默认浏览器: $(xdg-settings get default-web-browser)"
}
if [ "${1:-}" = "--register-default-browser" ]; then
  register_default_browser
  exit 0
fi

echo "==> 探测系统..."
if [ -f /etc/os-release ]; then
  . /etc/os-release
  DISTRO="$ID"
else
  DISTRO="unknown"
fi
echo "    发行版: ${PRETTY_NAME:-$DISTRO}"

# ---------- 1. 系统级依赖 ----------
install_apt() {
  echo "==> [apt] 安装 Tauri v2 + git2 依赖..."
  sudo apt-get update
  sudo apt-get install -y --no-install-recommends \
    build-essential \
    curl wget file \
    pkg-config cmake \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libgit2-dev \
    libsecret-1-dev \
    libdbus-1-dev
}

case "$DISTRO" in
  ubuntu|debian|linuxmint|pop) install_apt ;;
  *) echo "!! 未识别发行版 ($DISTRO)，请手动安装 Tauri 前置: https://tauri.app/start/prerequisites/"; exit 1 ;;
esac

# ---------- 2. 工具链校验 ----------
echo "==> 校验工具链..."
need() { command -v "$1" >/dev/null 2>&1 || { echo "!! 缺少: $1"; MISSING=1; }; }
MISSING=0
need cargo; need rustc; need node; need npm; need cmake; need pkg-config
if [ "$MISSING" = "1" ]; then
  echo "!! 请先安装缺失工具后重试"; exit 1
fi
echo "    cargo : $(cargo --version)"
echo "    node  : $(node --version)"
echo "    npm   : $(npm --version)"

# ---------- 3. 前端依赖 ----------
echo "==> 安装前端依赖 (npm install)..."
npm install

# ---------- 4. 编译校验 (仅编译, 不启动 GUI) ----------
echo "==> 编译 Rust 桥 (cargo check, 首次会拉取并编译依赖, 需数分钟)..."
# 使用系统 libgit2, 避免从源码 vendored 编译
export LIBGIT2_SYS_USE_PKG_CONFIG=1
cd src-tauri && cargo check --message-format short; cd ..

echo ""
echo "✅ 环境就绪!"
echo "   启动开发模式(需桌面显示):  npm run tauri dev"
echo "   打包:                      npm run tauri build"
echo ""
echo "⚠️ 注意: keyring 凭据库需要桌面密钥服务 (gnome-keyring) 在运行时可用;"
echo "   服务器/无头环境请改用 keyring 的 mock 或环境变量注入 token (见 README)。"
