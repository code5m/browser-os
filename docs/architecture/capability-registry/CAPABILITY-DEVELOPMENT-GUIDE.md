# Capability 开发约定

新增普通 Capability 的最小组成是：

```text
capability-<id>/
  manifest.ts       # 唯一元数据入口
  index.ts          # lifecycle 与 Host adapter
  public.ts         # 对外契约（如需要）
  ui/               # 自己的组件
  state/            # 自己的状态所有者
```

Manifest 负责身份、版本、`runtimeApiVersion`、依赖约束、配置 schema、Contribution 声明和持久化边界。`enabled` 只表示启用意图，不能代替模块配置，也不能代表 active 或 healthy。

## 接入边界

普通 Capability 不直接修改 `App.vue`、`MainArea.vue`、`SettingsPanel.vue`、Runtime 核心或其他 Capability 的 `state/ui`。需要出现在 Shell 时，向 Contribution Registry 注册 `commands`、`menus`、`sidebar`、`panels`、`settings` 或 `workbench` 贡献；Shell 只消费 slot，不判断能力 ID。

Capability 之间只能通过 `public.ts`、ports、事件和 Contribution 交互。`scripts/check-capability-boundary.mjs` 会拒绝 Runtime 直连和跨 Capability 内部导入。

## 生命周期与状态

Runtime 统一发布 `discovered`、`registered`、`activating`、`activated`、`failed`、`deactivating`、`deactivated` 事件。Manager、Inspector、测试和未来 CLI 都应读取 Runtime snapshot，不自行推导状态。

`Disable` 只停止运行并摘除 Contribution，保留代码和持久化数据；`Remove/Purge` 本阶段不执行物理删除。

## 验收

```bash
npm run check:capability-discovery
npm run check:capability-boundary
npm run check:pluggable-runtime
npm run check:capability-removal
```

新增 Capability 必须至少证明：Manifest 可发现、依赖可装配、生命周期事件完整、Contribution 可注册和摘除、反向依赖阻止不安全停用。
