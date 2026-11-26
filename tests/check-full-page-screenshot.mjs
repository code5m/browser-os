import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
const read=path=>readFileSync(new URL(path,import.meta.url),"utf8");
const manager=read("../tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs");
const native=read("../src-tauri/src/capabilities/browser/commands.rs");
const main=read("../src-tauri/src/main.rs");
const bridge=read("../src/bridge.ts");
const ui=read("../src/components/layout/ActivityBar.vue");
const manifest=read("../src/capabilities/browser/manifest.ts");
assert.match(manager,/SnapshotRegion::FullDocument/);
assert.match(manager,/Page\.captureScreenshot/);
assert.match(manager,/captureBeyondViewport/);
// The Windows screenshot path must not import webview2-com's private pwstr
// module. Keep UTF-16 storage live while passing public PCWSTR arguments.
assert.doesNotMatch(manager,/webview2_com::pwstr/);
assert.match(manager,/PCWSTR\(method\.as_ptr\(\)\)/);
assert.match(manager,/PCWSTR\(params\.as_ptr\(\)\)/);
assert.match(manager,/callback: impl FnOnce/);
assert.match(native,/webview\.label\(\) != "main"/);
assert.match(native,/CAPTURE_BUSY/);
assert.match(native,/CAPTURE_OUTCOME_UNKNOWN/);
assert.match(native,/let _ = capture_cancel\(&app, &ticket\);\s*return Err\("CAPTURE_OUTCOME_UNKNOWN"\.into\(\)\);/);
assert.match(native,/if receipt\.phase\.terminal\(\) \{\s*capture_ack\(&ticket\)\?/);
assert.match(native,/CAPTURE_DIMENSION_LIMIT/);
assert.match(native,/safe_paths::create_new_file\(/);
assert.match(main,/browser::commands::capture_full_page/);
assert.match(bridge,/captureFullPage/);
assert.match(ui,/aria-label="整页截图"/);
assert.match(ui,/bridge\.captureFullPage\(id\)/);
assert.match(manifest,/browser\.capture\.full-page/);
assert.doesNotMatch(native,/document\.cookie|access_token|eval_in_tab/);

// Renderer startup must verify both the budget and the parent admission.
const isolated=read("../src-tauri/src/isolated_capture.rs");
const supervisor=read("../src-tauri/src/native_supervisor.rs");
assert.match(isolated,/verify_current_render_budget\(MEMORY_LIMIT\)/);
assert.match(isolated,/release_start_pipe\(START_MAGIC\)/);
assert.match(isolated,/validate_worker_admission\(std::io::stdin\(\).lock\(\)\)/);
assert.match(native,/fn reserve_publication/);
assert.match(native,/CAPTURE_CANCELLED_BEFORE_PUBLICATION/);
assert.match(native,/native_entered_at\(\)/);
assert.match(supervisor,/pending_capture_domain_recovery/);
assert.match(supervisor,/claim_capture_domain_recovery/);
assert.match(bridge,/requestCaptureDomainRecovery/);
assert.match(ui,/target\.epoch,target\.sequence,target\.ticket/);
assert.match(native,/slot\.cancel_intent = true/);
assert.match(native,/isolated_capture::checked_png\(bytes\)/);
console.log("FULL_PAGE_CAPTURE_CONTRACT=PASS");
