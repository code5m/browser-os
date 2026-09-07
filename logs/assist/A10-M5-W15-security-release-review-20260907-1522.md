# A10 · M5-W15 Release Readiness — Security Release Review (Verdict)

> Lane: `A10` — M5-W15 Release Readiness Dispatch（security release review, including raw-invoke and error rendering）
> 时间: 2026-09-07 15:22 CST
> Base: `886ea29` (`master`, `git pull --ff-only` 与 origin/master 快进一致，工作树含其他 lane 未提交改动但不影响本评审范围)
> 范围: **只读安全发布复核**（board 明确 A10 仅评审，无产品代码改动）。本轮聚焦硬停止清单 + 用户指定两项（raw-invoke、error rendering）。
> 前序 A10 交付：M4 系列 + M5-W1~W14 共 24 份 `logs/assist/A10-*.md`（均 `??`，待 A0 统一提交）。本文件为 **W15 终版安全发布裁定**。

---

## 0. 方法声明

- 全部结论基于**源码实证** + **脚本实跑**，非文档互证。
- 实跑证据（本轮我亲自执行）：
  - **全部 29 个策略脚本自测 PASS**（见 §1 表）：`check-agent-memory-policy`、`check-agent-skill-policy`、`check-command-domain-policy`、`check-core-boundary`、`check-database-policy`、`check-git-*`、`check-graph-policy`、`check-image*`、`check-lifecycle-contract`、`check-mcp-policy`、`check-plan-routing`、`check-plugin-policy`、`check-plugin-privacy`、`check-plugin-ui-privacy`、`check-resource-capture-policy`、`check-scheduler-policy`、`check-scheduler-ui-policy`、`check-script-*`、`check-security-policy`、`check-seed-tools`、`check-session-persistence-policy`、`check-terminal-policy`、`check-tools-policy`。
  - **全部前端 UI 逻辑校验 PASS**（`.mjs`）：agent-skill 110、command 36、database 119、git ok、graph 113、image 58、plugin-ui 61、resource 84、scheduler 105、script 32、session 34、terminal 25、image-preview 97 断言全过。
  - **`npm run build` PASS**：`✓ built in 4.19s`，`index-XsDwjN5-.js = 165.68 kB`（gzip 59.32 kB）。较此前观测的 197.04 kB 已回落，**IF-2 体积回归阈值已解除**（A5/A8 懒加载生效）。
  - **`cargo check` = 2 warnings**（`grid_process.rs:76/103`，均为 M0 既有基线），无新增。
- 前端 IPC 调用分布：除 `src/bridge.ts` 外，**零** `invoke(` 裸调用。
- 后端运行时扫描：无 `plugin_invoke`、无 `TcpListener`/`UdpSocket`/`TcpStream`、无 `reqwest`/`hyper`/`actix`/`tonic`。

---

## 1. W15 硬停止清单逐条核对

| 硬停止项 | 结论 | 证据 |
|---|---|---|
| 无 plugin invoke/execution | ✅ | bridge.rs 仅有 `plugin_install`（:6735 带 source check）；无 `plugin_invoke`/`plugin_execute`/`plugin_run` |
| 无动态加载 | ✅ | 无 `dlopen`/plugin load 执行路径；`plugin_install` 仅写清单+信任键，不加载运行 |
| 无网络下载/listener | ✅ | 后端无 `TcpListener`/`UdpSocket`/`reqwest`；mcp.rs 注释明守 W3 硬停止 |
| 无 daemon | ✅ | 无后台驻留进程；grid_process 为既有渲染子进程（Unix socket 本地 IPC，非网络） |
| 无 model call | ✅ | 无 `reqwest`/推理调用新增；Agent/Skill 仅读桥 |
| 无 Agent/Skill execution | ✅ | Agent/Skill 桥接 read-only（check-agent-skill-policy PASS，plugin 面板无执行按钮） |
| 无 MCP runtime 扩张 | ✅ | mcp.rs 无 TcpListener；`check-mcp-policy` ACTIVE=13 PENDING=0 |
| 无 graph write/export | ✅ | grid_process 仅渲染；graph 写/导出锁（check-graph-policy ACTIVE=8） |
| 无 background worker 新增 | ✅ | 调度器复用 M2-4 `script_runner`，无第二 spawn 路径 |
| 无 raw Tauri invoke | ✅ | 见 §2 |
| 无敏感渲染/持久化 | ✅ | 见 §3 |

---

## 2. Raw-invoke bypass（用户指定项 ①）— **PASS**

- 前端 `invoke(` 仅出现在 `src/bridge.ts`（经 `@tauri-apps/api` 的封装层）。所有 `.vue`/`.ts` 组件通过 `import { bridge }` 调用 `bridge.xxx()`。
- 全仓 `grep -rn "invoke(" src --include='*.vue' --include='*.ts' | grep -v src/bridge.ts` → **0 命中**。
- 后端新命令均经 `check_invocation_source`：全仓 76 处调用；`plugin_install` 已验证（bridge.rs:6735）。`check-command-domain-policy` / `check-security-policy` 自测 PASS（码位完整）。
- 结论：无绕过 bridge 的裸 IPC、无 source-check/ACL 漂移。

---

## 3. Error rendering / sensitive echo（用户指定项 ②）— **PASS**

