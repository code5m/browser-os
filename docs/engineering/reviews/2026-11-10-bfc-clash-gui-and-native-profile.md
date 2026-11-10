# BrowserOS BFC：Clash Verge 授权配置及原生 WebView 实测记录

日期：2026-11-10。基线：PR #61 和 PR #62 均已合并 `master`，合并 SHA `9400d8f811319171d6720f153edce463220e16ee`。

## 1. Clash Verge 官方管理界面（已执行，已验证）

用户明确授权通过 Clash Verge 管理界面进行**单域名**网络验证。本轮通过用户会话的官方 Clash Verge 托盘 `com.canonical.dbusmenu`，在 `订阅` 菜单确认当前选中订阅后触发相同订阅重新应用，**没有**直接操作特权 root 内核进程、提权、关闭证书验证或关闭 TUN。改动限于当前订阅已有 Merge 覆盖层下的 `dns.fake-ip-filter`：保留原 2 条规则，追加精确的 `space.bilibili.com`。事前备份位于用户配置目录的同名 `.bfc-gui-20261110.save`（不提交仓库；不包含在报告中）。

- 官方 GUI 操作后生成运行时配置的时间戳更新，过滤规则条数 2 → 3；仍为 `tun.enable=true`，`tun.stack=gvisor`。
- `getent ahostsv4 space.bilibili.com` 不再返回原来的 `28.0.0.x` Fake-IP，转为实际 Bili CDN 地址。
- 仅请求公开 B站收藏页面的 `curl --head`：HTTP 200 且 TLS 证书验证结果 0。
- 独立 **匿名 WebKitGTK** 临时测试进程：相同系统 HTTP 代理的 B站页面加载从此前的 `g-tls-error-quark` 改为 `WEBKIT_FINISHED failed=False`。没有读取任何用户 Cookie。
- 以上证明**网络层和原生 WebKit TLS 恢复**，不能证明账户已登录或已经抓取收藏夹。

## 2. 现有安装版真实 GUI 点击证据（网络通过，登录尚未通过）

- 本机已有 `/usr/bin/mvp-browser-os` 安装版比本轮新合并修复旧（安装日期 2026-11-10 08:47）。在 GNOME/XWayland 实际启动并通过原生子 WebView `tab-1` 打开 `https://space.bilibili.com/471851304/favlist`，日志证明子 WebView 创建、导航，**没有**出现 TLS `loadFailed`。
- 通过 GNOME AT-SPI 官方无障碍 GUI Action，点击 BrowserOS `主页` → `BFC 内容库` → `开始读取当前账号收藏`。本地 BFC 服务仍显示 `localOnly=true`、`bilibiliNetworkRequests=0`、`completeSnapshot=false`。
- 此次新启动的 BrowserOS 原生 B站页面可见 DOM 中**明确出现了“登录 / 立即登录”提示**。BFC GUI 返回“无法确认：页面未能提供可核实的账号主页 ID”。因此本轮新进程**无法确认真实 MID=471851304 登录状态**，没有产生真实分页及累计条数。不能以 URL 所含 MID 冒充会话登录 MID，也不使用 Chrome 登录 Cookie、帐号凭证或 API 模拟回执。
- 已对合并后 `master` 提交执行本地 Rust Release 构建及 GitHub 安装版工作流，需以各自实际结果为准；原先安装在 `/usr/bin` 的旧二进制不能冒充最新修复版本。

## 3. 登录状态跨重启的原生 WebKit 依赖问题（新修复）

检查现有 `tauri-plugin-browser-tabs`：通过 `WebviewBuilder::new` 创建普通网页子 WebView 时未提供显式 `data_directory`。Wry `WebContext::new(None)` 在 Linux 走 WebKit 默认上下文，而显式给定数据目录时才配置网站数据管理器与 WebKit 原生会话持久化。已新增稳定的**应用私有目录**供普通浏览器标签页复用，且与主应用 UI WebView 分离。目录置于 Tauri `app_data_dir()` 下，固定名称，不依赖输入的 tab ID、MID 或网址；验证不允许目录符号链接越界并限制 owner 权限 `0700`。

- 所有会话数据完全由 WebKit 原生管理，BFC 本地服务和 Rust BFC 命令**不读取、不导出、不上传** Cookie、localStorage 密钥或认证信息；普通浏览器必要的官方登录行为除外。
- 持久化策略只保证**今后经官方 B站页面正常登录后的会话有稳定存储基础**；不会从之前的临时会话或其他浏览器迁移认证信息。已打开但未登录的新 BrowserOS 进程仍需用户本人完成 B站正常登录。
- 对原生 Tab/profile 的私有目录固定性加入 Rust 回归测试，保留现有 BFC `11` 项原生只读契约及前端注入失败/页码回执单测。
- B站本轮原生可见登录按钮为**页面可见的 div/span 形式“立即登录”**；在 `buildLoginProbeScript` 中补充有限范围的可见 header 登录控件识别，登录状态未知时不再依赖按钮标签；如果稍后获取到公开登录 MID，仍先处理 `logged_in`，避免临时占位登录控件误报。`tests/check-bfc-tab-binding.mjs` 新增两组互斥回归。
- 原生持久目录属于 `tauri-browser-tabs` 原生危险区改动，已按用户此次明确授权在 CI/本地通过 `CHECK_NATIVE_ALLOW_DIFF=1` 放行**仅危险区 diff 审核**，未修改或关闭其余安全检查、权限验证及 GUI 后置条件。

## 4. 完成标准与剩余阻塞（不得宣称全部完成）

| 验收层 | 本轮状态 |
|---|---|
| 当前 Clash Verge 订阅 GUI 应用单域名 Fake-IP 排除 | PASS |
| 本地服务健康/权限只读 | PASS，仍 `completeSnapshot=false` |
| 独立真实 WebKitGTK TLS 加载 | PASS |
| 本机旧 `/usr/bin` 安装版 BrowserOS 原生 B站 WebView TLS | PASS，页面可加载 |
| 合并后新版本 Native BFC 登录态识别 | 待新包实机验证；当前新进程显示登录提示 |
| 真实 MID 471851304 / 收藏夹名称 / 分页累计约 4869 条 | **未通过**；当前原生登录态缺失，不可模拟 |
| 从准确修复提交构建 / 安装版 GUI 回归 | 另行按 CI / 原生运行证据报告 |

后续当事用户需在**升级后的 BrowserOS 原生 B站页面**通过官方页面正常登录一次，然后从 BFC 面板点击开始读取。不得使用普通 Chrome 的登录状态替代原生 WebView，不得读取/转移任何 Cookie，不得绕过 HTTP 412、验证码或帐号保护。此文件仅记录实际验证的部分，不把未完成任务写为已通过。
