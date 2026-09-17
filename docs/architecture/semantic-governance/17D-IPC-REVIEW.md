# 17D-IPC-REVIEW.md — 对抗性第二轮 IPC 契约复核（Reviewer D）

**Repo:** `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
**Round:** adversarial round-2 · Reviewer D
**Method:** 对每个命令独立重推完整链路 — `FE caller → bridge.ts 签名 → invoke 命令名 → 实参对象键 → Rust #[tauri::command] 签名（参数名+类型）→ main.rs generate_handler! 注册 → ACL → 返回 DTO → FE 消费类型`。
**只读：** 全程 read_file / search_content / search_file，未修改任何产品代码、脚本或 round-1 文档 00–16。

---

## 0. Headline Verdicts

| # | Round-1 命题 | Reviewer D 裁决 | 依据 |
|---|---|---|---|
| 1 | `create_grid` 无半改契约，CONSISTENT | **CONFIRMED** | SR-EVID-0001…0006 |
| 2 | `grid_position` = 定位+尺寸+显示，CONSISTENT | **CONFIRMED** | SR-EVID-0007…0010 |
| 3 | `sync_browser_scene` 为 IPC DRIFT_RISK | **OVERSTATED（应重分类）** | SR-EVID-0025/0026/0027 |
| 4 | 无守卫 typed invoke 占位 = 5 条 | **UNDERSTATED（实为 7 条）** | SR-EVID-0018…0021/0024 |
| 5 | 无守卫占位总数 = 9 条 | **UNDERSTATED（实为 11 条）** | SR-EVID-0021/0024 |
| 6 | checker 正则漏 typed invoke | **CONFIRMED（量化 49/161）** | SR-EVID-0018/0019 |
| 7 | BROKEN = 0 | **CONTRADICTED（实为 1：`move_path`）** | SR-EVID-0015/0016/0017 |
| 8 | EVID-022「注册↔ACL 闭包牢固」 | **CONTRADICTED** | SR-EVID-0016 |
| 9 | checker GATE PASS | **CONTRADICTED（应 FAIL）** | SR-EVID-0035 |
| 10 | CONSISTENT = 39 | **UNDERSTATED（实为 51）** | §3 + §4 |
| 11 | UNVERIFIED = 0 | **CONFIRMED** | §4 |

---

## 1. SR-EVID 证据目录（Reviewer D）

**SR-EVID-0001** Claim: FE `createGrid` 同时发送 `n` 与 `urls`。 FACT
File: `src/bridge.ts` L534-535 — `createGrid: (n: number, urls: string[]) => invoke<number>("create_grid", { n, urls })`
Caller: `useBrowserStore.ts:314` Why: FE 侧键完整。Confidence: HIGH

**SR-EVID-0002** Claim: Rust `create_grid` 同时接受二者。 FACT
File: `src-tauri/src/bridge.rs` L3851 — `pub fn create_grid(app: AppHandle, n: usize, urls: Vec<String>) -> Result<usize, String>`
Confidence: HIGH

**SR-EVID-0003** Claim: 唯一 FE 调用点传入二者，**不存在** `createGrid(n)` 旧调用点。 FACT
File: `src/stores/useBrowserStore.ts` L300-314 — 306-311 构建 urls（长度恒 = n），314 `const created = await bridge.createGrid(n, urls);`
Why: **决定性驳倒**"半改契约"假设。Confidence: HIGH

**SR-EVID-0004** `create_grid` 已注册（main.rs:1440）。FACT · **SR-EVID-0005** ACL 放行（default-commands.toml:42）。FACT

**SR-EVID-0006** 返回 DTO 对齐：Rust `Result<usize,String>` ↔ FE `invoke<number>`；FE L316 `created < n` 判降级，与后端内存预算守卫一致。FACT

**SR-EVID-0007** FE `gridPosition` 发 `{index,x,y,width,height}`（bridge.ts 542-545）。FACT
**SR-EVID-0008** Rust `grid_position` 参数逐一对齐（bridge.rs 4022-4053；50ms/同 rect 去重；4053 `GridCmd::UpdateRect`）。FACT
**SR-EVID-0009** `UpdateRect` 子进程处理器执行 position+size+**show**（main.rs 391-425；397 set_position / 399-403 set_size / 414 update_rect / **423 win.show()**）。FACT
**SR-EVID-0010** 不存在独立"显示宫格"命令（`show_grid|grid_show|fn show\b` 零命中）。FACT
**SR-EVID-0011** `close_grid` 是"关全部"语义（bridge.rs 3915-3918 `for index in mgr.indices()`）。FACT
**SR-EVID-0012** hide 系列签名闭合（608 / 635；注册 1446/1447；ACL 48/49）。FACT

