# 06-IPC-CONTRACTS — Semantic Governance Audit (Agent D)

**Scope:** Cross-layer IPC contract closure for the audited themes: grid creation/show/hide/position, browser tab lifecycle, `sync_browser_scene`, terminal, credentials, bookmarks, workspace/file ops.
**Method:** static source triage — `src/bridge.ts` (FE wrappers) ↔ `src-tauri/src/bridge.rs` (Rust `#[tauri::command]`) ↔ `src-tauri/src/main.rs` (`generate_handler!`) ↔ `src-tauri/permissions/*.toml` (ACL) ↔ `scripts/check-command-set-consistency.py` (three-way guard).
**Principle audited:** ONE CONTRACT = one cross-layer operation → one explicit contract.

---

## A. Evidence Catalog (EVID)

### FACTS

- **EVID-001** Claim: FE wrapper `createGrid` sends both `n` and `urls`. Classification: FACT. File: `src/bridge.ts`. Symbol: `bridge.createGrid`. Current line: 534-535. Observed behavior: `invoke<number>("create_grid", { n, urls })`. Callers: `useBrowserStore.ts` (EVID-003). Callees: `invoke("create_grid")`. Confidence: HIGH.
- **EVID-002** Claim: Rust `create_grid` takes `n: usize` **and** `urls: Vec<String>`. Classification: FACT. File: `src-tauri/src/bridge.rs`. Symbol: `create_grid`. Current line: 3850-3851. Observed behavior: `pub fn create_grid(app: AppHandle, n: usize, urls: Vec<String>) -> Result<usize, String>`. Confidence: HIGH.
- **EVID-003** Claim: The only FE caller of `createGrid` passes both args. Classification: FACT. File: `src/stores/useBrowserStore.ts`. Symbol: `createGrid` call. Current line: 314. Confidence: HIGH.
- **EVID-004** Claim: `create_grid` is registered and ACL-allowed. Classification: FACT. File: `src-tauri/src/main.rs`. Symbol: `bridge::create_grid`. Current line: 1440. ACL: `src-tauri/permissions/default-commands.toml` line 42. Confidence: HIGH.
- **EVID-005** Claim: FE `gridPosition` passes `{index, x, y, width, height}`. Classification: FACT. File: `src/bridge.ts`. Symbol: `bridge.gridPosition`. Current line: 542-545. Confidence: HIGH.
- **EVID-006** Claim: Rust `grid_position` receives `index,x,y,width,height` and emits `GridCmd::UpdateRect`. Classification: FACT. File: `src-tauri/src/bridge.rs`. Symbol: `grid_position`. Current line: 4021-4054. Confidence: HIGH.
- **EVID-007** Claim: The `UpdateRect` child-process handler performs position + size + **show**. Classification: FACT. File: `src-tauri/src/main.rs`. Symbol: `GridCmd::UpdateRect` handler. Current line: 391-424 (esp. 423 `win.show()`). Confidence: HIGH.
- **EVID-008** Claim: No dedicated "show grid" command exists. Classification: FACT. File: `src-tauri/src`. Symbol: `show_grid|grid_show|fn show`. Confidence: HIGH.
- **EVID-009** Claim: `sync_browser_scene` is NOT implemented in Rust or Vue. Classification: FACT. File: `src-tauri` and `src`. Symbol: `sync_browser_scene`. Confidence: HIGH.
- **EVID-010** Claim: Governance docs prescribe a single `sync_browser_scene` adapter as the only entrypoint for native WebView commands. Classification: FACT. File: `解决过的问题/解决普通模型无法处理方案/终极方案v1.md` (L508), `agent和project.md` (L352), `解决普通模型方案.md` (L277/329/1016). Confidence: HIGH.
- **EVID-011** Claim: No `BrowserRuntime`/`syncScene` adapter class exists in `src`. Classification: FACT. Confidence: HIGH.
- **EVID-012** Claim: Native WebView commands are invoked from ≥2 modules, not one adapter. Classification: FACT. File: `src/stores/useBrowserStore.ts` and `src/composables/useBrowserHost.ts`. Confidence: HIGH.
- **EVID-013** Claim: The checker regex only matches *untyped* `invoke("cmd", …)`; it skips `invoke<T>("cmd", …)`. Classification: FACT. File: `scripts/check-command-set-consistency.py`. Symbol: `invoked()`. Current line: 83. Confidence: HIGH.
- **EVID-014** Claim: `skill_list`, `agent_list`, `skill_install`, `agent_install`, `skill_run` are wired in `bridge.ts` but have **no Rust `#[tauri::command]`**. Classification: FACT. Confidence: HIGH.
- **EVID-015** Claim: The 5 unlanded commands are neither in the checker `KNOWN` allowlist nor detected by its regex (all typed). Classification: FACT. Confidence: HIGH.
- **EVID-016** Claim: The 4 `KNOWN` allowed placeholder invokes are all *untyped* → detectable by the regex. Classification: FACT. Confidence: HIGH.
- **EVID-017** Claim: `close_grid` iterates all grid indices → it is the "close all" semantic. Classification: FACT. File: `src-tauri/src/bridge.rs`. Symbol: `close_grid`. Current line: 3914-3915. Confidence: HIGH.
- **EVID-018** Claim: `import_browser_credentials` DTO matches. Classification: FACT. File: `src/bridge.ts` L478-479; `src-tauri/src/bridge.rs` L6775-6778; `BrowserCredentialRow` L6634-6638. Confidence: HIGH.
- **EVID-019** Claim: `fill_browser_credential` returns a `String` status code that matches the FE `AutofillResult` union. Classification: FACT. File: `src/bridge.ts` L484-488; `src/types.ts` L189-192; `src-tauri/src/bridge.rs` L6971-6976. Confidence: HIGH.
- **EVID-020** Claim: Tab command signatures align with FE. Classification: FACT. File: `src-tauri/src/bridge.rs`. Symbols/lines: `tab_new` L4266; `tab_close` L4273; `tab_open` L4279; `tab_position` L4299; `tab_activate` L4663. Confidence: HIGH.
- **EVID-021** Claim: `main.rs` has TWO `generate_handler!` blocks — CHILD (L116-122) and MAIN (L1402-1553). Classification: FACT. Confidence: HIGH.
- **EVID-022** Claim: Every command in the MAIN `generate_handler!` is present in `default-commands.toml`. Classification: FACT. Confidence: HIGH.
- **EVID-023** Claim: Remote ACL allows only `report_resources, report_title, report_grid_load_failed`; the 3 remote-invoke drifts are armed in checker `KNOWN`. Classification: FACT. Confidence: HIGH.

