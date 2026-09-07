# A1 · M5-W9 reconciliation checkpoint（2026-09-07 14:30 CST · Lane A1 · 文档对账 + 整包交付 · 不 push）

> **STATUS**：DRAFT（待 A0 拣入后定）
> **LANE**：A1（docs-only reconciliation）
> **WAVE**：M5-W9 · **Runtime-Free Polish Dispatch**（PARALLEL_COMMAND_BOARD.md L175-210，Added 2026-09-07 14:30 CST by A0）
> **BASE**：`97118d6`（origin/master HEAD · A0 W8 拣入完成 · 4 commit = `f51549f` + `a840fcb` + `94e763e` + `4d7be97` + `97118d6` = 53 files +6038 -101 + 5 files +665 -180 + 3 files patch whitespace 规范化）
> **HEAD**：工作树（11 文件改动 + 1 new checkpoint · 11 文件 = 7 M5-*.md + 2 主文档 + 1 TODO + 1 后续新增 checkpoint；未提交，未 push）
> **A1 W9 任务（PARALLEL_COMMAND_BOARD L185-186 A1 行）**：*START DOCS ONLY* · *Reconcile W8 as accepted-with-fixes and mark W9 as active NEXT* · *Update M5 child-card status for MCP read-only bridge, Agent/Skill read-only bridge, Graph UI polish, plugin review, and build metrics threshold 22%*
> **整包交付结束**：本文件 + `logs/checkpoints/Lane-A1-M5-W9-reconciliation-20260907-1430.patch` 整包；不 push；A0 拣入期合并策略同 W8（A1 W8 整包 + A1 W9 整包合并拣入避免 A0 分两次消）

---

## 0. 一句话

按 `PARALLEL_COMMAND_BOARD.md` L175-210（**M5-W9 Runtime-Free Polish Dispatch**，Added 2026-09-07 14:30 CST by A0）的 A1 行指令 *"Reconcile W8 as accepted-with-fixes and mark W9 as active NEXT. Update M5 child-card status for MCP read-only bridge, Agent/Skill read-only bridge, Graph UI polish, plugin review, and build metrics threshold 22%"*，A1 在 W9 仅做文档对账：**(a)** 工作树 + origin/master 同步 + 拣入现状检视（HEAD = `97118d6` 干净 + 4 commit A0 拣入 W8 + 3 主文档 A0 W8 update 已就位）；**(b)** M5-0 头部标题加 W8 reconciliation + W9 active + 时间戳加 W8 拣入 14:30 CST（4 commit 回填：f51549f + a840fcb + 94e763e + 4d7be97 + 97118d6）+ W9 active 14:30 CST 行 + 基准补 5 commit；**(c)** M5-0 新增 `[W8 reconciliation]` 段（4 commit 拣入事实回填 + 13 项 W8 reconciliation 自检 PASS 清单 + 7 项 W8 残留债挂账 = DEBT-43/DEBT-46~51/DEBT-12/DEBT-22/DEBT-04/DEBT-23~26/DEBT-31~33）+ `[W9 active]` 段（11 lane 全部活跃 + A1 W9 = DOCS ONLY + W9 角色表 11 项 + W9 hard stops 8 条 + A1 W9 整包交付索引）；**(d)** M5-9/10/11/12 四张子卡头部状态行升级为 W8 reconciliation + W9 ACTIVE（11 lane 状态按 W9 dispatch 拆分）；**(e)** M5-13 头部加 W8 reconciliation + W9 ACTIVE 状态行 + 新增 `[W8 reconciliation]` 段（4 commit 拣入事实回填 + 13 项 W8 reconciliation 自检 PASS 清单 + 7 项 W8 残留债挂账）+ 新增 `[W9 verification scope]` 段（**11 lane final verification** 14 FAC 矩阵：FAC-1~12 沿用 + FAC-13 build metrics 22% 阈值 + FAC-14 A11 final verification matrix + W9 FAIL_ACTION 8 条红线 + A11 W9 delta 必标字段 9 项）；**(f)** M5-14 顶部加 W8 reconciliation + W9 ACTIVE 行（IF-2 22% 阈值复检 + M5 final debt ledger 51→53 条 W9 增量预期 = 2：FAC-1.b M5-1.b 收口 + DEBT-04 W10+ A19 plugin UI 派发状态 + W9 复检 4 项必跑）；**(g)** 三份主文档（AI-模型切换 + 详细设计 + 后续需求TODO）L1 加 A1 W9 update 行（与 A0 W8 update 行并列，描述 A1 W9 reconciliation 整包工作）；**(h)** 写本 checkpoint + patch；不写产品代码；不重写 §1~§11 决策史；不移动 `NEXT`；不提交 / 不 push。

---

## 1. W9 一行 prompt

