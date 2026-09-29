// src/capabilities/agent/state/useAgentStore.ts
// M5-6 Agent 面板状态机。
//
// STAGE E（Capability Library Expansion v1）：
//   - Skill truth 已迁出至 src/capabilities/skill/state/useSkillStore（§21：Skill 不再被 Agent 偷偷持有）。
//   - 安装/确认二段式闸门迁出至 src/stores/useInstallConfirmStore（中性协调器，agent/skill 共用）。
// 本文件仅持有 Agent truth。
//
// 约束（board M5-W5 + A6 契约）：
//   - 组件不得直接 invoke，一切经 `bridge.ts`；
//   - 后端 agent_* 命令未落地前 backendReady=false，所有动作 no-op 且零 invoke（LIMITED START 口径）；
//   - secret 值不进 store、不落盘、不进审计（W4/W5 边界）。

import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  AGENT_SKILL_COMMANDS_AVAILABLE,
  AGENT_SKILL_READONLY_COMMANDS_AVAILABLE,
  bridge,
} from "../../../bridge";
import type {
  AgentDef,
  AgentSessionUI,
  PermissionPreview,
  StreamChunk,
  ValidationReport,
} from "../../../types";
import { appendChunk } from "../../../utils/agentSkillUi";
import { useInstallConfirmStore } from "../../../stores/useInstallConfirmStore";

// 纯函数：secret 纵深脱敏（与 useSkillStore 同口径，兜底后端 CredentialLeak Display 脱敏）。
const SECRET_REDACT_PATTERNS: RegExp[] = [
  /sk-[A-Za-z0-9]{8,}/g,
  /AKIA[0-9A-Z]{12,}/g,
  /Bearer\s+[A-Za-z0-9._-]+/g,
  /eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/g,
  /-----BEGIN[^]*?PRIVATE KEY-----/g,
];
function redactSecretsInText(s: string): string {
  return SECRET_REDACT_PATTERNS.reduce((acc, re) => acc.replace(re, "***"), s);
}
function sanitizeErrors(errors: string[]): string[] {
  return errors.map(redactSecretsInText);
}
function sanitizeAgentDef(d: AgentDef | null): AgentDef | null {
  if (!d) return null;
  return {
    ...d,
    description: redactSecretsInText(d.description),
    system_prompt: redactSecretsInText(d.system_prompt),
  };
}
// 预览能力有界：展示前 N 项，其余标为溢出（不丢总数信息，前端按需展开）。
const PREVIEW_CAP = 20;
function boundCaps(caps?: string[]): { shown: string[]; overflow: number; total: number } {
  const c = caps ?? [];
  return { shown: c.slice(0, PREVIEW_CAP), overflow: Math.max(0, c.length - PREVIEW_CAP), total: c.length };
}

export const useAgentStore = defineStore("agent", () => {
  const agents = ref<AgentDef[]>([]);
  const agentInstalls = ref<Record<string, unknown>>({});
  // 流式会话（agent:// 事件累积）；仅 UI 展示，不落盘。
  const sessions = ref<Record<string, AgentSessionUI>>({});
  const selectedAgentId = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  // 后端命令可用性。A5 落地后置 true（见 `bridge.AGENT_SKILL_COMMANDS_AVAILABLE`）。
  const backendReady = ref<boolean>(AGENT_SKILL_COMMANDS_AVAILABLE);

  // W9：只读校验消费态（M5-W7 桥已落地，AGENT_SKILL_READONLY_COMMANDS_AVAILABLE=true）。
  // 仅持有后端返回的结构化结果（校验报告 / 解析 def / 权限预览），**不持有原始输入文本**。
  const agentValidation = ref<ValidationReport | null>(null);
  const agentParseDef = ref<AgentDef | null>(null);
  const agentPreview = ref<PermissionPreview | null>(null);

  const selectedAgent = computed(() =>
    agents.value.find((a) => a.id === selectedAgentId.value) ?? null,
  );
  const agentStatus = computed<"empty" | "loading" | "ok" | "error">(() => {
    if (loading.value) return "loading";
    if (!agentValidation.value) return "empty";
    return agentValidation.value.valid ? "ok" : "error";
  });
  const boundedAgentPreview = computed(() => boundCaps(agentPreview.value?.capabilities));

  /// 后端未就绪时统一拦截：**一条 invoke 都不发**，避免对不存在的命令反复报错。
  function guard(): boolean {
    if (backendReady.value) return true;
    error.value =
      "后端 Agent/Skill 命令（agent_list / agent_install / agent_chat 等）尚未就绪（A5 W4 仅落地 domain + policy），面板为只读壳";
    return false;
  }

  function fail(e: unknown): void {
    error.value = e instanceof Error ? e.message : String(e ?? "未知错误");
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

  function selectAgent(id: string): void {
    selectedAgentId.value = selectedAgentId.value === id ? null : id;
  }

  // 安装：发 agent_install -> 若 CONFIRM_REQUIRED 收到 request_id
  // -> 写入中性协调器的 pendingConfirms -> UI 弹 PermissionPreviewModal -> ackConfirm 落库。
  async function installAgent(id: string): Promise<boolean> {
    if (!guard()) return false;
    loading.value = true;
    error.value = null;
    try {
      const result = (await bridge.agentInstall(id)) as { request_id?: string } | void;
      if (result && result.request_id) {
        useInstallConfirmStore().setPending("install_agent", result.request_id, { id });
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

  // 确认闸门委托中性协调器（不再让 agent store 持有 skill 确认项）。
  async function ackConfirm(requestId: string, decision: "approve" | "deny"): Promise<boolean> {
    const ok = await useInstallConfirmStore().ackConfirm(requestId, decision);
    if (ok) await loadAgents();
    return ok;
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

  // W9：只读校验消费。仅解析/校验/预览（M5-W7 桥），不安装/不执行/不写持久化。
  // 经 AGENT_SKILL_READONLY_COMMANDS_AVAILABLE 开关放行——与 install/run 就绪态解耦。
  async function validateAgent(text: string): Promise<void> {
    if (!AGENT_SKILL_READONLY_COMMANDS_AVAILABLE) {
      error.value = "只读校验命令未就绪";
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      const [report, def, preview] = await Promise.all([
        bridge.agentValidate(text),
        bridge.agentParse(text).catch(() => null),
        bridge.agentPermissionPreview(text).catch(() => null),
      ]);
      agentValidation.value = { valid: report.valid, errors: sanitizeErrors(report.errors) };
      agentParseDef.value = sanitizeAgentDef(def);
      agentPreview.value = preview;
    } catch (e) {
      fail(e);
    } finally {
      loading.value = false;
    }
  }

  // 清空只读校验消费态，回到确定的 empty 态。
  function clearValidation(): void {
    agentValidation.value = null;
    agentParseDef.value = null;
    agentPreview.value = null;
    error.value = null;
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
    agents,
    agentInstalls,
    sessions,
    selectedAgentId,
    selectedAgent,
    loading,
    error,
    backendReady,
    loadAgents,
    selectAgent,
    installAgent,
    ackConfirm,
    runAgent,
    cancelRun,
    pushChunk,
    endSession,
    // W9 只读校验消费态
    validateAgent,
    clearValidation,
    agentValidation,
    agentParseDef,
    agentPreview,
    agentStatus,
    boundedAgentPreview,
  };
});
