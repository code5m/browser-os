# BrowserOS 前端体积双基线 CI 规范（2026-10-31）

## 为什么需要双基线

2026-10-31 已核对相同 Ubuntu 24.04、Node 22、npm ci + npm run build、同一个原始字节统计程序：
- master@81a9feca: 12,975,246 B / 169 files / 166 JS chunks
- perf@8090f000: 12,976,614 B / 169 files / 166 JS chunks
- perf 比 master 仅 +1,368 B（+0.01054%）；双方最大 ts.worker 均为 6,989,587 B。
- 历史 M0@052b18a 的 dist 为 795,517 B，原冻结 45% 限额下 master 与 perf 都超限。**不能把 1531% 说成 perf 相对 master 的回归。**

证据：
- master: https://github.com/code5m/browser-os/actions/runs/41152741505
- perf: https://github.com/code5m/browser-os/actions/runs/41163201194

## 实施规则（不修改原历史门禁）

1. **历史审计**：原 M0 JSON、measure-build-metrics.py 的阈值（30% warning / 45% limit）与历史超限判断原样保留。报告明确写出 BREACH，不能假称 M0 已 PASS，也不能修改基线数字。
2. **已批准的例外**：沿用 2026-10-27 的 docs/engineering/coding-editor-budget.json。真实生产 dist 必须通过 scripts/report-coding-editor-budget.mjs --verify-dist --enforce-budget：逐文件哈希验证、worker 的源码所有权归属、严格锁定 Monaco/Dompurify 版本、编辑器不进入初始静态 JS 图。依旧限制编辑器独占 12 MiB、核心/共享 1.5 MiB、总计 13.25 MiB。无法证明归属的字节必须计入核心/共享。
3. **性能分支新增的 1% 审查线**：在**同一 CI runner**上，以归档方式提取 PR base/master 的精确 SHA，重新 npm ci、npm run build；使用同一个 collect_dist 统计所有文件。不用旧 M0 当本次增量基准。若 HEAD 相对 master 增长 >1% 则阻断，并要求单独评审。这个新限额是收紧性能回归检查，绝非增加 M0 45% 上限。
4. **防篡改**：当前分支与 master 的历史基线、统计实现和已批准 Monaco 预算 Git Blob 必须一致；有差异一律阻断。Node、npm 版本、精确 base SHA 和配置文件差异随构建记录保留。
5. **证据与失败**：/tmp/browseros-frontend-size-budget.json 和 /tmp/browseros-editor-budget-audit.json 作为 Actions artifact 上传。若任一依赖无法验证、审计不通过、基准构建失败，fail closed；不能靠跳过测试制造绿灯。
6. **不改变其他门禁**：Rust fmt/check/test、安全审计、Grid / Agent 生命周期、安装版 GUI 与真实 5/30 分钟性能仍独立必须通过，不能因前端体积验收通过就合并。

CI 执行：python3 scripts/check-frontend-size-dual-budget.py --self-test；
python3 scripts/check-frontend-size-dual-budget.py --master-ref origin/master。

只有经过已批准专项审计且与 master 增量符合性能审查线时，历史 M0 体积超限才可作为**显式、有限范围的例外**。这与 scripts/pre-merge.sh 原有 --approved-editor-budget 判定一致；原历史超限始终记录为 BREACH，不改写。
