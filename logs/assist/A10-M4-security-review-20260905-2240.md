# A10 · M4 安全评审（预实现阶段）

> Lane: `A10` — M4 security review
> 时间: 2026-09-05 22:40 CST（23:20 追加 §8 补充评审）
> Base: 开工时 `e6e09cf`；**交付时 HEAD = `3544c09`**（+2 个 docs 提交 `f376346` / `3544c09`）。
> 注：指挥板 23:00 版写 `Current mainline: master at f376346`，实测落后 1 个提交（A3/A4 亦独立发现）。
> 车道依据：指挥板 §Dispatch Waves **Wave 1** 第 86 行 ——「A10: Baseline security review of existing M4 plans and prior gates; **product-code changes forbidden**」，与本文件交付一致。
> 开工时工作树干净、分支 `master`、与 `origin/master` 同步（无 ahead/behind）。
> 范围: 评审笔记 / 策略缺口 / `logs/assist/`，**不改产品代码**
> 方法: 源码比对（现有安全基线）+ M4 护栏与验收门禁逐条推演，**不做实现、不代签**

---

## 1. 结论摘要

**STATUS = PASS_WITH_DEBT**（A10 自身交付完成；但评审对象尚未存在）

**关键前提（必须先看）**：截至本次评审，A2 / A3 / A4 / A7 **均无任何产品代码产出**（`src/` `src-tauri/` `scripts/` `Cargo.toml` 一律零改动）——

- `src-tauri/src/` 下不存在 `database*` / `scheduler*` 任何文件（`find` 全仓确认，仅 `target/doc` 下的 gtk/tokio 文档命中）；
- `详细设计与实施计划.md:451-459` 的 M4-1~M4-8 仍是未展开的单行 WBS（全部 `[ ]`），A1 的展开尚未落地；
- `logs/assist/` 下唯一的数据库相关文档是 `database-schema-taskcard-20260902-1146.md`（2026-09-02 的旧预研，非 A2 本轮契约）。

指挥板优先级第 8 条要求「A10 reviews security-sensitive M4 lanes **after their output exists**」。因此本轮**不能**出具对 A2/A3/A4/A7 实现的实质裁定（无对象可裁），改为交付其在实现前**必须先满足的安全准入契约**。

> **评审期间并发变更（已交叉核对）**：本文档写作过程中，Lane A1 于 22:25 落地了
> `logs/checkpoints/M4-20260905-2225.md`（M4 任务卡展开与依赖图，含 14 项实测与 7 条事实修正 F-1~F-7），
> Lane A11 亦产出 GUI 清单与验证矩阵。A1 的展开**改变了 5 处与本文相关的实现前提**
> （凭据键格式、持久化形态、关机任务数、ACL 插入锚点、调度运行时），
> 本文已逐条复核并**按 A1 实测口径修订**（见 G-1 / G-7 / G-8 / G-9 / G-10 中的「A1 修正」标注）。
> **A2/A3/A4/A7 认领前，须同时读本文件与 A1 展开卡**；两者冲突时以 A1 的实测数据为准，安全红线以本文件为准。

这样做的理由：M4 的 8 条护栏（指挥板 89-98）目前只是**口号级**陈述（如「生产判定不确定时 fail-closed」），尚未落到任何可执行的判定函数、不变量或测试上。若等 A3/A4 写完再评审，护栏会以「实现已定型」的方式落地，届时整改成本远高于先给契约。**本文件即把护栏翻译成各 lane 的准入检查表。**

---

## 2. 现有安全基线（事实清单，含源码定位）

M4 必须复用而非重建下列能力：

