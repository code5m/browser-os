# Capability Platform vNext — 交付索引

> 本目录是**索引**，不复制正文。各主题正文复用既有真源文档（避免第二份同义文档）。
> 当前里程碑：tag `capability-platform-vnext-hardening-code-pass`（**CODE PASS**）。
> **HUMAN_VISUAL / Final Human Acceptance = PENDING**，故**无** `capability-platform-vnext-pass`。

| # | 主题 | 真源文档（复用/新建） |
|---|---|---|
| README | 交付索引 | 本文件 |
| 01 | Leader Summary | `docs/architecture/capability-platform/STAGE-J-PLATFORM-VNEXT-CLOSEOUT.md`（结论与矩阵）+ `FINAL-LEADER-DEMO.md` |
| 02 | Architecture | `docs/architecture/capability-platform/STAGE-J-PLATFORM-VNEXT-CLOSEOUT.md`（§1–§10） |
| 03 | Capability Matrix | 同上「FULL PRODUCT CAPABILITY MATRIX（23 条，UNKNOWN=0）」 |
| 04 | Semantic Governance | `docs/architecture/semantic-registry/`（states/intents/owners/side-effects）+ `SCR-script-owner-20260923.md` |
| 05 | UI System | `docs/architecture/ui-system/`（`ui-components.yaml`、`ui-boundary-baseline.json`、`HANDOFF_UI_SYSTEM.md`） |
| 06 | Native Boundary | `docs/architecture/native-boundary/NATIVE_CAPABILITY_BOUNDARY_AUDIT.md` + `native-commands.yaml`（148/148） |
| 07 | Resource Governance | `docs/architecture/capability-platform/RESOURCE-OWNERSHIP-MATRIX.md` + `docs/architecture/capability-registry/resources.yaml` |
| 08 | Dependency Graph | `docs/architecture/capability-registry/dependencies.yaml` + `dependencies.yaml` 审计（见 03 文档 §2） |
| 09 | Composition | `STAGE-J-PLATFORM-VNEXT-CLOSEOUT.md` §5（33/33、11/11、12/12）+ `NAVIGATION-GOVERNANCE.md` |
| 10 | AI Maintainability | `docs/architecture/ui-system/AI_UI_CHANGE_SURFACE_REPORT.md` + 本轮结论（见 11/14） |
| 11 | Known Debt | `docs/delivery/capability-platform-vnext/11-KNOWN-DEBT.md`（新建） |
| 12 | Human Acceptance | `docs/architecture/capability-platform/FINAL-HUMAN-ACCEPTANCE.md`（H01–H15）+ `STAGE-J-HUMAN-ACCEPTANCE-H01-H10.md` |
| 13 | Leader Demo | `docs/architecture/capability-platform/FINAL-LEADER-DEMO.md` |
| 14 | Release Checklist | `docs/delivery/capability-platform-vnext/14-RELEASE-CHECKLIST.md` |

## 本轮硬化（H-A…H-E）一句话结论

- **H-A** script 语义收口：workspace 子能力，owner `useScriptStore`，GOVERNED（未错误升格为独立能力）。
- **H-B** 能力契约漂移：新增 `check-capability-contract-drift.mjs`，**已接入 `npm run check`**，fail=0。
- **H-C** native 边界：148/148 命令有 owner，UNKNOWN=0；Shell 直连重资源被修（App.vue → `browser.closeBrowser()`）。
- **H-D** 资源治理：21 项资源，**MEASURED=0 / RUNTIME_OBSERVED=4 / STRUCTURAL=7 / DECLARED=10 / UNKNOWN=0**；C5 不虚标。
- **H-E** NOT_INTEGRATED 6 项全部有理由（MUST_MIGRATE=0、UNKNOWN=0）+ 导航治理（渲染已贡献驱动）。
