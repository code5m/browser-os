import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

// Static source-contract smoke checks only. They cannot establish Windows
// compilation, WebView2 reliability, native filesystem safety or GUI PASS.
const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const cas = read('../src-tauri/src/capabilities/workspace/cas.rs');
const safePaths = read('../src-tauri/src/capabilities/workspace/safe_paths.rs');
const safePathsWindows = read('../src-tauri/src/capabilities/workspace/safe_paths_windows.rs');
const moves = read('../src-tauri/src/capabilities/workspace/fs_cmds.rs');
const session = read('../src-tauri/src/session.rs');
const tabs = read('../tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs');
const cargo = read('../src-tauri/Cargo.toml');
const lock = read('../src-tauri/Cargo.lock');
const process = read('../src-tauri/src/windows_process.rs');
const winGrid = read('../src-tauri/src/grid_process_windows.rs');
const scriptRunner = read('../src-tauri/src/capabilities/script/script_runner.rs');
const terminal = read('../src-tauri/src/capabilities/terminal/terminal.rs');
const ptyCargo = read('../src-tauri/vendor/portable-pty/Cargo.toml');
const profileCargo = read('../tauri-browser-tabs/crates/tauri-plugin-browser-tabs/Cargo.toml');
const ptyApi = read('../src-tauri/vendor/portable-pty/src/lib.rs');
const ptyConpty = read('../src-tauri/vendor/portable-pty/src/win/conpty.rs');
const ptyWinSpawn = read('../src-tauri/vendor/portable-pty/src/win/psuedocon.rs');
const ptyWinAttr = read('../src-tauri/vendor/portable-pty/src/win/procthreadattr.rs');
const ptyWinChild = read('../src-tauri/vendor/portable-pty/src/win/mod.rs');
const bfc = read('../src-tauri/src/capabilities/bfc/commands.rs');
const windowsPipe = read('../src-tauri/src/windows_pipe.rs');
const gridIpc = read('../src-tauri/src/grid_ipc.rs');
const gridRuntime = read('../src-tauri/src/bridge/grid_runtime.rs');
const launchPolicy = read('../src-tauri/src/security_policy.rs');
const nativeService = read('../src-tauri/src/framework/commands.rs');
const fileCommands = read('../src-tauri/src/bridge/filesystem_commands.rs');
const windowsSandbox = read('../src-tauri/src/capabilities/agent/windows_sandbox.rs');
const windowsMcp = read('../src-tauri/src/capabilities/agent/coding_mcp_windows.rs');
const windowsLsp = read('../src-tauri/src/capabilities/agent/coding_lsp_windows.rs');
const linuxLsp = read('../src-tauri/src/capabilities/agent/coding_lsp.rs');
const windowsApps = read('../src-tauri/src/windows_apps.rs');
const systemCommands = read('../src-tauri/src/bridge/system_commands.rs');
const codingPublicApi = read('../src-tauri/src/capabilities/agent/public_api.rs');
const platformSecurity = read('../crates/browseros-platform-security/src/windows.rs');
const platformSecurityLib = read('../crates/browseros-platform-security/src/lib.rs');
const privateLease = read('../crates/browseros-platform-security/src/windows/lease.rs');
const packageAcl = read('../crates/browseros-platform-security/src/windows/appcontainer.rs');
const auditLock = read('../src-tauri/src/capabilities/workspace/audit_lock.rs');
const nativeTestRunner = read('../scripts/windows-native-test-runner.ps1');
const nativeSupervisor = read('../scripts/windows-native-test-supervisor.cs');
const nativeRequired = JSON.parse(read('../scripts/windows-native-required-tests.json'));
const nativeRecovery = read('../crates/browseros-platform-security/src/windows/recovery.rs');
const sandboxLifecycle = read('../src-tauri/src/capabilities/agent/windows_sandbox/lifecycle.rs');
const shutdown = read('../src-tauri/src/shutdown.rs');
const bridgeLifecycle = read('../src-tauri/src/bridge.rs');
const sessionExit = read('../src-tauri/src/capabilities/session/commands.rs');
const browserOwner = read('../src-tauri/src/capabilities/browser/commands.rs');
const browserRuntime = read('../src-tauri/src/bridge/tab_runtime.rs');
const recoveryPanel = read('../src/capabilities/agent/ui/SandboxRecoveryPanel.vue');
const terminalState = read('../src/capabilities/terminal/state/useTerminalStore.ts');
const nativeDispatcher = read('../.github/workflows/browseros-developer-feedback.yml');
const terminalCommands = read('../src-tauri/src/bridge/terminal_commands.rs');
const startup = read('../src-tauri/src/startup.rs');
const main = read('../src-tauri/src/main.rs');
const workspace = read('../src-tauri/src/capabilities/workspace/workspace.rs');
const tauriConfig = JSON.parse(read('../src-tauri/tauri.conf.json'));

