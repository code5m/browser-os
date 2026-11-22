# GitHub Actions Workflow Catalog

Workflow registration is machine-enforced by `docs/engineering/governance.json`.

| File | Role | Trigger policy |
| --- | --- | --- |
| `bfc-targeted-regression.yml` | BFC script syntax, synthetic DOM, readback and tab-binding regression | relevant BFC changes on all branch pushes / PRs; no real B站 requests or native builds |
| `windows-linux-v2-experimental.yml` | Reusable Windows/Linux native regression, private storage, BFC smoke; optional diagnostic NSIS | `workflow_call` only, invoked from registered Developer Feedback manual native mode; exact matching `source_sha`; no automatic trigger or GUI claim |
| `capability-v2-ui-safety.yml` | fast UI policy feedback | all branch pushes / PR to master; risk-aware jobs; manual full |
| `capability-v2-validation.yml` | fast merge gate with explicit optional native and installed GUI modes | master pushes / non-draft PRs targeting master (including ready-for-review); manual full |
| `capability-v3-hotplug.yml` | focused Hot-Plug acceptance | all branch pushes / PR to master; risk-aware jobs; manual full |
| `browseros-developer-feedback.yml` | fast feedback; optional manual .deb OR Windows/Linux native mode | ordinary pushes use Quick Gate; `windows_native=false` and preview=false by default; native mode explicitly bypasses Quick Gate and calls reusable native workflow |
| `engineering-governance.yml` | governance drift | all branch pushes / PR to master; risk-aware jobs; manual full |
| `supply-chain-assurance.yml` | supply-chain gate and SBOM evidence | all branch pushes / PR to master; risk-aware jobs; manual full |
| `browseros-sustained-performance.yml` | opted-in installed RSS/PSS/CPU evidence | manual dispatch only; no weekly schedule |
| `mirror-to-gitee.yml` | retired legacy mirror placeholder | manual only; no branch/tag push trigger |

The v2/v3 filenames remain for historical compatibility. Every developer branch push gets the Quick Gate, and existing focused safety/hot-plug/governance workflows remain active. Full Validation no longer starts on developer pushes or draft PRs; it runs on master pushes, non-draft PRs targeting master (including the ready-for-review transition), and explicit manual dispatch. The fail-closed `scripts/ci-impact.py` still selects `docs`, `ui`, or `full` for runs that actually invoke Full Validation. Unknown, native, bridge, dependency, workflow and mixed-sensitive changes remain full. A skipped check is never evidence of passing that check. Commits explicitly created with GitHub's `[skip ci]` instruction do not run push checks; that exception records NOT_RUN, never PASS.

The independent Preview job builds a release Debian package only after an explicit workflow_dispatch request with build_preview=true (default false). Branch pushes never compile a native preview. The formerly weekly installed-resource benchmark is also manual-only.

Developer Feedback concurrency is scoped by branch and by `quick` or `native` mode. A lightweight push therefore cannot cancel a manually dispatched native run for a frozen SHA; a newer native dispatch still supersedes an older native run on the same branch. Manual Debian Preview keeps the Quick Gate lane and its existing cancellation behavior.

Full Validation provides off (default), rust-tests, packaged-gui, and packaged-resource modes, selected by manual input, one PR label, or the CI_NATIVE_MODE repository variable. The current source frontend is built ONCE into a SHA-bound artifact and reused in static and optional native jobs. The same-run master size audit builds a different source commit, which is an approved regression baseline, not a duplicate of the current source.

Skipping Rust or GUI is reported as SKIPPED, not PASS. For release evidence, explicitly run rust-tests and packaged-resource at the same SHA in two separately requested workflows. Details: [Optional Native CI and Build Reuse](./CI-OPTIONAL-NATIVE-20261101.md).

## Evidence