```
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W9 Runtime-Free Polish Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

---

## 2. 整包交付清单（**A1 W8 reconciliation 拣入事实回填 + A1 W9 reconciliation** · 7 M5-*.md 修订 + 3 主文档 + 1 checkpoint + 1 patch = 11 文件 + 1 new checkpoint）

| # | 文件 | 修订类型 | 状态 | 来源 |
|---|------|----------|------|------|
| 1 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | M（头部标题 + 时间戳链 +2 行 + [W8 reconciliation] 段 79 行 + [W9 active] 段 79 行）| **A1 W9 修订**：头部标题加 W8 reconciliation + W9 active；时间戳链加 W8 拣入 14:30 CST（4 commit 回填）+ W9 active 14:30 CST；新增 `[W8 reconciliation · 2026-09-07 13:30 CST]` 段 + `[W9 active · 2026-09-07 14:30 CST]` 段 | **A1 W9** |
| 2 | `logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md` | M（+2 行：W8 reconciliation + W9 ACTIVE）| **A1 W9 修订**：头部 L9 末尾追加 W8 reconciliation 4 commit 回填 + W9 ACTIVE 标注（A8 W9 = GRAPH UI SMALL + A7 W9 = GRAPH CONTRACT DOCS ONLY + A3 W9 = POLICY/REVIEW ONLY）| **A1 W9** |
| 3 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | M（+2 行：W8 reconciliation + W9 ACTIVE）| **A1 W9 修订**：头部 L9 末尾追加 W8 reconciliation 4 commit 回填 + W9 ACTIVE 标注（A9 W9 = PLUGIN REVIEW ONLY + A19 W9 仍 SUPPORT DOCS ONLY）| **A1 W9** |
| 4 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | M（+2 行：W8 reconciliation + W9 ACTIVE）| **A1 W9 修订**：头部 L9 末尾追加 W8 reconciliation 4 commit 回填 + W9 ACTIVE 标注（A9 W9 = PLUGIN REVIEW ONLY commands_isolation 维持 pure / stub 错误结构不变 / audit shape 维持 key_hash_only）| **A1 W9** |
| 5 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | M（+2 行：W8 reconciliation + W9 ACTIVE）| **A1 W9 修订**：头部 L9 末尾追加 W8 reconciliation 4 commit 回填 + W9 ACTIVE 标注（A19 W9 仍 SUPPORT DOCS ONLY + DEBT-04 收口推 W10）| **A1 W9** |
| 6 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | M（头部 +2 行 + 77 行新段）| **A1 W9 修订**：L1 标题加 W8 reconciliation + W9 verification scope + L6 后追加 W8 reconciliation 4 commit 状态行 + W9 ACTIVE 状态行 + 新增 `[W8 reconciliation · 2026-09-07 14:30 CST]` 段 + 新增 `[W9 verification scope · 2026-09-07 14:30 CST]` 段（14 FAC 矩阵 + W9 FAIL_ACTION 8 条红线 + A11 W9 delta 必标字段 9 项）| **A1 W9** |
| 7 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | M（+2 行：W8 reconciliation + W9 ACTIVE）| **A1 W9 修订**：L2 标题加 W8 reconciliation + W9 active + L5 后追加 W8 reconciliation 4 commit 状态行 + W9 ACTIVE 状态行（IF-2 22% 阈值 + M5 final debt ledger 51→53 条 + W9 复检 4 项必跑）| **A1 W9** |
| 8 | `AI-模型切换与接手清单.md` | M（+1 行：L1 追加 A1 W9 update 行）| **A1 W9 修订**：L1 A0 14:30 update 行后追加 A1 W9 update 行 | **A1 W9** |
| 9 | `详细设计与实施计划.md` | M（+1 行：L1 追加 A1 W9 update 行）| **A1 W9 修订**：L1 A0 14:30 update 行后追加 A1 W9 update 行 | **A1 W9** |
| 10 | `后续需求TODO.md` | M（+1 行：L1 追加 A1 W9 update 行）| **A1 W9 修订**：L1 A0 14:30 update 行后追加 A1 W9 update 行 | **A1 W9** |
| 11 | `logs/checkpoints/A1-M5-W9-reconciliation-20260907-1430.md` | **新增** | A1 W9 整包 checkpoint（本文件）| **A1 W9** |

> **A1 W8 reconciliation 整包已被 A0 拣入**（4d7be97 合并拣入 A1 W7 + A1 W8 整包 20 文件 = 9 + 11），**A1 W9 整包不复用 A1 W8 patch**。A1 W9 patch = `git diff --binary` 仅含 A1 W9 11 文件改动 + 1 new checkpoint（`logs/checkpoints/A1-M5-W9-reconciliation-20260907-1430.md`）= A1 W9 整包独立交付。
> **A1 W8 patch + checkpoint 状态**：A1 W8 patch = `logs/checkpoints/Lane-A1-M5-W8-reconciliation-20260907-0945.patch`（120682 bytes / 11 文件改动 / git apply --reverse PASS）+ A1 W8 checkpoint = `logs/checkpoints/A1-M5-W8-reconciliation-20260907-0945.md`（316 行 / §0-§9）—— **两文件均已被 A0 拣入 master**（4d7be97 stat 包含两文件），工作树不再保留；A1 W9 整包为新独立交付。

---

## 3. W8 → W9 关键事实回填（A1 头部状态行内容）

### 3.1 W8 拣入 4 commit 事实（`97118d6` HEAD = A0 拣入完成）

| commit | 含义 | 触及文件 | A1 W8 整包归属 | A1 W9 整包回填 |
|--------|------|----------|----------------|----------------|
| `f51549f` | A6 W8 panel consumption plan + UI logic tests | A6 W8 文档 | （A1 W8 整包不动）| M5-0/9/10/11/12 头部 + M5-13/M5-14 状态行 |
| `a840fcb` | A11 W8 verification delta functional GREEN | A11 W8 文档 | （A1 W8 整包不动）| M5-0/13 状态行 + M5-13 [W8 reconciliation] 段 |
| `94e763e` | A3 W8 MCP policy phase debt 关闭 | A3 W8 政策码 5 files +665 -180 | （A1 W8 整包不动）| M5-0/13/14 状态行 + M5-13 [W8 reconciliation] 段 + M5-13 [W9 verification scope] 段 FAC-2 |
| `4d7be97` | A0 W8 拣入 A1 W7 + A1 W8 整包合并 | 53 files +6038 -101 | A1 W7 + A1 W8 整包 20 文件 = 9 + 11 | （A1 W9 整包独立交付）|
| `97118d6` | A0 W8 拣入 patch whitespace 规范化 | 3 份 W8 patch 文件空白整理 | （A1 W8 整包不动）| M5-0/13/14 状态行 + M5-13 [W8 reconciliation] 段 |

### 3.2 W8 reconciliation 自检 PASS 13 项（[W8 reconciliation] 段已收）

- [x] **A5 `CredentialLeak` 错误体脱敏**（94e763e 关闭 F-W8-1 critical · Display 不 clone secret）
- [x] **A6 `npm run build` PASS**（4d7be97 关闭 F-W6-2 UI 编译失败）
- [x] **A8 graph UI 41 断言 PASS**（4d7be97 关闭 F-W6-1 graph-ui 缺断言）
- [x] **A9 `check-plugin-policy.py` ACTIVE=6**（W7 + W8 累计；plugin install/enable/delete/download 仍缺）
- [x] **A11 verification delta 功能性 GREEN**（a840fcb 关闭 W7-1/W7-2/W7-3 配方未消 = 已消）
- [x] **A3 MCP policy phase debt 关闭**（94e763e 关闭 DEBT-44；ACTIVE=8 PENDING=0）
- [x] **cargo fmt --all**（4d7be97 修 W7-2 8 处未格式化）
- [x] **patch whitespace normalize**（97118d6 修 W7-3 patch 空白）
- [x] **build metrics W8 实测 21.07% ≤ 22% PASS**（IF-2 W8 关闭）
- [x] **cargo_warnings delta = 0**（PluginCapability unused import 已删）
- [x] **pre-merge ALL_PASS**（A11 W8 verification delta 确认）
- [x] **A1 W7 + W8 reconciliation 整包 20 文件合并拣入**（4d7be97 关闭 DEBT-42）
- [x] **A3 local commit boundary 已由 A0 94e763e 解禁**（A3 仍 NO_NEW_RMCP）

### 3.3 W8 残留债挂账 7 项（DEBT-43/46~51/12/22/04/23~26/31~33）

- **DEBT-43**（DRY-F1 残留扩大 = SENSITIVE_KEY_NAMES 双份 + 图谱容量常量三处拷贝）→ A2 W9 review 必填；A2 W9 抽 `domain.rs` 单源
- **DEBT-46~51**（A5/A6/A7/A8/A9/A10 W8 必批闭环项）→ A0 W8 拣入期已通过 4d7be97 + 94e763e + a840fcb 闭环
- **DEBT-12**（plugin install/enable/delete/download 仍缺）→ 后续 wave，非 W8 必消
- **DEBT-22**（build metrics 21% 阈值 IF-2）→ A0 94e763e 后改 22%，W9 阈值不变
- **DEBT-04**（plugin UI 仍 0% ACTIVE）→ A19 W9+ 派发
- **DEBT-23~26**（终端 M3 挂账）→ 非 M5 范畴
- **DEBT-31~33**（A7 graph 桥接 A3/MCP 阻塞）→ A3 解禁后可推 M5-2.b 卡

### 3.4 W9 派发 11 lane 角色摘要

| Lane | W9 模式 | 主要交付 | 触及产品代码 | A1 W9 必填 |
|------|---------|----------|--------------|------------|
| A1 | **DOCS ONLY** | W8 reconciliation 整包拣入事实回填 + W9 active NEXT 标记 + 5 子卡状态更新 + build metrics 22% | **无**（仅文档）| ✅ 本整包 |
| A2 | REVIEW ONLY | A3/A5 W8 修复后 command boundary 复审 + DEBT-43 DRY-F1 抽 `domain.rs` 单源 | **无** | ❌ A2 W9 |
| A3 | POLICY/REVIEW ONLY | MCP policy current-phase green + M5-2.b 卡预备；**不**实施 rmcp/server/listener/network | `scripts/check-mcp-policy.py` policy-only hunk | ❌ A3 W9 |
| A4 | PRIVACY REVIEW ONLY | A5 `CredentialLeak` redaction 后所有命令 error/display 面的 secret echo 复审 | **无** | ❌ A4 W9 |
| A5 | PRODUCT CODE SMALL | Agent/Skill read-only bridge hardening 收口：redacted validation errors + frontend parse/permission preview 边界 case 测试 | `agent.rs` + `skills.rs` + `bridge.rs` + `main.rs` + `default-commands.toml` + `bridge.ts` + `types.ts` + `check-agent-skill-policy.py` | ❌ A5 W9 |
| A6 | UI LOGIC ONLY | Agent/Skill 面板消费磨光：确定性 empty/error/loading + 无 secret 文本 echo + bounded preview 渲染 | `src/components/**` + `src/stores/**` + `types.ts` + `bridge.ts` + `check-agent-skill-ui-logic.mjs` | ❌ A6 W9 |
| A7 | GRAPH CONTRACT DOCS ONLY | 图谱桥契约终稿：runtime-free 与 blocked backend runtime 分离 | **无** | ❌ A7 W9 |
| A8 | GRAPH UI SMALL | 图谱 UI 磨光：filter/search/layout 确定性 + 保留 bounded arrays + 改进 no-backend/read-only 状态 | `src/components/**` + `src/stores/**` + `check-graph-ui-logic.mjs` | ❌ A8 W9 |
| A9 | PLUGIN REVIEW ONLY | plugin surface 复审：manifest/lifecycle 维持 pure + 确认 install/enable/delete/download 仍缺 | `logs/assist/A9-M5-W9-*.md` + optional `check-plugin-policy.py` policy-only hunk | ❌ A9 W9 |
| A10 | SECURITY FINAL REVIEW | 批量复审 W8/W9 outputs：source check / ACL parity / read-only / redaction / 无 runtime 扩张 | `logs/assist/A10-M5-W9-*.md` | ❌ A10 W9 |
| A11 | FINAL VERIFICATION | W9 final verification matrix：命令结果 + build metrics 21.07% ≤ 22% + cargo warnings unchanged + pre-merge result + 残留债 + push readiness | `logs/assist/A11-M5-W9-*.md` + `logs/checkpoints/A11-M5-W9-*.md` | ❌ A11 W9 |

---

## 4. A1 W9 硬停止遵守记录（PARALLEL_COMMAND_BOARD L211-）

| 硬约束 | 类别 | 状态 | 自证 |
|--------|------|------|------|
| **No mcp_* runtime / no rmcp server/listener / no network / no plugin install·enable·delete·download / no skill execution / no model calls** in W9. | 范围 | ✅ 守 | A1 W9 整包仅触及 `logs/checkpoints/M5-20260906/*.md` + 3 主文档 L1 + 1 new checkpoint，**0 行产品代码** |
| **No mcp_*.rs file changes outside scripts/check-mcp-policy.py policy-only hunks** in W9. | 范围 | ✅ 守 | A1 W9 整包不触及 `src-tauri/src/mcp.rs` / `scripts/check-mcp-policy.py` / 任何 `mcp_*.rs` 文件 |
| **Build metrics threshold 22% (unchanged from W8 closure). 22% gate MUST NOT regress.** | 阈值 | ✅ 守 | A1 W9 整包不触及任何 .rs/.toml/.json/.vue/.ts 源文件；build metrics 保持 W8 实测 21.07% ≤ 22% PASS |
| **No token/cookie/Authorization/body/prompt-secret logging, audit, persistence, checkpoint, or frontend state**. | 敏感 | ✅ 守 | A1 W9 整包不记录任何凭据/密钥/secret；checkpoint 内容全为事实陈述（commit hash / 政策码 / FAC 编号 / 债务编号 / 阈值数字）|
| **No mvp_core / no script_execution / no mcp_runtime / no plugin_runtime expansion** in W9. | 范围 | ✅ 守 | A1 W9 整包不触及 `mvp_core/` / `script_runner` / `script_execution` / `mcp_runtime` / `plugin_runtime` 任何文件 |
| **No new cargo warnings. Cargo warnings delta MUST = 0.** | 编译 | ✅ 守 | A1 W9 整包不修改任何 .rs/.toml 文件；cargo_warnings delta = 0 维持 |
| **Lanes must deliver a coherent patch/checkpoint and must not ask A0 to merge tiny partial notes**. | 交付 | ✅ 守 | A1 W9 整包 = 1 patch + 1 checkpoint + 11 文件改动（合并拣入策略同 W8 = A1 W9 整包独立交付）|
| **Only A0 pushes to remote**. | 权限 | ✅ 守 | A1 W9 整包不 `git add` / 不 `git commit` / 不 `git push`；工作树保留修改 |

---

## 5. 验证命令（仅文档工作树）

### 5.1 工作树状态（拣入现状 + A1 W9 改动）

```bash
git status --short --branch
# 预期: ## master...origin/master [领先 0]
# 预期: 11 M + 1 ?? (A1 W9 new checkpoint)
```

### 5.2 diff stat（A1 W9 scope）

```bash
git diff --stat -- 'logs/checkpoints/M5-20260906/' 'AI-模型切换与接手清单.md' '详细设计与实施计划.md' '后续需求TODO.md'
# 预期: 10 files changed, 7 M5-*.md + 3 主文档
```

### 5.3 diff check（无空白/格式错误）

```bash
git diff --check -- 'logs/checkpoints/M5-20260906/' 'AI-模型切换与接手清单.md' '详细设计与实施计划.md' '后续需求TODO.md'
# 预期: 0 输出（clean）
```

### 5.4 A1 W9 product-code touches = 0

```bash
git diff --name-only -- 'logs/checkpoints/M5-20260906/' 'AI-模型切换与接手清单.md' '详细设计与实施计划.md' '后续需求TODO.md' | grep -vE '^logs/checkpoints/|^AI-模型切换与接手清单\.md$|^详细设计与实施计划\.md$|^后续需求TODO\.md$'
# 预期: 0 行（仅 logs/checkpoints + 3 主文档 = 全部 doc；无产品代码）
```

### 5.5 M5-0 顶部时间戳链命中

```bash
grep -cE "W8 拣入：2026-09-07 14:30 CST|W9 active：2026-09-07 14:30 CST" logs/checkpoints/M5-20260906/M5-0-overview.md
# 预期: 2（2 行时间戳追加）
```

### 5.6 M5-0 [W8 reconciliation] 段 + [W9 active] 段存在

```bash
grep -cE "## \[W8 reconciliation · 2026-09-07 13:30 CST\]|## \[W9 active · 2026-09-07 14:30 CST\]" logs/checkpoints/M5-20260906/M5-0-overview.md
# 预期: 2（2 段新增）
```

### 5.7 M5-9/10/11/12 头部 W8 reconciliation + W9 ACTIVE 标注命中

```bash
for f in M5-9-graph-ui-agent-consume M5-10-plugin-manifest-lifecycle M5-11-plugin-commands-isolation M5-12-plugin-ui; do
  echo "$f: W8 reconciliation $(grep -cE 'W8 reconciliation（2026-09-07 14:30 CST · A0 拣入）' logs/checkpoints/M5-20260906/$f.md) | W9 ACTIVE $(grep -cE '\*\*W9\*\* ACTIVE' logs/checkpoints/M5-20260906/$f.md)"
done
# 预期: 每张子卡 W8 reconciliation 1 + W9 ACTIVE 1
```

### 5.8 M5-13 [W8 reconciliation] 段 + [W9 verification scope] 段存在

```bash
grep -cE "## \[W8 reconciliation · 2026-09-07 14:30 CST\]|## \[W9 verification scope · 2026-09-07 14:30 CST\]" logs/checkpoints/M5-20260906/M5-13-verification-matrix.md
# 预期: 2（2 段新增）
```

### 5.9 M5-13 [W9 verification scope] 段含 14 FAC 矩阵

```bash
grep -cE "FAC-1|FAC-1.b|FAC-2|FAC-3|FAC-4|FAC-5|FAC-6|FAC-7|FAC-8|FAC-9|FAC-10|FAC-11|FAC-12|FAC-13|FAC-14" logs/checkpoints/M5-20260906/M5-13-verification-matrix.md
# 预期: ≥14（14 FAC 编号命中）
```

### 5.10 M5-14 顶部 W8 reconciliation + W9 ACTIVE 行存在

```bash
grep -cE "W8 reconciliation（2026-09-07 14:30 CST · A0 拣入）|\*\*W9\*\* ACTIVE" logs/checkpoints/M5-20260906/M5-14-debt-ledger.md
# 预期: ≥2（W8 reconciliation 1 + W9 ACTIVE 1）
```

### 5.11 3 主文档 A1 W9 update 行存在

```bash
for f in AI-模型切换与接手清单.md 详细设计与实施计划.md 后续需求TODO.md; do
  echo "$f: A1 W9 update $(grep -cE 'A1 update 2026-09-07 14:30 CST' $f)"
done
# 预期: 每份文档 1（A1 W9 update 行 = 14:30 CST）
```

### 5.12 patch 文件检查

```bash
ls -la logs/checkpoints/Lane-A1-M5-W9-reconciliation-20260907-1430.patch
# 预期: 仅含 A1 W9 触及 11 文件 + 1 new checkpoint（untracked 不会进 diff）
git apply --check --reverse logs/checkpoints/Lane-A1-M5-W9-reconciliation-20260907-1430.patch
# 预期: 0 输出（PASS，patch 与工作树改动一致）
```

---

## 6. 关键决策摘要（A1 W9 立场，留待 A0 拣入时审视）

1. **A1 W9 整包独立交付（不复用 A1 W8 patch）**。理由：A1 W8 整包已被 A0 拣入 master（4d7be97 stat = 53 files +6038 -101 含 A1 W7 + A1 W8 patch/checkpoint 共 4 文件），A1 W9 整包不应再用 A1 W8 patch 二次 apply；A1 W9 patch = `git diff --binary` 仅含 A1 W9 11 文件改动 + 1 new checkpoint，独立且自包含。
2. **3 份主文档 L1 加 A1 W9 update 行（不替换 A0 W8 update 行）**。理由：A0 W8 update 行（14:30 CST "W8 focused validation passed"）描述 A0 拣入事实，**不**应被 A1 W9 update 行覆盖；A1 W9 update 行描述 A1 W9 reconciliation 整包工作（11 文件改动 + 1 new checkpoint），作为 A0 拣入期可识别 A1 W9 整包的标记。
3. **M5-0 顶部 history 链追加 W8 拣入 14:30 CST + W9 active 14:30 CST 两行**（不重写原有 W8 active 09:45 CST 行）。理由：保留 W7 reconciliation / W8 active / W8 reconciliation / W9 active 时间链完整，W8 拣入事实回填与 W9 active 标记并列。
4. **M5-9/10/11/12 头部状态行 W8 ACTIVE → W8 reconciliation + W9 ACTIVE（追加两行）**。理由：与 M5-0/M5-13/M5-14 同步，4 张子卡头部状态行升级模式与 W7 reconciliation 同步。
5. **M5-13 [W9 verification scope] 段含 14 FAC 矩阵（FAC-1~12 沿用 + FAC-13 build metrics 22% + FAC-14 A11 final verification matrix）**。理由：W9 派发强调"build metrics threshold 22%"与"A11 W9 final verification matrix"，必须新增 2 项 W9 必检 FAC；FAC-1~12 沿用 [W8 verification scope] 段定义（不重写，仅加 W9 final 状态列）。
6. **M5-13 [W9 verification scope] 段 W9 FAIL_ACTION 8 条红线**。理由：W8 FAIL_ACTION 11 项保留 + W9 新增 5 项 = ① build metrics 22% 阈值回归 ② cargo_warnings delta > 0 ③ A3/A5 W9 实施期引入 mcp_* runtime / rmcp server ④ A7/A8 W9 实施期引入后端 graph command / graph runtime ⑤ FAC-1.b M5-1.b seam DEBT 未消；沿用 W8 的 ⑥ ⑦ ⑧（plugin runtime / security runtime / Token secret 落 audit）。
7. **A0 拣入期合并策略同 W8（A1 W8 整包 + A1 W9 整包合并拣入）**。理由：避免 A0 拣入期分两次消；A1 W8 已拣入 4d7be97 → A1 W9 patch 应在 4d7be97 之后 apply（git apply --3way 兼容或 A0 用 cherry-pick 模式）。
8. **A1 W9 不写 mcp_*/rmcp/plugin install·enable·delete·download/skill execution/model calls 任何产品代码**。理由：PARALLEL_COMMAND_BOARD L211- W9 hard stops 8 条红线；A1 = DOCS ONLY lane，**无** W9 实施责任。
9. **A1 W9 不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json / plugin.rs / mcp.rs / bridge.rs / main.rs / bridge.ts / types.ts / check-*-policy.py**。理由：W8 hard stop 1 沿用至 W9；A1 = DOCS ONLY lane，**无** W9 实施责任；任何 .rs/.toml/.json/.vue/.ts 源文件改动属其他 lane W9 实施期责任。

