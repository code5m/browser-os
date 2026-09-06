# M5-A11 · W1 验证增量（Verification Delta after A2 / A10）

> Lane：**A11（M5 verification）** ｜ 路由 `AI:BALANCED / R:medium`
> 时间：2026-09-06 ~08:30 CST ｜ 作者：CodeBuddy Hy4
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W1 Implementation Dispatch → A11：**START VERIFICATION**「Update M5 verification matrix with A2's actual boundary gate, commands, and pre-merge result. One verification delta after A2/A10.」
> 范围声明：本增量**只产出验证日志，零产品代码改动**。不新增 Cargo/npm 依赖、不新增命令/ACL/UI 面板（严守 §M5-W1 Hard Stops「Only A2 may touch product code in W1」）。`git push` 归 A0，本车道不推送。
> 本文件即 W1 指名的「**One verification delta after A2/A10**」。姊妹件：`logs/assist/M5-A11-W1-boundary-gate-checklist-20260906-0830.md`（给 A2/A0 的挂载清单 + 复跑命令块）。
> 对齐：`logs/checkpoints/M5-A11-verification-matrix-20260906-0820.md`（08:20 前瞻矩阵；本增量修正其中已被实跑证伪的陈旧断言，见 §2 / §7）。

---

## 0. 启动门禁实跑（WORKSPACE_IDENTITY.md 要求）

```bash
cat .workspace-identity            # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master  ✅
pwd                                # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git status --short --branch        # ## master...origin/master [领先 2]；工作树含本 lane 之外多 lane 的 W1 增量（见 §6 注释）  ✅
git log --oneline -2              # 404f514 docs(M5): integrate prework and dispatch core boundary wave / 5ca8f9f ...
```

判定：**通过**（目录 = 规范主副本、分支 = `master`）。工作树同时含 A1/A2/A3/A4/A5/A6/A8/A9/A10 的 W1 增量（未提交），本车道只新增本文件，不触碰任何高冲突文件。

> 并发提示：本工作树在验证期间被多 lane 增量实时改写（`main.rs` / `Cargo.toml` 在两次采样间从「未完成」变「已完成」）。本增量所有结论均取自**最后一次一致快照**（边界脚本实跑 PASS 之后），并已通过 `cargo test 329` / `cargo check` 复核行为不变。

---

## 1. A2 W1「实际边界门」实跑验证（W1 核心交付）

### 1.1 门禁脚本存在性与三模式

`scripts/check-core-boundary.py` 已落地（未提交，`??` 状态）。实跑：

```bash
$ python3 scripts/check-core-boundary.py --self-test
CORE_POLICY_SELF_TEST=PASS（ACTIVE=7，坏样本=9，含注释/字符串阴性样本）
$ python3 scripts/check-core-boundary.py
core boundary policy: all invariants hold（ACTIVE=7，core 文件=2）
$ python3 scripts/check-core-boundary.py --expect-pending
CORE_PENDING_RESULT=NONE（2 个 pending 码位均未实现，阶段一同 package 双 target 下预期如此）
```

| 模式 | 期望 | 实测 | 判定 |
|---|---|---|---|
| `--self-test` | EXIT 0，好样本零误报 + 坏样本全检 + 注释/字符串阴性 | PASS（ACTIVE=7，9 坏样本含变异防呆，注释样本不误报） | ✅ |
| 默认扫描 | EXIT 0，真实仓库零违规 | `all invariants hold`（core 文件=2） | ✅ |
| `--expect-pending` | pending 码位当前不可判 → NONE | NONE | ✅ |

**脚本质量**：7 个 ACTIVE 码位（`CORE_TAURI_IMPORT` / `CORE_APP_HANDLE` / `CORE_BRIDGE_REF` / `CORE_SECOND_EXEC_PATH` / `CORE_LIB_UNDECLARED` / `CORE_SHIM_CONFLICT` / `CORE_EMPTY_PLACEHOLDER`）+ 2 个 PENDING（`CORE_TREE_TAURI` / `CORE_DEP_NOT_ALLOWLISTED`，阶段二独立 crate 后转 ACTIVE）。词法剥离字符串与 `//`/`///` 注释，规避 A2 v3 §1.1 已实测的「注释里解释『不依赖 AppHandle』导致误报」坑。设计符合 A0 W1 裁定（R8：编译层守不住 core⇸tauri，脚本是唯一防线）。

