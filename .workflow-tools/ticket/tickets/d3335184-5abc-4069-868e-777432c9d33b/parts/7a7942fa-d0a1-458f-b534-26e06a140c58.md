## Objective
Prevent raw credentials, provider secrets, and unapproved hidden prompt content from crossing the backend/frontend or logging boundary.

## Owner
`context-agent/backend` owns redacted read models and boundary enforcement; `context-agent/frontend` owns safe rendering.

## Spec
Satisfies `context-agent-autonomous-client.6` in spec `73aa2bce-cf09-4319-b9c5-758599898724`.

## Provider dependencies
Respect provider secret/config boundaries and approved prompt projections.

## Acceptance evidence
Negative fixtures prove secrets and hidden prompt content do not appear in frontend state, API responses, or logs; approved provenance remains visible.

## Validation
- `cargo test -p backend redaction_boundary`
- Credential/secret scan

## Boundaries
Depends on CA-02 and CA-03. Do not broaden into unrelated repository secret scanning.