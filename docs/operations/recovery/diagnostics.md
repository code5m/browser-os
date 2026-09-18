# Agent C — Runtime Diagnostics（运行时取证）

脚本：`scripts/collect-diagnostics.sh`　产物：`diagnostics/<timestamp>/{system,process,app,git}.txt`

---

## 1. 定位

**DIAGNOSTICS BEFORE ROLLBACK** —— 异常发生时先取证，再决定动作。

本脚本解决："现在到底发生了什么？" 它**只读**、**不杀进程**、**不重启应用**，因此可以在故障现场反复执行而不会破坏证据。

## 2. 用法

```bash
./scripts/collect-diagnostics.sh [label]    # label 默认 diag
./scripts/collect-diagnostics.sh --list
./scripts/collect-diagnostics.sh --help
```

输出：

```
DIAGNOSTICS_CREATED
path: /…/diagnostics/20260918-231503
timestamp: 2026-09-18T23:15:04+08:00
files: system.txt process.txt app.txt git.txt
redaction: applied (no keyring / cookie / user-file content collected)
```

## 3. 四个文件各有什么

| 文件 | 内容 | 用来判断 |
|---|---|---|
| **system.txt** | 内核/OS/架构、桌面会话（`XDG_SESSION_TYPE`、`DISPLAY`/`WAYLAND_DISPLAY`）、CPU/loadavg/`free -h`、磁盘 `df`、原生依赖版本（`webkit2gtk-4.1/4.0`、`libsoup-3.0`、`gtk+-3.0`）、已安装 webkit 包版本 | 环境问题 vs 代码问题；原生库缺失/版本漂移 |
| **process.txt** | 应用进程（`pgrep -af`，脱敏）、**WebKit 子进程计数**（`WebKitWebProcess` / `WebKitNetworkProcess`）、僵尸进程数、按 CPU/RSS 的 Top10 | 卡死/泄漏/多进程残留；宫格子窗是否还活着 |
| **app.txt** | 已安装二进制（路径/大小/mtime/**sha256**）、`dpkg` 状态与版本、桌面 entry 与图标是否存在、`ldd` 中的 webkit/gtk/gstreamer 链接、**应用数据目录元数据**（只列名字+大小+时间）、日志目录清单 + **最新日志与 crash.log 的脱敏尾部 40 行** | 版本错配、安装损坏、原生链接问题、崩溃栈 |
| **git.txt** | HEAD/branch/describe/latest tag/dirty 文件/最近 10 提交，并提示"dirty 说明运行二进制可能不等于源码" | 跑的代码是哪一份 |

## 4. 敏感数据红线（脚本强制遵守）

**绝不采集**：

- 系统密钥库（keyring）任何条目 —— 包括 git token、数据库口令、已导入网站密码
- cookie 内容与 WebKit 持久会话数据
- 用户文件内容（抓取的网页正文/图片、`repos/` 工作副本、`~/Documents/极智笔记/*.md`、归档 vault）
- 会话/浏览历史正文

**只采元数据**：应用数据目录仅列 **文件名 + 大小 + mtime**，不打开任何文件。

**日志强制脱敏**：所有动态文本（日志尾、进程命令行）都过 `redact()`：

- URL 中的 `token/access_token/api_key/apikey/key/secret/password/passwd/pwd/auth/sig/signature` 参数值 → `<REDACTED>`
- `Bearer <xxx>` → `Bearer <REDACTED>`
- `sk-…`、`AKIA…` → `<REDACTED>`
- PEM 私钥块之后内容 → `<REDACTED>`

> 注意：模块自身错误已由后端 `sanitize_message` 处理，但**第三方 stderr（WebKit/GStreamer）不过滤**，所以日志尾部必须走这里的正则脱敏，不能直接转发原始日志。

## 5. 取证顺序（Standard Triage）

1. **先跑一次**（现场不要动）：
   ```bash
   ./scripts/collect-diagnostics.sh <symptom>
   ```
2. **看 app.txt 的二进制 hash 与 dpkg**：判断"装的版本"与"源码 HEAD"是否一致（`git.txt` 的 dirty 会提示不一致）。
3. **看 system.txt 的原生依赖**：webkit/gtk 缺失或版本漂移 → 环境问题，不是业务 bug。
4. **看 process.txt 的 WebKit 子进程数**：异常增多 = webview 泄漏；为 0 但应用声称宫格打开 = 资源已消失。
5. **看 app.txt 的日志尾 + crash.log**：panic/segfault 栈在这里。
6. **再决定**：环境问题 → 修环境；代码问题 → 走 `git-workflow.md` 回滚矩阵；安装问题 → 走 `release-recovery.md`。

## 6. 不做什么（避免二次破坏）

- 不 kill、不 restart、不 signal 任何进程（现场优先）。
- 不清理缓存、不删 `~/.local/share/...` 下任何内容。
- 不修改任何被跟踪文件。
- 不因为"看起来是缓存问题"就删数据目录 —— 那是 `data-backup.md` 的备份还原范畴，且必须先备份。

## 7. 与崩溃处置的配合

原生崩溃/挂起的分类与处置清单见 `crash-handling.md`；本脚本是其**第一步取证工具**。
