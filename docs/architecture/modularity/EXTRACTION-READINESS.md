# Extraction Readiness Matrix（抽取就绪度矩阵）

> Phase 1 交付物（手册 §38 / §49）。
> 真值快照 HEAD：`8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`。
>
> **判定口径**（手册 §38）：基于 OWNER_UNIQUE / PUBLIC_CONTRACT / DEPENDENCY_STABILITY /
> RESOURCE_BOUNDARY / TEST_SURFACE / REVIEW_SURFACE / DOCUMENTATION_MATURITY /
> BUILD_INDEPENDENCE / VERSIONING_NEED / REUSE / TEAM_BOUNDARY。
> 等级：NOT_READY / DIRECTORY_READY / PACKAGE_READY / REPOSITORY_CANDIDATE。
> 本仓**无机器字段记录**，下表人工派生，未证实项标 `UNKNOWN`。

---

## 汇总

| 等级 | 模块 |
|---|---|
| DIRECTORY_READY | bookmark, database, git, task, clipboard, apps, tools, graph, skill, plugin, agent, vault, terminal |
| NOT_READY | browser, grid, workspace, credential, resource_collection, session, script, workbench, settings, home |
| PACKAGE_READY | 0（全仓无 M2 物理隔离） |
| REPOSITORY_CANDIDATE | 0（手册 §40 默认禁止新建仓库） |

---

## 逐模块判定

| 模块 | Readiness | 理由（证据） |
|---|---|---|
| bookmark | DIRECTORY_READY | 单一 owner(已进 Registry) + 窄契约 + LIGHT 资源 + 目录隔离；缺 README/测试面(D0/RV1) |
| database | DIRECTORY_READY | C2 + 窄契约(view='db') + 目录隔离；store 未迁入、缺 absence 门禁 |
| git | DIRECTORY_READY | C2 + 目录隔离 + checker 固化；依赖 credential/workspace 稳定 |
| task | DIRECTORY_READY | C2 + BACKGROUND 资源边界清晰 + 目录隔离 |
| clipboard | DIRECTORY_READY | C2 + session 态不落盘 + 目录隔离 |
| apps | DIRECTORY_READY | C2 + detached 子进程不归本能力 + 目录隔离 |
| tools | DIRECTORY_READY | C2 + tool:// 子 webview 边界 + 目录隔离 |
| graph | DIRECTORY_READY | C2 + 只读快照零副作用 + 目录隔离 |
| skill | DIRECTORY_READY | C1 + LIGHT + 目录隔离；activatable=false |
| plugin | DIRECTORY_READY | C2 + 目录隔离；runtime LOCKED（无 owned 资源实例） |
| agent | DIRECTORY_READY | C1 + 目录隔离；后端未实现（仅只读壳） |
| vault | DIRECTORY_READY | 目录隔离 + 只读快照零原生资源 |
| terminal | DIRECTORY_READY | 已收口独立 owner + PTY 边界清晰；PTY 不可冻结为已知约束 |
| browser | NOT_READY | 与 grid 深度耦合(Debt-7B-1) + webview 不可物理卸载 + 无 README/测试面 |
| grid | NOT_READY | 非独立目录 + 共享 browser owner + VERY_HEAVY 多 webview |
| workspace | NOT_READY | 子域分散(5 owner) + 物理未完全抽离(files/artifact/repo/script/snippet) |
| credential | NOT_READY | Rust owner + 安全边界 + 前端零明文；抽取需安全审计 |
| resource_collection | NOT_READY | M0（store 在 src/stores）+ NOT_INTEGRATED |
| session | NOT_READY | M0 + 关停链路 + NOT_INTEGRATED |
| script | NOT_READY | M0（物理在 workspace/ui）+ NOT_INTEGRATED |
| workbench | NOT_READY | M0 + NOT_INTEGRATED |
| settings | NOT_READY | FRAMEWORK_SERVICE 非用户可组合能力；驻 src/settings |
| home | NOT_READY | 跨 3 能力依赖 + 窄契约未充分文档化 |

---

## 诚实缺口

- TEST_SURFACE / REVIEW_SURFACE / DOCUMENTATION_MATURITY 三项本仓全缺（D0 / 无 RV2），
  故即便 DIRECTORY_READY 也**未达 PACKAGE_READY**（手册 §39 要求 docs≥D3 + 独立测试面 + review surface 有限）。
- 提升路径：先完成 Pilot README(D1→D3) + 固化契约 + 建测试面，再评估 PACKAGE_READY。
- 本 Phase 1 **不做任何物理抽取**（红线）。
