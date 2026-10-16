<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useDatabaseStore } from '../state/useDatabaseStore';
import { tableChanges, dirtyTable, importTableRows, exportTableSql } from '../model/studioTable.mjs';
import { exportRows } from '../model/studioSql.mjs';
const db = useDatabaseStore();
const table = computed(() => db.document.tableData);
const config = computed(() => db.configs.find(c => c.id === db.document.connId));
const changes = computed(() => { try { return table.value ? tableChanges(table.value) : []; } catch { return []; } });
const canEdit = computed(() => !!table.value?.page?.editable && db.document.transaction === 'idle' && !db.busy && !table.value?.ticket);
const blocked = computed(() => db.busy || !!table.value?.ticket || db.document.closing);
const filterColumn = ref(''), filterOperator = ref('eq'), filterValue = ref(''), exportFormat = ref('csv'), acknowledged = ref(false);
const fileInput = ref<HTMLInputElement|null>(null);
const shownRows = computed(() => table.value?.page?.rows.slice((table.value.displayPage || 0) * 50, ((table.value.displayPage || 0) + 1) * 50) || []);
function request(current:any, extra:Record<string,unknown>={}) { return { name:current.name, offset:current.offset, filters:current.filters, order:current.order, ...extra }; }
async function load(offset = table.value?.offset || 0, discard = false) {
  const current = table.value, document = db.document;
  if (!current || db.busy || current.ticket || (!discard && dirtyTable(current))) return;
  const reply = await db.tableOperation('table_read', request(current, { offset }), false, document.id);
  if (!reply) return;
  current.page = reply; current.offset = offset; current.displayPage = 0;
  current.edits = {}; current.inserts = []; current.removed = []; current.cell = null;
  if (!filterColumn.value) filterColumn.value = reply.table.columns[0]?.name || '';
}
function value(row:any, index:number) {
  const name = table.value.page.table.columns[index].name;
  const draft = table.value.edits[row.row_id];
  return draft && Object.prototype.hasOwnProperty.call(draft,name) ? draft[name] : row.values[index];
}
function edit(row:any, index:number, insert = false) {
  if (!canEdit.value) return;
  const current = table.value, column = current.page.table.columns[index];
  if (column.generated || column.binary) return;
  const original = insert ? row.values[column.name] : value(row,index);
  current.cell = { id:insert ? row.id : row.row_id, column:column.name, insert };
  current.cellNull = original === null; current.cellText = original == null ? '' : original;
}
function saveCell() {
  const current=table.value, cell=current?.cell;
  if (!cell || !canEdit.value) return;
  const text=current.cellNull ? null : current.cellText;
  if (text !== null && (new TextEncoder().encode(text).length > 65536 || text.includes('\0'))) { current.notice='单元格超过 64 KiB 或含二进制内容，未修改草稿。'; return; }
  if (cell.insert) {
    const row=current.inserts.find((row:any)=>row.id===cell.id);if(row)row.values[cell.column]=text;
  } else {
    current.edits[cell.id] ||= {};current.edits[cell.id][cell.column]=text;
  }
  current.cell=null;current.notice='修改只在本页草稿中，尚未写入数据库。';
}
function addRow() {
  if (!canEdit.value || changes.value.length>=100) return;
  table.value.inserts.push({id:crypto.randomUUID(),values:{}});
  table.value.notice='新行未填写的字段使用数据库默认值；空字符串与 NULL 分别处理。';
}
function removeRow(row:any) {
  if(!canEdit.value)return;
  const rows=table.value.removed;
  table.value.removed=rows.includes(row.row_id)?rows.filter((id:string)=>id!==row.row_id):[...rows,row.row_id];
}
function revert() {
  if(blocked.value)return;
  const current=table.value;current.edits={};current.inserts=[];current.removed=[];current.cell=null;current.notice='已撤销本页未应用的草稿；没有修改数据库。';
}
async function preview() {
  const current=table.value, doc=db.document;
  if(!canEdit.value||current.cell)return;
  try {
    const edits=tableChanges(current);if(!edits.length)return;
    const reply=await db.tableOperation('table_preview',request(current,{changes:edits}),false,doc.id);
    if(reply){current.ticket=reply;acknowledged.value=false;}
  } catch(e) {current.notice=String(e);}
}
async function cancelPreview() {
  const current=table.value, doc=db.document;
  if(!current?.ticket||db.busy)return;
  const reply=await db.tableOperation('table_cancel',request(current,{ticket:current.ticket.ticket}),false,doc.id);
  if(reply){current.ticket=null;acknowledged.value=false;}
}
async function apply() {
  const current=table.value, doc=db.document;
  if(!current?.ticket||!acknowledged.value||db.busy)return;
  const ticket=current.ticket.ticket;
  current.ticket=null;acknowledged.value=false;
  const result=await db.tableOperation('table_apply',request(current,{ticket}),true,doc.id);
  if(!result){current.notice='本次应用没有得到成功回执。请查看错误和事务状态，不要盲目重试。';return;}
  current.edits={};current.inserts=[];current.removed=[];current.cell=null;
  current.awaitingCommit=true;current.notice=`已在当前事务中应用 ${result.applied} 项改动。还未提交；请使用上方“提交事务”或“回滚事务”。`;
  await load(current.offset,true);
}
async function addFilter() {
  if(!table.value||blocked.value||dirtyTable(table.value)||table.value.filters.length>=16||!filterColumn.value)return;
  table.value.filters.push({column:filterColumn.value,operator:filterOperator.value,value:['is_null','not_null'].includes(filterOperator.value)?null:filterValue.value});
  await load(0);
}
async function removeFilter(index:number) {
  if(blocked.value||dirtyTable(table.value))return;
  table.value.filters.splice(index,1);await load(0);
}
async function sort(name:string) {
  if(blocked.value||dirtyTable(table.value))return;
  const old=table.value.order.find((x:any)=>x.column===name);
  table.value.order=[{column:name,descending:old?!old.descending:false}];await load(0);
}
async function imported(event:Event) {
  const input=event.target as HTMLInputElement,file=input.files?.[0];input.value='';
  const current=table.value,docId=db.activeDocument;
  if(!file||!canEdit.value||!current)return;
  try {
    if(file.size>5*1024*1024)throw new Error('文件超过 5 MiB');
    const format=file.name.toLowerCase().endsWith('.json')?'json':file.name.toLowerCase().endsWith('.tsv')?'tsv':'csv';
    const rows=importTableRows(await file.text(),format,current.page.table.columns);
    if(db.activeDocument!==docId||table.value!==current)throw new Error('页签已变化，请在原页重新选择文件');
    if(rows.length+changes.value.length>100)throw new Error('每批最多 100 项改动，请减少导入行数');
    current.inserts.push(...rows);current.notice=`已载入 ${rows.length} 行导入草稿。没有执行 SQL，请预览并核对。`;
  }catch(e){current.notice='导入未执行：'+String(e);}
}
function exportData() {
  const current=table.value;if(!current?.page)return;
  try {
    const columns=current.page.table.columns.map((c:any)=>c.name),rows=current.page.rows.map((r:any)=>r.values);
    const text=exportFormat.value==='sql'?exportTableSql(current.name,columns,rows,config.value?.kind):exportRows(columns,rows,exportFormat.value);
    const url=URL.createObjectURL(new Blob([text],{type:'text/plain;charset=utf-8'}));
    const link=document.createElement('a');link.href=url;link.download=`browseros-table-${new Date().toISOString().slice(0,10)}.${exportFormat.value}`;link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
    current.notice='仅导出当前服务端批次已读取的原始行，不包含未应用的草稿。';
  }catch(e){current.notice='导出失败：'+String(e);}
}
watch(()=>db.activeDocument,()=>{filterColumn.value='';acknowledged.value=false;if(table.value&&!table.value.page)void load();},{immediate:true});
watch(()=>[db.document.transaction,db.document.busy],()=>{const current=table.value;if(current?.awaitingCommit&&db.document.transaction==='idle'&&!db.busy){current.awaitingCommit=false;current.notice='事务已结束，正在重新读取数据库结果。';void load(current.offset,true);}});
</script>
<template>
<section v-if="table" class="table-data-editor" aria-label="表数据编辑器">
  <header class="tools"><strong>{{table.name}} · 表数据</strong><span>改动 {{changes.length}} 项</span><button :disabled="blocked||dirtyTable(table)" @click="load()">重新读取</button><button :disabled="!canEdit||changes.length>=100" @click="addRow">新增行</button><button :disabled="blocked||!dirtyTable(table)" @click="revert">撤销本页草稿</button><button :disabled="!canEdit||!changes.length||table.cell" @click="preview">预览参数化 DML…</button></header>
  <p v-if="!table.page">{{db.busy?'正在读取表结构与数据…':'尚未读取数据；请检查数据源及下方错误。'}}</p>
  <template v-else>
    <p class="hint">所有筛选与排序均重新请求数据库；主键自动追加为稳定排序条件。每批最多 100 行，未读取的数据不会冒充已导出。</p>
    <p v-if="!table.page.stable_order" class="warning">此对象没有可验证的主键，翻页顺序可能随并发修改变化；只能查看，不能编辑。</p>
    <p v-if="!table.page.editable" class="warning">当前只读：写入要求明确的非生产权限、主键、事务表以及可安全比较的文本数据。视图、二进制列或非事务表不会被强行编辑。</p>
    <div class="tools"><select v-model="filterColumn" :disabled="blocked||dirtyTable(table)" aria-label="筛选字段"><option v-for="column in table.page.table.columns" :key="column.name" :value="column.name">{{column.name}}</option></select><select v-model="filterOperator" :disabled="blocked||dirtyTable(table)" aria-label="筛选条件"><option value="eq">等于</option><option value="ne">不等于</option><option value="contains">包含</option><option value="gt">大于</option><option value="ge">大于等于</option><option value="lt">小于</option><option value="le">小于等于</option><option value="is_null">是 NULL</option><option value="not_null">不是 NULL</option></select><input v-if="!['is_null','not_null'].includes(filterOperator)" v-model="filterValue" :disabled="blocked||dirtyTable(table)" placeholder="参数值，不拼接到 SQL" aria-label="筛选值"/><button :disabled="blocked||dirtyTable(table)||table.filters.length>=16" @click="addFilter">应用筛选</button><span v-for="(filter,index) in table.filters" :key="index">{{filter.column}} {{filter.operator}} {{filter.value}}<button :disabled="blocked||dirtyTable(table)" :aria-label="'移除筛选 '+filter.column" @click="removeFilter(index)">×</button></span></div>
    <div class="grid"><table><thead><tr><th>操作</th><th v-for="column in table.page.table.columns" :key="column.name"><button :disabled="blocked||dirtyTable(table)||column.binary" :title="column.datatype" @click="sort(column.name)">{{column.primary?'🔑 ':''}}{{column.name}}{{table.order[0]?.column===column.name?(table.order[0].descending?' ↓':' ↑'):''}}</button></th></tr></thead><tbody>
      <tr v-for="row in shownRows" :key="row.row_id" :class="{removed:table.removed.includes(row.row_id)}"><td><button :disabled="!canEdit" @click="removeRow(row)">{{table.removed.includes(row.row_id)?'恢复':'删除'}}</button></td><td v-for="(column,index) in table.page.table.columns" :key="column.name" :class="{changed:table.edits[row.row_id]&&Object.prototype.hasOwnProperty.call(table.edits[row.row_id],column.name)}"><button class="cell" :disabled="!canEdit||column.generated||column.binary||table.removed.includes(row.row_id)" :title="String(value(row,index)??'NULL').slice(0,1000)" @click="edit(row,index)">{{value(row,index)===null?'NULL':String(value(row,index)).slice(0,240)}}</button></td></tr>
      <tr v-for="row in table.inserts" :key="row.id" class="inserted"><td><button :disabled="!canEdit" @click="table.inserts=table.inserts.filter((item:any)=>item.id!==row.id)">移除新行</button></td><td v-for="(column,index) in table.page.table.columns" :key="column.name"><button class="cell" :disabled="!canEdit||column.generated||column.binary" @click="edit(row,index,true)">{{!Object.prototype.hasOwnProperty.call(row.values,column.name)?'DEFAULT':row.values[column.name]===null?'NULL':String(row.values[column.name]).slice(0,240)}}</button></td></tr>
    </tbody></table></div>
    <section v-if="table.cell" class="cell-editor"><strong>编辑 {{table.cell.column}} · 只修改草稿</strong><label><input v-model="table.cellNull" type="checkbox"/>设为 NULL（不同于空字符串）</label><textarea v-model="table.cellText" :disabled="table.cellNull" rows="4" aria-label="单元格值"/><button :disabled="!canEdit" @click="saveCell">保存到草稿</button><button @click="table.cell=null">不修改</button></section>
    <section v-if="table.ticket" class="confirmation" aria-label="DML 执行预览"><strong>请核对数据源、SQL 和绑定参数</strong><p>每行会重新锁定并比对原值。任一冲突导致整批回滚；应用成功后仍需单独提交事务。</p><div v-for="(statement,index) in table.ticket.statements" :key="index"><code>{{statement.sql}}</code><details><summary>{{statement.parameters.length}} 个绑定参数</summary><pre>{{JSON.stringify(statement.parameters,null,2)}}</pre></details></div><label><input v-model="acknowledged" type="checkbox"/>我已核对 {{config?.name}} / {{table.name}} 的 {{table.ticket.statements.length}} 项操作</label><button :disabled="!acknowledged||db.busy" @click="apply">应用到事务（不自动提交）</button><button :disabled="db.busy" @click="cancelPreview">取消预览</button></section>
    <footer class="tools"><button :disabled="blocked||dirtyTable(table)||table.offset===0" @click="load(table.offset-100)">上一批</button><span>服务端第 {{table.offset/100+1}} 批 · {{table.page.rows.length}} 行</span><button :disabled="blocked||dirtyTable(table)||!table.page.has_more||table.offset>=50000" @click="load(table.offset+100)">下一批</button><button :disabled="!(table.displayPage||0)" @click="table.displayPage=0">显示前 50 行</button><button :disabled="table.page.rows.length<=50" @click="table.displayPage=1">显示后 50 行</button></footer>
    <div class="tools"><input ref="fileInput" type="file" accept=".csv,.tsv,.json" class="file-input" @change="imported"/><button :disabled="!canEdit" @click="fileInput?.click()">导入为新增草稿…</button><select v-model="exportFormat" aria-label="表数据导出格式"><option value="csv">CSV</option><option value="tsv">TSV</option><option value="json">JSON</option><option value="sql">SQL INSERT</option></select><button :disabled="db.busy" @click="exportData">导出当前批次</button><small>导入最多 100 行；CSV 空字段保持空字符串，JSON null 保持 NULL。</small></div>
  </template>
  <p v-if="table.notice" role="status">{{table.notice}}</p>
