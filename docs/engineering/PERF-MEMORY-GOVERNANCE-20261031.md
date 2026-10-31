# BrowserOS 内存治理专项（2026-10-31）

- 上游：`perf/p0-p2-20261031@96ccd638`
- 独立修复分支：`fix/perf-memory-governance-20261031`
- 目标：在内存紧张时安全回收可恢复资源，不丢 AI 对话草稿，不让进程等待锁住桌面；不变更默认关闭的自动休眠。
- 本专项仅提供已提交的代码及待验收用例；**不是**已通过完整 Rust 构建/安装版 GUI 的证明。

## 当前修复

| 区域 | 原风险 | 修改与边界 |
| --- | --- | --- |
| Tab 休眠 | 被动页面有 iframe、canvas、富文本控件等复杂状态；snapshot 缺字段仍可能通过 | 增加复杂页面保护；`protected` 必须明确为 false、坐标须有效，解析失败拒绝休眠 |
| Tab 恢复竞态 | 休眠快照等待最多 2 秒，期间用户可能重新激活、导航或关闭休眠选项 | 对启用状态、激活状态、原 URL、休眠标记与持续闲置时间进行前后复核 |
| 压力回收节流 | 一次扫描可能同时销毁全部空闲页签、Grid，造成回收/重建风暴 | 按最久闲置顺序处理；每轮最多 Tab 1/2/4 个、Grid 1/1/2 个（正常/警告/临界）；逐个 Tab 回收前重新检查当前内存压力 |
| Grid AI 页数据 | 先前隐藏 Grid 即可被销毁，URL+rect 无法恢复未发送提示词、流式回复和 SPA 状态 | **拒绝自动销毁外部 URL 的 Grid**，并由子 WebView 只读 `HibernationProbe` 实时确认 URL、空 DOM、无交互元素；仅真正空白的 `about:blank` 才按限额休眠。缓存的 `last_url` 不可作为独立安全凭据；有损页面回收要先建设明确授权/快照恢复协议 |
| Grid 回收竞态 | 选取候选到关闭进程之间页面可能被重新激活 | 收集前后核对隐藏时间/焦点/空白状态，取消误回收；若 CloseTab 已执行且状态变化则触发恢复重放；旧进程退出前禁止同 index 复用 |
| Grid 导航快照 | 初始 about:blank 已通过链接/脚本转向外部站点，但原生 `last_url` 仍可能停留在初始目标 | 记录 Grid 的 `tab-navigated` 真正 URL，并通过子进程 `eval_result` 的专用 `HibernationProbe` 回读实时 DOM，失败拒绝回收 |
| Grid Memory Guard | 状态栏按 700 MiB 预留，创建按 1,800 MiB 预留；已有进程也重复计费 | 两者统一按 1,800 MiB 保护浏览器与系统，仅新创建的 Grid 消耗 450 MiB/格预算；不能读取可用内存时拒绝新增进程 |
| 进程回收 | `Child::wait()` 无界等待可能挂住 Grid 管理/关机 | SIGKILL 后最多轮询 750ms，超时交给后台 `waitpid` 清理，避免堵住 Grid 锁 |
| LSP Cache | 过期驱逐、容量置换、lsp_close 在全局 Mutex 内触发 sandbox 进程销毁或等待锁 | 先摘除缓存引用，再在**全局锁外**关闭 LSP 进程与清理私有 scratch；同样使用有界退出等待 |

## 需要验证的行为

1. **零数据丢失**：待发送 ChatGPT/Kimi/DeepSeek 文字、流式回答、富文本、上传控件、登录表单、嵌入式应用均不能被自动回收。某些编辑状态是应用内部状态，单纯 DOM 检查不能证明所有第三方页面可恢复，所以 Grid 外部 URL 暂不自动休眠。
2. **回收一致性**：Tab snapshot 期间重新激活/导航/关闭开关不应销毁；Grid 在恢复与休眠交叉时仍有正确 WebView/进程归属。
3. **正常压力**：达到闲置阈值时每次只处理有界资源；压力解除后不继续使用先前的临界阈值。
4. **内存预算**：四格均存活时，即使 MemAvailable 低也可继续复用四格；新增格不足则拒绝或降级，UI 展示与实际创建使用同一计算口径。
5. **LSP 隔离**：关闭一个工作区的 LSP 不阻塞其他工作区缓存，正在执行中的 JSON-RPC 在当前会话完成后安全退出；清理临时源码副本。
6. **真机**：比较 `master`、`perf` 和本分支相同负载的 5/30 分钟 CPU、PSS、RSS、OOM、Grid 恢复次数、tab hibernation 次数及 UI 输入/导航 p95/p99。测量脚本不能代替实际 Ubuntu 图形界面。

## 验收命令（未声明已经通过）

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml adaptive_hibernation_tests
cargo test --manifest-path src-tauri/Cargo.toml grid_memory_budget_tests
cargo test --manifest-path src-tauri/Cargo.toml adaptive_grid_hibernation_tests
cargo test --manifest-path src-tauri/Cargo.toml grid_process_termination_tests
node --check src-tauri/injected/grid-hibernation-probe.js
node tests/check-grid-hibernation-probe.mjs
cargo test --manifest-path src-tauri/Cargo.toml capabilities::agent::coding_lsp
npm run check
npm run check:coding-workbench
```

## 尚未实现

- 任意第三方 Grid/AI 站点的完整会话快照与原子恢复，不允许用 URL 重载假冒恢复。
- 基于 cgroup/PSI 的跨环境实时压力监控，尚需实际机器数据设计阈值。
- 安装版真实 PSS/p95、OOM 回归与 5/30 分钟对照实验，当前 `GUI_PENDING`。
- Monaco TS/JS 全量 Worker 同步修复由独立 PR #44 处理；本 PR 不重复修改该文件。

结论：本分支目标是先让内存治理「不误删、不会无限等待、报告口径一致」，随后再基于真实 PSS 选择可以安全增加的回收范围。