### INFERENCES
- **EVID-024** Claim: CASE-006 is **CONSISTENT** — no half-changed `create_grid(n)` vs `create_grid(n,urls)` contract exists. Classification: INFERENCE. Confidence: HIGH.
- **EVID-025** Claim: CASE-005 is **CONSISTENT** — `grid_position` does position+size+**show** and that is reflected in the Rust doc comment + absence of a separate show command. Classification: INFERENCE. Confidence: HIGH.
- **EVID-026** Claim: CASE-009 is **DRIFT_RISK** — the single canonical exit `sync_browser_scene` was never implemented; the "render browser scene" operation is fragmented across ~8 discrete native commands with multiple callers. Classification: INFERENCE. Confidence: HIGH.
- **EVID-027** Claim: The checker reports GATE PASS for the agent/skill placeholders only because its regex is blind to typed invokes, and only the 4 untyped ones are in `KNOWN`. The 5 unlanded commands are unguarded drift. Classification: INFERENCE. Confidence: HIGH.
- **EVID-028** Claim: The 5 unlanded commands are not runtime-BROKEN because `AGENT_SKILL_COMMANDS_AVAILABLE=false` prevents their invocation; they are dormant contract placeholders. Classification: INFERENCE. Confidence: MEDIUM.

### RECOMMENDATIONS
- **EVID-029** Recommendation: Fix the checker regex to also capture `invoke<Type>("cmd", …)` and add the 5 commands to `KNOWN` (or land their Rust backends). Confidence: HIGH.
- **EVID-030** Recommendation: Either implement `sync_browser_scene` as the single scene adapter, or update governance docs to reflect the discrete-command reality and designate `useBrowserStore`/`useBrowserHost` as the sanctioned adapters. Confidence: MEDIUM.

