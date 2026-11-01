# BrowserOS P0–P2 性能 TODO / 实施记录

**基线**：`master@81a9feca`；**日期**：2026-10-31；**目标分支**：`perf/p0-p2-20261031`。

状态定义：`[x]` = 代码已提交 **且** 必要验收通过；`[ ]` = 未完成或未验证。分支有代码不等于已通过测试。

| 编号 | 优先级 | 工作项 | 本轮实现/证据 | 状态 |
| --- | --- | --- | --- | --- |
| PERF-01 | P0 | Tauri Grid create/close/open/position 非阻塞入口 | `bridge.rs` 已迁移耗时命令到 `async + spawn_blocking`，内部 blocking 入口供原生测试复用；前置契约检查已通过，Rust check/test 与真实 GUI 未验收 | [ ] |
| PERF-02 | P0 | 资源监控线性进程树聚合与线程迁移 | O(N) `process_tree_rss_kb`、资源采样后台 worker、750ms PID 感知缓存及进程树测试已提交；准确性/失效边界待 Rust 测试与真机比对 | [ ] |
| PERF-03 | P0 | Grid 定位 latest-only 合并/单飞/限频 | `useGridHost.ts` latest-only 单飞合并已提交；跨会话 resize/zoom、错误重试与 p95 IPC 实测待验收 | [ ] |
| PERF-04 | P1 | Grid 渐进启动与复用/生命周期 | 增量 resize、健康 child 复用、URL 不变不导航、休眠恢复失败回滚、关闭 Grid WebView 和独立进程组回收已提交；真机生命周期/子进程清理待验收 | [ ] |
| PERF-05 | P1 | LSP 持久安全 Session | 已实现按工作区隔离的 bwrap LSP Session 复用（上限 2、空闲 TTL 300s、后台回收、didChange、失败驱逐）；安全隔离/诊断一致性/恢复与 Rust 测试待验收 | [ ] |
| PERF-06 | P1 | Monaco editor 实例复用/增量 worker | 普通 Monaco Editor 已跨文件 setModel 复用；TS/JS worker 仍 eagerModelSync，需实现增量模型同步并完成 UI 回归 | [ ] |
| PERF-07 | P1 | 文件树大目录虚拟化/分批/刷新 | `list_dir`/`read_file` 已移 worker，读取上限 8MiB；Agent tree_page 的 snapshot/offset 和前端「加载更多」已实现，1万文件虚拟滚动与并发刷新/GUI 验收尚缺 | [ ] |
| PERF-08 | P1 | Agent 按 session 锁/Provider 有界并发与流式 token | Agent 只读 tree/tree_page/read/search/sessions_list 已脱离长锁；AI Provider 改为每 provider 有界并发(2)、SSE 流式与取消，且增加取消后清理进程单测；按 session 写冲突域、审批/取消端到端仍未验收 | [ ] |
| PERF-09 | P2 | 自适应内存压力休眠 | 默认关闭；已增加内存压力 10m/2m/30s 阈值、保护媒体/焦点/未保存表单与文件选择、WebView/隐藏 Grid 进程释放及 URL+滚动位置恢复；未验收数据安全、加载和关闭竞态 | [ ] |
| PERF-10 | P2 | 基线/性能 CI/安装版 5、30 分钟实测 | 已新增 scheduled/手动 installed-client 持续采样 workflow、5/30 分钟脚本、采样精度和硬件 provenance；只有自测证据，未拿到安装版真实前后对比指标 | [ ] |