</section>
</template>
<style scoped>
.table-data-editor{display:flex;flex-direction:column;min-width:0;gap:8px;color:inherit}.tools{display:flex;align-items:center;flex-wrap:wrap;gap:6px}.tools strong{margin-right:auto}.grid{overflow:auto;max-height:460px;border:1px solid #d6e0eb}table{border-collapse:collapse;min-width:100%;font:12px/1.5 ui-monospace,monospace}th,td{border:1px solid #e0e7ef;padding:3px;max-width:360px;white-space:pre}th{position:sticky;top:0;background:#f1f5fa;text-align:left;z-index:1}.cell{min-width:50px;max-width:350px;overflow:hidden;text-overflow:ellipsis;text-align:left;width:100%;background:transparent;border:0}.cell:disabled{opacity:1;color:inherit;cursor:default}.changed,.inserted{background:#f1faed}.removed{background:#fff0ef;text-decoration:line-through}.hint,small{font-size:11px;color:#627689}.warning{padding:7px;background:#fff7e3;color:#78591e}.confirmation,.cell-editor{border:1px solid #cbd8e6;border-radius:5px;padding:10px;background:#f6f9fc}.confirmation pre{max-height:160px;overflow:auto}.confirmation code{white-space:pre-wrap;overflow-wrap:anywhere}.cell-editor textarea{box-sizing:border-box;width:100%;font:12px/1.6 ui-monospace,monospace}.cell-editor label,.confirmation label{display:block;margin:7px 0}button,input,select,textarea{font:inherit;border:1px solid #c5d3e2;border-radius:4px;padding:5px 7px;color:inherit;background:#fff}button{cursor:pointer}button:disabled{opacity:.45;cursor:not-allowed}.file-input{display:none}p{margin:3px 0;overflow-wrap:anywhere}
</style>
