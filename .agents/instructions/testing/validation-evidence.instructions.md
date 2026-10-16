---
description: "Use when recording validation specs or executions in the test-api store. Covers test-mcp tools, CLI fallback, and evidence linking to tickets."
---

## Recording Validation Evidence (test-api / test-mcp)

Validation results live in a queryable test-result store (`test-api`), not inline in tickets. Record a `ValidationSpec` for each check and a `ValidationExecution` for each run, then reference the stored entries from the ticket.

Store layout (mirrors `.ticket` / `.spec`):

```text
<store-root>/.test/<workspace>/specs/<spec-id>.json
<store-root>/.test/<workspace>/executions/<execution-id>.json
```

### Record via test-mcp (preferred)

Use the `test-mcp` MCP tools when available:

- `test_record_spec` — create/overwrite a validation spec (`id`, `title`, `command`, `detail`, `ticket_ids`, `spec_ids`, `acceptance_criterion_ids`).
- `test_record_execution` — record an outcome (`id`, `validation_spec_id`, `outcome` = `passed|failed|blocked`, `executed_at` RFC3339, `detail`, `ticket_ids`, `spec_ids`, `log_ids`).
- `test_get_spec` / `test_get_execution` — fetch one entry by id.
- `test_list_specs` — list all validation specs.
- `test_list_executions` — query executions by `ticket_id`, `validation_spec_id`, and/or `outcome`.

Always set `ticket_ids` on executions so the evidence can be queried back from the owning ticket.

The handoff package's explicit `entity_store_root` is authoritative. Pass that
same root to every spec write, execution write, and query; do not infer a test
store from `command_cwd`. Immediately after recording an execution, fetch the
same execution id through the same transport and root and verify its outcome,
ticket ids, spec ids, and source pointer. A successful write without this
read-back is not durable validation evidence.

### Record via the `test` CLI (fallback)

```bash
# Record a validation spec
./target/debug/test.exe --store-root "$PWD/.test" \
  record-spec --id vt-core-tests --title "Core unit tests" \
  --command "cargo test -p ticket-vscode-core" --ticket <ticket-id>

# Record an execution linked to the ticket
./target/debug/test.exe --store-root "$PWD/.test" \
  record --id <execution-id> --spec-id vt-core-tests \
  --outcome passed --detail "16 passed" --ticket <ticket-id>

# Query the evidence linked to a ticket
./target/debug/test.exe --store-root "$PWD/.test" --toon list --ticket <ticket-id>
```

### Reference evidence from a ticket

Instead of pasting verbose results into the ticket description, add a concise pointer:

- the store root (e.g. `memory-api/.test/default/`),
- the validation spec ids and execution ids, and
- the `test ... list --ticket <id>` query that reproduces the evidence.

Keep `blocked` outcomes visible in the ticket with their reason, but let the store hold the full command and outcome trail.
