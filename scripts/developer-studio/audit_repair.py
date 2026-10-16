"""One-shot PR35 audit repair; deleted when the readable repaired sources are committed.
The input is the SHA256-verified integration patch from run 38842522636, not
prepare.py or an unversioned source generator. Every replacement fails closed.
"""
from pathlib import Path
root = Path.cwd()
def edit(path, old, new, count=1):
    p=root/path; s=p.read_text()
    assert s.count(old)==count, (path, old[:80], s.count(old))
    p.write_text(s.replace(old,new))
p='src/capabilities/database/ui/TableDataEditor.vue'
edit(p, "async function load(offset = table.value?.offset || 0, discard = false) {\n  const current = table.value, document = db.document;\n  if (!current || db.busy || current.ticket || (!discard && dirtyTable(current))) return;\n  const reply = await db.tableOperation('table_read', request(current, { offset }), false, document.id);\n  if (!reply) return;\n  current.page = reply; current.offset = offset; current.displayPage = 0;\n  current.edits = {}; current.inserts = []; current.removed = []; current.cell = null;\n  if (!filterColumn.value) filterColumn.value = reply.table.columns[0]?.name || '';\n}", """const readVersions = new WeakMap<object, number>();
function ownsTable(documentId:string, current:any) {
  const owner = db.documents.find(doc => doc.id === documentId);
  return !!owner && !owner.closing && owner.tableData === current;
}
async function load(offset = table.value?.offset || 0, documentId = db.activeDocument, current = table.value) {
  const owner = db.documents.find(doc => doc.id === documentId);
  if (!current || !ownsTable(documentId, current) || owner!.busy || current.ticket || dirtyTable(current) || current.cell) return;
  const version = (readVersions.get(current) || 0) + 1;
  readVersions.set(current, version);
  const reply = await db.tableOperation('table_read', request(current, { offset }), false, documentId);
  if (!reply || !ownsTable(documentId, current) || readVersions.get(current) !== version || dirtyTable(current) || current.cell) return;
  current.page = reply; current.offset = offset; current.displayPage = 0;
  // Loading never discards drafts. Only explicit Revert or a confirmed Apply
  // can consume them; a response for A must never refresh the active B tab.
  if (db.activeDocument === documentId && !filterColumn.value) filterColumn.value = reply.table.columns[0]?.name || '';
}""")
edit(p,"!db.busy && !table.value?.ticket);","!db.busy && !db.document.closing && !db.document.closeFailed && !table.value?.awaitingCommit && !table.value?.ticket);")
edit(p,"const blocked = computed(() => db.busy || !!table.value?.ticket || db.document.closing);","const blocked = computed(() => db.busy || !!table.value?.ticket || db.document.closing || db.document.closeFailed);")
edit(p,"  if(!result){current.notice=", "  if (!ownsTable(doc.id, current)) return;\n  if(!result){current.notice=")
edit(p,"await load(current.offset,true);", "await load(current.offset,doc.id,current);")
edit(p,"void load(current.offset,true);", "void load(current.offset,db.document.id,current);")
edit(p,"if(reply){current.ticket=reply;acknowledged.value=false;}","if(reply && ownsTable(doc.id,current)){current.ticket=reply;if(db.activeDocument===doc.id)acknowledged.value=false;}")
edit(p,"if(reply){current.ticket=null;acknowledged.value=false;}","if(reply && ownsTable(doc.id,current)){current.ticket=null;if(db.activeDocument===doc.id)acknowledged.value=false;}")
edit('src-tauri/src/capabilities/database/table_data.rs', '.map_err(|_| "DB_TABLE_MUTATION_FAILED")', '.map_err(|_| "DB_TABLE_MUTATION_FAILED".to_owned())',2)
p='src-tauri/src/capabilities/git/studio.rs'
edit(p,'fn commit_selected(root: &Path, input: &StudioInput, cancel: &AtomicBool) -> Result<Value, String> {\n', '''fn ensure_scoped_commit_allowed(root: &Path) -> Result<(), String> {
    let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
    if repo.state() != git2::RepositoryState::Clean {
        return Err("GIT_STUDIO_OPERATION_IN_PROGRESS_USE_CONTINUE".into());
    }
    Ok(())
}
fn commit_selected(root: &Path, input: &StudioInput, cancel: &AtomicBool) -> Result<Value, String> {
    // Recheck at the actual write boundary, not only at preview time. A merge
    // must commit its complete real index, never a HEAD-based partial index.
    ensure_scoped_commit_allowed(root)?;
''')
edit(p,'            if action == "commit" || action == "amend" {\n','            if action == "commit" || action == "amend" {\n                ensure_scoped_commit_allowed(root)?;\n')
p=root/'src/capabilities/database/state/useDatabaseStore.ts'; s=p.read_text()
def replace(old,new,n=1):
    global s
    assert s.count(old)==n,(old[:90],s.count(old))
    s=s.replace(old,new)
