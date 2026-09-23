# FINAL-INDEPENDENT-RED-TEAM（PHASE F-B）

> 假设：**「这套 Capability Platform 只是包装得很漂亮的单体应用。」**
> 主动寻找证据证明它；找不到才允许判 PASS。每项给出 STATUS / EVIDENCE / FILES / CHECKER / RISK / ACTION。
> **不写「已有 gate，所以 PASS」** —— 必须落到真实文件与可复跑命令。

**汇总：PASS = 20，FINDING = 5，BLOCKER = 0**

---

## RT-01 Capability 是否只是目录搬家？ — **PASS**
- EVIDENCE：能力含 `manifest/public/state/ui/index` + **贡献**；`MainArea` 用通用 `viewOf(view)` 遍历 `WORKBENCH_MAIN` 渲染，
  **无**任何能力面板静态 import。vault/home/settings 三个迁移均未改 MainArea 业务分支即可渲染。
- FILES：`src/components/layout/MainArea.vue`、`src/capabilities/{vault,home,settings}/index.ts`
- CHECKER：`check-capability-composition`（33/33）、`check-composition-profiles`（11/11）
- RISK：低 / ACTION：保持

## RT-02 public.ts 是否只是重新导出 internal store？ — **PASS**
- EVIDENCE：`home/public.ts` 是**窄意图**（`favoriteDirectory/favoriteCurrentPage/favoriteCurrentDir` 包装器，不导出 store）；
  `browser/public.ts`、`clipboard/public.ts` 为**受控出口**（显式 re-export，注释声明不持有状态）——属既定「controlled exit」模式，非万能 facade。
- FILES：`src/capabilities/home/public.ts`、`src/capabilities/browser/public.ts`
- CHECKER：`check-capability-platform` PLT2-16（public contract locator 指向真实代码）
- RISK：低 / ACTION：保持；新增 public 不得导出整个 store

## RT-03 Capability state owner 是否仍在旧 global store？ — **PASS**
- EVIDENCE：vault→`capabilities/vault/state`、home→`capabilities/home/state`；`semantic-registry/states.yaml`
  `owner_implementations` 路径已同步（useVaultStore/useHomeStore/useScriptStore）。
- FILES：`docs/architecture/semantic-registry/states.yaml`
- CHECKER：`check-semantic-registry`（PASS，fail=0）
- RISK：低 / ACTION：后续迁移须同步 locator（RT-23 已验证）

## RT-04 Shell 是否仍直接控制 Capability lifecycle？ — **PASS**
- EVIDENCE：GOV-01/02 —— ResourceGovernor **不 import 任何能力 owner/store/UI**，只经 runtime 公共 API。
- FILES：`src/capability/platform/*`、`scripts/check-composition-profiles.mjs`
- CHECKER：`check-composition-profiles` GOV-01/02 PASS
- RISK：低 / ACTION：保持

## RT-05 Shell 是否仍直接触发 native heavy resource？ — **PASS**
- EVIDENCE：原 `App.vue` 直连 `bridge.closeBrowser()` 已改为 `browser.closeBrowser()`（public 动作）。
  现 Shell 仅剩 `flushSessions`（resource=NONE）与 `resourceStats`（只读遥测，NONE）。
- FILES：`src/App.vue:226`、`src/capabilities/browser/state/useBrowserStore.ts:564`
- CHECKER：`check-native-capability-boundaries` NATIVE-03（fail=0）
- RISK：低 / ACTION：门禁常开（仅 `bridge.<cmd>()` 计为直连，避免误报也避免漏报）

## RT-06 Capability absent 是否只是 UI hidden？ — **PASS**
- EVIDENCE：absent → 贡献未注册 → `viewOf()` 返回 undefined → **不挂载**；实现未激活、资源未创建。
- CHECKER：`runtime-resource-absence`（12/12）、`check-capability-platform`（29/0）
- RISK：低 / ACTION：保持

## RT-07 Browser absent 可能创建 Grid/WebView？ — **PASS**
- EVIDENCE：RRA-01 framework `createGrid` **0 次**（createGrid 是 grid-child 唯一出生点）；RRA-02 `gridOpen` 恒 false。
- CHECKER：`runtime-resource-absence` RRA-01/02（RUNTIME_OBSERVED）
- RISK：低 / ACTION：保持

## RT-08 Terminal absent 可能创建 PTY/process？ — **PASS**
- EVIDENCE：RRA-03 framework `ensureTerm` → PTY 创建 **0 次**。
- CHECKER：`runtime-resource-absence` RRA-03
- RISK：低 / ACTION：保持

