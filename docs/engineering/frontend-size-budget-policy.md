# BrowserOS 当前前端体积预算（2026-11-06）

## 生效规则

- **批准的三项绝对上限**：Monaco 等编辑器独占资产 12 MiB；Core/Shared 1.5 MiB；所有构建资产合计 13.25 MiB。实际数字由 `docs/engineering/coding-editor-budget.json` 提供；不得重新归属共享字节以规避上限。
- **真实产物核对**：对生产构建 `dist` 所有文件进行完整盘点与 SHA256 验证，校验编辑器源码所有权、锁定的依赖版本以及初始 JS 不包含编辑器代码。
- **增量门禁**：同一 CI runner 对精确 master/base SHA 与当前 SHA 分别构建，使用相同 Node/npm、工具和统计口径；当前总字节增长超过 1% 则 FAIL。
- **缺证据即失败**：构建、哈希、分项预算和同轮 master 对比任何一项失败，都不得标记 PASS。

## 输出与审计

`.github/workflows/capability-v2-validation.yml` 运行 `scripts/check-frontend-size-dual-budget.py`，生成 `browseros-frontend-size-budget.json` 和 `browseros-editor-budget-audit.json`。报告仅以当轮实际构建的预算和增量为结论；历史累积超限提示已从当前 CI 结果移除。

历史构建档案仍可用于回溯原始测量，但不产生当轮构建的超限状态，也不得代替安装包体积、内存、冷启动或 GUI 测试证据。未来调整预算须提交经过审查的独立变更，不得篡改构建清单、源代码归属或跳过验证。

### 独立旧采集器的处理（2026-11-06）

`scripts/measure-build-metrics.py --compare` 仍允许以任意历史样本查询体积差异与 Rust warning 变化，但只做诊断，不再包含旧的百分比硬上限、BREACH 判定或批准例外。新的正式阻断点是绝对预算、逐文件 SHA256 与同轮 master 1% 增量审查。旧原始日志可以单独回溯，但不再作为当前超限提示。
