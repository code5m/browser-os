# BrowserOS P0–P2 性能 TODO / 实施记录

**基线**：`master@81a9feca`；**日期**：2026-10-31；**目标分支**：`perf/p0-p2-20261031`。

状态定义：`[x]` = 代码已提交 **且** 必要验收通过；`[ ]` = 未完成或未验证。分支有代码不等于已通过测试。

| 编号 | 优先级 | 工作项 | 本轮实现/证据 | 状态 |
| --- | --- | --- | --- | --- |
| PERF-01 | P0 | Tauri Grid create/close/open/position 非阻塞入口 | `bridge.rs` 新增 async + spawn_blocking；保留 blocking 内部函数供原生测试使用，需 CI/GUI 验证 | [ ] |
| PERF-02 | P0 | 资源监控线性进程树聚合与线程迁移 | `process_tree_rss_kb` + 嵌套进程单测、后台 worker；**新增 750ms PID 感知缓存**；待测试缓存失效及内存读数准确性 | [ ] |
| PERF-03 | P0 | Grid 定位 latest-only 合并/单飞/限频 | `useGridHost.ts` 单飞最新状态；**仍需会话重建、resize/zoom 真机回归及限频验证** | [ ] |
| PERF-04 | P1 | Grid 渐进启动与复用/生命周期 | 仅保留原有 300ms 错峰 + 内存守卫；尚未实现复用 | [ ] |
| PERF-05 | P1 | LSP 持久安全 Session | 仍为每次新建 bwrap/rust-analyzer，必须先验证快照与隔离 | [ ] |
| PERF-06 | P1 | Monaco editor 实例复用/增量 worker | 普通编辑器已实现跨文件 setModel 复用；还需 TS/JS worker 增量更新及 UI 回归验证 | [ ] |
| PERF-07 | P1 | 文件树大目录虚拟化/分批/刷新 | 已将 `list_dir` 放到 worker，`read_file` 放到 worker + 限 8 MiB；虚拟化/分页未完成，需兼容性验证 | [ ] |
| PERF-08 | P1 | Agent 按 session 锁/Provider 有界并发与流式 token | 尚未实施；不得取消文件写入锁及审查 | [ ] |
| PERF-09 | P2 | 自适应内存压力休眠 | 尚未实施；不可丢失网页未保存表单 | [ ] |
| PERF-10 | P2 | 基线/性能 CI/安装版 5、30 分钟实测 | 现有采样脚本可复用；尚无本轮前后对比指标 | [ ] |

## 本轮变更与验证流水
- `97c714b2` — 后台化耗时 Tauri 命令，进程树 O(N) 汇总，大文件防护及进程树单测。
- `5448cbc2` — Grid 矩形 IPC 最新状态合并（仍需验证重启与定位时序）。
- `4e833a90` — 普通 Monaco 编辑器在文件切换时 setModel 复用。
- `4c79c782` — 750ms PID 感知的 ResourceStats 缓存。
- 源码获取：GitHub connector，可读取与提交。
- Ubuntu 桌面：**离线**；GUI 验证：`GUI_PENDING`。
- 自动化 CI：已触发，但尚未确认成功；Rust fmt/check、JS checks、pre-merge：**待结果**。
- 实测改善数据：**无**。不得声称性能提升 x% 或 P0–P2 全部完成。

## 未完成工作与复核重点
1. 立即确认各 GitHub Actions job 结果；有失败则定位原因，修复后重新运行。
2. 检查 Tauri Command 包装与 `main.rs` 内部调用签名，保证异步调用契约、权限不变。
3. 针对 Grid 关闭重开、不同 session/布局、快速拖动和缩放验证最新状态与错误重试。
4. 完成 PERf-02 缓存限频及 PERf-04~10 的实现与测试，不得将规划当完成。
5. Ubuntu 设备在线后跑发布版、真机 GUI、性能基线及对比；记录硬件环境、PSS/RSS、p95、原始日志与截图。
6. 全部门禁通过后再合并 `master`；受保护分支不执行强推。

## 结果准则
- `CODE_PASS`：构建与自动门禁实际通过。
- `PACKAGE_PASS`：实际打包成功并验证。
- `GUI_PENDING`：真实桌面尚未验收（当前）。
- `GUI_PASS`：用户确认桌面实测。
- `BLOCKED`：CI 失败、安全不变量破坏或必要验收条件不满足。

**结论（截至此记录）：P0–P2 未完成；已有部分 P0/P1 代码提交，但完整功能与真机验收仍未完成。**