---

## 7. A0 拣入期复核清单（A0 必逐条勾对）

1. [ ] **A1 W9 整包 = 1 patch + 1 checkpoint + 11 文件改动**（7 M5-*.md + 3 主文档 + 1 new checkpoint）。
2. [ ] **M5-0 头部标题加 W8 reconciliation + W9 active**。
3. [ ] **M5-0 顶部 history 链追加 W8 拣入 14:30 CST + W9 active 14:30 CST 两行**。
4. [ ] **M5-0 [W8 reconciliation · 2026-09-07 13:30 CST] 段完整**（4 commit 拣入事实 + 13 项 W8 reconciliation 自检 PASS + 7 项 W8 残留债挂账）。
5. [ ] **M5-0 [W9 active · 2026-09-07 14:30 CST] 段完整**（11 lane 全部活跃 + W9 角色表 11 项 + W9 hard stops 8 条 + A1 W9 整包交付索引）。
6. [ ] **M5-9/10/11/12 头部 L9 末尾追加 W8 reconciliation + W9 ACTIVE 标注**。
7. [ ] **M5-13 L1 标题加 W8 reconciliation + W9 verification scope**。
8. [ ] **M5-13 L6 后追加 W8 reconciliation 状态行 + W9 ACTIVE 状态行**。
9. [ ] **M5-13 [W8 reconciliation · 2026-09-07 14:30 CST] 段完整**。
10. [ ] **M5-13 [W9 verification scope · 2026-09-07 14:30 CST] 段含 14 FAC 矩阵 + W9 FAIL_ACTION 8 条红线 + A11 W9 delta 必标字段 9 项**。
11. [ ] **M5-14 L2 标题加 W8 reconciliation + W9 active**。
12. [ ] **M5-14 L5 后追加 W8 reconciliation 状态行 + W9 ACTIVE 状态行**。
13. [ ] **3 份主文档 L1 A0 W8 update 行后追加 A1 W9 update 行**（不替换 A0 W8 update 行）。
14. [ ] **A1 W9 patch 仅含 A1 W9 触及 11 文件（不含 A1 W8 patch / A1 W7 patch）**。
15. [ ] **A1 W9 patch = `git diff --binary` 输出，`git apply --check --reverse` PASS**。
16. [ ] **A1 W9 整包工作树 0 行产品代码**（仅 11 文件改动 = 7 M5-*.md + 3 主文档 + 1 new checkpoint）。
17. [ ] **A1 W9 整包不 `git add` / `git commit` / `git push`**。
18. [ ] **A1 W9 整包不触及 ACL / Capability / pre-merge.sh / Cargo.toml / package.json / 任何 .rs/.toml/.json/.vue/.ts 源文件**。
19. [ ] **A0 拣入期合并策略同 W8（A1 W8 整包 + A1 W9 整包合并拣入）**。
20. [ ] **A1 W9 整包不重写 §1~§11 决策史（仅头部状态行 + [W8 reconciliation] + [W9 active] 段 + 顶部 history 链追加）**。