### 1.2 产品代码集成完整性（A2 W1 指派）

| 产物 | 期望 | 实测 | 判定 |
|---|---|---|---|
| `src-tauri/Cargo.toml` | 仅增 `[lib] name="mvp_core" path="src/lib.rs"`，**不增 `[dependencies]`** | `M src-tauri/Cargo.toml`：`[lib] name="mvp_core" path="src/lib.rs"`（已加） | ✅ |
| `src-tauri/src/lib.rs` | `pub mod core; pub use crate::core::*;` | 已落地（未提交） | ✅ |
| `src-tauri/src/core/{mod.rs,keyring_store.rs}` | 纯逻辑收口，零 tauri / 零 `crate::bridge` / 零第二执行路径 | 已落地（keyring_store 经 `git mv` 从 `src/keyring_store.rs` 迁入） | ✅ |
| `src-tauri/src/main.rs` | 删 `mod keyring_store;`，改 `pub use mvp_core::keyring_store;` shim | `M src-tauri/src/main.rs`：已删 `mod keyring_store;`，加 `pub use mvp_core::keyring_store;` | ✅ |

`git status` 关键行：
```
 M src-tauri/Cargo.toml
R  src-tauri/src/keyring_store.rs -> src-tauri/src/core/keyring_store.rs
 M src-tauri/src/main.rs
?? scripts/check-core-boundary.py
?? src-tauri/src/core/mod.rs
?? src-tauri/src/lib.rs
```

### 1.3 行为不变（M5-1.a「不搬行为」硬指标）

```bash
$ cargo test --manifest-path src-tauri/Cargo.toml     # → test result: ok. 329 passed; 0 failed  （与搬迁前基线完全一致）
$ cargo check --manifest-path src-tauri/Cargo.toml    # → Finished，仅 2 个既有 grid_process.rs dead_code 警告（非 W1 引入）
$ cargo fmt --manifest-path src-tauri/Cargo.toml -- --check   # → EXIT 0（W1 新代码已格式化，IF-1 未被 W1 新引）
```

**判定**：模块搬迁 + re-export shim **零行为回归**，测试总数 329 不减不增，满足 M4 终态基线。

### 1.4 W1 硬停止守门（零越界）

| W1 Hard Stop | 检查 | 结果 |
|---|---|---|
| 仅 A2 可碰产品代码 | A11 未改任何 `.rs`/`Cargo.toml`/`pre-merge.sh` | ✅（本车道零产品代码） |
| 无新 Cargo/npm 依赖 | `git diff src-tauri/Cargo.toml` 仅 `[lib]` 段，无 `[dependencies]` 新增行 | ✅ |
| 无新 Tauri 命令 / ACL | `bridge.rs` / `main.rs` / `default-commands.toml` diff 仅含 re-export shim，无 `generate_handler!` 增项、无 ACL 新增行 | ✅ |
| core 不得引 tauri / AppHandle / `crate::bridge` / 第二执行路径 | 见 §2 五类泄漏复核 | ✅ |

---

## 2. 五类泄漏代码层复核（逐条执行 A10 §4，A10 原 BLOCKED 现可解）

A10 `A10-M5-W1-security-review-20260906-1500.md` 设计层 PASS，代码层原 `BLOCKED`（彼时 A2 补丁未落盘）。本 A11 实跑其 §4 全部命令，结果如下：

| 维度 | A10 §4 命令（节选） | 实测 | 判定 |
|---|---|---|---|
| **L1** core 引 tauri | `grep -rE 'use tauri|AppHandle|AppState' src-tauri/src/core \| grep -v '//\|///'` | 空（core 内仅 `use keyring::...`，注释已剥离） | ✅ PASS |
| **L2** 第二执行路径 | `grep -rEn 'std::process::Command|Command::new|sh -c|bash -c' src-tauri/src/core` | 空 | ✅ PASS |
| **L3** 隐藏依赖 | `git diff src-tauri/Cargo.toml` 仅 `[lib]` 段 | 无 `+[dependencies]` / 无 `+<dep> =` 行 | ✅ PASS |
| **L4** 命令/ACL 漂移 | `git diff bridge.rs main.rs default-commands.toml` | 仅 `pub use mvp_core::keyring_store;` shim，无命令/ACL 新增 | ✅ PASS |
| **L5** 策略误报 | `python3 scripts/check-core-boundary.py --self-test` + 注释阴性样本 | self-test PASS，含 `// use tauri` 类注释阴性样本（不误报） | ✅ PASS |

