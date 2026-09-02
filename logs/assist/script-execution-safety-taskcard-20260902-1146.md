# script-execution-safety-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §4.1 **M2-1 + M2-2**（需求 #1 脚本库中心 / #4 常用 Linux 命令库）
> 状态：⏸ **任务卡 / 未实现执行器 / 未执行验收 / 不宣称 PASS**
> 关联：`M2-3.a-prework-20260902-1055.md`（ScriptMeta）· `M2-script-library-ui-static-shell-20260902-1146.md` · `M3-terminal-shutdown-taskcard-20260902-1146.md`（**阻塞依赖**）

---

## 1. 目标

定义**脚本执行通道的安全边界**：禁止 `sh -c` 拼接、参数数组传递、工作目录锁定、超时、取消、进程组回收、输出背压、审计，以及对应的反向用例。

🚨 **本卡的每一行都是命令注入防线**：`shell:allow-spawn` 当前是 `args: true`，**ACL 不校验参数**，所有校验必须在 `run_script` 内部完成（**K4**）。

---

## 2. 现状证据（2026-09-02 实测）

### 2.1 shell 能力已开且参数不校验

```json
// src-tauri/capabilities/default.json
{
  "identifier": "shell:allow-spawn",
  "allow": [
    { "name": "bash",       "cmd": "bash",       "args": true },
    { "name": "sh",         "cmd": "sh",         "args": true },
    { "name": "powershell", "cmd": "powershell", "args": true }
  ]
},
"shell:allow-stdin-write",
"shell:allow-kill"
```

- `args: true` = 允许**任意参数数组** → ACL 层**不做任何内容校验**。
- 前端依赖已有 `@tauri-apps/plugin-shell ^2.3.5`。

### 2.2 执行器尚未存在

| 项 | 现状 |
|---|---|
| `run_script` 命令 | ❌ 不在 59 个白名单命令中 |
| `ScriptMeta` 领域模型 | ❌ `src-tauri/src/domain.rs` 中无 |
| 脚本目录 | ❌ 无 `scripts/` 目录 |
| 进程管理能力 | ⚠️ 仅有终端的 `child.kill()`（只杀直接子进程，见 `M3-terminal-shutdown-taskcard` §4.4） |
| 退出收口 | ❌ 无 `ShutdownCoordinator` → **本任务被阻塞** |

### 2.3 现有可复用件

| 件 | 位置 |
|---|---|
| 审计（1000 条上限） | `src-tauri/src/workspace.rs` 的 `log_audit` |
| 两段式安全闸门范式 | `src-tauri/src/bridge.rs:688-790` |
| 路径 canonicalize 范式 | 见 `M2-3.a` §（K7） |
| keyring | `src-tauri/src/keyring_store.rs` |

---

## 3. 必改文件候选

| 文件 | 改动 | 必要性 |
|---|---|---|
| **新增** `src-tauri/src/script_runner.rs` | 执行器：参数数组传递、超时、取消、进程组、背压 | 必须 |
| `src-tauri/src/bridge.rs` | 新增 `run_script` / `cancel_script` / `script_status` | 必须 |
| `src-tauri/src/domain.rs` | 新增 `ScriptMeta` / `ScriptRunResult` | 必须 |
| `src-tauri/permissions/default-commands.toml` | 新增 3 个命令（**K1**） | 必须 |
| `src-tauri/capabilities/default.json` | `shell:allow-spawn` 收窄或移除（改由 Rust 侧 `std::process::Command` 直接起） | 建议 |
| `src-tauri/src/shutdown.rs` | 注册 `ScriptShutdown` | 必须（依赖 TASK-10） |
| **新增** `src-tauri/scripts/*.sh` | 内置脚本（或 `<builtin>` 映射） | 后续 |

---

## 4. 契约 / 数据结构

### 4.1 请求 / 响应

