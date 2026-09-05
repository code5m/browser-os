# M4-A11 验证证据包 · 定向命令矩阵

> Lane：**A11（M4 verification evidence）** ｜ 路由 `AI:BALANCED / R:medium`
> 初版：2026-09-05 22:40 CST ｜ **本次更新：2026-09-05 23:xx CST（并入 A2 的 M4-1.a/b/d 与 M4-1.c STOPPED、A6 的 M4-5 契约现状）** ｜ 作者：CodeBuddy Hy4
> 范围声明：本包**只产出验证日志 / 清单 / 台账，零产品代码改动**；`git push` 归 A0，本车道不提交、不推送。
> 状态：`BASE=e6e09cf`（初版取样点）→ 并行期间 HEAD 漂移 `f376346` → `3544c09` → **本次更新基线 `47fce60`**（A0 已把 A1 / A2（M4-1.b 的 `domain.rs` +346）/ A6 / A10 / A11 文档集成入 `master`；`git pull --ff-only` → 已是最新，工作树干净）。

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
| A6（M4-5 契约） | 纯契约：`python3 scripts/check-plan-routing.py`；`bash scripts/pre-merge.sh`；**零产品代码自证** `git diff --stat -- src src-tauri scripts package.json package-lock.json`（应为空）；M4-5.d 若授权则 `python3 scripts/check-scheduler-policy.py --self-test` + 默认 + `--expect-pending` | `TaskDef` / 时钟 / 错过执行 / 取消语义有书面口径；`SCHED_*` pending 集合与 `PENDING_CODES` 双向一致 | **已交付（M4-5.a/b/c = `CONTRACT_FROZEN`，零产品代码；M4-5.d 夹具骨架待 A0 授权，见 §3.5）** |
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

### T2 · GUI / 人工（**无真机取证一律记 `NOT_RUN`，不得代签 PASS**）

| # | 命令 | 用途 |
|---|---|---|
| T2-1 | `python3 scripts/m0-6c-gui-regression.py --self-test` | GUI 回归汇总脚本自检（**不启动 GUI**，CI 可跑） |
| T2-2 | `bash run-gui.sh`（或 `bash mvp-start.sh`） | 真机人工验收，逐条走 `logs/assist/M4-A11-gui-manual-checklist-20260905-2240.md` |
| T2-3 | `ps -eo pid,pgid,cmd \| grep mvp-browser-os` | 退出后无残留进程 / 进程组 |
| T2-4 | `cat ~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/audit.json` | 审计取证：**不得出现**密码 / token / 连接串 |
| T2-5 | `bash scripts/collect-m0-baseline.sh`（M0 基线对比） | M4 验收门禁第 5 条要求的 M0 基线对比 |

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