## 本轮变更与验证流水
- `97c714b2` — 后台化耗时 Tauri 命令，进程树 O(N) 汇总，大文件防护及进程树单测。
- `5448cbc2` — Grid 矩形 IPC 最新状态合并（仍需验证重启与定位时序）。
- `4e833a90` — 普通 Monaco 编辑器在文件切换时 setModel 复用。
- `4c79c782` — 750ms PID 感知的 ResourceStats 缓存。
- `78a02655` — Grid 按目标数量增量调整，保留存活 child。
- `bc078af1` — 暴露 Grid child 最近 URL 快照。
- `b08058ab` — Grid resize 时 URL 未变化则不重复导航。
- `675a0784` — Agent 只读工作区操作绕开全局写会话锁。
- 源码获取：GitHub connector，可读取与提交。
- Ubuntu 桌面：**离线**；GUI 验证：`GUI_PENDING`。
- CI 检查（`6849612c`，北京时间 2026-10-31 11:03 所见）：Engineering Governance、UI Safety、Hot-Plug Acceptance = PASS；Supply Chain Assurance 同一 SHA 同时存在 PASS 和 FAIL 运行，FAIL 是两条 **真实 RustSec 漏洞**；Full Validation 一轮 FAIL、一轮尚运行，详见下文。不得将此前成功运行当最新整套门禁已全绿。
- 实测改善数据：**无**。不得声称性能提升 x% 或 P0–P2 全部完成。

## 2026-10-31 最新真实 GitHub CI 核验（不等于安装版验收）