Only opted-in GUI-mode validation uploads GUI summary, driver report and app log as a short-retention Actions artifact so CI evidence does not vanish with the runner and does not permanently inflate the Git repository. Optional native-regression owns exactly one Rust-test compile OR one packaged Release/GUI path; the validate job never recompiles Rust. Preview uploads `.deb`, SHA256 and source SHA without pretending that a package build is GUI acceptance. Supply-chain assurance audits both production and development Node dependencies, including the build toolchain, and uploads deterministic SBOM/checksum evidence.

## Blocking policy

UI Safety, Full Validation, Hot-Plug, Engineering Governance and Supply Chain remain engineering quality gates for their applicable triggers. The new Developer Feedback/Preview workflow is non-blocking and cannot replace a full merge/release gate. Configure GitHub branch protection to require the appropriate Full Validation job results for merge to `master`; this Git repository change alone cannot configure branch protection. The legacy Gitee mirror is retired and must not gate GitHub `master`.

Any workflow added/removed must update `governance.json` in the same change.

## Local Git credential guards (2026-10-25)

The registered `.githooks/pre-commit` checks staged content for known local AI credentials and recognizable provider tokens. `.githooks/pre-push` checks all outgoing commit trees for each requested ref before the existing GitHub fast-forward checks. The guards report file names and error codes only. `npm run check:ai-config` and `pre-merge.sh` cover import regression fixtures and current-file secret checks. Credential values are read in memory from explicit local sources and OS Secret Service; they are never stored in reports or Git assets.

## Change-aware execution and evidence

- `docs`: only narrative documentation; skip compiler, GUI, Hot-Plug, and advisory scans. Engineering Governance remains active for all pushes and PRs.
- `ui`: up to six safe leaf-view/UI source files plus documentation; run Node setup, app checks and production build, and UI policy check; skip Rust compiler/packaged GUI where no native or bridge contracts changed.
- `full`: changes to Rust, Tauri configuration, root app, scripts, dependency manifests, workflow rules, native/bridge, or any unknown file. Run every original validation gate. Manual dispatch forces this mode.
- Dependency scanning runs when dependency manifests, security/SBOM scripts, workflows or manual dispatch require it; unrelated source changes do not reinstall `cargo-audit`.
- Classifier exceptions, nonexistent diff boundary, and unexpected file names must fail closed to `full`; no filename patterns may weaken a required release gate.
- Before producing a Release, manually run Full Validation rust-tests and packaged-resource separately at the exact SHA, plus Supply Chain Assurance. Stable tags cannot be moved to shortcut verification.
- Draft PR: fast feedback only; preview is manual opt-in. Mark ready for review: Full Validation runs; all required checks must succeed before merge.
- Download the preview from the Developer Feedback workflow's `browseros-preview-<sha>` artifact. Check `SOURCE_COMMIT` and `SHA256SUMS` and install it on the authorized test desktop. Preview is `PACKAGE_PASS` at most; only user-confirmed real desktop acceptance may be `GUI_PASS`.
- GitHub required-check settings must be reconciled with conditionally skipped jobs; branch protection should require the always-running impact/governance jobs instead of skipped child jobs, without weakening full/release evidence.

## Windows/Linux native source identity and result accounting

The dispatch entry is **BrowserOS Developer Feedback**, file `.github/workflows/browseros-developer-feedback.yml`. This same path already exists with `workflow_dispatch` on the default branch `master` (confirmed at `f2c56312866d8756607a2a2339a240acc9d26641`). The feature-only native workflow is now a `workflow_call` implementation, not an advertised standalone dispatch entry.

To request native validation, dispatch Developer Feedback through GitHub CLI or the workflow-dispatch API with the repair branch as `ref`, `windows_native=true`, and the full `source_sha` of that branch revision. The default-branch UI may still show its older input form; the branch-ref API/CLI call is the defined entry. When prompting for inputs, the CLI reads workflow contents from the selected `--ref`; explicit input fields are sent with that same dispatch ref; different inputs on master alone do not establish a broken branch entry. Execution status must be read from exact-SHA workflow runs; this catalog is not execution evidence. The source job refuses a SHA different from `github.sha`, so test definitions and tested code belong to the same commit. The relative reusable workflow loads from the same selected revision. This does **not** require editing or merging anything into master. Do not dispatch the branch-only native filename directly. GitHub's documented [manual workflow rules](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow) apply; this implementation does not bypass repository permissions or policies.

