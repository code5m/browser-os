# Git Capability — Full-Stack Boundary (Capability Library Expansion v1)

> 切片：Git（从 `components/workspace/` 内嵌面板升格为独立 Capability）。
> 收口评级：**C2 ISOLATED**（Building Block Contract `v1.maturity = "C2"`）。
> 状态：`COMPATIBILITY_WRAPPED`（manifest.status）；组合态 `profiles.yaml: git = COMPATIBILITY_WRAPPED`。
> Hot-plug：`HP0 STATIC`（manifest.v1.hotPlug.level = "HP0"，installPolicy = "static"）。

## 1. 真实调用链（UI → Native）

```
GitPanel.vue / GitDiffViewer.vue / GitWriteConfirmDialog.vue   [capabilities/git/ui/]
        │ 点击/交互
        ▼
useGitStore (Pinia, id="git")                                   [src/stores/useGitStore.ts]
        │ 只读: loadStatus/loadBranches/loadDiff
        │ 写  : requestWrite → confirmWrite（两阶段闸门，fail-closed）
        ▼
bridge.gitStatus / gitDiff / gitBranchList / requestGitWrite /
confirmGitWrite / gitLog / gitCommitDiff                       [src/bridge.ts]
        │ Tauri invoke
        ▼
Rust tauri::command  git_status / git_diff / git_branch_list /
request_git_write / confirm_git_write / git_log / git_commit_diff
                                                                 [src-tauri/src/bridge.rs, 注册于 main.rs]
        │ spawn git 子进程 / 读磁盘仓库
        ▼
git CLI + 本地仓库（用户数据，非应用托管）
```

## 2. 分层归属标记

| 层 | 文件 | 归属 | 说明 |
|---|---|---|---|
| UI | `capabilities/git/ui/{GitPanel,GitDiffViewer,GitWriteConfirmDialog}.vue` | OWNED_BY_CAPABILITY | 物理已迁入能力目录 |
| Public Contract | `capabilities/git/public.ts` | OWNED_BY_CAPABILITY | 再导出 `useGitStore` + `GitWriteConfirmDialog`；禁第二真源 |
| Contribution | `registerGitContributions` → `REPO_SUBVIEW` 槽 | SHARED_INFRASTRUCTURE (宿主=RepoPanel) | 通用 Contribution Registry；Shell 不 import 内部 |
| Manifest | `capabilities/git/manifest.ts` | OWNED_BY_CAPABILITY | 纯数据 + v1 Building Block Contract |
| Entry/Index | `capabilities/git/index.ts` | OWNED_BY_CAPABILITY | `gitCapability` + `onActivate=registerGitContributions` |
| State Owner | `useGitStore` (id="git") | OWNED_BY_CAPABILITY（语义） / LEGACY_COUPLING（物理） | 语义 owner 已声明；**物理仍在 `src/stores/`，未迁入 `capabilities/git/state/`** |
| Intent | `useGitStore.stage/unstage/discard/commit/createBranch/checkoutBranch/push` | OWNED_BY_CAPABILITY | 仅组装参数，闸门语义一致 |
| Application Logic | `useGitStore` 加载/两阶段写闸门 | OWNED_BY_CAPABILITY | 含前端预校验 + dangerous 二次确认本地锁 |
| Adapter | `src/bridge.ts` git_* 封装 | SHARED_INFRASTRUCTURE / NATIVE_ADAPTER | 经 Tauri invoke 到 Rust |
| Native | `src-tauri/src/bridge.rs` git_* commands | NATIVE_ADAPTER | 经 git CLI / 文件系统 |
| Credential | 推送瞬间后端从系统密钥库读取 | CREDENTIAL BOUNDARY | 前端零凭据、零持久化；manifest.permissions 含 `keyring.access` |
| Filesystem | 用户本地 git 仓库 | USER DATA（非应用托管） | 不属能力资源所有权 |
| Side Effect | spawn git 子进程（写操作） | OWNED_BY_CAPABILITY（瞬态） | 非长驻重资源 |
| Resource | 写任务门（`request_git_write` 5 分钟过期） | OWNED_BY_CAPABILITY（瞬态） | 无长驻 PTY/连接/WebView |

