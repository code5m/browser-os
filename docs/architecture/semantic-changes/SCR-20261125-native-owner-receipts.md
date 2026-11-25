# SCR-20261125: Browser native mutation receipts and protected Coding execution

**Decision:** Owner-approved target, ADR-20261125 D01/D03/D04/D05/D06/D08. User explicitly requested implementation and durable decisions on 2026-11-25. Source candidate only; validation NOT_RUN.

## Semantic scope

The existing useBrowserStore remains owner of browser product state, and the existing native Browser owner remains responsible for WebView instances/layout. The receipt adapter serializes existing host mutations; it is not a second tab list or page lifecycle manager. Prepare/poll/inspect/ack do not dispatch a WebView effect. Start admits the stored action at most once per ticket. Cancel before dispatch terminates the request; a running cancellation request does not claim that the engine stopped. Only terminal host receipts can be acknowledged; unknown outcomes retain admission. Host dispatch is not renderer paint or navigation completion.

The existing Coding session owns the authenticated run receipts and Foundation task snapshot. A pending approval does not become task success; unknown external effects require explicit reconciliation. The existing Foundation TaskRegistry is reused, rather than introducing another task engine. Reviewed no-replace export is distinct from shared-file in-place apply; it does not overwrite proposal targets.

## Registry and gates

Native command inventory declares tab_mutation as Browser-owned, main-only. The legacy main-only mutation commands share its admission lane. Existing position/hide contracts and semantic registry owners do not change. No checker exclusions, whitelist of failures, or gate changes. Existing task and side-effect product semantics are preserved; this SCR records the internal receipt semantics and explicit owner-approved recovery boundary.

## Remaining evidence and accepted limitations

Full renderer cancellation / engine allocation isolation, complete Foundation tool-loop convergence, recovery after a frontend restart (including a usable UI for inspect/reconcile/ack), internal native bypass paths, GTK incident stacks, and platform-specific behavior still require verification or implementation as documented. Never treat a host receipt as a renderer completion certificate. Record all future regressions. No CI, tests, build, GUI verification or installation executed.