| 能力 | 位置 | 现状 |
|---|---|---|
| 凭据隔离 | `src-tauri/src/keyring_store.rs` | `SERVICE="com.jizhijiandan.mvp"`，`save/get/delete_token(repo_id)`；**无 `Debug`/`Display` 派生**，不会被意外日志化（好） |
| 敏感 URL 脱敏 | `src-tauri/src/security_policy.rs:416` `redact_sensitive_url` | 去 userinfo + 21 个敏感 query 键（`SENSITIVE_QUERY_KEYS`:350）置 `***` + 限长 2048 |
| 来源校验 | `src-tauri/src/bridge.rs:923` `check_invocation_source` → `security_policy.rs:487` `check_remote_invocation` | 仅 `main` label 放行；其余需一次性意图令牌（`IntentRegistry`，TTL 30s） |
| 审计落盘 | `src-tauri/src/workspace.rs:381` `log_audit(action, detail)` | 追加式 JSON，上限 1000 条 |
| 审计脱敏 | `src-tauri/src/bridge.rs` `sanitize_audit_text` | 截断 300 字节 + `…`，不切 UTF-8；**仅 git 写路径调用** |
| 生命周期收口 | `src-tauri/src/bridge.rs:778` `register_shutdown_tasks` | 顺序：`stop-background-workers → flush-sessions → close-tabs → kill-terminals → shutdown-grid` |
| 关机协调器 | `src-tauri/src/shutdown.rs:85` `ShutdownCoordinator::register(name, task)` | 失败/panic 降级、重复退出安全；**测试硬编码了真实注册顺序**（`shutdown.rs:386-391` 注释） |
| M2-4 执行通道 | `src-tauri/src/script_runner.rs` | `start_run`:843 / `start_command`:882 / `cancel(run_id)`:282 / `kill_all_running`:909 / `effective_timeout`:330（默认 60s，上限 600s） |
| 启动目标校验 | `security_policy.rs:170` `check_launch_target` | 禁 `sh/bash/zsh/fish/powershell/cmd` 直启 |
| 静态策略门禁 | `scripts/check-security-policy.py` `detect_gaps`:79 | 具名缺口码 `SEC-01..SEC-09`；`--self-test` 夹具模式；pre-merge 双跑（`pre-merge.sh:217-219`） |
| ACL | `src-tauri/permissions/default-commands.toml` | `commands.allow` 扁平列表；`capabilities/default.json` 引用 `default-commands`；另有 `remote-collect.toml`（浏览器子 webview 专用，能力极小） |

已确立的**测试约定**（各命令族各有一条）：`remote_invocation_to_*_commands_is_rejected`（`bridge.rs:5134 / 5293 / 5425 / 5625`）。M4 新增命令须沿用。

---

## 3. 发现（G-1 ~ G-12，按风险排序）

### G-1【高】凭据键命名空间会冲突，且吊销路径从未被使用
**事实**：`KeyringStore` 全部键共用同一 `SERVICE`，且 `delete_token` 当前标注 `#[allow(dead_code)]`（`keyring_store.rs:22`）——即**删除路径无任何调用、无测试**。
**风险**：数据库连接凭据若直接复用 `save_token(conn_id)`，与 git 仓库 token 共享命名空间，`conn_id` 与 `repo_id` 碰撞即互相覆盖/泄漏；且断开连接时无法可靠吊销凭据。
**要求（A3）**：新增 DB 专用键前缀（如 `db:<conn_id>`）与服务项，不得复用 repo 命名空间，并须有单测证明 `db:<x>` 与 `<x>` **不碰撞**；凭据吊销须有**显式入口**（如「忘记连接」）并移除 `#[allow(dead_code)]`、补删除路径测试。
**验收**：`cargo test` 覆盖「保存 → 读取 → 断开 → 读取失败」；测试断言键前缀隔离与删除生效。

> **A1 修正（F-7，已采纳）**：A1 实测确认 `KeyringStore` 按**精确字符串**取键，并进一步裁定
> `db_disconnect` **默认不删凭据**（避免误断连即丢密码），删除仅由显式「忘记连接」入口触发。
> 本文原稿要求「断开即吊销」**按 A1 口径放宽**为「必须存在显式吊销入口」——两者安全目标一致
> （凭据可被主动清除），A1 的可用性理由成立，采纳。

### G-2【高】`redact_sensitive_url` 覆盖不到非 URL 形态的 DSN
**事实**：解析失败时走 `Err(_) => truncate_url(raw)`（`security_policy.rs:445` 附近），**只限长、不脱敏**。而 key=value 形态 DSN（`Server=..;Password=..;`）与结构化连接配置（host/port/user/password 分字段）**根本不经过任何脱敏函数**。
**风险**：连接失败时的错误信息、审计 detail、日志会原样携带口令。
**要求（A4 + A3）**：
1. 凭据**只能**以类型化字段流转，**永不参与字符串序列化**（结构化配置优先于 DSN 字符串）；
2. 新增结构化 DSN 脱敏（key=value 形态的口令键同样置 `***`），并纳入 `SENSITIVE_QUERY_KEYS` 同源维护；
3. 任何进入 `eprintln!`/错误串/审计的连接描述，必须先过脱敏。
**验收**：不变量测试断言「含 `Password=P@ss` 的 DSN 经脱敏后不含 `P@ss`」。

