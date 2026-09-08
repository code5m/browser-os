# A1 · M5-W18-R2 Evidence Closure Report

> Lane: A1（RESEARCH，R2 evidence closure）
> R1 verdict: `REWORK`（A0 audit 2026-09-08 09:05 CST）
> BASE = `origin/master` `d6127c4`（docs(M5-W18): dispatch evidence closure research）
> Worktree: `/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3` @ branch `codex/m5-w18-a1`
> 依据：`PARALLEL_COMMAND_BOARD.md` L1439-1484（M5-W18-R2 Evidence Closure Dispatch），A1 行 L1457
> R1 报告：`logs/research/M5-W18/A1-product-baseline-20260908.md`（保留为草稿，不作为实现授权）
> 边界：只读研究；零产品代码改动；不 push。仅产出本报告 + checkpoint 至 `logs/research/M5-W18/`。

---

## 0. 输出头（board L1416 格式）

```text
LANE=A1
STATUS=PASS_WITH_DEBT
BASE=d6127c4
HEAD=<本 worktree R2 commit，见 A1-checkpoint-R2>
REFERENCE_EVIDENCE=CURRENT_PRODUCT: src-tauri/src/*.rs, src/*.ts, src/components/*.vue, src-tauri/permissions/*.toml, src-tauri/capabilities/*.json, scripts/check-*, LICENSE, logs/checkpoints/M5-20260906/M5-14-debt-ledger.md; REFERENCE_SOURCE: WORKSPACE_IDENTITY.md, PARALLEL_COMMAND_BOARD.md L1439-1484, logs/checkpoints/A0-M5-W18-R1-audit-20260908.md
FILES=logs/research/M5-W18/A1-product-baseline-R2-20260908.md, logs/research/M5-W18/A1-checkpoint-R2-20260908.md
SOURCE_MAP=src-tauri/src/*.rs (28 modules, 30,832 lines), src/stores/*.ts (16), src/components/* (9 dirs), src-tauri/permissions/*.toml (2), src-tauri/capabilities/*.json (2) + dev-capabilities/main.json (1), scripts/check-* (50: 29 py + 20 mjs + 1 sh)
CLASSIFICATION=N/A（A1 为基线 lane，不做 COPY/ADAPT 判定；该判定属 A4/A7/A10 职责）
VERIFY=read-only：grep/wc/python3 静态比对；未运行构建或测试（research-only 边界）
CHECKPOINT=logs/research/M5-W18/A1-checkpoint-R2-20260908.md
MERGE_NOTES=A1 是 A0 集成顺序首位；本 R2 报告修正 R1 的 4 处事实错误，产出 machine-checkable fact appendix（§7）与修正后 gap list（§3）
NEXT=A0 据本报告打开 W19 时，须先消 W17 拓留债（W17-D1~D5）与 DbValue 二源漂移（见 §2.G4 修正）
```

---

## 1. R2 任务与 R1 审计修正

R2 assignment（board L1457）：
> Re-run the current-product inventory from `origin/master`. Record the exact 135/135/46 command facts, 29-Python/50-total policy counting rules, current Rust/frontend test inventory, real stores/components, build-size gate, locked authorities, and current duplicate `DbValue` definitions. Produce a machine-checkable fact appendix and a corrected gap list; remove stale W17 assumptions.

A0 audit 对 A1 的 REWORK 理由（`A0-M5-W18-R1-audit-20260908.md` L36）：
> Useful inventory outline, but command counts, DbValue state, test counts, and some W17 debt are stale or unsupported.

### 1.1 显式修正清单（R2 contract §4：逐条 retract/replace）

