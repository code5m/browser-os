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

## Manifest 与模块配置

Manifest 是运行时元数据的唯一入口。除已有依赖、冲突、贡献和生命周期声明外，Manifest 可声明：

```text
kind: core | feature | optional
config: defaults + properties + additionalProperties
```

`enabled` 只决定是否装配，`config` 只承载该能力自己的参数。两者不会混为一个开关对象；未知配置项、类型错误、枚举值错误和必填项缺失会在激活前拒绝。

停用语义是 Disable：代码仍在 catalog 中，贡献被摘除，持久化数据保留。Remove/Purge 不由本运行时伪装实现；真正移除前必须通过 `check:capability-removal`，确认没有反向依赖、持久化残留或治理引用。

激活失败会回滚本轮已启动的能力；重复停用是幂等的，不会重复调用清理生命周期。仍有启用依赖方时，Runtime 以 `DEPENDENT_PRESENT` 拒绝停用。

## 门禁

```bash
npm run check:pluggable-runtime
npm run check:capability-removal
```

门禁覆盖：Manifest 校验、配置 schema、配置解析、依赖自动补齐、确定性激活顺序、`enabled=false`、缺失依赖拒绝、反向依赖保护、激活回滚、Contribution 注册与摘除、移除前置检查。