### G-3【高】审计脱敏靠调用方自觉，而 SQL 文本本身就是凭据汇
**事实**：`log_audit(action, detail: String)` 内部无脱敏；目前只有 git 写路径调用 `sanitize_audit_text`。
**风险**：数据库审计天然想记 SQL 原文；而 `CREATE USER ... IDENTIFIED BY 'x'`、`SET PASSWORD = 'x'`、`COPY ... WITH PASSWORD` 等语句**把口令写进 SQL 字面量**。「记录 SQL」=「记录凭据」，这是数据库功能特有的、git 路径不存在的泄漏面。
**要求（A3 / A7）**：审计 detail 只记**语句类别 + 参数摘要哈希 + 影响行数 + 目标连接 id**，**禁止存 SQL 原文**；错误文本一律过 `sanitize_audit_text`。
**验收**：不变量测试断言 db/task 审计 detail 不含 `password|pwd|secret|token|identified by`（照抄 `bridge.rs:4860` `git_write_audit_detail_contains_no_credentials_or_path_lists` 的写法）。

### G-4【高】SQL 风险分类必须「不可解析即拒绝」，且必须处理多语句
**事实**：仓库现有任何分类器都不存在；护栏只写「写操作默认拒绝」。
**风险**：关键词/正则分类器易被绕过：注释（`/*x*/ DELETE ...`）、字符串字面量、CTE（`WITH t AS (...) DELETE FROM ...`，首关键字是 `WITH` 不是 `DELETE`）、**堆叠语句**（`SELECT 1; DROP TABLE t;`）。多数驱动默认允许多语句执行，只看首关键字等于放行后半段。
**要求（A4）**：
1. 确定性剥离注释与字符串字面量后再分类；
2. **批中任一语句非 SELECT 即整批拒绝**（不允许「只判第一条」）；
3. 解析不了 → **拒绝**（fail-closed），不得降级放行；
4. 驱动层显式关闭多语句执行（`allowMultiStatements` / 等价项）。
**验收**：变异坏样本集（注释绕过 / CTE 绕过 / 堆叠绕过 / 大小写变形 / 不可解析输入），策略脚本 `--self-test` 必须全检出。

### G-5【高】「生产判定不确定 fail-closed」缺少信号契约
**事实**：设计文档 `:427` 提出 `is_production_database` 纯函数层，但未定义信号集。
**风险**：朴素主机名匹配（含 `prod`/`prd`）既有假阴性（`db-01.corp`）也有假阳性（`prod-test-local`）；无信号时若返回 `false` 即等于默认放行写操作，直接违反护栏。
**要求（A2 定信号契约 / A4 实现）**：定义带 `Unknown` 变体的判定返回类型；`Unknown` → 拒绝写；名称启发式**不得作为唯一信号**（须叠加显式标记/环境类信号）；连接建立失败、主机不可解析、库名为空均须归入 `Unknown`。
**验收**：测试矩阵含「不可解析主机 / 空库名 / 仅名称匹配 / 显式非生产标记」四类，断言仅前三类拒绝写。

### G-6【中高】结果集上限与取消无任何现成原语，且必须在取数循环内生效
**事实**：`security_policy.rs` 的 `MAX_RESOURCE_ITEMS=500`、`MAX_TEXT_FIELD_BYTES=64KiB` 只服务资源瀑布，与查询结果无关；无行数/字节数上限、无查询取消。
**风险**：`SELECT *` 大表 → 后端物化整个结果集再序列化 JSON → 主进程 OOM，再经 IPC 打到 webview。**物化后再截断等于没截断**。
**要求（A3）**：行数与字节上限在**取数循环中**生效（超限即停止取数并丢弃剩余），返回 `truncated: bool`；取消语义对齐 `script_runner::cancel`（复用其错误/取消语义），并保证连接归还池中。
**验收**：测试断言「上限内结果完整、超限时行数不超上限且 `truncated=true`」；取消后连接仍可用。