## RT-09 Database absent 可能创建 connection？ — **FINDING（未实测）**
- EVIDENCE：`db_connect` owner=database，但**无**对应运行时资源探针；结论为 STRUCTURAL（资源法：瞬态建连即弃）。
- FILES：`docs/architecture/native-boundary/native-commands.yaml`、`RESOURCE-OWNERSHIP-MATRIX.md`
- CHECKER：仅 NATIVE-04 对齐，无 absent probe
- RISK：中（缺席结论未实测） / ACTION：随 Debt-7A-1 增加 DB 连接探针

## RT-10 Plugin absent 可能启动 runtime？ — **FINDING（未实测）**
- EVIDENCE：`plugin_install/plugin_enable` owner=plugin，runtime LOCKED；无启停探针。
- RISK：中 / ACTION：Debt-H-3，补 plugin runtime probe

## RT-11 Agent absent 可能启动 execution runtime？ — **FINDING（未实测）**
- EVIDENCE：`agent_parse/validate` 为只读；执行会话无探针。
- RISK：中 / ACTION：Debt-H-3

## RT-12 Native ownership YAML 是否被 checker 真正使用？ — **PASS**
- EVIDENCE：`check-native-capability-boundaries.mjs` **读取同一 YAML** 并据其判 NATIVE-01/02/03/04/07；
  自检含 8 个夹具（含「未登记命令」「registry 冗余」负例），真实扫描 fail=0。
- FILES：`scripts/check-native-capability-boundaries.mjs`、`docs/architecture/native-boundary/native-commands.yaml`
- RISK：低 / ACTION：保持

## RT-13 allowed_callers 是否变成万能 allow-list？ — **PASS**
- EVIDENCE：仅 **12** 条（script 9 条 + `get_start_dirs` + `launch_app` + `clipboard_write`），
  每条**必须**带 `allowed_rationale`（缺理由即 FAIL），且只覆盖「子能力托管 / 已声明 optional 依赖」。
- CHECKER：`check-native-capability-boundaries.mjs`（allowed_rationale 校验 + NATIVE-02）
- RISK：低 / ACTION：新增条目须写理由

## RT-14 148 native commands 是否有漏登记？ — **PASS**
- EVIDENCE：Rust 扫描 148 条 vs YAML 148 条，**双向零漂移**（UNMATCHED=[] / EXTRA=[]）。
- CHECKER：NATIVE-06/07（fail=0）
- RISK：低 / ACTION：新增 `#[tauri::command]` 会立刻 FAIL

## RT-15 manifest / YAML / profiles / catalog 是否仍有漂移？ — **PASS**
- EVIDENCE：本轮已按**运行时真值**对齐（bookmark/vault/workspace → browser 为 **optional**，bridge 归 shared infra 归一化）。
- CHECKER：`check-capability-contract-drift`（fail=0）
- RISK：低 / ACTION：保持

## RT-16 drift checker 是否真正进入 release gate？ — **PASS**
- EVIDENCE：已写入 `package.json` 的 `check` 链（`node scripts/check-capability-contract-drift.mjs`），`npm run check` EXIT=0 时其 fail=0。
- FILES：`package.json`
- RISK：低 / ACTION：不得再摘下

## RT-17 optional 是否被错误当 required？ — **PASS**
- EVIDENCE：bookmark/vault/workspace 的 browser 依赖已改 **optionalDependencies**；
  CB-03 已修正为「required ∪ optional 均视为已声明」（未声明仍 FAIL）。
- FILES：`docs/architecture/capability-registry/capabilities.yaml`、`scripts/check-capability-boundaries.mjs`
- RISK：低 / ACTION：保持

## RT-18 required 依赖缺失是否 deterministic reject？ — **PASS**
- EVIDENCE：`bootstrapAssembly` 在 `report.ok=false` 时**抛错**（携带 rejections），绝不「启动后 undefined」；
  本轮曾因 YAML 空行导致装配失败即为该机制生效的实证。
- FILES：`src/capability/index.ts:143-147`
- RISK：低 / ACTION：保持

## RT-19 Contribution Registry 是否变成 Service Locator？ — **PASS**
- EVIDENCE：按 `slot` + `view` 声明式注册/遍历渲染（`getSurfaceContributions`/`getDockTabContributions`），
  无「按名字任意取组件/服务」的取回接口；Shell 只认 slot/view/元数据。
- FILES：`src/capability/contribution/{registry,types}.ts`
- RISK：低 / ACTION：不得新增 `getById(name)` 式取回

