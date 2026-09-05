#!/usr/bin/env python3
"""Expose the M3.a terminal output-pipeline invariants as a reproducible fixture.

契约来源：logs/checkpoints/M3-20260905-2030.md（F1~F10）。

M3.a 是 #9（fileterm 借鉴）的**内核收口**：PTY 输出不能再是「读 4 KiB 就
emit 一次全局事件」，必须是 worker → sync_channel → pump → sink 的三段式管道，
且 resize 真实生效、进程组整体回收、退避有停止条件。

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

    # ---- F10：前端 resize 防抖上报（真实行列到后端）----
    if "termResize" not in pane:
        v.append("TERM_PANE_RESIZE_MISSING:TerminalPane 未调用 termResize（F5 前端侧）")
    if not re.search(r"TERM_RESIZE_DEBOUNCE_MS\s*=\s*\d+", pane):
        v.append("TERM_PANE_DEBOUNCE_MISSING:TerminalPane 缺 resize 防抖常量")

    return v


def read_repo(root: Path) -> dict:
    def read(rel: str) -> str:
        p = root / rel
        try:
            return p.read_text(encoding="utf-8")
        except OSError:
            return ""

    return {
        "terminal_rs": read("src-tauri/src/terminal.rs"),
        "bridge_rs": read("src-tauri/src/bridge.rs"),
        "main_rs": read("src-tauri/src/main.rs"),
        "cargo_toml": read("src-tauri/Cargo.toml"),
        "acl_toml": read("src-tauri/permissions/default-commands.toml"),
        "bridge_ts": read("src/bridge.ts"),
        "system_store_ts": read("src/stores/useSystemStore.ts"),
        "terminal_pane_vue": read("src/components/system/TerminalPane.vue"),
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
    add(
        "term_resize 回到空实现",
        mutate(
            bridge_rs=good["bridge_rs"].replace(
                "    terminal::resize(session, cols, rows)",
                "    let _ = (app, id, cols, rows);\n    let _ = session;\n    Ok(())",
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

    # 17. 前端 resize 防抖被移除
    add(
        "TerminalPane 移除 resize 防抖上报",
        mutate(
            terminal_pane_vue=good["terminal_pane_vue"]
            .replace("TERM_RESIZE_DEBOUNCE_MS = 140", "TERM_RESIZE_DEBOUNCE_MS = zz_zz")
            .replace("bridge.termResize", "zz_termresize_zz")
        ),
        "terminal_pane_vue",
        "TERM_PANE_RESIZE_MISSING",
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
