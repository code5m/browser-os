# FINAL-HUMAN-ACCEPTANCE（H01–H15）

> 状态：**READY — 待人工执行**（本文件**不**启动验收，也不声称已通过）。
> 前置：tag `capability-platform-vnext-hardening-code-pass`（CODE PASS）。
> **HUMAN_VISUAL = PENDING**；人工验收未执行前，**禁止**创建 `capability-platform-vnext-pass`。
>
> 复用：`docs/architecture/capability-platform/STAGE-J-HUMAN-ACCEPTANCE-H01-H10.md` 已覆盖 **H01–H10**
> （Workbench/UI Preservation、Home、Workspace、Browser、Grid HIDE/DESTROY、Terminal、Bookmark HP2、
> Git、Database、Agent），本文件**不重复**，仅补齐 **H11–H15** 并给出统一 FAIL 模板。

## 通用格式（每项）

- **USER ACTION**：人工操作
- **EXPECTED UI**：预期界面
- **EXPECTED STATE**：预期状态真值
- **MACHINE EVIDENCE**：可复跑的机器旁证
- **PASS CONDITION**：通过条件
- **FAIL TEMPLATE**：失败记录格式

**FAIL TEMPLATE**
```text
[H??] FAIL
step:        <具体哪一步>
observed UI: <实际界面>
observed state: <实际状态/值>
machine evidence: <门禁命令 + 实际输出>
repro:       <commit sha + profile/切换序列>
severity:    BLOCKING / NON_BLOCKING
```

---

## H11 — Skill

- **USER ACTION**：进入「技能」视图；查看/校验一个 skill；再卸载/禁用该能力后重进。
- **EXPECTED UI**：技能列表与权限预览正常；**卸载后侧栏/导航项消失，主区不空白、不报错**。
- **EXPECTED STATE**：`skill` owner = `useSkillStore`；absent 时无 skill 运行时对象。
- **MACHINE EVIDENCE**：`node scripts/check-capability-registry.mjs`（fail=0）；`npm run check`（EXIT=0）；`runtime-resource-absence.mjs`（12/12）。
- **PASS CONDITION**：卸载后无死视图、无报错；门禁全绿。

## H12 — Plugin / Task / Graph

- **USER ACTION**：分别进入插件 / 定时任务 / 知识图谱；执行查看类操作；再分别卸载后重进。
- **EXPECTED UI**：三视图正常；卸载后对应导航项消失，主区不空白。
- **EXPECTED STATE**：`plugin`(HEAVY/NATIVE, runtime LOCKED)、`task`(BACKGROUND)、`graph`(MEDIUM) 各自 owner 单一；absent 时无对应运行时。
- **MACHINE EVIDENCE**：`check-capability-composition.mjs`（33/33）；`check-composition-profiles.mjs`（11/11）。
- **PASS CONDITION**：三者可独立 absent 且不留死页签。

## H13 — Clipboard / Apps / Tools

- **USER ACTION**：剪贴板历史查看；应用列表与启动；工具子 webview 打开；分别卸载后重进。
- **EXPECTED UI**：三者正常；卸载后导航项消失。
- **EXPECTED STATE**：clipboard **不落盘**（会话态）；apps 启动为 detached 子进程（不由能力回收）；tools 为 `tool://` 子 webview。
- **MACHINE EVIDENCE**：`node scripts/check-clipboard-persistence-logic.mjs`（若存在）/`npm run check`；native 门禁 NATIVE-01..07（fail=0）。
- **PASS CONDITION**：剪贴板重启后为空（符合 B11-1）；卸载后无残留面板。

## H14 — Resource Absence + Shutdown

- **USER ACTION**：以 framework-only 装配启动（或卸载 Browser/Terminal）；观察宫格与终端入口；随后正常关闭应用。
- **EXPECTED UI**：无 Browser → 无宫格/无 webview 入口；无 Terminal → 无终端页签与 PTY。
- **EXPECTED STATE**：**grid-child = 0、PTY = 0**（RUNTIME_OBSERVED）；Browser absent → 无 browser 重资源。
- **MACHINE EVIDENCE**：`node scripts/runtime-resource-absence.mjs` → RRA-01（framework createGrid 0 次）、RRA-02（gridOpen 恒 false）、RRA-03（PTY 0 次）、RRA-07（closeGrid = DESTROY）。
- **PASS CONDITION**：RRA-01/02/03 全 PASS；关闭流程无挂起进程报错。

## H15 — Persistence / Error Handling / Restart

- **USER ACTION**：修改设置（主题/键位/休眠）与主页快捷方式 → 重启应用；再制造一次错误（打开不存在路径/断网 DB 查询）。
- **EXPECTED UI**：设置与主页数据持久；错误为**可读提示**且**不泄露路径/凭据**；UI 不空白。
- **EXPECTED STATE**：settings 经 `src/settings/public.ts` 契约；home 经 `capabilities/home`；错误不回显敏感串。
- **MACHINE EVIDENCE**：`check-sensitive-side-effects.mjs`（fail=0）；`check-home-client-policy.py`（invariants hold）；`check-home-ui-logic.mjs`（41/0）。
- **PASS CONDITION**：重启后持久化一致；错误提示脱敏；门禁全绿。
