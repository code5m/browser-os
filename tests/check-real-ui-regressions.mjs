import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import {parseDatabaseConnectionUrl} from "../src/utils/dbConnectionUrl.ts";
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
for(const bad of ["mysql://admin:pw@localhost/app","mysql://user@localhost/app","mysql://localhost/app?password=secret","mysql://localhost/app?allowMultiQueries=true","mysql://localhost/app?sslMode=DISABLED&sslMode=REQUIRED","jdbc:sqlite:../data.db","sqlite:////tmp/data.db","postgresql://localhost/db#frag","file:///tmp/a","mysql://localhost","mysql://localhost/app/sub"])reject(bad);
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
console.log("REAL_UI_REGRESSION_PASS="+assertions);