```rust
// src-tauri/src/domain.rs
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ScriptRunRequest {
    pub script_id: String,
    /// 参数按「声明顺序的数组」传递；键名仅用于展示与校验，不进命令行
    pub params: Vec<ScriptParamValue>,
    /// 客户端生成的幂等键（可选，用于重放保护）
    pub request_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ScriptParamValue {
    pub name: String,
    pub value: String,       // bool/enum 也已字符串化；password 走 keyring 引用
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScriptRunResult {
    pub run_id: String,
    pub status: RunStatus,        // success | failed | cancelled | timeout
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub truncated: bool,          // 输出是否被截断
    pub error: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus { Running, Success, Failed, Cancelled, Timeout }
```

```rust
// src-tauri/src/bridge.rs
#[tauri::command]
pub async fn run_script(app: AppHandle, req: ScriptRunRequest) -> Result<String, String>;   // 返回 run_id

#[tauri::command]
pub fn cancel_script(app: AppHandle, run_id: String) -> Result<(), String>;

#[tauri::command]
pub fn script_status(app: AppHandle, run_id: String) -> Result<ScriptRunResult, String>;
```

### 4.2 事件（流式输出）

| 事件 | payload | 说明 |
|---|---|---|
| `script-output` | `{ runId, chunk, seq }` | 分片，单片 ≤ 8 KB；`seq` 单调递增便于前端排序/去重 |
| `script-finished` | `ScriptRunResult` | 终态，含 `status` / `exitCode` / `durationMs` |

### 4.3 命令构造（**核心安全契约**）

```
❌ 禁止：
   Command::new("sh").arg("-c").arg(format!("{} {}", script, user_input))
   Command::new("bash").arg("-c").arg(user_input)

✅ 必须：
   Command::new(interpreter)                    // 来自 ScriptMeta，枚举白名单，非用户输入
       .arg(script_abs_path)                    // canonicalize 后且在允许根内
       .args(ordered_param_values)              // 每个值作为独立 argv 元素
       .current_dir(locked_working_dir)         // 锁定，非用户可控
       .env_clear()                             // 或显式白名单 env
       .stdin(Stdio::null())                    // 默认不交互
       .stdout(Stdio::piped())
       .stderr(Stdio::piped())
```

**要点**：

| 项 | 规则 |
|---|---|
| 解释器 | 仅 `'bash' \| 'sh' \| 'python3' \| 'node'` 四选一，来自 `ScriptMeta.interpreter`，**绝不接受用户输入** |
| 脚本路径 | `canonicalize()` + 必须落在 `allowed_script_roots` 内（内置目录 + `<app_data>/scripts/`），**不得**包含 `..` 逃逸或指向软链接外的路径 |
| 参数 | 每个值一个 `argv` 元素 → shell 不会重新解析 → `;` `|` `$()` 反引号全部变成**字面量** |
| 顺序 | 严格按 `ScriptMeta.params` 声明顺序；**键名不进命令行**（键名也需白名单校验，防 `--flag` 注入） |
| env | `env_clear()` 后显式注入最小集（`PATH`、`HOME`、`LANG`、`TERM`），**不继承**宿主环境（防 `LD_PRELOAD` / `IFS` 类攻击） |
| stdin | 默认 `Stdio::null()`；需要交互的脚本显式声明并走 `script_stdin` 命令 |

### 4.4 参数校验（fail-closed，前后端双做，**后端为准**）

```rust
fn validate_param(p: &ScriptParam, raw: &str) -> Result<String, String> {
    // 1) 类型校验
    match p.type {
        ParamType::Number => raw.parse::<f64>().map_err(|_| "参数必须为数字")?,
        ParamType::Bool   => if raw != "true" && raw != "false" { return Err("布尔值非法".into()); },
        ParamType::Enum   => if !p.options.iter().any(|o| o.value == raw) { return Err("不在可选范围内".into()); },
        _ => {}
    }
    // 2) 正则校验（若声明）
    if let Some(pat) = &p.pattern {
        if Regex::new(pat)?.is_match(raw) == false { return Err("参数格式不合法".into()); }
    }
    // 3) path 类型：canonicalize + 前缀校验（K7）
    if p.type == ParamType::Path {
        let c = std::fs::canonicalize(raw).map_err(|_| "路径不存在")?;
        if !allowed_roots.iter().any(|r| c.starts_with(r)) { return Err("路径不在允许范围内".into()); }
    }
    // 4) 长度上限
    if raw.len() > 4096 { return Err("参数过长".into()); }
    // 5) 拒绝 NUL（argv 注入的经典绕过）
    if raw.contains('\0') { return Err("参数含非法字符".into()); }
    Ok(raw.to_string())
}
```

