# Phase 7E — Resource Governance v1（终稿）

> 日期：2026-09-19　分支：`feature/capability-platform-v1`

---

## 1. 交付物

```text
docs/architecture/capability-registry/profiles.yaml
    Composition Profiles（Minimal / Developer / Full）+ 每能力可组合性真实状态
scripts/capability-resource-report.mjs
    Capability Resource Report 生成器（--out / --json / --self-test / --help）
docs/architecture/capability-platform/phase7e-resource-governance/Capability-Resource-Report.md
    由脚本从 registry 真源生成（非手写，避免文档与真源漂移）
```

---

## 2. Resource Class（13 类）

```text
LIGHT  MEDIUM  HEAVY  VERY_HEAVY  MULTI_WEBVIEW  NATIVE
PROCESS  PTY  WEBVIEW  NETWORK  BACKGROUND  SECRET  SECURITY_SENSITIVE
```

## 3. Resource Lifecycle（5 态）

```text
ACTIVE → BACKGROUND → SUSPENDED → HIBERNATED → DESTROYED
```

> **不是所有能力都支持所有 lifecycle。** 每个能力在其 manifest 中显式声明 `lifecycle.supported`；
> Runtime 对未声明的转换抛错（`SUSPEND_NOT_SUPPORTED`），由 RT-09 断言覆盖。

---

## 4. 关键问题：「关闭某个功能后，到底能省什么？」

**诚实回答（DECLARED 口径）：**

| 关闭对象 | 可预期释放 | 今晚能否做到 |
|---|---|---|
| Grid | 释放 N 个 native webview + 对应渲染内存（**最大的资源收益点**） | ⬜ TARGET，不可 |
| Browser | 释放 native webview | ⬜ TARGET，不可 |
| Terminal | 终止 PTY 子进程 | ⬜ TARGET，不可（PTY 不能安全冻结） |
| Database | 断开连接、释放连接池 | ⬜ TARGET，不可 |
| Task | 停止后台调度，消除定时唤醒 | ⬜ TARGET，不可 |
| Bookmark | 仅释放少量内存状态（LIGHT） | ✅ 可编排，但**收益很有限** |

**必须向领导说明的两点**：
1. LIGHT 类能力关闭后省的资源**很有限**；真正的资源收益来自 HEAVY / VERY_HEAVY / PROCESS（Grid、Browser、Terminal），而这些今晚**不能物理卸载**。
2. 今晚交付的是**资源分类体系与可见性**，不是"一键省内存"。

---

## 5. Composition Profiles（三套演示口径）

| Profile | 组成 | 说明 |
|---|---|---|
| **Minimal** | workspace, bookmark | 最小可用：文件工作区 + 书签 |
| **Developer** | workspace, browser, terminal, git, database, bookmark | 开发向 |
| **Full** | 全部 18 个已登记能力 | 完整（含 credential/session/plugin 三个设计上不参与组合的） |

**三态必须区分（禁止混淆）**：

```text
CURRENTLY_COMPOSABLE        今晚：无            ← 真正可独立启停
COMPATIBILITY_WRAPPED       bookmark（唯一）      ← 经 Adapter 接入，非物理卸载
TARGET_COMPOSABLE           其余 14 个            ← 目标态，未实现
NOT_COMPOSABLE_BY_DESIGN    credential / session / plugin  ← 安全/常驻/锁定
```

> **禁止**在演示中声称"可以一键切换 Profile"。Profile 是设计口径，不是今晚已实现的按需装配。

---

## 6. 护栏自检（防止报告"说谎"）

```text
CAPABILITY_RESOURCE_REPORT_SELFTEST=PASS (8/8)
  RPT-01 每个能力都有资源分类
  RPT-02 报告不含任何实测数字（MB/GB/KB/%）
  RPT-03 显式声明 measurement NOT_AVAILABLE
  RPT-04 可组合性状态合法
  RPT-05 不得声称 CURRENTLY_COMPOSABLE
  RPT-06 bookmark 必须标为 COMPATIBILITY_WRAPPED
  RPT-07 至少三个 Profile
  RPT-08 registry 本身校验通过（不绕过既有门禁）
```

RPT-02 / RPT-03 / RPT-05 是**防编造**护栏：任何试图在报告里写实测数字或把目标态包装成已实现，都会被自检拦下。

---

## 7. 已登记债务

| ID | 内容 |
|---|---|
| Debt-7A-1 | 无按能力资源实测机制（继承），报告只能给 DECLARED 口径 |
| Debt-7E-1 | Profile 切换未实现：三个 Profile 是设计口径，不能真正按需装配 |
| Debt-7E-2 | HIBERNATED 状态未实现：无能力声明支持，也无资源释放实现 |
| Debt-7E-3 | 资源释放收益未经实测验证（依赖 Debt-7A-1） |

---

## 8. 结论

```text
PHASE_7E_RESULT: PASS（资源分类/生命周期/Profile/Resource Report 齐备，8/8 自检，零编造数字）
```

下一步：**Phase 7F — Preview Release Acceptance + 领导验收材料**。
