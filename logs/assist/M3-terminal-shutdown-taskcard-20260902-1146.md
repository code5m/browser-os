# M3-terminal-shutdown-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §5 **M3-1** + **M0-2**（终端生命周期 / 关闭窗口资源释放）· 需求 **#3 关闭窗口能否正确释放资源** / **#9 终端**
> 状态：⏸ **任务卡 / 未改代码 / 未执行验收 / 不宣称 PASS**
> 关联：勘误 **E4** · 🚨 **本任务是 5 个下游任务的共同阻塞项，优先级最高**

---

## 1. 目标

建立统一的 **ShutdownCoordinator**，覆盖四条退出路径（主窗关闭 / 系统退出 / tab 关闭 / 面板卸载），确保 PTY 及其**整个子进程树**被回收；并定义脚本、插件、Agent 等未来长生命周期资源**如何接入**。

---

## 2. 现状证据（2026-09-02 实测）

### 2.1 主窗关闭只收宫格

```rust
// src-tauri/src/main.rs:591-593
WindowEvent::CloseRequested { .. } => {
    state.grid_manager.shutdown_all();     // ← 只收宫格子进程
    // ❌ 完全不碰 state.terminals
}
```

### 2.2 `.run()` 无 `RunEvent` 分支

```rust
// src-tauri/src/main.rs:674-678
.run(tauri::generate_context!())
    .unwrap_or_else(|e| {
        let _ = std::fs::write("/tmp/mvp-life.log", format!("RUN_ERROR: {e}\n"));
        std::process::exit(1);
    });
// ❌ 没有 .on_event / RunEvent match → 无 Exit / ExitRequested 钩子
```

### 2.3 8 处硬退出跳过析构

```bash
$ grep -n "std::process::exit" src-tauri/src/main.rs
107, 146, 157, 248, 252, 256, 468, 677     # 共 8 处
```

→ `std::process::exit` 不运行析构函数，**`Drop` 兜底不可靠**。

### 2.4 前端卸载不杀进程

```ts
// src/components/system/TerminalPane.vue:66-70
onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  term?.dispose();                 // 只销毁 xterm 视图
  system.bindTermWriter(null);     // ❌ 不调 term_kill
});
```

### 2.5 现有能力

```rust
// src-tauri/src/bridge.rs:2019-2027
#[tauri::command]
pub fn term_kill(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut terms = state.terminals.lock().unwrap();
    if let Some(mut s) = terms.remove(&id) {
        let _ = s.child.kill();     // ⚠️ 仅 kill 直接子进程，非进程组
    }
    Ok(())
}
```

```rust
// src-tauri/src/bridge.rs:102
pub terminals: Mutex<HashMap<String, TerminalSession>>,
```

---

## 3. 必改文件候选

| 文件 | 改动 | 必要性 |
|---|---|---|
| **新增** `src-tauri/src/shutdown.rs` | `ShutdownCoordinator`（注册/注销/统一关闭 + 超时） | 必须 |
| `src-tauri/src/main.rs` | ① `mod shutdown;` ② `.run()` 加 `RunEvent` 分支 ③ `CloseRequested` 接入 coordinator ④ 审视 8 处 `process::exit` | 必须 |
| `src-tauri/src/bridge.rs` | ① 新增 `term_kill_all`（或复用）② `term_kill` 改为进程组回收 ③ 注册进 coordinator | 必须 |
| `src-tauri/permissions/default-commands.toml` | 新增 `term_kill_all`（**K1**：漏加会被 ACL 静默拒绝） | 必须 |
| `src/components/system/TerminalPane.vue` | 区分「隐藏面板」vs「真正关闭」；真正关闭时调 kill | 必须 |
| `src/stores/useSystemStore.ts` | `killShell()` 明确语义；新增 `disposeShell()` | 必须 |

---

## 4. 契约 / 数据结构

### 4.1 `ShutdownCoordinator`

```rust
// src-tauri/src/shutdown.rs（草案）
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 一个可被统一关闭的资源
pub trait Shutdown: Send + Sync {
    /// 资源名，用于日志与超时报告
    fn name(&self) -> &str;
    /// 优雅关闭；返回的 Duration 是本资源希望的宽限期
    fn shutdown(&self) -> Duration;
}

pub struct ShutdownCoordinator {
    items: Mutex<Vec<Arc<dyn Shutdown>>>,
    /// 全局宽限期上限
    grace: Duration,
}

impl ShutdownCoordinator {
    pub fn new(grace: Duration) -> Self { /* … */ }

    pub fn register(&self, item: Arc<dyn Shutdown>) { /* … */ }

    /// 统一关闭：并发通知，串行等待，超时即放弃（不阻塞退出）
    pub fn shutdown_all(&self) -> ShutdownReport {
        let start = Instant::now();
        let mut report = ShutdownReport::default();
        let items = self.items.lock().unwrap().clone();
        for it in items {
            let want = it.shutdown();
            let budget = want.min(self.grace.saturating_sub(start.elapsed()));
            // ⚠️ 关键：不得无限 join；用 budget 上限
            report.records.push(ShutdownRecord {
                name: it.name().to_string(),
                elapsed: /* 实际等待 */,
                timed_out: /* … */,
            });
        }
        report
    }
}

#[derive(Default)]
pub struct ShutdownReport {
    pub records: Vec<ShutdownRecord>,
}
pub struct ShutdownRecord {
    pub name: String,
    pub elapsed: Duration,
    pub timed_out: bool,
}
```

