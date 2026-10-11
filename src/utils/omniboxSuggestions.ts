// Omnibox presentation decisions. No store, IPC, DOM or duplicate business state.
export type OmniboxKind = "web" | "dir" | "file";
export type OmniboxMode = "auto" | "url" | "dir";
export interface OmniboxItem {
  value: string;
  label: string;
  type: string;
  kind: OmniboxKind;
}
export function isLocalPath(input: string): boolean {
  const s = input.trim();
  return s.startsWith("/") || s.startsWith("~") || /^[A-Za-z]:[\\/]/.test(s);
}
export function isWebsite(input: string): boolean {
  try {
    const parsed = new URL(input.trim());
    return (parsed.protocol === "http:" || parsed.protocol === "https:") && !!parsed.hostname;
  } catch { return false; }
}
export function websiteKey(input: string): string {
  if (!isWebsite(input)) return "";
  const url = new URL(input.trim());
  return url.protocol.toLowerCase() + "//" + url.host.toLowerCase() +
    url.pathname.replace(/\/+$/, "") + url.search;
}
export function filterOmnibox(items: OmniboxItem[], value: string, typed: boolean, mode: OmniboxMode): OmniboxItem[] {
  const seen = new Set<string>();
  const needle = typed ? value.trim().toLowerCase() : "";
  return items.filter(item => {
    if (!item.value || typeof item.value !== "string") return false;
    if (mode === "url" && item.kind !== "web") return false;
    if (mode === "dir" && item.kind === "web") return false;
    const key = item.kind + ":" + (item.kind === "web" ? websiteKey(item.value) : item.value);
    if (seen.has(key)) return false;
    seen.add(key);
    return !needle || (item.value + " " + item.label).toLowerCase().includes(needle);
  }).slice(0,6);
}
export function safeRecentDirs(data: unknown): string[] {
  if (!Array.isArray(data)) return [];
  return [...new Set(data.filter((v): v is string => typeof v === "string" && v.length <= 2048 && isLocalPath(v)))].slice(0,12);
}
