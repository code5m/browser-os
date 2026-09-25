import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import TurndownService from 'turndown';
import DOMPurify from 'dompurify';
import { bridge } from '../../../bridge';
import { useBrowserStore } from "../../../capabilities/browser/public";

type Reply = { index:number; label:string; selected:boolean; markdown:string; state:'pending'|'reading'|'ready'|'failed'; error:string; saved:string };
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
export const useGridArchiveStore = defineStore('gridArchive', () => {
  const rows = ref<Reply[]>([]);
  const busy = ref(false);
  const saving = ref(false);
  const expanded = ref(false);
  const mode = ref<'latest'|'all'>('latest');
  const path = ref('');
  const tags = ref('AI');
  const message = ref('');
  const success = computed(() => rows.value.filter(r => r.state === 'ready' && r.selected));
  const failed = computed(() => rows.value.filter(r => r.state === 'failed'));
  const converter = new TurndownService({ codeBlockStyle:'fenced', headingStyle:'atx', bulletListMarker:'-' });
  converter.remove(['script','style','button','img','iframe']);
  async function extract(retry = false) {
    if (busy.value || saving.value) return;
    const browser = useBrowserStore();
    expanded.value = true; busy.value = true; message.value = '';
    const session = browser.gridSession;
    const requestedMode = mode.value;
    if (!retry) rows.value = Array.from({length:Math.min(browser.gridCount,12)}, (_, index) => ({ index, label:`A${index+1}`, selected:true, markdown:'', state:'pending', error:'', saved:'' }));
    const queue = rows.value.filter(r => !retry || r.state === 'failed');
    await Promise.all(Array.from({length:Math.min(3,queue.length)}, async () => {
      while (queue.length) {
        const row = queue.shift()!; row.state = 'reading'; row.error = ''; row.saved = '';
        try {
          const data = await bridge.gridReadReplies(row.index);
          if (browser.gridSession !== session || !browser.gridOpen) throw new Error('GRID_NOT_OPEN');
          if (data.error) throw new Error(data.error);
          if (!Array.isArray(data.replies) || !data.replies.length) throw new Error('NO_ASSISTANT_REPLY');
          const replies = requestedMode === 'latest' ? data.replies.slice(-1) : data.replies;
          row.markdown = replies.map(html => converter.turndown(DOMPurify.sanitize(html, { FORBID_TAGS:['img','iframe','object','form'] }))).join('\n\n---\n\n');
          // Preserve reply text in full; error redaction truncates to 300 chars and is not a content converter.
          row.markdown = redactReplyUrls(row.markdown);
          row.label = `A${row.index+1}-${data.provider || 'AI'}`;
          if (!row.markdown.trim()) throw new Error('NO_ASSISTANT_REPLY');
          row.state = 'ready'; row.error = data.truncated ? '已截断，当前结果可能不完整' : '';
        } catch(e) { row.state = 'failed'; row.error = failures[e instanceof Error ? e.message : String(e)] || '提取失败或超时，请重试'; }
      }
    }));
    busy.value = false;
  }
  async function copy() {
    if (!success.value.length) return;
    try { await navigator.clipboard.writeText(success.value.map(r => `# ${r.label}\n\n${r.markdown}`).join('\n\n---\n\n')); message.value = `已复制 ${success.value.length} 个窗口`; }
    catch { message.value = '剪贴板不可用，请使用分别保存'; }
  }
  async function save() {
    if (saving.value || busy.value || !path.value.trim()) return;
    const batch = success.value.filter(r => !r.saved);
    if (!batch.length) { message.value = '没有待保存回复'; return; }
    saving.value = true; message.value = '';
    try {
      const result = await bridge.archiveReplies(path.value, batch.map(r => ({label:r.label,markdown:r.markdown})), tags.value.split(/[，,\s]+/).filter(Boolean));
      for (let i=0; i<batch.length; i++) { batch[i].saved = result[i]?.path || ''; batch[i].error = result[i]?.error ? '保存失败，可重试未保存项' : batch[i].error; }
      message.value = `本次保存 ${result.filter(r => r.path).length} / ${batch.length} 个文件`;
    } catch { message.value = '保存失败，请检查目录权限和标签数量（最多 12 个）'; }
    finally { saving.value = false; }
  }
  return { rows, busy, saving, expanded, mode, path, tags, message, success, failed, extract, copy, save };
});
