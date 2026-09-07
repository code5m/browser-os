# A1 M5-W16 reconciliation checkpoint（DOCS ONLY · 2026-09-08 11:00 CST）

> Lane A1（M5-W16 · reconcile W15 acceptance + author M6 WBS/ordering proposal）
> 派发来源：`PARALLEL_COMMAND_BOARD.md` L1163+（M5-W16 M5 Closeout and M6 Charter Dispatch）
> A0 W15 acceptance 来源：`logs/checkpoints/A0-M5-W15-accept-W16-dispatch-20260908-1015.md`
> 配套交付：**M6 WBS/排序提案** `logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md`
> 整包交付：本 checkpoint + `logs/checkpoints/Lane-A1-M5-W16-20260908-1100.patch`（patch 内 12 文件）
> A1 W16 立场 = **DOCS ONLY · 零产品代码 · 不 push**

---

## §1 任务接收

**W16 派发** = M5 Closeout and M6 Charter Dispatch（board L1163+）：*W16 produces the only permissible next step: a reviewed M6 charter before any new runtime authority is implemented. Only A0 pushes.*

**A1 W16 任务边界（冻结）**：
1. Reconcile W15 acceptance（基线 `052b18a`）
2. Author the M6 WBS/ordering proposal（草案，纯文档）
3. Hard stops（board L1181）：无产品代码 / 无新命令·ACL·bridge·DTO / 无 plugin 调用执行 / 无动态加载 / 无网络下载监听 / 无 daemon / 无模型调用 / 无 Agent-Skill 执行 / 无 MCP 扩张 / 无 graph 写导出 / 无后台 worker / 无 raw Tauri invoke / 无敏感渲染持久化
4. **不** push

**启动门禁实测**：

| 项 | 实测 | 判定 |
|---|---|---|
| `pwd` | `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3` | ✅ |
| `.workspace-identity` | `WORKSPACE_ID=BACKV3_MAIN` | ✅ |
| branch | `master` | ✅ |
| `git fetch` + `git pull --ff-only` | 与 `origin/master` 一致，`052b18a`；工作树 **clean**（A0 已拣入 A1 W15 整包并推送） | ✅ |
| A1 W15 整包去向 | `A1-M5-W15-reconciliation-20260908-0930.md` + `Lane-A1-M5-W15-reconciliation-20260908-0930.patch` 已进 `master` 历史 | ✅ 已被 A0 拣入 |

---

## §2 整包改动清单（patch 内 12 文件 = 10 修订 + 2 new）

| # | 文件 | 改动 |
|---|------|------|
| 1 | `PARALLEL_COMMAND_BOARD.md` | L3 时间戳 + L6 主线条 `886ea29` → `052b18a`（W15 PUSHED·accepted）+ L7 主线条链 + L8 `Current NEXT` → `M5-W16` |
| 2 | `详细设计与实施计划.md` | A1 W16 update 行（紧随 A0 2026-09-08 10:15 行后） |
| 3 | `AI-模型切换与接手清单.md` | A1 W16 update 行 |
| 4 | `后续需求TODO.md` | A1 W16 update 行 |
| 5 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | L1 title 加 `W15 PUSHED + W16 active` + 末尾 [W16 active] 段 + **25% → 25.2% 上限更正** |
| 6 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | 头增 W15 PUSHED + W16 ACTIVE status 行 |
| 7 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | 头增 W15 PUSHED + W16 ACTIVE status 行 |
| 8 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | 头增 W15 PUSHED + W16 ACTIVE status 行 |
| 9 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | L1 title + **[W15 verification scope] 门禁项 25.0 → 25.2 更正** + W15 PUSHED 结果回填 |
| 10 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | L2 title + 末尾 [W16 active] 段（W15 PUSHED 对账 + 25.2% 更正 + 手动 GUI 证据残留挂账） |
| 11 | `logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md` | **M6 WBS / 排序提案（new · A1 W16 第二项交付）** |
| 12 | `logs/checkpoints/A1-M5-W16-reconciliation-20260908-1100.md` | **本 checkpoint（new）** |

> 交付物另含 `logs/checkpoints/Lane-A1-M5-W16-20260908-1100.patch`（补丁容器，不自我包含）。

---

## §3 W15 acceptance 对账（依据 `052b18a` + A0 acceptance）

**A0 在 `052b18a` 中拣入 W15 并接受（`STATUS=PASS`）**：

- 两个高危写确认对话框（`ConfirmModal.vue` / `GitWriteConfirmDialog.vue`）获得 `role/aria-modal`、初始焦点、Tab/Shift+Tab 焦点环、Escape 关闭、焦点归还
- **未改动**任何 backend 命令 / ACL / bridge / DTO / 网络 / worker / plugin 执行 / MCP 扩张 / Agent-Skill 执行 / graph 写导出 / 模型调用面

**A0 拣入期调整（重要，已回填到本整包）**：
- A6 的第一个通用 async-state 组件会把总体积增长抬到 **25.60%** → **已移除**（受影响视图本就有本地 empty/loading UI）
- 焦点管理范围收窄为两个高危写确认
- 最终实测 **25.14%**，A0 为该无障碍工作**一次性**把上限由 25.0% 抬到 **25.2%**；**M5 后续任何增长需新的 A0 显式决策**

**A0 验证实跑**：

