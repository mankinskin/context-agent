---
description: "Use when authoring, reviewing, or validating end-to-end (E2E) tests that must drive full-stack applications through user-observable interfaces without backend mocking or in-test power features."
applyTo: "**/e2e/**,**/*.e2e.*,**/*.spec.ts,**/*.spec.tsx,**/*.spec.js,**/*.spec.jsx"
---

# End-to-End Testing: UI-Driven Authoring Rules

## Purpose

This instruction file defines the repository's normative rules for authoring end-to-end (E2E) tests that exercise full-stack behavior through user-observable interfaces only. The scope is intentionally narrow: it specifies what an E2E test may drive, what an E2E test may assert, and which bypass-oriented power features are forbidden inside checked-in E2E test files.

For test execution scope, assertion quality, Planner/Worker authorship, and validation evidence recording, follow the cross-references below instead of restating those rules here.

## User-Observable Interface

A user-observable interface is an input or output surface a real user could employ or perceive, including:

- Web or graphical frontends driven through browser automation via mouse, keyboard, or accessibility-locator interactions.
- Terminal or CLI frontends driven through spawned processes whose stdin/stdout/stderr represent the user surface.
- Persisted or exported outputs that the user can reach through those frontend flows.

Internal function calls, programmatic RPCs, storage writes, fixture hooks, or test helpers that mutate service state without going through a user-observable action are not part of the user-observable interface.

## Allowed Actions

Checked-in E2E tests may:

- Type text into inputs, select items, click buttons, and send mouse or keyboard actions that a real user could perform.
- Use role, label, and other accessibility-tree locators, such as Playwright `getByRole` or `getByLabel`, because those locators refer to user-perceivable structure.
- Launch the real backend process or the repository's normal service startup flow for the test run.
- Capture screenshots, accessibility snapshots, CLI output, and test logs as validation evidence.

## Forbidden Bypasses

The following are forbidden inside checked-in E2E test files (`*.spec.*`, `*.e2e.*`) and scripts whose purpose is to run those E2E tests:

- Directly calling internal APIs, functions, or module-level entry points to produce application state that would otherwise require a user action.
- Injecting state or mutating persistence/storage except via actions a real user could take through the user-observable interface.
- Using in-test `eval` or REPL-like features, `state-save` / `state-load` CLI helpers, or equivalent repository-adjacent power features. These tools remain available for ad-hoc interactive debugging outside committed E2E test code.
- Network-level interception or mocking that prevents the test from exercising the real backend. `page.route` or equivalent request interception that returns synthetic application API responses is forbidden for checked-in E2E tests.
- Starting a fake, stubbed, or in-memory backend process in place of the real, fully wired backend.

These forbiddances keep checked-in E2E tests truthful: a passing E2E test must prove the user-facing flow works through the real stack, not that a test-only shortcut can be made to pass.

## Real Backend Requirement

Every checked-in E2E test must run against a real, fully wired backend. Running the real backend binary or service in a reproducible test deployment, such as a docker-compose fixture that starts the actual service, satisfies this rule when the fixture exposes the same endpoints and configuration shape as the normal service.

A fake, stubbed, or in-memory substitute does not satisfy the real backend requirement. Network interception that replaces application API responses with synthetic data also violates the real backend requirement, even when browser actions are otherwise user-like.

## Live Validation After Fixes

When the requested work fixes a browser-visible, API-visible, or container-visible behavior, checked-in tests are not enough by themselves. Run one explicit live validation path through the deployed local service that the user would actually open.

Live validation must record all of the following when available:

- The startup command or task and the URL under test.
- The real fixture files, credentials mode, and user actions used to drive the flow.
- The relevant API statuses and stable response fields observed during the flow.
- The final visible UI state: success artifacts, displayed error details, or both.
- Downloaded or exported outputs reached through the UI when the flow claims to produce artifacts.

Do not stop at a DOM assertion when the user reported a live container/browser failure. Rebuild or restart the same local service path, then drive the page with real mouse, keyboard, file upload, and button-click actions. If the app runs in Docker, follow the Docker log and rebuild rules in [docker-container-debugging.instructions.md](https://github.com/mankinskin/workflow-tools/blob/main/.agents/instructions/workflow/docker-container-debugging.instructions.md).

## Assertion Surface

E2E assertions must be scoped to things a user can observe:

- Rendered DOM or graphical output.
- Accessibility-tree roles, labels, names, and states.
- Screenshots or targeted visual snapshots.
- CLI stdout/stderr and terminal UI state.
- Persisted, exported, or downloaded outputs the user reaches through the UI or CLI.

Assertions must not inspect internal-only artifacts such as in-memory objects, process-local variables, internal module state, or server logs that are not exposed to a real user through the tested interface.

Server logs and Docker logs may be used as supplemental debugging evidence outside the E2E assertion itself. Log inspection proves maintainer-visible trace behavior; it does not replace user-observable assertions for the E2E pass.

Snapshots are permitted as a verification mechanism. Prefer targeted snapshots over broad page snapshots when a focused assertion communicates the intended behavior more clearly.

## Cross-References

Follow these related instruction files for adjacent testing responsibilities:

- [test-execution.instructions.md](test-execution.instructions.md) covers test-scope selection, validation commands, and browser-based execution strategy.
- [assertions.instructions.md](assertions.instructions.md) covers assertion quality and regression-test focus.
- [split-responsibility-testing.instructions.md](split-responsibility-testing.instructions.md) covers Planner/Worker authorship boundaries for tests.
- [validation-evidence.instructions.md](validation-evidence.instructions.md) covers recording validation evidence and linking evidence to tickets.
- [fullstack-live-validation.instructions.md](fullstack-live-validation.instructions.md) covers owned responsive native/container/live runners, sparse live origins, budget and artifact read-back.
- [error-boundary-handling.instructions.md](https://github.com/mankinskin/workflow-tools/blob/main/.agents/instructions/workflow/error-boundary-handling.instructions.md) covers frontend-facing error details versus trace-level maintainer context.

## Non-Goals

- This file does not mandate a single E2E framework such as Playwright or Puppeteer.
- This file does not define unit, component, integration, benchmark, or HTTP stress testing practice.
- This file does not retroactively rewrite existing E2E suites or CI scripts; non-compliant existing flows need separate remediation work.
