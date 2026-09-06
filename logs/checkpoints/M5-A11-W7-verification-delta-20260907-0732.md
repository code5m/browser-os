# M5-A11 · W7 验证增量（Verification Delta After 5f92ece Integration; A3/A5 W7 Pending）

```text
LANE=A11
STATUS=BLOCKED（W7 集成树 pre-merge FAIL：cargo fmt 未过 + 构建指标 warnings_increased；cargo test 编译中断。
  三者同源：5f92ece 集成卫生问题（plugin.rs import 错位 + bridge.rs 未格式化）。
  A8/A9 W6 验证绿；A3/A5 W7 代码未到本仓。）
BASE=5f92ece（graph UI + plugin policy slices 由 A0 集成；W7 dispatch HEAD=a26fbaf）
HEAD=logs/checkpoints/M5-A11-W7-verification-delta-20260907-0732.md
FILES=logs/checkpoints/M5-A11-W7-verification-delta-20260907-0732.md
VERIFY=见 §1 全门表（当前工作树，含 5f92ece 集成）
CHECKPOINT=本文件
MERGE_NOTES=§5 红灯与修复配方（交 A0/A9；A11 不改产品代码）；§6 A3/A5 W7 待交付
NEXT=待 A0 修 5f92ece 集成卫生（plugin.rs import + cargo fmt 全量）→ 重新 pre-merge ALL_PASS → A3/A5 W7 交付后补验
```

> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W7 Integration Dispatch（L136-172）→ **A11 = START VERIFICATION**：「Update W7 verification matrix and M5 final readiness list after A3/A5 outputs.」scope `logs/assist/M5-A11-W7-*.md`、`logs/checkpoints/M5-A11-W7-*.md`。
> 范围声明：本增量**只产出验证文档，零产品代码改动**。W7 仅开 A3（M5-2 MCP 只读桥）/A5（Agent/Skill 只读桥）产品代码 lane（L167）；其余 lane docs/review/support。
> 姊妹件：W4 `M5-A11-W4-*.md`、W5 `M5-A11-W5-*.md`、W6 `M5-A11-W6-verification-delta-20260906-2242.md`（W6 标注的「A8/A9 pending」本波已落地验证）。

---

## 0. 启动门禁与调度匹配

```bash
cat .workspace-identity              # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master  ✅
pwd                                  # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git fetch origin && git pull --ff-only   # 已是最新（5f92ece 本地=origin/master；W7 dispatch a26fbaf 已 push）
git status --short --branch          # 仅 M5-0-overview.md(M) + A10 W7 review 笔记(untracked)；无 A3/A5 W7 代码
git log --oneline -6                 # a26fbaf(W7 dispatch) / 5f92ece(A8/A9 W6 集成) / 412d0eb / d08d095(A11 W6) ...
```
- **调度匹配**：board 头部 `Current NEXT: M5-W7 integration implementation`（L7）；W7 仅 A3/A5 产品代码（L167）。W7 事实（L140）：graph UI shell + plugin manifest/lifecycle policy 已集成；W7 是窄只读命令桥波，非运行时激活。
- W7 硬停止（L165-171）：仅 A3/A5 可写产品代码；命令只读（无 skill 执行/plugin 安装/MCP server/图重建 worker）；新命令须 source check+ACL+前端 bridge/types+策略覆盖+测试；无 secret 日志/持久化；先 pull 不 push。

---

## 1. W7 验证矩阵（当前工作树实跑，含 5f92ece 集成）

