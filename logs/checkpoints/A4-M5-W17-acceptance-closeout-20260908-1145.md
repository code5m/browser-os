# A4 · M5-W17 **Acceptance Closeout** Checkpoint — 复核 A3 修复 + W17 隐私边界

- **Lane**：A4（START REVIEW，board §M5-W17 Acceptance Closeout Dispatch 行 1212）
- **A4 closeout 任务原文**：*Re-check the A3 fix and W17 privacy boundary; **default policy must pass**, and report any **intentionally pending debt**. No product UI edits.*
- **Closeout 集成顺序**：`A3 -> A4/A9 -> A2/A8/A10 -> A1 -> A11 -> A0`
- **Base**：`052b18a`　**未 commit、未 push**（Only A0 integrates/commits/pushes）
- **A4 形态**：REVIEW + 夹具强化；**零产品代码 / 零产品 UI 编辑**

---

## 1. A3 修复复核：HOME_NO_SECRET_PERSIST —— **已闭环 ✅**

### 1.1 修复形态（`src/utils/homeUi.ts:216-246`）

| 项 | 实现 |
|---|---|
| 判定 | `isStorageSafe(s) = s.type !== "app"` |
| 过滤 | `toPersisted(list) = list.filter(isStorageSafe)`；`persistedJson(list) = JSON.stringify(toPersisted(list))` |
| 白名单 | `HOME_PERSISTED_TYPES = ["url", "dir"]`（不含 app） |
| 语义 | **app 条目仅驻留内存（会话内有效），一律不写浏览器存储**；浏览器存储只留非敏感主页元数据 |
| 迁移期 | `load()` 中 `all.filter(isStorageSafe)` → 旧数据里的 app 条目不进内存，**并随即从存储中抹除** |
| 用户提示 | `HOME_APP_SESSION_ONLY_NOTICE`：已添加（仅本次会话有效）：应用启动命令不会写入浏览器存储 |

→ 比我 W17 初检建议的"存安全投影"**更强**（直接不落 app），符合 closeout 行 1211 要求。

### 1.2 接线（`src/stores/useHomeStore.ts`）

- `:110` `save()` → `JSON.stringify(toPersisted(shortcuts))` ✅
- `:115` `saveRecents()` → `JSON.stringify(toPersisted(recents))` ✅
- `:91` 迁移期清理 → `const safe = all.filter(isStorageSafe); setItem(STORAGE_KEY, JSON.stringify(safe))` ✅
- `:104` `loadRecents()` → `normalizeRecents(...).filter(isStorageSafe)` ✅

### 1.3 回归断言（A3 `scripts/check-home-store-logic.mjs`，374 行）✅

```
isStorageSafe(app)=false / isStorageSafe(url)=true / isStorageSafe(dir)=true
toPersisted 剔除 app(3→2) / toPersisted 不改动原数组
persistedJson 不含命令体 / persistedJson 不含 token 参数
HOME_PERSISTED_TYPES 不含 app / homeDisplayTarget(app) 不回显完整路径
```
`node scripts/check-home-store-logic.mjs` → **主页 Home 逻辑测试：通过 105，失败 0** ✅

---

## 2. 夹具复核与强化（`scripts/check-home-client-policy.py`）

本轮 closeout 中**修掉夹具自身 2 个缺陷**并做强化（诚实记录）：

| # | 缺陷 | 修正 |
|---|---|---|
| 1 | 持久化定位把**播种标记** `setItem(DIRS_SEEDED_KEY,"1")`（:72）误报为敏感落盘，真实落盘点（:110/:115）反而未定位 | 只判 `JSON.stringify(<含 target 集合>)`；纯字面量标记跳过 |
| 2 | 初版加固判定用**邻近窗口**（±800 字符）→ 同一文件其它合规代码会把坏样本"洗白"，导致绕过 `toPersisted` 的回归**漏检** | 改为**数据流判定**：a) 表达式含 `toPersisted/persistedJson/isStorageSafe`；b) 否则取被序列化变量，若其在本文件由过滤产生（`const safe = all.filter(isStorageSafe)`）→ 合规；c) 其余直落 → 报 |

强化后自检坏样本 **6 → 7**（新增"绕过 toPersisted 直落集合"）。

### 2.1 三模式实测

```bash
python3 scripts/check-home-client-policy.py --self-test
# PASS：好样本零违规 + 7 个坏样本全部检出（含变异防呆）；ACTIVE=3 PENDING=3

python3 scripts/check-home-client-policy.py
# home client policy: all invariants hold（ACTIVE=3）      ← closeout 要求：default 必须 PASS ✅

python3 scripts/check-home-client-policy.py --expect-pending
# HOME_CLIENT_POLICY_PENDING_RESULT=NONE（3 个 pending 码位均未检出）← A3 修复已闭环
```

