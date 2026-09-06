# A10 · M5-W2 (MCP/rmcp) 设计层安全复审（Design-layer Security Review of M5-2）

> Lane: `A10` — M5-W2 安全复审（`AI:DEEP / R:xhigh`）
> 时间: 2026-09-06 16:30 CST（W1 边界复审已 PASS 后，接续的下一张 A10 切片）
> BASE=`854bc40`（A2 已落 core 边界门；本复审在边界门之上审 M5-2 设计）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W1（A10=review）+ 用户指令「按 M5-W1/M5-W2 职责继续推进」；`M5-2-rmcp-mcp-policy.md`（A1 权威卡）+ `A3-M5-mcp-20260906-0827-W1-delta.md`
> 范围：**设计层复审 + 实证校准**（M5-2 产品代码尚未启动，见 §2）。**零产品代码改动、零策略脚本改动**。

---

## 0. Lane 输出模板

```text
LANE=A10
STATUS=PASS_WITH_DEBT（M5-2 卡设计层安全无倒退；但实施前置门尚未齐备）
BASE=854bc40
HEAD=docs only（本文件）
FILES=logs/assist/A10-M5-W2-mcp-design-security-review-20260906-1630.md
VERIFY=§2 实证：capability.rs 缺失 / rmcp·tokio 未入 Cargo.toml / check-mcp-policy.py 自测 FAIL 且未挂 pre-merge / open_tool 仍不在 ACL / lib 名为 mvp_core / withGlobalTauri=true · csp=null
CHECKPOINT=logs/assist/A10-M5-W2-mcp-design-security-review-20260906-1630.md
MERGE_NOTES=① M5-2 卡安全设计扎实（fail-closed/只读默认/能力白名单/禁 TCP/禁执行类/审计脱敏/shutdown 序）PASS；② 但 D-1 的 check-mcp-policy.py 自测 FAIL + 未挂 pre-merge，须 A3/A14 修脚本+挂载后方可作 M5-2 门；③ capability.rs(U-4) 缺失，阻塞 M5-2 §3；④ D-5 缓解改用 A3 §2.2 optional+required-features（更正 A10 Wave-0 复审 §5 D-5 旧解法）；⑤ B8/B9（read_file/list_dir 路径根 + tab_list URL 脱敏）卡中未定义，须补防任意本地读；⑥ D-6 MCP 来源通道须落 core 内抽象
NEXT=M5-2.a 开工前须：A0 裁决 B1(a/b) 与 B7；A3/A14 修 check-mcp-policy.py 自测并挂 pre-merge；落 capability.rs(U-4)；补 B8/B9 路径根策略；定 D-6 core 内 MCP 来源通道
```

---

## 1. 范围与方法

M5-W2 = M5-2（内嵌 rmcp + `McpGlobalPolicy` + 首组只读 MCP 工具）。A10 复审聚焦 Wave-0 提出的四类绕过（C1 第二执行路径 / C2 命令暴露无 ACL / C3 凭据泄漏 / C4 不安全安装）在 M5-2 设计上的覆盖度，并校准 A3 W1-delta（§8）对 A10 的两处更正请求。

方法：设计层以 A1 `M5-2` 卡 + A3 W1-delta 为权威；**每项结论均用当前工作树实证校准**（§2），非文档互证。

---

## 2. 实证自测（当前仓库状态，M5-2 产品代码尚未启动）