| 门 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | **编译中断**：4 errors 全为 `PluginCapability` not found（plugin.rs:308/324/432/440）；0 tests 可跑 | ❌ RED |
| cargo check 告警 | `cargo check --locked` | **3 warnings**（plugin.rs:20 unused `PluginCapability`；grid_process.rs:76 `index`/`comms`；grid_process.rs:103 `new`） | ❌ RED（基线 2） |
| cargo fmt | `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` | **FAIL（exit 1）**；bridge.rs:8/6128/6231/6244/6295/6318 未格式化（含 mcp_policy_get/mcp_capability_preview 封装） | ❌ RED |
| core 边界门 | `python3 scripts/check-core-boundary.py --self-test` | `CORE_POLICY_SELF_TEST=PASS`（ACTIVE=7） | ✅ |
| M5-2 MCP 门 | `python3 scripts/check-mcp-policy.py --self-test` / default | `MCP_POLICY_SELF_TEST=PASS`（ACTIVE=5，PENDING=9）/ default PASS | ✅ |
| M5-3 agent-memory 门 | `python3 scripts/check-agent-memory-policy.py --self-test` | `AGENT_KV_POLICY_SELF_TEST=PASS`（ACTIVE=5） | ✅ |
| M5-4/5 agent-skill 门 | `python3 scripts/check-agent-skill-policy.py --self-test` / default | `AGENT_SKILL_POLICY_SELF_TEST=PASS`（ACTIVE=2，PENDING=4）/ default PASS | ✅ |
| M5-7/8 graph 门 | `python3 scripts/check-graph-policy.py --self-test` / default | `GRAPH_POLICY_SELF_TEST=PASS`（ACTIVE=7）/ default PASS | ✅ |
| A8 W6 graph UI 逻辑 | `node scripts/check-graph-ui-logic.mjs` | **图谱 UI 逻辑测试：通过 34，失败 0** | ✅ |
| A9 W6 plugin 门 | `python3 scripts/check-plugin-policy.py --self-test` / default / --expect-pending | `PLUGIN_SELF_TEST=ALL_PASS`（ACTIVE=1，PENDING=5）/ `PLUGIN_POLICY=PASS` / `PLUGIN_PENDING_OK` | ✅ |
| M2-4 执行通道 | `python3 scripts/check-script-exec-policy.py --self-test` | `SELF_TEST_RESULT=ALL_PASS` | ✅ |
| tools / db / scheduler 等 | 各自 self-test | 均 PASS（同 W6；5f92ece 未改动这些脚本逻辑） | ✅ |
| 前端构建 | `npm run build` | ✓ built；`index-DllHbynV.js`=173.06 kB（gzip 58.85）；面板懒加载分块（GraphPanel 9.7k / AgentManagerPanel 3.7k / SkillManagerPanel 2.2k / PermissionPreviewModal 6.8k / TaskPanel 24.8k / DatabasePanel 15.4k；xterm 334k 独立） | ✅ |
| 构建指标对比 | `measure-build-metrics.py --compare 4f0e8ab --skip-build` | `total_bytes_pct=20.63`（**<21% 阈值**，A0 于 5f92ece 抬 19→21），`cargo_warnings=1 delta`，**`warnings_increased=true`** | ❌ RED |
| **集成门禁** | `bash scripts/pre-merge.sh` | **`PRE_MERGE_RESULT=FAIL`**（EXIT=1；FAIL：cargo fmt main + build metrics regression） | ❌ RED |

> 注：cargo test 编译中断（4 errors 同源）→ pre-merge 未跑 cargo test（pre-merge 仅 `cargo check`，L170），故 pre-merge 表面只报 fmt+metrics 两项 FAIL；**cargo test 红灯是 A0「before push」清单（board L835-844）必拦项**，须单独修复。

---

## 2. 红灯定位（W7-1 / W7-2 / W7-3 同源）

三盏红灯均来自 `5f92ece` 集成卫生：

- **W7-1（cargo test 编译中断）**：`src/plugin.rs` `#[cfg(test)] mod tests`（L271-446）在 L308/324/432/440 构造 `PluginCapability {..}`，但测试模块导入仅 `use crate::domain::{PluginEntry, PluginSignature};`（L274），**未导入 `PluginCapability`** → `error[E0422]: cannot find struct PluginCapability`。4 errors 全同根。
- **W7-3（构建指标 warnings_increased）**：顶层导入 `use crate::domain::{.. PluginCapability ..}`（plugin.rs:19-21）在非测试构建中 `PluginCapability` 未被使用 → `warning: unused import: PluginCapability`（plugin.rs:20:34）。基线 cargo_warnings=2，当前=3 → `warnings_increased=true`，measure FAIL。size 20.63% 已 <21% 阈值（合规）。
- **W7-2（cargo fmt）**：`bridge.rs` 多处未格式化（import 顺序、struct 字面量多行展开、`mcp_policy_get`/`mcp_capability_preview` 签名折行）；`cargo fmt --all --check` exit 1。属既有 M5-2 集成遗留（5f92ece 未动 bridge.rs，见 §4），非 W7 新增。

