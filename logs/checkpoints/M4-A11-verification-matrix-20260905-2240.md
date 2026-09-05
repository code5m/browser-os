# M4-A11 验证证据包 · 定向命令矩阵

> Lane：**A11（M4 verification evidence）** ｜ 路由 `AI:BALANCED / R:medium`
> 初版：2026-09-05 22:40 CST ｜ **本次更新：2026-09-05 23:xx CST（并入 A2 的 M4-1.a/b/d 与 M4-1.c STOPPED、A6 的 M4-5 契约现状）** ｜ 作者：CodeBuddy Hy4
> 范围声明：本包**只产出验证日志 / 清单 / 台账，零产品代码改动**；`git push` 归 A0，本车道不提交、不推送。
> 状态：`BASE=e6e09cf`（初版取样点）→ 本次更新开工基线 **`47fce60`**（`git pull --ff-only` → 已是最新，工作树干净）→ 写档期间 HEAD 继续漂移到 **`fb5b78f`**（board 派发实现车道）→ **`a75ba24`**（并入本次前段改动）。**本次更新的取证一律以「命令 + 实测输出」为准，不绑定 SHA 语义**。
> 交付环境：目录 = `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`，分支 `master`，与 `origin/master` 同步（`git pull --ff-only` → `已经是最新的`）。

**姊妹交付物（本包与之对齐，不重复发明）**

- A1：`logs/checkpoints/M4-20260905-2225.md` —— M4 卡展开与依赖图；**§5 测试矩阵 ID 段**（`T-db-c1~c8` / `N-sql-1~16` / `T-db-p1~p12` / `T-db-cmd-1~14` / `T-db-ui-1~N` / `T-sched-c1~c14` / `T-task-1~12` / `T-trig-1~10` / `T-sched-ui-1~N`）与 **§6 违规码位前缀**（`DB_*` / `DBUI_*` / `SCHED_*` / `SCHEDUI_*`）为冻结口径，本矩阵直接引用。
- A2：`logs/checkpoints/M4-1.a-20260905-2245.md`、`M4-1.b-20260905-2250.md`、`M4-1.c-20260905-2255.md`、`M4-1.d-20260905-2300.md` —— 数据库契约四卡，现状见 **§3.5 契约现状看板**。
- A6：`logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md` —— 调度核心契约（M4-5.a/b/c）冻结，现状见 **§3.5 契约现状看板**。
- A10：`logs/assist/A10-M4-security-review-20260905-2240.md` —— 发现 G-1~G-12、§5 门禁增量要求、§6 与既有债务交互；本矩阵 §4 的护栏行给出 G 编号映射。

---

## 1. 启动门禁实跑（WORKSPACE_IDENTITY.md / PARALLEL_COMMAND_BOARD.md 要求）

```bash
cat .workspace-identity            # WORKSPACE_ID=BACKV3_MAIN
pwd                                # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git status --short --branch        # ## master...origin/master（工作树干净）  ✅
git log --oneline -12              # HEAD 侧为并行车道文档提交  ✅
```

判定：**通过**（目录 = 规范主副本、分支 = `master`、工作树干净且脏文件不属于其他车道）。

---

## 2. BASE 证据（本次实跑，非引用历史）

| 项 | 命令 | 实测结果 | 取样点 |
|---|---|---|---|
| Rust 全量单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | `test result: ok. 237 passed; 0 failed; 0 ignored`（7.04 s） | **`47fce60`**（本次更新复跑） |
| Rust 全量单测（更新前） | 同上 | `232 passed; 0 failed` | `3544c09`（初版基线） |
| WBS 路由标签 | `python3 scripts/check-plan-routing.py` | `check-plan-routing: ok (50 WBS rows)`，EXIT=0 | `f376346` |
| 门禁自检 | `bash scripts/pre-merge.sh --self-test` | `SELF_TEST_RESULT=ALL_PASS` | `e6e09cf` |
| 同步远端 | `git pull --ff-only` | `已经是最新的。`（与 `origin/master` 同步） | `47fce60` |
| M4-5.d 调度夹具 | `python3 scripts/check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS: 2 好样本零违规（真实仓库 + 合成参考实现） + 17 个坏样本全部检出（含变异防呆）；ACTIVE=9 PENDING=8`，EXIT=0 | `a75ba24` |
| M4-5.d 调度夹具（默认 / pending） | `python3 scripts/check-scheduler-policy.py`；`… --expect-pending` | 两者 EXIT=0 | `a75ba24` |
| 工作树 | `git status --short --branch` | 干净，`master` | 全程 |

> **232 → 237 的来源**：A2 的 M4-1.b 类型提案（`d6457fe`，`src-tauri/src/domain.rs` +353/+346 行）新增 `T-db-c1~c5` 五个单测。因此 **T0-3 的下限由 232 上调为 237**（见 §3 T0-3）。

引用 A2 / A6 已复跑的证据（同 HEAD 附近，本车道不重复烧机器，A0 集成时复跑）：

- `cargo test m4_1_db_contract` → `5 passed; 0 failed`（A2 `M4-1.d` §9）
- `cargo check --locked` → warning 仍为既有 2 类（`grid_process.rs` 死代码）
- `npm run build` → EXIT=0，主 JS 161.36 kB（未变）
- `bash scripts/pre-merge.sh` → `PRE_MERGE_RESULT=ALL_PASS`

---

## 3. 定向命令矩阵

### T0 · 全车道通用（每次交付前必跑，任一项失败 = 该 lane 不具备送审资格）