**SR-EVID-0013** 12 条 tab 命令参数名与类型全闭合（tab_new 4266 / tab_close 4273 / tab_open 4279 / tab_position 4299-4306 / tab_list 4612 / tab_set_title 4620 / tab_go_back 4630 / tab_go_forward 4636 / tab_reload 4642 / eval_in_tab 4649 / set_tab_hibernation 4550 / tab_activate 4663）。
Why: round-1 表**遗漏** 6 行（tab_reload / tab_set_title / tab_go_back / tab_go_forward / eval_in_tab / set_tab_hibernation）。FACT

**SR-EVID-0014** terminal 4 命令闭合：`term_spawn_channel`(4771) `TermInfo{id, probe}` 与 FE `invoke<{id:string;probe?:boolean}>`（bridge.ts 651-652）**逐字段相等**；term_write 4787 / term_resize 4801 / term_kill 4817。FACT

**SR-EVID-0029** `term_spawn` 注册+ACL 齐全但**全仓零 FE 调用**（bridge.rs 4758 / main.rs 1464 / ACL 66；src/ 内唯一出现是 useSystemStore.ts:344 的**注释**）。`issue_intent`（bridge.rs 2029 / main.rs 1472 / ACL L9）同情况。FACT

**SR-EVID-0030** 3 条凭据命令闭合，返回码集合**精确相等**（import 6775-6778+6634-6638；list 6807；fill 6971-6976；AUTOFILL_CODES 10 项 + FILL_FAILED，bridge.rs 7057-7058 **恰好等于** FE `AutofillResult` 11 元联合）。FACT

**SR-EVID-0031** 书签 3 命令闭合（3467/3480/3488；注册 1422-1424；ACL 25-27）。FACT
**SR-EVID-0032** 文件/工作区 15 条闭合（3496/3556/3591/3614/3640/3673/3650/3662/3704/3736/3761/3780/3803/3810 + fs_cmds.rs:8）。FACT

**SR-EVID-0015 — ★ 本轮决定性发现**
Claim: **`move_path` 是可达的 BROKEN 契约**。 FACT
File: `src/bridge.ts` 507-509 → `src/stores/useWorkspaceStore.ts` 679-690（684 `await bridge.movePath(src, dst);`）→ `src-tauri/src/main.rs:1432` → `src-tauri/src/fs_cmds.rs:21` → `default-commands.toml` **缺失**
Observed: FE 发 `invoke("move_path", { src, dst_dir: dstDir })`；Rust `pub fn move_path(app, src: String, dst_dir: String) -> Result<(),String>`（**参数名与类型完全匹配**）；已注册；但 ACL `commands.allow` 中**不存在** `move_path`
Caller: 文件树拖拽 → moveConfirm → confirmMove()；Callee: bridge.movePath
Side effects: 无（被拦截），UI 走 catch → `layout.showToast("移动失败: " + ...)`（L687-689）
Why: 直接**推翻** BROKEN=0 与 EVID-022；用户可达、DRAG-MOVE 必失败。Confidence: HIGH

**SR-EVID-0016** 注册集与 ACL 集**恰好差 1 项**，唯一差项是 `move_path`（148 − 147 = 1；夹在 L34 reveal_path 与 L35 open_source 之间）。FACT
**SR-EVID-0017** 不存在其它授予 `move_path` 的 ACL 源（capabilities/default.json、browser-remote.json、remote-collect.toml 均无）。FACT

**SR-EVID-0018** checker 正则**确实**匹配不到 typed invoke（`check-command-set-consistency.py:82-83` `invoke\(\s*["\']([^"\']+)["\']`；`invoke<SkillDef[]>("skill_list", …)` 不匹配）。FACT
**SR-EVID-0019** 盲区量化：161 个 invoke 调用点中仅 **49** 个可见（≈30%），其余 ≈112 个 typed 不可见。FACT
**SR-EVID-0020** `KNOWN["main_invoke_not_registered"]` 仅含 4 项（agent_chat / agent_chat_cancel / confirm_agent_install / confirm_skill_install）。FACT
**SR-EVID-0021** **11** 条 agent/skill 命令在 src-tauri 全仓不存在任何 Rust 定义/注册/ACL（含 round-1 漏掉的 `skill_runs_list`、`agent_runs_list`）。FACT
**SR-EVID-0022** 其中 **7 条是 typed**（正则不可见 + 不在 KNOWN）= 完全无守卫（bridge.ts 401/403/417-418/420-421/431-432/462/463）。FACT
**SR-EVID-0023** 4 条 armed 占位确实是 untyped（425/428/436/438）。FACT
**SR-EVID-0024** 11 条均不可运行（dormant），因此**不是** BROKEN（`AGENT_SKILL_COMMANDS_AVAILABLE=false` bridge.ts:102 + `useAgentStore.guard()` 82-87）。FACT

