# M0 基线采样契约

> 文档角色：M0-0.a 的冻结契约，是 M0-1 自动化脚本、M0-0.b/c 基线采集和 M0-5 资源验收的共同输入。
> 契约版本：V1.1。
> 文档修订：V1.2-status，2026-08-30 14:27 CST；仅同步执行状态，未修改 V1.1 指标语义。
> 冻结时间：V1.0 于 2026-08-29 07:50 CST；V1.1 于 2026-08-30 完成可执行性修订。
> 编写/裁决模型：Codex 主任务；独立契约审阅模型：`gpt-5.6-terra / medium`；状态审计模型：`gpt-5.6-luna / low`。
> 状态：FROZEN。旧 V1.0 结果保持原版本，不得静默套用 V1.1 口径。

---

## 1. 决策与边界

本检查点只冻结“测什么、在哪测、测几次、怎么算、证据放哪”，不采集最终性能数值，也不实现自动化脚本。

- `logs/baseline-2026-08-27.md` 是 `dev profile` 下的历史观察摘要，状态固定为 `EXPLORATORY`，不能作为后续性能升降的比较基线。
- 正式候选基线必须来自干净提交、release 构建和可追溯原始证据；工作树非干净时只能归档为 `EXPLORATORY`。
- M0-1 必须先把本契约编码为脚本，M0-0.b/c 再执行采集。关键路径固定为：
  `M0-0.a -> M0-1.a -> M0-1.b -> M0-1.c -> M0-0.b -> M0-0.c`。
- M0-0 的 PASS 只证明基线完整、可重复，不等于当前性能或资源行为已经达标；资源泄漏和性能回退由 M0-5/M0 总门禁裁决。
- 圈复杂度抽样从 M0 基线门禁移除：当前没有冻结的解析器和稳定算法。以后若恢复，必须新增独立指标 ID，不能混入既有结果。

## 2. 状态词

| 状态 | 含义 |
|------|------|
| `REQUIRED_NOW` | M0-0.b/c 必须采集，缺失即 FAIL |
| `DEFERRED(<WBS>)` | 能力尚不存在，由指定 WBS 首次实现时建立基线，不计入当前 PASS 分母 |
| `BLOCKED` | 指标应采但缺少前置能力；必须写明负责人和解除条件 |
| `PASS` | 证据、口径和样本完整；不表示数值一定优秀 |
| `FAIL` | 缺字段、缺原始证据、命令失败或样本不足 |
| `UNSTABLE` | 同批样本波动率超过 10%，必须归因后才能进入比较基线 |
| `INCOMPARABLE` | 契约、工作负载、profile 或环境指纹不一致，禁止输出性能升降结论 |
| `EXPLORATORY` | 调研数据，仅供定位，不能充当门禁基线 |

## 3. 固定运行剖面

正式采集必须同时满足：

1. 从仓库根目录运行；根目录由 `git rev-parse --show-toplevel` 解析，禁止使用脚本内硬编码绝对路径。
2. `git status --porcelain=v1` 为空；记录完整 SHA、短 SHA、分支或 detached 状态。
3. 前端先执行 `npm run build`，桌面端执行 `cargo build --manifest-path src-tauri/Cargo.toml --release --locked`；依赖安装时间不计入构建指标。
4. GUI 运行统一设置 `WEBKIT_DISABLE_DMABUF_RENDERER=1`、`GDK_BACKEND=x11`；采集报告同时记录宿主会话原值。
5. 使用隔离目录 `XDG_DATA_HOME/XDG_CACHE_HOME/XDG_CONFIG_HOME=/tmp/mvp-browser-os-m0/<run_id>/xdg-*`，不删除或污染用户真实数据。
6. 冷启动每次使用新的隔离目录；预热启动与正式样本不得复用同一进程。若另测暖启动，使用新指标 ID，不得覆盖冷启动结果。
7. 使用 release 二进制 `src-tauri/target/release/mvp-browser-os`，并记录其 SHA-256；debug、Vite ready 或仅 Rust 进程启动都不算 GUI 冷启动。
8. 采样期间接交流电，关闭系统更新和高负载任务；开始前记录 1/5/15 分钟 load average、可用内存和电源模式。无法读取的字段写 `UNAVAILABLE:<原因>`，不得留空。
9. 浏览器和宫格场景只使用 `about:blank` 或仓库内固定夹具，不依赖公网响应速度。
10. 同一台机器一次只运行一个被测应用实例；记录根 PID，并用 PID + `/proc/<pid>/stat` starttime 标识进程，避免 PID 复用误判。上一轮退出后确认已记录进程全部消失再开始下一轮。
11. `M0_RUN_MODE=formal` 必须使用本节冻结的完整样本数；任何缩短样本必须显式使用 `M0_RUN_MODE=smoke`，结果只能是 `EXPLORATORY`，不得为 `PASS`。
12. 质量门禁构建 release 后，把二进制 SHA-256 作为 `M0_EXPECTED_BINARY_SHA256` 交给资源门禁；资源门禁只在实际哈希完全一致时允许正式测量。

