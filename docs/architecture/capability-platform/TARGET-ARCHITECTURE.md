# TARGET ARCHITECTURE — Universal Capability Building-Block Platform v1

> 本文只区分 **CURRENT（现状/实测）** 与 **TARGET（目标/未达成）**。禁止把目标写成现状。

## 1. 产品公式

```
PRODUCT = FRAMEWORK CORE + Σ(CAPABILITY MODULES) + ASSEMBLY CONFIGURATION
```

`profile` / `products/*.json` **只是 PRESET ASSEMBLY**，绝不是产品架构边界。
产品由「能力集合」决定，不是由 `App.vue` 决定。

## 2. TARGET 物理结构（理想）

```
Repository
├── Framework Core           壳 + Runtime + Registry + Contribution + Resolver + Assembly + Governor
├── Capability SDK / Contracts    Building Block Contract v1（本文项目：src/capability/platform/）
├── Capability Modules       src/capabilities/<id>/{manifest,index,public,contracts,intents,lifecycle,resource,state,ui}
├── Native Adapters          Tauri command 层 + bridge.ts
├── Shared Infrastructure    utils / composables / styles
├── Assembly Engine          依赖解析 + 装配 + 自定义组合
├── Resource Governor        资源归属与释放治理
└── Product Assemblies       config/capability-products/*.json（可任意 custom）
```

理想终局是 `packages/{framework-core,capability-sdk,capability-*}` 的多包结构；
**但不得为了目录漂亮强行迁移**（见 `07-MULTI-MODULE-ARCHITECTURE.md` 的诚实裁决）。

## 3. CURRENT（本次实测）

| 维度 | 现状 |
|---|---|
| 具备 Building Block Contract v1 的能力 | 4（bookmark / workspace / browser / terminal） |
| Catalog 机器可读 | ✅ `src/capability/platform/catalog.ts` + `npm run capability-demo list` |
| Dependency Resolver | ✅ 真实工作（强依赖缺失/环/冲突/未知 id 启动前确定性拒绝） |
| Assembly Engine | ✅ 真实驱动 `bootstrapAssembly()`（preset + custom） |
| 生命周期模型 | ✅ 8 态 + 合法迁移表 |
| Hot-Plug 试点 | ✅ Bookmark HP2（运行时 register/unregister，贡献真实增删） |
| Framework-only 自动引导 | ✅ `bootstrapAssembly([])` 零注册、零贡献 |
| Shell 是否仍知道能力实现 | ⚠️ **部分**：4 个 C3 能力已零直连；但 `MainArea.vue` 仍直接 import 约 10 个尚未积木化的面板（清单见 05 / 债务见 15） |
| 物理多包 | ❌ 单 Vite 应用，未做 workspace 迁移（诚实裁决：见 07） |

## 4. 关键铁律（平台不得违反）

1. **Runtime 不拥有业务 state**，只编排生命周期。
2. **Contribution Registry 不成为 Service Locator**，只存 contribution 元数据。
3. **能力之间不得 import 对方 internal store/composable/component**，只能经 public contract。
4. **Shell 不得 import 能力内部实现**。
5. **absent ≠ UI hidden**：缺席必须导致零注册、零贡献、零重资源。
6. **成熟度/热插拔等级必须以真实验证为唯一依据**，禁止把目标写成现状。