### G-7【中高】调度器是「无人值守执行路径」，会架空现有意图令牌模型
**事实**：现有命令侧写操作都过 `check_remote_invocation`（需 `main` label 或已消费的一次性令牌）。**定时触发既无 webview label，也无用户意图令牌**——现有模型对调度器天然不适用。
**风险**：一旦为调度器开「内部调用」旁路，就会形成一条绕过路径与超时上限的**第二执行通道**，直接违反护栏「no second shell/process execution path」。
**要求（A7）**：
1. 只能调 `script_runner::start_run` / `start_command`，**禁止**自建 `Command::new`；
2. 与人工触发**共用同一套策略门**（路径根、`check_launch_target`、`effective_timeout` 60s/600s 上限），不得存在「调度器专用」的放宽分支；
3. 审计带 `origin=scheduler` 标记，与人工执行可区分；
4. **禁止自我重入**（同一任务上一轮未结束不得再触发）；
5. 新建任务默认 `enabled=false`，启用为显式动作。
**验收**：测试断言「调度触发与人工触发走同一函数」「重入被拒」「默认禁用」。

> **A1 修正（F-1 / F-5，已采纳并强化）**：
> - **F-5**：首期 `TaskKind` 仅 `Script` / `Command`（`Tool` 不做，因子 webview 工具无 headless 执行入口）。
>   安全收益：攻击面收敛到两条已有执行通道，本文 G-7「禁止第二执行通道」的可实现性因此提高。
> - **F-1**：`tokio` 当前**仅为传递依赖（1.53.1，不可 `use`）**，M4-1.a 须裁决「提升 tokio 为直接依赖」
>   还是「`std::thread` + `Condvar` 可中断 sleep」。**该裁决直接影响 G-7 的取消语义与 G-9 的关机 join**：
>   若选 `std::thread` + `Condvar`，取消与有界等待须自建中断协议，且**不得**因此绕开
>   `effective_timeout`（60s/600s 上限）；若选 tokio，须注意新增 runtime 对 M0-4 依赖清理口径
>   与 build metrics 基线的影响（见 G-11 / D24）。两者都须满足「无时间窗内无人值守的不可取消执行」。

### G-8【中】`tasks.json` 是可执行载荷的明文文件，损坏恢复须 fail-closed
**事实**：设计文档 `:442` 原定 `workspace/tasks.yaml`；验收门禁 `:465` 提到「任务损坏」但无设计。
**风险**：解析失败时若静默丢弃或「修复」为启用态，等于配置文件可被外部篡改后自动执行任意脚本；非原子写在崩溃时会留下半截文件。
**要求（A7）**：tmp + rename 原子写；解析失败 → **全部任务置为禁用** + 落审计，**不得**自动启用或静默丢弃；任务定义只引用**已保存连接 id**，**不得**内联任何凭据（凭据只存 Keyring）。
**验收**：损坏文件用例断言「无任务被启用」「有审计记录」「不 panic」。

> **A1 修正（F-4，已采纳）**：持久化形态由 `tasks.yaml` **统一改为 `tasks.json`**。
> A1 实测项 11 确认：既有持久化（`repos.json` / `bookmarks.json` / `sessions/` / `snippets.json` / `scripts.json`）
> **全部 JSON**，且已有统一范式：**tempfile + rename 原子写、损坏跳过、容量 FIFO、删除幂等**。
> 引入 YAML 会形成两套持久化形态与两套原子写路径（无收益复杂度）。
> **安全对齐说明**：既有范式中的「损坏跳过」对任务文件而言 == 「不加载任何任务」== 本文要求的
> 「全部置为禁用」，因此**与 fail-closed 要求一致**，A7 直接沿用既有范式即可满足本条，
> 无需自造恢复逻辑（自造反而易写成「修复为启用态」）。

### G-9【中】关机顺序与测试强耦合，调度器必须最先停止
**事实**：`register_shutdown_tasks`（`bridge.rs:778`）顺序被 `shutdown.rs:386-391` 的测试**硬编码**；当前首个任务是 `stop-background-workers`。
**风险**：若调度器 abort 注册靠后（如在 `kill-terminals` / `kill-running-scripts` 之后），定时器可能在拆卸过程中触发，在进程回收**之后**再拉起子进程——正是「退出后无子进程残留」门禁要防的场景。
**要求（A7）**：scheduler 停止注册为**第一个**任务（或紧随 `stop-background-workers`）；需 join 在飞任务，但必须设**有界等待**（协调器任务为同步执行，长 join 会拖垮退出预算——注意未决债务 D25：`on_channel_dead` 未 wait，同族问题）；同步更新 `shutdown.rs` 的顺序期望。

