# B-M3.a · 终端输出管道与生命周期内核（落地卡）

- 上游展开：`logs/checkpoints/M3-20260905-2030.md`（冻结 F1~F13，含对 fileterm 草案的 5 处事实修正）
- 认领执行：**本 agent**，按用户 2026-09-05「继续」指令在 M2 收口（M2-6.e）后领取 NEXT 卡
- IMPLEMENTER_MODEL：本 agent（用户指令驱动，与裁定同模型，独立性弱于跨模型复核）

---

## 一、采纳的冻结决策（F1~F13，原样落地）

| 码 | 落地方式 |
|----|----------|
| F1 | 新增 `src-tauri/src/terminal.rs`：`start_pipeline(reader, out, id, stop, capacity, flush_interval_ms, lossy, on_channel_dead)`；worker 线程读 PTY → `sync_channel(TERM_QUEUE_CAPACITY=128)` → pump 线程 → sink。worker 侧只用 `try_send`（永不阻塞） |
| F2 | 队列满丢弃当前块并计入 `DropCounter`（worker），pump 侧 `notify_drops` 按「≥64 块或 ≥1 s」上报 `kind:"flow"`；**不向终端字节流插入任何伪内容**。`measure=true` 时 `lossy=false`（阻塞 `send`，不丢帧） |
| F3 | pump 用 `recv_timeout(flush_interval_ms.max(1))` 聚合，`should_flush(batch_len, elapsed_ms, interval)` 纯函数判定（≥64 KiB 或窗口到点）。测量模式窗口 = 0 |
| F4 | `trait TermOutput` + `EventSink(AppHandle)`（Event 广播，`term_spawn` 保留给 M0 内部驱动）+ `ChannelSink(Channel<Value>)`（`term_spawn_channel`，前端唯一入口）；两者共享 `spawn_terminal(out, measure)`。新命令已入 `default-commands.toml` 并在 `main.rs` 注册 |
| F5 | `TerminalSession` 持 `master: Box<dyn MasterPty + Send>`；`term_resize` 调 `terminal::resize` → `master.resize(PtySize{rows,cols,0,0})`，失败返回明确错误；前端 `TerminalPane` 140 ms 防抖后 `termResize(termId, term.cols, term.rows)` |
| F6 | spawn 记 `pgid = child.process_id()`（unix `setsid` ⇒ `pid==pgid`）；`terminate_group`：`SIGTERM` → 50 ms 轮询 `script_runner::process_group_alive`（2 s）→ `SIGKILL` → `wait()`。**复用 M2-4.b 的 pub 函数，未改 `script_runner` 冻结面** |
| F7 | session 持 `stop: Arc<AtomicBool>` + `handles: Option<TerminalHandles>`；`terminate_session` 置 stop → 杀组 → `join_pipeline`（轮询 `is_finished` 2 s，超时不 join，只告警，避免阻塞退出路径） |
| F8 | `send_with_backoff`：基数 10 ms × 2^n（上限 30 s），总预算 60 s；超预算 → `stop=true` + `killpg(pgid, SIGKILL)`（`on_channel_dead` 回调）+ 发 `kind:"exit", reason:"channel_dead"`（最后一帧不再退避） |
| F9 | 零新依赖：只用 `std::sync::mpsc` + `std::thread` + 既有 `libc`；夹具锚定 `Cargo.toml` 无 `tokio`/`crossbeam` |
| F10 | `bridge.ts` 从 `@tauri-apps/api/core` 导入 `Channel`，新增 `createTermChannel(cb)` / `termSpawnChannel(ch)`；`useSystemStore.startShell` 改走 Channel 并经 `onTermChannelMsg(msg)` 按 `kind` 分发（data/flow/exit）；`termBuffer` 兜底保留；`types.ts` 增 `TermMessage`/`TermExitReason` 镜像 |
| F13 | 终端为交互式 shell，命令由用户键入，无新攻击面：**不写 `audit.json`**；`channel_dead` 走 `eprintln!`（既有 stderr 镜像入 `session-*.log`）。若将来开放程序化批量 `term_write` 须重估 |
| F2/F3 测量兼容 | bridge.rs 增 `term_measure_mode(app)`（`m0_config_of(app).driver == "term-throughput"`），测量模式走大队列（4096）+ 窗口 0 + 不丢帧，保证 `__M0_TERM_BEGIN__/__M0_TERM_END__` 计数与 M0-0.b 基线可比 |

补充裁定（实现期新增，记入本卡）：
- **`App.vue:117` 的 `onTermData` 全局注册保留**——`term_spawn`（Event sink）仅供 `main.rs` 的 M0-6.c scenario-8 与 `M0_DRIVER` 资源循环使用，保留监听使 M0 回归行为完全不变；前端终端面板一律走 Channel。
- `term_resize` 由「静默空实现」改为「真实 resize」，属**正向修复**，不是契约放宽。

---

## 二、落地改动清单

