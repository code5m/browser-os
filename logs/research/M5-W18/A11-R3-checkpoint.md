# A11 · M5-W18-R3 Lane Checkpoint — Prototype Acceptance Package

```text
LANE=A11
DISPATCH=M5-W18-R3-UX
STATUS=PROVISIONAL_PENDING_A5_A10_A1_REVISED
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a11/mvp-browser-os-v3
BRANCH=codex/m5-w18-a11
BASE=200f0f1cc032eb7dbd0229ab53a711a5ff1e3d6f (origin/master)
HEAD=b920920
CONSUMED_PEERS=A1 6470fce, A2 63b0a6e, A3 0258c5b, A4 8e77799, A6 d5eb144, A7 6e1f2ba, A8 0db481a, A9 23d12cd
NOT_CONSUMED=A5 (branch still at 200f0f1), A10 (branch still at 200f0f1)
FILES=logs/research/M5-W18/A11-R3-{integration-manifest.json,acceptance-checklist.md,progress.md,acceptance-matrix.py,run-20260908.out,checkpoint.md}
      logs/checkpoints/A11-M5-W18-R3-20260908.md
      logs/checkpoints/Lane-A11-M5-W18-R3-acceptance-20260908-1314.patch
VERIFY=A11-R3 ACCEPTANCE MATRIX: 55 PASS / 0 FAIL / 3 NOT_RUN / 2 GAP_CONFIRMED / 0 GAP_CLOSED / 2 WARN ; GATE: PASS
FINDINGS=F-A11-1..F-A11-8 (8, all OPEN)
NO_PRODUCT_CODE=true
NO_PEER_FILE_EDIT=true
NO_PUSH=true
```

## 1. 启动门禁（逐条实跑）

| 检查 | 结果 |
|---|---|
| 读了 `WORKSPACE_IDENTITY.md` | ✅ 当前派发 `M5-W18-R3-UX`；W18 lane 例外条款允许 `m5-w18-aN` 工作树 |
| 读了 `PARALLEL_COMMAND_BOARD.md` Current Dispatch Entry | ✅ R3 只重开交互研究与原型；禁改产品源码/依赖/ACL/capability/原生运行时/用户数据 |
| 读了 `M5-W18-R3-UX-TASKS-20260908.md` | ✅ A11 卡 = 维护 R3 集成清单 + 用户评审清单；A10 与 A1 终稿后才定稿 |
| 推导 WORKDIR / 分支 | `/home/ainfinit/.codex/worktrees/m5-w18-a11/mvp-browser-os-v3` @ `codex/m5-w18-a11`（lane id 11 三者严格一致） |
| 启动三件套 | `cat .workspace-identity`（内容为 `BACKV3_MAIN` 文案，工作树由主仓复制而来；按 lane 例外以路径+分支+lane id 判定合法，与 A3 记录一致）、`pwd`、`git status --short --branch` 均执行 |
| `git fetch origin` + `git rebase origin/master` | ✅ Successfully rebased；原 5 个 R2B 提交已被 A0 集成入 master，git 提示 skipped（预期），现与 `origin/master` 同点 |
| 工作树干净 | ✅ rebase 后 `git status --short` 无改动 |
| 是否 push | **否** |

## 2. 交付物

| 文件 | 作用 |
|---|---|
| `A11-R3-integration-manifest.json` | **真源**：基线/参考钉版/11 lane 的分支与 HEAD/8 项发现/15 条门限实测/60 条验收项及其探针 |
| `A11-R3-acceptance-checklist.md` | **用户评审清单**：尺寸、模式、右键作用域、J1–J6、Git 14 单元、无障碍、失败态与安全，逐条"怎么验 + 现状" |
| `A11-R3-progress.md` | 蓝图 §9 要求的 "场景 → 现有能力 → 设计就绪 → 代码完成 → 原生验收 → 用户试用" 账本 |
| `A11-R3-acceptance-matrix.py` | 复核脚本：结构自检 + 探针 + 门限算术 + 跨 lane 口径离散告警；纯标准库 |
| `A11-R3-run-20260908.out` | 复核输出（`--report`） |
| 本文件 + `logs/checkpoints/A11-M5-W18-R3-20260908.md` + 补丁 | 给 A0 的收口材料 |

复核命令（任何人可复现）：

```bash
python3 logs/research/M5-W18/A11-R3-acceptance-matrix.py --report   # 人读
python3 logs/research/M5-W18/A11-R3-acceptance-matrix.py --json     # 机读
```

脚本解析产物时**先查工作树文件，再回退 `git show <ref>:<path>`**（`HEAD` / `origin/master` / `codex/m5-w18-a1..a11`），
因此在 lane 内（他 lane 文件不在盘上）与 A0 集成后（全在盘上）都能跑通；取不到时记 `NOT_RUN` 而不是 `PASS`。

## 3. 消费的同侪证据（只读取，未合并、未 cherry-pick、未改他 lane 文件）

