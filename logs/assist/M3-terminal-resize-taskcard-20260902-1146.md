# M3-terminal-resize-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §5 **M3-2**（终端尺寸自适应）· 需求 **#9 借鉴/集成 fileterm 终端**
> 状态：⏸ **任务卡 / 未改代码 / 未执行验收 / 不宣称 PASS**
> 关联：勘误 **E2 + E3**（`errata-to-taskcards-20260902-1146.md`）→ **E2/E3 必须同 PR**

---

## 1. 目标

让 PTY 真实跟随前端 xterm 尺寸变化，使 `vim` / `top` / `htop` / `less` 等依赖 `TIOCSWINSZ` 的全屏程序显示正确。

---

## 2. 现状证据（2026-09-02 实测）

### 2.1 后端空实现

```rust
// src-tauri/src/bridge.rs:2011-2016
/// 调整终端大小（列/行）。
#[tauri::command]
pub fn term_resize(app: AppHandle, id: String, cols: u16, rows: u16) -> Result<(), String> {
    let _ = (app, id, cols, rows);      // ← 参数全吞，从不调 MasterPty::resize
    Ok(())
}
```

### 2.2 会话结构缺 `master`

```rust
// src-tauri/src/bridge.rs:114-117
pub struct TerminalSession {
    pub writer: Box<dyn std::io::Write + Send>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
    // ❌ 无 master → 无处调用 resize
}
```

### 2.3 `term_spawn` 丢弃了 master

```rust
// src-tauri/src/bridge.rs:1926-1995（关键片段）
let pair = pty_system.openpty(PtySize { rows: 24, cols: 100, pixel_width: 0, pixel_height: 0 })?;
let child = pair.slave.spawn_command(cmd)?;
drop(pair.slave);
let mut writer = pair.master.take_writer()?;        // 借 &mut pair.master
let _ = writer.write_all(b"\n");
let reader = pair.master.try_clone_reader()?;
// ❌ pair.master 之后再未被保存 → 结构体构造时只放了 writer + child
let session = TerminalSession { writer: Box::new(writer), child };
```

### 2.4 前端零调用

```bash
$ grep -rn "termResize" src/
src/bridge.ts:208-209:  termResize: (id, cols, rows) => invoke("term_resize", { id, cols, rows }),
# 仅定义，零调用方
```

```ts
// src/components/system/TerminalPane.vue:55-58
resizeObserver = new ResizeObserver(() => {
  fit?.fit();          // 只重排 xterm 前端，不通知 PTY
});
```

### 2.5 依赖版本

- `portable-pty = "0.8"`（`src-tauri/Cargo.toml`）
- `@xterm/xterm ^6.0.0`、`@xterm/addon-fit ^0.11.0`（`package.json`）

---

## 3. 必改文件候选

| 文件 | 改动 | 必要性 |
|---|---|---|
| `src-tauri/src/bridge.rs` | ① `TerminalSession` 增 `master` 字段 ② `term_spawn` 保存 master ③ `term_resize` 真实调用 `master.resize()` | 必须 |
| `src/components/system/TerminalPane.vue` | `ResizeObserver` 回调内取 `proposeDimensions()` → 调 store 的 resize | 必须 |
| `src/stores/useSystemStore.ts` | 新增 `resizeShell(cols, rows)`，含节流 + 去重 + 静默失败 | 必须 |
| `src/bridge.ts` | `termResize` 增加注释说明静默策略（可选微调） | 可选 |

**不需要改**：`src-tauri/permissions/default-commands.toml`（`term_resize` 已在 59 个命令中）、`src-tauri/src/main.rs`（已注册）。

---

## 4. 契约 / 数据结构

### 4.1 后端

```rust
// src-tauri/src/bridge.rs
pub struct TerminalSession {
    /// 保留 master 以便后续 resize（portable-pty 0.8 的 resize 只需 &self）
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub writer: Box<dyn std::io::Write + Send>,
    pub child:  Box<dyn portable_pty::Child + Send + Sync>,
}

#[tauri::command]
pub fn term_resize(app: AppHandle, id: String, cols: u16, rows: u16) -> Result<(), String> {
    // 1) 参数护栏：0 会让部分 PTY 实现恐慌
    if cols == 0 || rows == 0 { return Ok(()); }           // 静默忽略
    let cols = cols.clamp(1, 1000);
    let rows = rows.clamp(1, 1000);

    let state = app.state::<AppState>();
    let mut terms = state.terminals.lock().unwrap();
    let session = terms.get_mut(&id).ok_or_else(|| "终端不存在".to_string())?;

    // 2) 真实 resize；失败静默（高频调用不应打扰用户）
    let _ = session.master.resize(portable_pty::PtySize {
        rows, cols, pixel_width: 0, pixel_height: 0,
    });
    Ok(())
}
```

