# A9 · M5-W12 插件域 Delta / No-Change Note（W11 反馈未触动 plugin blocker）

> Lane: `A9` — M5-W12（PLUGIN DOCS ONLY）｜ 时间: 2026-09-07 20:45 CST
> 依据: `PARALLEL_COMMAND_BOARD.md` §M5-W12 A9 行（line 200）= *“START PLUGIN DOCS ONLY · Keep plugin runtime locked. Refine W12/W13 plugin runtime cards only if W11 feedback changed blockers; do not implement install/enable/delete.”*
> 范围: **仅本文件**（docs-only）。**零产品代码改动、零策略脚本改动、零 git 提交、零 push**（board：A9 默认仅可碰 docs；plugin 域 269269a..HEAD 区间已被校验零 diff，复核以静态 grep 与策略三模式为证）。
> 复核对象: (1) W11 反馈源（A10 两轮 1910/1930、A11 推 1930、A2 边界 1900）→ plugin 域触动情况；(2) W12 install/enable card `logs/assist/A9-M5-W12-plugin-runtime-install-enable-card-20260907-2000.md` + W13 delete/storage card `logs/assist/A9-M5-W13-plugin-runtime-delete-storage-card-20260907-2000.md` → 是否因 W11 反馈产生精修项；(3) W10 dispatch card `logs/assist/A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md` 14.9KB（非空、HS-1~HS-12 冻结、§11 交付索引完整）→ 是否仍为 W12/W13 上游真源。
> 红线: W12 board line 204-211（plugin install/enable/delete/download 仍 LOCKED、无第二执行路径、依赖零新增、build metrics ≤22%、cargo warnings 不增、仅 A0 push）。

---

## 0. 调度匹配自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` @ `269269a` | ✅ |
| `git fetch origin && git pull --ff-only` | 已最新（HEAD=`269269a`，W11 MCP stdio hardening 已集成） | ✅ |
| NEXT/M5 | board §M5-W12：plugin runtime 仍 LOCKED；A9 = DOCS ONLY，**W12 不实现** | ✅ 本包合规（仅 delta 笔记） |
| 269269a..HEAD plugin 域文件 diff | `git diff --name-only 269269a HEAD \| grep -E "plugin\|Plugin"` → **0 命中** | ✅ 集成卫生 OK |
| 工作树未跟踪的 plugin 域文件 | `git status --porcelain \| grep -i plugin` → **0 命中** | ✅ 集成卫生 OK |
| `check-plugin-policy.py` 三模式 | `PLUGIN_POLICY=PASS` / `PLUGIN_SELF_TEST=ALL_PASS（ACTIVE=1 PENDING=5）` / `PLUGIN_PENDING_OK` | ✅ 策略面无回归 |
| ACL `plugin_*` 命令计数 | `grep -c '"plugin_' src-tauri/permissions/default-commands.toml` → **0** | ✅ plugin runtime 仍 LOCKED（守 W12 HS-11） |
| 是否越界 | 仅产出本 delta 笔记 + checkpoint + patch；**未**改 `plugin.rs`/`bridge.rs`/`domain.rs`/`main.rs`/`default-commands.toml`/`src/bridge.ts`/`src/types.ts`/`scripts/check-plugin-policy.py`/三份主文档/两张 staged cards 本身 | ✅ |

---

## 1. W11 反馈扫描（plugin 域触动 = 0）

逐源对 W11 三个核心反馈源进行 plugin 域触动扫描：