| # | R1 错误声明 | 修正后事实 | 证据类型 | 验证命令/路径 |
|---|---|---|---|---|
| C1 | R1 §2.3："总数：**137** 个 `#[tauri::command]`；ACL 放行 **137** 条" | **135** registered commands in `generate_handler!`，**135** ACL entries in `default-commands.toml`，**46** typed `invoke()` calls in `bridge.ts`。137 是 `#[tauri::command]` 注解总数，含 2 个未注册函数。 | `CURRENT_PRODUCT` + `EXECUTED_SYNTHETIC_TEST` | `grep -c "#\[tauri::command\]" src-tauri/src/*.rs` → 137；`python3` parse `generate_handler!` → 135；`grep -c "invoke(" src/bridge.ts` → 46 |
| C2 | R1 §2.2 & §3.G4："DbValue **三重**真源漂移…前端 `types.ts` 用 **PascalCase**" | DbValue 当前为**二源**漂移：`database.rs:122`（live，`I64/F64/Binary{bytes:usize}`）vs `domain.rs:1094`（dead-code，`Int/Float/BlobLen(u64)`）。`types.ts:603` 已修正为 **snake_case**（`"null"/{bool}/{int}/{float}/{text}/{blob_len}`），与 `domain.rs` 对齐，**不与 `database.rs` 对齐**。B8-1 已修复（types.ts:601-602 注释确认）。R1 的"PascalCase"描述是**过时的**。 | `CURRENT_PRODUCT` | `src/types.ts:600-609`；`src-tauri/src/database.rs:122-135`；`src-tauri/src/domain.rs:1094-1101` |
| C3 | R1 §2.6："`scripts/check-*.py\|mjs` **49** 个" | **50** total `check-*` scripts：**29** Python + **20** MJS + **1** shell。R1 的 49 漏计 1 个。 | `CURRENT_PRODUCT` + `EXECUTED_SYNTHETIC_TEST` | `ls scripts/check-*.py \| wc -l` → 29；`ls scripts/check-*.mjs \| wc -l` → 20；`ls scripts/check-*.sh \| wc -l` → 1；total → 50 |
| C4 | R1 §2.8："32 条债务行…P0 级标记 4 处" | M5-14 debt ledger 643 行，**21** debt rows with `D`-prefix ID，**2** lines mentioning `P0`。R1 的 32/4 是**未精确计数**。W17 残留债为 **W17-D1~D5**（5 条，非 R1 所写的 D1~D6）；W17-D6 在 ledger 中标注为"新增"且归 A7/A0 修复，不属 A1 基线挂账。 | `CURRENT_PRODUCT` | `grep -cE "^\|.*D[0-9]" logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` → 21；`grep -c "P0" …` → 2；`grep -n "W17-D" …` → D1-D5 + D6(new) |
| C5 | R1 §2.1："src-tauri/src/*.rs（29 模块）" | **28** `.rs` files in `src-tauri/src/`，**30,832** total lines。R1 的 29 模块计数错误。 | `CURRENT_PRODUCT` + `EXECUTED_SYNTHETIC_TEST` | `ls src-tauri/src/*.rs \| wc -l` → 28；`cat src-tauri/src/*.rs \| wc -l` → 30,832 |
| C6 | R1 §2.7："Rust `#[test]`/`#[tokio::test]`：470 个" | **470** `#[test]` confirmed；**0** `#[tokio::test]`。`database.rs` has **23** `#[test]`（audit said 22；actual is 23 — minor audit discrepancy noted）。R1 的 470 总数正确但未分解 `#[tokio::test]`。 | `CURRENT_PRODUCT` + `EXECUTED_SYNTHETIC_TEST` | `grep -rn "#\[test\]" src-tauri/src/ \| wc -l` → 470；`grep -rn "#\[tokio::test\]" src-tauri/src/ \| wc -l` → 0；`grep -c "#\[test\]" src-tauri/src/database.rs` → 23 |

---

## 2. 修正后当前产品端到端基线（10 维度）

### 2.1 graph / database Rust 模块

| 模块 | 行数 | 关键符号 | 状态 | 证据 |
|---|---|---|---|---|
| `graph.rs` | 733 | `GraphStore`(L185) / `GraphState`(L311) / `graph_query_impl`(L338) / `graph_node_get_impl`(L371) / `graph_stats_impl`(L380) / `load_snapshot`(L395) | 后端完整；命令已注册 + ACL 放行 | `CURRENT_PRODUCT`: `src-tauri/src/graph.rs` |
| `database.rs` | 1273 | `DbValue`(L122) / `DbPool`(L405) / `validate_config` / `validate_sqlite_path` / `detect_multiple_statements` / `resolve_timeout_secs`；**23 `#[test]`** | 后端完整；`db_connect/db_query/db_disconnect` 已落地 | `CURRENT_PRODUCT`: `src-tauri/src/database.rs` |
| `domain.rs` | 2521 | `DbValue`(L1094, **`#[allow(dead_code)]`**) / `DbConnectionConfig`(L911) / `DbQueryResult`(L1110) / `GraphNode/Edge/NodeView/EdgeView/QueryRequest/QueryLimits` | DTO 集中定义处，**含死代码 DbValue DTO** | `CURRENT_PRODUCT`: `src-tauri/src/domain.rs` |

