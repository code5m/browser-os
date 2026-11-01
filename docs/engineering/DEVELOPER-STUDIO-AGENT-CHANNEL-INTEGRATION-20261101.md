# BrowserOS 集成重构验收记录 — 2026-11-01

**集成分支**: `feature/perf-monaco-worker-internalization-20261031`。
**参考**: `research/architecture-reconstruction-audit-20261101`。
**复用来源**: `feature/perf-agent-channel-isolated-20261031@9e81a9cf`，原始两个 Rust Crate 的 19 个文件通过 Git 对象逐一复用，未修改来源分支。

## 已提交代码范围（不是最终 GUI 验收声明）

1. 共享 Monaco Worker/loader、Document Model 和 Document Manager。Workspace 的文件预览、编辑器和 Coding Workbench 接入同一绝对路径文档缓存，使用版本号、脏状态、订阅和保存后核对；文件授权仍由原业务 Owner 执行。
2. Developer Studio 使用既有 Capability Contribution 渲染 Coding/Database/Git/Terminal 工作台。Browser 维持现有独立 Native Host，不克隆 WebView。
3. 浏览器网页和内部模块使用同一不可变 Tab 投影供页签条展示、激活和关闭操作。Browser/Module 各自保留实际资源 Owner，避免第二份 WebView 状态。
4. Agent Channel 和 Agent Transition Rust Crate 接入 Tauri 编译依赖，现有 Coding Agent 的工具调用经共享 ToolCatalog 进行精确作用域的一次性授权；其模型循环、源码校验、文件审批与应用仍由原代码负责，不启用历史未实现的 agent_run。
5. Developer Studio 的 Browser/Database/Git/Coding Agent 使用一个有界任务队列、统一模型配置和显式授权的只读上下文；绝不从模型文本直接执行外部副作用。写操作继续在既有业务模块审批。
6. Grid 复用并行分支已审阅的 UDS 代际与断连恢复，增加最多 64 条 pending 请求的背压、p95/p99 滚动延迟和超时统计；不对非幂等命令自动重试。Grid 的 eval_in_tab 转入 spawn_blocking，避免在同步命令线程等待 UDS。
7. 本次不更改锁定的 GTK/WebKitGTK 子 WebView 显隐、尺寸、层级与事件处理。

## 验证命令

```bash
node tests/check-agent-task-runtime.mjs
node tests/check-unified-tabs.mjs
node tests/check-shared-document-manager.mjs
npm run check:developer-studio
npm run check:coding-workbench
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path crates/browseros-agent-channel/Cargo.toml
cargo test --manifest-path crates/browseros-agent-transition/Cargo.toml
bash scripts/pre-merge.sh
npm run verify:client
```

## 必须记录而不能假设的验收

- 真实 Ubuntu GNOME 的安装版冷启动、主窗口可绘制，WebView 关闭/切换/宫格 4 格连续运行。
- 同机同负载前/后 CPU、RSS、PSS、UDS pending/p95/p99、Grid PID 与重启次数、脚本同时执行时 UI 延迟。
- Workspace 与 Coding 同文件交替编辑、未保存草稿、外部写冲突、跨文件 LSP 跳转；SQL 语法/查询事务审批、Git 确认、PTY 恢复。
- Browser Agent 数据来源与授权、Database Agent 无副作用、Git Agent 只读和 Coding Agent 文件提案审批；模型取消与长时间网络请求。
- 需要评估 Capability Runtime / Hot-Plug 的异步生命周期和 DESTROY 语义（审计已有 P0 缺口），不能把本次编辑器/Agent Channel 接线等价于全平台 Owner/权限治理完成。

**本次文档登记状态**: 代码集成候选；独立开发机当前离线，不能声称本机 GUI_PASS；GitHub Actions 按具体提交和 Job 检查，通过后才可标 CODE_PASS。
