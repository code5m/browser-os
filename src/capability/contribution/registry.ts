// Capability Contribution Registry（Phase 8B.1 — Generic Contribution Registry）
//
// 单例编排层：能力向此处注册「贡献」，Shell 按 slot 遍历渲染。
// 与 Capability Runtime 一样：本 Registry **只持编排元数据（id/slot/component 引用）**，
// 绝不持有业务状态（items/panelOpen 等）。
//
// 与 Semantic Governance 的边界：
//   - panelOpen/items 等业务真源仍在 useBookmarkStore 等 owner 手中（见 Semantic Registry）。
//   - 能力内部 UI 组件自行读取自己的 store 是允许的（位于能力包内，不违反 CB-02）。
//   - 当能力未注册/未激活时，对应 slot 为空，Shell 渲染空集——即 Bookmark absent 时 Shell 仍可启动。

import { shallowReactive } from "vue"
import type { Contribution, ContributionType } from "./types"

export interface ContributionRegistry {
  registerContribution(contribution: Contribution): void
  unregisterContribution(id: string): void
  /** 移除某能力在所有槽位中的贡献（停用/卸载的通用收口） */
  unregisterCapability(capabilityId: string): void
  /** 按所有者获取实际贡献，包含扩展槽位；供失败恢复使用。 */
  getByCapability(capabilityId: string): Contribution[]
  /** 取某槽全部贡献（按 order 升序），无则空数组 */
  getBySlot(slot: string): Contribution[]
  /** 取某槽的 surface 类贡献 */
  getSurfaceContributions(slot: string): Contribution[]
  /** 取某槽的 navigation 类贡献 */
  getNavigationContributions(slot: string): Contribution[]
  /**
   * 取某槽的**可认领页签**贡献（UI-4 DockContribution）。
   *
   * 只返回带 `view` 的 surface 贡献，按 order 升序。
   * 可用性（availability）是**隐式**的：能力未注册/未激活时其贡献根本不在表里，
   * 因此 absent capability 不会留下死页签 —— Shell 无需知道任何能力名。
   */
  getDockTabContributions(slot: string): Contribution[]
  /** 清空（供测试隔离使用） */
  clear(): void
}

export function createContributionRegistry(): ContributionRegistry {
  const contributions = shallowReactive(new Map<string, Contribution>())

  function sortByOrder(list: Contribution[]): Contribution[] {
    return [...list].sort((a, b) => (a.order ?? 0) - (b.order ?? 0))
  }

  return {
    registerContribution(contribution) {
      if (!contribution || !contribution.id) {
        throw new Error("contribution 必须有 id")
      }
      contributions.set(contribution.id, contribution)
    },
    unregisterContribution(id) {
      contributions.delete(id)
    },
    unregisterCapability(capabilityId) {
      for (const [id, contribution] of contributions) {
        if (contribution.capabilityId === capabilityId) contributions.delete(id)
      }
    },
    getByCapability(capabilityId) {
      return sortByOrder([...contributions.values()].filter((c) => c.capabilityId === capabilityId))
    },
    getBySlot(slot) {
      return sortByOrder([...contributions.values()].filter((c) => c.slot === slot))
    },
    getSurfaceContributions(slot) {
      return sortByOrder(
        [...contributions.values()].filter(
          (c) => c.slot === slot && (c.type as ContributionType) === "surface",
        ),
      )
    },
    getNavigationContributions(slot) {
      return sortByOrder(
        [...contributions.values()].filter(
          (c) => c.slot === slot && (c.type as ContributionType) === "navigation",
        ),
      )
    },
    getDockTabContributions(slot) {
      return sortByOrder(
        [...contributions.values()].filter(
          (c) => c.slot === slot && (c.type as ContributionType) === "surface" && !!c.view,
        ),
      )
    },
    clear() {
      contributions.clear()
    },
  }
}

/** 全局单例：能力注册、Shell 读取都走同一实例 */
export const contributionRegistry = createContributionRegistry()