replace('pendingBatch:boolean; closing:boolean;', 'pendingBatch:boolean; closing:boolean; closeFailed:boolean;')
replace("pendingBatch:false, closing:false, risk:","pendingBatch:false, closing:false, closeFailed:false, risk:")
a=s.index('  registerDatabaseCleanup(async () => {'); b=s.index('\n  /**',a)
s=s[:a]+'''  registerDatabaseCleanup(async () => {
    // A rejected cleanup blocks capability suspension. Never convert a failed
    // close acknowledgement into successful resource release.
    const owned = [...documents.value];
    const queryIds = owned.map(doc => doc.queryId).filter((id): id is string => !!id);
    const cancellations = await Promise.allSettled(queryIds.map(id => bridge.dbCancel(id)));
    const closures = await Promise.allSettled(owned.map(doc => releaseSession(doc)));
    if (closures.some(result => result.status === 'rejected' || !result.value) ||
        cancellations.some(result => result.status === 'rejected')) {
      throw new Error('DB_CLEANUP_NOT_CONFIRMED');
    }
    schemaGeneration += 1;
    columnsGeneration += 1;
    objectDetailRows.value = [];
    selectedObject.value = '';
    queryHistory.value = [];
    verifiedConnectionId.value = null;
    connecting.value = false;
    schemaBusy.value = false;
    for (const doc of owned) {
      doc.busy = false;
      doc.queryId = null;
      doc.pending = null;
    }
  });'''+s[b:]
replace('  const openingSessions = new Map<string, Promise<string>>();','''  const openingSessions = new Map<string, Promise<string>>();
  const closingSessions = new Map<string, Promise<boolean>>();
  function sessionFailure(doc:QueryDocument, error:unknown):boolean {
    // Preserve the string-error IPC ABI while retaining native failure facts.
    try {
      const fault = typeof error === 'string' ? JSON.parse(error) : error;
      if (fault && typeof fault === 'object' && fault.kind === 'database_session_fault') {
        doc.error = String(fault.code) + ': ' + String(fault.message);
        if (['idle','read_only','read_write','failed','unknown'].includes(fault.transaction)) doc.transaction = fault.transaction;
        if (fault.discard_connection === true) doc.closeFailed = true;
        return true;
      }
    } catch { /* Legacy errors are handled below, never guessed successful. */ }
    doc.error = describeError(error);
    return false;
  }''')
replace('    if(doc.sessionId) return doc.sessionId;', '''    if(doc.closeFailed || doc.transaction === 'unknown') throw new Error('DB_SESSION_CLOSE_OR_OUTCOME_UNCONFIRMED');
    if(doc.sessionId) return doc.sessionId;''')
