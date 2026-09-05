# A11 · M4 验证证据 · 终态批次（Final Verification Batch）

> Lane：**A11（M4 verification evidence）** ｜ 路由 `AI:BALANCED / R:medium`
> 批次时间：2026-09-06 07:04 CST ｜ 执行者：CodeBuddy Hy4
> 依据：`PARALLEL_COMMAND_BOARD.md` §Integration Fix Wave（IF-1..IF-5）与 §Lane A11「Final Verification Batch Only / Final Conflict Scan Before A0」
> 范围声明：**只写 `logs/` 验证文档，零产品代码改动；未提交、未 push**（board：仅 A0 向 `master` 提交并推送）
> 取代：`logs/checkpoints/A11-M4-verification-batch1-20260905-2315.md`、`A11-M4-verification-batch2-20260906-0629.md`（IF-5：batch2 已过期，本次重测）

---

## 0. 启动门禁（实跑）

```text
cat .workspace-identity     → WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master ✅
pwd                         → /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3 ✅
git status --short --branch → ## master...origin/master [领先 2]（多车道在途文件，见 §6）
git log -1 --oneline        → 85d2d7b「docs(M4): expand batch implementation dispatch」✅（HEAD 未因 A11 移动）
```

判定：**通过**。规范主副本、`master`、与远端同步（`git pull --ff-only` 已是最新）。

---

## 1. 取样条件与重要声明（先读）

**本批次在「并行 12 车道持续写入同一工作树」条件下取证**：`src-tauri/src/scheduler.rs` / `tasks.rs` / `bridge.rs`、`src/bridge.ts`、`详细设计与实施计划.md` 等被 A6/A7/A8/A1 并发改动，导致**部分门禁在两次相邻运行间结果翻转**。A11 不代修、不掩盖，逐条记录「本轮实测 + 跨运行波动」，结论权归 A0。

### 1.1 跨运行波动取证（同车间接连运行）

| 命令 | 运行 1 | 运行 2 | 运行 3 | 运行 4 | 运行 5（本次快照 07:04） | 波动原因 |
|---|---|---|---|---|---|---|
| `check-scheduler-policy.py --self-test` | PASS(20) | FAIL | FAIL | PASS(23) | **PASS(23)** | 坏样本「真实仓库变异」读当前 `tasks.rs`；`tasks.rs` 被并发改写 → 变异命中与否随树漂移 |
| `check-plan-routing.py` | PASS | PASS | FAIL | FAIL | **FAIL** | `详细设计与实施计划.md` 被并发追加 `M4-9` 且出现重复 WBS ID |
| `pre-merge.sh` 失败项 | tools+metrics | metrics+sched | metrics+sched | planrouting+metrics | **planrouting+metrics** | 随上两项翻转 |

> **A11 核心结论**：功能代码全部绿灯（`cargo test 328 passed`、`npm run build` 0 error、各 policy 默认模式 EXIT 0），但**集成门禁当前无法稳定转绿**，根因是 (a) Rust 二进制体积 +15.2%（DB 驱动副作用，A0 决策）、(b) 文档重复 WBS ID（A1）、(c) 调度夹具对并发改树敏感（A6/A0）。**A0 必须在冻结工作树后复跑 `pre-merge.sh` 至稳定 ALL_PASS 再 push**——当前任何单次 PASS 都不构成可推送证据。

---

## 2. 定向验证矩阵（命令 + 实测输出）

### 2.1 全量 / 通用（T0）