### 4.2 注册项清单（当前 + 未来）

| 资源 | 当前状态 | 接入方式 | 阻塞任务 |
|---|---|---|---|
| **PTY 终端** | ✅ 存在（`terminals`） | `TerminalShutdown`：`term_kill_all` + 进程组 kill | 本任务 |
| 宫格子进程 | ✅ 存在（`grid_manager.shutdown_all()`） | 包装为 `GridShutdown` 注册 | — |
| 脚本子进程 | ❌ 未实现 | `ScriptShutdown`：按 runId 遍历进程组 kill | **M2-1/M2-2 脚本执行** |
| 数据库连接池 | ❌ 未实现 | `DbShutdown`：优雅 close + 超时放弃 | **M4-1~M4-4 数据库** |
| 定时任务线程 | ❌ 未实现 | `SchedulerShutdown`：停调度器 + 停表 + 终止已触发进程 | **M4-5~M4-8 定时任务** |
| Agent 流式任务 / Skill | ❌ 未实现 | `AgentShutdown`：取消 in-flight 请求 + 杀子进程 | **M5-4~M5-6 Agent/Skill** |
| 插件资源 | ❌ 未实现 | `PluginShutdown`：按 manifest 声明释放 | **M5-10~M5-12 插件运行时** |

### 4.3 退出路径矩阵

| 路径 | 触发点 | 应关闭什么 | 现状 | 目标 |
|---|---|---|---|---|
| **主窗关闭** | `WindowEvent::CloseRequested`（`main.rs:591`） | 全部（终端 + 宫格 + 未来的脚本/DB/调度/插件） | ❌ 只收宫格 | ✅ 调 `coordinator.shutdown_all()` |
| **系统退出**（`ExitRequested`） | `.run()` 的 `RunEvent`（**待新增**） | 全部 | ❌ 无分支 | ✅ 同上 + `app.exit(0)` |
| **应用已退出**（`RunEvent::Exit`） | 同上 | 兜底记录 | ❌ 无分支 | ✅ 写 `/tmp/mvp-life.log` 收尾报告 |
| **tab 关闭** | 未来的终端 tab | 仅该 tab 的 PTY | ❌ 无 tab 化 | ✅ 按 id kill，不经 coordinator |
| **面板隐藏**（点 ✕） | `TerminalPane.vue` 的 `system.terminalOpen = false` | **什么都不关** | ⚠️ 当前语义正确 | ✅ 保持（切回来会话还在） |
| **组件卸载**（`onBeforeUnmount`） | 路由/面板销毁 | 视语义决定 | ❌ 当前只 dispose xterm | ✅ 显式区分：隐藏=不杀，销毁=杀 |
| **重启 shell**（点 ↻） | `startShell(true)` | 仅当前 PTY | ⚠️ 需确认已杀旧 | ✅ 显式 `term_kill` 再 spawn |

### 4.4 进程组回收（关键）

当前 `s.child.kill()` **只杀直接子进程**。若用户在终端里跑了 `npm run dev`（会派生子进程树），孙子进程会变孤儿。

```rust
// 目标语义（伪码，实现时需按 portable-pty 0.8 API 校准）
fn kill_session(s: &mut TerminalSession) {
    // 1) 先优雅：SIGTERM（portable-pty 的 kill 通常是 SIGKILL，需确认）
    let _ = s.child.kill();
    // 2) 兜底：进程组 kill
    //    Linux：libc::killpg(pgid, SIGKILL)，pgid 取自 child 的 pid
    //    ⚠️ portable-pty 0.8 是否暴露 pid / 进程组需实际查证（见 §7 风险）
    // 3) 等待子进程被回收，避免僵尸
    let _ = s.child.wait();
}
```

**顺序强制**：`SIGTERM` → 等待宽限（默认 500 ms）→ `SIGKILL`（进程组）→ `wait()` 防僵尸。

---

## 5. 实现要点（步骤化）

