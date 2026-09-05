# A10 · M4 已合入内容安全复核（A1 / A2 / A6）

> Lane: `A10` — M4 security review（`AI:DEEP / R:xhigh`）
> 时间: 2026-09-05 23:45 CST
> Base: `47fce60`（`master`）
> 范围: **只读安全复核**。只写 `logs/assist/A10-*.md`，**零产品代码改动、零提交、零 push**
> 复核对象: A1 展开卡（`M4-20260905-2225.md`）、A2（`M4-1.b` domain.rs / `M4-1.c` / `M4-1.d`）、A6（`M4-5` 调度契约）
> 复核红线: 隐私 / 凭据 / 写权限 / 无限增长 / source check 与 ACL

---

## 1. 启动门禁取证（实测）

```text
cat .workspace-identity  → WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master   ✅
pwd                      → /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git pull                 → 已经是最新的。                                        ✅
git status --short --branch → ## master...origin/master（无 ahead/behind，无脏文件）✅
git log --oneline -3     → 47fce60 docs(M4-1.d): record database policy notes
                           d6457fe feat(M4-1.b): add database domain contract types
                           6d73ce1 docs(M4): integrate parallel lane checkpoints
```

门禁**全项通过**：规范主目录、`master`、工作树干净、与远程同步、`git pull` 无新提交。

---

## 2. 结论摘要

| 复核对象 | 裁定 | 说明 |
|---|---|---|
| **A2 / M4-1.b** `domain.rs` 类型提案 | **PASS** | 隐私与写权限两条红线**结构性**落实，超出预期（见 §4.1） |
| **A2 / M4-1.d** policy notes | **PASS_WITH_DEBT** | 信号契约质量高；但所引用的上限常量源自状态自相矛盾的 `M4-1.c`（R-6） |
| **A6 / M4-5** 调度契约 | **PASS_WITH_DEBT** | 设计严谨、主动采纳更严口径；但存在**审计冲刷**（R-1，高）与**危险片段无人值守放大**（R-3） |
| **A1** 展开卡 | **PASS**（本轮仅复核其被下游引用的裁决） | F-1/F-4/F-5/F-7 等事实修正经我独立复跑，均成立 |

**总体：`PASS_WITH_DEBT`**。共 **11 条发现**（R-1 ~ R-11），其中 **1 条【高】**、**4 条【中高/中】** 需 A0 处置，其余为低风险或信息项。
**未发现任何凭据明文落盘、隐私字段泄漏、source check 缺失或 ACL 漏登记。**

---

## 3. 复核方法

- 逐行读 `M4-1.b`(173 行)、`M4-1.c`(149 行)、`M4-1.d`(202 行)、`A6-M4-5`(616 行)、`M4-20260905-2225.md` 关键段。
- 对 A2 新增 `domain.rs` 段（`src-tauri/src/domain.rs:813-1009` 与单测 `1152-1260`）逐行审读。
- **所有关键结论回源码验证**，不采信文档自述：`workspace.rs:388`（审计 cap 1000）、`bridge.rs:2977`（`run_command` 无确认闸门）、`domain.rs:623`（`ScriptParam.secret`）、`script_runner.rs:46-48`（超时常量）。
- **未**运行 `cargo test` / `npm run build`：本轮零产品代码改动；A2 已交 `cargo test` 237 passed（基线 232 + 5）、pre-merge `ALL_PASS`，我采信其**自报**并标注为未经我复跑（见 §8 声明）。

---

## 4. 逐卡复核

### 4.1 A2 / M4-1.b —— `domain.rs` 类型提案【PASS】

三条安全红线**结构性**落实，不是靠约定：

| 红线 | 落实方式 | 证据 |
|---|---|---|
| **凭据** | `DbConnectionConfig` **类型上不存在** password/dsn/connection_string 字段；凭据只经 Keyring，键 `db:<conn_id>` | `domain.rs:905-941`；单测 `T-db-c3` 断言序列化键不得含 `password/passwd/pwd/secret/token/dsn/conn_str/uri/url` |
| **写权限** | `allow_write` 带 `#[serde(default)]` → 缺省 `false`（缺字段即无写权限）；`DbSslMode` 缺省 `Prefer` | `domain.rs:931-933`、`899-903`；单测 `T-db-c4` 断言历史 JSON 缺字段落在更严一侧 |
| **生产判定** | `production_hint: Option<bool>`，`None` = 无信号（不放行） | `domain.rs:934-936` |

