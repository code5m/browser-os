# M3 · 整体裁定（独立复核）

- 复核对象：M3-1 / M3-2 / M3-3（`M3.a`），证据链 `logs/checkpoints/M3-20260905-2030.md`（展开卡，冻结 F1~F13）+ `logs/checkpoints/B-M3.a-terminal-pipeline-20260905-2035.md`（落地卡）
- 复核人：本 agent（CodeBuddy Hy4）；**独立性声明：与 M3.a 实现者为同一 agent，独立性弱于跨模型复核**。为弥补，本裁定全部结论以「源码级比对 + 独立复跑门禁 + 真实进程取证」为准，不引用实现卡的自述结论。
- 时间：2026-09-05 21:01 CST

---

## 一、裁定结论

**`PASS_WITH_DEBT`**（`AI:DEEP / R:high`）。

- F1~F13 **全部成立**，无 BLOCKER，未突破 M0 四条 P0 红线。
- 复核发现 **2 项待整改**，已在本检查点内落地并复验（见 §三），不再另开 `M3-fix` 卡。
- 挂账债务：**D23**（终端 GUI 实点验收）、**D24**（M0-0.b 吞吐基线未在新管道下重采）、新增 **D25**（`on_channel_dead` 只 `killpg` 不 `wait`，异常路径下子进程短时僵尸）。

---

## 二、逐条裁定（源码级复核，非采信自述）

| 码 | 复核方法 | 结论 |
|----|----------|------|
| **F1 三段式** | 读 `terminal.rs:210-271`：`sync_channel(capacity)` + worker 只 `try_send`；pump 独立 `std::thread` | ✅ 成立。worker 无阻塞点（非测量模式） |
| **F2 背压/丢弃** | `DropCounter`（worker 计数，`:173-199`）→ `notify_drops`（pump 取走上报，`:354-376`）；上报走独立 `kind:"flow"`，未向终端字节流插任何伪内容 | ✅ 成立 |
| **F3 合并** | `should_flush` 纯函数（`:154`）+ `recv_timeout(flush_interval_ms.max(1))`；窗口 16 ms / 64 KiB | ✅ 成立（`max(1)` 规避 `recv_timeout(0)` 忙轮询，实现期正确处理） |
| **F4 双入口** | `trait TermOutput` + `EventSink`/`ChannelSink`；`term_spawn`（Event，`main.rs` 内部驱动）与 `term_spawn_channel`（Channel，前端入口）共享 `spawn_terminal`；`main.rs:778/1000` 零改动 | ✅ 成立，M0 回归路径未被破坏 |
| **F5 resize** | `TerminalSession` 持 `master`（`:410`）；`terminal::resize` → `master.resize(PtySize)`；`bridge.rs` 不再是 `let _ = (app,id,cols,rows)` | ✅ 成立（**本轮补了运行时取证，见 §三 P1**） |
| **F6 进程组回收** | `terminate_group`：`SIGTERM` → 50 ms 轮询 `script_runner::process_group_alive` → `SIGKILL` → `wait()`；复用 M2-4.b 的 pub 函数，`script_runner.rs` 冻结面未动 | ✅ 成立；T3 真实 `sleep 300` 整组回收由本人复跑通过 |
| **F7 关闭协议** | `stop: Arc<AtomicBool>`；pump 退出条件 = `Disconnected` 或（`stop` 且排空）；`join_pipeline` 轮询 `is_finished` 且有 2 s 上限（超时不 join，只告警） | ✅ 成立，退出路径不会被线程拖住 |
| **F8 退避** | `next_backoff_ms`（10 ms × 2^n，封顶 30 s）+ `within_retry_budget`（60 s）；超预算 → 置 stop → `killpg(SIGKILL)` → 发 `exit{reason:channel_dead}`（最后一帧不再退避） | ✅ 成立，无无限重试 |
| **F9 零新依赖** | `Cargo.toml` 无 `tokio`/`crossbeam`；只用 `std::sync::mpsc` + `std::thread` + 既有 `libc`，夹具已锚定 | ✅ 成立 |
| **F10 前端** | `bridge.ts` 从 `@tauri-apps/api/core` 导入 `Channel`；`useSystemStore.startShell` 走 `termSpawnChannel` 并按 `kind` 分发；`termBuffer` 兜底保留；M0 吞吐钩子只换数据来源、逻辑未变 | ✅ 成立 |
| **F11 夹具** | `scripts/check-terminal-policy.py`：17 不变量 / 17 坏样本（本轮 +1），已接 `pre-merge.sh` 第 21 项 | ✅ 成立 |
| **F12 范围外** | M3-4 体验项（临时历史 40 条、resize 静默窗口）确认未实现，登记 `M3.c` | ✅ 成立，无隐性扩范围 |
| **F13 审计** | 终端为交互式 shell、命令由用户键入，未新增程序化批量执行面 ⇒ 不写 `audit.json`；`channel_dead` 走 `eprintln!` 入 `session-*.log` | ✅ 成立（**前置条件须随 `term_write` 的程序化调用场景重估，已写入 O-2**） |

---

## 三、复核发现与整改（本检查点内落地）

### P1 · T4 真实 resize 无运行时取证（已整改）

