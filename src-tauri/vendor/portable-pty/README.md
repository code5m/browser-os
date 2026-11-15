# portable-pty 0.8.1 (local BrowserOS fork)

This directory vendors the upstream MIT-licensed `portable-pty` 0.8.1 source from
[wezterm/wezterm](https://github.com/wezterm/wezterm), source commit
`1ca5e6cc90467d5e8195a02f08314faa04ab8c50`.

The only behavioral extension is a fail-closed suspended ConPTY spawn path:

- `SlavePty::spawn_command_suspended` is an explicit API.
- Windows ConPTY implements it with Win32 `CREATE_SUSPENDED`.
- BrowserOS assigns the process to its kill-on-close Job Object and only then
  resumes the primary thread.
- The default trait implementation on platforms without this primitive returns
  an error rather than silently starting an unsupervised process.

Do not replace this fork with the registry crate without preserving this
contract and its Windows immediate-grandchild regression test. Keep upstream
license notices in `LICENSE.md`.
