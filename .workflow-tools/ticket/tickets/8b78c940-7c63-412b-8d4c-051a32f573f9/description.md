## Objective
Implement the Context Agent backend/frontend timeline contract for one session, correlating chat messages, model requests, model responses, tool calls, prompt assemblies, traces, validation evidence, and final artifacts by durable ids.

## Owner
`context-agent/backend` owns the composition API and DTOs; `context-agent/frontend` owns presentation and interaction state.

## Spec
Satisfies `context-agent-autonomous-client.1` in spec `73aa2bce-cf09-4319-b9c5-758599898724`.

## Provider dependencies
Consume the unified agent harness, durable session/workflow records, and observability contracts. Do not create a second session or telemetry source.

## Acceptance evidence
Provide read-back evidence for one session with durable session, run, message, request, tool, prompt, trace, validation, and artifact ids plus terminal state.

## Validation
- `cargo test -p backend timeline_contract`
- Frontend E2E timeline check

## Boundaries
Do not implement prompt provenance, MCP lifecycle, credit calculation, redaction, or reconnect behavior beyond the shared timeline contract.
## Planning artifact
Canonical ticket artifact: `C:/Users/linus/git/3/meta-workspace/.workflow-tools/ticket/tickets/8b78c940-7c63-412b-8d4c-051a32f573f9/ticket.toml`.

## Contract prerequisite
Before implementation, accept the API endpoint and DTO contract in spec `73aa2bce-cf09-4319-b9c5-758599898724`. CA-01 owns the first contract tests for the timeline event shape and identifier correlation.