replace('      const current=databaseLifecycleSnapshot();\n      if(doc.closing||!current.active', '      const current=databaseLifecycleSnapshot();\n      doc.sessionId=opened.session_id;\n      if(doc.closing||!current.active')
replace('        await bridge.dbStudioClose(opened.session_id); throw new Error(', '        await closeNativeSession(doc, opened.session_id); throw new Error(')
a=s.index('  async function releaseSession('); b=s.index('  async function sessionAction(',a)
s=s[:a]+'''  async function closeNativeSession(doc:QueryDocument, id:string):Promise<boolean> {
    try {
      for(let attempt=0;attempt<180;attempt++) {
        const reply=await bridge.dbStudioClose(id);
        if(reply.closed === true) {
          if(doc.sessionId === id) doc.sessionId=null;
          // Closing a socket cannot prove an ambiguous COMMIT was rolled back.
          if(doc.transaction !== 'unknown') doc.transaction='idle';
          doc.closeFailed=false;
          return true;
        }
        await new Promise(resolve=>setTimeout(resolve,250));
      }
      doc.error='DB_SESSION_CLOSE_NOT_CONFIRMED';
    } catch(error) { sessionFailure(doc,error); }
    doc.closeFailed=true;
    doc.transaction='unknown';
    return false;
  }
  function releaseSession(doc:QueryDocument=document.value):Promise<boolean> {
    const running=closingSessions.get(doc.id);
    if(running) return running;
    doc.closing=true;
    const closing=(async()=>{
      try {
        const opening=openingSessions.get(doc.id);
        if(opening) { try { await opening; } catch { /* Failed opening retains an unclosed session ID. */ } }
        const id=doc.sessionId;
        if(!id) return !doc.closeFailed;
        return await closeNativeSession(doc,id);
      } finally { doc.closing=false; }
    })();
    closingSessions.set(doc.id,closing);
    void closing.finally(()=>closingSessions.delete(doc.id));
    return closing;
  }
'''+s[b:]
replace('      doc.error=describeError(e);\n      if(doc.sessionId && !doc.closing) {', '      const structured=sessionFailure(doc,e);\n      if(!structured && doc.sessionId && !doc.closing) {',2)
replace("    } catch(e) {doc.error=describeError(e);}\n    finally {doc.busy=false;}\n  }\n  async function requestRunText", "    } catch(e) {sessionFailure(doc,e);}\n    finally {doc.busy=false;}\n  }\n  async function requestRunText")
p.write_text(s)
p=root/'src-tauri/src/capabilities/database/studio.rs'; s=p.read_text()
replace('database::{credential_key, DbPool, QueryCancel}', 'database::{credential_key, DbError, DbPool, QueryCancel}')
replace('domain::{DbConnectionConfig, SupportedDb}', 'domain::{DbConnectionConfig, DbErrorCode, DbQueryResult, SupportedDb}')
replace('    ReadWrite,\n    Failed,','    ReadWrite,\n    Failed,\n    Unknown,')
replace('    tables: TableState,\n}', '    tables: TableState,\n    reusable: bool,\n}')
replace('impl Drop for ConnectionOwner {','''impl ConnectionOwner {
    fn new(pool: DbPool) -> Self {
        Self { pool, transaction: TransactionState::Idle, tables: TableState::default(), reusable: true }
    }
}
impl Drop for ConnectionOwner {''')
replace('        if self.transaction != TransactionState::Idle {', '        if self.reusable && self.transaction != TransactionState::Idle {')
replace('''        let mut owner = ConnectionOwner {
            pool,
            transaction: TransactionState::Idle,
            tables: TableState::default(),
        };''','        let mut owner = ConnectionOwner::new(pool);')
replace('''        ConnectionOwner {
            pool: DbPool::Sqlite(conn),
            transaction: TransactionState::Idle,
            tables: TableState::default(),
        }''','        ConnectionOwner::new(DbPool::Sqlite(conn))')