| # | 命令 | 期望 | 失败口径 |
|---|---|---|---|
| T0-1 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` | EXIT 0，无 diff | 阻塞 |
| T0-2 | `cargo fmt --manifest-path tauri-browser-tabs/Cargo.toml --all --check` | EXIT 0 | 阻塞 |
| T0-3 | `cargo test --manifest-path src-tauri/Cargo.toml` | `passed ≥ 237`，`failed = 0`（**2026-09-05 23:xx 上调**：M4-1.b 新增 `T-db-c1~c5`） | 阻塞（**计数只增不减**） |
| T0-4 | `cargo check --manifest-path src-tauri/Cargo.toml --locked` | error 0；warning 不增（现状 2 类） | 阻塞 |
| T0-5 | `npm run build` | 0 error；主 JS 无异常增长 | 阻塞 |
| T0-6 | `python3 scripts/measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-<最早>.json --skip-build` | 总体积增长 ≤15%、cargo warning 不增 | 阻塞 |
| T0-7 | `python3 scripts/check-plan-routing.py` | `ok (… WBS rows)`（A1 增卡后行数同步增加） | 阻塞 |
| T0-8 | `bash scripts/pre-merge.sh` | `PRE_MERGE_RESULT=ALL_PASS` | 阻塞 |
| T0-9 | `git diff --check` + `git diff --cached --check` | 输出为空 | 阻塞 |
| T0-10 | `python3 scripts/check-lifecycle-contract.py --self-test` 及默认模式 | `ALL_PASS` / EXIT 0 | **M4-7 落地后必跑**（scheduler 进协调器） |

### T1 · 按车道定向（lane 自检用，A0 集成时逐 lane 复跑）

| Lane | 定向命令 | 期望 | 现状 |
|---|---|---|---|
| A1（卡展开） | `python3 scripts/check-plan-routing.py`；`git diff --stat -- 详细设计与实施计划.md 后续需求TODO.md` | 新卡带 `[S3\|LEVERAGE:n\|COMPLEX\|AI:*\|R:*]` 标签，路由脚本仍 `ok` | 未交付 |
| A2（M4-1 契约） | 文档为主：`python3 scripts/check-plan-routing.py`；`cargo test m4_1_db_contract`（`T-db-c1~c5`）；一次性核查 `grep -rni "jdbc" src-tauri/src src-tauri/Cargo.toml`；A3 引入依赖后 `cargo check --locked` 仍 0 + `git diff Cargo.lock` 人工复核 | 无 JDBC 侧车；`Cargo.lock` 同步入库；`DB_UNDECLARED_RUNTIME` 禁 `tokio/sqlx/diesel` **直接**依赖 | **a/b/d 已交付（契约冻结，b 落 `domain.rs`）；c 处于 STOPPED 且文件状态矛盾，见 §3.5** |
| A3（M4-2/3 后端） | `cargo test database`（覆盖 A1 的 `T-db-p1~p12` / `T-db-cmd-1~14`）；`python3 scripts/check-database-policy.py --self-test` + 默认 + `--expect-pending`；`python3 scripts/check-script-exec-policy.py`（证明未新增第二执行路径） | 全部 EXIT 0；`DB_*` pending 集合与 `PENDING_CODES` 双向一致；`db_*` 三处同步 | 未交付 |
| A4（M4-2.s / M4-3 安全闸门） | `python3 scripts/check-security-policy.py --self-test` + `--expect-current-gaps` + 默认；`python3 scripts/check-database-policy.py --self-test`（`N-sql-1~16` 失败用例为主）；`cargo test security_policy` | gap 集合与威胁矩阵一致（A10 §5-3 要求增 db/task 缺口码）；写操作默认拒绝 | 未交付 |
| A5（M4-4 DB UI） | `node scripts/check-database-ui-logic.mjs`；`python3 scripts/check-database-ui-policy.py --self-test` + 默认；`npm run build` | 断言全过（口径同 `check-terminal-ui-logic.mjs`：加载真实 `.ts`、只 mock bridge）；口令不进持久化 store（A10 G-12） | 未交付 |
| A6（M4-5 契约） | 纯契约：`python3 scripts/check-plan-routing.py`；`bash scripts/pre-merge.sh`；**零产品代码自证** `git diff --stat -- src src-tauri scripts package.json package-lock.json`（应为空）；M4-5.d 若授权则 `python3 scripts/check-scheduler-policy.py --self-test` + 默认 + `--expect-pending` | `TaskDef` / 时钟 / 错过执行 / 取消语义有书面口径；`SCHED_*` pending 集合与 `PENDING_CODES` 双向一致 | **已交付（M4-5.a/b/c = `CONTRACT_FROZEN`，零产品代码；M4-5.d 夹具脚本 `check-scheduler-policy.py` 已落地并接 pre-merge，三模式 EXIT 0，见 §3.5）** |
| A7（M4-6/7 后端） | `cargo test scheduler`（覆盖 `T-task-1~12` / `T-trig-1~10`，含真实进程取证；先落 A6 的 `T-sched-c1~c14` 假时钟单测）；`python3 scripts/check-scheduler-policy.py --self-test` + 默认 + `--expect-pending`；`python3 scripts/check-lifecycle-contract.py --self-test --expect-current-gaps + 默认`；`python3 scripts/check-script-exec-policy.py` | 复用 M2-4 通道；`tasks.json` 原子写 + 损坏 fail-closed（A10 G-8 与 A6 §3.4 统一取 `tasks.json`）；`stop-scheduler` 注册在 `stop-background-workers` 之后、`flush-sessions` 之前（A6 §5.5）；退出后无 timer / 进程残留 | 未交付（**A6 §14 已声明 A7 十项解锁条件全部满足，可放行**） |
| A8（M4-8 调度 UI） | `node scripts/check-scheduler-ui-logic.mjs`；`python3 scripts/check-scheduler-ui-policy.py --self-test` + 默认；`npm run build` | 断言全过（`T-sched-ui-1~N`）；历史排序 / 重试态 / 下次执行时间格式化 | 未交付 |
| A10（安全复核） | **复跑** A2/A3/A4/A7 的全部门禁命令 + 读其检查点 | 与 lane 自报结果一致，偏差需书面说明 | 未交付 |

### T1-a · 新命令「三处同步」通用核查（A3 / A7 落地新命令时必跑）

每个新命令必须同时出现在 `src-tauri/src/bridge.rs`（定义）、`src-tauri/src/main.rs`（`invoke_handler` 注册）、`src-tauri/permissions/default-commands.toml`（ACL）。一键检查：

```bash
for c in db_connect db_query db_disconnect task_list task_add task_update task_remove task_run_now; do
  a=$(grep -c "\"$c\"" src-tauri/permissions/default-commands.toml)
  b=$(grep -c "$c" src-tauri/src/main.rs)
  [ "$a" -ge 1 ] && [ "$b" -ge 1 ] && echo "OK  $c" || echo "MISSING  $c (acl=$a main=$b)"
done
```

期望：全部 `OK`。命令名以 lane 最终命名为准（A2/A6 契约冻结后由 A0 统一替换本列表）。

**2026-09-05 23:xx 更新（来自 A2 `M4-1.b` §3、A6 §6）**：

- 数据库命令实测为 **4 条候选**：`db_connect` / `db_query` / `db_disconnect` + **`db_forget_connection`（凭据吊销入口，A2 建议新增，A0 未裁决 → 债务 `D28`）**。
  - 若 A0 批准：循环列表追加 `db_forget_connection`，并同时校验 `keyring_store.rs` 的 `delete_token` 已移除 `#[allow(dead_code)]` 且有删除路径测试（A10 G-1）。
  - 若 A0 不批准：维持 3 条，A10 G-1 的「断开即吊销」要求降级为 `D28` 挂账，须显式记录。
