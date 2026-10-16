---
description: "Use when a Worker-tier agent is implementing against pre-authored tests, or when a test appears wrong during implementation. Covers the frontier-authors/worker-implements split, the retry-then-blocker exception path, and Playwright/browser-verification handling for Workers."
---

## Split-Responsibility Testing

Per the Planner/Architect and Worker roles in specification `1b654f30`, implemented by the canonical [write-and-die contract](https://github.com/mankinskin/workflow-tools/blob/main/.agents/instructions/workflow/write-and-die.instructions.md), a Worker executes exactly one plan step inside its declared `target_path` and may not expand scope — this rule specializes that boundary for tests. The specification is workspace-local; the published contract is the portable reference.

- The Planner/Architect tier authors test files for a ticket before any Worker step is dispatched against that ticket.
- **A Worker-tier agent may not edit test files once the frontier tier has authored them for a given ticket.** A Worker's `target_path` for an implementation step is the implementation file(s), not the test file(s) that define its `done_criteria`.
- A Worker may run the pre-authored tests (per [test-execution.instructions.md](test-execution.instructions.md)) and iterate on implementation until they pass. It may not weaken, skip, comment out, or rewrite assertions to make them pass.

## Exception Path — When a Test Is Genuinely Wrong

- A Worker that believes a pre-authored test is incorrect does not edit the test. It reports a blocker via the spec's `{pass: false, blocker: "..."}` contract, per the Worker capability boundary row "Report a blocker ... / MAY NOT: Attempt to re-plan around the blocker itself" (spec 1b654f30, Worker capability boundary table).
- Only the Planner/Architect tier — or a human reviewer acting in that capacity — is authorized to change a test file, after evaluating the reported blocker. The Worker resumes (or a new step is dispatched) once the test is corrected upstream.
- This is distinct from, and composes with, the mid-execution retry cap in [retry-limit.instructions.md](https://github.com/mankinskin/workflow-tools/blob/main/.agents/instructions/workflow/retry-limit.instructions.md): a Worker facing a failing pre-authored test retries once per that file's cap, then escalates via the blocker contract above rather than editing the test itself.

## Playwright / Browser-Verification Interaction

AGENTS.md mandates Playwright E2E coverage and browser verification for browser-facing changes. Under split-responsibility testing:

- The Planner/Architect tier authors the Playwright spec file(s) for a browser-facing step, same as any other test file.
- The Worker **runs** the pre-authored Playwright suite and performs the mandated manual browser verification (opening the affected viewer, confirming the feature visually, capturing screenshots) — it does not author or edit the `.spec.ts`/test file itself.
- If a Worker needs additional coverage for a scenario the pre-authored suite does not exercise, that is a blocker to report upstream (new step, new test authored by the Planner), not a test file the Worker writes itself.
