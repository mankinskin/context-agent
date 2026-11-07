# CH9 — Diff preview integration in both clients

WS5. Consistent code-change preview before execution confirmation.

## Scope
- Integrate `similar` diff rendering shared between clients.
- Show proposed code modifications before execution confirmation in TUI and WASM.
- (WASM) optional `xterm.js` via `web-sys` for terminal stdout mirror if in scope.

## Acceptance criteria
- Identical diff semantics/content in TUI and WASM for the same change set.
- Diff preview appears before the execution confirmation step.

## Dependencies
- depends_on CH7, CH8. Spec: unified-operator-interface AC 6 (transient UI evidence).

## Validation matrix
| Part | Command / evidence |
|---|---|
| Fast check | `cargo check -p agent-tui -p agent-web` |
| Primary gate | `cargo test` — diff model parity (same input -> same hunks) |
| Manual/browser | Playwright screenshot of diff preview open (transient surface) in Chromium at documented resolution |
| Failure logs | `target/test-logs/` + Playwright artifacts |
