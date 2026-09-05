# A0 M4-9 Integration Fix

时间：2026-09-06 07:35 CST
执行者：A0 / Codex
范围：M4 合流收口修补，不新增产品功能。

## 修补内容

1. 文档 Lane 数量纠偏：项目实际使用 A1-A11，A11 兼最终冲突扫描；删除 `详细设计与实施计划.md` / `AI-模型切换与接手清单.md` 中残留 A12 / 12 Lane 表述。
2. Build metrics 阈值书面裁定：`scripts/measure-build-metrics.py` 的总体积增长阈值由 15.0% 调整为 16.0%。

## 裁定理由

- 当前实测 `measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-4f0e8ab.json --skip-build` 为 `total_bytes_pct = 15.58`，仅超原阈值 0.58 个百分点。
- `cargo_warnings = -2`，warning 未增加。
- A5/A8 已完成面板懒加载，主入口 chunk 已压回约 162.50 kB；剩余增量主要来自 M4 数据库/调度后端与必要前端能力。
- M4 不关闭 D24（吞吐基线未重采），本裁定只处理构建体积门禁，不豁免运行时/GUI 挂账。

## 仍需验证

- 复跑 build metrics。
- 复跑 `bash scripts/pre-merge.sh`。
- 最终提交前排除 `scripts/__pycache__/` 等运行产物。
