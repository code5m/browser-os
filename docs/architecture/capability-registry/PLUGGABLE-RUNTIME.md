# Pluggable Capability Runtime

修订 2，2026-10-04。本说明区分可交互的 Manager、同步装配入口与原生桌面验收，避免把单项检查通过等同于整机验收。

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

默认启动会读取 `browser-os-capability-config`。只接受当前 profile 内的能力；停用配置只对达到 C3 且声明可启停的非 CORE 能力生效。强依赖会补回，不能借存储配置绕过依赖保护。显式 profile 参数或环境装配配置优先，不受默认启动的本地开关覆盖。

存储不可访问、JSON 损坏或字段类型错误不会阻断默认启动。保存失败时，Manager 明确显示“仅对本次会话生效”，不把当前状态已改变误报为配置已保存。

## 生命周期

能力必须先经过 `register → resolve → activate`。停用路径为：

```text
ACTIVE → SUSPENDED → DISABLED
```

停用后由 Pluggable Runtime 摘除该能力的所有 Contribution，并调用 `onDeactivate`。常驻能力或不满足资源策略的能力仍会被 Runtime 拒绝停用。

### Manager 操作

Manager 使用显式动作，不根据新的状态把同一次点击解释成另一种操作：ACTIVE 可暂停；SUSPENDED 同时提供恢复和停用；DISABLED 可启用。DISABLED 是 `enabled=false` 的展示名，不是新增 Runtime 状态。暂停只对本次会话生效，停用不删除业务数据。

Manager 只开放已治理、非常驻、C3 且具备完整启停声明的能力。存在启用中的强依赖方时不能暂停或停用。Settings 是框架常驻服务；当前 profile 未装配的能力显示 UNAVAILABLE，不能借 Manager 越权装配。默认 full 下，Bookmark 是本轮实际验证的可切换能力；没有扩大 Browser、Grid、Session 或 Terminal 的启停权限。

同一 Runtime 的 Manager 操作互斥。异步激活和暂停等待 hook 完成后提交状态，清理完成后再摘除入口。失败时恢复本能力的启用标志和原贡献，允许重试。此恢复仅覆盖编排元数据与贡献，业务 hook 的外部副作用必须由能力自身补偿；不是对任意资源的事务保证。旧同步装配 API 没有因此获得通用异步事务能力。

Contribution Registry 使用浅响应式集合，Shell 消费者按需计算；停用立即移除入口，重新启用恢复组件，不覆盖实际组件为 Manifest 元数据。Manager 主视图通过已注册贡献认领，不再同时显示缺失面板提示。

## Manifest 与模块配置

Manifest 是运行时元数据的唯一入口。除已有依赖、冲突、贡献和生命周期声明外，Manifest 可声明：

```text
kind: core | feature | optional
config: defaults + properties + additionalProperties
```

Manifest 还预留 `runtimeApiVersion` 与 `dependencyConstraints`，用于未来兼容性解析；当前不引入复杂包管理器。新增能力应遵循 [Capability 开发约定](./CAPABILITY-DEVELOPMENT-GUIDE.md)。

`enabled` 只决定是否装配，`config` 只承载该能力自己的参数。两者不会混为一个开关对象；未知配置项、类型错误、枚举值错误和必填项缺失会在激活前拒绝。

停用语义是 Disable：代码仍在 catalog 中，贡献被摘除，持久化数据保留。Remove/Purge 不由本运行时伪装实现；真正移除前必须通过 `check:capability-removal`，确认没有反向依赖、持久化残留或治理引用。

激活失败会回滚本轮已启动的能力；重复停用是幂等的，不会重复调用清理生命周期。仍有启用依赖方时，Runtime 以 `DEPENDENT_PRESENT` 拒绝停用。

## 门禁

```bash
npm run check:pluggable-runtime
npm run check
node scripts/check-capability-platform.mjs
npm run check:capability-removal
```

