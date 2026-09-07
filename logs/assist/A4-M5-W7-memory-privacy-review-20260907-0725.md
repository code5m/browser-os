# Lane A4 · M5-W7 Review Note — A5 命令 payload 与 agent memory 隐私/容量交互；及 W6 A8/A9 落地复核

> Lane=A4 · WAVE=M5-W7 · Status=**SUPPORT/REVIEW ONLY**（看板 §M5-W7：A4 仅评审 A5 命令 payload 与 memory/privacy 交互，无产品代码）
> 主审对象：A5 的 Agent/Skill 只读命令桥 payload（parse/validate/permission_preview 及其 W7 命令包装）
> 跟踪复核：W6 已集成的 A8 图谱 UI（5f92ece）与 A9 插件 manifest/生命周期（R8-1~5 / R9-1~6 逐条核验）
> 边界真相源：① A4 W4 agent memory KV 契约（5 ACTIVE 码）；② A7 W5 图谱契约（7 ACTIVE 码）；③ A5 W4 Agent/Skill 契约（`agent.rs`/`skills.rs`/`domain.rs`/`security_policy.rs`/`check-agent-skill-policy.py`）

## 0. 仓库实况（@ `a26fbaf`，已 `git pull --ff-only`，工作树 clean）

| 项 | 状态 | 证据 |
|---|---|---|
| A4 agent-memory 政策 | **PASS** | `check-agent-memory-policy.py --self-test` → `AGENT_KV_POLICY_SELF_TEST=PASS（ACTIVE=5）`；默认 `AGENT_KV_POLICY=PASS` |
| A7 graph 政策 | **PASS** | `check-graph-policy.py --self-test` → `GRAPH_POLICY_SELF_TEST=PASS（ACTIVE=7）`；默认 `GRAPH_POLICY=PASS` |
| A9 plugin 政策 | **PASS** | `check-plugin-policy.py --self-test` → `PLUGIN_SELF_TEST=ALL_PASS（ACTIVE=1 PENDING=5）`；默认 `[OK] PLUGIN_NO_SECRETS` / `PLUGIN_POLICY=PASS` |
| A5 agent-skill 政策 | **PASS** | `check-agent-skill-policy.py --self-test` → `AGENT_SKILL_POLICY_SELF_TEST=PASS（ACTIVE=2，PENDING=4）`；默认 `AGENT_SKILL_POLICY=PASS` |
| A8 graph UI 逻辑 | **PASS** | `node scripts/check-graph-ui-logic.mjs` → 通过 34，失败 0 |
| A5 命令桥（W7 产品代码） | **未进 mainline** | `bridge.rs`/`main.rs` 无 `agent_`/`skill_` tauri 命令注册；`agent.rs`/`skills.rs` 仅含 `parse/validate/permission_preview` 纯函数（`#[allow(dead_code)]`） |
| A8 图谱 UI / A9 插件 | **已集成（5f92ece）** | `useGraphStore.ts`/`graphUi.ts`/`NodeDetail.vue`/`check-graph-ui-logic.mjs`；`plugin.rs`(446)/`security_policy.rs`(+47)/`check-plugin-policy.py` |

→ 本评审为**两层**：① W7 主审 A5 命令 payload（A5 命令桥代码未进 mainline，故为前瞻式契约审计 + 红线）；② W6 跟踪复核（A8/A9 已落地，逐条 concrete 核验）。

## 1. W7 主审：A5 命令 payload 与 agent memory 隐私/容量交互

