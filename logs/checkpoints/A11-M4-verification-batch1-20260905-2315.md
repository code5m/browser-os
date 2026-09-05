# A11 · M4 验证证据批次包 #1（Batch Verification Evidence）

> Lane：**A11（M4 verification evidence）** ｜ 路由 `AI:BALANCED / R:medium`
> 批次时间：2026-09-05 23:15 CST ｜ 执行者：CodeBuddy Hy4
> 基线：`BASE=85d2d7b`（`git rev-parse --short HEAD`）；`origin/master = fb5b78f`（本地领先 2 个提交，均非 A11 提交）
> 触发依据：`PARALLEL_COMMAND_BOARD.md` §Batch Implementation Rule 与 §Lane A11: Batch Verification Evidence
> 范围声明：**只写 `logs/` 下验证/债务文档，零产品代码改动；未提交、未 push**（board：只有 A0 向 `master` 提交并推送）

---

## 1. 启动门禁与同步（实测）

```text
cat .workspace-identity            → WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master
pwd                                → /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3   ✅
git status --short --branch        → ## master...origin/master [领先 2]（他车道在途文件见 §6）
git log --oneline -12              → HEAD = 85d2d7b「docs(M4): expand batch implementation dispatch」
git fetch origin                   → 无新对象（origin/master = fb5b78f）
git pull --ff-only                 → 已经是最新的。
```

判定：**通过**（规范主副本、`master`、与远端同步；工作树中的他车道在途改动不属于 A11，A11 一律未触碰）。

---

## 2. 批次触发判定（board §Lane A11「Update after」四条）

| 触发批次 | 状态 | A11 动作 |
|---|---|---|
| DB 后端批次 A2 / A3 / A4 | 🟡 **在途未完**（23:15 取样时 `src-tauri/src/database*` 与 `scripts/check-database*` 均不存在；工作树有 `Cargo.toml` / `Cargo.lock` / `domain.rs` / `security_policy.rs` / `bridge.ts` / `types.ts` 在途改动。23:2x 复检时 `scripts/check-database-policy.py` 已出现但未验证） | **不代签**；只登记在途状态与复跑口径 |
| DB UI 批次 A5 | 🟡 **在途未完**（`DatabasePanel.vue` / `useDatabaseStore.ts` / `dbUi.ts` / `check-database-ui-logic.mjs` 已在工作树） | 同上 |
| 调度后端批次 A6 / A7 | 🟢 **A6 完成**（M4-5.d 夹具 + pre-merge 第 23 项 + 检查点）／🟡 A7 在途（`tasks.rs` 已在工作树） | **本包核心**：对 A6 完成部分出具验收与验证映射 |
| 调度 UI 批次 A8 | 🟡 **在途未完**（`TaskPanel.vue` / `TaskEditDialog.vue` / `useTaskStore.ts` / `taskUi.ts` 已在工作树） | 不代签 |

**本包定位**：调度侧 A6 批次已闭环 → 出**完整验证包**（矩阵 + 清单 + 台账 + 本检查点 + 补丁）；DB 侧与 A7/A8 在途 → 只做**状态记录与复跑口径**，任何一条都未签 PASS。

---

## 3. 本批次实测证据（命令 + 原文输出）

| # | 命令 | 结果 | 备注 |
|---|---|---|---|
| 1 | `cargo test --manifest-path src-tauri/Cargo.toml` | `test result: ok. 237 passed; 0 failed; 0 ignored`（7.05 s） | 取样于本批次开始（HEAD `a75ba24`→`85d2d7b` 之间） |
| 2 | `cargo test --manifest-path src-tauri/Cargo.toml domain` | `test result: ok. 26 passed; 0 failed`（211 filtered out） | 含 M4-1.b 的 `T-db-c1~c5` |
| 3 | `cargo test --manifest-path src-tauri/Cargo.toml scheduler` | `test result: ok. 0 passed`（237 filtered out） | 调度产物未落地，符合预期 |
| 4 | `cargo test --manifest-path src-tauri/Cargo.toml database` | **E0106 编译失败**，`src/domain.rs:1282` 起（缺生命周期标注） | **他车道在途代码**，非 A11 引入 |
| 5 | `python3 scripts/check-plan-routing.py` | `check-plan-routing: ok (50 WBS rows)` | EXIT 0 |
| 6 | `python3 scripts/check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS: 2 好样本零违规（真实仓库 + 合成参考实现） + 20 个坏样本全部检出（含变异防呆）；ACTIVE=14 PENDING=6` | EXIT 0 |
| 7 | `python3 scripts/check-scheduler-policy.py` | EXIT 0（无违规输出） | 9→14 ACTIVE 说明脚本在并发演进中 |
| 8 | `python3 scripts/check-scheduler-policy.py --expect-pending` | `SCHED_PENDING_RESULT=NONE（6 个 pending 码位均未实现）` | EXIT 0 |
| 9 | `git diff --check` | 无输出 | EXIT 0 |
| 10 | `ls src-tauri/src/database*` / `scheduler*` / `tasks*` | `database*` / `scheduler*` 无；`tasks.rs` 在途新增 | 批次完成度取证 |
| 11 | 23:2x 复检 `git status --short` | `?? scripts/check-database-policy.py`、`?? scripts/check-database-ui-logic.mjs`、`?? scripts/check-scheduler-ui-logic.mjs` 已出现；`M logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md` | 三条 M4 夹具/UI 逻辑脚本与 A6 契约正文在**封包后**继续演进；本包结论以 23:15 取样为准，A0 集成时按当时 HEAD 复跑 |