```bash
$ ls src-tauri/src/capability.rs           # → ABSENT（U-4 未决，M5-2 §3 真源缺失）
$ grep -nE 'rmcp|tokio =' src-tauri/Cargo.toml   # → ABSENT（M5-2 未启动，符合 W1 硬停止）
$ ls scripts/check-mcp-policy.py           # → 存在（496 行，较完整）
$ python3 scripts/check-mcp-policy.py --self-test
  - 无 MCP 产物时 PENDING 码位误触发（gate 失效）：['MCP_RUNTIME_LEAK']
  MCP_POLICY_SELF_TEST=FAIL                # ← 脚本自测 FAIL（门尚未有效）
$ grep -n 'check-mcp-policy' scripts/pre-merge.sh   # → 空（未挂 pre-merge）
$ grep -nE 'open_tool' src-tauri/permissions/default-commands.toml  # → 空（D-7 缺口仍在）
$ grep -nE 'mcp_' src-tauri/permissions/default-commands.toml        # → 空（M5-2 命令未注册）
$ grep -n 'name = "mvp_core"' src-tauri/Cargo.toml   # → 14: name = "mvp_core"（与 A3 §4 修正一致；A1 卡写的 mvp_browser_os_core 已过时）
$ grep -nE 'withGlobalTauri|csp' src-tauri/tauri.conf.json
  12: "withGlobalTauri": true,   15: "csp": null,       # D-3 跟踪项仍在
```

**关键发现**：D-1 的前提「`check-mcp-policy.py` 已建且自测 PASS 并挂 pre-merge」**当前未满足**——脚本虽存在，但 (a) 自测 FAIL、(b) 未挂 pre-merge。另 `capability.rs`(U-4) 缺失，D-5/D-6/D-7 仍待裁决。这些都是 M5-2 **开工前必须收口的门**，非设计倒退。

---

## 3. M5-2 卡设计层安全裁定（PASS）

| 维度 | 卡设计 | A10 裁定 |
|---|---|---|
| C1 第二执行路径 | §4.1 stdio-only 禁 TCP；§5 禁 npm；B3 禁 `script_runner` 之外的执行；复用 core 内部 API 而非 `bridge::*` | ✅ PASS（与 A2 四边界 `mcp→core` 单向一致） |
| C2 命令/ACL | §3 `mcp_server_start/stop`+`mcp_policy_get/set` 全进 ACL，末条恒 `list_artifact_images`；§6 N13 未知能力返回 `-32001 FORBIDDEN` 不泄露清单 | ✅ PASS（设计合规；实现期须机检 parity，见 D-2） |
| C3 凭据/审计 | §4.3 `read_only` 默认 true、`allow_dangerous_sql` 默认 false、`allowed_connection_ids` 默认空；§4.5 `detail` 禁记凭据/DSN/SQL 正文；`mcp_calls.json`(500) + `audit.json` 仅摘要 | ✅ PASS（fail-closed，继承 K3/K5） |
| C4 能力白名单 | §4.2 白名单 8 只读 + 黑名单 10+ 执行类首期不暴露；`capability.rs` 单一真源（D46） | ✅ PASS（设计）— **但 `capability.rs` 当前 ABSENT（U-4），见 §4-2** |

→ **M5-2 卡的安全设计在架构层无倒退，且把 M4 护栏（无第二路径/K3/K5/ACL 末条）落到 MCP 上下文。设计层 PASS。**

---

## 4. 实施前置门（MUST CLOSE before M5-2.a 开工）

### 4.1 · D-1（修订）：`check-mcp-policy.py` 自测 FAIL + 未挂 pre-merge
脚本已存在且码位较全（`MCP_RUNTIME_LEAK`/`MCP_OPTIONAL_DEP`/`MCP_BIN_GATED`/`MCP_NPM_*`/`MCP_LISTEN_PORT`/`MCP_TOOL_CALLS_COMMAND`/`MCP_CAPABILITY_DRIFT`）。但：
- **自测 FAIL**：`--self-test` 报「无 MCP 产物时 PENDING 码位误触发：['MCP_RUNTIME_LEAK']」→ 疑似自测的「好样本须零违规」断言未像 `check-core-boundary.py` 那样**按 ACTIVE/PENDING 分流**（参考其 `real_active = [c for c in real if c in ACTIVE_CODES]`）。PENDING 码位在「无产物」好样本下不应被判违规。
- **未挂 pre-merge**：`pre-merge.sh` 无任何 `check-mcp-policy` 引用。
- **处置**：由 A3/A14 修自测 PENDING 分流 + 挂 `pre-merge.sh`（位置在 `git diff --check` 之前，与 W1 core 门同节奏）。**在自测 PASS 且挂 pre-merge 前，M5-2 不可依赖该门**。