1. **新建 `src-tauri/src/shutdown.rs`**：`Shutdown` trait + `ShutdownCoordinator` + `ShutdownReport`。
2. **`main.rs` 加 `mod shutdown;`**，并在 `.setup()` 中创建 `Arc<ShutdownCoordinator>`，`.manage()` 进 state。
3. **包装现有资源**：
   - `TerminalShutdown`：持有 `AppHandle`，`shutdown()` 内遍历 `terminals` 逐个 kill。
   - `GridShutdown`：包装 `grid_manager.shutdown_all()`。
4. **改 `.run()`**：

```rust
.run(tauri::generate_context!())
```
→
```rust
.run(|app, event| match event {
    tauri::RunEvent::ExitRequested { api, .. } => {
        // 有长任务时可 api.prevent_exit()，完成后再 app.exit(0)
        let _ = api;
    }
    tauri::RunEvent::Exit => {
        let report = /* coordinator.shutdown_all() */;
        let _ = std::fs::write("/tmp/mvp-life.log", format!("EXIT: {report:?}\n"));
    }
    _ => {}
})
```

> ⚠️ 注意：`ExitRequested` 与 `WindowEvent::CloseRequested` 会**重复触发**。设计上必须让 `shutdown_all()` **幂等**（第二次调用应无副作用且快速返回）。

5. **改 `CloseRequested`**：`state.grid_manager.shutdown_all()` → `state.coordinator.shutdown_all()`。
6. **`term_kill` 改为进程组回收**（§4.4）。
7. **新增 `term_kill_all`** 并加入 `default-commands.toml`（K1）。
8. **前端**：`TerminalPane.vue` 的 `onBeforeUnmount` 按语义分流（见 §4.3）。
9. **审视 8 处 `process::exit`**：业务路径（如自检结束 `main.rs:468`）改为正常返回或走 coordinator；仅在启动阶段致命错误时保留。

---

## 6. 禁止事项

| # | 禁止 | 原因 |
|---|---|---|
| 1 | ❌ 只靠 `Drop` 兜底 | 8 处 `process::exit` 会跳过析构 |
| 2 | ❌ 在事件回调里同步 `join` 无超时的线程 | 死锁 UI 线程，窗口关不掉 |
| 3 | ❌ 一上来就 `SIGKILL` | 脚本来不及清理临时文件 |
| 4 | ❌ 「隐藏面板即杀进程」 | 用户切回来会话就没了（语义错误） |
| 5 | ❌ `shutdown_all()` 非幂等 | `ExitRequested` + `CloseRequested` 会双触发 |
| 6 | ❌ 在 `CloseRequested` 里做耗时 IO（写大文件、网络请求） | 关闭卡顿 |
| 7 | ❌ 新增命令忘了进 `default-commands.toml` | **K1**：ACL 静默拒绝，无日志，极难排查 |
| 8 | ❌ 用 `unsafe` 强行拿 pid | 先查证 portable-pty 0.8 的公开 API |

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| **高** | `portable-pty 0.8` 的 `Child` 是否暴露 pid / 进程组 API **未查证** | 实现前必须先查文档；若无，退而求其次：① `child.kill()` + `wait()` ② 用 `/proc/<pid>/task/*/children` 扫描（需 pid） ③ 若都不可行，**明确记录能力缺口**并降级为「尽力而为」 |
| 高 | `ExitRequested` + `CloseRequested` 双触发导致重复 kill / 卡死 | `ShutdownCoordinator` 用 `AtomicBool` 保证幂等 |
| 中 | `api.prevent_exit()` 使用不当导致应用关不掉 | 默认**不** prevent；仅在确有异步清理时短暂 prevent 并设超时兜底 |
| 中 | 改 8 处 `process::exit` 引入启动/自检回归 | 逐处改 + 每处单独 commit；自检路径（`main.rs:468`）优先保留 |
| 中 | 杀进程组误伤（PTY 与宿主共享进程组） | 必须先确认 PTY 子进程是否 `setsid` 成为组长；若与宿主同组，**绝对不能** `killpg` 宿主的组 |
| 低 | 僵尸进程 | 每个 kill 后 `wait()` |

---

## 8. 下游阻塞清单（本任务不完成，以下全部无法安全开工）

| 下游任务 | 文档 | 为何被阻塞 |
|---|---|---|
| **M2-1/M2-2 脚本执行** | `script-execution-safety-taskcard-20260902-1146.md` | 长生命周期进程，无收口 = 新泄漏源 |
| **M4-1~M4-4 数据库** | `database-schema-taskcard-20260902-1146.md` | 连接池/写锁需优雅关闭，否则数据损坏 |
| **M4-5~M4-8 定时任务** | `scheduled-task-taskcard-20260902-1146.md` | timer 线程 + 被触发子进程都要收 |
| **M5-4~M5-6 Agent/Skill** | `agent-skill-contract-taskcard-20260902-1146.md` | 流式任务取消 + 子进程回收 |
| **M5-10~M5-12 插件运行时** | `plugin-runtime-taskcard-20260902-1146.md` | 插件持有资源需按 manifest 释放 |