门禁覆盖：Manifest 校验、配置 schema、Discovery、边界约束、配置解析、依赖自动补齐、确定性激活顺序、`enabled=false`、缺失依赖拒绝、反向依赖保护、生命周期事件、激活回滚、Contribution 注册与摘除、移除前置检查。

本轮增加了异步失败恢复、并发拒绝、暂停直接恢复、重复 bootstrap、不可用存储与原型危险键、真实组件保留和多轮启停测试。新增生命周期测试已接入统一检查与 pre-merge。测试路径同步到迁移后的真实模块，并保留坏样本检查；没有通过移除安全断言来消除报错。

### 验收范围

正式前端构建和真实 release deb 解包执行文件均已逐项验证 Manager 打开、暂停、恢复、停用、完整重载后仍停用、进程重启后仍停用、重新启用和入口去重；未再出现缺失面板提示。真实截图与边界说明见 `logs/acceptance/capability-manager-20261004/README.md`。

能力平台检查 47/47、Runtime 检查 16/16、三轮生命周期测试、统一检查、构建和 pre-merge 均通过。Rust 只做了格式整理，历史文档空白已修复。构建体积通过移除不再需要的 Markdown/HTML 大依赖和收口 Browser 分块循环降至约 990 KB；没有修改阈值或基线。Workspace 编辑器最后一处未声明 Browser store 依赖已改为共享窄意图缝，UI 边界 warn=0；能力 Registry/Dependency 元数据也已对齐至 warn=0。

生成的 deb、desktop 入口、解包内容和解包 release 冷启动均已验证。2026-10-05 已完成交互式 apt 覆盖安装；`/usr/bin/mvp-browser-os` 与 deb 解包二进制 SHA-256 一致，统一安装版脚本 0 fail / 0 warn。真实 `/usr/bin` 已完成窗口像素、暂停/恢复、暂停/停用、刷新保持、进程重启保持和重新启用验收，结论为 `GUI_PASS`，证据见 `logs/acceptance/installed-client-20261005/README.md`。

### 后续优化顺序

1. 已实施：优先补齐操作闭环、并发保护、失败恢复、存储容错和贡献响应式更新，并把反例纳入检查。
2. 已实施：修复重构后失效的测试入口、模块路径和测试初始化，恢复资源、会话、终端、Git、脚本 UI、图片与命令安全检查的有效覆盖。
3. 已实施：追溯 `4f6f338` 及原生物理边界记录，确认创建页签时已移除 Session 草稿写入，保存直接读取真实页签。检查同步为禁止重新引入耦合、保证手动保存读取页签、保证退出保存尊重自动保存开关，并补齐对应反例；没有改动冻结的原生实现。
4. 已实施：核对 `f0733c2 → 5b4738f → ea62872` 的锁文件差异，仅增加 Vault/Clipboard 两个本地能力包及 TypeScript 开发依赖，现有第三方运行依赖未变化。三个依赖检查同步至已核对的 `ea62872` 固定哈希，继续拦截后续漂移；本轮没有新增依赖或改锁文件。
5. 已实施：消除 Browser public/store 跨 chunk 循环，移除空 vendor chunk；以本地安全转换器替换只用于窄场景的重量 Markdown/HTML 依赖，配套 XSS、HTML→Markdown 和 Vault 逻辑测试。项目体积回到既有 25.2% 门禁以内，未改基线。
6. 已实施：Workspace 关闭编辑器改走 `browserNav` 窄意图缝，清除最后一个 UI 未声明跨能力依赖；Registry/Dependency 清单按真实 bridge 与 optional 关系对齐。
7. 已实施：交互式安装生成的 deb；安装版脚本通过，并以 `/usr/bin/mvp-browser-os` 完成冷启动、像素、生命周期和进程重启回归，installed-client 已升级为 `GUI_PASS`。
8. 后续模块化顺序与逐能力证据见 `../capability-platform/FINAL-MODULARITY-AUDIT-20261004.md`；不得把 Bookmark 试点的成功外推成所有能力均可安全停用。
