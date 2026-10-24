import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import {parseDatabaseConnectionUrl,parseDatabaseHostAddress} from "../src/utils/dbConnectionUrl.ts";
import {historyContextRows} from '../src/capabilities/git/model/patchView.ts';
import {bookmarkTree} from '../src/capabilities/bookmark/model/bookmarkTree.ts';
const get=(p)=>readFileSync(new URL("../"+p,import.meta.url),"utf8");
let assertions=0;
function eq(x,y){assert.deepEqual(x,y);assertions++;}
function yes(x){assert.ok(x);assertions++;}
function reject(raw) {
  assert.throws(()=>parseDatabaseConnectionUrl(raw),e=>e instanceof Error && !e.message.includes(raw));assertions++;
}
eq(parseDatabaseConnectionUrl("jdbc:mysql://localhost:3306/app"),{kind:"mysql",host:"localhost",portText:"3306",database:"app",sslMode:"prefer"});
eq(parseDatabaseConnectionUrl("mysql://127.0.0.1/app?sslMode=REQUIRED"),{kind:"mysql",host:"127.0.0.1",portText:"3306",database:"app",sslMode:"require"});
eq(parseDatabaseConnectionUrl("jdbc:sqlite:/home/user/data.db"),{kind:"sqlite",host:"",portText:"",database:"/home/user/data.db",sslMode:"disable"});
eq(parseDatabaseConnectionUrl("sqlite:///data/test.db"),{kind:"sqlite",host:"",portText:"",database:"/data/test.db",sslMode:"disable"});
eq(parseDatabaseConnectionUrl("jdbc:postgresql://db.internal:5433/work?sslmode=require"),{kind:"postgres",host:"db.internal",portText:"5433",database:"work",sslMode:"require"});
eq(parseDatabaseConnectionUrl("jdbc:mysql://phantomdev.rwlb.rds.aliyuncs.com:3306/phantom_admin_staging?useUnicode=true&characterEncoding=UTF8&&useSSL=true&serverTimezone=Asia/Shanghai&allowMultiQueries=true&rewriteBatchedStatements=true"),{
  kind:"mysql",host:"phantomdev.rwlb.rds.aliyuncs.com",portText:"3306",database:"phantom_admin_staging",sslMode:"require",
  ignoredOptions:["useUnicode","characterEncoding","serverTimezone","allowMultiQueries","rewriteBatchedStatements"]
});
eq(parseDatabaseHostAddress("phantomdev.rwlb.rds.aliyuncs.com"),{host:"phantomdev.rwlb.rds.aliyuncs.com",portText:""});
eq(parseDatabaseHostAddress("db.internal:3307"),{host:"db.internal",portText:"3307"});
for(const bad of ["user:password@db.internal","db.internal:0","db.internal:65536","db.internal/other","db..internal","db.internal?sslmode=disable"])eq(parseDatabaseHostAddress(bad),null);
eq(parseDatabaseConnectionUrl("mysql://localhost/app?allowMultiQueries=true"),{kind:"mysql",host:"localhost",portText:"3306",database:"app",sslMode:"prefer",ignoredOptions:["allowMultiQueries"]});
for(const bad of ["mysql://admin:pw@localhost/app","mysql://user@localhost/app","mysql://localhost/app?password=secret","mysql://localhost/app?allowLoadLocalInfile=true","mysql://localhost/app?sslMode=DISABLED&sslMode=REQUIRED","jdbc:sqlite:../data.db","sqlite:////tmp/data.db","postgresql://localhost/db#frag","file:///tmp/a","mysql://localhost","mysql://localhost/app/sub"])reject(bad);
const chrome=get("src/components/layout/ActivityBar.vue");
yes(chrome.includes('ref="omniAnchor"') && chrome.includes(':style="suggestionsGeometry"') && chrome.includes('nextTick(syncSuggestionsGeometry)'));
yes(!chrome.includes('position: fixed') && !chrome.includes('position:absolute'));
const files=get("src/capabilities/workspace/ui/FilePanel.vue");
yes(files.includes("openLocalGit('diff')") && files.includes("openLocalGit('history')") && files.includes("openLocalGit('branches')"));
yes(!files.includes('v-if="fileContextActions.length" @click="openLocalGit'));
const grid=get("src/capabilities/grid/state/useGridStore.ts");
yes(grid.includes("gridStartupError") && grid.includes("GRID_NATIVE_CREATE_INCOMPLETE") && grid.includes("gridStarting.value = false"));
const panel=get("src/capabilities/database/ui/DatabasePanel.vue");
yes(panel.includes("parseDatabaseConnectionUrl") && panel.includes("connectionUrl.value = \"\"") && panel.includes("从 IntelliJ IDEA 导入连接"));
const context=historyContextRows('@@ -3 +3,2 @@\n-old\n+new\n+extra\n','a\nb\nnew\nextra\nc\nd\n');
eq(context.map(row=>row.before),['a','b','old','','c','d']);
eq(context.map(row=>row.after),['a','b','new','extra','c','d']);
eq(context.map(row=>row.beforeLine),[1,2,3,null,4,5]);
eq(context.map(row=>row.afterLine),[1,2,3,4,5,6]);
const tree=bookmarkTree([{id:'a',title:'one',url:'https://a.invalid',category:'书签栏'},{id:'b',title:'two',url:'https://b.invalid',category:'书签栏/环境/内网'},{id:'c',title:'three',url:'https://c.invalid',category:'书签栏/环境'}]);
eq(tree.map(node=>node.title),['one','环境']);
eq(tree[1].children.map(node=>node.title),['内网','three']);
eq(tree[1].children[0].children[0].url,'https://b.invalid');
console.log("REAL_UI_REGRESSION_PASS="+assertions);
