// M5-W17 A3：主页（Home）**纯逻辑层**（与 `src/utils/graphUi.ts` 同范式）。
//
// 无 invoke / DOM / store / Vue 依赖，可被 `scripts/check-home-store-logic.mjs` headless 直测。
//
// 目标（board §M5-W17，A3 行）：
//   1. 有界（bounded）        ：快捷方式 / 最近访问均有硬上限，绝不无界增长；
//   2. 已校验（validated）    ：来自 localStorage 的数据逐字段校验后才进 UI；
//   3. 稳定默认（stable defaults）：数据缺失/损坏 → 回落到确定的默认集合；
//   4. 迁移安全（migration-safe）：旧版/脏数据（缺 id、type 非法、字段类型错、超长）
//                              要么被就地修正，要么被丢弃，绝不把脏字段直接渲染到界面；
//   5. 零敏感披露（共享验收 #4）：展示/无障碍文案只给「安全摘要」，
//                              绝不回显完整本地路径、URL query / userinfo、凭据。
//
// 边界：本文件只做纯函数与常量；不新增命令/bridge/ACL、不碰后端、不改样式。

/** 快捷方式硬上限（有界：防止 localStorage 脏数据把首页撑爆）。 */
export const HOME_MAX_SHORTCUTS = 24;
/** 最近访问硬上限（有界）。 */
export const HOME_MAX_RECENTS = 12;
export const HOME_MAX_NAME_LEN = 64;
export const HOME_MAX_TARGET_LEN = 2048;
export const HOME_MAX_ICON_LEN = 8;
/** 展示/无障碍文案上限（避免超长路径/URL 刷屏）。 */
export const HOME_DISPLAY_MAX_LEN = 48;

export type HomeShortcutType = "url" | "app" | "dir";

export interface HomeShortcut {
  id: string;
  type: HomeShortcutType;
  name: string;
  target: string;
  icon: string;
}

export interface HomeRecent extends HomeShortcut {
  /** 最近一次打开的时间戳（毫秒；脏数据回落 0，不参与排序稳定性之外的逻辑）。 */
  at: number;
}

/** 主页「主要工作区」入口的视图名——**必须是 `MainView` 的子集**（不新增路由）。 */
export type HomeAreaView =
  | "browser"
  | "files"
  | "db"
  | "repo"
  | "term"
  | "tasks"
  | "skills"
  | "agents"
  | "graph"
  | "plugin";

export interface HomeArea {
  view: HomeAreaView;
  icon: string;
  label: string;
  hint: string;
}

/** 主要工作区（共享验收 #2：把既有模块暴露为可用入口，而不是营销页）。 */
export const HOME_PRINCIPAL_AREAS: readonly HomeArea[] = Object.freeze([
  { view: "browser", icon: "🌐", label: "浏览器", hint: "网页浏览与地址栏" },
  { view: "files", icon: "📂", label: "文件", hint: "本地目录与文件树" },
  { view: "db", icon: "🗄️", label: "数据库", hint: "连接与只读查询" },
  { view: "repo", icon: "🛰️", label: "仓库", hint: "Git 仓库状态" },
  { view: "term", icon: "💻", label: "终端", hint: "本地终端会话" },
  { view: "tasks", icon: "⏰", label: "定时任务", hint: "计划任务与运行记录" },
  { view: "skills", icon: "🛠️", label: "技能", hint: "已注册技能（只读）" },
  { view: "agents", icon: "🤖", label: "智能体", hint: "已注册智能体（只读）" },
  { view: "graph", icon: "🕸️", label: "图谱", hint: "知识图谱浏览（只读）" },
  { view: "plugin", icon: "🔌", label: "插件", hint: "插件清单与状态（只读）" },
]);

export const TYPE_LABELS: Record<HomeShortcutType, string> = {
  url: "网页",
  app: "应用",
  dir: "目录",
};

/** 面板三态文案（空/载入/错误都给确定文案；错误态**绝不回显原始错误串**——共享验收 #4）。 */
export const HOME_EMPTY_MESSAGE = "还没有快捷方式。可用上方「＋ 新增」添加常用网页、应用或目录。";
export const HOME_RECENTS_EMPTY_MESSAGE = "还没有最近访问记录。打开任一快捷方式后会记录到这里。";
export const HOME_ERROR_FALLBACK = "主页数据载入失败，已回退到安全默认。可点「默认」恢复默认快捷方式。";
export const HOME_LOADING_MESSAGE = "正在载入主页…";
export const HOME_SHORTCUT_LIMIT_MESSAGE = `快捷方式已达上限（${HOME_MAX_SHORTCUTS} 个），请先删除后再添加。`;

