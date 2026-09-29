# Pluggable Capability Runtime

当前运行时把 Capability 当作可装配积木处理，单一事实链如下：

```text
Manifest catalog
    ↓
enabled config
    ↓
Dependency Assembly
    ↓
Runtime register / activate
    ↓
Contribution Registry
```

## 配置

生产构建可以通过环境变量选择能力：

```bash
VITE_CAPABILITY_ENABLED=workspace,bookmark,clipboard
VITE_CAPABILITY_DISABLED=browser,terminal
```

`enabled` 只表达用户请求。强依赖由 Manifest 自动补齐；缺少强依赖、依赖环、冲突或未知能力会在启动前拒绝。可选依赖缺失只产生 degraded warning。

也可以在代码中调用 `bootstrapConfiguredCapabilityRuntime({ enabled })`。原有 `minimal`、`developer`、`full` profile 继续作为预设入口。

## 生命周期

能力必须先经过 `register → resolve → activate`。停用路径为：

```text
ACTIVE → SUSPENDED → DISABLED
```

停用后由 Pluggable Runtime 摘除该能力的所有 Contribution，并调用 `onDeactivate`。常驻能力或不满足资源策略的能力仍会被 Runtime 拒绝停用。

## 门禁

```bash
npm run check:pluggable-runtime
```

门禁覆盖：配置解析、依赖自动补齐、确定性激活顺序、`enabled=false`、缺失依赖拒绝、Contribution 注册与摘除。
