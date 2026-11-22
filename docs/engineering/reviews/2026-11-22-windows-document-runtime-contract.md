# Windows 文档、标签与原生执行契约：源码复审与修复

日期：2026-11-22（Asia/Shanghai）

## 结论与提交边界

本轮核对了用户给出的旧审查及两份附件。部分问题在当前代码仍成立，部分已由前序修复解决；“当前 SHA 没有 Windows 原生验收证据”属于验证状态，不能通过代码存在或旧 CI 通过来消除。

- 仓库：code5m/browser-os。
- 冻结基线：fix/windows-native-security-convergence-20261122 @ 69d882bbd1c089977b67d1655d667db51c7acac2。
- 原审查分支最新值：fix/windows-private-dir-acl-case @ 285542f57bd055592b4d5019e5a89e8555bcc805。
- GitHub compare 确认冻结基线是上述原审查分支的后代，领先 4 个提交、落后 0 个；不是从旧 master 移植补丁。
- 独立交付分支：fix/windows-document-runtime-contract-20261122。
- 最终修复提交是本文件所在提交，父提交固定为 69d882bb；提交消息包含 [skip ci]。
- 不更新原开发分支或 master，不创建 PR，不合并，不触发 workflow_dispatch。
- 按用户本轮明确要求：不运行测试、校验器、编译、lint、格式化检查、CI、安装或 GUI 操作。新增的回归内容仅为定义，全部 NOT_RUN。

所有“修复”结论均表示已修改源码，**不表示编译成功、测试通过或 Windows 运行验收通过**。

## 逐项核实

| 审查问题 | 冻结基线的源码事实 | 本轮处理 |
| --- | --- | --- |
| documentOwnership 拒绝 Windows 路径 | 仍然只接受 POSIX 绝对路径并拒绝反斜杠 | 已修改 |
| Workspace 父目录、包含关系、重命名、移动、树定位依赖 "/" | 仍存在，包含 IDEA 项目扫描入口 | 已修改 |
| 移动目录后跨 Owner 标签和 clean Coding 缓冲仍指向旧路径 | 仍存在 | Layout 更新目录标签；Coding 保留内容并使旧路径失效 |
| 导航守卫拒绝后仍切网页或进入目录 | 仍存在 | 先获得守卫许可，Native/目录读取完成后再发布有效意图 |
| 网页“关闭其他”关闭目标本身 | 仍存在 | 用投影的命名空间 key 保留目标，逐个委托 Owner |
| 关闭最后一个后台网页或记忆中的模块抢前台 | 仍存在 | 后台关闭不改变可见模块 |
| Ctrl+W / Ctrl+Tab 只操作网页 | 仍存在 | 共用可见投影和动作协调器，保留输入框/终端键盘归属 |
| Coding 先比较 Hash 再独立 atomic_write，回滚只凭 Hash | 仍存在 | 复用原生版本化写入；持久记录操作 ID 与 staged file identity |
| 模型请求长时间持有工作区锁，取消仅按 root 定位 | 仍存在 | 模型阶段按 Session 互斥；工作区锁只用于文件提交/恢复；取消按 root + session |
| Windows 文件名大小写别名使用不同侧车锁 | Coding 保留原始文件名，原始字节散列不能统一别名 | Windows 同一原生父目录共享提交锁，同时保留旧同名锁兼容 |
| Terminal 创建、Eval、部分文件与 Git 查询同步阻塞 | 仍存在 | 转入有容量限制的后台工作线程 |
| Grid zoom 旧延迟值覆盖新意图，100% 被跳过 | 仍存在 | 每 cell 串行读取当前意图，恢复 100% 也发送 |
| ACL 使用 icacls contains/全局计数 | 已过时；当前已使用共享 native SID/DACL 安全模块 | 保留前序修复，不复制校验器 |
| WebView2 Profile 与应用数据 ACL 逻辑分裂 | 已过时；当前两条链路使用共享安全 crate | 不重复开发 |
| data_dir getter 首次同步修 ACL 并 expect panic | 已过时；当前已有显式准备和准备状态发布 | 保留现有安全拒绝/启动流程 |
| ConPTY 无控制台直接 return 冒充通过 | 当前基线已有前序修复 | 不改回，不宣称原生执行成功 |
| Foundation Task/Event 回收、Rig 输出上限、Capability 释放路径 | 已有相应改善 | 不再列为本轮未修缺陷 |
| 全量 Windows Native/NSIS/真实 GUI 已通过 | 源码和历史 CI 不能证明 | 保持 NOT_RUN / GUI_PENDING |

