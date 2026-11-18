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

assert.match(cargo, /keyring = \{ version = "3", features = \["sync-secret-service", "windows-native"\] \}/);
assert.match(lock, /name = "keyring"[\s\S]*?"windows-sys 0\.60\.2"/);
assert.match(cargo, /Win32_System_JobObjects/);
assert.match(process, /JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE/);
assert.match(windowsPipe, /PIPE_REJECT_REMOTE_CLIENTS/);
assert.match(windowsPipe, /GetNamedPipeClientProcessId/);
assert.match(gridIpc, /MAX_GRID_WIRE_BYTES/);
assert.match(process, /AssignProcessToJobObject/);
assert.match(process, /attach_suspended_pid/);
assert.match(process, /CREATE_SUSPENDED/);
assert.match(winGrid, /attach_suspended_pid/);
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
assert.match(windowsSandbox, /attach_suspended_pid/);
assert.match(windowsMcp, /MCP_EXPLICIT_APPROVAL_REQUIRED/);
assert.match(windowsMcp, /MCP_SHELL_COMMAND_REJECTED/);
assert.match(windowsMcp, /real_appcontainer_stdio_initializes_lists_calls_and_scrubs_secrets/);
assert.match(windowsLsp, /protect_snapshot/);
assert.match(windowsLsp, /--no-proc-macro/);
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
assert.match(windowsSandbox, /canonical\.parent\(\) == Some\(system32_dir\.as_path\(\)\)/);
assert.match(windowsMcp, /MCP_CONFIG_REPARSE_DENIED/);
assert.match(safePathsWindows, /fn valid_windows_sid/);
assert.match(safePathsWindows, /sid_validation_preserves_literal_s_prefix/);
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
assert.match(windowsSandbox, /let directory = self\.folder\.join\("runtime"\)/);
assert.doesNotMatch(windowsSandbox, /let directory = state\.join\("runtime"\)/);
assert.match(windowsSandbox, /fn grant_profile_root_traverse/);
assert.match(windowsSandbox, /"cmd\.exe", "curl\.exe"/);
assert.match(windowsSandbox, /environment_block\(environment, current_directory, &profile\.folder\)/);
assert.doesNotMatch(windowsSandbox, /std::env::var\("LOCALAPPDATA"\)/);
assert.doesNotMatch(windowsSandbox, /"\/S"\.into\(\)/);
assert.doesNotMatch(windowsSandbox, /\(A;;GA;;;AC\)/);
assert.doesNotMatch(windowsSandbox, /S-1-15-2-1:\(OI\)\(CI\)M/);
assert.match(windowsSandbox, /SANDBOX_CREATE_PROCESS/);
assert.match(windowsSandbox, /OwnedSecurityDescriptor/);
assert.match(windowsSandbox, /#\[cfg\(not\(test\)\)\]/);
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
assert.match(tabs, /#\[cfg\(target_os = "windows"\)\]\s*fn ensure_private_browser_profile/);
assert.match(tabs, /secure_windows_browser_profile/);
assert.match(tabs, /reject_profile_reparse_tree/);
assert.match(tabs, /BROWSER_PROFILE_ACL/);
assert.match(tabs, /whoami\.exe/);
assert.match(tabs, /icacls\.exe/);
assert.match(tabs, /S-1-5-18/);
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
assert.match(launchPolicy, /#\[cfg\(not\(windows\)\)\]/);
assert.match(launchPolicy, /std::fs::canonicalize\(root\)\.map/);
assert.match(launchPolicy, /linux_differently_cased_sibling_never_inherits_workspace_access/);
assert.match(launchPolicy, /non_utf8_linux_names_do_not_merge_in_root_matching/);
assert.match(gitStudio, /fn read_worktree_file/);
assert.match(gitStudio, /case_variant_symlink_never_exposes_other_git_workspace/);
assert.doesNotMatch(gitStudio, /fs::read_to_string\(file\)/);
assert.doesNotMatch(gitStudio, /hash\.update\(fs::read\(path\)/);
// Only a package SID may read private snapshot *files*. The Windows
// restricted lowbox traverses directories through non-inheritable generic
// AppContainer ACEs; those grants must never propagate onto source files.
assert.match(windowsLsp, /grant_entry\(&icacls, &directory, "\*S-1-15-2-1:RX"\)/);
assert.doesNotMatch(windowsLsp, /S-1-15-2-1:\(OI\)/);
assert.doesNotMatch(windowsLsp, /grant_entry\(&icacls, &path, "\*S-1-15-2-1:RX"\)/);
assert.match(windowsLsp, /appcontainer-no-network-private-source-copy/);
assert.match(windowsLsp, /LSP_CHILD_NULL_DEVICE_DENIED/);
assert.match(windowsSandbox, /same-sized, incompatible DLLs/);
assert.match(windowsSandbox, /existing\.read_exact\(&mut actual/);
const v2Workflow = read('../.github/workflows/windows-linux-v2-experimental.yml');
assert.match(v2Workflow, /Generate and verify NSIS SHA256SUMS/);
assert.match(v2Workflow, /Get-FileHash -LiteralPath .* -Algorithm SHA256/);

// Non-Cargo LSP project-mode safety: explicitly supply bounded source graphs
// instead of requiring rust-analyzer to spawn Cargo/rustc through forbidden
// lowbox NUL device access. This is IDE analysis, not compiled diagnostics.
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
  windowsLsp.indexOf('fn reset_deadline(')
);
assert.ok(sandboxedLspStart.length > 0);
assert.doesNotMatch(sandboxedLspStart, /stage_runtime_peer\(/);
// Any project source files are copied before package ACLs are locked.
assert.match(windowsLsp, /write_reviewed_rust_project\(&destination\)\?/);
assert.match(windowsLsp, /write_reviewed_rust_project\(&snapshot\)\?/);
// Shared LOLBin guard applies to MCP as well as the native app launcher.
assert.match(launchPolicy, /pub\(crate\) fn is_blocked_executable_path/);
assert.match(windowsMcp, /security_policy::is_blocked_executable_path/);
assert.match(windowsMcp, /windows_lolbin_wrappers_are_rejected_without_spawning_sandbox/);
// MCP results are untrusted even in JSON object keys, not just values.
assert.match(windowsMcp, /std::mem::take\(items\)/);
assert.match(windowsMcp, /credential_in_json_key_is_redacted_in_nested_mcp_results/);
// Windows canonical UNC and verbatim drive forms retain absolute identity.
assert.match(safePathsWindows, /windows_path_normalizer_preserves_drive_and_unc_root_semantics/);

// Follow-up: four real Windows LSP requests, private source snapshots,
// AppContainer SID isolation, and ConPTY creation/child lifetime.
const nativeLspStart = windowsLsp.slice(
  windowsLsp.indexOf('fn start_with_snapshot('),
  windowsLsp.indexOf('fn reset_deadline(')
);
assert.match(nativeLspStart, /"linkedProjects":\[reviewed_graph\]/);
assert.doesNotMatch(nativeLspStart, /stage_runtime_peer\(/);
assert.match(windowsLsp, /"HOVER",/);
assert.match(windowsLsp, /"DEFINITION",/);
assert.match(windowsLsp, /"SYNTAX_DIAGNOSTICS",/);
assert.match(windowsLsp, /"SYMBOLS",/);
assert.match(windowsLsp, /legacy_cargo_subprocess_probes_are_diagnostics_only/);
assert.match(windowsLsp, /private_snapshot_requires_own_package_sid_and_rejects_other_profiles/);
assert.match(windowsLsp, /write-negative profile/);
const snapshotAcl = windowsSandbox.slice(
  windowsSandbox.indexOf('pub fn protect_snapshot_files('),
  windowsSandbox.indexOf('pub fn protect_snapshot(')
);
assert.ok(snapshotAcl.length > 0);
assert.match(snapshotAcl, /\(OI\)\(CI\)RX/);
assert.doesNotMatch(snapshotAcl, /\(OI\)\(CI\)M/);
assert.match(ptyWinSpawn, /fn is_live_pipe_handle/);
assert.match(ptyWinSpawn, /creation_pipes: Mutex<Option/);
assert.match(ptyWinSpawn, /self\.creation_pipes/);
assert.match(ptyWinSpawn, /std::env::current_exe\(\)/);
assert.doesNotMatch(ptyWinSpawn, /ConPtyFuncs::open\(Path::new\("conpty\.dll"\)\)/);
assert.match(ptyConpty, /invalid_dimensions_are_rejected_before_pipe_creation/);

console.log('WINDOWS_LINUX_V2_SOURCE_CONTRACT=PASS (static-only, Windows GUI pending)');
