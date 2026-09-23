# Skill Capability — Full-Stack Boundary（Capability Library Expansion v1, STAGE E）

> STAGE E 产物。物理：SkillManagerPanel 迁入 `src/capabilities/skill/ui/`；
> **关键解耦**：Skill truth 从 `useAgentStore` 迁出至专属 `useSkillStore`（`src/capabilities/skill/state/`），
> 安装/确认二段式闸门抽出为中性协调器 `useInstallConfirmStore`；Skill 与 Agent 不再纠缠。
> MainArea 只按槽渲染、不 import 能力内部。最后更新：2026-09-23。

## 成熟度（诚实，不谎报）

**C1 WRAPPED**（status=`COMPATIBILITY_WRAPPED`，manifest.v1.maturity=`C1`）。

评级依据：
- 边界隔离已完成：manifest/public/index/ui/state 五段边界；**语义 owner `useSkillStore` 专属**（不再被 Agent 持有，§21 满足）；
  中性安装/确认协调器 `useInstallConfirmStore`；MainArea 不再静态 import SkillManagerPanel（贡献驱动）。
- **功能缺口（非边界缺失）**：执行后端 `skill_run` / `skill_install` / `skill_list` 在 Rust 侧**未实现**
  （`AGENT_SKILL_COMMANDS_AVAILABLE=false` → `guard()` 拦截，前端不 invoke）。仅只读命令就绪
  （`skill_parse` / `skill_validate` / `skill_permission_preview`）。当前 Skill 为「只读壳」。

## Full-Stack Trace

```text
Skill UI (capabilities/skill/ui/SkillManagerPanel.vue)
  ↓ OWNED_BY_CAPABILITY（经 public.ts 消费语义 owner）
Skill State Owner: useSkillStore (id="skill", src/capabilities/skill/state/useSkillStore.ts)
  ↓ 意图（intents）
  loadSkills() / selectSkill() / installSkill() / validateSkill()
  ↓ PUBLIC_DEPENDENCY（bridge，AGENT_SKILL_COMMANDS_AVAILABLE=false）
Skill Adapter: src/bridge.ts → skillList?/skillInstall?/skillRun?（未注册）
  ↓ NATIVE_ADAPTER（Rust，仅只读）
src-tauri/src/bridge.rs:
  skill_parse          ← 已实现
  skill_validate       ← 已实现
  skill_permission_preview ← 已实现
  skill_list/run/install ← 未注册（guard 拦截）
安装闸门：
  installSkill → useInstallConfirmStore.setPending("install_skill") → PermissionPreviewModal → ackConfirm
  （中性协调器，agent/skill 共用，不再让任一能力持有对方 truth）
```

## 依赖关系（§21 解耦）

- dependsOn: `bridge`（manifest 已声明）。**不再依赖 `agent`**（STAGE E 解除纠缠）。
- Skill 与 Agent 为独立能力，共用：① readonly 命令族（skill_parse/validate/preview 等）；② 中性安装/确认协调器 `useInstallConfirmStore`。
- **无第二真源**：`useSkillStore` 是 Skill 唯一语义 owner；`useInstallConfirmStore` 是中性共享 infra（非能力）。

## Absence Behavior（§18/§29）

- Skill absent（profile 未列，如 `minimal`）→ `registerSkillContributions` 不运行
  → `WORKBENCH_MAIN` 槽中无 `view='skills'` 贡献 → MainArea `viewOf('skills')` 返回 `undefined` → 不渲染。
- Shell 不崩溃：通用 `viewOf` 分支对未知 view 返回 undefined 自然跳过。
- 已知缺口：mainView='skills' 导航项未贡献驱动 → absent 点该导航落空视图（不崩溃，UX 债务）。

## Legacy Debt（诚实，不静默消失）

1. 执行后端（skill_run/install/list）Rust 未实现 —— 真实功能缺口。
2. `useSkillStore` 物理在 `src/capabilities/skill/state/`（已就位，无迁移动作）；`useAgentStore` 仍物理在 `src/stores`。
3. 无 skill 专属 absence 运行时门禁。
4. `mainView='skills'` 导航项硬编码于 `useLayoutStore`/`homeUi`/`HomeLaunchers`（未贡献驱动）。

## SECOND_TRUTHS = 0

`public.ts` 仅再导出 `useSkillStore`（语义 owner），未创建 `runtime.skillOpen` / `skillVisible` 镜像状态。
DOMAIN STATE（useSkillStore）≠ CAPABILITY COMPOSITION STATE ≠ UI LOCAL STATE ≠ RESOURCE RESULT。