**结论**：A2 W1 边界补丁**代码层五类泄漏全部 PASS**。建议 A10 将其 `STATUS` 由 `BLOCKED` 翻为 `PASS` 并回填 `MERGE_NOTES`（A11 不代改 A10 文件，留待 A10/A0）。

---

## 3. 基线机器门禁实跑（刷新，回应 IF-5 防 stale）

> A0 在 board §Integration Fix Wave 点名「A11 旧报告 stale」。本增量以**当前实跑**为准，证伪 08:20 矩阵中的三处陈旧断言。

| 项 | 命令 | 08:20 矩阵陈旧断言 | 本次实跑（2026-09-06 ~08:30） | 判定 |
|---|---|---|---|---|
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | 328 passed（IF-5 口径） | **329 passed；0 failed** | ✅ 修正 |
| 集成门禁 | `bash scripts/pre-merge.sh` | FAIL（IF-1~IF-4） | **ALL_PASS**（但⚠️ 未含边界门，见 §4） | ✅ 修正 |
| tools 策略自检 | `python3 scripts/check-tools-policy.py --self-test` | FAIL（IF-3） | **OK（EXIT 0，好样本零违规 + 16 坏样本全检）** | ✅ 修正 |
| WBS 路由 | `python3 scripts/check-plan-routing.py` | ok (50 WBS rows) | ok (50 WBS rows) | ✅ 一致 |
| 新增代码 fmt | `cargo fmt --check`（src-tauri） | IF-1 对 M4 失败 | 新 core 代码 EXIT 0（W1 未引新 fmt 问题） | ✅ |

> 说明：`pre-merge.sh` 现报 `ALL_PASS` 是因为其**尚未挂载**边界门（§4 缺口）。一旦 §4 挂载完成，其结论将额外覆盖 core 边界，届时 `ALL_PASS` 才是含边界的强结论。

---

## 4. 唯一开放项：pre-merge.sh 未挂载 check-core-boundary.py

```bash
$ grep -nE 'check-core-boundary|boundary' scripts/pre-merge.sh
（无命中）
```

- **事实**：A2 W1 指派含「wire it into `scripts/pre-merge.sh`」；A10 unblock 条件 #2 亦要求。但 `scripts/pre-merge.sh` **当前未调用** `check-core-boundary.py`（既无 `--self-test` 也无默认挂载位）。
- **影响**：边界门脚本已存在且实跑 PASS，但**尚未在合并门禁中强制**——`bash scripts/pre-merge.sh` 现返回 `ALL_PASS` 却未验证 core 边界。W1 「boundary-first」裁定在 CI 层未真正生效。
- **归属**：属 A2 产品代码/CI 职责（W1 硬停止仅 A2 可碰产品代码与 pre-merge 挂载），**A11 不代改**。
- **建议挂载片段**（供 A2/A0 直接采用，详见姊妹件 `M5-A11-W1-boundary-gate-checklist-20260906-0830.md`）：在 `git diff --check` 之前插入：

```bash
# M5-W1 core boundary gate（A0 boundary-first；R8：编译器守不住，脚本是唯一防线）
echo "[pre-merge] M5-W1 core boundary gate…"
python3 scripts/check-core-boundary.py --self-test || { echo "CORE_BOUNDARY_SELFTEST_FAIL"; exit 1; }
python3 scripts/check-core-boundary.py || { echo "CORE_BOUNDARY_FAIL"; exit 1; }
```

---

## 5. 结论与裁定

| 维度 | 状态 |
|---|---|
| A2 W1 边界门脚本 | ✅ 落地，`--self-test` / 默认 / `--expect-pending` 三模式全 PASS |
| A2 W1 产品代码集成 | ✅ 完整可编译（`[lib]` + `lib.rs` + `core/` + `main.rs` shim + `git mv`） |
| 行为不变 | ✅ `cargo test` 329 passed（零回归）、`cargo check` 仅既有 2 警告、`fmt --check` 干净 |
| W1 硬停止（零新依赖 / 零命令·ACL / 仅 A2 碰代码） | ✅ 全满足 |
| 五类泄漏代码层（A10 L1~L5） | ✅ 全 PASS |
| 基线机器门（刷新，IF-5 防 stale） | ✅ pre-merge ALL_PASS / 329 / check-tools OK（修正 08:20 陈旧） |
| **pre-merge.sh 挂载边界门** | ⚠️ **OPEN**（唯一缺口，归 A2/A0，A11 不代改） |

