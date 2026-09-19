# Dependency Graph（依赖图）

> Phase 7A　基线：`semantic-governance-v1`
> 依赖边以**真实引用关系**为准（store→store、component→store、bridge→Rust 模块），不画理想图。

---

## 1. 共享基础设施（所有能力都必须依赖，不可卸载）

```text
                    ┌──────────────────────────────┐
                    │   App Shell (App.vue / Layout)│  INFRASTRUCTURE
                    └──────────────────────────────┘
                                  │
        ┌─────────────────────────┼─────────────────────────┐
        │                         │                         │
┌───────▼────────┐      ┌─────────▼────────┐      ┌─────────▼────────┐
│ bridge.ts      │      │ Pinia stores     │      │ security_policy  │
│ (native 适配)  │      │ (状态真源)        │      │ (Rust 安全边界)   │
└───────┬────────┘      └─────────┬────────┘      └─────────┬────────┘
        │                         │                         │
        └─────────────────────────┼─────────────────────────┘
                                  │
                    ┌─────────────▼─────────────┐
                    │ session.rs (持久化/关闭)   │
                    └───────────────────────────┘
```

**不可卸载理由**：`bridge.ts`（48 处 `invoke`）是唯一 Native 通道；`security_policy.rs` 是安全边界；`session.rs` 管持久化与关停顺序。任何 Capability 若绕过它们即为违规。

---

## 2. 能力依赖边（真实）

```text
Browser ──────► bridge (native webview)
   │
   ├──────────► Grid            (gridSession / grid_process.rs 多 webview)
   │
Workspace ────► bridge (fs_cmds.rs)
   │
   ├──────────► Files           (FilePanel / FileEditor)
   │                 │
   │                 └────────► FilePreview   (images.rs)
   │
Terminal ─────► bridge (terminal.rs PTY)  ── 与 Clipboard 共享 useSystemStore ⚠
   │
Bookmark ─────► Home (展示)
   │
Credential ───► Rust keyring ──► security_policy.rs   (SECURITY_SENSITIVE)
   │
Database ─────► bridge (database.rs) ──► Credential (连接凭据)
   │
Git ──────────► bridge ──► Credential (gitRepoToken) ──► Workspace (仓库路径)
   │
Agent ────────► bridge (agent.rs / agent_memory.rs)
   │
   ├──────────► Skill           (skills.rs)
   │
Plugin ───────► bridge (plugin.rs) ── ⚠ runtime LOCKED
   │
KnowledgeGraph ► bridge (graph.rs) ──► 引用 Skill/Agent 节点 id（64-hex）
   │
Notes ────────► bridge (save_note / collect_selection)
   │
   └──────────► Vault          (vault_open)
   │
ResourceCollection ──► bridge (resource_capture settings)
   │
Task ─────────► bridge (tasks.rs) ──► scheduler.rs (BACKGROUND)
   │
Session ──────► session.rs / shutdown.rs
   │
Script ───────► bridge (scripts.rs) ──► script_runner.rs (PROCESS)
   │
   └──────────► Snippet        (snippets.rs)
   │
Workbench ────► bridge (workbench.rs)
```

---

## 3. 耦合深度分级

| 等级 | 能力 | 现状说明 |
|---|---|---|
| **DEEP（今晚不动）** | Browser / Grid | webview + `grid_process.rs` + `browserSync.ts` 三层耦合；是用户已认可的**稳定高价值基线**，任何改动风险最高 |
| **DEEP（今晚不动）** | Terminal | PTY / process / replay 历史环形缓冲；且与 Clipboard 共享 `useSystemStore` |
| **MEDIUM** | Workspace / Files / FilePreview | 父子嵌套，但无 native 进程；可先做兼容包装 |
| **MEDIUM** | Git / Database | 依赖 Credential，依赖 workspace 路径；无长驻进程 |
| **LIGHT** | Bookmark / Notes / Vault / Skill / Snippet / Session | 无 native 进程，无长驻资源 → **试点候选** |
| **LOCKED** | Plugin | runtime 冻结，只登记不装配 |

---

## 4. 循环依赖风险（静态审计结论）

```text
CIRCULAR_DEPENDENCY_RISK: 当前静态审计未发现已存在的循环依赖。
```

潜在（未来）风险点，登记为待观察，不是现状缺陷：

1. `Git ↔ Credential`：Git 需要 token，Credential 若将来要审计"谁用了 token"会反向依赖 Git。
2. `KnowledgeGraph ↔ Agent/Skill`：图谱按 id 引用 Agent/Skill 节点；若将来 Agent 要"读取图谱做决策"则形成环。
3. `Script ↔ Task`：Task 调度可触发 Script，Script 若将来创建 Task 则成环。

> 处理方式：这些**当前不是环**。`check-capability-registry.mjs`（7B）将强制检测环，一旦真实出现即阻断。

---

## 5. 试点选择依据（供 7D 决策）

按"生命周期简单 / Native 依赖低 / 风险低"筛选：

```text
Bookmark   LIGHT · 无 native · 有明确 Owner(useBookmarkStore) · UI 简单   ★ 首选
Notes      LIGHT · 仅 bridge save_note/vault_open · Owner 未登记          ○ 次选（需先补 SCR）
Skill      LIGHT · skills.rs · Owner 未登记                              ○ 次选
Session    LIGHT · 但属关闭链路，改动影响启动/退出                         ✗ 不选
Browser/Grid  VERY_HEAVY · 稳定高价值基线                                 ✗ 明令不选
Terminal   PROCESS/PTY · 共享 store 耦合                                  ✗ 明令不选
```

**结论：7D 试点 = Bookmark**（唯一同时满足"有已登记 Owner + 无 native + LIGHT + UI 简单"的候选）。
