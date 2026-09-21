## Objective
Expose prompt assembly provenance for every included instruction file, agent template, skill, and prompt template.

## Owner
`context-agent/backend` owns the composition API and redacted prompt read model.

## Spec
Satisfies `context-agent-autonomous-client.2` in spec `73aa2bce-cf09-4319-b9c5-758599898724`.

## Provider dependencies
Consume the agent-template roster, instruction-file, skill, and prompt-template contracts. Preserve source identity, revision or path where available, and inclusion reason.

## Acceptance evidence
A prompt assembly read-back lists every included source with source identity and inclusion reason, while unapproved hidden prompt content remains protected.

## Validation
- `cargo test -p backend prompt_provenance`

## Boundaries
Depends on CA-01. Do not invent a parallel guidance catalogue or implement general MCP lifecycle management.