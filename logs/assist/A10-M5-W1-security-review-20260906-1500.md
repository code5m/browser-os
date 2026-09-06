# A10 · M5-W1 核心边界切片安全复审（Security Review of M5-1 Core Boundary Patch）

> Lane: `A10` — M5-W1 边界切片安全复审（`AI:DEEP / R:xhigh`）
> 时间: 2026-09-06 15:30 CST（W1；先于 A2 W1 产品代码落盘的**设计层复审 + 代码层复审预承诺**）
> BASE=`404f514`（A0 `docs(M5): integrate prework and dispatch core boundary wave`，当前本地 HEAD；工作树 clean，领先 origin/master 2）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W1 Implementation Dispatch → A10：**START REVIEW**，复核 A2 W1 边界补丁的泄漏（core 引 tauri / 第二执行路径 / 隐藏依赖 / 命令·ACL 漂移 / 策略误报）

---

## 0. 状态与结论（Lane 输出模板）

```text
LANE=A10
STATUS=BLOCKED（前置未达：A2 的 M5-1 W1 产品代码补丁尚未落盘；设计层复审已 PASS）
BASE=404f514
HEAD=docs only（本文件；未提交、未 push）
FILES=logs/assist/A10-M5-W1-security-review-20260906-1500.md（唯一文件）
VERIFY=见 §3（设计层已逐项核对 A2 v3 prework + A1 M5-1 W1-patched 卡）；代码层见 §4 预承诺命令
CHECKPOINT=logs/assist/A10-M5-W1-security-review-20260906-1500.md
MERGE_NOTES=① A2 产品代码未到，代码层复审无法执行；② W1 边界门是 Wave-0 发现 D-5/D-6/D-7 的结构前提（见 §6）
NEXT=A2 落地 check-core-boundary.py + pre-merge.sh + Cargo.toml[lib] + src/lib.rs 或 src/core/** → A10 重跑 §4 代码层复审
```

**一句话裁定**：M5-1 边界**设计层无安全倒退**（A2 v3 + A1 W1-patched 卡对五类泄漏都有对应硬规则，且 R8「编译层守不住、脚本是唯一防线」已被 A0 采纳为 boundary-first 前置）。但 **A2 的 W1 产品代码补丁当前不在本 checkout（工作树 clean、无 `check-core-boundary.py`/无 `src-tauri/src/core`/无 `src/lib.rs`/无 `Cargo.toml [lib]`）**，故代码层复审处于 `BLOCKED`，待 A2 交付后立即按 §4 执行。

---

## 1. 复审范围与方法（五类泄漏，来自 board W1 Hard Stops + A10 W1 指派）

board §M5-W1 Implementation Dispatch 硬停止与 A10 指派共同锁定本复审的 5 个泄漏维度：

1. **L1 · core 引 tauri**：core 内不得 `use tauri` / `tauri::` / `AppHandle` / `AppState`（排除注释行）。
2. **L2 · 第二执行路径**：core 内不得 `std::process::Command` / `Command::new` / `sh -c` / `bash -c`；不得自建进程路径（执行必须经 `script_runner` seam）。
3. **L3 · 隐藏依赖**：W1 硬停止「**无新 Cargo/npm 依赖**」；`git diff Cargo.toml` 仅可增 `[lib]` 段，不得增 `[dependencies]` 条目。
4. **L4 · 命令/ACL 漂移**：W1 硬停止「**无新 Tauri 命令、无新 ACL 条目**」；`bridge.rs`/`main.rs`/ACL 的 diff 仅可含 re-export shim，不得含 `generate_handler!` 增项或 ACL 新增行。
5. **L5 · 策略误报**：`check-core-boundary.py` 必须有真阴性+真阳性样本（`--self-test` 2 好 + 2 坏 + 1 阴（注释行不得误报）），否则门禁形同虚设（R8）。

复审方法 = **设计层**（已做，§3）+ **代码层**（预承诺，§4，待 A2 补丁）。

---

## 2. 工作树实证（为何代码层复审 BLOCKED）

```bash
$ git status --porcelain                  # 空（clean）
$ ls src-tauri/src/core 2>/dev/null        # 不存在
$ ls src-tauri/src/lib.rs 2>/dev/null       # 不存在（二进制仍仅 main.rs）
$ ls scripts/check-core-boundary.py 2>/dev/null  # 不存在
$ grep -rn "mvp_core\|mvp-core\|check-core-boundary" src-tauri src scripts 2>/dev/null | grep -v target   # 无命中
$ git log --oneline -3                     # 404f514 / 5ca8f9f（A0 文档集成，无 A2 W1 代码）
```