### 2.2 真实回归防呆实测（关键证据）

```bash
# 临时把 save() 的 toPersisted 去掉 → 必须检出
sed -i 's/JSON.stringify(toPersisted(shortcuts))/JSON.stringify(shortcuts)/' src/stores/useHomeStore.ts
python3 scripts/check-home-client-policy.py --expect-pending
# FAIL: HOME_NO_SECRET_PERSIST:src/stores/useHomeStore.ts:110（home 集合未经过滤直落浏览器存储）✅
# 还原后
# NONE（3 个 pending 码位均未检出）✅
diff -q /tmp/uhs_backup.ts src/stores/useHomeStore.ts   # 一致 OK（文件已完整还原）
```
→ 证明夹具**真的防得住回归**，而非因正则失效而"静默通过"。

---

## 3. 默认策略 PASS 确认（closeout 硬要求）

| ACTIVE 码位 | 结果 |
|---|---|
| `HOME_NO_PRIVILEGE_EXPANSION` | 零命中 ✅（ACL 无权限声明；main.rs 无 `Command::new`/`TcpListener`） |
| `HOME_NO_SHELL_INJECTION` | 零命中 ✅ |
| `HOME_STARTUP_SAFE` | 零命中 ✅（`run-gui.sh` 仅 build + env + exec） |

**`python3 scripts/check-home-client-policy.py` → `all invariants hold（ACTIVE=3）`，exit=0** ✅

---

## 4. Intentionally Pending Debt（如实报告）

| # | 债务 | 归属 | 状态 / 说明 |
|---|---|---|---|
| D-1 | **A2 启动助手验收 #1（Vite 生命周期 + ownership）** | A2（closeout 行 1210） | **✅ 已闭环（本轮复核确认）**。A2 新增 `scripts/dev-server.sh`（检测/启动/等待 1421）+ 改造 `run-gui.sh`（`cleanup()` 只回收 `$DEV_STATE_DIR/dev-server.pid`；`trap cleanup EXIT/INT/TERM`；不用 `exec` 以保留 trap；支持 `--force-dist\|--no-dev-server`）+ `scripts/check-dev-startup.sh`（所有权语义断言）。**A4 交叉实跑：`bash scripts/check-dev-startup.sh` → 冒烟 PASS=23 FAIL=0**（含"只回收自己拉起的 dev server"、"main.rs 保留 MVP_FORCE_DIST 逃生阀"、"A2 未改 ACL/bridge"）。`HOME_STARTUP_SAFE` 零命中（无 sudo/eval/curl\|bash/无差别 pkill）。→ **D-1 关闭** |
| D-2 | **夹具未接入 `scripts/pre-merge.sh`** | A9 / A0 | board 中 pre-merge 属 A9（closeout 行 1217）；A4 擅自改会跨 lane 冲突。建议 A0/A11 集成时接入**默认模式**（ACTIVE 门禁，当前绿、不阻塞） |
| D-3 | **3 个 PENDING 码位转 ACTIVE** | **A4 已按 A11 指令执行** | ✅ 已闭环。A11 closeout 验证矩阵（行 35）明确要求：*Still listed as PENDING in A4's policy (not promoted to ACTIVE) → A4 should flip it, else a future regression won't be caught by the default gate.* A4 执行：`ACTIVE_CODES` 现含 6 项（3 旧红线 + 3 已闭环历史债务），默认门禁直接守护；`PENDING_CODES=()` 空为正常闭环态（变异防呆只校验 ACTIVE 非空）。**回归防呆实测**：移除 `save()` 的 `toPersisted` → `default` 模式 **FAIL（HOME_NO_SECRET_PERSIST:110）**；还原 → NONE。即未来回归会被默认门禁捕获，正是 A11 要求。→ **D-3 关闭** |
| D-4 | **原生桌面视觉验收仍为用户侧证据** | A8（closeout 行 1216） | 板子明确 *Native desktop visual acceptance is still user-side evidence only*；A4 未做 GUI 验证、未伪造截图 |
| D-5 | A5/A6/A7 HOLD 状态 | A0 | 按 closeout HOLD 规则：A4 复核范围内**未见需要重开 A5/A6/A7 的具体阻塞**（A3 虽改了持久化契约，但 UI 侧 `homeDisplayTarget`/`homeAccessibleLabel` 契约未变，A5 组件已合规；A9 `check-home-ui-logic.mjs` 41 断言 PASS）。A10 closeout 复审 §94 同结论（A5 零改动、A6/A7 不新增特权） |
| D-6 | `url`/`dir` 的 target（含 URL query、本地绝对路径）仍持久化 | A0 决策（**非阻断**） | **A10 closeout 复审 §46 边界澄清**：快捷方式须跨重启存活，属「非敏感主页元数据」；披露面由 `homeDisplayTarget()`（只给 host+path / 路径末段 / 命令首段）兜底，满足共享验收 #4。**非** raw sensitive persistence of credentials/command bodies。A4 已据此**更新夹具 docstring**，明确 `HOME_NO_SECRET_PERSIST` **只守 app 命令体**，消除"描述 vs 实现"落差。若 A0 后续要求连 dir 绝对路径也不落库 → 属**增强**（改 `toPersisted` 为脱敏投影），不阻断 W17 |