旧工作流触发条件的描述也不能直接套用：冻结基线中 Windows V2 专项已使用受控调用链，Developer Feedback 的手动 Windows 原生开关默认关闭，并绑定 source SHA。本轮不修改工作流，不调用该入口。

## 跨平台文档路径契约

新增 documentPaths.mjs，作为前端文档身份与 UI 路径操作的共同入口；不赋予文件系统权限。

| 输入/边界 | 行为 |
| --- | --- |
| Linux 绝对路径 | 保持大小写区分 |
| Windows 本地盘符路径，正斜杠或反斜杠 | 归入同一默认大小写不敏感的词法身份 |
| 根目录加相对路径 | 按组件合成，不改变盘符或根类型 |
| 严格本地 verbatim disk 路径 | 支持，以匹配当前 Rust canonicalize 与 Prefix::VerbatimDisk |
| 不同盘符、Work / Worker 组件边界 | 不混淆 |
| 父目录 ".."、盘符相对路径、ADS、设备/NT namespace | 拒绝 |
| UNC、verbatim UNC、模糊尾随点/空格、保留设备名 | 拒绝 |
| Native read/write/CAS 实参 | 保留原始路径，不把折叠后的 key 当文件路径 |

Windows 前端身份是普通命名空间的词法协调，不是原生 File ID。特殊大小写敏感目录可能保守地共用 UI 锁或已有缓冲区身份；不宣称解析 hardlink、8.3 或所有别名。Native 句柄、允许根目录、reparse 拒绝及 CAS 继续决定实际访问。

原有 replace/conflicts/dirtyUnder/writingUnder/beginSave/beginMutation 均继续由一个 documentOwnership 实例提供。新增的失效元数据只存身份，不保存文本。

成功 rename/move 后，Workspace 更新自身已打开路径并通过 Layout action 更新目录标签。Coding 不擅自把已有 Native Session 绑定到新目录：保留缓冲内容，显示路径已变更，拒绝旧路径保存；用户关闭旧文档后从新位置重新打开。删除期间新产生的草稿也保留并拒绝写回已失效位置。

目录读取采用“准备意图 → 读取 → 检查意图与编辑版本 → 发布”。迟到响应、拒绝或读取失败不清除较新的内容。关闭活动目录标签也先读取下一目录，成功后才提交选择。

## 多文件写入与补偿

沿用 Workspace 原生保存引擎，未创建第二套文件写入实现。

1. 读取有界的文件快照，包含原生对象身份与字节；备份来自同一份已接受的快照。
2. 所有目标先预审。现存文件按对象 ID 去重；缺失目标按最近存在祖先的原生 ID 与路径组件去重。Windows 组件比较复用共享 crate 的 CompareStringOrdinal 封装。
3. 对包含 BrowserOS 存储目录的宽工作区，按祖先目录原生身份拒绝写入 Owner storage。此预审不代替最终 Native 文件授权。
4. 保存新的 operation_id、Session 准备状态及完整 journal。
5. 先创建并同步 stage 文件，把其对象身份写入 journal，再取得提交锁、复核预期版本并发布。receipt callback 在提交锁之前执行，避免它持久化同目录元数据时重入锁。
6. 新文件使用 no-replace，不能覆盖预审后出现的文件；现存文件复用 pinned parent、版本复核和原子替换，Windows 保留目标 DACL。
7. 补偿只操作能够匹配本次 staged identity 与 after_hash 的文件。恢复后的对象身份也先记录，使中断后的恢复可继续判断。
8. 没有 stage receipt 的条目从未获准发布，补偿时不触碰它；外部创建的未尝试目标不会因此被删除，也不会阻止其他自有写入补偿。
9. 缺少新操作证明的旧 journal、被外部修改或替换的文件保持原样，返回明确恢复冲突并保留提案和备份。

