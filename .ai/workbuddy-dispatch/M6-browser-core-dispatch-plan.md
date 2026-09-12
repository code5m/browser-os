# M6 — Browser Core Abstraction · Dispatch Plan

> Version: 2026-09-12
> 配套：`M6-browser-core-specification.md`、`M6-A0-governance-decision-record.md`
> Status: PLANNING — 不启动大规模源码迁移

---

## 1. 批次派发表

通用约束（所有批次）：

- 每批一个 commit，`M6-X` 前缀
- 每批结束必须跑 §3 回归门禁
- 违反 specification §6 的 I-1~I-12 不变量 → 立即停止并回报 Owner
- 禁止跨批次大改动

### M6-0 可行性 Spike（GO/NO-GO）

| 项 | 内容 |
|---|---|
| 目标 | 证明 BrowserScene → syncScene 能真正降低 useBrowserHost.ts 复杂度 |
| 允许改 | 仅 `logs/` 与 `docs/` 下的研究报告，**零产品代码** |
| 禁止改 | `src/**`、`src-tauri/**`、`scripts/**`、ACL、capability |
| 产出 | 一份 Spike 报告：给出 Scene 草案、syncScene 伪实现、以及 §2 三项指标的量化预估 |
| 出口 | GO → 进入 M6-A；NO-GO → 停止整条线，回报 Owner |
| 判据 | specification §7.1 |

### M6-A BrowserRuntime interface + Real adapter + MockRuntime

| 项 | 内容 |
|---|---|
| 目标 | 建立 interface；Real adapter 透传现有 bridge（行为零变化）；Mock 可 Node 运行 |
| 允许改 | `src/runtime/**`（新建，仅 interface + 两个实现）、`src/bridge.ts`（仅接线，不改签名） |
| 禁止改 | `src-tauri/**`、ACL、capability、任何组件 |
| 出口 | `check-browser-runtime.mjs` 状态由 NOT_IMPLEMENTED 转为 PARTIAL；行为零变化 |

### M6-B WebViewSafeShell + BrowserViewportAnchor

| 项 | 内容 |
|---|---|
| 目标 | 统一 WebView 遮挡边界与视口锚点 |
| 允许改 | `src/runtime/**`、`src/composables/**` |
| 禁止改 | `tauri-browser-tabs/**`（原生危险区）、`src-tauri/**` |
| 出口 | check-native-webview-overlay.mjs 仍 PASS；无新增 HTML 浮层覆盖 WebView |

### M6-C BrowserScene contract

| 项 | 内容 |
|---|---|
| 目标 | 冻结 Scene 数据结构 |
| 硬约束 | 不得含 webviewHandles / perTabWebviewId 等实现细节字段（spec §4.3） |
| 允许改 | `src/runtime/**`、`src/types.ts`（仅新增 Scene 类型） |
| 出口 | Scene 结构冻结，写入 specification §4.3 |

### M6-D syncScene 统一原生同步

| 项 | 内容 |
|---|---|
| 目标 | 收敛 tabPosition / gridPosition / gridSetZoom / hideWebview / hideAllWebviews |
| 允许改 | `src/runtime/**`、`src/composables/useBrowserHost.ts` |
| 禁止改 | `src-tauri/src/bridge.rs`（不改 Rust 命令契约） |
| 出口 | 幂等验证：同 scene 重复下发不产生额外 native 调用 |

### M6-E 渐进迁移旧 browser commands / callers

| 项 | 内容 |
|---|---|
| 目标 | adapter / strangler 方式逐步收口调用方 |
| 允许改 | `src/stores/**`、`src/components/**`（逐个迁移） |
| 出口 | 每个迁移点均有前后行为对比记录 |

### M6-F 清理确认无 caller 的旧调用

| 项 | 内容 |
|---|---|
| 目标 | 移除死代码 |
| 前置 | 必须 grep 证明零 caller，且不属于 ACL 注册项 |
| 出口 | 移除后全量门禁仍绿 |

### M6-G deterministic tests + MockRuntime tests

| 项 | 内容 |
|---|---|
| 目标 | 接入 MockRuntime 的前端测试；新增迁移门禁 |
| 允许改 | `scripts/check-browser-core-migration.mjs`（新建）、测试文件 |
| 出口 | 门禁接线进 pre-merge.sh |

### M6-H packaged desktop / real native WebView acceptance

| 项 | 内容 |
|---|---|
| 目标 | 真机验收 |
| 必验 | 多页签切换、宫格、面板展开收起、窗口缩放、Ctrl+Shift+T、普通关闭无弹框 |
| 出口 | **用户真实桌面验收**，不得自封 GUI_PASS |

---

## 2. M6-0 量化指标（Spike 必须回答）

| 指标 | 当前基线 | 需 Spike 给出 |
|---|---|---|
| useBrowserHost.ts 行数 | 207 | 改造后预估行数 |
| 手写防御状态类数 | 8（spec §1.3） | 改造后剩余类数 |
| WebView 同步调用点 | 6 类 bridge 调用 | 收敛后的调用点数 |

