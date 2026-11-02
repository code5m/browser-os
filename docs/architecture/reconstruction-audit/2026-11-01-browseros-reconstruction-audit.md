# BrowserOS 架构重构验收调研报告

> 审计日期：2026-11-01  
> 审计仓库：code5m/browser-os  
> 审计基线：master @ 9607e260f86b15a730219c619ab5a20cc01efe65  
> 调研分支：research/architecture-reconstruction-audit-20261101  
> 范围：GitHub 静态代码、架构注册表、检查脚本、文档及当前 CI 证据。只允许新增调研文档；没有修改应用代码、测试、构建脚本、CI 工作流或运行时配置。

## 0. 结论摘要

**当前不能判定 BrowserOS 已达到这十项重构验收标准。**

项目已经形成了相当完整的架构治理框架：Capability Runtime、模块 Manifest、Public API 入口、Semantic Owner Registry、Native Command Registry、资源归属表、生命周期检查、模块边界检查和安装版 GUI CI 都已经存在。

主要问题不是缺少架构文档，而是部分架构约束尚未成为真正的运行时约束或完整 CI 门禁。尤其是：

1. 资源生命周期存在同步/异步契约不一致，销毁语义不完整。
2. Public API 有入口，但接口过宽，缺少对真实导出和兼容性的强制验证。
3. Native Command 的 Owner 登记较完整，但仍有 100/178 个命令留在 bridge.rs。
4. 权限与确认控制没有形成覆盖所有关键副作用的统一策略。
5. 跨模块引用检查分散在多个专项脚本中，无法证明所有模块和 Rust 层都不存在未经批准的内部引用。
6. 安装版自动验收已有进展，但人工 GUI 验收、运行时性能前后对比尚未闭环。
7. 多份治理文档、注释和代码现状已经不一致。

建议把本报告作为下一阶段重构的验收基线，而不是把已有 Registry 或 CI PASS 当作重构完成的证明。

## 1. 十项验收标准审计矩阵

| # | 验收标准 | 当前判断 | 主要原因 |
|---|---|---|---|
| 1 | 所有业务模块有唯一 Owner | 部分达到 | Owner Registry 有 27 个唯一 Owner 名称，但 Manifest 允许空 Owner，且存在 recents 命名碰撞 |
| 2 | 所有新功能经正式 Public API | 部分达到 | 有 public.ts 和 publicContract，但导出过宽，缺少真实符号、版本及兼容性验证 |
| 3 | 所有 Native Command 有明确业务归属 | 登记层达到 | 178/178 条命令均有 Owner，共 22 个 Owner；物理代码仍未完全按 Owner 拆分 |
| 4 | 所有重要资源有明确生命周期 | 部分达到 | 有资源声明和 Owner 生命周期测试，但 destroy、异步完成和统一状态机不完整 |
| 5 | 不存在未经批准的跨模块内部引用 | 尚未证明 | 有边界检查，但存在别名、共享目录、Rust 内部依赖等覆盖缺口 |
| 6 | 所有关键副作用有权限和确认控制 | 未达到 | 没有统一的 command-level confirmation 策略；权限登记不等于运行时强制执行 |
| 7 | 模块测试可独立运行 | 部分达到 | 有独立检查脚本，但大多数业务模块仍处于单一应用测试体系 |
| 8 | 安装版 GUI 真实验收通过 | 部分达到 | CI 已执行安装包冷启动和自动 GUI 回归；本地人工验收仍待确认 |
| 9 | 性能数据与重构前基线对比 | 未达到 | 有前端体积对比和当前资源采样，但缺少同环境运行时前后基线 |
| 10 | 文档和实际代码一致 | 未达到 | Native 命令数、Owner 范围、Agent/Skill 状态、PR 状态等多处存在漂移 |

## 2. 业务模块 Owner

### 已经具备的基础

- Semantic Owner Registry 中有 27 个 Owner 记录，Owner 名称没有重复。
- Native Command Registry 中 178 条命令全部登记了 Owner，覆盖 22 个 Owner 分组。
- Workspace 已经从 God Store 拆出 File、Artifact、Repo、Script、Snippet 等子域 Owner。
- Developer、Terminal、Workspace 等模块已有专项 Owner 检查。

