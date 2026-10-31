# BrowserOS 性能优化审查与落地计划（2026-10-31）

> 基线：`master@81a9feca7c3bb04f0ac0ca591116e963daaabe5a`。本报告是源码审查与实施追踪，**不是**安装版跑分报告。状态以 [TODO](./PERFORMANCE-P0-P2-TODO.md) 为准。

## 目标与原则
- 浏览器、Grid、Monaco、文件工作区、Agent 并发运行时保持可交互；优先减少主线程阻塞、IPC 队列积压、重复计算与不可控的后台资源。
- 保持 WebKitGTK 既有定位/隐藏不变量、每宫格进程隔离、LSP bwrap 沙箱、审批/取消机制、现有 Tauri 参数和前端契约。
- 严禁伪造性能数值：相同硬件与固定工作负载，分别采集优化前后 p50/p95/p99、CPU、RSS/PSS、帧间隔及 5/30 分钟趋势；未测标记待验收。

## 发现的风险
1. **P0 — Tauri 同步命令**：`resource_stats` 扫描 `/proc`，`create_grid` 逐格等待 UDS/WebKit 冷启动且包含 sleep，`grid_position` 等待子进程响应，`list_dir` 与 `read_file` 进行同步文件 I/O；运行于 UI 命令通道时会影响交互。
2. **P0 — 重复统计**：原 `subtree_rss_kb` 对每个树节点遍历整张进程表，近似 O(N²)，在总树与每宫格上重复计算。
3. **P0 — Grid 定位洪泛**：布局变更频繁发出 `grid_position` 和缩放请求，老请求可能积压，正确方案是每格单飞与最新坐标覆盖。
4. **P1 — Grid 进程/首屏**：每格独立子进程保护崩溃隔离，但冷启动高 CPU/内存；已有 300ms 错峰、内存预算，需进一步评测进程生命周期和延迟创建。
5. **P1 — Rust LSP**：每次 LSP 请求启动 bwrap/rust-analyzer 并重建文件快照；需要经批准的每工作区会话复用、版本同步、超时回收和安全验证。
6. **P1 — Monaco**：文件切换会 dispose/recreate editor，TS/JS worker eager sync；保留 view state 的同时复用 Editor、增量同步活动模型。
7. **P1 — 文件树**：大目录读取、递归展开/刷新和大文件一次读取，导致 I/O、DOM 压力；需有界并发、虚拟树和安全预览。
8. **P1 — Agent**：Coding Agent 全局锁覆盖长时间 chat；Provider 全局锁只允许一项非流式请求。应拆分冲突域、保留写入串行和取消/审批，安全实现有界并发、首 Token 流式返回。
9. **P2 — 资源治理**：`hibernate` 当前等于 `suspend`；后台 WebView 未必释放内存；真正销毁可能丢失用户表单与滚动状态，应设计可恢复快照及保护名单后提供明确的休眠态。
10. **P2 — 性能验收**：已有 resource benchmark 和构建预算，但缺端到端延迟/帧性能 CI 与真实安装客户端 5/30 分钟证据。

## 分期验收（完成条件）
- P0：原生耗时命令不占 UI 线程；网格多次 resize 只保留最新坐标；单次进程表扫描 O(N)，且结果与基线语义一致；全量 Rust/JS 检查及真实 Grid GUI 回归。
- P1：Grid 2/4/6/9 格启动与恢复准确；LSP 在同授权工作区复用沙箱、无跨项目泄露；Monaco 切换不重新创建普通 editor；文件树 1 万文件无卡顿；Agent 并发、取消、审批/备份测试通过。
- P2：压力触发资源释放，无编辑表单数据丢失；有 5/30 分钟真实 RSS/PSS、CPU、帧间隔与打包安装验收；CI 有明确阈值并拒绝回归。

## 实施过程及状态
- 初步代码变更（待 CI）：Tauri 耗时命令 `spawn_blocking`；`resource_stats` 子树聚合 O(N)；`read_file` 8 MiB 保护；Grid geometry latest-only 单飞队列。
- 其余功能不能因为提交了规划文档就标记完成。风险与待验证清单详见 TODO。
- 本轮仅有 GitHub 远端代码及 Actions 能力；已连接 Ubuntu Desktop Commander 设备处于离线状态，无法执行安装版 GUI / 真实性能实测。

## 必要的复核指令
```bash
npm ci
npm run check
npm run build
npm run check:coding-workbench
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo check --manifest-path src-tauri/Cargo.toml
bash scripts/pre-merge.sh
git diff --check
# 真实桌面
npm run verify:client
node scripts/benchmark-resource-scenarios.mjs --pid PID --minutes 5 --out /tmp/browseros-idle5.json
node scripts/benchmark-resource-scenarios.mjs --pid PID --minutes 30 --out /tmp/browseros-idle30.json
```

## 回滚
以分项提交为回滚粒度；如 CI 失败先修复而不合并至受保护 master；原生定位/生命周期变更必须在 GUI 实测后才能宣称完结。