---

## 3. 回归门禁（每批必跑）

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
bash scripts/pre-merge.sh
node scripts/check-browser-runtime.mjs
node scripts/check-core-boundary.py
node scripts/check-native-webview-overlay.mjs
node scripts/check-window-drag.mjs
git diff --check
```

M6-G 后追加：

```bash
node scripts/check-browser-core-migration.mjs
```

---

## 4. 风险与停止条件

| 风险 | 处置 |
|---|---|
| RealRuntime 沦为改名（无实质收益） | M6-0 拦截；若 M6-A 后发现，停止并回报 |
| 触碰原生危险区（tauri-browser-tabs / linux.rs） | 需 Owner 显式授权，否则停止 |
| 体积预算（上限 25.2%，余量极小） | 每批测 build metrics；超限即停止瘦身或回报 |
| 行为回归（WebView 位置/焦点异常） | 立即 revert 该批 commit |
| 需改 Rust 命令契约 | 停止，回报 Owner（属独立授权） |

---

## 5. 与既有 lane / agent 的关系

依据 `.ai/registry.md`：

| 文件 | 归属 | 本波处理 |
|---|---|---|
| `docs/AI/00-Architecture.md` | Architecture Agent | Owner 已裁决 PENDING 清理（本波执行，见 governance record） |
| `scripts/check-browser-runtime.mjs` | Runtime Checker Agent | **本波不修改**，仅提出语义修正方案（§6） |
| `PROJECT-RULES.md` | Project Rules Agent | 需同步两条 PENDING，本波不擅自改，列入 A0 follow-up |
| `src/**` | — | M6-0 阶段零改动 |

---

## 6. checker 状态语义修正方案（Owner 第 8 条）

### 6.1 问题

当前 `scripts/check-browser-runtime.mjs` 输出：

```json
{"status":"PASS","symbolsFound":0,"targetStatus":"not-yet-implemented"}
```

**问题**：`status: PASS` 与 `not-yet-implemented` 同时出现，极易被误读为「BrowserRuntime 已验收」。历史上该门禁的 PASS 曾被当作架构达标信号。

**根因**：脚本把「零实现」编码为 PASS（哨兵语义），缺少独立的状态维度，状态与结论混在 `status` 字段里。

### 6.2 修正方案（提案，待 A0 批准后由 Runtime Checker Agent 实施）

引入独立 `implementationState` 字段，与 `status` 解耦：

| implementationState | 判定条件 | 含义 |
|---|---|---|
| `NOT_IMPLEMENTED` | 目标符号 0 命中 | 尚未实现（**不是通过**） |
| `PARTIAL` | 部分符号命中，或 interface 存在但无 Real+Mock 双实现 | 进行中 |
| `IMPLEMENTED_PASS` | 全部符号命中且所有规则通过 | 实现且合规 |
| `IMPLEMENTED_FAIL` | 符号命中但存在违规 | 实现但不合规 |

对应 `status`（门禁结论）：

| implementationState | status | exit code |
|---|---|---|
| NOT_IMPLEMENTED | `PASS`（哨兵，保持兼容） | 0 |
| PARTIAL | `PASS` | 0 |
| IMPLEMENTED_PASS | `PASS` | 0 |
| IMPLEMENTED_FAIL | `FAIL` | 1 |

输出示例（修正后）：

```json
{"check":"browser-runtime",
 "status":"PASS",
 "implementationState":"NOT_IMPLEMENTED",
 "humanReadable":"BrowserRuntime 尚未实现（非验收通过）",
 "summary":{"filesScanned":102,"symbolsFound":0}}
```

附加要求：

1. 文本输出必须显式打印 `implementationState`，不允许只打印 PASS
2. `--self-test` 增加 4 个状态用例，覆盖上述四种状态
3. `--strict` 模式下 `NOT_IMPLEMENTED` 视为 FAIL（供未来强制收口时启用）
4. 保留旧字段 `targetStatus` 一段时间以兼容既有脚本，标注 DEPRECATED

### 6.3 执行归属

- 本波**不修改** checker（Owner 第 8 条明确要求先写进计划）
- 由 **Runtime Checker Agent**（`.ai/registry.md` 归属）在 M6-G 前实施
- 需 A0 批准后方可改动脚本

---

## 7. 本波交付边界

本波（规划与治理收口）**仅**：

- 新增 `.ai/workbuddy-dispatch/M6-*` 三份文档
- 更新 `docs/AI/00-Architecture.md` §2.3 的 PENDING（Owner 第 7 条明确点名）

本波**不**：

- 修改任何 `src/**` 或 `src-tauri/**` 源码
- 修改 `scripts/check-browser-runtime.mjs`
- 修改 `PROJECT-RULES.md`
- 修改 `WORKSPACE_IDENTITY.md` 的 LOCK 字段（属 A0 权限）
- 提交或 push（除非 Owner 指示）
