// src/capabilities/skill/state/useSkillStore.ts
// Skill 能力的**语义 owner**（Capability Library Expansion v1，STAGE E）。
//
// 从 useAgentStore 迁出 skill 全部 truth（skills / selectedSkill / skillValidation 等），
// 不再让 Agent store 偷偷持有 Skill 状态（§21）。Agent 仅保留 agent truth。
// 安装/确认走中性协调器 useInstallConfirmStore（installSkill 写入 install_skill 确认项）。
//
// 约束（沿用 M5-6 口径）：
//   - 组件不得直接 invoke，一切经 bridge.ts；
//   - 后端 skill_* 命令未落地前 backendReady=false，所有动作 no-op 且零 invoke；
//   - secret 不进 store、不落盘、不进审计；错误串/解析 def 纵深脱敏。

import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  AGENT_SKILL_COMMANDS_AVAILABLE,
  AGENT_SKILL_READONLY_COMMANDS_AVAILABLE,
  bridge,
} from "../../../bridge";
import type { SkillDef, ValidationReport, PermissionPreview } from "../../../types";
import {
  isConfirmExpired,
  makePendingConfirm,
} from "../../../utils/agentSkillUi";
import { useInstallConfirmStore } from "../../../stores/useInstallConfirmStore";

// 纯函数：secret 纵深脱敏（与 useAgentStore 同口径，避免 prompt-secret 进前端状态）。
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
function sanitizeSkillDef(d: SkillDef | null): SkillDef | null {
  if (!d) return null;
  return { ...d, description: redactSecretsInText(d.description) };
}
const PREVIEW_CAP = 20;
function boundCaps(caps?: string[]): { shown: string[]; overflow: number; total: number } {
  const c = caps ?? [];
  return { shown: c.slice(0, PREVIEW_CAP), overflow: Math.max(0, c.length - PREVIEW_CAP), total: c.length };
}

export const useSkillStore = defineStore("skill", () => {
  const skills = ref<SkillDef[]>([]);
  const skillInstalls = ref<Record<string, unknown>>({});
  const selectedSkillId = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  // 后端命令可用性（A5 落地后置 true）。
  const backendReady = ref<boolean>(AGENT_SKILL_COMMANDS_AVAILABLE);

  // W9：只读校验消费态（仅结构化结果，不持有原始输入文本）。
  const skillValidation = ref<ValidationReport | null>(null);
  const skillParseDef = ref<SkillDef | null>(null);
  const skillPreview = ref<PermissionPreview | null>(null);

  const selectedSkill = computed(() =>
    skills.value.find((s) => s.id === selectedSkillId.value) ?? null,
  );
  const skillStatus = computed<"empty" | "loading" | "ok" | "error">(() => {
    if (loading.value) return "loading";
    if (!skillValidation.value) return "empty";
    return skillValidation.value.valid ? "ok" : "error";
  });
  const boundedPreview = computed(() => boundCaps(skillPreview.value?.capabilities));

  function guard(): boolean {
    if (backendReady.value) return true;
    error.value =
      "后端 Skill 命令（skill_list / skill_install 等）尚未就绪（A5 W4 仅落地 domain + policy），面板为只读壳";
    return false;
  }
  function fail(e: unknown): void {
    error.value = e instanceof Error ? e.message : String(e ?? "未知错误");
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

  function selectSkill(id: string): void {
    selectedSkillId.value = selectedSkillId.value === id ? null : id;
  }

  // 安装：发 skill_install -> 若 CONFIRM_REQUIRED 收到 request_id -> 写入中性协调器的 pendingConfirms。
  async function installSkill(id: string): Promise<boolean> {
    if (!guard()) return false;
    loading.value = true;
    error.value = null;
    try {
      const result = (await bridge.skillInstall(id)) as { request_id?: string } | void;
      if (result && result.request_id) {
        useInstallConfirmStore().setPending("install_skill", { id });
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

  // W9：只读校验消费（与 install/run 就绪态解耦；即便 backendReady=false 仍可用）。
  async function validateSkill(text: string): Promise<void> {
    if (!AGENT_SKILL_READONLY_COMMANDS_AVAILABLE) {
      error.value = "只读校验命令未就绪";
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      const [report, def, preview] = await Promise.all([
        bridge.skillValidate(text),
        bridge.skillParse(text).catch(() => null),
        bridge.skillPermissionPreview(text).catch(() => null),
      ]);
      skillValidation.value = { valid: report.valid, errors: sanitizeErrors(report.errors) };
      skillParseDef.value = sanitizeSkillDef(def);
      skillPreview.value = preview;
    } catch (e) {
      fail(e);
    } finally {
      loading.value = false;
    }
  }

  function clearValidation(): void {
    skillValidation.value = null;
    skillParseDef.value = null;
    skillPreview.value = null;
    error.value = null;
  }

  // 二段式确认闸门（委派中性协调器）：按 action 落库后，本 store 自行重载 skill 列表。
  // 不 import 协调器以外的 Capability store（CB-05）；协调器仅负责 bridge 派发 + 删除 pending。
  async function ackConfirm(requestId: string, decision: "approve" | "deny"): Promise<boolean> {
    const ok = await useInstallConfirmStore().ackConfirm(requestId, decision);
    if (ok) await loadSkills();
    return ok;
  }

  return {
    skills,
    skillInstalls,
    selectedSkillId,
    selectedSkill,
    loading,
    error,
    backendReady,
    loadSkills,
    selectSkill,
    installSkill,
    validateSkill,
    clearValidation,
    ackConfirm,
    skillValidation,
    skillParseDef,
    skillPreview,
    skillStatus,
    boundedPreview,
  };
});
