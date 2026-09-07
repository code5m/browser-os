# M5-W17 · 如何拉起原生桌面客户端（Lane A2 交付）

> 面向最终用户与验收人。本文件只描述**启动方式**，不含任何产品代码改动。
> 配套脚本：`run-gui.sh`（入口）、`scripts/dev-server.sh`（ownership-safe dev-server 助手）、
> `scripts/check-dev-startup.sh`（启动冒烟自检）。

---

## 1. 一句话启动（推荐）

在**自己的图形终端**（gnome-terminal / 桌面会话内的终端）里执行：

```bash
bash /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/run-gui.sh
```

窗口会从当前 Wayland/X11 桌面会话直接弹出。

需要留日志排查白屏/崩溃时，末尾接 `tee`：

```bash
bash /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/run-gui.sh 2>&1 | tee /tmp/mvp-gui.log
```

---

## 2. 前置条件

| 项 | 要求 | 说明 |
|---|---|---|
| 终端位置 | **必须在图形桌面会话内** | 脚本继承 `DISPLAY` / `WAYLAND_DISPLAY` / `XDG_RUNTIME_DIR`；SSH/纯 TTY 下无窗口 |
| 首次运行 | 需能编译 | 若 `src-tauri/target/debug/mvp-browser-os` 不存在，脚本自动执行 `cargo build` |
| 依赖 | 系统已装 Rust/WebKitGTK 等 Tauri 依赖 | 与既有开发要求一致，本轮未新增依赖 |

---

## 3. 启动时会依次发生什么

1. **（可选）编译**：二进制缺失时 `cargo build`（`--no-build` 可跳过）。
2. **确保 Vite dev server 就绪**：调用 `scripts/dev-server.sh ensure`，
   目标 `http://127.0.0.1:1421`。
   - debug 主窗口在 `src-tauri/src/main.rs:1210-1214` 强制指向该 External URL，
     因此 **1421 无人监听 = 经典的 "connection refused / 白屏"**。本步即修复该痛点。
   - 端口**已有服务在跑** → 判定为"他人所有"，**不接管、不重启、退出时不杀**（ownership-safe）。
   - 无人监听 → 由本脚本拉起，并有界等待就绪（默认 60s，超时明确报错且不留下孤儿进程）。
3. **注入图形环境 workaround**：
   `WEBKIT_DISABLE_DMABUF_RENDERER=1`（规避 Wayland 下 DMA-BUF 渲染器崩溃/白屏）、
   `GDK_BACKEND=x11`（规避 Tauri v2 在 Wayland 下子 webview 定位错位）。
4. **拉起客户端** `src-tauri/target/debug/mvp-browser-os`（不使用 `exec`，以便退出时清理）。
5. **退出时只回收"本次自己拉起的"那个 dev server**：通过**私有临时 state dir + pidfile**
   （`MVP_DEV_SERVER_STATE_DIR`）隔离，跨运行/他人启动的服务不受影响。

---

## 4. 常用变体

```bash
# 跳过 dev server，强制加载内嵌 dist（等价于 main.rs 的 MVP_FORCE_DIST 分支）
bash run-gui.sh --force-dist

# 二进制已存在，跳过 cargo build（启动更快）
bash run-gui.sh --no-build

# 查看帮助（不构建、不启动）
bash run-gui.sh --help

# 透传参数给客户端二进制
bash run-gui.sh -- <客户端参数...>
```

`dev-server.sh` 也可单独使用（排查端口问题时有用）：

```bash
bash scripts/dev-server.sh url      # 打印 dev server URL
bash scripts/dev-server.sh is-up    # 探测 up/down（退出码 0/1）
bash scripts/dev-server.sh ensure   # 确保就绪（已有则不动）
bash scripts/dev-server.sh stop     # 只停本助手记录的那个（无 pidfile 则不动）
```

可用环境变量覆盖：`MVP_DEV_SERVER_{PORT,HOST,URL,CMD,TIMEOUT_SECS,STATE_DIR}`。

---

## 5. Release 与 Debug 的区别（重要）

- **Debug**（本脚本默认）：`WebviewUrl::External("http://localhost:1421")` → 走 Vite dev server，
  支持前端热更新，但**必须先有 1421 在监听**（本脚本已自动保证）。
- **Release / `--force-dist`**：`WebviewUrl::App("index.html")` → 使用打包进二进制的
  `dist/` 资源，**不需要** dev server。
- 该资产选择逻辑由既有 `main.rs` 决定，**A2 本轮未改动 `main.rs`**，也未改动任何
  命令 / bridge / ACL / 生产运行时权限。

---

## 6. 排查

| 现象 | 处理 |
|---|---|
| 仍 `connection refused` | 确认未被 `--force-dist` 误加；看日志中 `[run-gui]` 是否打印 dev server 就绪；用 `bash scripts/dev-server.sh is-up` 复核 |
| 白屏/进程静默退出 | 脚本已自动注入 `WEBKIT_DISABLE_DMABUF_RENDERER=1`+`GDK_BACKEND=x11`；仍异常请附 `/tmp/mvp-gui.log` |
| 端口 1421 被别的进程占用 | 属"他人所有"，脚本**不会**替你杀掉。请自行确认该服务是否就是本项目 Vite，或改用其它端口（`MVP_DEV_SERVER_PORT`） |
| dev server 启动超时 | 等 60s 后明确失败并自动回收，不会留下孤儿；检查 `npm run dev` 能否独立跑通 |

---

## 7. 验收证据（本轮 Closeout 复跑）

```
$ bash scripts/check-dev-startup.sh
==== 冒烟结果：PASS=23 FAIL=0 ====   (exit 0)
```

覆盖：他人服务不接管不清理 / 自己拉起→就绪→只回收自己的 / 超时明确失败且无孤儿 /
run-gui.sh 静态契约（可执行、引用助手、EXIT cleanup、非 exec、保留 WEBKIT+GDK、
MVP_FORCE_DIST、`--help` 退出 0）/ main.rs debug↔release 资产选择与 ACL/bridge 未改回归。

> 原生客户端的**视觉效果仍需用户侧人工验收**（A8 负责）；本文件只保证"能起来、起来的是正确资产、退出不误伤他人进程"。
