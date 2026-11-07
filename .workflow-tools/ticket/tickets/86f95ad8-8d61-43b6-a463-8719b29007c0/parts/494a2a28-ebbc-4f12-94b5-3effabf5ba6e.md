# CH8 — Dioxus/WASM minimal interface parity

WS5 browser client.

## Scope
- Dioxus app compiled to `wasm32-unknown-unknown`, mirroring the same control set
  and session semantics as the TUI.
- Websocket consumption of lifecycle + incremental output.
- Same minimal interaction model; presentation may differ, semantics must match.

## Acceptance criteria
- Browser client exposes start / toggle-loop / pause / resume / stop / inspect.
- A session started in the TUI is observable and controllable from the browser (parity).

## Dependencies
- depends_on CH6. Spec: unified-operator-interface AC 1,2,3.

