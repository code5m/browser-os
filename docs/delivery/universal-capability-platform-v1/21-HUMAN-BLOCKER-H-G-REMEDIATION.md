# 21 · HUMAN ACCEPTANCE BLOCKER H-G 修复与文档更正

> 本文件是**工程审计轨迹**，不是失败羞耻。
> 目的：把此前「Browser absent → resource PASS」的表述，从
> **structures-only evidence** 更正为 **STRUCTURAL + RUNTIME 双维证据**。

---

## 1. 更正声明（§25）

```
PREVIOUS CLAIM:
  Browser absent → resource isolation PASS（依据：contribution 槽为空 / 能力未注册）

HUMAN ACCEPTANCE FINDING:
  H-G RUNTIME RESOURCE ABSENCE = FAIL
  VITE_CAPABILITY_PROFILE=framework 启动、无任何点击，主进程下出现
  4 个 `mvp-browser-os --grid-child N` 子进程。

ROOT CAUSE:
 宫格子进程是 Browser capability-owned 重资源，但其创建路径可被
 「能力可用性之外的原因」触发：
   PROBLEM A  Framework Core（useLayoutStore）反向依赖 Browser 内部实现，
              直接调用 browser.openGrid() / browser.closeGrid()。
   PROBLEM B  gridToolbarOpen 是 Shell 的 UI preference，却被当成
              「立即创建 Grid 重资源」的许可。
              preference ≠ availability ≠ activation ≠ resource existence

FIX:
  ① 唯一资源出生点收口：buildGrid() 前置 isBrowserResourceAllowed() 闸
  ② 判定真源单一：runtimeSingleton.isCapabilityActive(id)，fail-closed
  ③ Shell 解耦：toggleGridToolbar 只翻转 UI 偏好，不再触碰 Browser
  ④ 依赖方向纠正：Browser 单向 watch Shell 偏好，且先过闸
  ⑤ Terminal 同类防护：spawnTerm() 前置 isTerminalResourceAllowed()

POST-FIX EVIDENCE:
  进程级：GRID_CHILD_COUNT=0（修复前 4），主窗口 WebKit 2 个（EXPECTED）
  应用级：createGrid=0 / PTY=0；负例（闸恒放行）createGrid=1 → 断言有区分力
  UI 级：first-paint 文本完整（🏠主页📁浏览🗂️宫格☰菜单🤖📥采集⚙️ + 17 张启动卡）
```

### 诚实补充（审计发现，与人工观测并存）

当前 HEAD 的 `useLayoutStore.toggleGridToolbar()` 实际是**死代码**（全仓无调用方），
且函数体内引用的 `useBrowserStore` 在该文件**从未 import**，一旦被调用会抛 `ReferenceError`。
因此人工验收时的**具体触发点无法由 HEAD 源码静态复现**（可能与历史构建/缓存/环境有关）。

**修复因此选择收口在唯一出生点，而不是逐个调用方打补丁** ——
这样无论触发链来自何处，absent Browser 都不可能产出宫格子进程。
这条设计选择已由负例验证（SELF-03）。

---

## 2. 三维验收（§7）

| 维度 | 目标 | 结果 |
|---|---|---|
| **A. UI_PRESERVATION** | 现有优秀 UI/UX 保持 | **PASS**（.vue 零改动；first-paint 文本完整） |
| **B. CAPABILITY_ISOLATION** | Shell/Core 不绕过契约控制能力内部 | **PASS**（0 RESOURCE_CREATION_VIOLATION） |
| **C. RESOURCE_ISOLATION** | absent 能力不创建其重资源 | **PASS**（GRID_CHILDREN=0，PTY=0） |

三个维度**分开判定**，不再用「framework-only 是否很空」当指标。

---

## 3. WORKBENCH UX PRINCIPLE（§26，写入架构原则）

1. Shell 可以拥有丰富通用 UX。
2. Capability modularization **不以减少 UI 为目标**。
3. Capability migration 默认保持视觉/交互等价。
4. Shell 提供 generic slots；Capability 提供 contributions。
5. Capability-owned 重资源只由 Capability lifecycle 创建。
6. absent Capability 不得因历史 preference 自动创建资源。
7. legacy Shell UI **不应直接删除**，应逐项迁移 ownership。
8. UI ownership 与 resource ownership **必须分开**。
9. 「看不见」不能作为「资源不存在」的证据。
10. preference ≠ availability ≠ activation ≠ resource existence。

---

## 4. 本阶段**未做**（§21 反过度重构）

- ❌ Git / Database 升 C3
- ❌ 所有面板迁 Capability
- ❌ 全量 package 化 / HP3 loader
- ❌ 重写 Shell / 重写 Browser-Grid / 重写 Runtime
- ❌ 任何 UI redesign

只做：H-G blocker 修复 + 防同类复发的最小长期边界。

---

## 5. 遗留债务（显式，不静默消失）

| ID | 内容 | 处置 |
|---|---|---|
| HG-D1 | `ActivityBar` 等 Shell 仍直读写 `browser.gridCount/gridLayout/gridUrls/gridAiInput` 等展示字段（LEGACY_COUPLING，13 项） | 保留 UX，后续逐项迁 ownership |
| HG-D2 | framework profile 下点击 🗂️宫格进入空 grid 视图（资源不创建，但视图已切） | 未改（改了等于删 UI）；待能力缺失时的入口降级策略裁决 |
| HG-D3 | Browser capability WebView 与主窗口 WebKit 进程无法可靠区分 | 诚实记 UNKNOWN，不伪造 PASS |
| HG-D4 | 进程级资源采集未做成全自动脚本（本轮手工执行并留证据） | 后续可脚本化；自动化门禁已覆盖应用逻辑层 |
| HG-D5 | 13 项 LEGACY_SHELL_COUPLING 未迁移 | 见 `18-FRAMEWORK-UI-OWNERSHIP.md` §2.2 / §4 |
