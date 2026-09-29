import { defineStore } from "pinia";
import { computed, inject, ref } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { noteLinks, resolveNote, searchNotes } from "../internal/vault.mjs";
import { VAULT_PORTS_KEY, type VaultPorts } from "../ports";

// 领域层只经 inject(VAULT_PORTS_KEY) 取得 Host 提供的原生契约；不直接 import src/bridge.ts。
// 缺失则 fail-fast（见 createVaultCapability assertVaultPorts + 此处守卫）。
// 注意：inject 必须在 setup 上下文内调用；模块顶层无组件实例，inject 恒为 undefined，故守卫移入 store setup。
export const useVaultStore = defineStore("vault", () => {
  const injectedPorts = inject(VAULT_PORTS_KEY);
  if (!injectedPorts) {
    throw new Error("[vault] VAULT_PORTS_KEY 未提供 —— M2 边界违反：Host 必须 app.provide(VAULT_PORTS_KEY, ports)");
  }
  const ports: VaultPorts = injectedPorts;
  const path = ref("");
  const root = ref("");
  const notes = ref<{ path: string; text: string }[]>([]);
  const selected = ref("");
  const query = ref("");
  const line = ref(1);
  const anchor = ref("");
  const busy = ref(false);
  const error = ref("");
  const warning = ref("");
  const choices = ref<string[]>([]);
  const sourceMode = ref(false);
  const vaultCurrent = computed(() => notes.value.find((n) => n.path === selected.value));
  const vaultResults = computed(() => searchNotes(notes.value, query.value));
  const vaultEdges = computed(() => {
    const paths = notes.value.map((n) => n.path);
    const result: { from: string; to: string }[] = [];
    for (const note of notes.value)
      for (const target of noteLinks(note.text)) {
        const resolved = resolveNote(target, note.path, paths);
        if (resolved.length === 1) result.push({ from: note.path, to: resolved[0] });
        if (result.length >= 5000) return result;
      }
    return result;
  });
  const vaultBacklinks = computed(() =>
    [...new Set(vaultEdges.value.filter((e) => e.to === selected.value).map((e) => e.from))],
  );
  let generation = 0;
  async function open() {
    const request = ++generation;
    busy.value = true;
    error.value = "";
    try {
      const snapshot = await ports.native.openVault(path.value);
      if (generation !== request) return;
      root.value = snapshot.root;
      notes.value = snapshot.notes;
      if (!notes.value.some((n) => n.path === selected.value)) selected.value = notes.value[0]?.path || "";
      choices.value = [];
      line.value = 1;
      anchor.value = "";
      warning.value = `${snapshot.truncated ? "已达到 1000 文件 / 16 MiB 上限。" : ""}${
        snapshot.skipped ? `跳过 ${snapshot.skipped} 个不可读、过大或符号链接文件。` : ""
      }`;
    } catch {
      if (generation === request) error.value = "无法打开 Vault，请检查目录位置和读取权限。";
    } finally {
      if (generation === request) busy.value = false;
    }
  }
  async function pickDirectory() {
    error.value = "";
    try {
      const picked = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: path.value.trim() || undefined,
        title: "选择 Obsidian Vault 目录",
      });
      if (typeof picked === "string") {
        path.value = picked;
        await open();
      }
    } catch {
      error.value = "无法打开目录选择器，请检查客户端权限。";
    }
  }
  function select(path: string, row = 1) {
    selected.value = path;
    line.value = row;
    anchor.value = "";
    choices.value = [];
    error.value = "";
    if (row > 1) sourceMode.value = true;
  }
  function follow(target: string) {
    const matches = resolveNote(target, selected.value, notes.value.map((n) => n.path));
    if (matches.length === 1) {
      select(matches[0]);
      try {
        anchor.value = decodeURIComponent(target.split("#")[1] || "");
      } catch {}
    } else if (matches.length > 1) choices.value = matches;
    else error.value = "链接目标不存在或不在本 Vault 内。";
  }
  return {
    path,
    root,
    notes,
    selected,
    query,
    line,
    anchor,
    busy,
    error,
    warning,
    choices,
    sourceMode,
    vaultCurrent,
    vaultResults,
    vaultEdges,
    vaultBacklinks,
    open,
    pickDirectory,
    select,
    follow,
  };
});