## 4. 环境与来源指纹

每个 run 必须生成 `environment.json`。字段缺失时，该 run 不能成为正式基线。

| 类别 | 必填字段 |
|------|----------|
| 运行身份 | `run_id`、时间戳、契约版本、采集脚本版本 |
| Git | `commit_sha`、`short_sha`、`branch`、`git_status_porcelain`、仓库根目录 |
| 构建物 | `profile=release`、二进制路径、二进制 SHA-256、完整构建/启动命令 |
| 锁文件 | `src-tauri/Cargo.lock`、`package-lock.json` 的 SHA-256 |
| 系统 | OS、内核、架构、CPU 型号、逻辑核数、物理内存字节数 |
| 图形 | `XDG_SESSION_TYPE`、DISPLAY/Wayland 是否存在、分辨率、缩放、实际 `GDK_BACKEND` |
| WebKit | `WEBKIT_DISABLE_DMABUF_RENDERER`、WebKitGTK 与 GTK 版本 |
| 工具链 | `rustc`、`cargo`、`node`、`npm` 版本 |
| 运行条件 | locale、时区、load average、可用内存、电源模式、隔离 XDG 目录标识 |

指纹分成两类：

- `source_fingerprint`：commit、锁文件和二进制哈希，用于证明测的是哪份代码。
- `environment_fingerprint`：规范化后的系统、硬件、图形、工具链和运行条件，不包含时间戳、run_id、源码路径、commit 与二进制哈希。

比较两个提交时，契约版本、场景版本和 `environment_fingerprint` 必须一致；否则结果为 `INCOMPARABLE`。环境文件不得记录 token、Cookie、密钥、完整进程环境或 `XDG_RUNTIME_DIR` 等敏感值。

## 5. 指标目录