Windows 提交锁以已授权父目录的固定侧车文件为基础，覆盖该目录中的大小写及既有短名称别名；旧按名锁仍同时持有，以兼容使用相同拼写的旧调用方。只有升级后的调用方共享新的别名保护。更强的目录级串行化可能降低同目录并发吞吐，性能未测量。

保留边界：

- 这是 BrowserOS 协作写入与补偿恢复，不是跨多个文件的严格 ACID 事务。
- 不遵守侧车锁的第三方仍可能竞争最终 Linux/Windows replacement 或 Linux unlink；没有任意第三方进程的内核 CAS 保证。
- 原生 File ID + Hash + 操作 journal 提高恢复确定性，但不构成对恶意同用户进程、所有 ABA/ID 重用或全部断电情形的证明。
- 缺失组件的未来 Windows 8.3 别名不能提前靠字符串推导；最终 native no-replace 仍是防覆盖边界。
- 创建后未写入的空父目录、已持久化备份可能保留；不宣称失败后整个目录树完全回到原状。

## 任务与后台执行

Coding 的 Session 状态只有既有 Coding Owner 修改。模型等待期间保持 Session 锁，同一个 Session 的并发编辑返回 AGENT_BUSY；其他 Session 可以推进。文件 apply/recovery 仍短暂按 workspace 串行，Native CAS 负责文件版本冲突。取消 key 包含实际 Session ID，返回 cancelled 的请求在 Foundation 记录为取消。

BlockingLane 在入队前限制容量，permit 跟随实际执行 OS 操作的闭包；取消等待方不会在阻塞操作仍运行时提前释放容量。它不是新 Agent Runtime，也不承诺杀死已进入阻塞系统调用的线程。

容量：Workspace I/O 8、Git 只读 8、Terminal spawn 4、Tab Eval 16、Grid zoom 24。Grid 的发送另按 cell 串行，并在实际发送前读取最新 owner 意图。保留既有 native send 的 best-effort 语义，不把发送当作 ACK 验收成功。

IPC 命令名、DTO、来源授权和 Native 安全策略保留。main 的原生辅助入口及已在后台运行的 Agent Git facade 使用对应同步 Owner helper，避免把 Future 误接入旧同步调用链。

## 修改范围

| 文件 | 修改内容 |
| --- | --- |
| src/application/documentPaths.mjs | 共用词法路径规则 |
| src/application/documentPaths.d.ts | 上述规则的 TypeScript 声明 |
| src/application/documentOwnership.mjs | 采用路径规则；保留 Owner/锁并增加失效通知 |
| src/capabilities/workspace/state/useFileStore.ts | Windows 路径操作、导航发布、跨 Owner 路径变更 |
| src/capabilities/workspace/ui/FilePanel.vue | IDEA 父目录与文件名 |
| src/capabilities/workspace/ui/FileTreeNode.vue | Windows 别名选中与路径显示 |
| src/capabilities/agent/model/codingWorkbench.mjs | 文档别名去重，保留 Native 路径 |
| src/capabilities/agent/ui/CodingWorkbench.vue | 共用身份、失效路径 UI 与写入保护 |
| src/application/tabCoordinator.mjs | 只读投影之上的动作协调 |
| src/application/tabCoordinator.d.ts | 动作契约声明 |
| src/composables/useUnifiedTabs.ts | 统一快捷键和标签栏动作 |
| src/stores/useLayoutStore.ts | 守卫与版本化发布、目录标签更新 |
| src/capabilities/browser/state/useBrowserStore.ts | Native 动作串行，完成后发布活动状态 |
| src/components/layout/UnifiedTabBar.vue | 使用统一动作并修复关闭其他 |
| src/App.vue | Ctrl+W / Ctrl+Tab 使用同一投影 |
| src-tauri/src/capabilities/workspace/safe_paths.rs | Native 快照、版本化写入/删除、目录身份和回归定义 |
| src-tauri/src/capabilities/workspace/safe_paths_windows.rs | Windows 对应实现，别名锁、DACL 保留及回归定义 |
| src-tauri/src/capabilities/agent/coding.rs | 操作收据、保守恢复、目标预审、Session 并发/取消 |
| src-tauri/src/capabilities/agent/public_api.rs | 会话级取消契约 |
| src-tauri/src/framework/ai.rs | 传递真实 Session 取消身份与 Foundation 终态 |
| src-tauri/src/shared/blocking.rs | 有容量限制且取消安全的后台入口 |
| src-tauri/src/main.rs | 声明共享入口并适配原生辅助调用 |
| src-tauri/src/bridge/terminal_commands.rs | Terminal spawn 后台化 |
| src-tauri/src/bridge/tab_runtime.rs | Eval 后台化 |
| src-tauri/src/bridge/grid_runtime.rs | zoom 后台化及发送顺序 |
| src-tauri/src/bridge/filesystem_commands.rs | 文件预览后台化 |
| src-tauri/src/capabilities/workspace/mod.rs | Workspace I/O 容量 Owner |
| src-tauri/src/capabilities/workspace/cas.rs | 编辑器 CAS 命令后台化 |
| src-tauri/src/capabilities/workspace/fs_cmds.rs | 文件命令后台化 |
| src-tauri/src/capabilities/git/commands.rs | 只读 Git 命令后台化及同步 helper |
| src-tauri/src/capabilities/git/public_api.rs | Agent Git facade 保持同步 Owner 契约 |
| tests/check-cross-workbench-document-ownership.mjs | 路径与失效保护回归定义 |
| tests/check-cross-workbench-file-store.mjs | 文件树、原始路径、目录竞态回归定义 |
| tests/check-coding-workbench.mjs | Coding 路径别名回归定义 |
| tests/check-unified-tab-actions.mjs | 真实 Store/Owner 的守卫、异步完成和标签回归定义 |
| docs/engineering/reviews/2026-11-22-windows-document-runtime-contract.md | 本交付记录 |