**静默失败策略（强制）**：

| 情况 | 行为 | 理由 |
|---|---|---|
| `cols`/`rows` 为 0 | `Ok(())` 直接返回 | 布局过渡态会短暂出现 0 |
| 会话不存在（`termId` 已销毁） | 返回 `Err("终端不存在")`（**不**弹 UI） | 前端已 `.catch(() => {})` 吞掉 |
| `master.resize()` 失败 | `_ = ...` 忽略 + 可写 debug 日志 | 高频路径，UI 弹窗会造成刷屏 |
| 超过 1000×1000 | `clamp` | 防恶意/异常值 |

> 与上一批次 `M3-4.b-prework-20260902-1055.md` §「resize 静默窗口」的口径一致：静默 ≠ 无日志，失败要写 `bridge.debugLog` 便于排查。

### 4.2 前端

```ts
// src/stores/useSystemStore.ts
let resizeTimer: number | null = null;
let lastDims = { cols: 0, rows: 0 };

function resizeShell(cols: number, rows: number) {
  const id = termId.value;
  if (!id) return;                                        // 未启动 → no-op
  if (!Number.isFinite(cols) || !Number.isFinite(rows)) return;
  const c = Math.max(1, Math.min(1000, Math.floor(cols)));
  const r = Math.max(1, Math.min(1000, Math.floor(rows)));
  if (c === lastDims.cols && r === lastDims.rows) return;  // 去重
  lastDims = { cols: c, rows: r };
  if (resizeTimer) clearTimeout(resizeTimer);
  resizeTimer = window.setTimeout(() => {
    bridge.termResize(id, c, r).catch(() => {});           // 静默
  }, 80);                                                  // 节流
}
```

```ts
// src/components/system/TerminalPane.vue（onMounted 内）
resizeObserver = new ResizeObserver(() => {
  const dims = fit?.proposeDimensions();
  if (dims && dims.cols > 0 && dims.rows > 0) {
    if (term.cols !== dims.cols || term.rows !== dims.rows) {
      term.resize(dims.cols, dims.rows);
    }
    system.resizeShell(dims.cols, dims.rows);              // ← 新增
  } else {
    fit?.fit();                                            // 兜底
  }
});
resizeObserver.observe(termEl.value);
```

**注意**：`term_kill` / `startShell(true)` 重启后 `lastDims` 必须重置为 `{0,0}`，否则新会话首次 resize 会被去重吞掉。

---

## 5. 实现要点（步骤化）

1. **改 `TerminalSession`**：加 `master: Box<dyn portable_pty::MasterPty + Send>`。
   - `portable-pty 0.8` 的 `MasterPty` 是否 `Send` 需实际编译验证；若不 `Send`，改用 `Box<dyn MasterPty + Send + Sync>` 或把 `AppState.terminals` 的类型调整为 `Mutex<HashMap<String, TerminalSession>>`（已是）并确保不跨线程共享 `&mut`。
2. **改 `term_spawn`**：在 `take_writer()` / `try_clone_reader()` 之后，把 `pair.master` **move 进 session**。
   - 顺序关键：`take_writer(&mut self)` 与 `try_clone_reader(&self)` 都借用 master，必须在**最后**才 move master。
3. **改 `term_resize`**：如上 §4.1。
4. **改前端**：如上 §4.2。
5. **加 `useSystemStore.resizeShell`**，并在 `killShell`/`startShell` 里重置 `lastDims`。
6. **编译验证**：`cargo clippy` 对照 `logs/baseline-2026-08-27.md`（13 warning 基线）。

---

## 6. 禁止事项

