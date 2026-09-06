# M5-1 核心 workspace 下沉（Core Workspace Extraction）

> 子卡 ID：**M5-1** · 需求 #7（A2P/A2A）的实现前置 · `[S3|LEVERAGE:3|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A13**（A9 提案；A0 签发时定）
> 父卡：`详细设计与实施计划.md` L562（`M5-1 核心 workspace 下沉`）
> 主预研：`logs/assist/A2-M5-core-20260906-0749.md`（A2 v3 升级版：4 边界 + 6 处分歧 C-1~C-6 + 2 处 v3 新增 C-7/C-8 + V-7 常量副本）
> 配套：`logs/assist/A9-M5-split-20260905-2359.md` §5 · `A9-M5-A13plus-cards-20260906-0010.md` A13

---

## [W1 patched · 2026-09-06 08:50 CST] A2 v3 6+2 处分歧应用 + A0 W1 boundary-first 顺序重组

> 修订依据：A0 `404f514` 后 M5-W1 dispatch（`logs/checkpoints/A0-M5-W1-dispatch-20260906-0835.md`）A1 行；A2 v3 `A2-M5-core-20260906-0749.md` §12（6 处）/ §13（C-7/C-8/V-7）；A10 复审 `A10-M5-security-review-20260906-1410.md` §102 确认 A2 边界硬规则。
> 修订原则：A1 保留 M5-W0 权威 WBS 编号（`M5-0..M5-14`），事实数据按 A2 实测回填。修订**仅加 `[W1 patched]` 段与局部行内修正**，不重写 §1~§11 决策史。

### W1 修订点

| # | 来源 | W0 现状 | W1 修订 |
|---|---|---|---|
| W1-1 | A2 C-1 | §3 顺序 `0a → 0b` | **改 `0b → 0a`**：测试反向 import 必须先收口，否则 core 反向依赖 bin 编译失败 |
| W1-2 | A2 C-2 | §5 命令 B `cargo tree -p core \\| grep tauri` | 阶段一**降级 PENDING**（同 package 双 target 必然命中 tauri），改用源码级 `grep -rE '^\s*use tauri' src-tauri/src/core/` 断言（须排除注释行） |
| W1-3 | A2 C-3 | `M5-1.b` 同时指"切片 1/2 B 类搬入"与"阶段二 workspace 化" | **拆为 `M5-1.b`**（B 类 seam+搬入）+ **`M5-1.c`**（阶段二 workspace 化，可选） |
| W1-4 | A2 C-4 | §1 "23 模块" | **回填 "20 业务模块 + `main.rs`"** |
| W1-5 | A2 C-5 | §10 "反向边 4 处"列 `scheduler.rs:26/28/259/491/588/617/622/742` | **改 "scheduler 8 处 + 2 组测试反向 import"**：scheduler 实际为 `:28/259/491/588/617/622/742/761`（`:26` 无命中），另 `domain.rs:1525-1526` 与 `scripts.rs` 15 处测试引用 |
| W1-6 | A2 C-6 | §3 `[lib] path="src/core/mod.rs"` | 库根路径**二选一**：`src/core/mod.rs`（core 内 `crate::domain` 需在 `core/mod.rs` 声明）或 `src/lib.rs` + `pub mod core; pub use crate::core::*;`（实测模式） |
| W1-7 | A2 C-7 (v3) | （本卡未涉）→ `M5-14 §6 L83` 反向边挂 `M5-1.a` 解决 | 改挂 **`M5-1.b`**（切片 1 抽 trait 才解除） |
| W1-8 | A2 C-8 (v3) | §3/§5 `check-core-boundary.sh` | **改 `.py`**（仓库 `scripts/` 30+ 门禁脚本惯例 + `pre-merge.sh` 的 `python3 "$SCRIPT_DIR/x.py" --self-test` 挂法） |
| W1-9 | A0 W1 boundary-first | （W0 未排先后）| **新增实施序**：步骤 0（建 `check-core-boundary.py` 挂 `pre-merge.sh`）→ 0b（常量收口）→ 0a（建 core lib）→ 1（抽 trait 解反向边）→ 2（B 类 seam）→ c（workspace 化） |
| W1-10 | A2 V-7 (v3) | §10 仅列 3 个常量 | **补 2 个常量副本**：`DB_MAX_TEXT_FIELD_BYTES`（`database.rs:45` 删，自 `domain.rs:1027` `pub use`）+ `DB_SOFT_TO_HARD_GRACE_SECS`（`database.rs:52` 删，自 `domain.rs:1039` `pub use`，注意 `u32` vs `u64` 转换） |
| W1-11 | A2 §13.5 T-db-c6 退化 | 未提 | 切片 0b 后 `t_db_c6_limit_and_timeout_constants_are_aligned` 变"同一模块内自比"失去跨模块防漂移；**改字面值锁定 + 别名存在性断言**（代码示例见 A2 v3 §13.5） |