/** 稳定默认（首次使用 / 数据损坏时回落；内容与改造前保持一致，避免行为漂移）。 */
export function defaultShortcuts(): HomeShortcut[] {
  return [
    { id: "d1", type: "url", name: "百度", target: "https://www.baidu.com", icon: "🔍" },
    { id: "d2", type: "url", name: "Kimi", target: "https://kimi.moonshot.cn", icon: "🌙" },
    { id: "d3", type: "url", name: "DeepSeek", target: "https://chat.deepseek.com", icon: "🐋" },
    { id: "d4", type: "url", name: "B站", target: "https://www.bilibili.com", icon: "📺" },
    { id: "d5", type: "url", name: "GitHub", target: "https://github.com", icon: "🐙" },
    { id: "d6", type: "url", name: "Gitee", target: "https://gitee.com", icon: "🐴" },
  ];
}

// ---------------------------------------------------------------------------
// 基础校验/规整
// ---------------------------------------------------------------------------

export function isShortcutType(v: unknown): v is HomeShortcutType {
  return v === "url" || v === "app" || v === "dir";
}

/** 非字符串 → ""；超长 → 截断。绝不把 number/object/null 透传给 UI。 */
export function clampText(v: unknown, max: number): string {
  if (typeof v !== "string") return "";
  const t = v.trim();
  return t.length > max ? t.slice(0, max) : t;
}

/** djb2 变体：确定性哈希（迁移安全——同一 type+target 永远得到同一 id）。 */
function hash32(s: string): string {
  let h = 5381;
  for (let i = 0; i < s.length; i++) h = ((h << 5) + h + s.charCodeAt(i)) | 0;
  return (h >>> 0).toString(36);
}

/** 确定性 id：旧数据缺 id 时派生，保证去重与渲染 key 稳定（不用随机，避免每次加载漂移）。 */
export function stableId(type: HomeShortcutType, target: string): string {
  return `home-${type}-${hash32(`${type}|${target}`)}`;
}

/** 单条校验：脏数据 → 就地修正；无法修正 → null（由调用方丢弃）。 */
export function sanitizeShortcut(raw: unknown): HomeShortcut | null {
  if (!raw || typeof raw !== "object") return null;
  const r = raw as Record<string, unknown>;
  if (!isShortcutType(r.type)) return null;
  const name = clampText(r.name, HOME_MAX_NAME_LEN);
  const target = clampText(r.target, HOME_MAX_TARGET_LEN);
  if (!name || !target) return null;
  const id = clampText(r.id, 64) || stableId(r.type, target);
  const icon = clampText(r.icon, HOME_MAX_ICON_LEN) || "🔗";
  return { id, type: r.type, name, target, icon };
}

/** 列表规整：逐条校验 + 去重（type|target）+ id 去重 + 有界封顶。 */
export function normalizeShortcuts(raw: unknown): HomeShortcut[] {
  if (!Array.isArray(raw)) return [];
  const out: HomeShortcut[] = [];
  const seenKeys = new Set<string>();
  const seenIds = new Set<string>();
  for (const item of raw) {
    let s = sanitizeShortcut(item);
    if (!s) continue;
    const key = `${s.type}|${s.target}`;
    if (seenKeys.has(key)) continue;
    // 旧数据可能带重复/非法 id：冲突时改用确定性 id，保证渲染 key 唯一。
    if (seenIds.has(s.id)) s = { ...s, id: stableId(s.type, s.target) };
    seenKeys.add(key);
    seenIds.add(s.id);
    out.push(s);
    if (out.length >= HOME_MAX_SHORTCUTS) break;
  }
  return out;
}

