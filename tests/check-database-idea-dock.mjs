import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {KEY, normalizeDock, readDock, writeDock, resizedWidth, resizedOutput, markdownResult} from '../src/capabilities/database/model/ideaDock.mjs';
import {exportRows} from '../src/capabilities/database/model/studioSql.mjs';
import {projectTableRows} from '../src/capabilities/database/model/studioTable.mjs';
assert.equal(normalizeDock().side,'right');
assert.deepEqual(normalizeDock({side:'unsafe',navigator:Infinity,output:9000}),{side:'right',navigator:268,inspector:258,output:750,resultOpen:true,resultMaximized:false});
assert.equal(resizedWidth(300,100,'left',920,260),360);
assert.equal(resizedWidth(300,100,'right',920,260),200);
assert.equal(resizedOutput(260,-150,640),410);
assert.equal(resizedOutput(260,5000,640),130);
const cache = new Map();const storage={getItem:(key)=>cache.get(key),setItem:(key,value)=>cache.set(key,value)};
assert.equal(writeDock({side:'left',navigator:310,output:420,resultOpen:false},storage),true);
assert.equal(readDock(storage).navigator,310);assert.equal(readDock(storage).resultOpen,false);
cache.set(KEY,'{"navigator":1e100}');assert.equal(readDock(storage).navigator,560);
cache.set(KEY,'{bad');assert.equal(readDock(storage).side,'right');
assert.equal(readDock({getItem:()=>{throw new Error('private')}}).side,'right');
assert.equal(writeDock({}, {setItem:()=>{throw new Error('private')}}),false);
assert.match(markdownResult(['name'],[['alpha|beta']]),/alpha\\\|beta/);

const longText='x'.repeat(600);
const typedJson=JSON.parse(exportRows(['number','boolean','nullValue','longText'],[[42,true,null,longText]],'json'));
assert.equal(typedJson.rows[0][0],42,'JSON export preserves numeric type');
assert.equal(typedJson.rows[0][1],true,'JSON export preserves boolean type');
assert.equal(typedJson.rows[0][2],null,'JSON export preserves NULL');
assert.equal(typedJson.rows[0][3],longText,'JSON export preserves long text without display truncation');
const projected=projectTableRows({
  page:{table:{columns:[{name:'id'},{name:'name'}]},rows:[
    {row_id:'1',values:['1','before']},{row_id:'2',values:['2','remove']}
  ]},
  edits:{'1':{name:'draft'}},removed:['2'],inserts:[{id:'new-row',values:{name:'new'}}]
});
assert.deepEqual(projected,[
  {id:'1',key:'row:1',status:'changed',values:['1','draft']},
  {id:'2',key:'row:2',status:'removed',values:['2','remove']},
  {id:'new-row',key:'insert:new-row',status:'inserted',values:['DEFAULT','new']}
],'all table views share pending edits, deletes and inserts');

const panel=readFileSync('src/capabilities/database/ui/DatabasePanel.vue','utf8');
const workspace=readFileSync('src/capabilities/database/ui/DatabaseWorkspace.vue','utf8');
const output=readFileSync('src/capabilities/database/ui/DatabaseOutput.vue','utf8');
for(const token of ['db-nav-divider','db-inspector-divider','onPanelPointerDown','openConnectionSettings','database-navigator','db-dock-side']) assert.ok(panel.includes(token),'missing DB dock: '+token);
for(const token of ['db-output-splitter','onOutputPointerDown','ide-result-pane','resultViewMode','db-result-extractors']) assert.ok(output.includes(token),'missing result dock: '+token);
assert.match(workspace,/DatabaseOutput/);
assert.doesNotMatch(workspace,/!raw|selectResult\(index\)/,'workspace must not reference undefined result bindings');
assert.match(workspace,/db\.document\.raw/,'clear-result state must use the active document');
assert.match(output,/rawRows=computed/,'result export must start from raw typed rows');
assert.match(output,/exportRows\(v\.columns,rows\.value/,'export must use raw filtered rows');
assert.match(panel,/v-model="objectFilter"/,'object search must be present in the active source tree');
assert.doesNotMatch(panel,/class="schema-row"/,'legacy duplicate schema tree must be removed');
assert.match(panel,/database-object-details/,'selected object details remain available');
assert.match(panel,/@container \(max-width: 780px\)/,'narrow layout must use a navigator drawer');
const sfc=await import('@vue/compiler-sfc');
for(const name of ['DatabasePanel','DatabaseWorkspace','DatabaseOutput','TableDataEditor']){
  const filename='src/capabilities/database/ui/'+name+'.vue';
  const parsed=sfc.parse(readFileSync(filename,'utf8'),{filename});
  assert.deepEqual(parsed.errors,[],name+' must parse');
  const script=sfc.compileScript(parsed.descriptor,{id:'db-'+name});
  const compiled=sfc.compileTemplate({source:parsed.descriptor.template.content,filename,id:'db-'+name,compilerOptions:{bindingMetadata:script.bindings}});
  assert.deepEqual(compiled.errors,[],name+' template must compile');
}
assert.match(output,/aria-orientation="horizontal"/);
assert.match(readFileSync('src/capabilities/database/ui/TableDataEditor.vue','utf8'),/exportTableSql/);
assert.match(panel,/aria-orientation="vertical"/);
assert.match(panel,/db-dialog-sources/);
assert.match(panel,/source-children/);
assert.match(readFileSync('src/capabilities/database/ui/TableDataEditor.vue','utf8'),/table-transposed/);
console.log('DATABASE_IDEA_DOCK=PASS persisted drag bounds, source tree, result modes, safe export and Vue contracts');
