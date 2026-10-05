// Capability Composition Profiles（Phase 8E / Train F）
//
// 真实注册/加载 profile（不是 UI hide）：bootstrap 只注册 profile 列出的能力。
// 真源在此（不硬编码进 bootstrap）；未列出的能力**不注册** → 其贡献槽为空 →
// 绝不加载其内部 store / 不创建其重资源（如 Terminal 的 PTY）。
//
// 分类（诚实）：
//   minimal   = bookmark + workspace            （无 browser / 无 terminal → 无 WebView / 无 PTY）
//   developer = bookmark + workspace + browser + terminal + git + database（开发向全家）
//   full      = 全部已登记（含 git/database/agent/skill/plugin）—— 与 CAPABILITY_CATALOG 对齐
//
// 注意：browser / terminal 当前为 OPTIONAL（C3）。minimal 不注册它们即证明「absent → 零重资源」
// 是可组合性的端到端证据，而非仅隐藏按钮。
//
// STAGE F 修正（Capability Library Expansion v1）：`full` 曾仅列 4 个能力，导致 STAGE C/D/E
// 已迁入 capabilities/* 的 database/agent/skill（及本次 plugin）**贡献在运行期未注册** →
// MainArea 的 viewOf('db'/'agents'/'skills'/'plugin') 返回 undefined → 面板不渲染（静默 UI 回归）。
// 现 `full` 吸纳全部已登记能力，恢复 UI（UI_PRESERVATION）。git/database/agent/skill/plugin 均为
// read-only/薄封装，激活只注册贡献，不创建重资源（RUNTIME_RESOURCE_ABSENCE 不受影响）。

import { BOOKMARK_CAPABILITY_ID } from '../capabilities/bookmark'
import { WORKSPACE_CAPABILITY_ID } from '../capabilities/workspace'
import { WORKBENCH_CAPABILITY_ID } from '../capabilities/workbench'
import { BROWSER_CAPABILITY_ID } from '../capabilities/browser'
import { SESSION_CAPABILITY_ID } from '../capabilities/session'
import { TERMINAL_CAPABILITY_ID } from '../capabilities/terminal'
import { GIT_CAPABILITY_ID } from '../capabilities/git'
import { DATABASE_CAPABILITY_ID } from '../capabilities/database'
import { CREDENTIAL_CAPABILITY_ID } from '../capabilities/credential'
import { AGENT_CAPABILITY_ID } from '../capabilities/agent'
import { SKILL_CAPABILITY_ID } from '../capabilities/skill'
import { PLUGIN_CAPABILITY_ID } from '../capabilities/plugin'
import { KNOWLEDGE_GRAPH_CAPABILITY_ID } from '../capabilities/graph'
import { TASK_CAPABILITY_ID } from '../capabilities/task'
import { SCRIPT_CAPABILITY_ID } from '../capabilities/script'
import { CLIPBOARD_CAPABILITY_ID } from '@browser-os/capability-clipboard'
import { APPS_CAPABILITY_ID } from '../capabilities/apps'
import { TOOLS_CAPABILITY_ID } from '../capabilities/tools'
import { HOME_CAPABILITY_ID } from '../capabilities/home'
import { VAULT_CAPABILITY_ID } from '@browser-os/capability-vault'

// 注意：preset 只是「预设」，不是产品边界。任意合法组合走 VITE_CAPABILITY_ASSEMBLY（见 index.ts）。
export type CapabilityProfileId = 'framework' | 'minimal' | 'developer' | 'full' | 'custom'

export const CAPABILITY_PROFILES: Record<CapabilityProfileId, string[]> = {
  // framework-only：零能力，只有底座（验证「Shell 不依赖任何能力也能起来」）
  framework: [],
  minimal: [BOOKMARK_CAPABILITY_ID, WORKSPACE_CAPABILITY_ID],
  developer: [
    BOOKMARK_CAPABILITY_ID,
    WORKSPACE_CAPABILITY_ID,
    BROWSER_CAPABILITY_ID,
    SESSION_CAPABILITY_ID,
    TERMINAL_CAPABILITY_ID,
    CREDENTIAL_CAPABILITY_ID,
    GIT_CAPABILITY_ID,
    DATABASE_CAPABILITY_ID,
  ],
  full: [
    BOOKMARK_CAPABILITY_ID,
    WORKSPACE_CAPABILITY_ID,
    BROWSER_CAPABILITY_ID,
    SESSION_CAPABILITY_ID,
    WORKBENCH_CAPABILITY_ID,
    TERMINAL_CAPABILITY_ID,
    CREDENTIAL_CAPABILITY_ID,
    GIT_CAPABILITY_ID,
    DATABASE_CAPABILITY_ID,
    AGENT_CAPABILITY_ID,
    SKILL_CAPABILITY_ID,
    PLUGIN_CAPABILITY_ID,
    KNOWLEDGE_GRAPH_CAPABILITY_ID,
    TASK_CAPABILITY_ID,
    SCRIPT_CAPABILITY_ID,
    CLIPBOARD_CAPABILITY_ID,
    APPS_CAPABILITY_ID,
    TOOLS_CAPABILITY_ID,
    HOME_CAPABILITY_ID,
    // Vault 已是 M2 package（CAPABILITY_CATALOG 一员），此前漏列于 full profile
    // → 默认 full 下其 Host 注入的 onActivate 永不触发、vault.main 贡献未注册。
    // 补列以对齐 CAPABILITY_CATALOG，使 Host explicit registration 路径完整（无重复注册）。
    VAULT_CAPABILITY_ID,
  ],
  // custom 的清单不在此处：由运行时 VITE_CAPABILITY_ASSEMBLY 决定（这里必须是同一 Record 的一分子）
  custom: [],
}

export const DEFAULT_PROFILE: CapabilityProfileId = 'full'

export function resolveProfile(input?: string | null): CapabilityProfileId {
  const v = (input || '').trim().toLowerCase()
  if (v === 'framework' || v === 'minimal' || v === 'developer' || v === 'full' || v === 'custom') return v
  return DEFAULT_PROFILE
}

/** 从运行环境读取 profile（Vite 注入 / 构建期常量 / 默认值） */
export function profileFromEnv(): CapabilityProfileId {
  const raw =
    (typeof import.meta !== 'undefined' &&
      // @ts-expect-error Vite 注入
      (import.meta.env?.VITE_CAPABILITY_PROFILE as string | undefined)) ||
    undefined
  return resolveProfile(raw)
}
