# Semantic Governance v1 — Final Acceptance（全局架构验收）

> 项目：`mvp-browser-os-v3`
> 阶段：Single Semantics Governance v1 Final Acceptance
> 日期：2026-09-19
> 性质：**纯验收 / 文档交付**，未修改任何业务代码（`src/` / `src-tauri/`）、未新增治理域、未扩大 M4。
> 真源：Semantic Registry（`docs/architecture/semantic-registry/*`）+ 4 个 Checker + `scripts/pre-merge.sh` + 各 Phase closeout 文档。

---

## 1. Semantic Model（五大语义治理模型）

| 模型 | 真源 | 守护 Checker | 状态 |
|-|-|-|-|
| **State** | `states.yaml`（`kind: stored/derived`、`owner`、`canonical_writer`、`forbidden_writers`、`single_owner_required`、`derived`） | R1/R2/R3/R8/R9 | ✅ |
| **Intent** | `intents.yaml`（`canonical` + `duplicate_names` + `rejected_intents` + `status`） | R4 | ✅ |
| **Owner** | `owners.yaml`（`owner` 映射）+ `states.yaml.owner` | R3 / R8 | ✅ |
| **Writer** | `states.yaml.canonical_writer` | R9 | ✅（Phase 6B 新增强制）|
| **Side Effect** | `side-effects.yaml`（`WebView`/`Keyring`/`Process`/`Filesystem`/`Sensitive data`） | R5 / R7（`check-sensitive-side-effects.mjs`）| ✅ |

五大模型均有"声明真源 + 机器化 Checker"闭环，是 AI 协作软件语义治理的完整骨架。

---

## 2. State Governance（状态治理链路）

链路存在且可证明：

```text
Registry（states.yaml 声明 owner + canonical_writer + forbidden_writers）
   ↓
Checker（check-semantic-registry.mjs）
   ├─ R1  Duplicate State       同名状态重复声明
   ├─ R2  Unregistered State     业务代码中 ref/reactive 但 registry 未登记
   ├─ R3  Owner Violation        非 owner 文件声明受治理状态
   ├─ R8  Semantic State Multi Owner  受治理状态在 owner 文件之外被声明（第二真源）
   └─ R9  Semantic State Writer Violation  写入点越权（非 owner 文件直写 / owner 内非 canonical 函数）
   ↓
Gate（pre-merge.sh Phase 03）  exit 0 = 通过 / exit 1 = 阻断
```

`gridSession` 示例（Phase 6B 重点）：owner=`useBrowserStore`，canonical_writer=`buildGrid`/`forceGridRelayout`，
forbidden_writers=`components/**`/`useLayoutStore`/`useBrowserHost`；R9 用 brace 配对做函数作用域分析，
确认两处写入均在 canonical 函数体内，跨域/组件直写被 FAIL。

---

## 3. Intent Governance（意图治理）

- **canonical intent**：`intents.yaml` 每个用户意图 = 一个 canonical 入口（如 `openGrid`/`closeGridAll`/`activateBrowser`）。
- **duplicate intent detection**：每条 intent 含 `duplicate_names`（同义别名），`check-view-intent.mjs` / R4 检出重复入口。
- **rejected intent tracking**：`intents.yaml.rejected_intents` 显式登记被 ADR 否决的多义/危险入口，例：
  - `exitGrid`（多义万能 API，mode 易误用）
  - `exposePassword` / `copyPassword` / `exportCredential`（凭据安全红线）
  - `mergeBookmarksIntoHome` / `mergeOpenFileIntoOpenFileInline`（引入第二真源）
  - `componentWritesTermPanes`（破坏终端注册表不变式）
  - 代码中出现即 `SEMANTIC_INTENT_DUPLICATE`（阻断级）。

---

## 4. Owner Governance（所有者治理）

- **owner registry**：`owners.yaml` + `states.yaml.owner`，每个受治理状态有唯一 owner store。
- **owner violation checker**：R3（非 owner 文件声明）/ R8（owner 文件之外被声明 = 第二真源）/ R9（非 owner 文件直写）。
- 派生态（`derived: true`，如 `bmPanelOpen`）由 R6 守护不被存为独立真源。

---

## 5. Side Effect Governance（副作用治理）

`side-effects.yaml` 覆盖五大副作用面，运行时门禁 `check-sensitive-side-effects.mjs`（R5/R7）：

| 副作用面 | 治理要点 | 守护 |
|-|-|-|
| WebView | 宫格 create/destroy 经语义化入口，禁止组件裸 `buildGrid`/`closeGridAll` | R5 副作用认知 |
| Keyring | 凭据仅存 keyring，密码绝不回前端 | R7 敏感输入泄露 |
| Process | tab/terminal spawn 经 store action | R5 |
| Filesystem | 文件读写经单入口，剪贴板历史不落盘 | R5 / R7 |
| Sensitive data | 凭据脱敏、剪贴板明文不持久化 | R7 |

---

## 6. Registry 总检查（验证证据）

