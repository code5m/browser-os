// src/stores/useAgentStore.ts
// M5-6 Agent/Skill 面板状态机。
//
// 约束（board M5-W5 + A6 契约）：
//   - 组件不得直接 invoke，一切经 `bridge.ts`；
//   - 后端 skill_*/agent_* 命令由 A5（M5-4/5）落地；未落地前
//     `backendReady=false`，**所有动作 no-op 且零 invoke**（LIMITED START 口径）；
//   - 不执行 Skill、不安装插件、不调用实时运行时（W5 Hard Stop）；
//   - secret 值不进 store、不落盘、不进审计（W4/W5 边界）。

import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { AGENT_SKILL_COMMANDS_AVAILABLE, bridge } from "../bridge";
import type {
  AgentDef,
  AgentSessionUI,
  PendingConfirm,
  SkillDef,
  StreamChunk,
} from "../types";
import {
  appendChunk,
  isConfirmExpired,
  makePendingConfirm,
  type PendingConfirmAction,
} from "../utils/agentSkillUi";

export const useAgentStore = defineStore("agent", () => {
  const skills = ref<SkillDef[]>([]);
  const agents = ref<AgentDef[]>([]);
  const skillInstalls = ref<Record<string, unknown>>({});
  const agentInstalls = ref<Record<string, unknown>>({});
  // 流式会话（agent:// 事件累积）；仅 UI 展示，不落盘。
  const sessions = ref<Record<string, AgentSessionUI>>({});
  // 二段式闸门待确认项（A1 M5-6 §4.3）。过期自动清理。
  const pendingConfirms = ref<Map<string, PendingConfirm>>(new Map());
  const selectedSkillId = ref<string | null>(null);
  const selectedAgentId = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  // 后端命令可用性。A5 落地后置 true（见 `bridge.AGENT_SKILL_COMMANDS_AVAILABLE`）。
  const backendReady = ref<boolean>(AGENT_SKILL_COMMANDS_AVAILABLE);

  const selectedSkill = computed(() => skills.value.find((s) => s.id === selectedSkillId.value) ?? null);
  const selectedAgent = computed(() => agents.value.find((a) => a.id === selectedAgentId.value) ?? null);
  const pendingConfirmList = computed(() =>
    [...pendingConfirms.value.values()].filter((c) => !isConfirmExpired(c)),
  );

  /// 后端未就绪时统一拦截：**一条 invoke 都不发**，避免对不存在的命令反复报错。
  function guard(): boolean {
    if (backendReady.value) return true;
    error.value =
      "后端 Agent/Skill 命令（skill_list / agent_list / skill_install / agent_install / agent_chat 等）尚未就绪（A5 W4 仅落地 domain + policy），面板为只读壳";
    return false;
  }

  function fail(e: unknown): void {
    error.value = e instanceof Error ? e.message : String(e ?? "未知错误");
  }

  function setPending(action: PendingConfirmAction, payload: unknown): string {
    const c = makePendingConfirm(action, payload);
    const m = new Map(pendingConfirms.value);
    m.set(c.payload as string, c); // 临时 key；确认后由 ackConfirm 用真实 request_id 覆盖
    pendingConfirms.value = m;
    return c.payload as string;
  }

  async function loadSkills(): Promise<void> {
    if (!guard()) return;
    loading.value = true;
    error.value = null;
    try {
      skills.value = await bridge.skillList();
    } catch (e) {
      fail(e);
    } finally {
      loading.value = false;
    }
  }

  async function loadAgents(): Promise<void> {
    if (!guard()) return;
    loading.value = true;
    error.value = null;
    try {
      agents.value = await bridge.agentList();
    } catch (e) {
      fail(e);
    } finally {
      loading.value = false;
    }
  }

  function selectSkill(id: string): void {
    selectedSkillId.value = selectedSkillId.value === id ? null : id;
  }
  function selectAgent(id: string): void {
    selectedAgentId.value = selectedAgentId.value === id ? null : id;
  }

  // 安装：发 skill_install/agent_install -> 若 CONFIRM_REQUIRED 收到 request_id
  // -> 写入 pendingConfirms -> UI 弹 PermissionPreviewModal -> ackConfirm 落库。
  async function installSkill(id: string): Promise<boolean> {
    if (!guard()) return false;
    loading.value = true;
    error.value = null;
    try {
      const result = (await bridge.skillInstall(id)) as { request_id?: string } | void;
      if (result && result.request_id) {
        const m = new Map(pendingConfirms.value);
        m.set(result.request_id, makePendingConfirm("install_skill", { id }));
        pendingConfirms.value = m;
      }
      await loadSkills();
      return true;
    } catch (e) {
      fail(e);
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function installAgent(id: string): Promise<boolean> {
    if (!guard()) return false;
    loading.value = true;
    error.value = null;
    try {
      const result = (await bridge.agentInstall(id)) as { request_id?: string } | void;
      if (result && result.request_id) {
        const m = new Map(pendingConfirms.value);
        m.set(result.request_id, makePendingConfirm("install_agent", { id }));
        pendingConfirms.value = m;
      }
      await loadAgents();
      return true;
    } catch (e) {
      fail(e);
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function ackConfirm(requestId: string, decision: "approve" | "deny"): Promise<boolean> {
    if (!guard()) return false;
    const c = pendingConfirms.value.get(requestId);
    if (!c || isConfirmExpired(c)) {
      error.value = "确认请求已过期或不存在";
      return false;
    }
    loading.value = true;
    error.value = null;
    try {
      if (c.action === "install_skill") await bridge.confirmSkill(requestId, decision);
      else if (c.action === "install_agent") await bridge.confirmAgent(requestId, decision);
      const m = new Map(pendingConfirms.value);
      m.delete(requestId);
      pendingConfirms.value = m;
      await Promise.all([loadSkills(), loadAgents()]);
      return true;
    } catch (e) {
      fail(e);
      return false;
    } finally {
      loading.value = false;
    }
  }

  // 对话：发 agent_chat -> 订阅 agent://<id>/stream 由组件层 useAgentStream 负责，
  // 这里只建会话壳并下发；不引入第二执行路径（执行走后端既有 script_runner）。
  async function runAgent(agentId: string, prompt: string, sessionId: string): Promise<void> {
    if (!guard()) return;
    error.value = null;
    loading.value = true;
    try {
      sessions.value = {
        ...sessions.value,
        [sessionId]: { sessionId, agentId, status: "streaming", chunks: [] },
      };
      await bridge.agentChat(agentId, prompt, sessionId);
    } catch (e) {
      fail(e);
      if (sessions.value[sessionId]) {
        sessions.value = {
          ...sessions.value,
          [sessionId]: { ...sessions.value[sessionId], status: "error" },
        };
      }
    } finally {
      loading.value = false;
    }
  }

  async function cancelRun(runId: string): Promise<void> {
    if (!guard()) return;
    try {
      await bridge.agentRunCancel(runId);
    } catch (e) {
      fail(e);
    }
  }

  // 组件层流式回调：把 chunk 追加到会话（有界）。
  function pushChunk(sessionId: string, chunk: StreamChunk): void {
    const s = sessions.value[sessionId];
    if (!s) return;
    sessions.value = {
      ...sessions.value,
      [sessionId]: { ...s, chunks: appendChunk(s.chunks, chunk), status: "streaming" },
    };
  }

  function endSession(sessionId: string, status: AgentSessionUI["status"]): void {
    const s = sessions.value[sessionId];
    if (!s) return;
    sessions.value = { ...sessions.value, [sessionId]: { ...s, status } };
  }

  return {
    skills,
    agents,
    skillInstalls,
    agentInstalls,
    sessions,
    pendingConfirms,
    pendingConfirmList,
    selectedSkillId,
    selectedAgentId,
    selectedSkill,
    selectedAgent,
    loading,
    error,
    backendReady,
    loadSkills,
    loadAgents,
    selectSkill,
    selectAgent,
    installSkill,
    installAgent,
    ackConfirm,
    runAgent,
    cancelRun,
    pushChunk,
    endSession,
  };
});
