# Phase 7D — Pilot Capability Integration（终稿）

> 日期：2026-09-19　分支：`feature/capability-platform-v1`
> 试点能力：**Bookmark**

---

## 1. 为什么选 Bookmark（不选 Browser/Grid/Terminal）

按「生命周期简单 / Native 依赖低 / 风险低」筛选，Bookmark 是**唯一**同时满足以下全部条件的能力：

| 条件 | Bookmark | Browser/Grid | Terminal |
|---|---|---|---|
| 已在 Semantic Registry 登记 Owner | ✅ `useBookmarkStore` | ✅ | ✅ |
| 无 native 依赖 | ✅ | ❌ webview | ❌ PTY |
| 资源等级 | LIGHT | HEAVY / VERY_HEAVY | PROCESS |
| UI 简单 | ✅ | ❌ | ❌ |
| 稳定高价值基线（不宜动） | 否 | ✅ 是 | 部分是 |

> Browser/Grid 是用户已认可的**稳定高价值基线**；Terminal 涉及 PTY/process 且与 Clipboard 共享 `useSystemStore`。二者明令不作为第一试点。

---

## 2. 交付物

```text
src/capability/capabilities/bookmark.ts   Bookmark manifest + lifecycle 钩子（适配器）
src/capability/index.ts                   应用侧 bootstrap（幂等、失败不打断启动）
src/main.ts                               +11 行：bootstrapCapabilityRuntime（附加，不改既有逻辑）
scripts/check-capability-pilot.mjs        PLT-01..PLT-08 集成门禁
```

**未改动**：`src/stores/useBookmarkStore.ts`、`src/components/home/**`（由 PLT-05 的 git 断言证明）。

---

## 3. Compatibility First 的落地形态

```text
旧系统（useBookmarkStore，仍是唯一业务 Owner）
        ↓  声明式引用（semanticOwner: 'useBookmarkStore'）
Capability Adapter（bookmark.ts：manifest + 空钩子）
        ↓
Capability Runtime（register → resolve → activate）
```

适配器**不 import、不读写**任何业务 store；`onActivate` / `onSuspend` 是空实现（by design）。
业务状态真源 100% 留在 `useBookmarkStore`，Semantic Governance 零回归。

---

## 4. 试点必须证明的六件事

| 要求 | 断言 | 结果 |
|---|---|---|
| 1. Manifest 可以注册 | PLT-01 | ✅ |
| 2. Runtime 可以发现 | PLT-02（inspect 字段齐全） | ✅ |
| 3. Capability 可以 activate | PLT-03（state=ACTIVE） | ✅ |
| 4. 不破坏原业务 Owner | PLT-05（源码禁 import + git 证明业务文件未改） | ✅ |
| 5. 不加载其它可选能力也能工作 | PLT-04（仅 1 个注册项，强依赖为空） | ✅ |
| 6. 原 UI 行为保持 | PLT-05(b) git 断言 + `npm run build` 通过 | ✅ |

补充：PLT-06 manifest 与 `capabilities.yaml` 一致；PLT-07 suspend→activate 往返；PLT-08 inspect 不含业务状态字段。

```text
CAPABILITY_PILOT_RESULT=PASS (8/8)
npm run build: 成功（✓ built in 5.92s）
```

---

## 5. 过程中修掉的两个真实缺陷（不是降低标准）

1. **ESM/CJS 混用**：`check-capability-pilot.mjs` 里误用 `require('node:fs')`（ESM 中不可用）→ 改为顶层 `import`。
2. **自家断言误报**：PLT-05 初版用 `import[^\n]*useBookmarkStore` 匹配，把**注释**里的「不得 import useBookmarkStore」当成真实引入而误报。
   修正方式：先剥离注释，再只匹配真实 `from '...'` / `import(...)` / `require(...)` 语句。
   **注意**：修的是断言精度，**没有**放宽"适配器不得引入业务 store"这条约束。

---

## 6. 未做 / 明确不声称

| 项 | 说明 |
|---|---|
| 物理卸载 | **没有**。Bookmark 经适配器包装，底层仍是既有 store；不是真正独立启停 |
| UI 变化 | **没有**。试点不改任何界面；能力层当前不可见（除诊断日志） |
| 其它能力接入 | 仅 Bookmark。其余 17 个能力仍 `NOT_INTEGRATED` |
| 组合装配 | 仅注册了 1 个能力；Composition Profile 是 7E/7F 的演示口径 |

**状态标注（严禁混淆）**：
```text
Bookmark            = COMPATIBILITY_WRAPPED  ← 今晚真实状态
其余 17 个能力       = NOT_INTEGRATED
Browser/Grid/Terminal 物理启停 = TARGET_COMPOSABLE（未实现）
```

---

## 7. 已登记债务

| ID | 内容 |
|---|---|
| Debt-7D-1 | Bookmark 是 COMPATIBILITY_WRAPPED，非物理卸载；"关闭 Bookmark"不等于释放其 store |
| Debt-7D-2 | 能力层当前无 UI 呈现（仅诊断日志），领导演示依赖 7F 的 inspect 报告/文档 |
| Debt-7D-3 | 仅 1 个能力接入，Composition Profile 尚不能真实验证组合行为 |

---

## 8. 结论

```text
PHASE_7D_RESULT: PASS（Bookmark 试点 8/8，业务 Owner 未破坏，build 通过）
```

下一步：**Phase 7E — Resource Governance v1**。