### 1.1 隔离性结论（已确认，核心安全属性）
- **A5 域与 agent memory KV 完全隔离**：`AgentDef`/`SkillDef` DTO（`domain.rs:1968-2019`）**无 `agent_kv` 字段、无 agent memory 引用**；`parse/validate/permission_preview` 仅作用于自身 JSON，不调用 `agent_memory.rs`。图谱侧（`domain.rs:2034-2037`）亦明示 Skill/Agent 节点仅按 id 引用、不持有 `agent_kv` 值。→ **无跨 store 泄露路径**。
- **`permission_preview` 不泄露 secret**（`agent.rs:40-49`/`skills.rs:45-50`）：仅返回 `{gate, capabilities:[id]}`——gate 档 + 能力 id 列表，**不含 system_prompt / description / metadata / 任何 body**。
- **凭据闸已就位**（`agent.rs:30-35`/`skills.rs:31-40`）：`validate` 对 `system_prompt`/`description`/`inputs` 跑 `security_policy::contains_credential_leak`（security_policy.rs:2088，覆盖 sk-/api_key/secret/password/authorization/bearer/x-api-key/private_key）。
- **能力白名单 fail-closed**：`AGENT_CAPABILITY_V1`/`SKILL_CAPABILITY_V1`（`security_policy.rs:2082/2085`）当前为空 `&[]`，任何能力声明 → `UnknownCapability` 拒绝。→ **不可能授予 agent_kv 读/写能力**（白名单里无此类项）。

### 1.2 A5 W7 命令 payload 红线（命令桥落地后由 A4 二次复核）
- **R5-1（无 agent_kv 交互）**：W7 命令只能 parse/validate/preview AgentDef/SkillDef；**包装层不得新增 `agent_memory` 调用**。新 `agent_*`/`skill_*` 命令须插在 ACL 末条 K1（`list_artifact_images`）**之前**，不得破坏 K1 末条不变量。
- **R5-2（parse 输入有界）**【本评审主要发现】：`AgentDef::parse(text: &str)` / `SkillDef::parse(text: &str)`（`agent.rs:12`/`skills.rs:17`）**无输入长度上限**，直接 `serde_json::from_str` 于任意 `&str`。W7 命令若把前端传来的任意字符串原样喂入，可被超大 payload OOM。建议：在 domain 或命令层加定值上限 `AGENT_DEF_MAX_BYTES`/`SKILL_DEF_MAX_BYTES`（对齐 `MAX_TEXT_FIELD_BYTES=64KiB` 或更大但须固定），超出即拒。
- **R5-3（响应不泄露 secret）**：`permission_preview` 已合规（见 1.1）。若 W7 命令另含 `validate` 回显完整 def（含 `system_prompt`/`metadata`），须确认①不回显 agent_kv（当前无此字段，天然满足）②前端不在别处持久化该回显。
- **R5-4（审计脱敏）**：若命令写审计（如 parse/validate 调用），**不得序列化完整 system_prompt/definition**（含潜在 prompt-secret）。对齐 `AGENT_KV_AUDIT_NO_VALUE` 精神 + W7 硬停"no prompt-secret logging or persistence"：审计只记 id/结果/能力，不记 body。
- **R5-5（无执行/安装/网络/持久化写）**：命令须 read-only。`agent.rs`/`skills.rs` 头部已声明纯逻辑（不执行/不安装/不联网/不引桥）；命令包装层不得引入 `script_runner` 调用或 agent_kv 写入。
- **R5-6（bounded 响应）**：`metadata: serde_json::Value` 无界；建议命令响应只回 `{ok, errors, preview}` 摘要，或对 metadata 设上限（可借鉴 A9 `plugin.rs:139-142` 的 64KiB 上限做法）。

## 2. W6 跟踪复核：A8 图谱 UI（R8-1~5）/ A9 插件（R9-1~6）已落地核验

