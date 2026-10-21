# context-agent

A Context Agent playground built on the `template` Dioxus (frontend, WASM) +
Axum (backend) starter. Native persistent sessions and an owned offline
browser/restart proof are implemented. Responsive navigation, selective
preambles/widgets and the rooted editor remain separate roadmap work.

When this repo is checked out as the `context-agent` submodule inside the
`meta-workspace` superproject, the template-modernization dossier that
instantiated it is available at
[../transcripts/11-09-2026_file-combiner-template-modernization/](../transcripts/11-09-2026_file-combiner-template-modernization/README.md).
A standalone clone of this repo does not include that dossier.
When this repo is checked out as the `template` submodule inside the
`meta-workspace` superproject, the split-plan design dossier is available at
[../transcripts/28-08-2026_file-combiner-template-infrastructure/](../transcripts/28-08-2026_file-combiner-template-infrastructure/README.md).
A standalone clone of this repo does not include that dossier.

## What's here

- [frontend/](./frontend/) — a Dioxus 0.6 web app (Trunk + wasm32 target)
  with session creation, refresh, selection, timeline and diagnostics.
- [backend/](./backend/) — an Axum app exposing health, session, message,
  timeline, credits and weather routes, with same-origin static serving.
  Native Session API records are authoritative; in-memory state is a cache.
- `Dockerfile` — builds `FROM ${BASE_IMAGE}` (an `ARG` defaulting to the
  digest-verified `ghcr.io/mankinskin/infrastructure-base@sha256:...`
  image), inheriting the shared Rust/wasm/trunk toolchain and `cargo-chef`
  setup from the `infrastructure` repo. Defines
  `lint`/`test`/`builder`/`runtime` stages compatible with
  `infrastructure`'s `templates/ci-template.yml`. The GHCR package is
  private; without `read:packages` access, override with
  `--build-arg BASE_IMAGE=infrastructure-base:local` (see below).
- `build.sh` — local helper: `trunk build --release` for the frontend,
  `cargo build --release -p backend` for the backend.
- `scripts/run-container.sh` — builds and starts the container locally,
  serving the frontend at `http://127.0.0.1:8080` by default.
- `scripts/smoke-container.sh <image-tag> [port]` — runs a built image and
  checks `/`, `/health`, `/api/echo`; documented exit codes 10-14 for CI to
  interpret failures (see the script header).
- `scripts/verify-digest-pull.sh <org>/infrastructure-base@sha256:<digest>`
  — package 06's digest-based publish-to-consume check: pulls the exact
  digest, builds this repo against it, scans the pulled image for known
  secret patterns, and runs `smoke-container.sh` against the result.
  Classifies any failure into `registry`, `template`, or `secret-leak`
  (exit codes 20-22; see the script header). Requires the caller to have
  already authenticated to GHCR (e.g. `docker login ghcr.io`) if the
  package is private — this script never handles credentials itself.

See [USAGE.md](./USAGE.md) for how to extend this into a real application.

## Native sessions and provider boundary

`SESSION_STORE_ROOT` selects the native Session API store; manual development
defaults to `.session` relative to the backend working directory. Tests use
isolated fixture roots. Parents are validated, updates serialized, and genuine
user/prompt/request/result-or-failure events persisted before returning.
Stable message IDs are idempotent for the same content and prompt provenance;
changed payloads conflict. Failed IDs are not silently retried. Refresh and
selection restore persisted timelines after a backend restart.

Live requests use the published restricted Agent Builder execution policy,
not the unrestricted legacy constructor. Operators must supply
`CONTEXT_AGENT_CONFIG`, `CONTEXT_AGENT_BUDGET_LEDGER`,
`CONTEXT_AGENT_EVIDENCE_ROOT` and `CONTEXT_AGENT_PROVIDER_SESSION_ID`.
The policy bounds model/output, cumulative reservations and owned MCP
shutdown. Credentials stay server-side. The offline runner deliberately
omits live configuration: AI is unavailable, never a fabricated success.
Standalone lifecycle requests return an explicit unavailable diagnostic until
a real lifecycle operation is implemented. Missing token/cost telemetry is
unknown, not zero. The global write mutex prioritizes correct ordering over
independent-session throughput.

## Owned offline browser baseline and native bridge