**G1（graph 已落地，叠加非重写）**：`CURRENT_PRODUCT`。W18-R 目标"从零引入 Obsidian 图能力"不成立——产品已有 GraphStore 后端 + 前端消费层。A2/A3 须叠加在现有 GraphStore 之上。

### 2.2 DTOs — DbValue 二源漂移（修正 R1 的"三重"描述）

| 真源 | 位置 | 变体定义 | serde | 状态 | 证据 |
|---|---|---|---|---|---|
| **实际生效** | `database.rs:122` | `Null/Bool(bool)/I64(i64)/F64(f64)/Text(String)/Binary{bytes:usize}` | `snake_case` | 运行时使用（有 `byte_len()`） | `CURRENT_PRODUCT` |
| 死代码 | `domain.rs:1094` | `Null/Bool(bool)/Int(i64)/Float(f64)/Text(String)/BlobLen(u64)` | `snake_case` | `#[allow(dead_code)]`，未使用 | `CURRENT_PRODUCT` |
| 前端镜像 | `types.ts:603` | `"null" / {bool} / {int} / {float} / {text} / {blob_len}` | snake_case | **已修正**（B8-1 修复），与 `domain.rs` 对齐，**不与 `database.rs` 对齐** | `CURRENT_PRODUCT`: `src/types.ts:600-609` |

**G4（DbValue 二源漂移，修正后）**：当前风险是 `database.rs`（live，`I64/F64/Binary`）与 `domain.rs`+`types.ts`（`Int/Float/BlobLen`）的变体名不一致。前端已修正为 snake_case 且与 domain.rs 对齐，但 domain.rs 是死代码——**真正的对齐目标应是 `database.rs`（live 真源）**。A10/A11 须统一为单一真源。R1 的"三重漂移 + PascalCase"描述已过时（B8-1 已修）。

### 2.3 commands（修正 R1 的 137 → 135/135/46）

| 维度 | 值 | 证据 |
|---|---|---|
| `#[tauri::command]` 注解总数 | 137 | `EXECUTED_SYNTHETIC_TEST`: `grep -rn "#\[tauri::command\]" src-tauri/src/ \| wc -l` → 137 |
| `generate_handler!` 注册命令数 | **135** | `EXECUTED_SYNTHETIC_TEST`: python3 parse → 135 |
| `default-commands.toml` ACL 放行数 | **135** | `EXECUTED_SYNTHETIC_TEST`: python3 parse `commands.allow` → 135 |
| `bridge.ts` typed `invoke()` 调用数 | **46** | `EXECUTED_SYNTHETIC_TEST`: `grep -c "invoke(" src/bridge.ts` → 46 |
| 未注册的 `#[tauri::command]` | 2 | `INFERENCE`：137 注解 − 135 注册 = 2（非 ACL 命令，可能为测试 helper 或内部函数） |

注册分两段：主窗 `main.rs:1356` + 宫格子窗 `main.rs:112`。一致性门禁 `scripts/check-command-set-consistency.py` 存在且可执行。

### 2.4 stores（前端状态层）

**16** stores（`CURRENT_PRODUCT`: `ls src/stores/*.ts` → 16）：
`useAgentStore / useBookmarkStore / useBrowserStore / useDatabaseStore / useGitStore / useGraphStore / useHomeStore / useImagePreviewStore / useLayoutStore / usePluginStore / useResourceStore / useSessionStore / useSettingsStore / useSystemStore / useTaskStore / useWorkspaceStore`。

- `useDatabaseStore.ts`：218 行，消费 `db_connect/db_query/db_disconnect`。
- `useGraphStore.ts`：310 行，消费 `graph_query/graph_node_get/graph_stats`。
- **`useConnectionStore` 不存在**（`CURRENT_PRODUCT`: `find src -name "*onnection*tore*"` → 无结果）。A5 R1 报告引用此 store 是错误的。

### 2.5 panels（前端视图层）

面板目录（**9** dirs）：`browser / graph / home / layout / plugin / shared / system / workspace`（`CURRENT_PRODUCT`: `ls src/components/`）。

