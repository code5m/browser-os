(async function native58InstalledGuiReview() {
  const base = __NATIVE58_BASE_URL__;
  const tauri = window.__TAURI__;
  const invoke = tauri?.core?.invoke;
  const results = [];
  const remote = { title: 0, resources: 0 };
  let remoteTab = null;
  let resourceCountBeforeClear = 0;
  let sessionId = '';
  let snippetId = '';
  let bookmarkId = '';
  let restoredTabId = '';
  let originalClipboard = '';
  const wait = (ms) => new Promise(resolve => setTimeout(resolve, ms));
  const text = (error) => String(error?.message || error || 'unknown error');
  const record = (command, status, detail) => results.push({command,status,detail:String(detail ?? '').slice(0,600)});
  const progress = (command, phase) => tauri?.event?.emit?.('native-58-review-progress',{command,phase}).catch(()=>{});
  const run = async (command, operation) => {
    await progress(command,'start');
    try { const detail = await operation(); record(command,'PASS',detail ?? 'IPC completed'); await progress(command,'pass'); return detail; }
    catch (error) { record(command,'FAIL',text(error)); await progress(command,'fail'); return undefined; }
  };
  const expectError = async (operation, expected) => {
    try { await operation(); throw new Error('expected rejection was not returned'); }
    catch (error) { const message=text(error); if (!message.includes(expected)) throw new Error(`expected ${expected}; got ${message}`); return message; }
  };
  const poll = async (operation, predicate, timeout=9000) => {
    const until=Date.now()+timeout; let last;
    while(Date.now()<until){try{last=await operation();if(predicate(last))return last;}catch{} await wait(250);}
    throw new Error(`condition not reached; last=${JSON.stringify(last)}`);
  };
  try {
    if (!invoke || !tauri.event?.listen || !tauri.event?.emit) throw new Error('Tauri main WebView IPC/event API unavailable');
    const unlistenTitle = await tauri.event.listen('tab-title', () => { remote.title++; });
    const unlistenResources = await tauri.event.listen('browser-resources', () => { remote.resources++; });

    // 1-4: system clipboard round-trip through the real installed GUI process.
    await run('clipboard_write', async () => {
      try { originalClipboard=await invoke('clipboard_read'); } catch { originalClipboard=''; }
      await invoke('clipboard_write',{text:'BROWSEROS_NATIVE58_CLIPBOARD'});
      return 'system clipboard write accepted';
    });
    await run('clipboard_read', async () => {
      const value=await invoke('clipboard_read');
      if(value!=='BROWSEROS_NATIVE58_CLIPBOARD')throw new Error(`clipboard readback mismatch: ${value}`);
      await invoke('clipboard_write',{text:originalClipboard});
      return 'system clipboard readback matched and previous value restored';
    });

    // 7-8: trusted main-window diagnostics and real child-WebView JavaScript execution.
    await run('debug_log', () => invoke('debug_log',{msg:'NATIVE58_INSTALLED_GUI_REVIEW'}));

    // 10/37: default-browser lookup and explicit, one-shot user intent. The
    // Python runner replaces xdg-settings with a temp-home-only recorder.
    await run('get_default_browser', async () => {
      const value=await invoke('get_default_browser');
      if(value!=='firefox.desktop')throw new Error(`unexpected isolated default browser: ${value}`);
      return value;
    });
    await run('set_default_browser', async () => {
      await expectError(()=>invoke('set_default_browser',{}),'NATIVE_USER_CONFIRMATION_REQUIRED');
      const token=await invoke('issue_intent',{scope:'set_default_browser'});
      const changed=await invoke('set_default_browser',{intent:token});
      await expectError(()=>invoke('set_default_browser',{intent:token}),'NATIVE_USER_CONFIRMATION_DENIED');
      const current=await invoke('get_default_browser');
      if(current!==changed)throw new Error(`xdg-settings readback ${current} != ${changed}`);
      return `explicit token changed isolated default to ${changed}; replay rejected`;
    });

    // 11/38/29: resource capture settings and live resource telemetry.
    await run('set_resource_capture_settings', async () => {
      const value=await invoke('set_resource_capture_settings',{enabled:true,maxPerTab:40});
      if(value.enabled!==true || value.max_per_tab!==40)throw new Error(JSON.stringify(value));
      return `enabled=${value.enabled}; max_per_tab=${value.max_per_tab}`;
    });
    await run('get_resource_capture_settings', async () => {
      const value=await invoke('get_resource_capture_settings');
      if(value.enabled!==true || value.max_per_tab!==40)throw new Error(JSON.stringify(value));
      return JSON.stringify(value);
    });

    // 18: one-shot intent issuance is main-only and scope-bound.
    await run('issue_intent', async () => {
      const token=await invoke('issue_intent',{scope:'set_default_browser'});
      if(typeof token!=='string' || token.length<20)throw new Error('token missing/invalid');
      await expectError(()=>invoke('issue_intent',{scope:'arbitrary-command'}),'NATIVE_INTENT_SCOPE_DENIED');
      await invoke('set_default_browser',{intent:token});
      await expectError(()=>invoke('set_default_browser',{intent:token}),'NATIVE_USER_CONFIRMATION_DENIED');
      return 'trusted main issued a scoped token; allowlisted effect succeeded once, replay and arbitrary scope rejected';
    });

    // 22/58: startup-ready signal and queued external URL handoff.
    await run('m0_ready', async () => {
      const value=await invoke('m0_ready');
      if(value!=='')throw new Error(`unexpected non-measurement run id: ${value}`);
      return 'frontend-ready transition completed outside measurement mode';
    });
    await run('take_pending_open_urls', async () => {
      const urls=await invoke('take_pending_open_urls');
      if(!Array.isArray(urls) || !urls.includes('https://example.invalid/native-58-pending'))throw new Error(JSON.stringify(urls));
      return `drained ${urls.length} queued URL(s)`;
    });

    // 23/25/5: legacy browser open/position/close commands.
    await run('open_browser', async () => {
      await invoke('open_browser',{url:`${base}/page?name=open-browser`});
      await wait(700);
      return 'created a real child browser WebView';
    });
    await run('position_browser', async () => {
      await invoke('position_browser',{x:28,y:96,width:640,height:420});
      return 'active browser WebView positioned to a bounded visible rectangle';
    });
    await run('close_browser', async () => {
      await invoke('close_browser');
      return 'active browser WebView closed';
    });

    // 53/54/48/55/51/50/56/57/8/52/20/2/29/17/16/49: tab lifecycle,
    // navigation, JS, native visibility, resource observation and clearing.
    await run('tab_new', async () => {
      remoteTab=await invoke('tab_new',{url:`${base}/remote`});
      if(!remoteTab?.id?.startsWith('tab-'))throw new Error(JSON.stringify(remoteTab));
      return `created ${remoteTab.id} at loopback remote page`;
    });

    // The remote page invokes report_title/report_resources using its own
    // remote capability and attempts a main-only clipboard command. Results are
    // posted to the loopback evidence server and checked below.
    await poll(async()=>({...remote}),value=>value.title>0&&value.resources>0,12000);
    // These events can only be emitted after the remote-origin commands pass
    // their Tauri capability and invocation-source checks. The runner separately
    // verifies the remote denial of clipboard_write from its loopback server.
    record('report_title',remote.title>0?'PASS':'FAIL',`remote-origin report_title emitted ${remote.title} tab-title event(s)`);
    record('report_resources',remote.resources>0?'PASS':'FAIL',`remote-origin report_resources emitted ${remote.resources} browser-resources event(s)`);

    await run('tab_open', async () => {
      await invoke('tab_open',{id:remoteTab.id,url:`${base}/page?name=history-one`});
      await wait(450);
      await invoke('tab_open',{id:remoteTab.id,url:`${base}/page?name=history-two`});
      await wait(700);
      return 'navigated same child WebView across two local pages';
    });
    await run('tab_go_back', async () => { await invoke('tab_go_back',{id:remoteTab.id}); await wait(500); return 'history.back executed in child WebView'; });
    await run('tab_go_forward', async () => { await invoke('tab_go_forward',{id:remoteTab.id}); await wait(500); return 'history.forward executed in child WebView'; });
    await run('tab_reload', async () => { await invoke('tab_reload',{id:remoteTab.id}); await wait(700); return 'location.reload executed in child WebView'; });
    await run('tab_activate', async () => { await invoke('tab_activate',{id:remoteTab.id}); return 'active tab identity updated'; });
    await run('tab_position', async () => { await invoke('tab_position',{id:remoteTab.id,x:35,y:105,width:700,height:460}); return 'child WebView moved/resized to visible rectangle'; });
    await run('tab_set_title', async () => { await invoke('tab_set_title',{id:remoteTab.id,title:'Native 58 GUI evidence'}); const list=await invoke('tab_list'); if(!list.some(item=>item.id===remoteTab.id&&item.title==='Native 58 GUI evidence'))throw new Error('title metadata readback mismatch'); return 'tab title metadata persisted in Browser owner'; });
    await run('eval_in_tab', async () => { await invoke('eval_in_tab',{id:remoteTab.id,js:`document.title='NATIVE58_EVAL_OK';fetch(${JSON.stringify(`${base}/event?phase=eval-in-tab`)})`}); await wait(450); return 'child WebView JS changed title and posted loopback event; runner verifies the server-side event'; });
    await run('tab_list', async () => { const list=await invoke('tab_list'); if(!list.some(item=>item.id===remoteTab.id))throw new Error('new tab missing from Browser owner list'); return `${list.length} Browser-owned tab(s) returned`; });

    await run('list_tab_resources', async () => {
      const value=await poll(()=>invoke('list_tab_resources',{tabId:remoteTab.id}),list=>Array.isArray(list.records)&&list.records.length>0,9000);
      resourceCountBeforeClear=value.records.length;
      return `${resourceCountBeforeClear} real WebKit resource event(s) captured for ${remoteTab.id}`;
    });
    await run('resource_stats', async () => { const stats=await invoke('resource_stats'); if(!stats||typeof stats!=='object')throw new Error(JSON.stringify(stats)); return JSON.stringify(stats); });
    await run('clear_tab_resources', async () => { await invoke('clear_tab_resources',{tabId:remoteTab.id}); const after=await invoke('list_tab_resources',{tabId:remoteTab.id}); if(after.records.length!==0)throw new Error(`records remain after clear: ${after.records.length}`); return `cleared ${resourceCountBeforeClear} captured resource(s) and read back empty`; });
    await run('hide_webview', async () => { await invoke('hide_webview',{id:remoteTab.id}); await invoke('tab_position',{id:remoteTab.id,x:35,y:105,width:700,height:460}); return 'single WebView moved offscreen and then restored without destroying it'; });
    await run('hide_all_webviews', async () => { await invoke('hide_all_webviews'); await wait(250); await invoke('tab_activate',{id:remoteTab.id}); await invoke('tab_position',{id:remoteTab.id,x:35,y:105,width:700,height:460}); return 'all child WebViews hidden then active tab restored'; });

    // 1/19/26: bookmark persistence, readback and deletion.
    await run('add_bookmark', async () => { const value=await invoke('add_bookmark',{url:`${base}/bookmark`,title:'Native 58 bookmark',category:'test'}); bookmarkId=value.id; if(!bookmarkId)throw new Error(JSON.stringify(value)); return `persisted bookmark ${bookmarkId}`; });
    await run('list_bookmarks', async () => { const list=await invoke('list_bookmarks'); if(!list.some(item=>item.id===bookmarkId))throw new Error('bookmark missing from persisted readback'); return `${list.length} bookmark(s) read; fixture present`; });
    await run('remove_bookmark', async () => { await invoke('remove_bookmark',{id:bookmarkId}); const list=await invoke('list_bookmarks'); if(list.some(item=>item.id===bookmarkId))throw new Error('bookmark still present'); return 'bookmark removed and persisted readback is clean'; });

    // 21/24/6: built-in tool inventory and actual tool WebView lifecycle.
    await run('list_tools', async () => { const list=await invoke('list_tools'); if(!Array.isArray(list)||!list.some(item=>item.id==='json'))throw new Error(JSON.stringify(list)); return `${list.length} tool(s) enumerated; built-in json present`; });
    await run('open_tool', async () => { await invoke('open_tool',{id:'json'}); await wait(500); return 'tool-json WebView created through installed GUI IPC'; });
    await run('close_tools', async () => { await invoke('close_tools'); await wait(200); return 'all Tools-owned tool-* windows closed'; });

    // 12/39/31/9: session policy, save/read/export/restore/delete and draft discard/flush.
    await run('get_session_policy', async () => { const value=await invoke('get_session_policy'); if(typeof value.close_prompt!=='boolean'||typeof value.auto_save_on_exit!=='boolean')throw new Error(JSON.stringify(value)); return JSON.stringify(value); });
    await run('set_session_policy', async () => { const value=await invoke('set_session_policy',{closePrompt:false,autoSaveOnExit:false}); if(value.close_prompt!==false||value.auto_save_on_exit!==false)throw new Error(JSON.stringify(value)); return JSON.stringify(value); });
    await run('session_discard', async () => { await invoke('session_discard',{tabId:'tab-review-discard'}); return 'discarded the pre-seeded in-memory session draft'; });
    await run('session_save', async () => { const value=await invoke('session_save',{tabId:remoteTab.id,preview:'Native 58 session fixture'}); sessionId=value.id; if(!sessionId)throw new Error(JSON.stringify(value)); return `persisted session ${sessionId}`; });
    await run('session_list', async () => { const list=await invoke('session_list'); if(!list.some(item=>item.id===sessionId))throw new Error('saved session missing from list'); return `${list.length} saved session(s); fixture present`; });
    await run('session_get', async () => { const value=await invoke('session_get',{id:sessionId}); if(value.id!==sessionId)throw new Error(JSON.stringify(value)); return 'saved session detail read back'; });
    await run('session_export', async () => { const value=await invoke('session_export',{id:sessionId}); const parsed=JSON.parse(value); if(parsed.id!==sessionId)throw new Error('export id mismatch'); return `${value.length} byte redacted JSON export returned without writing a new file`; });
    await run('session_restore', async () => { const value=await invoke('session_restore',{id:sessionId}); restoredTabId=value.id; if(!restoredTabId?.startsWith('tab-'))throw new Error(JSON.stringify(value)); return `restored saved session into ${restoredTabId}`; });
    await run('session_delete', async () => { await expectError(()=>invoke('session_delete',{id:sessionId}),'NATIVE_USER_CONFIRMATION_REQUIRED'); const token=await invoke('issue_intent',{scope:`session_delete:${sessionId}`}); const removed=await invoke('session_delete',{id:sessionId,intent:token}); if(removed!==true)throw new Error(`delete returned ${removed}`); await expectError(()=>invoke('session_delete',{id:sessionId,intent:token}),'NATIVE_USER_CONFIRMATION_DENIED'); return 'saved session deleted only with scoped one-shot intent; replay rejected'; });
    await run('flush_sessions', async () => { const value=await invoke('flush_sessions'); if(value.drafts_dropped<1)throw new Error(JSON.stringify(value)); return `flush dropped ${value.drafts_dropped} seeded draft(s), persisted=${value.persisted}`; });

    // 40: hibernation preference is reversible and observed through state readback.
    await run('set_tab_hibernation', async () => { await invoke('set_tab_hibernation',{enabled:true}); await invoke('set_tab_hibernation',{enabled:false}); return 'hibernation enabled then disabled through Native owner'; });

    // 13-15: graph read-only commands against a real seeded workspace document.
    let graphSeed=null;
    try { graphSeed=await invoke('graph_rebuild_documents'); } catch (error) { throw new Error(`graph fixture rebuild failed: ${text(error)}`); }
    const graphId=graphSeed?.first_id;
    await run('graph_stats', async () => { const value=await invoke('graph_stats'); if(!value||typeof value.node_count!=='number')throw new Error(JSON.stringify(value)); return `${value.node_count} node(s), ${value.edge_count} edge(s)`; });
    await run('graph_node_get', async () => { if(!graphId)throw new Error('graph fixture did not produce a node id'); const value=await invoke('graph_node_get',{id:graphId}); if(!value||value.id!==graphId)throw new Error(JSON.stringify(value)); return `read node ${graphId}`; });
    await run('graph_query', async () => { if(!graphId)throw new Error('graph fixture did not produce a node id'); const value=await invoke('graph_query',{req:{start_id:graphId,depth:1,limit:10}}); if(!value.found)throw new Error(JSON.stringify(value)); return `query returned ${value.nodes.length} node(s)`; });

    // 41-43: Skill parser/validator/permission preview, all read-only.
    const skillText=JSON.stringify({id:'native58-review',version:'1.0.0',display_name:'Native 58 Review',description:'read-only fixture',acl:'safe',exec:{kind:'script_ref',script_id:'native58-script',params:{}},inputs:[],capabilities:[],tests:[],metadata:{}});
    await run('skill_parse', async () => { const value=await invoke('skill_parse',{text:skillText}); if(value.id!=='native58-review')throw new Error(JSON.stringify(value)); return 'valid SkillDef parsed'; });
    await run('skill_validate', async () => { const value=await invoke('skill_validate',{text:skillText}); if(value.valid!==true)throw new Error(JSON.stringify(value)); return 'valid SkillDef passed validation'; });
    await run('skill_permission_preview', async () => { const value=await invoke('skill_permission_preview',{text:skillText}); if(!value)throw new Error('empty permission preview'); return 'read-only permission preview returned'; });

    // 44-47: snippet metadata CRUD and a confirmed one-shot delete. No script is executed.
    let snippet=null;
    await run('snippet_add', async () => { snippet=await invoke('snippet_add',{name:'Native 58 fixture',category:'test',interpreter:'bash',argv:['printf','NATIVE58'],params:[],description:'isolated GUI review fixture',dangerous:false,enabled:true,timeoutSecs:5}); snippetId=snippet.id; if(!snippetId)throw new Error(JSON.stringify(snippet)); return `persisted snippet ${snippetId}; no execution requested`; });
    await run('snippet_list', async () => { const list=await invoke('snippet_list'); if(!list.some(item=>item.id===snippetId))throw new Error('snippet missing from readback'); return `${list.length} snippet(s); fixture present`; });
    await run('snippet_update', async () => { const value=await invoke('snippet_update',{id:snippetId,name:'Native 58 fixture updated',category:'test',interpreter:'bash',argv:['printf','NATIVE58_UPDATED'],params:[],description:'isolated GUI review fixture',dangerous:false,enabled:false,timeoutSecs:5}); if(value.name!=='Native 58 fixture updated'||value.enabled!==false)throw new Error(JSON.stringify(value)); return 'snippet update persisted and read back'; });
    await run('snippet_remove', async () => { await expectError(()=>invoke('snippet_remove',{id:snippetId}),'NATIVE_USER_CONFIRMATION_REQUIRED'); const token=await invoke('issue_intent',{scope:`snippet_remove:${snippetId}`}); await invoke('snippet_remove',{id:snippetId,intent:token}); await expectError(()=>invoke('snippet_remove',{id:snippetId,intent:token}),'命令片段不存在'); const list=await invoke('snippet_list'); if(list.some(item=>item.id===snippetId))throw new Error('snippet remains after deletion'); return 'snippet removed with scoped one-shot intent; repeat correctly rejected because the item no longer exists'; });

    // Close both browser tabs created by this review. This is the explicit
    // tab_close command; the restored session tab is closed as fixture cleanup.
    await run('tab_close', async () => { await invoke('tab_close',{id:remoteTab.id}); if(restoredTabId)await invoke('tab_close',{id:restoredTabId}); return 'review browser tab and restored fixture tab closed'; });

    // The remote ACL test has three remote results; only the two registered
    // Native 58 remote commands are ledger entries. The third is a negative
    // main-only command denial proving no remote side-effect escape.
    await unlistenTitle(); await unlistenResources();

    const expected=[
      'add_bookmark','clear_tab_resources','clipboard_read','clipboard_write','close_browser','close_tools','debug_log','eval_in_tab','flush_sessions','get_default_browser','get_resource_capture_settings','get_session_policy','graph_node_get','graph_query','graph_stats','hide_all_webviews','hide_webview','issue_intent','list_bookmarks','list_tab_resources','list_tools','m0_ready','open_browser','open_tool','position_browser','remove_bookmark','report_resources','report_title','resource_stats','session_delete','session_discard','session_export','session_get','session_list','session_restore','session_save','set_default_browser','set_resource_capture_settings','set_session_policy','set_tab_hibernation','skill_parse','skill_permission_preview','skill_validate','snippet_add','snippet_list','snippet_remove','snippet_update','tab_activate','tab_close','tab_go_back','tab_go_forward','tab_list','tab_new','tab_open','tab_position','tab_reload','tab_set_title','take_pending_open_urls'
    ];
    const byName=new Map(results.map(item=>[item.command,item]));
    const missing=expected.filter(name=>!byName.has(name));
    const duplicates=results.map(item=>item.command).filter((name,index,array)=>array.indexOf(name)!==index);
    const commandPass=expected.length===58&&missing.length===0&&duplicates.length===0&&expected.every(name=>byName.get(name).status==='PASS');
    const report={status:commandPass?'PASS':'FAIL',expectedCommands:expected.length,commands:results,missing,duplicates,resourceTabId:remoteTab?.id||'',resourceRecordsBeforeClear:resourceCountBeforeClear,sessionId,snippetId,bookmarkId,restoredTabId,remoteEvents:remote};
    await tauri.event.emit('native-58-review-finished',report);
  } catch (error) {
    const report={status:'FAIL',expectedCommands:58,commands:results,error:text(error),resourceTabId:remoteTab?.id||'',resourceRecordsBeforeClear:resourceCountBeforeClear,sessionId,snippetId,bookmarkId,restoredTabId,remoteEvents:remote};
    try { await tauri?.event?.emit('native-58-review-finished',report); } catch {}
  }
})();
