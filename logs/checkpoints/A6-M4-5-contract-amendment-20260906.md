# A6 · M4-5 契约 doc 增补提案（R-A6-2 / R-A6-3 / SCHED_DEDUP_NOT_LAST_FIRED）

> 提案者：Lane A6（CodeBuddy 会话，Hy4 / 腾讯混元）
> 依据：`logs/checkpoints/M4-5.d-20260905-2355.md` §10.1
> 提交时间：2026-09-06 11:25 CST
> 性质：**A0 套用提案**——A6 不越权改高冲突契约 doc
> `A6-M4-5-scheduler-contract-20260905-2330.md`（已 M 在工作树；M4-5.d 阶段 §6/§7/§8 由 A6 自改
> 属 A10 R-1 红线，详见该 doc §7「修订 A-1」；本提案针对 §4.4 / §5.1 / §5.2，**不**自改）
> 目标读者：A0 集成者（M4-5 final 集成时一并审）
> 适用版本：`master @ 85d2d7b7` + M4-5.d + M4-5.e 已落

---

## 0. 提案总览

| # | 提案编号 | 来源 | 插入位 | 机器可检码位 | 紧迫性 |
|---|---|---|---|---|---|
| 1 | **R-A6-2** | M4-5.e §10.1（O-A7-1） | 契约 doc §4.4 表格**末尾**新增一行 | `SCHED_CRON_DOM_DOW_UNION`（ACTIVE，见 M4-5.e §2.1） | 中（A7 已按裁定实现） |
| 2 | **R-A6-3** | M4-5.e §10.1（O-A7-2） | 契约 doc §5.1 与 §5.2 之间**新增 §5.1.1** | `SCHED_OCCUPIED_NOT_SKIPPED`（ACTIVE） | 中（A7 已按裁定实现） |
| 3 | **`SCHED_DEDUP_NOT_LAST_FIRED` 语义** | M4-5.e §10.1 | 契约 doc §3.2 `last_fired_at` 行**下方新增引用段** | `SCHED_DEDUP_NOT_LAST_FIRED`（ACTIVE） | 高（防 A7 误用 `task-runs.json` 判重） |

> **A0 决策点**：三条是否同时落，或先落 #3（防误用最高），或先落 #1/#2 与 A7 实现同步验收。
> A6 推荐**先落 #3**（纯结构补强，不动 A7 现有代码），**再落 #1/#2**（在 A7 落地的代码上做契约对齐）。

---

## 1. 提案 R-A6-2（cron DOM × DOW 组合规则）

### 1.1 现状（契约 doc §4.4 末段）

§4.4 当前只列 5 段语法元素（`*`/`*/n`/`a-b`/`a,b`/字面量），**没有**说明「day-of-month
与 day-of-week 同时非通配时如何组合」。语义空白会导致 A7 实现与种子工具
`cron-tool.html` 行为分歧：种子工具走 Vixie OR（任一字段命中即触发），后端若不
明示则隐式按 OR——属放大行为，与 A6「触发更少 = fail-closed」原则相反。

### 1.2 提案新增（§4.4 表格末尾追加一行）

> **DOM × DOW 组合**（A6 补裁定）：当 `day-of-month` 与 `day-of-week` **同时**含非通配
> 字段时，取**交集（AND）**——必须同时满足才触发。
>
> **理由**：触发更少 = fail-closed（「每月 1 日且周一」若按 Vixie OR 会在每周一都触发，
> 属放大）；Vixie 行为并非绝对标准，POSIX cron 不强制 OR；M0-4 依赖清理口径强调
> 「不引入 `cron` crate」即已脱离 Vixie 生态，无义务兼容其默认行为。
>
> **影响**：A7 `matches_naive` 实现中必须按 AND 解释；M4-8 UI 须在 cron 编辑器
> 标注「DOM 与 DOW 同时设置时取交集」；种子工具 `cron-tool.html` 的 6 位模式下若
> 也走 Vixie OR，则**前后端语义不一致**——已在 O-A6-4 中登记 M4-8 须显式标注 / 禁用
> 6 位模式（与本裁定互不冲突）。
>
> **机器可检**（`SCHED_CRON_DOM_DOW_UNION`）：
>
> ```python
> # scripts/check-scheduler-policy.py 已实现
> # 真实仓库变异样本：scheduler.rs / tasks.rs 内出现 OR 语义字符串 / Vixie 引用 → FAIL
> ```