**fail-closed 原则**：任何校验失败 → **拒绝执行**，不降级、不清洗后放行。

### 4.5 工作目录锁定

| 项 | 规则 |
|---|---|
| 默认 cwd | 脚本文件所在目录（canonicalize 后） |
| 用户可控？ | ❌ **不可控**。`path` 类型参数只作为 argv 传入，**不改变 cwd** |
| 逃逸防护 | cwd 必须在 `allowed_roots` 内 |

### 4.6 超时与取消

```
超时分层：
  soft timeout (默认 60s，ScriptMeta.timeoutSec 可配，上限 600s)
     → 发送 SIGTERM 给进程组
  hard timeout (soft + 5s)
     → 发送 SIGKILL 给进程组
  之后 wait() 回收，标记 status = Timeout
```

| 操作 | 行为 |
|---|---|
| 用户取消 | `cancel_script(run_id)` → SIGTERM → 等 5 s → SIGKILL → `status = Cancelled` |
| 超时 | 同上，但 `status = Timeout`（**与 Cancelled 区分**） |
| 幂等 | 对已终态的 runId 调用 cancel → 返回 `Ok(())`，无副作用 |
| 退出收口 | 应用退出时 `ScriptShutdown` 遍历全部 in-flight run，走同一路径 |

### 4.7 进程组回收

```rust
// 目标语义（伪码，实现时按平台校准）
#[cfg(unix)]
fn spawn_in_new_group(cmd: &mut std::process::Command) -> std::io::Result<Child> {
    use std::os::unix::process::CommandExt;
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();        // 新建会话/进程组，使 killpg 只影响脚本树
            Ok(())
        });
    }
    cmd.spawn()
}

fn kill_group(pid: i32, sig: i32) {
    unsafe { libc::killpg(pid, sig); }   // pid == pgid（setsid 后成立）
}
```

**顺序强制**：`setsid` → 记录 `pgid = child.id()` → 取消/超时时 `killpg(pgid, SIGTERM)` → 5 s → `killpg(pgid, SIGKILL)` → `waitpid` 防僵尸。

**禁止**：只 kill 直接子进程（会留下孤儿孙子进程）。

### 4.8 输出背压

| 项 | 规则 |
|---|---|
| 读取 | 独立线程读 stdout / stderr，**不阻塞**主线程与 UI |
| 缓冲 | 环形缓冲，上限 **5000 行或 4 MB**（先到先算） |
| 背压 | 管道满时暂停读取 + 前端若长时间不消费，丢弃最旧并置 `truncated = true` |
| 事件节流 | 每 30 ms 合并一次 `script-output`（避免高频 emit 刷爆 IPC） |
| 单片上限 | 8 KB |
| 二进制 | `String::from_utf8_lossy` 转换；不做 ANSI 过滤 |

### 4.9 审计

| 事件 | 记什么 | 不记什么 |
|---|---|---|
| `script.run.start` | runId、scriptId、scriptName、**脱敏后**参数（password → `***`）、起始时间 | 密码明文 |
| `script.run.finish` | runId、status、exitCode、durationMs、truncated | 输出正文 |
| `script.run.cancel` | runId、取消者（user/system/timeout） | — |
| `script.validate.reject` | scriptId、参数名、**拒绝原因**（不回显完整值，防日志注入） | 参数完整值 |

```rust
log_audit(&app, "script.run.start", json!({
    "runId": run_id, "scriptId": req.script_id,
    "params": masked_params,     // password 类型一律 "***"
}).to_string());
```

