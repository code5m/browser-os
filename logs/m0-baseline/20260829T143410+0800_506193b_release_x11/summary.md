# M0-1.b resource verification driver — 20260829T143410+0800_506193b_release_x11

> 检查点：M0-1.b；契约：V1.0；脚本：M0-1.b-1；场景：M0-1.b-v1
> 生成时间：2026-08-29 14:34:13 +0800
> 机器判定源：summary.json（本文件仅人类阅读；两者不一致时整批 FAIL）

## 总体状态：**BLOCKED**

驱动能力自检：PASS（proc-snapshot(root_pid=580277,members=1,rss/fd readable=1); stats(10,20,30)->{"median": 20.0, "min": 10.0, "max": 30.0, "volatility": "100.0%", "p95": null, "ols_slope": 10.0, "last_minus_first": 20.0, "pct": 200.0}）

产品钩子：ready=BLOCKED term=BLOCKED

| 指标 | 状态 | 说明 |
|------|------|------|
| startup_ready_ms | BLOCKED | 需 ready 钩子（M0-0.b 补齐） |
| idle_process_tree_rss_kib / fd_count | BLOCKED | 需 ready 钩子（M0-0.b 补齐） |
| tab/grid/terminal_cycle_rss_slope_kib | BLOCKED | 需 ready 钩子（M0-0.b 补齐） |
| resource_cycle_fd_delta / orphan_process_count | BLOCKED | 需 ready 钩子（M0-0.b 补齐） |
| terminal_10mib_elapsed_ms / frame_gap_* | BLOCKED | 需终端 begin/end 标记（M0-0.b 补齐） |

## 证据

- run 目录：`logs/m0-baseline/20260829T143410+0800_506193b_release_x11/`
- environment.json / scenario.json / summary.json / summary.md / SHA256SUMS
- raw/：命令原始 stdout/stderr；measurements/：结构化测量；commands/：完整命令行
