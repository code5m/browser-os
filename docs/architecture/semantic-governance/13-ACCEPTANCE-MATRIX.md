# 13 · Acceptance Matrix — 四层验收
> Chief（只读审计合成）· 每个关键语义迁移必须过 CODE / AUTOMATED / RUNTIME / USER 四层。
> 配套 `12-MIGRATION-PLAN.md` 与 `10-TARGET-SEMANTIC-CONTRACTS.md`。

---

## 一、Browser ↔ Grid（Phase 1，关联 CLAIM-S4-02 / CONTRACT-GRID-LIFECYCLE / CONTRACT-GRID-EXIT）

### 不得发生
- recreate 子进程、reload、destroy 活动页签、丢失 active URL、丢失瞬态页面状态。

| 场景 | CODE | AUTOMATED | RUNTIME | USER |
|---|---|---|---|---|
| browser → grid | `exitGrid`/`openModule('grid')` 调用；禁止组件裸 `buildGrid` | `check-grid-exit-intent.mjs` 0 命中 | 真实切换后 grid webview 可见、浏览器 webview 屏外 | 用户看到宫格铺满、浏览器不残留 |
| grid → browser（经导航，HIDE 路径） | `setView('browser')` 仅改 `mainView` | `check-view-switch.mjs` 无裸赋值 | 切回后浏览器 webview 复位显示、gridOpen 仍 true | 浏览器区域非空白、宫格进程存活 |
| grid → browser（经 close，DESTROY 路径） | `closeGridAll` 翻 gridOpen=false + bridge.closeGrid | `check-grid-lifecycle.mjs` gridOpen 仅 2 处赋值 | 切回浏览器空白被填满、宫格子进程退出 | 无 450MB 孤儿进程 |
| 往返 ×20 | 同上交替 | 集成脚本循环 20 次 | 每次切换稳定、无内存泄漏累积 | 流畅无闪烁残留 |
| close → restore | `closeGridAll` 后 `buildGrid` 幂等重建 | `create_grid` 首行 close_grid 幂等（EVID-GR-02） | 关闭后重开宫格恢复 URL 集 | 用户重开即得原宫格 |
| resize / maximize / restore | 仅 `reposition_visible` 重定位 | 无 destroy 路径触发 | 窗口变化后宫格重新铺满、不 recreate | 无空白/无重叠 |
| restart app | 会话恢复经 `restoreSession` | session 持久化断言 | 重启后宫格/页签按记录恢复 | 用户会话不丢 |

---

## 二、Grid visibility（CONTRACT-BROWSER-VISIBILITY / RULE-002）

| 场景 | CODE | AUTOMATED | RUNTIME | USER |
|---|---|---|---|---|
| `isBrowserVisible` 公式 | computed 仅 `mainView==='browser'`，无 `gridOpen` | `check-grid-close-logic.mjs` G2 改为仅 `mainView` | BrowserHost CSS 随 mainView 切换 | 浏览器仅在 browser 视图可见 |
| closeGridAll 注释/checker 漂移 | 删 `!gridOpen` 旧式描述 | G2 不报 FAIL | 不误隐藏浏览器 webview | 无 B9-4 空白回归 |
| 三标志不一致 | `gridOpen` + `mainView` + `gridToolbarOpen` 经单一 owner | `check-grid-lifecycle.mjs` RULE-013 收敛写入点 | 工具条展开态与宫格实例态一致 | 无"工具条展开但宫格没了" |

---

## 三、 create_grid IPC（CONTRACT-IPC-CONTRACT / CLAIM-IPC-01 / SMF-002）

| 场景 | CODE | AUTOMATED | RUNTIME | USER |
|---|---|---|---|---|
| FE/Rust 契约闭包 | `createGrid(n,urls)` 调 `create_grid(n,urls)`；注册 + ACL | `check-command-set-consistency.py` typed-invoke 正则覆盖 | 创建 N 格、各格打开 urls[i] | 宫格按预期数量+URL 打开 |
| 内存预算降级 | `if(degraded) gridCount.value=created` 保留 | 单测/集成断言 gridCount 回写 | 内存不足时 FE 格数跟随 Rust 预算 | 系统不 OOM 卡死 |
| 半改契约回归 | 无 `create_grid` 单边改签名 | 同上三源闭包 | FE 调 `create_grid` 不报错 | 无"调了没反应" |

---

## 四、 Credential owner（CONTRACT-CREDENTIAL-OWNER / RULE-011）

| 场景 | CODE | AUTOMATED | RUNTIME | USER |
|---|---|---|---|---|
| 密码红线 | 无 `password` 进 Pinia/localStorage/日志 | `check-credential-owner.mjs` RULE-011B 0 命中 | 明文密码只存 keyring | 凭据不泄漏到前端 |
| facade 收敛 | list/fill/import 经 `useCredentialStore` | RULE-011A 仅在 facade 命中 | 填充经精确 origin 匹配 | 仅同源页签可填充 |
| 激活页签绑定 | `fill` 传 `{credential_id, activeTabId}` | origin 匹配单测 | 休眠/重建页签下 origin 正确 | 填充不被 ORIGIN_MISMATCH 拒 |

---

## 五、 View switch / Tab intent（CONTRACT-VIEW-SWITCH / RULE-003 / RULE-009 / DUP-009）

| 场景 | CODE | AUTOMATED | RUNTIME | USER |
|---|---|---|---|---|
| mainView 单一 writer | 仅 `setView`/`openModule`/`closeModTab` 写 | `check-view-switch.mjs` 无裸 `layout.mainView=` | 切换后无 stale 覆盖层 | 无残留菜单/编辑器 |
| 页签命令经 store | 无组件裸 `bridge.tabClose/tabNew` | `check-tab-intent.mjs` 0 命中 | recentlyClosed/tabs[] 一致 | Ctrl+Shift+T 恢复准确 |
| activateWeb 判定 | 用 `mainView!=='browser'` 非 `isBrowserView()` | `check-tabbar-predicate.mjs` RULE-008 | grid 视图点页签正常显示 | 无"点了没反应" |

---

## 六、 Native scene adapter（CONTRACT-NATIVE-SHOW / RULE-006 / RULE-007）

| 场景 | CODE | AUTOMATED | RUNTIME | USER |
|---|---|---|---|---|
| 原生命令仅经授权适配 | `bridge.tabPosition/gridPosition/hideWebview/hideAllWebviews` 仅 useBrowserStore/useBrowserHost | `check-native-scene-adapter.mjs` 0 命中 | 显隐时序统一，无遮屏/空白 | 视图切换干净 |
| 禁止 position-as-hide hack | 无 `tabPosition(-30000)` 直发 | RULE-007 0 命中 | 隐藏统一走 hide_webview | 黑闪修复仍有效且不绕过去重 |