### 2.1 A8 图谱 UI（5f92ece：`useGraphStore.ts`/`graphUi.ts`/`NodeDetail.vue`/`check-graph-ui-logic.mjs` 34/34）
- **R8-1 ✓ 容量/消费 bounded**：`useGraphStore.ts:24-25` `MAX_NODES=5000`/`MAX_EDGES=20000`；`boundedInsert`（graphUi.ts:196-210）超上限丢最旧；`guard()`（useGraphStore.ts:67-72）在后端未就绪时**零 invoke**；`estimateCapacity`/`GRAPH_QUERY_LIMIT=1000` 暴露给 UI。
- **R8-2 ✓ 详情不渲染敏感 props（纵深）**：`summarizeNode`（graphUi.ts:74-82）显式 K7「剔除 props 正文，摘要中绝不含 props 字段」，只回 `{id,kind,kindLabel,label,isAgentConsumption}`；`NodeDetail.vue:20-21` 注明「props 已脱敏，界面不展示」。
- **R8-3 ✓ 前端态不持久化 secret**：store 为内存 `ref<Map>`；search `query` 仅组件内 `filter` 局部态；文件头注释明令「不触发 agent 消费、不重建图」（W6 硬停）。
- **R8-4 ✓ 不直读 agent_kv**：store 仅消费 `GraphNode`/`GraphEdge` DTO，无任何 `agent_memory` 调用。
- **R8-5 ✓ 引用完整性**：Skill/Agent 节点仅显 `id+label+kind`（`summarizeNode`），不额外 fetch A5 内部。
- **注（DRY，同 F1）**：容量常量 `5000`/`20000` 在 `useGraphStore.ts:24-25` 与 `graphUi.ts:26-27` 各硬编码一份（注释标注 `= GRAPH_MAX_NODES`）。5f92ece 已加 `src/types.ts`(+44)，建议前端常量统一从 `types.ts` 引入，消除第三/四处拷贝。

### 2.2 A9 插件（5f92ece：`plugin.rs`/`domain.rs`/`security_policy.rs`/`check-plugin-policy.py`）
- **R9-1 ✓ manifest 无 secret + 双扫**：`check-plugin-policy.py` → `[OK] PLUGIN_NO_SECRETS`；`validate_plugin_manifest` 经 `bound_text`（plugin.rs:231-236，长度上限 + `contains_credential_leak`）扫 `description`/`display_name`/`version`/`min_app_version`；`metadata` 限 `MAX_TEXT_FIELD_BYTES=64KiB`（plugin.rs:139-142）。
- **R9-2 ✓ registry bounded**：纯状态机 `can_transition`/`transition`（无 map/list 持久化）；manifest 自身字段有边界，metadata 有 64KiB 上限。
- **R9-3 ✓ 不跨 store / 不读 agent_kv**：grep `plugin.rs` 仅 `agent_memory` 出现在文档注释，**无任何 `agent_memory`/`agent_kv` 调用**；`PLUGIN_CAPABILITY_V1`（security_policy.rs:2126）空集合 fail-closed → 不可能授予 agent_kv 读/写；`entry_url` 限同源 webview（`http(s):`/`tool:`），无路径穿越进 agent memory 存储目录。
- **R9-4 ✓ 无 install/exec/网络**：`plugin.rs` 头部明令纯函数（无文件 I/O、无真验签、不联网）；`DANGEROUS_PLUGIN_CAPABILITIES`（plugin.rs:28-36）含 `credential.read` 但仅作风险分类，实际白名单空 → 全拒；无第二执行路径。
- **R9-5 N/A**：W6/W7 均未加 plugin 命令；若未来加，须 source check+ACL+bridge/types+政策+测试同包（看板硬停）。
- **R9-6 ✓ 审计脱敏**：文档明令「无凭据/secret 进入 manifest 或审计」（plugin.rs:16）；manifest 结构无 secret 字段可泄。
- **结论**：A8/A9 W6 全部红线在落地代码中满足（除 F1 常量重复为 DRY 建议），A4 对 W6 交付给 **PASS**。

## 3. 系统性发现