### W1 决策点（待 A0 在决策 1 拍板）

- A2 建议拆 `M5-1.a` 内部为两个 commit：先 0b（常量收口 + 改 t_db_c6），再建 0a（core lib + A 类搬入）—— A1 接受。
- A2 建议 `M5-1.a` 含 0a+0b；`M5-1.b` 含切片 1/2；`M5-1.c` 阶段二 workspace 化（可选，延后到 M5-2/M5-7 需要时）—— A1 接受。
- A2 建议 `check-core-boundary.py` 含 `--self-test` 2好+2坏+1阴（注释行不得误报）、默认扫描、`--expect-pending` 三模式 —— A1 接受。

---

## 0. 编号与锚定

- 批次任务号 `M5-1`；需求号 #7；WBS L562 一致。
- 内部子号（A0 决策 1 拍板后用）：
  - `M5-1.a` = 步骤 0b（常量收口 + 改 t_db_c6）+ 步骤 0a（core lib + A 类搬入）—— **A1 W1 接受 A2 建议**（原 W0 把 a=0a+0b 但顺序错；W1 改 0b 先于 0a）
  - `M5-1.b` = 步骤 1（抽 `RootsProvider`/`ProgressSink`/`PathResolver` trait，**解除 scheduler 反向边**）+ 步骤 2（B 类 seam）
  - `M5-1.c` = 步骤 c（阶段二 `Cargo.toml [workspace]` 化，可选，延后到 M5-2/M5-7 需要时）—— A1 W1 接受 A2 C-3 拆分
- 依赖：M4 PASS（`a1a2061`）✅、`A2-M5-core-*.md` v3 ✅、A0 签发 `M5-1.a` 实施卡、A2 先建 `check-core-boundary.py` 挂 `pre-merge.sh`（A0 W1 boundary-first 前置）。
- 必先解除的循环依赖：`scheduler.rs` → `bridge::AppState`（`scheduler.rs:28/259/491/588/617/622/742/761` 共 8 处，是 M5 全部能力能否复用 scheduler 的关键）+ `domain.rs:1525-1526` 与 `scripts.rs` 15 处测试反向 import（`M5-1.a` 步骤 0b 收口）。

---

## 1. GOAL

把当前单 crate（**20 业务模块 + `main.rs` / 24,859 行 / `main.rs` 扁平 mod**）拆为 **`mvp-core` 库 crate**（纯逻辑，零 Tauri 依赖）+ **现有二进制**（Tauri 专属），**主应用始终可独立构建**。彻底解除 `scheduler → bridge::AppState` 反向边；为 M5-2/4/7/10 等后续能力提供"纯逻辑落 core / 命令面与 transport 落二进制"的双层基础。

**不是**：下沉后立刻做新能力（M5-2/4/7/10 是其它卡）。**是**：把脚手架搭好，红线卡住，让后续每张 M5-x 都能无副作用地生长。

---

## 2. READ（必读）

1. `logs/assist/A2-M5-core-20260906-0749.md`（**全读**，主预研）
2. `src-tauri/src/domain.rs`（1804 行，契约根，**全读**）
3. `src-tauri/src/scheduler.rs`（996 行，重点看 L26-28/259/491/588/617/622/742 反向边）
4. `src-tauri/Cargo.toml`（现有依赖清单、`[lib]` 是否存在）
5. `scripts/pre-merge.sh`（基线 + 挂载位）
6. `详细设计与实施计划.md` L494-507（§7 M5 总目标）
7. `M5-0-overview.md` §7 红线总览