- **问题**：冻结测试矩阵 T4 要求「`master.resize(120,40)` → `get_size()` = 120×40」，实现期被替换为编译期的 `term_output_is_object_safe`（只证明 trait 对象安全）。静态夹具只能证明「代码里调了 `resize`」，**证明不了 resize 真的生效**——而 F5 正是从「静默空实现」翻正的功能点，缺运行时证据等于把最关键的一条留给了 D23 目视验收。
- **整改**：新增 `terminal::tests::resize_changes_real_pty_size`——真实 `openpty` + `/bin/sh` → `resize(&mut session,120,40)` → `master.get_size()` 断言 `cols==120 && rows==40` → `terminate_session` 收口（不留孤儿）。
- **复验**：`cargo test terminal` 8/8，`resize_changes_real_pty_size ... ok`。

### P2 · `term_kill` 持表锁执行最长约 4 s 的终止（已整改）

- **问题**：
  ```rust
  let mut terms = state.terminals.lock().unwrap();
  let Some(mut session) = terms.remove(&id) else { return Ok(()); };
  terminal::terminate_session(&mut session)   // 锁仍未释放
  ```
  `terminate_session` = 进程组宽限（≤2 s）+ 线程 join（≤2 s）。旧实现只 `child.kill()+wait()`（毫秒级），改造后**同一把锁的持有时间被放大到约 4 s**，期间 `term_write` / `term_resize` / `term_spawn` 全部阻塞在互斥量上——用户点「关闭/重启」时会连带卡住其它终端调用。属改造引入的退化，实现卡未识别。
- **整改**：先 `drop(terms)` 再终止（session 已先从表中摘除，语义不变，不存在并发二次移除）。
- **门禁化**：夹具新增不变量 `TERM_KILL_LOCK_HELD`（`terminal::terminate_session(&mut session)` 与 `drop(terms)` 必须同时存在）+ 第 17 个变异坏样本。
- **复验**：`check-terminal-policy.py --self-test` → 17/17 全检出；默认模式 all invariants hold。

### 顺带清理

- `terminal.rs:811` 既有 `let mut cmd` 触发 `unused_mut`（测试 profile 第 3 条 warning），本轮移除；`cargo check` 的 2 类既有 warning（`grid_process.rs` 死代码）数量不变。
- 展开卡 `M3-20260905-2030.md` F11 记「pre-merge 第 22 项」，实际为第 21 项（原 21 `git diff --check` 顺延为 22），已更正。

---

## 四、门禁复跑（本人独立执行，不采信交付卡）

| 门禁 | 结果 |
|------|------|
| `cargo test`（src-tauri） | ✅ **232/232**（+1 = 新增 resize 取证；含 T2 真实 PTY 输出到达 sink、T3 `sleep 300` 整组回收无孤儿两个真实进程取证） |
| `python3 scripts/check-terminal-policy.py --self-test` | ✅ 好样本零违规 + **17/17** 坏样本全检出（含变异防呆） |
| `python3 scripts/check-terminal-policy.py` | ✅ `terminal pipeline policy: all invariants hold` |
| `npm run build` | ✅ 0 error |
| `cargo check --locked` | ✅ 0 error；warning 仍为既有 2 类，未新增 |
| `cargo fmt` | ✅ 已格式化 |
| `bash scripts/pre-merge.sh` | ✅ `PRE_MERGE_RESULT=ALL_PASS`（含 M0-4.c 构建指标对比未回归） |

---

## 五、观察项（不阻断，登记备查）

- **O-1 / D25**：`on_channel_dead` 只 `killpg(SIGKILL)`、未 `child.wait()`。通道不可恢复且前端未再调 `term_kill` 时，子进程会短时处于僵尸态，直到会话被移除或走 M0-2 生命周期。判非阻塞（有兜底、不泄漏线程/PTY），登记 **D25**。
- **O-2**：F13「不写审计」的前提是终端仅交互式使用。若后续开放程序化/批量 `term_write`（如脚本驱动终端、Agent 自动输入），须重估攻击面并补审计——与 `script_*`/`snippet_*` 同口径。
- **O-3**：`term_spawn`（Event 广播 sink）仍在主窗口 ACL 内，后端无强制隔离，仅由前端夹具禁止调用。与改造前同面，**非本次回归**；后续若做多窗口/插件能力授予需一并收敛。
- **O-4**：`onTermChannelMsg` 对「`id` 不匹配且 `termId` 已设置」的消息静默丢弃。当前架构只有单个系统终端，无影响；多终端（fileterm F15）落地时需重新设计会话-通道映射。

---

## 六、债务与后续

| 编号 | 内容 | 判定 |
|------|------|------|
| **D23** | 终端 GUI 实点验收挂账（提示符、拉伸不错位、`cat` 大文件不卡） | 沿用 D20 口径，**不伪造**；无 GUI 自动化通道 |
| **D24** | M0-0.b 吞吐基线未在新管道下重采 | 测量模式已留直通开关（`driver=term-throughput` → 大队列 4096 + 窗口 0 + 不丢帧）保语义可比，待下次基线重采时一并验证 |
| **D25** | `on_channel_dead` 未 `wait()` 回收子进程 | 新增，非阻塞，有兜底 |

后续：`M3.c`（WBS M3-4 体验项：临时历史 40 条、resize 静默窗口）为**可选卡**，不阻塞 M4。M2 里程碑状态不变（PASS）。
