# repo-sanity-audit（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 状态：✅ 已执行（全部为真实命令回显，无猜测）

---

## 1. TASK：repo-sanity-audit

核对当前 pwd、branch、最近提交、关键文档存在性、NEXT 来源。
**规则**：关键文档不存在时必须记录实际命令输出摘要，不得猜测、不得创建替代主文档。

---

## 2. 实测输出（原始回显摘要）

### 2.1 pwd

```
/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
```

### 2.2 git status --short --branch

```
## master...origin/master [领先 16]
```

- 分支：`master`
- 上游：`origin/master`
- 本地**领先 16 个 commit**（尚未 push）
- 工作树**无任何修改/未跟踪文件**（`--short` 无条目输出）→ 起点干净

### 2.3 git log --oneline -8

```
b1a0810 docs(free-prework): M2-M5 前置工作汇总（编号映射/5 项现状勘误/阻塞依赖图/强模型最小阅读清单）
e7d4722 docs(free-prework): M5-15.a 插件权限模型草案（PluginManifest/权限分级/fail-closed 判定）
c52adb5 docs(free-prework): M5-13.a 图谱数据模型草案（GraphNode/GraphEdge/SQLite 邻接表/两阶段抽取）
d545362 docs(free-prework): M5-12.a Agent-Skill 契约草案（SkillDef/能力边界/流式回传/记忆层）
15c8ff6 docs(free-prework): M5-7.a A2P/A2A 协议草案（能力清单/stdio 传输/McpGlobalPolicy）
c6e03bd docs(free-prework): M4-58.a 定时任务契约（调度/幂等/失败恢复/审计拆分）
4863942 docs(free-prework): M4-14.a 数据库任务拆解（schema/迁移/回滚/验收/风险）
70e81b3 docs(free-prework): M3-4.b 终端低风险体验草案（历史上限/resize 静默窗口/前端入口）
```

口径结论：最近 8 个 commit 全部是上一批次 `free-model-prework-M2-M5`（20260902-1055）的**纯文档产出**，零代码改动。

### 2.4 ls -la（仓库根，节选）

存在的关键文件/目录：

| 条目 | 类型 | 备注 |
|---|---|---|
| `详细设计与实施计划.md` | 文件 38546 B（8-27 11:39） | ✅ 主文档存在 |
| `后续需求TODO.md` | 文件 32258 B（8-27 14:00） | ✅ 主文档存在 |
| `PROJECT-RULES.md` | 文件 6101 B | 项目规则 |
| `logs/` | 目录（9-02 10:55） | 含 `baseline-2026-08-27.md` + `assist/` |
| `logs/assist/` | 目录（9-02 11:23） | 15 份上一批次文档 |
| `src/` | 目录 | Vue3 + TS 前端 |
| `src-tauri/` | 目录 | Tauri v2 Rust 后端 |
| `tauri-browser-tabs/` | 目录 | 子 crate（子 webview 插件） |
| `variants/`、`prototype.html`、`gen_variants.py` | 原型/变体 | 原型资产 |
| `AI-模型切换与接手清单.md` | ❌ **不存在** | 见 §3 |

### 2.5 find（关键文档存在性探测）

命令：

```bash
find . -maxdepth 3 \( -name "AI-模型切换与接手清单.md" -o -name "详细设计与实施计划.md" \
  -o -name "后续需求TODO.md" -o -name "assist-index-*.md" \)
```

实际输出：

```
./详细设计与实施计划.md
./后续需求TODO.md
```

→ 4 个目标中**命中 2 个**，缺失 2 个。

---

## 3. ❌ 缺失文档清单（**不代为创建**）

