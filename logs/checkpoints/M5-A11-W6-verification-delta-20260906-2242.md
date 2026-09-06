# M5-A11 · W6 验证增量（Verification Delta After W5 Integration; A8/A9 W6 Pending）

```text
LANE=A11
STATUS=PASS_WITH_DEBT（当前已集成状态 pre-merge=ALL_PASS；A8/A9 W6 产品代码尚未交付至本仓 → 其门禁 PENDING；存量小缺口见 §6）
BASE=4b438ef（graph model/store policy slice 由 A0 集成；W6 dispatch HEAD=77b1e3e）
HEAD=logs/checkpoints/M5-A11-W6-verification-delta-20260906-2242.md
FILES=logs/checkpoints/M5-A11-W6-verification-delta-20260906-2242.md
VERIFY=见 §1 全门表（当前工作树：W5 集成 A6 Agent/Skill UI shell + A7 graph model/store slice；A8/A9 W6 代码未到）
CHECKPOINT=本文件
MERGE_NOTES=§5 A8/A9 待交付 + §6 债务台账
NEXT=待 A0 集成 A8/A9 W6 代码后补验（M5-9 graph UI logic / M5-10/11 plugin manifest+lifecycle policy）；当前集成态 ALL_PASS，可放行
```

> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W6 Parallel Dispatch（L136-171）→ **A11 = START VERIFICATION：「One verification delta」**（after A8/A9 outputs）。scope `logs/assist/M5-A11-W6-*.md`、`logs/checkpoints/M5-A11-W6-*.md`。
> 范围声明：本增量**只产出验证文档，零产品代码改动**。A8/A9 的 W6 产品代码**尚未落入本主仓工作树**（见 §2），其门禁留待 A0 集成后补验。未 push。
> 姊妹件：W4 `M5-A11-W4-verification-delta-20260906-1458.md`、W5 `M5-A11-W5-verification-delta-20260906-1551.md`。

---

## 0. 启动门禁与调度匹配

```bash
cat .workspace-identity              # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master  ✅
pwd                                  # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git fetch origin && git pull --ff-only   # 已经是最新的（77b1e3e）
git status --short --branch          # 仅 M5-0-overview.md / M5-9 卡 modified + A4 W6 review 笔记 untracked；无 A8/A9 产品代码
git log --oneline -4                 # 77b1e3e / 4b438ef / 1a2c9cd(A11 W5) / f99d2eb(A6 W5)
```
- **调度匹配**：board 头部 `Current NEXT: M5-W6 parallel implementation`（L7），W6 仅开 A8（M5-9 graph UI 纯逻辑/面板壳）与 A9（M5-10/11 plugin manifest/lifecycle 策略切片）产品代码 lane（L159-160）；A11 = START VERIFICATION（L161-162）。
- **W6 事实（L139）**：Agent/Skill UI shell 已集成；graph model/store policy slice 已集成（=`4b438ef`）；build metrics 阈值文档化为 **19%** 并计入 W5 债务。本增量实测与之吻合。

---

## 1. W6 验证矩阵（当前工作树实跑，含 W5 集成态）