**红线（K3）**：`ScriptMeta` / `ScriptRunRequest` 中**不得**出现可持久化到日志的密码字段；`password` 类型参数应从 keyring 取引用（`keyring://<key>`），绝不以明文跨越前后端。

---

## 5. 实现要点（步骤化）

1. **先完成 `M3-terminal-shutdown-taskcard`（ShutdownCoordinator）**，否则无退出收口。
2. `domain.rs` 加 `ScriptMeta` / `ScriptParam` / `ScriptRunRequest` / `ScriptRunResult` / `RunStatus`。
3. 新建 `script_runner.rs`：进程表 `HashMap<runId, ScriptHandle>`（`Mutex`）+ 参数校验 + `setsid` spawn + 双超时 + 背压读取 + 事件 emit。
4. `bridge.rs` 加 `run_script` / `cancel_script` / `script_status`。
5. `permissions/default-commands.toml` 加 3 个命令（**K1**）。
6. 注册 `ScriptShutdown` 进 coordinator。
7. 前端 `src/stores/useScriptStore.ts` 从 mock 切换到真实通道（**在 `M2-script-library-ui-static-shell` 之后**）。
8. 内置脚本目录 + `ScriptMeta` 登记（可后做）。

---

## 6. 禁止事项

| # | 禁止 | 原因 |
|---|---|---|
| 1 | ❌ `sh -c` / `bash -c` + 字符串拼接 | **命令注入的根本入口** |
| 2 | ❌ 让用户输入决定解释器、cwd、或任意 flag | 绕过 argv 保护 |
| 3 | ❌ 只做前端校验 | 前端可绕过；后端必须重校验（fail-closed） |
| 4 | ❌ 只 kill 直接子进程 | 孤儿孙子进程 |
| 5 | ❌ 无超时 / 无限等待 | 卡死应用 |
| 6 | ❌ 把输出正文写进 `audit.json` | 刷爆 1000 条上限（K5）+ 隐私 |
| 7 | ❌ 密码以明文跨越前后端 / 进日志 / 进历史 | K3 |
| 8 | ❌ 新增命令忘记进 ACL | K1，静默拒绝极难排查 |
| 9 | ❌ 用 `unsafe { killpg }` 而不 `setsid` | 可能误杀宿主进程组 |

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| **高** | 命令注入（历史最高危类别） | 参数数组 + 解释器白名单 + cwd 锁定 + 后端重校验 + 反向用例 R1~R5 |
| 高 | 无 `ShutdownCoordinator` 时实现 → 新增泄漏源 | 依赖 TASK-10，**未完成前不得开工** |
| 中 | `setsid` 后 `pgid` 假设不成立（某些平台） | 用 `libc::getpgid(child.id())` 核实；失败则降级并记录能力缺口（**不得**盲 killpg） |
| 中 | 输出洪水打爆前端 | 背压 + 5000 行/4 MB 上限 + 30 ms 节流 |
| 中 | 长时脚本阻塞应用退出 | hard timeout 5 s + coordinator 全局宽限 3 s |
| 中 | `env_clear()` 导致脚本找不到 `PATH` 下的工具 | 显式注入 `PATH=/usr/local/bin:/usr/bin:/bin`（可按平台配置） |
| 低 | 参数顺序错误导致脚本语义错 | 严格按 `params` 声明顺序；加契约测试 |

---