---

## 8. A1 W9 自证（连续性 + 决策自证 + 硬停止自证）

### 8.1 范围自证

- **A1 W9 整包触及文件清单**（11 文件 + 1 new checkpoint）：
  1. `logs/checkpoints/M5-20260906/M5-0-overview.md`（M：+158 / -0 · 头部 + 时间戳链 + [W8 reconciliation] 段 + [W9 active] 段）
  2. `logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md`（M：+4 / -0 · 头部 L9 末尾追加 W8 reconciliation + W9 ACTIVE）
  3. `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md`（M：+4 / -0 · 同上）
  4. `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md`（M：+4 / -0 · 同上）
  5. `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md`（M：+4 / -0 · 同上）
  6. `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md`（M：+79 / -0 · 头部 + L6 后状态行 + [W8 reconciliation] 段 + [W9 verification scope] 段）
  7. `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md`（M：+4 / -0 · 头部 L2 标题 + L5 后状态行）
  8. `AI-模型切换与接手清单.md`（M：+1 / -0 · L1 A1 W9 update 行）
  9. `详细设计与实施计划.md`（M：+1 / -0 · L1 A1 W9 update 行）
  10. `后续需求TODO.md`（M：+1 / -0 · L1 A1 W9 update 行）
  11. `logs/checkpoints/A1-M5-W9-reconciliation-20260907-1430.md`（NEW · 本 checkpoint）