| 缺失项 | 探测范围 | 实测结果 | 处置 |
|---|---|---|---|
| `AI-模型切换与接手清单.md` | `maxdepth 3` 全仓 | 零命中；全仓无同名文件 | ❌ 不创建替代物，仅在本文档与索引中标注缺失 |
| `assist-index-*.md` | `maxdepth 3` 全仓 | 零命中 | ✅ 本次按 TASK-2 新建（`docs-index-recovery-20260902-1146.md`，明确标注为**新建索引**，非恢复某份已存在文档） |
| `logs/assist/high-risk-acceptance-matrix-*.md` | `logs/assist/` | 零命中 | ❌ 不创建；本次以 `model-routing-matrix` + `acceptance-script-drafts` 两份**新产物**覆盖其**部分**职能，并在索引中标注「高风险验收矩阵仍缺失」 |
| `logs/assist/low-model-third-batch-summary-*.md` | `logs/assist/` | 零命中 | ❌ 不存在；上一批次汇总为 `free-model-prework-M2-M5-20260902-1055.md`（第二批次口径） |

**重要**：不创建替代主文档来掩盖文件不存在。所有缺失项在汇总与索引中以「❌ 缺失 / 待人工补齐」显式登记。

---

## 4. NEXT 来源核对

| 检查项 | 结果 |
|---|---|
| 仓库内是否存在 NEXT 标记文件 | ❌ 未发现（`find` 与 `ls` 均无 `NEXT*` 条目） |
| `logs/assist/` 内是否存在 NEXT 字段 | ❌ 未发现 |
| 上一批次汇总对 NEXT 的口径 | 「仓库中未发现 NEXT 标记文件/字段，本批次也未创建或修改任何 NEXT 相关产物」 |
| 本批次处置 | ✅ **不移动 NEXT、不创建 NEXT 文件**。NEXT 若存在于外部（对话调度器/外部编排），不在本仓库管辖范围，本批次无从也无权限改动 |

**结论**：NEXT 在本仓库内**无落盘来源**，其口径只能来自外部编排。任何声称「已按 NEXT 推进/推进到某任务」的说法在本仓库内**无法自证**。

---

## 5. 关键文档存在性 → 对 TASK 的影响

| TASK | 依赖文件 | 状态 | 影响 |
|---|---|---|---|
| 2 docs-index-recovery | `logs/assist/*`（15 份） | ✅ 存在 | 可正常分类索引 |
| 3 errata-to-taskcards | 上一批次 §3 勘误 + 实测 | ✅ 证据齐全 | 已实测复核，见下 |
| 4~8 M2 系列 | `后续需求TODO.md` §1/#2/#10、M2-7.b、M2-9.b | ✅ 存在 | 口径可对齐 |
| 9~11 M3 系列 | `bridge.rs`、`main.rs`、`TerminalPane.vue` | ✅ 存在 | 已取行号证据 |
| 12~19 M4/M5 系列 | 上一批次 M4-14.a/M4-58.a/M5-*.a | ✅ 存在 | 可复用，不重复造 |
| 20~21 路由与阅读清单 | 全部上述 | ✅ 存在 | 可产出 |

---

## 6. 🔬 本批次追加实测（为 TASK-3~11 提供证据）

### 6.1 勘误证据采集

| 命令 | 输出要点 |
|---|---|
| `ls -la src-tauri/src/tools` | `ls: 无法访问 'src-tauri/src/tools': 没有那个文件或目录` |
| `find . -name "*.html" -not -path "./node_modules/*"...` | 业务 HTML 仅命中 `index.html`/`prototype*.html`；`*tool*.html` **零命中** |
| `grep -rn "term_resize\|termResize" src src-tauri/src` | 仅 3 处：`src/bridge.ts:208-209`（定义）、`src-tauri/src/main.rs:671`（注册）、`src-tauri/src/bridge.rs:2013`（实现）→ **零调用方** |
| `sed -n '2013,2016p' src-tauri/src/bridge.rs` | `pub fn term_resize(...) { let _ = (app, id, cols, rows); Ok(()) }` → 参数全吞 |
| `grep -n "struct TerminalSession" -A 4 src-tauri/src/bridge.rs` | 仅 `writer` + `child`，**无 `master`** → resize 无法落地 |
| `grep -n "CloseRequested\|process::exit" src-tauri/src/main.rs` | `main.rs:591` 只 `grid_manager.shutdown_all()`；`process::exit` 出现 7 处（107/146/157/248/252/256/468/677 计 8 处） |
| `sed -n '660,679p' src-tauri/src/main.rs` | `.run()` 后仅 `unwrap_or_else` + `process::exit(1)`，**无 `RunEvent` 分支** |
| `cat src-tauri/tauri.conf.json` | `app.withGlobalTauri = true`；`security.csp = null`；`assetProtocol.enable = true` |
| `cat src-tauri/Cargo.toml` | `strip-ansi-escapes = "0.2"` 仍在依赖表；`gtk = "0.18"` / `wry = "0.55"` 仍声明 |
| `grep -rn "strip_ansi_escapes" src-tauri/src` | 零命中 → 死依赖 |