> **A1 修正（实测项 8，已采纳）**：关机任务实际为 **6 个**（本文原稿记为 5 个），
> 完整顺序 = `stop-background-workers` → `flush-sessions` → `close-tabs` →
> `kill-terminals` → `shutdown-grid` → **`kill-running-scripts`**。
> 关键推论：末位的 `kill-running-scripts`（调 `script_runner::kill_all_running`）是**兜底回收**，
> 因此 scheduler 停止**必须注册在它之前**——否则会出现「调度器又拉起脚本 → 兜底已执行完 → 子进程残留」的
> 竞态。推荐注册位：`stop-background-workers` 之后、`flush-sessions` 之前（先停新任务，再落盘状态）。
**验收**：关机顺序测试更新并通过；`cargo test shutdown` 全过。

### G-10【中】8 条新命令的 ACL / 来源校验 / 静态门禁增量
**事实**：ACL 是 `default-commands.toml` 的扁平 `commands.allow`；`remote-collect.toml` 只给浏览器子 webview 用。
**风险**：漏登记 → 命令不可用；**误登记进 `remote-collect`** → 外部页面可连库、可建定时任务（严重）。
**要求（A3 / A7）**：
1. `db_connect` / `db_query` / `db_disconnect` / `task_list` / `task_add` / `task_update` / `task_remove` / `task_run_now` 共 8 条加入 `default-commands.toml`；
2. 每条均调 `check_invocation_source`（仅主窗口）；
3. **禁止**进入 `remote-collect`；
4. 补 `remote_invocation_to_db_commands_is_rejected` 与 `remote_invocation_to_task_commands_is_rejected` 两条测试（沿用 `bridge.rs:5134` 等既有写法）；
5. `check-security-policy.py` 的 `detect_gaps` 新增 db/task 缺口码（`SEC-10+`），pre-merge 参照 `pre-merge.sh:217-219` 双跑 `--self-test` 与默认判定。

> **A1 修正（实测项 6，已并入本条，属高危实现陷阱）**：
> **新命令必须插在 ACL 末条 `list_artifact_images` 之前**（第 110 行），**不得**直接追加到文件末尾。
> 原因：`check-image-policy.py` 的坏样本按「**末行无逗号**」定位 ACL 末尾，M2-1 / M2-2.b / M2-3.b
> **三度踩坑**。若把 `db_*` / `task_*` 追加到末尾，将使既有坏样本判定失效、且新命令在
> 某些解析路径下不被识别。A3 / A7 修改 `default-commands.toml` 后必须复跑
> `check-image-policy.py --self-test` 与 `check-security-policy.py`。
>
> **另附一处待核（A1 实测项 5）**：`bridge.rs` 有 `#[tauri::command]` **105** 个，
> 而 `default-commands.toml` 有 **108** 条，存在 **3 条差额**。差额本身可能合理（如插件命令/别名），
> 但在 M4 新增 8 条命令前**应先核清这 3 条是什么**——若存在「已注册到 ACL 但 bridge.rs 无对应命令」
> 或反之的情况，说明 ACL 与命令集已漂移，在此基础上继续追加会把漂移放大且难以归因。
> 建议 A3 认领时顺带出具 105 vs 108 的对账结论（可并入 M4-3 检查点）。

### G-11【低中】M4-1 代码生成与依赖引入是供应链 + 构建门禁面
**事实**：设计文档 `:425` 计划 `build.rs` + `plugins/db-types/*.yaml` 生成 `SupportedDb`；仓库有构建度量门禁（`scripts/measure-build-metrics.py`）。
**风险**：YAML 未校验即生成 → 未知字段静默进入生成代码；新增 sqlx 等依赖会改变二进制体积/构建时长，与 M0 基线对比冲突。**注**：未决债务 D24（M0-0.b 吞吐基线未在新管道重采）会让这部分对比失真。
**要求（A2）**：`build.rs` 对 YAML 做 schema 校验、拒绝未知字段；锁定依赖版本（`cargo check --locked` 已在 pre-merge 中）；避免引入第二个异步运行时或重复 feature 导致体积膨胀，构建度量须与 M0 基线同口径对比并说明 D24 影响。

### G-12【低】前端口令绝不得进入任何持久化 store
**事实**：`src/stores/` 下多个 store 参与持久化，且关机路径有 `flush-sessions`（`bridge.rs` 注册）会落盘。
**风险**：口令一旦进入被持久化的 store，就会被 flush 写进磁盘文件，直接违反「凭据不进入前端状态或普通文件」。
**要求（A5）**：口令只允许存在于**组件局部瞬态**状态，提交后立即清空；**禁止**进入任何持久化 store、路由参数、本地缓存或错误提示。
**验收**：UI 逻辑测试（参照 `scripts/check-terminal-ui-logic.mjs` 的加载真实 store 的做法）断言口令不落持久化。