**这是我上一轮 G-1 / G-2 的最强落地形态**：我原本只要求「凭据以类型化字段流转、不序列化」，A2 直接做到「字段根本不存在」+ 单测锁死，优于我的要求。
`from_kind_str` 亦为 fail-closed（未知/大小写变形/前后空格一律 `None`，`domain.rs:856-863`）。

**未新增任何 `#[tauri::command]`**（`git show d6457fe -- domain.rs | grep -c tauri::command` → **0**），故本卡**不涉及** ACL / source check 面。✅

### 4.2 A2 / M4-1.d —— policy notes【PASS_WITH_DEBT】

- **生产判定信号契约（§3）质量高**，且**正确实现了我 G-5 的核心要求**：规则 4「无任何信号 → `Unknown`（不是 NonProduction）」= 无信息时按生产处理、拒绝写。
- 失败序设计正确：`S2`（名称命中 prod 词表）→ **Unknown** 而非 Production，即名称启发式**永不单独放行**；且 `S2` 命中优先于 `S2'`/`S3` 的 NonProduction 候选（规则 2 先于规则 3），`prod-dev.corp` 这类会落在 Unknown。✅
- 码位表（§5）覆盖了 `DB_CRED_IN_AUDIT`、`DB_RESULT_LIMIT_MISSING`、`DB_ACL_ORDER`、`DB_PERSIST_NOT_ATOMIC` 等我上一轮提出的点。

**债务**：本卡 §1 第 12~16 行（结果上限 / 截断 / 取消 / 超时 / 多语句）**全部引用 `M4-1.c`**，而 `M4-1.c` 状态自相矛盾 —— 见 **R-6**。

### 4.3 A6 / M4-5 —— 调度契约【PASS_WITH_DEBT】

A6 主动采纳了更严口径，应予肯定：

| 我的上一轮建议 | A6 处置 | 证据 |
|---|---|---|
| G-7.5「新建任务默认 `enabled=false`」 | **采纳**，且给出「自动执行 vs 列表可见性」不同险的论证，并存量为缺字段亦按 false | `A6 §3.5 裁定 R-A6-1` |
| G-9「scheduler 停止须注册在 `kill-running-scripts` 之前」 | **采纳**，落位索引 1（`stop-background-workers` 之后、`flush-sessions` 之前） | `A6 §5.5` |
| G-8「持久化 fail-closed、不内联凭据」 | **强化**：R-3 禁止 `ScriptParam.secret` 参数值入 `tasks.json`，且目标含必填 secret 参数时**拒绝创建**；并要求损坏文件 rename 为 `.corrupt` + 日志（O-A6-7），优于既有 `load_scripts_at` 的静默丢弃 | `A6 §3.3 R-3`、`§3.4` |
| G-10「ACL 插在 `list_artifact_images` 之前、命令过 source check」 | **采纳**：5 条命令全过 `check_invocation_source`，ACL 108→113 | `A6 §6` |
| G-3「审计不含凭据」 | **采纳**：R-3 + §6.4 + §7 脱敏行 | `A6 §7` |

**无限增长**面上的正向设计：`SCHED_MAX_TASKS=200`、`task-runs.json` 环形 500、`SCHED_MAX_SLOT_SCAN=512`、`catch_up_limit ≤ 10`、`Interval.every_secs ≥ 60`、`MAX_CONCURRENT_RUNS=8`。**增长边界基本封闭**，仅审计与运行记录存在挤出（R-1 / R-2）。

---

## 5. 发现（R-1 ~ R-11）

### R-1【高】审计冲刷：定时任务的每次执行写 `audit.json`，可在数小时内抹掉全部历史审计

**事实**：
- `workspace::log_audit` 上限 1000 条 FIFO（`workspace.rs:388-389`：`if list.len() > 1000 { list.drain(0..len-1000) }`）。
- A6 §7 冻结的审计事件含 **`task.run.start` 与 `task.run.finish`**，即**每次执行** 2 条。
- 最小间隔 = 60s（`F-A6-6`），cron 最小粒度 = 1 分钟 → 单任务最多 **1440 次/天**。

**推算**：1 个任务 × 2 条/次 × 1440 次/天 = **2880 条/天** → 容量 1000 的 `audit.json` 约每 **8.3 小时被完全替换一轮**；即使每次只写 1 条，约每 16.7 小时一轮。

