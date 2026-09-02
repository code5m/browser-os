# M3-terminal-history-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §5 **M3-3**（终端历史 / 滚动缓冲）· 需求 **#9 借鉴/集成 fileterm 终端**
> 状态：⏸ **任务卡 / 未改代码 / 未执行验收 / 不宣称 PASS**
> 关联：`M3-4.b-prework-20260902-1055.md`（历史上限草案）· `M3-terminal-resize-taskcard-20260902-1146.md`

---

## 1. 目标

定义终端输出历史的**保留策略**：历史上限、敏感输出过滤、不落盘策略、前端展示、清空入口、测试夹具、验收标准。
目标是在「用户可以往回看」与「内存/隐私可控」之间取一个明确、可实现、可验收的平衡。

---

## 2. 现状证据（2026-09-02 实测）

### 2.1 后端：输出只推不存

```rust
// src-tauri/src/bridge.rs:1958-1981（term_spawn 的读取线程）
loop {
    match reader.read(&mut buf) {
        Ok(0) => break,
        Ok(n) => {
            let data = String::from_utf8_lossy(&buf[..n]).to_string();
            if !data.is_empty() {
                let _ = app2.emit("term-data",
                    serde_json::json!({ "id": tid, "data": data }));
            }
        }
        Err(_) => break,
    }
}
// ❌ emit 完即弃，后端不保留任何历史
```

### 2.2 前端：xterm 自带滚动缓冲

- `src/components/system/TerminalPane.vue` 通过 `system.bindTermWriter(d => term?.write(d))` 直写 xterm。
- xterm 自身有 `scrollback`（默认 1000 行）→ **历史实际由 xterm 在前端内存中持有**，进程退出即丢。
- ❌ 无清空历史入口；无敏感过滤；无落盘。

### 2.3 遗留物

```ts
// src/stores/useSystemStore.ts
termLines   // 注释：「兼容保留，不再用于渲染」
```

→ 早期基于行的终端实现的残留，**当前零引用**（见勘误 E6），本任务可一并清理。

### 2.4 相关既有约束

- `src-tauri/src/workspace.rs`：`audit.json` **1000 条上限**（`list.drain(0..len-1000)`）。
- `bridge.rs:1942` 注释：xterm.js 自行解析 ANSI，**不过滤**（所以 ANSI 过滤不是历史任务的范围）。

---

## 3. 必改文件候选

| 文件 | 改动 | 必要性 |
|---|---|---|
| `src/components/system/TerminalPane.vue` | ① 显式设置 `scrollback` ② 加「清空」按钮 ③ 加历史行数指示 | 必须 |
| `src/stores/useSystemStore.ts` | ① 新增 `clearTermHistory()` ② 清理 `termLines` 遗留 | 必须 |
| `src-tauri/src/bridge.rs` | **可选**：若要后端环形缓冲，改读取线程 | 见 §4.3 决策点 |
| 新增 `src/utils/sensitive.ts` | 敏感输出过滤规则（**若**决定前端过滤） | 见 §4.2 决策点 |

---

## 4. 契约 / 策略

### 4.1 历史上限

| 层 | 上限 | 依据 |
|---|---|---|
| xterm `scrollback` | **5000 行** | 与脚本执行输出上限同口径（见 `M2-script-library-ui-static-shell` §6.4），避免两套数字 |
| 单行长度 | 不截断（xterm 自行换行） | — |
| 单条 `term-data` 事件 | 4096 字节（读取缓冲，已固定） | `bridge.rs` 现有 `buf = [0u8; 4096]` |
| 超出上限 | xterm 自动丢弃最旧；**后端不做二次管理** | 保持简单 |

```ts
// TerminalPane.vue
term = new Terminal({
  scrollback: 5000,
  // … 其它选项
});
```

### 4.2 敏感输出过滤：**决策点**

| 方案 | 描述 | 优点 | 缺点 | 结论 |
|---|---|---|---|---|
| A. 不过滤 | 完全依赖 xterm 显示原始输出 | 零误伤、零维护 | 用户复制/截图可能泄露 token | ⚠️ 见下 |
| B. 前端展示层打码 | 匹配已知模式后替换为 `***` | 防肩窥 | **破坏终端语义**（用户看不到自己刚打印的 token）；正则误伤 | ❌ **不采用** |
| C. 仅「复制/导出」时提示 | 复制大段输出时若命中敏感模式给出一次提示 | 不破坏显示 | 需维护模式库 | 🟡 可选，P2 |
| D. 不落盘 + 明确告知 | 历史**只存在于内存**，且 UI 明示 | 零误伤、实现简单 | 重启即丢（本来就是） | ✅ **本卡采用** |

**本卡结论（D 为主）**：

1. **不做**展示层打码（会误导用户以为系统已经处理，反而更危险）。
2. 历史**只存在内存**，不写任何文件、不进 `audit.json`、不进日志。
3. UI 明示：终端头部显示「历史仅保留在内存中，重启后清空」。
4. 若未来要做方案 C，必须有明确的正则白名单与误伤回退，**单独开卡**。