**SR-EVID-0033 — ★ round-1 结构性遗漏**
Claim: 6 条**已落地**的只读 agent/skill 命令是完整闭合契约，round-1 表里完全没有。 FACT
File: bridge.rs 6529 `agent_parse` / 6539 `agent_validate` / 6549 `agent_permission_preview` / 6597 `skill_parse` / 6607 `skill_validate` / 6617 `skill_permission_preview`
注册 main.rs **1518-1523**；ACL **121-126**；FE bridge.ts 407-414；`AGENT_SKILL_READONLY_COMMANDS_AVAILABLE = true`（bridge.ts:106）；`useAgentStore.ts:275-302` 真实调用。
Why: round-1 CONSISTENT 少计 6 的直接原因。Confidence: HIGH

**SR-EVID-0025** `sync_browser_scene` 在 src/ 与 src-tauri/ **均不存在**（10 个命中全是 md 文档）。FACT
**SR-EVID-0026** "渲染浏览器场景"由 ≥8 个离散原生命令、从 **2 个模块**发出（useBrowserStore.ts 约 13 处；useBrowserHost.ts 141 gridPosition / 155 hideWebview / 172 hideAllWebviews）。FACT
**SR-EVID-0027 — ★ 重分类依据** Claim: 单一 scene 适配器在架构基线中是 **APPROVED_TARGET（尚未建设的规划目标）**，不是"已实现后被破坏的契约"。 FACT
File: `docs/AI/00-Architecture.md` **L261**：「`BrowserRuntime`/`MockRuntime`/`BrowserScene`/`syncScene`… are `APPROVED_TARGET` only; 0 hits in `src/`. `src/bridge.ts` is the factual precursor but is not yet formalized.」
Why: 把"规划未落地"判为 IPC 契约 DRIFT_RISK 属**错桶**；应降级为 **GOVERNANCE_DRIFT**。Confidence: HIGH
**SR-EVID-0034** main.rs 确有两个 generate_handler! 块（116-122 CHILD；1402-1553 MAIN 148 条）。FACT
**SR-EVID-0035** 当前代码状态下 checker 应报 **GATE FAIL**（A 含 move_path，B_main 不含，`KNOWN` 空集 → "NEW DRIFT" → exit 1）。INFERENCE（静态推演，未执行）
**SR-EVID-0036** 远程 ACL 缺口确为 3 条且已 armed（collect.js 120/150/167 vs remote-collect.toml:8）。FACT

---

## 2. 重推的完整 IPC 契约表

### 2.1 Grid（8）— 全部 CONSISTENT
create_grid / close_grid / grid_open / grid_position / grid_set_zoom / grid_close_one / hide_all_webviews / hide_webview
（FE bridge.ts 534/537/539/542/548/551/555/558 ↔ Rust 3851/3915/3949/4022/3999/4059/635/608 ↔ 注册 1440-1447 ↔ ACL L42-L49）

### 2.2 Tab（12）— 全部 CONSISTENT
tab_new / tab_close / tab_activate / tab_open / tab_position / tab_list / **tab_reload**（round-1 未列表却在 focus 清单）/ **tab_set_title**（漏）/ **tab_go_back**（漏）/ **tab_go_forward**（漏）/ **eval_in_tab**（漏）/ **set_tab_hibernation**（漏）

### 2.3 Terminal（4 + 1 带外）
term_spawn_channel / term_write / term_resize / term_kill — 全部 CONSISTENT
**term_spawn** — DEAD_SURFACE（注册+ACL 齐全，src/ 零调用者）

### 2.4 Credentials（3）
import_browser_credentials / list_browser_credentials（round-1 未列表却在 focus 清单）/ fill_browser_credential — 全部 CONSISTENT

### 2.5 Bookmarks（3）
add_bookmark / list_bookmarks / remove_bookmark — CONSISTENT