**影响**：`audit.json` 是**全局共享**的（手工 git 写、脚本/命令执行、书签等都在里面）。高频定时任务会把**全部手工操作审计冲掉**，直接破坏「可审计性」这一 M4 红线，且不可逆、无告警。

**加重因素**：这些运行明细**已完整存放在 `task-runs.json`（cap 500，A6 §7）**，写进 `audit.json` 属**重复存储且有害**。A6 已识别「每 tick 写审计」的风险并明令禁止（§7），但**每次执行**这一层未堵。

**建议（A6/A7/A0）**：把 `task.run.start` / `task.run.finish` **移出** `audit.json` 事件表（明细留在 `task-runs.json`）；`audit.json` 只保留生命周期与失败事件（`task.add/update/remove/enable/auto_disabled/clock.rewind/run.reject/run.missed(over_limit)`）。若必须保留成功事件，则须改为独立审计文件或按任务限流采样。
**验收**：以 60s 间隔任务连跑 N 次，断言 `audit.json` 中非 task 类历史条目不被挤出。

### R-2【中高】`script-runs.json`（cap 200 FIFO）会被定时任务挤出手工执行记录

**事实**：A6 §7 复用既有 `script-runs.json`（`MAX_RUN_RECORDS=200`，FIFO）承载调度执行明细。
**影响**：一个 60s 间隔任务每天产生 1440 条记录，会在 **约 3.3 小时内**把 200 条环形缓冲全部替换为定时任务记录，**手工执行记录被挤出**。叠加既有债务 **D21**（`ScriptRunRecord` 无 `kind` 判别，A6 `O-A6-3` 已记），被留下的记录**无法区分是定时还是手工**，排障证据实质丢失。
**建议**：A7 落地时评估是否为调度记录启用独立文件或独立配额；至少在 A11 的验证清单中登记「高频任务下手工记录被挤出」为已知观察项。与 D21 合并权衡是否单开卡。

### R-3【中】`dangerous=true` 命令片段可被定时任务引用，确认只在创建时发生一次

**事实**：A6 `§3.3 R-7` 允许 Command 类任务引用 `dangerous == true` 的片段，仅要求带 `dangerous` 透传标记「供 M4-8 UI 二次确认」。
**实测校正（对 A6 与我上一轮都重要）**：我核了手工路径 —— `run_command`（`bridge.rs:2977`）**没有**确认闸门，只有 `check_invocation_source` + `check_id` + `start_command` 内部策略；`snippet.dangerous` 只是**写进审计**（`bridge.rs:3017-3021`），不拦截。二次确认闸门仅存在于 **git 写**路径（`confirmed_dangerous`，`bridge.rs:1913/1924`）。
**结论**：因此这不是定时任务**新引入的绕过**（手工同样无闸门），我按事实下调严重度；但它是**风险放大**——无人值守 × 周期重复 × 危险片段，而唯一确认只在创建时发生一次，之后每次自动执行都无人参与。
**建议**：由 A0 显式裁决其一：① 首期**禁止**定时任务引用 `dangerous=true` 片段（与 M4「默认拒绝」主题一致、无人为可执行）；② 保留但在 `TaskDef` 增加显式 `acknowledge_dangerous: bool`（默认 false）+ 每次执行写 `task.run.dangerous` 审计。
**不建议**依赖「UI 二次确认」——无人值守场景拿不到逐次同意。

### R-4【中】`secret` 参数值只在**创建时**校验，事后改标不会被复检

**事实**：`R-3` 在 `task_add` 时校验 `params` 不含 `ScriptParam.secret == true` 的值，目标含必填 secret 参数则拒绝创建。
**缺口**：若脚本**后来**被编辑、把某参数改标为 `secret = true`，已创建任务 `tasks.json` 中的**明文值不会被再校验也不会被清除**，且调度器继续按原值执行（`domain.rs:623` 确认 `secret` 是可变字段；`bridge.rs:2889` 确认片段可改）。
**建议**：A7 在调度器**加载任务时**与 `task_update` 时复跑 R-3；命中即自动 `enabled=false` + 审计（沿用 §5.2 持久性错误的既有处置），而非静默执行。

### R-5【中】未标记 `secret` 的参数值仍以明文落盘 `tasks.json`

