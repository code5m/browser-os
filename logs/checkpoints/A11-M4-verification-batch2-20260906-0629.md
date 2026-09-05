# A11 · M4 验证证据批次包 #2（Batch Verification Evidence）

> Lane：**A11（M4 verification evidence）** ｜ 路由 `AI:BALANCED / R:medium`
> 批次时间：2026-09-06 06:29 CST ｜ 执行者：CodeBuddy Hy4
> 基线：`BASE=85d2d7b`（`git rev-parse --short HEAD`）；`origin/master = fb5b78f`（本地领先 2 个提交，均非 A11 提交）
> 触发依据：`PARALLEL_COMMAND_BOARD.md` §Batch Implementation Rule 与 §Lane A11「Update after」四条批次
> 范围声明：**只写 `logs/` 下验证/债务文档，零产品代码改动；未提交、未 push**（board：只有 A0 向 `master` 提交并推送）

---

## 1. 启动门禁与同步（实测）

```text
cat .workspace-identity            → WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master
pwd                                → /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3   ✅
git status --short --branch        → ## master...origin/master [领先 2]（多车道在途文件，见 §5）
git log --oneline -1               → HEAD = 85d2d7b「docs(M4): expand batch implementation dispatch」
git fetch origin                   → 无新对象（origin/master = fb5b78f）
git pull --ff-only                 → 已经是最新的。
```

判定：**通过**（规范主副本、`master`、与远端同步；工作树中的他车道在途改动均非 A11，A11 一律未触碰）。

---

## 2. 批次落地判定（board §Lane A11「Update after」四条）

| 触发批次 | 状态 | A11 动作 |
|---|---|---|
| DB 后端批次 A2/A3/A4 | 🟢 **已落地** | 出验收映射 + 集成门禁证据；记录 D37/D39 |
| DB UI 批次 A5 | 🟢 **已落地** | 记 119 断言通过、`npm run build` 0 error |
| 调度后端批次 A6/A7 | 🟢 A6 完成 / 🟡 A7 已落地 `tasks.rs` 但**未接 `task_*` 命令** | A6 夹具闭环（ACTIVE=20 PENDING=0）；A7 类型警告待接线（D42） |
| 调度 UI 批次 A8 | 🟢 **已落地** | 记 102 断言通过 |

**本包核心结论**：四批次功能代码均已落地、`cargo test`（src-tauri）**323 passed**、前端逻辑夹具全绿；但 **`pre-merge.sh` 集成门禁当前 RED（5 项 FAIL）**，其中 **D37（多语句安全红线被 `database.rs` 违反）为最高优先阻塞**。A11 不代修、不代签 PASS，只如实记录并给出归属。

---

## 3. 本批次实测验证证据（命令 + 原文输出）

| # | 命令 | 结果 | 备注 |
|---|---|---|---|
| 1 | `cargo test --manifest-path src-tauri/Cargo.toml` | `test result: ok. 323 passed; 0 failed; 0 ignored`（7.06s） | 编译通过；较 09-05 237 增 86 |
| 2 | `cargo test domain` | 29 passed | 含 T-db-c1~c8 |
| 3 | `cargo test database` | 通过（A3 `database.rs` 测试） | 真实 SQLite 路径已测 |
| 4 | `cargo test scheduler` | 通过 | 假时钟断言 |
| 5 | `cargo test tasks` | 通过 | A7 cron 校验 |
| 6 | `npm run build` | 0 error；主 JS **197.04 kB**（gzip 70.52） | +22.11% vs 161.36 kB 基线 → **超 15% 门禁（D40）** |
| 7 | `python3 scripts/check-plan-routing.py` | ok (50 WBS rows) | EXIT 0 |
| 8 | `python3 scripts/check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS: 2 好样本 + 20 坏样本全检出；ACTIVE=20 PENDING=0` | EXIT 0 ✅ |
| 9 | `python3 scripts/check-scheduler-policy.py`（默认） | EXIT 0 ✅ |  |
| 10 | `python3 scripts/check-scheduler-policy.py --expect-pending` | `SCHED_PENDING_RESULT=NONE（0 个 pending 码位均未实现）` | EXIT 0 ✅ |
| 11 | `node scripts/check-database-ui-logic.mjs` | 119 断言全部通过 ✅ |  |
| 12 | `node scripts/check-scheduler-ui-logic.mjs` | 102 断言全部通过 ✅ |  |
| 13 | `python3 scripts/check-scheduler-ui-policy.py`（默认） | EXIT 0 ✅ |  |
| 14 | `python3 scripts/check-database-policy.py --self-test` | `DB_SELF_TEST_RESULT=FAIL`（`DB_MULTI_STATEMENT_FORBIDDEN`） | 🔴 D37 |
| 15 | `python3 scripts/check-database-policy.py --expect-pending` | `DB_PENDING_RESULT=FAIL`（`database.rs 使用 execute_batch 处理用户 SQL`） | 🔴 D37/D38 |
| 16 | `git diff --check` | 无输出 ✅ |  |
| 17 | `bash scripts/pre-merge.sh` | `PRE_MERGE_RESULT=FAIL`（EXIT=1，**5 项 FAIL**） | 见 §4 |

