# Apps Capability — Full-Stack Boundary（Capability Library Expansion v1, STAGE H）

> STAGE H 产物。物理：AppPanel 迁入 `src/capabilities/apps/ui/`；语义 owner **新建** `useAppsStore`
> （`src/capabilities/apps/state/`），由 `useSystemStore` 拆分而来（解除 **Debt-8E-1**）。
> 经通用 Contribution Registry 的 `WORKBENCH_MAIN` 槽（view='apps'）贡献给 MainArea。最后更新：2026-09-23。

## 成熟度（诚实，不谎报）

**C2 ISOLATED**（status=`COMPATIBILITY_WRAPPED`，manifest.v1.maturity=`C2`；governanceStatus=`GOVERNED`）。

- C2 达成：五段边界 + owner `useAppsStore` 唯一 + 贡献驱动。
- 非 C3：无 absence 运行时门禁；`mainView='apps'` nav 硬编码。

## 十七段契约

```text
Apps UI (capabilities/apps/ui/AppPanel.vue)
  ↓ OWNED_BY_CAPABILITY（经 public.ts）
State Owner: useAppsStore (id="apps")
  ↓ intents：loadApps / launchApp / onAppImgError（computed filteredApps）
  ↓ PUBLIC_DEPENDENCY（bridge）
Adapter: src/bridge.ts → listApps / launchApp
  ↓ NATIVE_ADAPTER（Rust）
src-tauri/src/bridge.rs: list_apps（扫描 .desktop）/ launch_app（security_policy::check_launch_target 白名单式解析，禁 sh -c；写审计；Command::spawn）
```

| 段 | 事实 |
|---|---|
| Identity/Manifest | `capabilities/apps/manifest.ts`（id=apps，C2，HP0） |
| Public Contract | `public.ts`（仅再导出 `useAppsStore`） |
| Dependencies | `bridge` |
| State Owner | `useAppsStore`（唯一） |
| Canonical Writers | 仅 store action 写 `apps`/`appFilter`/`brokenIcons` |
| Application Logic | 内联于 store（过滤/图标错误降级） |
| UI | `capabilities/apps/ui/AppPanel.vue`（懒加载，贡献注册） |
| Contributions | `apps.main.panel`（workbench-main, view='apps'） |
| Side Effects | 经 `bridge.listApps/launchApp`；UI 零裸 invoke |
| Adapter/Native | `bridge.ts` → Rust `list_apps`/`launch_app` |
| Permissions | `process.spawn`（launch_app） |
| Persistence | `runtime_only`（无前端持久化；主页 app 快捷方式禁落盘，`isStorageSafe`） |
| Resource Ownership | `v1.resources=[{CHILD_PROCESS, owned}]`（launch_app spawn，detached） |
| Lifecycle | ACTIVE/SUSPENDED |
| Absence | 见下 |
| Tests/Gates | `check-home-client-policy.py` / `check-home-ui-logic.mjs` + `npm run check` |

## 安全

- **启动命令白名单式解析**：`security_policy::check_launch_target` 解析为 (program,args)，**禁 `sh -c`**；每次启动写审计。
- **零能力隔离**：应用列表/启动不授予能力；`.desktop` 仅读。
- **主页快捷方式**：app 命令体（可能含参数/令牌/绝对路径）**禁止落浏览器存储**（`HOME_PERSISTED_TYPES=["url","dir"]`、`isStorageSafe` 排除 `type:"app"`，门禁 `HOME_NO_SECRET_PERSIST`）。
- 图标经 `convertFileSrc` 转 asset://（不成外链）。

## Absence（实测）

```text
framework/minimal/developer: apps=false
full: apps=true（总能力 14）
```
absent → 无 view='apps' → 不渲染 → `useAppsStore` 不实例化 → 零 `list_apps/launch_app` invoke。
**诚实边界**：已启动的 detached 外部应用进程不由本能力管理其生命周期。

## SECOND_TRUTHS = 0 / RESOURCE_LEAKS = 0
`public.ts` 仅再导出语义 owner。新增前端资源泄漏 = 0（未启动应用即无子进程）。
