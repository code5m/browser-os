# A5 · M5-W5 契约评审：A6 Agent/Skill UI 计划 vs 权威 `AgentDef`/`SkillDef` 契约

> LANE=**A5**（M5-W5 Parallel Dispatch，指挥板 §M5-W5 行 155）：**SUPPORT/REVIEW ONLY** — *Review A6 UI against AgentDef/SkillDef contracts; no product code unless fixing docs only*。
> WAVE=M5-W5（A0 于 `0e76a89` 派发；mainline `1610939` locally == origin/master，`git pull --ff-only` 已是最新，工作树干净）
> 性质：**只读评审 note**，零产品代码、零前端改动、不 push。
> 评审对象：A6 的 UI 数据契约计划（`logs/assist/A6-M5-W4-20260906-1810.md`）+ M5-6 卡（`logs/checkpoints/M5-20260906/M5-6-agent-skill-ui.md`）。
> 权威契约源：A5 W4 落入 `src-tauri/src/domain.rs`（`AgentDef`/`SkillDef`/`SkillExec`/`AclLevel`/`CapabilityRef`/`SkillInput`/`SkillTest`/`AgentDialect`/`A2aConfig`/`PermissionPreview`）+ `security_policy.rs`（`SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1`/校验 fn），已随 `1610939` 集成。
> 本文件结论可 1:1 供 A6 W5 落地 M5-6 时直接采用（§4 给出修正后 TS 镜像）。

---

## 1. 评审形态说明（为何是 plan 评审而非 code 评审）