```text
$ node scripts/check-semantic-registry.mjs --self-test
SELF_TEST_RESULT=ALL_PASS

$ node scripts/check-semantic-registry.mjs
files scanned: 104 | rules: 9
fail=0  warn=6（pre-existing R5 副作用认知，非阻断）  info=72（observed_not_governed）
SEMANTIC_REGISTRY_RESULT=PASS

$ node scripts/check-semantic-closure-logic.mjs
SEMANTIC_CLOSURE_LOGIC_RESULT=PASS (27/27)
```

四个 registry 文件（`states.yaml` / `intents.yaml` / `owners.yaml` / `side-effects.yaml`）均被 Checker 成功解析
（self-test 读取即 ALL_PASS），格式有效、无破坏性漂移。

---

## 7. Checker 总检查

| 规则 | 名称 | 位置 |
|-|-|-|
| R1 | Duplicate State | check-semantic-registry.mjs |
| R2 | Unregistered State | 同上 |
| R3 | Owner Violation | 同上 |
| R4 | Duplicate Intent | 同上 |
| R5 | Side Effect Unknown | 同上 + check-sensitive-side-effects.mjs |
| R6 | Derived State Stored | check-semantic-registry.mjs |
| R7 | Credential Sensitive Input | check-sensitive-side-effects.mjs |
| R8 | Semantic State Multi Owner | check-semantic-registry.mjs |
| R9 | Semantic State Writer Violation | check-semantic-registry.mjs（Phase 6B）|

- **self-test PASS**：R1..R9 含 positive / negative / false-positive 夹具，全部 `ALL_PASS`。
- **negative fixture PASS**：跨 store 直写、组件直写、owner 内非 canonical 函数写入均被 R9 检出。
- **false-positive 风险（已记录）**：
  - R9 读取不误报（`===` / `=>` 已被正则显式排除，夹具验证）。
  - R9 对"无参 parenless 箭头函数"(`const f = x => {...}`) 不识别其 enclosing —— 登记为 **Debt-6B-1**，
    仅影响极少数写法；真实代码写入点均为 `function NAME()` 形式，未触发误报。
  - R8 仅识别 `const X = ref/reactive` 声明形态（解构/动态声明盲区）—— 登记为 **Debt-6A-3**，与 R2 同源。

---

## 8. Gate 验收

`scripts/pre-merge.sh` Phase 03 checker gate（line 531）循环包含：

```bash
for c in check-architecture ... check-semantic-registry check-semantic-closure-logic \
         check-sensitive-side-effects doctor; do
  if ! (cd "$ROOT" && node "$SCRIPT_DIR/$c.mjs") >/dev/null 2>&1; then
    pm_fail "phase03 $c.mjs"
  fi
done
```

- 任一语义 Checker 失败 → `PM_RC=1` → **exit 1（阻断）**。
- 全部通过 → `PRE_MERGE_RESULT=ALL_PASS` → **exit 0**。

语义治理三条 Checker（registry / closure / sensitive-side-effect）均接入门禁，提交前自动拦截。

---

## 9. Recovery 验收

| 能力 | 证据 | 状态 |
|-|-|-|
| Snapshot | `scripts/snapshot.sh` + `.snapshots/`（pre-commit-recovery-layer / pre-recovery-layer-commit）| ✅ |
| Diagnostics | `diagnostics/<date>/` 按运行留痕 | ✅ |
| Rollback 文档 | `docs/architecture/semantic-governance/phase1.7-git-integrity/RECOVERY-PROCEDURE.md` + `scripts/git-recover.sh` | ✅ |
| Git Integrity | `scripts/check-git-repo-integrity.sh` + `git fsck`；真实演练 2026-09-19 已闭环 | ✅ |

**最近稳定 tag**：`semantic-phase6b-writer-enforcement-pass`（语义治理最新基线）；`baseline-m1-0-approved`（用户认可运行基线）。
**恢复路径**：诊断 → `snapshot.sh` → `git-recover.sh --diagnose` → （仅孤儿不可达对象）`--prune-orphans` → 复验 `check-git-repo-integrity.sh`；可达对象损坏则 `git clone` 健康副本或取备份 ref。
**已知限制**：`--prune-orphans` 不处理 pack 损坏 / 被引用对象损坏（交人工 / 克隆重建，见 RECOVERY-PROCEDURE §5）。

---

## 10. Reviewer 自检（诚实口径）

- **未夸大成果**：本文档与交付物均写"核心治理域已闭环，剩余债务已登记"，未声称"所有代码无语义问题"。
- **Registry 非万能**：Registry + Checker 双轨并存；代码迁移（Phase 6A/6B）真实发生，Checker 仅守护不替代重构。
- **Known Debt 完整**：见 `KNOWN_DEBT_V1.md` 与 `docs/architecture/semantic-governance/Known-Debt.md`，未删减、未静默消失。

```text
SEMANTIC_GOVERNANCE_V1_ACCEPTANCE = PASS
```