### 1.3 不需要修改的位置

- §3.2 字段语义（无字段变化）
- §3.3 定义期校验（无新校验项）
- §4.5 系统时间变化（无影响）

---

## 2. 提案 R-A6-3（plan_slots 层的 in_flight 占用裁决）

### 2.1 现状（契约 doc §5.1 + §5.2 间隙）

| 位置 | 现状 | 缺口 |
|---|---|---|
| §5.1 主流程 | "每次触发前依次检查：① stop ② 本任务 in_flight ③ 全局并发" | **执行前**判占用；没说**计划层**要不要做 |
| §5.2 第一行 | "本任务 `in_flight` 非空 → reentrant → 跳过且不排队" | **执行后**裁决；没区分"plan_slots 生成时已知"与"启动时才发现" |
| §5.1 CatchUp 段 | "if fired < catch_up_limit { 触发(slot, trigger=CatchUp); fired += 1 }" | 若每个 slot 都先在 plan_slots 滤掉占用，则 fired 自然 ≤ 1，**CatchUp 一轮多触发与占用跳过不可兼得** |

### 2.2 提案新增（§5.1 与 §5.2 之间插入 §5.1.1）

> **§5.1.1 plan_slots 层的 in_flight 占用裁决**（A6 补裁定 R-A6-3）：
>
> 1. `plan_slots`（即 §5.1 中的 `slots = 生成 (anchor, now] 内的全部计划触发点`）在生成时
>    **必须**先查本任务当前 `in_flight`：若非空，**该 slot 直接记 `SKIP_REENTRANT`** 进入
>    `task.run.skipped(reason=reentrant)`，**不**进入 §5.1 的触发循环。
> 2. 由此 §5.1 的 `fired` 计数**自然**仅统计「未被 plan_slots 滤掉的 slot 触发次数」，
>    配合 §5.2「占用则跳过且不排队」形成**双重保险**：plan_slots 层 + 执行前查 in_flight。
> 3. **关键不变量**（A6 新增）：
>    - 占用跳过**不**消耗 `catch_up_limit` 配额；
>    - 占用跳过**不**消耗 `retry` 配额（与 §5.2 既有「❌ 不消耗」一致）；
>    - 占用跳过**不**把该 slot 算作 missed（missed 仅由 §5.1 lateness > misfire_grace_secs 触发）。
> 4. **与 §5.1 CatchUp 的协作**：A7 `plan_slots` 必须先做 in_flight pre-filter，再做
>    `fired < catch_up_limit` 截断；二者顺序固定（O-A7-2 偏离契约初版的根因）。
>
> **理由**：把判占用前移到 plan_slots 层有三点收益——
> ① 减少 §5.1 主循环内每次查 in_flight 的重复 IO（O(1) → O(slots) 摊销）；
> ② 避免 CatchUp 一轮触发多个 slot 时「触发到第 N 个才发现占用」回滚复杂度；
> ③ 与 §5.4 `task_run_now` 的「同样受同任务 in_flight 约束」天然对齐——`plan_slots`
>    不分 Manual/Scheduled 共用同一条路径。
>
> **机器可检**（`SCHED_OCCUPIED_NOT_SKIPPED`）：
>
> ```python
> # scripts/check-scheduler-policy.py 已实现
> # 真实仓库变异样本：
> #   1) scheduler.rs / tasks.rs 出现"排队"语义（queue/enqueue/wait_list） → FAIL
> #   2) 占用路径不写 SKIP_REENTRANT 字样 → FAIL
> #   3) plan_slots 实现里 in_flight 检查在 catch_up_limit 之后 → FAIL
> ```