| Lane | HEAD | 消费内容 |
|---|---|---|
| A1 | `6470fce` | 浏览器优先替代壳层报告 + 原型 + 检查点（草案，自述待 A2–A9/A10 后出终稿） |
| A2 | `63b0a6e` | 密度审计：9 状态 × 10 尺寸实测、19 个顶栏控件归类、before/after 预算、`min_inner_size` 冲突 |
| A3 | `0258c5b` | 工具窗口规格 + 状态机（58 断言）+ 原型 + 补丁；D-A3-7（固定窗口与全部收起） |
| A4 | `8e77799` | 壳层状态与持久化契约 + 纯函数原型（T1–T15，15/15 PASS） |
| A6 | `d5eb144` | 右键作用域清单 + 命令注册契约（安全分级 / 确认层级 / 审计类 / 键盘与面板等价） |
| A7 | `6e1f2ba` | Git 14 单元矩阵；rebased 钉 `cee14e9`，COPY=0 / ADAPT=5 / REIMPLEMENT=9 / REJECT=3(delta) |
| A8 | `0db481a` | 视觉令牌 + 4 尺寸标注帧；折叠态 91.6%–94.5% 高 / 100% 宽 |
| A9 | `23d12cd` | 五面交互安全：五级安全分级、T0–T4、脏树门禁、受保护分支、审计脱敏 |

## 4. 复核结果（摘录）

```
[1] structural checks       8/8 PASS  (60 唯一 id、owner 可解析、finding 引用不悬空、rebased 已钉)
[2] checklist probes        55 PASS / 0 FAIL / 3 NOT_RUN(待派发) / 2 GAP_CONFIRMED / 0 GAP_CLOSED
[3] numeric gates           15 条全达标（含 6 条"现有产品基线预期失败"，作为改造前的反证）
    divergence WARN x2     collapsed width spread 4.0 (A1=96.0 A3=100.0 A8=100.0)
                           top_chrome_px  spread 3.0 (A1=68.0 A8=65.0)
A11-R3 ACCEPTANCE MATRIX: 55 PASS / 0 FAIL / 3 NOT_RUN / 2 GAP_CONFIRMED / 0 GAP_CLOSED / 2 WARN
GATE: PASS
```

两条 `WARN` 是**刻意保留**的：它们不是失败，而是把 F-A11-4 / F-A11-5 的口径分歧固定成可复算的断言，
任何人重跑都会看到同一组数字，A0 冻结口径后把 `divergence_tolerance_pp` 收敛即可自动转 PASS。

## 5. 发现与派发（全部 OPEN，A11 不自裁）

| ID | 级别 | 归属 |
|---|---|---|
| F-A11-1 | high | A10 裁定 → A3/A4 对齐 → A1 原型体现（**A4 说固定窗口在"全部收起"中保留，A3 说一起收起再一键恢复，二者直接冲突**） |
| F-A11-2 | high | A5 交付 → A1 修订 → A11 重跑（J2 与数据库模式现无任何验收证据） |
| F-A11-3 | medium | A1 终稿（原型只见 hunk/stash/cherry/branch/log/graph；worktree/blame/amend/reset/revert/conflict/patch/命令日志/diff 仅存在于报告表格） |
| F-A11-4 | medium | A0 冻结"后标题栏"口径（A2 拥有测量方法） |
| F-A11-5 | medium | A0 冻结顶部 chrome 定义与预算 |
| F-A11-6 | medium | A1 终稿补 aria/role，A10 复核 |
| F-A11-7 | low | A0 裁定命令入口快捷键，A6 登记 |
| F-A11-8 | low | A0 裁决 800×600 去留 |

## 6. 债务

- **D-A11-1**：`A1` 与 `A5`/`A10` 的终稿未到，故清单为 `PROVISIONAL`；A11 在 A0 集成后需重跑脚本并更新 `lanes.*.head`。
- **D-A11-2**：门限数字取自各 lane 自报（A1 报告 §4、A3 自测输出、A8 §5、A2 实测 out），A11 未独立复算几何；口径统一后应以 A2 方法重测一次。
- **D-A11-3**：Git 单元"原型可见性"用 token 探针近似（存在 ≠ 可操作）；原生验收时须人工走查 14 单元。
- **D-A11-4**：J5（自动化与运行观察）R3 卡未给任何 lane 派发原型，账本只能记 `设计就绪=NO`。

## 7. 边界声明

- 零产品代码改动；未改 `src/**`、`src-tauri/**`、`scripts/**`、ACL、capability、manifest、lockfile、用户 vault。
- 未改他 lane 文件；未改共享主文档（`PARALLEL_COMMAND_BOARD.md`、`WORKSPACE_IDENTITY.md`、三份主文档）。
- 未 push；未跑 `npm run build` / `pre-merge.sh`（本波零产品代码，蓝图 §7 明确"本轮仅文档/合成研究资产，不跑 11 次全量构建"）。
- 未复制 rebased / IntelliJ / Obsidian 任何源码或资产；参考钉版与许可归 A7/A10。
- `W19` 仍 `CLOSED`；本 lane 不开启任何编码片。
