/** Web workspaces, not API models. Login, model choice and availability belong to each provider. */
export const AI_PROVIDERS = [
  ['doubao','豆包','https://www.doubao.com/chat/'],
  ['deepseek','DeepSeek','https://chat.deepseek.com/'],
  ['kimi','Kimi','https://www.kimi.com/'],
  ['qwen','Qwen / 千问','https://chat.qwen.ai/'],
  ['yuanbao','腾讯元宝','https://yuanbao.tencent.com/'],
  ['zhipu','智谱清言','https://chatglm.cn/'],
  ['wenxin','文小言','https://yiyan.baidu.com/'],
  ['xinghuo','讯飞星火','https://xinghuo.xfyun.cn/'],
  ['gpt','ChatGPT','https://chatgpt.com/'],
  ['gemini','Gemini','https://gemini.google.com/'],
  ['grok','Grok','https://grok.com/'],
  ['perplexity','Perplexity','https://www.perplexity.ai/'],
  ['copilot','Copilot','https://copilot.microsoft.com/'],
  ['mistral','Le Chat / Mistral','https://chat.mistral.ai/'],
  ['poe','Poe','https://poe.com/'],
].map(([id,name,url])=>({id,name,url}));
const preset=(id,name,ids)=>({id,name,layout:ids.length===4?'quad':'grid',urls:ids.map(id=>AI_PROVIDERS.find(p=>p.id===id).url)});
export const BUILTIN_AI_PRESETS = [
  preset('cn-daily','国内日常',['doubao','kimi','qwen','deepseek']),
  preset('global-daily','国际通用',['gpt','gemini','grok','copilot']),
  preset('coding','代码交叉检查',['deepseek','qwen','gpt','kimi']),
  preset('research','资料检索对照',['perplexity','gemini','gpt','kimi']),
  preset('writing','中文写作对照',['doubao','kimi','yuanbao','wenxin']),
  preset('reasoning','推理答案对照',['deepseek','qwen','gpt','gemini']),
  preset('bilingual','中外双视角',['doubao','gpt']),
  preset('office','办公资料',['copilot','kimi','doubao','qwen']),
  preset('creative','创意发散',['doubao','grok','gemini','mistral']),
  preset('cn-six','国内六站',['doubao','deepseek','kimi','qwen','yuanbao','zhipu']),
  preset('global-six','国际六站',['gpt','gemini','grok','perplexity','copilot','mistral']),
  preset('compare-nine','九站广度对照（高内存）',['doubao','deepseek','kimi','qwen','gpt','gemini','grok','perplexity','poe']),
];
export function validateAiPreset(value) {
  if(!value || typeof value.id!=='string' || !/^[a-zA-Z0-9_-]{1,80}$/.test(value.id) || typeof value.name!=='string' || !value.name.trim() || value.name.length>60 || !['horizontal','quad','grid'].includes(value.layout) || !Array.isArray(value.urls) || value.urls.length<2 || value.urls.length>12 || (value.layout==='quad'&&value.urls.length!==4))throw Error('模板需要名称、2–12 个网页；四分布局必须是 4 格');
  const urls=value.urls.map(raw=>{
    if(typeof raw!=='string' || raw.length>2048)throw Error('网址无效');
    const u=new URL(raw), local=['localhost','127.0.0.1','[::1]'].includes(u.hostname);
    if(!(u.protocol==='https:'||(local&&u.protocol==='http:')) || u.username || u.password || [...u.searchParams.keys()].some(k=>/key|token|secret|password|credential|auth/i.test(k)) || u.hash || /claude/i.test(u.hostname))throw Error('仅允许无凭据的 HTTPS 网址或本机 HTTP；不包含 Claude');
    return u.href;
  });
  return {id:value.id,name:value.name.trim(),urls,layout:value.layout};
}
export function decodeAiPresets(raw) {
  try { if(typeof raw!=='string'||raw.length>131072)return [];const items=JSON.parse(raw);if(!Array.isArray(items)||items.length>32)return [];const parsed=items.map(validateAiPreset);if(new Set(parsed.map(x=>x.id)).size!==parsed.length)return [];return parsed; } catch { return []; }
}