---

## 3. WRITE（必改文件候选 · 不写实现，仅列契约落地位置）

| 文件 | 性质 | 说明 |
|---|---|---|
| `scripts/check-core-boundary.py` | **新增** | **A0 W1 boundary-first 步骤 0**：核心边界断言；`--self-test` 模式须 2 好样本零违规 + 2 合成坏样本全检出 + 1 阴（注释行不得误报）；默认扫描模式（`mvp-core` 内禁 `use tauri` / `use crate::bridge` / `use crate::main` / `use crate::terminal` / `use crate::grid_process` / `use crate::tools` / `use crate::shutdown`）；`--expect-pending` 模式（阶段一允许的 PENDING 项明示）。**用 `.py`** 不用 `.sh`（仓库 `scripts/` 30+ 门禁脚本惯例 + `pre-merge.sh` 的 `python3 "$SCRIPT_DIR/x.py" --self-test` 挂法） |
| `scripts/pre-merge.sh` | 修改 | 加 `python3 scripts/check-core-boundary.py --self-test` 与 `python3 scripts/check-core-boundary.py` 挂载位（**位置：`git diff --check` 之前**） |
| `src-tauri/Cargo.toml` | 修改 | 加 `[lib] name="mvp_browser_os_core" path="src/core/mod.rs"`（**或 `path="src/lib.rs"`** —— 二选一均可，见 [W1 patched] C-6）；保留 `[[bin]]`；**首切同 package 双 target**（阶段一），避免 workspace 嵌套冲突（坑位：现有 `tauri-browser-tabs/` 已是另一 workspace，**禁止**未排除地嵌套）<br>**[W1 协同发现 · 2026-09-06 08:50 CST]**：A2 W1 已选 **`path="src/lib.rs"`**（C-6 第二种），lib 名为 `mvp_core`（`Cargo.toml` +10 行）；不要在 A2 已交付基础上改回 `path="src/core/mod.rs"`。 |
| `src-tauri/src/lib.rs` | **新增**（C-6 选 lib 根时） | 二进制侧过渡：`pub mod core; pub use crate::core::*;` |
| `src-tauri/src/core/mod.rs` | **新增** | core 入口；re-export 切片 0a 搬入的 10 个 A 类模块；若选 C-6 的 `src/lib.rs` 根，则此处改为 `pub use` glob<br>**[W1 协同发现 · 2026-09-06 08:50 CST]**：A2 W1 已建 `src-tauri/src/core/mod.rs` 并搬入**首个** A 类模块 `keyring_store`（0 行为变化，纯重命名）。**`bridge.rs` 靠 `lib.rs` 的 `pub use crate::core::*;` re-export shim 桥接，零改写**。其余 9 个 A 类模块仍在 W1 后的切片 0a 后续 commit。 |
| `src-tauri/src/core/domain.rs` 等 | **新增**（移动） | 切片 0a 把 A 类模块整文件带测试搬入 |
| `src-tauri/src/main.rs` | 修改 | 加 `pub use mvp_browser_os_core::*;` 过渡 shim；其余模块的 `use crate::domain` 仍可编译<br>**[W1 协同发现 · 2026-09-06 08:50 CST]**：A2 W1 已加 `pub use mvp_core::*;` 过渡 shim、删 `mod keyring_store;`（因 keyring_store 已在 `core/mod.rs` 内）。A1 不需在文档中再加 `pub use mvp_browser_os_core::*;`（A2 选的是 `mvp_core` lib 名，不是 A1 卡的 `mvp_browser_os_core`）。 |
| `src-tauri/src/database.rs` | 修改（V-7 收口） | **步骤 0b 一并收口**：删 `database.rs:45/52` 的 `DB_MAX_TEXT_FIELD_BYTES` / `DB_SOFT_TO_HARD_GRACE_SECS` 副本，改 `pub use crate::domain::{DB_MAX_TEXT_FIELD_BYTES, DB_SOFT_TO_HARD_GRACE_SECS};`；注意 `u32` vs `u64` 转换（`Duration::from_secs(DB_SOFT_TO_HARD_GRACE_SECS as u64)`） |
| `src-tauri/src/domain.rs` | 修改（步骤 0b + T-db-c6） | ① 把 `HARD_GRACE_SECS`（`script_runner`）/ `MAX_TIMEOUT_SECS`（`script_runner`）/ `MAX_TEXT_FIELD_BYTES`（`security_policy`）收口入 `domain.rs`；② `t_db_c6_limit_and_timeout_constants_are_aligned` 改字面值锁定 + 别名存在性断言（避免"自比退化"，代码见 A2 v3 §13.5） |
| `src-tauri/src/scheduler.rs` | 修改 | **步骤 1**（`M5-1.b`）：抽 `RootsProvider` trait；`bridge::allowed_roots(app)` 改 trait 方法；**此时反向边彻底解除** |
| `src-tauri/src/script_runner.rs` | 修改 | **步骤 1**（`M5-1.b`）：定义 `ProgressSink` trait；`tauri::Emitter` 改 trait 方法 |
| `src-tauri/src/workspace.rs` `src-tauri/src/sync.rs` `src-tauri/src/tasks.rs` | 修改 | **步骤 1**（`M5-1.b`）：抽 `PathResolver` trait；`app.path().data_dir()` 改 trait 方法 |

