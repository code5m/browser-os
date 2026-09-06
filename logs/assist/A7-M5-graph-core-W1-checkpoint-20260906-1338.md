# A7 · M5-W1 检查点（Checkpoint · 图谱对齐 A2 core 边界 + 自测 + 解锁状态）

> LANE=A7　WAVE=M5-W1 Implementation Dispatch　STATUS=PASS_WITH_DOCS（SUPPORT DOCS ONLY，无产品代码）
> BASE=854bc40（A2 的 M5-1 core boundary gate 已落地并提交；本地 `master` 领先 `origin/master` 见下）
> 衔接：`A7-M5-graph-core-20260906-0755.md`（W0 契约）、`A7-M5-graph-core-W1-delta-20260906-0827.md`（W1 增量）
> 目标产物：`logs/assist/A7-M5-graph-core-W1-checkpoint-20260906-1338.md`（本文件）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W1（行 132-166）→ A7=SUPPORT DOCS ONLY；硬停行 160-166（仅 A2 可动产品代码，W1 禁 graph runtime/新命令/ACL）

---

## 0. 检查点结论（给 A0 / A17 / 后续 Lane）

1. **A7 在 M5-W1 的整包交付已完成且经自测验证**：W0 契约 + W1 增量（图谱纯函数落 `core/`、复用 `database.rs`、零第二执行路径、`AGRAPH_1` 硬门）已写入 `logs/assist/`；本文件记录自测证据与解锁状态。
2. **A2 的 M5-1 core 边界已落地并提交**（`854bc40 feat(M5): add core boundary gate`），本检查点实测验证该边界**可机检且当前 PASS** —— 即 W1 delta 的依赖闸门满足，未来图谱模块（由 A17 实施的 M5-7/8）放进 `core/` 即可通过 `check-core-boundary.py`。
3. **A7 在 M5-W1 无任何产品代码切片可做**：board 行 154 明确 A7=SUPPORT DOCS ONLY，硬停行 162-163 仅 A2 可动产品代码、W1 禁 graph runtime/新 Tauri 命令/ACL；图谱**实现责任**在 M5-7/8 卡中归 **A17**（非 A7）。故"下一张最小可集成切片"对 A7 而言就是本 docs 交付 + 本检查点，而非图谱产品代码。
4. **用户指令与 board 的偏差（须 A0 裁决）**：用户本轮要求"按 M5-W1/M5-W2 职责推进、做下一张最小可集成切片、自测+checkpoint"。但 board（08:35 最新版）**只有 M5-W1、无 M5-W2** 章节；且 M5-W1 硬停锁 A7 为 docs-only。为遵守协调协议与 WORKSPACE_IDENTITY 硬停（不得改出 lane scope 的文件），**A7 未写任何图谱产品代码**。解锁图谱实现切片的条件见 §4。

---

## 1. 工作树 / NEXT 匹配确认

- `pwd` = `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`（= WORKDIR，匹配）。
- 分支 = `master`（匹配；未切其他分支）。
- `git status --porcelain`：仅 1 个未跟踪文件 `logs/checkpoints/M5-20260906/M5-1.b-seam-trait-injection-and-b-extract.md`（属 A1，非 A7）；A2 的 core 边界改动已提交（非游离工作树）。**无他 lane 游离产品代码冲突** → 工作树干净（assist/checkpoint 文档类未跟踪属正常）。
- A7 交付物未跟踪（正常）：`logs/assist/A7-M5-graph-core-20260906-0755.md`、`A7-M5-graph-core-W1-delta-20260906-0827.md`、`A7-M5-graph-core-W1-checkpoint-20260906-1338.md`。
- NEXT 匹配：board 行 7 `Current NEXT: M5-W1 core boundary implementation; only Lane A2 may write product code, other lanes continue docs/review support` ↔ A7=SUPPORT DOCS ONLY（行 154）。**调度一致**。

---

## 2. 自测证据（Self-Test，实测）

| # | 验证项（对应 W1 delta 断言） | 命令/来源 | 结果 |
|---|---|---|---|
| T1 | core 边界门自测可跑 | `python3 scripts/check-core-boundary.py --self-test` | EXIT=0，`CORE_POLICY_SELF_TEST=PASS`（ACTIVE=7，坏样本=9，含注释/字符串阴性样本） |
| T2 | core 边界默认判定当前 PASS（无违规） | `python3 scripts/check-core-boundary.py` | EXIT=0，`core boundary policy: all invariants hold（ACTIVE=7，core 文件=2）` |
| T3 | `core/` 当前仅 `keyring_store.rs`+`mod.rs`（图谱尚未搬入，符合 W1 硬停） | `ls src-tauri/src/core/` | `keyring_store.rs` `mod.rs` |
| T4 | core 模块集为纯逻辑、零 Tauri 耦合，新增图谱模块按 `pub mod graph_*;` 同款挂载 | `cat src-tauri/src/core/mod.rs` | `pub mod keyring_store;`；注释明确切片 0a=A 类纯模块整文件搬入（图谱纯逻辑属此类） |
| T5 | `graph_store` 复用目标：`database.rs` 暴露 SQLite 访问原语（open/query/connect），单行连接代码路径 | `grep -nE 'rusqlite::Connection|Connection::open|pub fn (connect|query)' src-tauri/src/database.rs` | 命中 `rusqlite::Connection`、`Connection::open`（442）、`pub fn connect`（424）、`pub fn query`（477） |
| T6 | ACL 末条恒为 `list_artifact_images`（K1），未来 `graph_*` 命令插其前 | `tail -3 src-tauri/permissions/default-commands.toml` | `..., "db_disconnect", "list_artifact_images" ]` |
| T7 | `check-core-boundary.py` 已挂 `pre-merge.sh` | `grep -n check-core-boundary scripts/pre-merge.sh` | 行 404-409（--self-test/default/--expect-pending）+ 行 520-522 缺失检测 |
| T8 | `Cargo.toml` 已声明 `[lib] name = "mvp_core"` | `grep -n '\[lib\]' src-tauri/Cargo.toml` | 行 13-14 `[lib]` / `name = "mvp_core"` |
| T9 | `git diff --check` 干净（无尾随空白/冲突标记） | `git diff --check` | `DIFFCHECK_OK` |

