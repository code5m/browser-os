# BrowserOS 重构完成条件：master 只读架构审计与逐项验收清单（2026-11-01）

> **仓库**：`code5m/browser-os`  
> **唯一审计基线**：`master@9607e260f86b15a730219c619ab5a20cc01efe65`  
> **审计分支**：`research/architecture-reconstruction-audit-20261101`（从上述 master SHA 创建）  
> **执行范围**：仅通过 GitHub 读取仓库源码、治理登记、检查脚本和 Actions 证据；本分支只新增这份文档，不修改应用源码、测试、workflow、构建配置，不合并 master。  
> **状态口径**：`PASS` = 当次证据完整且验收命题已证明；`PARTIAL` = 部分实现/证据，但不具备全量证明；`FAIL/GAP` = 发现代码层面的反例或必要门禁缺口；`NOT_VERIFIED` = 本轮没有真实执行/证据，不等同 FAIL。  
> **重要限制**：本次未在 Ubuntu 本机安装/启动 GUI、未在本地执行 Rust/JS 测试、未做性能配对测量。CI 历史运行不是本次分支的重新验收；静态阅读结果须通过后续行为测试复现。

## 0. 执行结论

**当前不能宣称完成本次重构的十项验收。** BrowserOS 已具备 Capability Manifest、Runtime、Public Facade、Semantic/Native Registry、贡献槽及多项门禁的基本框架，但仍有可直接定位的关键缺口：

1. **[P0 / 已见代码] 同步 Runtime 可能提前宣告异步激活成功**：`src/capability/runtime.ts:185-209` 的 `activate/suspend` 直接调用允许返回 Promise 的生命周期 hook（`src/capability/types.ts:68-70`），而不等待结果即转为 ACTIVE/SUSPENDED。虽然 `activateAsync/suspendAsync` 已正确等待（runtime.ts:212-236），但默认 bootstrap 仍调用同步入口（`src/capability/index.ts:159,197`；`src/capability/platform/pluggable.ts:64`）。验收需要专门负例证实晚失败与资源回滚。
2. **[P0 / 已见代码] Hot-Plug 加载/回滚存在竞争窗口**：`src/capability/platform/orchestrator.ts:108-113` 的 `addContributions` 再次调用 loader 且丢弃返回 Promise；`src/capability/platform/hotplug.ts:114-115` 已经先调用 `await req.load`、然后 `target.addContributions`，可能触发加载两次、异步注册晚到。失败回滚部分采用 best-effort，不能仅凭函数注释宣称原子性。
3. **[P0 / 已见代码] Hot-Plug 权限拒绝不是必经流程**：`src/capability/platform/hotplug.ts:101` 使用可选 `req.permit`；`src/capability/platform/orchestrator.ts:133` 调用 `requestAdd` 没有传递 permit。应验证生产流中其他权限层是否补上，这不是对所有 Native API 已绕过鉴权的断言。
4. **[P0 / 已见代码] 生命周期“destroy”不等于资源真正释放**：`src/capability/resourceGovernor.ts:61-63` 仅 `runtime.disable`；`src/capability/types.ts:60-70` 没有 onDestroy hook，且 `runtime.disable` 拒绝 ACTIVE（runtime.ts:244-265）。`src/capability/platform/pluggable.ts:96-100` 在同步路径未等待 `onDeactivate`；`hotplug.ts:229-231` 的 disable 同样 fire-and-forget。应明确 destroy 的实际契约，而非仅用声明替代证据。
5. **[P0/P1 / 已见证据] 安全能力清单未构成全面强制策略**：`native-commands.yaml` 中共 **178** 个登记项，**178 个有 owner 名**；但 **58** 个 `permission: "-"`（表示登记层无具体权限标签，不代表一定无 Rust 校验）。`src-tauri/permissions/default-commands.toml:1-153` 对主窗口是统一的 `default-commands` 列表；`src-tauri/capabilities/default.json:5-18` 把它授给 main。各关键操作是否有独立作用域校验、二段确认与审计仍需逐命令核实。尤其 `session_delete`、`session_restore`、`eval_in_tab` 等不应仅凭 registry owner 字段判定安全。
6. **[P1 / 已见代码] 依赖边界检查存在静态盲区**：`scripts/check-capability-boundaries.mjs:82-86,126-130` 对不以 `.` 开头的 import 返回 null（常见路径别名不会在这条检查里解析），`isShared` 对若干共享目录放行（41-43,149）；`scripts/check-capability-boundary.mjs:14-15` 是极窄模式正则。故“现有 boundary check 通过”**不能推出**无未经批准的所有跨模块内部引用。
7. **[P1 / 已见文档] 当前文档与源码存在已知漂移**：`docs/architecture/native-boundary/NATIVE_CAPABILITY_BOUNDARY_AUDIT.md:9-10,100` 仍写 148 个命令及集中在 bridge；现在 registry 有 178 个，其中 **100** 个仍记录在 `src-tauri/src/bridge.rs`。`docs/architecture/native-boundary/native-commands.yaml:1785-1803` 的 `reveal_path/move_path` 指向不在当前树中的 `src-tauri/src/fs_cmds.rs`，需核对真实定义文件。另 `src/capabilities/agent/public.ts:5` 仍声称 Skill 无独立 store，而 `src/capabilities/skill/public.ts:6` 和 `state/useSkillStore.ts` 已存在。
8. **[P1/P2 / 未完成] 全量独立模块测试、Ubuntu 安装版人工验收与前后性能比较均无完整闭环**。已有单独的 Vault/Clipboard package 检查和 CI Xvfb 已安装版本测试，但不能代表全体模块独立验收，也不能替代用户桌面验收。

