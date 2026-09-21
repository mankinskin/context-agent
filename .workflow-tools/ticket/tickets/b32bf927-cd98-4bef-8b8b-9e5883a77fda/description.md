## Objective
Represent and display reasoning traces, model responses, command/tool traces, validation output, and final artifacts as distinct correlated evidence classes.

## Owner
`context-agent/backend` owns the evidence projection; `context-agent/frontend` owns the presentation.

## Spec
Satisfies `context-agent-autonomous-client.5` in spec `73aa2bce-cf09-4319-b9c5-758599898724`.

## Provider dependencies
Consume runtime logs, bounded command/spill output, validation evidence, and artifact records. A model response must never stand in for execution evidence.

## Acceptance evidence
Read back one timeline where trace, response, validation, tool, and artifact records remain structurally distinct.

## Validation
- `cargo test -p backend evidence_classes`
- Frontend evidence display check

## Boundaries
Depends on CA-02 and CA-03. Do not redesign provider observability storage.