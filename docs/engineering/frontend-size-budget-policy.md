# BrowserOS 前端构建体积预算（2026-10-24）

## 目的

避免长期使用已接近耗尽的旧体积余量，导致小幅正常 UI 功能增长反复阻断 Full Validation；同时保留有效的包体积治理、审计和性能质量门禁。

## 唯一预算定义

- 唯一可执行真源：`scripts/measure-build-metrics.py`。
- 历史比较基线仍为 `logs/m0-build-metrics/build-metrics-052b18a.json`（2026-09-07，`dist.total_bytes=795517` 字节）。不自动更新或伪造基线。
- **增长达到 30.0%**：CI 发出可观察的 warning，上传 JSON 体积证据；不单独造成构建失败。
- **增长超过 45.0%**：阻断 Full Validation 和 `pre-merge.sh`，不能合并；恰好 45.0% 被接受。
- 依旧校验 `cargo_warnings` 不增加（在 `pre-merge.sh` 采集）、所有 Rust/GUI/安全/架构/语义/供应链/数据库既有门禁；本次仅改变前端总字节增幅预算。
- `dist` 是构建资产字节总和，不应被当作安装包体积、实际内存占用、冷启动时间或模块级运行开销。

## 证据

`capability-v2-validation.yml` 的 Early frozen frontend size gate 会公开当前 `dist` 度量及同比，并始终尽可能上传 artifact：

`browseros-frontend-size-budget-<run_id>`，文件 `browseros-frontend-size-budget.json`

JSON 记录基线来源和字节数、当前资产明细、增长百分比、预警/硬门禁状态、阈值。如果缺少基线或构建失败，工作流不应将其解释为通过。

## 维护机制

1. 每次合并前基于同一 head SHA 运行完整 Full Validation；达到 30% 预警时，优化超大模块/重复依赖/懒加载并记录理由，但不反复提交仅为避开 0.1% 的表面改动。
2. 逼近 45% 时应专门安排体积治理，不得通过删除回归测试、隐藏构建文件或跳过检查逃避。
3. 只有有实际生产构建证据、Git diff、GUI 回归、审查记录的独立变更，才能迁移冻结基线或再次调整预算；禁止自动在每个 PR 上滚动刷新基线。
4. 对预算调整本身运行 `python3 scripts/measure-build-metrics.py --self-test`，它覆盖 30% 边界、45% 恰好可接受/超出必失败以及 cargo warning 回归。

## 2026-10-24 预算调整记录

用户明确要求“先调整门禁到45%”，据此将硬上限从 35% 调整为 45%。保留原历史基线、30% 预警和 warning 不增加约束，不改变资产统计口径。此前约 44.5% 的增长是预算内接受，并非通过优化降低；本次不代表体积治理完成、完整门禁或安装版 GUI 验收通过。
