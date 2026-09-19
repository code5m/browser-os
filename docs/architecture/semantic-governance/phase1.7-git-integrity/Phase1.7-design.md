# Phase 1.7 — Git Integrity & Recovery Hardening

> 状态：DESIGN（实现见 `scripts/check-git-repo-integrity.sh` / `scripts/git-recover.sh`）
> 基线：继承 `semantic-phase1-browser-grid-pass` + Semantic Registry + Semantic Gate + Recovery Layer + Handoff System
> 触发：上一轮 HANDOFF 提交 `2ff24aa` 对象损坏/丢失，暴露「仓库对象完整性无自动门禁、无标准化恢复流程」缺口。

---

## 1. State Model（受治理状态）

```text
git_repo_integrity
  = 仓库对象数据库是否无损（无 corrupt / missing / 悬空但被引用的对象）

git_repo_dirty
  = 工作树是否有未提交改动（不归本 Phase 裁决，仅快照记录；由既有 check-git-write-policy 等管）

git_orphan_corrupt_objects
  = 存在未被任何引用包含、且本身损坏（0 字节 / 无法 cat-file）的松散对象
  = 本 Phase 新增的核心治理状态：可安全清理，但必须先经可达性判定
```

### 观测但不治理（observed_not_governed）

- 悬空 blob / tree（正常垃圾，git gc 处理，不报错）
- 悬空 commit（可能代表丢失工作；WARNING 而非 FAIL，交人工判断）

---

## 2. Intent Model（意图）

```text
intent: verify_git_integrity
  触发：pre-merge 门禁 / CI / 手动
  语义：扫描仓库对象库，报告 corrupt/missing，exit 0=PASS / 1=FAIL
  owner：scripts/check-git-repo-integrity.sh（只读，不写 git 状态）

intent: recover_git_integrity
  触发：verify 失败后人工资极
  语义：快照 → 诊断 → 安全清理孤儿损坏对象 → 复验
  owner：scripts/git-recover.sh（仅 --prune-orphans 才写 .git；默认只读）

intent: snapshot_project_state
  触发：每个 Phase 开始时（Recovery §9）
  语义：只读记录 HEAD/branch/tags/dirty，落 .snapshots/
  owner：scripts/snapshot.sh（已存在，复用，不重造）
```

### rejected（明确不做）

```text
rejected: git_auto_gc_on_every_commit
  原因：gc 在损坏对象存在时可能中断，且自动 gc 会掩盖而非暴露问题

rejected: blind_git_reset_to_recover
  原因：可能丢弃完好数据；Recovery 规则禁止盲目 reset
```

---

## 3. Owner Model（Owner）

```text
git integrity verify   = scripts/check-git-repo-integrity.sh（只读）
git integrity recover  = scripts/git-recover.sh（受控写 .git）
git snapshot           = scripts/snapshot.sh（只读，复用）
gate integration       = scripts/pre-merge.sh（run_pre_merge + run_self_test）

约束：
  - 上述脚本不得成为新的「仓库真源」；它们只观测/修复对象库
  - 不得引入第二个 git 状态写入口（如自建 ref 管理）
```

---

## 4. Lifecycle（生命周期）

```text
Phase 开始（每个 Phase，Recovery §9）
  └─ snapshot.sh 记录 HEAD/branch/tags/dirty

pre-merge 门禁（每次提交/合并前）
  └─ check-git-repo-integrity.sh（默认模式）
       ├─ PASS  → 继续
       └─ FAIL  → 阻断；产出 fsck 错误摘要

  修复路径（人工资极，不自动）：
  └─ git-recover.sh --diagnose     （列出损坏对象 + 可达性）
  └─ git-recover.sh --prune-orphans（仅删「损坏且不可达」的松散对象）
  └─ 复验 check-git-repo-integrity.sh → PASS
  └─ 可选 git reflog expire --expire=now --all && git gc --prune=now 压缩
```

---

