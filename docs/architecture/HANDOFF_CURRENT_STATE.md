# HANDOFF — CURRENT PROJECT STATE

> 下一 Agent 无需重扫全仓即可继续。本文件为**当前真实状态快照**，事实来自 `git` 与已落地文件。
> 最后更新：2026-09-19　HEAD `2b030eb`
>
> ⚠️ **本文件经历了一次 git 对象损坏恢复**：上一版 HANDOFF 提交 `2ff24aa` 的对象损坏/丢失，
> 仓库已恢复至其完好父 `2b030eb`（registry + checker + pre-merge 接入全部完好）。
> `2ff24aa` 仅含本 HANDOFF 文档，内容已据真实状态重建。损坏的孤立 loose 对象无害，未删除。

---

## 1. Current Milestone

```text
Phase 0                     CLOSED   （Single Semantics Governance）
Phase 1 Browser/Grid        CLOSED   （tag: semantic-phase1-browser-grid-pass）
Phase 1.5 Semantic Registry  CLOSED   （Registry + Checker + SCR + 接入 pre-merge）
Phase 1.6 Semantic Reg. Acc  PASS    （验收闭环：registry/checker/pre-merge 均通过）
```

Phase 1.6 验收结论：

```text
REGISTRY: PASS   （4 件套存在，checker 可加载，真实仓库扫描 fail=0）
CHECKER:  PASS   （self-test ALL_PASS；R1-R5 negative 全检出；退出码 PASS=0/FAIL=1）
GATE:     PASS   （已接入 pre-merge；self-test 中 semantic 项无新增失败）
KNOWN_DEBT: 隔离（4 项 debt 未被触碰）
```

---

## 2. Git State

```text
branch:    master
HEAD:      2b030eb35c809dc44cd4887f42fe76bf08f3ba7b
working tree: 干净（仅未跟踪 .snapshots/ 与 diagnostics/ —— 取证产物，不入库）
```

### latest commits

```text
2b030eb  gates(phase1.5): wire semantic registry gate into pre-merge
3874bef  feat(phase1.5): semantic registry + semantic gate (governance infra only)
2ca7fb3  docs(phase1): closeout - final report + known debt registry
7e8d867  docs(phase1): mark R1-R6 passed by human GUI acceptance; record 2nd R7 check
4a5ec55  docs(phase1): record runtime GUI acceptance (stage 1) incl. R7 shutdown verification
88f8987  ops(recovery): add recovery and operational safety layer
ad587bc  test(phase1): add runtime acceptance harness for browser/grid semantics
a30fd57  feat(phase1): converge browser grid intent and lifecycle semantics
```

### latest tags（及分类）

```text
semantic-phase0-infra-pass        -> 481fbf7   [governance milestone]  Phase 0 基础设施通过
semantic-phase0-policy-pass        -> 04e4cbc   [governance milestone]  Phase 0 策略通过
semantic-phase1-browser-grid-code-pass -> a30fd57 [code milestone]      Phase 1 代码+静态门禁通过
semantic-phase1-browser-grid-pass       -> 7e8d867 [architecture milestone] Phase 1 完整验收（含 GUI）

semantic-registry-v1:  NOT EXISTS（属原 Phase 1.6 可选动作，本次未创建；如需创建见 §7）
```

---

## 3. Frozen Architecture Rules（不可违反）

```text
mainView
  = 用户当前希望看到的 Main Surface
  owner = useLayoutStore（setView 是唯一底层写入口）

gridOpen
  = Grid resource 是否存在
  owner = useBrowserStore

desiredGridVisibility
  = gridOpen && mainView === "grid"（纯派生，禁止存储）

isBrowserVisible
  = mainView === "browser"（不得重新耦合 gridOpen）

禁止:
  gridVisible stored state         （第二真源，C4/C5 常驻守护）
  exitGrid(mode)                   （多义万能 API，已 ADR 否决，出现即阻断）
  GridLifecycle enum / 新 Rust show_hide / 拆 position↔show
  （保留 position → show 既有 Native 契约）

Owner 收敛:
  View Navigation   = useLayoutStore
  Browser/Grid Life = useBrowserStore
  Native Execution  = useBrowserHost / bridge / Rust（只执行，不得成 domain owner）
```

---

## 4. Semantic Registry 入口

下一 Agent 修改**任何**状态 / Intent / API / Owner / Side Effect 前，**必须先查 Registry**：

```text
docs/architecture/semantic-registry/
  states.yaml       # 6 受治理状态 + observed_not_governed（显式登记未治理状态）
  intents.yaml      # 10 intent + rejected(exitGrid) + proposed_not_present
  owners.yaml       # 4 owner；owner-only 内部原语 vs public intent 区分
  side-effects.yaml # 6 副作用；sync_browser_scene 标 NOT_FOUND
  README.md         # 分类：CURRENT FACT / TARGET CONTRACT / ACCEPTED ADR
                     #      / PROPOSED CHANGE / KNOWN DEBT；禁止 Proposal 伪装 Current
```