**未复跑及理由**（A0 集成后按 T0-4 / T0-5 / T0-8 补跑）：

- `cargo test`（全量）/`npm run build`/`bash scripts/pre-merge.sh`：**在他车道在途代码未编译通过时复跑，会把在途失败记到 A11 名下**（见证据 #4）。
- `cargo check --locked`：依赖清单正在被 A3 修改（`Cargo.toml` / `Cargo.lock` 均为在途 M），此时 `--locked` 必然失败，结论无意义。

---

## 4. 与 A10 增量复核（R-1~R-11）的对接

> 来源：`logs/assist/A10-M4-security-recheck-A1A2A6-20260905-2345.md`（结论 `PASS_WITH_DEBT`，以源码实证替代采信）；A6 已在 `A6-M4-5.d-20260905-2350.md` §5 逐条接受。

| A10 项 | A11 处置 | 落点 |
|---|---|---|
| R-1（高）审计冲刷 | 转为债务 **D30** + 手测 **F-1** | 台账 / 清单 F 组 / 矩阵 §3.5.5 |
| R-2（中高）运行记录挤出 | 转为债务 **D31** + 手测 **F-2**（与 D21 同源） | 同上 |
| R-3（中）`dangerous` 片段无人值守放大 | 转为开放项 **O-A11-1**（A0 二选一）+ 手测 **F-3** | 台账开放项 / 清单 F 组 |
| R-4（中）`secret` 事后改标不复检 | 转为债务 **D32** + 手测 **F-4** | 同上 |
| R-5（中）未标 `secret` 明文落盘 | 转为债务 **D33**（残余风险，建议 A0 接受）+ 手测 **F-5** | 同上 |
| R-6（中高）M4-1.c 自相矛盾 | 维持 §3.5.1 的 `STOPPED_CONTRADICTORY_ARTIFACT` 口径；**澄清前 M4-1.d 的上限数值不得作为验收基线** | 矩阵 §3.5.1 |
| R-7（中）上限常量未落代码 | 转为债务 **D34** | 台账 |
| R-8（低）注释与脚本不符 | 转为债务 **D36** | 台账 |
| R-9（低）ACL 计数/锚点 | 转为开放项 **O-A11-2**（A4，按末条锚点判定） | 台账 / 矩阵 T1-a |
| R-10（信息）`O-A6-11` 失效 | ✅ **关闭**，A0 无需动作 | 台账开放项 |
| R-11（低）重试总时长上界未冻结 | 转为债务 **D35** | 台账 |

---

## 5. 债务台账增量（本批次）

| 编号 | 一句话 | 关闭验证 |
|---|---|---|
| **D30** | `audit.json`（cap 1000 FIFO）被高频定时任务冲刷，手工审计可在 ~8.3 小时内被替换一轮 | A0 裁决 → A7 删 `TASK_AUDIT_EVENTS` 两行；手测 **F-1** |
| **D31** | `script-runs.json`（cap 200）被定时任务挤出手工记录（~3.3 小时），叠加 D21 后不可辨 | A7 独立文件/配额；手测 **F-2** |
| **D32** | `secret` 参数事后改标不复检，明文值继续执行 | A7 加载期/更新期复跑；手测 **F-4** |
| **D33** | 未标 `secret` 的参数明文落盘 `tasks.json`（残余风险） | A0 接受 + A8 软提示；手测 **F-5** |
| **D34** | 上限常量未落代码，零代码级约束 | A3 落常量 + 单测 + 码位 |
| **D35** | 重试总时长硬上界未冻结（最坏 ≤5400s/任务） | A7 冻结 + `SCHED_RETRY_UNBOUNDED` |
| **D36** | `domain.rs:819` 注释声称被不存在的 `check-database-policy.py` 守护 | A4 落地脚本 + 订正注释 |

