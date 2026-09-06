# A10 · M5-W1 核心边界切片安全复审（Security Review of M5-1 Core Boundary Patch）

> Lane: `A10` — M5-W1 边界切片安全复审（`AI:DEEP / R:xhigh`）
> 首稿：2026-09-06 15:30（设计层复审 + 代码层预承诺，当时 A2 补丁未到 → `BLOCKED`）
> **终稿：2026-09-06 16:10（A2 补丁已落 `854bc40`，代码层复审已执行 → `PASS`）**
> BASE=`404f514`（A0 M5-W1 dispatch 集成）
> HEAD=`854bc40`（A2 `feat(M5): add core boundary gate`，含本文件首稿）+ 本终稿
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W1 Implementation Dispatch → A10：**START REVIEW**；`A0-M5-W1-dispatch-20260906-0835.md` 指派与硬停止

---

## 0. Lane 输出模板（终态）

```text
LANE=A10
STATUS=PASS
BASE=404f514
HEAD=854bc40（A2 边界补丁）+ 本终稿
FILES=logs/assist/A10-M5-W1-security-review-20260906-1500.md（本文件，首稿 BLOCKED → 终稿 PASS）
VERIFY=§4 全量执行：L1/L2 干净、L3 仅 [lib]、L4 仅 shim、L5 三模式 PASS、cargo test 329/329、pre-merge 已挂
CHECKPOINT=logs/assist/A10-M5-W1-security-review-20260906-1500.md
MERGE_NOTES=① 仅 keyring_store.rs 搬入 core（最小边界），无行为变更；② 阶段一同 package 双 target 下依赖白名单码位 PENDING，tokio 入 core 在阶段一无机器守门——见 §5；③ 与 Wave-0 D-5/D-6/D-7 衔接见 §6
NEXT=M5-W2：待 M5-2(MCP) 卡/产品代码出现后，复审 MCP 来源通道是否落 core、rmcp→tokio 是否被误提为主二进制依赖（D-5）
```

---

## 1. 复审范围与方法（五类泄漏，来自 board W1 Hard Stops + A10 W1 指派）

board §M5-W1 Implementation Dispatch 硬停止与 A10 指派共同锁定本复审的 5 个泄漏维度：

1. **L1 · core 引 tauri**：core 内不得 `use tauri` / `tauri::` / `AppHandle` / `AppState`（排除注释行）。
2. **L2 · 第二执行路径**：core 内不得 `std::process::Command` / `Command::new` / `sh -c` / `bash -c`；不得自建进程路径。
3. **L3 · 隐藏依赖**：W1 硬停止「**无新 Cargo/npm 依赖**」；`git diff Cargo.toml` 仅可增 `[lib]` 段，不得增 `[dependencies]` 条目。
4. **L4 · 命令/ACL 漂移**：W1 硬停止「**无新 Tauri 命令、无新 ACL 条目**」；`bridge.rs`/`main.rs`/ACL 的 diff 仅可含 re-export shim。
5. **L5 · 策略误报**：`check-core-boundary.py` 必须有真阴性+真阳性样本（`--self-test` 好+坏+阴），否则门禁形同虚设（R8）。

复审 = 设计层（§3，首稿已 PASS）+ **代码层（§4，本终稿已执行）**。

---

## 2. A2 在 854bc40 实际落地的边界切片（scope 核对）

```bash
$ git show 854bc40 --stat | grep -E 'scripts/|Cargo|src-tauri/src/(core|lib|main)'
 scripts/check-core-boundary.py                     | 591 ++++++++++++++++
 scripts/pre-merge.sh                               |  22 +
 src-tauri/Cargo.toml                               |  10 +
 src-tauri/src/{ => core}/keyring_store.rs          |   0      # 整文件搬移，零改动
 src-tauri/src/core/mod.rs                          |  24 +    # pub mod keyring_store;
 src-tauri/src/lib.rs                              |  30 +    # pub mod core; pub use crate::core::*;
 src-tauri/src/main.rs                             |   8 +-   # 删 mod keyring_store; 加 pub use mvp_core::keyring_store;
```

**结论**：落地的正是 board 要求的「最小 `mvp_core` 模块/lib 边界，不改行为」——仅把零 Tauri 耦合的 `keyring_store.rs` 搬进 `core/`，配 lib target + re-export shim；`check-core-boundary.py` 作为准入前置挂入 `pre-merge.sh`。**未搬迁 scheduler/database/bridge**（符合 W1 硬停止「不搬行为」）。scope 与指派完全一致。

---

## 3. 设计层复审（PASS · 对五类泄漏的设计覆盖，首稿结论，仍成立）

