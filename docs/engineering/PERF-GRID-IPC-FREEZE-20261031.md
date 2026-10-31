# Grid 卡死专项：UDS 重连竞态与编辑器模型同步

基线：`perf/p0-p2-20261031@0904c5ad`。独立工作分支：`fix/perf-grid-uds-stability-20261031`。

## 已提交（代码实现，尚未证明 GUI PASS）

1. `src-tauri/src/grid_process.rs`：为 UDS reader 与 writer 引入代际序号。旧 reader 在新连接建立后退出，不得删除新连接的 writer，也不得取消新连接的请求；旧连接中的缓存响应/焦点事件同样不能继续处理。
2. 监听器的查找/绑定/登记放在同一个临界区，避免并发启动相同 Grid 时重复绑定 socket。
3. 新连接 attach 时立即失败旧请求；将请求的 pending 登记移到 writer 锁内，统一 writer → pending 锁顺序，避免重连时请求加入错误连接。
4. UDS writer 设置 2 秒写超时。请求响应等待超时后不再直接盲目重试，因为首次请求可能已经执行。
5. `src/capabilities/agent/model/codingEditorServices.mjs`：禁用 TS/JS Monaco worker 的 `setEagerModelSync(true)`，继续通过 `syncOpenScriptModels` 显式同步已打开的 JS/TS 模型；跨文件跳转、诊断和增量变更需要浏览器回归验证。
6. 增加 Rust `grid_connection_generation_tests`：覆盖旧 reader 退出不能关闭新连接、不能误取消新请求；当前为**待执行的回归用例**。

## 风险与必要验收

- 检查 Rust fmt/check/test：特别是新的请求写入/断开处理、重连代际锁序与复合并发条件。
- Browser/React/Vue 工作台 TS/JS Worker 真实 Chromium UI：跨文件补全、跳转、诊断、关闭后模型销毁；不允许因降资源而损坏编辑器能力。
- 真机 Linux WebKitGTK：Grid 反复启动/关闭、故障重启、断连后重连、连续 resize、正常页面请求与旧连接迟到断开。
- 不在此 PR 修改 Grid/Agent 对外命令协议、全局资源治理、`master`、原 `perf` 分支。
- 未取得真实 CPU/PSS 和 p95 测量前，不声称获得具体性能提升或卡死问题彻底解决。

## 回归路径

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo test --manifest-path src-tauri/Cargo.toml grid_connection_generation_tests
cargo check --manifest-path src-tauri/Cargo.toml
npm run check:coding-workbench
# 在预配置 Chromium/Playwright 的 Vite 测试环境运行 TS Worker UI 用例
# 安装版 Linux Grid 断连/重连与资源采样另行验收
```

结论：代码修复已推送到隔离分支，自动测试和安装版 GUI 尚待实际结果，不把静态检查或单测代码存在当作 PASS。
