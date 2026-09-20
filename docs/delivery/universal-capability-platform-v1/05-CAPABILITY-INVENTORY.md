# 05 — Capability Inventory（真实代码扫描结果）

扫描对象：`src/**`（不依据旧文档猜测）。分类口径见 §7。`UNCLASSIFIED = 0`。

## A. 已是「积木」（具备 Building Block Contract v1，在 catalog 中）

| 能力 | 分类 | 成熟度 | Hot-Plug | 证据 |
|---|---|---|---|---|
| `workspace` | CAPABILITY | C3 | HP0（未验证贡献整体摘除） | `check-capability-platform.mjs`、`check-capability-pilot.mjs` |
| `bookmark` | CAPABILITY | C3 | **HP2** | 运行时 register/unregister 真实生效（贡献 3→0→3） |
| `browser` | CAPABILITY | C3 | HP0（WebView 生命周期未治理） | 既有 C3 门禁 + 缺席零贡献 |
| `terminal` | CAPABILITY | C3 | HP0（PTY/会话释放未验证） | 既有 C3 门禁 + 缺席零贡献 |

### workspace 内部（SUB_CAPABILITY / DOMAIN，均有独立 owner store）
`files`（useFileStore）· `artifact`（useArtifactStore）· `repo`（useRepoStore）·
`script`（useScriptStore）· `snippet`（useSnippetStore）· `audit`（主视图）· `editor`（FileEditor 主视图）

### browser 内部（SUB_CAPABILITY / DOMAIN）
`grid`（与 Browser 同 store 的原生重资源面）、`session`（会话恢复）、`resource/net`（网络资源瀑布）、`grid-archive`（useGridArchiveStore）

## B. 有独立 owner 但**还不是积木**（未进 catalog，Shell 直接引用或散落全局）

| 候选 | 分类 | 现状 | 为什么还不是积木 |
|---|---|---|---|
| `git` | CAPABILITY 候选 | C1 | 已有独立 owner（Train E 物理分离），但未进入 catalog / 无 manifest |
| `database` | CAPABILITY 候选 | C1 | 同上；凭据经 keyring 引用，面板仍由 MainArea 直连懒加载 |
| `task` | CAPABILITY 候选 | C0 | 面板（`TaskPanel`）由 Shell 直连懒加载，无 manifest |
| `graph` | CAPABILITY 候选 | C0 | `useGraphStore` + `GraphPanel`，无 manifest |
| `plugin` | CAPABILITY 候选 | C0 | `usePluginStore` + `PluginManager`，无 manifest |
| `agent` | CAPABILITY 候选 | C0 | `useAgentStore`，且存在 agent/skill 命令契约遗留 drift（既有 known drift） |
| `skill` | CAPABILITY 候选 | C0 | 同上（skill_* 命令在 known drift 列表中） |
| `clipboard` | CAPABILITY 候选 | C0 | `ClipboardPanel` 由 Shell 直连 import |
| `apps` | CAPABILITY 候选 | C0 | `AppPanel` 由 Shell 直连 import |
| `tools`（ToolBox） | CAPABILITY 候选 | C0 | 由 Shell 直连 import |
| `notes` | NOT_FOUND | — | 前端无 owner store；仅残留 backend 命令（`save_note`，已在 known drift 列表）→ 计为 IMPLEMENTATION_DETAIL 残留 |

## C. Framework Core / 基础设施 / 实现细节

| 模块 | 分类 | 说明 |
|---|---|---|
| `src/capability/**`（runtime / contribution / platform） | FRAMEWORK_CORE | 底座：Runtime + Registry + Contract + Resolver + Assembly + Lifecycle |
| `useLayoutStore` + `src/components/layout/**` | FRAMEWORK_CORE | Shell 主壳（ActivityBar / StatusBar / UnifiedTabBar / MainArea / WorkbenchRail） |
| `useSettingsStore` | FRAMEWORK_CORE | 全局设置（跨能力，属核心配置而非独立积木） |
| `bridge.ts` + Tauri Rust 命令层 | INFRASTRUCTURE | 原生适配层（NATIVE ADAPTER） |
| `useVaultStore` | SECURITY_INFRASTRUCTURE | 凭据保管；含用户面板但不应由 Shell 直连 |
| `useHomeStore` + `src/components/home/**` | SUB_CAPABILITY（框架表层） | 主页/快捷方式；当前由 Shell 直连，尚未契约化 |
| `useSessionStore` | DOMAIN（browser 子域） | 会话恢复，随 Browser 面板存在 |
| `useWorkbenchStore` | DOMAIN | 工作台 chrome |
| `useSystemStore` | DOMAIN | 系统信息（历史上承接过多个关注点，Train E 后已拆出 Terminal owner） |
| `useImagePreviewStore` | DOMAIN | 文件预览，随 workspace/files 域 |
| `src/composables/**` | 混合：INFRASTRUCTURE（`useModalFocus` a11y）+ 边界债（`useBrowserHost` 属 browser 能力却位于全局） | 见 KNOWN_DEBT |
| `src/utils/**`（`*Ui.ts` 等） | IMPLEMENTATION_DETAIL / 边界债 | 能力业务逻辑未随能力物理归位 |

## D. 结论

- **UNCLASSIFIED = 0**：所有候选均已分类，未分类项为 0。
- **分类完成 ≠ 全部 C3**：只有 4 个能力是真正的积木；其余为「候选 + 缺口清单」。
- 下一步迁移对象按「真实隔离收益 / 风险」排序：`git` → `database` → `task/graph/plugin/skill/agent` → `home/clipboard/apps/tools`。