- 调度命令实测为 **5 条**（A6 §6）：`task_list` / `task_add` / `task_update` / `task_remove` / `task_run_now`；**取消复用 `ScriptProcessTable::cancel`，不新增 `task_cancel`**（A6 §5.4）——若实现里出现 `task_cancel`，视为偏离契约，须回写 BLOCKED。
- ACL 插入位置：A1 F-5 / A2 `DB_ACL_ORDER` 要求 `db_*` / `task_*` 插在 **`list_artifact_images` 之前**（当前 ACL 末条）；核查命令：

```bash
grep -n "list_artifact_images\|\"db_\|\"task_" src-tauri/permissions/default-commands.toml
```

期望：`db_*` / `task_*` 的行号 **小于** `list_artifact_images` 的行号。

### T2 · GUI / 人工（**无真机取证一律记 `NOT_RUN`，不得代签 PASS**）

| # | 命令 | 用途 |
|---|---|---|
| T2-1 | `python3 scripts/m0-6c-gui-regression.py --self-test` | GUI 回归汇总脚本自检（**不启动 GUI**，CI 可跑） |
| T2-2 | `bash run-gui.sh`（或 `bash mvp-start.sh`） | 真机人工验收，逐条走 `logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md` |
| T2-3 | `ps -eo pid,pgid,cmd \| grep mvp-browser-os` | 退出后无残留进程 / 进程组 |
| T2-4 | `cat ~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/audit.json` | 审计取证：**不得出现**密码 / token / 连接串 |
| T2-5 | `bash scripts/collect-m0-baseline.sh`（M0 基线对比） | M4 验收门禁第 5 条要求的 M0 基线对比 |

---

## 3.5 M4 契约现状看板（A11 追踪 · 2026-09-05 23:xx 更新 · 基线 `47fce60`）

### 3.5.0 总表

| 卡 | Lane | 交付物 | 状态 | 产品代码 | 关键验证命令 / 证据 | A11 判定 |
|---|---|---|---|---|---|---|
| **M4-1.a** 依赖与驱动选型裁定 | A2 | `logs/checkpoints/M4-1.a-20260905-2245.md` | `CONTRACT_FROZEN`（选型裁定书） | **0**（`Cargo.toml`/`Cargo.lock`/`build.rs` 零改动） | `git diff --stat` 零；`check-plan-routing.py` ok；`pre-merge.sh` ALL_PASS（a §10） | **可验收。** 裁定：同步栈 `rusqlite`(bundled)+`mysql`+`postgres`，**不引 tokio 直接依赖**；禁 JDBC；不做 `build.rs` 代码生成与 YAML |
| **M4-1.b** `SupportedDb` 与连接配置 schema | A2 | `d6457fe`：`src-tauri/src/domain.rs` +346 行 + `M4-1.b-20260905-2250.md` | `CONTRACT_FROZEN`（类型提案已入库） | **+346 行**（无连接/无 IO/无命令） | `cargo test` **237 passed**（基线 232 + 新增 5）；`cargo test m4_1_db_contract` **5 passed**；`cargo check` warning 仍 2；`npm run build` 161.36 kB 未变；`pre-merge.sh` ALL_PASS（b §6） | **可验收。** 实证收益：`T-db-c1` 当场抓出 `#[serde(rename_all="snake_case")]` 把 `MySql` 序列成 **`my_sql`** 的缺陷（已加显式 rename） |
| **M4-1.c** 结果上限 / 取消 / 截断契约 | A2 | `M4-1.c-20260905-2255.md`（Batch Dispatch 收口版）+ `Lane-A2-M4-1-c-20260905-2318.patch` | **`COMPLETE`（STOPPED 记录已处置，见 §3.5.1）** | **+247 行**（`domain.rs`：`DB_*` 常量 + `DbLimitKind`/`DbQueryState`/`DbValue`/`DbQueryResult` + `T-db-c6~c8`） | `cargo test domain` **29 passed**；`cargo test m4_1_db_contract` **8 passed**（T-db-c1~c8）；`git diff --check` 无输出（A2 §11/§12） | **可验收。** 上限常量已落 `domain.rs`，A3/A4 据此解除阻塞（A2 §10）；R-7（上限常量未落代码）已由本卡关闭 |
| **M4-1.d** 冻结裁定 + 生产判定信号契约 + policy notes | A2 | `M4-1.d-20260905-2300.md` + `M4-1-20260905-2300.patch` | `POLICY_NOTES_DELIVERED`（A0 注记：**只解除 d 部分阻塞，不能证明 c 已完成**） | **0** | 17 条契约汇总（d §1）、A10 准入 4 条对照（d §2，其中「YAML schema 校验」为 N/A 偏离 → `O-A2-5`）、生产判定 S1~S5 与 4 类测试矩阵（d §3）、**14 条 `DB_*` 码位表**（d §5）、交接矩阵（d §4） | **可作为 d 验收**；但不得据此判定 M4-1 整体 PASS |
| **M4-5.a/b/c** 调度核心契约 | A6 | `logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md` | `CONTRACT_FROZEN` | **0**（A6 未改任何产品代码，`domain.rs` 仅作提案写入文档） | `check-plan-routing.py` ok(50)；`git diff --check` 无输出；`pre-merge.sh` ALL_PASS；`git diff --stat -- src src-tauri scripts package.json package-lock.json` 为空（A6 §12） | **可验收。** A6 §14 逐条声明 **A7 的 10 项解锁条件全部满足** → board Wave 2 的 A7 可放行 |
| **M4-5.d** 调度夹具骨架 + pre-merge 接入 | A6 | `scripts/check-scheduler-policy.py`（新建）、`scripts/pre-merge.sh`（+12）、`logs/checkpoints/A6-M4-5.d-20260905-2350.md` | `DELIVERED` | `scripts/` 两处（**控制器逐文件显式授权**，非调度器产品实现） | `--self-test` / 默认 / `--expect-pending` 三模式 EXIT 0；`bash -n scripts/pre-merge.sh` 通过；`pre-merge.sh` ALL_PASS（含「M4-5.d 调度契约不变量夹具」一行）；`git diff --check` 无输出（A6 §4） | **可验收。** 采用「**产物存在才判**」设计：调度产物落地前默认恒绿不阻塞，A7 一提交 `scheduler*.rs` / `tasks*.rs` / `TaskDef` 即自动生效（A6 §2.3 / §6） |

### 3.5.1 M4-1.c：`STOPPED_EMPTY_ARTIFACT` 与实有正文的矛盾（**已于 2026-09-06 收口**）

