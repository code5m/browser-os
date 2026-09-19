# HANDOFF — CURRENT PROJECT STATE

> 下一 Agent 无需重扫全仓即可继续。本文件为**当前真实状态快照**，事实来自 `git` 与已落地文件。
> 最后更新：2026-09-19
>
> ✅ **git 对象损坏已清理**：原 HANDOFF 提交 `2ff24aa` 链 5 个损坏松散对象（均为 0 字节），
> 经验证「不被任何引用（含 reflog）包含」后，经 `scripts/git-recover.sh --prune-orphans` 安全删除。
> `git fsck --full` 现已 **0 error**；`check-git-repo-integrity.sh` 真实仓库 PASS。
> 恢复前完好基点为 `2b030eb`（registry + checker + pre-merge 接入完好）。

---

## 1. Current Milestone

```text
Phase 0                     CLOSED   （Single Semantics Governance）
Phase 1 Browser/Grid        CLOSED   （tag: semantic-phase1-browser-grid-pass）
Phase 1.5 Semantic Registry  CLOSED   （Registry + Checker + SCR + 接入 pre-merge）
Phase 1.6 Semantic Reg. Acc  PASS    （验收闭环：registry/checker/pre-merge 均通过）
Phase 1.7 Git Integrity      CLOSED   （tag: semantic-phase1.7-git-integrity-pass）
```

Phase 1.7 验收结论：

```text
GIT_INTEGRITY CHECKER: PASS  （默认模式 fsck 解析；^error: 即 FAIL；悬空 commit 仅 WARN）
CHECKER self-test:     PASS  （/tmp 夹具：注入损坏检出 + 干净通过，ALL_PASS）
GATE integration:      PASS  （接入 pre-merge.sh 两段；--self-test 中 git-integrity 项绿）
REAL RECOVERY:         PASS  （清理 5 孤儿损坏对象后 fsck 0 error；真实仓库 checker PASS）
GUARD:                 PASS  （被引用损坏对象判定可达 → 恢复脚本中止，不清删）
NO REGRESSION:         PASS  （仅既有 terminal 债 FAIL，非本 Phase 引入）
```

---

## 2. Git State

```text
branch:    master（Phase 1.7 以 feature/phase1.7-git-integrity 实现，ff-merge 入 master）
HEAD:      semantic-phase1.7-git-integrity-pass（annotated tag = master tip）
working tree: 干净（仅未跟踪 .snapshots/ 与 diagnostics/ —— 取证产物，不入库）
```

### latest commits（Phase 1.7 在其上）

```text
<phase-tip>  docs(phase1.7): closeout + handoff update
<phase-feat> feat(phase1.7): git integrity gate + recovery procedure
c209325      docs(handoff): rebuild current-state handoff; mark Phase 1.6 acceptance PASS
2b030eb      gates(phase1.5): wire semantic registry gate into pre-merge
3874bef      feat(phase1.5): semantic registry + semantic gate (governance infra only)
2ca7fb3      docs(phase1): closeout - final report + known debt registry
7e8d867      docs(phase1): mark R1-R6 passed by human GUI acceptance; record 2nd R7 check
4a5ec55      docs(phase1): record runtime GUI acceptance (stage 1) incl. R7 shutdown verification
88f8987      ops(recovery): add recovery and operational safety layer
ad587bc      test(phase1): add runtime acceptance harness for browser/grid semantics
a30fd57      feat(phase1): converge browser grid intent and lifecycle semantics
```

### latest tags（及分类）

```text
semantic-phase0-infra-pass        -> 481fbf7   [governance milestone]  Phase 0 基础设施通过
semantic-phase0-policy-pass        -> 04e4cbc   [governance milestone]  Phase 0 策略通过
semantic-phase1-browser-grid-code-pass -> a30fd57 [code milestone]      Phase 1 代码+静态门禁通过
semantic-phase1-browser-grid-pass       -> 7e8d867 [architecture milestone] Phase 1 完整验收（含 GUI）
semantic-phase1.7-git-integrity-pass    -> <phase-tip> [infra/governance milestone] Phase 1.7 仓库完整性+恢复

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

Phase 1.7 新增（仓库完整性，ops 层，不进产品 Semantic Registry）:
  git 损坏对象 = 0 字节松散对象 / cat-file 失败的松散对象
  恢复 = 仅删「损坏且不可达」的松散对象；禁止 blind reset / 禁止删被引用对象
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

## 4b. Git Integrity / Recovery 入口（Phase 1.7 新增，ops 层）

下一 Agent 在**每个 Phase 开始前**与**怀疑仓库损坏时**：

```text
scripts/snapshot.sh                  # 只读快照 HEAD/branch/tags/dirty → .snapshots/
scripts/check-git-repo-integrity.sh  # 只读门禁：git fsck --full 解析；^error:→FAIL
scripts/git-recover.sh               # --diagnose / --prune-orphans（受控清理孤儿损坏）
scripts/pre-merge.sh                 # 已接入 check-git-repo-integrity（正式 + self-test）
docs/architecture/semantic-governance/phase1.7-git-integrity/Phase1.7-design.md
docs/architecture/semantic-governance/phase1.7-git-integrity/RECOVERY-PROCEDURE.md
```

流程：`snapshot（Phase 始） → pre-merge 门禁（提交前） → 失败则 git-recover --diagnose → --prune-orphans → 复验`

---

## 5. Current Task Status

```text
Completed:
  Phase 0 / 1 / 1.5 / 1.6 / 1.7 全部完成