### 4.2 · U-4：`capability.rs` 缺失
A1 `M5-2` §3 与 A4 W2 记要均要求 `capability.rs` 为能力白名单单一真源（D46，A3/A4/A5 共用）。当前 `src-tauri/src/capability.rs` **ABSENT** → M5-2 §3 的 `MCP_CAPABILITY_V1` 无落地处，且 `MCP_CAPABILITY_DRIFT` 码位无法成立。**须先落 `capability.rs`（含 `pub const MCP_CAPABILITY_V1`）**，否则 M5-2 首张卡无法自洽。

### 4.3 · D-5（缓解已修订，更正 A10 Wave-0 复审 §5）：`rmcp → tokio` vs F-1
A3 W1-delta §2 实测：阶段一同 package 双 target 下「独立 crate/bin 隔离 tokio」**失效**（W0 旧解），改为 **`optional + [[bin]] required-features`**（§2.2 草案）：
```toml
rmcp = { version = "3", default-features = false, optional = true }
tokio = { version = "1", features=[...], optional = true }
[features] mcp = ["rmcp","tokio"]          # default 不含 mcp
[[bin]] name="mcp_server" required-features=["mcp"]
```
- 默认 `cargo build/test` **不编译不链接** rmcp/tokio → 主二进制 + `mvp_core` 依赖面零变化，F-1 实质成立。
- `MCP_RUNTIME_LEAK`/`MCP_OPTIONAL_DEP`/`MCP_BIN_GATED` 已在 `check-mcp-policy.py` 定义，须待 §4.1 修自测+挂载后起效。
- ⚠️ 因 `CORE_DEP_NOT_ALLOWLISTED` 阶段一 **PENDING**（见 W1 复审 §5-1），**core 不得含 tokio 在阶段一只能靠 `check-mcp-policy.py` 的 `MCP_RUNTIME_LEAK`（限非 mcp 文件）守门**——故 §4.1 的修脚本+挂载是 D-5 的机器防线，缺一不可。
- A0 须二选一书面记录（A3 §2.3）：(a) 阶段一即上，给 F-1 限定豁免（optional+default-off+仅 mcp bin）；(b) MCP 推迟到阶段二独立 crate。A10 推荐 (a)（feature gate 对主应用影响为 0）。

### 4.4 · D-6：MCP 来源通道须为 core 内抽象
A3 §8 结论 5 采纳并精确化 A10 D-6：MCP 请求**无 `tauri::Webview`**，`check_invocation_source` 不能直接套用；不得把 `mcp` 塞进 `is_known_webview_label` 冒充 webview（红线）。须 M5-2.a 先把 **core 内 MCP 专用来源抽象**定形（新 `scope`+策略开关），否则所有 `mcp_*` 命令将绕过来源校验。落点 = core 内（非 `security_policy.rs` 临时加一条）。

### 4.5 · B8/B9（卡中未定义，须补，否则任意本地读）
A3 §7 列但 A1 `M5-2` 卡 **未给解**：
- **B8**：白名单含 `workspace.list_dir`/`workspace.read_file`，但卡未定义**路径根 allowlist** → MCP 将获得「远程任意本地文件读」。须在 `McpGlobalPolicy` 增 `allowed_fs_roots`（canonicalize 前缀，与插件 `fs_roots` 同源），工具实现强制校验。
- **B9**：`tab_list` 回传 URL 未脱敏 → 须返回前 `redact_sensitive_url`（复用 `security_policy::redact_sensitive_url`）。
- 两者须在 **M5-2.c（首组工具）前**收口。

