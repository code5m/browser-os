# BrowserOS 前端体积预算与非阻断预警（2026-11-06）

## 当前有效规则

- **体积是审计，不是拒绝合并的原因**。Editor 12 MiB、Core/Shared 1.5 MiB、Total 13.25 MiB 以及相对 master 1% 的增量阈值均保持可见，但**超出时只产生 WARN**。预算数字不自动滚动，也不因增加正常功能而拒绝 PR。
- **生产证据必须可信**：GitHub 构建当前精确 SHA 的 `dist`，`report-coding-editor-budget.mjs --verify-dist --audit-budget` 使用 Vite 生产等价内存构建，逐文件盘点并核对 SHA256；任何遗漏、不同产物、错误归属或无效统计都 FAIL。
- **源码依赖与性能结构约束仍严格**：锁定 Monaco/DOMPurify 版本；不能把共享字节错误归为编辑器独占；不能把编辑器代码引入初始静态 JS 图。对统计器核心函数、批准的审计政策做未评审篡改仍拒绝合并。
- **master 增量仍记录**：同一 runner 同口径构建明确的 master/base SHA、计算增量百分比，超过 1% 告警；如果 master 基准构建失败，记录 `UNAVAILABLE` 和详细警告，但当前生产构建和严格 SHA 验证若通过，CI 仍可 PASS。
- **真正 FAIL 的项目不变**：生产编译、Rust/Native、安全、工程治理、资源归属与要求执行的安装版 GUI 验收失败仍可阻断合并。绝不把这些失败伪装成体积告警。

## 证据与历史

GitHub Full Validation 保留 `/tmp/browseros-frontend-size-budget.json` 与 `/tmp/browseros-editor-budget-audit.json` 并上传 artifact。大小对比状态为 `PASS`、`WARN` 或 `UNAVAILABLE`，整体验证状态只在严肃的构建/完整性问题出现时 `BLOCKED`。超预算报告包含各项超出字节数，便于在后续性能优化时追踪趋势。

旧 M0 45% 累积超限规则已经移出当前 CI；原始日志不作发布决策。`scripts/measure-build-metrics.py --compare` 仅供历史诊断；旧的 `--approved-editor-budget` 子模式仍执行严格 SHA/依赖真实性审计，但字节超限是告警。任何正式改变批准的政策本身仍须独立审查。
