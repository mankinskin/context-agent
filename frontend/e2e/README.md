# Browser validation

Run the owned runner from the repository root; do not attach the suite to an
ambient server:

```powershell
node scripts\e2e-runner.mjs --lane offline --scenario sessions --media all --artifact-root <AR> --budget-ledger <AR>\live-budget.json
```

The existing cumulative ledger must be supplied; never recreate or reset the
ledger. `--help` lists supported selectors. `baseline` checks the responsive
shell, `bridge` checks native failure persistence, and `sessions` creates a
root, child, grandchild and independent root through ordinary UI actions.

Sessions uses native parent IDs for the tree. Select **New session** for an
independent root or **New child session** beneath the active session. The URL
fragment `#session=<id>` retains selection through reload/reconnect. Sessions
automatically hydrate on startup; **Refresh sessions** reloads native state.
Draft/focus survive viewport and orientation changes, not page reload.

The sessions test stays in the same browser context while the runner stops,
joins and restarts only the owned backend on the same port. A synthetic
non-weather message produces a real offline-unavailable error; the runner
reads the resulting four native events before and after restart. No model or
weather request is expected. The artifact verifier checks exact parents,
selected branch, ordered event IDs, served build hashes, unchanged ledger,
productive application-store fingerprints, screenshots/traces and cleanup.

`--media all` runs five isolated Chromium cases: desktop, mobile portrait and
landscape, tablet portrait and landscape. Touch projects perform actual taps.
Affected widths 639/640/641 and 899/900/901, plus orientation changes, preserve
selection/draft/focus. Emulation is not physical-device or iOS validation.

To check retained evidence without starting services or sending requests:

```powershell
node scripts\e2e-runner.mjs --verify-artifacts <AR>\<run-id>
```
