# M6-0 Spike Result — Browser Core Abstraction

> Date: 2026-09-13
> 裁决：Owner / Chief Architect
> 方式：六路并行只读 Spike，Chief / A0 统一裁决
> 配套：`M6-browser-core-specification.md`（已标记 DEFERRED）、
> `M6-A0-governance-decision-record.md`、`M6-browser-core-dispatch-plan.md`

---

## Verdict

**`GO_WITH_REDUCED_SCOPE`**

依据 Owner 规则第二条：*若只需局部重构 `useBrowserHost` 即可解决主要问题，不得强行上完整 BrowserRuntime。*

- **不实施**：`BrowserRuntime` / `BrowserScene` / `syncScene` / `MockRuntime`，降级 DEFERRED 设计参考
- **实施**：M6 Reduced Scope = S1 ~ S5（局部重构，零 Rust 改动）

---

## 1. 六路证据摘要

| Agent | 范围 | 核心结论 | 立场 |
|---|---|---|---|
| 1 | useBrowserHost 复杂度 | 8 类防御中 2 类可删、3 类应下沉、1 类死代码；最严重是"为绕过后端去重而选无去重命令"的抽象泄漏 | 可做，但建议最小版 |
| 2 | bridge / native 落点 | tab 走插件，grid 走子进程 UDS；**目标端 / 失败语义 / 隐藏语义三者不同构** | **关键否决** |
| 3 | BrowserScene 草案 | 草案可用，但须显式区分 `visible rect` 与 `hidden intent` | 中立 |
| 4 | Mock 测试 ROI | 项目已有 mock 注入机制（clipboard 脚本 data: URL stub），不依赖 Runtime | **NO-GO** |
| 5 | 迁移风险 | 最高风险是"去重缓存 + 隐藏态记忆 + 400ms 守护线程重放"三方耦合 | 谨慎 |
| 6 | Architecture Skeptic | 6 类调用**全在 1 个文件**，"散落 → 收敛"是伪命题 | **NO-GO** |

六路中 **4 路（2/4/5/6）独立得出"完整 Runtime 不成立"**。

---

## 2. 关键否决理由

### ① 「散落 → 收敛」是伪命题
6 类调用实际只落在 1 个 composable + 1 个 store 函数，components 零 invoke 已是既成事实。

### ② 去重主力在 Rust 侧，Scene 搬不走
真正对抗 GTK 时序的是后端 50ms 去重 / 0.02 zoom 阈值 / 400ms 守护线程重放。8 类前端防御每一类都是实测回归的止痛药（重试风暴、残留显示、刷屏）。搬到 Scene 只会换个文件重写。

### ③ 不动后端 = 净负收益；动后端 = 触碰锁定区
- **不动后端**：不把后端 5 个命令合并为 1 个 `apply_scene`，则 RealRuntime 只是 `bridge.tabPosition()` → `runtime.tabPosition()` 的包一层（spec §3.7 写死的停止条件：改名不是减债）
- **动后端**：触碰 I-9（`gtk_fixed_move + size_allocate`）与 I-10（只移 x=-30000，绝不 `hide()`）锁定规则；且 tab（插件）与 grid（UDS）不同构

> 收益在合并层，而合并层在锁定区里。

### ④ 项目已有 mock 注入机制
`check-clipboard-persistence-logic.mjs` 已用 data: URL stub 直载真实 store。历史 WebView bug 全部发生在"下发之后"，Mock 只能验"发了什么"，恰好漏掉。

---

## 3. REDUCED SCOPE 范围与结果

| 切片 | 内容 | 结果 |
|---|---|---|
| S1 | 删除死代码 `webviewsSuspended` | ✅ 6 处删除，src 零残留 |
| S2 | 抽取纯函数 `src/utils/browserLayout.ts` | ✅ gridCellRect / zoom / rect 归一化 |
| S3 | 抽取 `src/utils/browserSync.ts` 闭包工厂 | ✅ deduper / retrier / hidden-intent / 失效组 |
| S4 | 集成进 `useBrowserHost.ts` | ✅ 手写防御状态收敛为工厂 |
| S5 | deterministic 测试 `scripts/check-browser-sync-logic.mjs` | ✅ 48/48 PASS，零新增依赖 |

目标调用链：

```text
useBrowserHost.ts  →  browserSync.ts（去重/重试/隐藏/失效）
                   →  browserLayout.ts（纯几何）
                   →  src/bridge.ts（不改）
                   →  Rust（不改）
```

---

## 4. 治理连带结论

| 项 | 结论 |
|---|---|
| 完整 BrowserRuntime 路线 | **DEFERRED** |
| M6 Reduced Scope | 已授权并执行完毕 |
| R3B Workbench UI | **继续 DEFERRED**，不假装 ACCEPT |
| Product Code LOCK | 按 §3.3 Option A 收窄为 W18/W19/R3B 路线 |
| M7 Workspace Retrieval | 不受影响，可提前 |
