# SCR-20260923-script-owner — Script 语义 Owner 收口

> 状态：**CURRENT_FACT**（已实施，非提案）。
> 阶段：STAGE H-A（Platform Hardening，P0 / D1）。
> 基线：HEAD `701b354`（tag `capability-platform-vnext-code-pass` → `5810a29`）。
> 关联：`capabilities.yaml` script 块、`semantic-registry/states.yaml` owner_implementations。

---

## 1. 审计（基于真实代码，不预设结论）

### 1.1 物理归属

| 项 | 真实位置 |
|---|---|
| Owner store | `src/capabilities/workspace/state/useScriptStore.ts`（Pinia id = `script`） |
| 兄弟域 owner | `useArtifactStore` / `useRepoStore` / `useSnippetStore`（同目录） |
| UI | `src/capabilities/workspace/ui/ScriptPanel.vue`、`ScriptRunHistory.vue` |
| 纯逻辑 | `src/utils/scriptUi.ts` |
| 贡献点 | `src/capabilities/workspace/index.ts` → `registerMainView("scripts", ScriptPanel)` |

即：**Script 物理上属于 workspace 能力包**，与 artifact / repo / snippet 同宿。

### 1.2 状态、写入者、读取者、意图、副作用

| 维度 | 事实 |
|---|---|
| STATE | `scripts`（`ref<ScriptMeta[]>`）、`scriptForm`（`reactive<ScriptForm>`） |
| OWNER | `useScriptStore` —— **唯一 owner**，无第二处持有同义 script 状态 |
| CANONICAL_WRITER | `useScriptStore` 自身：`loadScripts` / `openScriptForm` / `saveScript` / `removeScript` |
| DERIVED_STATE | 无（`scripts` 来自后端 `scriptList`，非派生自他域） |
| INTENTS | `loadScripts`、`openScriptForm`、`saveScript`、`removeScript` |
| READERS | `ScriptPanel.vue`、`ScriptRunHistory.vue`（均在 workspace 包内，无跨能力读者） |
| SIDE_EFFECTS | `bridge.scriptList` / `scriptAdd` / `scriptUpdate` / `scriptRemove`（**后端持久化**，scope=disk） |
| 进程资源 | `script.run` 经后端派生进程执行（重资源），`resources.class=PROCESS`、`permissions=process.spawn` |
| DEPENDENCIES | `bridge`（required）；`useLayoutStore`（框架 toast，非业务依赖） |
| PERSISTENCE | disk（后端持有，前端不落浏览器存储） |

### 1.3 语义注册表现状（审计发现：D1 实为**登记漂移**，非 owner 缺失）

- `states.yaml` `owner_implementations` **已登记** `useScriptStore`
  （paths: `src/capabilities/workspace/state/useScriptStore.ts` / `src/stores/useScriptStore.ts`）—— 源自 Phase 8C-0D（tag `capability-phase8c0d-script-snippet-owner-pass`）。
- `scripts` / `scriptForm` 位于 `observed_not_governed`（与 clipboard / apps / tools 的本地 UI 状态同构，非治理缺失）。
- 而 `capabilities.yaml` script 块仍是 `semanticOwner: null` + `governanceStatus: OWNER_PENDING_SCR`。
- **结论**：语义侧 owner 早已存在；缺的是**能力注册表侧**的登记对齐。

---

## 2. 分类裁决

**裁决：Script = Workspace 子能力（SUB_CAPABILITY，workspace-hosted）。**

理由（真实证据）：

1. 物理与贡献均寄生于 workspace 包（`registerMainView("scripts", ...)` 由 workspace 注册）。
2. 与 artifact / repo / snippet 同构——三者同样有独立 semantic owner、同样无独立能力包。
3. 但 **不能简单并入 workspace 而删除本条目**：`script.run` 真实派生进程，需 `PROCESS` 资源与
   `process.spawn` 权限的独立治理面；并入后该资源将从能力矩阵中消失，形成治理盲区。

### 已否决的备选（REJECTED_ALTERNATIVES）

| 方案 | 否决理由 |
|---|---|
| A. 抽为独立能力包 `capabilities/script/` | 本轮目标是"焊牢平台"而非扩功能；迁移会新增物理边界与门禁扫描域同步成本，且**不改变任何核心产品语义**（owner 仍为 useScriptStore），收益不抵风险 |
| B. 直接并入 workspace、删除 script 条目 | 会丢失 PROCESS / process.spawn 的独立资源与权限治理面，制造治理盲区（违反本轮 H-C 目标） |
| C. 定为 Workbench tool | `script` 具业务持久化（disk）与系统能力（进程派生），非 Shell 工具 |

> **HARD STOP 判定**：A / B / C 均不改变核心产品语义（owner 恒为 `useScriptStore`），
> 不存在"两个同样合理且会改变核心产品语义"的方案 ⇒ **不触发 HARD STOP**，自主裁决为 B 的改良版（保留条目 + 声明子能力）。

---

## 3. 实施

`docs/architecture/capability-registry/capabilities.yaml` script 块：

```diff
-    semanticOwner: null
-    governanceStatus: OWNER_PENDING_SCR
-    status: NOT_INTEGRATED
+    semanticOwner: useScriptStore
+    governanceStatus: GOVERNED
+    status: COMPATIBILITY_WRAPPED
```

并在块前追加注释，声明子能力宿主、贡献方与保留条目的治理理由。

---

## 4. 契约要素（§6 要求）

| 要素 | 取值 |
|---|---|
| STATE | `scripts`、`scriptForm` |
| OWNER | `useScriptStore`（`src/capabilities/workspace/state/useScriptStore.ts`） |
| CANONICAL_WRITER | `useScriptStore.{loadScripts,openScriptForm,saveScript,removeScript}` |
| DERIVED_STATE | 无 |
| INTENTS | 同上四项 |
| SIDE_EFFECTS | `bridge.scriptList/scriptAdd/scriptUpdate/scriptRemove`（后端持久化）；`script.run` 派生进程 |
| RESOURCE_OWNER | script（PROCESS / process.spawn，destroyable） |
| DEPENDENCIES | `bridge`（required） |
| PUBLIC_CONTRACT | 包内消费（`ScriptPanel` / `ScriptRunHistory`）；**无跨能力消费者**，故不增设跨能力 public 出口（避免制造无消费方的假契约） |
| ABSENCE_BEHAVIOR | workspace absent → 无 `view='scripts'` 贡献 → 无 ScriptPanel、无脚本进程出生点 |

---

## 5. 不变量

- **SECOND_TRUTHS = 0**：script 状态唯一 owner；无第二处持有同义状态。
- 未移动任何文件 ⇒ **semantic locator / checker 路径不变**，门禁覆盖不受影响（非"假 PASS"）。
- 未降低任何 checker、未扩大 allowlist、未无据更新 baseline。

---

## 6. 遗留

- `scripts` / `scriptForm` 仍为 `observed_not_governed`（与 clipboard/apps/tools 同构，诚实登记）。
- `useSnippetStore` 同属 workspace 子域，但 **`snippet` 在 capabilities.yaml 无独立条目**（仅 `workspace.snippet` provide）；
  因 snippet 不派生进程、无独立重资源，**不**新增条目（避免为对称而制造空治理面）。
