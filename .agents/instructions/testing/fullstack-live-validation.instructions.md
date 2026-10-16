---
description: "Use when authoring or running the full-stack live-validation workflow that drives real browser/backend/provider flows across responsive media projects, distinct from mocked, offline-only, or forced-success coverage."
applyTo: "**/e2e-runner.mjs,**/*.live.spec.*,**/*.live.e2e.*,**/live-budget.json"
---

# Full-Stack Live Validation

## Purpose

This instruction defines the normative workflow and evidence contract for a
canonical full-stack live-validation run: one that drives a real browser
against a real backend (and, where in scope, a real provider) and proves the
result through read-back, not through test count. It specializes
[end-to-end-testing.instructions.md](end-to-end-testing.instructions.md)'s
real-backend/no-mock rules for the responsive, budget-constrained, live case;
it does not restate them.

## Workflow

Every full-stack live-validation scenario follows, in order:

1. **Discover** the owned runner and its `--help` surface before relying on any
   selector or flag; unknown selectors must fail rather than silently no-op.
2. **Authored acceptance** — tests and scenario criteria are authored before a
   Worker-tier implementation step runs against them, per
   [split-responsibility-testing.instructions.md](split-responsibility-testing.instructions.md).
   A Worker may run but not weaken pre-authored scenarios.
3. **Owned runner** executes the scenario; it owns fixture identity (UUIDs,
   ports, PIDs), build provenance (executable/hash, git revision, static
   asset hash) and the shared budget ledger. Ambient/ad-hoc running services
   are not accepted build provenance.
4. **Actual browser/backend/tool** — ordinary locators and real user actions
   (mouse, keyboard, touch) drive a real, fully wired backend and, for
   live-tier scenarios, a real provider/tool path. No forced clicks, no
   `page.route` interception, no fake/stubbed/in-memory backend, no mocked
   successful output, substituting for genuine execution.
5. **Maintainer read-back** — after the browser flow completes, a separate
   pass reads the runner's native manifest, events, assertions, and the
   budget ledger from disk. This is evidence verification, not a browser
   assertion, and follows
   [validation-evidence.instructions.md](validation-evidence.instructions.md)
   for recording and re-fetching the validation spec/execution.
6. **Review / user feedback** — present the same owned build and artifacts
   interactively; a scoped replan may follow feedback, but silence does not
   authorize extra scenario runs or a budget reset.

## Evidence Tiers

Keep these tiers distinct in every report; do not let one stand in for another:

- **Offline** — credential-free, no provider call; exercises UI and backend
  only.
- **Native** — real locally built executable/backend, no container layer.
- **Provider-fixture** — a unit/provider-boundary fixture that simulates a
  failure mode; permitted only as a non-live boundary check, never presented
  as live evidence.
- **Live** — a real outbound provider/tool call through the real backend,
  subject to the budget ledger below.
- **Container** — the same real backend/provider boundary rules, running
  inside the repository's container fixture instead of a native process.

A passing offline or provider-fixture run is never reported as proof of live
or container behavior.

## Implemented Runner Selectors

Run `node scripts/e2e-runner.mjs --help` from the receiving repository.
The shared native/offline and container browser selector is `--media all`.
For Context Agent, offline `--scenario all` covers baseline, bridge, sessions,
choice and editor/Git; the template supports only its generic echo scenario.
Container selection is `--lane container --scenario baseline --media all`
for Context Agent, and `--lane container --scenario echo --media all` for the
template. Never copy product scenarios into the template.

Context Agent's final live selector is
`--lane live --scenario all --media tablet`: spine then choice sequentially,
not five paid device runs. Live `--media all` is rejected. Supply the same
explicit `--artifact-root` and `--budget-ledger` throughout the route.
`--verify-artifacts <run-directory>` is a separate, exclusive read-only
operation; never combine verification with execution flags. Read every
produced run directory, not only the last project.

## Media And Responsive Acceptance

Use Chromium emulation only; never claim physical-device or iOS coverage.
Required projects, each a distinct browser context recording media, viewport,
orientation and input mode:

- Desktop `1440x900`, mouse and keyboard.
- Mobile `390x844` (portrait) and `844x390` (landscape), touch.
- Tablet `768x1024` (portrait) and `1024x768` (landscape), touch.

