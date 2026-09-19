# Resource Model（资源模型）

> Phase 7A　基线：`semantic-governance-v1`
> **重要：本文所有资源分类均为 `DECLARED RESOURCE CLASS`（声明式分类），不是实测数值。**
> 项目当前**没有**可靠的按能力内存/CPU 测量机制。凡未实测，一律标注 DECLARED，不得编造性能数字。

---

## 1. Resource Class（资源类别）

| Class | 含义 | 典型持有者 |
|---|---|---|
| `LIGHT` | 纯内存状态，无 native 资源、无长驻进程 | Bookmark / Notes / Vault / Skill / Snippet / Session |
| `MEDIUM` | 有中等内存或磁盘 IO，无 native 进程 | Git / KnowledgeGraph / FilePreview / Workbench / ResourceCollection |
| `HEAVY` | 持有 native webview 或较大常驻内存 | Browser / Plugin |
| `VERY_HEAVY` | 持有**多个** native webview | Grid |
| `NATIVE` | 依赖 native 模块 / 动态库 | Plugin / Browser |
| `PROCESS` | 会派生操作系统进程 | Terminal(PTY) / Script |
| `WEBVIEW` | 持有 webview 实例 | Browser / Grid |
| `NETWORK` | 会发起网络请求 | Database / Agent / MCP / Git(remote) |
| `BACKGROUND` | 有后台定时/调度活动 | Task(scheduler) / Sync |
| `SECURITY_SENSITIVE` | 持有凭据或密钥材料 | Credential / Database(连接串) / Git(token) |

---

## 2. Resource Lifecycle（资源生命周期）

```text
ACTIVE ──► BACKGROUND ──► SUSPENDED ──► HIBERNATED ──► DESTROYED
```

| 状态 | 含义 |
|---|---|
| `ACTIVE` | 正在占用资源并提供功能 |
| `BACKGROUND` | 无前台视图，仍持有资源（如后台调度） |
| `SUSPENDED` | 冻结视图与交互，保留状态真源与轻量资源 |
| `HIBERNATED` | 释放重资源（webview/进程），仅保留可重建的持久化状态 |
| `DESTROYED` | 全部释放；持久化状态按 `persistence` 策略保留或清除 |

> **不是所有能力都支持所有 lifecycle。** 每个能力在其 manifest 中显式声明 `lifecycle.supported` 子集；Runtime 对未声明的状态转换必须拒绝（7C 强制）。

---

## 3. 各能力资源分类（DECLARED）

| 能力 | Resource Class | 支持的 lifecycle | 常驻? | 可 suspend? | 可 destroy? |
|---|---|---|---|---|---|
| Bookmark | LIGHT | ACTIVE / SUSPENDED | 否 | ✅ | ✅ |
| Notes / Vault | LIGHT | ACTIVE / SUSPENDED | 否 | ✅ | ✅ |
| Skill / Snippet | LIGHT | ACTIVE / SUSPENDED | 否 | ✅ | ✅ |
| Session | LIGHT | ACTIVE | **是**（关停链路） | ⬜ | ⬜ |
| Workspace / Files | LIGHT | ACTIVE / SUSPENDED | 否 | ✅ | ⬜ |
| FilePreview | MEDIUM | ACTIVE / SUSPENDED | 否 | ✅ | ✅ |
| Git | MEDIUM | ACTIVE / SUSPENDED | 否 | ✅ | ✅ |
| KnowledgeGraph | MEDIUM | ACTIVE / SUSPENDED | 否 | ✅ | ✅ |
| ResourceCollection | MEDIUM | ACTIVE / BACKGROUND | 否 | ✅ | ⬜ |
| Workbench | MEDIUM | ACTIVE / SUSPENDED | 否 | ✅ | ⬜ |
| Agent | MEDIUM + NETWORK | ACTIVE / SUSPENDED | 否 | ✅ | ✅ |
| Database | NETWORK + SECRET | ACTIVE / SUSPENDED | 否 | ✅（断开连接） | ✅ |
| Script | PROCESS | ACTIVE | 否 | ⬜ | ✅（终止进程） |
| Task | BACKGROUND | ACTIVE / BACKGROUND | 否 | ✅（暂停调度） | ✅ |
| Terminal | PROCESS + PTY | ACTIVE | 否 | ⬜（PTY 不能安全冻结） | ✅（kill） |
| Browser | HEAVY + WEBVIEW + NATIVE | ACTIVE | **是**（用户基线） | ⬜ | ⬜ |
| Grid | VERY_HEAVY + MULTI_WEBVIEW | ACTIVE | **是**（用户基线） | ⬜ | ⬜ |
| Plugin | HEAVY + NATIVE | ACTIVE | 否 | ⬜ | ⬜（runtime LOCKED） |
| Credential | SECURITY_SENSITIVE | ACTIVE | **是**（keyring 句柄） | ⬜ | ⬜ |

---

## 4. "关闭某个功能后到底能省什么？" —— 诚实回答

> 领导演示时**只能说声明式结论**，不能报数字。

| 关闭对象（若未来真正实现） | 可预期释放（声明式） | 当前是否可做到 |
|---|---|---|
| Grid | 释放 N 个 native webview + 对应渲染内存 | ⬜ 否（TARGET） |
| Browser | 释放 native webview | ⬜ 否（TARGET） |
| Terminal | 终止 PTY 子进程 | ⬜ 否（TARGET） |
| Database | 断开连接、释放连接池 | ⬜ 否（TARGET） |
| Task | 停止后台调度，消除定时唤醒 | ⬜ 否（TARGET） |
| Bookmark | 仅释放少量内存状态（LIGHT） | ✅ 是（今晚试点，且因本身极轻，"省的"也很有限） |

**必须向领导说明的关键点**：
- LIGHT 类能力（Bookmark 等）**关闭后省的资源很有限**；资源治理的真正收益来自 HEAVY / VERY_HEAVY / PROCESS 类（Grid、Browser、Terminal），而这些**今晚不可物理卸载**。
- 今晚交付的是**资源分类体系与可见性**，不是"一键省内存"。

---

## 5. 测量现状（不编造）

```text
MEASURED_MEMORY_PER_CAPABILITY:  NOT AVAILABLE（无按能力测量机制）
MEASURED_CPU_PER_CAPABILITY:     NOT AVAILABLE
WEBVIEW_INSTANCE_COUNT:          可数（Grid/Browser 持有数量由代码结构决定）—— 未实测运行时
DECLARED_RESOURCE_CLASS:         AVAILABLE（本文 §3，即正式口径）
```

登记债务 **Debt-7A-1**：无按能力资源实测；7E 的 Capability Resource Report 只能给 DECLARED 口径。