## 1. 十项完成标准逐项审查

| # | 用户验收标准 | 当前判断 | 已有基础 / 本轮直接证据 | 差距与可复核退出条件 |
|---|---|---|---|---|
| A01 | 所有业务模块有唯一 Owner | **PARTIAL** | `docs/architecture/semantic-registry/owners.yaml` 有状态 Owner；`src/capabilities/*/manifest.ts` 有 `semanticOwner`；Agent 与 Skill 已分 store；Workspace 的 `recents` 与 Home 同名见 owners.yaml:995 | 编制以“业务实体/状态/写权限”为键的唯一 Owner 表；全仓识别重复定义和所有跨域写点。共享 *observed* 字段须标明确属派生态而非第二真源；抽取静态+运行测试。 |
| A02 | 所有新功能经正式 Public API | **PARTIAL / GAP** | `src/capability/platform/contract.ts:72-75,112` 定义 `publicContract`；`agent/public.ts`、`skill/public.ts` 暴露 facade | Public facade 直接 export store 不能自动保证最小接口/只读约束；逐个模块确认 API 稳定性、输入输出、语义版本、运行时权限与跨模块调用。增加 alias/动态 import 负例。 |
| A03 | 所有 Native Command 有业务归属 | **PARTIAL**（登记已具备） | `docs/architecture/native-boundary/native-commands.yaml` 有 178 项、均填 owner；`scripts/check-native-capability-boundaries.mjs:184-205` 对未知命令/owner 检查 | 必须核对 Rust 函数集合、`generate_handler!`、Tauri ACL、TS bridge 一致性、每条文件 locator、Owner 和副作用级别；两条 locator 目前已漂移。登记 100 个命令仍在 bridge.rs，物理拆分尚不完整。 |
| A04 | 所有重要资源有明确生命周期 | **FAIL/GAP** | Resource Governor、Manifest resource ownership 和 Hot-Plug harness 已有；Runtime 存同步/异步两类入口 | 逐资源验证出生、暂停、关闭、取消、错误回滚、重启、资源释放；同步/异步生命周期及双加载风险须先修，再做故障注入。 |
| A05 | 不存在未经批准的跨模块内部引用 | **NOT VERIFIED / GAP** | `check-capability-boundaries.mjs` 及 Native boundary check 已有，显式 allowed caller 需理由 | 现有扫描不覆盖所有 alias/动态 import、Rust 模块访问、共享目录越权；需要 AST 和 Rust 模块依赖图并给例外 TTL/理由/审批人。 |
| A06 | 关键副作用有权限和确认控制 | **FAIL/GAP**（覆盖证明不足） | 安全策略、`IntentRegistry`、二段 Git 写入、数据库安全政策与部分确认流程已实现 | `side-effects.yaml` 当前仅 4/40 条 `requires_declaration=true`，36 条 false；这是注释 awareness，不是权限/确认证明。Native permission 有 58 条 “-”；提供逐副作用矩阵及拒绝、过期、取消、重复确认、远程 WebView 测试。 |
| A07 | 模块测试可独立运行 | **PARTIAL** | `packages/capability-vault/package.json`、`packages/capability-clipboard/package.json` 各有 `check`；多项领域/热插拔脚本单跑 | 不能据此认定每个 module 均有独立测试入口、独立 fixture、隔离 side-effect、无全局环境污染；每模块至少 unit/contract/lifecycle/negative test。 |
| A08 | 安装版 GUI 真实验收通过 | **NOT VERIFIED** | Full Validation 使用 `dbus-run-session -- xvfb-run -a ... verify-installed-client.sh --install --resource-minutes 5`；单独自动 GUI 回归 | CI 虚拟显示器里的真实 .deb 启动不等于用户 Ubuntu GNOME 桌面上的完整人工验收。`FINAL-HUMAN-ACCEPTANCE.md:3-5` 为 `HUMAN_VISUAL=PENDING`；必须留安装版 SHA+截图+手工结果。 |
| A09 | 与重构前基线比较性能数据 | **NOT VERIFIED** | 体积基线 `master-7ae6fa86.json`，双基线文档 `FRONTEND-SIZE-DUAL-BASELINE-20261031.md`，5/30 分钟采样脚本与 Workflow | 仅有不同阶段构建体积和历史 CI 指标，没有“同机/同构建/同负载”的重构前后 CPU、RSS/PSS、冷启动和 p50/p95/p99 配对结果；不得写性能提升百分比。 |
| A10 | 文档与实际代码一致 | **FAIL**（有反例） | Native Registry、Semantic Registry、变更归档与契约漂移检查 | 148→178 命令数字不一致、Native 文件路径漂移、Agent Public 注释过期；需有自动生成/逐文件链接/日期化校验。 |