---

## 4. 集成门禁（pre-merge）红灯分解（2026-09-06 06:29）

```text
[pre-merge] FAIL: cargo fmt main                              → D39（45 文件未格式化）
[pre-merge] FAIL: build metrics regression                   → D40（+22.11% > 15%）
[pre-merge] FAIL: check-tools-policy.py --self-test           → D41（既有缺陷，非 M4）
[pre-merge] FAIL: check-database-policy.py --self-test        → D37（DB_MULTI_STATEMENT_FORBIDDEN）
[pre-merge] FAIL: check-database-policy.py --expect-pending   → D37/D38
[pre-merge] PRE_MERGE_RESULT=FAIL
```

**根因取证**：
- **D37（最高优先）**：`grep -n execute_batch src-tauri/src/database.rs` → `database.rs:939` 在用户 SQL 路径调用 `conn.execute_batch(...)`；`detect_multiple_statements`（`:386`）已实现但未在驱动执行前强制拦截用户 SQL。违反 M4-1.c §5 与 G-4「整批多语句拒绝」。
- **D39**：`cargo fmt --check -- src-tauri` → 45 文件有格式 diff（M4 引入文件未格式化）。
- **D40**：`rusqlite`(bundled)+`mysql`+`postgres` 在功能未接线即吃满体积预算（A10 D1-3/S1-5）。
- **D41**：`check-tools-policy.py` 坏样本「list_tools 晚于 list_artifact_images」变异失配，按漏检计（既有门禁缺陷）。
- **D42**：`cargo check` 生成 36 warnings（A7 `TaskDef` 等类型暂无消费者）。

---

## 5. 工作树在途文件快照（本判决据）

```text
M  src-tauri/Cargo.toml / Cargo.lock            （A3 依赖）
M  src-tauri/src/domain.rs                      （A2 M4-1.c 常量 + A7 调度类型）
M  src-tauri/src/security_policy.rs             （A4 分类器）
M  src-tauri/src/workspace.rs                    （A7）
M  src-tauri/src/bridge.rs / main.rs            （A4/A7 注册中）
M  src-tauri/permissions/default-commands.toml  （A4/A7）
?? src-tauri/src/database.rs / tasks.rs         （A3 / A7）
?? scripts/check-database-policy.py / check-database-ui-logic.mjs / check-scheduler-policy.py / check-scheduler-ui-logic.mjs / check-scheduler-ui-policy.py
?? src/components/workspace/DatabasePanel.vue / TaskPanel.vue / TaskEditDialog.vue
?? src/stores/useDatabaseStore.ts / useTaskStore.ts / src/utils/dbUi.ts / taskUi.ts
M  logs/checkpoints/M4-1.c-20260905-2255.md / M4-1.d-20260905-2300.md / A6-M4-5-scheduler-contract-20260905-2330.md（A2/A6 收口）
?? logs/assist/A10-M4-security-review-batch-{DB-1,SCHED-1}-20260905-2319.md / A9-M5-*.md
?? logs/checkpoints/A5-M4-4-database-ui-20260905-2320.md / A6-M4-5.d-20260905-2350.md / M4-5.d-20260905-2355.md / Lane-A{1,2,5,10,11}-*.patch
```

> A11 本包**仅改动** `logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md`、`logs/assist/M4-A11-debt-ledger-20260905-2240.md`、`logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md` 与新增本文件；其余一律未触碰。

---

## 6. 与 A10 批次复核（D1-*/S1-*）对接