- **graph 消费层完整**：`src/components/graph/` 含 `GraphPanel.vue / GraphViewer.vue / NodeDetail.vue / EdgeDetail.vue / GraphFilter.vue`（5 组件）+ `useGraphStore.ts`。
- **database 消费层存在**：`src/components/workspace/DatabasePanel.vue` + `useDatabaseStore.ts`。
- **检索消费层缺失**：无 search/grep/find 面板或 store（G3）。

### 2.6 policies（修正 R1 的 49 → 50/29-Python）

**50** `check-*` scripts（`CURRENT_PRODUCT` + `EXECUTED_SYNTHETIC_TEST`）：

| 类型 | 数量 | 验证 |
|---|---|---|
| Python `check-*.py` | **29** | `ls scripts/check-*.py \| wc -l` → 29 |
| MJS `check-*.mjs` | **20** | `ls scripts/check-*.mjs \| wc -l` → 20 |
| Shell `check-*.sh` | **1** | `ls scripts/check-*.sh \| wc -l` → 1（`check-dev-startup.sh`） |
| **Total** | **50** | `ls scripts/check-* \| wc -l` → 50 |

关键脚本：`check-command-set-consistency.py`（命令集一致性）、`check-dev-startup.sh`（debug 启动）、`pre-merge.sh`（集成门禁）。

### 2.7 tests（修正 R1 的 470 分解）

| 维度 | 值 | 证据 |
|---|---|---|
| Rust `#[test]` | **470** | `EXECUTED_SYNTHETIC_TEST`: `grep -rn "#\[test\]" src-tauri/src/ \| wc -l` → 470 |
| Rust `#[tokio::test]` | **0** | `EXECUTED_SYNTHETIC_TEST`: `grep -rn "#\[tokio::test\]" src-tauri/src/ \| wc -l` → 0 |
| `database.rs` `#[test]` | **23** | `EXECUTED_SYNTHETIC_TEST`: `grep -c "#\[test\]" src-tauri/src/database.rs` → 23 |
| 前端单元测试 | **0** | `CURRENT_PRODUCT`: `find src -name "*.test.ts" -o -name "*.spec.ts"` → 无；`package.json` 无 vitest/jest/cypress/playwright |

**G5（前端单测网 = 0）**：`CURRENT_PRODUCT`。前端质量仅靠 50 策略脚本 + 人工/原生实跑。

### 2.8 known debt（修正 R1 的 32/4 → 21/2）

`logs/checkpoints/M5-20260906/M5-14-debt-ledger.md`（643 行）。

| 维度 | R1 声称 | 实际 | 证据 |
|---|---|---|---|
| Debt rows (D-prefix) | 32 | **21** | `EXECUTED_SYNTHETIC_TEST`: `grep -cE "^\|.*D[0-9]" …` → 21 |
| P0 mentions | 4 | **2** | `EXECUTED_SYNTHETIC_TEST`: `grep -c "P0" …` → 2 |

W17 残留债（`CURRENT_PRODUCT`: ledger L610-615, L623-627）：
- **W17-D1**：体积门禁基线被脏树采集污染（干净树复测待 A11）
- **W17-D2**：`pre-merge` 未覆盖 W17 三个新门禁（`check-home-client-policy.py` / `check-home-store-logic.mjs` / `check-home-ui-logic.mjs`）
- **W17-D3**：A11 矩阵过期行
- **W17-D4**：活动条持久化最近目录路径（非新增，非 `HOME_NO_SECRET_PERSIST` 码位）
- **W17-D5**：原生客户端视觉验收 headless BLOCKED

**修正 R1 的 W17-D6 挂账**：W17-D6 在 ledger 中标注为"新增"且归 A7/A0 修复（B10-b `MainArea.vue` 兜底 `v-else` 配对错误），**不属 A1 基线挂账范围**。R1 将 D1~D6 列为 A1 基线债是越界的——A1 只记录，不承担 D6 修复责任。

历史挂账 D23-D26（M4-1 遗留）：D23 终端 GUI 实点 / D24 吞吐基线 / D25 `on_channel_dead` 未 wait / D26 历史未按字符封顶。

### 2.9 build-size budget

- 上限常量：`scripts/measure-build-metrics.py:39` `TOTAL_BYTES_GROWTH_LIMIT_PCT = 25.2`（`CURRENT_PRODUCT`）。
- W17 实测 25.55%（超限 +0.35pp，未抬上限）——此数据来自 R1 报告与 ledger，**非 A1 本轮复测**（A1 不运行构建）。A11 须在干净树复测（W17-D1）。
- 现状 metrics json：`logs/m0-build-metrics/build-metrics-052b18a.json` 等存在，字段完整性待 A11 核实（W17-D3）。

