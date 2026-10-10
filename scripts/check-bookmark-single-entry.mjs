#!/usr/bin/env node
// Guard the user-visible ONE bookmark entry and hot-plug consistency.
// The global slot contract remains available to external capabilities.
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
const read = p => readFileSync(new URL("../"+p, import.meta.url),"utf8");
const index = read("src/capabilities/bookmark/index.ts");
const manifest = read("src/capabilities/bookmark/manifest.ts");
const bar = read("src/components/layout/ActivityBar.vue");
const slots = read("src/capability/contribution/types.ts");
assert.doesNotMatch(index, /BookmarkEntryButton|bookmark\.entry-button/);
assert.doesNotMatch(manifest, /bookmark\.entry-button|activity-bar-trailing/);
assert.equal(existsSync(new URL("../src/capabilities/bookmark/ui/BookmarkEntryButton.vue",import.meta.url)), false);
for (const id of ["bookmark.sidebar","bookmark.address-star"]) {
  assert.ok(index.includes('id: "'+id+'"') && manifest.includes('id: "'+id+'"'), id+" missing manifest/runtime parity");
}
assert.ok(index.includes("contributionRegistry.registerContribution"), "live owner registration removed");
assert.ok(bar.includes('v-for="c in addressBarActions"'), "visible bookmark slot removed");
assert.doesNotMatch(bar, /v-for="c in trailingActions"/);
assert.ok(slots.includes("ACTIVITY_BAR_TRAILING"),"external slot compatibility must remain");
console.log("BOOKMARK_SINGLE_ENTRY_CONTRACT=PASS");
