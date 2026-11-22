import assert from 'node:assert/strict';
import {createDocumentOwnership, documentKey} from '../src/application/documentOwnership.mjs';
import {documentPathName, documentPathParent, joinDocumentPath, sameDocumentPath, containsDocumentPath, relativeDocumentPath, replaceDocumentPathPrefix} from '../src/application/documentPaths.mjs';

assert.equal(documentKey('/project/', 'src//a.ts'), '/project/src/a.ts');
assert.equal(documentKey('', '/project/src/./a.ts'), '/project/src/a.ts');
assert.throws(() => documentKey('', 'relative.ts'), /DOCUMENT_PATH_INVALID/, 'relative paths require an explicit absolute project root');
for(const invalid of ['../escape','a/../b','/project/../secret','']) {
  assert.throws(() => documentKey('/project', invalid), /DOCUMENT_PATH_INVALID/);
}
for (const alias of ['C:\\Work\\Main.rs', 'c:/work/main.rs', 'C:/WORK\\Main.rs', '\\\\?\\C:\\Work\\Main.rs']) {
  assert.equal(documentKey('',alias),'c:/work/main.rs');
}
assert.equal(documentKey('C:\\Work','src/main.rs'),'c:/work/src/main.rs');
assert.equal(documentKey('c:/Work/','src\\Main.rs'),'c:/work/src/main.rs');
assert.equal(documentKey('\\\\?\\C:\\Work','src/main.rs'),'c:/work/src/main.rs');
assert.equal(documentKey('', 'C:\\'), 'c:/');
assert.equal(documentKey('/project','src/./a.ts'),'/project/src/a.ts');
assert.notEqual(documentKey('','/project/A.ts'),documentKey('','/project/a.ts'));
assert.notEqual(documentKey('','C:/Work/a.ts'),documentKey('','D:/Work/a.ts'));
assert.equal(documentPathParent('C:\\Main.rs'),'C:\\');
assert.equal(documentPathParent('C:\\'),'C:\\');
assert.equal(documentPathParent('c:/Work/Main.rs'),'c:/Work');
assert.equal(documentPathParent('\\\\?\\C:\\Work\\Main.rs'),'\\\\?\\C:\\Work');
assert.equal(documentPathParent('/main.rs'),'/');
assert.equal(documentPathName('C:\\'),'C:\\');
assert.equal(documentPathName('C:\\Work\\Main.rs'),'Main.rs');
assert.equal(joinDocumentPath('C:\\','Work/Main.rs'),'C:\\Work\\Main.rs');
assert.equal(joinDocumentPath('\\\\?\\C:\\Work','src/main.rs'),'\\\\?\\C:\\Work\\src\\main.rs');
assert.equal(joinDocumentPath('/project','src/main.rs'),'/project/src/main.rs');
assert.equal(containsDocumentPath('c:/work','C:\\WORK\\src\\Main.rs'),true);
assert.equal(containsDocumentPath('c:/work','C:\\Worker\\Main.rs'),false);
assert.equal(sameDocumentPath('C:\\Work\\Main.rs','c:/work/main.rs'),true);
assert.equal(sameDocumentPath('/work/normal','/work/unsupported\\\\name'),false,'unsupported entries must not crash tree display predicates');
assert.equal(relativeDocumentPath('c:/work','C:\\WORK\\src\\Main.rs'),'src/Main.rs');
assert.equal(relativeDocumentPath('c:/work','D:\\Work\\Main.rs'),null);
assert.equal(replaceDocumentPathPrefix('C:\\WORK\\src\\Main.rs','c:/work','C:\\New'),'C:\\New\\src\\Main.rs');
assert.equal(replaceDocumentPathPrefix('C:\\Work\\Main.rs','c:/','D:\\'),'D:\\Work\\Main.rs');
assert.equal(replaceDocumentPathPrefix('/work-other/a','/work','/new'),'/work-other/a');
for (const invalid of [
  'C:relative', '\\root-relative', '\\\\server\\share\\a', '//server/share/a',
  '\\\\?\\UNC\\server\\share\\a', '\\\\.\\C:\\a', '\\??\\C:\\a',
  'C:\\Work\\..\\secret', 'C:\\Work\\file:stream', 'C:\\Work\\file.',
  'C:\\Work\\file ', 'C:\\Work\\NUL.txt', 'C:\\Work\\a\u0000b',
  '\\\\?\\C:\\Work/alias', '/project\\ambiguous',
]) {
  assert.throws(()=>documentKey('',invalid),/DOCUMENT_PATH_INVALID/,invalid);
}
assert.throws(()=>documentKey('C:\\Work','/drive-relative'),/DOCUMENT_PATH_INVALID/);
assert.throws(()=>joinDocumentPath('C:\\Work','D:\\escape'),/DOCUMENT_PATH_INVALID/);
assert.throws(()=>joinDocumentPath('C:\\Work','../escape'),/DOCUMENT_PATH_INVALID/);
assert.throws(()=>joinDocumentPath('/project','../escape'),/DOCUMENT_PATH_INVALID/);
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
assert.throws(()=>registry.beginMutation([{path:'/project',isDirectory:true}]),/DOCUMENT_CONCURRENT_DIRTY_EDIT/);
registry.replace('coding',[]);
assert.equal(registry.dirtyUnder('/project',true),false);
const release2 = registry.beginSave('inline','/project/a.ts');
release2();
const releaseMutation = registry.beginMutation([{path:'/project/dir',isDirectory:true}]);
assert.throws(()=>registry.beginSave('coding','/project/dir/file.ts'),/DOCUMENT_PATH_MUTATION_IN_PROGRESS/);
assert.throws(()=>registry.beginMutation([{path:'/project/dir/child',isDirectory:true}]),/DOCUMENT_PATH_MUTATION_IN_PROGRESS/);
const unrelatedSave=registry.beginSave('coding','/project/dir-other/file.ts');unrelatedSave();
releaseMutation();releaseMutation();
const afterMutation=registry.beginSave('coding','/project/dir/file.ts');afterMutation();
const activeWrite=registry.beginSave('coding','/project/dir/file.ts');
assert.throws(()=>registry.beginMutation([{path:'/project/dir',isDirectory:true}]),/DOCUMENT_SAVE_IN_PROGRESS/);
activeWrite();
registry.replace('inline',[]);
registry.replace('overlay',[]);
assert.deepEqual(registry.snapshot(),[]);
// Windows aliases must share both dirty ownership and write/mutation locks.
for (const alias of ['C:\\Work\\Main.rs', 'c:/work/main.rs', '\\\\?\\C:\\WORK\\Main.rs']) {
  const windows = createDocumentOwnership();
  windows.replace('coding', [{root:'C:\\Work',path:'Main.rs',dirty:true}]);
  assert.deepEqual(windows.conflicts('overlay',alias), ['coding']);
  assert.equal(windows.dirtyUnder('c:/',true),true,'a volume root contains its documents');
  assert.equal(windows.dirtyUnder('D:/',true),false,'different volumes remain distinct');
  assert.equal(windows.dirtyUnder('C:/Worker',true),false,'prefixes require a component boundary');
  assert.throws(()=>windows.beginSave('overlay',alias),/DOCUMENT_CONCURRENT_DIRTY_EDIT/);
  assert.throws(()=>windows.beginMutation([{path:'c:/work',isDirectory:true}]),/DOCUMENT_CONCURRENT_DIRTY_EDIT/);
  windows.replace('coding',[]);
  const release=windows.beginSave('overlay',alias);
  assert.equal(windows.writingUnder('C:\\',true),true);
  assert.throws(()=>windows.beginSave('inline','C:\\WORK\\MAIN.RS'),/DOCUMENT_SAVE_IN_PROGRESS/);
  release();
  const releaseMutation=windows.beginMutation([{path:'c:\\Work',isDirectory:true}]);
  assert.throws(()=>windows.beginSave('overlay',alias),/DOCUMENT_PATH_MUTATION_IN_PROGRESS/);
  const unrelated=windows.beginSave('overlay','C:\\Worker\\Main.rs');unrelated();
  releaseMutation();
}
// Native path mutations leave clean Coding buffers stale until explicitly
// closed/reopened. Keeping the same document on remount must not erase this.
const relocated=createDocumentOwnership();
const oldPath='C:\\Work\\Clean.rs';
relocated.replace('workspace',[{path:oldPath,dirty:false}]);
relocated.replace('coding',[{root:'c:/work',path:'CLEAN.RS',dirty:false}]);
let notifications=0;
const unsubscribe=relocated.onPathInvalidated('coding',()=>{notifications++;});
relocated.invalidatePath('c:/WORK',true,['workspace']);
assert.equal(notifications,1);
assert.equal(relocated.pathInvalidated('coding',oldPath),true);
assert.equal(relocated.pathInvalidated('workspace',oldPath),false,'the Owner that remaps its own buffers remains usable');
assert.throws(()=>relocated.beginSave('coding',oldPath),/DOCUMENT_PATH_CHANGED/);
relocated.replace('coding',[{root:'C:\\Work',path:'Clean.rs',dirty:true}]);
assert.throws(()=>relocated.beginSave('coding',oldPath),/DOCUMENT_PATH_CHANGED/,'typing or restoring the same retained buffer cannot re-authorize its old path');
unsubscribe();
relocated.replace('coding',[{path:oldPath,dirty:false},{path:'D:\\Other\\Next.rs',dirty:false}]);
relocated.invalidatePath('d:/other',true);
assert.equal(notifications,1,'an unmounted workbench has no retained listener');
assert.throws(()=>relocated.beginSave('coding','D:\\Other\\Next.rs'),/DOCUMENT_PATH_CHANGED/,'unmounted workbench metadata still protects retained documents');
relocated.replace('coding',[]);
relocated.replace('coding',[{path:oldPath,dirty:false}]);
const reopened=relocated.beginSave('coding',oldPath);reopened();
assert.equal(relocated.pathInvalidated('coding',oldPath),false,'explicit close and reopen starts a fresh native read contract');
const deletion=createDocumentOwnership();
deletion.replace('overlay',[{path:'/project/a.ts',dirty:false}]);
deletion.invalidatePath('/project/a.ts');
assert.throws(()=>deletion.beginSave('overlay','/project/a.ts'),/DOCUMENT_PATH_CHANGED/);
assert.equal(deletion.pathInvalidated('overlay','/project/a.ts/other'),false,'file invalidation does not include descendants');
console.log('CROSS_WORKBENCH_DOCUMENT_OWNERSHIP=PASS');