| # | 事实 | 取证命令 | 结果 |
|---|---|---|---|
| F1 | 文件在 `HEAD`（由 `6d73ce1` 引入）非 0 字节 | `git show 6d73ce1:logs/checkpoints/M4-1.c-20260905-2255.md \| wc -c` | **8549** |
| F2 | 文件前 125 行是完整契约正文（§0 一句话结论、§1 上限数值表、§2 结果 DTO、§3 取消三层、§4 超时分层、§5 多语句通道、§6 测试策略、§7 与 b 的一致性修正、§8 未做/未实测） | 直接读取文件 | 正文齐备 |
| F3 | 第 126~149 行是 A0 追加的「STOPPED」附录，称核查时为 **0 字节空文件** | 直接读取文件 | `STATUS=STOPPED_EMPTY_ARTIFACT`；「不将 M4-1.c 计为 PASS；不移动 NEXT；A2 需重新领取」 |
| F4 | A2 自己把 c 当已成立契约使用：`M4-1.d` §1 汇总表第 12~16 项直接引用 c §1~§5 | 读 `M4-1.d-20260905-2300.md` §1 | 依赖链已建立 |
| F5 | 全仓唯一的 `STOPPED` 记录即此处（`grep -rn "STOPPED" --include="*.md"` 仅命中本文件与 `M4-1.d` 的转述） | `grep -rn "STOPPED" --include="*.md" .` | 无其他 STOPPED 卡 |

**✅ 2026-09-06 收口（A2 `Lane-A2-M4-1-c-20260905-2318.patch`，Batch Dispatch 派工）**：A2 整版重写 `M4-1.c-20260905-2255.md`，**移除**尾部 STOPPED 附录（§9 如实记录「写入与核查的竞态」根因），状态改为 `COMPLETE`；上限常量与 `DbLimitKind`/`DbQueryState`/`DbValue`/`DbQueryResult` **落进 `domain.rs`**（+247 行，T-db-c6~c8 钉死跨模块一致性）。A11 **原 `STOPPED_CONTRADICTORY_ARTIFACT` 口径作废**，`M4-1.c` 计 `COMPLETE`，A3/A4 解除阻塞（A2 §10）。

> **遗留提醒**：指挥板 §Code Dispatch Now「`M4-1.c` remains STOPPED」与 §3.5.4「M4-1.c 自相矛盾」等表述**已过期**，A0 同步更正即可（A12 亦应扫一遍）。`O-A2-5`（YAML 校验 N/A）仍未裁，不影响本卡 PASS。

### 3.5.2 A2 已冻结、且会改写本矩阵期望值的契约点

| 契约点（出处） | 冻结值 | 对本矩阵 / 人工清单的改写 |
|---|---|---|
| 依赖栈（a §3/§6） | `rusqlite`(bundled) + `mysql` + `postgres`；禁 `tokio` **直接**依赖、禁 JDBC | T1-A2 行增加码位 `DB_UNDECLARED_RUNTIME` 期望；A7 亦不得为调度引 tokio runtime（A6 §14-7 已闭环） |
| 结果上限（c §1） | 行 **1000** / 字节 **4 MiB** / 单字段 **64 KiB** / SQL **64 KiB**，**前端不可调大** | 手测 **B-4** 判据改为「1000 行或 4 MiB **先到者**触发，且 `truncated=true` 明示，非静默」 |
| 取消三层（c §3） | L1 标志 / L2 每 64 行检查 / L3 驱动带外中断；取消即无结果，**连接必须归还且可用** | 手测 **B-5** 增加「取消后该连接仍能执行下一条查询」断言 |
| 超时（c §4） | 默认 30s / 上限 600s / soft→hard 宽限 5s；soft=带外取消，hard=**弃连接**（无 SIGKILL 语义） | 手测 C-6 / B-5 判据须写明「与 M2-4 分层同量级但机制不同」，禁止写成「复用 M2-4 超时」 |
| 生产判定（d §3） | 三态 `Production/NonProduction/Unknown`；**无信号 ⇒ `Unknown` ⇒ 拒写**；信号 S1~S5 | 手测 **B-3** 改用 d §3.5 四类输入表（不可解析主机 / 空库名 / 仅名称匹配 / 显式非生产标记） |
| 凭据（b §3） | Keyring 键 `db:<conn_id>`；断连**保留**凭据，`db_forget_connection` 才吊销 | T1-a 命令表 + T2-4 审计快照；`delete_token` 的 `#[allow(dead_code)]` 去留取决于 `D28` 裁决 |
| 平台降级（d §6 / `DB_PLATFORM_DEGRADE`） | 非 Linux 显式 `DB_NOT_SUPPORTED`，禁静默失败 | 人工清单新增：非 Linux 环境启动 → 数据库面板给出显式不支持提示（**当前无可用非 Linux 环境 → 记 `NOT_RUN`**） |
| 不做 YAML / 代码生成（a §4） | `SupportedDb` 手写于 `domain.rs` | A10 G-11 的「YAML schema 校验」为 **N/A 偏离**，需 A0 书面确认（`O-A2-5`）；未确认前 A4 不得自建校验机 |

### 3.5.3 A6（M4-5）契约现状与验证影响

- **状态**：`CONTRACT_FROZEN`，覆盖 M4-5.a（TaskDef）/ b（时钟与可注入 `Clock`）/ c（错过执行、重入、取消）；**产品代码零改动**。**M4-5.d 夹具脚本已实际落地**（`scripts/check-scheduler-policy.py`，工作树未提交，三模式 EXIT 0，证据检查点 `logs/checkpoints/A6-M4-5.d-20260905-2350.md`，见 §3.5.4）；NEXT = `M4-6.a`（A7）。
- **关键裁定**（写进验收判据）：① `TaskDef` **无 `timezone` 字段**（`chrono::Local` 求值）；② cron **仅 5 段**，秒级与宏定义期拒绝；③ `Interval.every_secs` 下界 **60**（对需求原文「every Ns」的保守收窄，`O-A6-5`）；④ 判重真相源 = `tasks.json` 的 `last_fired_at`（`task-runs.json` 只做历史）；⑤ 取消复用 `ScriptProcessTable::cancel`，**不新增 `task_cancel`**；⑥ `stop-scheduler` 注册在 `stop-background-workers` 之后、`flush-sessions` 之前；⑦ **新建任务默认 `enabled=false`**（R-A6-1，采纳 A10 G-7.5 更严口径）；⑧ 执行形态按 A2 的 F-1(b)：`std::thread` + `Condvar`，不引 tokio runtime。
- **对 T1-A7 / 手测 C 组的改写**：`cargo test scheduler` 需覆盖 `T-sched-c1~c14`（**假时钟**，A6 §9 已逐条给出断言）+ `T-task-1~12` / `T-trig-1~10`（真实进程取证）；手测 **C-3**（错过执行）按 `Skip` / `RunOnce` / `CatchUp` 三策略分别验证、**C-4**（时钟回拨）判据为「不追补、`last_fired_at` 不回改、`next_run_at` 从 `now` 重算」、**C-5**（重入）判据为「另一次记 `skipped(reentrant)` 且**不消耗**重试配额」。
- **待决项（非债务，A6 §10）**：`O-A6-2/6/7/10` → A7；`O-A6-4/9` → A8（**cron-tool.html 种子支持 6 位而后端只接受 5 位，UI 必须显式禁用 6 位模式**）；`O-A6-3/5/8/11` → A0。**`O-A6-11` 指出：A10 §G-8 标题仍写 `workspace/tasks.yaml`，与 A1 F-4（`tasks.json`）冲突，建议 A0 订正**，避免后卡按字面引入第二套序列化。
- **既有债务交互**：`D21`（运行记录无 `kind`）/ `D22`（缺 `cmd.run.cancel` / `cmd.run.finish`）由 A6 明确 **M4 不改 M2-4 既有记录结构**，改用 `task-runs.json` 以 `run_id` 关联 → **D21/D22 保持挂账**（台账已同步）。