| 指标 ID | 状态 | 定义与单位 | 样本/统计 | 判定 |
|---------|------|------------|-----------|------|
| `rust_fmt_main_exit` | `REQUIRED_NOW` | 主工程 `cargo fmt --check` 退出码 | 每提交 1 次 | 必须为 0 |
| `rust_fmt_plugin_exit` | `REQUIRED_NOW` | browser-tabs workspace `cargo fmt --check` 退出码 | 每提交 1 次 | 必须为 0 |
| `clippy_main_unique_warnings` | `REQUIRED_NOW` | clippy JSON 中按 package、文件、行列、lint 去重后的主工程 warning 数 | 每提交 1 次 | 相对已接受基线只减不增 |
| `clippy_plugin_unique_warnings` | `REQUIRED_NOW` | 同一 clippy JSON 中 browser-tabs 插件 warning 数 | 每提交 1 次 | 相对已接受基线只减不增 |
| `frontend_build_ms` | `REQUIRED_NOW` | 清空 `dist/` 后 `npm run build` 的墙钟毫秒，不含安装依赖 | 1 次预热 + 3 次正式；median/min/max | 波动率大于 10% 为 `UNSTABLE` |
| `dist_total_bytes` | `REQUIRED_NOW` | `dist/` 全部普通文件字节数之和 | 每次正式构建记录 | 后续增长超过 15% 阻断或书面豁免 |
| `largest_js_bytes` | `REQUIRED_NOW` | `dist/assets/` 中最大 `.js` 文件原始字节数 | 每次正式构建记录路径与值 | 不引用带 hash 的固定旧文件名 |
| `largest_js_gzip_bytes` | `REQUIRED_NOW` | 上述最大 JS 经 `gzip -9 -c` 的字节数 | 每次正式构建记录 | 与原始大小成对保存 |
| `release_binary_bytes` | `REQUIRED_NOW` | release 可执行文件字节数 | 每提交 1 次 | 后续增长超过 15% 需说明 |
| `startup_ready_ms` | `REQUIRED_NOW` | 从 release 进程成功 spawn 到主前端完成 mount、两次 animation frame 且一次 IPC 往返成功 | 1 次预热 + 3 个独立进程；median/min/max | 禁止用 Vite ready 或仅 `.show()` 代替 |
| `idle_process_tree_rss_kib` | `REQUIRED_NOW` | ready 后空闲 30 秒，主进程及全部后代 `VmRSS` 之和 | 每 5 秒采 1 次、持续 60 秒；取 median/min/max | 必须记录进程树成员 |
| `idle_process_tree_fd_count` | `REQUIRED_NOW` | 与 RSS 同一进程树可访问 FD 数之和 | 与 RSS 同时采样 | 读取失败的 PID 单列，不得默认为 0 |
| `tab_cycle_rss_slope_kib` | `REQUIRED_NOW` | 单页签固定场景关闭后的进程树 RSS 对循环序号的 OLS 斜率 | 5 次预热 + 20 次正式 | M0-5 裁决持续增长 |
| `grid_cycle_rss_slope_kib` | `REQUIRED_NOW` | 4 宫格固定场景关闭后的同口径斜率 | 5 次预热 + 20 次正式 | M0-5 裁决持续增长 |
| `terminal_cycle_rss_slope_kib` | `REQUIRED_NOW` | 单 PTY 创建/关闭后的同口径斜率 | 5 次预热 + 20 次正式 | M0-5 裁决持续增长 |
| `resource_cycle_fd_delta` | `REQUIRED_NOW` | 每类循环正式样本末值减初值 | 每类场景分别记录 | 持续增长必须归因 |
| `orphan_process_count` | `REQUIRED_NOW` | `opened - baseline` 得到资源打开后新增的后代 PID + starttime；关闭并等待 2 秒后直接复核 `/proc/<pid>/stat`，即使 PPID 已变化也计入 | 每个正式循环记录，应用根进程永不计入 | M0-5 必须为 0 |
| `terminal_10mib_elapsed_ms` | `REQUIRED_NOW` | 提交固定 10 MiB 输出命令到前端消费并绘制结束标记的耗时 | 1 次预热 + 3 次正式 | 缺结束标记为 FAIL |
| `terminal_frame_gap_p95_ms` | `REQUIRED_NOW` | 终端吞吐期间主前端 `requestAnimationFrame` 间隔 p95 | 与吞吐样本同步 | 用于后续回退比较 |
| `terminal_frame_gap_max_ms` | `REQUIRED_NOW` | 同场景最大帧间隔 | 与吞吐样本同步 | 大于 100ms 标记风险并归因 |
| `script_first_response_ms` | `DEFERRED(M2-4)` | 安全脚本执行通道首条输出延迟 | M2-4 首次实现时冻结负载 | 不计入当前 PASS |
| `database_first_row_ms` | `DEFERRED(M4-3)` | 固定本地查询首行延迟 | M4-3 首次实现时冻结数据集 | 不计入当前 PASS |

Clippy 使用一次结构化命令覆盖主工程及其 path dependency，按 `package_id` 分组；不得通过重复运行后数人类可读的 `warning:` 行。历史摘要中的“主工程 13、插件 1”只作为核对提示，正式数值由 M0-0.b 重新生成。

## 6. 固定场景

### 6.1 启动与空闲

1. 删除本 run 的旧 ready 文件并创建新的隔离 XDG 目录。
2. 记录单调时钟起点后 spawn release 二进制。
3. 主前端完成 mount、两次 `requestAnimationFrame`，再完成一次轻量 IPC 往返，写出带 `run_id` 的 ready 信号。
4. 收到匹配 run_id 的信号时记录 `startup_ready_ms`。现有 `/tmp/mvp-life.log` 的 `main-window-created-and-shown` 只能作为后端 show 辅助证据，不能替代可交互 ready。
5. ready 后不操作 30 秒，再按 5 秒间隔采集 60 秒 RSS/FD/进程树。

