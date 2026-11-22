# Windows 原生安全收敛修复台账

日期：2026-11-22  
仓库：`code5m/browser-os`  
修复分支：`fix/windows-native-security-convergence-20261122`  
起点：`fix/windows-private-dir-acl-case`，冻结基准 `285542f57bd055592b4d5019e5a89e8555bcc805`

> 本轮交付是源码、回归定义和文档的修改。没有运行测试、校验脚本、构建、CI、NSIS、GUI 或性能测量。文中的“已修改”不代表原生问题已修愈，也不代表 Windows/Linux 回归通过。macOS 只有明确列出的源码准备，不能据此宣布 macOS 已支持。

## 1. 状态与证据口径

本台账在新提交形成前编写，因此不预填一个尚不存在的“已验收 SHA”。提交完成后，以新提交的完整 40 位 SHA 作为未来人工复验输入。分支名、旧测试日志、另一次运行的构建产物都不能替代这个 SHA。

| 证据层级 | 本轮状态 | 可以据此说什么 |
| --- | --- | --- |
| 源码修改与源码互审 | 已完成本台账所列修改；存在明确未实现项 | 说明实现选择、错误路径、平台分支和预期行为 |
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
| W04 | 每次访问都尝试改 ACL，遇到活跃 WebView 文件占用；把 fast path 误写成“不扫描” | 先只读审计，合规则 0 repairs；只在可修复策略错误或路径缺失时进入迁移；profile 加入进程内浅层/周期深层复核 | 共享 crate；browser-tabs `commands.rs` |
| W05 | 小写字符串前缀不能证明 Windows 路径授权，Unicode、大小写或 UNC namespace 易误判 | 用实际句柄身份和祖先身份判断；保留 UTF-16；拼写比较使用 CompareStringOrdinal；先区分磁盘或 server/share namespace | `security_policy.rs`；`safe_paths_windows.rs` |
| W06 | 数据准备留在 getter 内，错误变成 panic；误以为 Tauri setup 的 Err 会落入 build 的错误处理 | 显式准备并发布 PreparedDataDirectory；主进程与 Grid child 在 setup 中直接报告准备失败并退出 | `startup.rs`；`main.rs`；`workspace/workspace.rs` |
| W07 | 单实例插件或 macOS Opened 可能先于应用 setup 到达，直接审计会读取未准备的根目录 | 增加有界内存 URL 队列；统一冷启动、单实例、Opened 入口；准备成功后原子交接队列 | `startup.rs`；`main.rs` |
| W08 | LSP runtime 选择、rustc peer 或 stage 错误被弱化，初始化完成被误当作完整编译能力 | 显式传播配置和 stage 错误，runtime 只选一次；必需 rustc metadata/cfg；禁用 Cargo/buildScripts/procMacros；声明 compilation_verified=false | `capabilities/agent/coding_lsp_windows.rs` |
| W09 | 将 AppContainer 包目录误归为普通 owner-only 私有树，或在创建失败路径泄漏资源 | 保留独立 package SID 权限策略；限制扫描；加强 state 初始化、句柄、参数和失败路径资源回收 | `capabilities/agent/windows_sandbox.rs` |
| W10 | ConPTY 测试因无控制台而假 PASS；kill 后无界 wait、异步等待句柄及 HPCON 关闭顺序有漏洞 | 移除 GetConsoleWindow 逃逸，增加真实退出/EOF 路径；处理 kill 错误与清理超时；修补 vendored portable-pty waiter、尺寸和关闭顺序 | `windows_process.rs`；`terminal.rs`；portable-pty vendor |
| M01 | `not(linux)` 被当成 Windows，mac 编译路由缺失或返回 Windows 错误 | 显式 Linux/Windows/unsupported 选择；未实现能力返回真实平台与明确错误；Grid 不伪造 IPC 成功 | agent 模块、`grid_process_unsupported.rs`、`bridge/grid_runtime.rs`、`bridge/system_commands.rs` |
| M02 | Unix 私有根仅靠路径 chmod；macOS extended ACL 不受普通 mode 位充分约束 | 新增 Linux/macOS 句柄锚定私有根准备；验证 UID、0700、身份；mac 原生清除并复读 extended ACL | `workspace/private_directory_unix.rs`；`safe_paths.rs` |
| M03 | keyring 未启用 apple-native，mac 默认可能落到 mock；锁文件只改直接依赖不完整 | 启用 apple-native，并补齐安全框架的跨 target 锁解析与版本消歧 | `src-tauri/Cargo.toml`；`src-tauri/Cargo.lock` |
| G01 | 源码契约、筛选测试、full suite、GUI 结果被混成一个“通过”；移动分支导致证据串 SHA | 工作流改为 manual-only、完整 source_sha、固定 checkout；required 零匹配/ignored/SKIP 失败；full suite 独立记账 | `.github/workflows/windows-linux-v2-experimental.yml`；`tests/check-windows-linux-v2-source-contract.mjs` |

