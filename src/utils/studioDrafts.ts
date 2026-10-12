export type StudioKind = "agent" | "skill";
export interface StudioStep { id: string; title: string; done: boolean }
export interface StudioDraft {
  id: string; kind: StudioKind; name: string; description: string; goal: string;
  inputs: string; outputs: string; constraints: string; steps: StudioStep[];
  definition: string; createdAt: string; updatedAt: string;
}
const limits = { agent: 64, skill: 200 };
const secret = /(sk-[A-Za-z0-9_-]{8,}|AKIA[0-9A-Z]{12,}|Bearer\s+[A-Za-z0-9._-]{8,}|-----BEGIN[\s\S]*?PRIVATE KEY-----|(?:api[_-]?key|password|authorization|secret)\s*[=:]\s*[^\s,]{6,})/i;
export const studioKey = (kind: StudioKind) => `browseros.studio.${kind}.v1`;
export function createStudioDraft(kind: StudioKind): StudioDraft {
  const id = crypto.randomUUID(), now = new Date().toISOString();
  return { id, kind, name: kind === "agent" ? "新智能体" : "新技能",
    description: "", goal: "", inputs: "", outputs: "", constraints: "", definition: "",
    createdAt: now, updatedAt: now,
    steps: [{ id: crypto.randomUUID(), title: "定义目标", done: false },
      { id: crypto.randomUUID(), title: "配置输入与输出", done: false },
      { id: crypto.randomUUID(), title: "检查定义与权限", done: false }] };
}
export function validateStudioDraft(value: unknown, kind: StudioKind): StudioDraft {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw Error("无效蓝图");
  const d = value as Record<string, unknown>;
  if (d.kind !== kind || typeof d.id !== "string" || !/^[a-zA-Z0-9-]{1,100}$/.test(d.id)) throw Error("蓝图类型或编号错误");
  for (const f of ["name","description","goal","inputs","outputs","constraints","definition","createdAt","updatedAt"]) {
    if (typeof d[f] !== "string" || (d[f] as string).length > (f === "definition" ? 65536 : 4000) || secret.test(d[f] as string)) throw Error(`字段 ${f} 不符合长度或保密要求`);
  }
  if (!Array.isArray(d.steps) || d.steps.length > 32 || d.steps.some((s: unknown) =>
    !s || typeof s !== "object" || typeof (s as StudioStep).id !== "string" ||
    typeof (s as StudioStep).title !== "string" || (s as StudioStep).title.length > 180 ||
    secret.test((s as StudioStep).title) || typeof (s as StudioStep).done !== "boolean")) throw Error("步骤格式无效");
  return { id:d.id, kind, name:d.name as string, description:d.description as string,
    goal:d.goal as string, inputs:d.inputs as string, outputs:d.outputs as string,
    constraints:d.constraints as string, definition:d.definition as string,
    createdAt:d.createdAt as string, updatedAt:d.updatedAt as string,
    steps:(d.steps as StudioStep[]).map(s=>({id:s.id,title:s.title,done:s.done})) };
}
export function parseStudioList(raw: string | null, kind: StudioKind): StudioDraft[] {
  if (!raw) return [];
  if (raw.length > 2_000_000) throw Error("存储超过安全上限");
  const data: unknown = JSON.parse(raw);
  if (!Array.isArray(data) || data.length > limits[kind]) throw Error("存储格式或数量异常");
  const items = data.map(d=>validateStudioDraft(d,kind));
  if (new Set(items.map(d=>d.id)).size !== items.length) throw Error("蓝图编号重复");
  return items;
}
export function saveStudioList(items: StudioDraft[], kind: StudioKind, storage: Pick<Storage,"setItem">): void {
  const serialized = JSON.stringify(parseStudioList(JSON.stringify(items),kind));
  if (serialized.length > 2_000_000) throw Error("本地存储空间超限");
  storage.setItem(studioKey(kind), serialized);
}
export const studioProgress = (d: StudioDraft) => d.steps.length ? Math.round(d.steps.filter(s=>s.done).length * 100 / d.steps.length) : 0;
