# BrowserOS 前端体积门禁（2026-11-06 更新）

该文档保留原路径以兼容已有引用，当前规则以 `scripts/check-frontend-size-dual-budget.py` 和 `docs/engineering/coding-editor-budget.json` 为准。

## 当前有效的阻断条件

1. **绝对体积预算**：精确验证生产 `dist` 的所有文件及 SHA256，按实际构建资产归属计算编辑器独占、Core/Shared 和总体积。三项均须符合已批准预算（编辑器 12 MiB、核心及共享 1.5 MiB、总计 13.25 MiB）；无法证明编辑器独占的字节归 Core/Shared。
2. **master 增量门禁**：在同一 runner、Node/npm 版本下，用明确的 PR base/master SHA 归档构建，对比同口径 `dist.total_bytes`；当前变更相对 master 不得增长超过 1%。
3. **完整证据**：记录候选 SHA、基准 SHA、产物体积、完整哈希验证、预算结果、增量和构建环境。任一项缺失或失败，一律阻断。
4. **权限与质量检查**：Rust / Native、UI Safety、工程治理、Hot-Plug 和安装版 GUI 验收等独立执行，不因体积 PASS 自动视为完成。

## 已停用的旧口径

旧版 M0 累积增长百分比与旧超限例外不再生成当轮的 `historical_m0` 字段、`BREACH` 告警或 `PASS_WITH_APPROVED_M0_EXCEPTION` 状态。它不能与本轮经过验证的绝对预算结果混合显示。原始历史构建档案属于代码审计材料，不是当前发布门禁，也不作为当前性能结论。

CI 执行：

```sh
python3 scripts/check-frontend-size-dual-budget.py --self-test
python3 scripts/check-frontend-size-dual-budget.py --master-ref origin/master
```

CI 上传 `browseros-frontend-size-budget.json` 和 `browseros-editor-budget-audit.json`；状态仅为 `PASS` 或 `BLOCKED`，不会因历史记录出现带例外的绿色状态。
