# acceptance-script-drafts（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 状态：📝 **只写设计，未创建任何脚本文件，未执行任何脚本**
> 用途：为 M2 / M3 / M4 / M5 生成验收脚本草案清单，供强模型实现时「照单落地」
> 硬门槛对照：`logs/baseline-2026-08-27.md`（clippy 13 warning、主 JS 505 KB）

---

## 1. 通用约定

每个脚本统一说明五项：**检查什么 / 输入 / 预期退出码 / 失败信息 / 是否可 CI 化**。

| 约定 | 值 |
|---|---|
| 位置 | `scripts/`（仓库根，**本批次未创建该目录**） |
| 语言 | Bash（`set -uo pipefail`，**不用** `set -e` 以便收集全部失败后再退出） |
| 退出码 | `0` = 全部通过；`1` = 有失败项；`2` = 用法错误/依赖缺失 |
| 输出 | 每行以 `PASS: ` / `FAIL: ` / `SKIP: ` 开头；末尾输出 `N 项失败` |
| 幂等 | 脚本**只读**或只写临时目录，不得修改仓库文件 |
| CI 化判定 | ✅ 可（无 GUI、无网络、确定性）/ ⚠️ 半可（需固定环境）/ ❌ 不可（需人工 GUI） |

### 1.1 通用失败信息格式

```
FAIL: <检查项> — <实际值> vs <期望值> — <定位提示>
```

例：`FAIL: 离线红线 — src-tauri/src/tools/json-tool.html 含 2 处 https:// — 请内联或移除`

### 1.2 通用前置检查（所有脚本共享 `scripts/_common.sh`）

| 项 | 检查 |
|---|---|
| 工作目录 | 必须为仓库根（含 `src-tauri/Cargo.toml` 与 `package.json`） |
| 工作树 | `git status --porcelain` 是否被脚本自身污染（脚本应零写） |
| 依赖 | `cargo` / `npm` / `sqlite3`（后者可选，缺失则 `SKIP` 相关项） |

---

## 2. M2 验收脚本清单

| # | 脚本 | 检查什么 | 输入 | 预期退出码 | 失败信息 | CI 化 |
|---|---|---|---|---|---|---|
| M2-S1 | `scripts/verify-tools-offline.sh` | 5 个种子工具：存在性、零外链、体积 ≤100 KB/个、合计 ≤300 KB、4 个 `tool-*` meta 齐全、禁 `alert`、无障碍最小项 | 无（默认扫 `src-tauri/src/tools/`） | 0 | `MISSING:` / `TOO-BIG:` / `HAS-EXTERNAL:` / `MISSING-META:` / `HAS-ALERT:` / `NO-LABEL:` | ✅ |
| M2-S2 | `scripts/verify-tools-logic.sh` | 5 个工具的核心逻辑契约（cron 5/6 段与越界、base64 UTF-8 往返、timestamp 0、json 非法输入、regex 灾难性回溯超时） | 无（若引入 `node` 跑纯函数测试则CI 化；否则人工） | 0 | `LOGIC-FAIL: <tool> <case>` | ⚠️（需 `node`） |
| M2-S3 | `scripts/verify-ui-static-shell.sh` | 三份静态壳（图片/脚本库/工具库）红线：零 `invoke`、零 `__TAURI__`、零 `v-html`/`innerHTML`、mock 零外链 | 无 | 0 | `SHELL-VIOLATION: <file>:<line>` | ✅ |
| M2-S4 | `scripts/verify-frontend-build.sh` | `npm run build` 通过 + 主 JS 体积 ≤ baseline 阈值 + 无新增依赖（对比 `package.json` diff） | baseline 路径（默认 `logs/baseline-2026-08-27.md`） | 0 | `BUILD-FAIL` / `SIZE-EXCEEDED: <actual> > <threshold>` / `NEW-DEP: <name>` | ✅ |
| M2-S5 | `scripts/verify-tool-meta-consistency.sh` | `ToolMeta` 字段与 `M2-7.b` 契约一致；`list_tools` 返回的字段覆盖 mock 中全部字段 | `src/types/tool.ts` + 契约文档 | 0 | `FIELD-DRIFT: <field>` | ✅ |

### 2.1 M2-S1 草案（示例，未创建）

