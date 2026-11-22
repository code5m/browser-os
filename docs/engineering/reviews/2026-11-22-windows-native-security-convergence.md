# Windows 原生安全收敛修复台账

日期：2026-11-22  
仓库：`code5m/browser-os`  
修复分支：`fix/windows-native-security-convergence-20261122`  
首轮起点：`fix/windows-private-dir-acl-case@285542f57bd055592b4d5019e5a89e8555bcc805`  
第二轮接续：`fecfae2b062fa281a0ced3d0f006dcef5ce39ee2`  
第三轮审查起点：`add44f3b599b526a14ba28c5f993f51cb4561bae`  
第四轮审查起点：`e039c83a33c076971f39fbe4f7943ebfea233c3f`；最新结论见 §9.3，本台账随其后的同分支修复提交交付

> 本轮交付是源码、回归定义和文档的修改。没有运行测试、校验脚本、构建、CI、NSIS、GUI 或性能测量。文中的“已修改”不代表原生问题已修愈，也不代表 Windows/Linux 回归通过。macOS 只有明确列出的源码准备，不能据此宣布 macOS 已支持。

## 1. 状态与证据口径

本台账在新提交形成前编写，因此不预填一个尚不存在的“已验收 SHA”。提交完成后，以新提交的完整 40 位 SHA 作为未来人工复验输入。分支名、旧测试日志、另一次运行的构建产物都不能替代这个 SHA。

| 证据层级 | 本轮状态 | 可以据此说什么 |
| --- | --- | --- |
| 源码修改与源码互审 | 完成本台账列出的源码修复；macOS 产品能力仍按明确边界标记未实现 | 说明实现选择、错误路径、平台分支和预期行为 |
| 回归测试及 source-contract 定义 | 已新增或调整，未执行 | 说明未来要覆盖哪些正反场景，不能说已经通过 |
| Windows/Linux 原生运行 | `NOT_RUN` | 不能断言本轮修复在系统上生效 |
| GUI、WebView2、ConPTY、LSP 联合运行 | `NOT_RUN` | 不能由单元测试定义推导 GUI 可用或进程已正确回收 |
| 大缓存性能、打包、安装、升级 | `NOT_RUN` | 不能宣称迁移耗时达标、NSIS 可安装或升级无回归 |
| macOS 编译、GUI、签名、公证及架构验收 | `NOT_RUN` | 不能称 macOS 支持完成 |

