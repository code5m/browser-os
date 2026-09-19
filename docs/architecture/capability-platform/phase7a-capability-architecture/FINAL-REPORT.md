# Phase 7A — Capability Architecture Audit & Design（终稿）

> 日期：2026-09-19
> 分支：`feature/capability-platform-v1`
> 基线：`semantic-governance-v1` (`316130d`)　本地 HEAD 开工点：`c0069b7`
> 阶段性质：**审计 + 设计，不改业务代码**

---

## 1. 交付物

```text
docs/architecture/capability-platform/phase7a-capability-architecture/
  Capability-Inventory.md    能力清单 + 分类（CAPABILITY/SUB/UI/SERVICE/ADAPTER/INFRA/DETAIL）
  Dependency-Graph.md        依赖边 + 耦合深度 + 循环依赖风险 + 试点选择依据
  Lifecycle-Model.md         生命周期状态机 + 今晚真实支持矩阵
  Resource-Model.md          Resource Class / Lifecycle + DECLARED 口径
  Shell-Boundary.md          Shell/Runtime/Capability/Registry 四层边界
  Migration-Plan.md          波次规划 + 回滚点
  FINAL-REPORT.md            本文
```

---

## 2. 必须回答的六个问题（诚实版）

### Q1：哪些模块可以真正独立启停？
**答：今晚一个都没有。**
所有 store 在启动时经 Pinia 统一实例化，`App.vue` 静态引用 8 个 store。当前可实现的最多是 `COMPATIBILITY_WRAPPED`（经适配器接入 Runtime），不是物理启停。

### Q2：哪些必须共享基础设施？
**答**：`bridge.ts`（48 处 `invoke`，唯一 Native 通道）、`security_policy.rs`（安全边界）、`session.rs`/`shutdown.rs`（持久化与关停）、`useLayoutStore`（导航）、Pinia 本体。

### Q3：哪些耦合太深？
**答**：
- Browser/Grid：webview + `grid_process.rs` + `browserSync.ts` 三层（DEEP）
- Terminal：PTY + 进程 + replay 环形缓冲，**且与 Clipboard 共享 `useSystemStore`**（DEEP，真实耦合发现）
- Workspace/Files/FilePreview：父子嵌套（MEDIUM，无 native）

### Q4：哪些是高资源模块？
**答**（DECLARED，非实测）：Grid（VERY_HEAVY + MULTI_WEBVIEW）、Browser（HEAVY + WEBVIEW + NATIVE）、Plugin（HEAVY + NATIVE）、Terminal（PROCESS + PTY）、Script（PROCESS）、Database/Agent（NETWORK）。

### Q5：哪些应该常驻？
**答**：Shell/Layout、Session（关停链路）、Credential（keyring 句柄）、Browser/Grid（用户已认可的稳定基线）。

### Q6：哪些可以 suspend / hibernate / destroy？
**答**：
- 可 suspend：Bookmark、Notes/Vault、Skill、Snippet、Workspace、FilePreview、Git、KnowledgeGraph、Agent、Database、Task、ResourceCollection、Workbench
- 可 hibernate（释放重资源保留可重建状态）：目前**无**真正实现（TARGET）
- 可 destroy：LIGHT/MEDIUM 类；Terminal/Script 可 kill 进程
- **Browser/Grid/Terminal 今晚不支持 suspend/destroy**（稳定基线与 PTY 安全）

---

## 3. 关键决定（DECISION）

| # | 决定 | 理由 |
|---|---|---|
| D-7A-1 | 试点能力 = **Bookmark** | 唯一同时满足「已登记 Owner + 无 native + LIGHT + UI 简单」；Browser/Grid/Terminal 明令不选 |
| D-7A-2 | 资源口径 = **DECLARED**，不报实测数字 | 无按能力测量机制；编造数字违反诚实原则（Debt-7A-1） |
| D-7A-3 | Plugin 仅登记，不装配 | runtime 仍 LOCKED（既有冻结决定） |
| D-7A-4 | Capability Registry **不得重复** Semantic Registry 事实 | manifest 只引用 `semanticOwner`，不重声明语义 |
| D-7A-5 | Skill/Notes/Script 进入 Registry 前**须先补 SCR 登记 Owner** | 否则 Runtime 成绕过后门 |
| D-7A-6 | 今晚不改业务代码 | 7A 为审计设计；7B 起才新增能力层文件 |

---

## 4. 已登记债务

| ID | 内容 | 状态 |
|---|---|---|
| Debt-7A-1 | 无按能力资源实测机制，Resource Report 只能给 DECLARED 口径 | KNOWN DEBT（保留） |
| Debt-7A-2 | `useSystemStore` 同时是 Terminal 与 Clipboard 的 Owner，二者无法独立编排 | KNOWN DEBT（保留，需 SCR 才能拆） |
| Debt-7A-3 | Skill / Notes / Script 无登记 Owner | KNOWN DEBT（7B 前置） |
| Debt-7A-4 | Plugin runtime LOCKED，不能声明可装配 | KNOWN DEBT（既有冻结） |

---

## 5. 门禁状态

```text
业务代码修改:        NONE（本阶段纯文档）
git fsck --full:     PASS
snapshot:            .snapshots/2026-09-19-overnight-baseline.txt
分支:                feature/capability-platform-v1
基线 tag:            semantic-governance-v1 -> 316130d（未移动）
```

---

## 6. 结论

```text
PHASE_7A_RESULT: PASS（审计与设计完成，零业务代码改动）
```

能力边界、依赖、生命周期、资源分类与外壳边界已**以真实代码证据**建立；
试点选择、推迟范围与债务均已显式登记，未把目标态伪装成现状。

下一步：**Phase 7B — Capability Contract & Registry**。