> 阶段二（`M5-1.c` 可选）：提升为根 `Cargo.toml [workspace] members = ["./src-tauri", "./crates/core"]`；**待 A0 在 M5-1.c 拍**（不在 W1 范围）。A1 不强行推进。

### 实施序（[W1 patched] A0 W1 boundary-first）

1. **步骤 0**（A2 产品代码）：A2 先建 `scripts/check-core-boundary.py`（含 `--self-test` 2好+2坏+1阴/默认扫描/`--expect-pending` 三模式）并挂 `scripts/pre-merge.sh` —— **本步是后续所有 core 内操作的准入前置**，挂载后 `pre-merge.sh` 跑空集必 PASS。
   - **[W1 协同发现 · 2026-09-06 08:50 CST]**：A2 在 W1 期间已交付 `scripts/check-core-boundary.py`（591 行，工作树 untracked，待 A0 拣入），含 `--self-test` / `--expect-pending` / 默认扫描三模式，7 ACTIVE 码位 + 2 PENDING 码位，契约来源已明示 A1/A2 文档。**A1 卡的"步骤 0 待 A2 写"已**事实完成**，仅待 A0 拣入。**A1 W1 修订不在此文件改动（与 A0 allowed scope 不冲突——A1 仅标注事实）。
2. **步骤 0b**（`M5-1.a` 第一 commit）：收口 5 个常量（`HARD_GRACE_SECS` / `MAX_TIMEOUT_SECS` / `MAX_TEXT_FIELD_BYTES` / `DB_MAX_TEXT_FIELD_BYTES` V-7 副本 / `DB_SOFT_TO_HARD_GRACE_SECS` V-7 副本）到 `domain.rs`，改 `t_db_c6_limit_and_timeout_constants_are_aligned` 防自比退化。
3. **步骤 0a**（`M5-1.a` 第二 commit）：建 `src-tauri/src/core/mod.rs`（或 `src/lib.rs` 根）+ A 类 10 模块整文件带 `#[cfg(test)]` 搬入。
4. **步骤 1**（`M5-1.b`）：抽 `RootsProvider` / `ProgressSink` / `PathResolver` trait，**解除 `scheduler → bridge::AppState` 反向边 8 处 + 测试反向 import 2 组**。
5. **步骤 2**（`M5-1.b`）：B 类模块 seam 改造。
6. **步骤 c**（`M5-1.c` 阶段二，可选）：根 `Cargo.toml [workspace]` 化（须 exclude `tauri-browser-tabs/`）。

> 步骤 0→0b→0a→1→2 不可换序：换序则步骤 0b 后的常量引用在步骤 0a 完成前编译失败，步骤 0a 完成的 core 边界在步骤 1 前不构成"反向边解除"。步骤 0（`check-core-boundary.py`）独立于其它步骤，可与 0a/0b 并行准备。

