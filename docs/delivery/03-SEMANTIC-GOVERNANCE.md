# 03 · 语义治理四支柱（管理者视图）

## 支柱一：Registry（语义真源）

四个 YAML 文件构成语义契约的**单一真源**，由人/AI 评审维护，不藏在代码里：

| 文件 | 回答的问题 |
|-|-|
| `states.yaml` | 这个状态归谁（owner）？谁能写（canonical_writer）？谁禁止写（forbidden_writers）？是派生还是存储？|
| `intents.yaml` | 这个用户意图的 canonical 入口是什么？有哪些同义别名（重复入口）？哪些意图被否决（rejected）？|
| `owners.yaml` | 各 store 的 owner 边界 |
| `side-effects.yaml` | 哪些副作用面（WebView/Keyring/Process/Filesystem/Sensitive）受管控 |

## 支柱二：Checker（机器校验）

`scripts/` 下多个 Node/Python Checker 读取 Registry，在代码里静态/运行时校验：

- **R1–R9**（`check-semantic-registry.mjs`）：重复状态、未登记状态、owner 越界、重复意图、
  未知副作用、派生态误存、凭据敏感输入、多 owner、writer 越权。
- **closure-logic**（`check-semantic-closure-logic.mjs`）：运行时加载真实 store 验证行为契约（27 断言）。
- **sensitive-side-effects**（`check-sensitive-side-effects.mjs`）：凭据/副作用面违规。

每个 Checker 自带 `--self-test` 夹具（positive / negative / false-positive），改动 Checker 不自嗨：
夹具不过 = 改动不被接受。

## 支柱三：Gate（提交门禁）

`scripts/pre-merge.sh` Phase 03 把上述语义 Checker 纳入提交前自动门禁：

```text
正常提交  →  全部 Checker 通过  →  PRE_MERGE_RESULT=ALL_PASS  →  exit 0
违规提交  →  任一 Checker 失败  →  pm_fail                  →  exit 1（阻断合并）
```

即：**语义契约被破坏的修改，在合并前就被自动拦下**，不会流入主干。

## 支柱四：Recovery（可恢复）

- **Snapshot**：`scripts/snapshot.sh` 在关键操作前留快照（`.snapshots/`）。
- **Diagnostics**：每次运行留诊断（`.diagnostics/<date>/`）。
- **Rollback**：`phase1.7-git-integrity/RECOVERY-PROCEDURE.md` + `git-recover.sh`，
  标准"诊断 → 快照 → 恢复点 → 继续"流程，禁止盲目 reset。
- **Git Integrity**：`check-git-repo-integrity.sh` + `git fsck`，真实演练已闭环（2026-09-19）。

## 四支柱如何协作

```text
开发者/Agent 修改代码
        │
        ▼
   Registry（约束真源）── 定义"什么算合规"
        │
        ▼
   Checker（依据 Registry 校验代码）── 回答"这次修改合规吗"
        │
        ▼
   Gate（提交前跑 Checker）── 不合规 = 阻断
        │
        ▼
   Recovery（万一改坏）── 快照/回滚/Git 恢复
```

> 这让 AI 协作开发从"改完靠人肉 code review 抓语义 bug"，变成"改完自动被审计 + 阻断 + 可恢复"。
