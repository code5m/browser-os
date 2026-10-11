# BrowserOS 体积与性能治理 · 2026-10-11

## 可复现事实

参考版本：`7ae6fa86be3c7f8044aeeb7c627981559c9f9da6`，GitHub Full Validation run `38098492736`；精确实测值见 `docs/engineering/footprint-baselines/master-7ae6fa86.json`。该基线与候选构建在 **相同平台和 lockfile** 条件下对比，安装包体积与源码行数不是一个指标。

- `measure-build-metrics.py` 的 **25.2%** 阈值继续为唯一正式增长阻断门禁（禁止更新或绕过），对比档案不替代它。
- `measure-capability-footprint.py` 汇总模块源码字节和实际 `dist/.deb` 字节；模块源码 **不能**按比例分配为安装成本。
- `report-build-assets.mjs` 报告 JS/CSS 文件及 gzip；HTML 引用不是实际首屏传输。
- `generate-footprint-review.mjs` 输出人类可读 HTML 与 JSON，缺失指标明确标识 **UNKNOWN**，不能用 0 伪造。
- `check-duplicate-css.mjs` 只识别精确相同的声明文本；**不能自动合并有不同 selector / scoped 的规则**。
- `benchmark-resource-scenarios.mjs` 在真实安装客户端的 PID 下支持 5 分钟和 30 分钟运行窗口（资源采样专用；系统锁屏、测试负载与软硬件版本应写在验收单）。不在 CI 无条件执行 30 分钟。没有实跑证据时必须标注待测。

## CI 与独立复核

```sh
node scripts/generate-footprint-review.mjs --self-test
node scripts/benchmark-resource-scenarios.mjs --self-test
node scripts/check-duplicate-css.mjs --self-test
node scripts/check-duplicate-css.mjs > /tmp/browseros-css-audit.json
# 打包后
node scripts/generate-footprint-review.mjs \
  --before docs/engineering/footprint-baselines/master-7ae6fa86.json \
  --after /tmp/browseros-footprint.json \
  --bundle /tmp/browseros-build-assets.json \
  --native /tmp/browseros-native-resource.json \
  --out /tmp/browseros-size-comparison.html --out-json /tmp/browseros-size-comparison.json
# 真实机器先启动安装版，并记录其 PID 后执行。默认不自动关机/重启。
node scripts/benchmark-resource-scenarios.mjs --pid PID --minutes 5 --out /tmp/browseros-idle5.json
node scripts/benchmark-resource-scenarios.mjs --pid PID --minutes 30 --out /tmp/browseros-idle30.json
```

## 合格条件

1. 构建产物、安装体积、各 Chunk 原始与压缩字节有 CI artifact，来自同一次 full workflow。
2. 资源范围明确：单 WebView / 整个进程树 / `/proc` PSS 与 RSS 不可混用；空值不算零。
3. Browser、Files、Grid 继续实际安装包冷启动与完整 GUI 回归。测试脚本不得制造“PASS”数据代替 GUI。
4. 公共工具抽取以函数契约等价为先，`fmtSize` 和 `formatBytes` 的字节单位及 `redactSecrets` 的不同安全 Owner 必须保留。
5. 真正安装**外部可执行能力**是 HP3 安全工作，现有 Plugin Stage-I 仅登记 manifest，`PLUGIN_NO_EXEC_SURFACE` 强制禁止假执行；本阶段不会通过“安装元数据”冒称完成可运行模块安装。