这是项目目前做得比较好的部分。问题在于：**登记完整不等于约束不可绕过。**

### 仍需解决

1. CapabilityManifestV1 的 semanticOwner 类型允许 string 或 null，validateManifestV1 允许 null。contract-drift 检查也只在 Registry 和 Manifest 两边都有值时比较，不会因为 Manifest 缺 Owner 就失败。
2. Owner Registry 明确记录了 workspace.recents 与 home.recents 同名碰撞，并建议改名为 workspaceRecents。这是一个尚未消除的语义歧义。
3. Registry 的 registry_scope 仍只描述 Phase 1-5，但文件内容已经包含后续阶段的 Workspace、Graph、Plugin 等 Owner，范围说明没有随实际内容更新。
4. 一些共享字段被标为 observed/shared，这是合理的只读投影方向，但必须明确其唯一写入 Owner，不能让观察者演变为第二个写入方。

### 验收建议

- 所有可激活的业务 Capability 必须有非空 semanticOwner。
- Owner Registry、Manifest、真实 Store/Service 写入点必须三方一致。
- 共享状态必须标记 sourceOwner、projectionOnly 和允许的读取方向。
- 对 workspace.recents/home.recents 做语义拆分或明确唯一归属。
- CI 对缺失 Owner、重复 Owner、越权写入直接失败，而不是只生成报告。

