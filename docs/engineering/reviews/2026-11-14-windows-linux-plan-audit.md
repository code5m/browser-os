# BrowserOS Windows / Linux 双平台方案 V1 审核报告（只读源码审查）

- 日期：2026-11-14
- 评审状态：**有条件通过设计方向；暂不批准按 V1 交接说明全面并行开发**
- 对象分支：**fix/bfc-readback-workbench-hardening-20261114**
- V1 文档提交基线：**d80f59b0c04457e743a5a18e6e8acae249ea7c6c**
- 审核类型：静态源码、仓库规则、工程治理和文档一致性审核；**未执行** Windows 编译、原生 GUI、真实账号或安装包测试
- 关联原始方案：[V1 设计方案](../../design/browseros-windows-linux-support-20261114.md)
- 关联原始计划：[V1 XP-000～XP-020](./2026-11-14-windows-linux-implementation-backlog.md)
- 建议方案：[V2 设计与实施方案](../../design/browseros-windows-linux-support-v2-20261114.md)
- 建议任务：[V2 Backlog 与 Agent 交接](./2026-11-14-windows-linux-implementation-backlog-v2.md)

> 本报告保存 2026-11-14 评审内容和当时依据，供其他 Agent 直接读取；它不是实施授权，不替代仓库根目录 AGENTS.md、PROJECT-RULES.md 或实机 PASS。本文的「问题」有些是已经确定的源码事实，有些是需要补齐的设计合同，应明确区分。

## 1. 总体结论

**总体技术方向正确、风险识别较好、R1/R2 分层基本合理，但执行依赖、安全边界与交付合同不足。** 推荐允许继续只读调研、独立 WebView2 技术验证和 CI 设计；在关键合同冻结前不批准 XP-003/004 之后的大规模开发。

值得保留的五条原则：

1. R1 仅交付 Windows BFC Beta，R2 才交付双平台工作台完整能力，不能靠修改打包配置宣称 Windows 可用。
2. 静态分析、Rust 编译、安装包生成、真实桌面 GUI 和真实 BFC 回读分别记录，Linux PASS 不等于 Windows PASS。
3. BFC 的 tab/MID/TTL 票据、主窗口授权、远程页 ACL、局域隔离、导入幂等与结果未知语义不可被平台适配破坏。
4. 原生 BrowserEngine、文件系统安全、进程/IPC、终端、沙箱独立评审，不以危险 fallback 换取可编译。
5. 小任务、可审查的 Diff、可复现测试和保留失败证据优先于模型数量与开发速度。

### 审核表

| 项目 | 审核结论 |
| --- | --- |
| Windows/Linux 技术方向 | 认可 |
| R1/R2 分层 | 认可，但 R1 文件安全和本地 BFC 服务需补合同 |
| BFC 安全红线 | 认可，必须落实在后端能力和 ACL |
| WebView2 核心可行性 | 尚未在真实 Windows 证明 |
| 任务先后、Agent 第一批分工 | 需要调整 |
| CI/测试方案 | 基本合理，required 时机、实机证据和治理联动需具体化 |
| 6～9 / 13～20 人周估算 | 仅作初步估算，XP-005A 后重估 |
| V1 可以直接全面开工？ | **否** |
| 是否修改了仓库原始功能代码？ | 本轮评审没有 |

## 2. P0 级审核发现（R1 开始前必须处理）

### P0-1：Windows Beta 的文件安全和数据写入边界尚未闭合