另有两项文档口径修正：`session.rs` 与 `system_commands.rs` 的原子替换注释修正，保留既有 Windows MoveFileEx 重试行为；工作流治理文件、WORKFLOW-CATALOG 与 parity 矩阵同步 manual-only 和 `NOT_RUN`，不据文档调整宣布新结果。

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
| AppContainer 独立策略 | 一次有界扫描，8,192 个对象 | 32 | 不据此承诺外部命令可硬中断 | 与共享 owner-only 策略不同 |

普通 ensure 的 fast path 仍做完整 deep scan，收益是原生 API 和合规时 0 写入、0 mutating opens；不是缓存命中后完全不扫描。root-only API 是明确的浅层例外。浏览器插件每进程首次完整审计，缓存后每次浅层复核，600 秒后进行深层只读复核；失败不能推进缓存时间。

时间预算在对象边界、目录枚举页和原生调用之间检查，不能硬中断某一次阻塞的文件系统调用。因此“配置了 15/30 秒”不等于“实测启动一定在 15/30 秒内完成”。多 Grid 首次审计、杀毒软件占用、大缓存、网络文件系统和 ReFS 均未完成本轮性能或兼容性验收。

### 3.4 失败不是事务回滚

只有结构化的 ACL_NOT_PRIVATE 或根路径缺失进入迁移。读取失败、权限拒绝、预算不足、未知 ACL/package policy、reparse/hardlink、树变化等错误不能触发无差别强修。

迁移可能已安全收敛一部分对象后失败。这时不删除原数据、不改变内容字节、不发布成功状态，**也不得把已经安全的 ACL 回滚到旧的不安全 ACL**。错误报告保留阶段、计数和原生错误等可诊断信息。处理占用者、所有权或系统错误后，可以重新执行同一幂等策略；不能承诺任何不受控异常都自动恢复。

私有根限定本地磁盘。通用路径 identity/ancestry helper 支持磁盘与 UNC，但先比较磁盘或 server/share namespace，避免文件 ID 跨命名空间碰撞。`safe_paths_windows.rs` 原本的 UNC 拒绝边界保留；通用授权 helper 不因此新增“所有 UNC 一律拒绝”的回退。

## 4. 启动屏障与早到事件

Tauri 2.11.5 在 build 阶段初始化插件，在 RuntimeRunEvent::Ready 中执行应用 setup；setup 的 Err 会被 Tauri 转成 panic。单实例插件因此可能先接到 URL，而应用的数据根尚未准备。本轮不再依赖外层 build/run 的 Result 捕获这个准备失败。

两处 setup 的第一项业务动作都是 prepare_workspace，失败后直接调用 report_failure 并退出。Windows 报告使用不需要 WebView profile 的原生 MessageBox；没有临时公共目录回退、自动删除数据或自动提权。

成功路径依次为：准备私有 workspace 根并发布 PreparedDataDirectory，尝试启用非 Linux 私有诊断日志，释放早到 URL。data_dir getter 只读已发布状态，不再承担隐式目录迁移。

