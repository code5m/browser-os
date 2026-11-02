#!/usr/bin/env node
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {parse,compileTemplate} from '@vue/compiler-sfc';
const components={
  'database':{file:'src/capabilities/database/ui/DatabasePanel.vue',keys:['navigator','inspector','connections','schema'],labels:['database-navigator','database-inspector','database-connections','database-schema']},
  'git':{file:'src/capabilities/git/ui/GitStudio.vue',keys:['toolbar','files','inspector'],labels:['git-workspace-tools','git-file-pane','git-commit-inspector']},
};
for(const [name,{file,keys,labels}] of Object.entries(components)){
  const source=readFileSync(file,'utf8');
  for(const key of keys){
    assert.ok(source.includes(`${name==='database'?'browseros.db.ui':'browseros.git.ui'}`),`${name} must persist disclosure`);
    assert.ok(source.includes(`'${key}'`),`${name} missing ${key} fold`);
  }
  for(const label of labels)assert.ok(source.includes(label),`${name} missing aria panel ${label}`);
  assert.match(source,/:aria-expanded=/,`${name} fold state must be accessible`);
  assert.match(source,/localStorage\.setItem\(/,`${name} disclosure preferences must persist`);
  if(name==='database'){
    assert.match(source,/v-show="dbConnectionsOpen"/);
    assert.match(source,/v-show="dbSchemaOpen"/);
    assert.match(source,/db-no-sidebar/);
    assert.match(source,/db-no-inspector/);
  }else{
    assert.match(source,/git-tools-folded/);
    assert.match(source,/git-files-folded/);
    assert.match(source,/git-inspector-folded/);
    assert.match(source,/v-show="gitInspectorOpen"/);
  }
  const parsed=parse(source,{filename:file});assert.equal(parsed.errors.length,0,`${file} must parse`);
  const compiled=compileTemplate({source:parsed.descriptor.template.content,filename:file,id:`studio-${name}`});
  assert.equal(compiled.errors.length,0,`${file} must compile: ${compiled.errors.join('; ')}`);
  console.log(`${name.toUpperCase()}_FOLDING=PASS ${keys.join(',')} persisted and Vue template compiled`);
}