### 2.6 Workspace / File（15 + 1 BROKEN）
browse_workspace / list_dir / read_file / read_image_data_url / write_file / get_start_dirs / reveal_artifact / reveal_path / **move_path ★BROKEN** / open_source / create_file / create_dir / delete_path / rename_path / clipboard_read / clipboard_write

### 2.7 Agent / Skill（6 landed + 11 unlanded）
**已落地闭合 6**：agent_parse / agent_validate / agent_permission_preview / skill_parse / skill_validate / skill_permission_preview（round-1 全漏）
**无守卫 typed 7**：skill_list / agent_list / skill_install / agent_install / skill_run / skill_runs_list / agent_runs_list（后两条 round-1 漏）
**armed untyped 4**：confirm_skill_install / confirm_agent_install / agent_chat / agent_chat_cancel

### 2.8 带外观察
| Item | 事实 | 分类 |
|---|---|---|
| `term_spawn` | 签名/注册/ACL 齐全，src/ 零调用者 | DEAD_SURFACE |
| `issue_intent` | L2029 / ✓1472 / ✓L9，零调用者 | DEAD_SURFACE |
| `sync_browser_scene` | 代码零命中；架构基线自述 APPROVED_TARGET | GOVERNANCE_DRIFT（非 IPC 契约漂移） |

---

## 3. CASE 重裁

**CASE-006 — create_grid：CONFIRMED（CONSISTENT）。** FE 封装、唯一 caller、Rust 签名、注册、ACL、返回类型全链闭合，**无任何半改痕迹**。round-1 正确。

**CASE-005 — gridPosition/UpdateRect：CONFIRMED（CONSISTENT）。** `main.rs:391-425`，末端 `win.show()`（L423）；不存在独立 show 命令。附带：显示经 `UpdateRect` 带去重，隐藏经不带去重的 `hide_*`——这是**设计取舍**（bridge.ts:553-554 有注释），非契约缺陷。

**CASE-009 — sync_browser_scene：OVERSTATED（应重分类为 GOVERNANCE_DRIFT）。** 事实层面 round-1 没错；但作为**一条 IPC 契约行**计入 DRIFT_RISK 是错桶：(a) 它从不是 IPC 命令名；(b) 架构基线明确声明其为 APPROVED_TARGET 并把 `src/bridge.ts` 认可为"事实上的前驱"⇒ **现状被显式认可**，不是"实现后漂移"。

**IPC_PARTIAL_CHANGE 模式：2 例（其中 1 例 round-1 完全没看到）**
1. 未落地的 agent/skill 占位 —— 实际 **11** 条（round-1 说 9），其中 **7 条 typed** 零门禁保护；因 `AGENT_SKILL_COMMANDS_AVAILABLE=false` + `useAgentStore.guard()` 不可运行 ⇒ DRIFT 非 BROKEN。
2. **注册↔ACL 断裂**：`move_path` 已注册但未放行 —— 典型"半完成落地"，且恰好是 round-1 声称不存在的那一类。

---

## 4. 独立重算的最终计数

| Bucket | Round-1 | Reviewer D | Δ |
|---|---|---|---|
| CONSISTENT | 39 | **51** | +12 |
| DRIFT_RISK | 10 | **11** | +1 |
| BROKEN | 0 | **1** | **+1（CONTRADICTED）** |
| UNVERIFIED | 0 | **0** | 0 |
| DEAD_SURFACE | — | 2 | +2 |
| GOVERNANCE_DRIFT | — | 1 | +1 |

**Severity**：S3(DRIFT)=11；S2(BROKEN，用户可达但非崩溃)=1；S4=0。

**CONSISTENT 39 → 51 的来源（+13 −1）**：
- a) agent/skill 只读命令**已落地且闭合** **+6**
- b) tab 命令漏行 **+6**（tab_reload / tab_set_title / tab_go_back / tab_go_forward / eval_in_tab / set_tab_hibernation）
- c) credentials 漏行 **+1**（list_browser_credentials）
- d) move_path 移出 CONSISTENT **−1**
合计 39+13−1 = 51

**DRIFT_RISK 10 → 11**：无守卫 typed 占位 5→**7**（漏 skill_runs_list、agent_runs_list）；armed untyped 4 不变；sync_browser_scene 移出 IPC 桶 ⇒ 11。

**BROKEN 0 → 1**：`move_path`。可达链路闭合到 UI：文件树拖拽 → moveConfirm → confirmMove() → bridge.movePath → 被 capabilities/default.json 拒绝 → UI 显示"移动失败"。**注册+实现+参数全对，唯独漏 ACL**——"可调用 ≠ 契约正确"的教科书反例。

