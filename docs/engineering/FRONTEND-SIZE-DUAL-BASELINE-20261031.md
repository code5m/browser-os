# BrowserOS 前端体积审计与质量门禁（2026-11-06 更新）

兼容旧文档引用保留此文件路径。唯一有效执行入口是 `scripts/check-frontend-size-dual-budget.py`，当前体积策略详见 `docs/engineering/frontend-size-budget-policy.md`。

## 永久阻断（FAIL）

- 当前 SHA 的生产构建失败、`dist` 不存在、清单/逐文件 SHA256 与同口径内存构建不符。
- 编辑器源码归属错误、共享资产漏算、初始静态 JS 错误引入编辑器、依赖锁定版本不一致或数据格式不合法。
- 统计器函数或已审查依赖/预算政策遭到未评审更改、必要的安全/质量/GUI 门禁失败。

## 只告警不阻断（WARN）

- 历史审批参考值：Editor 12 MiB、Core/Shared 1.5 MiB、Total 13.25 MiB，任意单项超过均仅生成 `::warning` 及分项字节证据。
- 相对同轮 master 精确提交 SHA 的体积增长超过 1%，只生成 `::warning` 及增量证据。基准构建不可用时记录 `UNAVAILABLE` 并告警，不伪造 PASS，且不影响已验证的当前生产构建。
- 旧 M0 累积 45% 超限记录不再形成当前状态；原始旧档案仍可供追溯。

执行：

```sh
python3 scripts/check-frontend-size-dual-budget.py --self-test
python3 scripts/check-frontend-size-dual-budget.py --master-ref origin/master
```

报告保留当前 SHA、master SHA、所有大小及告警，同时只在证据真实性、安全性或构建正确性失败时标为 `BLOCKED`。任何体积超限本身不得阻断合并。