**解释**：A03“登记完整”不代表命令物理归属完成；A08 CI 软件启动成功不等于用户确认的真实 GUI 全功能验收。所有状态仅针对审计 SHA。

## 2. 逐文件迁移与治理修复建议（本次不改代码）

| 优先级 | 具体代码/配置路径 | 观察及核查内容 | 推荐改动（仅下一阶段实现） |
|---|---|---|---|
| P0 | `src/capability/runtime.ts:185-236` | `activate/suspend` 不 await 可能返回 Promise 的 hook；`*Async` 已有 | 生产入口统一异步 transition，或强制同步 hook 的签名/运行时拒绝 thenable；为晚失败、回滚、状态清理增加测试。 |
| P0 | `src/capability/index.ts:75-225` | profile/assembly/configured 三种 bootstrap 使用同步激活或同步 `built.activate` | 统一入口状态机、依赖顺序、失败传播、活跃统计和 ready 确认；保持对现有调用方兼容。 |
| P0 | `src/capability/platform/pluggable.ts:53-100` | `deactivate` 同步、`onDeactivate` 未等待，rollback best effort | 严格异步 close、取消和回滚；保持 ACTIVE→SUSPENDED→DISABLED 的资源一致性。 |
| P0 | `src/capability/platform/hotplug.ts:100-132,172-233` | 可选权限、重复加载机会、`disableCapability` fire-and-forget、force 策略 | 统一必经授权接口/审计；将 register/loader/contribution 变为单次幂等事务；故障/超时回滚测试；强依赖 force 需显式审批而非仅布尔。 |
| P0 | `src/capability/platform/orchestrator.ts:54-133` | 独立 `states` map、`addContributions` 又调用 loader 并丢弃 Promise，add 不传 permit | 禁止第二状态真源；确保 runtime & platform 状态对齐；异步 loader 仅一次；强制 grant 决策由 Host 注入并 fail closed。 |
| P0 | `src/capability/resourceGovernor.ts:31-63` 和 `src/capability/types.ts:60-70` | `destroy` 实际只有 `disable`；声明无 onDestroy | 明确 `close/destroy/unregister` 的真实语义与释放责任；验收进程 PID、PTY、WebView、网络任务、缓存、连接和 listener。 |
| P0 | `src-tauri/permissions/default-commands.toml`、`src-tauri/capabilities/default.json` | main 窗口允许大量命令；细粒度授权需在命令路径证明 | 做能力×来源×操作的授权矩阵，不自动把 '-' 改放行；高危操作配置交互确认、意图 token、重放防护、审计记录和拒绝负例。 |
| P1 | `docs/architecture/native-boundary/native-commands.yaml` | 178 项；100 项 file 仍写 bridge.rs；`reveal_path`/`move_path` 的 file 已失效 | 逐命令核对真实定义、业务 Owner、ACL、权限标签、责任边界；把物理拆分成能力本地 commands.rs 和中性共享基础层。 |
| P1 | `src-tauri/src/bridge.rs` 与 `src-tauri/src/capabilities/*` | Native 命令正从 bridge 分离，仍有集中定义 | 保留 bridge 为稳定薄适配层，真实业务过程迁到 Owner；不为拆文件引入双份状态、重复安全检查。 |
| P1 | `scripts/check-capability-boundaries.mjs:27-150` | 只相对 import、共享目录直接放行 | 用 TS compiler/AST 解析 alias、re-export、dynamic import、type import；让 Shell/Shared 的跨模块依赖同样受正式 Public API 限制。 |
| P1 | `scripts/check-native-capability-boundaries.mjs:74-134,184-274` | 基于 regex 扫描 Rust command 和 `bridge.camelCmd(...)`，不是完整调用图/权限执行流 | 明确检查器的有效边界，加入 Rust AST/调用图或精确静态入口，引用审批表；防漏与误报正/负例。 |
| P1 | `src/capabilities/agent/public.ts:1-9`、`agent/manifest.ts:1-10` | 仍出现“Skill 无独立 store/待拆”旧描述 | 改注释与契约声明为实际 Skill Owner；核对 Agent/Coding Agent/Skill 的运行协议是否共用 Host 接口，不合并业务 Owner。 |
| P1 | `docs/architecture/native-boundary/NATIVE_CAPABILITY_BOUNDARY_AUDIT.md:9-10,100` | 148、bridge 集中两项已经过期 | 从 registry/真实 Rust 自动生成统计及对照表，注明 SHA，不再手工维持过时总数。 |
| P1 | `docs/architecture/semantic-registry/owners.yaml:583,995-1014` | Home/Workspace `recents` 同名；Workspace 标注 observed/跨域写 | 明确 canonical writer、派生只读镜像与受批准窄接口，新增跨域写负例和时间界限。 |
| P1 | `docs/architecture/semantic-registry/side-effects.yaml` | 40 项，36 项 `requires_declaration=false`；awareness marker 非安全确认 | 新增副作用分级和真正执行流证据：调用来源、Owner、permission、confirmation、undo/retry、审计、拒绝用例；对需确认者强制负例。 |
| P2 | `packages/capability-{clipboard,vault}/package.json` 与其他 `src/capabilities/*` | 目前只看到少量 package 级独立 `check` | 每模块同构 `check:<module>` / `test:<module>`；要能单独运行，无 UI/网络/密钥库真实副作用。 |
| P2 | `scripts/verify-installed-client.sh`、`.github/workflows/capability-v2-validation.yml` | Xvfb 自动打包/启动/5 分钟采样存在 | 增补人工 GNOME 系统验收表，验证安装路径 SHA、WebView、Grid、PTY、文件保存、Monaco、Agent、Git、DB、退出释放，失败回滚。 |
| P2 | `docs/engineering/FRONTEND-SIZE-DUAL-BASELINE-20261031.md` 和性能脚本 | 已有包体积比较，缺配对运行时基线 | 使用相同 Ubuntu/硬件/Node/Rust/lockfile/数据集测重构前后 5/30 分钟；保留原始 artifact 和审查阈值。 |