**后续新债务编号从 `D37` 起**（D27~D29 归 A2，D30~D36 归 A11 本批次）。

---

## 6. 人工清单增量（F 组，8 条）

`logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md` 新增 **F 组**（调度批次新增用例）：F-1 审计冲刷 / F-2 运行记录挤出 / F-3 危险片段引用 / F-4 secret 事后改标 / F-5 敏感值软提示 / F-6 cron 位数禁用 6 位 / F-7 新建任务默认关闭 / F-8 跳过原因可见。

---

## 7. 交付物与补丁

| 文件 | 状态 | 说明 |
|---|---|---|
| `logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md` | 改 | 新增 §3.5（契约现状看板：M4-1.a~d / M4-5.a~d 五行总表 + M4-1.c 矛盾取证 + 契约点改写表 + A6 影响 + 夹具码位对照 + A10 R-映射 + 在途批次快照） |
| `logs/assist/M4-A11-debt-ledger-20260905-2240.md` | 改 | 新增 D30~D36；新增开放项 O-A11-1 / O-A11-2；关闭 O-A6-11；更新 D21/D22/D24/D26 与各车道避坑摘要 |
| `logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md` | 改 | 新增 F 组 8 条 |
| `logs/checkpoints/A11-M4-verification-batch1-20260905-2315.md` | 新增 | 本文件 |
| `logs/checkpoints/Lane-A11-verification-batch1-20260905-2315.patch` | 新增 | 单补丁，`git diff --binary -- <A11 文件>` 生成 |

生成与校验命令：

```bash
git add -N logs/checkpoints/A11-M4-verification-batch1-20260905-2315.md
git diff --binary -- \
  logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md \
  logs/assist/M4-A11-debt-ledger-20260905-2240.md \
  logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md \
  logs/checkpoints/A11-M4-verification-batch1-20260905-2315.md \
  > logs/checkpoints/Lane-A11-verification-batch1-20260905-2315.patch
git apply --check -R logs/checkpoints/Lane-A11-verification-batch1-20260905-2315.patch   # 形状校验
```

> **补丁边界**：本补丁只含上述 A11 自有文件。工作树中 `scripts/pre-merge.sh`、`scripts/check-scheduler-policy.py`、`src-tauri/**`、`src/**`、三份主文档等的改动均属**其他车道在途成果**，A11 未纳入补丁、未修改、未删除。

---

## 8. A0 集成步骤（可直接执行）

```bash
git apply --check logs/checkpoints/Lane-A11-verification-batch1-20260905-2315.patch
git apply        logs/checkpoints/Lane-A11-verification-batch1-20260905-2315.patch
git add logs/checkpoints/A11-M4-verification-batch1-20260905-2315.md \
        logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md \
        logs/assist/M4-A11-debt-ledger-20260905-2240.md \
        logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md
# 待他车道在途代码编译通过后，再按 T0 复跑：
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
bash scripts/pre-merge.sh
git diff --check
```

**A0 需先裁三件事**（否则后续验收口径不成立）：

1. **M4-1.c 状态**（矩阵 §3.5.1 / A10 R-6）——决定 M4-1.d 的上限数值是否可作验收基线。
2. **D30 / R-1**——`task.run.start` / `task.run.finish` 是否移出 `audit.json`（本轮唯一可能实际破坏可审计性的问题）。
3. **O-A11-1 / R-3**——定时任务能否引用 `dangerous=true` 片段。

---

## 9. 声明（避免误读）

1. **不代裁决**：M4-1.c 矛盾状态、D28、D30、O-A11-1 / O-A11-2 均只做「事实记录 + 取证命令 + 影响面」，结论权归 A0。
2. **不代签**：DB 侧（A3/A4/A5）与 A7/A8 尚在途，本包**未对任何一条实现类用例签 PASS**；所有真机/目视类用例在环境不具备时记 `NOT_RUN`。
3. **快照性质**：并行 12 车道持续提交，本包所有结论以「命令 + 实测输出 + 取样时刻」为准，不绑定 SHA 语义；`scripts/check-scheduler-policy.py` 的码位数在本次取样期间就由 `ACTIVE=9 PENDING=8` 变为 `ACTIVE=14 PENDING=6`。
4. **零产品代码**：本批次未创建/修改任何 `src/`、`src-tauri/`、`scripts/`、三份主文档、依赖清单文件；未提交、未建分支、未 push。