Every BrowserOS job checks out the frozen SHA. The independently reviewed BFC service keeps its own pinned revision. Windows selections, shared SID/DACL crate, vendored ConPTY and WebView2 profile all call `scripts/windows-native-test-runner.ps1` with a `SelectionId` from `scripts/windows-native-required-tests.json`. The runner enumerates the selected binary first, resolves every required test name uniquely, then reconciles every actual result with the enumerated names. Missing, duplicate, ignored required, failed or runtime-SKIP cases cannot become required PASS; a single passed case does not prove the whole required group. Windows full suites use the same supervision and report ignored extras separately. The Linux framing step reads the same two-name selection and reconciles enumeration, individual results and counts; the separate multi-binary Linux full-suite summary remains distinct. The profile and vendor tests select dependency packages through the host's committed `src-tauri/Cargo.lock` using `--locked`; their no-dev-dependency constraint remains guarded in the source contract.

`scripts/windows-native-test-supervisor.cs` is definition-only until the explicitly authorized PowerShell run loads it. It creates each enumeration/test process suspended and atomically in a private kill-on-close Job with an explicit inherited-handle list. Cleanup requires the root process to be signaled and the Job's active process count to be zero; root exit by itself is insufficient. A root that leaves descendants, a timeout, unknown Job cleanup or required-name mismatch fails the selection; unknown cleanup quarantines later native/NSIS steps. Output goes directly to files, with a polled size threshold and bounded subsequent reads, not a claimed hard filesystem quota. Each executed selection must report the supervisor outcome; definitions alone are not execution evidence.

The filtered matrix and full Rust suites have separate status fields. Full-suite failures never subtract a second time from filter counts. Full runs with ignored cases are labelled `PASS_WITH_IGNORED`, whose ignored cases provide no acceptance evidence. Linux full native output is preserved separately. GUI, installed BFC login, signing and performance remain separate evidence.

NSIS requires `build_windows_installer=true` on the dispatcher (forwarded as `build_installer`, default false). Its artifact remains `UNVERIFIED`. Ordinary pushes do not enter this lane, and publishing a `[skip ci]` repair commit does not dispatch any workflow. Record each exact-SHA workflow result separately; installer generation alone does not establish installed-GUI acceptance.

## Owner integrity validation coverage (2026-11-22)

The existing manual dispatch policy remains unchanged. Full Validation's `full` lane also runs the complete managed Coding/Workbench UI gate, including its nine configured component scenarios; simulated transport remains distinct from installed native GUI. Packaged modes upload the already-built release Debian package together with its source SHA and SHA256, without starting a second preview build.

The native workflow now exercises the host's `mvp_core` library and Agent Foundation's unit tests on both platforms, in addition to the existing host binary suite. Foundation is selected with the host manifest and committed host lockfile; only the current production `adapter-rig` and `experimental-rig-run` features are requested. The Windows artifact must declare both features, and its required Rig driver tests must enumerate and pass. This does not claim coverage for disabled `durable-sqlite`, RMCP or ACP adapters.

Windows library binaries use the same Job supervisor and required-name manifest as the existing native tests. The core library has its own complete-suite selection with ignored cases forbidden, so its two tests cannot be mistaken for host-binary results. Linux framing and production-library evidence share `scripts/check-native-required-results.py`: it reconciles enumeration, each mandatory result, all result counts and runtime-SKIP diagnostics, and rejects missing, duplicate, ignored or failed required tests. The checker self-test includes negative evidence fixtures and runs in the existing static-contract job. Full host suites and platform library results remain separately reported. A committed coverage definition is not a test execution result; acceptance must cite the actual workflow run and exact source SHA.
