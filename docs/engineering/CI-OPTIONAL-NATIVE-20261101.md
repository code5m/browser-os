# BrowserOS 可选原生 CI 与单次构建契约（2026-11-01）

## 目标

Pull Request 和 master 的 **默认 Full Validation 只进行静态/安全/前端/体积预算/快速 Pre-merge 检查**，不编译 Rust、不构建或安装 Debian 包、不启动安装版 GUI，也不做 5/30 分钟持续资源采样。

当用户明确启用一种模式时，**一次工作流最多有一条 Rust 构建路径**。前端针对同一源码提交仅构建一次，结果作为带有 SHA 标识的 GitHub artifact 提供给后续检查。现有同一轮中的 master 基线体积比较仍会独立构建 **另一个提交** 的前端：它属于回归比较，不是重复构建当前提交。

## 选择模式

| 模式 | 默认 | 本次真正执行什么 | 本次不执行什么 |
| --- | --- | --- | --- |
| `off` | **默认** | 现有工程治理、UI/架构/运行时/前端/大小预算、Git、快速 Pre-merge | **Rust check/test、Release deb、安装版 GUI、持续资源采样** |
| `rust-tests` | 按需 | 复用当前前端产物，执行 **一次** `cargo test --locked`（包含编译和运行测试），Rust 格式检查 | 不额外 `cargo check`；不构建 Release、不做 GUI 或资源采样 |
| `packaged-gui` | 按需 | 复用当前前端产物，执行 **一次** Tauri Release 构建、deb 安装、GUI 冷启动和完整 GUI 回归 | 不重复执行 Tauri 的 frontend beforeBuild，不做 5 分钟持续资源采样，也不额外运行 Rust 单元测试 |
| `packaged-resource` | 按需 | 与 `packaged-gui` 相同的**一次** Release 构建，另外执行 5 分钟 RSS/PSS/CPU 采样 | 不重复编译第二个 Release，不额外运行 `cargo test` |

因为 Rust 测试二进制和 Release 安装包是**不同构建目标/配置**，不能诚实地声称一次编译覆盖两者。需要发行级全套证明时，在相同 Git SHA 分别运行 `rust-tests` 与 `packaged-resource` 两次手动工作流，分别要求通过。**单次只选择一种构建**，不把多个 Cargo 编译堆在同一次 CI 中。

## 如何选择

- **手动**：仓库 GitHub Actions → BrowserOS Full Validation → Run workflow → `native_mode` 选择上述模式；默认 `off`。Workflow Dispatch 在该工作流合入默认分支后可用。
- **单个 PR**：给 PR 加上 `ci:native-tests`、`ci:packaged-gui` 或 `ci:packaged-resource` 中**一个**标签；加/删标签触发重验，多个冲突标签会直接 FAIL，不会悄悄选一个。
- **统一配置**：仓库 Actions 变量 `CI_NATIVE_MODE`，可设为 `off`、`rust-tests`、`packaged-gui` 或 `packaged-resource`；缺省 `off`。
- 优先级：workflow_dispatch 的显式选项 ＞ PR 标签 ＞ 仓库变量 ＞ `off`。不允许其它值。

`BrowserOS Developer Feedback` 的 Quick Gate 仍然在普通 Push 上运行，但自动 deb Preview **已关闭**。需要下载预览包时手动 Run workflow，显式设置 `build_preview=true`。独立持续资源测评工作流也改为 **手动运行**，不再有每周自动计划。

## 去重和安全

1. `frontend-build` 只执行一次 `npm ci && npm run build`，上传 `dist/` 并带有当前 Git SHA 的 `browseros-ci-source-sha.txt`。
2. `validate` 和 `native-regression` 必须下载同一个 SHA 标识的产物，并验证标记与工作树提交一致；缺失、错 SHA 或空 `dist/index.html` 都 FAIL。
3. 默认 `validate` 完全不安装 WebKit 原生开发依赖，不触发 Cargo 编译。Pre-merge 在 CI 里显式设置 `PREMERGE_REUSE_FRONTEND=1` 和 `PREMERGE_SKIP_NATIVE_COMPILE=1`，复用已校验的前端产物。**本机原脚本默认行为不变：仍会自己构建前端并 cargo check。**
4. 当 `packaged-*` 开启时，安装包脚本使用新增 `--prebuilt-frontend`：仅匹配当前 Git SHA 的 dist 才能覆写 Tauri `beforeBuildCommand` 为空，否则拒绝构建。Release 在该原生任务中只编译一次，GUI 回归用 `--no-build` 复用该 Release。
5. 输出明确展示实际 mode；`SKIPPED` 不等于 `PASS`。Release 二进制体积、RSS/PSS、GUI 证据在未启用模式时必须报告 `UNKNOWN`，而不是取 0、沿用上一次产物或宣称通过。
6. 默认保留代码架构、工程治理、供应链、静态边界、前端体积和来源证明等门禁，不为了提速删掉安全策略。

## 验收与权衡

- `python3 scripts/check-ci-native-mode.py --self-test` 验证配置优先级、非法值、多标签冲突。
- `python3 scripts/check-ci-validation-contract.py` 防止“默认重新编译”“重复前端构建”“打包无来源检查”等规则回归。
- `bash -n scripts/pre-merge.sh scripts/verify-installed-client.sh`、`npm run check:engineering-governance`、`git diff --check` 验证脚本和治理基础。
- 在本地或 GitHub 执行快模式 Pre-merge 要 **先构建当前源码的 dist 并写入 SHA 标记**；错误标记必须 FAIL。
- 修改 CI 后必须看真实 PR 的 `impact`、`frontend-build`、`validate` 三个 Job，以及默认 `native-regression=SKIPPED`；再单独触发一个原生模式验证其真实行为。不能以 YAML 解析成功代替 GitHub 的真实验收。

**风险提示**：选择 `off` 的绿色 Full Validation 只能说明“快速/静态检查通过”，**不证明 Rust 编译、安装版 GUI 或资源测试通过**。若 master 或 Release 的正式验收仍要求原生结果，请额外在相同 SHA 选择运行相应模式并审查 artifact。是否把这些原生结果设为 GitHub Branch Protection 的硬性合并条件，需要仓库管理员另行配置；本次不会擅自更改该设置。