配套：

```text
scripts/check-semantic-registry.mjs   # R1-R5，--self-test / --json / --strict
scripts/pre-merge.sh                  # 已接入（正式门禁 + self-test 两段）
docs/architecture/semantic-changes/SCR-template.md  # 新增语义必须走 SCR
```

流程：`查 Registry → 已有则用之 → 无则填 SCR → Review → ADR → 更新 Registry → 才许写代码`

---

## 5. Current Task Status

```text
Completed:
  Phase 0 / Phase 1 / Phase 1.5 / Phase 1.6 全部完成

Pending:
  无（Phase 1.6 验收已 PASS；如需固化可选 semantic-registry-v1 tag）

Blocked:
  无

Next recommended task:
  Phase 2 设计任务（需用户明确授权；当前禁止自行进入 Phase 2）
```

---

## 6. Known Debt

```text
Debt-001  Grid UDS socket cleanup
  状态:    KNOWN DEBT
  来源:    sock/ 累计残留（第二次观测 138 个），最早 2026-08-25
  当前 Phase: 不处理
  禁止误处理: 清理会碰用户目录（~/.local/share/com.jizhijiandan.mvp/sock/），
            非业务代码范围；属文件级资源泄漏，当前无功能影响

Debt-002  toggleGridToolbar dead code
  状态:    KNOWN DEBT
  来源:    useLayoutStore 无调用方；内部 layout 反向 import useBrowserStore 有依赖方向风险
  当前 Phase: 不处理
  禁止误处理: 不要补 import / 删除 / 重构（零 caller，不触发；改动会引入循环依赖风险）

Debt-003  closeGridCell orphan API
  状态:    KNOWN DEBT
  来源:    API 存在，当前无 UI 产品入口
  当前 Phase: 不处理
  禁止误处理: 不要删除（可能未来能力）；不要新增 caller 绕过 SCR

Debt-004  Terminal checker debt
  状态:    KNOWN DEBT
  来源:    check-terminal-policy.py / check-terminal-ui-logic.mjs 自检失败
  当前 Phase: 不处理（Phase 外，与 Browser/Grid 无关）
  禁止误处理: 禁止修改 Terminal；该失败是 pre-merge --self-test 的既有 FAIL，非本次引入
```

---

## 7. Next Agent Instructions

**开始前先读取（不要重扫全仓）：**

```text
docs/architecture/HANDOFF_CURRENT_STATE.md
docs/architecture/semantic-registry/README.md
docs/architecture/semantic-registry/states.yaml
docs/architecture/semantic-registry/intents.yaml
docs/architecture/semantic-registry/owners.yaml
docs/architecture/semantic-registry/side-effects.yaml
docs/architecture/semantic-governance/phase1-browser-grid/FINAL-REPORT.md   （架构冻结真源）
docs/architecture/semantic-governance/Known-Debt.md                          （债务清单）
```

**禁止：**

```text
禁止重新扫描全仓历史上下文（以上文档已固化当前状态）
禁止修改业务代码（src/ src-tauri/）
禁止清理 Known Debt（Debt-001~004）
禁止扩大 Semantic Registry 范围（先走 SCR；治理域外不判失败）
禁止进入 Phase 2（除非用户明确授权）
禁止削弱任何 Checker（尤其 check-semantic-registry 的 false-positive fixture）
```

**可选（如需固化 Phase 1.6）：**

```bash
git tag -a semantic-registry-v1 -m "Semantic Registry + Checker + pre-merge gate accepted; Phase 1.6 acceptance PASS"
# 本地，不推送
```

**快速自查命令：**

```bash
node scripts/check-semantic-registry.mjs --self-test   # 应 ALL_PASS
node scripts/check-semantic-registry.mjs              # 应 fail=0
bash scripts/pre-merge.sh --self-test                 # semantic 项应无新增失败
```

---

## 8. 当前验证状态（Phase 1.6 验收证据）

```text
REGISTRY consistency:  PASS  （4 件套存在且格式有效；checker 加载成功；真实扫描 fail=0 warn=6 info=36）
CHECKER self-test:     PASS  （SELF_TEST_RESULT=ALL_PASS；positive 0 / negative R1-R5 全检出 / false-positive 0）
CHECKER exit code:     PASS  （PASS 路径 exit 0；negative 未检出则 exit 1 —— self-test 通过即证明 FAIL 路径有效）
GATE integration:      PASS  （pre-merge.sh 已含 check-semantic-registry 两段；--self-test 中 semantic 项无失败）
KNOWN_DEBT isolated:   PASS  （4 项债务未被触碰；HEAD 2b030eb 以来无业务代码改动）
```