## 3. 关键证据补充

### 3.1 Native Command 不可混淆的三种“完成”

- **业务 Owner 登记**：本轮解析 `native-commands.yaml`，**178/178** 个命令有非空 Owner；未发现 `UNKNOWN` 或 `LEGACY_UNOWNED`。这是登记层事实。
- **物理代码归属**：同表 **100/178** 的 `file` 仍是 `src-tauri/src/bridge.rs`，不能宣称“所有 Native 命令已各自封装到 Capability”。
- **执行授权与副作用安全**：同表 **58/178** 的 `permission` 为 `-`；不是 58 个已经被证明不安全，而是登记层尚未表达明确的授权标签。每个高风险命令必须额外用实际 Rust guard/ACL/确认流/拒绝负例证明。

静态不一致实例：`reveal_path` 与 `move_path` 的 `file` 指向 `src-tauri/src/fs_cmds.rs`，而该路径在本次 master tree 中不存在；需核对对应命令是否已经搬到 `bridge.rs` 或 `capabilities/workspace/fs_cmds.rs`。门禁 `scripts/check-native-capability-boundaries.mjs:184-205` 主要对命令名/Owner 双向校验，不能代替路径准确性校验。

### 3.2 关键副作用的验收口径

`docs/architecture/semantic-registry/side-effects.yaml` 有 **40** 条 `requires_declaration`（4 true、36 false）。其说明的“side-effect”标记属于调用者知情，不等价权限许可或用户确认。应区分：
- 仅读或低风险内部改变：可用静态范围 ACL、无需每次弹确认；
- 可逆写入/资源创建：作用域权限 + 审计 + 失败回滚；
- 高风险/不可逆/外部影响：明确用户确认、preview、token、过期、二次提交与负例测试；
- 远程 WebView：证明远端注入调用严格收口到 `remote-collect` 范围，不得继承 main 全量命令。

