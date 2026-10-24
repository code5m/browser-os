/** Editing hints only: never executes user SQL or claims server validation. */
export function sqlScope(tokens, cursor, catalog = {}) {
  const ts=tokens.filter(t=>!['space','comment','string'].includes(t.kind));
  const word=t=>t?.text?.toUpperCase();
  const name=t=>t?.kind==='identifier'?t.text.replace(/^[`"\[]|[`"\]]$/g,'').replaceAll('""','"').replaceAll('``','`'):null;
  const pairs=new Map(), stack=[];
  ts.forEach((t,i)=>{if(t.text==='(')stack.push(i);if(t.text===')'&&stack.length)pairs.set(stack.pop(),i);});
  const physical=new Set();
  function parse(start,end,inherited=new Map(),depth=0) {
    if(depth>16)return {relations:new Map(),ctes:inherited,columns:[],children:[]};
    const ctes=new Map(inherited), relations=new Map(),children=[]; let begin=start;
    if(word(ts[begin])==='WITH') {
      begin++;if(word(ts[begin])==='RECURSIVE')begin++;
      while(begin<end&&name(ts[begin])) {
        const key=name(ts[begin++]);let explicit=[];
        if(ts[begin]?.text==='('){const close=pairs.get(begin);if(close===undefined)break;explicit=ts.slice(begin+1,close).map(name).filter(Boolean);begin=close+1;}
        if(word(ts[begin])!=='AS'||ts[begin+1]?.text!=='(')break;
        const open=begin+1,close=pairs.get(open);if(close===undefined)break;
        ctes.set(key.toLowerCase(),[]);
        const body=parse(open+1,close,ctes,depth+1);children.push({start:ts[open].start,end:ts[close].end,scope:body});
        ctes.set(key.toLowerCase(),explicit.length?explicit:body.columns);begin=close+1;
        if(ts[begin]?.text!==',')break;begin++;
      }
    }
    for(let i=begin;i<end;i++) {
      if(ts[i]?.text==='('){const close=pairs.get(i);if(close!==undefined){if(['SELECT','WITH'].includes(word(ts[i+1])))children.push({start:ts[i].start,end:ts[close].end,scope:parse(i+1,close,ctes,depth+1)});i=close;}continue;}
      if(!['FROM','JOIN','UPDATE','INTO'].includes(word(ts[i])))continue;
      let j=i+1,cols=[],key;
      if(ts[j]?.text==='('){const close=pairs.get(j);if(close===undefined)continue;const body=parse(j+1,close,ctes,depth+1);cols=body.columns;children.push({start:ts[j].start,end:ts[close].end,scope:body});j=close+1;}
      else {key=name(ts[j]);if(!key)continue;j++;while(ts[j]?.text==='.'&&name(ts[j+1])){key+='.'+name(ts[j+1]);j+=2;}cols=ctes.get(key.toLowerCase());if(!cols){physical.add(key);cols=catalog[key]??[];}}
      if(word(ts[j])==='AS')j++;
      const alias=name(ts[j]);if(alias)relations.set(alias.toLowerCase(),cols);
      if(key&&!alias)relations.set(key.toLowerCase(),cols);
      i=j-1;
    }
    let select=begin;while(select<end&&word(ts[select])!=='SELECT')select++;
    const chunks=[];let chunk=[];
    for(let i=select+1;i<end;i++){if(word(ts[i])==='FROM')break;if(ts[i].text==='('){const close=pairs.get(i);if(close!==undefined){chunk.push(...ts.slice(i,close+1));i=close;continue;}}if(ts[i].text===','){chunks.push(chunk);chunk=[];}else chunk.push(ts[i]);}chunks.push(chunk);
    const columns=[];
    for(const part of chunks){const as=part.findIndex(t=>word(t)==='AS');if(as>=0&&name(part[as+1]))columns.push(name(part[as+1]));else if(part.at(-1)?.text==='*'){if(part.length===1)columns.push(...[...relations.values()].flat());else columns.push(...(relations.get(name(part[0])?.toLowerCase())??[]));}else if(part.length===1&&name(part[0]))columns.push(name(part[0]));else if(part.length===3&&part[1].text==='.'&&name(part[2]))columns.push(name(part[2]));}
    return {relations,ctes,columns:[...new Set(columns)],children};
  }
  let start=0,end=ts.length;
  for(let i=0;i<ts.length;i++)if(ts[i].kind==='separator'){if(ts[i].end<=cursor)start=i+1;else{end=i;break;}}
  const root=parse(start,end), visible=new Map();let current=root;
  for(let depth=0;depth<17;depth++){for(const entry of current.relations)visible.set(...entry);const child=current.children.find(c=>cursor>c.start&&cursor<c.end);if(!child)break;current=child.scope;}
  return {relations:visible,ctes:current.ctes,tables:[...physical]};
}
