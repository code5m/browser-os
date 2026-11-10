# BFC 原生子 WebView TLS / TUN 后续工程审查（2026-11-10）

## 基线与合并

- PR #61 已合并 master：`02e1d65f9d1eaac9b42e809cdb97f62ea8b8974d`。其代码侧 tabId 绑定、MID 探测和分页回执回归已经通过；并不代表真实 4869 条已同步。
- 本轮仅在 `fix/bfc-native-page-loading-20261110` 工作分支追加**原生加载错误归因**。用户未跟踪的 `.bevel/`、`需要解决的问题/` 均未修改。

## 真实证据：不是登录失败

1. BFC 本地服务 `127.0.0.1:4198` health：`ok:true`、`localOnly:true`、`bilibiliNetworkRequests:0`。本地服务健康仅能证明回环服务存活。
2. 开发机同一时间，官方 `https://space.bilibili.com/471851304/favlist` 的**匿名 HEAD** 通过当前本地代理和直连均返回 HTTP 200 且 TLS 证书验证通过。未读取/提交 Cookie 或其他账号材料。
3. 真实 BrowserOS Release 子 WebView `tab-1` 创建、定位成功，但 WebKitGTK 2.52.6 报 `Peer failed to perform TLS handshake: The TLS connection was non-properly terminated.`。它发生在页面加载/登录探测前；不是 HTTP 412、不是 `logged_out`，也不是收藏夹为空。
4. 独立 WebKitGTK **匿名临时上下文**诊断：显式使用本机 HTTP 代理及 SOCKS5 代理出现同一 TLS 错误；显式 `NO_PROXY` 的独立 WebKit 上下文可以完成加载。这说明问题涉及 WebKit 原生网络/路由上下文，不能把普通 `curl` 结果当作 BrowserOS 成功。
5. GNOME 代理为 `manual`；Clash Verge / Mihomo TUN 开启（`gvisor`）、运行时 DNS 为 `fake-ip`、IPv4 Fake-IP 地址池 `28.0.0.1/16`、IPv6 开启，`space.bilibili.com` 解析到 `28.0.0.x`，且运行时 Fake-IP 过滤列表没有 B站。Meta TUN 网卡可观察到 `28.0.0.1/30` 与 IPv6 虚拟地址。Clash Verge Issue [#7564](https://github.com/clash-verge-rev/clash-verge-rev/issues/7564) 记载相近症状，但这**不是唯一根因的确证**。
6. 当前订阅具有独立 Merge 覆写文件。曾对该文件暂存**仅 B站域名**的 Fake-IP 排除，并生成最小 runtime DNS 试改，运行配置语法检查通过；Mihomo 核心由系统服务管理，当前普通用户尝试发送重新加载信号收到 `PermissionError`，**未执行热重载**。两份文件均已与修改前备份逐字节比较相等并恢复，额外备份已清理；没有留下系统代理/TUN 配置变更、没有提权、没有关闭 TLS 证书校验。

## 本轮代码防护

- 浏览器 Tab owner 从原生 `tab-recovery` 的 `load-failed` / `failed` 事件中提取受限的错误类别（`tls` / `platform-limited` / `load`），只存临时类别和本机毫秒时间，TTL 2 分钟，不保存原始错误文本、URL、HTML、凭据等。
- BFC 在创建票据与探测期间核对近期原生错误，报告 `BFC_NATIVE_TAB_TLS_FAILED` / `BFC_NATIVE_TAB_LOAD_FAILED`；明确不能判断成 `logged_out`。只有明确 HTTP 412 才报告 `BFC_BILIBILI_HTTP_412`。页面恢复、手动刷新/导航/关闭时清理旧状态。
- BFC 读回依旧只有官方 B站可见 DOM，Cookie / localStorage 密钥 / 身份凭据不读不传；只读本机快照，未新增账号写入、绕过验证码或 HTTP 412 的能力。`completeSnapshot:false` 不被篡改。
- 新增 `tests/check-bfc-native-load-failure.mjs`，确认 TLS 不误认为登出、412 被单独归类、超时记录失效、敏感信息不被储存。

## 尚未通过的真实账号验收

- 修复后 BrowserOS **实际已登录 B站原生子 WebView** 正确识别 `MID=471851304`：未通过，阻塞为 TLS 页面加载失败。
- 原生分页读取约 4869 条和展示总页数、逐页进度：未通过，前置原生网页未成功加载；不能用模拟脚本测试或 Chrome 页面代替。
- 修复版新 `.deb` 安装到 `/usr/bin/mvp-browser-os` 并完成真实登录 GUI：待验收；本机 sudo 无交互授权，未替换旧安装版。

## 操作边界与建议的下一步

在有权限的 Clash Verge GUI **当前订阅**中，备份后仅尝试对已确认受影响的 `*.bilibili.com` 增加 `dns.fake-ip-filter` 排除；保留原有列表、TUN/路由和代理节点设置，重新激活该订阅，核实生成运行时 DNS 设置已生效并确实返回真实解析结果。不要直接修改生成缓存代替订阅层配置，也不要启用会替换当前自定义 DNS 的 GUI 覆写；如无效果立即回滚。网络侧恢复后，必须重新运行 `BrowserOS tab-N + MID + 真实分页 + 安装版 GUI` 的完整端到端验收。

**禁止**：关闭 TLS 校验，导出或复用 Cookie / Token，代理轮换绕过验证码/HTTP 412，未经授权修改系统服务，或将公共 HTTP 200 冒充账号读取成功。