### 3.5.4 夹具码位落地对照（A2 §5 + A6 §8）

| 脚本 | 码位数 | 默认 / pending | 归属 | 当前状态（2026-09-05 23:xx，工作树） |
|---|---|---|---|---|
| `scripts/check-database-policy.py` | 14（A2 §5：4 默认 + 10 pending） | `DB_CFG_HAS_PASSWORD_FIELD` / `DB_WRITE_DEFAULT_DENY` / `DB_JDBC_SIDECAR` / `DB_UNDECLARED_RUNTIME` + 10 pending | **A4**（事实归 A4，`O-A2-2` 已按此执行） | 🔴 **已落地但自检红灯**（2026-09-06 06:29 实测）：`--self-test` → `DB_SELF_TEST_RESULT=FAIL`（真实仓库存在违规 `DB_MULTI_STATEMENT_FORBIDDEN`）；`--expect-pending` → `DB_PENDING_RESULT=FAIL`（「`DB_MULTI_STATEMENT_FORBIDDEN`:database.rs 使用 execute_batch 处理用户 SQL」）；默认模式 EXIT 0。**根因 = A3 `database.rs` 在用户 SQL 路径用了 `execute_batch`（见 §3.5.8 D37）**。`scripts/pre-merge.sh` 已接（工作树 +行，未提交） |
| `scripts/check-scheduler-policy.py` | 17（A6 §8） | 见 A6 §8.1 / §8.2 | A6（M4-5.d，**已实际落地**） | ✅ **已闭环**（2026-09-06 06:29 实测）：`--self-test` → `SCHED_SELF_TEST_RESULT=PASS: 2 好样本零违规 + 20 个坏样本全部检出；ACTIVE=20 PENDING=0`（EXIT 0）；默认 EXIT 0；`--expect-pending` → `SCHED_PENDING_RESULT=NONE`。S1-2（CRON_MACRO 误报）已被 A6 修复（条纹码位由 14/6 演进到 20/0）。`scripts/pre-merge.sh` 已接为 M4-5.d 项（**当前 pre-merge 该项转绿**） |
| `scripts/check-database-ui-logic.mjs` | —（A5 新增） | headless 断言 `src/utils/dbUi.ts` | A5 | ✅ 119 断言全部通过（A5 §5 / §8 跨天复核一致） |
| `scripts/check-scheduler-ui-logic.mjs` | —（A8 新增） | headless 断言 `src/utils/taskUi.ts` | A8 | ✅ 102 断言全部通过（2026-09-06 06:29 实测） |
| `scripts/check-scheduler-ui-policy.py` | —（A8 新增） | 调度 UI 政策 | A8 | ✅ 默认模式 EXIT 0（2026-09-06 06:29 实测） |

> **A11 判读（更新）**：调度侧夹具已闭环（`M4-5.d` 落地 + 三模式 EXIT 0 + 已接 pre-merge），`O-A6-*` 中关于「M4-5.d 需 A0 授权 scope」的一条事实上已被 A0 放行；数据库侧夹具**仍缺**，按护栏 **G8**，**A3/A4 的产品代码一旦落地而 `check-database-policy.py` 仍未建立 → 判定违反 G8，须挂账**。T1 中数据库脚本相关行在脚本建立前降级为「跑 T0 + 既有脚本，脚本行记 `NOT_RUN（脚本未建）`」。
>
> **pre-merge 项号**：现有 22 项 + 已接入的 M4-5.d 项 = **23 项**；后续 M4 新增从**第 24 项**起（该文件属 A0 / 高冲突文件，lane 不得自行改）。
>
> **在途告警**：`scripts/check-scheduler-policy.py` 仍在被并发修改——本次 23:15 取样为 **`ACTIVE=14 PENDING=6` + 20 个坏样本**（23:0x 取样时为 `ACTIVE=9 PENDING=8` + 17 个坏样本）。**码位数以 A0 集成时刻为准**，本表不再锁死数值。

### 3.5.5 A10 增量复核（R-1~R-11）→ 验证命令映射

> 来源：`logs/assist/A10-M4-security-recheck-A1A2A6-20260905-2345.md`（复核 A1/A2/A6 已合入内容，结论 `PASS_WITH_DEBT`；A6 `A6-M4-5.d-20260905-2350.md` §5 已逐条接受并按「转 A0 / 转 A7 / 转 A11」分工）。
> 债务化结果见台账 **D30~D36**；需 A0 裁决的见台账开放项 **O-A11-1 / O-A11-2**。

| A10 编号 | 严重度 | 验证命令 / 判据 | 归属 |
|---|---|---|---|
| **R-1** 审计冲刷（`audit.json` cap 1000 FIFO 被高频任务冲刷） | 高 | 手测 **F-1** + `cargo test scheduler`；契约侧只需删 `TASK_AUDIT_EVENTS` 两行 | **A0 裁决 → A7 落地** |
| **R-2** `script-runs.json`（cap 200）被定时任务挤出手工记录 | 中高 | 手测 **F-2**；与 D21 合并权衡 | A7 / A11 登记 |
| **R-3** `dangerous=true` 片段可被定时任务引用（手工路径亦无闸门，实证 `bridge.rs:2977-3021`） | 中 | 手测 **F-3**；未裁决前 `NOT_RUN` | **A0 二选一（O-A11-1）** |
| **R-4** `secret` 参数事后改标不复检 | 中 | 手测 **F-4** + `SCHED_SECRET_PARAM_PERSISTED` 判据扩展 | A7 |
| **R-5** 未标 `secret` 的参数明文落盘 `tasks.json`（残余风险） | 中 | 手测 **F-5**（软提示，不硬拦截） | A0 接受 / A8 提示 |
| **R-6** `M4-1.c` 自相矛盾，而 M4-1.d 的上限/取消/超时常量全部引用它 | 中高 | §3.5.1 的 F1~F5 取证；**澄清前 M4-1.d 的上限数值不得作为验收基线** | **A0** |
| **R-7** 上限常量未落代码（`domain.rs` M4 段无任何 `const` / `MAX_*`） | 中 | `cargo test database` + 手测 B-4；建议加「常量缺失即报」码位 | A3 |
| **R-8** `domain.rs:819` 注释声称被不存在的 `check-database-policy.py` 守护 | 低 | `ls scripts/check-database-policy.py` + 注释订正 | A4 |
| **R-9** ACL 条目数将达 114~115，码位须按**末条锚点**判定 | 低 | T1-a 的 ACL 锚点核查命令 | A4（O-A11-2） |
| **R-10** `O-A6-11` 已失效（A10 的 G-8 标题早已修订为 `tasks.json`） | 信息 | 无需动作 | ✅ 已关闭 |
| **R-11** 重试总时长硬上界未冻结（最坏 ≤5400s/任务） | 低 | `cargo test scheduler` + `SCHED_RETRY_UNBOUNDED` | A7 |