### 3.3 “资源拥有”必须是完成过的生命周期，不是 metadata

对每种资源登记以下 8 项：**Owner → birth → active → idle/hibernate → cancel → close/destroy → crash/restart → observation/test**。
重点：Browser tab/WebView、Grid child 及进程组、PTY、Agent SSE/工具任务/MCP、Monaco model/worker、LSP session、DB query/pool、Watcher、定时任务、secret/keyring。对“资源释放”的验收应读真实 PID/句柄/连接/worker 数；不能把贡献槽数和资源零泄漏等价。

### 3.4 模块独立测试覆盖不可假定

- `npm run check` 是多项门禁串接（`package.json`）；不是模块独立运行证明。
- Vault / Clipboard 各自 package.json 提供 `check`，值得作为其他模块模板。
- `scripts/check-hot-plug-acceptance.mjs` 使用真实模块定义，但 `snapshot` 的 command/listener/timer 等计数使用固定 0（约 269-280 行）；该 harness 主要验证贡献槽与声明的 lifecycle 行为，不能用这些零值证明进程无监听器或定时器。
- `scripts/check-pluggable-lifecycle.mjs` 是 isolated fixture 验证，不等于全部能力的真实资源测试。

### 3.5 GUI 与 CI 的不同验收层

`.github/workflows/capability-v2-validation.yml:195-198,253-267` 的 Ubuntu runner 使用 `dbus-run-session` + `xvfb-run` 真实打包并跑回归；`scripts/verify-installed-client.sh:182-231,273-329` 验证 deb 安装与二进制匹配、Vue mount/first paint、5 分钟采样。这是有价值的自动安装证据，但**真实用户桌面**还需要显示管理器、窗口切换、IME、多个 WebView、交互链路、文件/DB/Git/Agent 操作和关闭后资源回收确认。

