## Objective
Display credit consumption from recorded model identity and token/cost telemetry, marking unavailable fields explicitly.

## Owner
`context-agent/backend` owns the projection API; `context-agent/frontend` owns the display.

## Spec
Satisfies `context-agent-autonomous-client.4` in spec `73aa2bce-cf09-4319-b9c5-758599898724`.

## Provider dependencies
Consume model-price awareness and recorded usage telemetry, including input, output, cache-read, and cache-write fields when available.

## Acceptance evidence
Read back model id, known token fields, calculated known cost, and explicit missing-data state without fabricating values.

## Validation
- `cargo test -p backend credit_telemetry`
- Frontend credit display check

## Boundaries
Depends on CA-02 and CA-03. Do not introduce a second pricing table or claim precision unsupported by telemetry.