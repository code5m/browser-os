# 11 — KNOWN DEBT（分类：BLOCKING / NON_BLOCKING / FUTURE）

> **BLOCKING = 0** ⇒ 允许 `capability-platform-vnext-hardening-code-pass`。
> 每项含 ID / 描述 / 领域 / 风险 / 为何未修 / 发布影响 / 下一步。

## BLOCKING

（无）

## NON_BLOCKING

| ID | DESCRIPTION | AREA | RISK | WHY_NOT_FIXED | RELEASE_IMPACT | NEXT ACTION |
|---|---|---|---|---|---|---|
| Debt-7A-1 | 无按能力的资源实测机制（内存/CPU/webview 实例数未测量） | Resource | 中：资源结论只能 DECLARED/STRUCTURAL | 需 OS/运行时探针基建，超出本轮「焊牢」范围 | 不影响发布；矩阵已诚实标注证据等级 | FUTURE：接入 `measure-resources.mjs` 到 CI 并产出基线 |
| Debt-J-1 | `workspace/ui/FileEditor.vue` 与 browser 的依赖口径在检查器间不一致（UI-04b-U warn） | Dependency/UI | 低：渲染与真源未受影响 | 检查器口径（internal vs public）差异，需统一语义 | 无功能影响 | 统一 public/internal 判定口径后清 warn |
| Debt-J-2 | `workspace` manifest 与 capabilities.yaml 曾漂移（本轮已对齐 optional browser） | Metadata | 低 | 已由 drift 门禁阻断 | 已收敛 | 保持门禁常开 |
| Debt-H-1 | 148 个 native 命令仍集中在 `src-tauri/src/bridge.rs`，未按能力拆原生适配器 | Native | 中：物理边界未达代码级 | 大搬迁风险高，本轮先做「归属真源 + 门禁」 | 治理已可判定（UNKNOWN=0） | FUTURE：按能力拆 `src-tauri/src/capabilities/<id>/` |
| Debt-H-2 | 导航入口清单仍硬编码（`useLayoutStore.activate*` / `ActivityBar.menuSections` / MainView 枚举） | Navigation | 低：渲染链路已贡献驱动，仅入口清单 | 改动涉及快捷键/全局动作，风险 > 收益 | 无死视图（渲染已贡献化） | FUTURE：导航贡献化专项 |
| Debt-H-3 | Database/Plugin/Agent absent 无运行时资源探针（仅 STRUCTURAL/DECLARED） | Resource | 中：缺席结论未实测 | 需对应 runtime probe | 已诚实标注，未高报 | 随 Debt-7A-1 一并解决 |
| Debt-C5 | 无能力可声称 C5 RESOURCE_RELEASABLE（destroy 入口存在 ≠ C5） | Resource/Maturity | 低：未虚报 | 需「存在→销毁→不再存在」实测 | 成熟度保持真实 | 有实测后按证据升级 |
| Debt-HUMAN | HUMAN_VISUAL / Final Human Acceptance = **PENDING** | Acceptance | — | 未执行人工目视 | **禁止** Human PASS tag | 执行 FINAL-HUMAN-ACCEPTANCE.md H01–H15 |

## FUTURE

| ID | DESCRIPTION | AREA | NEXT ACTION |
|---|---|---|---|
| F-1 | `resource_collection` 无前端能力包（遥测由后端承担） | Capability | 若需用户可见资源面板再建包 |
| F-2 | `script` 无独立代码包（workspace 托管，owner 已收口） | Capability | 保持 NOT_INTEGRATED；若拆包再升格 |
| F-3 | `grid` 与 browser 共 owner、无独立包 | Capability | 保持子能力登记 |
| F-4 | native 物理拆分（见 Debt-H-1） | Native | 与 Debt-H-1 合并推进 |
| F-5 | 导航贡献化（见 Debt-H-2） | Navigation | 与 UI-03 债务同族处理 |

## 债务政策

Known Debt **不是垃圾桶**：每项必须有 ID / 描述 / 领域 / 风险 / 未修原因 / 发布影响 / 下一步；
BLOCKING 必须清零才能发 RC（当前 **0**）；NON_BLOCKING 可保留但必须显式列出并定期复审。