**红线**：`term-data` 的事件内容**不得**写入 `audit.json`（高频，会刷爆 1000 条上限 —— K5）。

### 4.3 后端是否加环形缓冲：**决策点**

| 方案 | 描述 | 适用 |
|---|---|---|
| 后端不存（**推荐**） | 保持现状：读取线程只 emit | 单终端、前端持历史足够 |
| 后端加环形缓冲 | 后端存最近 N 行，`term_spawn` 后可回放 | 需要「重连/切 tab 恢复历史」时 |

**结论**：当前单终端、无 tab 化、无重连需求 → **后端不存**，保持零状态。
**触发条件**：当出现以下任一需求时，才升级为后端环形缓冲：
- 终端 tab 化（切 tab 回来要看到历史）
- 进程崩溃后重连
- 需要把终端输出作为 Artifact 归档

### 4.4 不落盘策略（强制）

| 项 | 策略 |
|---|---|
| 落盘 | ❌ **绝对不落盘**（不写文件、不写 `audit.json`、不写 debug 日志正文） |
| 跨会话 | ❌ 不恢复（重启即空，符合真实终端直觉） |
| 剪贴板 | 用户主动复制才可出内存；不自动复制 |
| 崩溃 | 随进程消亡（这是特性，不是缺陷） |
| 审计 | 只记「终端已启动/已关闭」这类**动作**，不记内容 |

```rust
// 允许：动作级审计
log_audit(app, "terminal.spawn", format!("{{ \"termId\": \"{id}\" }}"));
log_audit(app, "terminal.kill",  format!("{{ \"termId\": \"{id}\" }}"));
// 禁止：内容级
// log_audit(app, "terminal.data", data);   // ❌ 绝对禁止
```

### 4.5 前端展示

| 项 | 方案 |
|---|---|
| 头部 | 现有 `.term-head` 增加：① 历史行数指示（`⌁ 1234/5000`）② 「清空」按钮 ③ 悬停提示「历史仅保留在内存中，重启后清空」 |
| 清空 | 点「清空」→ `term.clear()` + `term.clearSelection()` + `term.scrollToBottom()`；**不杀进程、不重置 PTY** |
| 清空确认 | 行数 > 1000 时二次确认（复用 `ConfirmModal`）；否则直接清 |
| 快捷键 | `Ctrl/Cmd + L`（清屏，与真实终端一致）→ 调 `term.clear()`；另 `Ctrl/Cmd + Shift + K` 清空历史（含回滚） |
| 滚动到底按钮 | 用户上滚时出现悬浮「↓ 回到底部」按钮（xterm 无内置，需自研：监听 `onScroll`） |
| 搜索 | ❌ 本卡不做（xterm 需 `@xterm/addon-search`，会加依赖 + 体积） |

### 4.6 清空入口清单

| 入口 | 行为 | 杀进程？ |
|---|---|---|
| 头部「清空」按钮 | 清视图 + 清回滚 | ❌ |
| `Ctrl/Cmd + Shift + K` | 同上 | ❌ |
| `Ctrl/Cmd + L` / `clear` 命令 | xterm 清屏（保留回滚） | ❌（由 shell 处理） |
| 头部「↻ 重启」按钮 | `startShell(true)` → **新 PTY**，历史随旧 xterm 内容清空 | ✅ 杀旧进程 |
| 头部「⏹ 关闭进程」 | `killShell` | ✅ |
| 面板卸载 | 依赖 E4/TASK-10 的收口 | 见 `M3-terminal-shutdown-taskcard` |

**关键区分**：「清空历史」≠「关闭进程」。UI 上两者按钮必须视觉分离（清空用 🧹/⌫，关闭用 ⏹）。

---

## 5. 测试夹具（fixtures）

供人工与未来自动化使用，放在 `logs/assist/fixtures/terminal/`（实现时创建）：

| 夹具 | 内容 | 用途 |
|---|---|---|
| `gen-5000-lines.sh` | `for i in $(seq 1 5000); do echo "line $i"; done` | 验证 5000 行上限与丢弃最旧 |
| `gen-10001-lines.sh` | 10001 行 | 验证超出时的行为（应只剩 5000 行，最旧被丢） |
| `gen-long-line.sh` | 单行 200000 字符 | 验证超长行不崩溃 |
| `gen-ansi-heavy.sh` | 大量彩色 + 光标定位序列 | 验证 ANSI 不被过滤、不清空历史时错乱 |
| `gen-token-like.sh` | 输出含 `AKIA...`、`sk-...`、`Bearer eyJ...`、`password=xxx` | 验证**不过滤**（展示原样）+ 确认不落盘 |
| `gen-binary.sh` | `head -c 100000 /dev/urandom` | 验证二进制输出不崩溃（可能乱码但不应卡死） |
| `gen-slow.sh` | 每 0.5 s 输出一行，共 20 行 | 验证流式追加与自动滚动 |
| `verify-no-persist.sh` | 跑完上面任一夹具后，检查 `audit.json` 与数据目录无终端内容 | 验证不落盘 |