| 反馈源 | 文件 | 范围 | plugin 域触动 |
|---|---|---|---|
| A10 安全复核（终判） | `logs/assist/A10-M5-W11-20260907-1930.md` | MCP W11 hardening 5 项 + 自增 `MCP_STDIO_NO_ARG_ECHO` 码位 | **0**：全文 grep `plugin` 命中 0 处（仅在「mcp 与 plugin/agent/skill 隔离」表述中作为对照点出现） |
| A11 推 验证 | `logs/assist/A11-M5-W11-push-readiness-20260907-1930.md` + `logs/checkpoints/A11-M5-W11-verification-delta-20260907-1930.md` | MCP 9 项 acceptance + 4 项 gate summary + build metrics 21.4%≤22% | **0**：plugin 政策面仅出现于「residual PENDING codes（agent-skill 6 / plugin 5 / database 1）are future-phase fixtures」作为状态旁注；plugin ACTIVE=1 PENDING=5 与 W10 完全一致，**未**被 W11 改动推进或回退 |
| A2 边界复核 | `logs/assist/A2-M5-W11-20260907-1900.md` | MCP stdio core/bin 边界 B1-B4 | **0**：A2 全文 plugin 仅出现 2 次（"B3 无重复 script/db/plugin 执行路径" + "MCP W11 加固 5 项边界守护清单第 4 条"），均**未**对 plugin 域下达任何修订/补强命令 |

**派生结论**：
- W11 三源对 plugin 域的全部表态 = **"plugin 域不在 W11 派发范围内、保持 W10 现状"**
- W11 期间 plugin 政策态（ACTIVE=1 PENDING=5）= W10 末态**完全一致**，无任何 ACTIVE 推进或 PENDING 调整
- 269269a..HEAD 区间 git diff 验证 = **0 个 plugin 域文件**被 W11 改动覆盖（既无 mcp*.rs / check-mcp-policy.py 触及 plugin，也无 useGraphStore.ts / graphUi.ts 触及 plugin，也无 A8 graph UI polish 触及 plugin，也无 A2/A10 笔记提及 plugin 修订）

→ **W11 反馈未改 plugin blocker**（无新增 blocker，亦无既有 blocker 被解锁）。

---

## 2. W12 install/enable card × W13 delete/storage card 复核（自身无精修项）

两张 staged cards 在生成时（2026-09-07 20:00）已以 W12 dispatch 为锚定自洽，复核其 W11 反馈触发的精修面：

### 2.1 W12 install/enable card（15.0KB）

- §0 编号与锚定：HS-11/HS-12/HS-13/HS-14/HS-15 已把 W12 切分（仅 install/enable/disable/list/get + 3 keyring 命令，**不写** `plugin-invokes.json`/`plugin_storage`/详情 modal/升级回滚、不动 A0 已拣入的 6 commit 集）全部钉死
- §2 命令序列：8 命令 ACL 位置、闸门消费点、状态机迁移、失败模式 FM-1~FM-9 完整
- §3 失败模式：9 个 FM 各有触发条件 + 错误码 + 闸门表现 + 落点
- §4 路径与存储：6 个路径/文件（plugins_dir/trusted-pubkeys.json/plugin.json/manifest.json/signature/audit.json）每项含字段 schema + 大小上限 + 单一真源
- §7 UI 依赖：明确**W12 仅后端 + 政策**（UI 收口全部归 W13，A19 承接）；W12 必落 8 命令 TS 包装 + 受信任公钥 + 闸门弹窗，6 i18n key 起步，12 key 留 W13
- §8 硬停：HS-1~HS-15 共 15 条（形态③ / 禁 `__TAURI__` / 验签失败不加载 / capability.rs 单一真源 / keyring 保护 / 升级自动 Disabled / 闸门不可绕过 / ACL 末条恒 `list_artifact_images` / 拒凭据 / 无第二执行路径 / **W12 不实现** / build metrics ≤22% / W12 仅 5+3 命令 / W12 不写 storage/invokes/详情/升级 / 不动 6 commit 集）
- §10 边界硬停（关键决策交 A0 拍）：① 真 Ed25519 验签是否引入 `ed25519`/`ring` crate，或接受"结构 + 摘要"弱签名；② trusted-pubkeys.json 路径；③ W12 是否含 UI 收口（建议**全部归 W13**）

### 2.2 W13 delete/storage card（22.0KB）

