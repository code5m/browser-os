(async () => {
  const root = __FIXTURE_ROOT__;
  const results = [];
  const assert = (ok,name) => { results.push({name,pass:!!ok}); if (!ok) throw new Error(name); };
  const wait = async predicate => { for(let i=0;i<100;i++){if(predicate())return;await new Promise(r=>setTimeout(r,50));}throw new Error('wait timeout'); };
  const { emit } = await import('/node_modules/@tauri-apps/api/event.js');
  try {
    const { bridge } = await import('/src/bridge.ts');
    const { useVaultStore } = await import('/src/stores/useVaultStore.ts');
    const { useLayoutStore } = await import('/src/stores/useLayoutStore.ts');
    const { useDatabaseStore } = await import('/src/capabilities/database/state/useDatabaseStore.ts');
    const layout=useLayoutStore();const vault=useVaultStore();
    layout.openModule('vault');vault.path=root;await vault.open();
    assert(vault.notes.length===2,'native vault reads 2 notes');
    vault.follow('notes/Plan');assert(vault.selected==='notes/Plan.md','vault link navigation');
    assert(vault.backlinks.includes('Home.md'),'vault backlinks');
    const archived = await bridge.archiveReplies(root,[{label:'A1',markdown:'# Reply\n\n'+'full content '.repeat(100)},{label:'A2',markdown:''}],['native','AI']);
    assert(!!archived[0].path && !!archived[1].error,'archive partial success');
    const next = await bridge.vaultOpen(root);
    assert(next.notes.some(n=>n.path.includes('A1') && n.text.length>1000),'archive keeps full reply and is readable');
    let denied=false;try{await bridge.vaultOpen('/etc')}catch{denied=true}assert(denied,'vault outside root denied');
    const log=await bridge.gitLog('workbench-smoke');assert(log.length===1,'git history from real repository');
    assert((await bridge.gitCommitDiff('workbench-smoke',log[0].oid)).includes('+fixture'),'git real commit diff');
    const db=useDatabaseStore();layout.openModule('db');
    db.form.name='Smoke SQLite';db.form.database=root+'/data.db';db.form.productionHint='no';
    assert(await db.connect(''),'native db connect');
    assert(db.connections.length===1,'native connection list');
    assert(db.schema.some(t=>t.name==='items'),'native schema tree');
    db.sql='SELECT count(*) FROM items';db.requestRun();await wait(()=>!db.busy);
    assert(db.result?.rows[0][0]==='100','native SQL result decoded');
    const first=db.document;db.newDocument();assert(first.sql==='SELECT count(*) FROM items' && db.sql==='','independent SQL docs');
    db.sql='SELECT sum(a.value*b.value*c.value*d.value) FROM items a CROSS JOIN items b CROSS JOIN items c CROSS JOIN items d';
    db.requestRun();await new Promise(r=>setTimeout(r,100));await db.cancelQuery();await wait(()=>!db.busy);
    assert(db.error.includes('DB_CANCELLED'),'native in-flight cancel');
    let gridDenied=false;try{await bridge.gridReadReplies(99)}catch{gridDenied=true}assert(gridDenied,'grid index guard');
    await db.disconnect();assert(!db.connected,'native disconnect');
    await emit('workbench-smoke-finished',{pass:true,results});
  } catch(e) {
    await emit('workbench-smoke-finished',{pass:false,results,error:String(e)});
  }
})()