### UNVERIFIED
- **EVID-031** Claim: Runtime behavior of each command was not executed; assessed by source only. Classification: UNVERIFIED.

---

## B. IPC Contract Table (theme commands)

| Command | FE Signature (bridge.ts) | Rust Signature (bridge.rs) | Reg | ACL | DTO Match | Status |
|---|---|---|---|---|---|---|
| create_grid | `createGrid(n, urls)` → `{n,urls}` (L534) | `create_grid(app, n:usize, urls:Vec<String>)` (L3851) | ✓L1440 | ✓L42 | `usize`↔`number` | **CONSISTENT** (CASE-006) |
| close_grid (close-all) | `closeGrid()` (L537) | `close_grid(app)` (L3915) | ✓L1441 | ✓L43 | `()`↔void | **CONSISTENT** |
| grid_open | `gridOpen(i,u)`→`{index,url}` (L539) | `grid_open(app,index,url)` (L3949) | ✓L1442 | ✓L44 | string↔String | **CONSISTENT** |
| grid_position | `gridPosition(i,p{x,y,w,h})`→`{index,...p}` (L542) | `grid_position(app,index,x,y,w,h)` (L4022) → `UpdateRect`(+show) | ✓L1443 | ✓L45 | x/y/w/h↔f64 | **CONSISTENT** (CASE-005) |
| grid_set_zoom | `gridSetZoom(i,zoom)`→`{index,zoom}` (L548) | `grid_set_zoom(app,index,zoom:f64)` (L3999) | ✓L1444 | ✓L46 | f64↔number | **CONSISTENT** |
| grid_close_one | `gridCloseOne(i)`→`{index}` (L551) | `grid_close_one(app,index)` (L4059) | ✓L1445 | ✓L47 | usize↔number | **CONSISTENT** |
| hide_webview | `hideWebview(id)`→`{id}` (L558) | `hide_webview(app,id:String)` (L608) | ✓L1447 | ✓L49 | String↔string | **CONSISTENT** |
| hide_all_webviews | `hideAllWebviews()` (L555) | `hide_all_webviews(app)` (L635) | ✓L1446 | ✓L48 | ()↔void | **CONSISTENT** |
| tab_new | `tabNew(url)`→`{url}` (L571) | `tab_new(app,url:String)→TabInfo` (L4266) | ✓L1451 | ✓L53 | TabInfo↔TabInfo | **CONSISTENT** |
| tab_close | `tabClose(id)`→`{id}` (L573) | `tab_close(app,id:String)` (L4273) | ✓L1452 | ✓L54 | String↔string | **CONSISTENT** |
| tab_open (nav) | `tabOpen(id,u)`→`{id,url}` (L578) | `tab_open(app,id,url)` (L4279) | ✓L1453 | ✓L55 | String↔string | **CONSISTENT** |
| tab_position | `tabPosition(id,p)`→`{id,...p}` (L580) | `tab_position(app,id,x,y,w,h)` (L4299) | ✓L1454 | ✓L56 | f64↔number | **CONSISTENT** |
| tab_list | `tabList()` (L585) | `tab_list` (L1455) | ✓L1455 | ✓L57 | TabInfo[]↔Vec | **CONSISTENT** |
| tab_activate | `tabActivate(id)`→`{id}` (L576) | `tab_activate(app,id)` (L4663) | ✓L1457 | ✓L59 | String↔string | **CONSISTENT** |
| term_spawn_channel | `termSpawnChannel(ch)`→`{channel}` (L651) | `term_spawn_channel(app,channel:Channel)` (L4771) | ✓L1465 | ✓L67 | Channel↔Channel | **CONSISTENT** |
| term_write | `termWrite(id,data)`→`{id,data}` (L654) | `term_write(app,id,data)` (L4787) | ✓L1466 | ✓L68 | String↔string | **CONSISTENT** |
| term_resize | `termResize(id,cols,rows)`→`{id,cols,rows}` (L656) | `term_resize(app,id,cols:u16,rows:u16)` (L4801) | ✓L1467 | ✓L69 | u16↔number | **CONSISTENT** |
| term_kill | `termKill(id)`→`{id}` (L659) | `term_kill(app,id)` (L4817) | ✓L1468 | ✓L70 | String↔string | **CONSISTENT** |
| import_browser_credentials | `importBrowserCredentials(rows[{url,username,password}])`→`number` (L478) | `import_browser_credentials(app,rows:Vec<BrowserCredentialRow>)→usize` (L6775) | ✓L1524 | ✓L142 | row fields↔struct | **CONSISTENT** |
| fill_browser_credential | `fillBrowserCredential(credentialId,tabId)`→`AutofillResult` (L484) | `fill_browser_credential(app,webview,credential_id,tab_id)→String` (L6971) | ✓L1526 | ✓L144 | String↔union | **CONSISTENT** |
| browse_workspace | `browseWorkspace()`→`WorkspaceTree` (L263) | `browse_workspace` (L1421) | ✓L1421 | ✓L24 | — | **CONSISTENT** |
| bookmark add/list/remove | L474-477 | L3467/3480/3488 | ✓ | ✓ | Bookmark↔Bookmark | **CONSISTENT** |
| fs ops (read/write/create/delete/rename/move) | L491-523 | fs_cmds/bridge | ✓ | ✓ | — | **CONSISTENT** |
| skill_list / agent_list / skill_install / agent_install / skill_run | L401,403,417,421,431 (typed) | **no Rust impl** | ✗ | ✗ | n/a | **DRIFT_RISK** (unguarded) |
| agent_chat / agent_chat_cancel / confirm_agent_install / confirm_skill_install | L425,428,436,438 (untyped) | **no Rust impl** | ✗ | ✗ | n/a | **DRIFT_RISK** (KNOWN/armed) |

