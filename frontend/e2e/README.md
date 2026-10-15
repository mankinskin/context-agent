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

## Global preambles and lookup choice

Expand **Global preambles**, add an ID/text pair, and enable the desired
blocks. Configuration persists in `preambles.json` beneath the explicit native
session-store root. Only enabled blocks enter provider instructions. IDs are
unique ASCII letters/digits/underscore/hyphen (up to 64 bytes); configuration
allows 16 blocks and 4096 bytes of text per block. Storage and validation
failures are visible, not successful empty configurations.

**Start lookup choice** uses the configured provider and the existing synthetic
ticket grant. A successful initial response contains one typed lookup-a/
lookup-b widget. Submitting an option records one native user answer and
executes the follow-up in the same session. Reload restores the stable widget
and disables its submitted controls. Invalid/repeated answers are rejected.
Native tool events remain durable; the conversation renders each tool result
once through its assistant response rather than duplicating native events.

`--lane offline --scenario choice --media all` checks real unavailable/error
behavior, persistence and responsive controls without fabricating an AI widget.
`--lane offline --scenario all --media all` runs baseline, bridge, sessions and
choice across all five projects. The successful widget contract requires the
separately authorized paid flow:

```powershell
node scripts\e2e-runner.mjs --lane live --scenario choice-mcp --media mobile --artifact-root <AR> --budget-ledger <AR>\live-budget.json
```

The live origin submits initial/answer turns; four other projects inspect the
actual retained result without submitting. The runner enables the opt-in
provider observer only for this scenario and compares actual serialized
system/developer preamble SHA-256 and synthetic presence probes with expected
enabled-only instructions. Native config, widget/answer, tool identities,
served build hashes and cleanup are read back separately. Failure remains
failure; partial native choice/config artifacts are captured before cleanup
where available. The cumulative 20-reservation ledger, 1024 output cap,
120-second inclusive deadline and no-automatic-paid-retry rule still apply.