### 6.2 资源循环

每类场景独立启动应用、独立统计，顺序固定为 tab、grid、terminal：

- tab：创建 1 个 `about:blank` 页签，等待导航完成和 2 秒稳定期，关闭页签，再等待 2 秒采样。
- grid：创建 4 个 `about:blank` 宫格，等待全部 ready 和 2 秒稳定期，关闭全部宫格，再等待 2 秒采样。
- terminal：创建 1 个 PTY，等待提示符 ready，立即关闭，再等待 2 秒采样。

每类先做 5 次预热且不计入结果，再做 20 次正式循环。每次保存循环号、进程树、RSS、FD、残留 PID 和操作结果；某次操作失败时该批为 FAIL，禁止丢掉失败样本后继续算“最好结果”。

### 6.3 终端吞吐

固定负载为 10 MiB ASCII 输出，命令发送前启动前端帧间隔采样；负载必须包含唯一 begin/end 标记。计时终点是前端收到 end 标记并完成下一次 animation frame，不是后端读完 PTY。

```bash
printf '__M0_TERM_BEGIN__\n'; head -c 10485760 /dev/zero | tr '\0' 'x'; printf '\n__M0_TERM_END__\n'
```

原始证据必须包含实际消费字节数、begin/end 是否各出现一次、elapsed、p95/max frame gap。超时、标记缺失或应用退出均为 FAIL。

## 7. 统计与比较公式

- 样本基线值：`median(values)`；同时保存 `min`、`max` 和全部原始值。
- 波动率：`(max - min) / median * 100%`。median 为 0 时不计算百分比，改报绝对差。
- p95：仅对至少 20 个样本使用 nearest-rank；少于 20 个样本只报告全部值和 max，不伪报 p95。
- 资源斜率：对正式 20 次样本做普通最小二乘，单位为 KiB/cycle；同时报告 `last - first` 和末值相对初值百分比。
- 越小越好指标的回退率：`(candidate - baseline) / baseline * 100%`；越大越好指标必须另行声明公式。
- 性能回退超过 10%、前端或 release 二进制体积增长超过 15%、clippy 新增 warning 时阻断，除非存在负责人、理由、到期时间完整的书面豁免。
- 任一输入不满足可比较条件时输出 `INCOMPARABLE`，不能换机器、换 profile、换场景后仍声称“提升”或“退化”。

## 8. 证据目录与文件名

每个正式 run 使用以下结构：

```text
logs/m0-baseline/<run_id>/
  environment.json
  scenario.json
  summary.json
  summary.md
  commands/
  raw/
  measurements/
  screenshots/
  SHA256SUMS
```

- `run_id`：`YYYYMMDDTHHMMSS+0800_<short-sha>_release_<backend>`，例如 `20260829T075000+0800_c16ed27_release_x11`。
- 原始命令输出：`raw/<metric-id>_r<01..NN>.stdout.log` 与对应 `.stderr.log`。
- 结构化测量：`measurements/<metric-id>_r<01..NN>.json`。
- `scenario.json` 保存契约版本、场景版本、预热数、正式样本数、超时和稳定等待时间。
- `summary.json` 是机器判定源；`summary.md` 只做人类阅读，两者不一致时整批 FAIL。
- `SHA256SUMS` 覆盖本 run 内除自身外的全部证据文件；截图必须标明对应场景和样本号。
- 原始输出可以清理 ANSI 控制字符，但必须同时保留未修改原件；任何脱敏都记录规则。
- 多批次采集必须先使用 `M0_EVIDENCE_ROOT` 写入仓库外暂存区，并通过 schema、SHA256SUMS、commit、环境指纹和二进制哈希校验；全部批次完成后才一次性复制到本目录。禁止第一批证据先写入仓库导致后续批次工作树变脏。
- 正式入口固定为 `scripts/collect-m0-baseline.sh`：默认 1 批对应 M0-0.b，`--batches 3` 对应 M0-0.c；`--smoke` 只保留仓库外探索证据。