**单根修复配方（交 A0/A9；A11 不改代码）**：
1. plugin.rs L274：`use crate::domain::{PluginEntry, PluginSignature};` → `use crate::domain::{PluginEntry, PluginCapability, PluginSignature};`
2. plugin.rs L20：从顶层 import 移除 `PluginCapability`（仅测试使用）→ `use crate::domain::{AclLevel, PermissionPreview, PluginManifest, PluginSignature, PluginState};`
   → 同时消解 W7-1（测试编译）+ W7-3（unused import 告警 3→2=基线 → build metrics PASS）。
3. `cargo fmt --manifest-path src-tauri/Cargo.toml --all` 全量格式化（含 tauri-browser-tabs）→ 消解 W7-2。
   → 修复后预期：cargo test 可编译、cargo check 2 warnings（仅 grid_process.rs 两条既有，与 5f92ece 无关）、fmt 干净、measure `warnings_increased=false`、pre-merge ALL_PASS。

> grid_process.rs:76/103 两条 warning（index/comms 未读、new 未用）为既有债，非本波引入，单独跟踪（建议后续加 `#[allow(dead_code)]` 或消费方）。

---

## 3. A8/A9 W6 集成验证（绿）—— 闭合 W6 的「pending」

W6 增量（d08d095）标注「A8/A9 W6 未到本仓」；`5f92ece` 已集成，本波实测全绿：

- **A8 W6 graph UI**：`scripts/check-graph-ui-logic.mjs` → **34 passed, 0 failed** ✅（GraphPanel/GraphFilter/NodeDetail/EdgeDetail/GraphViewer + useGraphStore + graphUi.ts 纯逻辑）。前端构建懒加载分块（GraphPanel 9.7 kB），size 阈值内。
- **A9 W6 plugin policy**：`scripts/check-plugin-policy.py` → `ALL_PASS`（ACTIVE=1：PLUGIN_NO_SECRETS；PENDING=5：能力/状态机/签名/形态③/凭据字段，均按「产物存在才判」设计）+ default `PASS` + `--expect-pending OK` ✅。`src-tauri/src/plugin.rs`（446 行，纯函数 + 状态机 + 生命周期）、`domain.rs` Plugin* DTO、security_policy.rs 能力白名单，均按 W6 硬停止（无安装/卸载运行时、无下载执行、仅形态③、无凭据字段）。
- 命令表面核查：5f92ece 未新增任何 `#[tauri::command]`（A8/A9 W6 为 UI/策略切片，守「prefer 无命令」）。`git show --stat 5f92ece` 文件清单不含 `bridge.rs`/`main.rs`(仅 +1 行 `mod plugin;`)/`default-commands.toml` → ACL 末条仍为 `list_artifact_images`，无 ACL 漂移。

---

## 4. A3/A5 W7 产品代码：尚未交付至本仓（门禁 PENDING）

按 W7 调度，A3（M5-2 MCP 只读桥：list registry entries / preview capability verdicts / 返回脱敏 DTO）、A5（Agent/Skill 只读桥：parse/validate/permission-preview）应产代码，但本主仓工作树**当前不含**其新命令：

```bash
grep -rnE 'tauri::command' src-tauri/src/mcp.rs          # 仅冻结注释（"未来 mcp_policy_get"，首期仅冻结）；无命令
grep -rnE 'tauri::command' src-tauri/src/agent.rs src-tauri/src/skills.rs   # 无命令
grep -niE 'mcp_list|list_registry|agent_parse|skill_validate|permission_preview_command' src-tauri/src/main.rs  # 无 W7 新命令
git show --stat 5f92ece -- src-tauri/src/bridge.rs       # 空 → 5f92ece 未动 bridge.rs
git log -S 'mcp_policy_get' -- src-tauri/src/bridge.rs   # 空 → bridge.rs 既有 MCP 命令非 W7 新增
```
- **结论**：A3/A5 W7 命令桥（list-registry-entries 等）**PENDING**，待其代码由 A0 集成后补验。
- `bridge.rs` 既有 `mcp_policy_get`/`mcp_capability_preview`（M5-2 集成态，未格式化遗留）属 W7 红灯 W7-2 的 fmt 范畴，非 W7 A3 新增交付。
- 建议：A0 集成 A3/A5 W7 后，A11 补验项 = 新命令的 source check + ACL 奇偶 + 前端 bridge/types + `check-mcp-policy.py`/`check-agent-skill-policy.py` 活跃码提升 + 测试 PASS；并核对 W7 硬停止「只读」（无 skill 执行/plugin 安装/MCP server/图重建 worker）。