### 2.10 locked authorities（运行时权限锁定面）

| 维度 | 值 | 证据 |
|---|---|---|
| `capabilities/default.json` permissions | **4** | `CURRENT_PRODUCT`: `core:default` / `core:window:allow-create` / `browser-tabs:default` / `default-commands` |
| `capabilities/browser-remote.json` permissions | **2** | `CURRENT_PRODUCT`: `core:default` / `remote-collect` |
| `dev-capabilities/main.json` permissions | **4** | `CURRENT_PRODUCT`: 同 default.json，仅 `#[cfg(debug_assertions)]` 注册 |
| `security_policy.rs` `BLOCKED_LAUNCH_PROGRAMS` | **12** | `CURRENT_PRODUCT`: sh/bash/dash/ash/zsh/ksh/csh/tcsh/fish/powershell/pwsh/cmd |
| `security_policy.rs` `BLOCKED_LAUNCH_WRAPPERS` | **10** | `CURRENT_PRODUCT`: env/busybox/nohup/sudo/su/doas/pkexec/xargs/timeout/setsid |
| `security_policy.rs` `BLOCKED_INTERPRETERS` | **12** | `CURRENT_PRODUCT`: python/perl/ruby/node/nodejs/deno/bun/lua/luajit/php/tclsh/osascript |

运行时权限 LOCKED（board L7）。任何 W19 新功能命令必须入 `default-commands.toml` + capability，且不得扩张运行时权限面。

### 2.11 LICENSE

- 产品根 `LICENSE`：**MulanPSL-2.0**（木兰宽松许可证第2版）（`CURRENT_PRODUCT`: `head -1 LICENSE`）。
- dbx / zvec-grep：Apache-2.0（`REFERENCE_SOURCE`: W18 dispatch L27-29）。
- A8 R1 报告称产品为 Apache-2.0 是**错误的**（audit point 7 确认）。

---

## 3. 修正后 Canonical Gap Inventory

| # | 缺口 | 当前产品状态 | 上游目标 | 涉及 lane | 严重度 | 修正说明 |
|---|---|---|---|---|---|---|
| **G1** | Obsidian 图语义叠加 | GraphStore 后端 + 前端 GraphPanel **已存在** | wikilink/backlink/alias/heading-block/tag/frontmatter/orphan/unresolved/filters/depth | A2/A3 | 高 | 无修正（R1 正确） |
| **G2** | dbx 完整 workbench | database.rs 有连接/查询/取消/池 + DatabasePanel 基础 | schema browser / editor tabs / 分页·筛选·拷贝·导出 / 历史 / SQL 风险分析 / 连接树 | A4/A5/A6 | 高 | 无修正 |
| **G3** | 代码检索后端 | **全仓零检索能力** | managed ripgrep / FTS-BM25 / vector / hybrid-RRF / 增量索引 | A7/A8/A9 | 极高 | 无修正 |
| **G4** | DTO 单真源 | DbValue **二源**漂移：`database.rs`（live，`I64/F64/Binary`）vs `domain.rs`+`types.ts`（dead-code + 前端，`Int/Float/BlobLen`，snake_case 已对齐） | 统一单一真源（`database.rs` live） | A10/A11 | 高 | **修正 R1**：非三重漂移，前端已修为 snake_case；真正风险是 live vs dead-code 变体名不一致 |
| **G5** | 前端单测网 | 0 个前端单测 | 行为级单测覆盖新增 Vue 组件/store | A11 | 中 | 无修正 |
| **G6** | 体积预算余量 | 上限 25.2%，W17 达 25.55%（超限，待 A11 干净树复测） | 新增功能须保持 ≤25.2% delta | A4/A5/A7/A10 | 高 | **修正**：W17-D1 待复测，A1 不复测 |
| **G7** | 权限面零扩张 | 运行时 LOCKED；ACL 135 全放行 | 新命令须入 ACL + capability，不扩张 LOCKED 面 | A2-A9/A10 | 高 | **修正 R1**：137 → 135 |
| **G8** | 债务未消 | M5-14 含 D23-D26 + W17-D1~D5，P0×2 | W19 前须消或显式 carry-forward | A0/A11 | 中 | **修正 R1**：32 行 → 21 行；P0×4 → P0×2；W17-D1~D5（非 D1~D6，D6 归 A7/A0） |

