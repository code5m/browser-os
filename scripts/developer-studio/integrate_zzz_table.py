"""Integrate the real table editor without changing existing quality budgets."""
import re

path = 'src-tauri/src/capabilities/database/mod.rs'
s = read(path)
if 'pub mod table_data;' not in s:
    write(path, s.rstrip() + '\npub mod table_data;\n')

path = 'src-tauri/src/capabilities/database/table_data.rs'
s = read(path)
s = once(s, 'let mut rows=query(pool,&sql,&parameters,MAX_ROWS+1,cancel)?;let more=', 'let mut rows=query(pool,&sql,&parameters,MAX_ROWS+1,cancel)?;if rows.columns.len()!=catalog.columns.len(){return Err("DB_TABLE_ROW_SHAPE_CHANGED".into());}let more=') if 'let mut rows=query' in s else s
if 'pub fn cancel_preview(' not in s:
    at = s.index('\n#[cfg(test)]')
    s = s[:at] + '\npub fn cancel_preview(state: &mut TableState, ticket: Option<&str>) -> Value {\n    let removed = ticket.is_some_and(|id| state.previews.remove(id).is_some());\n    json!({"cancelled":removed})\n}\n' + s[at:]
write(path, s)

path = 'src-tauri/src/capabilities/database/studio.rs'
s = read(path)
if '// DEVELOPER_STUDIO_TABLE_ACTOR' not in s:
    s = 'use super::table_data::{self, TableRequest, TableState};\n// DEVELOPER_STUDIO_TABLE_ACTOR\n' + s
    # Keep module documentation before use declarations.
    lines = s.splitlines(keepends=True)
    prefix = lines[:2]
    remaining = lines[2:]
    end = 0
    while end < len(remaining) and remaining[end].startswith('//!'):
        end += 1
    s = ''.join(remaining[:end] + prefix + remaining[end:])
    s = once(s, '    pub timeout_secs: Option<u64>,\n}', '    pub timeout_secs: Option<u64>,\n    pub table: Option<TableRequest>,\n}')
    s = once(s, 'struct ConnectionOwner {\n    pool: DbPool,\n    transaction: TransactionState,\n}', 'struct ConnectionOwner {\n    pool: DbPool,\n    transaction: TransactionState,\n    tables: TableState,\n}')
    s = s.replace('transaction: TransactionState::Idle,\n', 'transaction: TransactionState::Idle,\n            tables: TableState::default(),\n')
    start = s.index('fn transact(')
    marker = '    match request.action.as_str() {\n'
    where = s.index(marker, start) + len(marker)
    s = s[:where] + '''        "table_read" => table_data::read(&mut owner.pool, cfg, &mut owner.tables,
            request.table.as_ref().ok_or("DB_TABLE_REQUEST_REQUIRED")?, cancel),
        "table_preview" => {
            if owner.transaction != TransactionState::Idle { return Err("DB_TABLE_FINISH_TRANSACTION_FIRST".into()); }
            table_data::preview(&mut owner.pool, cfg, &mut owner.tables,
                request.table.as_ref().ok_or("DB_TABLE_REQUEST_REQUIRED")?, cancel)
        }
        "table_cancel" => Ok(table_data::cancel_preview(&mut owner.tables,
            request.table.as_ref().and_then(|t| t.ticket.as_deref()))),
        "table_apply" => {
            if owner.transaction != TransactionState::Idle { return Err("DB_TABLE_FINISH_TRANSACTION_FIRST".into()); }
            authorize_transaction(cfg, &owner.pool, true, request.confirmed)?;
            match table_data::apply(&mut owner.pool, cfg, &mut owner.tables,
                request.table.as_ref().ok_or("DB_TABLE_REQUEST_REQUIRED")?, request.confirmed, cancel) {
                Ok(reply) => { owner.transaction = TransactionState::ReadWrite; Ok(reply) }
                Err(error) => {
                    if error.contains("OUTCOME_UNKNOWN") { owner.transaction = TransactionState::Failed; }
                    Err(error)
                }
            }
        }
''' + s[where:]
    # A caller must not mistake an implicit-commit DDL batch for an atomic DML batch.
    helper = '''/// Only ordinary DML is allowed inside an owned transaction. DDL/admin
/// statements may implicitly commit on some drivers and are not atomic batches.
fn studio_transactional_sql(sql: &str) -> bool {
    if sp::classify_sql_risk(sql).is_read_only() { return true; }
    let word = sql.trim_start().split(|c: char| !c.is_ascii_alphabetic()).next().unwrap_or("").to_ascii_uppercase();
    matches!(word.as_str(), "INSERT" | "UPDATE" | "DELETE" | "REPLACE")
}
'''
    s = s.replace('fn run_sql(\n', helper + 'fn run_sql(\n', 1)
    s = once(s, '    if owner.transaction == TransactionState::Failed {\n        return Err("DB_SESSION_ROLLBACK_REQUIRED".into());\n    }', '    if owner.transaction == TransactionState::Failed {\n        return Err("DB_SESSION_ROLLBACK_REQUIRED".into());\n    }\n    if owner.transaction != TransactionState::Idle && !studio_transactional_sql(sql) {\n        return Err("DB_SESSION_NON_TRANSACTIONAL_STATEMENT_DENIED".into());\n    }')
    s = once(s, '            for sql in &request.statements {\n                sp::evaluate_db_query_gate', '            for sql in &request.statements {\n                if !studio_transactional_sql(sql) { return Err("DB_SESSION_NON_TRANSACTIONAL_BATCH_DENIED".into()); }\n                sp::evaluate_db_query_gate')
    s = once(s, '            owner.transaction = TransactionState::Idle;\n            Ok(json!({"transaction":owner.transaction,"committed":true}))', '            owner.transaction = TransactionState::Idle;\n            owner.tables = TableState::default();\n            Ok(json!({"transaction":owner.transaction,"committed":true}))')
    s = once(s, '            owner.transaction = TransactionState::Idle;\n            Ok(json!({"transaction":owner.transaction,"rolled_back":true}))', '            owner.transaction = TransactionState::Idle;\n            owner.tables = TableState::default();\n            Ok(json!({"transaction":owner.transaction,"rolled_back":true}))')
    s += '''\n#[cfg(test)]\nmod table_actor_tests {\n    use super::*;\n    #[test]\n    fn studio_atomic_batch_rejects_implicit_commit_statements() {\n        for sql in ["CREATE TABLE x(id INT)", "ALTER TABLE x ADD v INT", "DROP TABLE x", "TRUNCATE TABLE x", "SET autocommit=1", "COMMIT"] {\n            assert!(!studio_transactional_sql(sql), "{sql}");\n        }\n        assert!(studio_transactional_sql("SELECT 1"));\n        assert!(studio_transactional_sql("UPDATE x SET v=1 WHERE id=1"));\n    }\n}\n'''
