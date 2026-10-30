# Coding workbench: OpenCode evidence and implementation scope

Date: 2026-10-25. Research source: `anomalyco/opencode` HEAD `49a112f18e9bf6a6a3bf50915495049af92e017d`, queried from upstream Git. Installed local CLI reports `1.18.34`; the upstream HEAD is separately identified and is not claimed to be the source commit of that installed binary.

## Source mapping

- [McpStdio.make](https://github.com/anomalyco/opencode/blob/49a112f18e9bf6a6a3bf50915495049af92e017d/packages/core/src/mcp/stdio.ts): direct child command/arguments, newline JSON-RPC transport, bounded outgoing queue and frames, scoped process cleanup including descendants, explicit startup/closed phases. BrowserOS adopts direct argv, bounded frames/channels, a per-call deadline, nonblocking stdin, and a dedicated Linux process group killed and reaped when the session drops. BrowserOS sessions currently last one tools/list or tools/call operation; long-lived reconnectable sessions are not implemented.
- [SkillScan.scan](https://github.com/anomalyco/opencode/blob/49a112f18e9bf6a6a3bf50915495049af92e017d/packages/core/src/config/plugin/skill-scan.ts): breadth/depth-bounded directory discovery and SKILL.md selection. BrowserOS scans approved project/user skill roots, skips symlinks, limits directory depth and entry count, lists metadata first, and only reads the selected SKILL.md. It does not execute referenced scripts. Credential-bearing lines are omitted from returned context.

These files were read from the exact commit above. No screenshot or launched upstream GUI was available during this implementation; this document makes no visual-fidelity claim.

Upstream [LICENSE at the same SHA](https://github.com/anomalyco/opencode/blob/49a112f18e9bf6a6a3bf50915495049af92e017d/LICENSE) was fetched and read: MIT, copyright 2025 opencode. This implementation is independently written Rust based on protocol and behavior research; upstream TypeScript source was not copied into the product. If later changes copy upstream source or substantial portions, retain its copyright and complete MIT permission notice with those distributed portions.

## Native context API

`coding_context::execute(root, storage, request)` accepts these `action` values:

- `skills_list`: discovered skill metadata, up to 256 entries. Approved roots are project `.agents/skills`, `.opencode/skills` and user `.agents/skills`, `.codex/skills`, `.codebuddy/skills`, `.workbuddy/skills`.
- `skill_read`: `path` must resolve to a discovered SKILL.md; bounded UTF-8, credential-bearing lines filtered.
- `knowledge_search`: `query` 2–200 characters searches project `docs` text files; skips hidden directories, symlinks, dependencies and build artifacts; maximum 40 excerpts. External knowledge roots, embeddings and indexing are not implemented.
- `mcp_list`: discovers BrowserOS storage `coding-mcp.json`, project OpenCode configuration and known user CodeBuddy/WorkBuddy/OpenCode configuration. Returns opaque server IDs and metadata without command/environment credentials. Discovery does not start processes.
- `mcp_tools`: explicit `approved: true`, `server_id` from discovery, optional `timeout_seconds` 1–60. Starts one server, initializes MCP, sends initialized notification, requests tools/list, closes the process group. Returns tool definitions and their annotations as untrusted data.
- `mcp_call`: same approval/configuration, plus `tool` and JSON `arguments`; performs initialize then tools/call. This is a direct user-requested invocation; agent-generated tool calls must never synthesize approval. Read-only annotations alone do not grant permission.

An explicitly selected server object may replace `server_id`: `server: {command: absolute executable path, args: string[], env_refs: {ENV_NAME: "ai:credential-reference"}}`. Credentials resolve within Rust from the existing OS keyring. Direct frontend plaintext environments are rejected. Existing external user configuration may already contain an environment; it is read into native memory only and scrubbed from returned server output. Project configurations must use keyring references. No configuration file is rewritten by discovery or execution.

## Verified boundaries and remaining work

Fixtures exercise real Python stdio processes for initialize, list, successful call, remote error and timeout. Native tests additionally reject missing approval, relative executables, unauthorized skill paths and unsafe context lines. Runtime errors expose bounded stable codes, not provider exception text or stderr. Server output remains untrusted; it cannot grant permissions or change user instructions.

Validation performed on 2026-10-25: `cargo test --manifest-path src-tauri/Cargo.toml coding_context --bin mvp-browser-os -- --nocapture` completed with 5 passed, 0 failed, exit 0. These include stdin backpressure timeout, descendant cleanup, discovered-ID connection and exact-value credential-echo redaction. `rustfmt` and `git diff --check` also passed. This proves the fixture protocol boundary; it does not validate any user's external plugin credentials or remote provider availability.

Current gaps: HTTP/SSE/Streamable HTTP MCP, OAuth flows, persistent sessions, pagination of tool lists, MCP resources/prompts and remote server requests are not implemented. Unsupported HTTP configuration returns `MCP_HTTP_NOT_IMPLEMENTED`. Existing user command arrays using `npx`/`uvx` resolve through absolute PATH directories in native code before the absolute executable validation; commands supplied directly by the frontend must already be absolute. Shell executables are rejected. This scope does not prove complete OpenCode parity, VS Code editor parity, installed GUI behavior or remote plugin compatibility.