### 2.3 不需要修改的位置

- §5.1 主流程伪码（保留 §5.1.1 的 pre-filter 后，§5.1 内每次触发前再查 in_flight 仍属「双重保险」不冲突）
- §5.2 表格（同 §5.1 兼容；表格里"本任务 in_flight 非空"对应"plan_slots 已标 SKIP_REENTRANT"或"运行时态 in_flight 又起"两种来源）
- §5.4 取消语义（无影响）

---

## 3. 提案 `SCHED_DEDUP_NOT_LAST_FIRED` 语义升级

### 3.1 现状（契约 doc §3.2 `last_fired_at` 行）

§3.2 已写明「用 `last_fired_at` 判重」，**但这是字段语义层**。A7 实现时可能
出现：

- A7-1（合规）：判重完全走 `tasks.json::last_fired_at`，与 `next_run_at` 同一次
  原子写。
- A7-2（违规）：`tasks.json` 内**仅**存 `last_fired_at` 字段，但调度判定函数
  实际从 `task-runs.json` 读最近一条 `scheduled_at` 做判重——`task-runs.json`
  走环形裁剪（cap 500）后旧 key 丢失，重启或长时间空闲的任务会**重复执行**。

§3.2 字面无法区分 A7-1 与 A7-2。需要把判重路径**结构化**为契约。

### 3.2 提案新增（§3.2 `last_fired_at` 行下方新增引用段）

> **§3.2.1 判重真相源结构化约束**（A6 补裁定）：
>
> 1. 调度判定模块（`scheduler.rs` 与 `tasks.rs` 内对 `next_fire_after` / `collect_missed_slots`
>    / `is_missed` / `should_fire` 四个函数及任何同义实现）的产品代码中**必须出现**
>    `last_fired_at` 标识符引用（字段名 / 局部变量 / 函数参数 / `tasks.json` 路径中含
>    `last_fired_at` 子串）——这是「判重真相源 = `tasks.json` 的 `last_fired_at`」的
>    **结构性**判定。
> 2. 反向禁止：上述函数内**不得**调用 `task-runs.json` 读取函数（按 F-A6-4，
>    `task-runs.json` 仅 UI 历史与排障）；不得出现 `idempotency` 字样（该字段在
>    F-A6-4 中已删除）。
> 3. 崩溃恢复：在 §5.1 主流程的「`if slot <= last_fired_at: continue`」之前，
>    必须先 `load_tasks()` 一次（而非 `load_task_runs()`），把 `last_fired_at` 装入
>    局部变量；不允许「`task-runs.json` 没有该 slot 的记录就认为未触发」的隐式
>    兜底路径。
>
> **理由**：把契约从「字段语义」升级为「代码结构」后，机器门禁可直接 grep 函数体
> 内部引用，无需解释语义；防 A7 在审查时挂「`last_fired_at` 字段在 `TaskDef` 中
> 存在」应付而实际走 `task-runs.json` 路径。
>
> **机器可检**（`SCHED_DEDUP_NOT_LAST_FIRED`）：
>
> ```python
> # scripts/check-scheduler-policy.py 已实现
> # 真实仓库变异样本：
> #   1) 上述四函数体内不出现 last_fired_at → FAIL
> #   2) 上述四函数体内出现 task-runs.json 路径或 load_task_runs 调用 → FAIL
> #   3) 上述四函数体内出现 idempotency 字样 → FAIL
> ```

### 3.3 不需要修改的位置

- §3.2 `last_fired_at` 字段语义行（保留）
- §5.1 主流程（保留「`if slot <= last_fired_at: continue`」不变；本提案只是把
  真相源结构化，行为本身已在 §5.1 体现）

---

## 4. A0 套用指令（最小动作清单）

### 4.1 提案 #3（`SCHED_DEDUP_NOT_LAST_FIRED` 语义）——推荐先落