---

### Hidden Side-Effect Column (per CONFLICT-01)
CONFLICT-01 裁定：06 契约签名一致（正确），但命令确有非声明的可见性副作用（07 正确）。为避免 IPC 契约文档遗漏隐藏语义，补充副作用列（主表 `grid_position` 行已含 `(+show)` 提示，此处集中显式声明）：

| Command | Declared Contract | Hidden Side-Effect | Severity |
|---|---|---|---|
| grid_position | position+size | 经 `UpdateRect` 末端 `win.show()` 实际 show（05/16 CLAIM-LC-04，EVID-GR-04，main.rs:423） | S3（架构耦合，18 SECOND-CONFLICT-004 由 S4 降 S3；不产生静默错误） |
| tab_position | position+size | `apply_bounds_inner` 内 `set_visible(true)`（EVID-LY-02，bridge.rs:555） | S3（同上，机制耦合） |
| tab_new | create tab | `tabPosition(-30000)` 直发屏外隐藏绕过 `hide_webview`（07 EVID-BYPASS-007，useBrowserStore.ts:170） | S3 |
| close_grid | close | `HideWindow` 先于 kill（HIDDEN 态过渡） | S3（架构耦合） |
| 其余命令 | — | 无未声明可见性副作用 | — |

> 注：未引入独立 `show_webview` 命令（06 EVID-008）——补独立 show 会引入 `set_visible(false)` 死锁风险（bridge.rs:567-569），故保持"显示熔接于 position"现状，仅在此显式声明（对应 00 DO NOT REFACTOR YET）。

---

## C. CASE Verdicts

**CASE-006 — create_grid contract:** **CONSISTENT** (EVID-001/002/003/004/024). The hypothetical half-changed contract `create_grid(n)` vs `create_grid(n, urls)` is **not present**: FE wrapper, FE caller, and Rust command all carry `urls`. Registered (main.rs:1440) and ACL-allowed (default-commands.toml:42). No drift.