**源码已确认：**
- [safe_paths.rs](https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/src-tauri/src/capabilities/workspace/safe_paths.rs) 在非 Linux 下对安全读取、无覆盖移动和描述符式 CAS 明确返回 Unsupported。
- [cas.rs](https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/src-tauri/src/capabilities/workspace/cas.rs#L47-L180) 的 write_file_cas 在非 Linux 分支却进入路径式 save_if_unchanged_path，包含普通 fs::rename；**并非所有非 Linux 文件命令都天然拒绝**。
- [session.rs](https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/src-tauri/src/session.rs#L30-L38) 的 atomic_write 使用固定 json.tmp + fs::rename；工作区等模块复用此持久化原语。
- [workspace.rs](https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/src-tauri/src/capabilities/workspace/workspace.rs) 中书签/会话/成果等读写必须以真实调用路径确认。

**风险：** V1 把 XP-011 全部放到 R2，但 R1 又计划开放书签、会话和基础本地能力。Windows 文件覆盖、锁、ACL、符号链接/重解析点与异常中断行为不同；不能只隐藏前端入口。

**处理：** R1 之前建立“原生命令 fail-closed 清单”与“R1 必需持久化最小安全合同”；XP-011 保留完整后端实施，但其 R1 子门禁前移。未验证的文件命令必须在 Rust 入口拒绝；对实际开放的 JSON、书签、会话验证并发、保留、崩溃和升级行为。不要将简单 canonicalize、普通 rename、先删再写伪装成安全等价。

### P0-2：Windows 的本地 BFC 服务未纳入明确交付合同

**源码已确认：** [bfc/commands.rs](https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/src-tauri/src/capabilities/bfc/commands.rs#L16-L21) 的 Session、health、capabilities、import 指向 **127.0.0.1:4198**；finish 在 BrowserOS 中执行本地 HTTP 协商与导入回执核对。它不是 BrowserOS 自己直接存储完整 BFC 数据。

当前垂直链路：

~~~text
Windows BrowserOS / WebView2
  -> B站网页登录 + 可见 DOM 回读 + BfcRuntime 校验
  -> HTTP loopback 127.0.0.1:4198 (session/CSRF/import)
  -> 独立 BFC 本地服务保存快照与返回回执
~~~

**设计合同缺口：**
1. 谁提供 Windows 兼容的 BFC 服务？它是否已具备 Windows 可运行证据？
2. 随 BrowserOS 交付、另行安装，还是仅接已运行的服务？用户如何启动和升级？
3. BFC 服务未启动、端口占用、版本不兼容和异常中断如何降级？
4. 如何证明 loopback-only、CSRF、接口版本、导入回执、幂等以及不向 B站发额外请求？
5. 单独安装 BrowserOS 时宣传边界是什么？

**处理：** 在 XP-000 建立 BFC 依赖/协议/安装和故障合同。允许先标为未决，不允许把尚未核实的“可在 Windows 完整导入”写为 PASS。

### P0-3：WebView2 最关键的技术验证顺序偏晚

**源码已确认：** [browser-tabs/commands.rs](https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs#L489-L532) 中 eval_result 的非 Linux 分支明确返回 GRID_PLATFORM_UNSUPPORTED；[platform/windows.rs](https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/platform/windows.rs) 是占位型尺寸适配，不提供带结果的脚本执行或 Profile 证据。

**问题：** V1 把 XP-005 安排在 XP-004 Windows 编译底座之后，若 WebView2 子 WebView、共享 Profile 或脚本结果通道的假设不成立，前期重构有返工风险。

**处理：** 拆为 **XP-005A（独立最小 Windows WebView2 POC，允许先于完整编译底座验证）** 和 **XP-005B（编译底座后集成）**。XP-005A 必须独立证实：
- 子 WebView 可定位，DPI/焦点和窗口生命周期可观测；
- 同一应用专属 User Data Folder 下的浏览器子视图重启后登录态持久，主可信 UI 隔离；
- 带 JSON 结果的 JS 执行，注入失败、超时、导航销毁有确定性且有大小上限；
- 不能通过 CDP、远程调试或 Cookie 导出绕过原生能力缺口。

### P0-4：Windows 10 首发平台定位需重新界定

V1 将 Windows 10 22H2 与 Windows 11 x64 同列首发支持目标。微软已在 **2025-10-14** 结束 Windows 10 22H2 常规支持；ESU 有单独条件。**2026 年软件可以兼容 Windows 10** 不等于该 OS 仍在常规支持期。

处理：**Windows 11 x64 是默认正式目标**；Windows 10 22H2 x64 为“有条件兼容/可选验证”，必须写明系统 ESU/安全维护前提，不将它纳入同等默认支持承诺。

微软来源：[Windows 10 生命周期公告](https://learn.microsoft.com/en-us/lifecycle/announcements/windows-10-end-of-support)、[发布信息与 ESU](https://learn.microsoft.com/en-us/windows/release-health/release-information)。

### P0-5：能力协商不能只是前端置灰

V1 提议增加 browser.basic、bfc.readback、grid、database、terminal、agent.lspSandbox 等能力标志，这很必要。但在未实现的平台上，只靠 Vue 隐藏入口无法约束原生 Command 或远程 WebView 调用。

处理：
1. 由 Rust 原生 runtime 给出事实状态，前端只消费，不能自行宣称 supported；
2. 未支持命令在原生入口 fail-closed，不允许落入不安全 fallback；
3. 任何新远程页权限都必须通过现有 capability/permissions 威胁审查；
4. 能力区分 available、unavailable、degraded、blocked，并可带稳定 reason code；
5. 只有对应平台实机和安全测试通过才允许将能力打开。

## 3. Backlog / Agent / CI 的具体缺口

| 级别 | 发现 | 修订方向 |
| --- | --- | --- |
| 高 | XP-002 依赖 XP-001；XP-003 依赖 XP-001，却在首批 Agent A/B/C 中被描述为可直接并行 | 可并行只读预研，不可无合同并行修改 |
| 高 | XP-010 依赖 XP-018，而推荐 PR 序列把 NSIS/OS 集成放在靠后位置 | 将 XP-018 拆成 XP-018A（最小打包原型）与 XP-018B（完整系统集成） |
| 高 | 非 Linux on_navigation 直接上报 NavigationFinished（导航回调并不等同最终加载成功） | XP-008 加导航请求、提交、完成、失败、进程失效与 generation，授权始终用原生实际 URL |
| 高 | Profile 只定义共享应用目录与 ACL | 补并发创建、环境选项一致性、主 UI 隔离、旧数据迁移、不可访问和卸载保留合同 |
| 中高 | 需要识别 HTTP 412，又允许完整资源瀑布扫描降级，单一 resourceEvents 标志过粗 | 拆出 browser.navigationHealth、browser.httpAccessSignals、browser.resourceWaterfall；有证据才标 available |
| 中高 | XP-003 只写清除 HOME/XDG/临时目录硬编码 | 保证 Linux 老路径、现有 Profile、书签会话数据在升级后不丢失 |
| 中 | 所有 PR 默认 150 行易迫使安全实现被拆碎 | 150 行作为拆分提醒，超过需解释；安全原子性、测试完整性优先 |
| 中 | “高级/普通模型”只按能力分工，未规定分支冲突、审查顺序、同文件占用 | 每任务独立分支/工作区、明确 owner、禁止同文件并行写、仅审核后合并 |
| 中 | Windows CI 初期允许 red，但缺陷分类和 required 提升规则需细化 | experimental 原样报告 FAIL；全部目标编译转绿并复核后才 required；不得跳过检查 |
| 中 | Linux X11 下已有锁定布局修复，平台抽象重构可能误触 | Linux 等价性需独立截图、冷启动和回归证据 |

**WebView2 官方补充依据：** 同一共享浏览器进程下，环境选项不一致会导致创建失败（ERROR_INVALID_STATE）；默认 User Data Folder 也可能建在不可写目录，必须指定受控应用目录。参考 [Microsoft WebView2 Win32 API](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl) 与 [进程相关事件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-related-events)。

## 4. AGENTS.md / PROJECT-RULES.md 与 V1 交接冲突

原交接模板强制阅读两份 Windows/Linux 文档，遗漏仓库根目录已生效的规范。

[AGENTS.md](https://github.com/code5m/browser-os/blob/d80f59b0c04457e743a5a18e6e8acae249ea7c6c/AGENTS.md) 要求依次阅读 AGENTS.md、PROJECT-RULES.md、任务文件、相关源码/测试；触及 CI/门禁需额外读 docs/engineering/README.md 与 governance.json。

不可遗漏：
- tauri-browser-tabs/**、src-tauri/capabilities/**、原生 GTK/WebKit 等为危险区；领取任务不自动授权普通 Agent 改动。
- Linux WebView 定位/隐藏、UI 无 HTML 浮层、无边框窗口与既有 1 tab = 1 WebView 规则已锁定。
- 普通关闭 Tab 不自动存档；平台迁移不能借题更改产品语义。
- 变更 GitHub Actions workflow 必须同步登记 docs/engineering/governance.json，并执行工程治理检查。
- Agent 需明确允许改动、禁止范围、回滚、证据与审查人；不能多人不受控地 push 同一分支。
- 原 V1 交接中的 git clone [https://...](https://...) 是 Markdown 展示，不是能直接运行的 Shell 语句；执行脚本必须使用纯文本仓库 URL。
- 不得将文档中的 Proposal 当成 PROJECT-RULES.md 已批准目标，不得把编译成功写成 GUI PASS。

**基线提醒：** 本次比对显示目标分支比当时的 master 多 34 个提交；这是 2026-11-14 的快照，不代表工作树干净或以后始终相同。实施时先刷新 refs，逐任务隔离，不能并发污染 BFC 修复分支。

## 5. 推荐的新阶段和依赖顺序

~~~mermaid
flowchart TD
  A["XP-000 支持/安全/服务合同"] --> B["XP-001 源码清单"]
  A --> P["XP-005A Windows WebView2 独立 POC"]
  B --> C["XP-002 Windows experimental CI"]
  B --> D["XP-003 AppPaths"]
  C --> E["XP-004 平台编译隔离"]
  D --> E
  E --> F["XP-005B WebView2 集成"]
  P --> F
  F --> G["XP-006/007/008 Profile/脚本/事件"]
  G --> H["XP-009 端到端夹具"]
  E --> I["XP-011 R1 最小文件/持久化安全门"]
  E --> J["XP-018A 最小 NSIS 原型"]
  H --> K["XP-010 R1 Windows BFC Beta"]
  I --> K
  J --> K
  K --> L["XP-011～017 与 XP-018B R2 完整能力"]
  A --> M["XP-019 Linux DEB/AppImage 并行验收"]
  L --> N["XP-020 发布准入"]
  M --> N
~~~

- **第一阶段：** XP-000、XP-001、XP-005A；XP-002 的 CI 方案可先设计，但代码提交在合同之后。
- **第二阶段：** XP-002/003/004 完成 Windows 可编译的 fail-closed 底座；Linux 不回归。
- **第三阶段：** XP-005B/006/007/008/009，R1 持久化和权限门，XP-018A 的 NSIS，最终 XP-010。
- **第四阶段：** XP-011 完整版、XP-012～017、XP-018B；XP-019 可独立并行，XP-020 分平台验收。

## 6. 第一批 Agent 的改进分工

| Agent | 任务 | 可以马上做什么 | 禁止/门禁 |
| --- | --- | --- | --- |
| 高级 Agent 0 | XP-000 | 冻结范围、能力协商、BFC 本地服务合同 | 不得擅自把 Proposal 改成 Owner 已批准 |
| 普通 Agent A | XP-001 | 只读扫描 Unix API、路径、production/test、现有 fallback | 不改 native |
| 普通 Agent B | XP-002 | 只设计 workflow/依赖矩阵 | XP-001 和治理合同完成前不提交 workflow |
| 高级 Agent C | XP-003 | AppPaths 接口与数据迁移设计 | 未审核前不机械替换现有 Linux 路径 |
| 高级 Agent D | XP-005A | 真实 Windows 独立 WebView2 POC，回执/Profile/定位 | 不能用 Linux/WSL 替代 Windows；不得 CDP 或导出 Cookie |

首批完成后统一交 Diff、Windows 版本、CI 日志、真实 GUI 截图/过程、失败与降级清单，再审核下一阶段。

## 7. R1 开工前核对表（默认未完成）

- [ ] Windows 11/Linux 支持矩阵和明确的不支持能力合同
- [ ] Windows BFC 本地服务的提供、安装、协议及版本兼容方式
- [ ] 独立 WebView2 子视图、Profile、JS 结果与失败语义的原生证据
- [ ] 所有 R1 未实现功能的 Rust fail-closed 边界与 ACL 验证
- [ ] 书签、会话、Profile、临时文件的 Windows 数据安全/持久化合同
- [ ] XP 依赖、单 Agent 可写范围、仓库治理和审查机制已修订
- [ ] Windows 编译、NSIS、GUI、BFC 实站验收与 Linux GUI 的独立证据模板

以上核对为**人工审核清单，不是仓库的实际完成状态**。任何空缺都不能从静态阅读推断 PASS。

## 8. 明确不在 Windows 双平台适配范围内的事项

目前 BFC 的业务链路是 **读取 B 站收藏页可见 DOM → 本地导入快照**；用户可能另有“整理 B 站原收藏夹、移动/复制视频、创建收藏夹”目标，但这属于**需要独立授权与安全合同的 B 站写回能力**，既不在 V1 R1 也不自动进入 V2 R1/R2。不得因 Windows 适配而默许远程写回、Cookie 导出或账号授权扩大。

## 9. 结论、限制与信息来源

**结论：有条件通过设计方向；不批准按当前 V1 交接模板直接全面开发。** 可以推进只读清单、最小技术验证、CI 合同和 R1 安全边界设计；关键原生能力确认后再估工作量与排期。

- **已核查：** 两份 V1 文档、AGENTS.md、PROJECT-RULES.md、governance、main.rs、browser-tabs、BFC Rust/TS、workspace/safe_paths、cas.rs、session.rs、部分 CI/packaging。
- **未核查/未执行：** 完整跨平台源码穷举、Windows Rust 构建、WebView2 GUI、Windows BFC 服务安装、B站真实账号回读、Windows 文件安全攻击测试、任何 CI PASS。
- **不可宣称：** Windows 已通过、NSIS 可安装、Windows BFC 登录态已持久、Windows R1 已完成；本文只是审查结果。
- **提交约束：** 只新增评审及 V2 文档；不改 V1 原文、不改任何源码、配置、CI、分支规则，不合并 master。

### 审核参考入口

1. [V1 设计](../../design/browseros-windows-linux-support-20261114.md)
2. [V1 Backlog](./2026-11-14-windows-linux-implementation-backlog.md)
3. [V2 设计](../../design/browseros-windows-linux-support-v2-20261114.md)
4. [V2 Backlog](./2026-11-14-windows-linux-implementation-backlog-v2.md)
5. [工程治理 README](../README.md)
6. [WebView2 官方文档](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl)
7. [Windows 10 生命周期](https://learn.microsoft.com/en-us/lifecycle/announcements/windows-10-end-of-support)