| A10 编号 | 严重度 | A11 处置（台账编号） |
|---|---|---|
| D1-1 重复常量 `DB_MAX_SQL_BYTES` | 中高 | 保留 A10 文档备查；当前数值一致未构成本轮阻塞；建议 A3/A4 统一到 `domain.rs` 单定义 |
| D1-2 `database.rs` 缺失 → 现落地但 `execute_batch` 违规 | 中 | **D37（集成阻塞，最高优先）** |
| D1-3 / S1-5 体积 +14.6%→+22.11% | 中高 | **D40（集成阻塞，A0 决策）** |
| D1-4 M4-1.c STOPPED | 低 | ✅ **已收口**：A2 重写 `COMPLETE`，本包 §3.5.1 同步 |
| D1-5 无命令暂不适用 ACL | 低 | 随 A7 接线后由 O-A11-2 跟踪 |
| S1-1 夹具自检曾失败 | 已修复·留档 | 无需处置 |
| S1-2 CRON_MACRO 误报 | 阻断 | ✅ **已修复**：调度夹具 ACTIVE 20 / PENDING 0，pre-merge 该项转绿 |
| S1-3 码位计数过期文案 | 低 | 建议 A6/A0 订正 pre-merge 文案 |
| S1-4 警告 2→27→36 | 中 | **D42（挂账，A7 接线后消失）** |
| R-1~R-11 | — | 见 batch#1（D30~D36、O-A11-1/2） |

---

## 7. 交付物与补丁

| 文件 | 状态 | 说明 |
|---|---|---|
| `logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md` | 改（+39 行） | §3.5.0（M4-1.c COMPLETE）、§3.5.1（收口）、§3.5.4（夹具码位现状）、§3.5.6（落地现状）、§3.5.7（实测证据）、§3.5.8（pre-merge 红灯分解） |
| `logs/assist/M4-A11-debt-ledger-20260905-2240.md` | 改（+9 行） | D37~D42（集成阻塞/安全红线/门禁缺陷）+ §3 避坑摘要更新 |
| `logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md` | 改（+16 行） | G 组（集成门禁 5 项验收） |
| `logs/checkpoints/A11-M4-verification-batch2-20260906-0629.md` | 新增 | 本文件 |
| `logs/checkpoints/Lane-A11-verification-batch2-20260906-0629.patch` | 新增 | 单补丁 |

生成与校验：
```bash
git add -N logs/checkpoints/A11-M4-verification-batch2-20260906-0629.md
git diff --binary -- \
  logs/checkpoints/M4-A11-verification-matrix-20260905-2240.md \
  logs/assist/M4-A11-debt-ledger-20260905-2240.md \
  logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md \
  logs/checkpoints/A11-M4-verification-batch2-20260906-0629.md \
  > logs/checkpoints/Lane-A11-verification-batch2-20260906-0629.patch
git apply --check -R logs/checkpoints/Lane-A11-verification-batch2-20260906-0629.patch   # 形状校验
```

---

## 8. A0 集成前必须解除的阻塞（按优先级）

1. **D37（最高优先·安全红线）**：`database.rs:939` 的 `execute_batch` 用户 SQL 路径必须改为单语句 `prepare` 通道；`detect_multiple_statements` 在驱动执行前强制拦截。A4 把 `DB_MULTI_STATEMENT_FORBIDDEN` 由 PENDING 转 ACTIVE。
2. **D39（格式）**：责任人跑 `cargo fmt --manifest-path src-tauri/Cargo.toml`。
3. **D40（体积门禁）**：A0 决策——抬阈并说明理由 / `rusqlite` 非 bundled / 延后 `mysql`+`postgres`。
4. **D41（tools 策略自检）**：A0 修 `check-tools-policy.py` 变异样本或临时豁免。
5. （非阻塞但挂账）**D42**（A7 接 `task_*` 后警告回落）、**D30~D36**（R-1~R-11 安全复核项）。

解除后由 A0 复跑 `bash scripts/pre-merge.sh` 至 ALL_PASS，再 `git push origin master`。

---

## 9. 声明

1. **不代修**：D37/D39/D40/D41 均只记录取证 + 归属 + 关闭验证命令，修复权归 A0 指派的责任 lane；A11 零产品代码改动。
2. **不代签**：四批次功能代码落地且 `cargo test` 全绿，但集成门禁红灯 → 本包**不以 PASS 收口**，记为 **`PASS_WITH_DEBT`**（功能可验收 / 集成阻塞未解）。
3. **快照性质**：并行 12 车道持续写入，结论以「命令 + 实测输出 + 取样时刻 2026-09-06 06:29」为准；调度夹具码位在取样期间由 9/8 → 14/6 → 20/0 演进，以最终 20/0 为准。
4. **M4-1.c 矛盾已收口**：原 `STOPPED_CONTRADICTORY_ARTIFACT` 口径作废（A2 batch 重写 COMPLETE），本包 §3.5.1 已同步。