write(path, s)

path = 'src/capabilities/database/state/useDatabaseStore.ts'
s = read(path)
if '// DEVELOPER_STUDIO_TABLE_DOCUMENTS' not in s:
    s = once(s, 'import { sqlStatements } from "../model/studioSql.mjs";', 'import { sqlStatements } from "../model/studioSql.mjs";\nimport { makeTableDocument, dirtyTable } from "../model/studioTable.mjs";')
    s = once(s, 'verdict:ProductionVerdict };', 'verdict:ProductionVerdict; tableData:ReturnType<typeof makeTableDocument>|null };')
    s = once(s, "risk:'unknown', verdict:'Unknown' });", "risk:'unknown', verdict:'Unknown', tableData:null });")
    s = once(s, "(doc.sql.trim() || doc.transaction !== 'idle')", "(doc.sql.trim() || doc.transaction !== 'idle' || dirtyTable(doc.tableData))")
    s = once(s, '(previous.sql.trim() || previous.raw || previous.sessionId)', '(previous.sql.trim() || previous.raw || previous.sessionId || previous.tableData)')
    methods = '''
  // DEVELOPER_STUDIO_TABLE_DOCUMENTS
  function openDataEditor(name:string) {
    const connId=activeId.value;
    if(!connId || !schema.value.some(item=>item.name===name)) { error.value='请从当前数据源对象树选择表或视图'; return; }
    const existing=documents.value.find(doc=>doc.connId===connId && doc.tableData?.name===name && !doc.closing);
    if(existing) { activeDocument.value=existing.id; return; }
    if(documents.value.length>=12) { error.value='最多打开 12 个查询页签'; return; }
    const next=makeDoc(++sequence,connId); next.name=name+' · 数据'; next.tableData=makeTableDocument(name);
    documents.value.push(next);activeDocument.value=next.id;
  }
  async function tableOperation(action:string, table:Record<string,unknown>, confirmed=false, documentId=activeDocument.value):Promise<any|null> {
    const doc=documents.value.find(item=>item.id===documentId),lifecycle=databaseLifecycleSnapshot();
    if(!lifecycle.active || !doc || !doc.connId || doc.busy || doc.closing || doc.pending || !['table_read','table_preview','table_apply','table_cancel'].includes(action))return null;
    const connId=doc.connId;doc.busy=true;doc.error='';
    try {
      const id=await ensureSession(doc);
      const reply=await bridge.dbStudioCall(id,{action,table,confirmed});
      const current=databaseLifecycleSnapshot();
      if(!current.active || current.generation!==lifecycle.generation || doc.connId!==connId || !documents.value.some(item=>item.id===documentId)) return null;
      if(reply.transaction)doc.transaction=reply.transaction;
      return reply;
    } catch(e) {
      doc.error=describeError(e);
      if(doc.sessionId && !doc.closing) {
        try { const state=await bridge.dbStudioCall(doc.sessionId,{action:'status'});doc.transaction=state.transaction; }
        catch { doc.transaction='unknown';doc.error+='；会话状态未知，请关闭会话，不要重试写操作。'; }
      }
      return null;
    } finally {doc.busy=false;}
  }
'''
    s = once(s, '    configs, activateDocument, requestRunText, sessionAction, releaseSession, selectBatchResult,', '    openDataEditor, tableOperation,\n    configs, activateDocument, requestRunText, sessionAction, releaseSession, selectBatchResult,')
    at=s.index('\n  return {\n',s.index('function selectBatchResult'))
    s=s[:at]+methods+s[at:]
    # Preserve the old bounded SQL preview action as an explicit alternative.
    # Table double-clicks now open the data editor via the shared entry point.
    start=re.search(r'^  function previewTable\(',s,re.M)
    end=re.search(r'^  function reuseHistory\(',s[start.end():],re.M)
    if not start or not end: raise RuntimeError('TABLE_ENTRY_ANCHOR_MISSING')
    s=s[:start.start()]+'  function previewTable(name: string) { openDataEditor(name); }\n\n'+s[start.end()+end.start():]
write(path,s)

path='src/capabilities/database/ui/DatabaseWorkspace.vue'
s=read(path)
if "import TableDataEditor from './TableDataEditor.vue';" not in s:
    s=once(s,"import SqlEditor from './SqlEditor.vue';","import SqlEditor from './SqlEditor.vue';\nimport TableDataEditor from './TableDataEditor.vue';")
    s=once(s,'  <SqlEditor :model-value=', '  <TableDataEditor v-if="db.document.tableData"/>\n  <SqlEditor v-else :model-value=')
    s=once(s,'<section v-if="view" class="results">','<section v-if="view && !db.document.tableData" class="results">')
write(path,s)

path='scripts/check-developer-studio.mjs'
s=read(path)
if "import './check-table-data.mjs';" not in s:
    s=once(s,"import assert from 'node:assert/strict';","import assert from 'node:assert/strict';\nimport './check-table-data.mjs';")
write(path,s)
print('DEVELOPER_STUDIO_TABLE_INTEGRATION=APPLIED')