assert.match(cargo, /keyring = \{ version = "3", features = \["sync-secret-service", "windows-native", "apple-native"\] \}/);
assert.match(lock, /name = "keyring"[\s\S]*?"windows-sys 0\.60\.2"/);
assert.match(cargo, /Win32_System_JobObjects/);
assert.match(process, /JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE/);
assert.match(windowsPipe, /PIPE_REJECT_REMOTE_CLIENTS/);
assert.match(windowsPipe, /GetNamedPipeClientProcessId/);
assert.match(gridIpc, /MAX_GRID_WIRE_BYTES/);
assert.match(process, /AssignProcessToJobObject/);
assert.match(process, /attach_suspended_pid/);
assert.match(process, /CREATE_SUSPENDED/);
assert.match(winGrid, /assign_suspended_pid/);
assert.match(winGrid, /CREATE_SUSPENDED/);
assert.match(scriptRunner, /creation_flags\(crate::windows_process::CREATE_SUSPENDED\)/);
assert.match(scriptRunner, /attach_suspended_pid/);
assert.match(process, /suspended_spawn_assigns_job_before_immediate_grandchild_can_run/);
assert.match(bfc, /health\.get\("ok"\)\.and_then\(Value::as_bool\) != Some\(true\)/);
assert.match(bfc, /verify_local_bfc_service/);
assert.match(bfc, /browseros-service-token/);
assert.match(bfc, /fake_loopback_port_occupant_cannot_receive_snapshot_before_identity_proof/);
assert.match(ptyCargo, /version = "0.8.1"/);
assert.match(ptyApi, /fn spawn_command_suspended/);
assert.match(ptyConpty, /spawn_command_suspended\(cmd\)/);
assert.match(ptyWinSpawn, /CREATE_SUSPENDED/);
assert.match(ptyWinAttr, /PROC_THREAD_ATTRIBUTE_JOB_LIST/);
assert.match(ptyWinAttr, /set_job_list/);
assert.match(ptyWinSpawn, /spawn_command_in_job/);
assert.match(ptyConpty, /spawn_command_in_job\(cmd, job/);
assert.match(ptyApi, /fn spawn_command_in_job/);
assert.match(process, /fn create_for_spawn/);
assert.match(process, /conpty_terminal_uses_atomic_job_list_and_reaps_immediate_grandchild/);
assert.match(terminal, /spawn_command_in_job\(cmd, job\.as_raw_handle\(\)\)/);
assert.match(terminal, /WINDOWS_TERMINAL_JOB_FAILED/);
assert.doesNotMatch(terminal, /spawn_command_suspended\(cmd\)/);
assert.doesNotMatch(ptyWinSpawn, /PSUEDOCONSOLE_INHERIT_CURSOR\s*\|/);
assert.match(ptyWinChild, /WaitForSingleObject\(handle, 0\)/);
assert.match(ptyWinChild, /terminate_handle/);
assert.match(process, /conpty_child_exit_code_259_is_terminal_and_kill_is_idempotent/);
assert.match(cas, /FILE_SAVE_PLATFORM_UNSUPPORTED/);
assert.doesNotMatch(cas, /save_if_unchanged_path\s*\(/);
assert.match(cas, /unsupported_platform_save_is_denied_without_mutating_the_file/);
assert.match(safePaths, /#\[path = "safe_paths_windows\.rs"\]/);
assert.match(safePathsWindows, /NtCreateFile/);
assert.match(safePathsWindows, /authorize_named_child/);
assert.match(safePathsWindows, /rename_replace_at/);
assert.match(safePathsWindows, /copy_dacl/);
assert.match(safePathsWindows, /windows_cas_pinned_parent_survives_junction_replacement/);
assert.match(safePathsWindows, /windows_agent_atomic_write_creates_nested_files_without_touching_junction_target/);
assert.match(windowsSandbox, /CreateAppContainerProfile/);
assert.match(windowsSandbox, /PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES/);
assert.match(windowsSandbox, /CapabilityCount: 0/);
assert.match(windowsSandbox, /PROC_THREAD_ATTRIBUTE_HANDLE_LIST/);
assert.match(windowsSandbox, /PROC_THREAD_ATTRIBUTE_JOB_LIST/);
assert.match(windowsSandbox, /NamedPrivateJob/);
assert.match(sandboxLifecycle, /NamedPrivateJob::create/);
assert.match(nativeRecovery, /JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE/);
assert.match(windowsSandbox, /ResumeThread/);
assert.match(windowsMcp, /MCP_EXPLICIT_APPROVAL_REQUIRED/);
assert.match(windowsMcp, /MCP_SHELL_COMMAND_REJECTED/);
assert.match(windowsMcp, /real_appcontainer_stdio_initializes_lists_calls_and_scrubs_secrets/);
assert.match(windowsLsp, /protect_lsp_snapshot/);
assert.match(windowsLsp, /LSP_METHOD_DISABLED/);
assert.match(windowsLsp, /real_rust_analyzer_runs_against_private_snapshot/);
assert.match(windowsLsp, /LSP_APPROVAL_REQUIRED/);
assert.match(windowsLsp, /LSP_SESSION_WORKSPACE_MISMATCH/);
assert.match(windowsLsp, /fn cancel_pending_for/);
// One-shot Linux-compatible LSP document requests remain isolated from
// Windows' persistent session protocol and require explicit module approval.
assert.match(windowsLsp, /fn one_shot_document_request/);
assert.match(windowsLsp, /fn start_documents/);
assert.match(windowsLsp, /LSP_MODULE_APPROVAL_REQUIRED/);
assert.match(windowsLsp, /struct OneShotPermit/);
assert.match(windowsLsp, /single_document_uri_rewrite_does_not_expose_other_file_urls/);
assert.match(windowsLsp, /closing_one_workspace_does_not_cancel_another_workspace_start/);
assert.match(windowsLsp, /if session\.closed/);
assert.doesNotMatch(windowsLsp, /cache\.epoch/);
assert.match(windowsSandbox, /sandbox_prerequisites_available/);
assert.match(windowsSandbox, /canonical\.parent\(\) == Some\(system32\.as_path\(\)\)/);
assert.match(windowsMcp, /MCP_CONFIG_REPARSE_DENIED/);
// Native identity and a shared ACL implementation replace localized CLI text.
assert.match(platformSecurityLib, /cfg\(windows\)/);
assert.match(safePathsWindows, /browseros_platform_security::windows/);
assert.match(safePathsWindows, /ensure_private_directory/);
assert.match(safePathsWindows, /handle_is_within_root/);
assert.match(safePathsWindows, /path_matches_handle/);
assert.match(safePathsWindows, /file_identity/);
assert.doesNotMatch(safePathsWindows, /whoami\.exe|icacls\.exe/);
assert.doesNotMatch(safePathsWindows, /fn normalized_path/);
assert.match(safePathsWindows, /fn remove_tree/);
assert.match(safePathsWindows, /mark_object_deleted/);
assert.match(safePaths, /libc::fdopendir/);
assert.match(safePaths, /pub fn remove_tree/);
assert.match(fileCommands, /safe_paths::remove_tree/);
assert.match(fileCommands, /fn write_file_authorized/);
assert.match(fileCommands, /ErrorKind::NotFound/);
assert.match(fileCommands, /native::create_new_file/);
assert.match(fileCommands, /safe_paths::list_directory/);
assert.doesNotMatch(fileCommands, /std::fs::read_dir\(&dir\)/);
assert.match(safePaths, /pub fn list_directory/);
assert.match(safePaths, /libc::AT_SYMLINK_NOFOLLOW/);
assert.match(safePathsWindows, /fn list_directory/);
assert.match(safePathsWindows, /check_pinned\(\)\?/);
assert.doesNotMatch(fileCommands, /std::fs::remove_dir_all\(&canonical\)/);
assert.match(windowsApps, /Get-StartApps/);
assert.match(windowsApps, /fn valid_packaged_id/);
assert.match(windowsApps, /launch_packaged_app/);
assert.match(gridRuntime, /installed_packaged_apps/);
assert.match(nativeService, /strip_prefix\("appsfolder:"\)/);
assert.match(systemCommands, /crate::session::atomic_write/);
assert.match(codingPublicApi, /"lsp_start", "lsp_request", "lsp_diagnostics", "lsp_close"/);
assert.ok(windowsLsp.includes('persistent_response_uris_mask_unreviewed_locations'));
assert.ok(windowsLsp.includes('skipped_busy_candidates'));
assert.ok(windowsLsp.includes('symbol_only.then_some(0)'));
assert.ok(linuxLsp.includes('if managed.closed'));
assert.ok(linuxLsp.includes('session.closed = true'));
assert.ok(linuxLsp.includes('skipped_busy_candidates'));
assert.ok(linuxLsp.includes('symbol_only.then_some(0)'));
assert.ok(safePaths.includes('fn entry_still_matches'));
assert.ok(safePaths.includes('pinned_file_identity_recheck_detects_replaced_name'));
assert.match(windowsLsp, /fn get_session\(root: &Path, id: &str\)/);
// Guard against recurring Windows AppContainer launch and isolation defects.
assert.match(windowsSandbox, /TcpListener::bind/);
assert.match(windowsSandbox, /host-file positive control invalid/);
assert.match(windowsSandbox, /\.raw_arg\(&read_command\)/);
assert.match(windowsSandbox, /fn process_command_line/);
assert.match(windowsSandbox, /command\.extend\(tail\.encode_utf16\(\)\)/);
assert.match(windowsSandbox, /OsStr::new\("runtime"\)/);
assert.doesNotMatch(windowsSandbox, /let directory = state\.join\("runtime"\)/);
assert.match(windowsSandbox, /AppContainerDirectory::open_profile/);
assert.match(windowsSandbox, /"cmd\.exe", "curl\.exe"/);
assert.match(windowsSandbox, /environment_block\(environment, current_directory, &profile\.folder\)/);
assert.doesNotMatch(windowsSandbox, /std::env::var\("LOCALAPPDATA"\)/);
assert.doesNotMatch(windowsSandbox, /"\/S"\.into\(\)/);
assert.doesNotMatch(windowsSandbox, /\(A;;GA;;;AC\)/);
assert.doesNotMatch(windowsSandbox, /S-1-15-2-1:\(OI\)\(CI\)M/);
assert.match(windowsSandbox, /SANDBOX_CREATE_PROCESS/);
assert.match(windowsSandbox, /OwnedSecurityDescriptor/);
assert.match(windowsSandbox, /pub fn stderr_diagnostic/);
assert.match(windowsSandbox, /LSP_RUSTC_CFG_PROBE_FAILED/);
assert.match(windowsLsp, /rustup_analyzer_binary/);
assert.match(windowsLsp, /LSP_SANDBOX_START_FAILED:\{error\}/);
assert.match(windowsMcp, /MCP_START_FAILED:\{error\}/);
assert.match(windowsMcp, /MCP_PAYLOAD_NOT_FOUND/);
assert.doesNotMatch(windowsLsp, /sandbox_available":false/);
assert.match(moves, /FILE_MOVE_PLATFORM_UNSUPPORTED/);
assert.match(session, /\.create_new\(true\)/);
assert.match(session, /\.sync_all\(\)/);
assert.match(session, /\.json\.tmp/);
assert.match(tabs, /PageLoadEvent::Started/);
assert.match(tabs, /PageLoadEvent::Finished/);
assert.match(tabs, /browser_profile_path/);
assert.match(tabs, /fn prepared_browser_profile/);
assert.match(tabs, /pub fn prepare_storage/);
assert.match(tabs, /PreparedPrivateDirectory::prepare/);
assert.doesNotMatch(tabs, /PROFILE_AUDIT|verified_at|from_secs\(600\)/);
assert.match(tabs, /secure_windows_browser_profile/);
assert.match(tabs, /browseros_platform_security::windows/);
assert.match(tabs, /ensure_private_directory/);
assert.match(tabs, /ensure_private_directory_root/);
assert.match(tabs, /audit_private_directory/);
assert.match(tabs, /root_identity/);
assert.doesNotMatch(tabs, /whoami\.exe|icacls\.exe/);
assert.match(tabs, /builder\.data_directory\(profile\)/);
assert.match(tabs, /webview\.eval_with_callback\(js, move \|raw_json\|/);
assert.match(tabs, /decode_native_eval_result\(&raw_json\)/);
assert.doesNotMatch(tabs, /GRID_PLATFORM_UNSUPPORTED/);
assert.doesNotMatch(tabs, /on_navigation\(move \|next_url\|/);

// Platform parity must follow actual native commands, not cfg-only stubs.
assert.match(gridRuntime, /fn list_windows_apps_blocking/);
assert.match(gridRuntime, /GetSystemDirectoryW/);
assert.match(gridRuntime, /GetFolderPath/);
assert.match(gridRuntime, /cfg\(target_os = "linux"\)\]\s*#\[tauri::command\]\s*pub fn list_apps/);
assert.match(gridRuntime, /cfg\(target_os = "windows"\)\]\s*#\[tauri::command\]\s*pub async fn list_apps/);
assert.match(launchPolicy, /fn launch_basename/);
assert.match(launchPolicy, /"rundll32", "regsvr32"/);
assert.match(launchPolicy, /fn resolve_program_file/);
assert.match(launchPolicy, /blocked.*Windows|Windows built-in LOLBins/);
assert.match(nativeService, /require_main\(&webview, "launch_app"\)/);
assert.match(fileCommands, /save_if_unchanged\s*\(/);
assert.match(fileCommands, /create_new_file\s*\(/);
assert.match(fileCommands, /create_directory_all\s*\(/);
assert.match(fileCommands, /rename_no_replace\s*\(/);
assert.match(fileCommands, /remove_regular\s*\(/);
assert.doesNotMatch(fileCommands, /std::fs::write\(&p, content/);
assert.match(safePaths, /pub fn create_new_file/);
assert.match(safePaths, /pub fn create_directory_all/);
assert.match(safePaths, /native_create_rename_delete_use_pinned_parents_and_no_overwrite/);
assert.match(safePaths, /pub fn rename_no_replace/);
assert.match(safePathsWindows, /pub\(super\) fn create_new_file/);
assert.match(safePathsWindows, /pub\(super\) fn create_directory_all/);
assert.match(safePathsWindows, /pub\(super\) fn rename_no_replace/);
assert.match(gridRuntime, /pub\(crate\) fn mem_total_mb\(\)/);
const resourceStats = read('../src-tauri/src/bridge/tab_runtime.rs');
assert.match(resourceStats, /GetProcessMemoryInfo/);
assert.match(resourceStats, /CreateToolhelp32Snapshot/);
assert.match(resourceStats, /mem_total_mb: mem_total_mb\(\)/);
const dbProfiles = read('../src-tauri/src/capabilities/database/profiles.rs');
const agentChannel = read('../src-tauri/src/framework/agent_channel.rs');
const gitStudio = read('../src-tauri/src/capabilities/git/studio.rs');
const gitLocal = read('../src-tauri/src/capabilities/git/local_workbench.rs');
const imageStore = read('../src-tauri/src/shared/images.rs');
assert.match(dbProfiles, /session::atomic_replace_file/);
assert.match(agentChannel, /session::atomic_replace_file/);
assert.match(gitStudio, /safe_paths::save_if_unchanged/);
assert.match(gitLocal, /safe_paths::save_if_unchanged/);
assert.match(imageStore, /safe_paths::create_new_file/);
assert.doesNotMatch(imageStore, /fs::rename\(&tmp, &target\)/);
assert.match(read('../src-tauri/src/native_58_gui_review.rs'), /NATIVE_58_GUI_REVIEW_LINUX_ONLY/);
assert.match(read('../src-tauri/src/workbench_smoke.rs'), /WORKBENCH_NATIVE_SMOKE_LINUX_ONLY/);

// Native path policy is platform-specific and unresolved roots never grant
// access. A Linux case-variant sibling must never inherit an allowed root.
assert.match(launchPolicy, /fn same_component/);
assert.match(launchPolicy, /path_is_within_root/);
assert.match(launchPolicy, /paths_are_same_file/);
assert.match(launchPolicy, /linux_differently_cased_sibling_never_inherits_workspace_access/);
assert.match(launchPolicy, /non_utf8_linux_names_do_not_merge_in_root_matching/);
assert.match(gitStudio, /fn read_worktree_file/);
assert.match(gitStudio, /case_variant_symlink_never_exposes_other_git_workspace/);
assert.doesNotMatch(gitStudio, /fs::read_to_string\(file\)/);
assert.doesNotMatch(gitStudio, /hash\.update\(fs::read\(path\)/);
// Only a package SID may read private snapshot *files*. The Windows
// restricted lowbox traverses directories through non-inheritable traverse-only
// AppContainer ACEs; those grants must never include listing or reach source files.
assert.match(windowsLsp, /protect_lsp_snapshot/);
assert.match(packageAcl, /if directory && self\.directory_traversal/);
assert.match(packageAcl, /&self\.all_packages, FILE_TRAVERSE, 0/);
assert.doesNotMatch(packageAcl, /&self\.all_packages, FILE_GENERIC_READ/);
assert.match(packageAcl, /&self\.package, self\.mask\(\), flags/);
assert.match(packageAcl, /fn exact_app_descriptor/);
assert.match(packageAcl, /seen\[slot\]/);
assert.match(packageAcl, /PACKAGE_SECURITY_READBACK/);
assert.match(packageAcl, /package_native_repair_removes_world_and_other_container_from_every_object/);
assert.match(packageAcl, /package_exact_policy_rejects_same_sid_deny_wrong_mask_and_inherited_traversal/);
assert.match(packageAcl, /package_tree_repairs_are_bounded_and_never_follow_a_hardlink/);
assert.match(packageAcl, /traversal_parser_rejects_counted_sid_beyond_ace_bounds/);
assert.match(windowsSandbox, /appcontainer_own_state_create_write_read_and_cross_package_denial/);
assert.match(windowsSandbox, /own_package_reads_snapshot_but_cannot_overwrite_source/);
assert.match(windowsSandbox, /runtime_actual_byte_budget_rejects_before_destination_write/);
assert.match(windowsSandbox, /repeated_runtime_component_compares_bytes_without_mutating_open/);
assert.match(windowsSandbox, /stderr_diagnostics_are_bounded_sticky_and_path_free/);
assert.doesNotMatch(windowsSandbox, /icacls\.exe|\/grant:r|\/inheritance:r/);
assert.match(windowsLsp, /appcontainer-no-network-private-source-copy/);
assert.match(windowsSandbox, /LSP_CHILD_NULL_DEVICE_DENIED/);
assert.match(windowsLsp, /\.profile\(\)\.stderr_diagnostic\(\)/);
assert.match(windowsSandbox, /SANDBOX_RUNTIME_STAGE_CONFLICT/);
assert.match(windowsSandbox, /existing\.read_exact\(&mut actual/);
const v2Workflow = read('../.github/workflows/windows-linux-v2-experimental.yml');
assert.match(v2Workflow, /Generate and verify NSIS SHA256SUMS/);
assert.match(v2Workflow, /Get-FileHash -LiteralPath .* -Algorithm SHA256/);
// Native jobs are opt-in and all evidence is bound to one requested commit.
assert.match(v2Workflow, /workflow_call:/);
assert.doesNotMatch(v2Workflow, /workflow_dispatch:/);
assert.match(nativeDispatcher, /workflow_dispatch:/);
assert.match(nativeDispatcher, /github\.event_name == 'workflow_dispatch' && inputs\.windows_native == true/);
assert.match(nativeDispatcher, /uses: \.\/\.github\/workflows\/windows-linux-v2-experimental\.yml/);
assert.match(nativeDispatcher, /quick:\s+if: inputs\.windows_native != true/);
assert.match(v2Workflow, /WORKFLOW_SOURCE_SHA/);
assert.doesNotMatch(v2Workflow, /^  (push|pull_request|schedule):/m);
assert.match(v2Workflow, /source_sha:/);
assert.ok(v2Workflow.includes('^[0-9a-fA-F]{40}$'));
assert.match(v2Workflow, /ref: \$\{\{ needs\.source\.outputs\.sha \}\}/);
assert.ok(v2Workflow.includes('$matrixPassed = $nativeSelections.Count - $failedSelections.Count'));
assert.ok(v2Workflow.includes('$fullSuiteFailure = $null'));
assert.doesNotMatch(v2Workflow, /\$failedSelections\.Add\('windows-full-suite/);
assert.match(nativeTestRunner, /Get-NativeSelectionManifest/);
assert.ok(nativeTestRunner.includes('$selection.required_count -ne $required.Count'));
assert.match(nativeTestRunner, /NATIVE_REQUIRED_NAME_MISSING_OR_AMBIGUOUS/);
assert.match(nativeTestRunner, /NATIVE_REQUIRED_TEST_DID_NOT_PASS/);
assert.match(nativeTestRunner, /NATIVE_TEST_ENUMERATION_EXECUTION_MISMATCH/);
assert.ok(nativeTestRunner.includes('$result.ignored -ne 0'));
assert.match(nativeTestRunner, /NATIVE_REQUIRED_TESTS_NOT_EXECUTED/);
assert.match(nativeTestRunner, /SKIP\(\?:PED\)/);
assert.match(nativeTestRunner, /PASS_WITH_IGNORED/);
assert.match(v2Workflow, /GUI: NOT_RUN/);
assert.match(v2Workflow, /Vendored ConPTY and process waiter regression/);
assert.match(v2Workflow, /Get-NativeRustTestBinary -Manifest 'src-tauri\/Cargo.toml' -Package 'portable-pty'/);
assert.match(v2Workflow, /Get-NativeRustTestBinary -Manifest 'crates\/browseros-platform-security\/Cargo.toml' -Package 'browseros-platform-security'/);
assert.match(v2Workflow, /Get-NativeRustTestBinary -Manifest 'src-tauri\/Cargo.toml' -Package 'tauri-plugin-browser-tabs'/);
assert.match(nativeTestRunner, /cargo test --manifest-path \$Manifest --locked -p \$Package --lib --no-run/);
assert.match(v2Workflow, /-SelectionId 'windows-full-suite'/);
assert.match(v2Workflow, /Get-NativeSelectionManifest/);
assert.match(nativeTestRunner, /NATIVE_JOB_CLEANUP_UNCONFIRMED/);
assert.doesNotMatch(nativeTestRunner, /\.Kill\(\$true\)/);
assert.match(nativeSupervisor, /JOB_LIST/);
assert.match(nativeSupervisor, /HANDLE_LIST/);
assert.match(nativeSupervisor, /IsProcessInJob/);
assert.match(nativeSupervisor, /QueryInformationJobObject/);
assert.match(nativeSupervisor, /accounting\.ActiveProcesses == 0/);
assert.match(nativeSupervisor, /CleanupComplete/);
assert.equal(nativeRequired.schema_version, 1);
for (const selection of nativeRequired.selections) {
  assert.equal(selection.required_count, selection.required_names.length);
  assert.ok(selection.required_count > 0);
}
assert.match(v2Workflow, /BROWSEROS_NATIVE_CLEANUP_BLOCKED/);
assert.match(v2Workflow, /LINUX_REQUIRED_FRAMING_NOT_EXECUTED/);
// Cargo can test this non-workspace path dependency via the host manifest
// because it declares no dev-dependencies. Adding any requires revisiting
// membership/lock ownership instead of quietly abandoning these tests.
assert.doesNotMatch(ptyCargo, /^\s*\[[^\]\n]*dev-dependencies(?:\.[^\]\n]+)?\]/m);
assert.doesNotMatch(profileCargo, /^\s*\[[^\]\n]*dev-dependencies(?:\.[^\]\n]+)?\]/m);

// Tauri can turn setup errors into a Ready-event panic. Both entry points
// report native preparation failures themselves and stop before any WebView.
// Config-created windows would precede this gate, so that list stays empty.
assert.deepEqual(tauriConfig.app?.windows, []);
assert.match(workspace, /pub fn prepare_data_dir/);
assert.match(workspace, /PRIVATE_DATA_DIR_PREPARATION_FAILED/);
assert.match(workspace, /PRIVATE_DATA_DIR_PARENT_PREPARATION_FAILED/);
assert.match(workspace, /ensure_private_directory_root/);
assert.match(workspace, /ensure_private_directory_root\([\s\S]*?PreparedPrivateDirectory::prepare\(/);
assert.match(workspace, /_preparation: preparation/);
assert.match(workspace, /prepare_application_data_directory\(&base\)/);
assert.match(workspace, /app\.manage\(PreparedDataDirectory/);
const dataDirectoryGetter = workspace.slice(
  workspace.indexOf('pub fn data_dir('),
  workspace.indexOf('\nfn ensure(')
);
assert.ok(dataDirectoryGetter.length > 0);
assert.match(dataDirectoryGetter, /PreparedDataDirectory/);
assert.doesNotMatch(dataDirectoryGetter, /ensure_private_directory|create_dir|panic!/);
assert.match(startup, /crate::workspace::prepare_data_dir\(app\)\?/);
assert.match(startup, /MessageBoxW/);
assert.match(startup, /PRIVATE_DATA_DIR/);
assert.equal(main.match(/if let Err\(error\) = startup::prepare_workspace\(app\.handle\(\)\)/g)?.length, 2);
assert.equal(main.match(/startup::report_failure\(&error\);\s*std::process::exit\(1\);/g)?.length, 2);
assert.doesNotMatch(main, /startup::prepare_workspace\(app\.handle\(\)\)\?/);

// Single-instance, cold argv and platform Opened callbacks must all cross the
// bounded in-memory gate. Preparation publishes readiness and drains the
// queue atomically under its one mutex, then invokes handlers outside the lock.
assert.equal(main.match(/manage\(startup::PendingOpenUrls::default\(\)\)/g)?.length, 2);
assert.equal(main.match(/startup::dispatch_open_url\(/g)?.length, 3);
assert.doesNotMatch(main, /bridge::handle_open_url\(/);
assert.match(startup, /MAX_PENDING_OPEN_URLS: usize = 64/);
assert.match(startup, /MAX_OPEN_URL_BYTES: usize = 8192/);
assert.match(startup, /url\.len\(\) > MAX_OPEN_URL_BYTES/);
assert.match(startup, /state\.pending\.len\(\) >= MAX_PENDING_OPEN_URLS/);
assert.match(startup, /state\.pending\.push_back\(url\.to_owned\(\)\)/);
assert.match(startup, /drop\(state\);\s*crate::bridge::handle_open_url\(app, url\)/);
assert.match(startup, /STARTUP_QUEUE_UNAVAILABLE/);
assert.match(startup, /STARTUP_QUEUE_POISONED/);
const releaseOpenUrls = startup.slice(
  startup.indexOf('fn release_pending_open_urls('),
  startup.indexOf('pub fn prepare_workspace(')
);
assert.ok(releaseOpenUrls.length > 0);
assert.match(releaseOpenUrls, /let pending = \{\s*let mut state = queue\.0\.lock\(\)[\s\S]*?state\.ready = true;\s*std::mem::take\(&mut state\.pending\)\s*\};\s*for url in pending/);
assert.match(startup, /pub fn prepare_workspace\([\s\S]*?prepare_data_dir\(app\)\?;[\s\S]*?release_pending_open_urls\(app\)/);

// Non-Cargo LSP project-mode safety: explicitly supply bounded source graphs
// with Cargo/build scripts/proc macros disabled. A separately staged rustc
// peer is required for cfg/data-layout probes; its failure is surfaced before
// launch. This remains IDE analysis, not compilation verification.
assert.match(windowsLsp, /fn write_reviewed_rust_project/);
assert.match(windowsLsp, /fn reviewed_project_for_initialize/);
assert.match(windowsLsp, /project_initialize_graph_is_inline_and_refuses_external_roots/);
assert.match(windowsLsp, /LSP_PROJECT_SOURCE_OUTSIDE_SNAPSHOT/);
assert.match(windowsLsp, /LSP_SNAPSHOT_HOST_STATE/);
assert.match(windowsLsp, /LSP_SNAPSHOT_ROOT_ACES/);
assert.match(windowsLsp, /own lowbox could not read source\/manifest/);

assert.match(windowsLsp, /"linkedProjects":\[reviewed_graph\]/);
assert.match(windowsLsp, /"sysroot":null/);
assert.match(windowsLsp, /reviewed_rust_project_requires_local_sources_and_excludes_cargo_spawns/);
assert.match(windowsLsp, /private_snapshot_requires_own_package_sid_and_rejects_other_profiles/);
assert.match(windowsLsp, /own lowbox source file read failed/);
assert.match(windowsLsp, /snapshot_acl_for_test/);

const sandboxedLspStart = windowsLsp.slice(
  windowsLsp.indexOf('fn start_with_snapshot('),
  windowsLsp.indexOf('fn begin_request(')
);
assert.ok(sandboxedLspStart.length > 0);
assert.match(sandboxedLspStart, /None => runtime_with_budget\(&operation\)\?/);
assert.match(sandboxedLspStart, /rustc_cfg_environment\(&profile, &server\)\?/);
assert.doesNotMatch(sandboxedLspStart, /if let Ok\(.*runtime/);
// Any project source files are copied before package ACLs are locked.
assert.match(windowsLsp, /write_reviewed_rust_project_with_budget\(&destination, operation\)\?/);
assert.match(windowsLsp, /write_reviewed_rust_project_with_budget\(&snapshot, &operation\)\?/);
// Shared LOLBin guard applies to MCP as well as the native app launcher.
assert.match(launchPolicy, /pub\(crate\) fn is_blocked_executable_path/);
assert.match(windowsMcp, /security_policy::is_blocked_executable_path/);
assert.match(windowsMcp, /windows_lolbin_wrappers_are_rejected_without_spawning_sandbox/);
// MCP results are untrusted even in JSON object keys, not just values.
assert.match(windowsMcp, /std::mem::take\(items\)/);
assert.match(windowsMcp, /credential_in_json_key_is_redacted_in_nested_mcp_results/);
// Local drive and verbatim-drive paths authorize through native identity.
assert.match(platformSecurity, /Prefix::Disk/);
assert.match(platformSecurity, /Prefix::VerbatimDisk/);
assert.match(platformSecurity, /native_path_identity_rejects_a_sibling_and_accepts_own_child/);

// Follow-up: four real Windows LSP requests, private source snapshots,
// AppContainer SID isolation, and ConPTY creation/child lifetime.
const nativeLspStart = windowsLsp.slice(
  windowsLsp.indexOf('fn start_with_snapshot('),
  windowsLsp.indexOf('fn begin_request(')
);
assert.match(nativeLspStart, /"linkedProjects":\[reviewed_graph\]/);
assert.match(nativeLspStart, /\.spawn\(&server, &\[\], &environment, &state\)/);
assert.match(nativeLspStart, /rustc_cfg_environment\(&profile, &server\)\?/);
const rustcCfgEnvironment = windowsLsp.slice(
  windowsLsp.indexOf('fn rustc_cfg_peer('),
  windowsLsp.indexOf('fn lsp_toolchain_contract(')
);
assert.ok(rustcCfgEnvironment.includes('directory.join("rustc.exe")'));
assert.match(rustcCfgEnvironment, /stage_runtime_peer\(&peer\)/);
assert.match(rustcCfgEnvironment, /LSP_RUSTC_CFG_STAGE_FAILED:\{error\}/);
assert.doesNotMatch(rustcCfgEnvironment, /"cargo(?:\.exe)?"/);
assert.match(windowsLsp, /LSP_RUSTC_CFG_STAGE_FAILED/);
assert.match(windowsLsp, /fn rustc_cfg_peer/);
assert.match(windowsLsp, /LSP_RUSTC_CFG_PEER_UNSAFE/);
assert.match(windowsLsp, /LSP_RUSTUP_SETTINGS_UNREADABLE/);
assert.ok(windowsLsp.includes('"cargo_enabled":false'));
assert.ok(windowsLsp.includes('"rustc_cfg_probe":"required"'));
assert.ok(nativeLspStart.includes('"buildScripts":{"enable":false}'));
assert.ok(nativeLspStart.includes('"procMacro":{"enable":false}'));
assert.match(windowsLsp, /cfg_peer_missing_or_directory_fails_before_runtime_spawn/);
assert.match(windowsLsp, /fn shutdown\(&mut self\)/);
assert.match(windowsLsp, /LSP_CLEANUP_TIMEOUT/);
assert.match(windowsLsp, /LSP_PROCESS_CLEANUP_INCOMPLETE/);
assert.match(windowsLsp, /close_complete/);
assert.match(windowsLsp, /cleanup_errors/);
assert.match(windowsLsp, /"HOVER",/);
assert.match(windowsLsp, /"DEFINITION",/);
assert.match(windowsLsp, /"SYNTAX_DIAGNOSTICS",/);
assert.match(windowsLsp, /"SYMBOLS",/);
assert.match(windowsLsp, /legacy_cargo_subprocess_probes_are_diagnostics_only/);
assert.match(windowsLsp, /private_snapshot_requires_own_package_sid_and_rejects_other_profiles/);
assert.match(windowsLsp, /write-negative profile/);
const snapshotAcl = windowsSandbox.slice(
  windowsSandbox.indexOf('fn protect_selected_tree('),
  windowsSandbox.indexOf('pub fn stage_runtime_peer(')
);
assert.ok(snapshotAcl.length > 0);
assert.match(snapshotAcl, /AppContainerAccess::ReadExecute/);
assert.match(snapshotAcl, /directory\.protect_tree/);
assert.doesNotMatch(snapshotAcl, /AppContainerAccess::Modify/);
assert.match(windowsSandbox, /PRIVATE_TREE_MAX_ENTRIES/);
assert.match(windowsSandbox, /PRIVATE_TREE_MAX_DEPTH/);
assert.match(windowsSandbox, /SANDBOX_PRIVATE_TREE_OUTSIDE_PROFILE/);
assert.match(windowsSandbox, /state_guard/);
assert.match(windowsSandbox, /state_directory_stays_pinned_until_profile_teardown/);
assert.match(ptyWinSpawn, /fn is_live_pipe_handle/);
assert.match(ptyWinSpawn, /creation_pipes: Mutex<Option/);
assert.match(ptyWinSpawn, /self\.creation_pipes/);
assert.match(ptyWinSpawn, /std::env::current_exe\(\)/);
assert.doesNotMatch(ptyWinSpawn, /ConPtyFuncs::open\(Path::new\("conpty\.dll"\)\)/);
assert.match(ptyConpty, /invalid_dimensions_are_rejected_before_pipe_creation/);

// Private storage is an exact owner/SYSTEM policy, not a textual allowlist.
// Preserve fail-closed behavior on unknown/package ACEs and incomplete walks.
assert.match(platformSecurity, /TokenUser/);
assert.match(platformSecurity, /GetTokenInformation/);
assert.match(platformSecurity, /GetSecurityInfo/);
assert.match(platformSecurity, /SetSecurityInfo/);
assert.match(platformSecurity, /EqualSid/);
assert.match(platformSecurity, /IsValidSid/);
assert.match(platformSecurity, /CompareStringOrdinal/);
assert.match(platformSecurity, /RootDirectory = parent\.as_raw_handle/);
assert.match(platformSecurity, /NtCreateFile/);
assert.match(platformSecurity, /GetFileInformationByHandleEx/);
assert.match(platformSecurity, /FILE_OPEN_REPARSE_POINT/);
assert.match(platformSecurity, /GetFileInformationByHandle/);
assert.match(platformSecurity, /nNumberOfLinks != 1/);
assert.match(platformSecurity, /HARDLINK_DENIED/);
assert.match(platformSecurity, /TREE_CHANGED_DURING_OPEN/);
assert.match(platformSecurity, /FileIdentity/);
assert.match(platformSecurity, /MAXIMUM_ALLOWED/);
assert.match(platformSecurity, /ReOpenFile/);
const privateRepair = platformSecurity.slice(
  platformSecurity.indexOf('fn repair_object('),
  platformSecurity.indexOf('#[derive(Debug)]\nstruct DirectoryEntry')
);
assert.ok(privateRepair.length > 0);
assert.match(privateRepair, /let writable = file/);
assert.match(privateRepair, /SetSecurityInfo/);
assert.match(privateRepair, /descriptor_is_private\(writable/);
assert.doesNotMatch(privateRepair, /ReOpenFile|reopen_maximum/);
const ensurePrivate = platformSecurity.slice(
  platformSecurity.indexOf('fn ensure_private('),
  platformSecurity.indexOf('pub fn ensure_private_directory(')
);
assert.ok(ensurePrivate.length > 0);
assert.match(ensurePrivate, /private_directory\(path, options, false, deep, start\)/);
assert.match(ensurePrivate, /migration_candidate\(&error\)/);
assert.match(ensurePrivate, /Err\(error\) => Err\(error\)/);
assert.match(platformSecurity, /downcast_ref::<StorageFailure>/);
assert.match(platformSecurity, /if self\.repair \{ MAXIMUM_ALLOWED \}/);

assert.match(platformSecurity, /REPAIR_READBACK/);
assert.match(platformSecurity, /ensure_private_directory_root/);
assert.match(platformSecurity, /PACKAGE_POLICY_CONFLICT/);
assert.match(platformSecurity, /UNSUPPORTED_ACE_REQUIRES_REVIEW/);
assert.match(platformSecurity, /max_objects/);
assert.match(platformSecurity, /max_depth/);
assert.match(platformSecurity, /max_repairs/);
assert.match(platformSecurity, /max_duration/);
assert.match(platformSecurity, /MIGRATION_BUDGET_EXCEEDED/);
const privateAclVerifier = platformSecurity.slice(
  platformSecurity.indexOf('fn descriptor_is_private('),
  platformSecurity.indexOf('fn repair_object(')
);
assert.ok(privateAclVerifier.length > 0);
assert.match(privateAclVerifier, /dacl\.is_null\(\)/);
assert.match(privateAclVerifier, /SE_DACL_PROTECTED/);
assert.match(privateAclVerifier, /AceType != ACCESS_ALLOWED_TYPE/);
assert.match(privateAclVerifier, /flags != expected_flags/);
assert.match(privateAclVerifier, /mask != FILE_ALL_ACCESS/);
assert.match(privateAclVerifier, /owner_count == 1/);
assert.match(privateAclVerifier, /system_count == system_expected/);
assert.match(privateAclVerifier, /count == 1 \+ system_expected/);
assert.match(platformSecurity, /native_private_tree_is_idempotent_and_preserves_bytes/);
assert.match(platformSecurity, /new_children_inherit_only_native_owner_and_system/);
assert.match(platformSecurity, /hardlinked_data_is_refused_without_changing_the_external_file/);
assert.match(platformSecurity, /unicode_ordinal_comparison_does_not_use_lossy_utf8/);
assert.match(platformSecurity, /low_object_budget_returns_incomplete_error_instead_of_pass/);
assert.match(platformSecurity, /shallow_parent_preparation_preserves_existing_child_dacl/);
assert.match(platformSecurity, /a_compliant_live_file_never_requires_a_mutating_open/);
assert.match(platformSecurity, /an_exact_native_sid_is_required_for_each_ace/);
assert.match(platformSecurity, /an_owner_deny_ace_is_not_an_owner_allow_ace/);
assert.match(platformSecurity, /an_owner_read_only_mask_is_not_full_control/);
assert.match(platformSecurity, /an_extra_world_ace_invalidates_an_otherwise_private_file/);
assert.match(platformSecurity, /directory_inheritance_flags_are_required_and_file_flags_are_empty/);

// Native acceptance may fail on an unsupported runner, but must not report a
// passing test after a GetConsoleWindow-based early return.
assert.doesNotMatch(process, /fn interactive_console_available/);
assert.doesNotMatch(process, /require_interactive_conpty!/);
assert.match(ptyWinChild, /struct ProcessWaitHandle\(OwnedHandle\)/);
assert.match(ptyWinChild, /Weak<Mutex<ProcessWaitState>>/);
assert.match(ptyWinChild, /if !state\.started/);
assert.match(ptyWinChild, /state\.finished = true/);
assert.match(ptyWinChild, /if state\.finished/);
assert.match(ptyWinChild, /owner\.run\(waiter_state\)/);
assert.match(ptyWinChild, /future_wait_owns_process_handle_across_repeated_polls/);
assert.match(ptyConpty, /validated_dimensions\(size.rows, size.cols\)\?/);
assert.match(ptyConpty, /resize\(validated_dimensions\(num_rows, num_cols\)\?\)\?/);
assert.match(ptyWinSpawn, /creation_pipes\.get_mut\(\)/);
assert.match(terminal, /TERM_CHILD_REAP_TIMEOUT/);
assert.match(terminal, /TERM_CLEANUP_PENDING/);

// Startup preparation is shared native evidence, not a persistent marker.
assert.match(privateLease, /CreateEventW/);
assert.match(privateLease, /CreateMutexW/);
assert.match(privateLease, /SE_KERNEL_OBJECT/);
assert.match(privateLease, /audit_kernel_object/);
assert.match(privateLease, /VOLUME_NAME_GUID/);
assert.match(privateLease, /SetEvent/);
assert.match(privateLease, /active_cache_churn_does_not_enter_a_deep_audit/);
assert.match(privateLease, /consumers_share_preparation_and_cold_restart_reaudits/);
assert.match(startup, /tabs\.prepare_storage\(\)/);
const createTab = tabs.slice(tabs.indexOf('pub fn create_tab('), tabs.indexOf('pub fn update_rect('));
assert.ok(createTab.indexOf('self.prepared_browser_profile()?') < createTab.indexOf('self.tabs.write()'));
assert.doesNotMatch(createTab, /secure_windows_browser_profile|audit_private_directory|PreparedPrivateDirectory::prepare/);

// Sensitive URL content remains in trusted routing, never diagnostic records.
assert.match(systemCommands, /fn external_url_audit_summary/);
assert.match(systemCommands, /external_url_audit_never_preserves_credentials_or_user_text/);
assert.doesNotMatch(systemCommands, /\[open-url\] accepted: \{url\}|log_audit\(app, "open_url(?:_rejected)?", url\.to_string\(\)\)/);
assert.match(workspace, /fn append_audit_at/);
assert.match(workspace, /AuditLock::acquire\(path\)[\s\S]*?load_audit_unlocked\(path\)\?/);
assert.match(auditLock, /LockFileEx/);
assert.match(auditLock, /LOCKFILE_FAIL_IMMEDIATELY/);
assert.match(auditLock, /libc::flock/);
assert.match(workspace, /concurrent_audit_appends_keep_every_entry/);
assert.doesNotMatch(main, /mvp-life\.log/);
assert.match(main, /workspace::record_lifecycle/);
const unixRoot = read('../src-tauri/src/capabilities/workspace/private_directory_unix.rs');
assert.match(unixRoot, /configured_base_alias_resolves_once_without_relaxing_the_owned_leaf/);
assert.match(unixRoot, /configured_alias_does_not_authorize_a_symlinked_app_leaf/);

// Global terminal-table locks never cover native writes or resize.
assert.match(terminalCommands, /input\.enqueue\(data\.as_bytes\(\)\)/);
assert.doesNotMatch(terminalCommands, /writer\.write_all|writer\.flush|session\.master/);
assert.match(terminal, /struct TerminalInput/);
assert.match(terminal, /TERM_INPUT_QUEUE_CAPACITY/);
assert.match(terminal, /struct CleanupRegistry/);
assert.match(terminal, /wait_while_running/);
assert.match(process, /cancel_thread_io/);
assert.match(process, /pub fn is_empty/);
assert.match(process, /Ok\(accounting\.ActiveProcesses\)/);
assert.match(process, /self\.active_processes\(\)\.map\(\|count\| count == 0\)/);
assert.match(terminal, /TERM_CLEANUP_COMPLETED_WITH_ERRORS/);
assert.match(terminal, /struct SpawnCleanupGuard/);
assert.match(terminal, /MAX_TERMINAL_SESSIONS: usize = 32/);
assert.match(terminal, /pending_cleanup_retains_master_and_workers_until_reaped/);
assert.match(terminal, /spawn_error_guard_retains_native_close_until_completed/);
assert.match(terminal, /asynchronous_input_failure_emits_a_fixed_exit_without_another_keystroke/);
assert.match(terminalCommands, /pub async fn term_kill/);
assert.match(terminalCommands, /pub async fn term_resize/);
assert.match(terminalCommands, /spawn_blocking/);
assert.match(terminalCommands, /session\.request_close\(\);[\s\S]*?terms\.remove\(&id\)/);
assert.match(main, /if report\.ok\(\) \{ 0 \} else \{ 1 \}/);

// Preparation, cancellation and deferred cleanup share one retained owner.
// These are wiring checks; native access and cleanup still need execution.
assert.match(platformSecurity, /pub struct OperationBudget/);
assert.match(windowsSandbox, /OperationCleanupGuard/);
assert.match(windowsSandbox, /SANDBOX_PREPARATION_INCOMPLETE/);
assert.match(windowsSandbox, /fn stderr_diagnostic/);
assert.match(windowsLsp, /fn observed_failure/);
assert.match(windowsLsp, /Duration::from_millis\(100\)/);
assert.match(windowsLsp, /cancelled_and_cleanup_reservations_do_not_release_capacity/);
assert.match(windowsLsp, /failed_native_start_keeps_capacity_until_cleanup_guard_drops/);
assert.match(windowsLsp, /released_native_cleanup_slot_is_reaped_without_another_request/);
assert.match(windowsLsp, /snapshot_counts_ignored_entries_and_empty_directories/);
assert.match(windowsLsp, /actual_snapshot_bytes_obey_remaining_read_and_write_capacity/);
assert.match(windowsLsp, /unfinished_io_worker_stays_owned_until_it_can_be_joined/);
assert.match(windowsLsp, /CLEANUP_SIGNAL/);
assert.doesNotMatch(windowsLsp, /icacls\.exe/);


// Terminal byte streams and ownership are separate from acceptance evidence.
assert.match(terminal, /struct Utf8StreamDecoder/);
assert.doesNotMatch(terminal, /String::from_utf8_lossy\(batch\)/);
assert.match(terminal, /utf8_decoder_preserves_every_split_of_multibyte_output/);
assert.match(terminal, /pipeline_preserves_split_utf8_across_flushes/);
assert.match(terminalState, /startingSessions/);
assert.match(terminalState, /TERM_BUFFER_MAX_BYTES = 128 \* 1024/);
assert.match(terminalState, /TERM_HISTORY_MAX_BYTES = 256 \* 1024/);
assert.match(terminalState, /receiveSessionMessage\(slot/);
assert.match(terminalState, /slot\.phase === "closed" \|\| slot\.phase === "closing"/);
assert.match(terminalState, /termBufferNotice/);
assert.doesNotMatch(terminalState, /termBuffers\.set\(id, \[\]\)/);

// One absolute application deadline; start resource release before awaiting it.
assert.match(shutdown, /APPLICATION_SHUTDOWN_BUDGET/);
assert.match(shutdown, /pub fn shutdown_until/);
assert.match(shutdown, /retained_tasks/);
assert.match(shutdown, /self\.owners_complete && self\.failures\(\)\.is_empty\(\)/);
assert.match(shutdown, /SHUTDOWN_DEPENDENCY_NOT_EXECUTED/);
assert.match(shutdown, /CleanupOutcome::Pending/);
assert.match(main, /\.shutdown_until\(deadline\)/);
assert.match(main, /worker: std::sync::Mutex<Option<std::thread::JoinHandle<\(\)>>>/);
assert.match(bridgeLifecycle, /register_with_deadline\("kill-terminals"/);
assert.match(bridgeLifecycle, /terminal::shutdown_until\(deadline\)/);
assert.match(terminalCommands, /fn publish_terminal/);
assert.match(terminalCommands, /state\.shutdown_requested\.load\(Ordering::Acquire\)/);
assert.match(windowsLsp, /LSP_APPLICATION_SHUTTING_DOWN/);
assert.match(windowsLsp, /fn retire_shutdown_sessions/);
assert.match(windowsLsp, /shutdown_admission_rejects_new_reservations_and_late_publication/);
assert.match(startup, /prepare_native_recovery/);
assert.match(sandboxLifecycle, /initialize_recovery_store/);
assert.match(sandboxLifecycle, /recover_abandoned_profiles/);
assert.match(sandboxLifecycle, /SANDBOX_RECOVERY_IDENTITY_NOT_BOUND/);
assert.match(nativeRecovery, /pub fn delete_recorded_profile_tree/);
assert.match(nativeRecovery, /RECOVERY_PROFILE_IDENTITY_CHANGED/);
assert.match(nativeRecovery, /SetFileInformationByHandle/);
assert.match(nativeRecovery, /OpenFileById/);
assert.match(nativeRecovery, /hold_profile_cleanup_namespace/);
assert.match(nativeRecovery, /RECOVERY_NAMESPACE_UNTRUSTED_WRITER/);
assert.match(sandboxLifecycle, /namespace\.recheck/);
const sandboxProduction = windowsSandbox.split('#[cfg(test)]\nmod tests')[0];
assert.doesNotMatch(sandboxProduction, /std::fs::remove_dir_all/);
assert.match(privateLease, /NOT a continuous descendant ACL monitor/);

// Saving can veto exit before any irreversible cleanup. A failed cleanup task
// cannot supply this guarantee, and an unload callback cannot await failures.
assert.match(sessionExit, /pub\(crate\) fn prepare_exit_sessions/);
assert.match(sessionExit, /SessionFlushOutcome::Failed/);
assert.match(sessionExit, /reconcile_snapshot_changes/);
assert.match(sessionExit, /if accept_exit && report\.can_exit\(\)/);
const exitPreflight = main.slice(main.indexOf('fn begin_graceful_shutdown('), main.indexOf('fn log_shutdown_report('));
assert.ok(exitPreflight.indexOf('prepare_exit_sessions(') < exitPreflight.indexOf('.shutdown_until('));
assert.match(exitPreflight, /if !saved\.can_exit\(\)/);
assert.doesNotMatch(exitPreflight, /shutdown_requested\.store\(true/);
assert.doesNotMatch(bridgeLifecycle, /register\("flush-sessions"/);
assert.match(browserOwner, /TAB_APPLICATION_SHUTTING_DOWN/);
assert.match(browserRuntime, /TAB_APPLICATION_SHUTTING_DOWN/);

// A diagnosis never grants deletion. Retry keeps the native identity/lease
// checks and has no caller path or record ID deletion parameter.
assert.match(sandboxLifecycle, /pub\(crate\) fn recovery_status\(until: Instant\)/);
assert.match(sandboxLifecycle, /pub\(crate\) fn retry_recovery\(until: Instant\)/);
assert.match(sandboxLifecycle, /recovery_status_preserves_unbound_intent_and_rejection/);
assert.match(nativeRecovery, /open_readonly/);
assert.match(nativeRecovery, /native_recovery_store_initialization_does_not_traverse_unrelated_children/);
assert.match(recoveryPanel, /sandbox_recovery_status/);
assert.match(recoveryPanel, /sandbox_recovery_retry/);
assert.match(codingPublicApi, /SANDBOX_RECOVERY_APPROVAL_REQUIRED/);
assert.match(bridgeLifecycle, /"shutdown-agent-lsp"/);
assert.match(bridgeLifecycle, /"shutdown-agent-sandbox"/);
assert.match(bridgeLifecycle, /grid_manager\.shutdown_until\(deadline\)\?/);
assert.match(windowsLsp, /"path":toolchain\.as_ref\(\)\.ok\(\)/);

console.log('WINDOWS_LINUX_V2_SOURCE_CONTRACT=PASS (static-only, Windows GUI pending)');
