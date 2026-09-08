# A9 R2B — 产品侧信任边界审查（含 R1 更正与本地攻击者模型补全）

- **Lane**: A9
- **Dispatch**: `A0-M5-W18-R2B-dispatch-20260908.md`（第二批证据闭环）
- **基线 SHA**: `origin/master` = `434e63f`；本 lane HEAD = `28a934a`（rebase 于 `434e63f` 之上）
- **范围**: R2B §10 给 A9 的产品侧信任边界审查 + A0 R1 审计两处 REWORK 缺陷的纠正
- **方法**: 静态源码审计（W18-R 研究边界），未运行 GUI / 未改产品代码
- **关联报告**: `A9-threat-model.md`（R1 zvec-grep 威胁模型）、`A9-source-map.md`（R1 来源映射）、`A9-R2B-capability-budgets.md`、`A9-R2B-webview-boundary.md`

---

## 0. 本批相对 R1 的两处更正（A0 R1 审计 REWORK 闭环）

### 0.1 更正 R1 的 `COPY(5)` 标签冲突（A0 R1 REWORK #1 + R2B §8）

A0 R1 指出 R1 把 zvec-grep 五个 TS 模块标成 `COPY`，但 zvec-grep 是 **TypeScript/Node 工程**，产品是 **Rust/Tauri**；TS→Rust 不是字面复制，必须重实现。

**更正结论**（与 A7 W18-R 架构判断一致 — `A7-zvec-grep-ingestion-index-architecture.md`）：

| R1 原标签 | 模块 | 更正后标签 | 理由 |
|---|---|---|---|
| COPY | 远程嵌入授权门（HMAC 签名 per-workspace grant + consent + anti-replay） | **REIMPLEMENT_FROM_BEHAVIOR**（设计意图 COPY，Rust 落地须重实现） | TS→Rust，且无现成 Rust 绑定（待 A10 裁决 `@zvec/zvec` 是否暴露 Rust API） |
| COPY | 本地 daemon 回环鉴权模型 | **ADAPT**（结构移植 + 强制 token） | 若采用 Node sidecar 则为 ADAPT，若 Rust 重写为 REIMPLEMENT |
| COPY | 日志/错误脱敏器（`sanitizeFields`） | **REIMPLEMENT_FROM_BEHAVIOR** | Rust 等价物；复用产品既有 `redact_sensitive_url` / `SENSITIVE_QUERY_KEYS` 脱敏契约 |
| COPY | MCP 工具集拆分 + 结果压缩 | **REIMPLEMENT_FROM_BEHAVIOR** | Rust 命令设计，沿用产品 ACL 三源一致性规则 |
| COPY | trace-header 纪律 + API-key 存储 | **REIMPLEMENT_FROM_BEHAVIOR**（trace）/ **ADAPT**（API-key → OS keychain） | keychain 复用产品 `KeyringStore` 先例（`configure_repo`） |

> R2B §8 规则「保留 COPY 仅给真实代码单元」在此落实为：**上游 TS 模块本身作为 A10 provenance 记录单位可标 COPY，但产品落地物一律 REIMPLEMENT/ADAPT**。R1 中 `COPY(5)` 一律降级为 `REIMPLEMENT_FROM_BEHAVIOR(5)`（其中 2 项细分为 ADAPT），不再有任何字面 `COPY` 落地产物的主张。

### 0.2 补全本地 socket / 文件系统攻击者假设（A0 R1 REWORK #2）

R1 仅覆盖「未鉴权 daemon 回环」（B2 局部）。补全如下，并映射到产品**真实** IPC 面：

1. **同用户恶意本地进程**：产品无对外监听 daemon；其本地 IPC 是 Tauri `invoke` 经 webview 桥，已被 ACL + `check_remote_invocation` 门控。真正风险是「运行于同一 OS 用户的恶意进程读 0600 文件」。
   - 凭据处理先例良好：`configure_repo` 的 token 写入系统密钥库 `KeyringStore`，**绝不回传前端**（`src-tauri/src/bridge.rs:2072-2091`）；DB 密码瞬时经 JS 参数但不落盘（A0 R1 修正 #4 + A6 BUG-HUNT 剪贴板明文落盘已修）。
   - 残留面：若未来 zvec-grep 采纳远程嵌入，**apiKey 必须也走 OS keychain**，不可用 0600 文件（同用户进程可读）。
2. **多用户主机**：index / grant / 签名密钥应 0600；但同用户进程即等效「本地攻击者」。缓解=密钥不落文件（keychain / 内存），与 #1 一致。
3. **符号链接逃逸**：产品已有 `resolve_program_file` 跟随符号链接判黑（`src-tauri/src/security_policy.rs:270-286`），证明「跟随 symlink 复核」是既有正确实践；**S4 搜索索引必须复用同一范式**（`check_path_within_roots` 须 canonicalize 后再比对 allowed roots），否则 symlink 可带出工作区。
4. **隐藏/忽略文件被索引**：属 S4 设计义务（见 §2.4），当前产品无搜索后端（A1 G3），故为「待建契约」而非现网缺陷。
5. **任意本地路径读**：已排除——`report_resources` 仅接受页面自报的 `items`（无路径参数），`check_invocation_source` 仅做 label 校验 + 载荷边界（`src-tauri/src/bridge.rs:1201-1218`），**无任意文件读入口**。