| 门 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | **370 passed**（2+368），0 failed | ✅ |
| cargo check 告警 | `cargo check --locked` | **2 warnings**（= 基线，W5 的 27 dead_code 已消解） | ✅ |
| core 边界门 self-test | `python3 scripts/check-core-boundary.py --self-test` | `CORE_POLICY_SELF_TEST=PASS`（ACTIVE=7，坏样本=9） | ✅ |
| M5-2 MCP 门 self-test | `python3 scripts/check-mcp-policy.py --self-test` | `MCP_POLICY_SELF_TEST=PASS`（ACTIVE=5，PENDING=9） | ✅ |
| M5-2 MCP 门 default | `python3 scripts/check-mcp-policy.py` | `MCP_POLICY=PASS` | ✅ |
| M5-3 agent-memory 门 self-test | `python3 scripts/check-agent-memory-policy.py --self-test` | `AGENT_KV_POLICY_SELF_TEST=PASS`（ACTIVE=5） | ✅ |
| M5-4/5 agent-skill 门 self-test | `python3 scripts/check-agent-skill-policy.py --self-test` | `AGENT_SKILL_POLICY_SELF_TEST=PASS`（ACTIVE=2，PENDING=4） | ✅ |
| M5-4/5 agent-skill 门 default | `python3 scripts/check-agent-skill-policy.py` | `AGENT_SKILL_POLICY=PASS` | ✅ |
| M5-7/8 graph 门 self-test | `python3 scripts/check-graph-policy.py --self-test` | `GRAPH_POLICY_SELF_TEST=PASS`（ACTIVE=7） | ✅ |
| M5-7/8 graph 门 default | `python3 scripts/check-graph-policy.py` | `GRAPH_POLICY=PASS` | ✅ |
| M2-4 执行通道 self-test | `python3 scripts/check-script-exec-policy.py --self-test` | `SELF_TEST_RESULT=ALL_PASS`（23 坏 + 1 好 + 码位完整） | ✅ |
| tools 策略 self-test | `python3 scripts/check-tools-policy.py --self-test` | `self-test OK`（1 好 + 16 坏，含变异防呆） | ✅ |
| db 策略 self-test | `python3 scripts/check-database-policy.py --self-test` | `DB_SELF_TEST_RESULT=PASS`（2 好 + 15 坏；ACTIVE=14，PENDING=1） | ✅ |
| scheduler 策略 self-test | `python3 scripts/check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS`（2 好 + 23 合成坏 + 23 真实变异；ACTIVE=23，PENDING=0） | ✅ |
| A6 Agent/Skill UI 逻辑 | `node scripts/check-agent-skill-ui-logic.mjs` | **57 assertions passed, 0 failed** | ✅ |
| Rust fmt | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | `FMT_CLEAN` | ✅ |
| 工作树 diff | `git diff --check` | `DIFF_CHECK_CLEAN` | ✅ |
| 前端构建 | `npm run build` | ✓ built；`index-BFuWKutL.js`=163.80 kB（gzip 58.74 kB） | ✅ |
| 构建指标对比 | `measure-build-metrics.py --compare 4f0e8ab --skip-build` | `exceeds_growth_limit=false`，`total_bytes_pct=18.58`（<19% 阈值），`cargo_warnings=0`，`warnings_increased=false` | ✅ |
| **集成门禁** | `bash scripts/pre-merge.sh` | **`PRE_MERGE_RESULT=ALL_PASS`**（EXIT=0，FAIL 行=0） | ✅ |

> 注：pre-merge 日志确认 `build metrics 未回归（基线 build-metrics-4f0e8ab.json）`，且所有策略门（core/mcp/agent-memory/agent-skill/graph/script-exec/tools/db/scheduler、baseline-check、verify-resources）均 PASS。当前集成态（W5：A6 Agent/Skill UI shell + A7 graph model/store slice via `4b438ef`）**全绿**。

---

## 2. A8/A9 W6 产品代码：尚未交付至本仓（门禁 PENDING）

按 W6 调度，A8（M5-9 graph UI）/A9（M5-10/11 plugin）应产出代码，但本主仓工作树**当前不含**其任何产物：

```bash
ls scripts/ | grep -iE 'check-plugin-policy|check-graph-ui-logic'   # 无
ls src-tauri/src/ | grep -iE 'plugin'                              # 无 plugin.rs
ls src/components/workspace/ | grep -iE 'graph|plugin'             # 无 graph/plugin UI 组件
node scripts/check-graph-ui-logic.mjs    # 文件缺失（Node 报错）→ 证实 A8 W6 未到
python3 scripts/check-plugin-policy.py   # 文件缺失 → 证实 A9 W6 未到
```

- **结论**：A8/A9 W6 门禁（M5-9 graph UI 逻辑测试、M5-10/11 plugin manifest/lifecycle 策略脚本）**PENDING**，待其代码由 A0 集成至本仓后补验。
- 据 board L159-160，A8 不得加后端命令/live agent 消费/model 调用/graph rebuild worker；A9 不得安装/删除/下载/执行/启用真实插件，纯 manifest/lifecycle 策略。**二者均未越界之虞**——因代码尚未落入，本车道仅预留验证位。
- 建议：A0 集成 A8/A9 后，A11 补验项 = `node scripts/check-graph-ui-logic.mjs` + `python3 scripts/check-plugin-policy.py --self-test/default`，并核对新增 `bridge.ts`/`types.ts`/ACL 命令奇偶与「prefer 无命令」硬停止。

---

## 3. W5 集成态补验（A6/A7 via 4b438ef / f99d2eb）—— 红灯已消解

W5 增量（1a2c9cd）报告唯一红灯：`cargo_warnings 2→27`（A7 M5-7/8 图谱契约 dead_code），导致 pre-merge FAIL。**本增量实测该红灯已消解**：

- `cargo check --locked` → **2 warnings（= 基线）**。推测 `4b438ef` 集成 graph model/store slice 时按 A7 W5 计划（模块级 `#![allow(dead_code)]`）消解了 domain.rs 的 `Graph*` 未用类型/常量。
- `measure-build-metrics` → `cargo_warnings=0`（增量），`warnings_increased=false`。
- **命令表面核查（W5/W6 硬停止「prefer 无命令」）**：`grep '#[tauri::command]' graph.rs` → 无；ACL 无 graph 条目 → `4b438ef` 未暴露新命令 ✅。A6 `f99d2eb`（Agent/Skill UI shell）亦无新命令（W5 已核）。

