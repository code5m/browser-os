import assert from 'node:assert/strict';
import {editorLanguage,createDocumentModel,applyVersionedDocumentEdit} from '../src/shared/editor/documentModel.mjs';
const tests=[['x.ts','typescript'],['x.js','javascript'],['x.rs','rust'],['x.py','python'],['x.sql','sql'],['x.md','markdown'],['x.json','json'],['x.unknown','plaintext']];
for(const [name,language] of tests)assert.equal(editorLanguage(name),language);
const created=new Map();
const fake={Uri:{from:parts=>({parts,toString(){return [parts.scheme,parts.authority,parts.path].join(':')}})},editor:{
 getModel:uri=>created.get(uri.toString())||null,
 createModel:(content,language,uri)=>{let value=content,version=1,disposed=false;const model={
 getValue:()=>value,setValue:next=>{value=next;version++},
 getVersionId:()=>version,isDisposed:()=>disposed,dispose:()=>{disposed=true},
 uri,language};created.set(uri.toString(),model);return model;}
}};
const model=createDocumentModel(fake,{path:'/project/file.ts',content:'before',namespace:'one'});
assert.equal(model.language,'typescript');
assert.throws(()=>createDocumentModel(fake,{path:'/project/file.ts',content:'other',namespace:'one'}),/ALREADY_OPEN/);
assert.throws(()=>applyVersionedDocumentEdit(model,2,'wrong'),/VERSION_CONFLICT/);
assert.equal(applyVersionedDocumentEdit(model,1,'after'),2);
assert.equal(model.getValue(),'after');
model.dispose();
assert.throws(()=>applyVersionedDocumentEdit(model,2,'later'),/DOCUMENT_CLOSED/);
console.log('BROWSEROS_SHARED_EDITOR_CONTRACT=PASS');
