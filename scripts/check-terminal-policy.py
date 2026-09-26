#!/usr/bin/env python3
"""Expose the M3.a/M3.c terminal invariants as a reproducible fixture.

契约来源：logs/checkpoints/M3-20260905-2030.md（F1~F10）+ M3.c（WBS M3-4：
终端临时历史 40 条、resize 静默窗口）。

M3.a 是 #9（fileterm 借鉴）的**内核收口**：PTY 输出不能再是「读 4 KiB 就
emit 一次全局事件」，必须是 worker → sync_channel → pump → sink 的三段式管道，
且 resize 真实生效、进程组整体回收、退避有停止条件。

Phase 8E / Train D（Terminal capability 隔离）同步：
  - 前端读取路径随物理迁移更新：useSystemStore → capabilities/terminal/state/useTerminalStore.ts；
    TerminalPane.vue / useTerminalResize.ts → capabilities/terminal/ui/**。
  - `TERM_HISTORY_CLEAR_MISSING` 检测器重基线化：原实现统计 `clearTermHistory()` 文本出现 ≥3 次，
    而 per-pane 重构（commit 2a96cb1）后真实清理点是 `clearTermHistory(id)`（调用）
    + `termHistories.delete(id)`（移除桶）。新检测要求**两者同在**，比原文本计数更严（不变量未放宽）。
    行为级证明见 scripts/check-terminal-owners.mjs（真实 store：killTerm → 回放为空）。

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
--self-test：好样本零违规 + 每个不变量至少一个变异坏样本被检出（含变异防呆）。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


def detect_violations(files: dict) -> list[str]:
    v: list[str] = []
    term = files.get("terminal_rs", "")
    bridge = files.get("bridge_rs", "")
    main = files.get("main_rs", "")
    cargo = files.get("cargo_toml", "")
    acl = files.get("acl_toml", "")
    ts_bridge = files.get("bridge_ts", "")
    store = files.get("system_store_ts", "")
    pane = files.get("terminal_pane_vue", "")
    resize_ts = files.get("terminal_resize_ts", "")

    # ---- F1：mpsc 队列 + worker 非阻塞（sync_channel / try_send）----
    if "sync_channel" not in term or "try_send" not in term:
        v.append("TERM_MPSC_MISSING:terminal.rs 缺 sync_channel/try_send（worker 必须非阻塞）")

    # ---- F2：丢弃计数与上报（DropCounter / notify_drops）----
    if "DropCounter" not in term or "notify_drops" not in term:
        v.append("TERM_DROP_MISSING:terminal.rs 缺 DropCounter/notify_drops（背压丢弃须可观测）")

    # ---- F3：聚合窗口与批量上限 ----
    if "TERM_FLUSH_INTERVAL_MS" not in term or "should_flush" not in term:
        v.append("TERM_MERGE_MISSING:terminal.rs 缺聚合窗口常量/should_flush（F3 合并）")

    # ---- F4：sink 抽象 + 双入口（Event 兼容 / Channel 单播）----
    if "trait TermOutput" not in term:
        v.append("TERM_SINK_MISSING:terminal.rs 缺 TermOutput 抽象（F4）")
    if "struct EventSink" not in term or "struct ChannelSink" not in term:
        v.append("TERM_SINK_MISSING:terminal.rs 缺 EventSink/ChannelSink 双 sink")
    if "fn term_spawn_channel" not in bridge:
        v.append("TERM_SPAWN_MISSING:bridge.rs 缺 term_spawn_channel 命令")
    if '"term_spawn_channel"' not in acl:
        v.append("TERM_ACL_MISSING:default-commands.toml 缺 term_spawn_channel（红线：新命令必须入 ACL）")

    # ---- F4：前端只能走 Channel 单播入口，不得回到 term_spawn ----
    # 注意：`"term_spawn_channel"` 不会被下面的精确匹配命中（右侧引号紧贴），
    # 故该规则只拦旧 Event 广播入口。
    if re.search(r'"term_spawn"', ts_bridge) or re.search(r'"term_spawn"', store):
        v.append("TERM_FRONTEND_OLD_SPAWN:前端调用旧 term_spawn（Event 广播）而非 term_spawn_channel")
    if re.search(r"\btermSpawn\(\)", store) or re.search(r"\btermSpawn\(\)", ts_bridge):
        v.append("TERM_FRONTEND_OLD_SPAWN:前端仍使用 termSpawn()（须改 termSpawnChannel）")
    if "createTermChannel" not in ts_bridge or "termSpawnChannel" not in ts_bridge:
        v.append("TERM_FRONTEND_CHANNEL_MISSING:bridge.ts 缺 createTermChannel/termSpawnChannel")
    if "termSpawnChannel" not in store:
        v.append("TERM_STORE_CHANNEL_MISSING:useSystemStore 未走 Channel 入口")

    # ---- F5：resize 真实生效（不得回到空实现）----
    if "let _ = (app, id, cols, rows)" in bridge:
        v.append("TERM_RESIZE_STUB:term_resize 回到空实现（F5 禁止）")
    if "terminal::resize(" not in bridge:
        v.append("TERM_RESIZE_MISSING:term_resize 未调用 terminal::resize")
    if ".resize(PtySize" not in term:
        v.append("TERM_MASTER_RESIZE_MISSING:terminal.rs 未调用 master.resize(PtySize)")

    # ---- F6：进程组回收（killpg + 存活轮询 + SIGTERM/SIGKILL）----
    for token, code in (
        ("killpg", "TERM_GROUP_KILL_MISSING"),
        ("process_group_alive", "TERM_GROUP_KILL_MISSING"),
        ("SIGTERM", "TERM_GROUP_KILL_MISSING"),
        ("SIGKILL", "TERM_GROUP_KILL_MISSING"),
    ):
        if token not in term:
            v.append(f"{code}:terminal.rs 缺进程组回收要素 {token}")
    if "session.child.kill()" in bridge:
        v.append("TERM_LIFECYCLE_KILL:生命周期仍用 child.kill()（只杀直接子进程，留孤儿）")
    if "terminal::terminate_session" not in bridge:
        v.append("TERM_LIFECYCLE_MISSING:bridge.rs 未统一走 terminal::terminate_session")
    # 复核整改 P2：终止最长约 4 s（进程组宽限 + 线程回收），必须先放表锁再终止，
    # 否则 term_write / term_resize / term_spawn 会一起卡在互斥量上。
    if "terminal::terminate_session(&mut session)" in bridge and "drop(terms)" not in bridge:
        v.append(
            "TERM_KILL_LOCK_HELD:term_kill 持 terminals 锁执行 terminate_session（最长约 4 s 阻塞其它终端 IPC）"
        )

    # ---- F7：关闭协议（stop 标志 + 线程回收）----
    if "Arc<AtomicBool>" not in term or "stop" not in term:
        v.append("TERM_STOP_MISSING:terminal.rs 缺 stop 标志（F7 关闭协议）")
    if "is_finished" not in term:
        v.append("TERM_JOIN_MISSING:terminal.rs 缺线程结束判定（worker/pump 不得泄漏）")

    # ---- F8：退避与停止条件 ----
    for token in (
        "TERM_RETRY_BASE_MS",
        "TERM_RETRY_MAX_BACKOFF_MS",
        "TERM_RETRY_BUDGET_MS",
        "fn next_backoff_ms",
        "fn within_retry_budget",
    ):
        if token not in term:
            v.append(f"TERM_BACKOFF_MISSING:terminal.rs 缺退避要素 {token}")

    # ---- F2/F3：测量模式直通（不合并、不丢帧），保证 M0-0.b 基线可比 ----
    if "term_measure_mode" not in bridge or 'driver == "term-throughput"' not in bridge:
        v.append("TERM_MEASURE_MISSING:bridge.rs 缺测量模式判定（直通开关）")
    if "!measure" not in term:
        v.append("TERM_MEASURE_BYPASS:terminal.rs 未按测量模式关闭丢弃/合并")

    # ---- F9：零新依赖（不引 tokio 等运行时）----
    if re.search(r"^\s*tokio\s*=", cargo, re.M):
        v.append("TERM_TOKIO_DEP:Cargo.toml 引入 tokio（F9 零新依赖）")
    if "crossbeam" in cargo:
        v.append("TERM_TOKIO_DEP:Cargo.toml 引入 crossbeam（F9 零新依赖）")

    # ---- F10：Channel 必须从 @tauri-apps/api/core 子路径导入 ----
    if not re.search(
        r"import\s*\{[^}]*\bChannel\b[^}]*\}\s*from\s*\"@tauri-apps/api/core\"", ts_bridge
    ):
        v.append("TERM_CHANNEL_IMPORT:Channel 未从 @tauri-apps/api/core 子路径导入")

    # ---- 命令注册（历史坑：注册缺失会被静默拒绝）----
    if "bridge::term_spawn_channel," not in main:
        v.append("TERM_HANDLER_MISSING:main.rs 未注册 term_spawn_channel")

    # ---- F10：前端 resize 静默窗口（M3.c 由防抖升级为「去重 + 静默 + 硬上界」）----
    # 精确到**调用形态**（`termResize(`）：probe 打点里的 `termResize.req` 是标签字符串，
    # 不构成「已上报 resize」的证据，不得让它掩盖真实调用被移除。
    if re.search(r"\btermResize\s*\(", pane) is None:
        v.append("TERM_PANE_RESIZE_MISSING:TerminalPane 未调用 termResize（F5 前端侧）")
    if "useTerminalResize" not in pane:
        v.append("TERM_RESIZE_QUIET_WIRING:TerminalPane 未接线 useTerminalResize（静默窗口不可绕过）")
    if "resize.dispose()" not in pane:
        v.append("TERM_RESIZE_DISPOSE_MISSING:TerminalPane 卸载未清静默定时器（用例 N5）")
    # 静默窗口常量（fileterm TERMINAL_RESIZE_OUTPUT_QUIET_MS 同族）
    if not re.search(r"TERM_RESIZE_QUIET_MS\s*=\s*\d+", resize_ts):
        v.append("TERM_RESIZE_QUIET_MISSING:useTerminalResize 缺静默窗口常量 TERM_RESIZE_QUIET_MS")
    # 硬上界：纯静默窗口会导致「持续慢拖时终端一直不重排」，必须有兜底下发
    if not re.search(r"TERM_RESIZE_MAX_WAIT_MS\s*=\s*\d+", resize_ts):
        v.append("TERM_RESIZE_MAXWAIT_MISSING:useTerminalResize 缺硬上界 TERM_RESIZE_MAX_WAIT_MS")
    # 去重：尺寸未变零下发（ResizeObserver 抖动不得变成 IPC）
    if "lastCols" not in resize_ts or "lastRows" not in resize_ts:
        v.append("TERM_RESIZE_DEDUP_MISSING:useTerminalResize 缺尺寸去重（同尺寸重复 invoke）")

    # ---- M3.c（WBS M3-4 E1）：终端临时历史 40 条 ----
    if not re.search(r"TERM_TEMP_HISTORY_LIMIT\s*=\s*40", store):
        v.append("TERM_HISTORY_LIMIT_MISSING:useSystemStore 缺临时历史上限 TERM_TEMP_HISTORY_LIMIT=40")
    if "pushTermHistory" not in store:
        v.append("TERM_HISTORY_RECORD_MISSING:useSystemStore 未登记输出进临时历史（历史上限形同虚设）")
    if "replayTermHistory" not in pane:
        v.append("TERM_HISTORY_REPLAY_MISSING:TerminalPane 未回放临时历史（面板重建后一片空白）")
    # 会话结束必须清空，否则旧会话输出会作为残影回放到新终端。
    # per-pane 语义（2a96cb1 起）：清理 = 调用 clearTermHistory(<id>) 置空该 pane 的历史
    # + termHistories.delete(<id>) 移除该 pane 的桶。要求两者同在（缺任一即视为未清理）。
    _clear_call = re.search(r"clearTermHistory\(\s*[A-Za-z_]\w*\s*\)", store)
    _bucket_drop = re.search(r"termHistories\.delete\(", store)
    if _clear_call is None or _bucket_drop is None:
        v.append(
            "TERM_HISTORY_CLEAR_MISSING:临时历史未在会话结束/重启时清空（跨会话残影）"
            f"［clearTermHistory(id)={bool(_clear_call)} termHistories.delete(id)={bool(_bucket_drop)}］"
        )
    # 隐私红线：仅内存，不落盘、不进审计、后端不留存
    for line in store.splitlines():
        if "localStorage" in line and "termHistory" in line:
            v.append("TERM_HISTORY_PERSIST:临时历史被写入 localStorage（红线：仅会话内内存，不落盘）")
    for line in bridge.splitlines():
        if "log_audit" in line and re.search(r"term", line, re.I):
            v.append("TERM_HISTORY_AUDIT:终端输出/历史进入审计日志（红线：高频会刷爆 audit 上限）")
    if re.search(r"term_history|TermHistory|termHistory|TERM_TEMP_HISTORY", term):
        v.append("TERM_HISTORY_BACKEND:terminal.rs 留存输出历史（契约：后端零状态，历史只在前端会话内）")

    return v


def read_repo(root: Path) -> dict:
    def read(rel: str) -> str:
        p = root / rel
        try:
            return p.read_text(encoding="utf-8")
        except OSError:
            return ""

    return {
        "terminal_rs": read("src-tauri/src/capabilities/terminal/terminal.rs"),
        "bridge_rs": read("src-tauri/src/bridge.rs"),
        "main_rs": read("src-tauri/src/main.rs"),
        "cargo_toml": read("src-tauri/Cargo.toml"),
        "acl_toml": read("src-tauri/permissions/default-commands.toml"),
        "bridge_ts": read("src/bridge.ts"),
        # Phase 8E/Train D：Terminal 域迁入能力包（owner 与 UI 均换路径）
        "system_store_ts": read("src/capabilities/terminal/state/useTerminalStore.ts"),
        "terminal_pane_vue": read("src/capabilities/terminal/ui/TerminalPane.vue"),
        "terminal_resize_ts": read("src/capabilities/terminal/ui/useTerminalResize.ts"),
    }


def run_self_test(root: Path) -> int:
    good = read_repo(root)
    base = detect_violations(good)
    if base:
        print("self-test FAIL: 当前仓库自身存在违规（应为空）")
        for x in base:
            print(f"  x {x}")
        return 1

    samples: list[tuple[str, dict, str, str]] = []

    def mutate(**kw) -> dict:
        d = dict(good)
        d.update(kw)
        return d

    def add(desc: str, mutated: dict, key: str, expect: str) -> None:
        if mutated[key] == good[key]:
            print(f"self-test FAIL: 坏样本「{desc}」未改动任何内容（变异失配，按漏检计）")
            sys.exit(1)
        samples.append((desc, mutated, key, expect))

    # 1. worker 回到阻塞/直发（移除 sync_channel + try_send）
    add(
        "移除 mpsc 队列与非阻塞发送",
        mutate(
            terminal_rs=good["terminal_rs"]
            .replace("sync_channel", "zz_sync_zz")
            .replace("try_send", "zz_trysend_zz")
        ),
        "terminal_rs",
        "TERM_MPSC_MISSING",
    )

    # 2. 移除丢弃统计
    add(
        "移除丢弃计数与上报",
        mutate(
            terminal_rs=good["terminal_rs"]
            .replace("DropCounter", "zz_dropzz")
            .replace("notify_drops", "zz_notifyzz")
        ),
        "terminal_rs",
        "TERM_DROP_MISSING",
    )

    # 3. 移除聚合窗口
    add(
        "移除聚合窗口与 should_flush",
        mutate(
            terminal_rs=good["terminal_rs"]
            .replace("TERM_FLUSH_INTERVAL_MS", "zz_flushzz")
            .replace("should_flush", "zz_shouldzz")
        ),
        "terminal_rs",
        "TERM_MERGE_MISSING",
    )

    # 4. 移除 Channel sink（退回只有事件广播）
    add(
        "移除 ChannelSink 单播 sink",
        mutate(terminal_rs=good["terminal_rs"].replace("ChannelSink", "zz_channelsink_zz")),
        "terminal_rs",
        "TERM_SINK_MISSING",
    )

    # 5. ACL 漏登记新命令
    add(
        "ACL 漏登记 term_spawn_channel",
        mutate(acl_toml=good["acl_toml"].replace('    "term_spawn_channel",\n', "")),
        "acl_toml",
        "TERM_ACL_MISSING",
    )

    # 6. 前端退回旧 Event 入口
    add(
        "前端退回 term_spawn（Event 广播）",
        mutate(
            system_store_ts=good["system_store_ts"].replace(
                "await bridge.termSpawnChannel(ch);", "await bridge.termSpawn();"
            )
        ),
        "system_store_ts",
        "TERM_FRONTEND_OLD_SPAWN",
    )

    # 7. resize 回到空实现
    #     锚点随真实代码校正（bridge.rs 现为 `let result = terminal::resize(session, cols, rows);`）；
    #     只替换调用表达式本身，保证「变异确实改动内容」且仍触发 TERM_RESIZE_STUB。
    add(
        "term_resize 回到空实现",
        mutate(
            bridge_rs=good["bridge_rs"].replace(
                "terminal::resize(session, cols, rows)",
                "{ let _ = (app, id, cols, rows); let _ = session; Ok(()) }",
            )
        ),
        "bridge_rs",
        "TERM_RESIZE_STUB",
    )

    # 8. 退回只杀直接子进程
    add(
        "进程组回收被移除（退回 child.kill）",
        mutate(
            terminal_rs=good["terminal_rs"]
            .replace("killpg", "zz_killpg_zz")
            .replace("process_group_alive", "zz_alive_zz")
        ),
        "terminal_rs",
        "TERM_GROUP_KILL_MISSING",
    )

    # 9. 生命周期回到 child.kill
    add(
        "生命周期 kill-terminals 退回 child.kill",
        mutate(
            bridge_rs=good["bridge_rs"].replace(
                "terminal::terminate_session(&mut session)",
                "session.child.kill().map(|_| ())",
            )
        ),
        "bridge_rs",
        "TERM_LIFECYCLE_KILL",
    )

    # 10. 移除关闭协议
    add(
        "移除 stop 标志与线程回收判定",
        mutate(
            terminal_rs=good["terminal_rs"]
            .replace("Arc<AtomicBool>", "zz_atomic_zz")
            .replace("is_finished", "zz_finished_zz")
        ),
        "terminal_rs",
        "TERM_STOP_MISSING",
    )

    # 11. 移除退避上限（可能变成无限重试）
    add(
        "移除退避上限与预算",
        mutate(
            terminal_rs=good["terminal_rs"]
            .replace("TERM_RETRY_MAX_BACKOFF_MS", "zz_cap_zz")
            .replace("within_retry_budget", "zz_budget_zz")
        ),
        "terminal_rs",
        "TERM_BACKOFF_MISSING",
    )

    # 12. 测量模式直通被绕过（吞吐基线失真）
    add(
        "测量模式直通被移除",
        mutate(
            bridge_rs=good["bridge_rs"].replace(
                "term_measure_mode", "zz_measure_zz"
            )
        ),
        "bridge_rs",
        "TERM_MEASURE_MISSING",
    )

    # 13. 引入 tokio（违反零新依赖）
    add(
        "引入 tokio 依赖",
        mutate(cargo_toml=good["cargo_toml"] + '\ntokio = { version = "1", features = ["full"] }\n'),
        "cargo_toml",
        "TERM_TOKIO_DEP",
    )

    # 14. Channel 从顶层导入（Tauri v2 下拿不到）
    add(
        "Channel 从顶层 @tauri-apps/api 导入",
        mutate(
            bridge_ts=good["bridge_ts"].replace(
                'import { invoke, Channel } from "@tauri-apps/api/core";',
                'import { invoke } from "@tauri-apps/api/core";\nimport { Channel } from "@tauri-apps/api";',
            )
        ),
        "bridge_ts",
        "TERM_CHANNEL_IMPORT",
    )

    # 15. 命令未注册
    add(
        "main.rs 未注册 term_spawn_channel",
        mutate(main_rs=good["main_rs"].replace("bridge::term_spawn_channel,\n", "")),
        "main_rs",
        "TERM_HANDLER_MISSING",
    )

    # 16. term_kill 持锁终止（阻塞其它终端 IPC）
    add(
        "term_kill 持表锁执行终止",
        mutate(bridge_rs=good["bridge_rs"].replace("    drop(terms);\n", "")),
        "bridge_rs",
        "TERM_KILL_LOCK_HELD",
    )

    # 17. 前端 resize 上报被移除
    add(
        "TerminalPane 移除 resize 上报",
        mutate(
            terminal_pane_vue=good["terminal_pane_vue"].replace(
                "bridge.termResize", "zz_termresize_zz"
            )
        ),
        "terminal_pane_vue",
        "TERM_PANE_RESIZE_MISSING",
    )

    # 18. 静默窗口常量被移除（拖动时每帧 invoke）
    add(
        "移除 resize 静默窗口常量",
        mutate(
            terminal_resize_ts=good["terminal_resize_ts"].replace(
                "TERM_RESIZE_QUIET_MS", "ZZ_QUIET_ZZ"
            )
        ),
        "terminal_resize_ts",
        "TERM_RESIZE_QUIET_MISSING",
    )

    # 19. 硬上界被移除（持续慢拖时终端一直不重排）
    add(
        "移除 resize 硬上界常量",
        mutate(
            terminal_resize_ts=good["terminal_resize_ts"].replace(
                "TERM_RESIZE_MAX_WAIT_MS", "ZZ_MAXWAIT_ZZ"
            )
        ),
        "terminal_resize_ts",
        "TERM_RESIZE_MAXWAIT_MISSING",
    )

    # 20. 尺寸去重被移除（同尺寸重复下发 IPC）
    add(
        "移除 resize 尺寸去重",
        mutate(terminal_resize_ts=good["terminal_resize_ts"].replace("lastCols", "zz_lcols_zz")),
        "terminal_resize_ts",
        "TERM_RESIZE_DEDUP_MISSING",
    )

    # 21. 静默窗口未接线（TerminalPane 直接防抖而非走 composable）
    add(
        "TerminalPane 未接线 useTerminalResize",
        mutate(
            terminal_pane_vue=good["terminal_pane_vue"].replace(
                "useTerminalResize", "zz_use_resize_zz"
            )
        ),
        "terminal_pane_vue",
        "TERM_RESIZE_QUIET_WIRING",
    )

    # 22. 卸载未清静默定时器（残留回调）
    add(
        "TerminalPane 卸载未 dispose 静默定时器",
        mutate(
            terminal_pane_vue=good["terminal_pane_vue"].replace(
                "resize.dispose()", "zz_no_dispose_zz"
            )
        ),
        "terminal_pane_vue",
        "TERM_RESIZE_DISPOSE_MISSING",
    )

    # 23. 历史上限被放宽/移除（数组无限增长）
    add(
        "移除终端临时历史上限常量",
        mutate(
            system_store_ts=good["system_store_ts"].replace(
                "TERM_TEMP_HISTORY_LIMIT", "ZZ_HISTLIM_ZZ"
            )
        ),
        "system_store_ts",
        "TERM_HISTORY_LIMIT_MISSING",
    )

    # 24. 输出未登记进历史（上限形同虚设）
    add(
        "输出未登记进临时历史",
        mutate(
            system_store_ts=good["system_store_ts"].replace("pushTermHistory", "zz_push_hist_zz")
        ),
        "system_store_ts",
        "TERM_HISTORY_RECORD_MISSING",
    )

    # 25. 面板重建未回放（切回终端一片空白）
    add(
        "TerminalPane 未回放临时历史",
        mutate(
            terminal_pane_vue=good["terminal_pane_vue"].replace(
                "replayTermHistory", "zz_replay_hist_zz"
            )
        ),
        "terminal_pane_vue",
        "TERM_HISTORY_REPLAY_MISSING",
    )

    # 26. 会话结束未清历史（旧会话输出成为新终端残影）
    #     per-pane 语义下真实清理点是 clearTermHistory(id) 调用（2a96cb1 起）。
    add(
        "会话结束未清空临时历史",
        mutate(
            system_store_ts=good["system_store_ts"].replace("    clearTermHistory(id);\n", "", 1)
        ),
        "system_store_ts",
        "TERM_HISTORY_CLEAR_MISSING",
    )

    # 26b. 只置空不移除桶（内存泄漏 + 桶残留）
    add(
        "会话结束只清空不移除历史桶",
        mutate(
            system_store_ts=good["system_store_ts"].replace("    termHistories.delete(id);\n", "", 1)
        ),
        "system_store_ts",
        "TERM_HISTORY_CLEAR_MISSING",
    )

    # 27. 历史被落盘（隐私红线）
    add(
        "临时历史被写入 localStorage",
        mutate(
            system_store_ts=good["system_store_ts"]
            + '\nlocalStorage.setItem("termHistory", JSON.stringify(termHistory));\n'
        ),
        "system_store_ts",
        "TERM_HISTORY_PERSIST",
    )

    # 28. 终端输出进审计（高频会刷爆 audit 上限）
    add(
        "终端输出写入审计日志",
        mutate(
            bridge_rs=good["bridge_rs"] + '\n// mutant\nlet _ = log_audit(&app, "terminal.data", "x");\n'
        ),
        "bridge_rs",
        "TERM_HISTORY_AUDIT",
    )

    # 29. 后端留存输出历史（违背后端零状态契约）
    add(
        "后端 terminal.rs 留存输出历史",
        mutate(terminal_rs=good["terminal_rs"] + "\n// mutant\nlet term_history: Vec<String> = Vec::new();\n"),
        "terminal_rs",
        "TERM_HISTORY_BACKEND",
    )

    failures = 0
    for desc, mutated, _key, expect in samples:
        got = detect_violations(mutated)
        if not any(x.startswith(expect) for x in got):
            print(f"self-test FAIL: 坏样本「{desc}」未检出 {expect}（实得 {got}）")
            failures += 1

    if failures:
        return 1

    print(
        f"self-test OK: 好样本零违规 + {len(samples)} 个坏样本全部检出（含变异防呆）"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--self-test", action="store_true", help="跑好样本 + 坏样本自检（含变异防呆）")
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[1]

    if args.self_test:
        return run_self_test(root)

    violations = detect_violations(read_repo(root))
    if violations:
        for x in violations:
            print(x)
        return 1
    print("terminal pipeline policy: all invariants hold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