**CASE-005 — gridPosition / UpdateRect:** **CONSISTENT** (EVID-005/006/007/008/025). `grid_position` performs position **and** size **and** show: it sends `GridCmd::UpdateRect`, whose child handler (main.rs:391-424) calls `set_position`, `set_size`, `update_rect`, and `win.show()` (L423). There is no separate "show grid" command.

**CASE-009 — sync_browser_scene:** **DRIFT_RISK** (EVID-009/010/011/012/026). The single canonical exit described in governance docs is **not implemented**; the "render browser scene" operation is fragmented across ≥8 discrete native commands invoked from **two** modules rather than one sanctioned adapter.

**IPC_PARTIAL_CHANGE pattern:** Two concrete instances —
1. Unguarded unlanded agent/skill wiring (EVID-014/015/027): 5 commands wired in `bridge.ts` (typed invokes) with **no Rust command** and **not** in the checker `KNOWN` allowlist, and the checker regex cannot see them → zero gate protection.
2. Governance-vs-code divergence for `sync_browser_scene`.

---

## D. Checker Cross-Check (`scripts/check-command-set-consistency.py`)

- The three-way guard covers A (main.rs), B_main (default-commands.toml), B_remote (remote-collect.toml), C_main (bridge.ts invocations), C_remote (collect.js).
- **Registration↔ACL closure is solid** (EVID-021/022): every command in the MAIN handler block is in `default-commands.toml`.
- **Known drifts armed** (py L44-51): 4 agent/skill placeholders + 3 remote. All consistent with code.
- **Critical gap (EVID-013/015/027):** the `invoked()` regex only matches *untyped* `invoke("cmd")`. The majority of `bridge.ts` calls are typed and **invisible** to the gate → 5 unlanded commands slip through. Recommendation: broaden regex to `invoke(?:<[^>]*>)?\(\s*["\']` and add the 5 to `KNOWN`.

---

## E. Counts

- CONSISTENT: **51**（`18` IPC_FINAL 终审：第二轮将 06 的 39 复核扩至 51，含 12 条来源抽查）
- DRIFT_RISK: **11**（7 条未落地 typed agent/skill 占位 + 4 条 armed 占位；第二轮核实为 7 非 5）
- BROKEN: **1**（`move_path` 四面闭合独缺 ACL，用户可达 100% 失败；`18` §2#3 / SECOND-CONFLICT-006，**推翻第一轮 `BROKEN=0`**）
- GOVERNANCE_DRIFT: **1**（`sync_browser_scene` 代码 0 命中，架构基线 APPROVED_TARGET）
- UNVERIFIED: **0**

> **Severity（IPC 主题，局部）**：S4 = 0（IPC 主题无红线违反；`move_path` BROKEN 显式 toast、可恢复 → 不计入 S4，归 S2）；S3 = 11（DRIFT_RISK）+ IPC 门禁零执行力（见 `18` S3#7）；S2 = 1（`move_path`）+ 其余派生项。全局风险计数以 `00` / `18` §4 为准，本块仅限 IPC 主题。

---

## F. Conflicts with other audit Agents

- **Agent E (native):** Direct overlap on CASE-005 and CASE-009. If E assumes a single native scene adapter exists, reality is discrete commands + child-process UDS.
- **Agent C (lifecycle):** Overlap on grid lifecycle — `close_grid`/`grid_close_one` (kill+restart), `hide_webview` hide semantics, show-via-`UpdateRect` coupling. Confirm position==show is lifecycle-coupled.
- **Agent A (state):** Overlap on tab/grid state ownership — `useBrowserStore.ts` is the caller. The FE state model and IPC contract must agree on who triggers `grid_position`/show.

### CONFLICT-01 裁决落地（06 vs 07）
06 把 `grid_position` 建模为纯布局契约（CASE-005 CONSISTENT），07 证明它经 `UpdateRect` 末端 `win.show()`（main.rs:423）实际 show。裁决：二者对各口径均成立——契约签名一致（06 正确），但命令确有非声明的可见性副作用（07 正确）。已在上方"Hidden Side-Effect Column"显式声明该副作用，不再遗漏。
