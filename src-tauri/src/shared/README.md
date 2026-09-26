# Shared Native Infrastructure Module: `images` (Rust)

> 迁移自 `src-tauri/src/images.rs`（Native Physical Boundary Matrix Pilot 4）。
> 分类：**SHARED_NATIVE_INFRASTRUCTURE**（矩阵 §2.1；target `src-tauri/src/shared/`）。
> 对照矩阵：`docs/architecture/native-physical-boundary/NATIVE-PHYSICAL-BOUNDARY-MATRIX.md` §2.1 / §8.5。
> 本目录（`shared/`）承载跨能力共享的原生基础设施模块；`images` 是第一个迁入者，后续 `security_policy` / `session` / `crashlog` / `shutdown` / `grid_ipc` 等同列（矩阵 §2.1）。

## 1. DDD 职责（Domain Responsibility）

图像字节级校验与归一化，作为所有图像落盘/读取前的**安全闸门**（fail-closed）：

- 校验图像字节的容量上限、MIME/类型白名单、尺寸/`exif` 风险、文件名主干形态（`validate_id`）。
- 提供 `ImageRef` / `ListedImage` / `ImageError` 等纯类型与稳定错误码。
- 不解码业务语义，只做"字节是否可信"的判定。

## 2. 边界（Boundary / Non-Responsibility）

- **不解码 / 不渲染 / 不持久化图像像素**：只校验字节元数据。
- **不持有会话语义 / 不引桥**（`crate::bridge`）；不持有 `AppState`、不注册 Tauri 命令。
- **不联网 / 不调外部服务**；不访问 keyring / 文件系统写（仅校验传入的 `&[u8]` / `Path` 形态）。
- 执行/落盘职责归属调用方（`workspace.rs` / `bridge.rs` 的图像命令），本模块只交付"已校验"的信任结论。

## 3. Commands

**无 Tauri 命令。** 图像相关命令（`image_*` / `list_artifact_images` 等）仍注册在 `bridge.rs`，由 `bridge.ts` 经 `invoke` 调用，并在命令体内部调用本模块纯函数。后续 bridge 拆分时再归并（矩阵 §3.2）。

## 4. Resources

无运行时资源（无进程 / 线程 / webview / PTY / socket / sqlite / keyring / 文件写句柄）。纯 CPU + `&[u8]` / `Path` 解析。

## 5. 生命周期（Lifecycle）

无状态、无生命周期。函数式纯逻辑，按调用即时执行、即时返回。

## 6. 依赖（Dependencies）

- `crate::domain` — `ImageError` / `ImageRef` / `ListedImage` 等共享类型（SHARED_NATIVE_INFRASTRUCTURE，类型真源）。
- `chrono` — 时间戳归一化（外部 crate）。
- `std::fs` / `std::path` / `std::fmt` — 仅类型与路径形态解析。
- 调用方（非依赖）：`bridge.rs` / `workspace.rs` / `tasks.rs` / `scheduler.rs` / `capabilities/script/scripts.rs` 经 `crate::images::` 复用 `validate_id` 等。

## 7. 禁止依赖（Forbidden Dependencies）

- 禁止 `crate::bridge`（`AppState` 共享态枢纽）。
- 禁止 `crate::script_runner` / `std::process` / 网络 / 后台 worker。
- 禁止任何 `WebviewWindow` / `State` 引用。
- 禁止反向依赖任何 CAPABILITY 模块（依赖方向单向：能力 → `shared`）。

## 8. Public / Native Contract

- 对外纯函数契约：`validate_id` / 字节容量与类型校验 / `ImageRef` 构造与归一化 / 稳定错误码 `ImageError`。
- 无 FFI / 无 IPC contract。TS 侧经 `bridge.ts` 命令调用，不直接 `import` 本 crate。
- 类型真源：`crate::domain` 中的图像类型（单一，禁止第二份定义）。
- 既有 19 处 `crate::images::` 调用点经 `main.rs` 顶部 `pub use crate::shared::images;` re-export shim 解析，**未逐处改写**。

## 9. 测试（Tests）

- `images.rs` 内 `#[cfg(test)] mod`（约 line 499 起）：字节容量越界、MIME 白名单、危险 exif、文件名主干、`ImageError` 稳定枚举等。
- 运行：`cd src-tauri && cargo test capabilities::shared`（或 `cargo test` 全量）。
- 另有 `bridge.rs` 测试（line ~5892 起）调用 `crate::images::`，随 bridge 全量测试覆盖。

## 10. Source of Truth

- 领域类型：`src-tauri/src/domain.rs`（图像类型）。
- 命令接线（暂留）：`src-tauri/src/bridge.rs`（`image_*` / `list_artifact_images`）。
- 前端接线：`src/bridge.ts`（invoke）+ `src/utils/image.ts`。
- 门禁真源：`scripts/check-image-policy.py`（扫描 `src-tauri/src/shared/images.rs`）。

## 11. Known Debt

- 图像命令仍未迁离 `bridge.rs`（LEGACY_MIXED_MODULE 残核的一部分）；属矩阵 §3.2 计划内的后续批。
- 图像类型真源是否应上提至 `core/`（与 `domain` 同列）待矩阵 §2.1 后续批裁定；本批仅做物理落位，不改领域语义。

## 12. Extraction Readiness

- **高。** 纯模块、零命令、零 `AppState` 耦合、跨模块依赖仅 SHARED（`domain`）+ 外部 crate（`chrono`）。
- 当前已可独立审核；达到阈值后可随 `shared/` 整体提级为 crate `mvp-shared-rust`（不改领域语义）。
- 同列下一个低风险候选：`security_policy.rs` / `session.rs`（SHARED，需先解除对 `bridge` hub 的耦合，见矩阵 §4）。