- **L1/L2**：A2 §3.3 + A1 卡 §4 的 `CORE_TAURI_IMPORT`/`CORE_APP_HANDLE`/`CORE_BRIDGE_REF`/`CORE_SECOND_EXEC_PATH` 覆盖；脚本实现 `strip_strings_and_comments` 专门剥离注释/字符串，规避 `domain.rs:47` 类注释误报（§4 实测验证）。
- **L3**：W1 硬停止「无新依赖」；脚本 `CORE_DEP_NOT_ALLOWLISTED` 为 PENDING（阶段一同 package 双 target 不可判），改用「Cargo.toml diff 仅 [lib]」人工/code-review 守门（§4 实测）。
- **L4**：re-export shim 使二进制侧 `crate::keyring_store::X` 免改写；W1 硬停止「无新命令/ACL」。
- **L5**：脚本含 7 ACTIVE 码位 + 2 PENDING，`--self-test` 含 2 好样本（合成参考实现 + 真实仓库）+ 1 注释/字符串阴性样本 + 9 坏样本（每个码位≥1，且带「变异防呆」：内容未真变则 `sys.exit(1)` 按漏检计）。设计层已确认 R8「编译层守不住、脚本是唯一防线」被采纳为 boundary-first 前置。

---

## 4. 代码层复审（已执行 · 全部 PASS）

在 `854bc40` 落地后的工作树实跑（只读验证，未改任何产品代码）：

### L5 · 策略脚本真实性（最关键 R8 门）
```bash
$ python3 scripts/check-core-boundary.py --self-test
CORE_POLICY_SELF_TEST=PASS（ACTIVE=7，坏样本=9，含注释/字符串阴性样本）   EXIT=0
$ python3 scripts/check-core-boundary.py
core boundary policy: all invariants hold（ACTIVE=7，core 文件=2）        EXIT=0
$ python3 scripts/check-core-boundary.py --expect-pending
CORE_PENDING_RESULT=NONE（2 个 pending 码位均未实现，阶段一同 package 双 target 下预期如此）  EXIT=0
```
→ 好样本零误报、坏样本全检出、注释/字符串阴性样本不误报、pending 码位如实未实现。**L5 PASS**。

### L1 · core 引 tauri
```bash
$ grep -rEn 'use tauri|tauri::|AppHandle|AppState' src-tauri/src/core | grep -vE '^\s*//|^\s*///'
（无输出）→ L1 CLEAN
```

### L2 · 第二执行路径
```bash
$ grep -rEn 'std::process::Command|Command::new|sh -c|bash -c' src-tauri/src/core
（无输出）→ L2 CLEAN
```

### L3 · 隐藏依赖（Cargo.toml diff 仅 [lib]）
```bash
$ git show 854bc40 -- src-tauri/Cargo.toml | grep -E '^\+' | grep -vE '^\+\+\+'
+# M5-1.a：纯逻辑库边界 …（注释）
+[lib]
+name = "mvp_core"
+path = "src/lib.rs"
+                                                        # ← 仅有 [lib] 段，无 [dependencies] 条目
$ grep -E '^\+[a-z0-9_-]+ = |^\+\[dependencies\]' /tmp/cargo_add.txt   # 空 → 无新依赖
→ L3 PASS（无隐藏依赖；`name`/`path` 属 [lib] 段内，非新依赖）
```

### L4 · 命令/ACL 漂移（main.rs 仅 shim）
```bash
$ git show 854bc40 -- src-tauri/src/main.rs
- mod keyring_store;                       # 删
+ pub use mvp_core::keyring_store;         # 加 re-export shim（仅此一处行为性改动）
$ git show 854bc40 --stat | grep -iE 'default-commands|bridge.rs'   # 空 → 无 ACL/命令变更
→ L4 PASS（无新命令、无新 ACL）
```

### pre-merge 门禁实挂（W1 门必须 ACTIVE，非仅存在）
```bash
$ grep -n 'check-core-boundary' scripts/pre-merge.sh
404: python3 "$SCRIPT_DIR/check-core-boundary.py" --self-test || pm_fail ...
406: python3 "$SCRIPT_DIR/check-core-boundary.py"          || pm_fail "core 边界被破坏…"
408: python3 "$SCRIPT_DIR/check-core-boundary.py" --expect-pending || pm_fail ...
520: [ -f "$SCRIPT_DIR/check-core-boundary.py" ] || FAIL
521: if ! python3 …/check-core-boundary.py --self-test … ; then FAIL
→ 主流程 + 总结双处挂载，任一失败即 `pm_fail` 阻断合并。L5 门已生效。
```

### 行为不变代理（M4 基线 329）
```bash
$ cargo test --manifest-path src-tauri/Cargo.toml
test result: ok. 329 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   # == M4 终态
$ cargo test --manifest-path src-tauri/Cargo.toml --lib
Finished `test` profile … ; lib target 编译通过（keyring_store 无 #[cfg(test)] 单测，行为由 bin 侧 329 覆盖）
→ 搬迁 + shim 零行为变更；lib target 可独立编译（core 边界成立）。
```

**五类泄漏全 PASS + 行为零回归 + 门禁已挂 → STATUS=PASS。**

---

## 5. 残留风险与建议（不影响本切片 PASS，供 A0/M5-W2 决策）

