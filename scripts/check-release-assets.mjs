import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../src-tauri/build.rs',import.meta.url),'utf8');
assert.match(source,/println!\("cargo:rerun-if-changed=\.\.\/dist"\)/);
const config=JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json',import.meta.url),'utf8'));
assert.equal(config.build.frontendDist,'../dist');
assert.equal(config.build.beforeBuildCommand,'npm run build');
assert.equal('devUrl' in config.build,false);
console.log('RELEASE_ASSET_TRACKING=PASS (build contract; not installed GUI proof)');
