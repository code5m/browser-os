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

## 5. 收藏数量回退解析回归（2026-11-10）

发现 `buildFavoritesReadbackScript` 普通模板字符串内的三条正文计数正则少了一层转义。生成后的脚本实际包含 `/共s*([d,]+)…/`，无法识别“共 2 个视频”“总计 2 条收藏”“2 个视频”。选中目录计数节点不存在时会误报 `BFC_TOTAL_UNKNOWN`。新增这三个页面夹具后，修复前复现失败，补齐转义后全部通过；原有选中目录、分页进度和最终快照断言保留。测试使用虚构 DOM，不证明真实账号读取成功。

核对时 `/usr/bin/mvp-browser-os` 与本地 release 文件 SHA256 不一致，运行中进程入口为 `/usr/bin/mvp-browser-os`。仅凭产物时间不能证明安装版对应当前提交；本次源码修复尚未安装。公众号管理后台的浏览器操作被站点安全策略拒绝，本轮未创建或发布公众号文章，也未绕过限制。

本轮收藏夹 DOM、注入脚本、标签绑定、工作区启动及图片懒加载检查通过。完整门禁在生产构建期间出现资源压力：可用内存约 499 MiB，4 GiB swap 全满；主动停止本轮 Vite 进程后可用内存恢复约 5.4 GiB。生产构建及完整门禁不能标记通过。桌面操作通道未提供可调用的原生窗口接口，真实 BrowserOS 收藏夹及公众号操作仍待验收。

后续同一次门禁的 `cargo check --locked` 已通过（12.97 秒）。体积核验脚本调用 Vite 进行第二次内存构建，内存压力再次上升，故终止本轮门禁及其构建子进程；未改门禁阈值或跳过检查伪造通过。恢复条件：可用的受限内存构建环境，以及可操作 BrowserOS 原生窗口的桌面通道。公众号站点安全拒绝须由平台支持的访问方式恢复，不能换通道绕过。

## 6. 安装复核与计数范围收紧（2026-11-10）

安装日志证明 21:47:31 已完成升级；当前进程启动于 21:47:39。对 deb **内部文件**、`/usr/bin/mvp-browser-os`、运行中 `/proc/PID/exe` 三者核验，SHA256 均为 `74fc3200925f54e4a7a05cf6e75dbf5594b14f8239049a197bf2c439fb491d79`。因此此前仅依据未打包 release 文件哈希不同而怀疑旧安装，证据不足；用户确实安装并运行了该包。21:50 后源码变更不在该包内。

本轮移除上一轮未经验证的任意 fav 节点计数回退及整页正文匹配，改为当前选中目录、收藏详情头部内的明确数量，包括“视频数：N”；自定义目录支持独立计数行。隐藏详情、冲突数量、未选中目录和推荐区域不作为回退计数来源。先增加用例复现失败，再修改解析器，虚构 DOM 回归通过。仍不读取 Cookie、不调用 B站 API、不将未知数量降级为当前卡片数量。

边界：尚未取得用户失败页面的真实 DOM，选择器兼容性与真实分页读取仍待安装版验证；不能将本地夹具通过宣称为真实收藏同步成功。依照 B站技能约束，本轮不进行账号在线请求。

## 7. 分页分批渲染回归（2026-11-11）

截图中的 72 是前两页累计数，不是失败页的条目数。使用虚拟时钟复现第三页先显示一条、2.5 秒后显示完整两条的过程：旧代码在约 1.5 秒时提前完成等待并报数量不符。

等待逻辑现比较整页去重 BV 序列（不再只取前五个链接），后续页面必须达到预计条目数，并且完整序列稳定至少 1.2 秒。序列变化重新计时；仍保持 20 秒等待上限，不反复点击翻页、不放宽总数校验。新增延迟成功、持续缺项超时、验证页停止、取消停止的行为回归，全部使用虚构数据且不访问 B站。

尚未解决/未验收：第一页若长期只加载部分条目，推算页容量仍可能不可靠；真实页面选择器兼容、账号级持久熔断、分批断点续读与独立翻页节奏预算需要后续专项验证。本轮不扩大原生命令契约，不宣称可以安全连续读取 136 页，也不保证不会触发平台限制。安装版与真实账号读取仍为 GUI_PENDING。

本轮检查：收藏夹 DOM 行为回归、注入脚本、标签绑定、Store 启动、图片懒加载和 `git diff --check` 通过；完整 `pre-merge.sh` 退出 0，输出 `PRE_MERGE_RESULT=ALL_PASS`（包含正式前端构建、Rust cargo check 与 dist 哈希核验）。未安装、未提交或推送，代码检查通过不等于真实收藏夹读取成功。

