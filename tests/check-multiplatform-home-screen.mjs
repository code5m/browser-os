import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import {PUBLISHING_PLATFORMS,platformById,officialPublisherUrl,matchesOfficialHost} from "../src/capabilities/wechat/model/publishingPlatforms.mjs";
const read=p=>readFileSync(new URL(p,import.meta.url),"utf8");
assert.equal(PUBLISHING_PLATFORMS.length,10);
assert.equal(new Set(PUBLISHING_PLATFORMS.map(p=>p.id)).size,10);
for(const p of PUBLISHING_PLATFORMS){
  assert.equal(new URL(p.url).protocol,"https:");
  assert.equal(new URL(p.url).hostname,p.officialHost);
  assert.equal(officialPublisherUrl(p.id),p.url);
  assert.equal(matchesOfficialHost(p.id,p.url),true);
  for(const bad of ["javascript:alert(1)","https://attacker.test/",`https://${p.officialHost}.evil.test/`,`https://a:b@${p.officialHost}/`])
    assert.equal(matchesOfficialHost(p.id,bad),false);
}
for(const id of ["wechat","baijiahao","toutiao","xiaohongshu","bilibili"])assert.ok(platformById(id));
assert.equal(officialPublisherUrl("unknown"),null);
const ui=read("../src/components/system/PublishingPlatformsPanel.vue");
assert.match(ui,/browser\.openBrowserAt\(url\)/);
assert.match(ui,/bridge\.wechatAccountStatus\(\)/);
assert.match(ui,/bridge\.wechatDraftPermissionCheck\(\)/);
assert.doesNotMatch(ui,/document\.cookie|localStorage|sessionStorage|evalInTab|access_token/);
assert.match(read("../src/components/system/WechatWorkbenchPanel.vue"),/v-model:selected-id="selectedPlatformId"/);
assert.match(read("../src/capabilities/home/ui/HomePanel.vue"),/📣 多平台内容工作台/);
assert.match(read("../src/capabilities/home/ui/HomePanel.vue"),/openWechatOfficial/);
assert.match(ui,/void inspectWechat\(\)/);
assert.match(read("../src/App.vue"),/layout\.activateHome\(\);\s*ready\.value = true/);
assert.match(read("../src/capabilities/wechat/manifest.ts"),/publishing\.platform\.directory/);
console.log("PUBLISHING_HOME=PASS");
