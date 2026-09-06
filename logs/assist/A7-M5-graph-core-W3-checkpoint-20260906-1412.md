# A7 · M5-W3 Checkpoint（图谱实现就绪卡交付 + 自测 + 阻塞项）

> LANE=A7　WAVE=M5-W3（SUPPORT DOCS ONLY）　STATUS=PASS_WITH_DOCS
> BASE=`98a3b01`（已 `git fetch origin && git pull --ff-only` 对齐 origin/master）
> 衔接：`A7-M5-graph-core-20260906-0755.md`（W0）、`A7-M5-graph-core-W1-delta-20260906-0827.md`（W1）、`A7-M5-graph-core-W1-checkpoint-20260906-1338.md`（W1 检查点）、`A7-M5-graph-core-W3-impl-card-20260906-1412.md`（本波实现就绪卡）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W3（行 133-168）

---

## 0. 检查点结论（给 A0 / A17 / 后续 Lane）

1. **A7 在 M5-W3 的整包交付已完成且经自测验证**：把 A1 的 M5-7/M5-8 预研卡升级为**实现就绪规范**（精确 DTO/schema + `core/` 纯函数拆分 + M5-W2 常量对齐 + `AGRAPH_1` 政策脚本规范 + 精确 shutdown 序 + 阻塞项状态），落在 A7 自有 `logs/assist/A7-M5-graph-core-W3-impl-card-20260906-1412.md`。
2. **A7 严守 W3 硬停**：仅 A2/A3 可动产品代码；A7 未写任何图谱产品代码（domain.rs 无 Graph*、bridge.rs 0 条 graph_ 命令、无 graph*.rs、无 check-graph-policy.py）。工作树无他 lane 冲突。
3. **唯一外部前驱阻塞 = B1（M5-1.b）**：A2 的 seam trait（PathResolver/RootsProvider/ProgressSink）+ B 类搬入在 W3 为 `START`、尚未 PASS（M5-1.b §9 行 237 已明示"M5-7/8 必须 M5-1.b PASS 后才能做"）。图谱路径解析/seam 注入依赖此项。
4. **A17 实施期自闭环项**：B2（domain 类型）、B3（check-graph-policy.py）、B4（ACL/bridge/types）、B5（shutdown 序单测）——均非前驱阻塞，实施卡已给出精确规范。

---

## 1. 工作树 / pull 匹配确认

- `git fetch origin && git pull --ff-only` → `PULL_RC=0`，本地 `master` = `98a3b01` = origin/master（分支状态 `## master...origin/master` 无 ahead/behind）。
- `pwd` = WORKDIR（匹配）；分支 = `master`（匹配）。
- `git status`：仅 A7 自有 `logs/assist/A7-M5-graph-core-*.md` 未跟踪（正常交付形态）；无他 lane 游离产品代码冲突。
- A7 未提交、未 push（符合 Merge Rule：仅 A0 可 push）。

---

## 2. 自测证据（Self-Test，实测 @ 98a3b01）

| # | 验证项 | 命令/来源 | 结果 |
|---|---|---|---|
| T1 | core 边界门自测 | `python3 scripts/check-core-boundary.py --self-test` | PASS（ACTIVE=7，坏样本全检） |
| T2 | core 边界默认 PASS | `python3 scripts/check-core-boundary.py` | PASS（core 文件=2） |
| T3 | 图谱类型未落 domain.rs | `grep -niE "graph\|GraphNode\|GraphEdge" src-tauri/src/domain.rs` | 0 命中（待 A17） |
| T4 | 图谱命令未落 bridge.rs | `grep -c "graph_" src-tauri/src/bridge.rs` | 0 |
| T5 | 无图谱源文件 | `ls src-tauri/src/graph*.rs` | 无（W3 docs-only 正确） |
| T6 | M5-W2 常量已集中 | `grep` `MAX_TIMEOUT_SECS`/`HARD_GRACE_SECS`/`MAX_TEXT_FIELD_BYTES` `domain.rs` | 命中 1015/1017/1019 |
| T7 | M5-7/M5-8 卡存在 | `ls logs/checkpoints/M5-20260906/M5-7*.md M5-8*.md` | 存在（A1 W0） |
| T8 | `git diff --check` | `git diff --check` | `DIFFCHECK_OK` |

---

## 3. 解锁状态（Unblock for M5-7/8 实施）

| 依赖（本卡 §6） | 状态 | 责任 | 证据 |
|---|---|---|---|
| M5-W1 core 边界门 | ✅ 已 PASS | A2 | 854bc40；T1/T2 |
| M5-W2 常量集中 | ✅ 已落 | A2 | T6 |
| `database.rs` 单连接复用 | ✅ 可用（非阻塞） | A3/A4 | 48KB；暴露 open/connect/query |
| M5-1.b seam + B 类搬入 | ⏳ W3 进行中，**未 PASS** | A2 + A10 | `core/seam.rs` 未落地；B1 |
| AGRAPH_1 政策脚本 | ⏳ 待 A17 建 | A17 | §3 规范已给 |
| ACL/bridge/types 镜像 | ⏳ 待 A17 | A17 | §5 规范已给 |

---

## 4. 下一张"最小可集成切片"的解锁条件（A0 裁决）

用户要求"按 M5-W3 完成整包交付"——A7 的 W3 整包（docs 实现就绪卡）已完成。图谱**产品代码**切片（M5-7/8）的推进需：

- **A0 在 M5-1.b PASS 后**，发 M5-7/8 实施 dispatch 给责任 Lane **A17**（M5-7/8 卡指定），并将本实现就绪卡作为实施输入；
- 实施卡已固化：DTO/schema（§2）、core 纯函数拆分（§2.2，满足 `COREBOUND_*`）、`graph_store` 复用 `database.rs` 单连接（§2.4）、`AGRAPH_1` 八码（§3）、精确 shutdown 序（§4）、9 条命令（§5）、阻塞项（§6）。
- A7 在 M5-1.b PASS 前**不写图谱产品代码**（W3 硬停 + B1 前驱阻塞）。

---

## 5. 输出模板回填

```text
LANE=A7
STATUS=PASS_WITH_DOCS
WAVE=M5-W3 (SUPPORT DOCS ONLY)
BASE=98a3b01
HEAD=docs only（A7-M5-graph-core-W3-impl-card-20260906-1412.md + A7-M5-graph-core-W3-checkpoint-20260906-1412.md）
FILES=logs/assist/A7-M5-graph-core-W3-impl-card-20260906-1412.md, logs/assist/A7-M5-graph-core-W3-checkpoint-20260906-1412.md
VERIFY=T1-T8 全 PASS/命中；无产品代码；git diff --check 干净；工作树无他 lane 冲突
CHECKPOINT=logs/assist/A7-M5-graph-core-W3-checkpoint-20260906-1412.md
MERGE_NOTES=W3 docs-only；未改 M5-7/8 卡（避免与 A1 冲突），本卡为 A7 自有实现就绪规范；唯一前驱阻塞 B1=M5-1.b（A2 W3 进行中）；
           与 A2 M5-1.b §9 行237、A3 check-mcp-policy.py 范式、M5-8 §4.5、A7 M4-7 冻结（stop-scheduler 序）一致；
           他 lane W3 产物（A2 core 提取 / A3 MCP）与 A7 文档无文件交集
NEXT=A0 待 M5-1.b PASS 后发 M5-7/8 实施 dispatch（A17）；本卡作实现就绪输入；若 A0 改派 A7 实施图谱，须先解除 W3 硬停（仅 A2/A3 可动产品代码）
```