```bash
#!/usr/bin/env bash
# scripts/verify-tools-offline.sh（草案，未创建、未执行）
set -uo pipefail
DIR="${1:-src-tauri/src/tools}"
FAIL=0
fail(){ echo "FAIL: $*"; FAIL=$((FAIL+1)); }

# 检查：5 个文件存在
n=$(ls -1 "$DIR"/*.html 2>/dev/null | wc -l)
[ "$n" -eq 5 ] || fail "工具数量 $n != 5"

# 检查：零外链（排除注释行）
while IFS= read -r f; do
  ext=$(grep -o "https\?://" "$f" | wc -l)
  [ "$ext" -eq 0 ] || fail "HAS-EXTERNAL: $f ($ext 处)"
done < <(ls -1 "$DIR"/*.html 2>/dev/null)

# 检查：体积
while IFS= read -r f; do
  sz=$(wc -c < "$f")
  [ "$sz" -le 102400 ] || fail "TOO-BIG: $f ($sz bytes)"
done < <(ls -1 "$DIR"/*.html 2>/dev/null)

# 检查：4 个 meta
for m in tool-name tool-description tool-category tool-version; do
  c=$(grep -l "name=\"$m\"" "$DIR"/*.html 2>/dev/null | wc -l)
  [ "$c" -eq 5 ] || fail "MISSING-META: $m (仅 $c 个文件有)"
done

# 检查：禁 alert/confirm/prompt
c=$(grep -l "alert(\|confirm(\|prompt(" "$DIR"/*.html 2>/dev/null | wc -l)
[ "$c" -eq 0 ] || fail "HAS-ALERT: $c 个文件"

[ "$FAIL" -eq 0 ] && { echo "ALL PASS"; exit 0; }
echo "$FAIL 项失败"; exit 1
```

---

## 3. M3 验收脚本清单

| # | 脚本 | 检查什么 | 输入 | 预期退出码 | 失败信息 | CI 化 |
|---|---|---|---|---|---|---|
| M3-S1 | `scripts/verify-term-resize.sh` | resize 三处：空实现已消失、`TerminalSession` 有 `master`、真实 `.resize(` 调用、参数 `clamp`、前端 `resizeShell` 存在、节流/去重存在 | 无 | 0 | `EMPTY-IMPL` / `NO-MASTER` / `NO-RESIZE-CALL` / `NO-CLAMP` / `NO-FRONTEND-CALL` / `NO-THROTTLE` | ✅ |
| M3-S2 | `scripts/verify-term-history.sh` | `scrollback: 5000` 已设、清空入口存在、读取线程无 `fs::write`/`log_audit`、`termLines` 已清理、审计无 data 类 | 无 | 0 | `NO-SCROLLBACK` / `NO-CLEAR` / `PERSIST-LEAK` / `TERMLINES-LEFT` | ✅ |
| M3-S3 | `scripts/verify-term-no-persist.sh` | 动态：跑完输出夹具后，检查数据目录与 `audit.json` **不含**终端输出内容 | 应用数据目录路径 | 0 | `FAIL: audit.json 含终端输出` / `FAIL: 发现终端历史落盘文件` | ⚠️（需先人工跑终端） |
| M3-S4 | `scripts/verify-shutdown-coordinator.sh` | `shutdown.rs` 存在、`RunEvent` 分支存在、`CloseRequested` 接入 coordinator、`term_kill_all` 已进 ACL、进程组 kill 存在、`process::exit` 数量下降 | 无 | 0 | `NO-COORDINATOR` / `NO-RUNEVENT` / `ACL-MISSING` / `NO-PROCESS-GROUP` / `EXIT-COUNT: n >= 8` | ✅ |
| M3-S5 | `scripts/verify-no-orphan-processes.sh` | 动态：关闭应用后检测孤儿进程（`sleep` / `yes` / 脚本名） | 进程匹配模式 | 0 | `ORPHAN-FOUND: <pid> <cmd>` | ❌（需人工 GUI 操作） |
| M3-S6 | `scripts/verify-clippy-baseline.sh` | `cargo clippy` warning 数不高于 baseline（13） | baseline 数值（默认 13） | 0 | `CLIPPY-REGRESSION: n > 13` | ✅ |

---

## 4. M4 验收脚本清单

