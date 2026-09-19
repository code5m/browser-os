# Git Integrity Recovery Procedure（Phase 1.7）

> 标准化仓库对象损坏恢复流程。原则：**诊断 → 记录 → 恢复点 → 继续**；禁止盲目 reset。

---

## 0. 触发信号

`scripts/check-git-repo-integrity.sh` 报 FAIL，或人工发现 `git fsck --full` 报：

```text
error: 对象文件 .git/objects/XX/YYYY 为空
error: <oid>：对象损坏或丢失：...
error: <oid> does not point to a valid object!
```

---

## 1. 诊断（只读）

```bash
# 1) 记录当前状态（Recovery §9 snapshot）
bash scripts/snapshot.sh before-recover

# 2) 列出损坏对象与可达性
bash scripts/git-recover.sh --diagnose
```

`--diagnose` 会输出：
- 每个损坏松散对象的 SHA
- 它是否被 `git rev-list --all --reflog --objects` 包含（可达性）
- 结论：若全部「不可达」→ 可安全清理；若任一「可达」→ 停止，需人工/克隆恢复

---

## 2. 安全清理孤儿损坏对象（写 .git）

仅当诊断结论为「全部不可达」时执行：

```bash
bash scripts/git-recover.sh --prune-orphans
```

`--prune-orphans` 的护栏（任一不满足则中止，绝不删除）：
1. 只处理**松散对象**（`.git/objects/xx/yyy`），不碰 pack。
2. 只对 `git cat-file -t` 失败（损坏/为空）的对象操作。
3. 对象必须**不在** `git rev-list --all --reflog --objects` 中（不可达）。
4. 删除前先快照；删除后复验 `git fsck --full` 无 error。

> 0 字节的松散对象结构上必坏（合法对象从不为 0 字节），且不可达 → 删除无数据损失。

---

## 3. 复验

```bash
bash scripts/check-git-repo-integrity.sh   # 期望 PASS (exit 0)
bash scripts/pre-merge.sh --self-test      # 期望包含 git-integrity 项 ALL_PASS
```

---

## 4. 可选压缩

清理后建议释放引用历史并压缩：

```bash
git reflog expire --expire=now --all
git gc --prune=now
```

---

## 5. 不可自动处理的情况（交人工 / 克隆重建）

- 被引用对象损坏（`git rev-list --all` 命中）：说明某 commit/tree/blob 真实丢失 →
  `--prune-orphans` 会**主动中止**。恢复手段：`git clone` 健康副本、`git unpack-objects`、
  或从备份 ref（`refs/remotes/backup/master`）取回。
- pack 文件损坏：需 `git unpack-objects < pack` 重建或重新克隆。
- 多引用同时损坏、无法判定 HEAD：先 `git for-each-ref` 人工确认完好引用，再定点恢复。

---

## 6. 本次真实演练（2026-09-19）

上一轮 HANDOFF 提交 `2ff24aa` 及其引入的 5 个对象（含 HANDOFF 内容 blob `4511fed02`）
损坏/丢失（均为 0 字节）。已确认：
- `refs/heads/master` = `c209325`（完好父 `2b030eb` 之上重建的 HANDOFF）
- 5 个损坏对象**不被任何引用（含 reflog）包含** → 孤儿
- 经 `git-recover.sh --prune-orphans` 清理后 `git fsck --full` 无 error
- `check-git-repo-integrity.sh` 真实仓库 PASS

恢复点：`2b030eb`（registry + checker + pre-merge 接入完好）是上一完好状态。