### 3.5.6 实现批次落地现状（2026-09-06 06:29 取样）

| 批次（board §Lane A11「Update after」） | 状态 | 落地证据 |
|---|---|---|
| **DB 后端批次 A2/A3/A4** | 🟢 A2 完成 / 🟡 A3+A4 落地但欠清理 | A2：`domain.rs` +247（M4-1.c 常量）、`M4-1.c` COMPLETE；A3：`Cargo.toml/lock` 加 `rusqlite`(bundled)/`mysql`/`postgres`、**`src-tauri/src/database.rs` 已建**（含 `DbPool`/`detect_multiple_statements`/真实 SQLite 测试）；A4：`security_policy.rs` +896（分类器/生产判定/写闸门）、**`scripts/check-database-policy.py` 已建** |
| **DB UI 批次 A5** | 🟢 落地，跨天无回归 | `DatabasePanel.vue` + `useDatabaseStore.ts` + `dbUi.ts` + `check-database-ui-logic.mjs`（119 断言通过）；`npm run build` 0 error |
| **调度后端批次 A6/A7** | 🟢 A6 完成 / 🟡 A7 落地但欠接线 | A6：`check-scheduler-policy.py`（ACTIVE=20 PENDING=0，闭环）；A7：**`src-tauri/src/tasks.rs` 已建**（cron 解析/校验/类型/测试），但**无 `scheduler.rs`、无 `task_*` 命令**（S1 批复核一致） |
| **调度 UI 批次 A8** | 🟢 落地 | `TaskPanel.vue` + `TaskEditDialog.vue` + `useTaskStore.ts` + `taskUi.ts` + `check-scheduler-ui-logic.mjs`（102 断言通过）+ `check-scheduler-ui-policy.py` |
| **A10 安全复核** | 🟢 两批交付 | `A10-M4-security-review-batch-DB-1-20260905-2319.md`（D1-1~D1-5）、`A10-M4-security-review-batch-SCHED-1-20260905-2319.md`（S1-1~S1-5） |

### 3.5.7 本轮实测验证证据（2026-09-06 06:29，工作树 `85d2d7b`，未提交）

| # | 命令 | 结果 | 备注 |
|---|---|---|---|
| 1 | `cargo test`（src-tauri 全量） | **323 passed; 0 failed**（7.06s） | 编译通过；较 09-05 的 237 增 86（M4 测试已并入） |
| 2 | `cargo test domain` | 29 passed | 含 M4-1.b/c 的 T-db-c1~c8 |
| 3 | `cargo test database` | 通过（A3 `database.rs` 测试） | 真实 SQLite 路径已测 |
| 4 | `cargo test scheduler` | 通过（A6/A7 假时钟断言） | ACTIVE 码位已生效 |
| 5 | `cargo test tasks` | 通过（A7 `tasks.rs` cron 校验） |  |
| 6 | `npm run build` | 0 error；主 JS **197.04 kB**（gzip 70.52） | 较 M3.c 基线 161.36 kB **+22.11%**，超 15% 门禁（S1-5/D1-3） |
| 7 | `python3 scripts/check-plan-routing.py` | ok (50 WBS rows) | EXIT 0 |
| 8 | `python3 scripts/check-scheduler-policy.py --self-test` | PASS（20 坏样本全检出）；ACTIVE=20 PENDING=0 | EXIT 0 ✅ |
| 9 | `python3 scripts/check-scheduler-policy.py`（默认） | EXIT 0 ✅ |  |
| 10 | `node scripts/check-database-ui-logic.mjs` | 119 断言通过 ✅ |  |
| 11 | `node scripts/check-scheduler-ui-logic.mjs` | 102 断言通过 ✅ |  |
| 12 | `python3 scripts/check-scheduler-ui-policy.py`（默认） | EXIT 0 ✅ |  |
| 13 | `git diff --check` | 无输出 ✅ |  |
| 14 | `bash scripts/pre-merge.sh` | **PRE_MERGE_RESULT=FAIL（EXIT=1）** | 5 项红灯，见 §3.5.8 |

> 注：`cargo test database` 在 09-05 23:15 曾因他车道在途 `domain.rs` 改动报 E0106，现已编译通过（取数循环/截断/取消相关测试已随 `database.rs` 落地）。

### 3.5.8 集成门禁（pre-merge）现状 —— 🔴 RED（5 项 FAIL，需 A0 指派修复）

| # | FAIL 项 | 根因（取证） | 归属 / 处置 |
|---|---|---|---|
| 1 | `cargo fmt main` | `tasks.rs` 等 M4 文件未格式化（`-+ 断言` 行 diff）。`cargo fmt --check -- src-tauri` 显示 **45 个文件有格式 diff** | 易修：`cargo fmt --manifest-path src-tauri/Cargo.toml`（A0 或责任人跑一次）；**D39** |
| 2 | `build metrics regression` | 主 JS 197.04 kB vs 基线，+22.11% > 15% 门禁（D1-3/S1-5） | **A0 决策**：抬阈值并书面说明 / `rusqlite` 改非 bundled / 延后引 `mysql`+`postgres`；**D40** |
| 3 | `check-tools-policy.py --self-test` | 坏样本「ACL 顺序错误：list_tools 晚于 list_artifact_images」变异失配（按漏检计）。**既有门禁缺陷，非 M4 引入** | A0 修 `check-tools-policy.py` 或临时豁免；**D41**（标注：非 M4 债务，但阻塞集成） |
| 4 | `check-database-policy.py --self-test` | `DB_MULTI_STATEMENT_FORBIDDEN`：真实仓库 `database.rs:939` 用 `execute_batch` 处理用户 SQL，违反 M4-1.c §5（禁 `execute_batch`/`simple_query`/`batch_execute`） | **安全红线 G-4 被实现层违反**；A3 改走 `prepare` 单语句通道 / 或 A4 把策略检测收窄到「用户 SQL 路径」；**D37（最高优先）** |
| 5 | `check-database-policy.py --expect-pending` | 同上：`DB_MULTI_STATEMENT_FORBIDDEN` 在 PENDING 但被检出已实现，应转 ACTIVE | 与 D37 同根；转 ACTIVE 后默认模式方会拦截 |

