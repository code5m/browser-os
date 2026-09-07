# Lane A9 — M5-W17 BUG-HUNT Follow-up Checkpoint（START REVIEW）

> 生成：2026-09-07 22:06 CST · Lane A9（M5-W17 · BUG-HUNT Follow-up Dispatch → 行 1245：**START REVIEW**）
> 依据：`PARALLEL_COMMAND_BOARD.md` §BUG-HUNT Follow-up Dispatch → A9：
> `scripts/check-command-set-consistency.py`（新增，policy 脚本），`src-tauri/src/commands.rs`、`src-tauri/permissions/default-commands.toml`、`src/injected/collect.js`、`src/bridge.ts`、policy scripts；
> **Verify open_tool and collect.js command ACL/source consistency; add the smallest static consistency guard. No new capability beyond already registered local commands.**
> BUG-HUNT order：`A2/A3/A5/A6/A7/A8/A10/A9 -> A4 -> A11 -> A0`。A9 仅测试/复查 + 加静态门禁，不写产品代码、不补 ACL（补 ACL 归其它 lane / A0）。
> 工作树含其它 lane 未提交改动（以工作树为准）；HEAD 未变（`052b18a`）。

```
LANE=A9
STATUS=PASS（BUG-HUNT 三源一致性门禁已加；open_tool / collect.js 漂移已核实并记录为已知漂移，无新增不一致）
BASE=052b18a
HEAD=logs/checkpoints/Lane-A9-M5-W17-bug-hunt-command-set-20260907-2206.patch
FILES=scripts/check-command-set-consistency.py,
      logs/checkpoints/Lane-A9-M5-W17-bug-hunt-command-set-20260907-2206.md
VERIFY=python3 scripts/check-command-set-consistency.py （期望 GATE: PASS，exit 0，仅含已知漂移）
VERIFY2=python3 scripts/check-command-set-consistency.py --report （期望完整报告，exit 0）
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W17-bug-hunt-command-set-20260907-2206.md
METRICS=Registered(A)=135, DefaultACL=134, RemoteACL=3, MainInvoke=46, RemoteInvoke=3；已知漂移=open_tool + collect.js×3 + agent/skill×4；新增不一致=0
PATCH=logs/checkpoints/Lane-A9-M5-W17-bug-hunt-command-set-20260907-2206.patch
RISKS=见下
NO_PUSH=confirmed（board Merge Rule：仅 A0 推送）
```

## 交付物

- **`scripts/check-command-set-consistency.py`**（新增，纯静态文本比对，零运行时依赖）：
  - 三方比对：**A**=Rust 主窗口注册命令（`src-tauri/src/main.rs` 的 `generate_handler!`）；**B_main**=`default-commands.toml` 的 `commands.allow`；**B_remote**=`remote-collect.toml` 的 `commands.allow`；**C_main**=`src/bridge.ts` 的 `invoke("...")`；**C_remote**=`src-tauri/injected/collect.js` 的 `invoke("...")`。
  - 检查四类差集：`A-B_main`（已注册但主窗口 ACL 未放行）、`B_main-A`（死 ACL）、`C_main-A`（前端封装未注册=死调用）、`C_remote-B_remote`（远程 webview 调用未放行的命令）。
  - 行为：默认 **gate 模式**仅当存在「非已知漂移」时非零退出（CI 门禁）；`--report` 仅打印、始终 exit 0。
  - 内建 **KNOWN_DRIFT 允许清单**，收录 BUG-HUNT-SUMMARY 已记录的存量漂移，使门禁只拦「新增」漂移、不阻塞已知在跟进项（呼应 B12-02「三源无一致性门禁，漂移只靠偶然发现」）。

## 核实结论（open_tool / collect.js 一致性）