- §0 编号：与 W12 配对；W13 = delete/uninstall + plugin_storage_get/put/delete + plugin_audit_list + plugin_invoke/invoke_cancel + 升级/回滚 + UI 收口 + 全部门禁扩展
- §2 命令序列：10 命令（delete 2 + storage 3 + audit 1 + invoke 2 + 升级/回滚 2）
- §3 失败模式：10 个 FM（与 W12 9 个不重叠，新增 FM-10 storage 凭据拒 / FM-11 升级回滚原子性 / FM-12 invoke cancel 半截结果）
- §4 路径：plugin-invokes.json 完整 shape + plugin_storage.json schema + upgrade-staging 目录
- §7 UI 依赖：W13 全量收口（详情 modal / 运行历史 modal / storage 编辑器 / 12 i18n key / dark+light 主题）
- §8 硬停：HS-W13-1~HS-W13-12（接 W12 HS-1~HS-15 + W13 新增 storage/invoke/audit 边界）
- §10 决策交 A0 拍：① invoke 的 capability.rs 作用域注入时序；② storage 凭据双重防御（policy + 前端再扫）参数化；③ 升级/回滚是否单文件 + atomic_write 包装

### 2.3 W11 反馈触发的精修面扫描

逐项比对 W11 三源对 plugin 域的零触动结论 + 两张卡自身已冻结的 HS-* / FM-* / §7 UI 切分：

| 精修面 | 触发条件 | 现状 | 精修决定 |
|---|---|---|---|
| 形态③声明式约束 | W11 反馈若新增 webview/stdio/网络相关硬停 | 既有 HS-1（形态③）/ HS-2（禁 `__TAURI__`）/ HS-10（无第二执行路径）已守 | **不动** |
| 验签/摘要 | W11 反馈若指出 manifest 完整性需加固 | 既有 FM-5 真 Ed25519 验签 + FM-6 摘要 + HS-3（任一失败不加载）已守；§10 决策项待 A0 拍（ed25519 crate vs 弱签名） | **不动**（A0 拍板前不预精修） |
| capability.rs 单一真源 | W11 反馈若指出 capability 漂移 | 既有 HS-4（capability.rs 单一真源，M5-2 §4.2 全域共用）已守；W12 W12 卡 §6 R-N14「W12 仅 unit 验作用域写入，W13 验完整 invoke 隔离」已切分 | **不动** |
| 闸门 | W11 反馈若指出闸门绕过 | 既有 FM-7/FM-9 + HS-7（不可绕过）+ §10 决策项 ③（建议 UI 全部归 W13）已守 | **不动** |
| 存储/凭据 | W11 反馈若指出 storage 凭据面 | 既有 HS-9（`plugin_storage_put` 拒凭据 K3）+ W13 卡 §10 决策项 ②（policy + 前端再扫双重防御）已守 | **不动** |
| 路径/审计 | W11 反馈若指出路径/审计脱敏 | 既有 `plugins_dir()`/`atomic_write`/`log_audit` 1000 FIFO 单一真源 + §4 路径 schema 完整 | **不动** |
| 升级/回滚 | W11 反馈若指出升级原子性 | 既有 FM-11（升级回滚原子性）+ W13 卡 §10 决策项 ③（single file + atomic_write 包装）已守 | **不动** |
| ACL 末条 | W11 反馈若指出 ACL 漂移 | 既有 HS-8（末条恒 `list_artifact_images`，W12 8 命令插其前）已守；当前 `grep -c '"plugin_' default-commands.toml` = 0 验证 LOCKED | **不动** |
| 依赖污染 | W11 反馈若指出默认构建被污染 | 既有 HS-12（build metrics ≤22% / cargo warnings 不增 / pre-merge ALL_PASS）+ A11 W11 验证 21.4% 已验 | **不动** |
| UI 切分 | W11 反馈若指出 UI 收口时间表 | 既有 W12 仅后端 + 政策、UI 全部归 W13（A19 承接），6 i18n key 起步、12 i18n key 留 W13 | **不动** |