**事实**：R-3 依赖脚本作者**正确标注** `ScriptParam.secret`。未标注但实际敏感的参数（如把 token 当普通参数传）会明文进 `tasks.json`。`tasks.json` 是普通文件，属 M4 红线「凭据不得进普通文件」的字面违反面。
**评估**：这是**残余风险而非缺陷**——后端无法可靠推断任意参数是否敏感，强行做值形态启发式会带来误报与新的复杂度。
**建议**：A0 显式接受并登记为债务；同时要求 A8 在 M4-8 UI 对 `params` 输入做「疑似敏感值」提示（软提示，不做硬拦截）。

### R-6【中高】`M4-1.c` 是自相矛盾的产物，而 M4-1.d 的上限/取消/超时常量全部引用它

**事实**：`logs/checkpoints/M4-1.c-20260905-2255.md` 当前同时包含——
- **第 1~125 行**：A2 写就的**完整**契约（上限数值表、结果 DTO、取消三层、超时分层、多语句、测试策略）；
- **第 126~149 行**：A0 的 `STOPPED_EMPTY_ARTIFACT` 裁定（「核查时为 0 字节空文件…后续 Lane A2 需要重新领取并完成 M4-1.c / M4-1.d」）。

即：**正文已补写，但尾部的 STOPPED 裁定未撤销**，文件自身状态矛盾。
**影响面**：`M4-1.d` §1 第 12~16 行（行 1000 / 4 MiB / 64 KiB / SQL 64 KiB；三层取消；30/600/5 超时；多语句显式关闭）**全部以 `c §1`~`c §5` 为出处**。这些恰是「无限增长 / 写权限」红线所依赖的数值。
**建议（A0）**：明确裁定 `M4-1.c` 状态（建议：撤销尾部 STOPPED 裁定、保留正文，并按 §8「未做/未实测」如实在主文档标注为 `CONTRACT_FROZEN(文档态, 常量未落代码)`）。**在状态澄清前，`M4-1.d` 的上限数值不应被视为已冻结的验收基线。**

### R-7【中】上限常量未落代码，当前零代码级约束

**事实**：`M4-1.c §8` 明说「上限常量**未写入 `domain.rs`**…由 A3 落实现侧」。我 grep `domain.rs` 的 M4 段（813-1009）确认**无任何 `const` / `MAX_*` 定义**（仅错误码字符串）。
**评估**：就 M4-1「纯契约卡」定位而言这是合理的（避免无消费者常量，M2-3.b 教训），**不算缺陷**；但须明确记录：**截至 `47fce60`，结果行数/字节上限在代码层面尚不存在**，唯一约束是 A3 的自觉。
**建议**：A3 落地时必须把这些常量与 `DbErrorCode` 同处定义并配单测；建议加码位守护「常量缺失即报」，避免 A3 遗漏。

### R-8【低】`domain.rs:819` 注释声称被 `check-database-policy.py` 守护，但该脚本不存在

**事实**：`domain.rs:819` 写「三条硬约束（由 `check-database-policy.py` 与本文件单测**双向**守住）」；实测 `ls scripts/check-database-policy.py` → **不存在**。A2 在 `M4-1.d §10` 与 `O-A2-2` 已如实声明未创建该脚本，归属改给 A4。
**评估**：**非缺陷**（A2 已诚实声明、未冒领），但代码注释与仓库实际状态不符，后续读者会误以为有外层静态守护。
**建议**：A4 落地脚本时同步把该注释从「双向守住」改为「由本文件单测守住，`check-database-policy.py` 见 `DB_*` 码位（M4-1.d §5）」；在脚本落地前，三条硬约束处于**单侧守护**（仅 Rust 单测 `T-db-c3` / `T-db-c4`），可满足但应知情。

### R-9【低】ACL 条目数将超过 A6 的 113 预估

**事实**：A6 §6 预估 108 → 113（5 条 `task_*`）。但 `M4-1.c §3` 还冻结了 `db_cancel`（第 4 条 db 命令），`O-A2-1` 另有待决的 `db_forget_connection`（第 5 条）。加上 A1 的 `db_*` 三条，实际将达 **114~115**。
**影响**：纯计数问题，但 `DB_ACL_ORDER` / `SCHED_ACL_ORDER` 码位须覆盖**全部**新命令，漏一条即「命令静默不可用 + 夹具不报警」。
**建议**：A4 冻结命令名时一次性给全 db 命令清单，并让两个 ACL 码位按「末条锚点之前必须包含全部 `db_*`/`task_*`」来判定，而非按固定条数。

### R-10【信息】`O-A6-11` 已失效，无需订正