1. **阶段一依赖白名单未机器守门（接 Wave-0 D-5）**：`CORE_DEP_NOT_ALLOWLISTED` 在阶段一同 package 双 target 下 PENDING（正确，因 `[dependencies]` 对 lib/bin 共享，tauri 必须在）。**后果**：若后续 lane 在 `Cargo.toml [dependencies]` 新增 `tokio`/`tauri-plugin-*` 并被 core 间接引用，脚本在阶段一抓不到。W1 仅靠「Cargo.toml diff 仅 [lib]」的人工/code-review 守门。
   - **建议（M5-W2 前置）**：在 `pre-merge.sh` 增一条「`git diff` 的 `src-tauri/Cargo.toml` 不得出现新增 `[dependencies]` 条目（除 A0 显式裁决）」的轻量断言，作为阶段一的临时依赖守门；**或在 M5-2 启动前把 core 提升为阶段二独立 crate，使 `CORE_DEP_NOT_ALLOWLISTED` 转 ACTIVE**。否则 D-5（rmcp→tokio 不得入主二进制依赖图）无机器防线。

2. **`CORE_TREE_TAURI` 阶段一 PENDING（正确）**：同 package 下 `cargo tree -p mvp-core` 必命中 tauri，故该判据在阶段二才转 ACTIVE。阶段一已用源码级 `grep` 替代，无遗漏。

3. **allowlist 漂移（无害）**：脚本 `CORE_DEP_ALLOWLIST`（阶段二才启用）比 A2 v3 提案多了 `open`/`arboard`/`freedesktop-icons`。因该码位阶段一 PENDING、不起作用，当前无影响；阶段二启用前建议 A0 与 A2 对齐白名单，避免误放桌面集成 crate。

4. **core 当前仅 2 文件**：`keyring_store.rs` + `mod.rs`。M5-1.a 后续切片（搬 domain/security_policy/database 等 10 个 A 类）仍须经本脚本 + `cargo test` 总数对齐双重守门；每搬一个模块须在 `main.rs` 补一行 `pub use mvp_core::<mod>;` 并删 `mod <mod>;`，`CORE_SHIM_CONFLICT` 已覆盖该冲突。

---

## 6. 前向关联：W1 边界门是 Wave-0 发现 D-5/D-6/D-7 的结构前提（仍成立）

- **D-5（rmcp→tokio vs F-1）**：M5-2 的 `rmcp` 会引 `tokio`。本切片已立 `mvp_core` 边界，但阶段一 `CORE_DEP_NOT_ALLOWLISTED` 为 PENDING（§5-1）。**D-5 的机器防线在阶段一缺失**，须按 §5-1 补临时依赖断言或升阶段二。
- **D-6（MCP 无 webview 致来源校验不可套用）**：MCP 专用来源通道须是 core 内抽象（`core ⇸ bin`，R-B3）。本切片立起的 core 边界使其可干净落地，但 `capability.rs` 真源（U-4）尚未建（见 A4 W2 切片记要），MCP 来源通道仍待 capability 边界。
- **D-7（open_tool ACL 缺口）**：能力真源 `capability.rs` 须落 core。本切片把 `security_policy`/`domain` 等收口路径预留好，为 M5-2 的「工具真源=能力白名单」提供可信落点。

→ **A10 建议 A0**：M5-2（MCP）派发门槛中，除「core 边界已立」外，须显式包含「阶段二独立 crate 或临时依赖断言已就位（D-5 守门）」与「capability 真源已立（D-6/D-7 守门）」两项，否则 D-5/D-6 缓解无机器守门。

---

## 7. Unblock 条件（已于 854bc40 满足）

A10 在首稿（BLOCKED）中列的条件：① `check-core-boundary.py` 三模式；② `pre-merge.sh` 挂载；③ `Cargo.toml [lib]` 无新依赖；④ `lib.rs`/`core/**`；⑤ `main.rs` 仅 shim。**五项全部满足**（§2/§4 实证）。故 STATUS 由 `BLOCKED` 转为 `PASS`。

---

## 8. 声明（避免误读）

- 本轮**零产品代码改动、零策略脚本改动**（A10 为 REVIEW lane；W1 硬停止仅 A2 可写产品代码）。仅更新本复审文档（首稿 BLOCKED → 终稿 PASS）。
- 未 rebase / 未 commit / 未 push（board Merge Rule：仅 A0 可推送）。本文件为 A10 自有 lane 文档，与同树中他 lane 未提交产物（A1 `M5-*.md`、A4/A5 `logs/*`）无交集，未触碰其文件。
- 结论基于：① A2 854bc40 实际 diff；② `check-core-boundary.py` 源码与三模式实跑；③ `pre-merge.sh` 挂载实证；④ `cargo test` 329/329；⑤ `grep` 五类泄漏实测。**非文档互证**。
- 本文件即 A10 M5-W1 的**完整交付（设计层 PASS + 代码层 PASS）**；待 M5-W2（MCP）产品代码/卡出现，A10 接续 §6 的 D-5/D-6/D-7 复审。