→ A2 的 M5-1 W1 产品代码（board 指派：`scripts/check-core-boundary.py`、`scripts/pre-merge.sh`、`src-tauri/Cargo.toml`、`src-tauri/src/lib.rs` 或 `src-tauri/src/core/**`）**均未落盘**。A2 W1 指派为「可写产品代码」，但本 checkout 尚未收到其产物（可能在 A2 自有 worktree 或尚未执行）。按 board Startup Gate：「若唯一阻塞是前置 lane 未交付，则不出产品代码，产只读 assist 注记 + 精确 unblock 条件」——本文件即该注记。

---

## 3. 设计层复审（PASS · 对五类泄漏的设计覆盖）

基于 `logs/assist/A2-M5-core-20260906-0749.md`（v3）+ `logs/checkpoints/M5-20260906/M5-1-core-workspace-split.md`（W1-patched）。逐类判定：

### L1 core 引 tauri → ✅ 设计覆盖，须盯注释行误报
- A2 §3.3 `COREBOUND_TAURI_IN_CORE` + A1 卡 §3/§5 命令 A（`grep -rE '^\s*use tauri' src-tauri/src/core/`，**须排除注释行**）对齐。
- A2 v3 §1.1 已实测：`domain.rs:47`、`database.rs:11`、`images.rs:4/404/895`、`session.rs:4`、`scripts.rs:797` 全是**文档注释**里的"不依赖 AppHandle"解释；脚本若不过滤注释行会误报。→ **设计已预知此坑**，要求排除注释。复审要点：核对脚本实现确实排除了 `//` 注释与 `///` doc 注释行（§4 命令会注入一条注释行阴性样本验证）。

### L2 第二执行路径 → ✅ 设计覆盖，须盯 seam 不引入新路径
- A2 §2.1 `R-B5`（core 内禁 `std::process::Command` / shell 词法）+ `COREBOUND_EXEC_PATH` + A1 卡 §4 FORBID（整文件带 `#[cfg(test)]` 迁，M4 护栏随迁）。
- 风险点（设计层提示，非阻断）：切片 1 的 `ProgressSink`/`PathResolver`/`RootsProvider` 三个 trait 是**注入点**而非执行点；只要 `script_runner` 仍经 `std::process::Command` 且仅在 **B 类 seam 改造后**才进 core（M5-1.b），W1（仅 M5-1.a = 0b+0a）搬的是 A 类纯逻辑，**本切片不含执行引擎**，故 L2 在 W1 切片的暴露面为 0。✅

### L3 隐藏依赖 → ⚠️ 设计允许但须硬查「零新依赖」
- W1 硬停止明确「无新 Cargo/npm 依赖」；A2 §3.3 依赖白名单与黑名单（`tauri`/`tauri-build`/`tauri-plugin-*`/`tokio` 禁）。
- 设计层提示：A2 白名单含 `portable-pty`/`strip-ansi-escapes`（现为 `terminal.rs`/C 类所用，C 类不进 core）。若 core 实际不引用它们，把它们列入核心白名单属**过度放行**（低风险，因属已有依赖、非新增），但复审仍须确认 `git diff Cargo.toml` **只增 `[lib]`、不增任何 `[dependencies]`**（§4 命令）。这是 L3 在 W1 的唯一硬门。

### L4 命令/ACL 漂移 → ⚠️ 设计 shim 安全，须硬查 diff 纯度
- A2 §3.2 re-export shim：`pub use mvp_browser_os_core::{...};` 在 `main.rs`，二进制子模块 `use crate::domain::X` 免改写。
- 风险点：W1 硬停止「无新 Tauri 命令、无新 ACL 条目」。A2 W1 范围只到 0a（建 lib + 搬 A 类），**不注册任何命令、不碰 ACL**。复审须确认 `bridge.rs`/`main.rs`/`default-commands.toml` 的 diff **仅含 shim 的 `pub use` 行**，无 `generate_handler!` 增项、无 ACL 新增行（§4 命令）。若 A2 越界加了命令/ACL，属 W1 硬停止违例，直接 BLOCK。