---

## 4. Requirements Checklist（保留 R1 的 R1-R12，无修正）

R1 的 §4 Requirements Checklist（R1-R12）内容正确且完整，下游 lane 仍须逐条回答。完整文本见 `A1-product-baseline-20260908.md` §4（R1 草稿保留）。本 R2 报告不重复以避免漂移；下游 lane 读 R1 §4 即可。

---

## 5. 跨 lane 一致性约束（保留，无修正）

R1 §5 的 5 条约束（research-only / worktree 隔离 / license / ACL 容量硬约束 / DTO 单真源）正确。本 R2 补充：

6. **R2 evidence contract**（board L1443-1451）：每个事实声明须标注 `CURRENT_PRODUCT` / `REFERENCE_SOURCE` / `OBSERVED_BEHAVIOR` / `OFFICIAL_DOC` / `EXECUTED_SYNTHETIC_TEST` / `INFERENCE`，附精确路径/行号或命令/结果。不得将 inference 冒充 observed behavior。
7. **R2 输出范围**（board L1446）：仅限 `logs/research/M5-W18/A1-*` 文件 + lane checkpoint。不改产品代码、manifest、lockfile、脚本、capability、ACL、vault。

---

## 6. 未解问题（A1 视角，修正后）

| # | 问题 | 归属 | 证据类型 |
|---|---|---|---|
| U1 | 2 个 `#[tauri::command]` 注解未注册在 `generate_handler!` 中（137 注解 − 135 注册）。需 A11 用 `check-command-set-consistency.py` 精确核对差集。 | A11 | `INFERENCE` |
| U2 | `DatabasePanel.vue` 功能完整度（是否仅 M4 基础，缺 dbx 级 schema browser/导出）需 A5 核实。 | A5 | `INFERENCE` |
| U3 | 体积 metrics json 字段完整性（W17-D3）待 A11 补齐。A1 不运行构建。 | A11 | `INFERENCE` |
| U4 | 当前产品 graph snapshot 格式（`load_snapshot` 路径）是否与 Obsidian `graph.json` 兼容，需 A2 比对。 | A2 | `INFERENCE` |
| U5 | `database.rs` 的 23 `#[test]` vs audit 所称 22 — 差异 1。可能是 audit 后新增 1 个测试，或 audit 计数偏差。不影响基线结论。 | A0/A11 | `CURRENT_PRODUCT` + `REFERENCE_SOURCE` |

---

## 7. Machine-Checkable Fact Appendix

> 本节所有事实可通过命令复现验证。每条附验证命令与期望输出。

