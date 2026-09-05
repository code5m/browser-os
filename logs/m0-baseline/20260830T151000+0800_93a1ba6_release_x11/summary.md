# M0-1.a baseline quality gate — 20260830T151000+0800_93a1ba6_release_x11

> 检查点：M0-1.a；契约：V1.1；脚本：M0-1.a-4；场景：M0-1.a-v2
> 生成时间：2026-08-30 15:11:53 +0800
> 运行模式：formal（smoke 只能生成 EXPLORATORY）
> 机器判定源：summary.json（本文件仅人类阅读；两者不一致时整批 FAIL）

## 总体状态：**PASS**

| 指标 | 状态 | 值 |
|------|------|----|
| rust_fmt_main_exit | 0 == 0 ? PASS : FAIL | exit=0 |
| rust_fmt_plugin_exit | 0 == 0 ? PASS : FAIL | exit=0 |
| clippy_main_unique_warnings | PASS | 13 |
| clippy_plugin_unique_warnings | PASS | 0 |
| frontend_build_ms | PASS | warmup=2182ms; samples=3; median=2245ms; min=2235; max=2282; vol=2.1% |
| dist_total_bytes | PASS | 544870 |
| largest_js_bytes | PASS | 507980 (assets/index-DCX6UJ8H.js) |
| largest_js_gzip_bytes | PASS | 146078 |
| release_binary_bytes | 0 == 0 ? PASS : FAIL | 9135520; sha256=6adb8197130f3ea278693dc4df61321b6097cf93b9b068678e5b626de63024eb |

## BLOCKED / DEFERRED（信息性，不计入 M0-1.a PASS 判定）

产品 ready/终端钩子状态：ready=READY term=READY

## 证据

- run 目录：`logs/m0-baseline/20260830T151000+0800_93a1ba6_release_x11`
- environment.json / scenario.json / summary.json / summary.md / SHA256SUMS
- raw/：各指标命令原始 stdout/stderr；measurements/：结构化测量；commands/：完整命令行