> W5 债务记账的「cargo_warnings 2→27」项**已结**（转为 A0 文档化阈值决策，见 §4）。

---

## 4. 构建指标阈值决策（W5 债务计入）

- `measure-build-metrics.py`：`TOTAL_BYTES_GROWTH_LIMIT_PCT = 19.0`（L31，A0 于 2026-09-06 书面由 15%→16%→19% 逐级抬升，吸收 W5 Agent/Skill 三面板懒加载 chunk 增长）。
- 当前实测 `total_bytes_pct = 18.58%`（163.80 kB vs 基线），**低于 19% 上限，余量仅 0.42%**——属合规但偏紧；后续若 A8 graph UI / A9 plugin UI 进主 chunk 需警惕越阈。
- `cargo_warnings` 回归基线 2，阈值内无需豁免。
- 基线文件仍存两份（`4f0e8ab` + `6f4e554`）；pre-merge 以 `sort|head -1` 取最旧（更严）的 `4f0e8ab`，行为正确，建议 A0 归档旧基线。

---

## 5. Before-A0 冲突扫描

- **空文件**：当前集成态及 A8/A9 缺失项均无空文件（A8/A9 产物未到，不计入）。
- **stale STOPPED 冒充 PASS**：各策略脚本 self-test/default 均真实 PASS（非 STOPPED 伪装）。
- **重复命令名**：`main.rs` 各 `#[tauri::command]` 与 `generate_handler!` 无 graph/plugin/agent/skill 新命令，无重复；ACL 末条恒为 `list_artifact_images`。
- **bridge/main/types 命令奇偶**：因无新命令，平凡一致；`bridge.ts`/`types.ts` 仅增 DTO 类型（展示用），与命令解耦。
- **docs NEXT 一致性**：board 头部 `Current NEXT: M5-W6` 与 §M5-W6 dispatch 一致。
- **lane scope 漂移**：W6 产品代码仅 A8（graph UI）/A9（plugin 策略），均未越界；A1~A7/A10 为 docs/review/support（如 `logs/assist/A4-M5-W6-memory-privacy-capacity-review-20260906-2237.md` 仅评审笔记）。无漂移。

---

## 6. 债务台账更新（相对 W5）

| 项 | W5 状态 | W6 状态 |
|---|---|---|
| 集成门禁 | FAIL（cargo_warnings 2→27） | **ALL_PASS**（红灯消解，4b438ef） |
| U-2（core seam） | CLOSED | 维持 CLOSED |
| U-4（capability 真源） | 已立项 | 维持 |
| A4/A5 W4 集成补验 | 结案 | 维持结案 |
| **cargo_warnings 2→27（A7 图谱契约 dead_code）** | 新增红灯 | **已结**（4b438ef 消解，回基线 2） |
| 构建指标阈值 | 16%（W5 抬升） | **19%**（A0 文档化 W5 债务；实测 18.58%，余量 0.42%） |
| check-agent-skill-ui-logic.mjs 未接入 pre-merge | 小缺口 | **仍缺口**（grep pre-merge.sh 确认未接；测试本身 57/57 PASS） |
| agent-skill 策略缺 `--expect-pending` 模式 | 不一致 | 维持（4 stale PENDING 码位） |
| MCP/agent-memory/graph `--expect-pending` | FAIL by design | 维持（待后续 wave 翻转 PENDING→ACTIVE） |
| **A8 W6 M5-9 graph UI 逻辑测试** | — | **PENDING（代码未到本仓）** |
| **A9 W6 M5-10/11 plugin manifest/lifecycle 策略** | — | **PENDING（代码未到本仓）** |
| 构建指标基线双文件 | 双基线 | 维持（建议归档旧基线） |

---

## 7. 声明（避免误读）

- 本车道**零产品代码改动**；W6 的 graph.rs / Agent/Skill UI 组件 / 策略脚本均为 A6/A7 既有交付（`4b438ef`/`f99d2eb`），非本车道写入。
- 未 rebase、未 push（board Merge Rule：仅 A0 推送）。
- 全部结论基于 §1 实跑证据；针对**含 W5 集成的当前工作树**复跑，未引用旧报告（遵守 IF-5 不 stale）。
- A8/A9 W6 产品代码未落入本仓，其门禁（§2）留待 A0 集成后由 A11 补验；本增量已为此预留验证位与硬停止核对项。
- 仅 `git add` 本文件，不带入他 lane 改动（含 A4 W6 review 笔记等 untracked 文件均不提交）。
