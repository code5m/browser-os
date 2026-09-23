# Plugin Capability — Full-Stack Boundary（Capability Library Expansion v1, STAGE F）

> STAGE F 产物。物理：PluginManager 迁入 `src/capabilities/plugin/ui/`，语义 owner `usePluginStore`
> 迁入 `src/capabilities/plugin/state/`；经通用 Contribution Registry 的 `WORKBENCH_MAIN` 槽
> （view='plugin'）贡献给 MainArea。MainArea 只按槽渲染、不 import 能力内部。
> 最后更新：2026-09-23。

## 成熟度（诚实，不谎报）

**C2 ISOLATED**（status=`COMPATIBILITY_WRAPPED`，manifest.v1.maturity=`C2`；governanceStatus=`LOCKED`）。

评级依据（按本项目 `C0..C5` 标度）：
- **C2 ISOLATED 达成**：实现经明确边界隔离——`manifest.ts` / `public.ts` / `index.ts` / `state/` / `ui/` 五段边界；
  语义 owner `usePluginStore` 唯一（无第二真源）；MainArea 不再静态 import PluginManager（贡献驱动）。
- **非 C3**：① `governanceStatus=LOCKED`——plugin 运行时（loader/执行）**设计上未实现且不可组合启停**；
  ② 无 plugin 专属 absence 运行时门禁（依赖 profile 预设 + 贡献缺席，缺自动化断言）；
  ③ `mainView='plugin'` 导航项仍硬编码于 `useLayoutStore`/`homeUi`/`HomeLaunchers`（未贡献驱动）。
- **非 C4/C5**：无运行时 enable/disable 真实装配、无资源可释放（Stage-I 从不创建 load 实例）。

## 十七段 Full-Stack 契约

```text
Plugin UI (capabilities/plugin/ui/PluginManager.vue)
  ↓ OWNED_BY_CAPABILITY（经 public.ts 消费语义 owner）
Plugin State Owner: usePluginStore (id="plugin", src/capabilities/plugin/state/usePluginStore.ts)
  ↓ 意图（intents）
  refreshList(filterState) / getDetail(id) / install() / enable(id) / disable(id)
  / refreshKeys / addKey(keyId,pubkey,note) / removeKey(keyId) / selectFromList / clearError
  ↓ PUBLIC_DEPENDENCY（bridge，外部基础设施）
Plugin Adapter: src/bridge.ts →
  pluginList / pluginGet / pluginInstall / pluginEnable / pluginDisable
  / pluginKeysAdd / pluginKeysList / pluginKeysRemove
  ↓ NATIVE_ADAPTER（Rust tauri::command）
src-tauri/src/bridge.rs（Stage-I 登记簿，已实现）：
  plugin_install(br:7833) plugin_enable(7883) plugin_disable(7898) plugin_list(7913)
  plugin_get(7925) plugin_keys_add(7939) plugin_keys_list(7980) plugin_keys_remove(7991)
  ↓ 后端资源（Rust）
src-tauri/src/plugin.rs：plugins.json / trusted-pubkeys.json（session::atomic_write）
  —— 仅本地登记簿 + 生命周期状态机；**无 loader / 无解包 / 无真验签 / 无动态加载 / 无执行**。
```

## 十七段逐项事实