a=s.index('fn run_sql('); b=s.index('fn transact(',a)
s=s[:a]+'''/// Versioned structured failure inside the existing string-error IPC contract.
/// Retire unusable connections before serialization; preserve unknown outcomes.
fn fault(owner: &ConnectionOwner, code: &str, message: &str, discard: bool) -> String {
    json!({"kind":"database_session_fault","version":1,"code":code,"message":message,
        "discard_connection":discard,"transaction":owner.transaction,
        "outcome_unknown":owner.transaction == TransactionState::Unknown}).to_string()
}
fn terminal_fault(owner: &mut ConnectionOwner, code: &str, message: &str) -> String {
    owner.reusable = false;
    owner.transaction = TransactionState::Unknown;
    owner.tables = TableState::default();
    fault(owner, code, message, true)
}
fn finish_control(owner: &mut ConnectionOwner, command: &str) -> Result<(), String> {
    if !owner.reusable {
        return Err(fault(owner, "DB_SESSION_CONNECTION_NOT_REUSABLE", "Close this session and verify the database outcome", true));
    }
    if let Err(message) = control(&mut owner.pool, command) {
        let code = if command == "COMMIT" { "DB_SESSION_COMMIT_OUTCOME_UNKNOWN_CHECK_DATABASE" }
                   else { "DB_SESSION_ROLLBACK_OUTCOME_UNKNOWN_CLOSE_SESSION" };
        return Err(terminal_fault(owner, code, &message));
    }
    owner.transaction = TransactionState::Idle;
    owner.tables = TableState::default();
    Ok(())
}
fn rollback_active(owner: &mut ConnectionOwner) -> Result<(), String> {
    if !owner.reusable {
        return Err(fault(owner, "DB_SESSION_CONNECTION_NOT_REUSABLE", "A rollback cannot be confirmed on this connection", true));
    }
    if owner.transaction != TransactionState::Idle { finish_control(owner, "ROLLBACK")?; }
    Ok(())
}
fn finish_query(
    owner: &mut ConnectionOwner,
    result: Result<DbQueryResult, DbError>,
    cancel: &QueryCancel,
    write: bool,
) -> Result<Value, String> {
    // Never erase DbError::discard_connection by mapping to text first.
    if let Err(error) = &result {
        if error.discard_connection {
            return Err(terminal_fault(owner, error.code_str(), &error.message));
        }
    }
    let interrupted = cancel.is_cancelled() || result.as_ref().err().is_some_and(|e|
        matches!(e.code, DbErrorCode::Cancelled | DbErrorCode::Timeout));
    if interrupted {
        if write && owner.transaction == TransactionState::Idle {
            return Err(terminal_fault(owner, "DB_SESSION_WRITE_OUTCOME_UNKNOWN_CHECK_DATABASE", "Do not retry an interrupted autocommit write"));
        }
        rollback_active(owner)?;
        return Err(fault(owner, "DB_SESSION_CANCELLED_OR_TIMED_OUT", "Operation stopped; any owned transaction rollback was acknowledged", false));
    }
    match result {
        Ok(result) => serde_json::to_value(result).map_err(|_| "DB_SESSION_RESULT_INVALID".into()),
        Err(error) => {
            if owner.transaction != TransactionState::Idle { owner.transaction = TransactionState::Failed; }
            Err(fault(owner, error.code_str(), &error.message, false))
        }
    }
}
fn run_sql(
    owner: &mut ConnectionOwner,
    cfg: &DbConnectionConfig,
    sql: &str,
    confirmed: bool,
    cancel: &QueryCancel,
    timeout: Option<u64>,
) -> Result<Value, String> {
    if !owner.reusable { return Err(fault(owner, "DB_SESSION_CONNECTION_NOT_REUSABLE", "Close the retired session", true)); }
    if owner.transaction == TransactionState::Failed { return Err("DB_SESSION_ROLLBACK_REQUIRED".into()); }
    if owner.transaction != TransactionState::Idle && !studio_transactional_sql(sql) {
        return Err("DB_SESSION_NON_TRANSACTIONAL_STATEMENT_DENIED".into());
    }
    let write = !sp::classify_sql_risk(sql).is_read_only();
    if owner.transaction == TransactionState::ReadOnly && write {
        return Err("DB_SESSION_READ_ONLY_TRANSACTION".into());
    }
    sp::evaluate_db_query_gate(sql, cfg, owner.pool.encrypted(), confirmed)
        .map_err(|code| code.as_str().to_string())?;
    if cancel.is_cancelled() { return Err("DB_SESSION_CANCELLED_BEFORE_EXECUTION".into()); }
    let id = uuid::Uuid::new_v4().to_string();
    let result = owner.pool.query(sql, cancel, timeout, &id);
    finish_query(owner, result, cancel, write)
}
'''+s[b:]
replace('''    match request.action.as_str() {
''','''    if !owner.reusable && request.action != "status" {
        return Err(fault(owner, "DB_SESSION_CONNECTION_NOT_REUSABLE", "Close the retired session", true));
    }
    match request.action.as_str() {
''')
replace('''                        owner.transaction = TransactionState::Failed;
                    }
                    Err(error)''','''                        return Err(terminal_fault(owner, "DB_TABLE_ROLLBACK_OUTCOME_UNKNOWN_CLOSE_SESSION", &error));
                    }
                    Err(error)''')
replace('''            control(&mut owner.pool, "COMMIT")
                .map_err(|_| "DB_SESSION_COMMIT_OUTCOME_UNKNOWN_CHECK_DATABASE".to_string())?;
            owner.transaction = TransactionState::Idle;
            owner.tables = TableState::default();''','''            finish_control(owner, "COMMIT")?;''')
replace('''            if owner.transaction != TransactionState::Idle {
                control(&mut owner.pool, "ROLLBACK")?;
            }
            owner.transaction = TransactionState::Idle;
            owner.tables = TableState::default();''','''            rollback_active(owner)?;
            owner.tables = TableState::default();''')
