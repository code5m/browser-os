# 04 — Side Effect Closure Audit

> 扫描所有重要副作用（WebView / Process / PTY / Filesystem / Keyring / IPC / Network），
> 检查：是否已登记 / 有 Owner / 有安全边界 / 有 Checker。

## 1. 已登记副作用（Registry 覆盖，范围内）

| ID | Side Effect | call_sites | Owner | Checker | Decision |
|---|---|---|---|---|---|
| SE-01 | gridPosition / tabPosition（定位=显隐） | bridge.gridPosition/tabPosition | native_execution | R5(awareness) | GOVERNED |
| SE-02 | closeGrid（destroy webviews） | bridge.closeGrid | browser_grid_lifecycle | R5 | GOVERNED |
| SE-03 | hideWebview / hideAllWebviews（offscreen，资源存活） | bridge.hide* | native_execution | R5 | GOVERNED |
| SE-04 | rebuildGrid（真重建） | rebuildGrid | browser_grid_lifecycle | R5 | GOVERNED |
| SE-05 | writeFile（文件系统变更） | bridge.writeFile | workspace_filepanel | R5(文档级) | GOVERNED |
| SE-06 | bookmarkPersist（后端持久化） | bridge.bookmarkAdd | bookmark | R5(文档级) | GOVERNED |
| SE-07 | termProcess（spawn/kill/write PTY） | bridge.spawnTerm | terminal | R5(文档级) + check-terminal-policy.py(29) | GOVERNED |
| SE-08 | keyringWrite / keyringDelete（写/删密钥库） | KeyringStore.save/delete_token | credential | R5 + S1/S2(keyring 契约) | GOVERNED |
| SE-09 | keyringRead（读密钥库） | KeyringStore.get_token | credential | S1/S2 | GOVERNED（Phase 5.1-C） |
| SE-10 | sensitiveInput（敏感输入护栏） | db.connect(password) | credential | R7 | GOVERNED（Phase 5.1-B） |

## 2. 范围内副作用登记评估

- 范围内 10 类副作用全部登记，且具有 owner 与安全边界（keyring 命名空间隔离、termProcess 资源释放、
  writeFile 文档级登记避免 R5 噪声）。
- `keyringRead`/`keyringWrite`/`keyringDelete`/`sensitiveInput` 已有机器约束（S1/S2 + R7），非仅文档。

## 3. 范围外 — 未登记的重要副作用（MISSING，需 SCR）

| ID | Side Effect | call_sites（代码） | 真实行为 | Owner（应有） | Decision |
|---|---|---|---|---|---|
| SE-11 | 脚本/片段执行（spawn OS 进程） | `run_script`/`run_command`（script_runner.rs:768 `Command::new`） | 创建/销毁 OS 进程、执行用户脚本 | 无 | **MISSING → SCR** |
| SE-12 | 插件安装/启用 | `plugin_install`/`plugin_enable`（plugin.rs） | fs 写 + 进程 + 动态加载 | 无 | **MISSING → SCR**（安全敏感，最高优先级） |
| SE-13 | Git push/pull（网络 egress） | `git_*`（git2，sync.rs） | 网络推送/拉取远端仓库 | 无 | **MISSING → SCR** |
| SE-14 | DB 连接/查询（网络 DB） | `db_connect`/`db_query`/`db_disconnect`（database.rs，mysql/postgres） | 网络连接 + 凭据注入 | 无 | **MISSING → SCR**（注意：凭据侧已登记，但网络查询侧未） |
| SE-15 | Agent/Skill 安装与运行 | `agent_*`/`skill_*`/`agent_chat`（process + fs） | spawn 进程、写 agent_kv | 无 | **MISSING → SCR** |
| SE-16 | 会话导出/恢复 | `session_export`/`session_restore`（session.rs:32,260 fs） | 写磁盘归档 | 无 | **MISSING → SCR** |
| SE-17 | Vault 打开/知识库索引 | `vault_open`（fs） | 读本地 vault 目录 | 无 | **MISSING → SCR** |
| SE-18 | 资源捕获设置 | `set_resource_capture_settings`/`clear_tab_resources`（截图/fs） | 写磁盘/截图 | 无 | **MISSING → SCR** |
| SE-19 | 系统剪贴板读写 | `clipboard_read`/`clipboard_write`（arboard，bridge.rs） | 读/写 OS 剪贴板 | 无 | **MISSING → SCR**（凭据红线相关，须与 clipHistory 区分） |
| SE-20 | MCP server 启动 | `mcp_registry_list`/`mcp_server`（stdio JSON-RPC 进程） | spawn 子进程 | 无 | **MISSING → SCR** |

## 4. 范围内/外副作用闭合结论

- **范围内：CLOSED**（10 类登记 + keyring 机器约束）。
- **范围外：OPEN**（10 类未登记副作用，无 owner/checker）。其中 SE-12（插件安装）与 SE-19（剪贴板）
  属安全敏感，优先级最高。

## 5. 小模型风险

| DUP-ID | Side Effect | WHY_SMALL_MODEL_MAY_MISUNDERSTAND | POSSIBLE_WRONG_CHANGE | PREVENTION |
|---|---|---|---|---|
| DUP-301 | position() | 误以为"只是移动" | 用 tabPosition 做实显隐切换语义 | side-effects.yaml 登记 position=show 契约 |
| DUP-302 | hide vs destroy | hide* ≠ closeGrid | 用 hide 替代 closeGrid 造成资源泄漏 | Registry 显式区分 |
| DUP-303 | run_script | 以为是普通函数调用 | 未意识到 spawn OS 进程的安全边界 | 须登记 SE-11 + 安全策略 |
| DUP-304 | clipboard_write | 与 clipHistory 混淆 | 把凭据原文写入 clipHistory（已 Bug-HUNT 修复，须保持） | 不持久化 + redactSecrets |
