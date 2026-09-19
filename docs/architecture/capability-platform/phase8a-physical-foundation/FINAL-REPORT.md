# Phase 8A — Physical Foundation（物理基础）

> 分支：`feature/capability-platform-v1`
> 基线：`capability-preview-v1-code-pass` (`c749426`)　开工点：`15bb6e4`

---

## 1. 本轮口径变更（重要）

Preview v1 的 `COMPATIBILITY_WRAPPED` 不再算 composable。本轮统一成熟度：

| 级别 | 含义 | 计入 CURRENTLY_COMPOSABLE |
|---|---|---|
| C0 REGISTERED | 仅登记 | ❌ |
| C1 WRAPPED | adapter 接入 Runtime | ❌ |
| C2 ISOLATED | 物理边界形成 | ❌ |
| C3 OPTIONAL | 不启用时 Shell 仍能启动 | ✅ |
| C4 RUNTIME_CONTROLLABLE | 生命周期真实生效 | ✅ |
| C5 RESOURCE_RELEASABLE | 资源真实不创建/释放 | ✅ |

> 按此口径，Preview v1 结束时 **CURRENTLY_COMPOSABLE = 0**（全部最多 C1）。

---

## 2. 交付物

```text
docs/architecture/capability-platform/phase8a-physical-foundation/BOUNDARY-RULES.md
  目录约定 / public 面 / 依赖方向 / 反模式禁令 / 成熟度定义
scripts/check-capability-boundaries.mjs
  CB-01 跨 Capability import 内部          CB-05 跨 Capability 取 state/store
  CB-02 Shell import Capability 内部       CB-06 绕过 public entrypoint
  CB-03 未声明 dependency                  CB-07 非 adapter 触碰 native (warn)
  CB-04 Capability 循环依赖（必须/可选分层）
  --help / --self-test / --json / --strict
```

**架构要点**：核心 `analyze(files, deps, optionalDeps)` 接受「虚拟文件表」，
自检用夹具、真实扫描读磁盘 —— **测的是同一套逻辑**，避免"测试一套、运行另一套"。

---

## 3. 自检与真实扫描

```text
SELF_TEST: PASS (12/12)
  3 positive  + 6 negative + 3 false-positive 夹具

真实扫描: CAPABILITY_BOUNDARIES_RESULT=PASS (fail=0 warn=1)
  warn = CB-04 agent <-> knowledge_graph 双向「可选」依赖环

语义治理回归: SEMANTIC_REGISTRY_RESULT=PASS (fail=0 warn=6 info=72)
              SEMANTIC_CLOSURE_LOGIC_RESULT=PASS (27/27)
BUILD: ✓ built in 5.07s
```

---

## 4. 过程中修掉的三个真实缺陷（自家 bug，非降低标准）

1. **CB-07 不可达**：`if (!t.owner) continue` 写在 CB-07 判定之前，
   而 bridge 永远不属于任何 capability → 该规则**永远不可能触发**。已把 CB-07 前移。
2. **带扩展名的 import 无法解析**：真实代码里 `.vue` import 带扩展名，
   而文件表 key 是去扩展名的，导致所有 Shell→UI 越界**全部漏检**。已兼容两种写法。
3. **两个 checker 的依赖口径不一致**（详见 §5）。

---

## 5. 真实发现：依赖口径不一致 + 一个设计异味

真实扫描首次运行时报 `CB-04 FAIL: agent -> knowledge_graph -> agent`。

追查后确认：这是**必须/可选依赖未分层**导致的口径冲突——
Registry 的 C4 契约明确「可选依赖不参与成环阻断」（且有专门的 false-positive 夹具），
而边界 checker 初版把所有依赖一视同仁。

**处理方式（不是放宽规则）**：让两个 checker 共享同一个依赖真源
（`loadRegistry()` 统一解析 `capabilities.yaml`），并分层判定：

```text
必须依赖（dependsOn）成环        → CB-04 fail（真正阻断装配）
仅「可选依赖」构成的环           → CB-04 warn（设计异味，不阻断）
```

这样 `--strict` 仍会把异味拦下（当前 `--strict` = FAIL），普通模式放行但**记录为债务**。

---

## 6. 已登记债务

| ID | 内容 |
|---|---|
| Debt-8A-1 | `agent ↔ knowledge_graph` 双向可选依赖环（CB-04 warn；不阻断装配，属设计异味，待裁决是否改为单向） |
| Debt-8A-2 | 边界 checker 尚未接入 pre-merge（按 §5 要求：checker 自身未证明可信前不接入；现自检已 12/12，接入留待下一轮） |
| Debt-8A-3 | `src/capabilities/` 目录尚未创建 —— 本阶段只立规矩，未搬代码（下一 Phase 8B 开始） |

---

## 7. Independent Review（自审）

| 攻击点 | 结论 |
|---|---|
| 是否只是搬目录？ | 本阶段未搬目录，只建立可强制的规则机器 ✅ |
| 是否只是包装？ | 规则可被机器执行且有 negative 夹具证明能拦住违规 ✅ |
| 是否真的 C3？ | 尚未（无 capability 已物理迁移） → 诚实记为 C0/C1 ✅ |
| 是否新增第二 State truth？ | 无 ✅ |
| Runtime 是否变 God Object？ | 未改 Runtime ✅ |
| 是否为了通过测试降低约束？ | 否 —— 修的是自家 checker bug，并补强了漏检能力 ✅ |

---

## 8. 结论

```text
PHASE_8A_RESULT: PASS（物理边界规范 + 门禁就绪，自检 12/12，真实扫描 fail=0）
```

下一步：**Phase 8B — Bookmark 真正物理迁移（目标 ≥ C3）**。