## 9. 检查点验收

### M0-0.a

- [x] 指标 ID、单位、状态和责任 WBS 已冻结。
- [x] 运行剖面、环境字段和两类指纹已冻结。
- [x] 预热、样本数、统计和可比较条件已冻结。
- [x] 启动、空闲、资源循环和终端吞吐场景已去除“N 次”“主窗可见”“高吞吐”等歧义词。
- [x] 证据目录、命名和完整性校验已冻结。
- [x] M0-0 与 M0-1 的循环依赖已通过检查点关键路径解除。

冻结时结论（历史）：`M0-0.a = PASS`；V1.0 冻结后的下一检查点当时是 `M0-1.a`。

当前执行状态（不改变契约）：`M0-1.a/b/c = PASS`。`ac0ecac` 上生成的三批正式候选因 `about:blank` 被错误改写为百度 URL 且 aggregate 不稳定而 `REJECTED`；`e8975d6` 已修复 URL 与页签生命周期问题。当前必须在包含修复的干净提交上重新执行 `M0-0.b`，通过后再重新执行 `M0-0.c`；重跑通过前不得宣称 M0 完成。

### M0-0.b

所有 `REQUIRED_NOW` 指标必须有 release 数据、原始输出、环境/来源指纹和足量样本。任一必需指标为 `BLOCKED`、缺原始日志或工作树非干净，结论均为 FAIL；不得复用 `ac0ecac` 的被拒候选证据。

执行与验收：

```bash
scripts/collect-m0-baseline.sh
```

总控必须确认质量与资源 summary 均为 `PASS`、二进制哈希一致、每个 run 的 `SHA256SUMS` 有效，才可归档并判定 M0-0.b。

### M0-0.c

在同一 commit、同一契约和同一环境完成共 3 批完整采集。三批 median 的波动率超过 10% 时标记 `UNSTABLE` 并裁决；没有裁决不得建立正式比较基线。

```bash
scripts/collect-m0-baseline.sh --batches 3
```

## 10. V1.0 冻结时参考环境（非正式基线）

以下信息只证明契约在当前桌面环境接受过可执行性审阅，不是 M0-0.b 数值证据：

| 字段 | 冻结时观察值 |
|------|--------------|
| Git | `c16ed27`，`feature-M0-baseline`，工作树当时干净 |
| OS / Kernel | Ubuntu 24.04.4 LTS / Linux 7.0.0-30-generic / x86_64 |
| CPU / RAM | AMD Ryzen 7 5700U，16 逻辑核 / 16059621376 bytes |
| 桌面会话 | Wayland 会话，DISPLAY `:0`；应用约定强制 `GDK_BACKEND=x11` |
| Rust | rustc/cargo 1.96.0 |
| Node | Node 26.7.0 / npm 11.19.0 |
| Native | WebKitGTK 2.52.3 / GTK 3.24.41 |
| Lock hashes | Cargo `52e6cc4f...fbed9971`；npm `024f51ed...2d67ee6` |
| Locale / timezone | `zh_CN.UTF-8` / CST |
| 未取得字段 | 分辨率与电源模式在当前沙箱不可读；正式桌面采集必须补值或写明 `UNAVAILABLE` 原因 |

V1.0 冻结时的已知缺口包括硬编码 debug 启动、缺少可交互 ready 信号和缺少原始证据目录；这些缺口已在 V1.1 的产品钩子、门禁和总控脚本中补齐，不再作为当前阻塞项。

## 11. V1.1 修订记录

- 正式与 smoke 模式分离，短样本不再可能输出 `PASS`。
- release 二进制哈希由质量门禁传给资源门禁并强制匹配。
- 资源循环增加 `prepare/opened/done/sampled` 四阶段握手，消除最后一轮退出竞态。
- 孤儿进程改为资源新增候选集并直接按 `/proc` starttime 复核，排除应用根进程且覆盖 PPID 变化。
- idle 在第 5 至 60 秒采满 12 点并输出聚合统计；启动耗时使用单调时钟。
- 新增仓库外暂存、单批/三批聚合和一次性归档，修复多批次工作树自污染。