PendingOpenUrls 在 Builder 中管理，早于插件初始化生效。准备之前最多保存 64 条、每条 8,192 字节；超限、队列不可用或锁中毒会明确拒绝，并只记录错误代码，不记录原始 URL。ready=true 与提取 pending 在同一 Mutex 内完成，随后在锁外调用审计和 URL 处理。新到请求可与已取出的旧请求交错处理，但不会掉入“已取空队列、尚未置 ready”的丢项窗口。

单实例、冷启动 argv、macOS Opened 共用该入口。保留单实例提前转交行为，不要求第二实例为转交 URL 先重新迁移完整数据树。`app.windows=[]` 必须保持：Tauri 会在应用 setup 前创建配置窗口，source-contract 对此前置配置建立约束。

本屏障约束 workspace 与非 Linux 日志初始化，**不代表所有持久 I/O 都被拦截**。Linux 原有 early crashlog 继续在 prepare 前创建/写入/清理自己的日志。该行为本轮保留，不包装成已收敛到新私有根的结果。

## 5. LSP、AppContainer 与 ConPTY 的真实能力边界

### 5.1 LSP 的可初始化、可分析与可编译不是同一结论

Windows LSP 明确传播 rustup 配置、必需 rustc peer 和 stage 失败；runtime 只选择一次，避免探测通过后又启动另一个 runtime。rustc metadata/cfg 属于必需输入，不能空缺后仍报告完整成功。Cargo、buildScripts、procMacros 维持禁用，返回 compilation_verified=false，不把语言服务初始化当作完整项目编译证据。

Session 保存两个 I/O JoinHandle。关闭时处理 Job、队列、线程与进程，线程/进程等待有界；close_complete 和 cleanup_errors 反映实际清理情况，不在资源未回收时报告关闭成功。本轮仅修改实现和回归定义，真实 LSP 收发、关闭与子进程残留未重测。

### 5.2 AppContainer 保留独立 package 策略

AppContainer 目录仍通过 GetAppContainerFolderPath 获取，位于 app_data 之外。state 给 package 修改权限，runtime/snapshot 给读取执行权限；generic AppContainer 仅获得目录的非继承 RX（含列目录/遍历），不继承到源码文件。这类 package SID 树不能套用普通 owner+SYSTEM owner-only 规则，共享安全 crate 遇到 S-1-15-* 或不支持的 ACE 类型会明确拒绝。

本轮把 ACL 合并约束为一次有界扫描，state 只初始化一次并持有 no-delete 句柄；spawn 参数和环境前置处理，补齐 attribute-list、Win32 字符串分配错误路径的资源释放，Job 先于 profile 回收。

**剩余边界必须保留在待办中：** AppContainer package 策略仍使用独立 icacls 命令，没有改成共享句柄化 owner-only 实现；命名路径到命令执行之间的竞争，以及外部命令阻塞风险，尚未靠新的原生证据排除。不能用“普通私有根已采用 native API”推导 package ACL 同样已经完成替换。

### 5.3 ConPTY 与 vendored portable-pty

`windows_process.rs` 去掉依赖 GetConsoleWindow 的假 PASS 路径，回归定义要求真实执行、限时等待退出及 EOF/join。`terminal.rs` 避免 kill 失败后继续无界 wait，线程清理超时明确返回错误；`/proc/{pgid}` 不存在断言仅限 Linux，其他 Unix 保留真实 process_group_alive 断言。

vendor 修改集中在以下四个已有文件：

- `src-tauri/vendor/portable-pty/README.md`：记录本地补丁范围和后续维护要求。
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

