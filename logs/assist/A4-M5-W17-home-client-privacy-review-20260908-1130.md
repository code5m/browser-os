# A4 — M5-W17 主页/客户端启动 隐私与安全静态审查（REVIEW + 新建策略夹具）

- **Lane / 分派**：A4，`M5-W17 Desktop Client Completeness and Home Recovery Dispatch`（board §1164-1212，A4 行 1199：*START REVIEW*）
- **A4 任务原文**：`scripts/check-home-client-policy.py` (new), `logs/assist/`, `logs/checkpoints/` → *Static privacy/security review for W17 UI/startup changes; guard against **credentials, raw sensitive URLs/query values, shell injection, and privilege expansion**. No product UI edits.*
- **WORKDIR / 身份**：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`，`.workspace-identity = BACKV3_MAIN`，分支 `master`
- **基线 / HEAD**：`052b18a`（origin/master，W15 PUSHED·accepted）；`git fetch && git pull --ff-only` = 已最新
- **集成顺序**：`A2 -> A3 -> A5 -> A6 -> A7 -> A4/A9 -> A8/A10/A11 -> A0`（A4 在 A2/A3/A5/A6/A7 之后）
- **形态**：REVIEW + **新建策略夹具脚本**（board 明确允许 `scripts/check-home-client-policy.py`）。**零产品 UI 编辑**（未改 `src/**` 任何文件）
- **未 push**（board：Only A0 pushes）

---

## 0. 交付物

| 文件 | 内容 |
|---|---|
| `scripts/check-home-client-policy.py` | **新建**静态守门夹具：3 ACTIVE + 3 PENDING 码位，`--self-test` / 默认 / `--expect-pending` 三模式 |
| `logs/assist/A4-M5-W17-home-client-privacy-review-20260908-1130.md` | 本审查 note |
| `logs/checkpoints/A4-M5-W17-home-client-privacy-review-20260908-1130.md` | checkpoint |
| `logs/checkpoints/Lane-A4-M5-W17-home-client-privacy-review-20260908-1130.patch` | binary patch |

---

## 1. 夹具设计（`scripts/check-home-client-policy.py`）

沿用 A4 既有夹具约定（`check-plugin-ui-privacy.py` / `check-mcp-policy.py` 同风格），判定对象 **gated**（文件不存在即 no-op，不阻塞批次）：

- `src/stores/useHomeStore.ts`（A3 状态契约）
- `src/components/home/**`（A5 主页表面）
- `run-gui.sh`（A2 启动助手）
- `src-tauri/permissions/default-commands.toml`、`src-tauri/src/main.rs`（特权扩张守门）

### ACTIVE 码位（W17 永久红线，出现即 FAIL）

| 码位 | 守门 |
|---|---|
| `HOME_NO_PRIVILEGE_EXPANSION` | 禁止借 W17 新增 Tauri 命令 / ACL 条目 / `shell:`·`fs:`·`http:`·`network:` 权限 / 进程派生（`Command::new`）/ 监听（`TcpListener`） |
| `HOME_NO_SHELL_INJECTION` | 禁止把主页 `target`/`cmd` 未校验地**拼接**进命令执行（定位到拼接表达式，非裸分号） |
| `HOME_STARTUP_SAFE` | 启动助手禁 `sudo`/`pkexec`、`chmod 777`、`curl|bash`、`eval $`、无差别 `killall/pkill node|vite|npm` |

### PENDING 码位（既有 home 代码债务，A3/A5 接线后应转 ACTIVE）

| 码位 | 守门 |
|---|---|
| `HOME_NO_SECRET_PERSIST` | 浏览器存储不得持久化含凭据 / 敏感 query / 本地绝对路径 / app exec 命令体的 `target` |
| `HOME_NO_RAW_ERROR_ECHO` | 错误渲染只用稳定码，禁 `e?.message` / `String(e)` / `.toString()`（W16 P2） |
| `HOME_NO_SENSITIVE_TARGET_RENDER` | 渲染面（含 `:title` 属性）禁原样暴露 `target`（W17 共享验收 #4） |

### 实测三模式

```bash
python3 scripts/check-home-client-policy.py --self-test
# HOME_CLIENT_POLICY_SELF_TEST_RESULT=PASS: 1 好样本零违规 + 6 个坏样本全部检出（含变异防呆）；ACTIVE=3 PENDING=3

python3 scripts/check-home-client-policy.py
# home client policy: all invariants hold（ACTIVE=3）        ← 默认门禁 PASS，不阻塞批次

python3 scripts/check-home-client-policy.py --expect-pending
# HOME_CLIENT_POLICY_PENDING_RESULT=FAIL
#   HOME_NO_SECRET_PERSIST:src/stores/useHomeStore.ts:71（app exec 命令体落浏览器存储）
#   HOME_NO_RAW_ERROR_ECHO:src/stores/useHomeStore.ts:180
#   HOME_NO_SENSITIVE_TARGET_RENDER:src/components/home/HomePanel.vue:48
```

自检工程中修掉的两个自身缺陷（诚实记录）：
1. 初版 shell 注入检测用"元字符 + 邻近关键词"窗口匹配 → TS 正常语句到处是 `;`，好样本误报；改为**只匹配拼接表达式**（`target + "...&&..."` / 模板串 `${target}` / `exec(...)` 实参）。
2. 初版盘符正则 `[A-Za-z]:[\\/]` 把 URL `https://` 的 `s:/` 误判为 `C:\` → 判定本地路径前先做 `_scrub_schemes()` 抹掉 scheme。

---

## 2. 对前序 lane 产物的审查结论

### 2.1 A3 `src/utils/homeUi.ts` —— **正面，已实现零敏感披露** ✅

A3 的纯逻辑层直接兑现了 W17 共享验收 #4 与 W16 横切契约 P2/P3/P6：

| A3 实现 | 对应红线 |
|---|---|
| `looksSensitive()`（URL userinfo / `?token=`·`?password=` / `ghp_`·`github_pat_`·`glpat-` / `Bearer`·`Basic` / `sk-`·`AKIA`） | 凭据识别 |
| `homeDisplayTarget()`：URL 只给 `scheme+host+path`（剥 userinfo/query/hash）；dir 只给路径末段；app 只给命令首段 | 无敏感 URL/query/本地路径披露 |
| `homeAccessibleLabel()`：类型 + 名称，不含完整路径/URL/凭据 | 无障碍名安全 |
| `panelStateHome()`：错误态只给 `HOME_ERROR_FALLBACK` 固定文案，**不回显原始 error** | W16 P2 稳定错误 |
| `HOME_MAX_SHORTCUTS=24` / `HOME_MAX_RECENTS=12` / 各字段长度上限 + `normalizeShortcuts` 逐字段校验 | 有界 + 迁移安全 |

→ **A3 的工具函数本身就是 A4 建议形态**；问题不在工具，在**接线**（见 §2.2）。

### 2.2 三项具体未接线（PENDING FAIL，给 A3/A5 的精确整改清单）

| # | 位置 | 现状 | 建议整改 |
|---|---|---|---|
| F-1 | `src/stores/useHomeStore.ts:71` | `save()` 把含 `type:"app"` 的 `target`（exec 命令体）与 dir 绝对路径**全量**写入 `localStorage` | 落盘前用 `homeDisplayTarget()` 或新增 `persistableTarget()` 只存安全投影；或仅存 `id/type/name/icon`，target 走后端/会话态 |
| F-2 | `src/stores/useHomeStore.ts:180` | `layout.showToast("启动失败: " + (e?.message ?? e))` —— **原始错误回显**（W16 P2 红线） | 改用 `panelStateHome({error}).message` / `HOME_ERROR_FALLBACK`，或按稳定码映射文案，**丢弃 `e`** |
| F-3 | `src/components/home/HomePanel.vue:48` | `:title="s.target"` —— 鼠标悬停即暴露 target 原文（绝对路径 / exec / 含 token 的 URL） | 改用 `:title="homeDisplayTarget(s)"`，无障碍名用 `homeAccessibleLabel(s)` |

→ 三项全部**已有现成修复工具**（`homeUi.ts` 已导出对应函数），属"接线"而非"新设计"。建议 A3/A5 在 W17 收口时转 ACTIVE。

#### 2.2.1 实时复核（A3/A5 并发修复后，同一工作会话内）

本夹具产出整改清单后，A3/A5 在**同一工作会话内**已实时修掉其中两项：

| ID | 初检 | 实时复核 |
|---|---|---|
| F-2 `useHomeStore.ts` raw error 回显 | FAIL | **已修**（`--expect-pending` 不再命中）✅ |
| F-3 `HomePanel.vue:48` `:title="s.target"` | FAIL | **已修** ✅ |
| F-1 `useHomeStore.ts` app exec 命令体落 localStorage | FAIL（:71） | **仍存在**（行号随 A3 编辑漂移到 :72）⚠️ |

→ 这印证了 `--expect-pending` 作为"给实现 lane 的整改清单"的定位是有效的：**默认门禁不阻塞，整改清单被实时消费**。

> **行号漂移说明**：本 note 中的行号是检出的**时刻值**；A3/A5 仍在并发编辑这些文件，最终行号以 A0 集成前重跑 `python3 scripts/check-home-client-policy.py --expect-pending` 的实测输出为准。

### 2.3 A2 `run-gui.sh` —— **安全面 PASS，功能面未满足验收 #1** ⚠️

- **安全（A4 守门范围）PASS**：无 `sudo`/`chmod 777`/`curl|bash`/`eval`/无差别 `pkill`；仅 `cargo build` + 导出 `WEBKIT_DISABLE_DMABUF_RENDERER=1` / `GDK_BACKEND=x11` + `exec "$BIN"`。→ `HOME_STARTUP_SAFE` 零命中。✅
- **功能缺口**：W17 共享验收 #1 要求 *"the documented helper detects/starts the Vite dev server, waits for it, launches the desktop app, and cleans up only the server it owns on exit"*。当前脚本**不启动也不等待 Vite**。而 `src-tauri/src/main.rs:1210` 在 `cfg!(debug_assertions)` 且未设 `MVP_FORCE_DIST` 时强制 `WebviewUrl::External("http://localhost:1421")` → **debug 二进制依赖 Vite 在 1421**；Vite 未起正是 acceptance #1 要消除的 `connection refused`。
- **建议（给 A2，非 A4 修改）**：
  1. 增加 Vite 探测（`nc -z 127.0.0.1 1421`）→ 未起则 `npm run dev &` 并记录 `SERVER_PID`；
  2. `trap cleanup EXIT` 中**只 kill 本脚本自己起的 PID**（ownership-safe），不得 `pkill/killall node|vite|npm`；
  3. 可选：为本夹具新增 `HOME_DEV_SERVER_OWNED` PENDING 码位（断言：若脚本启动 dev server，必须存在 PID 捕获 + trap 清理 + 无无差别 kill）。**该码位尚未加入，留给 A2 与本 lane 协商后补**（A4 不在本轮擅自扩大 ACTIVE 面）。

### 2.4 A5 主页组件 —— **初检空文件，已自愈** ✅

- 初检：`src/components/home/HomeLaunchers.vue` = **0 byte**，违反 board §5 规则 5（*Do not create empty files…*）。
- 实时复核：A5 已补写为 **4587 bytes**，并新增 `HomeShortcuts.vue`（7673 B）、`HomeShortcutEditor.vue`（5372 B）。
- 新组件安全渲染抽查：`grep -nE "s\.target|:title|e\.message"` 仅命中 `HomeShortcuts.vue:29` 的**类型校验**（`typeof s?.target === "string" ? s.target : ""`），非渲染面 → **无 target 原文/hover 暴露** ✅。

→ 空文件问题已由 A5 自愈，无需 A0 介入。

### 2.5 尚未落地的 W17 产物

- `scripts/check-home-store-logic.mjs`（A3 须交付）
- `scripts/check-home-ui-logic.mjs`（A9 须交付）

→ 本夹具对二者为 gated no-op，不阻塞。

---

## 3. 与 W16 横切契约（P1–P6）的承接

| W16 不变式 | W17 兑现情况 |
|---|---|
| P1 Redacted DTO at boundary | home 数据无跨进程 DTO；`homeDisplayTarget()` 等价于前端侧红脱敏投影 ✅（待接线） |
| P2 Stable error_code only | `homeUi.panelStateHome()` 已实现；`useHomeStore:180` **未接线** ❌（F-2） |
| P3 No browser persistence of secrets | `homeUi` 未解决落盘口径；`useHomeStore:71` **违例** ❌（F-1） |
| P4 No raw Tauri invoke | W17 未新增 invoke；home 仍走既有 `bridge` ✅ |
| P5 Audit redaction | 主页无审计写入 ✅ |
| P6 No sensitive rendering/persistence | `homeDisplayTarget` 已实现；`HomePanel:48` **未接线** ❌（F-3） |

---

## 4. W17 Hard Stops 对齐

| Hard Stop | A4 评估 |
|---|---|
| 不新增命令执行 / 插件调用 / 动态加载 / 远程下载监听 / 守护 / 模型调用 / Agent·Skill 执行 / MCP 运行时 / 图写导出 / 后台 worker | `HOME_NO_PRIVILEGE_EXPANSION` 实测零命中（ACL 无权限声明、main.rs 无进程派生/监听）✅ |
| No new dependency / No new Tauri command / bridge / ACL / fs permission / network privilege | 同上；A4 未改任何权限文件 ✅ |
| 共享验收 #4：无敏感 URL/query/凭据/本地路径披露 | `homeUi` 已提供工具；3 处未接线（F-1..F-3）⚠️ |
| 键盘可达 + 无障碍名 | `homeAccessibleLabel` / `areaAccessibleLabel` 已提供 ✅（A9 脚本待验） |
| 不规避 build-metric guard | A4 零前端改动，metrics 不受影响（见 §6） |
| 交付完整批次：checkpoint + 命令结果 + `git diff --check` + binary patch | 见 §6/§7 ✅ |
| 不 commit、不 push | ✅ |

---

## 5. 风险与诚实残留

1. **PENDING 未闭环**：F-1..F-3 三项在 A3/A5 接线前保持 PENDING；本夹具 `--expect-pending` 会 FAIL，**这是有意的整改清单**，默认门禁（ACTIVE）仍 PASS，不阻塞 A0 集成。
2. **A2 验收 #1 功能缺口**：夹具只守"启动脚本安全形态"，**不**守"是否启动/等待 Vite"。功能缺口需 A2 自补 + A8 手动 QA（真实客户端）确认；A4 不伪造验证。
3. **本夹具未接入 `scripts/pre-merge.sh`**：board 中 pre-merge 仅 A9"if integration is required"；A4 擅自改 pre-merge 会与其他 lane 冲突。**建议 A0/A11 在集成时接入默认模式**（ACTIVE 门禁，不阻塞）。
4. **A5 空文件**：`HomeLaunchers.vue` 0 byte，需 A5 处理。
5. **A4 未跑 `npm run build` / 未做 GUI 验证**：A4 零产品代码改动，metrics 不适用；GUI 验收归 A8。

---

## 6. 自检证据链

| 检查 | 命令 | 结果 |
|---|---|---|
| 启动门禁 | `pwd` / `git branch` / `git pull --ff-only` | BACKV3_MAIN / master / 已最新（`052b18a`）✅ |
| 夹具自检 | `python3 scripts/check-home-client-policy.py --self-test` | PASS（好样本零违规 + 6 坏样本全检出，ACTIVE=3 PENDING=3）✅ |
| 默认门禁 | `python3 scripts/check-home-client-policy.py` | PASS（`all invariants hold（ACTIVE=3）`）✅ |
| 整改清单 | `python3 scripts/check-home-client-policy.py --expect-pending` | FAIL（3 项，见 §2.2）⚠️（设计如此） |
| 语法 | `python3 -m py_compile scripts/check-home-client-policy.py` | OK ✅ |
| 零产品 UI 编辑 | `git status --short -- src` | 仅 A3/A5 自身改动；**A4 未触碰** ✅ |
| `git diff --check` | — | 见 §7 patch 生成前执行 |

---

## 7. LANE 输出模板（W17 格式）

```text
LANE: A4
STATUS: PASS_WITH_DEBT
SCOPE: scripts/check-home-client-policy.py (new), logs/assist/A4-M5-W17-home-client-privacy-review-20260908-1130.md, logs/checkpoints/A4-M5-W17-home-client-privacy-review-20260908-1130.md
DELIVERED: 主页/客户端启动静态隐私安全守门夹具（3 ACTIVE + 3 PENDING，三模式）；对 A2 run-gui.sh / A3 homeUi.ts+store / A5 HomePanel+HomeLaunchers 的静态审查结论；A3/A5 三项精确整改清单（F-1/F-2/F-3）
VERIFY: python3 scripts/check-home-client-policy.py --self-test: PASS(ACTIVE=3 PENDING=3, 6 bad samples detected); python3 scripts/check-home-client-policy.py: PASS(all invariants hold ACTIVE=3); python3 scripts/check-home-client-policy.py --expect-pending: FAIL(3 hits: useHomeStore.ts:71 / useHomeStore.ts:180 / HomePanel.vue:48); python3 -m py_compile: OK
METRICS: N/A（A4 零产品代码/零前端改动，不触碰 build metrics；未跑 npm run build）
PATCH: /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/logs/checkpoints/Lane-A4-M5-W17-home-client-privacy-review-20260908-1130.patch
RISKS: (1) F-1..F-3 待 A3/A5 接线后转 ACTIVE；(2) A2 run-gui.sh 未启动/等待 Vite，debug 构建依赖 1421（main.rs:1210），acceptance #1 待 A2 补 + A8 实机确认；(3) 夹具未接 pre-merge（待 A0/A11 决定）；(4) A5 HomeLaunchers.vue 为 0 byte 空文件（board §5 规则 5 违规）
NO_PUSH: confirmed
```

**A4 Lane W17 结论**：W17 主页/启动面的**隐私与安全静态守门已落地**（新建夹具，默认门禁绿、不阻塞集成）；A3 `homeUi.ts` 的零敏感披露实现**质量良好**；遗留 3 项接线债务（F-1..F-3）已定位到行号并给出即改方案。A4 **不阻塞** W17 集成。

---

## 8. Acceptance Closeout 复核（board §1203-1229，A4 行 1212）

> A4 closeout 任务：*Re-check the A3 fix and W17 privacy boundary; default policy must pass, and report any intentionally pending debt.*

### 8.1 A3 修复 —— HOME_NO_SECRET_PERSIST **已闭环** ✅

- `homeUi.ts:216-246` 新增 `isStorageSafe`（`type !== "app"`）/ `toPersisted` / `persistedJson` / `HOME_PERSISTED_TYPES=["url","dir"]`。
- 语义：**app 条目仅驻留内存（会话内），一律不写浏览器存储**；`load()` 迁移期把旧数据里的 app 条目剔除并**随即抹除**。比 A4 初检建议的"存安全投影"更强。
- 接线：`useHomeStore.ts:110/:115` 走 `toPersisted`；`:91` 迁移清理走 `filter(isStorageSafe)`；`:104` recents 同口径。
- 回归断言（A3 `check-home-store-logic.mjs`，374 行）：`persistedJson 不含命令体/token 参数`、`toPersisted 剔除 app(3→2)`、`HOME_PERSISTED_TYPES 不含 app` 等 → **105 断言全 PASS**。

### 8.2 夹具本轮的自我修正（诚实记录）

| # | 缺陷 | 修正 |
|---|---|---|
| 1 | 把播种标记 `setItem(DIRS_SEEDED_KEY,"1")`（:72）误报为敏感落盘，真实落盘点未定位 | 只判 `JSON.stringify(<含 target 集合>)`；纯字面量跳过 |
| 2 | 加固判定用邻近窗口（±800 字符）→ 同文件合规代码把坏样本"洗白"，**绕过 toPersisted 会漏检** | 改**数据流判定**：表达式含加固函数，或被序列化变量在本文件由过滤产生 → 合规；其余直落 → 报 |

坏样本 6 → 7（新增"绕过 toPersisted 直落集合"）。

### 8.3 三模式 + 真实回归防呆

```bash
--self-test        PASS（好样本零违规 + 7 坏样本全检出）
default            PASS（all invariants hold ACTIVE=3）   ← closeout 硬要求 ✅
--expect-pending   NONE（3 个 pending 均未检出）          ← A3 修复闭环 ✅
```
**真实回归防呆**：临时移除 `save()` 的 `toPersisted` → 精确检出 `:110`；还原 → `NONE`；`diff -q` 确认文件完整还原。

### 8.4 Intentionally Pending Debt（D-1..D-5）

详见 `logs/checkpoints/A4-M5-W17-acceptance-closeout-20260908-1145.md` §4：
D-1 A2 `run-gui.sh` 未管 Vite 生命周期（debug 依赖 `localhost:1421`）｜D-2 夹具未接 pre-merge（A9/A0）｜D-3 PENDING 待 A0 转 ACTIVE｜D-4 桌面视觉验收仍为用户侧证据（A8）｜D-5 A5/A6/A7 HOLD 无重开理由。

### 8.5 A4 closeout 结论

A3 修复复核**合格**，默认策略 **PASS**，夹具经回归防呆实测有效；**A4 不阻塞 W17 closeout**。

---

## 9. Closeout 末环：A11 验证矩阵采纳 + PENDING→ACTIVE flip（board L1219 / A11 矩阵行 35）

### 9.1 A11 采纳本夹具 + 给出 Flip 指令

A11 `logs/assist/A11-M5-W17-closeout-matrix-20260907-2139.md`：
- 行 18：将本夹具纳入 closeout 验证矩阵 —— `A4 home policy | default PASS + self-test PASS(ACTIVE=3/PENDING=3) + --expect-pending NONE (RC=0)`。
- 行 35：**明确行动项** —— *Still listed as PENDING in A4's policy (not promoted to ACTIVE) → **A4 should flip it, else a future regression won't be caught by the default gate***（3 项已闭环债务仍标 PENDING 会导致默认门禁漏检回归）。

### 9.2 A4 执行 flip（× 2）

`scripts/check-home-client-policy.py`：
1. `HOME_NO_SECRET_PERSIST` / `HOME_NO_RAW_ERROR_ECHO` / `HOME_NO_SENSITIVE_TARGET_RENDER` 由 `PENDING_CODES` 移入 `ACTIVE_CODES`。
2. `PENDING_CODES` 置空 `()`；docstring 标注「空为正常闭环态」；变异防呆只校验 ACTIVE 非空（不再要求 PENDING 非空）；`--expect-pending` 语义改为「确认无遗留历史 pending 债务」（NONE / RC=0）。

### 9.3 flip 后三模式 + 回归防呆实测

```bash
--self-test        PASS（1 好样本零违规 + 7 坏样本全检；ACTIVE=6 PENDING=0）
default            PASS（home client policy: all invariants hold ACTIVE=6）   ← 默认门禁现守 6 项
--expect-pending   NONE（0 个 pending 码位均未检出）                          ← 兼容 A11 矩阵记录
# 回归防呆（核心）：移除 save() 的 toPersisted
sed -i 's/JSON.stringify(toPersisted(shortcuts))/JSON.stringify(shortcuts)/' src/stores/useHomeStore.ts
python3 scripts/check-home-client-policy.py   → HOME_CLIENT_POLICY_RESULT=FAIL
                                            HOME_NO_SECRET_PERSIST:src/stores/useHomeStore.ts:110（home 集合未经过滤直落…）
# 还原后 --expect-pending → NONE，diff -q 一致
```
→ **A11 行 35 要求已满足**：未来回归会被默认门禁捕获。

### 9.4 债务终态更新

- **D-3 关闭**（已按 A11 指令 flip，default 门禁守护 6 项）。
- 剩余：D-2（夹具未接 pre-merge，A9/A0 范围）、D-4（桌面视觉验收用户侧证据，A8）、D-5（A5/A6/A7 HOLD）、D-6（url/dir 绝对路径有意保留，A10 判非阻断）。
- A4 W17 closeout **全部 actionable 项完成**，不阻塞 A0 集成。
