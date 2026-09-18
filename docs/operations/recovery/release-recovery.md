# Agent E — Release Recovery（构建 / 安装 / 启动恢复）

---

## 0. 关键事实（实测 + 脚本确认）

| 项 | 值 |
|---|---|
| 包名 / 产品名 | `mvp-browser-os` |
| **deb 版本** | **恒为 `0.1.0`**（来自 `src-tauri/tauri.conf.json` 的 `version`） |
| git 版本标签 | `v0.2.0 … v0.11.0` —— **另一套体系**，与 deb 版本无关 |
| deb 产物路径 | `src-tauri/target/release/bundle/deb/mvp-browser-os_0.1.0_amd64.deb` |
| 安装后二进制 | `/usr/bin/mvp-browser-os` |
| 桌面入口 | `/usr/share/applications/mvp-browser-os.desktop` |
| 图标 | `/usr/share/icons/hicolor/512x512/apps/mvp-browser-os.png` |
| 一键打包+安装 | `npm run release:install`（= `bash scripts/verify-installed-client.sh --install`） |
| 仅打包 | `npm run verify:client`（`--build`），无头机可加 `--no-start` |

⚠️ **判断"装的是哪一版"不能看 deb 版本号**（恒 0.1.0，同版本会被重复覆盖安装）。必须看 **构建时间 + commit + 二进制 sha256** —— 这些都由 `diagnostics/*/app.txt` 提供。

---

## 1. 失败模式与处置

### F1 构建成功，但打包装错（dist 未更新 / 旧前端被打进包）

- **判据**：`npm run build` 通过，但运行表现是旧 UI；`app.txt` 的二进制 mtime 早于最近前端改动。
- **处置**：`tauri.conf.json` 的 `beforeBuildCommand` 为 `npm run build`，正常会先构建前端；若怀疑跳过了：
  ```bash
  rm -rf dist && npm run build          # 强制重建前端
  bash scripts/verify-installed-client.sh --build
  ```
- **风险**：`dist` 与 `src-tauri/target` 缓存都可能造成"构建成功但产物旧"；必要时 `cargo clean` 会显著拉长构建时间。

### F2 deb 安装失败

- **判据**：`apt`/`dpkg` 报错（依赖、锁、权限）。
- **处置**：
  ```bash
  sudo dpkg -i src-tauri/target/release/bundle/deb/mvp-browser-os_0.1.0_amd64.deb
  sudo apt -f install                    # 修依赖
  dpkg-query -W -f='${Package} ${Version} ${Status}\n' mvp-browser-os
  ```
- **注意**：`release:install` 会 `sudo apt install --reinstall` 覆盖同版本包。**sudo 密码必须由用户在本地终端输入**，Agent 不得索要/记录密码；非交互环境只能生成命令交给用户执行。

### F3 启动失败（装得上、起不来）

- **判据**：`verify-installed-client.sh` 的冷启动校验失败，或手动启动立即退出。
- **处置**（先取证再动）：
  ```bash
  ./scripts/collect-diagnostics.sh startup-fail
  sed -n '/LOGS/,$p' diagnostics/$(ls -t diagnostics | head -1)/app.txt
  ```
  看 `system.txt` 的原生依赖（webkit2gtk/libsoup/gtk 是否缺失或漂移）与 `process.txt`（是否有残留进程占用）。
- **风险**：别急着清数据目录 —— 先按 `data-backup.md` 备份，再考虑清 WebKit 缓存。

### F4 桌面入口失效（图标点不动 / 无菜单项）

- **判据**：`app.txt` 显示 desktop/图标 `absent`，或 desktop 文件存在但 `Exec=` 指向错误路径。
- **处置**：
  ```bash
  ls -l /usr/share/applications/mvp-browser-os.desktop /usr/share/icons/hicolor/512x512/apps/mvp-browser-os.png
  cat /usr/share/applications/mvp-browser-os.desktop
  sudo update-desktop-database 2>/dev/null || true
  ```
  仍不行 → 重装 deb（F2）。

### F5 二进制错配（版本/内容与源码不一致）

- **判据**：`git.txt` 显示工作树 dirty，或 `describe` 与预期 tag 不符；`app.txt` 的 sha256 与已知好产物不同。
- **处置**：回到已知好 tag 重新构建（deb 版本恒 0.1.0，`apt` 不会"降级"）：
  ```bash
  git worktree add --detach /tmp/rb-release semantic-phase1-browser-grid-code-pass
  cd /tmp/rb-release && npm ci && npm run release:install
  ```
- **风险**：覆盖 `/usr/bin/mvp-browser-os` 前，务必已用 `collect-diagnostics.sh` 记录旧二进制的 sha256（否则丢失证据）。

---

## 2. 标准恢复流程（bad release）

```text
bad release
   ↓
stop app            pkill -f mvp-browser-os   （先停，避免 atomic_write 中间态）
   ↓
collect diagnostics ./scripts/collect-diagnostics.sh bad-release
   ↓
install known good deb
                    # 优先：已有 releases/<version>/ 产物（见 §3）
                    sudo apt install --reinstall ./releases/<version>/mvp-browser-os_0.1.0_amd64.deb
                    # 或：从已知好 tag 重建
                    npm run release:install
   ↓
verify hash         sha256sum /usr/bin/mvp-browser-os  （与 releases/<version>/checksum 比对）
   ↓
start               mvp-browser-os &  或点桌面图标
   ↓
collect diagnostics ./scripts/collect-diagnostics.sh after-recovery
```

---

## 3. 发布产物结构（当前**尚未建立**，本文件为其规范）

仓库目前**没有 `releases/` 目录**，deb 只留在 `src-tauri/target/release/bundle/deb/`（会被 `cargo clean` 清掉，且无法回答"这个 deb 是哪个 commit 打的"）。建议建立：

```text
releases/
└── <version>/                 # 建议用 <deb版本>+<date>+<short-sha>，如 0.1.0+20260918+ad587bc
    ├── mvp-browser-os_0.1.0_amd64.deb
    ├── checksum               # sha256 of deb AND of /usr/bin/mvp-browser-os
    ├── commit                 # 完整 commit sha
    ├── tag                    # 对应 git tag（无 tag 则写 describe 结果）
    ├── build-time             # 构建时间戳（判断版本的关键，因为 deb 版本号恒定）
    └── changelog              # 相对上一版的变更
```

建立产物的最小命令：

```bash
V="0.1.0+$(date +%Y%m%d)+$(git rev-parse --short HEAD)"
mkdir -p "releases/$V"
cp src-tauri/target/release/bundle/deb/mvp-browser-os_0.1.0_amd64.deb "releases/$V/"
sha256sum "releases/$V/mvp-browser-os_0.1.0_amd64.deb" > "releases/$V/checksum"
sha256sum /usr/bin/mvp-browser-os >> "releases/$V/checksum"
git rev-parse HEAD > "releases/$V/commit"
git describe --tags > "releases/$V/tag"
date -Is > "releases/$V/build-time"
```

> `releases/` 为**本地未跟踪产物**（本层禁止增加 ignore）；若要长期保存应放到仓库外或专用存储。

---

## 4. 已知好基线（最后手段）

| 基线 | 用途 |
|---|---|
| `baseline-m1-0-approved`（提交 `04ad3f5`） | 用户实测稳定的 M1-0；功能最少但最稳，作为"回滚到底"的兜底 |
| `semantic-phase1-browser-grid-code-pass`（`a30fd57`） | 当前语义收敛后的最佳回滚点 |

回到基线 = **从该 tag 重新构建 deb 再安装**（不能靠 apt 降级，版本恒 0.1.0）。