- `grep -rnE "SkillDef|AgentDef|useAgentStore|check-agent-skill-ui-logic" src/ src-tauri/src/ scripts/` 在全仓**命中 0 条前端引用**；`search_file *agent*skill*` / `*kill*`(components) / `*agent-skill-ui*` 均为 0。
- 即：A6 的 M5-6 UI **产品代码尚未进 `origin/master`**（仅有 A6 W4 的规划 doc + 我 W4 的 Rust 域）。
- 故本评审对照 **A6 已写的 UI 数据契约计划** 与 **权威 Rust DTO（已落地）**，在 A6 开工前拦截契约错位，避免其按错误计划 1:1 落地。
- **Concrete code review 的 unblock 条件**（见 §7）：A6 实际 M5-6 UI 代码（components/agent/*、stores/useAgentStore.ts、agentUi/skillUi.ts、check-agent-skill-ui-logic.mjs、types/bridge 镜像）须先落地（集成或可见 worktree），A5 再做逐行评审。
  → **补充**：评审进行中 A6 的 W5 实际代码（`src/types.ts` 镜像 + `src/utils/agentSkillUi.ts` 纯逻辑层）已落入工作树，故追加 **§10 concrete code review**，结论为**契约合规 PASS**（F1/F2/F3 在代码中已修正，未进入实现）。

---

## 2. 权威契约锚点（grep 实证，W4 已集成 @1610939）

`src-tauri/src/domain.rs` 实际定义（节选，`serde(rename_all="snake_case")`）：

| 类型 | 实际形状（JSON 序列化后） | 行 |
|---|---|---|
| `AclLevel` | `"safe"` / `"confirm"` / `"dangerous"` | 1913-1919 |
| `SkillExec`(tag=`kind`) | `{kind:"script_ref",script_id,params}` / `{kind:"command_ref",command_id,params}` / `{kind:"sequence",steps:[...]}` | 1925-1939 |
| `CapabilityRef` | **`{ "id": "..." }`（结构，非裸 string）** | 1942-1945 |
| `SkillInput` | `{name, required:bool, description}`（仅三字段） | 1947-1955 |
| `SkillTest` | `{name, args: <json>}`（字段是 `args`，非 `expect`） | 1957-1963 |
| `SkillDef` | `{id,version,display_name,description,acl,exec,inputs[],capabilities[],tests[],metadata}` | 1965-1983 |
| `AgentDialect` | `"open_ai_compatible"` / `"external_cli"` / `"custom"` | 1986-1992 |
| `A2aConfig` | `{delegate_to:bool, delegated_from:bool}` | 1994-2001 |
| `AgentDef` | `{id,version,display_name,description,dialect,system_prompt,default_capabilities[],a2a,metadata}` | 2004-2019 |
| `PermissionPreview` | `{gate: AclLevel, capabilities: string[]}`（后端 `permission_preview()` 返回） | 2021-2027 |

`src-tauri/src/security_policy.rs`：
- `SKILL_CAPABILITY_V1: &[&str] = &[]` / `AGENT_CAPABILITY_V1: &[&str] = &[]`（**已存在、当前为空、单一真源、fail-closed**）— `domain.rs:2067,2070`
- `check_skill_capabilities` / `check_agent_capabilities` / `contains_credential_leak` 已实现 — `security_policy.rs:2073+`

验证（本波实跑）：`cargo test --manifest-path src-tauri/Cargo.toml` → **359 passed / 0 failed**（A5 模块 13/13 通过；A4 `agent_memory` 此前 3 例失败已在 `1610939` 合流时修平）；`grep` 证实树中无 A6 UI 代码。

---

## 3. A6 计划 vs 权威契约 —— 核对表与处置

> 本节为**计划级**核对（对照 A6-W4 doc §3 与 M5-6 卡）。A6 实际代码的核对见 **§10**（结论：契约合规，F1/F2/F3 代码已修正）。

| # | 项 | A6 计划（A6-M5-W4 doc §3 / M5-6 卡） | 权威契约（W4 实际） | 结论 | 处置 |
|---|---|---|---|---|---|
| **F1** | `AgentDialect` 取值 | `"openai"\|"anthropic"\|"a2a"` | `open_ai_compatible`/`external_cli`/`custom`（snake_case） | ❌ **完全错位** | A6 必须改为 `"open_ai_compatible"\|"external_cli"\|"custom"`；所有下拉/标签同步改。否则 UI 无法解析后端 `dialect` 且标签全错。 |
| **F2** | `CapabilityRef` 类型 | `export type CapabilityRef = string` | `struct { id: String }` → `{"id":"..."}` | ❌ **结构 vs 裸串** | 改为 `interface CapabilityRef { id: string }`；`capabilities: CapabilityRef[]` 即 `Array<{id}>`。与白名单比对须用 `cap.id` 对照 `SKILL_CAPABILITY_V1: string[]`。 |
| **F3** | `SkillTest` 字段 | `{ name; expect }` | `{ name; args: json }` | ❌ **字段名错** | A6 自标 D-A6-1「A5 落码时定」——现权威为 `args`。改 `expect` → `args: unknown`。 |
| F4 | capability 单源状态 | §7.3「缺失（G4）」 | **已存在但为空 `&[]`** | ⚠ 计划过时 | UI 必须：(a) 不假设非空；(b) 渲染空态「暂无可授权能力」；(c) `grantedCapabilities ⊆ whitelist` 在白名单为空时**全部 deny（fail-closed）**，须向用户明示「当前无能力可授」。 |
| F5 | `SkillInput` 富化字段 | `type/options/pattern/dangerous/...` | 域仅 `name/required/description` | ⚠ 前端扩展可接受 | 仅作 UI 展示/编辑用；回传后端时被 serde 忽略（无 `deny_unknown_fields`）。M5-4.b 编辑命令落地时再对齐。 |
| F6 | `SkillExec` 形状 | `{kind:"script_ref",scriptId,params}` | 一致（tag=`kind`） | ✅ | 保持。 |
| F7 | `AclLevel` 取值 | `"safe"\|"confirm"\|"dangerous"` | 一致 | ✅ | 保持。 |
| F8 | `A2aConfig` 布尔开关 | `{delegateTo, delegatedFrom}: boolean` | 一致 | ✅ | A6 W4 已正确校正（否决 W3 数组模型）。 |
| F9 | `PermissionPreview` | 自建（名称+版本+ACL+capabilities） | 后端 `permission_preview()` 返回 `{gate, capabilities:string[]}` | ✅ 对齐 | UI 可直接消费 `capabilities`（已解析 id 列表）渲染权限预览弹窗。 |
| F10 | `metadata` | `Record<string,unknown>` | `serde_json::Value` | ✅ | 保持。 |

**F1/F2/F3 为阻断级契约错位**：A6 若按现有计划 1:1 落地，前端类型与后端 `AgentDef`/`SkillDef` 反序列化不兼容（dialect 解析失败、capabilities 比较恒错、SkillTest 字段丢失）。

---

## 4. 修正后 TS 镜像（A6 W5 直接采用，覆盖 A6-M5-W4 doc §3.3）

```ts
// 镜像 A5 W4 domain.rs（蛇形经 serde 转驼峰；权威源，勿自行改名）
export type AclLevel = "safe" | "confirm" | "dangerous";

export type SkillExec =
  | { kind: "script_ref"; scriptId: string; params: Record<string, unknown> }
  | { kind: "command_ref"; commandId: string; params: Record<string, unknown> }
  | { kind: "sequence"; steps: SkillExec[] };

// F2 修正：CapabilityRef 是结构，不是裸 string
export interface CapabilityRef { id: string; }

export interface SkillInput {
  name: string;
  required: boolean;
  description: string;
  // 以下为 UI 扩展（后端 SkillInput 仅含上三字段；回传时被忽略）
  type?: "string" | "number" | "bool" | "path" | "enum" | "password";
  options?: { value: string; label: string }[];
  pattern?: string;
  dangerous?: boolean;
  dangerReason?: string;
}

export interface SkillTest {
  name: string;
  args: unknown;        // F3 修正：expect → args（域权威字段）
}

// F1 修正：AgentDialect 实际取值
export type AgentDialect =
  | "open_ai_compatible"   // 域 OpenAiCompatible
  | "external_cli"         // 域 ExternalCli
  | "custom";              // 域 Custom

export interface A2aConfigUI { delegateTo: boolean; delegatedFrom: boolean; }

export interface SkillDef {
  id: string; version: string; displayName: string; description: string;
  acl: AclLevel; exec: SkillExec;
  inputs: SkillInput[]; capabilities: CapabilityRef[];
  tests: SkillTest[]; metadata: Record<string, unknown>;
}
export interface AgentDef {
  id: string; version: string; displayName: string; description: string;
  dialect: AgentDialect; systemPrompt: string;
  defaultCapabilities: CapabilityRef[];
  a2a: A2aConfigUI; metadata: Record<string, unknown>;
}
export interface PermissionPreview {
  gate: AclLevel;
  capabilities: string[];   // 来自后端 permission_preview()
}

// F4：能力白名单（单一真源，当前为空，由后端常量镜像/命令下发）
//   SKILL_CAPABILITY_V1 / AGENT_CAPABILITY_V1: string[]，初为空
//   UI 比较：grantedCapabilities.every(c => whitelist.includes(c.id))
//   白名单为空时所有 granted 均 deny（fail-closed），须渲染空态。
```

---

## 5. 契约就绪确认 + W5 边界守门

- **契约已 UI-ready**：所有字段已枚举、serde 形状确定、`permission_preview()` 可用、能力单源已就位（空）、W4 范围无执行/无 live runtime。A6 可 1:1 落地 TS 镜像 + `useAgentStore` + 面板 shell。
- **W5 Hard Stop 复核**（A6 须遵守，A5 评审关注点）：
  - 不执行 skill、不安装 plugin、不调 live runtime → UI 写操作**全部经 `bridge.ts`**（禁裸 `invoke`，M5-6 卡 FORBID 已定）；流式 `agent://...` 仅做 disabled shell（W5 禁 agent consumption）。
  - 无新后端命令：W5 偏好「无命令」，M5-5 的 15 条命令未落地 → `useAgentStore` 的 `loadSkills/installSkill/runAgent` 等须为**禁用态/占位**，并在 checkpoint 标注单一后端 unblock。
  - 无新依赖：`npm run build` 须 PASS、UI 逻辑测试 `node scripts/check-agent-skill-ui-logic.mjs` 须 PASS（A6 须新建该脚本）。
  - 有界 + 隐私：`params.password` 仅组件本地 `ref`、不进 store/localStorage/审计；`sessions[].chunks` 单会话 ≤64KB 截断标 `truncated`；能力/系统提示不落盘密钥。

---

## 6. 卡片 / 文档不一致（交由 A1/A0 修）

- **F6（卡片陈旧）**：M5-6 卡 §0/§2 写「既有 `src/stores/useAIStore.ts`」（且要求「全读」）——但 `useAIStore.ts` **不存在**（R-A6-10 已实锤）。应改为「`useAgentStore.ts`（待建）」。A5 建议 A1 修卡，避免 A19 实施时误判。
- **F7（路由矛盾）**：M5-6 卡 §3 列 `src/router/agent.ts`（新增 router），但 A6 W3+W4 计划明确「无路由落点，用 `useLayoutStore` 懒加载」。二者冲突；W5 亦强调「panel shell、no live graph UI or agent consumption」。建议 A0/A1/A6 裁决：要么删卡中 router 条目，要么在计划显式说明 router 仅做 `defineAsyncComponent` 注册（非 vue-router 页面）。

---

## 7. 阻塞态 / Unblock 条件

1. **A6 M5-6 UI 代码未进树** → 本评审为 plan 级；**concrete code review 待 A6 落地代码**。
2. **M5-5 15 命令 DTO 未落地** → `src/types.ts`/`bridge.ts` 镜像、`useAgentStore` 写操作均为占位（符合 W5 无命令硬停）。
3. **能力白名单为空** → UI 须以空态呈现（F4），不假设非空。

→ A6 从 M5-6.a 开工时，须以 §4 修正镜像替换 A6-M5-W4 doc §3.3 的 `AgentDialect`/`CapabilityRef`/`SkillTest` 三处，并处理 F4 空白名单；其余（AclLevel/SkillExec/A2aConfig/PermissionPreview）已对齐可直用。

---

## 8. 验证（read-only）

```text
git status --short --branch   → ## master...origin/master（干净，无 A5 改动）
grep -rnE "SkillDef|AgentDef|useAgentStore|check-agent-skill-ui-logic" src/ src-tauri/src/ scripts/ → 0（佐证 A6 UI 代码未进树）
cargo test --manifest-path src-tauri/Cargo.toml  → 359 passed / 0 failed（A5 模块 13/13；域契约稳定）
git diff --check              → 无输出（本文件为新增 untracked，仅文档）
# W5 Hard Stop 全遵守：仅写 logs/assist/，未改 src/ / src-tauri/ / scripts/ / 主文档 / ACL
```

---

## 9. Lane 输出模板（A5 · M5-W5）

```text
LANE=A5
STATUS=PASS_WITH_DEBT          # M5-W5：SUPPORT/REVIEW ONLY；契约评审 note（A6 UI 计划 vs 权威 AgentDef/SkillDef），零产品代码
BASE=1610939                  # A0 push W5 dispatch；git pull --ff-only 已最新
HEAD=logs/assist/A5-M5-W5-agent-skill-ui-contract-review-20260906-1900.md
FILES=logs/assist/A5-M5-W5-agent-skill-ui-contract-review-20260906-1900.md
VERIFY=cargo test 359 passed/0 failed（A5 13/13）；grep 证实树中无 A6 UI 代码；git diff --check 干净；W5 Hard Stop 全遵守
CHECKPOINT=logs/assist/A5-M5-W5-agent-skill-ui-contract-review-20260906-1900.md
MERGE_NOTES=评审拦截 A6 计划 3 处阻断级契约错位（F1 AgentDialect=open_ai_compatible/external_cli/custom；F2 CapabilityRef={id} 非裸 string；F3 SkillTest.args 非 expect）+ F4 空白名单空态；§4 已给修正后 TS 镜像供 A6 直接采用。A6 实际 UI 代码未进 origin/master，concrete code review 待其落地（unblock 见 §7）。另交 A1/A0 修 M5-6 卡两处不一致（F6 useAIStore 不存在；F7 router 与无路由计划矛盾）。
NEXT=A6 落地 M5-6 UI 代码（components/agent/* + useAgentStore + agentUi/skillUi.ts + check-agent-skill-ui-logic.mjs + types/bridge 镜像，须采纳 §4 修正）→ A5 做逐行 code review；同时等 A16/A19 落 M5-4.b/M5-5 命令后解 W5 无命令占位。
```

**NEXT**：等 A6 交付 M5-6 UI 代码（采纳 §4 修正镜像）→ A5 转 concrete code review；等 A16/A19 落 M5-4.b 执行层 / M5-5 命令 → 解 W5「无命令」占位并触发 `bridge.ts`/`check-agent-skill-ui-logic.mjs` 实测。不移动主文档 NEXT、不 push。

---

## 10. 补充：A6 W5 实际代码评审（types.ts 镜像 + agentSkillUi.ts 已落地）

评审进行中，A6 的 W5 实际代码落入工作树（grep 实证），故升级为 concrete code review。

### 10.1 落地物
- `src/types.ts`（+139 行）：`AgentDef`/`SkillDef`/`SkillExec`/`AclLevel`/`CapabilityRef`/`SkillInput`/`SkillTest`/`AgentDialect`/`A2aConfig`/`PermissionPreview` 镜像。
- `src/utils/agentSkillUi.ts`（361 行）：纯逻辑层（校验/展示/脱敏/有界流式/二段式闸门辅助），可被 node 直接 import 断言。

### 10.2 契约合规结论：PASS（F1/F2/F3 在代码中已修正）
| 项 | 计划文档(A6-W4) | 实际代码(types.ts) | 结论 |
|---|---|---|---|
| F1 `AgentDialect` | `openai`/`anthropic`/`a2a` | `open_ai_compatible`/`external_cli`/`custom`（types.ts:690） | ✅ 已修正 |
| F2 `CapabilityRef` | `string` | `{ id: string }`（types.ts:659）+ `agentSkillUi.ts:149` 用 `c.id` | ✅ 已修正 |
| F3 `SkillTest` | `{name, expect}` | `{ name; args: unknown }`（types.ts:671-674） | ✅ 已修正 |
| F4 空白名单 | 计划称缺失 | `renderCapabilityList`/`classifyCapability` 对空 whitelist → 全 `unknown`（fail-closed） | ✅ 已对齐 |
| `AclLevel`/`SkillExec`/`A2aConfig`/`PermissionPreview` | — | 与 domain.rs 一致 | ✅ |

> 说明：A6 实际实现直接读了我 W4 的 `domain.rs` 权威类型，未沿用其 W4 规划 doc 的陈旧取值；故计划级 F1/F2/F3 错位**未进入代码**。`A6-M5-W4-20260906-1810.md` §3.3 应同步修订以免误导后续读者（建议 A6/A0 修）。

### 10.3 边界守门：PASS
- `agentSkillUi.ts` 纯逻辑、无 `invoke`、无 DOM、无 store 依赖；可被 `node` 直接断言（符合 W5「helper module + headless logic test」）。
- 隐私：`redactSecrets`/`serializeSkillForm` 将 secret 键值替换为 `***`，不落盘/不进 store（K3 对齐）。
- 有界：`appendChunk` 单会话缓冲 ≤ `MAX_STREAM_BUFFER_BYTES=65536`，超出丢最旧并记 `droppedBytes`（不增长无界）。
- 闸门：`makePendingConfirm`/`isConfirmExpired` 二段式；`panelState` 在 `backendReady=false` 时返回只读壳文案（W5 无命令占位）。

### 10.4 缺口（交 A6 闭环，非 A5 契约问题）
- **G1（Must Deliver 缺口）**：`scripts/check-agent-skill-ui-logic.mjs` **未落地**。board A6 W5 Must Deliver 明确要求「UI logic test PASS」+ 该脚本；当前 `agentSkillUi.ts` 无回归护栏。A6 须补该 mjs（断言 `aclLabel`/`execSummary`/`renderCapabilityList` 空名单/`redactSecrets`/`appendChunk` 有界等），否则无法机器自证。
- **G2（分阶段）**：`src/components/agent/*`（ChatPanel/SkillManager/AgentManager/PermissionPreviewModal）尚未落地。W5 偏好「helper module + headless logic test」优先，组件可作后续子步；board Must Deliver 列 `npm run build PASS`——当前无组件故 build 不受影响（无回归），全面板 UI 为 follow-up。

### 10.5 最终结论
A5 对 A6 M5-6 的契约评审：**契约合规 PASS**；A6 实际代码正确镜像了我 W4 的 `AgentDef`/`SkillDef` 契约并守住了 W5 边界。剩余两项（G1 缺逻辑测试脚本、G2 组件待补）属 A6 W5 交付收尾，与 A5 无关。A5 不修改任何 `src/` 文件（review-only）。
