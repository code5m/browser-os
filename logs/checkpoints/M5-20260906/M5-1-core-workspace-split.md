# M5-1 核心 workspace 下沉（Core Workspace Extraction）

> 子卡 ID：**M5-1** · 需求 #7（A2P/A2A）的实现前置 · `[S3|LEVERAGE:3|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A13**（A9 提案；A0 签发时定）
> 父卡：`详细设计与实施计划.md` L562（`M5-1 核心 workspace 下沉`）
> 主预研：`logs/assist/A2-M5-core-20260906-0749.md`（10 步 A 路径 + 3 切片 + 4 反向边违例）
> 配套：`logs/assist/A9-M5-split-20260905-2359.md` §5 · `A9-M5-A13plus-cards-20260906-0010.md` A13

---

## 0. 编号与锚定

- 批次任务号 `M5-1`；需求号 #7；WBS L562 一致。
- 依赖：M4 PASS（`a1a2061`）✅、`A2-M5-core-*.md` ✅、A0 签发 `M5-1.a` 实施卡。
- 唯一必先解除的循环依赖：`scheduler.rs` → `bridge::AppState`（`scheduler.rs:26/28/259/491/588/617/622/742`），是 M5 全部能力（MCP/Agent/Graph/Plugin）能否复用 scheduler 的关键。

---

## 1. GOAL

把当前单 crate（23 模块 / 24,859 行 / `main.rs` 扁平 mod）拆为 **`mvp-core` 库 crate**（纯逻辑，零 Tauri 依赖）+ **现有二进制**（Tauri 专属），**主应用始终可独立构建**。彻底解除 `scheduler → bridge::AppState` 反向边；为 M5-2/4/7/10 等后续能力提供"纯逻辑落 core / 命令面与 transport 落二进制"的双层基础。

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
| `src-tauri/Cargo.toml` | 修改 | 加 `[lib] name="mvp_browser_os_core" path="src/core/mod.rs"`；保留 `[[bin]]`；**首切同 package 双 target**（阶段一），避免 workspace 嵌套冲突（坑位：现有 `tauri-browser-tabs/` 已是另一 workspace，**禁止**未排除地嵌套） |
| `src-tauri/src/core/mod.rs` | **新增** | core 入口；re-export 切片 0a 搬入的 10 个 A 类模块 |
| `src-tauri/src/core/domain.rs` 等 | **新增**（移动） | 切片 0a 把 A 类模块整文件带测试搬入 |
| `src-tauri/src/main.rs` | 修改 | 加 `pub use mvp_browser_os_core::*;` 过渡 shim；其余模块的 `use crate::domain` 仍可编译 |
| `src-tauri/src/scheduler.rs` | 修改 | 切片 1：抽 `RootsProvider` trait；`bridge::allowed_roots(app)` 改为 trait 方法；**此时反向边彻底解除** |
| `src-tauri/src/script_runner.rs` | 修改 | 切片 1：定义 `ProgressSink` trait；`tauri::Emitter` 改 trait 方法 |
| `src-tauri/src/workspace.rs` `src-tauri/src/sync.rs` `src-tauri/src/tasks.rs` | 修改 | 切片 1：抽 `PathResolver` trait；`app.path().data_dir()` 改 trait 方法 |
| `scripts/pre-merge.sh` | 修改 | 加 `check-core-boundary.sh` 断言：`grep -rl 'use tauri' src-tauri/src/core` 必须为空；`cargo tree -p mvp_browser_os_core` 不含 `tauri` |
| `scripts/check-core-boundary.sh` | **新增** | 切片 0a 完成后挂载；自检 `--self-test` 必须 2 好样本零违规 + 2 合成坏样本全检出 |

> 阶段二（M5-1.b 可选）：提升为根 `Cargo.toml [workspace] members = ["./src-tauri", "./crates/core"]`；**待 A0 在 M5-1.b 拍**。A1 不强行推进。

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

# A. 切片 0a 实施后立即跑（必须空）
grep -rl 'use tauri' src-tauri/src/core/ | tee /tmp/core-tauri-hits   # 应为空

# B. core 不依赖 tauri
cargo tree -p mvp_browser_os_core 2>&1 | grep -E "tauri|tauri-build" | tee /tmp/core-deps    # 应为空