replace('''                            let _ = control(&mut owner.pool, "ROLLBACK");
                            owner.transaction = TransactionState::Idle;''','''                            rollback_active(owner)?;''')
replace('''                        let rollback = control(&mut owner.pool, "ROLLBACK");
                        owner.transaction = if rollback.is_ok() {
                            TransactionState::Idle
                        } else {
                            TransactionState::Failed
                        };
                        return Err(error);''','''                        if owner.reusable { rollback_active(owner)?; }
                        return Err(error);''')
replace('''                control(&mut owner.pool, "COMMIT")?;
                owner.transaction = TransactionState::Idle;''','''                finish_control(owner, "COMMIT")?;''')
replace('''            if envelope.response.send(outcome).is_err() {
                break;
            }''','''            let retired = !owner.reusable;
            if retired { handle.closing.store(true, Ordering::Release); }
            if envelope.response.send(outcome).is_err() || retired { break; }''')
replace('''    control(&mut owner.pool, text)?;
    owner.transaction = if writable''','''    if let Err(message) = control(&mut owner.pool, text) {
        return Err(terminal_fault(owner, "DB_SESSION_BEGIN_OUTCOME_UNKNOWN", &message));
    }
    owner.transaction = if writable''')
a=s.index('    #[test]\n    fn studio_transaction_rollback_preserves_database()')
s=s[:a]+'''    #[test]
    fn studio_fatal_driver_error_retires_connection() {
        let mut owner = owner();
        let error = DbError::fatal(DbErrorCode::Timeout, "fixture hard timeout");
        let failure = finish_query(&mut owner, Err(error), &QueryCancel::new(), false).unwrap_err();
        let payload: Value = serde_json::from_str(&failure).unwrap();
        assert_eq!(payload["discard_connection"], true);
        assert_eq!(payload["transaction"], "unknown");
        assert!(!owner.reusable);
        assert!(run_sql(&mut owner, &cfg(), "SELECT 1", false, &QueryCancel::new(), None).is_err());
    }
    #[test]
    fn studio_failed_rollback_never_reports_idle() {
        let mut owner = owner();
        owner.transaction = TransactionState::ReadWrite;
        assert!(finish_control(&mut owner, "ROLLBACK").is_err());
        assert_eq!(owner.transaction, TransactionState::Unknown);
        assert!(!owner.reusable);
    }
    #[test]
    fn studio_commit_failure_is_unknown_and_not_retried() {
        let mut owner = owner();
        if let DbPool::Sqlite(conn) = &owner.pool {
            conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);").unwrap();
        }
        begin(&mut owner, &cfg(), true, true).unwrap();
        if let DbPool::Sqlite(conn) = &owner.pool { conn.execute("INSERT INTO child VALUES(9)", []).unwrap(); }
        let failure = finish_control(&mut owner, "COMMIT").unwrap_err();
        let payload: Value = serde_json::from_str(&failure).unwrap();
        assert_eq!(payload["outcome_unknown"], true);
        assert_eq!(owner.transaction, TransactionState::Unknown);
        assert!(!owner.reusable);
        assert!(transact(&mut owner, &cfg(), &SessionRequest { action: "commit".into(), confirmed: true, ..Default::default() }, &QueryCancel::new()).is_err());
    }
    #[test]
    fn studio_cancelled_transaction_checks_real_rollback() {
        let mut owner = owner();
        begin(&mut owner, &cfg(), true, true).unwrap();
        if let DbPool::Sqlite(conn) = &owner.pool { conn.execute("INSERT INTO fixture VALUES(1,'temporary')", []).unwrap(); }
        let error = DbError::new(DbErrorCode::Cancelled, "fixture cancellation");
        assert!(finish_query(&mut owner, Err(error), &QueryCancel::new(), true).is_err());
        assert_eq!(owner.transaction, TransactionState::Idle);
        assert!(owner.reusable);
        if let DbPool::Sqlite(conn) = &owner.pool { let count: i64=conn.query_row("SELECT count(*) FROM fixture", [], |row|row.get(0)).unwrap();assert_eq!(count,0); }
    }
'''+s[a:]; p.write_text(s)
p=root/'src-tauri/src/capabilities/git/studio.rs'; s=p.read_text(); a=s.index('    #[test]\n    fn studio_scoped_commit_preserves_unselected_index()')
s=s[:a]+'''    fn assert_in_progress_commit_is_safe(operation: &str, continuation: &str, parents: usize) {
        let root = fixture();
        git(&root, &["branch", "-M", "base"]).unwrap();
        git(&root, &["switch", "-c", "incoming"]).unwrap();
        fs::write(root.join("a.txt"), "incoming a\\n").unwrap();
        fs::write(root.join("b.txt"), "incoming b\\n").unwrap();
        git(&root, &["add", "--", "a.txt", "b.txt"]).unwrap();
        git(&root, &["commit", "-m", "incoming changes"]).unwrap();
        git(&root, &["switch", "base"]).unwrap();
        fs::write(root.join("a.txt"), "base a\\n").unwrap();
        git(&root, &["add", "--", "a.txt"]).unwrap();
        git(&root, &["commit", "-m", "base changes"]).unwrap();
        assert!(git(&root, &[operation, "incoming"]).is_err());
        fs::write(root.join("a.txt"), "resolved a\\n").unwrap();
        git(&root, &["add", "--", "a.txt"]).unwrap();
        let before = snapshot(&root).unwrap();
        for action in ["commit", "amend"] {
            let input = StudioInput { action: action.into(), paths: vec!["a.txt".into()], message: Some("partial denied".into()), ..Default::default() };
            assert_eq!(validate(&root, &input).unwrap_err(), "GIT_STUDIO_OPERATION_IN_PROGRESS_USE_CONTINUE");
            assert_eq!(commit_selected(&root, &input, &AtomicBool::new(false)).unwrap_err(), "GIT_STUDIO_OPERATION_IN_PROGRESS_USE_CONTINUE");
            assert_eq!(snapshot(&root).unwrap(), before);
        }
        let input=StudioInput { action:continuation.into(), ..Default::default() };
        validate(&root,&input).unwrap();
        assert_eq!(execute(&root,&input,None,&AtomicBool::new(false)).unwrap()["success"],true);
        assert_eq!(git(&root,&["show","HEAD:b.txt"]).unwrap(),"incoming b\\n");
        assert_eq!(git(&root,&["show","HEAD:a.txt"]).unwrap(),"resolved a\\n");
        assert_eq!(git(&root,&["show","-s","--format=%P","HEAD"]).unwrap().split_whitespace().count(),parents);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn studio_merge_rejects_partial_commit_and_keeps_complete_tree() {
        assert_in_progress_commit_is_safe("merge", "merge_continue", 2);
    }
    #[test]
    fn studio_rebase_rejects_partial_commit_and_keeps_complete_tree() {
        assert_in_progress_commit_is_safe("rebase", "rebase_continue", 1);
    }
    #[test]
    fn studio_cherry_pick_rejects_partial_commit_and_keeps_complete_tree() {
        assert_in_progress_commit_is_safe("cherry-pick", "cherry_pick_continue", 1);
    }
'''+s[a:]; p.write_text(s)
p=root/'src/capabilities/database/ui/DatabaseWorkspace.vue'; s=p.read_text(); a=s.index('async function closeSession(){'); b=s.index('\nfunction sort(',a)
s=s[:a]+'''async function closeSession(){
  const owner=db.document;
  notice.value='正在关闭会话资源，不会自动提交事务。';
  const closed=await db.releaseSession(owner);
  if(db.activeDocument!==owner.id)return;
  notice.value=closed?'会话资源已关闭。如果事务结果未知，仍需核对数据库，不能据此认定已回滚。':'关闭尚未确认，请重试关闭；不要重复写入。';
}'''+s[b:]
s=s.replace("db.document.transaction==='failed'", "['failed','unknown'].includes(db.document.transaction)")
s=s.replace(':disabled="db.document.busy||db.confirmOpen"', ':disabled="db.document.busy||db.confirmOpen||db.document.closing||db.document.closeFailed||db.document.transaction===\'unknown\'"')
p.write_text(s)
p=root/'src/capabilities/git/ui/GitStudio.vue'; s=p.read_text(); target='const hints:Record<string,string>={'
assert s.count(target)==1
s=s.replace(target,target+"GIT_STUDIO_OPERATION_IN_PROGRESS_USE_CONTINUE:'仓库正在合并、变基或挑选提交；请使用对应的继续操作，不能部分提交。',")
p.write_text(s)
print('PR35_AUDIT_SOURCE_REPAIR_APPLIED')