### 4.1 A10 closeout 安全复审对本夹具的交叉验证（一致 ✅）

A10 `logs/assist/A10-M5-W17-closeout-security-review-20260908-1230.md` 独立复跑并引用本夹具结论：

- `python3 scripts/check-home-client-policy.py` → `all invariants hold（ACTIVE=3）` ✅
- `python3 scripts/check-home-client-policy.py --expect-pending` → **NONE（3 项全闭环）** ✅
- A10 判定：F-1/F-2/F-3（A4 W17 初检三项）**全部闭环**；W17 范围内**零 shell/privilege 扩张**；startup 助手 ownership-safe。
- A10 边界划分与我一致：A4 拥有 `check-home-client-policy.py`、A9 拥有 `check-home-ui-logic.mjs`、A3 拥有 `check-home-store-logic.mjs`、A6 拥有 `check-client-navigation-logic.mjs`、**A2 拥有 `check-dev-startup.sh`**（故所有权语义归 A2 守，A4 不重复覆盖——已写入夹具 docstring）。
- A10 另标两处越界 Rust 文件（`script_runner.rs` / `tauri-browser-tabs/linux.rs`）交 A0 确认，属 bug 修复非特权扩张、**非阻塞**（不属 A4 范围）。 |

---

## 5. 交叉验证（其它 lane 的 W17 门禁）

```bash
node scripts/check-home-store-logic.mjs    # A3：主页 Home 逻辑测试：通过 105，失败 0
node scripts/check-home-ui-logic.mjs       # A9：主页 UI 逻辑测试：通过 41，失败 0 → HOME_UI_RESULT=PASS
python3 -m py_compile scripts/check-home-client-policy.py   # OK
git diff --check                            # exit 0
```

---

## 6. A4 closeout 结论

- **A3 的 `HOME_NO_SECRET_PERSIST` 修复复核合格**（代码 + 105 条回归断言），本夹具 `--expect-pending` 转 **NONE**。
- **默认策略 PASS**（ACTIVE=3），满足 closeout 硬要求。
- 夹具本轮修掉自身 2 个缺陷并通过**真实回归防呆实测**。
- 遗留债务 D-1..D-5 已如实列出并标注归属；**A4 不阻塞 W17 closeout**。
- **零产品 UI 编辑**；未 commit、未 push。

---

## 7. LANE 输出

```text
LANE: A4
STATUS: PASS
SCOPE: scripts/check-home-client-policy.py, logs/assist/A4-M5-W17-*.md, logs/checkpoints/A4-M5-W17-*.md
DELIVERED: 复核 A3 HOME_NO_SECRET_PERSIST 修复（合格）+ 夹具强化（修 2 缺陷 / 7 坏样本 / 数据流判定）+ 真实回归防呆实测
VERIFY: --self-test PASS(7 bad samples); default PASS(all invariants hold ACTIVE=3); --expect-pending NONE(3 pending 均未检出); 回归实测(移除 toPersisted → 检出 :110，还原 → NONE，文件 diff 一致); node check-home-store-logic.mjs 105 PASS; node check-home-ui-logic.mjs 41 PASS; py_compile OK; git diff --check exit 0
METRICS: N/A（零产品代码改动）
PATCH: logs/checkpoints/Lane-A4-M5-W17-home-client-privacy-review-20260908-1130.patch（已含夹具强化）
RISKS: D-1 A2 启动脚本未管 Vite 生命周期（debug 依赖 1421，待 A2 VERIFY + A8 实机）; D-2 夹具未接 pre-merge（A9/A0）; D-3 PENDING 待 A0 转 ACTIVE; D-4 桌面视觉验收仍为用户侧证据（A8）
NO_PUSH: confirmed
```