1. `src-tauri/src/terminal.rs`（新，约 560 行）：常量、消息契约（`TermMessage`/`TermExitReason`/`to_value`）、`TermOutput` trait + 双 sink、纯策略函数（`should_flush`/`next_backoff_ms`/`within_retry_budget`/`DropCounter`）、`start_pipeline`/`run_pump`/`flush_batch`/`notify_drops`/`send_with_backoff`、`spawn_terminal`/`resize`/`terminate_session`/`terminate_group`/`join_pipeline` + 6 个单测。
2. `src-tauri/src/bridge.rs`：删除旧 `TerminalSession`/`TermInfo`/四命令实现，改为 `pub use crate::terminal::{TermInfo, TerminalSession}` + 命令壳（`term_spawn` / `term_spawn_channel` / `term_write` / `term_resize` / `term_kill`）+ `term_measure_mode`；M0-2 `kill-terminals` 生命周期任务改调 `terminal::terminate_session`。
3. `src-tauri/src/main.rs`：`mod terminal;` + 注册 `bridge::term_spawn_channel`。
4. `src-tauri/permissions/default-commands.toml`：新增 `"term_spawn_channel"`。
5. `src/bridge.ts`：`Channel` 从 `@tauri-apps/api/core` 导入；新增 `createTermChannel` / `termSpawnChannel`；移除 `termSpawn`。
6. `src/types.ts`：新增 `TermMessage` / `TermExitReason`。
7. `src/stores/useSystemStore.ts`：`startShell` 改 Channel 入口；新增 `onTermChannelMsg`（按 kind 分发）+ `droppedChunks/droppedBytes/resetDroppedStats`。
8. `src/components/system/TerminalPane.vue`：`scheduleResize`（`TERM_RESIZE_DEBOUNCE_MS=140`）替换原直接 `fit.fit()`；卸载清理定时器；头部展示丢弃字节数。
9. `scripts/check-terminal-policy.py`（新）：16 个不变量 + 16 个变异坏样本。
10. `scripts/pre-merge.sh`：新增第 21 项（M3.a 终端夹具），更新头部清单与 self-test 存在性检查。

---

## 三、门禁结果（2026-09-05 20:33）

| 门禁 | 结果 |
|------|------|
| `cargo test`（src-tauri） | ✅ 231/231（新增 `terminal::tests` 6 项：flush_policy / backoff / drop_counter / object_safe / **真实 PTY 输出到达 sink** / **进程组整组回收**） |
| `python3 scripts/check-terminal-policy.py --self-test` | ✅ 好样本零违规 + **16/16 坏样本全检出**（含变异防呆） |
| `python3 scripts/check-terminal-policy.py` | ✅ `terminal pipeline policy: all invariants hold` |
| `npm run build` | ✅ 0 error（144 modules） |
| `cargo check --locked` | ✅ 0 error；warning 数维持既有 2 类（`grid_process.rs` 死代码），**未新增** |
| `cargo fmt --check`（src-tauri） | ✅ |
| `bash scripts/pre-merge.sh` | ✅ `PRE_MERGE_RESULT=ALL_PASS` |
| 构建指标对比 | ✅ 未回归（总体积增长 ≤15%、warning 不增加） |

真实进程取证（机器可取，非人工目视）：
- T2 `pipeline_delivers_real_pty_output_and_stops`：真实 `openpty` + `/bin/sh`，写入 `echo __T_OK__`，5 s 内 pump 把输出送到 sink；随后置 `stop`，2 s 内 worker/pump 均 `is_finished()`。
- T3 `terminate_group_kills_whole_process_group`：shell 内起 `sleep 300`，`terminate_group` 后 `process_group_alive(pgid)==false` 且 `/proc/<pgid>` 不存在（**sleep 未成孤儿**）。

---

## 四、范围外（移交）

- **GUI 实点验收（D23 挂账）**：终端面板目视验收（提示符、拉伸不错位、`cat` 大文件不卡）无 GUI 自动化通道，沿用 D20 口径**不伪造**。
- **M0-0.b 吞吐基线未在新管道下重采（D24 挂账）**：测量模式已按 F2/F3 走直通以保持语义可比，但未在 `driver=term-throughput` 下实跑一次新采集，需在下次 M0 基线重采时一并验证。
- **M3.c（WBS M3-4）**：临时历史 40 条、resize 静默窗口未实现。
- **M3 整体裁定**：建议跨模型复核（本卡实现与裁定同模型，独立性弱）。

---

## 五、经验沉淀

- **给既有命令加 `Channel` 参数前必须查内部调用方**：`term_spawn` 被 `main.rs` 的 M0 回归驱动以 Rust 方式调用（无 Channel 可传）；且 Tauri 2 的 `Channel` **未实现 `Deserialize`**，`Option<Channel>` 会落入 `impl<D: Deserialize> CommandArg for D` 而编译失败。结论是新增命令而非改签名（`term_spawn_channel`），M0 工具零改动。
- **`Child` drop 不会杀进程**：fileterm 草案「worker 靠 child drop → EOF 退出」在本项目不成立，必须显式杀进程组 + `stop` 标志。
- **测量/基线通道不能被新优化污染**：M0-0.b 吞吐依赖 begin/end 标记与字节计数，合并与丢弃都会让历史基线失真，必须为测量模式留直通开关。
- 夹具变异避坑（沿用 M2-9）：token 必须**不含原串子串**且**全量替换**；`mutate(**kw)` 的键名要与 `read_repo` 的键完全一致（本次曾写成 `acl=` 而键是 `acl_toml=`，导致「未改动」防呆正确拦截）。