- **A1 W9 整包产品代码 touches = 0**（无 Rust/TS/Vue/Cargo.toml/package.json 改动）。
- **A1 W9 整包 mcp_*/rmcp/plugin 改动 = 0**（无 mcp.rs / plugin.rs / bridge.rs / main.rs / check-mcp-policy.py / check-plugin-policy.py 改动）。

### 8.2 连续性自证

- **W7 reconciliation 整包**（A1 W7 9 文件 = +292 / -15）→ A0 W7 拣入期未消（DEBT-42 挂账）
- **W8 reconciliation 整包**（A1 W8 11 文件 = +488 / -15）→ A0 W8 拣入期已合并拣入（4d7be97 stat = 53 files +6038 -101 含 A1 W7 + A1 W8 = 9 + 11 = 20 文件）
- **W9 reconciliation 整包**（A1 W9 11 文件 = +260 / -0，本 checkpoint 另算）→ 留待 A0 W9 拣入期消
- **连续性 = 5 wave 整包连续交付**（W3 reconciliation + W4 active + W5 reconciliation + W6 reconciliation + W7 reconciliation + W8 active + W8 reconciliation + W9 active = 8 wave 状态 = 5 整包连续 = 0 中断）

### 8.3 硬停止自证

| 硬约束 | 自证结果 |
|--------|----------|
| 不写产品代码 | ✅ §8.1 已自证（0 行 Rust/TS/Python/Vue/Cargo/package.json）|
| W9 mcp_*/rmcp/server/listener/network 零触碰 | ✅ §8.1 已自证（无 mcp.rs / plugin.rs 改动）|
| W9 build metrics 22% 阈值不回归 | ✅ A1 W9 整包不触及 .rs/.toml 文件，build metrics 维持 21.07% ≤ 22% PASS |
| 不写 mcp_* / rmcp / plugin runtime / skill execution / network / model calls 代码 | ✅ A1 W9 整包不涉及 |
| 不记录 token/cookie/Authorization/body/prompt-secret | ✅ A1 W9 整包不涉及任何敏感凭据 |
| 交付 coherent patch/checkpoint 不求 A0 分批 | ✅ 1 checkpoint + 1 patch（11 文件改动 + 1 new checkpoint，独立且自包含）|
| 不 push | ✅ 工作树保留修改，未 `git add` / `git commit` / `git push` |
| 不动 ACL / pre-merge.sh / Cargo.toml / package.json | ✅ A1 W9 整包不触及 |
| 不移动 `NEXT` | ✅ A0 W8 update 行 "NEXT=M5-W9" 维持 |
| 不重写各卡 §1~§11 决策史 | ✅ 仅头部状态行 + [W8 reconciliation] / [W9 active] / [W9 verification scope] 段 + 顶部 history 链追加 |
| 不抢 A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 各自工作区 | ✅ A1 W9 仅在 M5-* 横切卡写顶部段 + 3 主文档各加 1 行 W9 update 行 |