仅收敛应用私有根，不 chown、不跟随 symlink，也不无条件递归改 Linux 现有树。fchmod(0700) 后复读类型、owner、mode 和 dev/ino，再从命名空间重走确认身份；Linux 通过 mode 的 group-class mask 限制 POSIX ACL 有效权限，不新增按少数磁盘类型维护的广泛文件系统白名单。

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
| 活跃 profile | 浏览器运行期间只读合规审计不需 mutating open | 占用或不合规时明确失败；缓存失败不推进时间 | 同 SHA GUI/WebView2 进程与 profile 记录 |
| Windows 路径 | 非 ASCII、Unicode 大小写、允许的 UNC 同 namespace | 相似前缀、不同 share/volume、文件身份替换 | 授权允许与拒绝双向结果 |
| 启动 | 主进程和 Grid 准备后启动；早到 URL 不丢 | ACL 准备失败原生对话框；队列满/超长；Ready 前 URL 不读 workspace | 原生启动/事件记录；不能由 source-contract 替代 |
| LSP | 指定 runtime 与必需 rustc、初始化/请求/关闭 | 缺 peer、stage 失败、关闭超时、子进程残留 | 真实协议、Job/进程/线程清理结果；不声称编译验证 |
| AppContainer | package 可读执行 runtime/snapshot、可写 state | 越界访问拒绝、禁止写 runtime/snapshot、ACL 竞争与命令失败 | 包 SID 下真实访问与失败记录 |
| ConPTY | 无交互控制台宿主也执行；输入输出、尺寸边界、退出 EOF | kill 失败、句柄关闭竞争、越界尺寸、线程超时、后台孙进程 | 原生执行及进程树/句柄清理证据 |
| Linux 保持 | 原有正常私有树、终端/进程组与浏览器主路径 | ACL/符号链接拒绝、清理异常、错误传播 | 独立 Linux 结果，不借 Windows 通过推导 |
| macOS 存储/Keychain | APFS 两类大小写配置、extended ACL 清理、真实 Keychain | symlink、异 UID、ACL API 不支持、Keychain 拒绝/锁定 | macOS 原生访问与重启结果 |
| macOS GUI/分发 | Finder 启动、WKWebView store、双架构签名安装 | PATH 缺 runtime、TCC 拒绝、helper 签名/架构错误 | 打包 GUI 与系统权限结果，未实现项保持 unsupported |
| 性能 | 小/大已有树、首次及重复启动、多 Grid | 限额触发、单调用阻塞、占用、失败重试 | 对象数、修复数、耗时、硬件/文件系统；无性能证据不得报达标 |

## 9. 工作流、复验记录与收尾规则

`.github/workflows/windows-linux-v2-experimental.yml` 只接受 workflow_dispatch；输入 source_sha 必须是完整 40 位十六进制提交标识，各 job 固定 checkout 同一个 SHA。manual-only 表示之后可以人工触发，不表示本轮已触发，也不授权额外运行。

`tests/check-windows-linux-v2-source-contract.mjs` 是源码接线约束，纳入启动准备、空配置窗口、共享安全依赖、平台选择和明确的测试入口。未来共享 crate 入口包含 `--locked`，portable-pty 库测试通过 host manifest 独立选择。required 筛选零匹配、ignored 或 SKIP 必须失败；full-suite 状态单列，不能再参与 filter 通过数计算。

每次人工复验记录至少包含：完整 source_sha、run/artifact 链接、OS/架构/文件系统、构建及功能参数、筛选名与实际执行数量、passed/failed/ignored、full-suite 独立结果、GUI/性能是否执行。原生测试未运行、被跳过、仅构建成功或仅源码契约通过时，对应功能仍记未证实。

本轮没有创建新原生结果，因此结论限定为：统一了普通私有树的权限实现与错误边界，补齐启动时序和平台接线，并建立可人工复验的回归定义。AppContainer 独立策略的路径竞争、真实大树性能、原生 GUI/进程回收和 macOS 产品支持仍需按矩阵取得证据；不得用旧 run 的结果填补这些空白。

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
- [manual-only 工作流](../../../.github/workflows/windows-linux-v2-experimental.yml)
- [源码契约定义](../../../tests/check-windows-linux-v2-source-contract.mjs)
