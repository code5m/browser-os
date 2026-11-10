# BrowserOS BFC 只读收藏回读：Native Tab 绑定与登录探测审核（2026-11-10）

## 用户现场与复现范围

- 用户在此前的 BrowserOS 原生 B 站收藏页确认登录：MID `471851304`，默认收藏夹约 `4869` 条；BFC 面板却显示 `logged_in: 0`、`BFC_TOTAL_UNKNOWN`。
- 此轮实查本机 `127.0.0.1:4198/api/health`：`ok=true`、`localOnly=true`、`bilibiliNetworkRequests=0`，本地 BFC Node 服务保持运行；`/api/capabilities`：`readSnapshotOnly=true`、`completeSnapshot=false`。没有修改 BFC 本地服务仓库。
- 实查时 **没有正在运行的 `/usr/bin/mvp-browser-os` 进程**，虽然该程序已安装。无法从用户之前登录的 B 站原生 WebView 提取这次修复的真实账号回执，不能将用户此前确认的登录当作本轮修复后的验收结果。

## 已确认的根因与代码修复

1. `useBfcStore.ensureBilibiliTab()` 曾在当前浏览器 tab 不属于 B 站时从 `browser.tabs` 选择**第一个**匹配 B 站的 tab。可能绑定到没有登录、不是收藏夹的错误 WebView。现在 `startReadback` 必须绑定 `browser.activeTabId` 对应的实际网页 tab；缺失、非 `tab-N`、不匹配或非 B 站分别报明确错误。不再静默选择另一个账号页签。UnifiedTabBar 原本通过 `browser.tabSwitch(id)` 驱动所选 Browser tab；BFC 使用同一个 ID。
2. Rust `bfc_readback_begin` 追加 `AppState.active_tab`、`AppState.tabs` 和原生插件 `TabManagerState.get_tab_ids()` 一致性校验，并校验 HTTPS B 站来源。不会使用 `app.get_webview(tabId)` 错误查找插件持有的子 WebView，也不会把 BFC 工作台本身当成 B 站页面。session ticket 与原生 label `tab-N` 双重绑定，错误码 `BFC_TAB_ID_MISMATCH`。
3. `eval_in_tab` 返回 `ok` 只代表分发，**不代表脚本成功运行**。登录探测必须收到来自 B 站原生子 WebView 的 `bfc_readback_submit(login)`/`abort` 回执，否则以 `BFC_LOGIN_PROBE_INJECTION_FAILED` 超时/注入失败结束，永不伪装为 `logged_out`。收藏脚本另外提交 `favorites_start`，页面未执行时不能等到 25 分钟才发现。
4. 登录脚本只读取**可见 DOM 头像链接**和明确范围的公开页面登录 MID (`__INITIAL_STATE__.loginInfo.mid`、明确 `isLogin=true` 的公开变量)，不读取 Cookie、localStorage、sessionStorage、授权头、用户名/密码或账号凭据；曾访问过的他人主页 MID 不作为登录身份。会在有限时间内等待头像 DOM 异步加载，区分访问受限、未登录、身份未知、桥注入失败。
5. 收藏夹脚本优先核对当前所选收藏夹 DOM 可见数量（含`默认收藏夹（4869）`样式），不盲信页面无关视频数字；只读取可见 BV 卡片，逐页上报 `progress` 真实计数。Rust 校验页码递增、计数单调、合计/文件夹/分页总数一致，最终完整后才允许通过回环服务保存只读快照；不会生成假条数。最大分页从 200 增至 300 以容纳约 244 页，条目硬上限 5000、来源/副作用限制不变。
6. 错误代码明确区分 `BFC_TAB_NOT_ACTIVE`、`BFC_TAB_ID_MISMATCH`、`BFC_LOGIN_PROBE_INJECTION_FAILED`、`BFC_LOGIN_IDENTITY_NOT_FOUND`、`BFC_FAVORITES_PAGE_REQUIRED`、`BFC_BILIBILI_ACCESS_LIMITED`、`BFC_BILIBILI_HTTP_412`、`BFC_TOTAL_UNKNOWN` 等。只记录脱敏标题、tab ID、公开 URL origin/path（不含 query/hash）、是否收藏夹。遇 HTTP 412、验证码或访问限制即停止，严禁绕过。

## 证据分级（必须分别验收）

| 验收层 | 本轮证据与状态 |
| --- | --- |
| BFC 本地服务 | `/api/health`、`/api/capabilities` 实查通过，快照仍 `completeSnapshot=false`；**服务在线不等于收藏已同步** |
| 模拟 DOM/脚本 | `tests/check-bfc-tab-binding.mjs`、`tests/check-bfc-favorites-dom.mjs`、原有 BFC 只读脚本/ACL 契约测试通过；模拟正确/错误 tabId、已登录公开 MID、注入不可用不能报登出、可见收藏数量及逐页提交。模拟数据并非真实 4869 条。 |
| Rust / 前端 / 工程门禁 | `cargo test --locked --manifest-path src-tauri/Cargo.toml capabilities::bfc` **11 PASS**；完整 Rust **638 PASS，0 FAIL，18 ignored**；`npm run check:module-contracts`、`npm run check`、`node scripts/check-native.mjs` PASS；生产构建 `npm run build` PASS（常规 chunk warnings 不影响构建结果）。 |
| BrowserOS 原生 B 站子 WebView + 已登录 MID | **待验证**。实查时原安装版进程未运行，没有可控且仍登录的真实子 WebView；不能宣称在此轮读取出 MID `471851304`，也不能宣称分页 4869 条已读取。 |
| 修复后安装版 `.deb` GUI | **待验证**，需基于修复提交构建并安装、确认产物哈希，启用用户已授权 B 站登录原生 tab，执行完整开始读取、分页/总数验证。不要拿普通 Chrome 页面、模拟脚本或旧安装版替代。 |

## 真实安装版验收步骤（必须由完成者提供实测证据）

1. 从修复 SHA 打包，校验 SHA256 并在机器上安装；浏览器启动登录用户的真实原生 B 站收藏页，选中 UnifiedTabBar 中正确的 `tab-N`，确认可见 URL、标题及收藏夹数。
2. 打开 BFC 模块，点击“开始读取当前账号收藏”，检查脱敏 `BFC_TAB_BIND` 包含同一 `tab-N`、公开 URL origin/path；Rust `bfc_readback_begin` 与 `eval_in_tab` 使用同一 ID，主/远端都只接受合法权限。
3. 检查 `bfc_readback_status` 从 `awaiting_login_probe` → `logged_in`（MID `471851304`）→ `reading_favorites`（非零进度与收藏夹名）→ `ready` → 本地提交结果，不能以 `BFC_TOTAL_UNKNOWN` 和 `logged_in:0` 结束。每页真实 BV/标题、总页数、总条数应从页面显示读取并相互核对，不预设必为 4869；用户原先看到的是约数。
4. 若页面已登录但仍无法取 MID：报告 `BFC_LOGIN_IDENTITY_NOT_FOUND`；若注入未执行或拒绝：报告 `BFC_LOGIN_PROBE_INJECTION_FAILED`；若实际页面出现 HTTP 412/人机验证：停止并报告，不使用 Cookie、登录绕过、代理轮换或 API 读取。
5. 核对 `completeSnapshot:false` 的本地服务仍标记不完整，不得在运行未获完整页面证据时改成“完整同步”。

**合并约束**：代码测试和静态只读契约通过不等于真实账号取证；未能提供 2 项 Native/安装版 GUI 真实证据时，保持 PR 待验收，不得把该任务写为完全修复/已合并。
