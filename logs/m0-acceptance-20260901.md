# M0 Acceptance Evidence Draft

STATUS=OWNER_APPROVED
CHECKPOINT=M0-7.c
MODEL=Codex gpt-5.5 high
DATE=2026-09-01 09:05 CST
NEXT=M1-0

## Conclusion

M0-7.a 已汇总 M0-0~M0-6 证据；M0-7.b 已由强模型独立复核安全、生命周期、性能与 GUI 结论；M0-7.c 已获得负责人/用户明确确认：“验收通过”。本报告状态升为 `OWNER_APPROVED`，M0 安全与稳定性基线正式 PASS。M1~M3 解锁为可按 WBS 领取，M4/M5 仍需等待前置里程碑。

## Closed WBS Evidence

| WBS | 状态 | 关键提交 | 证据 |
|---|---|---|---|
| M0-0 完整基线 | PASS with adjudicated UNSTABLE ranges | `93a1ba6`, `5872bc5` | `logs/checkpoints/M0-0.c-20260830-1538.md`; manifest `logs/m0-baseline/manifests/20260830T153538+0800_93a1ba6_M0-0.c/` |
| M0-1 自动门禁 | PASS | `2b476d3`, `73e9dfb` | `logs/checkpoints/M0-1.c-20260829-1530.md`; `scripts/pre-merge.sh`; `scripts/GATE-CONTRACT.md` |
| M0-2 生命周期重构 | PASS | `aaed1c7`, `a2cbe79`, `15a0087` | `logs/checkpoints/M0-2.d-20260831-0725.md`; `logs/m0-resource-ownership-v1.md`; `scripts/check-lifecycle-contract.py` |
| M0-3 安全边界重构 | PASS with accepted residual gaps | `30b9cc9`, `3661fb3`, `16e99b5` | `logs/checkpoints/M0-3.d-20260831-0929.md`; `logs/m0-security-threat-matrix-v1.md`; `scripts/check-security-policy.py` |
| M0-4 构建/依赖清理 | PASS | `c74ce34`, `b3a81b9`, `e216f03` | `logs/checkpoints/M0-4.c-20260831-1019.md`; `logs/m0-build-metrics/build-metrics-6f4e554.json`; `scripts/measure-build-metrics.py` |
| M0-5 资源生命周期闭环 | PASS | `68c78d7`, `e998bbd`, `ebc49f7` | `logs/checkpoints/M0-5.c-20260831-1652.md`; formal run `logs/m0-baseline/20260831T164058+0800_6534963_release_x11/`; analysis `logs/checkpoints/M0-5.c-analysis-20260831-164058.md` |
| M0-6 崩溃恢复与 GUI 回归 | PASS | `57a31ea`, `b3331a0`, `5056fbc`, `f1612d7` | `logs/checkpoints/M0-6.c-20260901-0726.md`; GUI evidence `logs/m0-6c-gui-evidence/20260901-0726/`; selftest `logs/m0-grid-selftest/M0-6.c-20260901-selftest.log` |

## Verification Summary

| Gate | Result |
|---|---|
| `python3 scripts/m0-6c-gui-regression.py --self-test` | EXIT=0，`M0_6C_GUI_REGRESSION_SELF_TEST=PASS` |
| `cargo test --manifest-path src-tauri/Cargo.toml` | EXIT=0，47 passed / 0 failed，warning 仍为既有 2 条 |
| `npm run build` | EXIT=0，Vite build 通过；保留既有 `useBrowserStore` 静态/动态 import 混用告警 |
| `bash scripts/pre-merge.sh` | EXIT=0，`PRE_MERGE_RESULT=ALL_PASS` |
| `git diff --check` | EXIT=0 |

## M0-7.b Review

| Area | Result | Notes |
|---|---|---|
| Lifecycle | PASS | `check-lifecycle-contract.py --self-test / --expect-current-gaps / 默认` 均 EXIT=0；机器可检 lifecycle gap 为 0 |
| Security | PASS with residual accepted risks | `check-security-policy.py --self-test / --expect-current-gaps` 均 EXIT=0；3 个残余 gap 与威胁矩阵一致 |
| Resources | PASS | M0-5 formal run PASS；orphan max 为 0；`timerfd` 与 RSS 残余解释可接受，后续 owner 保留 |
| GUI/crash | PASS | M0-6.c 自动化 GUI 回归 PASS；`GRID_SELFTEST` PASS；本地 AI mock 限制已明示 |
| Build/test gates | PASS | `cargo test` 47 passed；`npm run build` 通过；`pre-merge` ALL_PASS；`git diff --check` 通过 |

## Residual Risks For M0-7.b

| Risk | Current Decision | Required Review |
|---|---|---|
| M0-0 aggregate raw status is `UNSTABLE` | 已在 `M0-0.c` 裁决为正式基线：15 项用 median，6 项只能用区间和风险说明 | 确认 M0-5/M3 owner 是否足够，不能把 raw aggregate 改写成 PASS |
| M0-3 security residual gaps | `REMOTE_WILDCARD_IPC`、`EVAL_WITHOUT_SOURCE_CHECK`、`READ_ONLY_BROWSE_WITHOUT_PATH_POLICY` 保留在威胁矩阵 | 确认补偿控制、功能取舍和后续 owner 是否可接受 |
| M0-5 resource platform noise | grid/terminal FD 正增长归因为 WebKit `anon_inode:timerfd` 平台计时器；terminal RSS `+7.46%` 未越过 10% | 复核 formal run 与分析是否支持不阻塞 M0 |
| M0-6.c login-state evidence | 使用本地 AI mock 持久 cookie，验证壳层恢复与会话持久化 | 明确不等同第三方真实账号人工验收；决定是否需要放到 M1/M2 冒烟补测 |
| Xwayland screenshot limitation | 截图/录屏可能黑屏，正式证据改用事件、pid、app.log | 确认证据类型是否满足 M0 工程基线 |
| M0-7.c owner confirmation | 已完成 | 2026-09-01 09:05 CST 用户/负责人确认“验收通过”；M1~M3 改为可启动但尚未开始 |

## Go / No-Go Draft

当前 M0-0~M0-7 已有可追溯证据，M0-7.b 强模型复核通过，M0-7.c 负责人确认完成。M0 结论为 `OWNER_APPROVED` / PASS；NEXT=`M1-0`，先展开 M1 检查点后再启动具体功能。