## 去重、架构建议与剩余验收

保留既有 Rust CAS、加密草稿、Document Owner、Monaco Runtime、agent-foundation 和共享 Windows security crate。未移植旧 fileSaveGuard，没有整体 rebase/合并旧 Monaco 或 Agent Channel 分支，也没有创建第二套 Document Manager、Task Registry 或工具执行器。

Tool Registry/审批统一、durable SQLite 正式接入、完整 Process Supervisor/Model Gateway 契约仍属于架构演进；“多处实现”本身不能证明存在授权绕过。本轮修复具体执行/取消/文件契约，不把上述长期迁移宣称为已经完成。

Monaco Worker 性能需要实际同步量、内存和功能指标。按用户禁止运行要求没有测量，不改 setEagerModelSync(false)，不宣称获得性能收益。自由拖动跨类型标签和跨模块持久恢复也不是这次已经存在的功能回归，不临时构造另一套标签状态。

| 剩余验收 | 状态 | 责任 Owner |
| --- | --- | --- |
| 新增逻辑/Store/Native 回归、Rust/TS 编译、lint/validator | NOT_RUN_BY_REQUEST | 各修改模块维护者 |
| Linux 文件/取消/恢复及桌面回归 | NOT_RUN_BY_REQUEST | Workspace / Agent / Linux Native |
| Windows ACL 正反、冷启动耗时、同 SHA 文件 CAS | NOT_RUN_BY_REQUEST | Windows Native / Security / Workspace |
| Windows AppContainer 同 SID/跨 SID、MCP/LSP、ConPTY | NOT_RUN_BY_REQUEST | Windows Native / Agent / Terminal |
| NSIS 构建、产物 Hash、安装 | NOT_RUN_BY_REQUEST | Release / Windows Native |
| 真实 WebView2、窗口、文件、终端、会话 GUI | GUI_PENDING | Windows Desktop 验收 |
| BFC 真实登录、持久化及收藏回读 | GUI_PENDING | BFC / Browser Owner |
| 同 SHA 全量 CI | NOT_RUN_BY_REQUEST | 仓库维护者 |

历史 BFC/Windows 失败不能被本次源码修改自动注销。本轮没有重跑它们，也没有声称已复现或修复其中每项运行失败。恢复冲突的旧 journal 需要保留后人工处理，不能自动按 Hash 覆盖文件。

若后续获得运行授权，应针对本文件所在最终提交统一验证，不把不同 SHA、旧构建或其他平台的结果拼接成通过结论。