历史证据保留原归属：[GitHub Actions run 44321240531](https://github.com/code5m/browser-os/actions/runs/44321240531) 的原生筛选结果是 **28/30 filters**，full suite 是 **607 passed / 3 failed / 14 ignored**。旧工作流把 full-suite 失败混入筛选计数，曾造成“27/30”的错误表述。这些数字属于该 run 记录的旧提交，不能套到本分支起点或本轮新 SHA；本轮也没有重跑它们。

## 2. 修复总账

下表编号用于追踪修改，不是已通过的测试数量。所有条目的原生验收状态均为 `NOT_RUN`。

| 编号 | 原问题或易踩坑点 | 本轮修改 | 主要源码位置 |
| --- | --- | --- | --- |
| W01 | 用 `whoami` / `icacls` 文本、账户名后缀或多份规则判断身份，容易受语言和实现漂移影响 | 新建共享 Windows 安全 crate，用 TokenUser、GetSecurityInfo、EqualSid 与原生 ACE 结构实现统一身份和 DACL 策略 | `crates/browseros-platform-security/` |
| W02 | 只看根目录或聚合 ACE 数量，遗漏已有文件、逐对象缺失或额外授权 | 逐对象检查 owner、DACL、ACE 类型、SID、mask、继承 flags、owner/SYSTEM 各自数量，修复后原生复读 | 共享 crate；`safe_paths_windows.rs` |
| W03 | 私有子目录合规，却忽视父目录的 `DELETE_CHILD`；错误地递归覆写不同用途的后代 | 对 app_data 父根做浅层准备；workspace 与浏览器 profile 各自按自己的边界准备；抑制 SetSecurityInfo 隐式子树传播 | `workspace/workspace.rs`；browser-tabs `commands.rs` |
| W04 | 每次访问都尝试改 ACL，遇到活跃 WebView 文件占用；把 fast path 误写成“不扫描” | 先只读审计，合规则 0 repairs；只在可修复策略错误或路径缺失时进入迁移；启动使用跨进程原生准备凭证；首次冷准备深扫，活跃期仅浅层复核，不在页签路径周期深扫 | 共享 crate；browser-tabs `commands.rs` |
| W05 | 小写字符串前缀不能证明 Windows 路径授权，Unicode、大小写或 UNC namespace 易误判 | 用实际句柄身份和祖先身份判断；保留 UTF-16；拼写比较使用 CompareStringOrdinal；先区分磁盘或 server/share namespace | `security_policy.rs`；`safe_paths_windows.rs` |
| W06 | 数据准备留在 getter 内，错误变成 panic；误以为 Tauri setup 的 Err 会落入 build 的错误处理 | 显式准备并发布 PreparedDataDirectory；主进程与 Grid child 在 setup 中直接报告准备失败并退出 | `startup.rs`；`main.rs`；`workspace/workspace.rs` |
| W07 | 单实例插件或 macOS Opened 可能先于应用 setup 到达，直接审计会读取未准备的根目录 | 增加有界内存 URL 队列；统一冷启动、单实例、Opened 入口；准备成功后原子交接队列 | `startup.rs`；`main.rs` |
| W08 | LSP runtime 选择、rustc peer 或 stage 错误被弱化，初始化完成被误当作完整编译能力 | 显式传播配置和 stage 错误，runtime 只选一次；必需 rustc metadata/cfg；禁用 Cargo/buildScripts/procMacros；声明 compilation_verified=false | `capabilities/agent/coding_lsp_windows.rs` |
| W09 | 将 AppContainer 包目录误归为普通 owner-only 私有树，或在创建失败路径泄漏资源 | 共享原生句柄基础设施，采用独立 package SID 策略，exact DACL 回读；实际字节、对象数、deadline/cancel、state pin 与错误清理 | `capabilities/agent/windows_sandbox.rs` |
| W10 | ConPTY 测试因无控制台而假 PASS；kill 后无界 wait、异步等待句柄及 HPCON 关闭顺序有漏洞 | 移除假跳过；有界输入、可取消退避、独立关闭所有权及 Job 实际清空检查；生产关闭路径进入原生用例，补前端背压与失败反馈 | `windows_process.rs`；`terminal.rs`；portable-pty vendor |
| M01 | `not(linux)` 被当成 Windows，mac 编译路由缺失或返回 Windows 错误 | 显式 Linux/Windows/unsupported 选择；未实现能力返回真实平台与明确错误；Grid 不伪造 IPC 成功 | agent 模块、`grid_process_unsupported.rs`、`bridge/grid_runtime.rs`、`bridge/system_commands.rs` |
| M02 | Unix 私有根仅靠路径 chmod；macOS extended ACL 不受普通 mode 位充分约束 | 新增 Linux/macOS 句柄锚定私有根准备；验证 UID、0700、身份；mac 原生清除并复读 extended ACL | `workspace/private_directory_unix.rs`；`safe_paths.rs` |
| M03 | keyring 未启用 apple-native，mac 默认可能落到 mock；锁文件只改直接依赖不完整 | 启用 apple-native，并补齐安全框架的跨 target 锁解析与版本消歧 | `src-tauri/Cargo.toml`；`src-tauri/Cargo.lock` |
| G01 | 源码契约、筛选测试、full suite、GUI 结果被混成一个“通过”；移动分支导致证据串 SHA | 复用默认分支已有 Developer Feedback 的手动入口，以选定 ref 调用 reusable native；所有 required 入口统一计数，依赖锁与 source SHA 冻结 | `.github/workflows/windows-linux-v2-experimental.yml`；`tests/check-windows-linux-v2-source-contract.mjs` |

另有两项文档口径修正：`session.rs` 与 `system_commands.rs` 的原子替换注释修正，保留既有 Windows MoveFileEx 重试行为；工作流治理文件、WORKFLOW-CATALOG 与 parity 矩阵同步“专项验收需显式选择”和 `NOT_RUN`，不据文档调整宣布新结果。

## 3. Windows 私有根：共享策略、迁移与安全重试

### 3.1 身份和每个对象的检查必须分开成立

普通应用私有树采用 owner/current-user 与 SYSTEM 的明确策略。目录和文件分别检查权限 mask、继承标志及 ACE 数量；一个文件多出来的 world ACE，不能被另一个文件缺失的 owner ACE 抵消。owner 的 deny ACE 不能冒充 allow ACE，read-only mask 不能冒充 full control。空 DACL、缺失 DACL、未知类型 ACE、package/capability SID 都不能被字符串启发式判成合规。

迁移通过句柄相对遍历，拒绝 reparse、非单链接普通文件、重复对象及枚举与打开后的身份变化。迁移对象从首次打开使用 MAXIMUM_ALLOWED，同一句柄完成必要修复和回读，避免 ReOpenFile 与自身禁止 delete sharing 的句柄冲突。仅创建缺失目录时，父句柄补充的访问权限定为 FILE_ADD_SUBDIRECTORY、FILE_READ_ATTRIBUTES、SYNCHRONIZE。

共享实现不启用权限提升。异 owner 或缺少必要 READ_CONTROL/WRITE_OWNER 等访问权时，可以明确失败，不能静默接管目录。该策略也不能阻止同 SID 或管理员在审计后持续修改权限；句柄身份检查不是永久冻结文件系统。

### 3.2 父目录与 profile 是不同边界

先对 app_data base 调用 `ensure_private_directory_root`，仅处理这个目录自身，防止父目录的 DELETE_CHILD 削弱子根保护。随后分别准备 `base/mvp-browser-os` 和浏览器 profile。浅层 API 不能变成任意递归重置：现有后代必须按各自策略审计和处理。

MAXIMUM_ALLOWED 用于抑制 SetSecurityInfo 的隐式子树传播，后代由遍历器逐一处理。这尤其用于避免“只修父目录”时顺手改掉另一个用途的子树。WebView 父根和 profile 的准备位于 profile 使用之前，但真实 WebView2 打开、活跃文件占用及多 Grid 并发还没有新原生证据。

### 3.3 fast path 仍需要审计，预算不是性能成绩

| 边界 | 扫描/修复上限 | 深度 | 共用时间预算 | 说明 |
| --- | --- | --- | --- | --- |
| 默认私有树 | 50,000 个对象；50,000 次修复 | 64 | 15 秒 | 只读前置审计与迁移使用同一起始时间 |
| WebView2 profile | 200,000 个对象 | 128 | 30 秒 | 大 profile 的真实耗时尚未测量 |
| AppContainer 独立策略 | 8,192 个对象 | 32 | 调用链共用 OperationBudget deadline/cancel | 原生逐对象策略，与 owner-only 区分 |

普通底层 ensure 保留完整 deep scan 语义。应用与 Profile 的生产入口在它之外持有 `PreparedPrivateDirectory`：跨进程原生 mutex 串行冷准备，ready event 只在完整审计/必要迁移及根身份复读完成后发布。事件名称绑定 OS Volume GUID、文件身份与策略版本，实际 owner/DACL 会原生回读；不信任磁盘标记或环境变量。主进程与 Grid 在整个消费期持有根 pin 和事件句柄。后加入进程只复核根；最后一个参与者退出时内核凭证消失，下一次冷启动重新深扫。

页签创建只读取已准备的 Profile 并核对根身份，准备检查先于 tabs 写锁。删除了每 600 秒对活动缓存深扫的路径。首次冷准备发生在窗口和浏览器消费者创建前；残留旧版本/WebView2 孤儿进程若妨碍完整冷审计，仍明确失败并要求关闭占用者后重试，不把树变化当作安全通过。时间预算在对象边界、目录枚举页和原生调用之间检查，不能硬中断某一次阻塞的文件系统调用。因此“配置了 15/30 秒”不等于“实测启动一定在 15/30 秒内完成”。多 Grid 首次审计、杀毒软件占用、大缓存、网络文件系统和 ReFS 均未完成本轮性能或兼容性验收。

### 3.4 失败不是事务回滚

只有结构化的 ACL_NOT_PRIVATE 或根路径缺失进入迁移。读取失败、权限拒绝、预算不足、未知 ACL/package policy、reparse/hardlink、树变化等错误不能触发无差别强修。

迁移可能已安全收敛一部分对象后失败。这时不删除原数据、不改变内容字节、不发布成功状态，**也不得把已经安全的 ACL 回滚到旧的不安全 ACL**。错误报告保留阶段、计数和原生错误等可诊断信息。处理占用者、所有权或系统错误后，可以重新执行同一幂等策略；不能承诺任何不受控异常都自动恢复。

私有根限定本地磁盘。通用路径 identity/ancestry helper 支持磁盘与 UNC，但先比较磁盘或 server/share namespace，避免文件 ID 跨命名空间碰撞。`safe_paths_windows.rs` 原本的 UNC 拒绝边界保留；通用授权 helper 不因此新增“所有 UNC 一律拒绝”的回退。

## 4. 启动屏障与早到事件

Tauri 2.11.5 在 build 阶段初始化插件，在 RuntimeRunEvent::Ready 中执行应用 setup；setup 的 Err 会被 Tauri 转成 panic。单实例插件因此可能先接到 URL，而应用的数据根尚未准备。本轮不再依赖外层 build/run 的 Result 捕获这个准备失败。

两处 setup 的第一项业务动作都是 prepare_workspace，失败后直接调用 report_failure 并退出。Windows 报告使用不需要 WebView profile 的原生 MessageBox；没有临时公共目录回退、自动删除数据或自动提权。

成功路径依次为：准备 workspace 并保留跨进程凭证，尝试启用非 Linux 私有诊断日志；Windows 初始化 sandbox 记录存储，主进程进行有界遗留恢复、Grid 不重复扫描；随后调用 TabManager::prepare_storage 准备共享浏览器 Profile，最后释放早到 URL。data_dir getter 只读已发布状态，不再承担隐式目录迁移。

PendingOpenUrls 在 Builder 中管理，早于插件初始化生效。准备之前最多保存 64 条、每条 8,192 字节；超限、队列不可用或锁中毒会明确拒绝，并只记录错误代码，不记录原始 URL。ready=true 与提取 pending 在同一 Mutex 内完成，随后在锁外调用审计和 URL 处理。新到请求可与已取出的旧请求交错处理，但不会掉入“已取空队列、尚未置 ready”的丢项窗口。

单实例、冷启动 argv、macOS Opened 共用该入口。保留单实例提前转交行为，不要求第二实例为转交 URL 先重新迁移完整数据树。`app.windows=[]` 必须保持：Tauri 会在应用 setup 前创建配置窗口，source-contract 对此前置配置建立约束。

本屏障约束 workspace 与非 Linux 日志初始化，**不代表所有持久 I/O 都被拦截**。Linux 原有 early crashlog 继续在 prepare 前创建/写入/清理自己的日志。该行为本轮保留，不包装成已收敛到新私有根的结果。

## 5. LSP、AppContainer 与 ConPTY 的真实能力边界

### 5.1 LSP 的可初始化、可分析与可编译不是同一结论

Windows LSP 明确传播 rustup 配置、必需 rustc peer 和 stage 失败；runtime 只选择一次，避免探测通过后又启动另一个 runtime。rustc metadata/cfg 属于必需输入，不能空缺后仍报告完整成功。Cargo、buildScripts、procMacros 维持禁用，返回 compilation_verified=false，不把语言服务初始化当作完整项目编译证据。

从入口开始共享绝对 deadline/cancel，覆盖配置、快照、权限、runtime staging、spawn 和 initialize。快照计入目录/跳过项等全部访问对象，并按实际读写字节计额；取消在遍历和复制边界观察。starting reservation 只置取消标志，不提前返还容量；已发布与延迟清理资源共同占用上限，直到真实清理完成。Session 持有进程及两条 I/O JoinHandle，超时转交有界清理所有者，并由独立后台 reaper 推进，不依赖下一次业务请求；close_complete 同时反映 pending_starts、pending_cleanup、cleanup_errors。NUL/cfg 等启动错误在生产路径使用有界、脱敏的固定诊断码提前反馈，不通过放宽设备权限换取成功。以上为源码行为，真实 LSP 仍 NOT_RUN。

### 5.2 AppContainer 保留独立 package 策略

AppContainer 目录仍通过 GetAppContainerFolderPath 获取，位于 app_data 之外。state 给 package 修改权限，runtime/snapshot 给读取执行权限；generic AppContainer 在应用完全自有子树上仅获得非继承 FILE_TRAVERSE，不含 FILE_LIST_DIRECTORY、READ_DATA 或写权限；自有 package 保留业务所需 RX/M。这类 package SID 树不能套用普通 owner+SYSTEM owner-only 规则，共享安全 crate 遇到 S-1-15-* 或不支持的 ACE 类型会明确拒绝。

本轮替换 AppContainer 的路径式 icacls。完全自有 state/runtime/snapshot 子树按独立原生策略收敛并逐对象回读，陌生 explicit grant/deny 不再残留后仍返回成功。通用 package 只允许自有目录上的非继承 FILE_TRAVERSE，不传播到文件；逐对象精确重写会清除自有子树上旧的 generic RX。OS 管理的 profile 祖先保留系统 ACE，仅句柄化添加并核对必要 traversal，不套用自有子树整体重置策略。旧版本若已在 OS 祖先留下 generic RX，而没有足够的来源证据，本轮不会擅自降权系统 ACE；不宣称整个 OS 祖先链都已去除列举权限。合规 profile 先完整只读探测，只有确认缺少授权才申请改权句柄；state 长期句柄仅保留必要读取/遍历权限，避免一直持有 MAXIMUM_ALLOWED/DELETE 与子进程 cwd 自冲突。runtime 复制按实际读取量限额，不再使用无界 io::copy 后才比较长度。设置权限、复制和 spawn 使用相同 deadline/cancel，失败资源由有界所有者继续清理。自有 state 创建写读、跨包 state 拒绝、snapshot 写入拒绝、旧宽泛 ACL 修复均补了原生回归源码，执行为 NOT_RUN。

### 5.3 ConPTY 与 vendored portable-pty

`windows_process.rs` 去掉依赖 GetConsoleWindow 的假 PASS 路径，回归定义要求真实执行、限时等待退出及 EOF/join。`terminal.rs` 的输入通过有界队列由独立线程写入，IPC 全局终端表锁只用于取得轻量控制对象；输出退避及时观察 stop。关闭持有唯一资源所有者，终止进程、取消输入、释放 master/HPCON，同时保留 reader 排空，再回收线程。关闭先登记再从共享表移除，后续 kill 能持续找到同一清理任务；终端活跃和待清理资源合计上限为 32。IPC kill/resize 放在 blocking worker，避免占用 GUI 事件线程。只有主进程、Job 全部后代、PTY 与 I/O worker 均释放后才返回完成；已完成但有历史异常与 pending 分开反馈，避免前端永久假关闭中。超时返回 pending 并保留资源/JoinHandle，而非 detach；`/proc/{pgid}` 不存在断言仅限 Linux，其他 Unix 保留真实 process_group_alive 断言。

本分支的 vendor 补丁及接线涉及：

- `src-tauri/vendor/portable-pty/README.md`：记录本地补丁范围和后续维护要求。
- `src-tauri/vendor/portable-pty/src/lib.rs`：保留原生 Job spawn trait，导出实现测试替身所需的 Error 类型；不新增运行时依赖。
- `src-tauri/vendor/portable-pty/src/win/procthreadattr.rs`：以 Job list 创建属性在子进程开始执行前建立所属关系。
- `src-tauri/vendor/portable-pty/src/win/mod.rs`：OwnedHandle 单 waiter、Weak 取消和 finished/waker 交接。
- `src-tauri/vendor/portable-pty/src/win/conpty.rs`：创建和 resize 的尺寸限制均为 1..32767，并调整 Inner 关闭顺序。
- `src-tauri/vendor/portable-pty/src/win/psuedocon.rs`：配合 pipe 到 HPCON 的关闭顺序修正。

vendor 补丁必须和实际 Cargo path dependency 接线一起保存。未来入口包含通过 host manifest 选择 `-p portable-pty --lib` 的独立库测试；本轮没有运行它，也没有启动 ConPTY 或 GUI。

## 6. macOS：已做的源码准备与未实现清单

| 层面 | 本轮已做 | 仍不能据此声称 |
| --- | --- | --- |
| 平台选择 | Linux/Windows/unsupported 显式分支；context/LSP status 报真实 OS；默认浏览器与应用发现未实现时明确报错 | macOS context sandbox、LSP 或系统集成已原生实现 |
| Grid | 增加 unsupported manager，Linux/Windows 专属 GridStream/child IPC 保持 cfg；create/open/position/zoom 修改状态前拒绝 | macOS 具备 Grid 子进程、原生嵌入或 IPC |
| Unix 私有根 | Linux/macOS openat/mkdirat、O_NOFOLLOW、UID/0700 与身份复读；mac extended ACL 原生处理 | 所有子树已递归改权、任意文件系统均支持、后续路径 I/O 永久没有竞态 |
| Keychain | 启用 apple-native，修复 Cargo.lock 的安全框架依赖与版本消歧 | 签名应用的 Keychain 首次授权、升级、锁屏或拒绝情形已验收 |
| Unix 终端定义 | Linux-only `/proc` 断言隔离，保留通用进程组检查 | macOS GUI 终端、前后台 Job 和完整退出链已验收 |
| 浏览器数据 | Windows profile 准备边界已改；mac 平台差异被明确列入计划 | WKWebView 自定义数据目录、跨窗口/子进程 profile 隔离已实现 |
| 分发与架构 | 本台账明确后续先后顺序 | arm64/x86_64 构建、签名、公证、安装、GUI 已通过 |

### 6.1 Unix 私有根策略

路径使用原始 OsStr 字节，不做 lossy Unicode 转换；只接受绝对路径，限制总长度和组件深度，拒绝 NUL、`.`、`..` 与文件系统根。逐组件用 O_DIRECTORY|O_NOFOLLOW 打开，缺失时 mkdirat 后重新打开；最终对象必须是当前 effective UID 所有的目录。

仅收敛应用私有根，不 chown、不无条件递归改 Linux 现有树。OS/用户配置选定的 app_data base 可以先解析一次可信配置别名（包括自定义 XDG base、macOS 系统路径别名），随后追加应用自有叶目录并执行严格 O_NOFOLLOW/UID/0700/ACL 策略；发布的是已解析实际路径。应用自有叶、未解析组件和后续文件授权仍不跟随 symlink，悬空配置别名不会通过 create 被跟随。fchmod(0700) 后复读类型、owner、mode 和 dev/ino，再从命名空间重走确认身份；Linux 通过 mode 的 group-class mask 限制 POSIX ACL 有效权限，不新增按少数磁盘类型维护的广泛文件系统白名单。

macOS 普通 chmod 不足以表示 extended ACL 已清空，因此使用 acl_init/acl_valid/acl_set_fd_np/acl_get_fd_np/acl_get_entry 原生链路，并按 Darwin 返回值语义复读。只有明确的“无 ACL”结果可接受；EOPNOTSUPP 等不支持或其他错误不能吞成成功。后续文件读写仍需各自维持安全边界，启动时的路径审计不替代所有 fd-anchored I/O。

### 6.2 apple-native 与锁文件不能只改一行

keyring 3.6.3 缺少 apple-native 时，macOS 可能选择 mock backend；本轮启用真实 Apple backend。Cargo 锁解析涵盖全部 target，因此还要处理 iOS 的 security-framework 2.x 可选依赖，不能只看到 macOS 的 3.x 就省略它。

锁文件补 security-framework 2.11.1（checksum `897b2245f0b511c87893af39b033e5ca9cce68824c4d7e7630b5a1d339658d02`），复用既有 core-foundation 0.9.4、security-framework-sys 等可满足依赖；保留既有 3.7.0。keyring 依赖明确两个版本，rustls-native-certs 与 rustls-platform-verifier 对 3.7.0 的引用同步消歧。没有用 mock 回退掩盖 native backend 缺失，也没有执行 Cargo 来声称锁文件已验证。

## 7. 下一次做 macOS 的实施顺序

1. **先写平台能力表和最小产品范围。** 明确 tab、Grid、LSP、context sandbox、终端、默认浏览器、应用发现各项是 native、unsupported 还是后续目标；编译路由必须显式，禁止用 not(linux) 代替 Windows。
2. **先确定身份与存储，再接持久业务。** 选定正式 bundle identifier、app_data/profile/Keychain 命名与升级策略；在大小写敏感和不敏感 APFS 上覆盖 Unicode 拼写、非 ASCII 用户目录、符号链接及同名异大小写。授权依据使用系统身份语义，不全局 lowercase 或自行做 Unicode 归一化。
3. **确定 Finder 启动下的 runtime 发现。** 不依赖交互式 shell 的 PATH；先验证打包 runtime、显式配置和 Homebrew 等实际布局，再启动 LSP/终端。失败要明确给出缺失阶段，不能静默回到另一个工具链。
4. **把文件权限、App Sandbox、TCC 分开实现。** chmod/ACL 不能代表 TCC 授权，也不能代表 App Sandbox 范围。若采用 App Sandbox，按实际文件选择与重启需求设计 security-scoped bookmarks；不要靠要求用户授予完整磁盘访问来替代具体设计。
5. **先做最小 WKWebView 验证再迁移整个浏览器层。** Tauri 的 data_directory 不能直接当作 macOS 任意目录隔离能力；data_store_identifier 的系统版本边界与最低支持版本需一致。覆盖重启持久化、多个窗口、多个 store、升级与删除 store，先证明预期隔离，再接 Grid 或更多窗口。
6. **在分发形态里验证子进程。** 签名、公证、Hardened Runtime、嵌套 helper/framework 与 runtime 的路径和权限一起设计；分别验证 arm64 和 x86_64 及其 native 依赖，不由一台开发机或 Rosetta 成功推导两架构可用。
7. **最后运行分层矩阵并冻结证据。** 从源契约到原生权限，再到 GUI/生命周期、安装升级和性能；每层记录真实环境、执行命令、完整 SHA、通过/失败/忽略及 artifacts。未实现能力持续返回明确 unsupported，直到对应原生正反场景均有证据。

## 8. 未来人工复验矩阵

以下是后续得到执行授权后应运行的验收设计，不是本轮执行结果。每个记录必须带新提交完整 SHA；执行状态当前全部为 `NOT_RUN`。

| 维度 | 必须包含的正场景 | 必须包含的反场景/失败语义 | 需要的证据 |
| --- | --- | --- | --- |
| 原生 SID/DACL | 当前 SID + SYSTEM、目录/文件正确 ACE、重复 ensure 为 0 repairs | 相似 SID、deny 冒充 allow、read-only 冒充 full、额外 world ACE、每个对象缺 ACE | 原生读取结果；不能只用输出文本匹配 |
| 已有数据迁移 | 内容字节不变、父根浅层准备保留子策略、安全重试 | reparse、硬链接、异 owner、未知/package ACE、树变化、预算耗尽 | 成功与失败对象计数、阶段、错误码及内容对照 |
| 活跃 profile | 首次冷准备、跨进程加入、活动缓存淘汰不触发深扫 | 原生凭证不可信、根替换、冷扫描不完整时拒绝 | 同 SHA WebView2/多 Grid、重启与对象计数记录 |
| Windows 路径 | 非 ASCII、Unicode 大小写、允许的 UNC 同 namespace | 相似前缀、不同 share/volume、文件身份替换 | 授权允许与拒绝双向结果 |
| 启动 | 主进程和 Grid 准备后启动；早到 URL 不丢 | ACL 准备失败原生对话框；队列满/超长；Ready 前 URL 不读 workspace | 原生启动/事件记录；不能由 source-contract 替代 |
| LSP | 指定 runtime 与必需 rustc、初始化/请求/关闭 | 缺 peer、stage 失败、关闭超时、子进程残留 | 真实协议、Job/进程/线程清理结果；不声称编译验证 |
| AppContainer | package 可读执行 runtime/snapshot、可写 state | 越界访问拒绝、禁止写 runtime/snapshot、ACL 竞争、原生调用失败、deadline/cancel | 包 SID 下真实访问与失败记录 |
| ConPTY | 无交互控制台宿主也执行；输入输出、尺寸边界、退出 EOF | kill 失败、句柄关闭竞争、越界尺寸、线程超时、后台孙进程 | 原生执行及进程树/句柄清理证据 |
| Linux 保持 | 原有正常私有树、终端/进程组与浏览器主路径 | ACL/符号链接拒绝、清理异常、错误传播 | 独立 Linux 结果，不借 Windows 通过推导 |
| macOS 存储/Keychain | APFS 两类大小写配置、extended ACL 清理、真实 Keychain | symlink、异 UID、ACL API 不支持、Keychain 拒绝/锁定 | macOS 原生访问与重启结果 |
| macOS GUI/分发 | Finder 启动、WKWebView store、双架构签名安装 | PATH 缺 runtime、TCC 拒绝、helper 签名/架构错误 | 打包 GUI 与系统权限结果，未实现项保持 unsupported |
| 性能 | 小/大已有树、首次及重复启动、多 Grid | 限额触发、单调用阻塞、占用、失败重试 | 对象数、修复数、耗时、硬件/文件系统；无性能证据不得报达标 |

## 9. 手动入口与未执行边界

此前“只新增 workflow_dispatch 文件”的方案确实缺少默认分支登记条件。本轮复用 **`browseros-developer-feedback.yml`**：它已在默认 `master@f2c56312866d8756607a2a2339a240acc9d26641` 存在并支持手动触发。修复分支为该路径增加默认关闭的 `windows_native` 输入；以后单独授权时选择修复 ref 和其完整 source_sha，才调用同修订的 `windows-linux-v2-experimental.yml` reusable workflow。后者只有 workflow_call，避免再把功能分支独有文件宣传成可直接调度的入口。source job 要求 source_sha 等于所选 workflow ref 的 github.sha。普通 push 的快速检查规则保留；本次提交使用 `[skip ci]`。这里没有修改 master、创建 PR、合并或实际 dispatch。

Windows 所有 required 入口，包括 shared crate、portable-pty、WebView2，统一使用 `windows-native-test-runner.ps1`。独立 manifest 冻结关键用例名；同一 Job supervisor 先枚举测试，再逐名对应实际结果。仅有 passed>=1 不再足够，缺失/重复/ignored/SKIP/failed 均不能成为 required PASS。测试进程通过 PROC_THREAD_ATTRIBUTE_JOB_LIST 在创建时进入独立 kill-on-close Job，终止后需要主进程 signaled 且 Job ActiveProcesses==0；只观察父进程退出不能成为清理成功。清理未知时 quarantine 后续 native/NSIS 步骤。输出文件大小采用轮询阈值，不宣称硬磁盘配额。Profile 和 vendor 通过 host manifest 的 committed lockfile 运行；full suite 独立报告，ignored 不算验收。Linux framing 同步读取 required 名称清单并对账枚举、逐名结果和数量；Linux 全量 native 回归仍单独汇总并保留输出，GUI 和性能独立。NSIS 另需 build_windows_installer=true，产物永远不会自动成为安装/GUI 通过证据。

每次未来复验都要记录完整 SHA、平台、构建参数、实际测试数量与 passed/failed/ignored、独立 full-suite 结果，以及 GUI/性能是否运行。本轮执行状态全部为 NOT_RUN。

## 9.1 本轮新增的遗漏收敛

| 复审遗漏 | 源码处理 | 执行证据 |
| --- | --- | --- |
| AppContainer 未知 explicit ACE 保留、路径命令竞争 | 独立原生 package 策略、受控句柄、exact DACL 回读 | NOT_RUN |
| LSP 预算晚建立、实际复制无界、取消提前释放名额 | 首步共享 budget、actual bytes/全部 entry 限额、资源 lease 保留到清理完成 | NOT_RUN |
| ConPTY 生产关闭与测试 drop(pair) 不一致 | 原生用例调用生产 spawn/terminate owner，保留排空与取消 I/O 顺序 | NOT_RUN |
| 输入背压阻塞全局表锁、输出最长 30 秒不可取消 | per-session bounded writer、全局锁外控制、20 ms stop 观察 | NOT_RUN |
| 超时后丢弃 JoinHandle | 独立清理 owner 与 pending 状态；容量包含待清理资源 | NOT_RUN |
| 前端吞输入/关闭错误 | 有界 UTF-8 分块、背压重试/取消、中文失败与清理中反馈 | NOT_RUN |
| Profile 周期 deep audit 与活跃缓存 churn 冲突 | 内核准备凭证、冷启动深扫，热路径仅 root audit | NOT_RUN |
| 每个 Grid 重扫同一数据树 | 同一 root identity/volume/policy epoch 共享准备 | NOT_RUN |
| 外部 URL 泄露到 stderr/audit | 只记录固定 scheme 类别和字节数，无 host/path/query/fragment/userinfo | NOT_RUN |
| audit 原子替换仍丢并发更新 | 独立私有锁文件，LockFileEx/flock 覆盖读改写和损坏备份全段 | NOT_RUN |
| 固定 mvp-life.log | 改为已有 bounded private audit 的 lifecycle 事件 | NOT_RUN |
| XDG/系统路径别名升级兼容 | 配置 base 一次解析，应用自有叶继续严格拒绝链接 | NOT_RUN |
| 手动流程只存在功能分支不可调度 | 复用默认分支已有 dispatcher 路径，原生文件转 workflow_call | NOT_RUN |
| 三个测试入口可能零执行/依赖漂移 | 统一结果计数器，Profile/vendor 使用主应用锁文件 | NOT_RUN |

不存在“没有运行就改写成 PASS”的补丁。Windows/Linux 编译、原生效果、WebView2、NSIS/安装升级、BFC 真实账号、GUI 与真实性能仍需另行执行；macOS Grid、LSP sandbox、WKWebView 数据隔离及签名分发也没有在本请求中实现。

## 9.2 第三轮：哪些复审意见成立，哪些需要限定

对照的是 add44f3b 的实际源码，不把几份重复审查相加成“新缺陷数量”。源码缺陷、条件性兼容风险和按请求未运行的验收分别记录：

| 复审意见 | 判断 | 本次源码处理 |
| --- | --- | --- |
| ALL APPLICATION PACKAGES 的 generic RX 包含列目录 | 成立；旧注释已有 RX/list 说明，但权限超出仅遍历需要 | 完全自有子树收窄为 FILE_TRAVERSE，own package 保留 RX/M，补跨包 list 拒绝正反例定义 |
| profile 即使合规仍申请 MAXIMUM_ALLOWED | 可以确认有无条件改权打开；共享冲突尚未复现 | 先完整只读 probe，确实缺 grant 才打开修改；继续用原生身份复核 |
| 32 个终端按每个 4 秒等待，应用无总截止 | 成立 | 关闭入口后，各资源独立启动清理，共享 APPLICATION_SHUTDOWN_BUDGET=10 秒的绝对截止；依赖关系保留 flush→close-tabs→Grid |
| 进程退出后内存 reaper 不能完成 Profile 清理 | 成立 | AppContainer 持久记录、活动 owner/Job 证据和下次启动恢复；未知记录保留，不借任意日志路径删除 |
| Channel 早到数据被 spawn 返回后的初始化清空 | 成立 | starting slot 在调用前存在，返回时绑定身份，保留已收数据 |
| 隐藏终端无界输出、关闭后迟到消息重建 buffer | 成立 | 输出有字节及条数预算、截断提示；关闭 slot 丢弃后续消息，不累积无限 tombstone |
| 每批 from_utf8_lossy 损坏跨批字符 | 成立 | 流式保存不完整 UTF-8 尾部；有效中文/emoji 跨 flush 不替换，真实无效字节及 EOF 残缺有明确规则 |
| 测试超时只证明主进程退出 | 成立 | 原子 Job 归属、完整进程数归零，未知清理阻断后续用例 |
| 过滤器仅 passed>=1 不能发现必需用例减少 | 成立 | required 用例 manifest、执行前枚举、逐名结果、数量一致性；不把无效旧过滤器保留为证据 |
| default branch 与 ref 输入不同，所以分支入口一定不能用 | 这个结论不成立 | 使用已登记 workflow 路径与显式 ref；CLI 交互询问时从所选 ref 读取输入定义，显式输入与同一 ref 一起交给 dispatch API。网页默认表单不作为执行契约；实际 dispatch 仍 NOT_RUN |
| 冷准备租约没有持续子对象监测 | 是机制边界，不能表述成已实现持续防篡改 | 明确 root-only 热路径只证根身份/ACL；不重新在创建标签页时深扫，不把后台抽查包装成对同一普通用户进程的隔离保证 |
| 新 SHA 没有 Windows/Linux/GUI PASS | 事实，但不是又一个源码缺陷 | 按用户要求不执行；所有对应状态保持 NOT_RUN |

### 应用退出与残留恢复的边界

关窗立即关闭终端 bridge 发布入口；native terminal admission 和 Windows LSP/AppContainer admission 同步进入关闭状态。已经进入 OS 创建但尚未发布的终端也计入资源许可，返回后再次检查关闭状态；不能把 session 重新插回已经 drain 的表。LSP 启动预留、已发布会话和清理中的会话都保留容量，接收循环观察应用关闭，不等待原请求超时后才取消。

ShutdownCoordinator 在独立线程启动各资源 owner，所有原生等待接收同一个 absolute deadline；慢任务不阻止其他独立资源开始释放。会话落盘与关闭 WebView 仍有明确依赖。线程创建失败时保留闭包及其资源，依赖该任务的后续任务保持 Pending，不伪装成已经执行。各任务结果全部返回后，还需 join 已启动 task owner，owners_complete 才能使总报告成功；不能只依据结果列表全为 Ok。截止后报告 Pending/失败并以非零退出；scheduler/task JoinHandle 在进程仍存活时继续由 owner 持有，不能将“协调器给出了报告”写成“所有资源清完”。

应用退出仍会终止这些内存 owner。可恢复的 AppContainer 状态必须在资源逃逸前记录持久 ownership intent，并用严格 BrowserOS 名称、原生派生 package SID、OS 解析位置、根对象身份和活动 lease/Job 再确认。恢复不使用 journal 里的任意路径作为打开或删除目标；身份不符、链接和仍持有独占记录 lease 的活跃 owner 都不能被接管。若已获得记录独占权但该记录绑定的 Job 仍活跃，则进入保留 lease 的清理流程，不能当成恢复完成或直接删除仍被进程使用的 Profile。

文件删除走有界句柄相对清理；注销另用 DeleteAppContainerProfile 名称 API，不能宣称它变成了句柄 API。绑定阶段识别 OS Profile 的整个 package 范围，记录并固定它外侧 namespace 的原生身份；恢复时从 volume/file ID 再打开并复核，持外侧 pin 跨注销调用，不持有 API 将删除范围内的文件句柄。布局无法确认、外侧目录身份改变或存在不可信写权限时保持 Pending。未完成根身份绑定的创建崩溃、旧版没有记录的遗留 Profile，都不属于可自动证明的删除对象。

恢复初始化、正在扫描的恢复生产者、Profile controls、待清理任务及 reaper 自身都纳入关闭屏障。不能在队列暂时为空时漏掉稍后登记的恢复任务，也不能在 registry 尚未安装成功时忽略已经启动的清理线程。服务停止只发生在关闭接收且生产者和资源都归零后；只 join 已结束线程，超时保留 owner 和记录。无法证明所有权或未在预算内完成时保留记录，阻止新的 sandbox 准备并输出固定错误码，不删除未知目录。

主进程在私有 workspace 已准备后执行一个 2 秒预算的恢复阶段；Grid 只初始化记录存储，不重复扫描。恢复失败不令整个 GUI 崩溃，Agent 的原生 sandbox 功能保持阻断。预算在原生调用边界检查，不能硬中断任意文件系统驱动调用；应用级等待总截止和文件系统单调用可中断性不是同一承诺。

### 保留的实际验收空白

本轮没有调用原生测试执行器、Add-Type、Cargo、Node/Python 校验器、构建器、checksum、workflow_dispatch、安装包或 GUI 驱动。新的回归定义证明的是以后必须覆盖的场景，不是已经获得的运行结果。Windows 原生效果、Linux 回归、同一 SHA 的安装版、真实 BFC 会话与大缓存性能都仍是 NOT_RUN。

### 下次移植 macOS 时沿用的约束

将身份/权限语义、资源所有权、数据流边界、容量限制与证据归属分别设计：不把 POSIX mode 当作 extended ACL 的全部语义，不把线程已入 reaper 当作应用退出完成，不把一次字符串转换当作流式 Unicode 解码，也不把一个过滤器通过当作整组用例已执行。先明确每项 native 能力及失败行为，再补真实平台用例；源码更新和运行证据分别交付。

## 9.3 第四轮：核对 e039c83a 的复审意见

这次把几份复审中的重复项合并后，按实际调用链确认。不能把“尚未运行”重复算作源码遗漏，也不能仅凭函数名猜测是否递归或依赖包是否可被 Cargo 选中。

| 意见 | 核对结论 | 本轮处理 |
| --- | --- | --- |
| 退出自动保存失败仍被计为成功并清草稿 | 成立，是数据保存错误传播缺陷 | 内部逐项 Saved/Failed/Skipped 报告；失败和变化草稿保留；保存失败、busy、panic 均不启动退出清理 |
| Grid 初始化 journal 再递归扫描全部 app data | 误报 | 第二参明确是 deep=false；只有 Walk.visit 的 deep 分支枚举子项。保留浅层身份/ACL 复读，补无关子树不被访问的原生回归定义 |
| 恢复 pending/rejected 后没有用户可用的再扫描入口 | 成立 | 有界只读状态、固定原因码、记录列表与显式安全重试；完整复扫成功才清健康失败标志 |
| Grid 关闭忽略全局 deadline、错误与子进程结果 | 成立 | 应用退出独立 admission、同一截止时间、进程/Job/线程所有者及未完成报告；普通关闭保留再次打开能力 |
| LSP 先等待完才汇总 Sandbox | 诊断和调度改进，不能据此认定已有进程泄漏 | 两个独立 coordinator 任务收到同一绝对 deadline，各自返回结果 |
| 从 src-tauri manifest 用 -p 选 portable-pty/browser-tabs 必定选错 workspace | 误报 | 两者是应用的 path dependencies；Cargo 可以按 -p 测试依赖包。保留已有 --locked 和无 dev-dependency 约束，不改独立 workspace 锁文件 |
| 当前没有 Windows/Linux/GUI PASS | 属实，按本轮明确要求未运行 | 所有执行状态继续 NOT_RUN，不伪造验收结果 |

Cargo 行为依据：[Cargo 维护者对依赖包 -p 选择的解释](https://github.com/rust-lang/cargo/issues/13962)及 [cargo test 包选择规则](https://doc.rust-lang.org/cargo/commands/cargo-test.html#package-selection)。依赖 crate 将来新增 dev-dependencies 时仍须重新审查包/锁归属，不能移除 --locked 规避问题。本轮没有执行 cargo metadata 或 cargo test。

### 会话保存和退出的完整交接

退出保存现在是不可逆清理之前的预检。普通 flush 保持原有公开 DTO，遇到部分失败返回 Err。内部报告保留各 tab/session ID、成功/失败/跳过与并发变化；原始保存错误不进入全局事件或日志。只有成功落盘且草稿仍等于快照时才删除该草稿；同时复读权威 tabs 的新增、关闭、URL/标题变化。保存、删除和策略更新共享写入互斥，竞争返回固定 busy 结果。

专用 prepare_exit_sessions 在最后一次比较时同时持有 writer、drafts、tabs 锁，确认成功后设置既有 bridge admission flag；之后没有文件 I/O 或可失败的业务步骤。main 只在 can_exit 成功后启动原生清理。Browser 新建/导航/标题的最终发布在同一 tabs 锁下检查该 flag，不能将迟到结果写回已接受的退出快照；迟到的 WebView 交还原有插件 owner 关闭。主进程退出任务枚举实际插件持有的 WebView，避免把休眠页签或未发布资源误当作普通表项。

保存失败、发生并发变化或保存 panic 时，窗口和页签保留，状态栏持续显示退出已取消及重试办法；普通短消息不会覆盖它。保存 worker 的 JoinHandle 从 spawn 到发布使用同一槽锁，失败后必须结束并 join 才能开始下一次退出，避免快速失败重试覆盖旧 handle。

移除了 bridge 关闭任务中无条件成功的重复 flush，以及不能等待结果的 beforeunload 保存。普通 Tab 关闭仍然不保存、不确认；用户未启用 auto_save_on_exit 时不静默落盘。资源瀑布仍是点时快照，既有 50 条存档容量和强制杀进程边界未改变。

10 秒预算从保存预检已被接受后的不可逆资源清理开始；不能用保存超时作为丢弃页签后强退的理由。文件系统单次调用仍没有硬中断承诺。

### 安全恢复可诊断、可重试

RecoveryStatus 仅输出固定状态/原因码、opaque 记录编号、上次扫描计数和当前 pending cleanup 数量，不输出路径、SID、原始 journal 或错误文本。最多枚举 512 个元数据、返回 64 条内容，诊断预算上限 750ms；重试最多 10 秒，并受传入截止时间缩短。状态和重试各自使用独立 native 枚举 cursor，避免两个读取调用共享目录 handle 时互相影响。

Coding Workbench 的固定布局面板提供重新检查、显式安全重试和待核对记录。重试使用 startup 已绑定的 REGISTRY，前端不能传目录或记录 ID 作为删除目标。只读查询不会清除 rejected；后台清完一个资源也不等于所有记录都安全。只有完整复扫成功且没有未完成 owner，才恢复 admission。

未知 Intent、身份不匹配和不能证明归属的旧资源仍保留，面板显示需人工核对及固定原因；没有“忽略并删除”按钮。新入口解决可证明资源在暂时占用/预算不足后的恢复与健康重置，不宣称能自动处理没有所有权证据的资源。

接入面板时还核对了语言服务调用方：Windows lsp_status 原先没有提供前端选择器实际读取的 path/name，本轮补齐可信 rust-analyzer 的这两个字段，保留既有工具链准备错误和 compilation_verified=false。

### Grid 与 Agent 的退出结果

Grid 的应用关闭先停止接收并覆盖在途创建，再向资源 owner 发出终止请求。最终成功需要子进程/Job、相关 I/O worker 和清理 owner 均完成；未结束的资源保留在进程内清理登记中并返回 Pending/错误，不能仅因已经调用 kill 就汇报成功。Linux 清理进程组时保留 leader 身份，Windows 独立检查 Job active process count；共享 deadline 不为每个进程重新计时。焦点显隐调度也纳入同一 worker 所有权，保持原有 150ms 延迟和显隐算法。

LSP 与 Sandbox 分别登记独立关闭任务。Grid、Terminal、Agent 共享同一个清理截止时间，但各自拥有和报告资源。进程强制结束之后仍无法由内存登记继续清理；AppContainer 的持久恢复记录继续承担下次启动的已证明资源恢复。

### 本轮交付和执行边界

增加/同步了真实保存失败、草稿并发替换、新增草稿、权威页签变化、opt-out、退出 worker 重试、恢复记录只读观察、未知 Intent 保留、预算失败后安全重试及 Grid 生命周期的回归定义；原生名称清单也同步。所有定义仅写入仓库，未执行。

没有触发 Actions、测试、校验器、构建、checksum、安装包、GUI 或合并；没有修改 master、创建 PR 或 dispatch 工作流。安装版、Windows 原生效果、Linux 回归和性能数据保持 NOT_RUN。持续子对象 ACL 监控仍不是冷准备租约的承诺，不会为了制造“全部完成”而把它写成已实现。

这轮暴露的改进点是：审查模块时同时跟到真实调用方、失败分支和最终消费点；核对布尔参数与工具合同后再认定缺陷；修订测试定义必须保留“未执行”的事实。未来 macOS 移植同样需要区分源码合同、调用链接线与实际平台证据。

## 10. 源码索引

- [共享 Windows 安全实现](../../../crates/browseros-platform-security/src/windows.rs)
- [Windows 安全路径消费者](../../../src-tauri/src/capabilities/workspace/safe_paths_windows.rs)
- [Unix 私有根](../../../src-tauri/src/capabilities/workspace/private_directory_unix.rs)
- [workspace 准备与 getter](../../../src-tauri/src/capabilities/workspace/workspace.rs)
- [启动准备与 URL 队列](../../../src-tauri/src/startup.rs)
- [main 平台选择与启动接线](../../../src-tauri/src/main.rs)
- [browser-tabs profile 接线](../../../tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs)
- [Windows LSP](../../../src-tauri/src/capabilities/agent/coding_lsp_windows.rs)
- [Windows AppContainer](../../../src-tauri/src/capabilities/agent/windows_sandbox.rs)
- [portable-pty 本地补丁说明](../../../src-tauri/vendor/portable-pty/README.md)
- [可复用 Windows 原生工作流](../../../.github/workflows/windows-linux-v2-experimental.yml)
- [源码契约定义](../../../tests/check-windows-linux-v2-source-contract.mjs)

### 第二轮新增源码

- [跨进程私有目录准备](../../../crates/browseros-platform-security/src/windows/lease.rs)
- [AppContainer 独立原生策略](../../../crates/browseros-platform-security/src/windows/appcontainer.rs)
- [跨进程审计锁](../../../src-tauri/src/capabilities/workspace/audit_lock.rs)
- [终端生产生命周期](../../../src-tauri/src/capabilities/terminal/terminal.rs)
- [已登记的手动调度入口](../../../.github/workflows/browseros-developer-feedback.yml)
- [required native 统一计数器](../../../scripts/windows-native-test-runner.ps1)

### 第三轮新增源码与官方合同

- [应用级关闭协调器](../../../src-tauri/src/shutdown.rs)
- [AppContainer 原生恢复基础设施](../../../crates/browseros-platform-security/src/windows/recovery.rs)
- [Agent Profile 持久生命周期](../../../src-tauri/src/capabilities/agent/windows_sandbox/lifecycle.rs)
- [终端有界输出与启动状态](../../../src/capabilities/terminal/state/useTerminalStore.ts)
- [Windows 测试进程 Job supervisor](../../../scripts/windows-native-test-supervisor.cs)
- [必需原生测试名称清单](../../../scripts/windows-native-required-tests.json)
- [GitHub 手动调度及指定 ref 规则](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)
- [GitHub CLI 的所选 ref 与输入读取实现](https://github.com/cli/cli/blob/trunk/pkg/cmd/workflow/run/run.go)
- [DeleteAppContainerProfile 的重复调用与句柄关闭合同](https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deleteappcontainerprofile)
- [Windows 嵌套 Job 的子进程与终止语义](https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs)

### 第四轮新增源码

- [退出保存报告与原子接受](../../../src-tauri/src/capabilities/session/commands.rs)
- [Grid 共用清理所有权](../../../src-tauri/src/grid_lifecycle.rs)
- [Grid Windows 进程适配](../../../src-tauri/src/grid_process_windows.rs)
- [Grid Linux 进程适配](../../../src-tauri/src/grid_process.rs)
- [恢复诊断与重试面板](../../../src/capabilities/agent/ui/SandboxRecoveryPanel.vue)