| # | 脚本 | 检查什么 | 输入 | 预期退出码 | 失败信息 | CI 化 |
|---|---|---|---|---|---|---|
| M4-S1 | `scripts/verify-script-safety.sh` | 命令注入防线：全仓零 `sh -c`/`bash -c`、零 `"-c"`、解释器白名单、`.args(` 数组传递、`current_dir` + `env_clear`、参数校验函数存在、`setsid`/`killpg` 存在、输出上限与 `truncated`、审计脱敏 `***` | 无 | 0 | `SH-C-FOUND` / `NO-INTERPRETER-WHITELIST` / `NO-ARGS-ARRAY` / `NO-CWD-LOCK` / `NO-PROCESS-GROUP` / `NO-BACKPRESSURE` / `NO-MASK` | ✅ |
| M4-S2 | `scripts/verify-script-injection.sh` | 动态：用 R1~R5 的注入串调用 `run_script`，验证被当作字面量/被拒 | 脚本 id + 参数 | 0 | `INJECTION-SUCCEEDED: <payload>` | ⚠️（需真实执行通道） |
| M4-S3 | `scripts/verify-scheduler.sh` | 幂等键存在且唯一、`CatchUp` 上限 ≤3、原子写（`rename`/`.tmp`）、损坏备份（`.corrupt`）、`tick` 内无 `log_audit`、`SchedulerShutdown` 注册 | 无 | 0 | `NO-IDEMPOTENCY` / `NO-CATCHUP-LIMIT` / `NOT-ATOMIC` / `SILENT-WIPE` / `AUDIT-FLOOD` / `NO-SHUTDOWN` | ✅ |
| M4-S4 | `scripts/verify-scheduler-misfire.sh` | 动态：三种 misfire 策略（Skip/FireOnce/CatchUp）的实际触发次数 | 任务 id + 期望次数 | 0 | `MISFIRE-MISMATCH: 实际 n != 期望 m` | ❌（需人工控制时间） |
| M4-S5 | `scripts/db-verify.sh` | 库完整性、迁移版本连续、无明文凭据列、Artifact 条目对账、`idempotency_key` UNIQUE 约束存在 | db 路径 + workspace 目录 | 0 | `integrity_check 非 ok` / `版本不连续` / `发现疑似凭据列` / `条目数不一致` / `缺少幂等唯一约束` | ✅ |
| M4-S6 | `scripts/db-migrate-dryrun.sh` | 在**临时副本**上跑一遍迁移，验证不破坏、可完成 | 源 db 路径 | 0 | `MIGRATE-FAIL: <detail>` | ✅ |
| M4-S7 | `scripts/db-backup-check.sh` | 备份存在性、`integrity_check`、保留数量（≤10 份 + 7 天） | backups 目录 | 0 | `NO-BACKUP` / `BACKUP-CORRUPT` / `RETENTION-EXCEEDED` | ✅ |
| M4-S8 | `scripts/db-rollback-drill.sh` | 从备份恢复到**临时位置**并校验（演练回滚） | 备份文件路径 | 0 | `RESTORE-FAIL` / `VERIFY-FAIL` | ✅ |
| M4-S9 | `scripts/db-audit-scan.sh` | 全库扫描疑似凭据明文（password/secret/token 列名或值模式） | db 路径 | 0 | `CREDENTIAL-HIT: <table>.<col>` | ✅ |

---

## 5. M5 验收脚本清单