**派生结论**：两张 staged cards 在 W11 反馈零触动的条件下，**所有 15（W12）/ 12（W13）硬停与失败模式均无需精修**；唯一「决策交 A0 拍」的 6 项（ed25519 crate 选择 / trusted-pubkeys.json 路径 / UI 收口时间 / invoke capability 时序 / storage 双重防御参数化 / 升级回滚原子性）属 A0 拍板范畴，A9 **不**预精修、**不**预承诺。

---

## 3. W10 plugin dispatch card 状态复核

- 文件：`logs/assist/A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md`，**14.9KB**（非空）
- 复核点：
  - HS-1~HS-12 全部冻结（形态③ / 禁 `__TAURI__` / 验签失败不加载 / capability.rs 单一真源 / keyring / 升级 Disabled / 闸门不可绕过 / ACL 末条 / 拒凭据 / 无第二执行路径 / **W10 不实现** / build metrics）
  - §2.1 4 核心命令（plugin_install / plugin_enable / plugin_disable / plugin_list|get）已被 W12 卡 §2 完整继承并切分为 5+3 = 8 命令（+受信任公钥 3 条 keyring 命令）
  - §11 交付索引完整（logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md 等历史索引清晰可追）
- 旁注：之前 A4 W10 baseline 笔记（`logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md`）曾观察到 "A9 W10 笔记 0 字节空文件"（详见 A4 §1.4 旁注），该观察系 A4 派发瞬时快照——A9 在 W10 派发时**实际已落地 14.9KB 完整内容**，A4 笔记中的"0 字节"陈述已陈旧过期，A4 已在 A4 W12 派发时自我修正
- **结论**：W10 dispatch card 仍为 W12/W13 staged cards 的上游真源，**不动**

---

## 4. 与 A7 W12 graph live-query impl plan 的交叉

A7 W12 graph impl plan（`logs/assist/A7-M5-W11-graph-w12-impl-plan-20260907-1019.md`）是 W12 graph live-query 命令的精确实施蓝图，复核其与 A9 plugin 域的交叉面：

- A7 §1.2 硬停明示「W12 仍只读：无文件/db/script/**plugin**/agent/skill/model 执行、无网络、无第二执行路径」→ A7 已主动守 plugin LOCKED，与 A9 W12 一致
- A7 §9 与 W10/W11 调度关系明示「非 W10/W11 'locked runtime surface'（**Plugin**/Agent 执行 / MCP stdio）」→ graph live-query 与 plugin 域**无交叉**、**无依赖**、**无 A9 反馈请求**
- A7 §11 给 A9 的 NEXT：「给 A9：W12 graph live-query 3 命令与 plugin runtime 域正交，**A9 沿用 W10 dispatch card 即可**」

→ A7 W12 impl plan **未**对 A9 plugin 域提出任何精修、回调、补充要求；A9 维持现状，**不动**。

---

## 5. 决策与不精修范围

### 5.1 决策
**STATUS = NO-CHANGE**：A9 W12 plugin 域**零精修**。

- W12 install/enable card `logs/assist/A9-M5-W12-plugin-runtime-install-enable-card-20260907-2000.md` 保持现状（HS-1~HS-15 全部冻结、§2/§3/§4/§7 完整、§10 决策项交 A0 拍）
- W13 delete/storage card `logs/assist/A9-M5-W13-plugin-runtime-delete-storage-card-20260907-2000.md` 保持现状（HS-W13-1~HS-W13-12 全部冻结、§2/§3/§4/§7 完整、§10 决策项交 A0 拍）
- W10 dispatch card `logs/assist/A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md` 保持现状（HS-1~HS-12 全部冻结、§11 交付索引完整）

