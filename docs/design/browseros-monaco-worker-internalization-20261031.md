# BrowserOS Monaco Worker 增量同步：隔离实施记录

日期：2026-10-31
基线：perf/p0-p2-20261031 @ 0904c5ad22d4b1da92919dbcd052bfe9aede24af
实施分支：feature/perf-monaco-worker-internalization-20261031

## 本次边界

- 仅改动 Monaco 专有的 codingEditorServices.mjs，新增纯策略模块 codingScriptModelSync.mjs、独立测试和本记录。
- 不改既有 CodingCodeEditor.vue、CodingWorkbench.vue、其他业务、Grid、Agent Runtime、Tauri/Rust、IPC、依赖、预算、CI 门禁与工程治理资产。
- 不合并 master、不修改原 perf 分支，不自动创建 PR。
- Monaco 仍是 TypeScript/JavaScript 的浏览器编辑器；无需改写 Rust。
- 当前属于 PERF-06 的局部实施，不声明整个 PERF-06、P0-P2 或安装版验收完成。

## 内化的能力

BrowserOS 自有模型同步策略独立于 Monaco 的 UI 和 Worker 加载机制：
1. 以 URI、模型对象身份、语言类型和版本号识别变更。
2. 对每个 TS/JS Worker 分别记录成功确认的文件版本；跨 TS/JS 引用所需的全部已打开脚本文件仍可被两个 Worker 接收。
3. 文件切换时不重复提交未改变的模型 URI；新增、重开、修改模型才提交增量。
4. 文件关闭、模型销毁、Worker 终止或所有 Editor 服务释放时失效缓存。
5. 同步请求串行化，失败不提交确认态；请求执行期间 Worker 重启或文档修改不得登记过期版本。
6. 保留原有 TypeScript/JavaScript 默认编译参数、eagerModelSync 和语言服务初始化重试，避免将性能优化变成语言能力退化。

## 结果与限制

独立纯策略测试覆盖重复切换、TS/JS 混合文件、版本变化、文件重开、关闭、失败、并发与 Worker 重启。
模拟 40 个 TS 模型、连续 20 次请求时，原方法每次传 40 个 URI，共 800 次 URI 请求；新策略仅第一次传 40 个，后续不重复请求。
**这是模拟的 URI 请求量，不是 CPU、RSS、延迟、真实进程或桌面 GUI 指标；绝不能称为提速 95%。**

- 定向测试：node tests/check-coding-script-model-sync.mjs
- 原有编码工作台逻辑：npm run check:coding-workbench
- 前端构建：npm run build
- 真实 Monaco + TS Worker UI：npm run check:coding-ui:managed（需要可运行的 Chromium / Playwright 环境）
- 完整预合并门禁：bash scripts/pre-merge.sh
- 安装版：按项目规则另行在 Ubuntu 上验收

## 验收状态

纯策略模块已通过独立 Node 测试（非 BrowserOS 安装版运行）。
完整仓库构建、真实 Monaco Worker UI、Rust/CI 和 Ubuntu 安装版不由本次纯策略测试证明；结果以对应 CI 与设备验收为准。
原 perf/PERF-06 中没有被本次覆盖的 eager Worker 协作和整体性能基准继续保持待验收状态。
