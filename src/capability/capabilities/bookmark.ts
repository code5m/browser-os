// Bookmark Capability — 试点适配器（Phase 7D）
//
// 兼容优先（Compatibility First）：
//   适配器 = 「声明 manifest + 提供 lifecycle 钩子」，**不接管业务状态真源**。
//   业务 Owner 仍然是 useBookmarkStore（Semantic Registry 已登记）。
//
// 硬约束：本文件**不得** import useBookmarkStore，也不得读写其任何 state。
//         违反即等于 Runtime/Adapter 成为第二真源 → 语义治理回归。
//         由 scripts/check-capability-pilot.mjs 的 PLT-05 静态断言强制。

import type { CapabilityDefinition } from '../types'

export const BOOKMARK_CAPABILITY_ID = 'bookmark'

export const bookmarkCapability: CapabilityDefinition = {
  id: BOOKMARK_CAPABILITY_ID,
  name: 'Bookmark',
  category: 'CAPABILITY',

  provides: [
    'bookmark.list',
    'bookmark.add',
    'bookmark.remove',
    'bookmark.togglePanel',
  ],

  // 强依赖为空 —— 保证"不加载其它可选能力时也能工作"
  dependsOn: [],
  optionalDependencies: ['bridge'],

  lifecycle: {
    supported: ['ACTIVE', 'SUSPENDED'],
    default: 'ACTIVE',
    activatable: true,
    resident: false,
    // 钩子只做编排侧日志，绝不读写业务状态
    onActivate: () => {
      /* no-op by design: 业务状态由 useBookmarkStore 自己持有 */
    },
    onSuspend: () => {
      /* no-op by design */
    },
  },

  resources: { class: ['LIGHT'], suspendable: true, destroyable: true },

  permissions: [],

  persistence: { scope: 'disk', sensitive: false },

  entrypoint: 'src/stores/useBookmarkStore.ts',

  // 语义 Owner 是「引用」Semantic Registry，不是在此重新声明语义
  semanticOwner: 'useBookmarkStore',
  governanceStatus: 'GOVERNED',
  status: 'COMPATIBILITY_WRAPPED',
}
