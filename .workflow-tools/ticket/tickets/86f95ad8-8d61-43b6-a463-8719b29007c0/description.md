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

## Validation matrix
| Part | Command / evidence |
|---|---|
| Fast check | `cargo check -p agent-web --target wasm32-unknown-unknown` |
| Primary gate | `cargo test` for shared logic + Playwright smoke (control set present, stream renders) |
| Manual/browser | External Chromium-family browser at documented resolution; screenshot of live stream + mode toggle |
| Failure logs | `target/test-logs/` + Playwright artifacts |
