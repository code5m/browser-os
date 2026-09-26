# Capability Native Module: `git` (Rust)

> 迁移自 `src-tauri/src/sync.rs`（Native Physical Boundary Matrix Pilot 12）。
> 分类：**CAPABILITY_NATIVE(git)**（矩阵 §2.3；target `src-tauri/src/capabilities/git/`）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.3 / §8.15。
> 同级 TS 能力：`src/capabilities/git/`（成熟度 C1，命令接线仍经 `bridge.ts`）。
> 注意：本模块是 **Git 同步内核**（M-sync），**无 `#[tauri::command]`**；同步命令体在 `bridge.rs`。

## 1. DDD 职责（Domain Responsibility）

Git 能力的原生实现，目前迁入 `sync.rs`（Git 同步内核）：

- `repo_dir(&app, repo_id)`：解析配置仓库本地目录。
- `sync_repo` / `sync_repo_full`：把 artifacts 推到配置仓库（commit / push / token 取用）。
- 凭据经 `keyring_store`（token），不落 DSN；只读 git repo dir，不持有 git 能力语义。

## 2. 边界（Boundary / Non-Responsibility）

- **不含任何 `#[tauri::command]`**：同步命令（`request_sync` / `confirm_sync` 等）命令体在 `bridge.rs`（经 `generate_handler!`），本模块只提供内核。
- **不持有 git 能力语义**：只做「把 artifacts 推到配置仓库」的执行，Git 领域抽象归 `domain` / 前端 `useGitStore`。
- 禁止改写非同步目标文件（`check-git-write-policy.py` 机器守护：只允许 push artifacts，禁止脚本改 `src-tauri/src` 下非同步文件）。
- 无 WebView / PTY / socket / 子进程（git 二进制调用除外，受 `security_policy` 约束）。

## 3. Commands

**本模块无命令。** git/sync 命令体归属 git 能力，注册在 `bridge.rs`（经 `generate_handler!`），命令体调用本模块内核。命令体在 **bridge.rs 逐 command 分解阶段**迁移至 `capabilities/git/commands.rs`。

## 4. Resources

- 文件系统：配置仓库本地目录（git repo dir）；`git2` 操作。
- OS keyring：`keyring_store` 取用 git token（不落盘明文）。
- 无持久化 AppState 字段（sync.rs 零 `bridge::AppState` 耦合；命令体以局部句柄持有）。

## 5. 生命周期（Lifecycle）

- 启动：无长生命周期资源；命令体触发时解析 repo dir + 取 token。
- 运行：`git2` clone/fetch/commit/push；失败回滚本地未推送改动。
- 退出：无残留（git 操作同步完成）。

## 6. 依赖（Dependencies）

- `crate::domain`：artifact / repo 类型（SHARED）。
- `crate::keyring_store`：git token 取用（SHARED_NATIVE_INFRASTRUCTURE）。
- `crate::workspace`：repo dir 解析依赖（CAPABILITY_NATIVE，经 shim）。
- 外部 crate：`git2` / `tauri`（`AppHandle` / `Manager`）/ 标准库。
- **零 `bridge::AppState` 耦合**（grep 确认 sync.rs 无 `app.state` / `State<` / `bridge::AppState`）。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（AppState 共享态枢纽 / 命令 hub）。
- 禁止改写非同步目标文件（仅允许 push artifacts）。
- 禁止 token 明文落盘（一律经 `keyring_store`）。

## 8. Public / Native Contract

- 内核接口：`repo_dir` / `sync_repo` / `sync_repo_full` + 辅助。
- 既有 `crate::sync::` 调用点（workbench_smoke.rs ×1）经 `main.rs` 顶部 `pub use crate::capabilities::git::sync;` re-export shim 解析，**未逐处改写**。
- TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。

## 9. Security / ACL

- git/sync 命令在 `permissions/default-commands.toml` 放行（与 `bridge.ts` / `src/types.ts` 镜像一致）。
- 禁止脚本改写 `src-tauri/src` 非同步文件（`check-git-write-policy.py` ACTIVE 守护）。
- 门禁真源：`scripts/check-git-write-policy.py`（扫 sync.rs / domain.rs / bridge.rs / main.rs / ACL）。

## 10. Tests

- `sync.rs` 内 `#[cfg(test)] mod`（line ~1017 / ~1234：repo_dir / sync 幂等 / token 取用）。
- 运行：`cd src-tauri && cargo test capabilities::git`（或 `cargo test` 全量）。

## 11. Source of Truth

- Git 同步内核：`src-tauri/src/capabilities/git/sync.rs`（本目录）。
- 命令体（暂留）：`src-tauri/src/bridge.rs`（`request_sync` / `confirm_sync`）。
- 前端接线：`src/bridge.ts`（`requestSync` / `confirmSync`）+ `src/types.ts` + `src/stores/useGitStore.ts`。
- 门禁真源：`scripts/check-git-write-policy.py`。

## 12. Known Debt

- git/sync 命令体仍在 `bridge.rs`（LEGACY_MIXED_MODULE 残核）；属矩阵 §8 末段 bridge 分解计划。
- 本目录仅 `sync.rs` 一个文件；若后续 git 能力扩展（如多 remote、status 查询）再补 `git.rs` 同目录。

## 13. Extraction Readiness

- **高。** 纯同步内核、零命令、零 AppState 字段、零 `bridge::AppState` 耦合；跨模块依赖仅 SHARED（`domain` / `keyring_store` / `workspace`）+ 标准库 + `git2` / `tauri`。
- 达到阈值后可随 `git` 能力提级为 crate `mvp-git-rust`（不改领域语义）。
- 同能力下一个低风险同批候选：git 命令体 `commands.rs`（需先解除对 `bridge` hub 的耦合，见矩阵 §4 / 协议 §8）。