| # | 脚本 | 检查什么 | 输入 | 预期退出码 | 失败信息 | CI 化 |
|---|---|---|---|---|---|---|
| M5-S1 | `scripts/verify-a2a-protocol.sh` | id 必须字符串、`params` 必须 object、单行 ≤1 MB、能力 fail-closed、超时与硬超时、进程组、审计独立文件、`AgentShutdown` 注册 | 无 | 0 | `NO-ID-VALIDATION` / `NO-PARAMS-VALIDATION` / `NO-LINE-LIMIT` / `NO-FAIL-CLOSED` / `NO-TIMEOUT` / `NO-PROCESS-GROUP` / `AUDIT-FLOOD` / `NO-SHUTDOWN` | ✅ |
| M5-S2 | `scripts/verify-a2a-capability.sh` | 动态：未授予能力调用被拒；scope 不匹配被拒；Agent 崩溃有 `TRANSPORT_CLOSED` | agentId + 能力名 | 0 | `CAPABILITY-LEAK: <cap> 被放行` | ⚠️ |
| M5-S3 | `scripts/verify-skill-contract.sh` | 无 `SkillImpl::Inline`、只走 `run_script`、能力白名单**共用** `capability.rs`、安装后不自动启用、密码脱敏、原子写、命令进 ACL、`SkillShutdown` | 无 | 0 | `INLINE-CODE-FOUND` / `NO-RUNSCRIPT` / `CAPABILITY-DUPLICATED` / `AUTO-ENABLED` / `NO-MASK` / `NOT-ATOMIC` / `ACL-MISSING` | ✅ |
| M5-S4 | `scripts/verify-graph-model.sh` | 稳定 id（非 `Uuid::new_v4`）、`depth ≤ 2` 硬约束、查询层隐私过滤、`filtered_by_privacy` 字段、容量上限、幂等 upsert + FK 级联、查询不写审计、`GraphShutdown` | 无 | 0 | `UNSTABLE-ID` / `NO-DEPTH-LIMIT` / `NO-PRIVACY-FILTER` / `NO-CAPACITY-LIMIT` / `NO-CASCADE` / `AUDIT-FLOOD` | ✅ |
| M5-S5 | `scripts/verify-graph-privacy.sh` | 动态：以不同 `max_privacy` 查询，验证 `restricted` 不泄漏且计数正确；导出不含 private/restricted | db 路径 | 0 | `PRIVACY-LEAK: <node>` / `FILTER-COUNT-MISMATCH` / `EXPORT-LEAK` | ✅ |
| M5-S6 | `scripts/verify-plugin-runtime.sh` | zip-slip 防护（路径校验 + symlink 拒绝）、体积上限、`setsid`/`killpg`、`env_clear` + cwd 锁定、运行时不持 `AppHandle`、安装后不启用、原子写、审计独立文件、命令进 ACL、`PluginShutdown` | 无 | 0 | `NO-ZIPSLIP-GUARD` / `NO-SIZE-LIMIT` / `NO-PROCESS-GROUP` / `NO-ENV-CLEAR` / `HAS-APPHANDLE` / `AUTO-ENABLED` / `NOT-ATOMIC` / `AUDIT-FLOOD` / `ACL-MISSING` | ✅ |
| M5-S7 | `scripts/verify-plugin-zipslip.sh` | 动态：构造含 `../` 与 symlink 条目的恶意包，验证安装被拒且**无文件被写出** | 恶意包路径 | 0 | `ZIPSLIP-SUCCEEDED: <path>` | ✅ |
| M5-S8 | `scripts/verify-no-orphan-plugins.sh` | 动态：卸载/退出后插件进程树全部回收 | 进程匹配模式 | 0 | `ORPHAN-FOUND: <pid> <cmd>` | ❌（需人工 GUI） |

---

## 6. 跨里程碑通用脚本

| # | 脚本 | 检查什么 | 输入 | 预期退出码 | 失败信息 | CI 化 |
|---|---|---|---|---|---|---|
| X-S1 | `scripts/verify-acl-registry.sh` | **K1**：`bridge.rs` 中全部 `#[tauri::command]` 函数名 ⊆ `default-commands.toml` 白名单 | 无 | 0 | `ACL-MISSING: <cmd>`（逐个列出） | ✅ |
| X-S2 | `scripts/verify-no-vhtml.sh` | **K8**：`src/` 下零 `v-html` / `innerHTML` | 无 | 0 | `VHTML-FOUND: <file>:<line>` | ✅ |
| X-S3 | `scripts/verify-audit-cap.sh` | **K5**：`audit.json` 条目 ≤ 1000；高频行为有独立明细文件 | 应用数据目录 | 0 | `AUDIT-OVERFLOW: n > 1000` / `NO-DEDICATED-FILE` | ⚠️（需数据目录） |
| X-S4 | `scripts/verify-no-credential-leak.sh` | **K3**：`domain.rs` / 审计 / 历史中零密码字段或明文 | 无 | 0 | `CREDENTIAL-FIELD: <file>:<line>` | ✅ |
| X-S5 | `scripts/verify-atomic-write.sh` | **K6**：所有落盘点用 `atomic_write`/`rename`；损坏有 `.corrupt` 备份逻辑 | 无 | 0 | `NON-ATOMIC-WRITE: <file>:<line>` / `SILENT-WIPE: <file>` | ✅ |
| X-S6 | `scripts/verify-baseline.sh` | 全量门槛：clippy warning 数、主 JS 体积、包体积（若有基线） | baseline 路径 | 0 | `REGRESSION: <metric> <actual> > <baseline>` | ✅ |
| X-S7 | `scripts/verify-worktree-clean.sh` | 收尾：`git status --porcelain` 为空 + `git diff --check` 退出 0 | 无 | 0 | `DIRTY: <file>` / `WHITESPACE-ERROR` | ✅ |

### 6.1 X-S1 草案（示例，未创建）