`docs/architecture/capability-platform/FINAL-HUMAN-ACCEPTANCE.md:3-5` 仍明确 `HUMAN_VISUAL = PENDING`。不得把 CI 模拟显示环境作为人工完成证据。

### 3.6 性能基线不能把不同版本和不同口径混用

历史可用数据：
- `docs/engineering/footprint-baselines/master-7ae6fa86.json`（2026-10-11 / run 38098492736）：`frontend_dist_bytes=995732`，`release_binary_bytes=11827168`，`deb_file_bytes=5161532`。
- `logs/m0-build-metrics/build-metrics-052b18a.json`：历史 Vite `dist.total_bytes=795517`，但这是早期版本，不能当作已包含 Monaco 后的同口径 baseline。
- `docs/engineering/FRONTEND-SIZE-DUAL-BASELINE-20261031.md`：同构建条件下 `master@81a9feca` 的 dist 为 12,975,246 B，`perf@8090f000` 为 12,976,614 B，差 +1,368 B（+0.01054%）。这个数**只说明当时这两点之间的产物字节差**，不是当前 master 的真实 CPU/RSS/冷启动性能结论。
- `.github/workflows/browseros-sustained-performance.yml` 支持 5/30 min 自动采样，但本次未取得同一硬件、同一操作脚本、重构前后两组完整运行指标。

建议新 baseline artifact schema 记录 `before_sha/after_sha`、OS/桌面/内核/CPU/RAM、依赖锁文件哈希、测试场景 seed、启动 p50/p95/p99、Idle/10 Tab/4 Grid/Monaco/Agent/DB 场景 CPU/RSS/PSS、WebView/child PID、峰值和恢复、日志与截图。提前批准合理回归阈值；所有未测用 `NOT_MEASURED`，不能写 0。

## 4. 推荐验收次序与不可绕过的门禁

### Phase 0：先把 P0 契约补牢（代码阶段，**本次未实施**）
1. 统一异步 lifecycle 和 bootstrap 的状态转换，在异常、重入、取消、force/remove、进程退出上验证 fail-close。
2. Hot-Plug 单次异步 loader、注册与贡献原子性，强制授权和明确业务 Owner 回滚。
3. 对所有高影响 Native/副作用入口补真实权限—确认—审计—反例矩阵，先锁定高风险的文件改写/删除、命令运行、Git 写入、Agent apply、数据库操作、脚本、插件安装。

### Phase 1：稳定边界与可维护性（代码阶段，**本次未实施**）
1. 建立唯一 Owner schema 和可检查的 `public.ts` 公共契约；跨模块只通过 public exports/Host Services。
2. 扩充 TS/Rust 全仓模块图；alias、re-export、动态 import、shared/core 等全部可追踪，例外必须带 Owner、理由、有效期和审批人。
3. 逐个检查 Native 178 个命令的定义位置和 handler/ACL/bridge 对齐；把剩余桥中业务实现逐 Owner 迁出，留薄 facade。
4. 对每类资源补独立生命周期和负例测试；保留 Runtime 只编排、不接管业务真源的原则。