A6 §10 `O-A6-11` 称「A10 的 G-8 标题仍写 `workspace/tasks.yaml`，建议 A0 订正」。
我实测核对：工作区与**已提交版本**（`git show HEAD:...`）第 129 行均为
`### G-8【中】`tasks.json` 是可执行载荷的明文文件，损坏恢复须 fail-closed`。
**该标题已在上轮修订为 `tasks.json`**，A6 读到的是修订前副本。**O-A6-11 可关闭，A0 无需动作。**

### R-11【低】重试总时长硬上界未冻结

A6 §5.3 把「重试**总时长**硬上界常量」交给 M4-7。按已冻结值最坏估算：`max_attempts≤5` × `timeout≤600s` + 4 段 `delay≤600s` = **≤ 5400s（90 分钟）/ 任务**。并发上界 8，故不会无限堆积，但单任务占用时长可观。
**建议**：A7 落地时冻结该常量并配单测（可纳入 `SCHED_RETRY_UNBOUNDED` 码位）。

---

## 6. 红线对照总表

| 红线 | 结论 | 依据 |
|---|---|---|
| **隐私** | ✅ 无违规 | `DbConnectionConfig` 结构性无凭据字段（T-db-c3）；审计 detail 不含参数值/正文（`A6 §6.4`）；BLOB 不回传字节（`M4-1.c §2`） |
| **凭据** | ⚠️ 残留 2 项 | 结构性设计优秀；残留 R-4（事后改标不复检）、R-5（未标 secret 明文落盘） |
| **写权限** | ✅ 达标 | `allow_write` 默认 false；三态 `Unknown ⇒ 拒写`；`enabled` 默认 false；分类器「不可解析即拒」已进码位 |
| **无限增长** | ⚠️ 边界基本封闭，审计有缺口 | 任务/历史/扫描/并发均有上界；缺口 = **R-1（审计冲刷）**、**R-2（运行记录挤出）** |
| **source check / ACL** | ✅ 达标 | 本轮无新命令落地；契约层已冻结「5 条 `task_*` 全过 `check_invocation_source` + ACL 插在 `list_artifact_images` 之前」；待落地时覆盖 db 命令（R-9） |

---

## 7. 需 A0 处置项（按优先级）

1. **R-1（高）**：裁决 `task.run.start/finish` 是否移出 `audit.json` —— 这是本轮唯一可能**实际破坏可审计性**的问题。
2. **R-6（中高）**：澄清 `M4-1.c` 状态（撤销或确认尾部 STOPPED 裁定），否则 M4-1.d 的上限数值无有效基线。
3. **R-3（中）**：裁决定时任务能否引用 `dangerous=true` 片段（建议首期禁止）。
4. **R-2（中）**：登记「高频任务挤出手工运行记录」为已知观察项，与 D21 一并权衡。
5. **R-4（中）**：要求 A7 在调度器加载/`task_update` 时复跑 R-3 校验。
6. **R-10**：关闭 `O-A6-11`，无需订正我的文档。

---

## 8. 声明（避免误读）

- **本轮为文档级复核**：复核对象全部是**契约/类型提案文档**，M4 尚无任何可执行的数据库或调度代码（`src-tauri/src/` 无 `database*` / `scheduler*`）。因此本文件**不构成对 M4 实现的运行时安全裁定**，实现落地后须复审。
- **未复跑测试**：`cargo test` 237 passed / `pre-merge.sh` ALL_PASS 采信 A2 自报（`M4-1.d §9`），本轮**未由我复跑**（我零代码改动，且复跑不改变文档结论）。
- **未改产品代码、未 `git add`、未提交、未 push**（按指挥板 Merge Rule：只有 A0 可向 `master` 提交与推送）。本轮唯一新增 = 本文件。
- **独立性**：A2 与 A6 均为 CodeBuddy Hy4（与我同模型），A6 §11.3 已自陈「与 A1 展开者同模型，独立性弱于跨模型裁定」。故本复核**刻意以源码实证替代采信**：R-1 的容量与频率我回查了 `workspace.rs:388`；R-3 的「手工也无闸门」我回查了 `bridge.rs:2977-3021`；R-4 的可变性我回查了 `domain.rs:623` 与 `bridge.rs:2889`。三条结论均非文档互证。
- 上一轮 `A10-M4-security-review-20260905-2240.md` 的 G-1~G-12 仍然有效，本文件为**增量复核**，不替代之。
