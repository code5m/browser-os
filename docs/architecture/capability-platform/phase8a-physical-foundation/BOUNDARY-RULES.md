# Capability Boundary Rules（物理边界规范）

> Phase 8A　基线：`capability-preview-v1-code-pass` (`c749426`)
> 目的：在"真正搬代码"之前先立规矩，并让机器能强制它。

---

## 1. 成熟度定义（本轮唯一口径）

| 级别 | 名称 | 含义 | 是否计入 `CURRENTLY_COMPOSABLE` |
|---|---|---|---|
| C0 | REGISTERED | 仅 Registry 登记 | ❌ |
| C1 | WRAPPED | 经 compatibility adapter 接入 Runtime | ❌ |
| C2 | ISOLATED | 物理边界形成，内部实现禁止跨 Capability import | ❌ |
| C3 | OPTIONAL | 不注册/不启用时 Shell 仍能正常启动 | ✅ |
| C4 | RUNTIME_CONTROLLABLE | enable/disable/activate/suspend 真实生效 | ✅ |
| C5 | RESOURCE_RELEASABLE | 禁用/hibernate/destroy 后 Native/Process/WebView/Connection 真实不创建或释放 | ✅ |

> **只有 C3+ 才能计入 composable。** 禁止把 C0/C1 写成 composable。

---

## 2. 目录约定

```text
src/
  capabilities/<name>/
    index.ts          ← 唯一公开入口（public entry）
    manifest.ts       ← 能力声明
    contracts/        ← 对外类型契约（公开）
    state/            ← 内部：状态（禁止外部直接 import）
    services/         ← 内部：业务逻辑
    ui/               ← 内部：界面组件
    adapters/         ← 内部：native / IO 适配（唯一允许碰 native 的地方）
    lifecycle/        ← 内部：activate/suspend 钩子
    resource/         ← 内部：资源策略
  capability/         ← Runtime / SDK（编排层，不做业务 Owner）
  shell/              ← 应用外壳（layout / navigation / capability slots）
```

**公开面（Public Surface）**：`index.ts`、`manifest.ts`、`contracts/**`
**内部面（Internal）**：其余全部目录

---

## 3. 依赖方向（硬规则）

```text
Shell          → Capability public entry            ✅
Capability A   → Capability B public entry（须声明依赖）  ✅
Shared         → 任何地方都可以依赖（但不得变垃圾桶）      ✅

Shell          → Capability internal             ❌ CB-02
Capability A   → Capability B internal           ❌ CB-01
Capability A   → B public entry（未声明依赖）      ❌ CB-03
任何外部        → Capability state/store          ❌ CB-05
任何外部        → Capability 非 public 深路径       ❌ CB-06
Capability 非 adapter 文件 → native bridge        ❌ CB-07
```

---

## 4. Boundary Checker

`scripts/check-capability-boundaries.mjs`（CB-01..CB-07）

| 规则 | 级别 | 含义 |
|---|---|---|
| CB-01 | fail | 跨 Capability import 内部实现 |
| CB-02 | fail | Shell import Capability 内部实现 |
| CB-03 | fail | 未声明 dependency 却依赖另一 Capability |
| CB-04 | fail | Capability 循环依赖 |
| CB-05 | fail | 直接跨 Capability Store mutation / 直取 state |
| CB-06 | fail | 绕过 public entrypoint |
| CB-07 | warn（--strict 下 fail） | Capability 非 adapter 文件直接触碰 native side effect |

```bash
node scripts/check-capability-boundaries.mjs              # 真实扫描
node scripts/check-capability-boundaries.mjs --self-test  # 自检
node scripts/check-capability-boundaries.mjs --strict
node scripts/check-capability-boundaries.mjs --json
node scripts/check-capability-boundaries.mjs --help
```

**Checker 自身未证明可信前不接入 pre-merge**（本阶段仅本地验证）。

---

## 5. 反模式禁令

```text
禁止把代码全搬进 shared/ 来"消除 import"      → shared 只能是稳定抽象
禁止万能 Event Bus / globalEventBus.emit(*)   → 必须注册 typed event contract
禁止把业务 State Owner 迁进 Runtime           → Runtime 只做 orchestration
禁止建立 DI 容器 / 反射加载 / God Runtime
禁止改动 Semantic Registry 的语义（owner/writer/intent）
  —— 但允许在文件物理迁移后同步更新 governed_files 的"路径"（非语义变更）
```

---

## 6. 重要约束：Semantic Registry 的路径耦合

Semantic Registry 的 `states.yaml` 通过 `governed_files` **按路径**确定治理范围，当前包含：

```text
src/stores/useLayoutStore.ts        src/stores/useBrowserStore.ts
src/composables/useBrowserHost.ts   src/stores/useWorkspaceStore.ts
src/components/workspace/FilePanel.vue
src/stores/useBookmarkStore.ts      src/stores/useSystemStore.ts
```

**因此**：任何物理移动这些文件的操作，必须同步更新 `governed_files` 路径，
否则 R2/R8/R9 会**静默失去覆盖**（比报错更危险）。

处理方式：迁移即同步修路径 + 全量重跑语义门禁证明覆盖不丢。
**语义本身（owner/writer/state 含义）不变。**
