import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// An emitted asset belongs to the editor allowance only when a verified
// worker hash proves ownership, or every actual importing chunk is exclusive
// editor code. Unowned and shared assets always remain Core + Shared.
export function isExclusiveEditorAsset(owners, workerSourceHashVerified = false) {
  return workerSourceHashVerified ||
    (owners.length > 0 && owners.every(owner => owner.exclusive === true));
}

export function evaluateEditorBudget(report, policy) {
  assert.equal(policy.schema, 'browseros-coding-editor-budget-v1');
  assert.equal(report.schema, 'browseros-editor-budget-audit-v1');
  assert.equal(typeof report.css_split, 'boolean', 'CSS split mode must be explicitly measured and reported');
  assert.equal(report.dist_verification?.status, 'PASS', 'Production bytes must be verified');
  assert.ok(report.dist_verification.files > 0);
  assert.deepEqual(report.initial_js_graph?.editor_js_files, [], 'Editor must remain lazy');
  const fields = ['exclusive_editor_bytes', 'core_and_shared_bytes', 'total_bytes'];
  for (const field of fields) {
    assert.ok(Number.isSafeInteger(report[field]) && report[field] >= 0, `Invalid measured ${field}`);
    assert.ok(Number.isSafeInteger(policy.limitsBytes[field]) && policy.limitsBytes[field] > 0, `Invalid limit ${field}`);
  }
  assert.equal(report.exclusive_editor_bytes + report.core_and_shared_bytes, report.total_bytes, 'Unaccounted bytes');
  const failures = fields.filter(field => report[field] > policy.limitsBytes[field]);
  return { status: failures.length ? 'FAIL' : 'PASS', failures, limits_bytes: policy.limitsBytes };
}

if (process.argv[1] === fileURLToPath(import.meta.url) && process.argv.includes('--self-test')) {
  const policy = JSON.parse(readFileSync(new URL('../docs/engineering/coding-editor-budget.json', import.meta.url)));
  assert.equal(isExclusiveEditorAsset([{exclusive:true}]), true);
  assert.equal(isExclusiveEditorAsset([{exclusive:true},{exclusive:false}]), false);
  assert.equal(isExclusiveEditorAsset([]), false);
  assert.equal(isExclusiveEditorAsset([{exclusive:false}], true), true);
  const sample = {schema:'browseros-editor-budget-audit-v1',css_split:false,dist_verification:{status:'PASS',files:1},initial_js_graph:{editor_js_files:[]},exclusive_editor_bytes:3200000,core_and_shared_bytes:1300000,total_bytes:4500000};
  assert.equal(evaluateEditorBudget(sample, policy).status, 'PASS');
  assert.equal(
    evaluateEditorBudget({...sample, css_split:true}, policy).status,
    'PASS',
    'Production CSS splitting is valid only with exact emitted-byte and owner accounting',
  );
  for (const field of ['exclusive_editor_bytes','core_and_shared_bytes']) {
    const probe = {...sample, [field]:policy.limitsBytes[field]+1};
    probe.total_bytes=probe.exclusive_editor_bytes+probe.core_and_shared_bytes;
    assert.ok(evaluateEditorBudget(probe, policy).failures.includes(field));
  }
  const editorAtLimit=policy.limitsBytes.exclusive_editor_bytes;
  const coreForTotalFailure=policy.limitsBytes.total_bytes-editorAtLimit+1;
  assert.ok(coreForTotalFailure>=0&&coreForTotalFailure<=policy.limitsBytes.core_and_shared_bytes,'Total gate must independently constrain the two component budgets');
  const totalProbe={...sample,exclusive_editor_bytes:editorAtLimit,core_and_shared_bytes:coreForTotalFailure,total_bytes:policy.limitsBytes.total_bytes+1};
  assert.deepEqual(evaluateEditorBudget(totalProbe, policy).failures,['total_bytes']);
  for (const probe of [{...sample,css_split:'true'},{...sample,dist_verification:null},{...sample,total_bytes:1},{...sample,total_bytes:NaN},{...sample,initial_js_graph:{editor_js_files:['editor.js']}}]) {
    assert.throws(()=>evaluateEditorBudget(probe,policy));
  }
  console.log('CODING_EDITOR_BUDGET_SELF_TEST=PASS caps/unaccounted/invalid/unverified/eager-editor');
}
