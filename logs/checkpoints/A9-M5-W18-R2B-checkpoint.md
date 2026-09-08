# Lane A9 — M5-W18-R2B Checkpoint

```yaml
LANE: A9
DISPATCH: A0-M5-W18-R2B-dispatch-20260908.md   # 第二批证据闭环
RESEARCH_DIR: logs/research/M5-W18/
BASE_SHA: origin/master = 434e63f
HEAD_SHA: 28a934a   # rebase 于 434e63f 之上（本批新增 4 文件，未 push）
CONSUMED_PEERS:
  A6: 38faa2da7d521840966c83575cf152a573a68f91   # 资源生命周期/剪贴板明文落盘修复（B11-1）
  A7: 07fda2f87c319f6731d065f69da1e00dae944a00   # zvec-grep 摄取/索引架构（TS→Rust 重实现判定）
  A8: f4a4f3bab71df7322a96c499facf201e35ffd441   # 基准/体积指标更正（R2B 审计 #6 未定项）
PROJECT_RULES: PROJECT-RULES.md (规则 1/2/3/3.5/3.6 已锁)
MODE: W18-R research-only (零产品代码, 不 push)
STATUS: PASS_WITH_DEBT   # 产品侧边界静态已证；S4 索引契约待冻结；native 验收 NOT_RUN
```

## 1. 本批交付物（全部 `logs/research/M5-W18/` 或 `logs/checkpoints/`）

| 文件 | 内容 | 类型 |
|---|---|---|
| `A9-R2B-trust-boundary.md` | 产品侧信任边界审查（根目录授权/路径穿越/symlink/隐藏·忽略/脱敏/索引范围/远端外发/远程页 vs 本地进程区分/debug-release 边界）+ R1 两处 REWORK 更正 | 研究 |
| `A9-R2B-webview-boundary.md` | 原生 WebView 边界（坐标/DPI/遮挡/焦点/隐藏/宫格）+ 5 个 A11 可执行场景 | 研究 |
| `A9-R2B-capability-budgets.md` | S0/S1/S4 允许能力·禁止旁路·必须反例 | 研究 |
| `A9-M5-W18-R2B-checkpoint.md` | 本 checkpoint | checkpoint |

> 既保留 R1 成果（`A9-threat-model.md` / `A9-source-map.md` / `A9-checkpoint.md`）未删除；R2B 以差异与证据补正（R2B §8 永久防回退项 8：不静默删历史）。

## 2. R1 两处 REWORK 闭环（A0 R1 审计）

- **REWORK #1（COPY 标签冲突）**：R1 `COPY(5)` 更正为 `REIMPLEMENT_FROM_BEHAVIOR(5)`（其中 2 项细分为 ADAPT：启动黑名单移植、API-key→keychain）。依据：zvec-grep 是 TS 工程、产品是 Rust/Tauri，TS→Rust 非字面复制（A7 W18-R 架构判定一致）；R2B §8「COPY 仅给真实代码单元」。
- **REWORK #2（本地攻击者模型不全）**：补全同用户恶意进程（0600 文件可读→密钥须 keychain，复用 `configure_repo`→`KeyringStore` 先例）、多用户主机、symlink 逃逸（`resolve_program_file` 范式须复用于索引）、隐藏/忽略文件（S4 设计义务）、任意路径读已排除（`report_resources` 无路径参数）。映射到产品真实 IPC = webview 桥（已被 ACL+意图令牌门控）。

## 3. 主要结论（源码验证）

- **远程页威胁已收窄**：`browser-remote.json` 通配 `https://*`/`http://*` + `remote-collect` 仅 3 个无副作用回传命令；`save_note`/`collect_selection`/`request_open_terminal` 不在远程集且须 `main` 签发的一次性令牌（`bridge.rs:1985,2015`）。
- **本地进程威胁已隔离**：`BLOCKED_LAUNCH_PROGRAMS(12)`/`WRAPPERS(10)`/`INTERPRETERS(12)` + symlink 复核 + 元字符闸门（`security_policy.rs:158-286,431`）。
- **凭据脱敏契约完整**：`redact_sensitive_url` 21 敏感键（`security_policy.rs:481,547`）；token 走 keychain；剪贴板明文落盘已修（A6）。
- **release 无 Vite 依赖**：`tauri.conf.json` 无 `devUrl`；debug capability 在 `dev-capabilities/`（不进 `capabilities/`）。

## 4. 未解决问题 / 债务（交 A0/A11）

- **D-A9-1（High）**：S4 搜索索引的 symlink 跟随复核、隐藏/忽略文件、查询内容脱敏、索引范围——须 A7/A8/A9 同契约冻结（蓝图 S4 阻塞 = 三 lane 相同契约）；当前产品无搜索后端（A1 G3），属待建非现网缺陷。
- **D-A9-2（Low）**：系统已装 M1-0 旧二进制（`/usr/bin/mvp-browser-os`，commit `04ad3f5`）ACL 不含当前收紧；集成清单须标注「版本看构建时间+commit，不能仅看 deb 0.1.0」。
- **D-A9-3（Info）**：A7 绑定/sidecar 结论未定（R2B 审计 #5）、A8 指标更正未定（#6）；A9 安全契约独立于二者，不影响本批结论。
- **native 验收**：WebView 边界 5 场景本批 `NOT_RUN`（A9 无 GUI 通道），交 A11 在 debug/release 双通道跑。

## 5. 自检

- 工作树仅新增 4 个研究/checkpoint 文件；未改 `src-tauri/`、未改其他 lane 文件、未改 `PARALLEL_COMMAND_BOARD.md`、未改调度规则。
- 未 push（仅 A0 可 push）。
- 所有主张均附 `file:line` 源码证据，未运行产品二进制。
