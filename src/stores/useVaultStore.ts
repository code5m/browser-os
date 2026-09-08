import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { bridge } from '../bridge';
import { noteLinks, resolveNote, searchNotes } from '../utils/vault.mjs';
export const useVaultStore = defineStore('vault', () => {
  const path = ref('');
  const root = ref('');
  const notes = ref<{ path:string; text:string }[]>([]);
  const selected = ref('');
  const query = ref('');
  const line = ref(1);
  const anchor = ref('');
  const busy = ref(false);
  const error = ref('');
  const warning = ref('');
  const choices = ref<string[]>([]);
  const sourceMode = ref(false);
  const current = computed(() => notes.value.find(n => n.path === selected.value));
  const results = computed(() => searchNotes(notes.value, query.value));
  const edges = computed(() => {
    const paths = notes.value.map(n => n.path);
    const result: {from:string; to:string}[] = [];
    for (const note of notes.value) for (const target of noteLinks(note.text)) {
      const resolved = resolveNote(target, note.path, paths);
      if (resolved.length === 1) result.push({from:note.path,to:resolved[0]});
      if (result.length >= 5000) return result;
    }
    return result;
  });
  const backlinks = computed(() => [...new Set(edges.value.filter(e => e.to === selected.value).map(e => e.from))]);
  let generation = 0;
  async function open() {
    const request = ++generation;
    busy.value = true; error.value = '';
    try {
      const snapshot = await bridge.vaultOpen(path.value);
      if (generation !== request) return;
      root.value = snapshot.root; notes.value = snapshot.notes;
      if (!notes.value.some(n => n.path === selected.value)) selected.value = notes.value[0]?.path || '';
      choices.value = []; line.value = 1; anchor.value = '';
      warning.value = `${snapshot.truncated ? '已达到 1000 文件 / 16 MiB 上限。' : ''}${snapshot.skipped ? `跳过 ${snapshot.skipped} 个不可读、过大或符号链接文件。` : ''}`;
    } catch { if (generation === request) error.value = '无法打开 Vault，请检查目录位置和读取权限。'; }
    finally { if (generation === request) busy.value = false; }
  }
  function select(path: string, row = 1) { selected.value = path; line.value = row; anchor.value = ''; choices.value = []; error.value = ''; if (row > 1) sourceMode.value = true; }
  function follow(target: string) {
    const matches = resolveNote(target, selected.value, notes.value.map(n => n.path));
    if (matches.length === 1) { select(matches[0]); try { anchor.value = decodeURIComponent(target.split('#')[1] || ''); } catch {} }
    else if (matches.length > 1) choices.value = matches;
    else error.value = '链接目标不存在或不在本 Vault 内。';
  }
  return { path, root, notes, selected, query, line, anchor, busy, error, warning, choices, sourceMode, current, results, edges, backlinks, open, select, follow };
});