A missing, failed, or skipped required project blocks the scenario; it is not
a success-shaped skip.

Layout follows viewport width, not a device/user-agent label:

- `<=640`px — one primary pane.
- `641-900`px — primary content with collapsible secondary access.
- `>=901`px — parallel panes.

Re-check the `639/640/641` and `899/900/901` boundaries whenever a change
affects layout. Primary touch actions (navigation, choice, save/submit) are
at least `44x44` CSS pixels on touch projects. Forbid document-level
horizontal overflow. Preserve implemented selection/draft/widget/editor/focus
state across resize and orientation changes; if a focused control becomes
hidden during a layout transition, move focus to its visible counterpart
instead of requiring focus to remain on a hidden node.

## Sparse Live Origins

Live traffic is not a Cartesian product of scenario x media. The approved
route uses desktop-origin spine, mobile-portrait-origin choice, then final
tablet-portrait-origin spine and choice. Each scenario run has one
live-submitting origin context; every other context for that run inspects the genuine retained/persisted UI state
through ordinary navigation and never submits to the provider. Retained-state
inspection proves responsive rendering of already-produced state; it is not
fresh model execution and must not be reported as one.

## Timing And Budget Discipline

- Offline scenario ceiling: 60 seconds inclusive of cleanup.
- Live scenario ceiling: 120 seconds inclusive of requests, tool round-trips,
  assertions, and cleanup, with the last 10 seconds reserved for
  cancel/join.
- Provider calls use exactly the model and output-token cap the owning
  runner declares (for this workflow family: `gpt-5-mini`, max output 1024
  tokens at the provider transport).
- Reuse one durable, exclusively locked cumulative reservation ledger across
  the whole route and any reruns; reserve pessimistically at the transport
  boundary before each real outbound attempt, including tool follow-ups. The
  reservation ceiling is 20 for this workflow family and is never
  exceeded, including across crashes or restarts. A missing ledger blocks
  live calls.
- No automatic retry and no "repeat until green" loop on a live or
  budget-bound scenario; exhaustion is a blocker, not a reason to widen the
  budget.

## Credential Boundary

- `gh` credential retrieval happens only inside the server-side credential
  boundary (for example, a captured `gh auth token` behind configured
  `GITHUB_COPILOT_TOKEN`/`COPILOT_API_KEY` precedence); never forward a token
  to a browser context, MCP child process, trace, or artifact.
- SSH agent forwarding for a container image build is opt-in only through an
  explicit `E2E_SSH_AUTH_SOCK` environment variable read by the owned build
  step. An ambient `SSH_AUTH_SOCK` does **not** opt a build in by itself.
- Forward the socket only to the trusted Docker build using `--ssh`;
  runtime containers never receive an agent socket or provider credential.
  The manifest records `container.ssh_agent_forwarded`.
- Never read, print, or forward a private key file, and never start a second
  SSH/credential agent on the run's behalf.
- Template default CI builds stay native and credential-free. A container
  build that needs real credentials is a manual/production-gated path, never
  the default CI build.

## Read-Back And Cleanup

Browser pass counts do not prove captured data. Separately inspect the actual
native session/tool transcript, outbound preamble/request cap, ledger and
image/executable/served-static hashes relevant to the scenario. Editor
acceptance requires the saved disk value, Git diff and unchanged external
sentinel. A schema field or source code that should produce a value is not
read-back evidence. Unknown provider usage/cost remains unknown.

Every process/container/fixture is run-owned; cleanup cancels and joins
children within the deadline and records zero remaining resources and absent
owned fixtures. Record terminal status and inspect the existing execution
before retrying; never launch a duplicate to diagnose a quiet process.
Do not stop unrelated services or prune Docker globally. Container build,
readiness and scenario budgets are separate; the 60-second scenario ceiling
is not a promise that a cold image builds in 60 seconds.

## Non-Goals

- This file does not define a new agent, a paid-by-default policy, or a
  general accessibility standard.
- This file does not replace
  [end-to-end-testing.instructions.md](end-to-end-testing.instructions.md),
  [test-execution.instructions.md](test-execution.instructions.md),
  [split-responsibility-testing.instructions.md](split-responsibility-testing.instructions.md),
  or [validation-evidence.instructions.md](validation-evidence.instructions.md);
  it composes with them for the live, responsive, budget-constrained case
  only.
