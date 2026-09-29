# @browser-os/capability-clipboard

Clipboard 能力包 —— Frontend M2 真实 npm workspace package（TRUE Pilot，2026-09-28）。

## 边界（M2 PACKAGE_ISOLATED）

- **独立 package root**：`packages/capability-clipboard/`。
- **显式 exports**：仅 `.`（runtime API）与 `./manifest`（metadata API）。runtime API ≠ metadata API。
- **Host 通过 public contract 消费**：`@browser-os/capability-clipboard` 与 `@browser-os/capability-clipboard/manifest`。
- **包不 import Host implementation**：`src/bridge.ts`、`src/stores/useLayoutStore`、`src/capability/contribution/registry`、`src/capability/*` implementation、`src/utils/*`、`src/shared/ui` 一律禁止。
- **旧 `src/capabilities/clipboard` 已删除**（PKG-10）。

## 端口（Host 注入，provide-inject）

- `CLIPBOARD_PORTS_KEY`（InjectionKey<ClipboardPorts>）
  - `native`: `clipboardRead()` / `clipboardWrite(text)` —— 替代直连 `bridge`。
  - `ui`: `showToast(msg)` / `requestClose()` / `redactSecrets(text)` / `EmptyState` —— 替代直连 `useLayoutStore` / `utils/redact` / `shared/ui`。

## 贡献注册

包只导出 `clipboardContribution` 描述子；Host 在 `src/capability/index.ts` 单点 `contributionRegistry.register(clipboardContribution as any)`。**包不自注册**。

## B11-1 红线（冻结）

剪贴板内容不落盘：`loadClipHistory` 不读 localStorage；`saveClipHistory` 仅内存上限裁剪（CLIP_CAP=30），不写任何持久化层。门禁：`scripts/check-clipboard-persistence-logic.mjs`。

## 校验

```
npm run check -w @browser-os/capability-clipboard
```

等价于 `tsc --noEmit` + `check-package.mjs`（PKG-01..PKG-12）+ `check-clipboard-logic.mjs` + `check-clipboard-persistence-logic.mjs`。