### 8.4 决策自证

- **A0 W8 拣入 4 commit 事实**：`4d7be97` 整合 A1 W7+W8 整包 20 文件 + `94e763e` 关闭 MCP policy phase debt + `a840fcb` 关闭 A11 W7 3 red lights + `97118d6` 规范化 patch 空白 → A1 W9 整包必回填 4 commit 事实到 M5-0/M5-13 顶部 history 链 + [W8 reconciliation] 段。
- **A0 14:30 W8 update 行**（3 主文档 + M5-14 顶部）：A0 已在 W8 拣入期同步加 L1 update 行；A1 W9 整包**不**覆盖 A0 update 行，**仅**追加 A1 W9 update 行描述 A1 W9 整包工作。
- **W9 build metrics 阈值 22% 维持**（A0 W8 拣入期抬 19% → 22%）：A1 W9 整包不触及 .rs/.toml 文件，build metrics 维持 21.07% ≤ 22% PASS；A0 W9 拣入期重采后阈值仍 22%。
- **A3 W8 HOLD 解除**（94e763e 落地）：A3 W9 = POLICY/REVIEW ONLY（不实施 rmcp/server/listener/network）；A1 W9 整包不替 A3 解决 M5-2.b 卡预备（归 A3 W9 实施期）。
- **M5-1.b seam DEBT-03 收口**（A2 W9 review 必填）：A1 W9 整包在 M5-13 [W9 verification scope] 段 FAC-1.b 标注 DEBT 状态（沿用 W8），并新增 W9 FAIL_ACTION ⑤ "FAC-1.b M5-1.b seam DEBT 未消 → 阻断合入"。