# C. 切片 1 后反向边已解除
grep -nE 'use crate::bridge' src-tauri/src/core/scheduler.rs   # 应为空

# D. 二进制仍全绿
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# E. 边界脚本自检
bash scripts/check-core-boundary.sh --self-test
bash scripts/pre-merge.sh   # ALL_PASS

# F. 主应用可构建性
cargo build --manifest-path src-tauri/Cargo.toml --release
```

---

## 6. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | A 类 10 个模块全在 `src-tauri/src/core/`，二进制侧过渡 shim 完整 | 命令 A 0 命中 + `git diff --stat` |
| 2 | `mvp-core` 零 Tauri 依赖 | 命令 B 0 命中 |
| 3 | `scheduler → bridge::AppState` 反向边彻底解除 | 命令 C 0 命中 |
| 4 | `cargo test` 全绿（含 `mvp_browser_os_core::domain` 等新 lib target） | 命令 D |
| 5 | `check-core-boundary.sh --self-test` PASS（2 好 + 2 坏） | 命令 E |
| 6 | `pre-merge.sh` ALL_PASS | 命令 E |
| 7 | M4 护栏（无第二执行路径 / 凭据 / 审计脱敏 / 结果上限）随迁无遗漏 | 整文件带 `#[cfg(test)]` 一起移 |
| 8 | 切片 0b 完成：契约常量收口到 `domain.rs`（`HARD_GRACE_SECS`/`MAX_TIMEOUT_SECS`/`MAX_TEXT_FIELD_BYTES` 等） | `grep -n 'HARD_GRACE_SECS' src-tauri/src/core/domain.rs` 命中 |
| 9 | 二进制 `release` 构建通过 | 命令 F |
| 10 | 不破 K1（ACL 末条恒为 `list_artifact_images`）/K3（凭据不出 core）/K5（审计 1000） | 静态断言 |

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
- **NEXT（本卡完成后由 A0 拍）**：
  - 若 A0 签 `M5-1.a`（切片 0a+0b）→ 切回 A2 实施
  - 若 A0 签 `M5-1.b`（含切片 1/2，B 类搬入）→ 切回 A2 实施，工作树需注意 `scheduler.rs` 改动可能与 A7 冲突
  - 若 A0 跳 `M5-1` 直发 `M5-2` → 反向边未解，**阻断**

---

## 10. 反向边与契约常量随迁清单（给实施者参考）

> 不写实现，仅列需随迁的精确行号（A2 prework §1.1 + §4 整理）

### 反向边（4 处）

| 文件:行 | 形态 | 解除方式 |
|---|---|---|
| `scheduler.rs:26` | `use crate::bridge::AppState;` | 切 `RootsProvider` trait |
| `scheduler.rs:28` | 同上 | 同上 |
| `scheduler.rs:259` | `bridge::allowed_roots(app)` 调用 | 改 trait 方法 |
| `scheduler.rs:491` | 同上 | 同上 |
| `scheduler.rs:588` | `&AppState` 取 | 改注入 |
| `scheduler.rs:617` | 同上 | 同上 |
| `scheduler.rs:622` | 同上 | 同上 |
| `scheduler.rs:742` | 同上 | 同上 |
| `domain.rs:1525` | 测试反向 import `script_runner` 常量 | 切片 0b 收口 |
| `domain.rs:1526` | 测试反向 import `security_policy` 常量 | 切片 0b 收口 |

### 契约常量（切片 0b 必迁）

| 常量 | 现位置 | 迁至 |
|---|---|---|
| `HARD_GRACE_SECS` | `script_runner.rs` | `domain.rs` |
| `MAX_TIMEOUT_SECS` | `script_runner.rs` | `domain.rs` |
| `MAX_TEXT_FIELD_BYTES` | `security_policy.rs` | `domain.rs` |
| `MAX_QUERY_*` 系列 | `database.rs` | 已在 `domain.rs`（M4-1 已冻结） ✅ |
| `SCHED_*` 系列 | `domain.rs` | 已在 `domain.rs`（M4-7 已冻结） ✅ |

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
- 与 A2 prework 文档无文件冲突（`logs/assist/A2-M5-core-*.md` 与本卡分别在 `assist/` 与 `checkpoints/M5-20260906/`）