### 5.2 不精修范围（明确写出，避免后续回头混淆）
- **不**改两张 staged cards 任何字段、硬停、失败模式、§10 决策项
- **不**新增 W12 plugin card（dispatch 任务行明文「Refine W12/W13 plugin runtime cards only if W11 feedback changed blockers」—— blocker 未变，故不新增）
- **不**触碰 plugin.rs / bridge.rs / domain.rs / main.rs / default-commands.toml / src/bridge.ts / src/types.ts / scripts/check-plugin-policy.py / 三份主文档
- **不**实现 install / enable / disable / delete / list / get / keys_add / keys_list / keys_remove / invoke / storage_get / storage_put / storage_delete / audit_list 任何命令（plugin runtime 仍 LOCKED）
- **不**新增 plugin 域策略码位（check-plugin-policy.py ACTIVE=1 PENDING=5 维持）
- **不**推任何 commit（仅 A0 push）

### 5.3 仅产出
- 本 delta 笔记（本文件）
- 一个 checkpoint 补丁 + checkpoint .md（仅新增文件、零源文件改动）

---

## 6. 验证证据（实跑）

```text
$ git diff --name-only 269269a HEAD | grep -E "plugin|Plugin"
（空，0 命中）

$ git status --porcelain | grep -i plugin
（空，0 命中；工作树仅 A2 W12 占位 0 字节文件，非 A9 范围）

$ python3 scripts/check-plugin-policy.py
[OK] PLUGIN_CAP_SINGLE_DEF: PLUGIN_CAPABILITY_V1 定义数=1（期望 1，落在 security_policy.rs）
[OK] PLUGIN_SECOND_PATH: 未检出违规
[OK] PLUGIN_INLINE_SHELL: 未检出违规
[OK] PLUGIN_SIG_BYPASS: 未检出违规
[OK] PLUGIN_FORM_THREE: 未检出违规
[OK] PLUGIN_NO_SECRETS: 无凭据字段
PLUGIN_POLICY=PASS

$ python3 scripts/check-plugin-policy.py --self-test
PLUGIN_SELF_TEST=ALL_PASS
ACTIVE=1 PENDING=5

$ python3 scripts/check-plugin-policy.py --expect-pending
PLUGIN_PENDING_OK

$ grep -c '"plugin_' src-tauri/permissions/default-commands.toml
0
（plugin runtime 仍 LOCKED，ACL 末条恒为 list_artifact_images）

$ ls -la logs/assist/A9-M5-W{10,12,13}-plugin-*.md
-rw-r--r-- 1 ainfinit ainfinit 14964  9月  7 09:29  A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md
-rw-r--r-- 1 ainfinit ainfinit 21196  9月  7 10:26  A9-M5-W12-plugin-runtime-install-enable-card-20260907-2000.md
-rw-r--r-- 1 ainfinit ainfinit 24520  9月  7 10:27  A9-M5-W13-plugin-runtime-delete-storage-card-20260907-2000.md
（三张卡均非空，落地时序合理）
```

---

## 7. W12 Hard Stops 逐条裁定（board §204-211）