**W1 裁定**：A2 的 M5-1.a 边界切片**代码层验证 PASS、行为零回归**，可作为 M5-2（MCP）等后续 lane 的结构前提（呼应 A10 §6 D-5/D-6/D-7）。**在 A2 将 `check-core-boundary.py` 挂载进 `pre-merge.sh` 之前，W1 不视为完全「boundary-first 强制」**——该挂载是 A0 签署下一实现 dispatch 前的唯一 blocking 项。

---

## 6. 补充验证（post-commit 快照 ~08:35）：pre-merge 挂载已落地，全门 ALL_PASS

> 提交本增量后，`scripts/pre-merge.sh` 被 A2 挂载了边界门（`git diff` 显示新增 `# 24. M5-1.a core 边界不变量夹具` 与三段 `check-core-boundary.py` 调用，含 `--self-test` / 默认 / `--expect-pending`）。本段为追加验证，确认 W1 唯一开放项已关闭。

```bash
$ grep -nE 'check-core-boundary|CORE_BOUNDARY' scripts/pre-merge.sh
404:  python3 "$SCRIPT_DIR/check-core-boundary.py" --self-test ... || pm_fail "check-core-boundary.py --self-test"
406:  python3 "$SCRIPT_DIR/check-core-boundary.py" ... || pm_fail "check-core-boundary.py（core 边界被破坏…）"
408:  python3 "$SCRIPT_DIR/check-core-boundary.py" --expect-pending ... || pm_fail "…（有 pending 码位已实现…）"
520:  [ -f "$SCRIPT_DIR/check-core-boundary.py" ] || { echo "FAIL: check-core-boundary.py missing"; rc=1; }
521:  if ! python3 "$SCRIPT_DIR/check-core-boundary.py" --self-test ...; then echo "FAIL…"; rc=1; fi

$ bash scripts/pre-merge.sh 2>&1 | tail -4
[pre-merge] M5-1.a core 边界不变量夹具（CORE_* 码位）…
[pre-merge] git diff --check（工作树 + 暂存区，机器证据除外）…
[pre-merge] git diff --check（origin/master merge-base..HEAD）…
[pre-merge] PRE_MERGE_RESULT=ALL_PASS
```

| 项 | 结果 |
|---|---|
| pre-merge.sh 挂载边界门 | ✅ 已挂载（`--self-test` + 默认 + `--expect-pending` 三段） |
| 全门实跑含边界 | ✅ `PRE_MERGE_RESULT=ALL_PASS`（含 `M5-1.a core 边界不变量夹具` 步骤） |
| W1 开放项状态 | ✅ **已关闭**（归 A2，现由 A2 完成挂载） |

**W1 最终裁定**：A2 的 M5-1.a 边界切片**代码层 PASS、行为零回归、且已在 pre-merge 强制（boundary-first 生效）**，可作为 M5-2（MCP）等后续 lane 的结构前提。§4 的「唯一开放项」现已解决，W1 边界门验证**全部完成**。

---

## 7. 声明（避免误读）

- 本车道**零产品代码改动、零策略脚本改动**（W1 硬停止仅 A2 可碰产品代码；A11 为 VERIFICATION lane）。
- 未 rebase / 未 push（board Merge Rule：仅 A0 可推送）。本增量与 A2/A10 的 W1 产物无交集（仅新增验证文档）。
- 结论基于：① `scripts/check-core-boundary.py` 三模式实跑；② `cargo test`/`cargo check`/`cargo fmt --check` 实跑；③ `git diff`/`grep` 对 L1~L5 的逐条复核；④ 与 A10 §4 命令对齐。非文档互证。
- 工作树并发提示见 §0：本结论取自最后一次一致快照，已用 `cargo test 329` 复核行为不变。
- 提交动作：仅 `git add` 本增量两个文件后提交（不带入其他 lane 的未提交 W1 增量），`不 push`。