/** 最近访问规整：同 shortcuts 的校验口径，额外规整 `at`，并按时间倒序（新的在前）。 */
export function normalizeRecents(raw: unknown): HomeRecent[] {
  if (!Array.isArray(raw)) return [];
  const out: HomeRecent[] = [];
  const seenKeys = new Set<string>();
  for (const item of raw) {
    const s = sanitizeShortcut(item);
    if (!s) continue;
    const r = item as Record<string, unknown>;
    const at =
      typeof r.at === "number" && Number.isFinite(r.at) ? Math.max(0, Math.trunc(r.at)) : 0;
    const key = `${s.type}|${s.target}`;
    if (seenKeys.has(key)) continue;
    seenKeys.add(key);
    out.push({ ...s, at });
    if (out.length >= HOME_MAX_RECENTS) break;
  }
  // 倒序（新的在前）；相等时保持输入顺序（稳定，不随机）。
  return out.sort((a, b) => b.at - a.at);
}

export function recentFromShortcut(s: HomeShortcut, at: number): HomeRecent {
  return { ...s, at: Number.isFinite(at) ? Math.max(0, Math.trunc(at)) : 0 };
}

/** 头插 + 去重 + 封顶：最近访问只保留最新 cap 条（有界）。 */
export function pushRecent(
  list: HomeRecent[],
  item: HomeRecent,
  cap: number = HOME_MAX_RECENTS,
): HomeRecent[] {
  const key = (r: HomeRecent) => `${r.type}|${r.target}`;
  const k = key(item);
  const filtered = list.filter((r) => key(r) !== k);
  return [item, ...filtered].slice(0, Math.max(0, cap));
}

/** 通用有界追加：按 key 去重后追加，超上限保留**最新**的 cap 条。 */
export function boundedPush<T>(
  list: T[],
  items: T[],
  cap: number,
  keyFn: (t: T) => string,
): T[] {
  const seen = new Set(list.map(keyFn));
  const out = list.slice();
  for (const it of items) {
    const k = keyFn(it);
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(it);
  }
  return out.length > cap ? out.slice(out.length - Math.max(0, cap)) : out;
}

// ---------------------------------------------------------------------------
// 持久化安全（M5-W17 Acceptance Closeout · 债务 HOME_NO_SECRET_PERSIST）
//
// `type: "app"` 的 `target` 是**应用启动命令体**（可能带参数、临时令牌、本地路径等），
// 把它写进 localStorage 等于把可执行命令与潜在凭据长期留在浏览器存储里。
// 因此：app 条目**仅存在于内存（会话内有效）**，落库前一律剔除；
// 浏览器存储只保留非敏感主页元数据（url / dir 的名称、图标、目标）。
// 迁移期：旧数据里已存在的 app 条目在载入时被剔除并随即从存储中抹除。
// ---------------------------------------------------------------------------

/** 用户可见提示：应用条目仅会话内有效（不含任何命令体/路径）。 */
export const HOME_APP_SESSION_ONLY_NOTICE =
  "已添加（仅本次会话有效）：应用启动命令不会写入浏览器存储。";

/** 允许写入浏览器存储的类型（不含 app）。 */
export const HOME_PERSISTED_TYPES: readonly HomeShortcutType[] = Object.freeze(["url", "dir"]);

/** 该条目可否安全落库：app 的 target 是命令体 → 否。 */
export function isStorageSafe<T extends HomeShortcut>(s: T): boolean {
  return s.type !== "app";
}

/** 落库前过滤：只保留非敏感主页元数据；app 命令体仅驻留内存（会话内）。 */
export function toPersisted<T extends HomeShortcut>(list: T[]): T[] {
  return list.filter(isStorageSafe);
}

/** 便于回归断言：落库载荷的序列化结果（**绝不含 app 命令体**）。 */
export function persistedJson(list: HomeShortcut[]): string {
  return JSON.stringify(toPersisted(list));
}

// ---------------------------------------------------------------------------
// 零敏感披露（共享验收 #4）：只给「安全摘要」
// ---------------------------------------------------------------------------

// 刻意不带 g 标志（避免 lastIndex 状态影响 .test 结果）。
const SENSITIVE_PATTERNS: RegExp[] = [
  /[a-z][a-z0-9+.-]*:\/\/[^\s/:@]+:[^\s/@]+@/i,
  /[?&](?:access_token|refresh_token|private_token|token|password|secret|api_key)=/i,
  /\b(?:gh[pous]_[A-Za-z0-9]{16,}|github_pat_[A-Za-z0-9_]{20,}|glpat-[A-Za-z0-9_-]{16,})\b/,
  /\b(?:Bearer|Basic)\s+[A-Za-z0-9+/=._-]{8,}/,
  /\b(?:sk-[A-Za-z0-9]{16,}|AKIA[A-Z0-9]{12,})\b/,
];