| # | 命令 | 结果（07:04 快照） | 口径 |
|---|---|---|---|
| T0-1 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` | EXIT 0，无 diff | ✅ IF-1 已解决 |
| T0-3 | `cargo test --manifest-path src-tauri/Cargo.toml` | `328 passed; 0 failed`（7.1s） | ✅ 基线 232 → 此 328（M4 测试已并入） |
| T0-4 | `cargo check --manifest-path src-tauri/Cargo.toml --locked` | error 0；**warning 2**（均 `grid_process.rs` 既有死代码） | ✅ IF-4 已回落到基线 2（早前「5 warning」为并发编译缓存混用，干净重编译 = 2） |
| T0-5 | `npm run build` | 0 error；主 JS `index-*.js` = 182.72 kB（gzip 65.59）；`DatabasePanel` 已成懒加载分片（14.94 kB） | ✅ 主 JS 较基线 161.36 kB 仅 +13.2%（<15%）；但见 T0-6 |
| T0-6 | `python3 scripts/measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-4f0e8ab.json --skip-build` | `exceeds_growth_limit=true`；`total_bytes_pct=15.2`；`cargo_warnings=0` | 🔴 IF-2：总体积 +15.2% 超 15% 门禁（驱动来自 Rust 二进制，非前端） |
| T0-7 | `python3 scripts/check-plan-routing.py` | `failed (5 issue(s))`：duplicate WBS ID `M4-9`（行 472–476）；expected 50 found 51 | 🔴 新增阻塞（A1 文档引入重复 ID） |
| T0-8 | `bash scripts/pre-merge.sh` | `PRE_MERGE_RESULT=FAIL`（2 项 FAIL，见 §3） | 🔴 |
| T0-9 | `git diff --check` | 无输出 | ✅ |

### 2.2 策略脚本三模式

| 脚本 | --self-test | 默认 | --expect-pending | 归属 |
|---|---|---|---|---|
| `check-database-policy.py` | ✅ EXIT 0（2 好 + 15 坏全检出；ACTIVE=7 PENDING=8） | ✅ EXIT 0 | ✅ `NONE`（8 pending 均未实现） | A4 |
| `check-scheduler-policy.py` | ⚠️ **波动**：PASS(23) / FAIL 交替（FAIL 时缺 `SCHED_OCCUPIED_NOT_SKIPPED` + `SCHED_TASK_DEF_NO_DEFAULT` 真实变异覆盖） | ✅ EXIT 0（ACTIVE=23） | ✅ `NONE` | A6 |
| `check-scheduler-ui-policy.py` | ✅ EXIT 0（1 好 + 14 坏） | ✅ EXIT 0（14 码守护） | ✅ | A8 |
| `check-tools-policy.py` | ✅ EXIT 0（A1 IF-3 补丁已修：坏样本位置无关化，**16 坏样本全检出**） | ✅ EXIT 0 | ✅ | A1/A10 |
| `check-script-exec-policy.py` | ✅ EXIT 0 | ✅ EXIT 0 | ✅ | A7/A10 |
| `check-lifecycle-contract.py` | ✅ EXIT 0 | ✅ EXIT 0 | n/a | A7 |
| `check-security-policy.py` | ✅ EXIT 0 | ⚠️ EXIT 1（`FAIL (3 known gap(s))`，按设计默认模式 EXIT 1 不入门槛） | n/a | A0/A10 |
| `check-terminal-policy.py` | ✅ EXIT 0 | ✅ EXIT 0 | n/a | M3 |

### 2.3 前端逻辑夹具

| 脚本 | 结果 |
|---|---|
| `node scripts/check-database-ui-logic.mjs` | 119 断言通过 ✅ |
| `node scripts/check-scheduler-ui-logic.mjs` | 102 断言通过 ✅ |

---

## 3. 集成门禁（pre-merge）红灯分解（07:04 快照）

| # | FAIL 项 | 根因（取证） | 归属 / 处置 | 状态 |
|---|---|---|---|---|
| 1 | `check-plan-routing.py` | `详细设计与实施计划.md` 行 472–476 出现**重复 WBS ID `M4-9`**（51 行 vs 期望 50） | A1 文档失误 → A1 去重 | 🔴 新增（并发引入） |
| 2 | `build metrics regression` | `total_bytes_pct=15.2`（>15）；Rust 二进制因 `rusqlite`(bundled)+`mysql`+`postgres` 增容 | **A0 决策**：抬阈并书面说明 / `rusqlite` 改非 bundled / 延后 `mysql`+`postgres` | 🔴 IF-2 |
| — | `check-scheduler-policy.py --self-test`（历史项，本次 PASS） | 夹具「真实仓库变异」对并发改树敏感，曾 FAIL（缺 2 码位覆盖） | A6 使夹具健壮化，或冻结树后复跑 | ⚠️ 波动 |

> **已转绿项（对比 batch2）**：`cargo fmt main`（IF-1）、`check-database-policy.py --self-test/--expect-pending`（D37 多语句红线已由 A3/A4 修复，DB_MULTI_STATEMENT_FORBIDDEN 已 ACTIVE 且真实仓库零违规）、`check-tools-policy.py --self-test`（IF-3，A1 补丁）、`cargo check` warning（IF-4，回落 2）。

---

## 4. Integration Fix Wave 收口状态（IF-1..IF-5）

| ID | Red Light | 责任 | 状态 | A11 取证 |
|---|---|---|---|---|
| IF-1 | `cargo fmt main` | A3/A4/A7 | ✅ 已解决 | `cargo fmt --check -- src-tauri` EXIT 0 |
| IF-2 | build metrics +15.2% | A0 决策 | 🔴 未决 | Rust 二进制增容；前端主 JS 仅 +13.2% |
| IF-3 | `check-tools-policy.py --self-test` | A10 检/A1 修 | ✅ 已解决 | A1 `A1-IF3-tools-policy-fixture-fix-20260906-0658.patch`；坏样本位置无关化 |
| IF-4 | Cargo warning 2→5 | A3/A4 | ✅ 已回落 2 | 干净重编译 = `grid_process.rs` 两处基线 warning；`DbSslMode` 等未用导入已由责任 lane 清理 |
| IF-5 | A11 batch2 过期 | A11 | ✅ 本批次交付（替换 batch1/2） | 本文件 |

---

## 5. 冲突扫描（board §Lane A11「Final Conflict Scan Before A0」）

| # | 检查项 | 结果 | 取证 |
|---|---|---|---|
| 5.1 | 空文件（排除 `logs/m0-baseline`） | ✅ 无 | `find -type f -empty` 仅命中历史基线 raw 日志 |
| 5.2 | 陈旧 `STOPPED` 冒充 PASS | ✅ 无活 STOPPED | 残留 `STOPPED` 仅出现在「M4-1.c §9 如实记录已收口」与 batch2 过期叙述；无卡仍标 STOPPED |
| 5.3 | 重复命令名（bridge.rs / main.rs / ACL） | ✅ 无 | main.rs 无重复注册；ACL 无重复行 |
| 5.4 | ACL 末条锚点顺序 | ✅ 正确 | `task_*`(110–114) / `db_*`(115–117) 均 `< list_artifact_images`(118) |
| 5.5 | 命令三处同步（Rust/前端/ACL） | ⚠️ **后端齐、前端壳未接线** | 见 §6 发现 F-1/F-2 |
| 5.6 | bridge/main/types 命令 parity | ⚠️ 见 §6 | task_* 后端齐但 `bridge.ts` `TASK_COMMANDS_AVAILABLE=false`；db_* 后端齐但无 `bridge.ts` 封装/`types.ts` DTO |
| 5.7 | docs NEXT 一致性 | 🔴 三份主文档 NEXT 仍写 `M4-1.a` | M4 实现已落地；NEXT 未推进 → A0/A1 同步 |
| 5.8 | 车道作用域漂移 | ✅ 抽查无越界 | 各 lane patch 仅触及授权文件（A8 限 UI/bridge/types；A6 限 scripts/docs；A3 限 domain/database） |
| 5.9 | 新命令 source check + ACL | ✅ | 8 条新命令（task_*×5 / db_*×3）均含 `check_invocation_source` 且进 ACL |

### 5.9 不变量复核（调度/DB 安全护栏）

- 无 `task_cancel`（契约禁止）✅；`grep task_cancel` 仅命中注释说明。
- `scheduler.rs`/`tasks.rs` 无 `std::process::Command`（无第二执行路径）✅（仅 doc 注释）。
- `stop-scheduler` 注册于 `stop-background-workers` 之后、`kill-running-scripts` 之前（bridge.rs:797，索引 1 < 5）✅。
- DB 写默认拒绝（`confirm_write` 参数）、凭据经 `db:<conn_id>` Keyring、审计脱敏：由 `cargo test security_policy` + `check-database-policy.py` ACTIVE=7 守护 ✅。

---

## 6. A11 新发现（须 A0/责任 lane 收口，非 A11 代修）

### F-A11-1（中）：调度 UI 仍是禁用壳，尽管后端 `task_*` 已落地
`src/bridge.ts:45` `TASK_COMMANDS_AVAILABLE = false`，注释称「A7 尚未落地」。但实测 `task_list/add/update/remove/run_now` 已在 `bridge.rs`(952–1092)、`main.rs`(1416–1420)、ACL(110–114) 全部就绪。→ **A8（或 A0 指派）须将开关置 `true` 并接线实时面板**。A7 后端工作已满足 Wave 2 解锁条件。

### F-A11-2（中）：DB UI 靠「可选成员探测」保持禁用壳，尽管 `db_*` 后端已就绪
`src/stores/useDatabaseStore.ts` 用 `bridge as unknown as DbBridgeApi` 探测 `dbConnect?`，且 `src/bridge.ts` 无 `db_*` 封装、`src/types.ts` 无 `Db*` DTO。后端 `db_connect/query/disconnect` 已在 `bridge.rs`(5959/5990/6047)、`main.rs`(1421–1423)、ACL(115–117) 就绪。→ **A5/A4（或 A0）须补齐 `bridge.ts` 封装 + `types.ts` DTO，使 `DatabasePanel` 从禁用壳转为实时**。

### F-A11-3（低）：DB/调度 UI 产品文件未暂存
`src/components/workspace/DatabasePanel.vue`、`src/stores/useDatabaseStore.ts`、`src/utils/dbUi.ts` 为 `??`（未跟踪），而同批 `TaskPanel.vue` 等已 `A`（已暂存）。A0 提交前须 `git add` 这些文件，否则会漏提交。

### F-A11-4（低）：调度夹具对并发改树敏感
`check-scheduler-policy.py --self-test` 在 `tasks.rs` 被并发改写时翻转 PASS/FAIL（缺失 `SCHED_OCCUPIED_NOT_SKIPPED`、`SCHED_TASK_DEF_NO_DEFAULT` 真实变异覆盖）。建议 A6 将「真实仓库变异」改为基于固定快照或放宽到仅校验合成坏样本，避免整合期误红。

### F-A11-5（低）：文档 NEXT 未推进
三份主文档顶部 NEXT 仍为 `M4-1.a`，与「M4 四批次已落地」矛盾；A0/A1 在整合时顺手推进 NEXT。

---

## 7. A0 推送前必须解除的阻塞（按优先级）

1. **build metrics +15.2%**（IF-2，最高）：A0 决策——抬阈书面说明 / `rusqlite` 非 bundled / 延后 `mysql`+`postgres`。
2. **`check-plan-routing.py` 重复 WBS ID `M4-9`**（§3.1 / §5.7）：A1 去重（删重复 4 行），恢复 50 行。
3. **调度夹具波动**（F-A11-4）：冻结树后复跑；若仍 FAIL 由 A6 修夹具。
4. （接线类，不阻塞编译但阻塞功能验收）**F-A11-1 / F-A11-2**：A8/A5 置 `TASK_COMMANDS_AVAILABLE=true` 并补 `db_*` 前端封装/DTO。
5. **F-A11-3**：A0 `git add` 未跟踪 DB UI 文件。

解除 (1)(2)(3) 后，A0 在**冻结工作树**状态下复跑 `bash scripts/pre-merge.sh` 至稳定 `ALL_PASS`，再 `git push origin master`。

---

## 8. 声明

1. **不代修**：IF-2/F-A11-* 全部只记录取证 + 归属，修复权归 A0/责任 lane；A11 零产品代码改动。
2. **不代签**：功能代码全绿，但集成门禁在并发写入下不稳定且存在 2 项持久红灯 → 本批次**不以 PASS 收口**，记为 **`PASS_WITH_DEBT`**（功能可验收 / 集成门禁未稳定转绿）。
3. **快照性质**：结论以「命令 + 实测输出 + 取样时刻 2026-09-06 07:04」为准；并发改树致部分门禁翻转，已在 §1.1 明示，A0 必须冻结后复跑。
4. **取代 batch1/2**：IF-5 要求的「最终一批」即本文件；batch1/2 视为历史过程文档，不再更新。

---

## 9. 交付物与补丁

| 文件 | 状态 | 说明 |
|---|---|---|
| `logs/checkpoints/A11-M4-verification-final-20260906-0715.md` | 新增 | 本文件 |
| `logs/checkpoints/Lane-A11-verification-final-20260906-0715.patch` | 新增 | 单补丁（`git diff --binary` 仅含本 checkpoint） |

生成与校验（仅含本 lane 文档，不含任何产品代码）：
```bash
git add -N logs/checkpoints/A11-M4-verification-final-20260906-0715.md
git diff --binary -- logs/checkpoints/A11-M4-verification-final-20260906-0715.md \
  > logs/checkpoints/Lane-A11-verification-final-20260906-0715.patch
