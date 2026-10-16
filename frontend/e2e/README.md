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
`--lane offline --scenario all --media all` runs baseline, bridge, sessions,
choice and editor-git across all five projects. The successful widget contract requires the
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
## Rooted editor/Git proof

The offline `editor-git` lane starts an owned backend with
`CONTEXT_AGENT_EDITOR_ROOT` set only to its UUID-owned fixture. The UI uses
`GET/PUT /api/editor/file` for `notes/demo.txt` and `GET /api/editor/diff`.
Selectors: `Rooted file editor`, `File path`, `File content`, `Open file`,
`Save file`, and `Git diff`. Paths outside that single bounded UTF-8 file,
links/reparse points, `.git`, and files over 65536 bytes are rejected.

Run `--lane offline --scenario editor-git --media all` with the same artifact
root and cumulative ledger. Five ordinary UI cases exercise open/edit/save,
real visible diff, rejection/recovery, reload and draft/focus preservation at
the orientation and breakpoint boundaries. No message POST or provider send
is required. Chromium emulation does not establish physical-device/iOS support.

Before fixture cleanup, `scripts/e2e-editor.mjs` separately reads the saved disk
file, independently runs Git diff/root checks, reads the external sentinel,
and exercises traversal/foreign-root and real junction (symlink on Unix)
rejections over HTTP. A missing link fixture blocks the run. Inspect the
retained `editor/demo.txt`, `editor/diff.patch`, `editor/sentinel.txt` and
`editor/readback.json`; artifact verification checks the values and hashes,
not a passing browser label. `cleanup.json` must report joined processes,
zero remaining resources and absent fixtures. The ledger must remain identical.

Backend reads are bounded to 65537 bytes for size rejection; saves use an
exclusive same-directory temporary file, sync and atomic replacement with
failure cleanup. Git ignores inherited `GIT_*` overrides and global/system
configuration, disables external diff/textconv/fsmonitor/pagers, limits each
command to 3 seconds, root output to 8192 bytes and diff output to 262144
bytes. Failure returns structured, correlated diagnostics. The editor is
disabled unless the operator explicitly configures the owned repository root.

## Final-build and container selectors

`--lane container --scenario baseline --media all` builds the current local
Dockerfile and dependency graph unchanged. The runner uses an existing local
`infrastructure-base:local` (or `E2E_BASE_IMAGE`), unique labelled image/container
names and an ephemeral IPv4 loopback port. No SSH agent is forwarded by default.
For the approved Windows OpenSSH agent, explicitly set
`$env:E2E_SSH_AUTH_SOCK = '\\.\pipe\openssh-ssh-agent'` before the container
command. The socket is forwarded only to Docker build via `--ssh`; private key
contents are never read or supplied as a secret, and runtime containers receive
no agent or provider credential. The manifest records whether forwarding was
enabled. Trust the Dockerfile before exposing the agent signing capability.
If the working dependency graph requires SSH access without that opt-in,
the build fails visibly; the runner never rewrites the manifests or substitutes
a previously built image. Actual health/session responses, UI session IDs,
served static hashes, copied image executable, image identity, stopped-container
exit and absent owned resources are read back separately. The legacy
`scripts/smoke-container.sh` checks only HTTP health/root/session contracts,
not responsive browser acceptance; Context Agent has no template echo contract.

`--lane live --scenario all --media tablet` runs the existing spine then
choice-MCP scenarios sequentially, each with a separate 120-second inclusive
deadline, one tablet-portrait submitting origin and four retained no-submit
contexts. A failed spine terminates the sequence. `--media all` remains
forbidden for live execution; the selector never resets or retries the shared
ledger. This final paid gate is deferred while required container coverage
is blocked. `--verify-artifacts <run-id-directory>` is exclusive; mixing
execution selectors is rejected before any resource or provider starts.