---

## 9. 整包交付结束（A1 W9 → A0 W9 拣入）

**DELIVERABLE 清单**：

1. **checkpoint**：`logs/checkpoints/A1-M5-W9-reconciliation-20260907-1430.md`（本文件 · §0-§9 完整）
2. **patch**：`logs/checkpoints/Lane-A1-M5-W9-reconciliation-20260907-1430.patch`（本轮生成 · `git diff --binary` 输出 · 覆盖 11 文件 = 7 M5-*.md + 3 主文档 + 1 checkpoint）
3. **整包合并策略**：A1 W9 整包**独立交付**（不复用 A1 W8 patch），A0 拣入期合并策略同 W8 = 4d7be97 后 cherry-pick / git apply --3way 兼容。
4. **A1 W9 拣入责任全部归 A0**：
   - **A0 拣入期必消**：① A1 W9 整包 11 文件改动 + 1 new checkpoint apply；② M5-0 头部 history 链 W8 拣入 14:30 CST + W9 active 14:30 CST 两行追加；③ M5-0 [W8 reconciliation] 段 + [W9 active] 段两段新增；④ M5-9/10/11/12 头部 W8 reconciliation + W9 ACTIVE 两行追加；⑤ M5-13 头部 + L6 后状态行 + [W8 reconciliation] 段 + [W9 verification scope] 段；⑥ M5-14 顶部 W8 reconciliation + W9 active 两行；⑦ 3 份主文档 L1 A1 W9 update 行；⑧ 14 FAC 矩阵 W9 final verification 必跑。
   - **A2 W9 实施期必填**：DEBT-43（DRY-F1 残留扩大 = SENSITIVE_KEY_NAMES 双份 + 图谱容量常量三处拷贝）→ A2 W9 review note + A2/A1 W9 抽 `domain.rs` 单源
   - **A3 W9 实施期必填**：MCP policy current-phase green 复审 + M5-2.b 卡预备 plan（**不**实施 rmcp/server/listener/network）
   - **A4 W9 实施期必填**：A5 `CredentialLeak` redaction 后 secret echo 复审 + DEBT-32 残留债收口
   - **A5 W9 实施期必填**：Agent/Skill read-only bridge hardening 收口（redacted validation errors + frontend parse/permission preview 边界 case 测试）
   - **A6 W9 实施期必填**：Agent/Skill 面板消费磨光（确定性 empty/error/loading + 无 secret 文本 echo + bounded preview 渲染）
   - **A7 W9 实施期必填**：图谱桥契约终稿（runtime-free 与 blocked backend runtime 分离）
   - **A8 W9 实施期必填**：图谱 UI 磨光（filter/search/layout 确定性 + 保留 bounded arrays + 改进 no-backend/read-only 状态）
   - **A9 W9 实施期必填**：plugin surface 复审（manifest/lifecycle 维持 pure + 确认 install/enable/delete/download 仍缺）
   - **A10 W9 实施期必填**：批量复审 W8/W9 outputs（source check / ACL parity / read-only / redaction / 无 runtime 扩张）
   - **A11 W9 实施期必填**：W9 final verification matrix（命令结果 + build metrics 21.07% ≤ 22% + cargo warnings unchanged + pre-merge result + 残留债 + push readiness）