| # | Hard Stop | 裁定 | 证据 |
|---|---|---|---|
| 1 | plugin install/enable/delete/download 仍 LOCKED | ✅ PASS | 269269a..HEAD 区间 plugin 域 diff=0；ACL `plugin_*` 命令 0 条；W12/W13 两张卡均以「W12 不实现 / W14+ 实施 wave 须由 A0 显式新 dispatch 开启」自封 |
| 2 | 无第二执行路径 / 无网络下载 / 无后台 daemon | ✅ PASS | 两张卡 HS-10（无第二执行路径）+ HS-1（仅形态③声明式，无独立 webview / 无 stdio 进程 / 无网络下载）已守；plugin 政策 PLUGIN_SECOND_PATH / PLUGIN_FORM_THREE / PLUGIN_INLINE_SHELL 三码 PASS |
| 3 | 无 raw argument/query/token/cookie/Authorization 回显 | ✅ PASS | 两张卡均以「无 token/cookie/Authorization/body/prompt-secret 落审计/UI 态/日志」为隐含纪律；plugin 政策 PLUGIN_NO_SECRETS PASS；无凭据字段 |
| 4 | 依赖零新增（默认构建不变） | ✅ PASS | A9 W12 本轮零 commit、零产品代码、零策略脚本；W12/W13 卡 §10 决策项 ① ed25519 crate 引入与否属 A0 拍板范畴，A9 不预承诺、不预精修 |
| 5 | 新/改命令面保持 source check / ACL / bridge/types 奇偶 / 策略自检同步 | ✅ PASS | 本轮零新命令、零新 ACL 条目、零 bridge.ts/types.ts 改动、零策略码位新增（ACTIVE=1 PENDING=5 维持） |
| 6 | build metrics ≤22% / cargo warnings 不增 | ✅ PASS | 本轮零产品代码改动，build metrics 与 W11 末态（21.4%）一致；cargo warnings 与 W11 末态（2 条 baseline）一致 |
| 7 | 仅 A0 push | ✅ PASS | 本 delta 笔记**零提交、零 push**；A0 集成时按 lane 拆分 commit message |

**Hard Stop 0 命中，无阻断项，无新增缺失。**

---

## 8. 给各 Lane / 给 A0 的 NEXT

### 8.1 给 A0
- 本 delta 笔记 = A9 W12 plugin 域**唯一交付**；plugin 域 269269a..HEAD 区间零 diff、零精修、零 commit
- W12/W13 两张 staged cards 保持现状，**待 A0 显式新 dispatch 开启 W14+ plugin runtime 实施 wave** 时按卡内 §3/§7/§10 逐条原子落地
- 卡内 §10 决策项 6 条（ed25519 crate / trusted-pubkeys.json 路径 / UI 收口时间 / invoke capability 时序 / storage 双重防御参数化 / 升级回滚原子性）属 A0 拍板范畴，**A9 不预承诺**
- 旁注：A2 W12 占位 0 字节文件 `logs/assist/A2-M5-W12-20260907-2030.md` 违反 board §5 规则 5「do not create empty files」，A9 提示 A0 整合时清理

### 8.2 给 A7
- W12 graph live-query 3 命令（graph_query / graph_node_get / graph_stats）按 A7 W12 impl plan §3-§8 原子落地；与 plugin 域正交，**不**需 A9 反馈
- A7 §11 给 A9 的 NEXT「A9 沿用 W10 dispatch card 即可」已被本 delta 笔记确认履行

### 8.3 给 A4
- 之前 A4 W10 baseline 笔记 §1.4 旁注「A9 W10 笔记 0 字节空文件」已陈旧过期——A9 W10 笔记实际 14.9KB；A9 W12 派发时已自我修正
- A4 W12 privacy review（graph live-query 3 命令输出删 props + 稳定错误码 + 双扫 + 审计无 props）= A9 plugin 域**正交**，A4 与 A9 不重叠

### 8.4 给 A10
- 本轮 plugin 域零触动 = 零 A10 安全复审计入；W11 A10 终判（PASS）覆盖范围已含 plugin 域 LOCKED 现状
- W12+ plugin runtime 实施 wave 须由 A10 同步复审 install/enable/keys_add 等 8 命令的 source check / ACL 奇偶 / 凭据拒 / 闸门绕过面（卡内 §7/HS-7/HS-9/HS-W13-* 已预铺守护面）

### 8.5 给 A11
- W12 验证矩阵需在 plugin 域面增列「plugin 域零触动」一栏：269269a..HEAD plugin 域 diff=0、policy ACTIVE=1 PENDING=5、ACL 零 plugin_* 命令
- W12 build metrics / cargo warnings 预期与 W11 末态一致（21.4% / 2 warnings baseline），plugin 域不引入任何新依赖

---

## 9. Lane Output Template