证据：[owners.yaml](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/architecture/semantic-registry/owners.yaml#L9-L14)、[recents 归属记录](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/architecture/semantic-registry/owners.yaml#L995-L1014)、[CapabilityManifestV1](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capability/platform/contract.ts#L72-L115)、[Manifest 校验](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capability/platform/contract.ts#L138-L228)。

## 3. Public API

### 已经具备的基础

- 多数 Capability 已有 manifest.ts、index.ts、public.ts。
- Manifest 已声明 publicContract。
- Workspace、Browser、Agent、Skill 等模块已有对外入口。
- 依赖装配可以通过 provides/requires 和 publicContract 名称解析契约。

### 关键缺口

1. PublicContractEntry 目前只有 name 和 locator，没有稳定的 API 版本、导出符号签名、错误契约、权限契约或弃用策略。
2. validateManifestV1 没有验证 locator 是否指向真实导出，也没有验证 public.ts 是否真的导出该符号。providedContractNames 主要用于名字集合。
3. 多个 public.ts 直接导出整个 Pinia Store。调用方因此可以接触大量内部 state 和 actions，而不是只能使用少量稳定的模块方法。
4. check-capability-boundaries.mjs 把 index.ts、manifest.ts、public.ts 都视为 public。index.ts 和 manifest.ts 是装配/描述文件，不应自动等同于业务 API。
5. 边界检查对非相对路径别名不做解析；部分共享目录也被排除在该检查之外。因此不能仅凭该检查通过就证明所有消费者都走正式 Public API。
6. check-capability-package.mjs 的包级契约检查只覆盖独立 packages，不代表所有 src/capabilities 内的模块都具备同等强度的 API 校验。

### 验收建议

每个模块只允许一个明确的业务公共入口，建议使用 public.ts 或独立 package exports。公共接口应导出方法、类型和事件，不直接暴露完整 Store。Manifest 至少声明 API 版本和符号清单；CI 必须验证每个 locator、导出符号和调用方依赖均真实存在。

证据：[PublicContractEntry 与 Manifest](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capability/platform/contract.ts#L72-L115)、[Agent public.ts](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capabilities/agent/public.ts)、[Workspace public.ts](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capabilities/workspace/public.ts)、[边界检查规则](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/scripts/check-capability-boundaries.mjs#L27-L149)。

## 4. Native Command 归属与拆分

### 已经达到的部分

对当前 master 的 native-commands.yaml 进行统计：

- Native Command 总数：178。
- 已登记 Owner：178。
- Owner 分组：22。
- 缺失 Owner：0。
- 仍位于 src-tauri/src/bridge.rs：100。
- 位于其他能力模块文件：78。

因此，**命令的语义 Owner 登记已经基本完成，但物理实现仍有明显的集中化问题。**

### 关键缺口

1. bridge.rs 仍承载 100 个命令，约占总数的 56%。Capability 目录虽然已有 Agent、Browser、Database、Git、Graph、Plugin、Script、Session、Skill、Task、Terminal、Tools、Workspace 等 Native 模块，但还没有完成全量 Owner 内聚。
2. Native Registry 中 reveal_path 和 move_path 的 file 字段仍指向已不存在的 src-tauri/src/fs_cmds.rs；当前文件位于 src-tauri/src/capabilities/workspace/fs_cmds.rs。
3. native-commands.yaml 的 generated 日期仍为 2026-09-23；NATIVE_CAPABILITY_BOUNDARY_AUDIT.md 仍记录 148 个命令全部集中在 bridge.rs，与当前 178 个命令、100 个仍在 bridge.rs 的现状不一致。
4. Native Command 检查能够验证命令定义、注册、Owner 和部分调用关系，但没有验证 Registry 的 file 字段是否与实际函数位置一致。

### 验收建议

- 每个 Native Command 只有一个业务 Owner。
- 命令实现放在 Owner 模块，main.rs 只负责统一注册，不承载业务实现。
- bridge.rs 只保留兼容转发或明确归属的框架命令。
- Native Registry 从真实 Rust 命令定义和注册表生成，file、owner、permission、resource 全部与代码做双向校验。
- 任何未登记命令、重复命令、错误 Owner、错误物理路径均使 CI 失败。

证据：[Native Command Registry](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/architecture/native-boundary/native-commands.yaml)、[旧 Native 审计快照](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/architecture/native-boundary/NATIVE_CAPABILITY_BOUNDARY_AUDIT.md#L9-L100)、[Native 边界检查](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/scripts/check-native-capability-boundaries.mjs)。

## 5. 资源生命周期

这是本次调研发现的核心架构风险之一。

### 已有能力

- CapabilityRuntime 已有 register、resolve、activate、suspend、disable、unregister。
- 生命周期接口包含 onActivate、onSuspend、onDeactivate。
- 有 activateAsync 和 suspendAsync。
- Browser、Grid、Terminal、Database、Git 等模块已经有 Owner 级生命周期实现和 Hot-Plug Acceptance 检查。

### 核心缺口

**A. 同步/异步生命周期不一致**

CapabilityLifecycle 明确允许生命周期 Hook 返回 Promise，但默认 bootstrap 和 pluggable runtime 使用同步 activate/suspend。同步 runtime.activate/suspend 调用 Hook 后不 await；异步 Hook 的完成、失败和清理结果因此无法被调用方可靠感知。

bootstrapCapabilityRuntime 的 profile 路径调用 rt.activate；bootstrapAssembly 也调用 rt.activate；createPluggableRuntime.activate 同样调用 runtime.activate。虽然 Runtime 提供 activateAsync/suspendAsync，但这些默认装配路径没有统一使用异步生命周期契约。

**B. Destroy 语义没有真正实现**

- CapabilityState 声明了 DESTROYED。
- CapabilityLifecycle 没有 onDestroy。
- CapabilityRuntime 没有 destroy 方法。
- ResourceGovernor.destroy 只调用 runtime.disable，不会调用资源释放 Hook，也不会 unregister。
- runtime.disable 明确拒绝 ACTIVE 状态；ResourceGovernor.destroy 却直接调用 disable，因此对 ACTIVE 能力不能完成预期销毁。
- ResourceGovernor 文档称资源释放由 onDestroy 负责，但类型契约没有 onDestroy。

这意味着目前的 destroy 更接近“尝试停用”，不是资源销毁。

**C. 通用 Hot-Plug 路径没有形成可信的统一状态机**

createOrchestrator 另有一份 states Map；setState 只更新这份 Map，不更新 CapabilityRuntime。requestAdd 可以把平台状态标成 ACTIVE，却没有调用 runtime.activate 或 onActivate。requestAdd 的 permit 是可选参数，而 createOrchestrator.add 没有传入 permit。加载器还会在 requestAdd 和 addContributions 两处被调用，后者不等待 Promise。

这套 requestAdd/requestRemove 路径没有被当前 check-hot-plug-acceptance 的核心测试覆盖；该测试主要验证 CapabilityManager/transitionCapability 和所选 Owner 生命周期。不能把专项 Hot-Plug PASS 理解成所有动态安装、移除和资源销毁路径都已验收。

### 验收建议

统一为一个异步状态机：

DEFINED → REGISTERED → READY → ACTIVE ↔ SUSPENDED → DISPOSING → DESTROYED

每个资源必须有唯一 Owner 和明确的 acquire/release；dispose 必须 await，幂等执行，有超时、取消、失败记录和补偿策略。所有状态只能由同一个 Runtime 保存，禁止 Orchestrator 再维护第二份状态真相。对每个资源类型做创建、暂停、恢复、销毁、重复销毁、Hook 失败、依赖阻止移除的测试。

证据：[Lifecycle 类型](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capability/types.ts#L22-L70)、[Runtime 同步/异步实现](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capability/runtime.ts#L185-L277)、[Bootstrap 激活路径](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capability/index.ts#L75-L222)、[Pluggable Runtime](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capability/platform/pluggable.ts#L53-L99)、[Resource Governor](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capability/resourceGovernor.ts#L8-L62)、[Hot-Plug Orchestrator](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capability/platform/orchestrator.ts#L54-L133)。

## 6. 跨模块引用

当前不是没有边界检查，而是检查规则分散且覆盖不完整。

- check-capability-boundaries.mjs 只解析相对路径；别名 import 不进入该规则。
- 该规则把 index.ts、manifest.ts、public.ts 全部视为 public。
- shared 目录在该规则中被跳过。
- check-capability-boundary.mjs 的另一组检查只覆盖部分 runtime/platform 和 state/ui/index 路径。
- check-native-capability-boundaries.mjs 的 NATIVE-02 主要依据 bridge.<command> 调用点检查能力间直调，并不能构成完整的 TypeScript、Rust 模块依赖图。
- Resource、Developer、Workspace、Terminal 等专项检查有价值，但它们不能替代全仓统一的依赖图约束。

### 验收建议

生成完整模块依赖图，统一解析相对路径、tsconfig/Vite 别名、动态 import、Rust use/mod/pub(crate) 和 Tauri invoke。默认只允许依赖对方 public.ts 或正式 package exports。shared 只能依赖基础设施，不能反向依赖业务模块。Rust 层也必须有 Owner 边界检查。

最终 CI 输出所有跨模块边及其批准理由；没有批准记录的内部引用直接失败。

证据：[通用 Capability 边界检查](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/scripts/check-capability-boundaries.mjs)、[窄边界检查](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/scripts/check-capability-boundary.mjs)、[Native 边界检查](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/scripts/check-native-capability-boundaries.mjs)。

## 7. 关键副作用的权限与确认

**当前这是未达到验收标准的项目。**

统计当前 Native Command Registry：

- 178 条命令中，58 条 permission 明确登记为 “-”。
- Registry 没有统一的 confirmation 字段。
- 41 个副作用类别中，4 个 requires_declaration=true，36 个为 false，另有 1 个未声明。
- side-effects.yaml 的 awareness_marker 是代码注释提醒，不是运行时权限或确认机制。
- check-native-semantic-closure 只要求 permission 字段存在，permission 为 “-” 也可以通过。
- check-sensitive-side-effects.mjs 主要检查 Keyring 凭据泄露，不覆盖所有文件写入、进程启动、浏览器脚本执行、持久化删除等副作用；它没有进入根 check 命令链或当前 GitHub Actions 工作流。
- Tauri default capability 将 default-commands 授予 main 窗口，属于粗粒度 WebView 访问控制，不是每个业务副作用的细粒度授权。

目前已经存在一些有效的专项确认流程，例如 Git 写入预览/确认、Coding Agent 文件修改审批、部分 Skill 安装确认、Credential Keyring 限制。但不能据此推导所有关键副作用都有同等级控制。

Native Registry 中 permission 为 “-” 的示例包括 session_delete、session_restore、snippet_remove、remove_bookmark、clipboard_read、clipboard_write、eval_in_tab。它们不一定都需要相同级别的确认，但必须有明确的风险分级和策略，不能统一留空。

### 验收建议

为每条关键 Native Command 增加明确的副作用策略：

- effect：read / write / delete / process / network / credential / browser-eval。
- permission：命名权限，默认拒绝未声明操作。
- confirmation：none / user-gesture / preview-and-confirm / elevated-confirm。
- scope：目标路径、工作区、进程、WebView 或资源范围。
- audit：是否记录调用人、目标、时间和结果。
- rollback：可回滚、幂等补偿或不可逆。

对于删除、执行进程、写入外部文件、修改 Git、访问密钥、在网页上下文执行脚本等操作，必须在 Native 边界再次校验，而不能只依赖前端按钮是否弹出确认框。确认应绑定目标和操作摘要，防止确认后目标被替换。

证据：[副作用 Registry](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/architecture/semantic-registry/side-effects.yaml)、[Native Command Registry](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/architecture/native-boundary/native-commands.yaml)、[Tauri 默认 capability](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src-tauri/capabilities/default.json)、[默认命令 ACL](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src-tauri/permissions/default-commands.toml)、[Keyring 专项检查](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/scripts/check-sensitive-side-effects.mjs)。

## 8. 模块测试独立运行

### 已有基础

- Capability Runtime、Pluggable Lifecycle、Hot-Plug Acceptance 有独立脚本。
- Browser/Grid/Terminal、Workspace、Developer 等模块有专项静态检查。
- Vault 和 Clipboard 有独立 package 及 package-level check。
- 根 check 流程覆盖大量架构和回归检查。

### 缺口

- 根 package.json 没有统一的 test 脚本；根 check 是长串行门禁。
- 大部分业务模块仍在 src/capabilities 中作为同一个应用的一部分，没有各自的 package/test 入口。
- 只有少数模块有独立 package-level check；其它模块主要依赖集中式脚本和全应用检查。
- Hot-Plug Acceptance 验证的是所选模块生命周期和 transitionCapability，不代表每个模块都有独立行为测试，也没有覆盖 generic orchestrator 的完整 add/remove 流程。

### 验收建议

每个业务模块必须提供可独立运行的 test、typecheck、contract-check 和 integration-test 入口。单模块测试不得依赖整个 BrowserOS GUI 启动。根 check 只负责聚合调用模块测试，不应成为唯一可执行路径。

证据：[根 package.json](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/package.json)、[Hot-Plug Acceptance](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/scripts/check-hot-plug-acceptance.mjs)、[Pluggable Lifecycle](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/scripts/check-pluggable-lifecycle.mjs)。

## 9. 安装版 GUI 验收

### 已经通过的自动化部分

最新 master Full Validation 运行：  
[Run 41284054999](https://github.com/code5m/browser-os/actions/runs/41284054999)

Validate Job 已通过：

- Packaged GUI cold-start acceptance。
- Full Tauri GUI regression。
- Resource governance and visual comparison。
- 安装包资源证据上传。

当前工作流实际执行安装包安装，并在 DBus/Xvfb 环境启动安装后的客户端；5 分钟资源采样也已纳入该流程。这比单纯的前端构建检查更有价值。

### 尚未完成的部分

- FINAL-HUMAN-ACCEPTANCE 和 STAGE-J-HUMAN-ACCEPTANCE 文档仍保留人工验收待确认项。
- CI 的 Xvfb/DBus 安装版验收不能替代真实 Ubuntu 桌面环境下的人工操作验收。
- 本次审计时 Full Validation 尚未整体通过：native-regression 子任务在 cargo fmt --check 阶段失败，涉及 src-tauri/src/grid_process.rs 和 src-tauri/src/main.rs；Validate Job 当时仍在执行。

因此状态应记录为：**CI 安装版自动验收有通过证据；本地人工 GUI 验收未确认；Full Validation 整体不通过/未完成。**

证据：[Full Validation 工作流](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/.github/workflows/capability-v2-validation.yml)、[安装版验收脚本](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/scripts/verify-installed-client.sh)、[人工验收状态](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/architecture/capability-platform/FINAL-HUMAN-ACCEPTANCE.md)、[人工验收清单](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/architecture/capability-platform/STAGE-J-HUMAN-ACCEPTANCE-H01-H10.md)。

## 10. 性能基线与对比

### 已有数据

- FRONTEND-SIZE-DUAL-BASELINE 有 master 与 perf 同口径前端产物大小对比：12,975,246 bytes 对比 12,976,614 bytes，差异 1,368 bytes，约 +0.01054%。
- 历史 M0 前端体积基线记录为 795,517 bytes；当前产物约 12.98 MB。该差异主要说明产品/编辑器能力增长，不能直接等同于运行时性能退化。
- footprint-baselines/master-7ae6fa86.json 记录的 frontend_dist_bytes 为 995,732，且包含 release binary/deb 体积；该文件并不包含运行时 CPU、PSS/RSS、IPC 延迟或 UI 响应数据。
- 最新 Full Validation 已运行 5 分钟安装版资源采样，并上传资源证据。

### 尚未达到的部分

1. 当前基线文件主要是前端产物、二进制和安装包大小，不是重构前运行时性能基线。
2. 没有看到同一硬件、同一系统、同一构建参数下，重构前后 CPU、PSS/RSS、线程数、子进程数、启动耗时、IPC p95/p99、空闲/休眠/恢复耗时的完整对比。
3. 5 分钟当前版本采样不等于前后对比；30 分钟稳定性证据也需要单独记录。
4. PERFORMANCE-P0-P2-TODO 仍标记实测改善数据缺失和 GUI_PENDING；其中 PR #43 的状态文字仍称 Draft/未合并，但 PR #43 已经合并，说明该 TODO 文件本身需要状态更新。

### 验收建议

建立不可变的 pre-refactor baseline，保存 commit、硬件、OS、显卡、构建参数、测试场景和原始 JSON。重构分支与基线必须同机或同规格 Runner 测量。至少比较：

- 冷启动、热启动、首屏可交互时间。
- 空闲、普通浏览、多标签、Grid、Monaco、Agent 执行时的 PSS/RSS、CPU、线程和子进程。
- Tab 休眠、恢复、关闭、长时间运行后的资源回收。
- Native IPC p50/p95/p99。
- 5 分钟和 30 分钟稳定性。
- UI 卡顿、长任务、内存增长斜率和资源泄漏。

最终报告必须给出基线、候选版本、差值、百分比、测试环境和原始证据文件；只有体积对比不能通过性能验收。

证据：[双基线体积对比](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/engineering/FRONTEND-SIZE-DUAL-BASELINE-20261031.md)、[Footprint 基线 JSON](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/engineering/footprint-baselines/master-7ae6fa86.json)、[性能证据说明](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/engineering/SIZE-AND-PERFORMANCE-EVIDENCE.md)、[P0-P2 性能 TODO](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/engineering/PERFORMANCE-P0-P2-TODO.md)。

## 11. 文档与代码一致性

当前至少有以下可确认的漂移：

| 文档/代码 | 发现 | 影响 |
|---|---|---|
| NATIVE_CAPABILITY_BOUNDARY_AUDIT.md | 仍写 148 个命令全部位于 bridge.rs；当前 Registry 是 178 个，其中 100 个在 bridge.rs | 不能作为当前 Native 架构现状 |
| native-commands.yaml | generated 日期为 2026-09-23；reveal_path/move_path 的文件路径已失效 | 命令位置证据不可信 |
| owners.yaml / side-effects.yaml | registry_scope 仍停留在 Phase 1-5，但内容已扩展到后续模块 | 治理范围说明过期 |
| PERFORMANCE-P0-P2-TODO.md | PR #43 仍写 Draft/未合并；实际已合并 | 进度状态过期 |
| agent/manifest.ts、agent/public.ts | 仍写 Skill 状态由 Agent Store 持有、Stage E 才拆分 | 与当前 Skill 独立 Store/Manifest 的事实冲突 |
| security_policy.rs | 顶部注释称没有生产调用方；framework/commands.rs 已调用 check_launch_target | 安全策略文档注释过期 |
| owners.yaml | 明确记录 workspace.recents 与 home.recents 同名碰撞 | 语义归属仍有待收敛项 |

建议将架构文档分为三类：当前规范、自动生成的 Registry、历史审计快照。历史快照必须注明基线 commit 和 superseded 状态；当前规范中的 Owner、Native Command、资源、权限和 Public API 信息应尽可能从代码生成或由 CI 双向验证。

证据：[Native 审计快照](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/docs/architecture/native-boundary/NATIVE_CAPABILITY_BOUNDARY_AUDIT.md)、[Agent Manifest](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capabilities/agent/manifest.ts#L4-L84)、[Skill Manifest](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src/capabilities/skill/manifest.ts#L4-L80)、[security_policy.rs](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src-tauri/src/security_policy.rs#L1-L15)、[framework commands](https://github.com/code5m/browser-os/blob/9607e260f86b15a730219c619ab5a20cc01efe65/src-tauri/src/framework/commands.rs#L1-L32)。

## 12. 下一阶段整改优先级

### P0：先建立可信的架构约束

- [ ] 统一 Capability Runtime 状态机，全部生命周期 Hook 使用可 await 的 API。
- [ ] 增加 dispose/destroy 语义，明确资源释放、贡献撤销、依赖移除和失败补偿。
- [ ] 统一副作用策略：permission、confirmation、scope、audit、rollback，并在 Rust Native 边界强制执行。
- [ ] 将 semanticOwner 对业务 Capability 设为必填；消除 Owner 同名碰撞。
- [ ] 建立覆盖 TypeScript 别名、动态 import、Rust 模块和 Native invoke 的完整依赖边界检查。
- [ ] 修复当前 Full Validation 的 Rust 格式失败；本次调研不修改该代码。

### P1：让模块真正可维护、可独立演进

- [ ] 收窄 Public API，不再把完整 Pinia Store 当作默认公共契约。
- [ ] PublicContract 加入 API 版本、导出符号和兼容性检查。
- [ ] 按 Owner 拆分剩余 Native Command；bridge.rs 只负责框架注册和兼容转发。
- [ ] Native Registry 的文件路径、Owner、权限、资源字段与实际代码双向验证。
- [ ] 为每个业务模块提供独立 test/typecheck/contract-check 入口。
- [ ] 为 generic Hot-Plug add/remove、异步 Hook、失败回滚和资源销毁补齐集成测试。

### P2：完成真实产品验收与量化

- [ ] 完成真实 Ubuntu 安装版 GUI 人工验收并记录 H01-H15。
- [ ] 补齐重构前运行时性能基线，执行相同环境下的前后对比。
- [ ] 完成 5 分钟和 30 分钟资源稳定性验证，保存原始 JSON。
- [ ] 修正过期 Registry、架构说明和 TODO 状态；历史文档明确标记快照版本。
- [ ] 建立从代码/Registry 自动生成架构图和验收报告的流程。

## 13. 最终验收门槛

以下条件必须全部满足，才可以宣布本轮重构完成：

1. 所有业务 Capability 的 Owner 非空、唯一，所有状态写入都能追溯到唯一 Owner。
2. 所有跨模块调用只能通过正式 Public API；没有未经批准的内部引用。
3. 所有 Native Command 都有唯一业务 Owner，注册、物理位置、权限和资源声明一致。
4. 所有重要资源都有明确 acquire/release/dispose 生命周期，销毁可 await、幂等且有失败补偿。
5. 所有关键副作用都有权限、确认、范围和审计控制，未声明操作默认拒绝。
6. 每个业务模块都能独立运行测试；根级测试只是聚合器。
7. 安装版 GUI 自动验收通过，真实桌面人工验收通过。
8. 性能对比使用同环境重构前基线，数据和原始证据可复现。
9. 文档、Manifest、Registry、代码和 CI 检查结果一致。
10. Master 与 PR 两套流水线全部 PASS，且没有已知豁免项掩盖实际缺口。

---

## 审计边界声明

本报告是基于 GitHub master 指定 commit 的静态代码与文档审查，以及当前 Actions 状态形成的研究结论。没有在本次调研中运行本地安装包，也没有代替用户进行桌面 GUI 操作。CI 的自动化结果仅按对应 Workflow/Job 记录，不推断为人工验收。

本分支只新增本调研报告；没有修改业务代码、测试代码、构建脚本、CI 工作流或运行时配置。没有合并到 master，也没有创建 PR。
