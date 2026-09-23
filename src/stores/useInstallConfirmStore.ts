// src/stores/useInstallConfirmStore.ts
// 中性安装/确认协调器（Capability Library Expansion v1，STAGE E）。
//
// 背景：Agent/Skill 的「二段式安装闸门」曾全部塞在 useAgentStore 内（pendingConfirms / ackConfirm
// 同时处理 install_skill 与 install_agent），导致 Skill 的 truth 被 Agent 偷偷持有（§21 禁止）。
// 现抽出为中性协调器：agent 与 skill 两个能力各自经 setPending 写入自己的确认项，
// ackConfirm 按 action 派发（install_skill -> bridge.confirmSkill / install_agent -> bridge.confirmAgent），
// 不再让任一能力拥有对方的 truth。
//
// 依赖方向：useAgentStore / useSkillStore 在顶部静态 import 本文件（单向）；
// 本文件**不** import 任何 Capability store（CB-05）。列表重载由各自能力 store 的 ackConfirm
// delegate 后完成（useSkillStore / useAgentStore 各 reload 自有列表），协调器只负责 bridge 派发。

import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { AGENT_SKILL_COMMANDS_AVAILABLE, bridge } from "../bridge";
import { isConfirmExpired, makePendingConfirm, type PendingConfirmAction } from "../utils/agentSkillUi";

export const useInstallConfirmStore = defineStore("install-confirm", () => {
  const pendingConfirms = ref<Map<string, ReturnType<typeof makePendingConfirm>>>(new Map());

  const pendingConfirmList = computed(() =>
    [...pendingConfirms.value.values()].filter((c) => !isConfirmExpired(c)),
  );

  function setPending(action: PendingConfirmAction, payload: unknown): string {
    const c = makePendingConfirm(action, payload);
    const m = new Map(pendingConfirms.value);
    m.set(c.payload as string, c); // 临时 key；确认后由 ackConfirm 用真实 request_id 覆盖
    pendingConfirms.value = m;
    return c.payload as string;
  }

  async function ackConfirm(requestId: string, decision: "approve" | "deny"): Promise<boolean> {
    // 后端命令未就绪（A5 W4 仅落地 domain + policy）→ 一条 invoke 都不发。
    if (!AGENT_SKILL_COMMANDS_AVAILABLE) return false;
    const c = pendingConfirms.value.get(requestId);
    if (!c || isConfirmExpired(c)) return false;
    try {
      if (c.action === "install_skill") await bridge.confirmSkill(requestId, decision);
      else if (c.action === "install_agent") await bridge.confirmAgent(requestId, decision);
      const m = new Map(pendingConfirms.value);
      m.delete(requestId);
      pendingConfirms.value = m;
      // 列表重载由各能力 store 自行负责（useSkillStore.ackConfirm / useAgentStore.ackConfirm
      // 在 delegate 后 reload 自有列表），本中性协调器不 import 任何 Capability store（CB-05）。
      return true;
    } catch {
      return false;
    }
  }

  return { pendingConfirms, pendingConfirmList, setPending, ackConfirm };
});