---

## 4. FORBID

- **不**引任何 Tauri 依赖入 `mvp-core`（含 `tauri`/`tauri-build`/任何 tauri plugin crate）
- **不**在 core 内 `use crate::bridge` 或任何二进制模块（解除反向边）
- **不**嵌套 workspace：`tauri-browser-tabs/` 已是另一 workspace，根 `Cargo.toml [workspace]` 若建必先 exclude 该路径
- **不**搬 C 类（`bridge`/`main`/`terminal`/`grid_process`/`tools`/`shutdown`）
- **不**在切片内静默丢弃调用：M4 护栏（无第二执行路径 / `check_invocation_source` / Keyring / 审计脱敏 / 结果上限）随模块一并搬入且测试随迁
- **不**改 `audit.json` 1000 上限（K5）、不破 `tick 内禁写审计`（A6 冻结）
- **不**改 `last_fired_at` 判重真相源（A6 冻结）
- **不**改 `stop-scheduler` 注册在 `stop-background-workers` 之后的注册序（A7 冻结）
- **不**移动 `NEXT`（仍为 M5-W0）
- **不** push

---

## 5. COMMANDS（实现期跑）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# [W1 patched] 步骤 0 完成后（任何 core 内操作前）
python3 scripts/check-core-boundary.py --self-test    # PASS：2好+2坏+1阴
python3 scripts/check-core-boundary.py                # 默认扫描：空集 PASS

# A. 切片 0a 实施后立即跑（必须空；[W1 patched] C-2 替代原命令 B）
grep -rE '^\s*use tauri' src-tauri/src/core/ | grep -v '^\s*//' | tee /tmp/core-tauri-hits   # 应为空

# A'. 步骤 0b 完成后 V-7 副本已收口
grep -nE 'pub const (DB_MAX_TEXT_FIELD_BYTES|DB_SOFT_TO_HARD_GRACE_SECS)' src-tauri/src/database.rs   # 应为空（已改为 pub use）
grep -nE 'pub const (DB_MAX_TEXT_FIELD_BYTES|DB_SOFT_TO_HARD_GRACE_SECS)' src-tauri/src/core/domain.rs  # 应命中

# B. [W1 patched] 阶段一 PENDING：cargo tree 必然命中 tauri（[dependencies] lib/bin 共享）
#    阶段二独立 crate 后转 FAIL。
cargo tree -p mvp_browser_os_core 2>&1 | grep -E "tauri|tauri-build" | tee /tmp/core-deps    # 阶段一允许命中；阶段二应为空

# C. [W1 patched] 步骤 1（M5-1.b）后反向边 8 处全解除
grep -nE 'bridge::allowed_roots|use crate::bridge::AppState|&AppState' src-tauri/src/scheduler.rs | tee /tmp/sched-revedges   # 应为空
grep -nE 'use crate::script_runner::HARD_GRACE_SECS|use crate::security_policy::MAX_TEXT_FIELD_BYTES' src-tauri/src/core/domain.rs   # 应为空（已改字面值锁定 + 别名存在性）

# C'. [W1 patched] 步骤 1 后 scripts.rs 测试反向 import 已迁
grep -nE 'use crate::workspace::(load_scripts_at|save_scripts_at|write_body_at|read_body_at|delete_body_at|body_path_in)' src-tauri/src/scripts.rs | tee /tmp/scripts-revedges   # 应为空

# D. 二进制仍全绿
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# E. 边界脚本自检 + 总体门禁
python3 scripts/check-core-boundary.py --self-test
bash scripts/pre-merge.sh   # ALL_PASS