### L5 策略误报 → ✅ 设计覆盖（最关键的 R8 门），须盯 self-test 样本真实性
- A2 §3.3 + A1 卡 §3/§5 明确 `check-core-boundary.py`（`--self-test` 2好+2坏+1阴 / 默认扫描 / `--expect-pending`）；A0 W1 boundary-first 把「步骤 0 建脚本挂 pre-merge」列为**所有 core 操作的准入前置**（R8：编译层守不住 core⇸tauri，脚本是唯一防线）。
- 设计层已正确：阶段一 `COREBOUND_TREE_TAURI`（`cargo tree -p core | grep tauri`）因同 package 双 target 必然命中 tauri，故**阶段一 PENDING、阶段二转 FAIL**（C-2 已采纳），阶段一改用源码级 `grep` 断言。→ 避免拿必然失败的判据当 PASS 门槛。✅
- 唯一须硬查（代码层）：脚本 `--self-test` 是否真有「坏样本被抓 + 注释行阴性不误报」的断言逻辑，而非空跑。§4 命令会实跑 `--self-test` 并查退出码。

**设计层小结**：五类泄漏在设计层均有对应硬规则，无设计倒退；L3/L4 的「零新依赖 / 零命令·ACL」与 L5 的「self-test 真实性」三项是 W1 落到补丁后的**硬缺口**，必须在代码层复核。

---

## 4. 代码层复审（预承诺 · A2 补丁落盘后由 A10 立即执行）

A2 一旦交付，A10 在本 checkout 跑以下命令（**不改任何文件**，纯验证）：

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# —— L1 core 引 tauri ——
echo "[L1] core 内 tauri 引用（须排除注释行，结果应为空）"
grep -rE '^\s*use tauri|use tauri::|tauri::AppHandle|tauri::AppState|AppHandle|AppState' \
  src-tauri/src/core 2>/dev/null | grep -vE '^\s*//|^\s*///' | tee /tmp/w1-L1
test -s /tmp/w1-L1 && echo "L1 FAIL: core 含 tauri 引用" || echo "L1 PASS"

# —— L2 第二执行路径 ——
echo "[L2] core 内第二执行路径（须为空）"
grep -rEn 'std::process::Command|Command::new|sh -c|bash -c' src-tauri/src/core 2>/dev/null \
  | tee /tmp/w1-L2
test -s /tmp/w1-L2 && echo "L2 FAIL" || echo "L2 PASS"

# —— L3 隐藏依赖：Cargo.toml 仅增 [lib]，无新 [dependencies] ——
echo "[L3] Cargo.toml diff（须仅有 [lib] name/path 段，无新增依赖行）"
git diff src-tauri/Cargo.toml | tee /tmp/w1-L3
# 人工判：diff 中不得出现 '+' 开头且匹配依赖名的新 [dependencies] 条目；[lib] 段 + 行允许
grep -E '^\+\[dependencies\]|^\+[a-z0-9_-]+ = ' /tmp/w1-L3 && echo "L3 FAIL: 新增依赖" || echo "L3 PASS（无新依赖）"

# —— L4 命令/ACL 漂移：bridge/main/ACL diff 仅含 shim ——
echo "[L4] bridge.rs / main.rs / ACL diff（须无 generate_handler! 增项、无 ACL 新增行）"
git diff src-tauri/src/bridge.rs src-tauri/src/main.rs src-tauri/permissions/default-commands.toml \
  | tee /tmp/w1-L4
grep -E '^\+.*generate_handler!|^\+invoke_handler|^\+[a-z_]+(\s*=\s*|")' /tmp/w1-L4 \
  && echo "L4 FAIL: 新增命令/ACL" || echo "L4 PASS（仅 shim）"

# —— L5 策略脚本 self-test 真实性 ——
echo "[L5] check-core-boundary.py --self-test（须 PASS，含 2好+2坏+1阴）"
python3 scripts/check-core-boundary.py --self-test && echo "L5 self-test PASS" || echo "L5 FAIL"
python3 scripts/check-core-boundary.py && echo "L5 default PASS" || echo "L5 FAIL"
# 验证注释行阴性：脚本内须有对 '// use tauri' 类注释的负向样本（查 fixtures 或 self-test 内联样本）
grep -rEn 'use tauri' scripts/fixtures/check-core-boundary* 2>/dev/null | grep -E '//|#' \
  && echo "L5 注释阴性样本存在" || echo "L5 注：未见注释阴性样本，须人工确认"