---

## 4. 各 Lane 安全准入检查表（实现前必须满足）

### A2 · M4-1 数据库驱动与生成契约
- [ ] 依赖选型明确**禁用 JDBC 侧车**（设计文档 `:422` 已定，须在契约中重申）
- [ ] `SupportedDb` 枚举 + YAML schema 校验（G-11）
- [ ] 生产判定**信号契约**定稿，含 `Unknown` 变体与「名称启发式不得为唯一信号」（G-5）
- [ ] 凭据命名空间与连接 id 方案定稿（G-1 的输入）
- [ ] 依赖与二进制体积影响评估，说明与 D24 的关系（G-11）
- [ ] **F-1 运行时裁决**（tokio 升为直接依赖 vs `std::thread`+`Condvar`）须写明对取消语义、关机 join 与 build metrics 的影响（G-7 / G-9 / G-11）
- [ ] **F-2 生成方式裁决**（build.rs 代码生成 vs 手写枚举白名单）若选代码生成，须说明 YAML 校验与生成物可审计性（G-11）

### A3 · M4-2 数据库后端核心
- [ ] DB 专用 Keyring 键前缀 + 吊销路径（G-1）
- [ ] 凭据只以类型化字段流转，不序列化（G-2）
- [ ] 审计不含 SQL 原文（G-3）
- [ ] 行数/字节上限在**取数循环内**生效 + `truncated` 标记 + 取消（G-6）
- [ ] 8 条命令的 ACL / 来源校验 / `remote_invocation_to_*` 测试（G-10，与 A7 分工）

### A4 · M4-3 数据库安全闸门
- [ ] SQL 风险分类器：剥离注释字符串 → 整批处理（任一非 SELECT 即拒）→ **不可解析即拒**（G-4）
- [ ] 驱动层关闭多语句（G-4）
- [ ] `is_production_database` 纯函数 + `Unknown → 拒绝写` + 四类信号测试矩阵（G-5）
- [ ] 结构化 DSN 脱敏（G-2）
- [ ] 新增策略脚本（db policy）：沿用 `--self-test` + 变异坏样本集（参照 `check-terminal-policy.py`），并接入 pre-merge（G-10）

### A7 · M4-6/M4-7 调度器后端
- [ ] 只复用 `script_runner::start_run/start_command`，无第二执行通道（G-7）
- [ ] 与人工触发共用全部策略门，无调度器专用放宽（G-7）
- [ ] 审计带 `origin=scheduler`（G-7）
- [ ] 禁止自我重入；新建任务默认禁用（G-7）
- [ ] `tasks.json`（**非 YAML**）沿用既有 JSON 原子写范式 + 损坏不加载（== 全禁用）+ 审计；不内联凭据（G-8）
- [ ] scheduler 停止注册在 `kill-running-scripts` **之前**（推荐 `stop-background-workers` 之后）+ 有界 join + 更新 `shutdown.rs` 顺序测试（G-9）
- [ ] 调度运行时裁决（F-1：tokio 直接依赖 vs `std::thread`+`Condvar`）不得放松取消与超时上限（G-7）
- [ ] 5 条 `task_*` 命令的 ACL / 来源校验 / 测试（G-10）

---

## 5. 门禁增量要求

M4 各 lane 结束后，A0 集成前至少应新增/通过：

1. `cargo test`：db 分类与生产判定单元测试、调度重入/持久化损坏/关机顺序测试。
2. 静态策略脚本：新增 db 策略脚本，`--self-test` 与默认判定双跑并接入 `scripts/pre-merge.sh`（照 `check-terminal-policy.py` 接法）。
3. `check-security-policy.py`：`detect_gaps` 增加 db/task 缺口码（G-10）。
4. 审计隐私不变量测试：db/task 审计 detail 无凭据（照 `bridge.rs:4860`）。
5. 前端逻辑测试：口令不进持久化 store（G-12）。

---

## 6. 与既有债务的交互