| # | 禁止 | 原因 |
|---|---|---|
| 1 | ❌ E2 与 E3 分开提交 | 只改一端功能仍不生效，且难以二分 |
| 2 | ❌ 每个 resize 事件都 invoke | 拖动会触发数十次/秒，刷爆 IPC |
| 3 | ❌ resize 失败时给用户弹窗 | 高频路径，会刷屏 |
| 4 | ❌ 在持有 `terminals` 锁时做阻塞 IO / 发事件 | 死锁风险 |
| 5 | ❌ 用容器像素尺寸当作 cols/rows | 必须用 `fitAddon.proposeDimensions()` |
| 6 | ❌ `resize` 失败直接 `panic!` / `unwrap` | `panic = "abort"`（release profile），会崩应用 |
| 7 | ❌ 顺手重构 `term_spawn` 的读取线程 | 与 resize 无关，增加评审面积 |

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| 中 | `MasterPty` 不是 `Send` → 编译失败 | 先写最小验证：`Box<dyn MasterPty + Send>`；不行则把 master 单独存 `HashMap<String, Box<dyn MasterPty>>`（不要求 Send，仅在命令内使用） |
| 中 | 拖动窗口高频 resize 导致 PTY syscall 风暴 | 80 ms 节流 + 尺寸去重（§4.2） |
| 中 | `vim` 在 resize 瞬间的重绘残影 | 属正常终端行为；若严重，可在 resize 后延迟 50 ms 再发一次相同尺寸（「双发」策略，**仅在验证确有残影时**才加） |
| 低 | `clamp(1,1000)` 与实际终端最大尺寸不符 | 值域足够宽；极端值由 `Ok(())` 静默兜底 |
| 低 | 多终端（未来 tab 化）时 `lastDims` 串号 | `lastDims` 需按 `termId` 分桶（当前单终端，先留 TODO） |

---

## 8. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 开终端 → `vim test.txt` → 拖宽窗口 | 内容重排正确，无残影，`~` 填充不越界 |
| R2 | `top` → 拉高窗口 | 进程列表行数随窗口增加 |
| R3 | `top` → 压缩窗口到很矮 | 不崩溃，只显示表头 + 少量行 |
| R4 | `less /var/log/syslog` → 改变窗口高度 | 分页行数跟随 |
| R5 | 极端窄（<20 列） | 不 panic；PTY 仍在 |
| R6 | 1 秒内快速拖动 30 次 | 实际 invoke ≤ 13 次；CPU 无尖峰；无 panic |
| R7 | 终端未启动时（点开面板前）触发 resize | 零 invoke，后端无报错 |
| R8 | `killShell` 后立即拖窗口 | 前端静默失败，无弹窗 |
| R9 | `startShell(true)`（重启）后拖窗口 | 新会话能正常收到首个 resize（`lastDims` 已重置） |
| R10 | `htop`（若安装）→ 全屏 → 拖窗口 | 布局跟随，不出现方块乱码 |

---

## 9. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 9.1 空实现已消失
grep -n "let _ = (app, id, cols, rows)" src-tauri/src/bridge.rs
# 期望：零命中

# 9.2 master 已持有
grep -n "master" src-tauri/src/bridge.rs | head -20
# 期望：TerminalSession 中有 master 字段，term_spawn 中有保存

# 9.3 真实 resize 调用
grep -n "\.resize(" src-tauri/src/bridge.rs
# 期望：命中 master.resize(PtySize{..})

# 9.4 参数护栏
grep -n "clamp" src-tauri/src/bridge.rs | head
# 期望：命中

# 9.5 前端调用链
grep -rn "resizeShell" src/stores/useSystemStore.ts src/components/system/TerminalPane.vue
# 期望：两处命中（定义 + 调用）

# 9.6 节流与去重
grep -n "setTimeout\|clearTimeout\|lastDims" src/stores/useSystemStore.ts
# 期望：全部命中

# 9.7 编译与静态门槛
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# 对照 logs/baseline-2026-08-27.md（13 warning 基线，不得增加）

npm run build && ls -lh dist/assets/*.js
# 对照 baseline 的 505 KB 硬门槛

# 9.8 人工 GUI（AI 不得代签）
# 依次执行 §8 的 R1~R10，逐条记录实际表现
```

---

## 10. 失败动作

| 失败 | 动作 |
|---|---|
| 编译报 `MasterPty` 不满足 `Send` | 改为单存 master 的 `HashMap`（不跨线程），或把整个 `terminals` 放入 `Mutex<...>` 且仅在命令作用域内取用；**不得**用 `unsafe impl Send` 绕过 |
| `vim` 仍错位 | 核对 cols/rows 是否来自 `proposeDimensions()`；确认后端确实收到了 invoke（加临时 debug 日志） |
| 拖动时终端卡顿 | 节流窗口 80 → 120 ms；或改用 `requestAnimationFrame` 合并 |
| clippy 新增 warning | 对照 baseline，新增即回退 |
| 主 JS 体积超 505 KB | 检查是否误引入依赖；不得放宽阈值 |
| 重启 shell 后首次 resize 无效 | 重置 `lastDims`（§4.2 注意事项） |

---

## 11. 推荐模型

`AI:DEEP`（Rust 所有权 + portable-pty API + 前端 xterm fit 链路 + 人工 GUI 验收四方协同）。
**必须人工 GUI 验收**：R1（`vim`）、R2（`top`）、R10（`htop`）无法用静态检查替代。
