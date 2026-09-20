# 04 — Building Block Contract v1

Schema 实现：`src/capability/platform/contract.ts`（纯类型 + 纯校验，零副作用）。
校验器：`validateManifestV1()`，由 `check-capability-platform.mjs` PLT2-02 对每个能力执行。

## 字段

| 组 | 字段 |
|---|---|
| 身份 | `id`（kebab/dot）、`version`（semver）、`displayName`、`description` |
| 成熟度 | `maturity`（C0–C5）、`maturityEvidence`（**必须**给出支撑的 checker/测试名） |
| 依赖 | `dependencies`（强）、`optionalDependencies`（降级）、`conflicts` |
| 提供 | `provides`、`requires`、`permissions`、`publicContract[{name, locator}]` |
| UI | `contributions[{id, slot, type, view?}]` |
| 资源 | `resources[{kind, ownership, evidence?}]` |
| 持久化 | `persistenceScope`、`persistenceSensitive` |
| 策略 | `activationPolicy`、`deactivationPolicy`、`installPolicy`、`uninstallPolicy`、`hotPlug` |
| 归属 | `entrypoint`、`semanticOwner` |

明确禁止：manifest 中不得嵌入 secret。

## 校验规则（违反即 FAIL）

- 结构与类型：id/version/maturity/HP 取值合法，集合字段为 string[]。
- 依赖自洽：`dependencies ∩ conflicts = ∅`，禁止自依赖，禁止重复依赖。
- **HP 自洽（反伪造）**：HP0 不得声明任何运行时操作；HP1 只允许 enable/disable；HP2 不得声明 install；
  HP3 必须 allow install；**非 HP3 必须给出 `limitationReason`**。
- **成熟度自洽**：声称 C4 必须给 evidence；声称 C5 的 evidence 必须含 release/destroy/evidence 字样。
- 必填：至少一个 contribution；entrypoint 非空。

## 为什么不允许伪造

契约里的 `hotPlug.level` 直接决定 UI/编排是否允许用户「加入/移除」。
若把 HP0 写成 HP2，用户会在运行时拔掉一个状态不可回收的能力 → 泄漏。
因此校验器把「声明的能力」与「是否真的允许」做一致性校验（PLT2-18）。
