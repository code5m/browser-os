# Semantic Coverage Matrix（Phase C — audit baseline）

> 机器真源：`docs/architecture/semantic-registry/{states,intents,owners,side-effects}.yaml`
> 生成依据：真实 registry 统计 + `src/capabilities/*` 目录盘点（无虚构治理对象）。
> 目标：`UNKNOWN = 0`；`FULLY_GOVERNED` 不人为提高；`OWNER_DECLARED_ONLY` 归零或显式裁决。

## 能力模块（src/capabilities/*，16 个目录）

| MODULE_ID | DOMAIN | DECLARED_SEMANTIC_OWNER | OWNER_REGISTERED | STATES | INTENTS | SIDE_EFFECTS | LOCATORS_VALID | PUBLIC_CONTRACT | STATUS |
|---|---|---|---|---|---|---|---|---|---|
| agent | agent | useAgentStore | yes | 11 | 6 | yes (agentInstall/agentChat) | yes | yes | FULLY_GOVERNED |
| apps | apps | useAppsStore | yes | 4 | 2 | yes (appLaunch) | yes | yes | FULLY_GOVERNED |
| bookmark | bookmark | useBookmarkStore | yes | 6 | 5 | yes (legacy via call_sites) | yes | yes | FULLY_GOVERNED |
| browser | browser+grid | useBrowserStore | yes (FROZEN) | 6 | 7 | yes (grid native) | yes | yes | FULLY_GOVERNED (frozen, Phase E 保护) |
| clipboard | clipboard | useClipboardStore | yes | 2 | 5 | yes (clipboardWrite) | yes | yes | FULLY_GOVERNED |
| database | database | useDatabaseStore | yes | 21 | 4 | yes (dbConnect/dbQuery) | yes | yes | FULLY_GOVERNED |
| git | git | useGitStore | yes | 27 | 3 | yes (gitWrite) | yes | yes | FULLY_GOVERNED |
| home | home | useHomeStore | yes | 6 | 7 | yes (vaultOpen? n/a) | yes | yes | FULLY_GOVERNED |
| skill | skill | useSkillStore | yes | 9 | 3 | yes (skillInstall) | yes | yes | FULLY_GOVERNED |
| terminal | terminal | useTerminalStore | yes | 11 | 8 | yes (spawn) | yes | yes | FULLY_GOVERNED |
| tools | tools | useToolsStore | yes | 3 | 2 | yes (toolOpen) | yes | yes | FULLY_GOVERNED |
| vault | vault | useVaultStore | yes | 14 | 4 | yes (vaultOpen) | yes | yes | FULLY_GOVERNED |
| graph | graph | useGraphStore | yes | 19 (8 stored + 11 derived; nodes/edges 为 graph.rs 投影缓存) | 10 | yes (graphQuery/graphNodeGet/graphStats 读型) | yes | yes | FULLY_GOVERNED（SCR-Final-3：真实 capability；数据真源 graph.rs，前端投影缓存） |
| plugin | plugin | usePluginStore | yes | 7 (6 stored + 1 derived; busy 为 useBookmarkStore 共享键) | 9 | yes (pluginList/Get/Install/Enable/Disable/Keys*) | yes | yes | FULLY_GOVERNED（SCR-Final-3：五态机真源在原生，前端投影） |
| task | task | useTaskStore | yes | 11 | 6 | yes (taskAdd/taskUpdate/taskRemove/taskRunNow) | yes | yes | FULLY_GOVERNED |
| workspace | workspace | useWorkspaceStore | yes | 1 (audit; recents 与 home.recents 同名碰撞，留 observed) | 5 | yes (auditLog 读型) | yes | yes | FULLY_GOVERNED（SCR-Final-3：Core 仅 audit+recents+编排，子域各自 owner，防 God Store 回归） |

## Framework / Service 域（非 capability 模块，但已注册）