```bash
#!/usr/bin/env bash
# scripts/verify-acl-registry.sh（草案，未创建、未执行）
# 检查什么：bridge.rs 里所有 #[tauri::command] 函数都已登记进 default-commands.toml
# 输入：无
# 预期退出码：0
# 失败信息：ACL-MISSING: <cmd>
# 是否可 CI 化：✅
set -uo pipefail
FAIL=0
grep -A 1 '^#\[tauri::command\]' src-tauri/src/bridge.rs \
  | grep -oP 'pub (async )?fn \K\w+' \
  | while read -r cmd; do
      grep -q "\"$cmd\"" src-tauri/permissions/default-commands.toml \
        || { echo "FAIL: ACL-MISSING: $cmd"; exit 1; }
    done
```

---

## 7. 汇总统计

| 里程碑 | 脚本数 | 全 CI 化 | 半 CI 化 | 需人工 |
|---|---|---|---|---|
| M2 | 5 | 4 | 1 | 0 |
| M3 | 6 | 4 | 1 | 1 |
| M4 | 9 | 7 | 1 | 1 |
| M5 | 8 | 6 | 1 | 1 |
| 通用 X | 7 | 6 | 1 | 0 |
| **合计** | **35** | **27** | **5** | **3** |

> **关键结论**：35 个脚本中 **27 个可完全 CI 化**（约 77%），但**涉及进程回收、GUI 交互、时间控制的 8 个必须人工参与**。
> 因此：**CI 全绿 ≠ 任务可 PASS**，人工验收项必须在验收记录中单独留证。

---

## 8. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| 中 | 脚本本身成为「走过场」（全绿但没检查到点上） | 每个脚本的**失败路径**必须先用故意破坏的样本验证一次（"mutation test"） |
| 中 | `sqlite3` CLI 在目标环境不存在 | 脚本先检测，缺失则 `SKIP` 相关项并输出提示（不算 PASS） |
| 中 | 体积/性能类检查在不同机器上不稳定 | 固定阈值来自 `logs/baseline-2026-08-27.md`；性能脚本标 ⚠️ 不进 CI 门禁 |
| 中 | 脚本数量多，维护成本 | 先实现 X 系列通用脚本（7 个，收益最大），里程碑脚本随实现进度按需补 |
| 低 | 脚本误报导致阻塞 | 失败信息必须含定位提示（文件:行），便于快速判断是真失败还是误报 |

**落地建议顺序**：

1. **X-S1 / X-S2 / X-S6 / X-S7**（4 个，覆盖 K1 / K8 / 基线 / 工作树，收益最高、实现最简单）
2. **M2-S1 / M2-S3 / M2-S4**（M2 静态检查）
3. **M3-S1 / M3-S2 / M3-S4 / M3-S6**（终端三卡 + clippy）
4. **M4-S1 / M4-S3 / M4-S5**（安全 + 调度 + 数据库）
5. **M5-S1 / M5-S3 / M5-S4 / M5-S6**（四张契约卡）
6. 其余按需

---

## 9. 验收命令（**本批次仅产出文档，未创建/未执行任何脚本**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 9.1 确认本批次未创建任何脚本（应为 0）
ls scripts 2>&1 | head -1                       # 期望：No such file or directory
find . -maxdepth 2 -name "verify-*.sh" -not -path "./node_modules/*" | wc -l   # 期望 0

# 9.2 确认未执行任何验收（无产物日志）
ls -1 logs/assist/*-result*.txt 2>/dev/null | wc -l   # 期望 0

# 9.3 未来实现后的自检（示意）
# bash scripts/verify-acl-registry.sh
# bash scripts/verify-no-vhtml.sh
# bash scripts/verify-clippy-baseline.sh
```

---

## 10. 失败动作

| 失败 | 动作 |
|---|---|
| 某脚本实现后自己跑不通（非目标场景失败） | 修脚本，不得放宽检查项 |
| 脚本全绿但功能实际有问题 | 说明检查项覆盖不足 → 补检查项 + 做一次「故意破坏」验证 |
| 依赖缺失导致 `SKIP` | `SKIP` **不算 PASS**；补齐依赖后重跑 |
| 阈值需调整 | 必须更新 `logs/baseline-*.md` 并注明原因，**不得**直接改脚本里的数字 |

---

## 11. 推荐模型

- 通用 X 系列（7 个）：`AI:FAST` 可实现
- 里程碑静态检查（`grep` 类）：`AI:BALANCED`
- 动态/端到端脚本（M3-S5 / M4-S2 / M5-S2 / M5-S7）：`AI:DEEP`
- **人工必做**：M3-S5、M4-S4、M5-S8（进程回收 / 时间控制 / GUI）