/** 文本是否含敏感形态（URL userinfo / 凭据参数 / token 前缀 / Authorization 值）。 */
export function looksSensitive(text: string): boolean {
  return SENSITIVE_PATTERNS.some((re) => re.test(text));
}

function safeOrFallback(label: string, fallback: string): string {
  if (!label) return fallback;
  return looksSensitive(label) ? fallback : label;
}

function safeUrlLabel(raw: string): string {
  try {
    const u = new URL(raw);
    // 只保留 scheme+host+path：丢弃 userinfo / query / hash / 凭据参数。
    const path = u.pathname === "/" ? "" : u.pathname;
    return safeOrFallback(
      clampText(`${u.protocol}//${u.hostname}${path}`, HOME_DISPLAY_MAX_LEN),
      "网页",
    );
  } catch {
    // 非法 URL：不回显原文（可能含 token 或本地路径），统一给类型标签。
    return "网页";
  }
}

function safeBaseLabel(raw: string, fallback: string): string {
  const parts = raw.replace(/[\\/]+$/, "").split(/[\\/]/).filter(Boolean);
  const base = parts.length ? parts[parts.length - 1] : "";
  return safeOrFallback(clampText(base, HOME_DISPLAY_MAX_LEN), fallback);
}

function safeCommandLabel(raw: string): string {
  const first = raw.trim().split(/\s+/)[0] ?? "";
  const parts = first.split(/[\\/]/).filter(Boolean);
  const base = parts.length ? parts[parts.length - 1] : first;
  return safeOrFallback(clampText(base, HOME_DISPLAY_MAX_LEN), "应用");
}

/** 安全展示文案：目录/应用只给末段名，URL 只给 host+path，绝不回显完整路径或凭据。 */
export function homeDisplayTarget(s: HomeShortcut | HomeRecent | null | undefined): string {
  if (!s) return "";
  const t = clampText(s.target, HOME_MAX_TARGET_LEN);
  if (!t) return TYPE_LABELS[s.type] ?? "项目";
  if (s.type === "url") return safeUrlLabel(t);
  if (s.type === "dir") return safeBaseLabel(t, "目录");
  return safeCommandLabel(t);
}

/** 无障碍名（共享验收 #4）：类型 + 名称，不含完整路径/URL/凭据。 */
export function homeAccessibleLabel(s: HomeShortcut | HomeRecent | null | undefined): string {
  if (!s) return "主页项目";
  const typeText = TYPE_LABELS[s.type] ?? "项目";
  const name = clampText(s.name, HOME_MAX_NAME_LEN) || homeDisplayTarget(s) || typeText;
  return `${typeText}：${name}`;
}

/** 工作区入口的无障碍名：label + hint，确定性、有界。 */
export function areaAccessibleLabel(a: HomeArea | null | undefined): string {
  if (!a) return "工作区";
  const label = clampText(a.label, 32) || "工作区";
  const hint = clampText(a.hint, 48);
  return hint ? `打开${label}：${hint}` : `打开${label}`;
}

// ---------------------------------------------------------------------------
// 面板三态（共享验收 #2：连贯的空/载入/错误态，且不是营销页）
// ---------------------------------------------------------------------------

export type HomePanelState = "loading" | "empty" | "ready" | "error";

export function panelStateHome(input: {
  loading?: boolean;
  count?: number;
  error?: unknown;
}): { state: HomePanelState; message: string } {
  const loading = input?.loading === true;
  const rawCount = input?.count;
  const count =
    typeof rawCount === "number" && Number.isFinite(rawCount)
      ? Math.max(0, Math.trunc(rawCount))
      : 0;
  if (loading) return { state: "loading", message: HOME_LOADING_MESSAGE };
  // 错误态：不回显原始 error（可能含路径/凭据），只给确定的兜底文案。
  if (input?.error) return { state: "error", message: HOME_ERROR_FALLBACK };
  if (count <= 0) return { state: "empty", message: HOME_EMPTY_MESSAGE };
  return { state: "ready", message: "" };
}