- **D23**（终端 GUI 实点验收挂账）：无直接交互，但 M4 的 DB UI / 定时任务 UI 同样需要 GUI 实点，**不得援引 D23 作为免验收理由**。
- **D24**（M0-0.b 吞吐基线未重采）：直接影响 G-11 的构建/基线对比可信度，A2 须显式声明。
- **D25**（`on_channel_dead` 未 wait）：与 G-9 同族（退出路径不等待），A7 处理有界 join 时应一并评估是否夹带修复。
- **D26**（终端历史按块计数）：无交互。

---

## 7. 本次未覆盖（声明，避免误读为已审）

- **未评审任何 M4 实现**：截至交付，A2/A3/A4/A7 仍无**产品代码**产出（仅 A1 的展开卡与 A11 的验证文档落地），本文件是**准入契约**，**不构成**对 A2/A3/A4/A7 实质的安全裁定。实现落地后须由 A10 或 A0 指定复核人做**实现后复审**。
- 未评审 M4-4 / M4-8（A5 / A8）UI 的视觉与交互，仅就口令持久化（G-12）给出红线。
- 未运行 `cargo test` / `npm run build`：本 lane 不改产品代码，无需复跑；基线以既有记录为准（M3.c 终态：`cargo test` 232/232）。
- 未修改 `PARALLEL_COMMAND_BOARD.md`（A0 的协调文件，避免并发写冲突）——建议 A0 将优先级第 8 条更新为「A10 已发布准入契约，A2/A3/A4/A7 实现前须对照 `logs/assist/A10-M4-security-review-20260905-2240.md` 的自检表」。
- **独立性声明**：本文结论基于对 `security_policy.rs` / `keyring_store.rs` / `bridge.rs` / `shutdown.rs` / `script_runner.rs` / `workspace.rs` / `scripts/check-security-policy.py` 的源码比对，未采信任何 lane 的自述结论；A1 展开卡中被本文采纳的部分（F-1 / F-4 / F-5 / F-7 / 实测项 5/6/8/11）均为**可复现的实测数据**（依赖清单、文件行数、持久化目录），已标注采纳理由。

---

## 8. 补充评审（23:20 追加）：A3 / A4 Wave-2 预研 与 车道归属冲突

> 本节在 §1~§7 成文后追加。期间 A3 / A4 / A5 / A9 的**只读预研**落地，A2 仍零产出，A7 文件为空（仍在写入）。
> A3 / A4 均自报 `STATUS = BLOCKED`（Wave 2，前置未完成），**零产品代码改动**——与本文 §1 判断一致。

### 8.1 对 A3 / A4 预研结论的独立复核

我复跑了 A3 §2 的 6 条阻塞核验中与安全相关的关键几条，**结论一致**：

| A3 核验项 | A10 独立复跑 | 结果 |
|---|---|---|
| `SupportedDb` 未交付 | `grep -rn 'SupportedDb' src-tauri/src` | 一致：`.rs` 零命中 |
| 依赖选型未落地 | `grep -n 'sqlx\|rusqlite\|postgres' src-tauri/Cargo.toml` | 一致：DB 依赖为空 |
| M4-1 WBS 未勾选 | `详细设计与实施计划.md` | 一致：仍为 `- [ ]` |
| 分类器未交付 | `grep -n 'classify_sql_risk\|is_production_database' src-tauri/src/*.rs` | 一致：零命中 |

→ **A3/A4 的 Wave-2 阻塞判断成立**，其「不写产品代码、只出 assist note」的处置符合指挥板第 33 行条款。A10 对其自律性无异议。

### 8.2 A4 已吸收 A10 结论（确认无异议）

A4 预研 §4.1 已明确写入：「审计 detail 只含 `op/conn_id/风险等级/行数`，**禁含 SQL 正文、参数值、凭据、连接串**（F5 + **A10 G-3**）」，
并逐条对齐「① `check_invocation_source` 置首行；② 失败与成功双路径审计」。
→ 与本文 **G-3 / G-10** 一致，**A10 确认无异议**。A4 后续实现时还须补齐本文 **G-1（键前缀 `db:<conn_id>`）** 与 **G-6（行数上限在取数循环内生效）**——这两条 A4 预研尚未覆盖，属命令层职责，开工时须纳入。

### 8.3 【A10 安全意见】A3 与 A4 的分类器归属冲突

**冲突事实**（A3 §3 与 A4 §3 各自独立上报，两者互斥且都自称有效）：

