# Provider-owned MCP cancellation settlement

BrowserOS does not infer cancellation from a local process exit, a transport
notification, timeout, or missing status file. A provider may opt in using its
independently pinned Ed25519 public key in the user's native MCP configuration
(`coding-mcp.json`, field `browseros_effect_authority`). The key is public; the
provider's private signing key must never enter BrowserOS or the child environment.

On admission BrowserOS supplies bound effect metadata: version, run_id,
idempotency_key, review_revision, account, namespace. The provider must persist
the original operation's idempotency key and enforce its own terminal-state
transition. If a cancellation request arrives, only a durable final cancellation
that prevents **all future application of this exact effect** permits
`cancelled_final`. A cancellation race that has applied the effect must be
reported as `applied_final`. An independently established permanent
non-application without explicit cancellation may be `not_applied_final`.
Pending, requested, acknowledged, or unknown cancellation is NOT final.

Deliver a JSON object to the user's native private data directory
`mcp-effect-status/<effect UUID>.json` by atomic publication via an authorized
provider/status relay. No original operation is invoked to obtain this status.

Required fields: version (1), run_id, idempotency_key, review_revision,
account, namespace, outcome (one of the three final outcomes above), signature.
The Ed25519 signature is standard padded base64 over the exact UTF-8 bytes of
the following newline-delimited ASCII fields, without a trailing newline:

```
browseros.mcp.effect.final.v1
1
<run_id>
<idempotency_key>
<review_revision>
<account>
<namespace>
<outcome>
```

The native owner verifies the signature, all original binding fields, and the
authenticated Run receipt before settlement. Existing applied/not-applied
signatures remain compatible. Late contradictory effect evidence is retained
without rewriting the terminal receipt. Cancellation is a distinct signed
outcome, not a fabricated claim that the provider has implemented this contract.
