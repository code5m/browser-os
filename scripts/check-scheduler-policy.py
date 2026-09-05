#!/usr/bin/env python3
"""Expose the M4-5 scheduler contract invariants as a reproducible fixture.

契约来源：`logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md`
（Lane A6 冻结的 `TaskDef` / 时钟 / 错过执行 / 取消语义），并承接 A1 展开卡
`logs/checkpoints/M4-20260905-2225.md` 的 F6 / F7 / F8 / F9 / F10 与 M4 护栏。

本夹具守住的底线（M4-5.e 后默认模式 23 个 ACTIVE 码位，无 pending）：

  执行通道（F6）
  - 调度器只做触发：`scheduler*.rs` / `tasks*.rs` 不得出现第二套进程/spawn 路径

  时钟（F8）
  - 判定函数不得直接读系统时钟，必须经可注入 `Clock`（否则「错过执行 /
    系统时间变化」不可测）

  错过执行（F9）
  - `MissedRunPolicy` 必须是 `TaskDef` 字段，不得退化为运行时常量

  持久化（F10 / 契约 §3.4）
  - 任务落盘必须走 `session::atomic_write`（tmp + rename），不得裸 `fs::write`

  审计（契约 §7）
  - tick / 主循环内禁止 `log_audit`（audit.json 上限 1000 FIFO，每秒 1 条约
    17 分钟即把全部审计冲掉）

  方言（契约 §4.4）
  - cron 仅 5 段；`@daily` / `@reboot` 等宏一律不得支持

  字段兼容（契约 §3.4）
  - `TaskDef` 新增字段一律 `#[serde(default)]`（缺 default 会让历史 tasks.json
    被 `unwrap_or_default()` 整份静默丢弃）

  凭据（契约 §3.3 R-3 / M4 护栏）
  - `ScriptParam.secret` 参数值不得落盘；定义期必须有 `TASK_SECRET_PARAM_FORBIDDEN` 拒绝

  依赖形态（A2 `M4-1.a §3` F-1 裁定 (b) + 契约 §4.4）
  - 不得把 `tokio` 提升为直接依赖（应用层 `std::thread` + `Condvar` 可中断等待）
  - 不得引入 `tokio-cron-scheduler` / `cron` crate（cron 方言自研纯函数，零新依赖）

  退出收口（F7）
  - scheduler 一旦存在，`stop-scheduler` 必须注册进 `ShutdownCoordinator`，
    且其注册位置必须在 `kill-running-scripts` **之前**（先停触发，再杀运行中的）

  审计频率（A10 `R-1`，本卡据此修订契约 §7）
  - `task.run.start` / `task.run.finish` **不得**写进 `audit.json`：最小间隔 60s 的
    任务单任务即 1440 次/天，会冲掉 cap 1000 的审计环形缓冲；执行明细归 `task-runs.json`

  默认态（契约 §3.5 裁定 R-A6-1）
  - 新建任务默认 `enabled = false`（自动执行默认关闭）

本卡默认模式守 **23 个 ACTIVE 码位**，无 pending。

M4-5.e（Integration Fix Wave，Lane A6）：A7 的调度实现已落地，本卡据此补三项
**「真实仓库」裁定码位**，把 A7 登记的两处契约偏离从「注释约定」升级为「机器门禁」：

  O-A7-1 → `SCHED_CRON_DOM_DOW_UNION`（契约 §4.4 补裁定 R-A6-2）
  - cron `day-of-month` 与 `day-of-week` 同时非通配时取**交集（AND）**，不是 Vixie
    标准的并集（OR）。理由：触发更少即 fail-closed，且语义简单可测。若改为 OR，
    用户配置的「每月 1 日且周一」会在**每周一**都触发，属放大而非收敛。

  O-A7-2 → `SCHED_OCCUPIED_NOT_SKIPPED`（契约 §5.2 补裁定 R-A6-3）
  - 同任务 `in_flight` 占用时 `plan_slots` 必须记 `SKIP_REENTRANT` **跳过**，
    不得排队。契约 §5.1（CatchUp 一轮可补多个）与 §5.2（占用则跳过且不排队）
    在满足「同任务不并发」时不可兼得：以跳过为准，超出部分下一轮按 reentrant
    跳过且不消耗重试配额。

  判重真相源 → `SCHED_DEDUP_NOT_LAST_FIRED`（契约 §3.2 / F-A6-4 强化）
  - 原 `SCHED_HISTORY_AS_IDEMPOTENCY` 只按「出现 idempotency 字样」判定，过于
    脆弱。补结构性判定：调度决策模块的产品代码必须引用 `last_fired_at`
    （`tasks.json` 真相源）；改用 `task-runs.json` 历史判重会在环形裁剪后
    重复执行。

关于「产物存在才判」：`scheduler*.rs` / `tasks*.rs` / `TaskDef` 在 M4-5 契约阶段
**尚不存在**（实现归 M4-6 / M4-7，Lane A7），因此全部检测采用「存在才判」：
目标产物不存在时该码位自动降级为 no-op，产物落地后自动生效。**这是刻意的**，
避免未实现就把 pre-merge 染红，同时保证 A7 一落地就被门禁接管。

PENDING 码位：M4-5.d 收尾时 Lane A7 已落地调度实现且通过全部自检，原 8 个
pending 码位全部提升为 ACTIVE（`PENDING_CODES = ()`，不再有 pending）。
`--expect-pending` 用于「转 ACTIVE 完成」后的空集合校验（返回 NONE）。若日后
需要为尚未实现的能力预留门禁，可重新在 `PENDING_CODES` 登记。

默认模式：ACTIVE 不变量全部成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
--self-test 三段式（含变异防呆：坏样本必须真的改动内容，否则按漏检计）：

  1. 好样本 A：真实仓库零违规（证明**无误报**）；
  2. 好样本 B：合成合规参考实现零违规（产物齐备时不误报）；
  3. 双向变异：每个码位至少一个坏样本被检出，且坏样本来自**两个来源**——
     - 合成参考实现的变异（证明规则逻辑本身能命中理想形状）；
     - **真实仓库的变异**（M4-5.e 新增，证明规则对 A7 的真实实现形状同样
       能命中，而不是因为函数名/字段名对不上而永久空转）。
     每个 ACTIVE 码位都必须被「真实仓库变异」覆盖，否则按漏检计。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# ----------------------------- 码位清单 -----------------------------

ACTIVE_CODES: tuple[str, ...] = (
    "SCHED_SECOND_EXEC_PATH",
    "SCHED_CLOCK_NOT_INJECTABLE",
    "SCHED_MISSED_POLICY_FIELD",
    "SCHED_PERSIST_NOT_ATOMIC",
    "SCHED_AUDIT_IN_TICK",
    "SCHED_CRON_MACRO_SUPPORT",
    "SCHED_TASK_DEF_NO_DEFAULT",
    "SCHED_SECRET_PARAM_PERSISTED",
    "SCHED_ENABLED_DEFAULT_TRUE",
    # Batch Implementation Dispatch（board 23:55）明确要求强制的三项
    "SCHED_TOKIO_PROMOTED",
    "SCHED_TOKIO_CRON_DEP",
    "SCHED_SHUTDOWN_NOT_REGISTERED",
    "SCHED_SHUTDOWN_ORDER",
    # A10 R-1（审计冲刷）：契约 §7 据此修订，见 M4-5.d 检查点 §5
    "SCHED_AUDIT_PER_RUN_EVENT",
    # M4-5.d 收尾（Lane A7 已落地且通过自检）：原 pending 全部提升为 ACTIVE，不再有 pending
    "SCHED_CMD_NOT_REGISTERED",
    "SCHED_ACL_ORDER",
    "SCHED_AUDIT_LEAKS_PARAMS",
    "SCHED_RETRY_UNBOUNDED",
    "SCHED_CATCHUP_UNBOUNDED",
    "SCHED_HISTORY_AS_IDEMPOTENCY",
    # M4-5.e（Integration Fix Wave）：A7 实现落地后补的三项真实仓库裁定码位
    "SCHED_CRON_DOM_DOW_UNION",
    "SCHED_DEDUP_NOT_LAST_FIRED",
    "SCHED_OCCUPIED_NOT_SKIPPED",
)

PENDING_CODES: tuple[str, ...] = ()

# ----------------------------- 检测常量 -----------------------------

# 第二套执行路径（F6）。刻意**不含** `spawn(`：该词过于宽泛（如 `std::thread::spawn`
# 是合法且必需的），纳入只会得到恒真告警、稀释真正的 shell 拼接信号（同
# check-command-domain-policy.py 的 SHELL_FORBIDDEN_RUNTIME 口径）。
EXEC_FORBIDDEN = (
    "std::process::Command",
    "Command::new",
    "sh -c",
    "bash -c",
    '"sh", "-c"',
    '"bash", "-c"',
)

# 裸时钟 API（F8）：判定函数内出现即不可测
CLOCK_APIS = (
    "Utc::now()",
    "Local::now()",
    "Instant::now()",
    "SystemTime::now()",
)

# 判定函数：时间语义只经可注入 Clock
JUDGMENT_FNS = (
    "next_fire_after",
    "collect_missed_slots",
    "is_missed",
    "should_fire",
    "validate_trigger",
    "validate_task",
)

# 调度主循环：tick 内禁止写审计
LOOP_FNS = ("tick", "run_loop", "scheduler_loop")

# cron 宏（契约 §4.4：不支持 @reboot / @daily / @yearly …）
CRON_MACRO_RE = re.compile(r"[\"']@(?:reboot|daily|yearly|monthly|weekly|hourly)[\"']")

# `TaskDef` 中**允许**不带 `#[serde(default)]` 的必填字段（契约 §3.1）
TASK_REQUIRED_FIELDS = frozenset(
    {"id", "name", "kind", "target_id", "trigger", "created_at", "updated_at"}
)

# 定时任务命令（A1 冻结的 5 条；契约 §6）
TASK_COMMANDS = ("task_list", "task_add", "task_update", "task_remove", "task_run_now")

# 允许进 `audit.json` 的定时任务事件（契约 §7，经 A10 R-1 修订）：
# 只保留**生命周期与失败**类低频事件；每次执行的 start/finish 一律不进审计。
TASK_AUDIT_EVENTS = (
    "task.add",
    "task.update",
    "task.remove",
    "task.enable",
    "task.run.manual",
    "task.run.skipped",
    "task.run.missed",
    "task.run.reject",
    "task.auto_disabled",
    "task.clock.rewind",
    "task.runs.list",
)

# 每次执行都会产生的事件（A10 R-1）：写进 audit.json 会在数小时内冲掉 cap 1000 的
# 环形缓冲；执行明细应落 `task-runs.json`，审计只留失败/补偿/生命周期。
PER_RUN_AUDIT_EVENTS = ("task.run.start", "task.run.finish")

# 依赖形态：tokio 不得升为直接依赖（A2 M4-1.a §3 F-1 裁定 (b)）
TOKIO_DIRECT_RE = re.compile(r"^\s*tokio\s*=", re.M)
# cron 方言自研，零新依赖（契约 §4.4）
CRON_CRATE_RE = re.compile(r"^\s*(?:tokio-cron-scheduler|cron)\s*=", re.M)

# 审计 detail 禁止出现的片段（沿用 bridge.rs 既有审计脱敏单测口径）
AUDIT_FORBIDDEN = ("values", "argv", "params", "output", "body", "secret")

ACL_ANCHOR = "list_artifact_images"

# cron dom/dow 语义（裁定 R-A6-2 / O-A7-1）：`(self.days >> …)` 与 `(self.weekdays >> …)`
# 两个子句之间的连接符必须是 `&&`（交集）。Rust 惯用写法把 `&&` 放在**行首**，
# 因此取「dow 子句之前最近的那个连接符」判定，兼容行首/行尾两种排版。
CRON_DOM_CLAUSE_RE = re.compile(r"\(self\.days\s*>>.*?\)\s*&\s*1\s*==\s*1")
CRON_DOW_CLAUSE_RE = re.compile(r"\(self\.weekdays\s*>>.*?\)\s*&\s*1\s*==\s*1")
CRON_JOIN_RE = re.compile(r"(&&|\|\|)")

# 判重真相源（契约 §3.2 / F-A6-4）：调度决策模块的产品代码必须引用该字段。
# 用 `tasks.json` 的 `last_fired_at` 判重；`task-runs.json` 只做历史，不参与判重。
DEDUP_TRUTH_FIELD = "last_fired_at"

# O-A7-2 / 裁定 R-A6-3：同任务 in_flight 占用时必须「跳过」而非「排队」。
OCCUPIED_SKIP_TOKEN = "SKIP_REENTRANT"

FIELD_RE = re.compile(r"^\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:")


# ----------------------------- 通用工具 -----------------------------


def strip_comments(source: str) -> str:
    without_blocks = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
    return "\n".join(line.split("//", 1)[0] for line in without_blocks.splitlines())


def rust_fn_body(source: str, name: str) -> str:
    match = re.search(rf"\b(?:pub\s+)?fn\s+{re.escape(name)}\s*\(", source)
    if not match:
        return ""
    depth = 0
    index = match.end() - 1
    while index < len(source):
        if source[index] == "(":
            depth += 1
        elif source[index] == ")":
            depth -= 1
            if depth == 0:
                break
        index += 1
    start = source.find("{", index)
    if start < 0:
        return ""
    depth = 0
    for i in range(start, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[start : i + 1]
    return source[start:]


def rust_struct_body(source: str, name: str) -> str:
    m = re.search(rf"\bstruct\s+{re.escape(name)}\b[^{{]*\{{", source)
    if not m:
        return ""
    start = source.find("{", m.start())
    depth = 0
    for i in range(start, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[start : i + 1]
    return source[start:]


def fields_without_default(body: str, required: frozenset[str]) -> list[str]:
    """返回结构体中缺 `#[serde(default)]` 且非必填的字段名。"""
    missing: list[str] = []
    attrs: list[str] = []
    for raw in body.splitlines():
        line = raw.strip()
        if line.startswith("#["):
            attrs.append(line)
            continue
        m = FIELD_RE.match(raw)
        if m:
            name = m.group(1)
            if "serde(default" not in " ".join(attrs) and name not in required:
                missing.append(name)
            attrs = []
    return missing


# ----------------------------- 检测规则 -----------------------------


def strip_test_fns(source: str) -> str:
    """剔除 `#[test]` / `#[tokio::test]` 标注的测试函数体。

    契约守护的是**产品行为**，不是「拒绝测试向量」。A7 的 `tasks.rs` 在测试里用
    `"@daily"` 作为「应被拒绝」的样例（断言 macro 不被支持）；这种出现不应触发
    `SCHED_CRON_MACRO_SUPPORT`。同理，测试代码里出现 `std::process::Command` /
    裸时钟也算测试桩，不反映运行时产品路径。
    """
    lines = source.splitlines()
    out: list[str] = []
    i, n = 0, len(lines)
    while i < n:
        line = lines[i]
        if re.match(r"^\s*#\[(?:tokio::)?test\b", line):
            # 跳到紧随其后的 fn 签名
            j = i
            while j < n and not re.match(r"^\s*(?:pub\s+)?fn\s+", lines[j]):
                j += 1
            if j >= n:
                break
            # 括号匹配跳过整个函数（含签名与结尾 `}`）
            body = "\n".join(lines[j:])
            start = body.find("{")
            if start < 0:
                i = j + 1
                continue
            depth = 0
            end = len(body)
            for k in range(start, len(body)):
                if body[k] == "{":
                    depth += 1
                elif body[k] == "}":
                    depth -= 1
                    if depth == 0:
                        end = k + 1
                        break
            consumed = body[:end].count("\n") + 1
            i = j + consumed
            continue
        out.append(line)
        i += 1
    return "\n".join(out)


def detect_hits(files: dict) -> dict[str, list[str]]:
    """返回 {码位: [说明]}；ACTIVE 与 PENDING 一并检测，由调用方按模式过滤。"""
    hits: dict[str, list[str]] = {}

    def hit(code: str, detail: str) -> None:
        hits.setdefault(code, []).append(detail)

    domain = files.get("domain_rs", "")
    tasks = files.get("tasks_rs", "")
    sched = files.get("scheduler_rs", "")
    bridge = files.get("bridge_rs", "")
    main_rs = files.get("main_rs", "")
    workspace = files.get("workspace_rs", "")
    acl = files.get("acl_toml", "")
    cargo = files.get("cargo_toml", "")

    tasks_code = strip_comments(tasks)
    sched_code = strip_comments(sched)
    domain_code = strip_comments(domain)

    # 产品行为专用视图：剔除测试函数体（见 strip_test_fns）
    tasks_prod = strip_test_fns(tasks_code)
    sched_prod = strip_test_fns(sched_code)

    taskdef_body = rust_struct_body(domain_code, "TaskDef")
    has_taskdef = bool(taskdef_body)

    # ---- ACTIVE 1) F6：禁止第二套执行路径（仅看产品代码，剔除测试桩）----
    for label, code in (("scheduler", sched_prod), ("tasks", tasks_prod)):
        if not code.strip():
            continue
        for bad in EXEC_FORBIDDEN:
            if bad in code:
                hit("SCHED_SECOND_EXEC_PATH", f"{label}:{bad}")

    # ---- ACTIVE 2) F8：判定函数不得裸读系统时钟 ----
    for label, src in (("scheduler", sched), ("tasks", tasks)):
        if not src.strip():
            continue
        for fn in JUDGMENT_FNS:
            body = rust_fn_body(src, fn)
            if not body:
                continue
            for api in CLOCK_APIS:
                if api in body:
                    hit("SCHED_CLOCK_NOT_INJECTABLE", f"{label}::{fn}:{api}")

    # ---- ACTIVE 3) F9：错过执行策略必须是 TaskDef 字段 ----
    if has_taskdef:
        if not re.search(r"\bpub\s+missed_run_policy\s*:", taskdef_body):
            hit("SCHED_MISSED_POLICY_FIELD", "TaskDef 缺 missed_run_policy 字段")
        if not re.search(r"\bpub\s+enum\s+MissedRunPolicy\b", domain_code):
            hit("SCHED_MISSED_POLICY_FIELD", "domain.rs 缺 MissedRunPolicy 枚举")

    # ---- ACTIVE 4) F10：任务落盘必须原子写 ----
    for label, src in (("tasks", tasks), ("workspace", workspace)):
        if not src.strip():
            continue
        for fn in ("save_tasks_at", "save_tasks"):
            body = rust_fn_body(strip_comments(src), fn)
            if body and "atomic_write" not in body:
                hit("SCHED_PERSIST_NOT_ATOMIC", f"{label}::{fn}")

    # ---- ACTIVE 5) 契约 §7：tick 内禁止写审计 ----
    if sched.strip():
        for fn in LOOP_FNS:
            body = rust_fn_body(sched, fn)
            if body and "log_audit" in body:
                hit("SCHED_AUDIT_IN_TICK", f"scheduler::{fn}")

    # ---- ACTIVE 6) 契约 §4.4：cron 不得支持宏（仅产品代码；拒绝/错误串里的字面量不算支持）----
    cron_reject_re = re.compile(
        r"(not supported|unsupported|reject|invalid|denied|forbidden|不允许|不支持|拒绝)", re.I
    )
    for label, code in (("scheduler", sched_prod), ("tasks", tasks_prod)):
        if not code.strip():
            continue
        for m in CRON_MACRO_RE.finditer(code):
            line = code[max(0, m.start() - 200) : m.end() + 200]
            if cron_reject_re.search(line):
                continue  # 出现在「拒绝/非法」语义中（如错误串、测试样例），非运行时支持
            hit("SCHED_CRON_MACRO_SUPPORT", f"{label}:{m.group(0)}")

    # ---- ACTIVE 7) 契约 §3.4：新增字段一律 serde(default) ----
    if has_taskdef:
        for name in fields_without_default(taskdef_body, TASK_REQUIRED_FIELDS):
            hit("SCHED_TASK_DEF_NO_DEFAULT", f"TaskDef.{name}")

    # ---- ACTIVE 8) 契约 §3.3 R-3：secret 参数值不得落盘 ----
    if has_taskdef and re.search(r"\bpub\s+params\s*:", taskdef_body):
        validator = tasks_code or sched_code
        if validator.strip() and "TASK_SECRET_PARAM_FORBIDDEN" not in validator:
            hit(
                "SCHED_SECRET_PARAM_PERSISTED",
                "任务参数持久化路径缺 TASK_SECRET_PARAM_FORBIDDEN 定义期拒绝（契约 §3.3 R-3）",
            )

    # ---- ACTIVE 9) 契约 §3.5 裁定 R-A6-1：默认 enabled=false ----
    if has_taskdef and re.search(r"\bpub\s+enabled\s*:\s*bool", taskdef_body):
        if 'default_task_enabled' in taskdef_body:
            body = rust_fn_body(domain_code, "default_task_enabled")
            if not body:
                hit("SCHED_ENABLED_DEFAULT_TRUE", "domain.rs 缺 default_task_enabled 函数")
            elif "false" not in body or re.search(r"\btrue\b", body):
                hit("SCHED_ENABLED_DEFAULT_TRUE", "default_task_enabled 未返回 false")

    # ---- ACTIVE 10) 依赖形态：tokio 不得升为直接依赖（A2 M4-1.a §3 F-1 裁定 (b)）----
    # 只扫 `Cargo.toml`（**直接依赖**）：`Cargo.lock` 中本来就有 tokio（tauri 传递依赖），
    # 扫 lock 会得到恒真告警 —— A2 已明确「同步栈」的正确表述是「应用层不使用 async、
    # 不自建 runtime、不新增 tokio 直接依赖」，而**不是**「依赖树里没有 tokio」。
    if cargo.strip():
        if TOKIO_DIRECT_RE.search(cargo):
            hit(
                "SCHED_TOKIO_PROMOTED",
                "Cargo.toml 把 tokio 提升为直接依赖（F-1 裁定 (b)：应用层 std::thread + Condvar）",
            )
        if CRON_CRATE_RE.search(cargo):
            hit(
                "SCHED_TOKIO_CRON_DEP",
                "Cargo.toml 引入 cron/tokio-cron-scheduler（契约 §4.4：5 段方言自研，零新依赖）",
            )

    # ---- ACTIVE 11/12) 退出收口（F7）：scheduler 存在即必须按序注册 ----
    scheduler_exists = "mod scheduler" in main_rs or bool(sched.strip())
    if scheduler_exists:
        if '"stop-scheduler"' not in bridge:
            hit("SCHED_SHUTDOWN_NOT_REGISTERED", "scheduler 已存在但未注册 stop-scheduler")
        else:
            stop_idx = bridge.find('"stop-scheduler"')
            kill_idx = bridge.find('"kill-running-scripts"')
            if kill_idx < 0:
                hit("SCHED_SHUTDOWN_ORDER", "bridge.rs 缺 kill-running-scripts 锚点")
            elif stop_idx > kill_idx:
                hit(
                    "SCHED_SHUTDOWN_ORDER",
                    "stop-scheduler 注册在 kill-running-scripts 之后（F7：须先停触发再杀运行）",
                )

    # ---- ACTIVE 13) A10 R-1：每次执行的 start/finish 不得进 audit.json ----
    # 明细归 task-runs.json（见 scheduler.rs reap_finished append_task_run）。
    # 同时扫描 bridge.rs 与 scheduler.rs：A7 落地时只查 bridge 留下 gap，
    # scheduler 若回退写 task.run.start/finish 会**静默失效**。
    for event in PER_RUN_AUDIT_EVENTS:
        for label, src in (("bridge", bridge), ("scheduler", sched)):
            if f'"{event}"' in src:
                hit(
                    "SCHED_AUDIT_PER_RUN_EVENT",
                    f"{label}:{event} 写入 audit.json（最小间隔 60s 时单任务 1440 次/天，会冲掉 cap 1000 缓冲；明细归 task-runs.json）",
                )

    # ================= PENDING：M4-6 / M4-7 职责 =================

    # ---- PENDING 1) 五命令三处同步（ACL + main.rs 注册 + 来源校验）----
    for cmd in TASK_COMMANDS:
        body = rust_fn_body(bridge, cmd)
        if not body:
            continue  # 尚未实现（归 M4-6.b），存在才判
        if "check_invocation_source" not in body:
            hit("SCHED_CMD_NOT_REGISTERED", f"{cmd}:缺 check_invocation_source")
        if f'"{cmd}"' not in acl:
            hit("SCHED_CMD_NOT_REGISTERED", f"{cmd}:缺 ACL 条目")
        if f"bridge::{cmd}" not in main_rs:
            hit("SCHED_CMD_NOT_REGISTERED", f"{cmd}:缺 main.rs handler 注册")

    # ---- PENDING 2) ACL 顺序：必须插在 list_artifact_images 之前 ----
    anchor = acl.find(f'"{ACL_ANCHOR}"')
    if anchor >= 0:
        for cmd in TASK_COMMANDS:
            idx = acl.find(f'"{cmd}"')
            if idx > anchor:
                hit("SCHED_ACL_ORDER", f"{cmd} 排在 {ACL_ANCHOR} 之后")

    # ---- PENDING 3) 审计 detail 不得泄露参数值 / 输出 ----
    for event in TASK_AUDIT_EVENTS:
        index = bridge.find(f'"{event}"')
        if index < 0:
            continue
        window = bridge[index : index + 260].lower()
        for bad in AUDIT_FORBIDDEN:
            if bad in window:
                hit("SCHED_AUDIT_LEAKS_PARAMS", f"{event}:{bad}")

    # ---- PENDING 4) 重试必须有硬上界 ----
    if "attempt" in sched_code and "max_attempts" not in sched_code:
        hit("SCHED_RETRY_UNBOUNDED", "scheduler 含 attempt 逻辑但缺 max_attempts 上界")

    # ---- PENDING 7) CatchUp 必须有硬上界 ----
    if "catch_up" in sched_code and "catch_up_limit" not in sched_code:
        hit("SCHED_CATCHUP_UNBOUNDED", "scheduler 含 catch_up 逻辑但缺 catch_up_limit 上界")

    # ---- PENDING 8) 判重不得依赖历史（契约 §3.2 / F-A6-4）----
    for label, code in (("scheduler", sched_code), ("tasks", tasks_code)):
        if not code.strip():
            continue
        if re.search(r"\bidempotency", code):
            hit(
                "SCHED_HISTORY_AS_IDEMPOTENCY",
                f"{label}:判重依赖历史幂等键（环形裁剪后会重复执行，真相源应是 last_fired_at）",
            )

    # ================= M4-5.e：真实仓库裁定码位（A7 落地后补） =================

    # ---- ACTIVE 21) 裁定 R-A6-2 / O-A7-1：cron dom/dow 取交集，不得退化为并集 ----
    cron_body = rust_fn_body(tasks_code, "matches_naive")
    if cron_body:
        flat = " ".join(cron_body.splitlines())
        m_dom = CRON_DOM_CLAUSE_RE.search(flat)
        m_dow = CRON_DOW_CLAUSE_RE.search(flat)
        if m_dom and m_dow and m_dow.start() > m_dom.end():
            joins = CRON_JOIN_RE.findall(flat[: m_dow.start()])
            if joins and joins[-1] == "||":
                hit(
                    "SCHED_CRON_DOM_DOW_UNION",
                    "cron dom/dow 取并集（OR）——应取交集 AND：OR 会让「每月1日且周一」"
                    "在每周一都触发，属放大而非 fail-closed（裁定 R-A6-2）",
                )

    # ---- ACTIVE 22) 契约 §3.2 / F-A6-4：判重真相源必须是 last_fired_at ----
    # 结构性判定，补强只认「idempotency 字样」的 SCHED_HISTORY_AS_IDEMPOTENCY。
    if sched_prod.strip() and DEDUP_TRUTH_FIELD not in sched_prod:
        hit(
            "SCHED_DEDUP_NOT_LAST_FIRED",
            f"scheduler 产品代码未引用 {DEDUP_TRUTH_FIELD}（判重真相源；"
            "改用 task-runs.json 历史会在环形裁剪后重复执行）",
        )

    # ---- ACTIVE 23) 裁定 R-A6-3 / O-A7-2：占用时跳过，不排队 ----
    if sched_code.strip() and "catch_up" in sched_code:
        if rust_fn_body(sched_code, "plan_slots") and OCCUPIED_SKIP_TOKEN not in sched_code:
            hit(
                "SCHED_OCCUPIED_NOT_SKIPPED",
                f"plan_slots 在 in_flight 占用时未记 {OCCUPIED_SKIP_TOKEN} 跳过"
                "（裁定 R-A6-3：跳过而非排队，超出 slot 下一轮按 reentrant 跳过且不消耗重试配额）",
            )

    return hits


# ----------------------------- 输入读取 -----------------------------


def _read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return ""


def _glob_concat(root: Path, pattern: str) -> str:
    parts = [_read(p) for p in sorted((root / "src-tauri/src").glob(pattern)) if p.is_file()]
    return "\n".join(parts)


def read_repo(root: Path) -> dict:
    return {
        "domain_rs": _read(root / "src-tauri/src/domain.rs"),
        # 实现归 M4-6 / M4-7：文件可能不存在（→ 空串 → 存在才判）
        "tasks_rs": _glob_concat(root, "tasks*.rs"),
        "scheduler_rs": _glob_concat(root, "scheduler*.rs"),
        "bridge_rs": _read(root / "src-tauri/src/bridge.rs"),
        "main_rs": _read(root / "src-tauri/src/main.rs"),
        "workspace_rs": _read(root / "src-tauri/src/workspace.rs"),
        "acl_toml": _read(root / "src-tauri/permissions/default-commands.toml"),
        # 依赖形态只看**直接依赖**（Cargo.lock 里本来就有 tokio 传递依赖）
        "cargo_toml": _read(root / "src-tauri/Cargo.toml"),
    }


# ----------------------------- 合成参考实现 -----------------------------
# 用途：证明「产物一旦落地，码位不会误报，且能被变异检出」。
# 这些字符串**不写入仓库**，只存在于自检进程内。

DOMAIN_GOOD = '''
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskDef {
    pub id: String,
    pub name: String,
    pub kind: TaskKind,
    pub target_id: String,
    #[serde(default)]
    pub params: HashMap<String, String>,
    #[serde(default = "default_task_enabled")]
    pub enabled: bool,
    pub trigger: TaskTrigger,
    #[serde(default)]
    pub missed_run_policy: MissedRunPolicy,
    #[serde(default = "default_catch_up_limit")]
    pub catch_up_limit: u32,
    #[serde(default)]
    pub misfire_grace_secs: u64,
    #[serde(default)]
    pub retry: RetryPolicy,
    #[serde(default)]
    pub timeout_secs: u32,
    #[serde(default)]
    pub last_fired_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_task_enabled() -> bool {
    false
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicy {
    Skip,
    RunOnce,
    CatchUp,
}
'''

TASKS_GOOD = '''
// 判定函数一律经可注入 Clock 取时间（F8）
pub fn next_fire_after(clock: &dyn Clock, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let _ = clock.now_utc();
    Some(after)
}

pub fn collect_missed_slots(clock: &dyn Clock, task: &TaskDef) -> Vec<DateTime<Utc>> {
    let _ = clock.now_utc();
    Vec::new()
}

pub fn is_missed(now: DateTime<Utc>, slot: DateTime<Utc>, grace_secs: u64) -> bool {
    now.timestamp() - slot.timestamp() > grace_secs as i64
}

pub fn should_fire(now: DateTime<Utc>, slot: DateTime<Utc>) -> bool {
    now >= slot
}

pub fn validate_task(task: &TaskDef) -> Result<(), TaskError> {
    Ok(())
}

pub fn validate_params(meta: &[ScriptParam], params: &HashMap<String, String>) -> Result<(), TaskError> {
    for p in meta {
        if p.secret && params.contains_key(&p.name) {
            return Err(TaskError::Code("TASK_SECRET_PARAM_FORBIDDEN"));
        }
    }
    Ok(())
}

pub fn save_tasks_at(path: &Path, list: &[TaskDef]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    crate::session::atomic_write(path, &content)
}

/// 5 段 cron 命中判定。**dom / dow 取交集（AND）**（裁定 R-A6-2 / O-A7-1）。
pub fn matches_naive(&self, at: &NaiveDateTime) -> bool {
    (self.days >> (at.day() - 1)) & 1 == 1
        && (self.weekdays >> at.weekday().num_days_from_sunday()) & 1 == 1
}
'''

SCHEDULER_GOOD = '''
// 调度循环：只做触发，执行唯一入口是 script_runner（F6）
pub fn tick(tasks: &mut Vec<TaskDef>, table: &ScriptProcessTable) {
    for task in tasks.iter_mut() {
        let _ = table.running_count();
    }
}

pub fn fire(task: &TaskDef, table: &Arc<ScriptProcessTable>) -> Result<String, RunError> {
    crate::script_runner::start_run(table, &task_meta(task), Path::new(""), &HashMap::new(), &roots(), &home(), None, None)
}

fn attempt_bounds(task: &TaskDef) -> u32 {
    SCHED_MAX_ATTEMPTS.min(task.retry.max_attempts)
}

fn catch_up_bounds(task: &TaskDef) -> u32 {
    SCHED_MAX_CATCH_UP.min(task.catch_up_limit)
}

/// O-A7-2（裁定 R-A6-3）：in_flight 占用时记 reentrant 跳过，不排队。
pub fn plan_slots(slots: &[DateTime<Utc>], occupied: bool, catch_up: bool) -> Vec<&'static str> {
    if occupied {
        return vec![crate::tasks::SKIP_REENTRANT];
    }
    if catch_up {
        let _ = slots.len();
    }
    Vec::new()
}

/// 判重真相源 = `last_fired_at`（tasks.json），**不是** task-runs.json 历史。
pub fn dedup_slots(
    slots: &[DateTime<Utc>],
    last_fired_at: Option<DateTime<Utc>>,
    occupied: bool,
) -> Vec<&'static str> {
    if occupied {
        return vec![crate::tasks::SKIP_REENTRANT];
    }
    let _kept: Vec<_> = slots
        .iter()
        .filter(|s| last_fired_at.map(|l| **s > l).unwrap_or(true))
        .collect();
    Vec::new()
}
'''

BRIDGE_GOOD = '''
#[tauri::command]
pub fn task_list(app: AppHandle, webview: tauri::Webview) -> Result<Vec<TaskDef>, String> {
    check_invocation_source(&webview, "task_list", None, &app)?;
    let list = crate::tasks::load(&app);
    workspace::log_audit(&app, "task.runs.list", format!("count={}", list.len()));
    Ok(list)
}

#[tauri::command]
pub fn task_run_now(app: AppHandle, webview: tauri::Webview, id: String) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "task_run_now", None, &app)?;
    check_id(&id, "任务 id")?;
    let run_id = crate::scheduler::fire_now(&app, &id)?;
    // A10 R-1：手工触发是低频用户动作，用 `task.run.manual`；
    // 每次执行的 start/finish 不进 audit.json（明细落 task-runs.json）。
    workspace::log_audit(&app, "task.run.manual", format!("task_id={id} run_id={run_id} trigger=manual"));
    Err("stub".into())
}

pub fn register_shutdown_tasks(app: &AppHandle) -> Result<(), String> {
    let coordinator = app.state::<crate::shutdown::ShutdownCoordinator>();
    coordinator.register("stop-background-workers", move || Ok(()))?;
    coordinator.register("stop-scheduler", move || Ok(()))?;
    coordinator.register("kill-running-scripts", move || Ok(()))?;
    Ok(())
}
'''

MAIN_GOOD = '''
mod scheduler;
mod tasks;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            bridge::task_list,
            bridge::task_run_now,
        ])
        .run(tauri::generate_context!())
        .expect("startup failed");
}
'''

ACL_GOOD = '''[
    "task_list",
    "task_add",
    "task_update",
    "task_remove",
    "task_run_now",
    "list_artifact_images"
]
'''

# 依赖形态样本：无 tokio 直接依赖、无 cron crate（注意 `chrono` 不得被 cron 规则误命中）
CARGO_GOOD = '''[package]
name = "mvp-browser-os"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = ["unstable", "protocol-asset"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
libc = "0.2"
'''


def reference_impl() -> dict:
    return {
        "domain_rs": DOMAIN_GOOD,
        "tasks_rs": TASKS_GOOD,
        "scheduler_rs": SCHEDULER_GOOD,
        "bridge_rs": BRIDGE_GOOD,
        "main_rs": MAIN_GOOD,
        "workspace_rs": "",
        "acl_toml": ACL_GOOD,
        "cargo_toml": CARGO_GOOD,
    }


# --------------------- 真实仓库变异（M4-5.e） ---------------------
# 合成坏样本只能证明「规则逻辑对理想形状能命中」。M4-5.e 追加真实仓库变异：
# 直接对 A7 落地的 scheduler.rs / tasks.rs / domain.rs / bridge.rs / ACL / Cargo.toml
# 注入违规，逐个码位验证门禁确实会拦。这能挡住「函数名/字段名对不上导致码位
# 永久空转」这类**假通过**——产物落地后最危险的不是误报，是静默失效。


def build_real_mutations(repo: dict) -> list[tuple[str, dict, str]]:
    """基于真实仓库内容构造变异样本。

    返回 `[(说明, 变异后的文件字典, 期望命中的码位)]`。每个样本都从**未变异的
    原始仓库**出发，互不污染；变异未真正改动内容即按漏检计（变异防呆）。
    """
    samples: list[tuple[str, dict, str]] = []

    def add(desc: str, key: str, fn, expect: str) -> None:
        before = repo.get(key, "")
        after = fn(before)
        if after == before:
            print(f"self-test FAIL: 真实仓库变异「{desc}」未改动 {key}（变异失配，按漏检计）")
            sys.exit(1)
        mutated = dict(repo)
        mutated[key] = after
        samples.append((desc, mutated, expect))

    def inject_into_fn(src: str, fn_name: str, snippet: str) -> str:
        """把 `snippet` 插进指定函数体的末尾（在收尾 `}` 之前）。"""
        body = rust_fn_body(src, fn_name)
        if not body or not body.endswith("}"):
            return src
        return src.replace(body, body[:-1] + snippet + "\n}", 1)

    # ---- 执行通道 / 时钟 ----
    add(
        "scheduler 自建进程执行路径",
        "scheduler_rs",
        lambda s: s + "\nfn smuggle(c: &str) { let _ = std::process::Command::new(c); }\n",
        "SCHED_SECOND_EXEC_PATH",
    )
    add(
        "next_fire_after 裸读系统时钟",
        "tasks_rs",
        lambda s: inject_into_fn(s, "next_fire_after", "    let _ = Utc::now();"),
        "SCHED_CLOCK_NOT_INJECTABLE",
    )

    # ---- 契约字段 ----
    add(
        "删除 TaskDef.missed_run_policy 字段",
        "domain_rs",
        lambda s: s.replace("    pub missed_run_policy: MissedRunPolicy,\n", "", 1),
        "SCHED_MISSED_POLICY_FIELD",
    )
    add(
        "save_tasks_at 退回裸写（去掉 atomic_write）",
        "tasks_rs",
        lambda s: s.replace(
            "crate::session::atomic_write(path, &content)",
            "std::fs::write(path, content).map_err(|e| e.to_string())",
            1,
        ),
        "SCHED_PERSIST_NOT_ATOMIC",
    )
    add(
        "TaskDef.timeout_secs 去掉 serde(default)",
        "domain_rs",
        lambda s: s.replace(
            "    /// 0 = 沿用 `script_runner::DEFAULT_TIMEOUT_SECS`(60)；上限 600\n"
            "    #[serde(default)]\n    pub timeout_secs: u32,\n",
            "    pub timeout_secs: u32,\n",
            1,
        ),
        "SCHED_TASK_DEF_NO_DEFAULT",
    )
    add(
        "default_task_enabled 改为返回 true",
        "domain_rs",
        lambda s: s.replace(
            "fn default_task_enabled() -> bool {\n    false\n}",
            "fn default_task_enabled() -> bool {\n    true\n}",
            1,
        ),
        "SCHED_ENABLED_DEFAULT_TRUE",
    )

    # ---- 审计 ----
    add(
        "tick 内写审计（刷爆 audit 上限）",
        "scheduler_rs",
        lambda s: inject_into_fn(s, "tick", '    workspace::log_audit(&app, "task.tick", String::new());'),
        "SCHED_AUDIT_IN_TICK",
    )
    add(
        "每次执行的 task.run.start 写进 audit.json",
        "bridge_rs",
        lambda s: s + '\nlet _ = "task.run.start";\n',
        "SCHED_AUDIT_PER_RUN_EVENT",
    )
    add(
        "task.add 审计 detail 泄露参数值",
        "bridge_rs",
        lambda s: s.replace('        "task.add",', '        "task.add", format!("{:?}", values),', 1),
        "SCHED_AUDIT_LEAKS_PARAMS",
    )

    # ---- 方言 / 凭据 ----
    add(
        "cron 支持 @daily 宏",
        "tasks_rs",
        lambda s: s + '\nconst MACROS: [&str; 1] = ["@daily"];\n',
        "SCHED_CRON_MACRO_SUPPORT",
    )
    add(
        "cron dom/dow 退化为并集 OR",
        "tasks_rs",
        lambda s: s.replace(
            "&& (self.weekdays >> weekday) & 1 == 1",
            "|| (self.weekdays >> weekday) & 1 == 1",
            1,
        ),
        "SCHED_CRON_DOM_DOW_UNION",
    )
    add(
        "去掉 secret 参数的定义期拒绝",
        "tasks_rs",
        lambda s: s.replace("TASK_SECRET_PARAM_FORBIDDEN", "TASK_PARAM_ACCEPTED"),
        "SCHED_SECRET_PARAM_PERSISTED",
    )

    # ---- 依赖形态 ----
    add(
        "Cargo.toml 把 tokio 提升为直接依赖",
        "cargo_toml",
        lambda s: s + '\ntokio = { version = "1", features = ["full"] }\n',
        "SCHED_TOKIO_PROMOTED",
    )
    add(
        "Cargo.toml 引入 cron crate",
        "cargo_toml",
        lambda s: s + '\ncron = "0.12"\n',
        "SCHED_TOKIO_CRON_DEP",
    )

    # ---- 退出收口 ----
    add(
        "bridge.rs 未注册 stop-scheduler",
        "bridge_rs",
        lambda s: s.replace('coordinator.register("stop-scheduler"', 'coordinator.register("stop-other"', 1),
        "SCHED_SHUTDOWN_NOT_REGISTERED",
    )

    def _move_stop_after_kill(s: str) -> str:
        moved = s.replace('coordinator.register("stop-scheduler"', 'coordinator.register("stop-moved"', 1)
        return moved.replace(
            'coordinator.register("kill-running-scripts"',
            'coordinator.register("kill-running-scripts", move || {});\n'
            '        coordinator.register("stop-scheduler"',
            1,
        )

    add(
        "stop-scheduler 排到 kill-running-scripts 之后",
        "bridge_rs",
        _move_stop_after_kill,
        "SCHED_SHUTDOWN_ORDER",
    )

    # ---- 命令三处同步 / ACL ----
    add(
        "task_list 未进 ACL",
        "acl_toml",
        lambda s: s.replace('    "task_list",\n', "", 1),
        "SCHED_CMD_NOT_REGISTERED",
    )
    add(
        "task_list 排到 list_artifact_images 之后",
        "acl_toml",
        lambda s: s.replace('    "task_list",\n', "", 1).replace(
            '    "list_artifact_images"',
            '    "list_artifact_images",\n    "task_list"',
            1,
        ),
        "SCHED_ACL_ORDER",
    )

    # ---- 上界 / 判重 / 占用 ----
    add(
        "重试去掉 max_attempts 硬上界",
        "scheduler_rs",
        lambda s: s.replace("max_attempts", "attempt_cap"),
        "SCHED_RETRY_UNBOUNDED",
    )
    add(
        "CatchUp 去掉 catch_up_limit 硬上界",
        "scheduler_rs",
        lambda s: s.replace("catch_up_limit", "catch_up_cap"),
        "SCHED_CATCHUP_UNBOUNDED",
    )
    add(
        "判重改用历史幂等键",
        "scheduler_rs",
        lambda s: s + '\nlet idempotency_key = format!("{}:{}", task.id, 0);\n',
        "SCHED_HISTORY_AS_IDEMPOTENCY",
    )
    add(
        "判重不再引用 last_fired_at（改用历史）",
        "scheduler_rs",
        lambda s: s.replace("last_fired_at", "last_run_ts"),
        "SCHED_DEDUP_NOT_LAST_FIRED",
    )
    add(
        "占用时改为排队而非跳过",
        "scheduler_rs",
        lambda s: s.replace("SKIP_REENTRANT", "SKIP_QUEUED"),
        "SCHED_OCCUPIED_NOT_SKIPPED",
    )

    return samples


# ----------------------------- 自检 -----------------------------


def run_self_test(root: Path) -> int:
    failures: list[str] = []

    # 0) 码位集合自身一致
    overlap = set(ACTIVE_CODES) & set(PENDING_CODES)
    if overlap:
        failures.append(f"码位重复定义（ACTIVE ∩ PENDING）：{sorted(overlap)}")

    # 1) 好样本 A：真实仓库（调度产物已由 A7 落地，且必须零违规 → 证明无误报）
    real = detect_hits(read_repo(root))
    if real:
        failures.append(f"真实仓库存在违规（应为空）：{sorted(real)}")

    # 2) 好样本 B：合成参考实现（产物齐备 → 仍必须零违规，证明无误报）
    good = reference_impl()
    ref = detect_hits(good)
    if ref:
        failures.append(f"合成参考实现存在违规（应为空，说明码位误报）：{sorted(ref)}")

    samples: list[tuple[str, dict, str, str]] = []

    def mutate(**kw) -> dict:
        d = dict(good)
        d.update(kw)
        return d

    def add(desc: str, mutated: dict, key: str, expect: str) -> None:
        """登记坏样本，并做**变异防呆**：内容必须真的改动，否则按漏检计。"""
        if mutated.get(key, "") == good.get(key, ""):
            print(f"self-test FAIL: 坏样本「{desc}」未改动任何内容（变异失配，按漏检计）")
            sys.exit(1)
        samples.append((desc, mutated, key, expect))

    # ---- ACTIVE 坏样本（9）----

    add(
        "调度器自建进程执行路径",
        mutate(scheduler_rs=good["scheduler_rs"] + '\nfn smuggle(c: &str) { let _ = std::process::Command::new(c); }\n'),
        "scheduler_rs",
        "SCHED_SECOND_EXEC_PATH",
    )

    add(
        "判定函数裸读系统时钟",
        mutate(tasks_rs=good["tasks_rs"].replace(
            "    let _ = clock.now_utc();\n    Some(after)",
            "    let now = Utc::now();\n    Some(now + after)")),
        "tasks_rs",
        "SCHED_CLOCK_NOT_INJECTABLE",
    )

    add(
        "TaskDef 去掉 missed_run_policy 字段",
        mutate(domain_rs=good["domain_rs"].replace(
            "    #[serde(default)]\n    pub missed_run_policy: MissedRunPolicy,\n", "")),
        "domain_rs",
        "SCHED_MISSED_POLICY_FIELD",
    )

    add(
        "任务落盘退回裸 fs::write",
        mutate(tasks_rs=good["tasks_rs"].replace(
            "crate::session::atomic_write(path, &content)",
            "std::fs::write(path, content).map_err(|e| e.to_string())")),
        "tasks_rs",
        "SCHED_PERSIST_NOT_ATOMIC",
    )

    add(
        "tick 内写审计（刷爆 audit 上限）",
        mutate(scheduler_rs=good["scheduler_rs"].replace(
            "        let _ = table.running_count();",
            '        workspace::log_audit(&app, "task.tick", String::new());')),
        "scheduler_rs",
        "SCHED_AUDIT_IN_TICK",
    )

    add(
        "cron 支持 @daily / @hourly 宏",
        mutate(tasks_rs=good["tasks_rs"] + '\nconst MACROS: [&str; 2] = ["@daily", "@hourly"];\n'),
        "tasks_rs",
        "SCHED_CRON_MACRO_SUPPORT",
    )

    add(
        "TaskDef 新增字段缺 serde(default)",
        mutate(domain_rs=good["domain_rs"].replace(
            "    #[serde(default)]\n    pub timeout_secs: u32,\n",
            "    pub timeout_secs: u32,\n")),
        "domain_rs",
        "SCHED_TASK_DEF_NO_DEFAULT",
    )

    add(
        "secret 参数值可被落盘（缺定义期拒绝）",
        mutate(tasks_rs=good["tasks_rs"].replace(
            'return Err(TaskError::Code("TASK_SECRET_PARAM_FORBIDDEN"));',
            "continue;")),
        "tasks_rs",
        "SCHED_SECRET_PARAM_PERSISTED",
    )

    add(
        "新建任务默认 enabled 改为 true",
        mutate(domain_rs=good["domain_rs"].replace(
            "fn default_task_enabled() -> bool {\n    false\n}",
            "fn default_task_enabled() -> bool {\n    true\n}")),
        "domain_rs",
        "SCHED_ENABLED_DEFAULT_TRUE",
    )

    # ---- ACTIVE 坏样本（Batch Implementation Dispatch 追加 3 项）----

    add(
        "tokio 升为直接依赖（违反 F-1 裁定 (b)）",
        mutate(cargo_toml=good["cargo_toml"] + '\ntokio = { version = "1", features = ["full"] }\n'),
        "cargo_toml",
        "SCHED_TOKIO_PROMOTED",
    )

    add(
        "引入 tokio-cron-scheduler（违反自研方言口径）",
        mutate(cargo_toml=good["cargo_toml"] + '\ntokio-cron-scheduler = "0.13"\n'),
        "cargo_toml",
        "SCHED_TOKIO_CRON_DEP",
    )

    add(
        "每次执行的 start/finish 写进 audit.json（A10 R-1）",
        mutate(bridge_rs=good["bridge_rs"].replace('"task.run.manual"', '"task.run.start"')),
        "bridge_rs",
        "SCHED_AUDIT_PER_RUN_EVENT",
    )

    # ---- PENDING 坏样本（6）----

    add(
        "task_list 未进 ACL",
        mutate(acl_toml=good["acl_toml"].replace('    "task_list",\n', "")),
        "acl_toml",
        "SCHED_CMD_NOT_REGISTERED",
    )

    add(
        "task_* 插在 list_artifact_images 之后",
        mutate(acl_toml='[\n    "list_artifact_images",\n    "task_list",\n    "task_run_now"\n]\n'),
        "acl_toml",
        "SCHED_ACL_ORDER",
    )

    add(
        "task.run.start 审计泄露参数值",
        mutate(bridge_rs=good["bridge_rs"].replace(
            'format!("task_id={id} run_id={run_id} trigger=manual")',
            'format!("task_id={id} params={:?}", values)')),
        "bridge_rs",
        "SCHED_AUDIT_LEAKS_PARAMS",
    )

    add(
        "scheduler 已存在但未注册 stop-scheduler",
        mutate(bridge_rs=good["bridge_rs"].replace(
            '    coordinator.register("stop-scheduler", move || Ok(()))?;\n', "")),
        "bridge_rs",
        "SCHED_SHUTDOWN_NOT_REGISTERED",
    )

    add(
        "stop-scheduler 排在 kill-running-scripts 之后",
        mutate(bridge_rs=good["bridge_rs"].replace(
            '    coordinator.register("stop-scheduler", move || Ok(()))?;\n'
            '    coordinator.register("kill-running-scripts", move || Ok(()))?;',
            '    coordinator.register("kill-running-scripts", move || Ok(()))?;\n'
            '    coordinator.register("stop-scheduler", move || Ok(()))?;')),
        "bridge_rs",
        "SCHED_SHUTDOWN_ORDER",
    )

    add(
        "重试无上界（去掉 max_attempts）",
        mutate(scheduler_rs=good["scheduler_rs"].replace(
            "SCHED_MAX_ATTEMPTS.min(task.retry.max_attempts)",
            "SCHED_MAX_ATTEMPTS.min(task.retry.attempts)")),
        "scheduler_rs",
        "SCHED_RETRY_UNBOUNDED",
    )

    add(
        "CatchUp 无上界（去掉 catch_up_limit）",
        mutate(scheduler_rs=good["scheduler_rs"].replace(
            "SCHED_MAX_CATCH_UP.min(task.catch_up_limit)",
            "SCHED_MAX_CATCH_UP.min(task.catch_up)")),
        "scheduler_rs",
        "SCHED_CATCHUP_UNBOUNDED",
    )

    add(
        "判重改用历史幂等键（裁剪后会重复执行）",
        mutate(scheduler_rs=good["scheduler_rs"] + '\nlet idempotency_key = format!("{}:{}", task.id, 0);\n'),
        "scheduler_rs",
        "SCHED_HISTORY_AS_IDEMPOTENCY",
    )

    # ---- M4-5.e 坏样本（3）：真实仓库裁定码位 ----

    add(
        "cron dom/dow 退化为并集 OR（O-A7-1 / 裁定 R-A6-2）",
        mutate(tasks_rs=good["tasks_rs"].replace(
            "        && (self.weekdays >> at.weekday().num_days_from_sunday()) & 1 == 1",
            "        || (self.weekdays >> at.weekday().num_days_from_sunday()) & 1 == 1")),
        "tasks_rs",
        "SCHED_CRON_DOM_DOW_UNION",
    )

    add(
        "判重不再引用 last_fired_at（改用历史）",
        mutate(scheduler_rs=good["scheduler_rs"].replace("last_fired_at", "last_run_ts")),
        "scheduler_rs",
        "SCHED_DEDUP_NOT_LAST_FIRED",
    )

    add(
        "占用时改为排队而非跳过（O-A7-2 / 裁定 R-A6-3）",
        mutate(scheduler_rs=good["scheduler_rs"].replace(
            "return vec![crate::tasks::SKIP_REENTRANT];", 'return vec!["queued"];')),
        "scheduler_rs",
        "SCHED_OCCUPIED_NOT_SKIPPED",
    )

    # ---- 执行坏样本（合成参考实现变异）----
    covered: set[str] = set()
    for desc, mutated, _key, expect in samples:
        got = detect_hits(mutated)
        if expect in got:
            covered.add(expect)
        else:
            failures.append(f"坏样本「{desc}」未检出 {expect}（实得 {sorted(got)}）")

    # ---- M4-5.e：真实仓库变异（证明门禁对 A7 实际产物形状同样能命中，不空转）----
    real_repo = read_repo(root)
    real_samples = build_real_mutations(real_repo)
    real_covered: set[str] = set()
    for desc, mutated, expect in real_samples:
        got = detect_hits(mutated)
        if expect in got:
            real_covered.add(expect)
        else:
            failures.append(f"真实仓库变异「{desc}」未检出 {expect}（实得 {sorted(got)}）")

    # 每个 ACTIVE 码位都必须被「真实仓库变异」覆盖（挡住假通过）
    real_uncovered = sorted(set(ACTIVE_CODES) - real_covered)
    if real_uncovered:
        failures.append(f"以下 ACTIVE 码位没有任何真实仓库变异覆盖（可能永久空转）：{real_uncovered}")

    # 每个码位都必须至少被一个合成坏样本覆盖（ACTIVE + PENDING 全覆盖）
    uncovered = sorted((set(ACTIVE_CODES) | set(PENDING_CODES)) - covered)
    if uncovered:
        failures.append(f"以下码位没有任何合成坏样本覆盖：{uncovered}")

    if failures:
        print("SCHED_SELF_TEST_RESULT=FAIL")
        for f in failures:
            print(f"  x {f}")
        return 1

    print(
        f"SCHED_SELF_TEST_RESULT=PASS: 2 好样本零违规（真实仓库 + 合成参考实现）"
        f" + {len(samples)} 个合成坏样本全检出 + {len(real_samples)} 个真实仓库变异全检出"
        f"（含变异防呆）；ACTIVE={len(ACTIVE_CODES)} PENDING={len(PENDING_CODES)}"
    )
    return 0


# ----------------------------- 入口 -----------------------------


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--self-test", action="store_true", help="好样本 + 坏样本双向自检（含变异防呆）")
    ap.add_argument(
        "--expect-pending",
        action="store_true",
        help="验证 pending 码位仍未实现；一旦可检出即应转入默认判定",
    )
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[1]

    if args.self_test:
        return run_self_test(root)

    hits = detect_hits(read_repo(root))

    if args.expect_pending:
        pending_hits = [c for c in PENDING_CODES if c in hits]
        if pending_hits:
            print("SCHED_PENDING_RESULT=FAIL")
            for code in pending_hits:
                for detail in hits[code]:
                    print(f"  {code}:{detail}")
            return 1
        print(f"SCHED_PENDING_RESULT=NONE（{len(PENDING_CODES)} 个 pending 码位均未实现）")
        return 0

    active_hits = sorted(c for c in hits if c in ACTIVE_CODES)
    if active_hits:
        print("SCHED_POLICY_RESULT=FAIL")
        for code in active_hits:
            for detail in hits[code]:
                print(f"  {code}:{detail}")
        return 1
    print(f"scheduler policy: all invariants hold（ACTIVE={len(ACTIVE_CODES)}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
