# A9 R2B — S0 / S1 / S4 候选能力预算（允许能力 · 禁止旁路 · 必须反例）

- **Lane**: A9
- **Dispatch**: R2B §10 末尾「给每个候选 S0/S1/S4 列允许能力、禁止旁路和必须跑的反例」
- **来源基线**: `capabilities/default.json`、`browser-remote.json`、`dev-capabilities/main.json`、`permissions/default-commands.toml`、`remote-collect.toml`、`security_policy.rs`、`PROJECT-RULES.md`、`WORKBENCH_BLUEPRINT-20260908.md` §6
- **约束**: 候选开发片（S0/S1/S4）按蓝图 §6 尚未即时编码授权；本预算为**安全契约冻结建议**，交 A0 裁决、A11 入集成清单

---

## 通用禁止旁路（适用所有候选片）

1. **禁止用扩大 `browser-remote.remote.urls` 修复 debug IPC**（R2B §8 明示）：debug 来源修复走 `dev-capabilities/main.json`（`localhost:1421`），不得把 `https://*`/`http://*` 改成更宽。
2. **禁止恢复 `queue_resize` / `set_size_request` / `webview.hide()`**（规则 1/3.5 死锁/沾满红线）。
3. **新命令三源一致**：`main.rs` 注册 + `default-commands.toml`/`remote-collect.toml` ACL + 前端 `invoke` 必须同包提交（A9 BUG-HUNT 三方比对门禁 `check-command-set-consistency.py` 已知漂移须消除）。
4. **禁止向远程 webview（`tab-*`/`grid-*`）开放任何有副作用命令**：side-effect 命令须 `main` 签发的一次性意图令牌（`bridge.rs:1985`）。
5. **凭据不落明文文件**：apiKey/token 走 OS keychain（`KeyringStore`）；同用户进程可读 0600 文件，故禁止用文件存密钥。

---

## S0 — 现有契约与可运行基线

**允许能力**（已存在，维持）：
- 主窗：`core:default` + `core:window:allow-create` + `browser-tabs:default` + `default-commands`（135 命令，`default-commands.toml`）。
- 远程 webview：`remote-collect` 仅 `report_resources`/`report_title`/`report_grid_load_failed`（`remote-collect.toml:8`）。
- debug：`dev-capabilities/main.json`（`localhost:1421`，`#[cfg(debug_assertions)]`）。
- release：`tauri.conf.json` 无 `devUrl` → bundled `tauri://localhost`，无 Vite 依赖。

**禁止旁路**：
- 不得给 release 二进制加 `devUrl`（破坏桌面运行时来源门）。
- 不得把 `remote-collect` 扩到含 side-effect 命令。
- 不得改动 `security_policy.rs` 启动黑名单（BLOCKED_LAUNCH_PROGRAMS/WRAPPERS/INTERPRETERS）使其可绕过（规则 1 同源防回退）。

**必须跑反例（交 A11）**：
- `S0-1`：从 `tab-*` webview `invoke('save_note',...)` 无令牌 → 必须拒。
- `S0-2`：伪造 label `browser` 调 `report_resources` → `check_webview_label` 必须拒。
- `S0-3`：release 构建启动后访问 `localhost:1421` → 必须无 Vite dev 依赖（来源门）。
- `S0-4`：`launch_app('env bash -c id')` → `BLOCKED_LAUNCH_WRAPPERS` 必须拒（`security_policy.rs:176`）。

---

## S1 — 工作台外壳（含原生 WebView 边界）

**允许能力**（已存在/锁规则）：
- `core:window:allow-create`、`browser-tabs:default`、原生子 webview 定位用 `gtk_fixed_move`+`size_allocate`（规则 1）、CSS px 坐标（规则 3）、隐藏用 `-30000` 屏外位移（规则 3.5）。
- 新布局命令若新增**远程 webview 调用**：必须同步入 `remote-collect.toml`（规则 3.6）。

**禁止旁路**：
- 不得因重新布局恢复 `queue_resize`/`set_size_request`/`webview.hide()`（永久防回退项 3）。
- 不得为修 debug IPC 宽化 `browser-remote.remote.urls`。
- 菜单/搜索/弹窗不得被 tab/grid webview 覆盖（z-order 契约）。

**必须跑反例（交 A11，本批 `NOT_RUN`）**：
- `S1-1`：原生菜单不被 webview 遮挡（见 `A9-R2B-webview-boundary.md` S-WV-1）。
- `S1-2`：DPI 1/1.5/2 坐标稳定（S-WV-2）。
- `S1-3`：隐藏→切回焦点归还且无死锁（S-WV-3）。
- `S1-4`：IME 输入不误触全局快捷键（S-WV-5）。
- `S1-5`：窄窗 800×600 下活动栏/搜索可达、文本不遮挡（蓝图 §3）。

---

## S4 — 工作区明确匹配搜索（范围/忽略/有界/取消/导航）

**允许能力**（待建，须冻结）：
- **本地索引仅在 allowed roots 并集内**；无远程外发（R2B §10.7）。
- 索引尊重 `.gitignore`/忽略规则（对齐 A7 `DEFAULT_IGNORE_RULES`、`MAX_GITIGNORE_CACHE_ENTRIES=4096`）。
- 结果有界（`DEFAULT_LIMIT`/`RECALL_MAX_DEPTH` 由 A8 定阈）、可取消、点击带资源类型+定位信息（蓝图 §3-5 统一导航）。
- 查询内容/路径若含敏感键，复用 `redact_sensitive_url` / `SENSITIVE_QUERY_KEYS` 脱敏后再入库/展示。

**禁止旁路**：
- 索引**不得跟随 symlink 逃逸 allowed roots**（`check_path_within_roots` 须 canonicalize，复用 `resolve_program_file` 范式）。
- **默认不索引 `.env` 等含密文件**，或索引前脱敏（R2B §10.4）。
- 不得在未获 per-workspace HMAC 授权时开启远程 embedding（R1 DEFER）。
- 搜索结果不得伪装成磁盘文件（蓝图 §3-5 统一导航）。

**必须跑反例（交 A11）**：
- `S4-1`：工作区内 symlink → `/etc` → 索引不得逃逸（symlink 复核）。
- `S4-2`：`.env` 含 `api_key=...` → 不得原样入库或须脱敏。
- `S4-3`：尝试启用远程 embedding 无 HMAC grant → 必须失败关闭。
- `S4-4`：查询串含 `api_key=xxx` → 入库/展示前经 `redact_sensitive_url` 置 `***`。
- `S4-5`：`../../etc/passwd` 作为 rename/索引目标 → `check_path_component` 拒。

---

## 跨片一致性

- S0/S1/S4 共享同一 ACL 三源一致性门禁与同一 `security_policy` 路径/意图令牌/脱敏契约。
- S4 安全契约依赖 A7（引擎）+ A8（基准）同批冻结；A9 仅提供安全边界，不裁决引擎选型（R2B 审计 #5/#6 未定项不影响本预算）。