---

## 9. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 开终端执行 `sleep 300 &` → 关闭主窗 → `ps -ef \| grep "[s]leep 300"` | **0 命中** |
| R2 | 开终端跑 `yes > /dev/null` → 退出应用 → 观察 CPU | 回落到基线，无 `yes` 残留 |
| R3 | 开终端跑 `npm run dev`（有子进程树）→ 关闭 | 子进程树全部回收（不只直接子进程） |
| R4 | 隐藏终端面板（点 ✕）→ 再打开 | **会话仍在**（`echo hi` 有响应） |
| R5 | 点 ↻ 重启 shell | 旧进程被杀，新会话可用 |
| R6 | 连续点两次「关闭窗口」（快速双击） | 不卡死、不重复报错 |
| R7 | 应用内有未结束的 `sleep` 时点关闭 | 窗口在 ≤3 s 内关闭（宽限期上限） |
| R8 | `kill -9` 主进程（模拟崩溃） | 不强求（尽力而为），但重启后不应有脏状态 |
| R9 | 终端里 `exit` | shell 自行退出，前端显示「[终端已退出]」，无残留 |
| R10 | `term_kill_all` 调用两次 | 第二次无副作用、无报错（幂等） |
| R11 | 关闭时 `/tmp/mvp-life.log` | 有 `EXIT:` 收尾报告，含各资源耗时 |

---

## 10. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 10.1 coordinator 已建立
ls -l src-tauri/src/shutdown.rs
grep -n "mod shutdown" src-tauri/src/main.rs
# 期望：均命中

# 10.2 RunEvent 分支
grep -n "RunEvent" src-tauri/src/main.rs
# 期望：ExitRequested / Exit 均命中

# 10.3 CloseRequested 已接入 coordinator
sed -n '588,596p' src-tauri/src/main.rs
# 期望：出现 coordinator.shutdown_all()

# 10.4 新增命令已进 ACL（K1）
grep -n "term_kill_all" src-tauri/permissions/default-commands.toml
# 期望：命中

# 10.5 硬退出收敛
grep -c "std::process::exit" src-tauri/src/main.rs
# 期望：< 8（记录实际值并说明保留项）

# 10.6 进程组回收
grep -n "killpg\|SIGKILL\|SIGTERM" src-tauri/src/bridge.rs src-tauri/src/shutdown.rs
# 期望：命中（或明确记录能力缺口）

# 10.7 动态：孤儿进程检测（人工 GUI）
#   a) 启动应用，终端执行：sleep 300 &
#   b) 关闭窗口
#   c) 宿主机执行：
ps -ef | grep -c "[s]leep 300"          # 期望 0
pgrep -c -f "mvp-browser-os" || true    # 期望 0 或仅残留守护

# 10.8 收尾报告
cat /tmp/mvp-life.log                    # 期望有 EXIT: 行

# 10.9 编译门槛
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# 对照 logs/baseline-2026-08-27.md（13 warning）
```

---

## 11. 失败动作

| 失败 | 动作 |
|---|---|
| 仍有 `sleep 300` 残留 | 先确认 `RunEvent::Exit` 是否真的被触发（打日志）；再确认 kill 路径是否执行；**不得**靠「延长 sleep 时间」掩盖 |
| 关闭窗口卡住 >3 s | 检查是否有无超时 `join`；加宽限期上限 |
| 隐藏面板后会话丢失 | 立即回退「卸载即杀」的改动；补 R4 用例 |
| `portable-pty` 无 pid/进程组 API | **明确记录能力缺口**在本文档 §7，降级为「kill 直接子进程 + wait」，并在汇总中标注为已知限制；不得用 `unsafe` 绕过 |
| 新增命令被 ACL 静默拒绝 | 按 K1 查 `default-commands.toml`；这是本项目历史高频坑 |
| clippy warning 增加 | 对照 baseline，新增即回退 |

---

## 12. 推荐模型

`AI:DEEP`（进程生命周期 + Tauri 事件循环 + Rust 并发 + 人工 GUI 验收）。
方案评审建议 `AI:DEEP-xhigh`（`ExitRequested`/`prevent_exit` 语义与幂等设计值得先评审再动手）。
**人工 GUI 验收必做**：R1/R2/R3/R7 无法静态替代。
