# 新能力开发指南（以后不要再往 App.vue 里塞功能）

## 流程

```
create capability → declare contract → declare deps → declare contributions
→ declare resources → register → test → assemble
```

## 1. 建包

```
src/capabilities/<id>/
├── manifest.ts      契约（含 v1 块）
├── public.ts        对外契约（别人只能用这个）
├── contracts/       契约类型
├── intents/         意图
├── lifecycle/       onActivate / onSuspend
├── resource/        资源策略
├── state/           业务真源 owner
├── ui/              组件（只在本包内被引用）
└── index.ts         registerXxxContributions()
```

模板：`templates/capability/manifest.template.ts`。

## 2. 写 manifest（最关键的诚实性要求）

- `dependencies`：缺了就跑不起来的（强依赖缺失 → 启动前拒绝）。
- `optionalDependencies`：缺了只是降级。
- `resources`：你占什么就声明什么；没有就说没有。
- `hotPlug`：**做到哪写到哪**。 HP0 就写 HP0 并给 `limitationReason`。
- `maturityEvidence`：填真实 checker / 测试名，不许编。

## 3. register

导出 `registerXxxContributions()`，向 slot 注册贡献；Shell 会自动渲染，**不要改 Shell**。

## 4. test

```bash
npm run check
node scripts/check-capability-platform.mjs
node scripts/capability-demo.mjs inspect <id>
```

## 5. assemble

在 `config/capability-products/products.json` 里加组合，或直接用 CLI 自定义：

```bash
node scripts/capability-demo.mjs resolve <id>,workspace
```

## 不要做的事

- 不要让 Shell import 你的组件/store。
- 不要去 import 别的 capability 的 `state/*` 或 `ui/*`。
- 不要为了让 UI 出现而放宽 checker。