# —— 行为不变代理：测试总数不减 + pre-merge ALL_PASS ——
echo "[行为] 测试总数对齐 + 门禁"
cargo test --manifest-path src-tauri/Cargo.toml -- --list 2>/dev/null | grep -c '::' | tee /tmp/w1-tests
# 与迁移前基线 329 对比（M4 终态）；须一致
bash scripts/pre-merge.sh && echo "PRE-MERGE ALL_PASS" || echo "PRE-MERGE FAIL"
git diff --check && echo "DIFF CHECK CLEAN" || echo "DIFF CHECK DIRTY"
```

判定门槛：L1/L2/L3/L4/L5 全 PASS 且 `pre-merge.sh ALL_PASS` + `git diff --check` 干净 → A10 将 STATUS 由 BLOCKED 转 `PASS`，回填本文件 MERGE_NOTES。

---

## 5. Unblock 条件（精确）

A10 解除 BLOCKED 当且仅当 **A2 在本 checkout（或经 A0 合入 master，且 A10 已 `git pull --ff-only`）交付以下全部**：

1. `scripts/check-core-boundary.py`（含 `--self-test` 2好+2坏+1阴 / 默认扫描 / `--expect-pending` 三模式，R8 门）。
2. `scripts/pre-merge.sh`（新增该脚本的 `--self-test` 与默认挂载位，位置在 `git diff --check` 之前）。
3. `src-tauri/Cargo.toml`（仅增 `[lib] name="mvp_browser_os_core" path=...`，**无新依赖**）。
4. `src-tauri/src/lib.rs`（或 `src-tauri/src/core/mod.rs`）core 根 + `pub mod core; pub use crate::core::*;`。
5. `src-tauri/src/core/**`（切片 0b+0a：常量收口 + 10 个 A 类模块整文件带 `#[cfg(test)]` 搬入）。
6. `src-tauri/src/main.rs` 仅增 re-export shim，无命令/ACL 新增。

交付后 A10 跑 §4 全量命令；任一 L* FAIL → 维持 BLOCKED 并指明失哪项。

---

## 6. 前向关联：W1 边界门是 Wave-0 发现 D-5/D-6/D-7 的结构前提

本复审（W1）与 A10 M5 Wave-0 v3 复核（`A10-M5-security-review-20260906-1410.md`）的 D-5/D-6/D-7 直接耦合：

- **D-5（rmcp→tokio vs F-1）**：M5-2 MCP 的 `rmcp` 会引 `tokio`。Wave-0 建议解 = MCP 落**独立 crate/bin 仅依赖 `mvp-core`**，主二进制依赖图零变化。→ **`mvp_core` 边界（W1）+ `check-core-boundary.py` 的 R-B6 依赖白名单（禁 `tokio`）正是 D-5 的机器防线**：M5-2 开始前必须已存在 `check-core-boundary.py`，否则 tokio 可能被误提为主二进制直接依赖。
- **D-6（MCP 无 webview 致来源校验不可套用）**：MCP 专用来源通道须是 **core 内抽象**（`core ⇸ bin`，R-B3）。若 core 边界未立，MCP 来源通道易被迫伪造 `AppState` 冒充 webview（红线）。→ W1 边界立起后，M5-2 才能干净地增 core 内来源通道。
- **D-7（open_tool ACL 缺口 + 奇偶门禁）**：能力真源 `capability.rs` 须落 **core**（A4/A9 共识）。W1 把 `security_policy`/`domain` 收进 core，为 M5-2 的「工具真源=能力白名单」提供可信落点。

→ **A10 建议 A0**：M5-2（MCP）的派发门槛中，必须包含「`check-core-boundary.py` 已落且 `--self-test` PASS」作为前置（与 A0 W1 boundary-first 裁定一致）。否则 D-5/D-6 的缓解将无机器守门。

---

## 7. 声明（避免误读）

- 本轮**零产品代码改动、零策略脚本改动**（W1 硬停止仅 A2 可写产品代码；A10 为 REVIEW lane）。
- 未 rebase / 未 commit / 未 push（board Merge Rule：仅 A0 可推送）。本文件为新增独立文档，与 A2 未交付的 W1 产物无交集（A2 产物尚不存在）。
- 结论基于：① A2 v3 prework（逐行 grep 实测事实）+ ② A1 M5-1 W1-patched 卡 + ③ A0 W1 dispatch 裁定 + ④ 本 checkout `git status`/`ls`/`grep` 实证（§2）。非文档互证。
- 本文件是 A10 M5-W1 的**设计层完整交付 + 代码层预承诺**；待 A2 补丁落盘，A10 补跑 §4 并将 STATUS 由 BLOCKED 更新为 PASS/BLOCKED 终态（回填本文件 §0 模板 + MERGE_NOTES）。