### Phase 2：证明已经可以发布（验收阶段，**本次未实施**）
1. 每个模块独立执行 unit / contract / lifecycle / permission negative tests，生成逐模块可追踪报告。
2. Push 与 PR 两套流水线的 Rust fmt/check/test、前端预算、Full Validation、安装包 cold-start 与全部安全门禁都需达到 PASS，不以单个快门禁替代。
3. Ubuntu GNOME 已安装 `.deb` 人工 GUI 验收表逐条签字；确认实际测试 binary SHA 与 PR head 绑定。
4. 同机同构建口径运行 before/after 5/30min 基准并上传原始证据。
5. 文档覆盖面/注册表/代码 locator/对外 API 差异检查全绿，再合并 master。

### 十项最终“完成”断言（DoD）

- [ ] `OWNER_UNIQUE=PASS`：所有业务状态有且只有一名真实 writer owner，例外有审批，复写/影子状态为 0。
- [ ] `PUBLIC_API_ONLY=PASS`：所有新增跨模块调用由正式 Public API/Host Service 提供，稳定性测试和 alias 反例通过。
- [ ] `NATIVE_OWNED=PASS`：每条 Rust 命令均有有效 Owner、准确源码位置、handler/ACL/bridge 对照与副作用等级，未知项和漂移均为 0。
- [ ] `RESOURCE_LIFECYCLE=PASS`：全部重要资源 birth/reuse/cancel/destroy/restart 覆盖，异常/重复调用不残留进程、worker、连接、监听器。
- [ ] `NO_UNAPPROVED_INTERNAL_REFS=PASS`：全仓 TS/Rust 越界引用扫描及负例通过，没有未经授权依赖。
- [ ] `SIDE_EFFECT_AUTH=PASS`：所有高风险写/进程/网络/密钥/外部调用均验证作用域权限、确认与审计，拒绝/过期/取消/重放覆盖。
- [ ] `MODULE_TESTS_ISOLATED=PASS`：每个模块可单独执行不依赖其它模块真实副作用的最小验证。
- [ ] `INSTALLED_GUI=PASS`：CI 安装包验收 + Ubuntu GNOME 用户手动验收双通过，并有相同 SHA 证据。
- [ ] `PERFORMANCE_BASELINE=PASS`：同机 before/after 5/30min 完整基准、可复现脚本和原始证据，未测不写 0，结论需有差异/置信条件。
- [ ] `DOC_CODE_PARITY=PASS`：文档登记数、API、路径、状态与实际源码一致，漂移检查为 0。

## 5. 此次 Github CI 快照（仅反映 master 被检视时的运行）

本轮 GitHub Connector 查看最新主干工作流时，`master@9607e260` 的 Engineering Governance、Developer Feedback、UI Safety、Hot-Plug Acceptance、Supply Chain Assurance 有成功运行记录；但 Full Validation 运行 `41284054999` 的 `native-regression` 子任务在 `cargo fmt --check` 失败（涉及 Grid process / main.rs 格式），后续 Rust check/test 不可据此宣布通过。该时刻工作流仍有运行中任务，**此信息不是最终最新状态承诺**。应以此 SHA 对应的最后一轮工作流完整结果复核。已通过的独立 workflow 不会抵消 Full Validation 子任务失败。

- GitHub CI：<https://github.com/code5m/browser-os/actions/runs/41284054999>
- 主干 Commit：<https://github.com/code5m/browser-os/commit/9607e260f86b15a730219c619ab5a20cc01efe65>
- 本调研分支：<https://github.com/code5m/browser-os/tree/research/architecture-reconstruction-audit-20261101>

## 6. 立即复核清单（禁止把计划写成 PASS）

**本轮已经完成**：确认仓库和 master 精确 SHA；创建独立 research 分支；只读检视 Capability/Owner/Native/SideEffect/Resource/Tests/GUI/性能治理源码和文档；定位上述具体文件与可疑调用链；将缺口变成可执行 DoD。

**本轮未执行**：代码改动、源码测试、独立模块完整执行、真实 GUI、性能实测、代码修复、PR 提交/合并。本报告为审计初版，文件链接与行号以固定审计 SHA 为准；后续补充运行数据时必须注明 `verified_sha`，不能用另一个 SHA 的 PASS 覆盖当前结论。