> **结论**：M4 四批次（DB/UI/Sched 后端+UI）**功能代码均已落地且 `cargo test` 全绿、前端构建/逻辑夹具全绿**，但**集成门禁当前不可过**——卡在 (a) 多语句安全红线被 `database.rs` 违反（D37，最严重）、(b) 二进制体积超门禁（D40）、(c) 格式化与既有 tools 策略自检（D39/D41）。A0 须先解除 D37/D39/D40/D41 四项，pre-merge 方能转绿、方可推送。

---

## 4. 护栏 → 命令可追溯矩阵

| # | 护栏（PARALLEL_COMMAND_BOARD / 详细设计与实施计划 §M4 验收门禁） | 验证命令 | 责任车道 |
|---|---|---|---|
| G1 | 数据库写操作默认拒绝 | A4 新夹具 + `cargo test security_policy` + 手测 B-2 | A4 / A5（UI 确认） |
| G2 | 生产判定不确定 → fail-closed | A4 夹具（生产判定含未知态即拒）+ 手测 B-3 | A4 |
| G3 | 凭据走 Keyring，不进日志 / 前端状态 / 普通文件 / 审计 detail / 检查点 | A4 夹具 + T2-4 审计快照 + 手测 B-7 | A4 / A10 |
| G4 | SQL 结果行 / 字节上限 + 可取消 | `cargo test database` + 手测 B-4 / B-5 | A3 |
| G5 | 调度复用 M2-4 执行通道，禁第二执行路径 | `python3 scripts/check-script-exec-policy.py` + A7 新夹具 + `grep -rn "std::process::Command" src-tauri/src/scheduler*.rs`（应为空） | A7 / A10 |
| G6 | 调度退出经既有生命周期协调器 | `python3 scripts/check-lifecycle-contract.py`（默认）+ `cargo test scheduler` + T2-3 | A7 |
| G7 | 新命令过 source check 并入 ACL | T1-a 三处同步 + lane 新夹具（现有先例：`check-terminal-policy.py` 的 `TERM_ACL_MISSING`、`check-tools-policy.py` 的 `TOOL_ACL_MISSING`） | A3 / A7 |
| G8 | 每条产品代码 lane 必须新增或扩展策略脚本与测试 | A0 集成时核对：新脚本进 `scripts/` 且进 `pre-merge.sh` | A0 |
| G9 | M4 验收门禁第 2 条：SQLite / MySQL / PostgreSQL 各有连接、只读查询、断开与失败用例 | `cargo test database` + 手测 A 组（**环境不备 → 记 `NOT_RUN`，不得签**） | A3 / A11 |
| G10 | M4 验收门禁第 3 条：休眠 / 重启 / 错过执行 / 重复触发 / 系统时间变化 / 任务损坏 / 不并发重入 | `cargo test scheduler` + 手测 C 组 | A7 / A11 |
| G11 | M4 验收门禁第 4 条：定时执行复用 M2-4 并写审计；退出后无 timer / 连接池 / 子进程残留 | T2-3 + T2-4 + 手测 C-6 | A7 / A11 |
| G12 | M4 验收门禁第 5 条：契约 / 集成 / 安全测试及 M0 基线对比全 PASS | T0 全套 + T2-5 | A0 |

**与 A10 发现（G-1~G-12）的映射**（A10 是安全侧独立结论，本表只做编号对齐，不代为裁决）：

| A10 编号 | 对应本表 | 落入的验证命令 |
|---|---|---|
| G-1（Keyring 命名空间冲突 / 吊销路径未用） | G3 | A3 夹具 + 手测 B-7 + T2-4 |
| G-2（`redact_sensitive_url` 覆盖不到非 URL 形态 DSN） | G3 | A4 夹具（DSN 形态坏样本）+ 手测 B-7 |
| G-3（SQL 文本本身是凭据汇，脱敏靠自觉） | G3 | 审计脱敏测试（`bridge.rs:4860` 同款）+ T2-4 |
| G-4（不可解析即拒绝 + 多语句） | G1 | `N-sql-1~16` 失败用例 + 手测 B-9 |
| G-5（生产判定缺信号契约） | G2 | A2/M4-1 契约冻结 + 手测 B-3 |
| G-6（结果上限须在取数循环内生效） | G4 | `cargo test database` + 手测 B-4 |
| G-7（调度是无人值守执行路径，架空意图令牌） | G5 / G11 | A7 夹具（审计带 `origin=scheduler`、禁自我重入、新建默认禁用）+ 手测 C-6 |
| G-8（`tasks.yaml` 明文可执行载荷） | G10 | A7 原子写 + 损坏 fail-closed 全禁用 + 手测 C-7 / C-8 |
| G-9（调度器必须最先停止 + 有界 join） | G6 / G11 | `check-lifecycle-contract.py` + `shutdown.rs` 顺序测试 + T2-3 |
| G-10（8 条新命令的 ACL / 来源校验 / 门禁增量） | G7 | T1-a 三处同步 + `check-security-policy.py` 增 db/task 缺口码 |
| G-11（代码生成与依赖引入 = 供应链面） | G12 | `cargo check --locked` + `git diff Cargo.lock` 人工复核 + T2-5 |
| G-12（前端口令不得进持久化 store） | G3 | `check-database-ui-policy.py` + 手测 B-7 |

---

## 5. 新增门禁脚本的接入约定（给 A3 / A4 / A5 / A7 / A8）

沿用 `scripts/GATE-CONTRACT.md` 与既有 20+ 脚本的口径，避免 M4 期间出现第二种脚本格式：

