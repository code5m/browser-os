# 06 · 恢复能力（Recovery）

> 语义治理不仅"事前拦截"，也"事后可恢复"。改坏时有标准流程回退，禁止盲目 `git reset`。

## 1. 四类能力

| 能力 | 证据 | 说明 |
|-|-|-|
| Snapshot | `scripts/snapshot.sh` + `.snapshots/` | 关键操作前留快照（pre-commit-recovery-layer / pre-recovery-layer-commit）|
| Diagnostics | `.diagnostics/<date>/` | 每次 Checker/迁移运行留痕，可审计 |
| Rollback 文档 | `phase1.7-git-integrity/RECOVERY-PROCEDURE.md` | 标准恢复流程 |
| Git Integrity | `scripts/check-git-repo-integrity.sh` + `git fsck` | 仓库对象损坏检测 + 清理 |

## 2. 恢复路径（标准流程）

```text
触发：check-git-repo-integrity.sh 报 FAIL / git fsck 报错
  1) 诊断（只读）：snapshot.sh before-recover + git-recover.sh --diagnose
        → 列出损坏对象 SHA + 可达性（是否被引用包含）
  2) 仅当"全部不可达"：git-recover.sh --prune-orphans
        → 护栏：只删松散对象 / 只删 cat-file 失败对象 / 不可达 / 删前先快照 / 删后复验
  3) 复验：check-git-repo-integrity.sh（PASS）+ pre-merge.sh --self-test
  4) 可选：git reflog expire + git gc --prune=now
```

## 3. 最近稳定 tag（恢复点）

| Tag | 含义 |
|-|-|
| `semantic-governance-v1` | 语义治理 v1 总验收基线（本次新增）|
| `semantic-phase6b-writer-enforcement-pass` | 语义治理最新阶段基线 |
| `baseline-m1-0-approved` | 用户认可的运行基线（master M1-0，稳定好用）|

回退用法：`git checkout <tag>` 或 `git revert <commit>`；健康副本 `git clone` 亦可。

## 4. 已知限制（诚实登记）

- `--prune-orphans` **不处理** pack 文件损坏、被引用对象损坏（说明某 commit/tree/blob 真实丢失）。
- 上述情况 Checker 会**主动中止**，交人工 / `git clone` 健康副本 / 备份 ref（`refs/remotes/backup/master`）取回。
- 真实演练：2026-09-19 上一轮 HANDOFF 引入的 5 个损坏对象（均不可达孤儿）已被安全清理，`git fsck` 无 error。

> 恢复纪律核心：**诊断 → 记录 → 恢复点 → 继续**，绝不盲目 reset 丢失历史。
