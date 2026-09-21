## Objective
Reconstruct active or completed long-running Context Agent sessions from durable records after browser reconnect.

## Owner
`context-agent/backend` owns the reconnect/read API; `context-agent/frontend` owns reconnect rendering and interaction state.

## Spec
Satisfies `context-agent-autonomous-client.7` in spec `73aa2bce-cf09-4319-b9c5-758599898724`.

## Provider dependencies
Consume durable session runtime, workflow, handoff, and worktree state. Browser-local memory may accelerate rendering but is not the source of truth.

## Acceptance evidence
A reconnect reconstructs the timeline and terminal state from durable records after local browser state is cleared or unavailable.

## Validation
- `cargo test -p backend durable_reconnect`
- Reconnect E2E check

## Boundaries
Depends on CA-01. Do not duplicate session persistence or alter provider workflow semantics.