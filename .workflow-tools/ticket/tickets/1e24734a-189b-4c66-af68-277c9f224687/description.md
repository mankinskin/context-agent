## Objective
Expose MCP tool-server lifecycle actions that are bound to the active session/worktree context and fail closed when that context cannot be resolved.

## Owner
`context-agent/backend` owns the product-facing lifecycle API; provider MCP/session contracts remain authoritative for routing and execution.

## Spec
Satisfies `context-agent-autonomous-client.3` in spec `73aa2bce-cf09-4319-b9c5-758599898724`.

## Provider dependencies
Consume session-anchored workspace resolution and default tool-suite semantics. Never fall back to the process working directory.

## Acceptance evidence
Show one successful lifecycle action tied to a resolved session/worktree and one unresolved-context failure with an actionable diagnostic.

## Validation
- `cargo test -p backend mcp_session_binding`

## Boundaries
Depends on CA-01. Do not duplicate provider-owned session routing or implement prompt provenance.