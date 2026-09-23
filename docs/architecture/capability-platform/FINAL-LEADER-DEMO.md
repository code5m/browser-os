# FINAL-LEADER-DEMO（3–5 分钟）

> 面向决策者：**展示平台能力与治理结果**，不展示几千行 checker 输出。
> 每个演示点给出「一句话结论 + 一个可点动作 + 一句机器旁证」。

| # | 演示点 | 操作 | 结论 / 旁证 |
|---|---|---|---|
| 1 | **Workbench UI 保持成熟** | 正常浏览主页 / 文件 / 浏览器 / 设置 | 迁移未改坏 UX；UI 门禁 `UI_BOUNDARIES PASS (fail=0, vacuous=0)` |
| 2 | **Capability Catalog** | 打开 `docs/architecture/capability-registry/capabilities.yaml` | 23 条能力，每条有 owner / 状态 / 资源 / 依赖；**UNKNOWN = 0** |
| 3 | **Full-stack Capability 范例（Git）** | 进入 Git 视图，做一次 status/diff | 能力包 = manifest + public + state + ui + contribution；`check-capability-composition 33/33` |
| 4 | **贡献驱动 UI** | 卸载 Bookmark → 观察侧栏与主区 | 导航项与面板**自动消失**，无死页签；`check-composition-profiles 11/11` |
| 5 | **Bookmark HP2** | 运行时 enable/disable Bookmark | 唯一真实可编排试点；`check-capability-platform 29 PASS` |
| 6 | **Browser absent → 无 Grid 资源** | framework-only 启动 | grid-child **= 0**（RRA-01 `createGrid` 0 次、RRA-02 `gridOpen` 恒 false） |
| 7 | **Terminal absent → 无 PTY** | framework-only 启动 | PTY 创建 **0 次**（RRA-03），无终端子进程 |
| 8 | **Native 命令全部有 owner** | 打开 `docs/architecture/native-boundary/NATIVE_CAPABILITY_BOUNDARY_AUDIT.md` | **148/148** 条 `#[tauri::command]` 有归属，**UNKNOWN = 0**；越权直连被 `check-native-capability-boundaries` 判 FAIL |
| 9 | **如何新增一个 Capability** | 讲流程：manifest → state/ui → public → contribution → registry/profiles | **无需改 App.vue / MainArea 业务 switch**（本夜 vault/home/settings 三例亲证） |
| 10 | **诚实：仍是 Known Debt** | 打开 `docs/delivery/capability-platform-vnext/11-KNOWN-DEBT.md` | 无 BLOCKING；NON_BLOCKING/FUTURE 明确列出（native 未物理拆分、无按能力实测、导航入口清单未贡献化、HUMAN_VISUAL PENDING） |

## 收口一句话

> 平台已从「目录整理」变成**可机器判定的治理体系**：能力元数据不漂移、native 不再是盲区、
> 资源有 owner、缺席能力不创建资源；**人工目视验收（H01–H15）仍待执行**，
> 因此当前只是 **code-pass**，不是 Human PASS。
