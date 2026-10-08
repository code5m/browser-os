#!/usr/bin/env node
import assert from "node:assert/strict";
import fs from "node:fs";
import { evaluateWorkflow, evaluateAll, WORKFLOWS, MAX_EVIDENCE_AGE_MS } from "../src/components/system/engineeringHealthModel.mjs";
const sha = "a".repeat(40), older = "b".repeat(40), now = Date.parse("2026-10-09T00:00:00Z");
const make = (name, extra = {}) => ({
 id: 123, name, event: "push", head_sha: sha,
 status: "completed", conclusion: "success",
 created_at: "2026-10-08T23:00:00Z", updated_at: "2026-10-08T23:20:00Z", ...extra,
});
assert.equal(evaluateWorkflow(WORKFLOWS[0].name, sha, [make(WORKFLOWS[0].name)], now).state, "PASS");
assert.equal(evaluateWorkflow(WORKFLOWS[0].name, sha, [make(WORKFLOWS[0].name,{conclusion:"failure"})], now).state, "FAIL");
assert.equal(evaluateWorkflow(WORKFLOWS[0].name, sha, [make(WORKFLOWS[0].name,{status:"in_progress",conclusion:null})], now).state, "RUNNING");
assert.equal(evaluateWorkflow(WORKFLOWS[0].name, sha, [make(WORKFLOWS[0].name,{head_sha:older})], now).state, "UNKNOWN");
assert.equal(evaluateWorkflow(WORKFLOWS[0].name, sha, [make(WORKFLOWS[0].name,{event:"pull_request"})], now).state, "UNKNOWN");
assert.equal(evaluateWorkflow(WORKFLOWS[0].name, sha, [make(WORKFLOWS[0].name,{updated_at:new Date(now-MAX_EVIDENCE_AGE_MS-1).toISOString()})], now).state, "STALE");
assert.equal(evaluateAll(sha, WORKFLOWS.map(w=>make(w.name)), now).overall, "PASS");
assert.equal(evaluateAll(sha, [], now).overall, "UNKNOWN");
assert.equal(evaluateAll(sha, [...WORKFLOWS.map(w=>make(w.name)),make(WORKFLOWS[0].name,{id:900,conclusion:"failure",created_at:"2026-10-08T23:30:00Z"})], now).overall, "FAIL");
const panel = fs.readFileSync("src/components/system/EngineeringHealthPanel.vue","utf8");
const settings = fs.readFileSync("src/components/system/SettingsPanel.vue","utf8");
for(const required of ["onMounted", "jsonRequest", "refresh()", "role=\"alert\"", "工程健康中心", "校验一致", "statusText"]){
  assert.ok(panel.includes(required), "panel missing "+required);
}
assert.ok(settings.includes("EngineeringHealthPanel"), "Settings must mount health");
assert.ok(settings.includes("工程健康"), "Settings needs Chinese entry");
console.log("ENGINEERING_HEALTH_UI_RESULT=PASS positive_and_negative_fixtures=9");
