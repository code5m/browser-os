# Capability Registry

> Phase 7B — Capability Contract & Registry
> 基线：`semantic-governance-v1`　分支：`feature/capability-platform-v1`

---

## 1. 它解决什么（与 Semantic Registry 的分工）

| 问题 | 由谁回答 |
|---|---|
| 「这个状态/意图**是什么意思**？谁拥有、谁能写、有什么副作用？」 | **Semantic Registry**（冻结基线） |
| 「这个能力**提供什么、依赖什么**、生命周期与资源策略是什么？」 | **Capability Registry**（本文） |

**禁止重复维护同一个事实**：
- manifest 只用 `semanticOwner` **引用** Semantic Registry 的 owner，**不得重新声明**语义。
- 需要新增 State / Intent / Owner / Writer / SideEffect → 先走 **SCR → Reviewer → 更新 Semantic Registry**，Capability Registry 才能引用。
- 违反由 `check-capability-registry.mjs` + `check-semantic-registry.mjs` 双门禁阻断。

---

## 2. 文件索引

| 文件 | 内容 |
|---|---|
| `capabilities.yaml` | 能力清单 + manifest（18 个 CAPABILITY） |
| `dependencies.yaml` | 依赖边、共享基础设施、禁止边、已知耦合 |
| `resources.yaml` | Resource Class / 生命周期 / 每能力资源策略（DECLARED） |

---

## 3. CapabilityManifest 字段契约

| 字段 | 必填 | 说明 |
|---|---|---|
| `id` | ✅ | 唯一标识（小写下划线），重复即阻断 |
| `name` | ✅ | 展示名 |
| `category` | ✅ | `CAPABILITY` / `SUB_CAPABILITY` / `UI_COMPONENT` / `SERVICE` / `ADAPTER` / `INFRASTRUCTURE` / `IMPLEMENTATION_DETAIL` |
| `provides` | ✅ | 该能力对外提供的能力点 |
| `dependsOn` | ✅ | 强依赖（能力 id 或共享基础设施 id） |
| `optionalDependencies` | ✅ | 可选依赖，缺失不阻断 resolve |
| `lifecycle` | ✅ | `supported` / `default` / `activatable` / `resident` |
| `resources` | ✅ | `class` / `suspendable` / `destroyable`（class 必须来自 resources.yaml） |
| `permissions` | ✅ | 需要的权限（可为空数组） |
| `persistence` | ✅ | `scope` / `sensitive` |
| `entrypoint` | ✅ | 代码落点（文件/目录） |
| `semanticOwner` | ➕ | **引用** Semantic Registry owner；未登记填 `null` |
| `governanceStatus` | ➕ | `GOVERNED` / `OWNER_PENDING_SCR` / `LOCKED` |
| `status` | ➕ | `NOT_INTEGRATED` / `COMPATIBILITY_WRAPPED` / `TARGET_COMPOSABLE` |

> `➕` 为本 Registry 的治理扩展字段（不重复语义，只标注治理状态）。

---

## 4. 硬规则

```text
R-A  semanticOwner = null 或 governanceStatus != GOVERNED  →  lifecycle.activatable 必须为 false
R-B  resources.class 每一项必须在 resources.yaml 的 resource_classes 中定义
R-C  dependsOn / optionalDependencies 引用的 id 必须存在（能力或 shared_infrastructure）
R-D  不得出现循环依赖
R-E  id 不得重复
R-F  不得出现 dependencies.yaml 中 forbidden_edges 声明的方向
R-G  每个 capability 必须在 resources.yaml 有对应 policy
R-H  status = COMPATIBILITY_WRAPPED 的能力才能被 Runtime activate（其余只能 register/resolve）
```

---

## 5. Capability SDK 最小接口（设计，不造框架）

> 目标：**四个概念、零 DI 容器、零 God Runtime**。
> 类型定义见 `src/capability/types.ts`。

```ts
// 1) 能力定义 —— 就是 manifest 的类型化镜像
interface CapabilityDefinition {
  id: string
  name: string
  category: CapabilityCategory
  provides: string[]
  dependsOn: string[]
  optionalDependencies: string[]
  lifecycle: CapabilityLifecycle
  resources: CapabilityResourcePolicy
  permissions: string[]
  persistence: { scope: string; sensitive: boolean }
  entrypoint: string
  semanticOwner: string | null
  governanceStatus: 'GOVERNED' | 'OWNER_PENDING_SCR' | 'LOCKED'
  status: 'NOT_INTEGRATED' | 'COMPATIBILITY_WRAPPED' | 'TARGET_COMPOSIBLE'
}

// 2) 生命周期声明 —— 能力"声明"自己支持什么，Runtime 不得越权
interface CapabilityLifecycle {
  supported: CapabilityState[]      // ACTIVE / SUSPENDED / BACKGROUND ...
  default: CapabilityState
  activatable: boolean              // 受 R-A 约束
  resident: boolean                 // 是否常驻（不可卸载）
  onActivate?: () => void | Promise<void>
  onSuspend?: () => void | Promise<void>
}

// 3) 上下文 —— Runtime 传给能力的**只读**句柄，不是状态容器
interface CapabilityContext {
  readonly capabilityId: string
  readonly state: CapabilityState
  // 只允许能力声明自己要用的东西；Runtime 不注入业务状态
  log(message: string): void
}

// 4) 资源策略 —— DECLARED，不含实测数字
interface CapabilityResourcePolicy {
  class: ResourceClass[]
  suspendable: boolean
  destroyable: boolean
}
```

**明确不做**（今晚）：
- DI 容器 / 反射加载 / 动态 `require`
- Runtime 持有业务状态
- 能力之间直接互相调用（只能经 `provides` 声明）

---

## 6. 运行校验

```bash
node scripts/check-capability-registry.mjs              # 扫描真实 registry
node scripts/check-capability-registry.mjs --self-test  # 自检（positive/negative/false-positive 夹具）
node scripts/check-capability-registry.mjs --strict     # 提示级也判失败
node scripts/check-capability-registry.mjs --json       # 机器可读
node scripts/check-capability-registry.mjs --help
```