| MODULE_ID | DOMAIN | OWNER | STATUS |
|---|---|---|---|
| view_navigation | navigation | useLayoutStore | FULLY_GOVERNED |
| browser_grid_lifecycle | grid | useBrowserStore (FROZEN) | FULLY_GOVERNED |
| native_execution | native | bridge/Rust | FULLY_GOVERNED |
| browser_tabs | tabs | useBrowserStore | FULLY_GOVERNED |
| files | file IO | useFileStore | FULLY_GOVERNED |
| artifact | artifact | useArtifactStore | FULLY_GOVERNED |
| repo | repo | useRepoStore | FULLY_GOVERNED |
| script | script | useScriptStore | FULLY_GOVERNED |
| snippet | snippet | useSnippetStore | FULLY_GOVERNED |
| credential | credential | credential | FULLY_GOVERNED |
| settings | settings | useSettingsStore | NOT_APPLICABLE（Phase H：保持 SERVICE，不升格） |
| session | session | useSessionStore | FRAMEWORK SERVICE（NOT_APPLICABLE） |
| resource | resource | useResourceStore | FRAMEWORK SERVICE（NOT_APPLICABLE） |
| workbench | workbench | useWorkbenchStore | FRAMEWORK SERVICE（NOT_APPLICABLE） |
| layout | layout | useLayoutStore | FULLY_GOVERNED（view_navigation 覆盖） |

## 计数汇总（audit baseline）

- TOTAL_MODULES（capability + framework）：16 + 15 = 31
- FULLY_GOVERNED：16（capability）+ 10（framework，不含 settings/session/resource/workbench 4 个 NOT_APPLICABLE）= 26
- OWNER_PENDING_SCR：0（graph/plugin/workspace 已据实裁决 FULLY_GOVERNED）
- UNGOVERNED：0
- NOT_APPLICABLE：settings、session、resource、workbench = 4
- UNKNOWN：0
- REGISTERED_STATES：按 owner 聚合（去重后）≈ 204（含 graph 19 + plugin 7 + workspace 1 + 历史）
- REGISTERED_INTENTS：106（含 graph 10 + plugin 9 + workspace 5）
- REGISTERED_OWNERS：25（含 graph/plugin/workspace）
- REGISTERED_SIDE_EFFECTS：42（call_sites 归属，含 graph 3 + plugin 9 + workspace 1）
- INVALID_IMPLEMENTATION_LOCATORS：0（self-test + real scan fail=0）
- DUPLICATE_STATES / OWNERS / WRITERS / INTENTS：待 Phase G 审计
- DERIVED_STORED：0（R6 真实扫描 fail=0）
- UNREGISTERED_WRITERS：0（真实扫描 fail=0）
- PUBLIC_CONTRACT_BYPASS：待 Phase G
- NATIVE_OWNER_BYPASS：0
- RESOURCE_OWNER_MISMATCH：待 Phase G
- BROWSER_GRID_FROZEN_SEMANTICS：UNCHANGED
- BUSINESS_CODE_CHANGED：NO（仅 registry + checker）
- FILES_MOVED：NO
- CAPABILITY_MATURITY_CHANGED：NO

## 下一步

1. Phase D：漂移审计（02-STATE-SOURCES.md 旧 useBrowserStore 引用；home/apps/settings/vault/agent/database locator 差异）。
2. Phase F/G：Final 3 SCR 已完成（graph/plugin/workspace 据实裁决 FULLY_GOVERNED，详见 scr/FINAL-3-SCR.md）；UNGOVERNED=0、OWNER_PENDING_SCR=0；checker self-test ALL_PASS + real scan fail=0。
3. Phase B：语义治理冻结（独立红队 21 攻击）+ tag `semantic-governance-v2-full-coverage-pass`。
4. Phase C：物理边界整改（store 迁移 capabilities/*/state/ + UI 归位 + shared/ui canonical root）。
5. Phase D：M2 package pilot（1 前端 + 1 native）。
