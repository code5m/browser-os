# Module Documentation Matrix（Phase D3）

> 机器真源：各 `src/capabilities/<id>/manifest.ts`（C/HP 字段）+ 实际代码（M/RV/D 推导）。
> 本表是索引，细节见各模块 `README.md`（§31 给出 C/HP/M/RV/D 评估与证据）。
> 评级定义（官方，Phase D3 纠正后）：
> C0 REGISTERED · C1 WRAPPED · C2 ISOLATED · C3 OPTIONAL · C4 RUNTIME_CONTROLLABLE · C5 RESOURCE_RELEASABLE
> HP0 STATIC · HP1 RUNTIME_ENABLE_DISABLE · HP2 RUNTIME_REGISTER_UNREGISTER · HP3 RUNTIME_INSTALL_UNINSTALL
> M0 LOGICAL_ONLY · M1 DIRECTORY_ISOLATED · M2 PACKAGE_ISOLATED · M3 INDEPENDENTLY_BUILDABLE · M4 INDEPENDENTLY_VERSIONED · M5 INDEPENDENT_REPOSITORY
> RV0 GLOBAL_CONTEXT_REQUIRED · RV1 OWNER_NAVIGABLE · RV2 BOUNDED_REVIEW · RV3 INDEPENDENT_TEST_SURFACE · RV4 INDEPENDENT_RELEASE_REVIEW
> D0 NO_MODULE_DOC · D1 README · D2 RESPONSIBILITY_BOUNDARY · D3 DOMAIN_CONTRACT_RESOURCE_TEST · D4 MACHINE_LINKED · D5 INDEPENDENT_REVIEW_READY

## 文档化范围（非 Pilot）

| 模块 | README | C | HP | M | RV | D | 物理债务 |
|---|---|---|---|---|---|---|---|
| bookmark | ✅ | C3 | HP2 | M1 | RV1+RV2 | D3 | 无（参考范本） |
| agent | ✅ | C1 | HP0 | M1 | RV1(+RV2弱) | D3 | useAgentStore 在 `src/stores/`（待迁入） |
| database | ✅ | C2 | HP0 | M1 | RV1+RV2 | D3 | useDatabaseStore 在 `src/stores/`（待迁入） |
| browser | ✅ | C3 | HP0 | M1(带债) | RV1(漂移)+RV2(弱) | D3 | useGridArchiveStore 在 `src/stores/` + 4 个 .vue 在 `src/components/browser/` |
| home | ✅ | C2 | HP2 | M1 | RV1(弱)+RV2(强) | D3 | 无 state/UI 债；语义 registry 路径缺失 |
| apps | ✅ | C2 | HP0 | M1 | RV1(弱)+RV2(弱) | D3 | 无 state/UI 债；dependsOn `bridge` 命名债 |
| graph | ✅ | C2 | HP0 | M1 | RV1+RV2(强) | D3 | 无 state/UI 债；persistence 命名张力 |
| clipboard | ✅ | C2 | HP0 | M1 | RV1+RV2 | D3 | clipOpen 视图态留 layout |
| plugin | ✅ | C2 | HP0 | M1 | RV1+RV2(强) | D3 | 运行时 LOCKED；无执行面 |
| task | ✅ | C2 | HP0 | M1 | RV1+RV2(强) | D3 | task-runs 读取缺口 |
| terminal | ✅ | C3 | HP0 | M1 | RV1+RV2(强) | D3 | PTY suspend 不释放 |
| tools | ✅ | C2 | HP0 | M1 | RV1+RV2(强) | D3 | 工具子 webview 零能力隔离 |
| vault | ✅ | C2 | HP2 | M1 | RV1+RV2(间接) | D3 | persistence/maturityEvidence 漂移 |
| settings（SERVICE 候选） | ✅ | C2 | HP2 | M0 | RV1+RV2(间接) | D3 | 候选·非 Capability；store/UI 在体系外 |

## Pilot（本阶段跳过，已有 Pilot README）

| 模块 | 备注 |
|---|---|
| skill | Pilot；不在此文档化范围 |
| git | Pilot；`useGitStore` 物理债务由 Pilot README 覆盖（D3 不触碰） |
| workspace | Pilot；`src/components/workspace/*` 混留 UI 由 Pilot README 覆盖 |

## 门禁

`scripts/check-module-documentation.mjs`（机器可证明事实）：README 存在 + module id + manifest/public 路径 + Source-of-Truth 段 + 引用路径存在；含 positive/negative/self-test。
- 最新运行：`MODULE_DOCUMENTATION: PASS (14 modules documented & verified)`
- `--self-test`：`SELF_TEST: PASS`

## 物理债务汇总（均显式注册，未静默消失）

1. **store 在 `src/stores/`（跨 D3 边界）**：`useAgentStore`、`useDatabaseStore`、`useGridArchiveStore`（browser 域 grid 归档，且为 `gridOpen/gridSession` 的 MULTIPLE_WRITERS）、`useGitStore`（Pilot git，本阶段不触碰）、`useSettingsStore`（settings 候选）。
2. **UI 在 `src/components/`**：`src/components/browser/*`（4 个 .vue）、`src/components/workspace/*`（git/script 混留，Pilot）。
3. **bridge.rs 高度集中**：所有原生命令集中在 `src-tauri/src/bridge.rs` + `main.rs` `generate_handler!`，各能力未单独抽原生模块（所有 README §17 诚实声明）。
4. **语义 registry 漂移**：`02-STATE-SOURCES.md` 仍引用旧 `src/stores/useBrowserStore.ts` 路径；home/apps/settings 的 store 未入 `02-STATE-SOURCES.md`（仅 module-identity.yaml 登记 UI）。
5. **其它漂移**：vault `persistence.scope` none vs runtime_only；vault/agent/database 的 `maturityEvidence` 引用名实不符（通用脚本或非覆盖本模块）。

## 下一步（NEXT）

Semantic Governance Coverage Remediation（消 §02-STATE-SOURCES.md 漂移、补 RV3 vitest、收敛 browser grid 双写、对齐 vault 漂移）。
