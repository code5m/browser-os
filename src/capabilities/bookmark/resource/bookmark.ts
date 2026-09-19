import type { CapabilityResourcePolicy } from "../../../capability/types"

// Bookmark 资源策略（Phase 7E 口径：DECLARED 非实测）
// LIGHT：仅内存状态 + 异步后端文件读写，无 WebView / Process / PTY。
export const bookmarkResourcePolicy: CapabilityResourcePolicy = {
  class: ["LIGHT"],
  suspendable: true,
  destroyable: false,
}
