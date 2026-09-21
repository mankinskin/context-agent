## Objective
Reconcile the active ticket-store lifecycle so implementation readiness has one reachable, documented state. The current store permits `planned -> open` and `open -> planned`, but rejects `planned -> ready` and `open -> ready`; the Context Agent Phase 1 plan currently names `ready` as the intended implementation-ready state.

## Scope
Inspect the canonical lifecycle policy and active store schema, choose one authoritative state sequence, update the owning workflow/spec guidance, and verify a reachable transition for planned implementation work. Do not implement Context Agent product behavior.

## Context Agent dependency
This ticket is a prerequisite for [CA-01 Timeline Contract](context-agent/.workflow-tools/ticket/tickets/8b78c940-7c63-412b-8d4c-051a32f573f9/ticket.toml), [CA-02 Prompt Provenance](context-agent/.workflow-tools/ticket/tickets/1ef56eb5-a957-45a0-981b-83acda2f9d89/ticket.toml), and [CA-03 Session-Bound MCP](context-agent/.workflow-tools/ticket/tickets/1e24734a-189b-4c66-af68-277c9f224687/ticket.toml). The Context Agent tickets must remain blocked until the state decision is resolved.

## Evidence
The ticket API rejected `planned -> ready` with allowed successors `cancelled`, `in-implementation`, `on-hold`, and `open`; it rejected `open -> ready` with allowed successors `cancelled` and `planned`.

## Acceptance criteria
- One canonical lifecycle sequence is documented for `planned`, `open`, `ready`, and `in-implementation`.
- The active ticket schema and lifecycle guidance agree on the sequence.
- A dry-run or direct validation proves the selected implementation-start transition is reachable.
- CA-01 through CA-03 have no unresolved state-model ambiguity after this ticket is resolved.

## Validation
- Ticket lifecycle/state transition validation for the active store
- Ticket health check on this ticket and CA-01 through CA-03

## Boundaries
Do not change Context Agent source code, start implementation, or silently rename existing ticket states.