---

## 1. 根目录授权（R2B §10.1，对应蓝图 §3-5）

- **契约**：`check_path_within_roots(path, roots)`（`security_policy.rs:341`）是「路径必须落在 allowed roots 内」的唯一真源；`allowed_roots(app)` 由 `bridge.rs:1129` 提供（工作区稳定 ID 映射的根目录集）。
- **现状**：`save_note` 写盘前 `check_path_component` 防 `..`/斜杠/空字节（`security_policy.rs:390`；`bridge.rs:2043-2044`），`check_delete_target` 禁止删除根目录本身（`security_policy.rs:406`）。
- **风险点**：蓝图 §3-5 明确要求「不要直接把输入路径当成授权」。当前 `allowed_roots` 来源依赖工作区配置；**S4 搜索 / 文件浏览命令须以 `check_path_within_roots` 为强制闸门，不允许用调用方传入的任意 path 作为授权依据**。
- **结论**：根目录授权机制已具备，但「输入路径≠授权」的语义须在每个新建文件类命令的契约里显式重申（交 A7/A8/A9 同契约冻结）。

## 2. 路径穿越 / 符号链接 / 隐藏·忽略文件 / 索引范围（R2B §10.2-10.4, 10.6）

| 项 | 产品现状（源码证据） | 结论 |
|---|---|---|
| 路径穿越 | `check_path_component` 拒 `.`/`..`/分隔符/空（`security_policy.rs:390`）；`check_path_within_roots` 限 allowed roots（341） | 主窗文件写/删已覆盖；远程 webview 无文件写入口 |
| 符号链接 | `resolve_program_file` 跟随 symlink 判黑（270-286）；`check_path_within_roots` 须 canonicalize 后才能防逃逸 | **S4 索引须复用此范式**；当前无搜索后端，属待建契约 |
| 隐藏/忽略文件 | 无搜索后端，未实现 | **S4 设计义务**：索引须尊重 `.gitignore`/忽略规则（对齐 A7 `DEFAULT_IGNORE_RULES`、`MAX_GITIGNORE_CACHE_ENTRIES=4096`），且**默认不索引 `.env` 等含密文件**，或索引前经脱敏 |
| 索引范围 | 全仓零检索后端（A1 G3） | 索引契约待 A7（引擎）+ A8（基准）+ A9（安全）三方冻结；范围=allowed roots 并集，禁止越界 |

- **R2B 反例（必须跑，交 A11）**：① 工作区内 symlink 指向 `/etc` → 索引不得逃逸；② `.env` 含 `api_key=...` → 不得原样入库或须脱敏；③ `../../etc/passwd` 作为 rename 目标 → `check_path_component` 拒绝。

## 3. 查询内容 / 凭据脱敏（R2B §10.5）

- **URL 凭据脱敏**：`redact_sensitive_url`（`security_policy.rs:547`）移除 userinfo、对 21 个敏感 query 键（token/password/secret/api_key/jwt…，`SENSITIVE_QUERY_KEYS:481`）值置 `***`、fragment 同处理、限长 2048 字节。资源瀑布 DTO 明确不含 headers/Cookie/Authorization/body（`bridge.rs:1227-1228`）。
- **凭据落盘**：`configure_repo` token → `KeyringStore`（2072-2091）；剪贴板明文落盘已由 A6 BUG-HUNT 修复（内存有界、不写磁盘、渲染走 `redactSecrets`）。
- **日志脱敏**：`log_audit` 上限 1000 FIFO（`bridge.rs` 多处审计只记 tab_id/计数/动作，不记明文）。
- **结论**：查询/凭据脱敏契约已较完整；**S4 搜索查询内容若含 `api_key=...` 必须复用 `redact_sensitive_url` / `SENSITIVE_QUERY_KEYS` 再入库或展示**（待建，交 A7 抽取层 + A9 安全契约）。

## 4. 远端外发（R2B §10.7）

- **产品当前无对外远端数据外发通道**：唯一「远程」面是从外部网页经 `report_*` 回传**事件**到本地 app（非外发到第三方）。
- **zvec-grep 远程 embedding**：本批维持 R1 结论 = **DEFER / 默认关闭**。仅在 per-workspace HMAC 授权 + 用户 consent + anti-replay 齐备时方可开启（REIMPLEMENT_FROM_BEHAVIOR，非 COPY）。
- **模型下载**：**REJECT / 默认关闭**（R1 结论，无本地模型下载需求）；若未来引入，须与 embedding 外发**分别**授权并默认关闭（R2B §10.9）。

