# Checkpoint · A8 · M5-W17 Acceptance Closeout (Manual QA)

LANE: A8
STATUS: PASS_WITH_DEBT
SCOPE: logs/assist/, logs/checkpoints/ (no product code)
DELIVERED: honest 4-area acceptance record (desktop · narrow-window · home launcher · startup) + HOME_NO_SECRET_PERSIST resolved + A5/A6/A7 HOLD confirmed
SEE: logs/assist/A8-M5-W17-acceptance-closeout-20260907-1745.md

VERIFY (exact commands : results):
- bash scripts/pre-merge.sh : PRE_MERGE_RESULT=ALL_PASS
- npm run build : ✓ ~4.6s; no chunk >500kB
- node scripts/check-graph-ui-logic.mjs : 113/0
- node scripts/check-home-store-logic.mjs (A3) : 105/0 (HOME_NO_SECRET_PERSIST resolved)
- node scripts/check-home-ui-logic.mjs (A9) : 41/0 → HOME_UI_RESULT=PASS
- python3 scripts/check-home-client-policy.py (A4) : ACTIVE=3
- bash scripts/check-dev-startup.sh (A2) : PASS=23 FAIL=0 (startup/localhost:1421 规避已验证)
- python3 scripts/measure-build-metrics.py : total=795391B (+7855B vs W15, ≤25.2%); over_500kb=False; cargo_warnings=2
- grep 'invoke(' W17 scope : 0 matches

METRICS: total=795391B (W15 +7855B, ≤25.2% ceiling); over_500kb=False; cargo_warnings=2; 范围内无新增运行时权限。

PATCH: N/A (MANUAL QA, no code changes).

A5/A6/A7 HOLD: 三者文件均为已改(M)状态、无具体阻塞；按 HOLD 规则未重开/未重复开发。

RISKS: GUI 视觉验收在 headless 环境 BLOCKED（用户侧证据，board L1205 明示）；cargo_warnings=2 等于 W15 基线；启动冒烟为进程/契约级，真实窗口渲染仍用户侧。

NO_PUSH: confirmed.