The credential-free owned runner builds the frontend, starts an isolated
backend on a free loopback port, provisions synthetic external fixtures, and
runs [baseline.spec.ts](./frontend/e2e/baseline.spec.ts) or
[bridge.spec.ts](./frontend/e2e/bridge.spec.ts). It never uses the ambient server
or the weather/provider path. Preserve the artifact root and ledger between
runs:

```sh
node scripts/e2e-runner.mjs --lane offline --scenario baseline \
  --artifact-root <retained-artifact-root> --budget-ledger <retained-artifact-root>/live-budget.json
```

Use `--scenario bridge` with the same retained root and ledger for the native
proof: ordinary UI submission, explicit offline failure, owned backend
stop/join/restart, then ordinary UI restoration. Retained manifest/transcript
copies prove the same session/start/parent and four ordered unique events,
including failure and no assistant success. Productive session-store
fingerprints of native `source=context-agent` manifests/transcripts must remain
identical. Concurrent agent capture records and store indexes are outside that
application-data scope; no whole-workspace no-write claim is made. The receipt
records the exact roots, source filter, exclusion and observed hashes.

`--help` documents the complete interface. Live and future scenarios fail
explicitly until they have an owned implementation.

The route owner initializes the cumulative ledger once before the first run;
the runner requires the existing ledger and never creates or resets the
budget. Offline runs retain before/after ledger snapshots and require identical
bytes, even when earlier live runs have already spent reservations.

Each UUID directory retains the built executable and complete static assets,
served-byte hashes, browser JSON result, trace, desktop/narrow screenshots,
sequenced events, bounded redacted logs and observed cleanup records. The
backend runs from that retained build, so a later rebuild does not invalidate
earlier evidence. Cleanup preserves the directory and ledger, deletes only
the run's fixtures and cancels/joins only recorded process trees. Historical
process records include exit timestamps because operating systems reuse PIDs.

Builds have independent 15-minute ceilings. The offline scenario starts after
builds, has a 60-second ceiling and reserves its final 10 seconds for cleanup;
readiness is capped at 30 seconds within that budget. A failed build,
readiness, browser flow or cleanup retains failed evidence rather than
success-shaped assertions. To check a retained run without starting services:

```sh
node scripts/e2e-runner.mjs --verify-artifacts <retained-artifact-root>/<run-uuid>
node --test scripts/e2e-runner.test.mjs
```

The verifier checks actual result counts, artifact hashes, process/event
correspondence and ledger snapshots, not just passing labels or file presence.
The baseline does not select the optional live-weather test; running the
existing Playwright suite directly still includes that test.

## Local build (no Docker)

```sh
./build.sh
# frontend/dist now exists; run the backend directly:
STATIC_DIR=frontend/dist cargo run --release -p backend
```

## Local build (Docker, default: pulls the pinned GHCR digest)

Requires `docker login ghcr.io` with `read:packages` access to the private
`infrastructure-base` package and an SSH agent holding a deploy key with
read access to the private `agent-builder` repository:

```sh
docker build --ssh default -t template-local .
docker run -p 8080:8080 template-local
# http://localhost:8080/       -> the Dioxus app
# http://localhost:8080/health -> 200 "ok"
```

Or use the helper:

```sh
bash scripts/run-container.sh
# http://127.0.0.1:8080/ -> the Dioxus app
```

Override the defaults with `IMAGE`, `CONTAINER_NAME`, or `PORT`, for example:

```sh
PORT=9090 IMAGE=my-app:local bash scripts/run-container.sh
```

## Local build (Docker, no GHCR access)

```sh
# in a sibling checkout of the infrastructure repo:
docker build -t infrastructure-base:local --target base .

# back in this repo:
docker build --ssh default --build-arg BASE_IMAGE=infrastructure-base:local -t template-local .
docker run -p 8080:8080 template-local
```

## Verifying a published digest (package 06 — already proven for the digest above)

To re-verify (e.g. after a new `infrastructure` release):

```sh
docker login ghcr.io  # if the package is private; not handled by the script
bash scripts/verify-digest-pull.sh mankinskin/infrastructure-base@sha256:<digest>
```

A mutable tag alone (`:v0.1.3`) is never sufficient pinning — always use
the exact digest. Once this passes, either build with
`--build-arg BASE_IMAGE=ghcr.io/mankinskin/infrastructure-base@sha256:<digest>`
going forward, or change the `Dockerfile`'s `ARG BASE_IMAGE` default to that
digest.