| 维度 | 指挥板 23:00 版 | A1 展开卡（22:29） |
|---|---|---|
| SQL 风险分类器归属 | **A3**（M4-2 pool/safety） | **A4**（M4-2.s，落 `security_policy.rs`、零 db 依赖） |
| 依赖方向 | A4 依赖 A3 的接口 | A3 依赖 A4 的分类器 |
| Merge Order | A3=3，A4=4 | A4=3，A3=4 |

A3 已正确指出：两份文件若都照办 → **A3 与 A4 互相等待 → 死锁**。

**A10 立场（安全视角，供 A0 裁决，不擅自采纳）**：**支持展开卡口径，即分类器归 A4、零 db 依赖、先于 A3 合入**。理由按安全权重排序：

1. **可测试性**——「零 db 依赖」使分类器成为**纯函数层**，可脱离驱动/连接池做完整坏样本测试（含 G-4 的注释绕过 / CTE 绕过 / 堆叠绕过 / 不可解析输入）。这正是本项目既有范式（`security_policy.rs` / `scripts.rs` / `snippets.rs` / `images.rs` 均为纯函数层 + 配套策略脚本 + 变异坏样本）。
2. **门禁前置**——分类器是本文标注【高】的**硬阻断项 G-4 / G-5**。由**先合入**的 lane 承载，可让高危判定在 A3 的池/查询核心之前就过门禁并被 A11 见证，避免「等所有 IO 写完才发现写操作没被拦住」的返工。
3. **评审独立性**——若分类器与连接池/查询核心同属 A3，则安全判定与 IO 实现同 lane，G-4 的坏样本集会与连接池改动互相搅动，**失败归因与安全独立性双双下降**。
4. **与 A3 的技术前提自洽**——指挥板 Wave 2 写「A4 依赖 A3 的 pool/safety 接口」，但 A1 展开卡第 328 行给出的前提是「M4-2.s **零 db 依赖**」，二者自相矛盾；零依赖的一方显然应排在前。

**A10 的硬性要求（不随归属变更而放宽）**：

- 无论最终归 A3 还是 A4，**G-4 / G-5 的验收标准不变**，且必须有**唯一 owner**。
- 若 A0 采纳指挥板口径（分类器归 A3），则须把本文 §4「A4 · M4-3 数据库安全闸门」清单**整体移交 A3**，A4 只保留命令层 + 来源校验 + 审计（G-3 / G-10）。
- **严禁**出现 A3 与 A4 同时改 `security_policy.rs` 分类器函数的窗口（会在 `domain.rs` 与 `security_policy.rs` 上直接撞车）。裁决落地后请 A0 确保本文 §4 检查表按新归属**重新分配并回写**。

---

## 9. A0 集成注意事项

- 本文件**零产品代码改动**，仅新增 `logs/assist/` 文档一个文件，与任何 lane 无文件冲突；A10 **未提交、未推送**（按指挥板「Only Lane A0 commits and pushes to master」）。
- **交付时工作树状态（重要）**：A10 开工时工作树干净（`e6e09cf`），交付时已变为脏——出现 A1 的 `logs/checkpoints/M4-20260905-2225.md`、A11 的 `logs/assist/M4-A11-gui-manual-checklist-*.md` 与 `logs/checkpoints/M4-A11-verification-matrix-*.md`，以及 `详细设计与实施计划.md` 的已暂存改动。这些**均属其他 lane**，A10 未触碰、未 `git add`、未提交。A0 集成时请按各 lane 交付物分别处理，勿与 A10 的文档混同。
- 高冲突文件预警：A3/A7 会同时改 `bridge.rs`、`domain.rs`、`src-tauri/permissions/default-commands.toml`；按指挥板合并序，A4(3) 早于 A3(4)、A7(5)，请 A0 按序集成并让后序 lane rebase。**特别注意 G-10 的 ACL 插入锚点**（须插在 `list_artifact_images` 之前），三方都改该文件时极易产生冲突与坏样本失效。
- 建议 A0 在合并 A3/A4/A7 后，指定 A10 做一次**实现后复审**（此时才有可裁对象），与本准入契约对照出具裁定。
- 建议 A0 将本文标注【高】的**五条**（G-1 凭据键隔离 / G-2 非 URL 形态 DSN 脱敏 / G-3 审计不存 SQL 原文 / G-4 分类器「不可解析即拒绝 + 整批处理」/ G-5 生产判定 `Unknown` 拒绝写）设为 **M4 里程碑放行的硬阻断项**——任一未满足即不得放行。
