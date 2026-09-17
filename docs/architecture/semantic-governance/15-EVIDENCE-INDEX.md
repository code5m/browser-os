# 15 · Evidence Index — 证据总账
> Chief（只读审计合成）· 汇总 `01–11` 与 `16-REVIEW-REPORT.md` 的证据。
> 重要：行号只是辅助，**每条证据同时记录 file + symbol**；证据随代码变化可能失效，以 symbol 重新定位。
> 格式建议（未来统一）：`EVID-XXXX` + Claim / Classification(FACT|INFERENCE|RECOMMENDATION|UNVERIFIED) / File / Symbol / Line / Confidence。
> 本轮各 Agent 使用了带前缀的 EVID id（如 EVID-0001、EVID-BT-01、EVID-005-TAB）；本索引保留原 id 并标注归属文档，便于交叉引用。

---

## A. 证据量概览
- 总证据条数（去重后）：约 90+（含各文档内 EVID-xxxx、CLAIM-xxxx、SEM-xxxx、DUP-xxx、SMF-xxx、FEAT-xxx）。
- FACT 占比：高（Review 独立复核确认全部被核验的核心 FACT 与源码一致，未见事实性误报）。
- INFERENCE：生命周期/架构耦合类（position==show、close==kill、create_grid 隐式销毁旧格）。
- RECOMMENDATION：10/11/12/14 中的契约与 checker（非现状）。
- UNVERIFIED：EVID-WS-01（workspace 切换对页签生命周期影响）、FEAT-006/008/011 部分未深读项。

## B. 按主题索引（证据 → 文档 → 关键符号）

### B1. 状态真源（归属 01 / 02）
| 证据 | 文档 | 符号 / 关键行 | 分类 | 置信 |
|---|---|---|---|---|
| EVID-0001..0006, 0011..0018 | 01/02 | `useLayoutStore.mainView:138`, `setView:193`; `useBrowserStore.gridOpen:20`, `gridToolbarOpen` 跨写; `isBrowserVisible:149`; `panelOpen`/`bmPanelOpen`; `aiNavOpen` 死重复 | FACT | HIGH |
| EVID-0007,0008 | 01 | `activeTabId:19`, `activeTab:148`; `bridge.rs` `AppState.active_tab:269`, `tab_activate:4663` | FACT | HIGH |
| EVID-0014 | 02 | `useWorkspaceStore.currentLocalPath:579-586`（四级 fallback） | FACT | HIGH |
| EVID-0015 | 01 | `CredentialList.vue` 无 store，局部 ref 镜像 | FACT(+INFERENCE keyring 权威) | HIGH |

### B2. 生命周期（归属 05 / 16 CLAIM-LC / CLAIM-S4）
| 证据 | 文档 | 符号 / 关键行 | 分类 | 置信 |
|---|---|---|---|---|
| EVID-BT-01..06 | 05 | `create_tab:723`, `spawn_child_window:104`; `tab_activate:4663`; `hide_bounds:570`; `close_tab:794`; `hibernate_tab:4559` | FACT | HIGH |
| EVID-GR-01..06 | 05 | `create_grid:3851`(含 `close_grid:3854` 隐式销毁); `hide_all_webviews:635`; `UpdateRect:391-424`(`win.show():423`); `close_grid:3915`(kill); `reposition_visible:437` | FACT / INFERENCE | HIGH |
| EVID-TR-01..04 | 05 | `spawn_terminal:552`; `onBeforeUnmount` 不 kill; `term_resize:4801`; `terminate_session:661` | FACT | HIGH |
| EVID-LY-01..02 | 05 | `BrowserHost.vue:15`; `syncViewVisibility:646`; `apply_bounds_inner:531`(`set_visible(true):555`) | FACT / INFERENCE | HIGH |
| EVID-WS-01 | 05 | workspace 切换对页签/宫格生命周期——无 Rust 命令 | UNVERIFIED | MEDIUM |
| CLAIM-LC-01..06, CLAIM-S4-02/03 | 16 | 同上重采样 | CONFIRMED | HIGH |

### B3. IPC 契约（归属 06 / 16 CLAIM-IPC）
| 证据 | 文档 | 符号 / 关键行 | 分类 | 置信 |
|---|---|---|---|---|
| EVID-001..023 | 06 | `bridge.ts createGrid:534`, `closeGrid:537`, `gridPosition:542`; `bridge.rs create_grid:3851`, `close_grid:3915`, `grid_position:4022`; `main.rs generate_handler!:1440`; ACL `default-commands.toml:42` | FACT | HIGH |
| EVID-013/014/015/027/028 | 06 | `check-command-set-consistency.py:83` 正则盲区; 5 个 typed 未落地命令; `AGENT_SKILL_COMMANDS_AVAILABLE=false:102` | FACT | HIGH |
| EVID-024..026 | 06 | CASE-006 CONSISTENT; CASE-005 CONSISTENT(含 show); CASE-009 DRIFT_RISK(sync_browser_scene 未实现) | INFERENCE | HIGH |
| CLAIM-IPC-01..06 | 16 | 重采样确认上述 | CONFIRMED | HIGH |

