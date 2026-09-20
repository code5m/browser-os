// Hot-Plug Orchestrator（§12 / §20 / §21）
//
// 把「通用 hotplug 流程」接到「真实 runtime + 真实 contribution registry」上。
// 本模块是薄适配层：不做业务决策、不持有业务 state、不 import 任何能力实现。
// 能力加载器由外部注入（产品装配层），避免这里长成一个 God Service Locator。

import type { CapabilityDefinition } from "../types";
import type { CapabilityRuntime } from "../runtime";
import type { ContributionRegistry } from "../contribution/registry";
import { CONTRIBUTION_SLOTS } from "../contribution/types";
import type { CapabilityManifestV1 } from "./contract";
import { requestAdd, requestRemove, disableCapability, enableCapability, type HotPlugTarget } from "./hotplug";
import { type PlatformState } from "./lifecycle";

export interface OrchestratorDeps {
  runtime: CapabilityRuntime;
  registry: ContributionRegistry;
  /** id → 真实能力定义（含 v1 契约） */
  definitions: Record<string, CapabilityDefinition>;
  /** id → 真实加载器（在能力包内注册贡献） */
  loaders?: Record<string, () => void | Promise<void>>;
  onDeactivate?: (id: string) => void | Promise<void>;
  hasActiveWork?: (id: string) => boolean;
}

const ALL_SLOTS: string[] = Object.values(CONTRIBUTION_SLOTS);

function initialState(recState: string, enabled: boolean): PlatformState {
  if (!enabled) return "DISABLED";
  switch (recState) {
    case "ACTIVE":
      return "ACTIVE";
    case "SUSPENDED":
      return "SUSPENDED";
    case "READY":
      return "ENABLED";
    default:
      return "REGISTERED";
  }
}

export interface Orchestrator {
  target: HotPlugTarget;
  add(id: string): Promise<{ id: string; state: PlatformState; notes: string[] }>;
  remove(id: string, force?: boolean): Promise<{ id: string; state: PlatformState; notes: string[] }>;
  disable(id: string): PlatformState;
  enable(id: string): PlatformState;
  contributionsOf(id: string): number;
  installedIds(): string[];
}

export function createOrchestrator(deps: OrchestratorDeps): Orchestrator {
  const { runtime, registry, definitions } = deps;
  const states = new Map<string, PlatformState>();

  function syncFromRuntime(): void {
    for (const entry of runtime.inspect()) {
      if (!states.has(entry.id)) {
        states.set(entry.id, initialState(entry.state, entry.enabled));
      }
    }
  }

  function installed(): Record<string, CapabilityManifestV1> {
    const out: Record<string, CapabilityManifestV1> = {};
    for (const entry of runtime.inspect()) {
      const def = runtime.get(entry.id)?.definition ?? definitions[entry.id];
      const v1 = (def as CapabilityDefinition | undefined)?.v1;
      if (v1) out[entry.id] = v1;
    }
    return out;
  }

  function contributionsOf(id: string): number {
    let n = 0;
    for (const slot of ALL_SLOTS) {
      n += registry.getBySlot(slot).filter((c) => c.capabilityId === id).length;
    }
    return n;
  }

  const target: HotPlugTarget = {
    get installed() {
      return installed();
    },
    stateOf(id) {
      if (!states.has(id)) syncFromRuntime();
      return states.get(id) ?? "AVAILABLE";
    },
    setState(id, to) {
      states.set(id, to);
    },
    addRecord(manifest) {
      const def = definitions[manifest.id];
      if (!def) throw new Error(`缺少该能力的真实定义，无法注册: ${manifest.id}`);
      runtime.register(def);
      try {
        runtime.resolve(manifest.id);
      } catch {
        /* 依赖未就绪时由上层装配保证；此处不吞业务错误之外的场景 */
      }
    },
    removeRecord(id) {
      runtime.unregister(id);
      states.delete(id);
    },
    addContributions(manifest) {
      const loader = deps.loaders?.[manifest.id];
      if (!loader) throw new Error(`未提供加载器: ${manifest.id}`);
      const maybe = loader();
      if (maybe && typeof (maybe as Promise<void>).then === "function") {
        void (maybe as Promise<void>);
      }
    },
    removeContributions(id) {
      for (const slot of ALL_SLOTS) {
        for (const c of registry.getBySlot(slot)) {
          if (c.capabilityId === id) registry.unregisterContribution(c.id);
        }
      }
    },
    onDeactivate: deps.onDeactivate,
    hasActiveWork: deps.hasActiveWork,
  };

  return {
    target,
    async add(id) {
      const def = definitions[id];
      if (!def?.v1) throw new Error(`该能力缺少 Building Block Contract v1: ${id}`);
      const load = deps.loaders?.[id];
      if (!load) throw new Error(`未提供加载器: ${id}`);
      return requestAdd(target, { manifest: def.v1, load });
    },
    async remove(id, force) {
      return requestRemove(target, { id, force });
    },
    disable(id) {
      return disableCapability(target, id);
    },
    enable(id) {
      return enableCapability(target, id);
    },
    contributionsOf,
    installedIds() {
      return runtime.inspect().map((e) => e.id).sort();
    },
  };
}

/** framework-only 装配：空能力集 —— 必须能成功 bootstrap（§51-A） */
export function isFrameworkOnly(runtime: CapabilityRuntime): boolean {
  return runtime.inspect().length === 0;
}
