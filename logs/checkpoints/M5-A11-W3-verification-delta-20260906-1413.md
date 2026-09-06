# M5-A11 · W3 验证增量（Verification Delta After W3 Outputs）

```text
LANE=A11
STATUS=PASS（验证；集成门禁 ALL_PASS；2 项非阻断发现）
BASE=98a3b01（W3 dispatch HEAD；本地 master 与 origin/master 已同步）
HEAD=logs/checkpoints/M5-A11-W3-verification-delta-20260906-1413.md
FILES=logs/checkpoints/M5-A11-W3-verification-delta-20260906-1413.md
VERIFY=见 §1 全门表（实跑，非 stale）
CHECKPOINT=本文件
MERGE_NOTES=见 §4（发现① seam U-2 未交付→M5-3.a 仍阻塞；发现② check-mcp-policy.py 未接入 pre-merge.sh）
NEXT=待 A0 翻板至 M5-2/M5-3 实质实现后，A11 再出验证增量；建议 A0 将 check-mcp-policy 接入 pre-merge
```

> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W3 Parallel Dispatch（行 133-168）→ **A11 = START VERIFICATION：「One verification delta after W3 outputs」**；scope `logs/assist/M5-A11-*.md`、`logs/checkpoints/M5-A11-*.md`。
> 范围声明：本增量**只产出验证文档，零产品代码、零策略脚本改动**（W3 的产品/脚本改动属 A2/A3，已由 A0 集成于 `712a14c`/`a654f0c`）。未 push。
> 姊妹件：W1 `M5-A11-W1-verification-delta-20260906-0830.md`、W2 `M5-A11-W2-verification-baseline-20260906-1342.md`。

---

## 0. 启动门禁与调度匹配

```bash
cat .workspace-identity              # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master  ✅
pwd                                  # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git fetch origin && git pull --ff-only   # 已是最新（本地==origin/master，领先差额已随 A0 推送清零）
git status --short --branch          # ## master...origin/master；仅 M5-0-overview.md 一处脏（A1/A0 板文档，非本车道）
git log --oneline -5                 # 98a3b01 / 712a14c / a654f0c / c4b0fb7 / 1e114b6
```
- **调度匹配**：board 头部 `Current NEXT: M5-W3 parallel implementation` 与本增量一致；A2 拥有 M5-1.b 核心抽取、A3 拥有 M5-2 MCP 命令注册/策略壳，其余 lane 仅文档/复审/支撑。本车道按 W3 A11 行交付单一验证增量。
- **工作树**：本车道 W2 提交 `1e114b6` 已随 A0 推送（领先清零）；当前仅 `M5-0-overview.md` 一处脏，属板文档不属于本车道，按 Merge Rule 不动。

---

## 1. W3 验证矩阵（全门实跑，非 stale）