| 检查 | 结果 | 对应 BUG-HUNT 发现 |
|---|---|---|
| `tools::open_tool` 已注册（main.rs:1451）+ bridge.ts 调用（bridge.ts:152） | ✅ 注册&调用存在 | — |
| `open_tool` **不在** `default-commands.toml` `commands.allow` | ❌ 缺失 → 主窗口也会 `not allowed` | B1-1 / B12-01 |
| `collect.js` 调 `collect_selection`/`save_note`/`request_open_terminal` | ✅ 调用存在 | — |
| 上述三命令**不在** `remote-collect.toml`（仅放行 `report_*`） | ❌ 缺失 → 远程页面右键菜单静默失败 | B1-2 / B9-3 / B12-03 |
| `bridge.ts` 调 4 个 agent/skill 契约占位（`agent_chat`/`agent_chat_cancel`/`confirm_agent_install`/`confirm_skill_install`）未注册 | ⚠ 已知（契约占位，标志位拦截，零 invoke） | 非缺陷 |

- `B_main - A = {}`：默认 ACL 无死条目（默认权限集与注册集一致）。
- 门禁当前 **GATE: PASS**：仅含上述已知漂移，无新增不一致。

## 不修改产品代码（符合 A9 scope）

- 仅新增 policy 脚本 + 本 checkpoint；**未改** `default-commands.toml` / `remote-collect.toml` / `main.rs` / `commands.rs` / `collect.js` / `bridge.ts` / 任何 `.rs`/`.vue`/store/桥/ACL。
- **补 ACL 修复**（把 `open_tool` 加入 `default-commands.toml`、把 collect.js 三命令的处理补进 `remote-collect.toml` 或让 collect.js 远端跳过）属于其它 BUG-HUNT lane（A2/A3/A5/A6/A7/A8/A10）/ A0 的整包职责，A9（REVIEW + 门禁）不越权新增能力。

## Verify（可复现）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
python3 scripts/check-command-set-consistency.py
# 期望：
#   Registered (A): 135 | Default ACL (B_main): 134 | Remote ACL (B_remote): 3
#   Main invocations (C_main/bridge.ts): 46 | Remote invocations (C_remote/collect.js): 3
#   [registered_not_in_default_acl] known: open_tool  (KNOWN DRIFT ...)
#   [main_invoke_not_registered]  known: agent_chat, agent_chat_cancel, confirm_agent_install, confirm_skill_install
#   [remote_invoke_not_in_remote_acl] known: collect_selection, request_open_terminal, save_note
#   GATE: PASS — only known drift present (no new inconsistency).
#   exit 0

python3 scripts/check-command-set-consistency.py --report   # 完整报告，exit 0
```

## Merge Notes

- **补丁范围**：`git diff HEAD -- scripts/check-command-set-consistency.py`（仅 A9 新增脚本；patch 已用 `git add -N` 渲染为新文件 157 行，生成后已 `git reset HEAD` 还原 index，不污染工作树）。
- **KNOWN_DRIFT 维护约定**：一旦某 lane 修复 `open_tool` / collect.js 三命令，该 lane 应从脚本 `KNOWN` 字典移除对应条目，使门禁重新武装、避免漂移被永久静默。此约定已写入脚本头部注释。
- **A0 集成建议**：在 `pre-merge.sh` 的 `run_self_test` 增加 `python3 scripts/check-command-set-consistency.py`（与 A9 W17 的 `check-home-ui-logic.mjs` 一并），把三源一致性纳入 CI 门禁。

## RISKS（诚实剩余风险）

- 门禁是**静态集合成员比对**，只捕获「注册/ACL/调用」三方集合不一致；不校验参数形态、权限作用域语义、或插件 crate 内部权限（7 个插件命令 close_tab/create_tab/list_tabs/navigate/set_visible/set_zoom/update_rect 由插件自身权限文件管辖，已刻意排除在本三源之外——若日后需覆盖，需扩展脚本解析插件 permission toml）。
- `KNOWN_DRIFT` 是把「已知存量」从失败中豁免的旋钮；若长期不随修复回收，会弱化门禁。已用注释约定要求修复方移除对应条目。
- 不替代各 lane 的 ACL 实质修复：open_tool / collect.js 功能失效仍待对应 lane 闭环（非 A9 职责）。

## FORBID 遵守（BUG-HUNT / W17）

- 仅新增 `scripts/check-command-set-consistency.py` + 本 checkpoint；**未改任何产品代码 / ACL / 三份主文档**。
- 未移动 `NEXT`；未 push。