## 8. 超时原因分类与安全诊断（2026-11-11）

用户再次反馈前两页累计 72 条后出现 BFC_PAGINATION_TIMEOUT。该截图不能证明是页面加载慢，更不能证明触发风控；上一轮只解决提前读取，没有取得真实第三页 DOM。

本轮变更：

- 可识别的页码必须与目标页一致，不能仅凭列表内容变化通过。
- 超时细分为 BFC_PAGINATION_NOT_ADVANCED（页码或内容未确认切换）、BFC_PAGINATION_ITEM_COUNT_MISMATCH（提取数量不符）、BFC_PAGINATION_UNSTABLE（未稳定）；仍然拒绝不完整快照。
- UI 将“页面条目数”更正为“累计已读取条目”，避免把 72 误认为失败页数量。
- 原收藏页在失败时保留 `document.documentElement.dataset.browserosBfcPaginationDiagnostic` JSON；只含 targetPage、observedPage、expectedItems、uniqueItems、videoLinks、visibleVideoLinks、containerFound、contentChanged、waitedMs。observedPage 为 null 表示未识别，不能默认当第一页。链接数不等于视频卡片数，也不能据此判定失效视频数量。
- 诊断只留在当前远程页面内存，不自动上传或持久化，不保存 URL、MID、ticket、BV、标题、正文和 Cookie。不改变 Tauri Command 参数或原生 WebView 生命周期。

本地回归涵盖第三页分批加载、永久缺项、停留原页、内容变但页码未变、验证页和取消。真实第三页的上述计数仍待取证；不把模拟场景认定为用户实际根因。下一步由有权操作安装版的验收者获取已有失败页面的诊断；旧版本没有该诊断，更新后也不应反复启动 136 页扫描。未安装、未提交或推送。

2026-11-12 收尾核对：本轮完整门禁日志 `/tmp/browseros-bfc-pagination-diagnostic-gate.log` 最终为 `PRE_MERGE_RESULT=ALL_PASS`，不重复运行相同检查。诊断代码为 CODE_PASS；真实第三页根因尚未取得证据，安装版为 GUI_PENDING。临时日志不是永久发布证据。

## 9. 部分结果与限页读取（2026-11-12）

用户明确批准调整读取契约和后端保存逻辑；未授权部署或提交。涉及 `bfc_readback_submit` 的 kind 新增 checkpoint/partial，参数字段不变；`bfc_readback_finish` 允许手动保存已验证的部分 Capture。未修改 GTK、WebView 生命周期和权限。

行为调整：页面声明总页数可识别时优先采用；数量不足且稳定 3 秒的页面保留为部分结果，不冒充完整页。每页写入经过校验的内存 Capture；单轮最多 3 页，达到预算或数量缺口后停止。跨页 BV 重复、收藏夹切换或报告总数变化提前停止。部分结果只能手动保存，发送 traversalFinished=false；保存回执按实际提交页数和卡片数核验，不再要求服务回执等于账户报告总量。快于轮询间隔的 ready/partial 结果不再误判为脚本失败。

失败行为：普通翻页失败保留此前的内存 Capture，可手动保存；访问受限状态不允许保存调用改变读取流程，不自动重试。保存结果不明时仍禁止自动重试；未发送可回到 partial 手动重试。没有放宽完整 favorites 的连续页、唯一 BV 和总数校验。

明确未完成：检查点目前仅在有期限的本次会话内（开始后约 10 分钟）保留，取消、过期或退出会丢失；尚无跨重启持久断点/续读按钮，不应反复开始来尝试推进页码。空列表、第一屏未完整加载、真实失效条目归因仍需专项验证；账号级跨会话持久熔断未完成。未通过真实账号扫描验证，未验证本机 BFC 对部分导入的实际落盘行为；只验证构造的 payload 与回执契约。回滚应只撤销本节相关实现和测试，不触碰此前计数及诊断修复。

验证：BFC Rust 定向测试 13/13 通过（包含部分结果回执计数、失败保留检查点、部分结果不宣称完整），DOM 行为、脚本安全、标签绑定、Store 启动、图片懒加载通过。完整门禁在 Vite 构建阶段出现内存压力（可用约 574 MiB，4 GiB swap 全满），已只停止本轮门禁及其构建进程，未终止用户客户端。不能标记本轮完整门禁通过；本轮状态 BLOCKED（完整构建待验证）、GUI_PENDING。未安装、提交或推送。
