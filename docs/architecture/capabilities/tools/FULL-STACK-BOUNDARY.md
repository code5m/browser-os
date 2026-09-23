# Tools Capability — Full-Stack Boundary（Capability Library Expansion v1, STAGE H）

> STAGE H 产物。物理：ToolBox 迁入 `src/capabilities/tools/ui/`；语义 owner **新建** `useToolsStore`
> （`src/capabilities/tools/state/`）——原 ToolBox.vue 把状态放在**组件内**（无 owner），本 stage 收敛为可寻址 owner。
> 经通用 Contribution Registry 的 `WORKBENCH_MAIN` 槽（view='tools'）贡献给 MainArea。最后更新：2026-09-23。

## 成熟度（诚实，不谎报）

**C2 ISOLATED**（status=`COMPATIBILITY_WRAPPED`，manifest.v1.maturity=`C2`；governanceStatus=`GOVERNED`）。

- C2 达成：五段边界 + owner `useToolsStore` 唯一 + 贡献驱动。
- 非 C3：无 absence 运行时门禁；`mainView='tools'` nav 硬编码。

## 十七段契约

```text
Tools UI (capabilities/tools/ui/ToolBox.vue)
  ↓ OWNED_BY_CAPABILITY（经 public.ts）
State Owner: useToolsStore (id="tools")
  ↓ intents：load() / open(t)（computed builtin/user）
  ↓ PUBLIC_DEPENDENCY（bridge）
Adapter: src/bridge.ts → listTools / openTool
  ↓ NATIVE_ADAPTER（Rust）
src-tauri/src/tools.rs: list_tools（内置种子 + 扫描 workspace/tools/*.html，只读）/ open_tool（WebviewWindowBuilder 子 webview，label=tool-<id>，tool:// 协议）
```

| 段 | 事实 |
|---|---|
| Identity/Manifest | `capabilities/tools/manifest.ts`（id=tools，C2，HP0） |
| Public Contract | `public.ts`（仅再导出 `useToolsStore`） |
| Dependencies | `bridge` |
| State Owner | `useToolsStore`（唯一；原为组件内 ref） |
| Canonical Writers | 仅 store action 写 `tools`/`error` |
| Application Logic | 内联于 store（builtin/user 分组） |
| UI | `capabilities/tools/ui/ToolBox.vue`（懒加载，贡献注册） |
| Contributions | `tools.main.panel`（workbench-main, view='tools'） |
| Side Effects | 经 `bridge.listTools/openTool`；UI 零裸 invoke |
| Adapter/Native | `bridge.ts` → Rust `list_tools`/`open_tool` |
| Permissions | 无 |
| Persistence | `runtime_only`（无前端持久化） |
| Resource Ownership | `v1.resources=[{WEBVIEW, owned}]`（工具子 webview） |
| Lifecycle | ACTIVE/SUSPENDED |
| Absence | 见下 |
| Tests/Gates | `check-tools-policy.py` / `check-seed-tools.py`（Rust）+ `npm run check` |

## 安全（零能力隔离）

- 工具窗口 `label=tool-<id>` **不属任何 capability**（门禁 `TOOL_CAPABILITY_LEAK` / `SEED_CAPABILITY_LEAK` 强制）；
  工具 HTML 即便 `invoke()` 亦被 ACL 拒绝。
- **路径越权防御**：`validate_user_tool_path`（canonicalize + `starts_with`）；id 分隔符拒绝；`MAX_USER_TOOL_BYTES=2 MiB`。
- **种子离线**：零 `http(s)://` 外链、零 `invoke/__TAURI__` 写原语；错误页零外链、不泄露绝对路径。
- 不涉及凭据渲染/持久化。

## Absence（实测）

```text
framework/minimal/developer: tools=false
full: tools=true（总能力 14）
```
absent → 无 view='tools' → 不渲染 → `useToolsStore` 不实例化 → 零 `list_tools/open_tool` invoke → **无工具子 webview 创建**。

## SECOND_TRUTHS = 0 / RESOURCE_LEAKS = 0
`public.ts` 仅再导出语义 owner。新增前端资源泄漏 = 0（不打开工具即无子 webview）。
