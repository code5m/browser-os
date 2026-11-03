#!/usr/bin/env node
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {parse,compileTemplate} from '@vue/compiler-sfc';
const components={
  'database':{file:'src/capabilities/database/ui/DatabasePanel.vue',keys:['navigator','inspector','connections','schema'],labels:['database-navigator','database-inspector','database-connections','database-object-details']},
  'git':{file:'src/capabilities/git/ui/GitStudio.vue',keys:['toolbar','files','inspector'],labels:['git-workspace-tools','git-file-pane','git-commit-inspector']},
};
for(const [name,{file,keys,labels}] of Object.entries(components)){
  const source=readFileSync(file,'utf8');
  const persistence=name==='git'?readFileSync('src/capabilities/git/model/gitPreferences.ts','utf8'):source;
  if(name==='git'){assert.match(source,/readGitDisclosure/);assert.match(source,/writeGitDisclosure/);}
  for(const key of keys){
    assert.ok(persistence.includes(`${name==='database'?'browseros.db.ui':'browseros.git.ui'}`),`${name} must persist disclosure`);
    assert.ok(source.includes(`'${key}'`),`${name} missing ${key} fold`);
  }
  for(const label of labels)assert.ok(source.includes(label),`${name} missing aria panel ${label}`);
  assert.match(source,/:aria-expanded=/,`${name} fold state must be accessible`);
  assert.match(persistence,/localStorage\.setItem\(/,`${name} disclosure preferences must persist`);
  if(name==='database'){
    assert.match(source,/v-show="dbConnectionsOpen"/);
    assert.match(source,/v-show="dbSchemaOpen"/);
    assert.match(source,/db-no-sidebar/);
    assert.match(source,/db-no-inspector/);
    // The global Workbench may already be collapsed. A first click must
    // reveal the navigator, never invert its invisible true state into false.
    assert.match(source,/if \(key==='navigator' && workbench\.collapsed\) \{[\s\S]*?workbench\.toggleTools\(\);[\s\S]*?dbNavigatorOpen\.value=true;[\s\S]*?return;/);
    assert.match(source,/function openConnectionSettings\(\) \{[\s\S]*?if \(workbench\.collapsed\) workbench\.toggleTools\(\);[\s\S]*?connectionDialogOpen\.value=true;/);
    assert.match(source,/@click="openConnectionSettings"/);
    assert.match(source,/dbNavigatorOpen && !workbench\.collapsed \? '◧ 收起资源树' : '◫ 展开资源树'/);

  }else{
    assert.match(source,/git-tools-folded/);
    assert.match(source,/git-files-folded/);
    assert.match(source,/git-inspector-folded/);
    assert.match(source,/v-show="gitInspectorOpen"/);
    // Every visible file toggle refers to a real file panel on each pane.
    assert.match(source,/pane==='changes'\|\|pane==='stash'\|\|pane==='github'/);
    assert.match(source,/v-else-if="pane==='stash'"><aside id="git-file-pane"/);
    assert.match(source,/v-else-if="pane==='github'"\><aside id="git-file-pane"/);

  }
  const parsed=parse(source,{filename:file});assert.equal(parsed.errors.length,0,`${file} must parse`);
  const compiled=compileTemplate({source:parsed.descriptor.template.content,filename:file,id:`studio-${name}`});
  assert.equal(compiled.errors.length,0,`${file} must compile: ${compiled.errors.join('; ')}`);
  console.log(`${name.toUpperCase()}_FOLDING=PASS ${keys.join(',')} persisted and Vue template compiled`);
}
