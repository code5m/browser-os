#!/usr/bin/env node
import assert from 'node:assert/strict';
import {discoverIdeaProjects} from '../src/capabilities/database/model/ideaDiscovery.mjs';
let reads=[];
const nodes = {
 '/home/test': ['Documents','IdeaProjects','.config','node_modules'],
 '/home/test/Documents': ['Projects'],
 '/home/test/Documents/Projects': ['alpha'],
 '/home/test/Documents/Projects/alpha': [],
 '/home/test/IdeaProjects': ['beta'],
 '/home/test/IdeaProjects/beta': [],
};
const io = {
 listDir: async path => (nodes[path]||[]).map(name=>({name,is_dir:true,path:path+'/'+name})),
 readFile: async path => {
   reads.push(path);
   if (path === '/home/test/Documents/Projects/alpha/.idea/dataSources.xml') return '<project>A</project>';
   if (path === '/home/test/IdeaProjects/beta/.idea/dataSources.xml') return '<project>B</project>';
   throw new Error('missing');
 },
 parse: xml => [{id:xml.includes('A')?'a':'b',name:'MySQL',kind:'mysql',supported:true}],
};
const result=await discoverIdeaProjects(['/home/test'],io);
assert.equal(result.sources.length,2,'find two real project configs');
assert.equal(result.partial,false);
assert.ok(result.sources.some(x=>x.path.endsWith('/alpha')));
assert.ok(result.sources.some(x=>x.path.endsWith('/beta')));
assert.ok(reads.every(x=>x.endsWith('/.idea/dataSources.xml')));
assert.ok(!reads.some(x=>x.includes('/.config/')||x.includes('node_modules')));
const noAccess=await discoverIdeaProjects(['/home/test/unknown'],{...io,listDir:async()=>{throw Error('denied')}});
assert.equal(noAccess.sources.length,0);
assert.equal(noAccess.permissionErrors,1);
const canceled=await discoverIdeaProjects(['/home/test'],io,()=>true);
assert.equal(canceled.sources.length,0);
console.log('IDEA_BOUNDED_DISCOVERY=PASS');
