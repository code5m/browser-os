# M0-3.a 安全威胁矩阵（V1.0）

> 建立时间：2026-08-31 ｜ 分支：`feature-M0-baseline`
> 范围：只**盘点与定契约**，不收口任何调用方（收口分属 M0-3.b/c/d）。
> 配套：契约模块 `src-tauri/src/security_policy.rs`；静态夹具 `scripts/check-security-policy.py`。
> 冻结基线证据（`logs/m0-baseline/**`）未改动；M1/M2 仍锁定；本文件不代表 M0 完成。

## 1. 资产与信任边界

| 资产 | 位置 | 说明 |
|------|------|------|
| 主窗口 `main` | Tauri 主 webview | 受信任，运行前端 bundle |
| 子 webview `tab-*` / `grid-*` | 浏览器页签 / 宫格 | **加载任意外部页面，属不可信内容** |
| 远端页面 IPC | `capabilities/browser-remote.json` | 外部域可通过 remote 权限调命令 |
| 本地文件系统 | `read_file`/`write_file`/`create_file`/`create_dir`/`delete_path` | 当前**无**允许根目录约束 |
| 外部进程 | `launch_app`（`sh -c`） | 当前**无**应用条目 allowlist |
| PTY 终端 | `term_spawn`/`term_write` | 用户在终端内的行为等同其本机权限，不作策略约束对象 |

**信任边界一句话**：`tab-*`/`grid-*` 内的外部页面是不可信输入；凡是「不可信内容能触达的命令」都必须有来源校验与意图校验。

## 2. 威胁清单（7 项，均标注源码证据）

| ID | 入口 | 威胁 | 现状证据 | 严重度 | 收口检查点 |
|----|------|------|---------|--------|-----------|
| SEC-01 | `bridge.rs:1629 launch_app(exec)` | 任意命令执行：`Command::new("sh").arg("-c").arg(cmd)`，无 allowlist、无审计 | `bridge.rs:1636-1637`；`default.json` 授予 `shell:allow-spawn`（bash/sh/powershell，`args: true`） | **P0** | M0-3.d |
| SEC-02 | `read_file` / `write_file` / `create_file` / `create_dir` / `delete_path` | 任意路径读写删：`std::fs::*` 直接使用传入路径，**无 canonicalize、无允许根目录、无符号链接检查**；`delete_path` 可 `remove_dir_all` 递归删 | `bridge.rs:1078/1084/1141/1157/1169` | **P0** | M0-3.c |
| SEC-03 | `capabilities/default.json` `windows`、`browser-remote.json` `webviews` | 残留已不存在的 `browser` label；若将来创建同名 webview 会继承默认权限集 | `default.json:6-9`、`browser-remote.json` webviews 首项 | **P1** | M0-3.b |
| SEC-04 | `capabilities/browser-remote.json` `remote.urls` | `https://*` / `http://*` 全通配：任意外部页面（含劫持/恶意页面）可调用 remote 权限集内命令 | `browser-remote.json` remote 段 | **P0** | M0-3.b |
| SEC-05 | `bridge.rs:68 .eval(&id.to_string(), js)` | 向子 webview 注入任意 JS；与 SEC-04 组合等于给外部页面留执行面 | `bridge.rs:68`、`bridge.rs:1672` | **P0** | M0-3.b |
| SEC-06 | 写/删类命令 | 无用户意图校验与审计日志：前端一次调用即可静默覆盖/删除用户文件 | 同 SEC-02 行号 | **P1** | M0-3.b/c |
| SEC-07 | `launch_app` 参数 | shell 元字符未过滤（`;` `&&` `\|` 反引号 `$( )` 重定向） | `bridge.rs:1629-1643` | **P0** | M0-3.d |

## 3. 已落地的最小契约（`src-tauri/src/security_policy.rs`）

| 契约函数 | 作用 | 拒绝用例（单测） |
|---------|------|-----------------|
| `check_webview_label` | 只承认 `main` / `tab-*` / `grid-*` | `browser` 被拒（对应 SEC-03） |
| `check_path_within_roots` | `canonicalize` + 允许根目录前缀 + 祖先符号链接目标校验 | `../` 逃逸被拒、符号链接逃逸被拒、未配置根目录默认拒绝（对应 SEC-02） |
| `check_shell_command` | 拒绝 `;&\|` `` ` `` `$` `><` 换行等元字符；只拒绝不清洗 | `code; rm -rf /`、`$(id)`、反引号、换行注入均被拒（对应 SEC-07） |
| `check_html` | 1 MiB 上限 + `<script`/`javascript:`/`<iframe`/`onerror=` | 超长与四类危险片段被拒 |
| `policy_fingerprint` | 启动日志打印策略版本，便于确认二进制对应的契约版本 | 单测断言含 `security-policy-v1` |

**刻意不做的事**：不在本检查点做「清洗」（如过滤掉元字符后继续执行）。清洗会制造安全假象，收口阶段应按**已解析应用条目**直接执行（M0-3.d）。

## 4. 可复跑证据

- 行为证据：`cargo test --manifest-path src-tauri/Cargo.toml security_policy`（拒绝用例，纯函数、无 GUI 依赖）。
- 形态证据：`scripts/check-security-policy.py`
  - `--self-test`：内置 fixture 证明检测器对「有缺口/已收口」两种源码都能正确判定；
  - `--expect-current-gaps`：断言当前缺口集合与本文档一致（防矩阵与代码漂移）；
  - 默认模式：当前**预期 `EXIT=1`** 并列出缺口，M0-3.b/c/d 收口后转 `EXIT=0`。
- 门禁接入：`scripts/pre-merge.sh` 只跑 `--self-test` 与 `--expect-current-gaps` 两种**应通过**的模式，默认模式不入门禁（现状缺口不是回归）。

## 5. M0-3 验收对照（供 M0-3.d 收尾时核对）

| 验收项（计划文档） | 覆盖位置 | 当前状态 |
|-------------------|---------|---------|
| 不存在的 `browser` label 已删除 | SEC-03 | ❌ 待 M0-3.b（契约已就绪） |
| 伪造 webview label 拒绝用例 | `check_webview_label` | ✅ 契约+单测就绪 |
| 无用户意图写入拒绝用例 | SEC-06 | ❌ 待 M0-3.b/c |
| 超长/危险 HTML 拒绝用例 | `check_html` | ✅ 契约+单测就绪 |
| `../` 逃逸拒绝用例 | `check_path_within_roots` | ✅ 契约+单测就绪 |
| 符号链接逃逸拒绝用例 | `check_path_within_roots` | ✅ 契约+单测就绪 |
| shell 元字符拒绝用例 | `check_shell_command` | ✅ 契约+单测就绪 |
| 允许路径与合法应用仍可用 | 正向用例（`/usr/bin/code`、允许根目录内文件） | ✅ 单测已含 |