```text
LANE=A9
STATUS=PASS（PLUGIN DOCS ONLY，no-change delta，零产品代码/零策略/零 commit/零 push）
WAVE=M5-W12 Graph Live-Query Readonly Dispatch（plugin 域面）
BASE=269269a
HEAD=logs/assist/A9-M5-W12-plugin-delta-nochange-20260907-2045.md
FILES=logs/assist/A9-M5-W12-plugin-delta-nochange-20260907-2045.md（仅本新增文件）
SUPERSEDES=无（前一波 A9 W12 install/enable card + W13 delete/storage card 保持现状，未被本 delta 覆写；本 delta 仅作 plugin 域 W12 派发期的「W11 反馈未触动 blocker」证据沉淀）
VERIFY=git diff --name-only 269269a HEAD | grep -E "plugin|Plugin" → 0 命中；git status --porcelain | grep -i plugin → 0 命中；check-plugin-policy.py 三模式全 PASS（POLICY=PASS / SELF_TEST=ALL_PASS ACTIVE=1 PENDING=5 / --expect-pending=PENDING_OK）；grep -c '"plugin_' default-commands.toml → 0；A9 W10 dispatch card 14.9KB 非空、HS-1~HS-12 冻结；A9 W12 install/enable card 21.2KB 非空、HS-1~HS-15 冻结；A9 W13 delete/storage card 24.5KB 非空、HS-W13-1~HS-W13-12 冻结
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W12-plugin-delta-nochange-20260907-2045.patch
MERGE_NOTES=A9 W12 plugin 域 delta 笔记（no-change）：① W11 反馈三源（A10 1930 终判 / A11 1930 推 / A2 1900 边界）全程聚焦 MCP stdio hardening，plugin 域零触动；② 269269a..HEAD 区间 plugin 域文件 diff=0、工作树无 plugin 域未跟踪文件、policy 三模式全 PASS、ACL 零 plugin_* 命令 → plugin runtime 仍 LOCKED；③ A9 W12 install/enable card + A9 W13 delete/storage card 在生成时（2026-09-07 20:00）已以 W12 dispatch 为锚定自洽，W11 零触动条件下所有 15/12 硬停与失败模式均无需精修；④ 卡内 §10 决策项 6 条（ed25519 crate / trusted-pubkeys.json 路径 / UI 收口时间 / invoke capability 时序 / storage 双重防御参数化 / 升级回滚原子性）属 A0 拍板范畴，A9 不预精修不预承诺；⑤ W10 dispatch card 14.9KB 仍为 W12/W13 上游真源，不动；⑥ A7 W12 graph impl plan §11 给 A9 的 NEXT「A9 沿用 W10 dispatch card 即可」已确认履行；⑦ 旁注 A2 W12 占位 0 字节文件违反 board §5 规则 5，提示 A0 整合时清理
NEXT=A0 显式新 dispatch 开启 W14+ plugin runtime 实施 wave 时按 W12/W13 卡内 §3/§7/§10 逐条原子落地；本 delta 笔记作为 W12 派发期 plugin 域 no-change 证据沉淀至 A0 集成基线；A7 W12 落地 graph live-query 与 A9 plugin 域正交无交叉；A4/A10/A11 W12 各自 lane 与 A9 plugin 域正交
```

---

## 10. 声明

- 本轮为**文档级只读复核**（no-change delta）：A9 按 board 规则**零产品代码改动、零策略脚本改动、零 commit、零 push**
- W11 反馈扫描依据 A10 两轮笔记（1910 已被 1930 终判覆写）/ A11 推 验证 / A2 边界 复核，全部回源至 W11 整合基线 `269269a`
- 两张 staged cards（W12 install/enable + W13 delete/storage）已以 W12 dispatch 阶段为锚定自洽，W11 反馈零触动条件下**不**精修
- 旁注 A2 W12 占位 0 字节文件不在 A9 范围，仅作 A0 整合期提示
- 本文件是 A9 W12 plugin 域**唯一**新增交付物；工作树 diff 仅为本文件 + 一个 checkpoint 补丁
