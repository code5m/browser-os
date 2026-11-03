import assert from 'node:assert/strict';
import {createDocumentOwnership, documentKey} from '../src/application/documentOwnership.mjs';

assert.equal(documentKey('/project/', 'src//a.ts'), '/project/src/a.ts');
assert.equal(documentKey('', '/project/src/./a.ts'), '/project/src/a.ts');
for(const invalid of ['../escape','a/../b','/project/../secret','']) {
  assert.throws(() => documentKey('/project', invalid), /DOCUMENT_PATH_INVALID/);
}
const registry = createDocumentOwnership();
registry.replace('overlay', [{path:'/project/a.ts', dirty:true}]);
registry.replace('inline', [{path:'/project/a.ts', dirty:false}]);
registry.replace('coding', [{root:'/project',path:'a.ts',dirty:false}]);
assert.deepEqual(registry.conflicts('overlay','/project/a.ts'), []);
assert.deepEqual(registry.conflicts('coding','/project/a.ts'), ['overlay']);
assert.equal(registry.dirtyUnder('/project',true),true);
assert.equal(registry.dirtyUnder('/project-other',true),false);
assert.equal(registry.dirtyUnder('/project/a.ts',false),true);
assert.throws(() => registry.beginSave('coding','/project/a.ts'),/DOCUMENT_CONCURRENT_DIRTY_EDIT/);
const release = registry.beginSave('overlay','/project/a.ts');
assert.equal(registry.writingUnder('/project',true),true);
assert.equal(registry.writingUnder('/project/a.ts',false),true);
assert.equal(registry.writingUnder('/project-not-here',true),false);
assert.throws(() => registry.beginSave('overlay','/project/a.ts'),/DOCUMENT_SAVE_IN_PROGRESS/);
release();
release(); // idempotent
assert.equal(registry.writingUnder('/project',true),false);
registry.replace('overlay',[{path:'/project/a.ts',dirty:false}]);
registry.replace('coding',[{root:'/project',path:'a.ts',dirty:true}]);
assert.equal(registry.conflicts('overlay','/project/a.ts').length,1);
assert.equal(registry.dirtyUnder('/project',true),true);
registry.replace('coding',[]);
assert.equal(registry.dirtyUnder('/project',true),false);
const release2 = registry.beginSave('inline','/project/a.ts');
release2();
registry.replace('inline',[]);
registry.replace('overlay',[]);
assert.deepEqual(registry.snapshot(),[]);
console.log('CROSS_WORKBENCH_DOCUMENT_OWNERSHIP=PASS');