| 段 | 事实（证据） |
|---|---|
| Identity / Manifest | `src/capabilities/plugin/manifest.ts`（id=plugin，C2，HP0） |
| Public Contract | `src/capabilities/plugin/public.ts`（仅再导出 `usePluginStore`，SECOND_TRUTHS=0） |
| Dependencies | `bridge`（外部基础设施，runtime 豁免）；无能力依赖 |
| State Owner | `usePluginStore`（id="plugin"）唯一 |
| Canonical Writers | 仅 `usePluginStore` 的 action 写 `list/detail/keys/filterState`；组件只读经 public |
| Intents | refreshList / getDetail / install / enable / disable / refreshKeys / addKey / removeKey |
| Application Logic | `src/utils/pluginUi.ts`（headless：状态机镜像、门控、脱敏投影、manifest 预检、五态投影） |
| UI | `capabilities/plugin/ui/PluginManager.vue`（懒加载，经贡献注册） |
| Contributions | `plugin.main.panel`（slot=workbench-main, view='plugin'） |
| Side Effects | 全部经 `bridge.plugin*`；UI **零裸 invoke**（门禁 `PLUGIN_UI_NO_RAW_INVOKE`） |
| Adapter / Native Boundary | `bridge.ts` → Rust `plugin_*`（8 命令，ACL 放行，来源校验） |
| Permissions | 能力侧无授权能力（`PLUGIN_CAPABILITY_V1 = []` fail-closed）；ACL 仅主窗口 |
| Persistence | **磁盘**（Rust `data_dir/plugins.json` + `trusted-pubkeys.json`，atomic_write）；**前端零浏览器存储** |
| Resource Ownership | Stage-I **不创建任何 load 实例** → `v1.resources=[]`（class `[HEAVY,NATIVE]` 为潜在声明） |
| Lifecycle | `ACTIVE`；停止/暂停不支持（suspendable=false） |
| Absence Behavior | 见下（实测证据） |
| Tests / Gates | `check-plugin-policy.py` / `check-plugin-privacy.py` / `check-plugin-ui-privacy.py` / `check-plugin-ui-logic.mjs`（70 断言）+ `npm run check` |

## 五态资源/生命周期模型（§18：绝不可合并成单一 boolean）

后端真源 = 单枚举 `PluginState`（8 态：`discovered/validating/signed_ok/signed_failed/loaded/enabled/disabled/uninstalled`）
+ `PluginResourceMeta{declared_hash, path_provided, verified}`。前端 `utils/pluginUi.ts::pluginFacets()` 做**派生投影**（非第二真源）：

| 概念 | 真身 | Stage-I 取值 | 说明 |
|---|---|---|---|
| AVAILABLE | 可发现目录中的条目 | 恒 `false` | 无 discovery/远端目录源（诚实缺口） |
| INSTALLED | 登记簿存在且 `state≠uninstalled` | 由 state 派生 | 单枚举承载 |
| ENABLED | `state===enabled` | 由 state 派生 | 仅生命周期开关 |
| ACTIVE | 运行时实例已加载运行 | 恒 `false` | 无 loader → **ENABLED ≠ ACTIVE** |
| RESOURCE_EXISTS | 活的本地资源实例 | 恒 `false` | 无 load 实例；与 `resource.path_provided/verified`（声明路径是否在允许根内）**非同一事** |

门禁 `check-plugin-ui-logic.mjs` §10 断言：五字段为**独立布尔**；`enabled` 状态下 `active===false`（证明绝不合并 ENABLED/ACTIVE）；`available/active/resourceExists` 恒 false（诚实缺口）。

## 安全边界（Credential / Filesystem / Native Loader，显式审计）

- **无 native loader / 无动态加载 / 无执行 / 无网络**：门禁 `PLUGIN_NO_EXEC_SURFACE` 禁 `plugin_invoke|Command::new|std::process|WebviewWindow|dlopen|reqwest|tokio::spawn`。
- **Filesystem**：`plugin_install` 对 `resource_path` 过 `security_policy::check_path_within_roots(allowed_roots)`，**仅取布尔**（不落路径）。
- **Credential**：metadata/各文本字段过 `contains_credential_leak`；审计只记稳定码；登记簿**不落**公钥原文（仅 sha256 前 16 hex 指纹）与签名原文（仅 algorithm+key_id）。
- **能力白名单 fail-closed**：`security_policy::PLUGIN_CAPABILITY_V1 = []` → 任何未登记能力安装即拒（`UnknownCapability`）。
- **来源校验**：每个 `plugin_*` 命令体内 `check_invocation_source`（ACL + `security_policy::check_remote_invocation`）。
- **前端**：零浏览器存储（`PLUGIN_UI_NO_BROWSER_STORAGE`）、零裸 invoke、错误只用稳定本地文案。

## Absence Behavior（§18/§29，实测证据）

STAGE F 用探针实测（esbuild 真实 bundle + `bootstrapCapabilityRuntime(profile)`）：