| 门 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | **329 passed；0 failed** | ✅ |
| core 边界门 self-test | `python3 scripts/check-core-boundary.py --self-test` | `CORE_POLICY_SELF_TEST=PASS`（ACTIVE=7，坏样本=9） | ✅ |
| core 边界门 default | `python3 scripts/check-core-boundary.py` | `all invariants hold`（ACTIVE=7，core 文件=2） | ✅ |
| core 边界门 pending | `python3 scripts/check-core-boundary.py --expect-pending` | `CORE_PENDING_RESULT=NONE` | ✅ |
| M5-2 MCP 门 self-test | `python3 scripts/check-mcp-policy.py --self-test` | `MCP_POLICY_SELF_TEST=PASS`（ACTIVE=4，PENDING=9） | ✅ |
| M5-2 MCP 门 default | `python3 scripts/check-mcp-policy.py` | `MCP_POLICY=PASS`（无违规，EXIT 0） | ✅ |
| M5-2 MCP 门 pending | `python3 scripts/check-mcp-policy.py --expect-pending` | `MCP_PENDING_RESULT=NONE`（9 pending 未实现，W1 守门通过） | ✅ |
| M2-4 执行通道 self-test | `python3 scripts/check-script-exec-policy.py --self-test` | `SELF_TEST_RESULT=ALL_PASS`（23 坏样本+1 好样本+码位完整性） | ✅ |
| tools 策略 self-test | `python3 scripts/check-tools-policy.py --self-test` | `self-test OK`（好样本零违规+16 坏样本全检） | ✅ |
| db 策略 self-test | `python3 scripts/check-database-policy.py --self-test` | `DB_SELF_TEST_RESULT=PASS`（ACTIVE=14，PENDING=1） | ✅ |
| db 策略 default | `python3 scripts/check-database-policy.py` | `all invariants hold`（ACTIVE=14） | ✅ |
| db 策略 pending | `python3 scripts/check-database-policy.py --expect-pending` | `DB_PENDING_RESULT=NONE`（1 pending） | ✅ |
| scheduler 策略 self-test | `python3 scripts/check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS`（ACTIVE=23，PENDING=0） | ✅ |
| scheduler 策略 default | `python3 scripts/check-scheduler-policy.py` | `all invariants hold`（ACTIVE=23） | ✅ |
| Rust fmt | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | `FMT_CLEAN` | ✅ |
| 工作树 diff | `git diff --check` | `DIFF_CHECK_CLEAN` | ✅ |
| 前端构建 | `npm run build` | ✓ built；`index-*.js`=162.50 kB（gzip 58.28 kB），低于 IF-2 ~197 kB 红线 | ✅ |
| **集成门禁** | `bash scripts/pre-merge.sh` | **`PRE_MERGE_RESULT=ALL_PASS`（EXIT=0）** | ✅ |

---

## 2. A2 W3 验证（切片 0b「契约常量收口」，commit `712a14c`）

**交付内容**（据 A2 checkpoint `Lane-A2-M5-1-0b-constants-20260906-1349.md`）：把 `HARD_GRACE_SECS`/`MAX_TIMEOUT_SECS`/`MAX_TEXT_FIELD_BYTES`/`DB_*` 收口到 `domain.rs`，原 `script_runner.rs`/`security_policy.rs`/`database.rs` 改为 `pub use` re-export；**连带修复 M2-4 执行通道门禁**（`check-script-exec-policy.py` 接受常量位于 `domain.rs` 或 `script_runner` re-export）。

| 验收项（W3 A2 硬停止） | 验证结果 |
|---|---|
| 边界门不受影响 | ✅ core 文件仍为 2，`check-core-boundary.py` 三模式全 PASS |
| 行为零变化 | ✅ `cargo test` 329，与 M5-1.a 终态逐条对齐 |
| **W2 发现 B 已修复** | ✅ `check-script-exec-policy.py --self-test` → `ALL_PASS`（A2 0b 将常量收口到 domain.rs 后适配了检测 + DOMAIN_MUTATIONS 变异防呆） |
| **IF-1 已修复** | ✅ `cargo fmt --check` 干净（A2 0b 顺带格式化了 database.rs/domain.rs） |
| 无新依赖 | ✅ `Cargo.toml`/`Cargo.lock` 未变 |
| 无命令/ACL/UI | ✅ 改动仅 bin crate 的 `domain.rs`/`security_policy.rs`/`script_runner.rs`/`database.rs` + 策略脚本，未碰 `src/core/`、`main.rs`、`bridge.rs`、ACL、`src/*` |
| 范围克制 | ✅ 仅 0b 常量收口，**未做** scheduler/database 行为抽取 |

> **发现①（非阻断）**：U-2 seam（trait/Clock+AuditSink 注入）**未在本切片交付**。`core/` 仍仅 `keyring_store.rs`+`mod.rs`，无 `seam.rs`；A2 checkpoint 的 NEXT 明确切片 1·2（M5-1.b 抽 trait 解 scheduler 反向边）才是 seam。→ **M5-3.a（agent_kv 纯存储）仍被 U-2 阻塞**，需待 A2 后续 seam 切片。

---

## 3. A3 W3 验证（M5-2 MCP 策略门，commit `a654f0c`）