### 4.6 · D-7：`open_tool` ACL 缺口 + 奇偶门禁
`open_tool` 在 `main.rs` handler + `bridge.ts` 调用但**不在 ACL**（实证仍缺）→ 前端被 Tauri 拒、MCP 若机械扫 handler 建工具清单则「前端拒、MCP 能调」的权限倒挂。须：① A0/A11 决定补 ACL 或注释有意不下发；② 补全量三处 parity（`check-m5-command-parity.py`，D-2）；③ MCP 工具真源强制 `capability.rs` 而非 handler 扫描（A3 已裁定）。

### 4.7 · B7：start/stop 命令 vs 独立 bin 二选一
若采纳 §4.3 的 feature-gated `mcp_server` bin（仅 `--mcp-server` 独立进程模式），则与 `M5-2` 卡的 `mcp_server_start`/`mcp_server_stop` 命令（GUI 内生命周期）**互斥**。A0 须裁决：砍掉 start/stop 命令（仅独立进程）或保留命令（则 bin 模式须配套）。影响命令/ACL 面，须 M5-2.a 前定。

---

## 5. 对前序 A10 复审的更正（应 A3 §8 请求）

- **D-5 缓解措施更正**：A10 Wave-0 复审（`A10-M5-security-review-20260906-1410.md` §5 #5）写「MCP 落独立 crate/bin 仅依赖 mvp-core+rmcp+tokio，主二进制依赖图零变化」——此乃 A3 §2 已修订掉的 W0 解法（同 package 双 target 下不成立）。**现更正为 A3 §2.2 `optional + [[bin]] required-features`**（本文件 §4.3）。
- **W1 边界复审已自洽**：A3 §8 结论 3 指出「A10 W1 复审把依赖白名单写成 tokio 禁」与落地脚本不符；经查 A10 `A10-M5-W1-security-review-20260906-1500.md` §5-1/§6 已正确写明 `CORE_DEP_NOT_ALLOWLISTED` 为 **PENDING（阶段一不判）**、白名单为「未列 tokio 非已禁」，与 A3 §8 一致，**无需再改**。
- **lib 名一致性**：A1 `M5-1` 卡写 `mvp_browser_os_core`，实际落地 `mvp_core`（A3 §4 已更正）。A10 W1 复审已用 `mvp_core`，一致。

---

## 6. 前向关联

- **W1 core 边界（已 PASS）是 M5-2 前提**：`mcp → core` 单向、core 不得含 tokio（`CORE_DEP_NOT_ALLOWLISTED` PENDING → 靠 §4.1/§4.3 的 `check-mcp-policy.py` 守门）。W1 门已立，但 M5-2 的依赖守门须独立建好（§4.1）。
- **D46（capability.rs 单一真源，责任 A3/A4/A5）**：U-4 缺失（§4.2）同时阻塞 M5-2 与 M5-3.b，须优先落。
- **D51（禁 npm/Node）**：`MCP_NPM_SDK_PRESENT`/`MCP_NODE_RUNTIME_PRESENT` 码位已在脚本，待 §4.1 挂载生效。

---

## 7. 声明

- 本轮**零产品代码改动、零策略脚本改动**（A10 为 REVIEW lane；M5-2 产品代码仍锁定，W1 硬停止未放开）。
- 未 rebase / 未 commit / 未 push（board Merge Rule：仅 A0 可推送）。本文件为 A10 自有 lane 文档，与同树他 lane 未提交产物（A1 `M5-*.md`、A4/A5 `logs/*`）无交集，未触碰其文件。
- 结论基于：① A1 `M5-2-rmcp-mcp-policy.md` + A3 `A3-M5-mcp-20260906-0827-W1-delta.md`；② §2 实证（capability.rs/Cargo.toml/check-mcp-policy.py 自测/pre-merge/ACL/tauri.conf.json 实查）。**非文档互证**。
- 本文件即 A10 M5-W2 的**设计层交付**：设计 PASS，实施前置门列 §4（D-1 脚本自测+挂载 / U-4 capability.rs / D-5 optional-feature / D-6 core 来源通道 / B8+B9 路径根+URL 脱敏 / D-7 open_tool parity / B7 模式裁决）。待 M5-2 产品代码落地，A10 接续机检复审。