Pending:
  无（Phase 1.7 验收 PASS；semantic-registry-v1 tag 仍可选，未创建）

Blocked:
  无

Next recommended task:
  Phase 2 — Workspace / FilePanel Semantic Governance（tag: semantic-phase2-workspace-pass）
  注意：Phase 2 须在独立 feature/phase2-workspace 分支，独立提交/打 tag/更新 Handoff
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
  禁止误处理: 禁止修改 Terminal；该失败是 pre-merge --self-test 的既有 FAIL，非本 Phase 引入

Debt-1.7-1  pack 内损坏对象无自动恢复
  状态:    KNOWN DEBT
  来源:    git-recover.sh 仅处理松散对象（loose）；pack 损坏需 git unpack-objects / 克隆重建
  当前 Phase: 记录（超出 Phase 1.7 范围，交专项）
  禁止误处理: 不要盲目 git gc 掩盖 pack 损坏；pack 损坏走克隆重建或 unpack-objects

Debt-1.7-2  无周期性后台完整性巡检
  状态:    KNOWN DEBT
  来源:    Phase 1.7 仅在 pre-merge 触发；CI/定时巡检不在范围
  当前 Phase: 记录（交 ops）
  禁止误处理: 不要为每次 commit 自动 gc（会掩盖而非暴露问题）
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
docs/architecture/semantic-governance/phase1.7-git-integrity/Phase1.7-design.md
docs/architecture/semantic-governance/phase1.7-git-integrity/RECOVERY-PROCEDURE.md
```

**禁止：**

```text
禁止重新扫描全仓历史上下文（以上文档已固化当前状态）
禁止修改业务代码（src/ src-tauri/）除非对应 Phase 明确要求
禁止清理 Known Debt（Debt-001~004 / Debt-1.7-1~2）
禁止扩大 Semantic Registry 范围（先走 SCR；治理域外不判失败）
禁止削弱任何 Checker（尤其 check-semantic-registry 的 false-positive fixture、check-git-repo-integrity 的 ^error: 判定）
禁止 blind git reset / 删被引用对象（恢复只走 git-recover.sh 护栏）
```

**可选（如需固化 Phase 1.6 / 1.7 为 release tag）：**

```bash
git tag -a semantic-registry-v1 -m "Semantic Registry + Checker + pre-merge gate accepted; Phase 1.6 acceptance PASS"
# 本地，不推送
```

**快速自查命令：**

```bash
node scripts/check-semantic-registry.mjs --self-test   # 应 ALL_PASS
node scripts/check-semantic-registry.mjs              # 应 fail=0
bash scripts/check-git-repo-integrity.sh --self-test  # 应 SELF_TEST: PASS
bash scripts/check-git-repo-integrity.sh              # 真实仓库应 GIT_INTEGRITY: PASS
bash scripts/pre-merge.sh --self-test                 # semantic + git-integrity 项应绿（terminal 债为既有 FAIL）
```

---

## 8. 当前验证状态（Phase 1.7 验收证据）

```text
GIT_INTEGRITY CHECKER:
  PASS  （默认模式解析 git fsck --full；corrupt/missing → FAIL；悬空 commit → WARN 非阻断）
CHECKER self-test:
  PASS  （/tmp 临时仓库：干净仓库→PASS；注入 0 字节损坏对象→FAIL；ALL_PASS；exit 0）
GATE integration:
  PASS  （pre-merge.sh run_pre_merge + run_self_test 均接入；--self-test 中 git-repo-integrity 项绿）
REAL RECOVERY:
  PASS  （git-recover.sh --prune-orphans 清理 5 个孤儿损坏对象；fsck 0 error；真实仓库 checker PASS）
GUARD (AC-6):
  PASS  （被引用损坏对象经 git rev-list --all --reflog 判定可达 → 恢复脚本中止不清删）
NO REGRESSION:
  PASS  （pre-merge --self-test 仅既有 terminal 债 FAIL，非本 Phase 引入；未改任何既有 checker）
KNOWN_DEBT isolated:
  PASS  （Debt-001~004 / Debt-1.7-1~2 均显式记录，未被触碰或隐藏）
```