- **F1（DRY，承 W6 残项并扩大）**：敏感常量 `SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS` 仍双份于 `agent_memory.rs:134` 与 `graph.rs:18`；现新增同类扩散：图谱容量常量 `5000`/`20000`/`1000`/`4` 出现在 `domain.rs` + `useGraphStore.ts:24-25` + `graphUi.ts:26-29`（三处）。建议：① `domain.rs` 增加单源 `pub const SENSITIVE_*` 供 agent_memory/graph 复用；② 前端常量统一从 `src/types.ts`（5f92ece 已加）引入，删除硬编码副本。与 M5-W2「常量集中」原则一致。
- **F2（W7 主发现 = R5-2）**：A5 `parse` 无输入上限，待 W7 命令落地时强制 `AGENT_DEF_MAX_BYTES`/`SKILL_DEF_MAX_BYTES`。

## 4. 解锁后二次复核清单（A5 命令桥进 mainline 后由 A4 执行）

- [ ] A5 命令是否只注册 parse/validate/permission_preview，且**未新增 agent_memory 调用**（R5-1）。
- [ ] 新 `agent_*`/`skill_*` 命令是否插在 ACL 末条 K1（`list_artifact_images`）**之前**（R5-1）。
- [ ] 命令层是否对 parse 输入做**字节上限**校验（R5-2 / F2）。
- [ ] 命令响应是否不回显完整 system_prompt/metadata 或**审计不序列化 body**（R5-3/R5-4）。
- [ ] 命令是否 read-only（无 exec/install/network/agent_kv 写）（R5-5）。
- [ ] 响应 metadata 是否 bounded（R5-6）。
- [ ] 交叉：A5 命令桥是否复用 `check-agent-skill-policy.py`（含隐私双扫 + 能力单源 + 无第二执行路径）作门禁（A10 同审）。

## 5. 与 W7 Hard Stops 对齐

- "Only A3 and A5 may write product code in W7" → 本评审**零产品代码**，符合。
- "W7 commands are read-only only: no skill execution, no plugin install…" → A5 域逻辑本就纯函数（R5-5 守）；A9 插件亦纯策略（R9-4 守）。
- "No token/cookie/Authorization/body/prompt-secret logging or persistence" → R5-4（A5 审计不序列化 body）+ R8-3（graph UI 不持久化）+ R9-6（plugin 无 secret）共同覆盖。
- "All lanes pull from origin/master first and must not push" → 已 `git pull --ff-only`，未 push。

## 6. LANE 输出模板

```text
LANE=A4
STATUS=PASS（REVIEW ONLY，无产品代码）
BASE=a26fbaf
HEAD=logs/assist/A4-M5-W7-memory-privacy-review-20260907-0725.md
FILES=logs/assist/A4-M5-W7-memory-privacy-review-20260907-0725.md
VERIFY=git status clean @a26fbaf；check-agent-memory-policy.py --self-test PASS(ACTIVE=5)+默认 PASS；check-graph-policy.py --self-test PASS(ACTIVE=7)+默认 PASS；check-plugin-policy.py --self-test ALL_PASS(ACTIVE=1 PENDING=5)+默认 [OK] PLUGIN_NO_SECRETS；check-agent-skill-policy.py --self-test PASS(ACTIVE=2 PENDING=4)+默认 PASS；check-graph-ui-logic.mjs 34/34；A5 W7 命令桥未进 mainline（桥接层待落地）；A8 图谱 UI/A9 插件已集成(5f92ece)
CHECKPOINT=logs/assist/A4-M5-W7-memory-privacy-review-20260907-0725.md
MERGE_NOTES=本 Lane W7 仅评审，无冲突；W7 主审 A5 命令 payload：确认与 agent memory KV 完全隔离、permission_preview 不泄 secret、能力白名单空 fail-closed；主要发现 R5-2(parse 输入无界) + 残项 F1(常量重复)；W6 跟踪复核 A8 图谱 UI(R8-1~5 全 ✓) 与 A9 插件(R9-1~6 全 ✓，R9-5 N/A)，A4 对 W6 交付给 PASS。边界真相源=A4 W4 5 码 + A7 W5 7 码 + A5 W4 Agent/Skill 契约
NEXT=待 A5 命令桥落地后按 §4 清单二次 concrete review；F1/F2 可建议 A0 套用补丁（常量单源化）
```