## 8. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 参数填 `; rm -rf /tmp/xxx` | 作为**字面量**传入；`/tmp/xxx` 未被删除；脚本报参数错误或忽略 |
| R2 | 参数填 `$(id)` / `` `id` `` / `${HOME}` | 不展开，原样传入 |
| R3 | 参数填 `--dangerous-flag` | 键名白名单校验拒绝（或作为值传入而不被解析为 flag） |
| R4 | 参数填 `../../etc/passwd`（path 类型） | canonicalize 后不在允许根内 → 拒绝 |
| R5 | 参数含 NUL 字节 / 超长（>4096） | 拒绝 |
| R6 | 参数绕过前端（直接构造 IPC 调用） | 后端独立拒绝（证明后端校验有效） |
| R7 | 运行 `sleep 300` 后点取消 | `status = Cancelled`，进程**树**全部消失 |
| R8 | 运行 `sleep 300` 且 timeout=5s | 5 s 后 `status = Timeout`，进程消失 |
| R9 | 运行输出 100 MB 的脚本 | 输出被截断（`truncated = true`），应用不 OOM |
| R10 | 运行派生子进程树的脚本后取消 | 全部回收（`ps --forest` 验证） |
| R11 | 应用中止（关窗）时有 in-flight 脚本 | 走 `ScriptShutdown`，不留孤儿 |
| R12 | password 参数运行后查 `audit.json` | 显示 `***`，无明文 |
| R13 | 对已结束的 runId 调 `cancel_script` | 返回 Ok，无副作用 |
| R14 | 高并发跑 10 个脚本 | 互不干扰，runId 唯一 |
| R15 | 脚本文件被外部替换/删除 | 拒绝执行并给出可读错误 |

---

## 9. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 9.1 禁 sh -c 拼接（全仓扫描）
grep -rn '"\-c"' src-tauri/src/ | wc -l                    # 期望 0
grep -rn 'sh -c\|bash -c' src-tauri/src/ | wc -l           # 期望 0

# 9.2 解释器白名单
grep -n "bash\|python3\|node" src-tauri/src/script_runner.rs | head
# 期望：枚举白名单，无用户输入的解释器

# 9.3 参数数组传递
grep -n "\.args(" src-tauri/src/script_runner.rs           # 期望命中
grep -n "\.arg(" src-tauri/src/script_runner.rs | wc -l    # 期望 >= 1（脚本路径）

# 9.4 工作目录锁定
grep -n "current_dir\|env_clear" src-tauri/src/script_runner.rs   # 期望均命中

# 9.5 进程组
grep -n "setsid\|killpg\|getpgid" src-tauri/src/script_runner.rs  # 期望命中

# 9.6 背压与上限
grep -n "5000\|8 \* 1024\|truncated" src-tauri/src/script_runner.rs  # 期望命中

# 9.7 审计脱敏
grep -n '"\*\*\*"' src-tauri/src/script_runner.rs src-tauri/src/bridge.rs  # 期望命中

# 9.8 命令已进 ACL（K1）
grep -c "run_script\|cancel_script\|script_status" src-tauri/permissions/default-commands.toml
# 期望 3

# 9.9 退出收口接入
grep -n "ScriptShutdown" src-tauri/src/shutdown.rs src-tauri/src/main.rs   # 期望命中

# 9.10 动态：孤儿进程检测（人工）
#   a) 通过前端运行一个派生子进程树的脚本
#   b) 点取消
#   c) 宿主机执行：
ps -ef --forest | grep -c "<script-name>"     # 期望 0

# 9.11 编译门槛
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# 对照 logs/baseline-2026-08-27.md（13 warning）
```

---

## 10. 失败动作

| 失败 | 动作 |
|---|---|
| 发现任何 `sh -c` 拼接 | 视为 P0 安全缺陷，立即整改；不得加「此处已过滤」的注释了事 |
| 后端校验缺失（只有前端） | 停止，补后端校验再继续 |
| 取消后仍有孤儿进程 | 补 `setsid` + `killpg`；不得只 kill 直接子进程 |
| `setsid` 不可用 / `pgid` 假设不成立 | 明确记录能力缺口，降级为「kill 直接子进程 + 尽力扫描子进程树」，并在汇总标注已知限制 |
| 输出洪水导致 OOM | 补背压与截断；不得靠「脚本不会输出那么多」搪塞 |
| `audit.json` 被刷爆 / 含明文密码 | 立即修复并清理已有记录（K3/K5） |
| clippy warning 增加 | 对照 baseline 回退 |

---

## 11. 推荐模型

`AI:DEEP`（Rust 进程管理 + 安全校验 + 事件流 + 人工验收）。
方案评审建议 `AI:DEEP-xhigh`（命令注入防线值得先评审）。
**人工 GUI 验收必做**：R1~R5（注入）、R7/R10（进程树回收）、R11（退出收口）。