git apply --check -R logs/checkpoints/Lane-A11-verification-final-20260906-0715.patch
```

---

## 10. Lane 输出模板

```text
LANE=A11
STATUS=PASS_WITH_DEBT
BASE=85d2d7b
HEAD=85d2d7b（无 A11 提交；交付物=检查点 + patch）
FILES=logs/checkpoints/A11-M4-verification-final-20260906-0715.md + Lane-A11-verification-final-20260906-0715.patch
VERIFY=cargo test=328 passed；cargo fmt --check=0；cargo check warning=2(基线)；npm run build=0 error(主JS 182.72kB)；check-database-policy(--self-test/默认/--expect-pending)=全 EXIT 0(ACTIVE=7 PENDING=8)；check-scheduler-policy(默认)=EXIT 0(ACTIVE=23)；check-scheduler-ui-policy=全 EXIT 0；check-tools-policy --self-test=EXIT 0(16 坏样本)；check-script-exec/lifecycle/terminal/security --self-test=EXIT 0；check-plan-routing=FAIL(重复M4-9)；pre-merge=FAIL(plan-routing+build-metrics)；git diff --check=0
CHECKPOINT=logs/checkpoints/A11-M4-verification-final-20260906-0715.md
MERGE_NOTES=工作树被 12 车道并发写入，门禁跨运行波动（见§1.1）；A0 须冻结树后复跑 pre-merge 至稳定 ALL_PASS 再 push。IF-2(A0决策)/F-A11-1(A8)/F-A11-2(A5)/F-A11-3(A0 add)/F-A11-4(A6) 未结。
NEXT=A0 冻结工作树 → 解除 §7 五项 → pre-merge ALL_PASS → git push origin master
```