## 3. SECOND_TRUTHS 审计

- Git 状态唯一真源 = `useGitStore`（manifest.semanticOwner = "useGitStore"）。
- 未发现重复 Git 状态副本；`App.vue` / `RepoPanel` / `GitHistory` 均只消费 `useGitStore`，不持有 Git 状态镜像。
- **SECOND_TRUTHS = 0**（Git 能力内）。

## 4. Absence Behavior（C3 证据基线）

机制：`bootstrapCapabilityRuntime` (`src/capability/index.ts:93-98`) 仅对「解析后 profile 的 `allowed` 集合内」的能力执行 `register→resolve→activate`；不在集合内的能力直接 `continue` 跳过。

- `minimal` profile（`profiles.yaml:35`）**不含 git** → git 不注册 → `onActivate` 不触发 → `registerGitContributions` 不运行 → `REPO_SUBVIEW` 槽无贡献 → `RepoPanel` 的 `<component :is="gitPanelComp">` 因 `gitPanelComp` 为 `undefined` 而**不渲染 GitPanel**。
- App 启动不崩溃；其它能力（workspace/bookmark…）不受影响。
- **确定性成立（代码路径保证）**。但当前 gate 套件**尚无专门的 git-absence 运行时断言**（仅 `COMPOSITION_PROFILES` 覆盖 framework profile 下 Dock 贡献=0）。

## 5. 已知债务（诚实不谎报）

1. **LEGACY_SHELL_COUPLING（阻断 C3）**：`src/App.vue` 仍静态 `import { GitWriteConfirmDialog } from "./capabilities/git/public"`，并在模板渲染该全局写闸门。Shell 直接消费能力公共面——虽经 `public.ts`（合规边界），但非「经通用 contribution/dialog 宿主」渲染，构成残余直连。C3 需改为经通用对话框宿主（如 Contribution Registry 的 GLOBAL_DIALOG 槽）挂载。
2. **STATE PHYSICAL LOCATION**：`useGitStore` 仍在 `src/stores/`，未迁入 `capabilities/git/state/`。语义 owner 正确，物理封装不完整——属 C2 允许范围，但应在后续切片中物理解耦。
3. **ABSENCE GATE 缺口**：缺 git 专属 absence 运行时断言。
4. **HP0 STATIC**：本切片为静态装配（profile 决定），无运行时 enable/disable（C4）或 install/uninstall（HP2/HP3）。不谎报。

## 6. 评级结论

- 成熟度：**C2 ISOLATED** —— 实现已通过 `public.ts` / `manifest` / `index` / `ui` 明确边界隔离，状态 owner 单一且无第二真源。
- 非 C3：因存在 Shell 静态直连（债务 1）与无运行时 absent 门禁（债务 3）。
- 非 C4/C5：git 无长驻重资源，写操作瞬态 spawn + 任务门过期；无需资源释放治理，故不评 C5（C5 仅适用于 PTY/DB 连接/WebView 等长驻资源）。
- 组合态：`COMPATIBILITY_WRAPPED`（Adapter 接入 Runtime，底层仍是既有 git CLI 系统）。

## 7. 依赖（manifest.dependsOn）

`["credential", "workspace", "bridge"]` —— 已声明。其中：
- `workspace`：GitPanel 经 `REPO_SUBVIEW` 槽贡献进 `RepoPanel`，并 `import { useRepoStore } from "../state/useRepoStore"`（git→workspace 跨能力 public 依赖，已登记入 `ui04b_cross_capability_public_baseline`）。
- `credential`：仅后端推送瞬间读密钥库，前端无耦合。
- `bridge`：git 命令经 Tauri bridge 下发。