### B4. 原生副作用（归属 07 / 16 CLAIM-SE / CLAIM-S4-05）
| 证据 | 文档 | 符号 / 关键行 | 分类 | 置信 |
|---|---|---|---|---|
| EVID-005-TAB, EVID-005-GRID, EVID-SHOW-007 | 07 | `apply_bounds_inner:555` set_visible(true); `UpdateRect:423` win.show(); 无 fn show_* | FACT | HIGH |
| EVID-HIDE-007, EVID-BYPASS-007 | 07 | `hide_webview:608`, `hide_all_webviews:635`, `tab_activate` hide_bounds 循环; `useBrowserStore.ts:170` tabPosition(-30000) | FACT / INFERENCE | HIGH / MED |
| EVID-ACTIVATE-007 | 07 | `tab_activate:4663` 休眠重建 webview | FACT | HIGH |
| EVID-TERM-CREATE, EVID-EVAL-RESULT, EVID-CLOSE-META | 07 | `term_spawn:4758`+`term_spawn_channel:4771`; grid eval 返回 "ok"; `close_tab:804-817` 元数据清除 | FACT | HIGH |
| CLAIM-SE-01..05 | 16 | 重采样确认 | CONFIRMED | HIGH/MED |

### B5. 重复/歧义语义（归属 08 / 16 CLAIM-S4-06 / CLAIM-MT）
| 证据 | 文档 | 符号 / 关键行 | 分类 | 置信 |
|---|---|---|---|---|
| DUP-001 | 08 | `isBrowserVisible:149` vs `isBrowserView():188` | FACT | HIGH |
| DUP-002 | 08 | `gridOpen:20` + `mainView:138` + `gridToolbarOpen:144` | FACT | HIGH |
| DUP-003/004/005/006/007/008/009 | 08 | 视图切换三入口; isBrowserVisible 旧公式漂移; 定位三包装; hide-vs-close 无独立原语; freeze 双源; store/tab 裸 bridge; activateWeb vs isActiveWeb | FACT / INFERENCE | HIGH |
| CLAIM-S4-06, CLAIM-MT-01..05 | 16 | 重采样确认 | CONFIRMED | HIGH |

### B6. 小模型失效模式（归属 09）
| 证据 | 文档 | 关联 CASE | 分类 | 置信 |
|---|---|---|---|---|
| SMF-001..012 | 09 | CASE-001, CASE-006, CASE-004, CASE-007, CASE-010a, CASE-010b, DUP-004/008/007/003/011/012 | FACT(源码陷阱) | HIGH |

### B7. 跨检查器冲突（归属 09 SMF-005 / 16 CLAIM-XC-01）
| 证据 | 文档 | 符号 / 关键行 | 分类 | 置信 |
|---|---|---|---|---|
| CLAIM-XC-01 | 16 | `check-ui.mjs:37,53,196,222-243,409`; `check-session-persistence-policy.py:21,230,573-574`; `check-native-webview-overlay.mjs:6,25-26,33`; `useBrowserStore.ts:185-187` | FACT(直接矛盾) | HIGH |

## C. 证据质量结论
1. 所有被 Review 重采样核验的 FACT 均与源码一致，**无事实性误报**。
2. 实质"冲突"均为评级口径差异（16 CONFLICT-01..05），非事实对立；建议 ADR-SEVERITY-001 统一。
3. UNVERIFIED 项（EVID-WS-01、FEAT-006/008/011 部分）需后续深读补齐，不阻塞 Phase 0/1。
4. 证据命名规范在 Agent 间不一致（前缀 vs 编号）；未来统一为 `EVID-XXXX` 并在每条包含 file+symbol，以降低代码变化后的失效风险。

## D. 未来证据记录规范（建议，纳入 README）
- 每条：`EVID-XXXX` / Claim / Classification / File / Symbol / Current line / Observed behavior / Callers / Callees / Why / Confidence。
- 必须同时记录 **file + symbol**（行号辅助），防止代码变化后证据失效。
- 明确区分 FACT / INFERENCE / RECOMMENDATION / UNVERIFIED，禁止把建议写成现状。