---

## 5. Before-A0 冲突扫描

- **空文件**：无（A3/A5 代码未到；其余仅文档）。
- **stale STOPPED 冒充 PASS**：各策略脚本 self-test/default 均真实 PASS（非 STOPPED 伪装）；cargo test 红灯已显式标记，未隐瞒。
- **重复命令名**：`generate_handler!` 无 mcp_/agent_/skill_/plugin_/graph_ 新增（W7 命令未到）；既有 mcp_policy_get/mcp_capability_preview 唯一。
- **bridge/main/types 命令奇偶**：5f92ece 未新增命令，既有 DTO 在 `src/types.ts`/`src/bridge.ts` 同步（5f92ece 含 +14 bridge.ts / +44 types.ts，对应 graph/plugin DTO）；无漂移。
- **docs NEXT 一致性**：board 头部 `Current NEXT: M5-W7` 与 §M5-W7 dispatch 一致。
- **lane scope 漂移**：W7 产品代码仅 A3/A5；A1/A2/A4/A6/A7/A8/A9/A10 均 docs/review/support（如 `logs/assist/A10-M5-W7-security-review-20260907-0100.md` 仅评审笔记）。无漂移。

---

## 6. 债务台账更新（相对 W6）

| 项 | W6 状态 | W7 状态 |
|---|---|---|
| 集成门禁 | ALL_PASS（W6） | **FAIL（fmt + build metrics + cargo test 三红灯）** |
| W5 红灯 cargo_warnings 2→27 | 已结（W6） | 维持结案 |
| A8/A9 W6 验证 | PENDING（代码未到） | **已验绿**（graph UI 34/0；plugin ALL_PASS） |
| **W7-1 cargo test 编译中断（plugin.rs PluginCapability import）** | — | **新增红灯**（4 errors，0 tests 可跑） |
| **W7-3 build metrics warnings_increased（2→3）** | — | **新增红灯**（同源 plugin.rs import；size 20.63%<21% OK） |
| **W7-2 cargo fmt（bridge.rs 未格式化）** | — | **新增红灯**（既有 M5-2 遗留） |
| 构建指标阈值 | 19%（W6 抬升） | **21%**（A0 于 5f92ece 抬 19→21；实测 20.63%，余量 0.37%） |
| grid_process.rs 两条 warning（index/comms, new） | 既有债 | 维持（与 5f92ece 无关） |
| check-agent-skill-ui-logic.mjs 未接 pre-merge | 小缺口 | 维持（测试本身 57/57 PASS；本波未触及） |
| agent-skill 策略缺 --expect-pending 模式 | 不一致 | 维持（4 stale PENDING 码位） |
| mcp/agent-memory/graph --expect-pending | FAIL by design | 维持（待后续 wave 翻转） |
| A3 W7 MCP 只读桥 | — | **PENDING（代码未到）** |
| A5 W7 Agent/Skill 只读桥 | — | **PENDING（代码未到）** |
| 构建指标基线双文件 | 双基线 | 维持（建议归档旧基线） |

---

## 7. 声明（避免误读）

- 本车道**零产品代码改动**；三盏红灯的修复配方（§2）为交给 A0/A9 的指引，A11 **未**修改 plugin.rs / bridge.rs / 任何脚本。
- 未 rebase、未 push（board Merge Rule：仅 A0 推送）。
- 全部结论基于 §1 实跑证据；针对**含 5f92ece 集成的当前工作树**复跑，未引用旧报告（遵守 IF-5 不 stale）。早前一次 `cargo fmt --check` 误报「clean」为工具重定向假象，已用 `TRUE_RC` 权威复跑校正为 FAIL。
- A3/A5 W7 产品代码未落入本仓，其门禁（§4）留待 A0 集成后由 A11 补验；本增量已为此预留验证位与 W7 硬停止核对项。
- 仅 `git add` 本文件，不带入他 lane 改动（含 A10 W7 review 笔记等 untracked 文件均不提交）。
