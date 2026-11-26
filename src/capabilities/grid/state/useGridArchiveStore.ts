import { defineStore } from 'pinia';
import { computed, ref, shallowRef, watch } from 'vue';
import { bridge, type GridReplySource } from '../../../bridge';
import { useLayoutStore } from '../../../stores/useLayoutStore';
import { isGridResourceAllowed } from '../resource/guard';
import { htmlToMarkdown } from '../../../utils/htmlToMarkdown';
import { useGridStore } from "./useGridStore";

type Reply = { index:number; label:string; selected:boolean; markdown:string; state:'pending'|'reading'|'ready'|'failed'; error:string; saved:string; retryable:boolean; saveUnknown:boolean };
type Batch = { revision:number; count:number; mode:'latest'|'all'; sources:(GridReplySource|null)[] };
const failures: Record<string,string> = { UNSUPPORTED_SITE:'此站点暂无回复适配器', REPLY_STREAMING:'回复还在生成，请稍后重试', NO_ASSISTANT_REPLY:'未找到 AI 回复', GRID_NOT_OPEN:'窗口已关闭', GRID_RESULT_LIMIT:'回复超过容量上限' };
function redactReplyUrls(markdown:string): string {
  return markdown.replace(/https?:\/\/[^\s<>"'\x60()]+/g, raw => {
    let urlText = raw;
    let suffix = '';
    while (/[.,!?;:]$/.test(urlText)) suffix = urlText.slice(-1) + suffix, urlText = urlText.slice(0, -1);
    try {
      const url = new URL(urlText); url.username = ''; url.password = ''; url.hash = '';
      for (const key of [...url.searchParams.keys()]) if (/token|password|secret|signature|api.?key|authorization|credential/i.test(key)) url.searchParams.set(key, 'REDACTED');
      return url.toString() + suffix;
    } catch { return raw; }
  });
}
function validSource(value: GridReplySource | null, index:number): boolean {
  if (value === null) return true;
  const epoch = (text:unknown, zero = false) => typeof text === 'string'
    && (zero ? /^(0|[1-9][0-9]{0,19})$/ : /^[1-9][0-9]{0,19}$/).test(text)
    && (text.length < 20 || text <= '18446744073709551615');
  return !!value && value.index === index && Number.isInteger(value.pid)
    && value.pid > 0 && value.pid <= 0xffffffff && epoch(value.spawn_epoch)
    && epoch(value.generation) && epoch(value.navigation_revision, true);
}
export const useGridArchiveStore = defineStore('gridArchive', () => {
  const grid = useGridStore();
  const layout = useLayoutStore();
  const rows = ref<Reply[]>([]);
  const busy = ref(false);
  const saving = ref(false);
  const expanded = ref(false);
  const mode = ref<'latest'|'all'>('latest');
  const path = ref('');
  const tags = ref('AI');
  const message = ref('');
  const saveNotice = ref('');
  const activeBatch = shallowRef<Batch|null>(null);
  const canExtract = computed(() => grid.gridOpen && !grid.gridBusy
    && !grid.gridCleanupUnconfirmed && !grid.gridLayoutSuspended
    && layout.mainView === 'grid' && isGridResourceAllowed());
  function current(batch:Batch) {
    return activeBatch.value === batch && canExtract.value
      && batch.revision === grid.gridArchiveRevision && batch.count === grid.gridCount
      && batch.mode === mode.value;
  }
  function invalidate() {
    const hadBatch = activeBatch.value !== null || rows.value.length > 0;
    activeBatch.value = null; rows.value = [];
    if (hadBatch) message.value = '宫格来源或提取范围已变化，请重新提取。旧结果不会进入新批次。';
  }
  watch(() => [grid.gridArchiveRevision, grid.gridCount, grid.gridOpen, grid.gridCleanupUnconfirmed,
    grid.gridLayoutSuspended, layout.mainView, mode.value] as const, invalidate, {flush:'sync'});
  const success = computed(() => activeBatch.value && current(activeBatch.value)
    ? rows.value.filter(r => r.state === 'ready' && r.selected) : []);
  const failed = computed(() => activeBatch.value && current(activeBatch.value)
    ? rows.value.filter(r => r.state === 'failed') : []);
  const retryableFailed = computed(() => failed.value.filter(r => r.retryable));
  const saveable = computed(() => success.value.filter(r => !r.saved && !r.saveUnknown));

  async function extract(retry = false) {
    if (busy.value || saving.value || !canExtract.value) return;
    const batch:Batch|null = retry ? activeBatch.value
      : {revision:grid.gridArchiveRevision, count:Math.min(grid.gridCount,12), mode:mode.value, sources:[]};
    if (!batch || (retry && !current(batch))) return;
    expanded.value = true; busy.value = true; message.value = '';
    try {
      if (!retry) {
        activeBatch.value = batch;
        rows.value = [];
        const snapshot = await bridge.gridReplySources(batch.count);
        if (!current(batch)) return;
        if (!snapshot || !Array.isArray(snapshot.sources) || snapshot.sources.length !== batch.count
            || !snapshot.sources.every(validSource)) throw new Error('GRID_REPLY_SNAPSHOT_INVALID');
        batch.sources = snapshot.sources;
        rows.value = Array.from({length:batch.count}, (_, index) => ({
          index, label:`A${index+1}`, selected:true, markdown:'', state:'pending', error:'',
          saved:'', retryable:batch.sources[index] !== null, saveUnknown:false,
        }));
      }
      const queue = rows.value.filter(r => !retry || (r.state === 'failed' && r.retryable));
      await Promise.all(Array.from({length:Math.min(3,queue.length)}, async () => {
        while (queue.length && current(batch)) {
          const row = queue.shift()!;
          const source = batch.sources[row.index];
          row.state = 'reading'; row.error = ''; row.saved = ''; row.markdown = '';
          if (!source) {
            row.state = 'failed'; row.retryable = false;
            row.error = '原宫格身份未确认，请重新提取整个批次。';
            continue;
          }
          try {
            const data = await bridge.gridReadReplies(row.index, source);
            if (!current(batch)) return;
            if (data.error) throw new Error(data.error);
            if (!Array.isArray(data.replies) || !data.replies.length
                || data.replies.some(value => typeof value !== 'string')) throw new Error('NO_ASSISTANT_REPLY');
            const replies = batch.mode === 'latest' ? data.replies.slice(-1) : data.replies;
            row.markdown = redactReplyUrls(replies.map(htmlToMarkdown).join('\n\n---\n\n'));
            row.label = `A${row.index+1}-${typeof data.provider === 'string' ? data.provider.slice(0,80) : 'AI'}`;
            if (!row.markdown.trim()) throw new Error('NO_ASSISTANT_REPLY');
            row.state = 'ready'; row.error = data.truncated ? '已截断，当前结果可能不完整' : '';
          } catch(error) {
            if (!current(batch)) return;
            const reason = error instanceof Error ? error.message : String(error);
            row.state = 'failed'; row.markdown = '';
            if (/GRID_READ_TARGET_STALE|GRID_EVAL_TARGET_STALE|GRID_REQUEST_DOCUMENT_STALE|GRID_NATIVE_DOCUMENT_STALE/.test(reason)) {
              row.retryable = false;
              row.error = '原始文档已变化，请重新提取整个批次。';
            } else row.error = failures[reason] || '提取失败或超时，可重试原始来源';
          }
        }
      }));
    } catch {
      if (current(batch)) { rows.value = []; activeBatch.value = null; message.value = '来源快照未确认，请重新提取。'; }
    } finally { busy.value = false; }
  }

  async function copy() {
    const batch = activeBatch.value;
    if (!batch || !current(batch) || busy.value || saving.value || !success.value.length) return;
    const selected = success.value.slice();
    try {
      await navigator.clipboard.writeText(selected.map(r => `# ${r.label}\n\n${r.markdown}`).join('\n\n---\n\n'));
      if (current(batch)) message.value = `已复制 ${selected.length} 个窗口`;
    } catch { if (current(batch)) message.value = '剪贴板不可用，请使用分别保存'; }
  }
  async function save() {
    const batch = activeBatch.value;
    if (!batch || !current(batch) || saving.value || busy.value || !path.value.trim()) return;
    const selected = saveable.value.slice();
    if (!selected.length) { message.value = '没有待保存的已确认回复'; return; }
    const items = selected.map(r => ({label:r.label,markdown:r.markdown}));
    const directory = path.value;
    const tagList = tags.value.split(/[，,\s]+/).filter(Boolean);
    saving.value = true; message.value = '';
    try {
      const result = await bridge.archiveReplies(directory, items, tagList);
      if (!Array.isArray(result) || result.length !== items.length || result.some((r,i) =>
          !r || r.label !== items[i].label ||
          (typeof r.path === 'string' && r.path.length > 0) === (typeof r.error === 'string' && r.error.length > 0))) {
        throw new Error('ARCHIVE_RECEIPT_INVALID');
      }
      const confirmed = result.filter(r => typeof r.path === 'string' && r.path.length > 0).length;
      const uncertain = result.some(r => r.error === 'ARCHIVE_WRITE_FAILED');
      saveNotice.value = `最近一次保存（${directory}）：确认 ${confirmed} / ${items.length} 个文件。` +
        (uncertain ? ' 部分写入结果未知，请先检查目录，不自动重试。' : '');
      if (!current(batch)) return;
      for (let i=0; i<selected.length; i++) {
        selected[i].saved = result[i].path || '';
        selected[i].saveUnknown = result[i].error === 'ARCHIVE_WRITE_FAILED';
        if (selected[i].saveUnknown) selected[i].error = '保存结果未知，请检查目录';
        else if (result[i].error) selected[i].error = '未通过保存校验';
      }
    } catch {
      saveNotice.value = `最近一次保存（${directory}）未收到可靠回执；可能已生成文件，请先核对目录，不自动重试。`;
      if (current(batch)) for (const row of selected) { row.saveUnknown = true; row.error = '保存结果未知，请检查目录'; }
    } finally { saving.value = false; }
  }
  return { rows, busy, saving, expanded, mode, path, tags, message, saveNotice, canExtract,
    success, failed, retryableFailed, saveable, extract, copy, save };
});