## 5. 区分「本机恶意进程 / 多用户」与「远程页面」威胁（R2B §10.8）

这是本批核心边界澄清：

- **远程页面威胁（首要）**：`browser-remote.json` 允许 `tab-*`/`grid-*` webview 加载 `https://*`/`http://*`，权限 `remote-collect`（`capabilities/browser-remote.json:5-8`）。`remote-collect.toml` **仅放行 3 个无副作用回传命令** `report_resources`/`report_title`/`report_grid_load_failed`（`:8`）。
- **关键门控**：三个 report 命令均调 `check_invocation_source` → `check_remote_invocation`（`bridge.rs:1188-1198`, `security_policy.rs:618-632`）：
  - label 必须已登记（`main`/`tab-*`/`grid-*`），伪造/残留 `browser` label 一律拒（`check_webview_label:329`）；
  - `report_*` 无副作用，**只需来源 label 校验 + 载荷边界**（`check_text_field` + `check_items_count(MAX_RESOURCE_ITEMS=500)`，`bridge.rs:1210-1212`）。
- **有副作用命令（save_note/collect_selection/request_open_terminal）在 `default-commands.toml` 仅主窗可用，不在 `remote-collect`**；即便被调用，`save_note` 须 `INTENT_SAVE_NOTE` 一次性令牌（`bridge.rs:2015`），而 `issue_intent` **只允许 `main` 签发**（`bridge.rs:1985-1987`），远程页无法取得令牌 → 无法写盘/开终端/收选区。
- **本机恶意进程 / 多用户威胁**：产品本地 IPC 即上述 webview 桥，已被 ACL + 意图令牌收口；启动外部程序受 `BLOCKED_LAUNCH_PROGRAMS(12)`/`WRAPPERS(10)`/`INTERPRETERS(12)` 黑名单 + symlink 复核 + 元字符闸门（`security_policy.rs:158-286, 431`）。**远程页与本地进程两路威胁已分别用「ACL+意图令牌」与「启动黑名单+路径门」隔离**，不得用「扩大 `browser-remote.remote.urls`」去修 debug IPC（R2B §8 明示）。

## 6. debug-only capability / release bundle / 旧二进制 / Vite 所有权 / 退出清理（R2B §10 末尾）

- **debug-only capability**：`dev-capabilities/main.json` 仅放行 `http://localhost:1421/*`（`local:false`，由 `#[cfg(debug_assertions)]` 编译门控），与主窗同权限集；**未进 `capabilities/` 目录**，release 二进制不含该 capability（`main.rs` 仅 debug 注册）。
- **release bundle 无 Vite 依赖**：`tauri.conf.json` `build` 仅有 `beforeDevCommand`，**无 `devUrl` 键**（核查 `grep devUrl` 无命中）→ release 用 bundled `tauri://localhost`，不回退 Vite dev server。满足 WORKSPACE_IDENTITY 桌面运行时来源门。
- **旧二进制误启动**：系统已装 M1-0（commit `04ad3f5`，09-01 构建）旧版，`/usr/bin/mvp-browser-os`（记忆 ID 31177397）。旧二进制 ACL 不含当前 `remote-collect` 三命令收紧与意图令牌——属部署侧风险，**A9 建议在集成清单标注「版本须看构建时间+commit，不能仅看 deb 0.1.0」**（沿用记忆 ID 32679167 基线治理）。
- **Vite 所有权**：release 不依赖 Vite（见上）；`beforeDevCommand` 仅 debug 期生效，退出清理由 Tauri 生命周期 + `stop-background-workers`/`stop-scheduler` 负责（A6 调度契约），不在 A9 范围但确认 capability 面零扩张。
- **退出清理**：`term_kill`/`db_disconnect`/`session_discard` 等命令已存在，退出协调属既有契约；A9 仅确认「release 不含 debug capability」使退出路径不依赖 dev 通道。

---

## 7. 综合判定

- **产品侧信任边界（远程页 / 本地进程 / 凭据 / 路径）已基本具备且优于 R1 仅聚焦 zvec-grep 的视角**：remote-collect 收窄到无副作用回传 + 意图令牌隔离副作用命令 + 启动黑名单 + URL 凭据脱敏，四道防线源码可见。
- **待建契约（非现网缺陷）**：S4 搜索的 symlink 跟随复核、隐藏/忽略文件、查询内容脱敏、索引范围——须在 A7/A8/A9 同契约里冻结（蓝图 S4 阻塞 = A7/A8/A9 相同契约）。
- **R1 两处 REWORK 已全部闭环**：COPY 标签更正为 REIMPLEMENT/ADAPT；本地攻击者模型补全并映射到真实 IPC 面。
- **跨 lane 消费**：A7 绑定/sidecar 结论未定（R2B 审计 #5），A9 安全契约独立于该争论——无论引擎 COPY 还是 Node sidecar，产品侧「HMAC per-workspace grant + 默认关闭 + 不落明文 apiKey」均成立。
