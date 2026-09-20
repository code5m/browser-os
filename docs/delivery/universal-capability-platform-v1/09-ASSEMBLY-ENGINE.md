# 09 — Assembly Engine

实现：`src/capability/platform/assembly.ts`；驱动真实启动：`src/capability/index.ts#bootstrapAssembly`。

## 输入 / 输出

```
输入：能力 id 集合（来自 products/*.json preset 或 CLI 任意 custom）
输出：requested / resolved / dependencies(autoIncluded) / activationOrder
      contributions / permissions / resources / warnings / rejections
```

## Profile ≠ 产品边界

`config/capability-products/products.json` 里的 preset 只是「常用组合」：

| preset | 组成 |
|---|---|
| `framework-only` | ∅（最小底座） |
| `workspace-only` | workspace |
| `browser` | bookmark + workspace + browser |
| `developer` | bookmark + workspace + browser + terminal |
| `workspace-terminal` | 自定义示例（证明任意组合可行） |
| `invalid-missing-dependency` | 负例 fixture（演示启动前拒绝） |

同时支持完全自定义：

```bash
node scripts/capability-demo.mjs resolve bookmark,terminal
node scripts/capability-demo.mjs resolve framework-only
```

## 「加入不需要修改 Shell」如何成立

- Shell 只按 slot 遍历 contribution 渲染（`<component :is>` + `getSurfaceContributions`）。
- 一个满足 Building Block Contract 的能力，把贡献注册进 registry 后 Shell 自动渲染；
  **本次实证据**：bookmark 在运行时 register 后贡献立即出现，且 Shell 代码零改动（PLT2-14a）。
- 若要让它进入产品，需要修改的是**装配配置**，不是 Shell 业务代码。

## 真实 bootstrap 证据

```
REAL BOOTSTRAP   registered=[bookmark,browser,terminal,workspace] activated=true error=none
REAL BOOTSTRAP   拒绝（deterministic）: 装配失败: UNKNOWN_CAPABILITY(...)
```