## RT-20 Capability Runtime 是否 God Object？ — **PASS**
- EVIDENCE：GOV-01/02（不 import owner/store/UI、不持有业务 state）；GOV-05（destroy 不直触 owner 资源）。
- CHECKER：`check-composition-profiles` GOV-01..05 PASS
- RISK：低 / ACTION：保持

## RT-21 Settings 是否变成 God Store？ — **PASS**
- EVIDENCE：仅 3 个框架偏好（theme / keymapScheme / tabHibernation），经 `src/settings/public.ts` 契约；
  判定为 **SERVICE（非产品能力）**，常驻不参与用户组合。
- FILES：`src/stores/useSettingsStore.ts`、`src/settings/public.ts`
- RISK：低 / ACTION：新增偏好须走契约

## RT-22 shared/ui 是否吸收业务语义？ — **PASS**
- EVIDENCE：仅 `EmptyState` / `ContextMenu` 等通用件；消费者 14 处可枚举；`UI_BOUNDARIES vacuous=0`。
- FILES：`src/shared/ui/`、`docs/architecture/ui-system/SHARED-UI-CONTRACT.md`
- CHECKER：`check-ui-boundaries`
- RISK：低 / ACTION：保持

## RT-23 Semantic locator 是否因迁移 silent miss？ — **PASS**
- EVIDENCE：vault/home/script 迁移后 `owner_implementations` 路径已同步；`check-semantic-registry` fail=0、closure 27/27。
  （本轮曾实测：vault 状态未登记会立刻产生 12 条 `SEMANTIC_UNREGISTERED_STATE` —— 证明该门禁**具备区分力**，不会静默漏检。）
- RISK：低 / ACTION：后续迁移同步 locator

## RT-24 baseline 更新是否可能洗绿？ — **PASS**
- EVIDENCE：UI 基线策略 `on_new=FAIL / on_removed=FAIL`；本轮唯一一次基线改动（`ui-boundary-baseline.json` 移除 `closeBrowser`）
  对应**真实代码变更**（App.vue 不再直连 native），且门禁当时先报 FAIL 后改代码再同步基线 —— 顺序正确、可追溯。
- FILES：`docs/architecture/ui-system/ui-boundary-baseline.json`、`src/App.vue`
- RISK：中（制度性风险） / ACTION：基线变更必须在提交信息中写明对应代码变更

## RT-25 HUMAN_VISUAL 是否仍诚实标 PENDING？ — **PASS**
- EVIDENCE：全仓文档与 tag 均标记 PENDING；**未创建** `capability-platform-vnext-pass`。
- RISK：低 / ACTION：保持

---

## 额外攻击面（§6）

| 项 | 结论 | 证据 / 处置 |
|---|---|---|
| second truth | **0** | `check-semantic-registry` R1/R8 fail=0；各域单一 owner |
| undeclared dependency | **0** | CB-03 fail=0（含 optional 视为已声明） |
| cross-capability internal import | **0** | CB-01 fail=0 |
| illegal cycles | **0** | CB-04（required 环 0；仅 agent↔graph optional 设计异味 warn） |
| resource leaks | **0** | `runtime-resource-absence` 12/12；`check-capability-resource-boundary` 12/12 |
| stale entrypoints | **已修** | bookmark/database/script entrypoint 已对齐真实文件（H-B） |
| **dead contributions / 无调用点 native 命令** | **FINDING** | `mcp_policy_get`、`mcp_registry_list`、`mcp_capability_preview`、`m0_ready`、`report_title`、`issue_intent`、`request_open_terminal` 无前端调用点 → 登记 **Debt-RT-1（NON_BLOCKING）** |
| broken relative imports | **0** | `npm run build` PASS |
| fake package modularity | **无虚报** | 17 条真实能力包；6 条 NOT_INTEGRATED 已有裁决（未高报） |
| C3/C4/C5 maturity inflation | **未高报** | C4=0、C5=0（destroy 入口 ≠ C5） |
| HP inflation | **未高报** | HP2 仅 4 条（bookmark/vault/home/settings），HP3=0 并写明禁止动态加载理由 |

---

## 结论

**RED_TEAM = PASS（BLOCKER = 0）**

- PASS **20** / FINDING **5** / BLOCKER **0**
- 5 项 FINDING 均为 **NON_BLOCKING**，已登记：
  - **Debt-RT-1**：7 个无调用点的 native 命令（死命令，建议清理或标注）
  - **Debt-H-3 / Debt-7A-1**：Database / Plugin / Agent absent 无运行时资源探针（结论仅 STRUCTURAL/DECLARED，未高报）
- 未发现任何「包装漂亮的单体」证据：能力具备真实物理包 + 贡献渲染 + owner 唯一 + native 归属可判定 + 缺席不创建资源。
