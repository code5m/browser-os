# Universal Hot-Plug Capability Platform v1 — 交付说明

一句话：**产品 = 最小底座 + Σ能力积木 + 装配配置**。

## 三分钟自验（全部为真实命令）

```bash
node scripts/capability-demo.mjs list              # 积木清单（含成熟度/依赖/贡献/资源）
node scripts/capability-demo.mjs matrix            # 16 组合插拔矩阵
node scripts/capability-demo.mjs use developer      # 按产品装配 + 真实 bootstrap
node scripts/capability-demo.mjs runtime-demo bookmark   # 不重启，拔掉再插回
node scripts/capability-demo.mjs demo              # 3~5 分钟领导串场脚本
node scripts/check-capability-platform.mjs         # 平台门禁（29/29）
npm run check                                      # 全既有门禁（含新门禁）
```

## 文档索引

| 文件 | 内容 |
|---|---|
| `01-LEADER-SUMMARY.md` | 给领导的通俗说明 |
| `02-CURRENT-VS-TARGET.md` | 已达成 vs 未达成（严格分开） |
| `03-FRAMEWORK-CORE.md` | 最小底座由什么组成 |
| `04-CAPABILITY-BUILDING-BLOCK-CONTRACT.md` | Building Block Contract v1 字段与校验 |
| `05-CAPABILITY-INVENTORY.md` | 全量能力清单与分类（UNCLASSIFIED=0） |
| `06-CAPABILITY-MATURITY-MATRIX.md` | 成熟度 C0–C5 与热插拔 HP0–HP3 现状 |
| `07-MULTI-MODULE-ARCHITECTURE.md` | 物理多包裁决（诚实结论） |
| `08-DEPENDENCY-RESOLVER.md` | 依赖解析规则与负例 |
| `09-ASSEMBLY-ENGINE.md` | 装配引擎与产品配置 |
| `10-RUNTIME-HOT-PLUG.md` | 生命周期与热插拔流程 |
| `11-RESOURCE-GOVERNANCE.md` | 资源归属与释放证据口径 |
| `12-PLUG-UNPLUG-MATRIX.md` | 16 组合结果 |
| `13-AI-MAINTAINABILITY.md` | 普通 AI 如何只改一个能力 |
| `14-LEADER-DEMO.md` | 演示脚本与话术 |
| `15-KNOWN-DEBT.md` | 已知债务与分类 |
| `16-NEXT-STEPS.md` | 下一步 |