## 5. Side Effect（副作用）

```text
check-git-repo-integrity.sh
  - 写：无（纯只读，git fsck 不修改对象库）
  - 网络：无
  - 凭据：无

git-recover.sh
  - --snapshot / --diagnose   ：只读
  - --prune-orphans           ：写 .git/objects/<xx>/<yyy> 删除文件；写 .snapshots/（经 snapshot.sh）
  - 禁止：reset --hard / branch -D / push --force / 修改任何被引用对象
```

---

## 6. Checker Plan（门禁设计）

### 6.1 `scripts/check-git-repo-integrity.sh`

- 默认模式：在仓库根运行 `git fsck --full`，解析输出：
  - 任何以 `error:` 开头且涉及 损坏/丢失/corrupt/missing/mmap/为空/does not point 的行 → **FATAL → exit 1**
  - `悬空 commit` / `dangling commit` → **WARNING**（非阻断，但报告，提示可能丢失工作）
  - `悬空 blob/tree` / `dangling blob/tree` → 忽略（正常）
- `--self-test`：在 `/tmp` 建临时仓库，注入 0 字节损坏对象验证 FAIL 路径，干净仓库验证 PASS 路径。不触碰真实仓库。
- `--repo <path>`：可选，指定被检仓库（默认仓库根）。
- 退出码：0 PASS / 1 FAIL / 2 USAGE。

### 6.2 接入 `pre-merge.sh`

- `run_pre_merge`：在 Phase 03 checker 循环中新增 `check-git-repo-integrity` 项（与 `check-semantic-registry` 同范式）。
- `run_self_test`：新增 `[ -f check-git-repo-integrity.sh ]` + `--self-test` 自检项。

---

## 7. Acceptance Matrix（验收矩阵）

```text
AC-1  check-git-repo-integrity.sh 默认模式在损坏仓库报 FAIL（exit 1）      [NEGATIVE FIXTURE]
AC-2  check-git-repo-integrity.sh 默认模式在干净仓库报 PASS（exit 0）      [POSITIVE]
AC-3  --self-test 在 /tmp 夹具下 ALL_PASS（注入损坏检出 + 干净通过）        [SELF-TEST]
AC-4  pre-merge.sh --self-test 含 git-integrity 项且 ALL_PASS              [GATE SELF-TEST]
AC-5  经 git-recover.sh --prune-orphans 清理当前 5 个孤儿损坏对象后，
      check-git-repo-integrity.sh 在真实仓库 PASS                          [REAL RECOVERY]
AC-6  git-recover.sh --prune-orphans 对「被引用损坏对象」安全中止（不删）   [GUARD]
AC-7  不降低任何既有 Checker；build / 其它门禁仍 PASS                       [NO REGRESSION]
AC-8  HANDOFF_CURRENT_STATE.md 更新 Phase 1.7 状态 + tag 分类               [HANDOFF]
```

---

## 8. 与既有基础设施的关系（避免重复语义）

```text
snapshot.sh         复用（只读快照），不重造
doctor.mjs          不修改；本 Phase 补齐其缺失的「对象完整性」维度
pre-merge.sh        新增 git-integrity 门禁项（不削弱既有）
check-git-write-policy.py / check-git-ui-*  是「Git 版本控制 UI 功能」门禁
                      （M5-W18 git 单元），与「仓库完整性」正交，命名区分：
                      本 Phase 用 check-git-REPO-integrity.sh（repo 对象库）
```

---

## 9. Known Debt（本 Phase 内）

```text
DEBT-1.7-1  包内损坏对象（pack corruption）无自动恢复
  当前 git-recover.sh 只处理松散对象（loose）；pack 损坏需 git unpack-objects / 克隆重建。
  处理：文档记录，本 Phase 不实现（超出范围，交专项）。

DEBT-1.7-2  无周期性后台完整性巡检
  当前仅 pre-merge 触发；CI/定时巡检不在本 Phase 范围（交 ops）。
```