5. **A0 拣入期复核清单**：见本 checkpoint §7（20 条逐项勾对）
6. **A1 W9 自证**：见本 checkpoint §8（4 节：范围/连续性/硬停止/决策）

**A1 W9 整包状态**：

```
LANE=A1
STATUS=PASS（待 A0 W9 拣入后定）
BASE=97118d6（origin/master HEAD）
HEAD=工作树（11 文件改动 + 1 new checkpoint；未提交）
FILES=
  M logs/checkpoints/M5-20260906/M5-0-overview.md              (+158)
  M logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md (+4)
  M logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md (+4)
  M logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md (+4)
  M logs/checkpoints/M5-20260906/M5-12-plugin-ui.md            (+4)
  M logs/checkpoints/M5-20260906/M5-13-verification-matrix.md  (+79)
  M logs/checkpoints/M5-20260906/M5-14-debt-ledger.md          (+4)
  M AI-模型切换与接手清单.md                                    (+1)
  M 详细设计与实施计划.md                                       (+1)
  M 后续需求TODO.md                                            (+1)
  A logs/checkpoints/A1-M5-W9-reconciliation-20260907-1430.md  (NEW)
VERIFY=
  §5 验证命令（git diff --stat 11 文件改动 + git diff --check 干净 + A1 W9 product-code touches = 0 + 顶部 history 链 grep 命中 + 2 段 [W8 reconciliation]/[W9 active] grep 命中 + 4 张子卡头部 grep 命中 + M5-13 2 段 grep 命中 + M5-14 状态行 grep 命中 + 3 主文档 A1 W9 update 行 grep 命中 + patch reverse check）
CHECKPOINT=
  logs/checkpoints/A1-M5-W9-reconciliation-20260907-1430.md（本文件）
MERGE_NOTES=
  - A1 W9 整包独立交付（不复用 A1 W8 patch）
  - A0 W9 拣入期必先消 A1 W9 整包 11 文件改动 + 1 new checkpoint apply
  - A1 W8 已被 A0 拣入 4d7be97 → A1 W9 patch 应在 4d7be97 之后 apply
  - A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 W9 实施期必填（11 lane 全部活跃）
  - A11 W9 必填 W9 final verification matrix（14 FAC）
NEXT=M5-W9 实施期承接 A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 + A1 W9 reconciliation 整包待 A0 拣入
```

**A1 W9 整包交付结束。**
