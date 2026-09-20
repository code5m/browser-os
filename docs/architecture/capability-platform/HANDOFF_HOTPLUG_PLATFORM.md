# HANDOFF — Universal Hot-Plug Capability Platform v1（Overnight Train）

## 精确状态

- BRANCH: `feature/capability-platform-v1`
- START_HEAD（本夜开始）: `50b616cea0a4578b11da036bae1f54376c8521f8`
- 平台内核稳定点 TAG: `universal-capability-platform-core-v1-pass` → `9b95bd073394ca9a4c0fa583a2b95b687c064d11`
- 演示 CLI 提交: `e101d6c`
- 受保护基线 TAG 未移动: `capability-modularization-v1-code-pass` → `04d794b5cf1bbd689616a3b598270f8c522bee46`
- NO PUSH / NO MASTER MERGE / 未改用户数据 / 未改系统 / 未暴露凭据

## 已完成（均有命令可复现）

| Train | 内容 | 验证 |
|---|---|---|
| H-A | 全量能力清单扫描，UNCLASSIFIED=0 | `05-CAPABILITY-INVENTORY.md` |
| H-C | Building Block Contract v1 + Catalog | PLT2-01/02 |
| H-D | Dependency Resolver + Assembly Engine | PLT2-03~08、19 |
| H-F | 16 组合插拔矩阵 | PLT2-09（16/16） |
| H-G | Runtime Lifecycle（8 态） | PLT2-13/13b |
| H-H | Hot-Plug HP1+HP2 试点（bookmark） | PLT2-14a~f |
| H-M | Leader CLI Demo + products 配置 | `capability-demo.mjs` |
| H-O | 独立红队 24 问 | `17-RED-TEAM.md` |

## 关键产物路径

```
src/capability/platform/contract.ts       Building Block Contract v1（schema + 校验）
src/capability/platform/assembly.ts       Dependency Resolver + Assembly Engine
src/capability/platform/lifecycle.ts      8 态生命周期
src/capability/platform/hotplug.ts        ADD/REMOVE 流程（含回滚）
src/capability/platform/orchestrator.ts   接真实 runtime + registry 的薄适配层
src/capability/platform/catalog.ts        Capability Catalog（只 import manifest 纯数据）
src/capability/index.ts#bootstrapAssembly Assembly 驱动真实 bootstrap
scripts/check-capability-platform.mjs     平台门禁（29/29）
scripts/capability-demo.mjs               领导演示 CLI
config/capability-products/products.json 产品装配配置（含 framework-only 与负例）
docs/delivery/universal-capability-platform-v1/   交付文档 01~17
```

## 未完成 / 已知债务

见 `15-KNOWN-DEBT.md`（D-1 ~ D-9）。最关键的三个：
- **D-1** Shell 仍直连约 10 个未积木化面板
- **D-10** Shell 仍硬编码 Dock 页签名（终端/资源/会话）
- **D-9** 物理多包未迁移（DEPENDENCY boundary 靠 checker，不靠构建）

## 下一条该执行的命令

```bash
# 若要继续 architecture 工作（解决 D-1 / D-10）：
#   1) 选 git 或 database（owner 已分离，收益最高）做 capability 化
#   2) 在 MainArea.vue 中删除对应 v-else-if 分支，改由 WORKBENCH_MAIN slot 渲染
npm run check && npm run build

# 若要进入 Human GUI 验收：
npm run tauri dev
```

## 当前成熟度速查

| 能力 | Maturity | Hot-Plug |
|---|---|---|
| bookmark | C3 | HP2 |
| workspace | C3 | HP0 |
| browser | C3 | HP0 |
| terminal | C3 | HP0 |
| git / database | C1 | — |
| task / graph / plugin / skill / agent | C0 | — |
