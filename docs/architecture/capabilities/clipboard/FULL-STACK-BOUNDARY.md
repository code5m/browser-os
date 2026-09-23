# Clipboard Capability — Full-Stack Boundary（Capability Library Expansion v1, STAGE H）

> STAGE H 产物。物理：ClipboardPanel 迁入 `src/capabilities/clipboard/ui/`；语义 owner **新建**
> `useClipboardStore`（`src/capabilities/clipboard/state/`），由 `useSystemStore` 拆分而来（解除 **Debt-8E-1**）。
> 经通用 Contribution Registry 的 `WORKBENCH_MAIN` 槽（view='clip'）贡献给 MainArea。最后更新：2026-09-23。

## 成熟度（诚实，不谎报）

**C2 ISOLATED**（status=`COMPATIBILITY_WRAPPED`，manifest.v1.maturity=`C2`；governanceStatus=`GOVERNED`）。

- C2 达成：五段边界（manifest/public/index/state/ui）+ 语义 owner `useClipboardStore` 唯一 + MainArea 贡献驱动。
- 非 C3：无 absence 运行时门禁；`mainView='clip'` nav 硬编码；面板开合态 `clipOpen` 仍归 `useLayoutStore`（视图态）。

## 十七段契约

```text
Clipboard UI (capabilities/clipboard/ui/ClipboardPanel.vue)
  ↓ OWNED_BY_CAPABILITY（经 public.ts）
State Owner: useClipboardStore (id="clipboard")
  ↓ intents：clipReadSilent / clipCopy / clipPaste / useClipItem / copyClipItem / clearClipHistory / startClipWatch
  ↓ PUBLIC_DEPENDENCY（bridge）
Adapter: src/bridge.ts → clipboardRead / clipboardWrite
  ↓ NATIVE_ADAPTER（Rust）
src-tauri/src/bridge.rs: clipboard_read / clipboard_write（arboard）—— 无资源创建
```

| 段 | 事实 |
|---|---|
| Identity/Manifest | `capabilities/clipboard/manifest.ts`（id=clipboard，C2，HP0） |
| Public Contract | `public.ts`（仅再导出 `useClipboardStore`） |
| Dependencies | `bridge`（外部基础设施） |
| State Owner | `useClipboardStore`（唯一） |
| Canonical Writers | 仅 store action 写 `clipText`/`clipHistory`；组件只读经 public |
| Application Logic | 内联于 store（事件驱动、去重、上限裁剪） |
| UI | `capabilities/clipboard/ui/ClipboardPanel.vue`（懒加载，贡献注册） |
| Contributions | `clipboard.main.panel`（workbench-main, view='clip'） |
| Side Effects | 经 `bridge.clipboard*`；UI 零裸 invoke |
| Adapter/Native | `bridge.ts` → Rust `clipboard_read/write` |
| Permissions | 无 |
| **Persistence** | **`session`（仅内存会话态，绝不落盘）**——B11-1 红线；前端零浏览器存储 |
| Resource Ownership | `v1.resources=[]`（无资源创建） |
| Lifecycle | ACTIVE/SUSPENDED（无可释放资源） |
| Absence | 见下 |
| Tests/Gates | `check-clipboard-persistence-logic.mjs`（16 断言）+ `npm run check` |

## 安全（B11-1）

- 剪贴板历史**不落盘**：`loadClipHistory` 为无副作用 no-op；`saveClipHistory` 仅内存裁剪 `CLIP_CAP=30`；**绝不** `localStorage.setItem`。
- 渲染脱敏：`ClipboardPanel.vue` 经 `redactSecrets(item.text)`（URL userinfo / 凭据 query / token 前缀 / Authorization），凭据原文不入渲染。
- `intents.yaml` 中 `copyPassword` 为 **REJECTED**（禁止把密码写入剪贴板）。

## Absence（实测）

```text
framework/minimal/developer: clip=false
full: clip=true（总能力 14）
```
absent → `registerClipboardContributions` 不运行 → 无 view='clip' → 不渲染 → `useClipboardStore` 不实例化 → 零 `clipboard_read/write` invoke。

## Debt-8E-1 关闭

`src/stores/useSystemStore.ts` 已删除（Clipboard/Apps 拆分）；`dependencies.yaml` Debt-8E-1 → `status: CLOSED`。
`states.yaml` owner_implementations 由 `useSystemStore` 改为 `useClipboardStore` / `useAppsStore` / `useToolsStore`。
`check-terminal-owners.mjs` TERM-02c 改指向三个继任 owner（避免静默少检查）。

## SECOND_TRUTHS = 0 / RESOURCE_LEAKS = 0
`public.ts` 仅再导出语义 owner。DOMAIN STATE ≠ COMPOSITION STATE ≠ UI LOCAL STATE ≠ RESOURCE RESULT。新增资源泄漏 = 0。