```bash
# logs/assist/fixtures/terminal/verify-no-persist.sh（草案，实现时创建，尚未执行）
#!/usr/bin/env bash
set -euo pipefail
DATA_DIR="${HOME}/.local/share/com.jizhijiandan.mvp/mvp-browser-os"   # 以实际 app_data_dir 为准
echo "== 检查 audit.json 是否含终端输出内容 =="
if grep -q "line 4999" "$DATA_DIR/audit.json" 2>/dev/null; then
  echo "FAIL: audit.json 含终端输出"; exit 1
fi
echo "== 检查数据目录是否新增终端历史文件 =="
if ls "$DATA_DIR" | grep -qi "term.*history\|scrollback"; then
  echo "FAIL: 发现终端历史落盘文件"; exit 1
fi
echo "PASS: 无终端内容落盘"
```

> ⚠️ `DATA_DIR` 路径需在实现时用 `app.path().app_data_dir()` 的真实值校准（Linux 下通常为 `~/.local/share/<identifier>`）。

---

## 6. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| 中 | 5000 行 × 长行 → 前端内存增长 | 单行不截断是必要的（终端语义）；5000 行对现代设备无压力；若实测有问题再降到 3000 |
| 中 | 用户误以为历史会保存 | UI 明示「仅内存，重启清空」（§4.5） |
| 中 | 「清空」被误点导致重要输出丢失 | >1000 行时二次确认；清屏 `Ctrl+L` 保留回滚 |
| 低 | 敏感信息被肩窥/截图 | 本卡明确「不解决」；如需解决另开卡（方案 C） |
| 低 | 清理 `termLines` 时误删其它引用 | 先 `grep -rn "termLines" src/` 确认零引用再删（勘误 E6） |

---

## 7. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 跑 `gen-10001-lines.sh` 后上滚 | 只剩 5000 行，最旧已丢弃；不崩溃 |
| R2 | 跑 `gen-long-line.sh` | 不卡死；可横向滚动/换行 |
| R3 | 跑 `gen-ansi-heavy.sh` | 颜色正确；历史不被清空 |
| R4 | 跑 `gen-token-like.sh` | 输出**原样显示**（不打码）；随后校验无落盘 |
| R5 | 跑 `gen-binary.sh` | 可能乱码但不崩溃；之后终端仍可用 |
| R6 | 点「清空」 | 视图与回滚都清；**进程仍在**（`echo hi` 仍响应） |
| R7 | 历史 1200 行时点「清空」 | 弹二次确认 |
| R8 | `Ctrl+L` | 清屏但上滚仍能看到历史（保留回滚） |
| R9 | 点「↻ 重启」 | 新 PTY；旧进程被杀；历史清空 |
| R10 | 跑完夹具后执行 `verify-no-persist.sh` | 退出码 0（PASS） |
| R11 | 上滚后新输出到达 | 不强制回到底部；出现「回到底部」按钮 |
| R12 | 重启应用 | 历史为空（符合预期） |

---

## 8. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 8.1 scrollback 已显式设置
grep -n "scrollback" src/components/system/TerminalPane.vue
# 期望：命中 scrollback: 5000

# 8.2 清空入口存在
grep -n "clear()\|clearSelection" src/components/system/TerminalPane.vue src/stores/useSystemStore.ts
# 期望：命中

# 8.3 后端不落盘：确认读取线程无 fs/audit 写入
sed -n '1958,1985p' src-tauri/src/bridge.rs | grep -n "fs::write\|log_audit"
# 期望：零命中

# 8.4 遗留 termLines 清理（勘误 E6）
grep -rn "termLines" src/ | wc -l
# 期望：0（清理后）

# 8.5 审计只记动作
grep -rn "log_audit" src-tauri/src/bridge.rs | grep -i "term"
# 期望：仅 spawn/kill 类，无 data 类

# 8.6 构建
npm run build && ls -lh dist/assets/*.js
# 对照 logs/baseline-2026-08-27.md 的 505 KB

# 8.7 人工 GUI
# 依次执行 §7 的 R1~R12，并运行 fixtures/terminal/verify-no-persist.sh
```

---

## 9. 失败动作

| 失败 | 动作 |
|---|---|
| 跑 10001 行后崩溃/卡死 | 降低 `scrollback` 至 3000 并复测；不得靠「用户不会跑这么多」搪塞 |
| 发现终端内容落盘 | 视为隐私缺陷，立即回退并定位写入点 |
| 「清空」误杀进程 | 修复按钮语义分离；补 R6 用例 |
| 二进制输出导致终端不可用 | 加 `from_utf8_lossy`（已有）+ 确认 xterm 能处理；仍不行则限制单条事件大小 |
| `termLines` 清理引入回归 | `git revert` 该改动，单独开卡 |

---

## 10. 推荐模型

`AI:BALANCED`（前端改动为主，后端仅可选的审计动作记录）。
**人工 GUI 验收必做**：R1/R4/R5/R10/R12 无法静态替代。
若升级为「后端环形缓冲 + tab 化历史恢复」→ `AI:DEEP`。