```bash
# 1. 在 §3.2 表「last_fired_at」行下方插入「### 3.2.1 判重真相源结构化约束」
#    全文见本文件 §3.2（含三个要点 + 理由 + 机器可检段）
# 2. 跑门禁验证不漂移：
python3 scripts/check-scheduler-policy.py --self-test
python3 scripts/check-scheduler-policy.py
# 3. 提交
git add logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md
git commit -m "docs(scheduler): 强化 last_fired_at 判重真相源为结构化契约（A6 amend R-3）"
```

### 4.2 提案 #1（R-A6-2 cron DOM × DOW）——与 A7 落地代码同步验收

```bash
# 1. 在 §4.4 表格末尾追加「DOM × DOW 组合」行
#    全文见本文件 §1.2
# 2. 验证：A7 的 matches_naive 必须按 AND 解释 DOM × DOW
#    真实仓库变异样本在 check-scheduler-policy.py 已有；
#    若 A7 实现为 OR，会被 SCHED_CRON_DOM_DOW_UNION 检出
# 3. 与 A7 沟通：是否需要 UI 端调整（M4-8）
```

### 4.3 提案 #2（R-A6-3 plan_slots in_flight）——与 A7 落地代码同步验收

```bash
# 1. 在 §5.1 与 §5.2 之间插入 §5.1.1
#    全文见本文件 §2.2
# 2. 验证：A7 的 plan_slots 必须先做 in_flight pre-filter 再做 catch_up_limit 截断
# 3. 真实仓库变异样本在 check-scheduler-policy.py 已有
```

### 4.4 套用后必跑

```bash
python3 scripts/check-scheduler-policy.py --self-test   # exit 0
python3 scripts/check-scheduler-policy.py                # exit 0
python3 scripts/check-scheduler-policy.py --expect-pending  # exit 0
git diff --check -- logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md
bash scripts/pre-merge.sh
```

---

## 5. 不在本提案范围（A6 主动收窄）

| 项 | 理由 | 归属 |
|---|---|---|
| §3.4 持久化的 `atomic_write` 引用方式 | M4-5.d 已有 `SCHED_PERSIST_NOT_ATOMIC` ACTIVE 守护 | 不动 |
| §4.2 时区（`chrono::Local`） | 已冻结（`Cargo.lock` 零 `chrono-tz` 命中） | 不动 |
| §5.3 重试语义 | M4-5.d 已有 `SCHED_RETRY_UNBOUNDED` / `SCHED_CATCHUP_UNBOUNDED` 双码守护 | 不动 |
| §6 DTO 与 ACL | A7 已实现，机器门禁 `SCHED_CMD_NOT_REGISTERED` / `SCHED_ACL_ORDER` 守护 | 不动 |
| M4-8 UI 联动 | 6 位模式禁用 / DOM × DOW 标注均归 M4-8 | 移交 A8（不属 A6） |

---

## 6. A6 自续证据链

| 时间 | 文件 | 状态 |
|---|---|---|
| 2026-09-05 23:30 | `A6-M4-5-scheduler-contract-20260905-2330.md` | M（§6/§7/§8 由 M4-5.d 改，§0 自我收窄） |
| 2026-09-05 23:55 | `M4-5.d-20260905-2355.md` | 已落（untracked），§10.1 列三条 A0 待办 |
| 2026-09-06 07:04 | `check-scheduler-policy.py`（1378 行） | 已落（untracked），含 R-A6-2/3 + `SCHED_DEDUP_NOT_LAST_FIRED` 三个码位 |
| 2026-09-06 11:25 | `M4-5.d-20260905-2355.md` §10.8 | 本次新增（集成窗口实测快照） |
| 2026-09-06 11:25 | `A6-M4-5-contract-amendment-20260906.md` | 本文件（A0 套用提案） |
| 2026-09-06 11:25 | `Lane-A6-M4-5final-20260906.md` + `.patch` | 本次新增（A6 final 收口） |
