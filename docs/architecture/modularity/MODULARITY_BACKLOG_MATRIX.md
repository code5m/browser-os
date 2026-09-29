# MODULARITY_BACKLOG_MATRIX — Train Wave 1 (Autonomous Modularity Migration Train)

> 真源：`docs/architecture/capability-registry/capabilities.yaml` + `docs/architecture/semantic-registry/states.yaml` + 本仓 live code audit（train base `044d4a4`）。
> 本文件是**当前事实**矩阵，不回写历史文档；只更新 CURRENT 事实。

## TRAIN METADATA
- TRAIN_BASE_SHA: `044d4a4b812f4f1a60f02f651aa92bd179827d24`
- TRAIN_BRANCH: `codex/modularity-train-wave1`
- TRAIN_WORKTREE: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3-modularity-train-wave1`
- MAIN_WORKTREE_POLICY: `EXTERNAL_INTEGRATION_WORKTREE` — 其未提交内容（Clipboard M2 WIP / TS Agent WIP / docs-scripts / 其它 lane WIP）一律 DO_NOT_MODIFY/STAGE/COMMIT/STASH/RESET/CHECKOUT/CLEAN。
- NATIVE_FROZEN: TRUE（Frontend M1/M2 不得改 Native，除非真实 package contract 需要；148 command truth 保持 REGISTERED=DEFINED=REGISTRY）。
- GENERIC_CHECKER_STATUS: `WAIT_FOR_CLIPBOARD_INTEGRATION`（train base 仅 `@browser-os/capability-vault` 一个已提交 M2 package；第二份 Clipboard checker 尚未进入 git baseline，禁止从主树复制未提交 checker 制造 Rule of Two）。

## WAVE 1 决策（live audit：bookmark / apps / tools / home）
重判准则：BOUNDARY_GAIN、AI_REVIEW_GAIN、HOST_COUPLING、PACKAGE_VALUE、MIGRATION_COST。目标不是提高 M2 数量。

| module | owner | CURRENT_M | TARGET_M | C | M1_READY | M2_READY | RISK | DECISION | 依据 |
|---|---|---|---|---|---|---|---|---|---|
| bookmark | useBookmarkStore | M1(DIRECTORY_ISOLATED) | M1 | C3 | ✓ | ✗(churn) | LOW | **KEEP_M1** | 已含 contracts/intents/lifecycle/resource；public.ts 纯再导出；无深 Host 耦合；打包=高成本零边界增益 |
| apps | useAppsStore | M1 | M1 | C2 | ✓ | ✗(churn) | LOW | **KEEP_M1** | 目录隔离+public 再导出；仅 `AppEntry` 来自 Host 全局类型；打包成本高增益低 |
| tools | useToolsStore | M1 | M1 | C2 | ✓ | ✗(churn) | LOW | **KEEP_M1** | 同 apps；仅 `ToolMeta` 来自 Host 全局类型 |
| home | useHomeStore | M1 | M1 | C2 | ✓ | ✗ | LOW | **KEEP_M1** | 跨能力真实消费者 workspace.favoriteDirectory；先前已判 KEEP_M1；自注册反模式待 M1 一致性收口 |

> 4 模块均已是 M1=DIRECTORY_ISOLATED；本波为**权威核定**（ratify），非新迁移。

## ACTIVE_LANE_EXCLUSION_SET（生成自 11 个 m5-w18 lane worktree 实时 diff 证据）
- 证据：11 个 `m5-w18-a1..a11` worktree 的未提交 diff **全部为 logs/research 报告，零产品代码**（W18-R 研究态）。
- 防御性排除（指令 #6，禁止 Train 修改）：
  `agent` `skill` `plugin` `task` `graph` `git` `database` `terminal`
  以及任何 m5-w18 活跃 lane 正在修改的文件。
- 本 Wave 候选（bookmark/apps/tools/home）均不在此集，无文件冲突 → 安全核定。

## 全模块 MODULARITY_BACKLOG_MATRIX
列：owner / CURRENT_M / TARGET_M / C / M1_READY / M2_READY / RISK / DECISION
（C = maturity；M1_READY=R 目录隔离+owner 清晰+public 清晰；M2_READY=P 满足 M1+host 耦合有界+可验证）

| module | owner | CURRENT_M | TARGET_M | C | M1_READY | M2_READY | RISK | DECISION |
|---|---|---|---|---|---|---|---|---|
| vault | useVaultStore | **M2(committed)** | M2 | C2 | ✓ | done | — | DONE |
| clipboard | useClipboardStore | M2(in-progress,uncommitted) | M2 | C2 | ✓ | 待 A0 收口 | — | DONE→WIP(主树,不归 Train) |
| bookmark | useBookmarkStore | M1 | M1 | C3 | ✓ | ✗ | LOW | KEEP_M1 |
| apps | useAppsStore | M1 | M1 | C2 | ✓ | ✗ | LOW | KEEP_M1 |
| tools | useToolsStore | M1 | M1 | C2 | ✓ | ✗ | LOW | KEEP_M1 |
| home | useHomeStore | M1 | M1 | C2 | ✓ | ✗ | LOW | KEEP_M1 |
| task | useTaskStore | M1 | M2 | C2 | ✓ | △ | MED | DEFER(车道A7) |
| graph | useGraphStore | M1 | M2 | C2 | ✓ | △ | MED | DEFER(车道A7) |
| git | useGitStore | M1 | M2 | C2 | ✓ | △ | MED | DEFER(车道A7 native) |
| database | useDatabaseStore(仍在 src/stores) | M1 | M2 | C2 | △未迁state | △ | MED | DEFER(车道A5) |
| agent | useAgentStore | M1+in-flight WIP | M2 | C1 | △WIP中 | ✗ | HIGH | DEFER(车道+WIP) |
| skill | useSkillStore | M1 | M2 | C1 | ✓ | △ | MED | DEFER(与agent关联) |
| plugin | usePluginStore | M1 | M2 | C2 | ✓ | △ | MED | DEFER(车道A7 W13/14) |
| terminal | useTerminalStore | M1 | M2 | C2 | ✓ | △ | HIGH | DEFER(native重/M3) |
| browser | useBrowserStore | M1 | M2 | — | ✓ | △ | HIGH | DEFER(中央/webview) |
| workspace | useWorkspaceStore | M1(中央hub) | M2 | — | ✓ | △ | HIGH | DEFER(中央枢纽) |
| grid | useBrowserStore | M0(src/components,NOT_INTEGRATED) | — | — | ✗ | ✗ | HARD_STOP | HARD_STOP_REQUIRED(冻结语义) |
| credential | KeyringStore | native NOT_INTEGRATED | — | — | ✗ | ✗ | HARD_STOP | HARD_STOP_REQUIRED(安全边界) |
| session | useSessionStore | M0(src/stores) | M1 | — | ✗ | ✗ | MED | KEEP(infra store,非 capability) |
| workbench | useWorkbenchStore | M0(src/stores) | M1 | — | ✗ | ✗ | MED | KEEP(infra store,非 capability) |
| script | useScriptStore | M0(workspace/ui 子) | M1 | — | ✗ | ✗ | MED | KEEP(Workspace 子能力) |
| settings | useSettingsStore | M1(SERVICE,src/settings) | — | — | ✓ | ✗ | LOW | KEEP(SERVICE,非 capability) |

## 备注 / KNOWN_DEBT
- KEEP_M1 模块的公共一致性债：home `index.ts` 在模块加载时自注册贡献（`registerHomeContributions()` 直接调用），而 bookmark/apps/tools 仅 export `registerXContributions()` 不调用——属贡献注册模式不一致，列为后续 M1 一致性收口项（非阻塞，待下一不冲突批次）。
- 冻结：grid（Browser/Grid frozen semantics）、credential（安全/ACL 边界）→ 不得 Train 触碰。
- Native 全冻结；Frontend M1/M2 不修改 Native。

## NEXT BATCH
- 下一批不冲突工作待定：依赖 Clipboard M2 被 A0 集成进 Train 可用 base（解锁 GENERIC_CHECKER 真实 duplication proof），或活跃 lane 释放其占用模块（agent/skill/plugin/task/graph/git/database/terminal）后方可进入 M2。
- 当前可立即做的低风险 M1 一致性收口（不冲突）：统一 4 模块贡献注册模式（消除 home 自注册反模式）。