**结论**：W1 delta 全部关键断言（边界可机检、图谱纯函数可落 core、graph_store 复用 database.rs、ACL K1 不变、零第二执行路径可由 `check-core-boundary.py`+未来 `check-graph-policy.py` 双守门）均获实测支撑。

---

## 3. 解锁状态（Unblock Status for M5-7/8 实施）

| 依赖（W1 delta §1/§4） | 状态 | 证据 |
|---|---|---|
| A2 M5-1 core boundary 落地 | ✅ 已满足 | `854bc40` 提交；T1/T2/T8 PASS |
| `check-core-boundary.py` 可机检且当前 PASS | ✅ 已满足 | T1/T2 |
| `database.rs` SQLite 访问原语可用（graph_store 复用） | ✅ 已满足 | T5 |
| A2 M5-1.b（B 类 seam / `PathResolver`） | ⏳ 切片 0b/1 待 A2 后续波次；图谱抽取器取路径经注入 `PathResolver` 或 `PathBuf` 入参，不依赖 AppHandle | `core/mod.rs` 注释「切片 0b 先于其余模块」「切片 1/2 注入 ProgressSink/PathResolver 后搬入 B 类」 |
| 图谱**实现**责任 Lane | ⚠️ 归 A17（M5-7/8 卡），非 A7 | `M5-7-graph-model-extract.md` / `M5-8-graph-store-query.md` 顶部「责任 Lane 候选：A17」 |

---

## 4. 下一张"最小可集成切片"的解锁条件（A0 裁决）

用户要求"做下一张最小可集成切片"。在 M5-W1 框架下，A7 的切片 = 本 docs 交付（已完成）。**真正图谱产品代码切片（M5-7/8）的推进需要以下任一条件成立**，否则 A7 不得写产品代码：

- **(a)** A0 在 `PARALLEL_COMMAND_BOARD.md` 发布 **M5-W2 dispatch**，将图谱实现责任显式分派给某 Lane（按 M5-7/8 卡应为 A17；若 A0 改派 A7，须同时解除 W1 硬停"仅 A2 可动产品代码"对该 lane 的约束）；或
- **(b)** A0 显式书面覆盖 M5-W1 硬停，授权 A7 在 M5-W1 内实施图谱模块（不推荐：破坏"仅 A2 可动产品代码"的统一闸门，且与 M5-7/8 卡的责任 Lane=A17 冲突）。

在 (a)/(b) 未成立前，A7 严守 SUPPORT DOCS ONLY：仅维护 `logs/assist/A7-M5-graph-core-*.md` 文档，不触碰 `src-*`/`Cargo.toml`/ACL/新命令/`pre-merge.sh`。

---

## 5. 未决风险 / 备注

- **board 缺 M5-W2**：用户指令提及 "M5-W1/M5-W2"，但最新 board（08:35）仅 M5-W1。已按 board 为真相源执行；若 A0 确已规划 M5-W2，请同步更新 board 后再令 A7 推进实现。
- **B 类 seam 未落地**：图谱抽取器若需运行时路径解析，依赖 A2 切片 0b/1 的 `PathResolver` 注入；在 A2 未落地前，core 内图谱模块应仅接收 `PathBuf` 入参（不引 AppHandle），与 `COREBOUND_TAURI_IN_CORE` 兼容。
- **零第二执行路径**：未来 `graph_maintenance` 任务体（M5-8 §4.4）必须走 `script_runner`，与 scheduler F6 / `check-core-boundary.py` 的 `creates a second execution path` 同源守门；W1 delta `AGRAPH_1` 已绑定。

---

## 6. 输出模板回填

```text
LANE=A7
STATUS=PASS_WITH_DOCS
WAVE=M5-W1 (SUPPORT DOCS ONLY)
BASE=854bc40
HEAD=docs only (logs/assist/A7-M5-graph-core-20260906-0755.md + A7-M5-graph-core-W1-delta-20260906-0827.md + A7-M5-graph-core-W1-checkpoint-20260906-1338.md)
FILES=logs/assist/A7-M5-graph-core-W1-checkpoint-20260906-1338.md
VERIFY=T1-T9 全 PASS（check-core-boundary.py 自测+默认均 EXIT=0；core 文件=2；database.rs 暴露 open/connect/query；ACL 末条=list_artifact_images；pre-merge 已挂；git diff --check 干净；工作树无他 lane 游离产品代码）
CHECKPOINT=logs/assist/A7-M5-graph-core-W1-checkpoint-20260906-1338.md
MERGE_NOTES=W1 SUPPORT DOCS ONLY；无 patch；A2 M5-1 core 边界已落地并提交（854bc40），W1 delta 依赖闸门满足；
           图谱实现责任=A17（M5-7/8），非 A7；board 仅 M5-W1、无 M5-W2，A7 严守硬停未写产品代码；
           与 A2 §8 行 420、M5-7/M5-8 卡一致；与他 lane W1 产物无文件交集
NEXT=A0 发布 M5-W2 dispatch（将图谱实现派予 A17，或由 A0 显式覆盖 W1 硬停改派 A7）→ A17 落 M5-7.a/M5-8.a（graph 纯函数入 core/、graph_store 复用 database.rs、AGRAPH_1 绑定 check-core-boundary.py）
```