- 分支：`perf/p0-p2-20261031`；PR：[#43](https://github.com/code5m/browser-os/pull/43)，保持 Draft、未合并 master。核查 head：`6849612c96fdb4462444cf98907ba78d3f1f4ba8`。
- PASS：Engineering Governance [run 41162367498](https://github.com/code5m/browser-os/actions/runs/41162367498)；UI Safety [run 41162367479](https://github.com/code5m/browser-os/actions/runs/41162367479)；Hot-Plug [run 41162367484](https://github.com/code5m/browser-os/actions/runs/41162367484)。前置检查已实测 Native command registry 178/178、AppState 27/27、Rust/TS 交叉层契约、LSP/Agent 工作台配置与负向 fixture = PASS（但**不代表** Rust 编译/测试/打包通过）。
- FAIL：Full Validation [run 41162363966](https://github.com/code5m/browser-os/actions/runs/41162363966)，失败发生在 **Early frozen frontend size gate**。基线 `logs/m0-build-metrics/build-metrics-052b18a.json` 为 795,517 bytes，本次 `dist` 为 12,976,614 bytes、169 文件，增长 **1531.22%**；最大 chunk `ts.worker-Cjxrm33T.js` 6,989,587 bytes。门禁 30% 警告 / 45% 阻断。此为实际打包体积检测结果，需按相同构建口径核对 master 与历史基线，**禁止改大或绕开现有阈值来制造 PASS**。另一个 [run 41162367529](https://github.com/code5m/browser-os/actions/runs/41162367529) 核查时仍进行中。
- FAIL：Supply Chain [run 41162367493](https://github.com/code5m/browser-os/actions/runs/41162367493)，`cargo audit` 检出漏洞 **RUSTSEC-2026-0385**（`ctutils 0.4.2`，推荐 `>=0.4.4`）和 **RUSTSEC-2026-0418**（`libz-sys 1.1.29`，推荐 `>=1.1.31`）；同 head 较早 [run 41162363948](https://github.com/code5m/browser-os/actions/runs/41162363948) PASS，但不能抵消更新的 RustSec 数据库 FAIL。应先升级兼容依赖并重新审计、构建。
- 关键改动：`docs/engineering/governance.json` 已登记 `browseros-sustained-performance.yml`；LSP Session 后台回收和错误驱逐、Grid 关闭 WebView 后回收进程组、资源采样时间校准、休眠重激活检查、滚动位置恢复等已提交。**这都是 CODE_CHANGE，尚无完整 Rust/GUI/PSS 真实性能验收**。
- 避免连续短间隔提交使 Full Validation 自动取消。接下来采用小批次稳定提交 → 等待该 SHA 的完整检查 → 失败后定点修复。

### 下一步阻塞顺序（均未完成）
1. 核对并修复真实前端体积预算；提供相同构建方式的 master 对比和分 chunk 增长分析，严禁无审批改阈值。
2. 在 `Cargo.lock` 安全更新 `ctutils` 与 `libz-sys` 及相关上游依赖，确认 `cargo audit` 无漏洞；不要忽略 advisory。
3. 通过最新 SHA 的 Full Validation 全套（包括 `cargo fmt`、`cargo check`、`cargo test`、真实打包和 GUI 自动回归），处理新增测试与流式响应的潜在编译错误。
4. 补齐文件树大目录虚拟化、Monaco worker 增量同步、Agent session 写锁冲突域与完整基准。
5. Ubuntu 安装版独立实测前后 5/30min CPU、RSS/PSS、首屏及编辑/导航 p50/p95/p99；尚无数据不得记录提速百分比。

## 前端体积基线核对与 CI 修复（2026-10-31）

- 已核对 master@81a9feca 与 perf@8090f000 同口径构建：前者 12,975,246 B，后者 12,976,614 B（+1,368 B，+0.01054%）。
- 历史 M0 795,517 B / 45% 门禁未删除或改阈值，master 已不满足历史限额；属于独立历史治理问题，不能误归因本次 perf 优化。
- CI 增加已有 Monaco 有限预算审计及同 runner 构建 master 的 1% 性能增量审查线；代码已提交不等于 CI PASS，请以最新工作流运行结果为准。
- 详细策略：docs/engineering/FRONTEND-SIZE-DUAL-BASELINE-20261031.md。
- Rust 编译、真机 GUI、P0–P2 真实性能验收仍独立阻塞，严禁因体积门禁修复就将全部 TODO 标记完成。

## Rust 编译阻塞修复（2026-10-31）

- 已提交修复代码：Coding 缺失 HashMap 导入；LSP 未定义 URI、可变锁及遗漏的测试 URI；AI ProviderSemaphore 借用守卫；Tauri Channel IPC 必选命令参数。
- 保留 `coding_service` 命令名、来源权限、流式 delta/complete/error；调用方未提供流式回调时由 TypeScript bridge 显式传递丢弃事件的 Channel。
- 独立 Rust CI 检查前先真实执行 `npm ci && npm run build` 生成 `dist`，不是创建占位文件；不改变原有前端体积门禁。
- **这仅表示代码修改已提交；Rust fmt/check/test、Full Validation 和真机 GUI 仍须以最新 CI 实际结果验收，不能提前标记 PASS。**

## 未完成工作与复核重点
1. 立即确认各 GitHub Actions job 结果；有失败则定位原因，修复后重新运行。
2. 检查 Tauri Command 包装与 `main.rs` 内部调用签名，保证异步调用契约、权限不变；检查 Grid 增量 resize 不误导航且失败回滚正确。
3. 针对 Grid 关闭重开、不同 session/布局、快速拖动和缩放验证最新状态与错误重试。
4. 完成 PERF-05 LSP 持久安全 Session、PERF-06 TS/JS worker 增量同步、PERF-08 Agent 按 session 写锁/Provider 有界并发及 SSE 流式、PERF-09 自适应资源休眠、PERF-10 交互基准/安装版实测；不得将规划当完成。
5. Ubuntu 设备在线后跑发布版、真机 GUI、性能基线及对比；记录硬件环境、PSS/RSS、p95、原始日志与截图。
6. 全部门禁通过后再合并 `master`；受保护分支不执行强推。

## 结果准则
- `CODE_PASS`：构建与自动门禁实际通过。
- `PACKAGE_PASS`：实际打包成功并验证。
- `GUI_PENDING`：真实桌面尚未验收（当前）。
- `GUI_PASS`：用户确认桌面实测。
- `BLOCKED`：CI 失败、安全不变量破坏或必要验收条件不满足。

**结论（截至此记录）：P0–P2 未完成；已有部分 P0/P1 代码提交，但完整功能与真机验收仍未完成。**