```text
framework: activated=true hasPluginView=false views=[]
minimal:   activated=true hasPluginView=false views=[files,arts,repo,scripts,commands,audit,editor]
developer: activated=true hasPluginView=false views=[...,db]
full:      activated=true hasPluginView=true  views=[...,db,agents,skills,plugin]
```

- Plugin absent（framework/minimal/developer）→ `registerPluginContributions` 不运行 → `WORKBENCH_MAIN` 槽无 `view='plugin'`
  → MainArea `viewOf('plugin')` 返回 `undefined` → 不渲染 PluginManager → `usePluginStore` 不被实例化 → **零 bridge.plugin\* invoke** → 无登记簿读写。
- Shell 不崩溃：通用 `viewOf` 分支对未知 view 返回 undefined 自然跳过。
- **资源缺席**：Stage-I plugin 本身不创建任何 runtime 资源（无 loader），故 absent 时资源恒为 0（`RUNTIME_RESOURCE_ABSENCE 12/12`）。

## STAGE F 顺带修复的运行期回归（诚实记录）

**发现**：`src/capability/profiles.ts` 的 TS 运行时真源 `CAPABILITY_PROFILES.full` 仅列 4 个能力
（bookmark/workspace/browser/terminal），而 STAGE C/D/E 已把 database/agent/skill 迁为贡献驱动并**移除了 MainArea 硬编码分支**。
bootstrap 只激活 profile 列出的能力 → 这三个能力的贡献**在运行期未注册** → `viewOf('db'/'agents'/'skills')` 为 undefined → 面板静默空白（UI 回归）。

**根因（第二层）**：这些能力的 manifest `dependsOn` 混合了「能力依赖」与「基础设施前置」（`bridge`/`credential`），
而 `Runtime.resolve` 把它们一并当能力严格校验 → `MISSING_DEPENDENCY` → 整个 bootstrap 循环中断。

**修复**：
1. `src/capability/runtime.ts`：显式定义 `EXTERNAL_INFRA_DEPS = {bridge, credential}`，`resolve` 豁免外部基础设施；
   真正的能力依赖（如 `git→workspace`）仍严格校验。
2. `src/capability/index.ts`：`ALL_CAPABILITIES` 调整为**依赖安全顺序**（被依赖者先行注册）。
3. `src/capability/profiles.ts`：`full` 吸纳全部已登记能力；`developer` 补入 git/database。

修复后实测：`full` 注册 9 个能力，`WORKBENCH_MAIN` 视图含 `db/agents/skills/plugin` → **UI 恢复（UI_PRESERVATION）**。

## Legacy Debt（诚实，不静默消失）

1. **plugin 运行时（loader/解包/真验签/动态加载/执行/卸载运行时）未实现** —— Stage-I LOCKED，真实功能缺口。
2. 无 plugin 专属 absence 运行时门禁（依赖 profile 预设 + 探针实测，缺常驻自动化断言）。
3. `mainView='plugin'` 导航项硬编码（`useLayoutStore`/`homeUi`/`HomeLaunchers`，未贡献驱动）。
4. `check-plugin-ui-logic.mjs` **未接线**到 `pre-merge.sh`（仅可手动/独立运行）。
5. `EXTERNAL_INFRA_DEPS` 豁免为显式白名单：若未来新增基础设施依赖名，须同步登记，否则 `resolve` 会（正确地）拒绝。

## SECOND_TRUTHS = 0 / RESOURCE_LEAKS = 0

`public.ts` 仅再导出 `usePluginStore`（语义 owner），未创建 `runtime.pluginOpen/pluginVisible/pluginActive` 镜像状态。
五态由 `detail` **派生**（pluginFacets），非常驻状态。
DOMAIN STATE（usePluginStore）≠ CAPABILITY COMPOSITION STATE ≠ UI LOCAL STATE（`manifestText`/`resourcePath`/`keyForm` 瞬时）
≠ RESOURCE RESULT（Stage-I 无 resource）。新增资源泄漏 = 0（`RUNTIME_RESOURCE_ABSENCE 12/12`）。
