// Clipboard 公共契约（Public Contract）—— 能力对外唯一稳定面。
// 仅再导出语义 owner（useClipboardStore），不创建镜像状态（SECOND_TRUTHS=0）。
export { useClipboardStore } from "./state/useClipboardStore"
export { clipboardManifest } from "./manifest"
export type { ClipItem } from "./state/useClipboardStore"