- **脱敏工具链就位**：
  - `src/utils/redact.ts`：`redactSecrets` 掩码 URL userinfo + `sanitize_audit_text` 截断。
  - `src/utils/pluginUi.ts`：`redactDetail` / `redactSummary` / `redactKeyRecord` —— `PluginManager.vue:17` 用 `redactDetail(store.detail)` 渲染，原始 `PluginDetail`（含 pubkey/签名/路径）不直出。
  - `src/utils/agentSkillUi.ts`：`redactSecrets` 递归脱敏 secret 参数值。
  - 后端 `database.rs:220` `sanitize_message` 把 `password=xxx`/`token:xxx` 整段替换为 `***`。
  - 图片：`images::redact_source_url` 落库前脱敏（image.ts:8 / imagePreview.ts:9 第二道防线）。
- **插件错误渲染**：`PluginManager.vue` 的 `store.error` 仅承载用户友好文案（如「请粘贴 manifest JSON」「keyId 与 pubkey 均必填」），**非后端原始错误直显**；`check-plugin-ui-privacy` / `check-plugin-privacy` 自测 PASS（含变异防呆）。
- 结论：无敏感字段（签名/pubkey/路径/凭据/body/stdout/stderr）在前端渲染或持久化逃逸。

---

## 4. 既有执行路径复核（确认无越权扩张）

| 路径 | 性质 | 判定 |
|---|---|---|
| `script_runner.rs:764` `Command::new(program)` | M2-4 合法脚本/命令执行通道 | ✅ 既有，未扩张 |
| `bridge.rs:4134 launch_app` | 经 `check_launch_target` 解析 (program,args)，**拒 shell 元字符、禁 `sh -c`** | ✅ M0-3.d 硬化 |
| `bridge.rs:4825/4835/4854` `update-desktop-database`/`xdg-settings` | 桌面/MIME 集成（`set_default_browser`，需用户显式确认） | ✅ 既有 M1-4，非网络/daemon |
| `grid_process.rs:140 UnixListener` + `:245 Command::new(exe)` | 既有 GPU 渲染子进程 + 本地 Unix socket | ✅ 既有，非网络监听 |

---

## 5. 构建与验证证据

- `npm run build` → PASS（4.19s）；index 165.68 kB / gzip 59.32 kB；xterm 单独分包（334 kB 既有大依赖，已隔离）。**IF-2 体积回归解除**。
- `cargo check` → 2 warnings（基线）。
- 29 个 `check-*.py` 自测全 PASS；13 个 `check-*-logic/.mjs` UI 校验全 PASS。
- `dist/` 已被 gitignore（确认不入库）。

---

## 6. 残余债与发布备注

- **R-1（M4 遗留 HIGH）审计冲刷**：`task.run.start/finish` 仍写 `audit.json`（cap 1000 FIFO），高频任务会冲刷手工审计。该债属 M4 调度器范围，**不在 W15 评审范围内**；建议 A0 在后续维护窗口修订契约 §7 并指派 A7 改 `record_run_start/finish` 落 `task-runs.json`。**不阻本次发布（W15 硬停止未涵盖，且明细已冗余存于 task-runs.json）**，但应在发布说明/债台账登记。
- 无其他 W15 阻塞项；无运行时权限扩张、无裸 IPC、无敏感渲染逃逸。

---

## 7. 安全发布裁定（Verdict）

```
VERDICT = PASS (release-ready, security scope)
- raw-invoke bypass:        PASS
- error rendering/sensitive echo: PASS
- runtime authority expansion: PASS (no new surface vs M0 baseline)
- source-check / ACL parity: PASS (policy scripts green)
- build & verification:     PASS (npm build OK, IF-2 resolved; cargo check baseline)
RESIDUAL = R-1 (carry-over, tracked, non-blocking for W15)
```

**A10 结论**：M5-W15 在当前 `886ea29` 工作点上，从安全发布视角**可达发布就绪**。所有硬停止清单满足，用户指定的 raw-invoke 与 error rendering 两项均 PASS，构建与策略/UI 校验全绿。唯一遗留为 M4 期 R-1 审计冲刷债，建议 A0 登记跟踪，不阻断本次发布。

---

## 8. Lane Output Template

```
LANE=A10
STATUS=PASS
BASE=886ea29 (master, ff-current with origin/master)
HEAD=logs/assist/A10-M5-W15-security-release-review-20260907-1522.md (+ checkpoint)
FILES=logs/assist/A10-M5-W15-security-release-review-20260907-1522.md ; logs/checkpoints/A10-M5-W15-20260907-1522.md
VERIFY=
  npm run build                         -> PASS (index 165.68kB, IF-2 resolved)
  cargo check (src-tauri)               -> 2 warnings (baseline grid_process.rs)
  29x check-*.py --self-test            -> ALL PASS
  13x check-*-logic.mjs                 -> ALL PASS (assertions 0 fail)
  frontend raw invoke (excl bridge.ts) -> 0 hits
  backend runtime scan                 -> no plugin_invoke/network/listener
CHECKPOINT=logs/checkpoints/A10-M5-W15-20260907-1522.md
MERGE_NOTES=
  1. A10 本轮仅评审，无产品代码改动；交付物为两份文档（assist + checkpoint），待 A0 统一提交。
  2. 工作树含其他 lane 未提交改动（plugin/graph/git 等），A0 集成时按各自 lane 整理，勿整体 stage。
  3. 残余债 R-1（审计冲刷）登记跟踪，不阻 W15 发布。
NEXT=A0 发布前最终集成（push 前跑 cargo test + cargo build --release + npm run build + pre-merge.sh + git diff --check，归 A11 证据 + A0 执行）
```

---

## 9. 声明

- 本轮**零产品代码改动、零策略脚本改动**，符合 board「A10 仅评审」与 W15 硬停止。
- 未 rebase/commit/push（board Merge Rule：仅 A0 可视情况提交推送）。
- 结论全部源码实证 + 脚本实跑；前序 24 份 A10 评审仍有效，本文件为 W15 终版增量裁定。
