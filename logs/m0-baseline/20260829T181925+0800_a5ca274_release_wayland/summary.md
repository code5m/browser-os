# M0-1.b resource verification driver — 20260829T181925+0800_a5ca274_release_wayland

> 检查点：M0-1.b；契约：V1.0；脚本：M0-1.b-2；场景：M0-1.b-v1
> 生成时间：2026-08-29 18:22:21 +0800
> 机器判定源：summary.json（本文件仅人类阅读；两者不一致时整批 FAIL）

## 总体状态：**PASS**

驱动能力自检：PASS（proc-snapshot(root_pid=714602,members=1,rss/fd readable=1); stats(10,20,30)->{"median": 20.0, "min": 10.0, "max": 30.0, "volatility": "100.0%", "p95": null, "ols_slope": 10.0, "last_minus_first": 20.0, "pct": 200.0}）

产品钩子：ready=READY term=READY

| 指标 | 状态 | 说明 |
|------|------|------|
| startup_ready_ms | MEASURED | measurements/startup_ready.json |
| idle_process_tree_rss_kib / fd_count | MEASURED | measurements/idle_process_tree_r01.json |
| tab/grid/terminal_cycle_rss_slope_kib | MEASURED | measurements/{tab,grid,terminal}_cycle.json |
| resource_cycle_fd_delta / orphan_process_count | MEASURED | measurements/*_cycle.json + orphan_*.json |
| terminal_10mib_elapsed_ms / frame_gap_* | MEASURED | measurements/terminal_throughput.json |

## 证据

- run 目录：`logs/m0-baseline/20260829T181925+0800_a5ca274_release_wayland/`
- environment.json / scenario.json / summary.json / summary.md / SHA256SUMS
- raw/：命令原始 stdout/stderr；measurements/：结构化测量；commands/：完整命令行

