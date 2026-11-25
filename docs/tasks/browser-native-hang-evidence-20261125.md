# Browser native hang evidence (ADR D06)

Source and diagnostic procedure, not an observed incident. Do not alter the locked GTK move/allocate/draw rules on the basis of this document.

The existing watchdog writes bounded metadata and now includes the current tab-mutation ticket, command, phase and age without URL, document, prompt or credentials. Correlate the ticket with IPC elapsed time and the actual native stack. A host callback receipt is not a frame-painted receipt.

When the real Linux device is connected and the Owner authorizes incident capture, record executable path, package version, source SHA, desktop/compositor, WebKitGTK version and exact reproduction actions. Identify the BrowserOS host PID and use the existing diagnostics directory. If an already installed debugger is available, collect a bounded userspace thread backtrace of the actual stalled host (e.g. GDB batch thread apply all bt); attach can pause the process and require ptrace permission. Preserve the original private report, redact any credentials or page data before sharing. Do not install tools, change ptrace policy or execute this procedure without authorization. Record timestamps, the main thread, lock holders/waiters, WebKit processes and matching screenshots. Do not equate /proc kernel wait channels with a userspace backtrace.

Windows native results must be recorded on Windows. Linux installed GUI evidence must identify the actual installed artifact (not a dev-server process). CODE_PASS, PACKAGE_PASS and GUI_PASS are independent. Current real-device incident and platform evidence: NOT_RUN / GUI_PENDING.
