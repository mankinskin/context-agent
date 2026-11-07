# CH11 — E2E + Playwright + manual browser verification evidence

WS7 release gate.

## Scope
- End-to-end tests: mode switch (conversation <-> loop), websocket reconnect +
  state continuity, sandbox command policy enforcement, multi-client observation of one session.
- Playwright coverage + screenshots for transient UI states (diff preview,
  mode-switch, live stream).
- Manual browser verification in external Chromium-family browser at a documented resolution.

## Acceptance criteria
- All four e2e scenarios pass.
- Playwright screenshots captured for transient surfaces (open state, before/after where useful).
- Browser resolution used is recorded in the ticket evidence.

## Dependencies
- depends_on CH7, CH8, CH9, CH10. Spec: unified-operator-interface AC 1,2,3,6.

## Validation matrix
| Part | Command / evidence |
|---|---|
| Fast check | `cargo check` across agent-harness crates |
| Primary gate | e2e suite (mode switch, reconnect, policy, multi-observer) + Playwright |
| Manual/browser | External Chromium browser at documented resolution; screenshots attached; MCP Playwright preferred, repo Playwright fallback |
| Failure logs | `target/test-logs/` + Playwright artifacts |