# F. 主应用可构建性
cargo build --manifest-path src-tauri/Cargo.toml --release
```

---

## 6. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | A 类 10 个模块全在 `src-tauri/src/core/`，二进制侧过渡 shim 完整 | 命令 A 0 命中 + `git diff --stat` |
| 2 | **[W1 patched]** 阶段一 PENDING：`cargo tree -p core` 命中 tauri 允许；阶段二独立 crate 后转 FAIL。阶段一用源码级命令 A 替代 | 命令 A 0 命中 + 阶段二后命令 B 0 命中 |
| 3 | **[W1 patched]** `scheduler → bridge::AppState` 反向边 **8 处**全解除（`scheduler.rs:28/259/491/588/617/622/742/761`）+ 测试反向 import **2 组**收口（`domain.rs:1525-1526` 改字面值锁定 + `scripts.rs` 15 处测试反向 `crate::workspace` 迁移） | 命令 C + C' 0 命中 |
| 4 | `cargo test` 全绿（含 `mvp_browser_os_core::domain` 等新 lib target） | 命令 D |
| 5 | **[W1 patched]** `check-core-boundary.py --self-test` PASS（2 好 + 2 坏 + 1 阴，注释行不得误报） | 命令 E |
| 6 | `pre-merge.sh` ALL_PASS | 命令 E |
| 7 | M4 护栏（无第二执行路径 / 凭据 / 审计脱敏 / 结果上限）随迁无遗漏 | 整文件带 `#[cfg(test)]` 一起移 |
| 8 | **[W1 patched]** 切片 0b 完成：5 个契约常量收口到 `domain.rs`（`HARD_GRACE_SECS`/`MAX_TIMEOUT_SECS`/`MAX_TEXT_FIELD_BYTES`/`DB_MAX_TEXT_FIELD_BYTES` V-7/`DB_SOFT_TO_HARD_GRACE_SECS` V-7） | 命令 A' + `grep -n 'HARD_GRACE_SECS' src-tauri/src/core/domain.rs` 命中 |
| 9 | 二进制 `release` 构建通过 | 命令 F |
| 10 | 不破 K1（ACL 末条恒为 `list_artifact_images`）/K3（凭据不出 core）/K5（审计 1000） | 静态断言 |
| 11 | **[W1 patched]** V-7 已收口：`database.rs` 删自有副本，改 `pub use crate::domain::*`；`Duration::from_secs(DB_SOFT_TO_HARD_GRACE_SECS as u64)` 类型转换正确 | 命令 A' |

---

## 7. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| 反向边未解 | 阻塞合入；`scheduler.rs` 仍是 M5 全部能力的拦路虎 |
| `cargo tree -p core` 命中 tauri | 阻断：core 必须纯逻辑 |
| 任一护栏随迁遗漏 | 阻断：必须整文件带测试迁；M4 护栏是 M5 基础 |
| `check-core-boundary --self-test` 不全检出合成坏样本 | 修脚本（不修实现避过）；脚本必须真起作用 |
| 切片 0a 后 `cargo test` 失败 | 改回单 crate；先找编译错根因再推进切片 1 |
| `withGlobalTauri` 全局暴露导致 core 边界漏 | 阻塞：先评估 `withGlobalTauri` 是否可按 webview 关闭（M5-10 决定） |

---

## 8. DOC_BACKWRITE

实施期（不在 A1 W0 范围）需回写：

1. `详细设计与实施计划.md` L562 把 `[ ]` 改 `[x]`，并补"切片 0a/0b/1 完成度"备注
2. `后续需求TODO.md` §7 状态从 `TODO` 改 `PARTIAL`（仅 #7 A2P/A2A 的子集）
3. `AI-模型切换与接手清单.md` §1 NEXT 移至 `M5-1.b`（或 `M5-2`，由 A0 拍）
4. `logs/checkpoints/M5-1.a-2026MMDD-HHMM.md`（实施卡 checkpoint，按 §5 模板）
5. `M5-14-debt-ledger.md` 减少"R1 反向边"项（如未减，标注 "M5-1.a 仍残留 N 处"）

---

## 9. COMMIT / NEXT