### 6.2 结构事实（供后续任务卡引用）

| 事实 | 值 |
|---|---|
| `default-commands.toml` 命令数 | **59**（含 `term_spawn`/`term_write`/`term_resize`/`term_kill`，无 `list_tools`/`run_script`） |
| `capabilities/default.json` | windows: `main`,`browser`；含 `shell:allow-spawn`（`args: true`）+ `shell:allow-stdin-write` + `shell:allow-kill` |
| `capabilities/browser-remote.json` | webviews: `browser`,`tab-*`,`grid-*`；`remote.urls = ["https://*","http://*"]`；仅 `remote-collect` 权限 |
| `src-tauri/src/` | `bridge.rs`(78029B) / `main.rs`(31419B) / `grid_process.rs` / `sync.rs` / `workspace.rs`(3196B) / `domain.rs`(2370B) / `crashlog.rs` / `grid_ipc.rs` / `keyring_store.rs` |
| `src/components/` | 22 个 `.vue`，分 `browser` / `home` / `layout` / `shared` / `system` / `workspace` 六组 |
| `workspace.rs` 审计上限 | `audit.json` 超过 **1000 条**即 `drain` 丢弃最旧 |
| 前端依赖 | `vue` / `pinia` / `@tauri-apps/api` / `@tauri-apps/plugin-shell` / `@xterm/xterm@^6` / `@xterm/addon-fit@^0.11`；**无路由、无 UI 库、无测试框架** |

---

## 7. 风险

| 级别 | 风险 | 说明 |
|---|---|---|
| 高 | NEXT 无落盘来源 | 外部编排与本仓库口径可能漂移，本批次无法校验 |
| 高 | `AI-模型切换与接手清单.md` 缺失 | 任务编号口径（`M2-1.a` vs WBS `M2-8`）无法官方校准，只能靠描述锚定 |
| 中 | 本地领先 16 commit 未 push | 若换机/换人接手，上游看不到本批次全部前置文档 |
| 中 | `logs/assist/` 15 份文档无索引 | 强模型容易读错/读全（约 240 KB）→ 本次 TASK-2 解决 |

---

## 8. 验收命令（✅ 已执行）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
pwd
git status --short --branch
git log --oneline -8
ls -la
find . -maxdepth 3 \( -name "AI-模型切换与接手清单.md" -o -name "详细设计与实施计划.md" -o -name "后续需求TODO.md" -o -name "assist-index-*.md" \)
git diff --check && echo "diff-check-exit=$?"
```

**结果**：全部执行成功；`git diff --check` 退出码 `0`（PASS）。

---

## 9. 失败动作

| 情况 | 动作 |
|---|---|
| pwd 不在 `/V3/mvp-browser-os-v3` | 立即停止，不做任何写操作 |
| 工作树不干净 | 先 `git status --porcelain` 记录，不自行 stash，交人工 |
| `git diff --check` 非 0 | 定位冲突块，仅报告，不改代码 |
| 主文档缺失 | 记录实测输出（已做 §3），**不创建替代主文档** |

---

## 10. 推荐模型

`AI:FAST`（已完成）。后续若需按 `AI-模型切换与接手清单.md` 校准编号口径 → `AI:BALANCED` + 人工确认。
