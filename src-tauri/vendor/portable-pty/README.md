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
  A cloned reader remains an independent output-pipe owner: callers must keep
  draining it until ClosePseudoConsole has returned, or close every output
  handle first. Dropping only the input writer does not establish shutdown.
  Older Windows builds can block inside ClosePseudoConsole; the BrowserOS
  terminal cleanup owner closes the sole master on a tracked close thread,
  keeps its reader draining, then joins the input/reader/pump. A timeout retains
  the owner and is reported as TERM_CLEANUP_PENDING, never as a completed close.
  See [Microsoft's close contract](https://github.com/MicrosoftDocs/Console-Docs/blob/main/docs/closepseudoconsole.md).
- Both creation and resize reject zero dimensions and values above 32767,
  before converting to Win32's signed COORD.
- Process completion is based on the signaled handle; exit code 259 remains a
  valid terminal exit code. Kill is idempotent and propagates real failures.
- Future polling starts at most one waiter, retaining an owned duplicate
  process handle for the whole wait. Repeated polls update its waker.
  Dropping the Future lets the waiter exit within its bounded polling interval.
- A bundled conpty.dll is only considered beside the running executable,
  never through a workspace-controlled current directory.

The public Error re-export names the error type already used by the backend
traits, allowing host lifecycle fixtures to implement those traits without
adding another dependency. It does not change the error or runtime behavior.

Keep these contracts, upstream license notices in LICENSE.md, and the native
descendant/input/exit-code regressions when updating or removing this fork.
Source changes and test definitions are not evidence of a native Windows or
macOS PASS. Native validation must identify the exact source and toolchain.
