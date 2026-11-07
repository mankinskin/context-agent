# CH7 — Ratatui minimal operator interface

WS5 native client.

## Scope
- Compact operator panel: session list, current live stream, mode controls.
- Dedicated input/event thread reading background websocket updates async;
  stable refresh, no UI stall on core work.
- Shared minimal interaction model: start conversation, toggle to loop,
  pause/resume/stop, inspect live events.

## Acceptance criteria
- All minimal controls operate against a running `agent-server` session.
- UI stays responsive while a loop streams events.

## Dependencies
- depends_on CH6. Spec: unified-operator-interface AC 1,3.

## Validation matrix
| Part | Command / evidence |
|---|---|
| Fast check | `cargo check -p agent-tui` |
| Primary gate | `cargo test -p agent-tui` — control -> command mapping, async stream handling (headless/mocked server) |
| Manual/browser | Manual TUI smoke run against local server; record terminal size used |
| Failure logs | `target/test-logs/` |
