---
description: "Use when running tests, choosing test scope, or executing validation commands. Covers test execution patterns, scope selection, and frontend/viewer validation."
---

## Quick Reference — Common Commands

```bash
# Run a single test by name (fastest, use first)
cargo test -p <crate> <test_name> -- --nocapture

# Run all tests in a crate
cargo test -p ticket-api
cargo test -p ticket-http

# Full workspace test (slow — only after local crate tests pass)
cargo test
```

## Test Execution Strategy

- Start with nearest unit/integration tests.
- Expand to crate-level runs once local failures are resolved.
- Keep working outward until the required validation passes or you have a clearly repeated blocker to report.
- Prefer the strongest focused validation surface already owned by the changed tool or crate; run the underlying command directly and record the exact command or manual step in ticket/spec summaries.
- For documentation or generated-guidance checks, run the relevant validation command directly and record unsupported coverage or manual follow-up explicitly.
- If dedicated automation is unavailable, use the closest manual or command-line validation path and record the limitation in the status summary.
- Avoid unrelated full-workspace test runs unless required.

For frontend-impacting changes:

- Run lint and typecheck in each affected frontend package.
- Run nearest unit/component tests for changed UI code.
- Run at least one browser-based end-to-end path that covers changed UX behavior.

For viewer/API integration changes:

- Add or run assertions that verify the viewer contract with context-api or ticket-api for changed endpoints.
- For filesystem-backed behaviors, include path-handling and access-boundary assertions.

For performance-sensitive paths (storage, BFS, graph queries):

- Add or run a Criterion benchmark in `crates/<crate>/benches/`.
- Confirm `phase3_get_indexed_many` is used instead of repeated `get_indexed()` calls.

For regression fixes:

- Prefer a failing reproducer assertion before or with the fix.
- Keep regression coverage focused on the reported failure mode.

## Long-Running E2E / Playwright Suites (Background Execution)

A full Playwright or e2e suite (webServer build + many specs) can run for
minutes. Never block on it synchronously and never poll it with `sleep` — the
harness already notifies on async-terminal completion, and a blocking wait
wastes a turn that could catch a systemic failure early.

- Launch the suite detached with output redirected to a log file (for example
  `nohup npx playwright test ... > /tmp/pw-run.log 2>&1 &`), then continue
  working; do not attach to the terminal or wait on it inline.
- Inspect progress by reading a **bounded tail** of the log file (e.g. `tail -c
  2000 /tmp/pw-run.log`), not the full file — this keeps each check cheap
  regardless of how large the suite's output grows.
- Watch for a **systemic failure signature** in the first few completed
  specs: the same connection error (`ECONNREFUSED`, `net::ERR_CONNECTION_REFUSED`),
  the same missing-selector timeout, or a webServer that never became healthy.
  When every test so far fails with the identical root cause, kill the
  detached process immediately instead of letting the rest of the suite run —
  the remaining specs will fail identically and burn the suite's full wall
  time for zero new information.
- Confirm the previous attempt's process has actually exited (no stray
  `cargo`/`playwright` process still holding the package-cache lock) before
  starting a retry; two overlapping cargo invocations serialize on the lock
  and look like a hang. This extends the general rule in
  [tool-output.instructions.md](https://github.com/mankinskin/workflow-tools/blob/main/.agents/instructions/workflow/tool-output.instructions.md)
  ("Long-Running Process Ownership") to the e2e/Playwright case specifically.
