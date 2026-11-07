## Validation matrix
| Part | Command / evidence |
|---|---|
| Fast check | `cargo check -p agent-tui -p agent-web` |
| Primary gate | `cargo test` — diff model parity (same input -> same hunks) |
| Manual/browser | Playwright screenshot of diff preview open (transient surface) in Chromium at documented resolution |
| Failure logs | `target/test-logs/` + Playwright artifacts |