| 项 | 结果 |
|---|---|
| `cargo test`（full Rust） | 431 passed |
| `cargo test --features mcp mcp_server` | 21 passed |
| `cargo build --release --locked` | success（2 条既有 `grid_process.rs` dead-code 警告） |
| `npm run build` | success（仅既有 mixed static/dynamic import 警告） |
| plugin / graph / MCP / Agent-Skill / UI privacy / UI logic 策略 | PASS |
| `node scripts/check-ui-a11y-logic.mjs` | PASS |
| `measure-build-metrics.py --compare --skip-build` | **25.14% ≤ 25.2%**，warning delta 0 |
| `bash scripts/pre-merge.sh` | ALL_PASS |
| `git diff --check` | PASS |

**A0 残留风险**：**手动 GUI/运行时证据仍是已记账的验收项，未伪造**（A1 已在 M6 WBS §1.3 / §6 列为开放问题并建议排期补录）。

---

## §4 体积门禁跟踪更正（A1 W15 → W16 的关键修订）

| 项 | A1 W15 记录 | W16 更正后 |
|---|---|---|
| 生效上限 | 25.0%（`measure-build-metrics.py` L38 `TOTAL_BYTES_GROWTH_LIMIT_PCT`） | **25.2%**（A0 一次性抬升，需同步改常量与注释） |
| 实测 | 24.89% | **25.14%** |
| 余量 | 0.11pp | **0.06pp** |
| M5-13 [W15 verification scope] 第 13 项 | `total_bytes_pct ≤ 25.0` | **≤ 25.2**（已更正） |
| M5-0 / M5-14 记录 | 25% | **25.2%**（已更正） |

**A1 不修的项（交 A0 / A11）**：
- `scripts/measure-build-metrics.py` L35-38：常量与注释仍写 25.0 / 24.89 → 需 A0 或 A11 同步为 25.2 / 25.14（**门禁脚本不属 A1 允许范围**）
- `scripts/pre-merge.sh` L76 文案仍写「总体积 ≤15%」（W15 已记录，仍未修）

---

## §5 M6 WBS / 排序提案（A1 W16 第二项交付）

**文档**：`logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md`（草案）

**核心内容**：
- M5 收口事实（已开放 6 类能力 / 仍 LOCKED 11 类权限 / 3 项残留）
- **6 个候选工作流**：M6-1 plugin stage-II 有界调用 · M6-2 MCP stdio 真实（只读绑定）· M6-3 Agent/Skill 执行授权 · M6-4 graph 受控写与导出 · M6-5 后台 worker 有界队列 · M6-6 分发与签名（**默认不进 M6，建议独立 M7**）
- **波次排序**：M6-W1 契约冻结+入口闸（= W16）→ W2 M6-2 → W3 M6-1 → W4 M6-3 → W5 M6-5 → W6 M6-4 →（W7 条件）M6-6
- **6 条排序原则**：一次只开一种运行时权限 / 默认 fail-closed / 复用既有闸门 / 命令五件套同包 / 体积硬顶 25.2% / 证据先于结论
- **M6 入口闸 11 项 checklist**（PO 书面批准 + A10 裁定 + A11 计划骨架 + A2/A3/A4/A5/A7/A9 输出 + 25.2% 预算 + A8 手动证据补录计划）
- **7 项非目标**、**4 个开放问题**（53 条债分配 / 是否再抬阈值 / M6-6 归属 / 手动证据补录责任）

---

## §6 A1 W16 立场

### 6.1 必做（已完成）

- ✅ reconcile W15 acceptance（`052b18a`，board L6/L7/L8 + 3 主文档 + M5-0/10/11/12/13/14）
- ✅ 更正体积门禁记录 25.0% → **25.2%**（M5-0 / M5-13 / M5-14）
- ✅ author M6 WBS/ordering proposal（独立文档）
- ✅ 记录 A0 残留风险（手动 GUI/运行时证据未补录）

### 6.2 必不做（A1 零产品代码 / 零 push）

- ❌ 不写 `src/**`、`src-tauri/**`、`scripts/**`（含门禁脚本常量与文案修正）
- ❌ 不动命令 / ACL / `bridge.ts` / `types.ts` / DTO
- ❌ 不实施 M6 任何候选工作流（W16 = 文档与设计 only，等 PO 批准）
- ❌ 不 commit、不 push；不收其他 lane 改动进本 patch

---

## §7 整包交付与自检

- ✅ `logs/checkpoints/A1-M5-W16-reconciliation-20260908-1100.md`（本 checkpoint）
- ✅ `logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md`（M6 WBS 提案）
- ✅ `logs/checkpoints/Lane-A1-M5-W16-20260908-1100.patch`（路径限定 A1 范围，patch 内 12 文件）
- ✅ 自检：`grep -c "^diff --git"` = **12**；`git apply --check --reverse` 通过

**patch 生成（路径限定）**：

```bash
git add -N logs/checkpoints/A1-M5-W16-reconciliation-20260908-1100.md \
           logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md
git diff --binary -- PARALLEL_COMMAND_BOARD.md 详细设计与实施计划.md \
  AI-模型切换与接手清单.md 后续需求TODO.md logs/checkpoints/M5-20260906 \
  logs/checkpoints/A1-M5-W16-reconciliation-20260908-1100.md \
  logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md \
  > logs/checkpoints/Lane-A1-M5-W16-20260908-1100.patch
```

---

## §8 关联指针

- W16 派发：`PARALLEL_COMMAND_BOARD.md` L1163-1185
- A0 W15 accept + W16 dispatch：`logs/checkpoints/A0-M5-W15-accept-W16-dispatch-20260908-1015.md`
- M6 WBS 提案：`logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md`
- A1 W15 整包（已拣入）：`logs/checkpoints/A1-M5-W15-reconciliation-20260908-0930.md`