```bash
# === Commands ===
# Registered commands in generate_handler!
python3 -c "
import re
with open('src-tauri/src/main.rs') as f: c = f.read()
h = re.findall(r'generate_handler!\[(.*?)\]', c, re.DOTALL)
r = set()
for x in h:
    for n in re.findall(r'([a-z_][a-z0-9_]*)\s*[,\(]', x):
        if n not in ('invoke','emit','listen','unlisten'): r.add(n)
print(len(r))
"  # EXPECT: 135

# ACL entries in default-commands.toml
python3 -c "
import re
with open('src-tauri/permissions/default-commands.toml') as f: c = f.read()
m = re.search(r'commands\.allow\s*=\s*\[(.*?)\]', c, re.DOTALL)
print(len([x.strip().strip('\"').strip(\"'\") for x in m.group(1).split(',') if x.strip()]))
"  # EXPECT: 135

# bridge.ts invoke calls
grep -c "invoke(" src/bridge.ts  # EXPECT: 46

# tauri::command annotations (includes 2 unregistered)
grep -rn "#\[tauri::command\]" src-tauri/src/ | wc -l  # EXPECT: 137

# === Policy scripts ===
ls scripts/check-*.py | wc -l   # EXPECT: 29
ls scripts/check-*.mjs | wc -l  # EXPECT: 20
ls scripts/check-*.sh | wc -l   # EXPECT: 1
ls scripts/check-* | wc -l      # EXPECT: 50

# === Tests ===
grep -rn "#\[test\]" src-tauri/src/ | wc -l           # EXPECT: 470
grep -rn "#\[tokio::test\]" src-tauri/src/ | wc -l    # EXPECT: 0
grep -c "#\[test\]" src-tauri/src/database.rs         # EXPECT: 23
find src -name "*.test.ts" -o -name "*.spec.ts" | wc -l  # EXPECT: 0

# === Stores ===
ls src/stores/*.ts | wc -l  # EXPECT: 16

# === Rust modules ===
ls src-tauri/src/*.rs | wc -l       # EXPECT: 28
cat src-tauri/src/*.rs | wc -l      # EXPECT: 30832
wc -l src-tauri/src/graph.rs        # EXPECT: 733
wc -l src-tauri/src/database.rs     # EXPECT: 1273
wc -l src-tauri/src/domain.rs       # EXPECT: 2521

# === DbValue definitions ===
grep -n "enum DbValue" src-tauri/src/database.rs  # EXPECT: 122:pub enum DbValue {
grep -n "enum DbValue" src-tauri/src/domain.rs    # EXPECT: 1094:pub enum DbValue {
# types.ts DbValue is snake_case (B8-1 fixed)
sed -n '603,609p' src/types.ts  # EXPECT: snake_case variants

# === Build-size gate ===
grep "TOTAL_BYTES_GROWTH_LIMIT_PCT" scripts/measure-build-metrics.py  # EXPECT: = 25.2

# === Capabilities ===
python3 -c "import json; print(len(json.load(open('src-tauri/capabilities/default.json'))['permissions']))"       # EXPECT: 4
python3 -c "import json; print(len(json.load(open('src-tauri/capabilities/browser-remote.json'))['permissions']))" # EXPECT: 2
python3 -c "import json; print(len(json.load(open('src-tauri/dev-capabilities/main.json'))['permissions']))"      # EXPECT: 4

# === Security blacklists ===
grep -A 1 "BLOCKED_LAUNCH_PROGRAMS" src-tauri/src/security_policy.rs | head -1  # EXPECT: [&str; 12]
grep -A 1 "BLOCKED_LAUNCH_WRAPPERS" src-tauri/src/security_policy.rs | head -1  # EXPECT: [&str; 10]
grep -A 1 "BLOCKED_INTERPRETERS" src-tauri/src/security_policy.rs | head -1     # EXPECT: [&str; 12]

# === License ===
head -1 LICENSE  # EXPECT: 木兰宽松许可证，第2版

# === Debt ledger ===
wc -l logs/checkpoints/M5-20260906/M5-14-debt-ledger.md                              # EXPECT: 643
grep -cE "^\|.*D[0-9]" logs/checkpoints/M5-20260906/M5-14-debt-ledger.md             # EXPECT: 21
grep -c "P0" logs/checkpoints/M5-20260906/M5-14-debt-ledger.md                       # EXPECT: 2

# === useConnectionStore absent ===
find src -name "*onnection*tore*"  # EXPECT: (no output)
```

---

## 8. R2 合规声明

- [x] Rebase onto `origin/master`（no-op，已 up to date at `d6127c4`）
- [x] 仅修改 `logs/research/M5-W18/A1-*` 文件 + lane checkpoint
- [x] 未修改产品代码、manifest、lockfile、脚本、capability、ACL、vault
- [x] 每个事实声明标注证据类型（`CURRENT_PRODUCT` / `REFERENCE_SOURCE` / `EXECUTED_SYNTHETIC_TEST` / `INFERENCE`）
- [x] 显式 retract/replace R1 的 6 处错误（§1.1 C1-C6）
- [x] 保留 R1 报告作为草稿，未静默改写
- [x] 未 push
- [x] Machine-checkable fact appendix（§7）附验证命令与期望输出

## 9. 结论

`STATUS=PASS_WITH_DEBT`

A1 R2 已完成当前产品端到端基线的修正盘点，所有事实可通过 §7 的命令复现验证。R1 的 6 处事实错误（命令计数 137→135/135/46、DbValue 三重→二源漂移、策略脚本 49→50/29-Python、债务 32/4→21/2、Rust 模块 29→28、W17-D1~D6→D1~D5）已显式修正。

**残留 debt**（不阻塞 A1 R2 PASS，但须在 W19 前消解）：
- W17-D1~D5（A11/A0 复测与覆盖）
- G4 DbValue 二源漂移统一（A10/A11）
- G5 前端单测网 bootstrap（A11）
- U1-U5 未解问题分派至 A2/A5/A11/A0