**UNVERIFIED 0 → 0**：确认。全局声明：未执行任何运行时代码，所有判定均为静态源判定。

---

## 5. Checker 交叉复核

| 维度 | Round-1 | Reviewer D |
|---|---|---|
| 三源覆盖 A/B_main/B_remote/C_main/C_remote | 正确 | CONFIRMED |
| 两个 generate_handler! 块 | 正确 | CONFIRMED |
| 「注册↔ACL 闭包牢固」 | FACT | **CONTRADICTED**（148 vs 147，差集 = {move_path}） |
| GATE 结论 | PASS（隐含） | **CONTRADICTED**（应 FAIL exit 1） |
| 正则盲 typed invoke | FACT | CONFIRMED 并量化（161 → 仅 49 可见） |
| KNOWN 4 + remote 3 | 正确 | CONFIRMED |
| 无守卫命令数 | 5 | **7** |

额外：`C_main` 只扫 `src/bridge.ts`；`src/stores/*` 若绕过 bridge 直接 invoke 将完全逃逸（当前无实际案例，不计入计数）。

---

## 6. Round-1 具体错误清单

| # | Round-1 断言 | 事实 | 级别 |
|---|---|---|---|
| 1 | EVID-022「MAIN handler 每条命令都在 ACL」 | 148 vs 147，move_path 缺失 | **事实错误** |
| 2 | BROKEN = 0 | move_path 必失败 | **事实错误** |
| 3 | 隐含 GATE PASS | 应为 FAIL | **事实错误** |
| 4 | 「5 条 typed 占位」 | 7 条 | **低估** |
| 5 | 表缺 tab_reload / list_browser_credentials | 均在本轮 focus 清单且闭合 | **遗漏** |
| 6 | 表缺 6 条已落地只读 agent/skill | 均注册+ACL+可调用 | **遗漏 → CONSISTENT 少计** |
| 7 | CASE-009 计为 IPC DRIFT_RISK 行 | 规划未落地，非 IPC 契约漂移 | **错桶** |
| 8 | EVID-008/009/012/021/023 | 全部复现通过 | **正确** |

---

## 7. Recommendations

1. **P0 — 修 `move_path` ACL**：在 `default-commands.toml` 的 `reveal_path`(L34) 与 `open_source`(L35) 之间补 `"move_path"`。一行修复，是当前唯一 BROKEN。
2. **P0 — 让门禁真的能抓到它**：确认 `check-command-set-consistency.py` 由 CI/pre-merge 实际执行；它具备检测能力却未体现为 FAIL，说明未跑或结论未被采信。
3. **P1 — 扩正则**：`invoke(?:<[^>]*>)?\(\s*["\']([^"\']+)["\']`，把 ≈112 个 typed 调用点纳入 C_main。
4. **P1 — 补 KNOWN**：加入 7 条（或落地后端）。扩正则后若不加 KNOWN，门禁会立刻 FAIL——这是期望行为，需用"治理决策"而非"再加白名单"关闭。
5. **P2 — 死面清理**：`term_spawn`、`issue_intent` 零调用者，建议标注 M0-internal 或移除（CAPABILITY ≠ CONTRACT）。
6. **P2 — CASE-009 处置**：不要新建占位；把 `src/bridge.ts` 正式命名为 sanctioned IPC adapter（架构文档 L261 已承认其为 factual precursor），并明确 `useBrowserStore`/`useBrowserHost` 职责切分（pos/zoom 归 host 节流层，lifecycle 归 store）。

---

## 8. 与其他 Agent 的交叉提示

- **Agent E（native）**：`grid_position` 的"定位即显示"在 `main.rs:423` 是硬耦合；若 E 的模型是"单一 native scene adapter"，请修正为"bridge.ts + 8 个离散命令 + 子进程 UDS"。
- **Agent C（lifecycle）**：`move_path` 与生命周期无关，但 `close_grid`/`grid_close_one`（先 HideWindow 后 kill）与 `hide_*` 应与其状态机对表。
- **Agent A（state）**：`useBrowserStore` 是 `create_grid` 唯一 caller 且承担降级后 `gridCount` 回写（L316-317）——返回值语义是前端状态真实依赖，不能只当 informational。

**签收：** Reviewer D · 只读 · 静态源判定 · 65 行契约 + 3 行带外全部闭合
