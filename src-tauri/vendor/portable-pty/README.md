# portable-pty 0.8.1 (local BrowserOS fork)

This directory vendors the upstream MIT-licensed portable-pty 0.8.1 source from
[wezterm/wezterm](https://github.com/wezterm/wezterm), source commit
1ca5e6cc90467d5e8195a02f08314faa04ab8c50.

## Platform contracts carried by this fork

- Production Windows terminals use SlavePty::spawn_command_in_job. A precreated
  kill-on-close Job Object is assigned by PROC_THREAD_ATTRIBUTE_JOB_LIST during
  CreateProcessW, before user code can create unsupervised descendants.
- The explicit spawn_command_suspended API remains for callers that own the
  attach-and-resume contract. Unsupported backends fail closed. It is not the
  production ConPTY terminal route.
- Ordinary ConPTY creation uses no cursor-inheritance handshake flags. The
  caller drains output and supplies UTF-8/virtual-terminal input.
- The creation pipe ends remain alive until the first child CreateProcessW
  succeeds. Error/no-client teardown closes them before ClosePseudoConsole.
  The master releases its pipe endpoints before closing the pseudo console.
- Both creation and resize reject zero dimensions and values above 32767,
  before converting to Win32's signed COORD.
- Process completion is based on the signaled handle; exit code 259 remains a
  valid terminal exit code. Kill is idempotent and propagates real failures.
- Future polling starts at most one waiter, retaining an owned duplicate
  process handle for the whole wait. Repeated polls update its waker.
  Dropping the Future lets the waiter exit within its bounded polling interval.
- A bundled conpty.dll is only considered beside the running executable,
  never through a workspace-controlled current directory.

Keep these contracts, upstream license notices in LICENSE.md, and the native
descendant/input/exit-code regressions when updating or removing this fork.
Source changes and test definitions are not evidence of a native Windows or
macOS PASS. Native validation must identify the exact source and toolchain.