**交付内容**：新增 `scripts/check-mcp-policy.py`（504 行，静态准入门禁，**非运行时代码**）；文档 `logs/assist/A3-M5-mcp-20260906-0757.md`（+ reconcile / W1-delta）。遵循 A2 `check-core-boundary.py` 范式：「产物存在才判」——M5-2 代码未落地时 PENDING 码位 no-op，默认扫描 EXIT 0。

| 验收项（W3 A3 硬停止） | 验证结果 |
|---|---|
| 策略门三模式 | ✅ self-test `PASS`（ACTIVE=4，PENDING=9）/ default `PASS`（EXIT 0）/ `--expect-pending` `NONE`（9 pending 未实现，W1 守门通过） |
| 无网络服务/后台监听 | ✅ 纯静态扫描脚本，零运行时 |
| 无 rmcp / tokio / npm 依赖 | ✅ 未改 `Cargo.toml`/`package.json` |
| 无新 Tauri 命令 / ACL | ✅ 未碰 `main.rs`/ACL/`bridge.rs`（§4 冲突扫描确认） |
| 复用 security_policy / 能力白名单单一真源 | ✅ 脚本设计对齐 `security_policy.rs`，无第二份能力定义 |

> **发现②（非阻断，建议 A0 收口）**：`check-mcp-policy.py` **未接入 `scripts/pre-merge.sh`**（`grep -nE 'mcp' pre-merge.sh` 无命中）。W3 分发要求 A3「pre-merge hook」，但当前该门禁仅 opt-in。建议 A0 在 `pre-merge.sh` 增加与 `check-core-boundary.py`/`check-script-exec-policy.py` 同范式的调用段，使 M5-2 实现期（PENDING→ACTIVE）能自动守门。

---

## 4. 集成门禁 + Before-A0 冲突扫描

- **集成门禁**：`PRE_MERGE_RESULT=ALL_PASS`（相对 W2 baseline 的 `FAIL` 已翻红为绿）。W2 两红灯（IF-1 fmt、script-exec 漂移）**均由 A2 0b 切片修复**——W2 发现 B 在此显式关闭。
- **命令表面扫描**：`git diff --name-only c4b0fb7 712a14c -- main.rs / default-commands.toml / bridge.rs / bridge.ts / types.ts` = **空** → W3 无新 Tauri 命令、无 ACL 顺序问题、无 bridge/main/types 命令奇偶。
- **空文件扫描**：W3 两提交范围无空文件。
- **NEXT 一致性**：board 头部 `Current NEXT: M5-W3` 与 dispatch 小节一致；A1 卡片 `M5-0-overview.md` 已标记 W3 活跃（脏文件属板维护，非阻断）。
- **lane scope 漂移**：W3 产品/脚本改动仅限 A2/A3 授权范围（bin crate 常量 + MCP 策略脚本），无越界。

---

## 5. 相对 W2 baseline 的状态变化

| 项 | W2（13:42） | W3（14:13） |
|---|---|---|
| `pre-merge.sh` | **FAIL**（IF-1 + script-exec 漂移） | **ALL_PASS** |
| W2 发现 B（script-exec 漂移） | 待 A0/A10 修 | **已修**（A2 0b 适配 `domain.rs` 常量） |
| IF-1（cargo fmt） | 脏 | **已清**（A2 0b 格式化） |
| M5-2 MCP 门禁 | 不存在 | 新增 `check-mcp-policy.py`（opt-in，未接 pre-merge） |
| 边界门 / cargo test | PASS / 329 | PASS / 329（不变） |
| seam（U-2） | 未交付 | 仍**未交付**（M5-3.a 仍阻塞） |

---

## 6. 声明（避免误读）

- 本车道**零产品代码、零策略脚本改动**（W3 的 `domain.rs` 等 + `check-script-exec-policy.py` 适配属 A2；`check-mcp-policy.py` 属 A3；均由 A0 集成）。本增量仅新增验证文档。
- 未 rebase、未 push（board Merge Rule：仅 A0 推送）。
- 全部结论基于 §1 实跑证据；未引用旧报告（遵守 IF-5 不 stale 要求）。
- 提交动作：仅 `git add` 本文件后提交，不带入他 lane 脏文件（`M5-0-overview.md`），不 push。