- **COMMIT**：本卡文档无 commit；A0 拣入后，签 `M5-1.a` 实施卡的 commit 由 A2（或指派）填
- **NEXT（本卡完成后由 A0 拍；[W1 patched] 编号已拆三档）**：
  - 若 A0 签 `M5-1.a`（**步骤 0b + 步骤 0a**，含常量收口 + A 类搬入）→ 切回 A2 实施；**此时反向边仍未解**（属 `M5-1.b`）
  - 若 A0 签 `M5-1.b`（含**步骤 1 抽 trait 解反向边** + 步骤 2 B 类 seam）→ 切回 A2 实施，工作树需注意 `scheduler.rs` 改动可能与 A7 冲突
  - 若 A0 签 `M5-1.c`（阶段二 workspace 化，可选）→ 独立 commit
  - 若 A0 跳 `M5-1` 直发 `M5-2` → 反向边未解，**阻断**（除非 A0 显式接受"反向边未解"风险）

---

## 10. 反向边与契约常量随迁清单（给实施者参考）

> 不写实现，仅列需随迁的精确行号（A2 v3 prework §1.1 + §4 + §13.4 V-7 + §13.5 整理）

### 反向边（8 处 + 2 组测试反向 import）· [W1 patched] C-5

| 文件:行 | 形态 | 解除方式 |
|---|---|---|
| `scheduler.rs:28` | `use crate::bridge::AppState;` | `M5-1.b` 步骤 1 切 `RootsProvider` trait |
| `scheduler.rs:259` | `bridge::allowed_roots(app)` 调用 | 改 trait 方法 |
| `scheduler.rs:491` | 同上 | 同上 |
| `scheduler.rs:588` | `&AppState` 取 | 改注入 |
| `scheduler.rs:617` | 同上 | 同上 |
| `scheduler.rs:622` | 同上 | 同上 |
| `scheduler.rs:742` | 同上 | 同上 |
| `scheduler.rs:761` | **[W1 patched] 补**：同模式 `&AppState` | 同上 |
| `domain.rs:1525-1526` | 测试反向 import `script_runner::HARD_GRACE_SECS` + `security_policy::MAX_TEXT_FIELD_BYTES` | `M5-1.a` 步骤 0b 收口 + 改 `t_db_c6_limit_and_timeout_constants_are_aligned` 字面值锁定 + 别名存在性 |
| `scripts.rs`（15 处测试） | 测试反向 import `crate::workspace::{load_scripts_at, save_scripts_at, write_body_at, read_body_at, delete_body_at, body_path_in}` | `M5-1.b` 步骤 2 抽 `PathResolver` trait 或 `scripts.rs` 整体迁入 core（取决于 A2 §3 裁定） |

### 契约常量（5 个，切片 0b 必迁；[W1 patched] C-4/V-7）

| 常量 | 现位置 | 迁至 | 备注 |
|---|---|---|---|
| `HARD_GRACE_SECS` | `script_runner.rs` | `domain.rs` | W0 已列 |
| `MAX_TIMEOUT_SECS` | `script_runner.rs` | `domain.rs` | W0 已列 |
| `MAX_TEXT_FIELD_BYTES` | `security_policy.rs` | `domain.rs` | W0 已列 |
| `DB_MAX_TEXT_FIELD_BYTES` | `database.rs:45` | **删副本**，`pub use crate::domain::{...};` | **[W1 patched] V-7 副本**（值同为 65 536） |
| `DB_SOFT_TO_HARD_GRACE_SECS` | `database.rs:52` | **删副本**，`pub use crate::domain::{...};` | **[W1 patched] V-7 副本**（值同为 5，但**类型不同** `u32` vs `u64`） |
| `MAX_QUERY_*` 系列 | `database.rs` | 已在 `domain.rs`（M4-1 已冻结） ✅ | W0 |
| `SCHED_*` 系列 | `domain.rs` | 已在 `domain.rs`（M4-7 已冻结） ✅ | W0 |

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开 + M5-W1 docs-only reconciliation，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档
- 未移动 `NEXT`（仍 M5-W1，等 A0 拣入后改）
- 未提交、未 push
- 与 A2 prework 文档无文件冲突（`logs/assist/A2-M5-core-*.md` 与本卡分别在 `assist/` 与 `checkpoints/M5-20260906/`）
- **[W1 patched] W1 修订仅加 `[W1 patched]` 段 + 局部行内修正（§0 编号、§1 模块数、§3 写入表与实施序、§5 命令、§6 判据、§9 NEXT、§10 反向边表 + 常量表），未重写 §1~§11 决策史**