1. **参数面**：无参 = 正式门禁（EXIT 0/1）；`--help` EXIT 0；`--self-test`（fixture 自检，EXIT 0/1）；非法参数 EXIT **2**。
2. **未实现码位**：用 `PENDING_CODES` + `--expect-pending` 模式（先例：`check-script-exec-policy.py`、`check-command-domain-policy.py`、`check-script-ui-policy.py`）。默认模式与 `--expect-pending` 模式**都要**接入 `pre-merge.sh`，防止「有 pending 码位已实现却仍挂账」。
3. **变异防呆**：`--self-test` 必须含「坏样本使对应不变量失败」的用例，坏样本 token 必须**不含原串子串且全量替换**；`mutate(**kw)` 的键名必须与 `read_repo` 的键名完全一致（历史踩坑：写成 `acl=` 而实际键是 `acl_toml=`）。
4. **零新增依赖**：Python 用标准库；前端 `.mjs` 用 Node 原生，直接 import 真实 `.ts`，只 mock bridge。
5. **接入位置**：`scripts/pre-merge.sh` 现有 22 项，M4 新增从**第 23 项**起顺序追加，并同步 `usage()` 帮助文本与 `run_self_test()` 存在性检查（该文件属 A0 / 高冲突文件，lane 不得自行改，交 A0 统一接入）。
6. **证据落盘**：每条 lane 的结论进 `logs/checkpoints/`；过程性材料进 `logs/assist/`。

7. **脚本名与码位前缀以 A1 `logs/checkpoints/M4-20260905-2225.md` §6 的冻结值为准**：`check-database-policy.py`（`DB_*`）、`check-database-ui-logic.mjs`、`check-database-ui-policy.py`（`DBUI_*`）、`check-scheduler-policy.py`（`SCHED_*`）、`check-scheduler-ui-logic.mjs`、`check-scheduler-ui-policy.py`（`SCHEDUI_*`）。本矩阵 T1 行已按该命名书写；若 lane 落地时改名，需同步 A1 §6 与本矩阵，由 A0 裁决。

---

## 6. A0 集成前的收口顺序（A11 视角）

按 board 的 merge order 逐条收口，每收一条 A0 跑一次 **T0 全套 + 该 lane 的 T1**：

1. A1（卡展开与依赖图） → 2. A2 / A6（契约冻结） → 3. A4（安全闸门） → 4. A3（后端核心） → 5. A7（调度后端） → 6. A5 / A8（UI） → 7. A10（安全复核） → 8. **A11 复跑全矩阵（T0 + 全部 T1 + T2-1 / T2-3 / T2-4）** → 9. A0 push。

**2026-09-05 23:xx 进度（基线 `47fce60`）**

| 步 | 车道 | 进度 |
|---|---|---|
| 1 | A1 | ✅ 已集成（`6d73ce1`） |
| 2a | A2 | ⚠️ **a / b / d 已集成；c 为 `STOPPED_CONTRADICTORY_ARTIFACT`，需 A0 先裁定**（§3.5.1） |
| 2b | A6 | ✅ 已集成（M4-5.a/b/c `CONTRACT_FROZEN`）；**M4-5.d 夹具脚本已落地**（工作树未提交，见 §3.5.4） |
| 3~7 | A3 / A4 / A5 / A7 / A8 / A10 | 🟡 board（2026-09-05 23:55 版）已切到「**batch implementation mode**」；A10 基线评审已集成，并新增 `logs/assist/A10-M4-security-recheck-A1A2A6-20260905-2345.md` |

**A0 下一步最优先的三件事（A11 视角）**：① 裁定 M4-1.c 的矛盾状态（阻塞 A3/A4 的一手依据）；② 裁决 `O-A2-1`~`O-A2-5` 与 `O-A6-3/5/8/11`（其中 `O-A2-2` 决定 `check-database-policy.py` 归属、`O-A6-11` 需订正 A10 §G-8 的 `tasks.yaml` 标题）；③ 提交已在工作树就位的 `scripts/check-scheduler-policy.py` + `scripts/pre-merge.sh` 第 23 项（两者均已存在但未提交，属 A0 归属）。

A11 在第 8 步提交终态证据：**每条命令 + 原文结果 + HEAD SHA + 环境指纹**；`NOT_RUN` 项必须写明缺什么环境，不得留空、不得填 PASS。

---

## 7. 判定口径（沿用既有纪律，不新发明）

- 证据三要素：**命令 + 结果 + 环境指纹**；三者缺一不构成证据。
- `M0_RUN_MODE=smoke` 的状态固定为 `EXPLORATORY`，**永不输出 PASS**（`scripts/GATE-CONTRACT.md` §1）。
- GUI 类用例未目视验收的一律挂账（`D20` / `D23` 口径），**不伪造 PASS、不代签**。
- 单测计数只增不减；若必须删测试，需在检查点写明替代覆盖。
- `cargo check` warning 现状为既有 2 类（`grid_process.rs`），M4 不得新增第 3 类。

---

## 8. 未覆盖声明与风险

1. 本包**不修改** `scripts/pre-merge.sh`（A0 归属）与任何产品代码；新增项只给编号建议。
2. 本机**未起 MySQL / PostgreSQL 实例**，G9 相关用例在 A11 阶段只能落到「人工清单 + `NOT_RUN`」，需 A3 或人工环境备齐后补。
3. Linux 下 Keyring 依赖 `secret-service`；若验收机无 gnome-keyring/kwallet，G3 的手测会表现为「凭据无法保存」，须先确认守护进程，否则该条记 `NOT_RUN` 而非 FAIL。
4. 并行期间 HEAD 频繁漂移，本包所有结论以「命令 + 实测输出」为准，不绑定具体 SHA 的语义；A0 集成时按当时 HEAD 重跑 T0。
5. A11 交付物为工作树中的新增文件（位于 `logs/**`），**不构成对其他车道的脏工作树阻塞**；其他 lane 的启动门禁应把 `logs/**` 未跟踪文件视为机器证据（口径同 `pre-merge.sh` 对 `logs/m0-baseline/**` 的排除）。
6. **本包不代为裁决**：M4-1.c 的矛盾状态、`O-A2-*` / `O-A6-*` 开放项、`D28` 的命令增删，全部只做「事实记录 + 取证命令 + 影响面」，结论权归 A0。
7. A11 本次**未复跑** `npm run build` / `bash scripts/pre-merge.sh` / `cargo check --locked`（避免重复烧机器，且工作树中 A0 的 `pre-merge.sh` 改动与新增脚本尚未提交，此时复跑会混入在途改动）；相关结果引用 A2 `M4-1.d` §9 在同 HEAD 附近的实测输出；**A0 集成时必须自行复跑**（T0-4 / T0-5 / T0-8）。本次实际复跑：`cargo test`（237 passed）、`python3 scripts/check-plan-routing.py`（ok 50）、`python3 scripts/check-scheduler-policy.py` 三模式（均 EXIT 0）、`git diff --check`（0）。
8. 本次更新只改 `logs/checkpoints/M4-A11-verification-matrix-*.md` 与 `logs/assist/M4-A11-debt-ledger-*.md`；**未触碰任何产品代码、未改 `scripts/`、未改三份主文档、未提交、未 push**。
