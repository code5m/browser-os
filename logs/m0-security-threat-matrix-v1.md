# M0 安全威胁矩阵（V1.2）

> 建立时间：2026-08-31（M0-3.a）｜ V1.1：M0-3.b 收口 ｜ **V1.2：M0-3.c 写路径收口** ｜ 分支：`feature-M0-baseline`
> 范围：M0-3.a 盘点定契约；M0-3.b 收口 capability/远程 IPC；**M0-3.c 收口写路径（只读浏览按裁决保留并登记 SEC-09）**；M0-3.d 收口 launch_app。
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
| SEC-02 | 写/删类文件命令（`write_file`/`create_file`/`create_dir`/`delete_path`/`rename_path`） | 任意路径写删、递归删、`rename` 新名称可带 `../` 跨目录移动 | 原 `bridge.rs` 文件命令区 | ~~P0~~ **已关闭** | ✅ M0-3.c |
| SEC-03 | `capabilities/*.json` | 残留已不存在的 `browser` label | 原 `default.json` windows、`browser-remote.json` webviews | ~~P1~~ **已关闭** | ✅ M0-3.b |
| SEC-04 | `browser-remote.json` `remote.urls` | `https://*`/`http://*` 全通配：外部页面可调用远程集内命令 | `browser-remote.json` remote 段 | **P0 → 可接受残余风险** | 补偿控制已落地（见 §5） |
| SEC-05 | `bridge.rs:68 .eval(&id.to_string(), js)` | 向子 webview 注入任意 JS；与 SEC-04 组合等于给外部页面留执行面 | `bridge.rs:68`、`bridge.rs:1672` | **P0** | M0-3.b |
| SEC-06 | 写/删类命令 | 无用户意图校验与审计 | 同 SEC-02；远程侧 `save_note`/`collect_selection` | **P1** | 远程侧 ✅ M0-3.b；本地路径 M0-3.c |
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

## 5. V1.2 变更（M0-3.c 写路径收口）

裁决（用户 2026-08-31）：**写操作强制收口；只读浏览保留但登记风险**。

| 变更 | 内容 |
|------|------|
| 允许根目录 | `allowed_roots()` = 主目录 / Desktop / Documents / Downloads / 成果工作区 / 笔记目录。取值与 `get_start_dirs` 对外承诺一致，避免「能列出却写不进」 |
| 写/删收口 | `write_file`/`create_file`/`create_dir`/`delete_path`/`rename_path` 全部经 `check_path_within_roots`（canonicalize + 根目录前缀 + 祖先符号链接目标校验） |
| 删根保护 | `check_delete_target` 禁止删除允许根目录本身，防一次调用清空工作区/主目录 |
| 重命名逃逸修复 | `rename_path` 原为 `parent.join(new_name)`，`new_name=../../etc/x` 即跨目录移动；新增 `check_path_component` 拒绝 `..`/分隔符/NUL |
| 新建目录写后复核 | `create_file`/`create_dir` 先校验已存在祖先、创建后再校验真实落点，防中途被符号链接替换 |
| open_source scheme 白名单 | 只允许 http/https；`file:`/`smb:` 等会被 `open::that` 交桌面环境执行的 scheme 全部拒绝 |
| SEC-09 登记（未收口） | `list_dir`/`read_file`/`browse_workspace`/`get_start_dirs` 等只读浏览仍可读取任意路径（信息泄露面），按裁决暂不收口以免破坏文件管理器，留待后续裁决 |

> 收口后机器可检缺口 5 → 4：`FILE_COMMANDS_WITHOUT_PATH_POLICY` 关闭，`READ_ONLY_BROWSE_WITHOUT_PATH_POLICY` 新增并登记为已知风险。
> 夹具改进：写命令是否接入策略改为**逐函数体**判定（此前全文匹配会被「引用但未使用」骗过）。

## 5.1 V1.1 变更（M0-3.b 收口）

| 变更 | 内容 |
|------|------|
| SEC-03 关闭 | `default.json` windows 删除 `browser`；`browser-remote.json` webviews 收紧为 `tab-*`/`grid-*` |
| SEC-08 新增并关闭 | 发现远程权限集 `remote-collect` 实际放行 **6 个**命令（含 `save_note` 写盘、`request_open_terminal` 授予 shell），与文件自述「只允许 collect_selection 与 report_resources」不符。已收紧为仅回传类 3 个 |
| 来源校验 | 上报与副作用命令全部加 `tauri::Webview` 参数，label 必须 ∈ {`main`,`tab-*`,`grid-*`} |
| 用户意图令牌 | 新增 `IntentRegistry`（一次性、30s TTL、作用域绑定）+ `issue_intent`（仅 `main` 可签发）；`save_note`/`collect_selection`/`request_open_terminal` 来自外部页面时必须出示令牌 |
| 载荷边界 | `report_resources` ≤ 500 条目；`report_title`/文本字段 ≤ 64 KiB；`collect_selection` 的 html 走 1 MiB 上限 |
| SEC-04 补偿控制 | 远程 URL 全通配是浏览器固有属性，无法收窄；改以「来源 + 意图 + 载荷边界」作为补偿控制，残余风险接受并登记 |

> 收口后机器可检缺口 5 → 4（移除 `CAPABILITY_STALE_BROWSER_LABEL`，新增 `REMOTE_SIDE_EFFECT_COMMANDS_EXPOSED` 并同步关闭）。
> 说明：`save_note`/`request_open_terminal` 经核查**无任何前端调用点**，`collect_selection` 仅主窗口 `ActivityBar` 按钮使用，因此收紧远程集不影响现有功能。

## 6. M0-3 验收对照（供 M0-3.d 收尾时核对）

| 验收项（计划文档） | 覆盖位置 | 当前状态 |
|-------------------|---------|---------|
| 不存在的 `browser` label 已删除 | SEC-03 | ✅ 已删除（M0-3.b） |
| 伪造 webview label 拒绝用例 | `check_webview_label` | ✅ 契约+单测就绪 |
| 无用户意图写入拒绝用例（远程侧） | SEC-06/08 | ✅ `remote_invocation_without_token_is_rejected` 等 4 项 |
| 超长/危险 HTML 拒绝用例 | `check_html` | ✅ 契约+单测就绪 |
| `../` 逃逸拒绝用例 | `check_path_within_roots` | ✅ 契约+单测+生产接入（M0-3.c） |
| 符号链接逃逸拒绝用例 | `check_path_within_roots` | ✅ 契约+单测+生产接入（M0-3.c） |
| shell 元字符拒绝用例 | `check_shell_command` | ✅ 契约+单测就绪 |
| 允许路径与合法应用仍可用 | 正向用例（`/usr/bin/code`、允许根目录内文件） | ✅ 单测已含 |
