# context-agent

A new tool instantiated from the `template` Dioxus (frontend, WASM) + Axum
(backend) starter app. Currently unmodified starter content -- no
Context Agent-specific feature work has been added yet. Follow
[USAGE.md](./USAGE.md) to add real endpoints and UI.

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

- `frontend/` — a Dioxus 0.6 web app (Trunk + wasm32 target), rendering a
  single input that POSTs to `/api/echo` and shows the result.
- `backend/` — an Axum app exposing `GET /health` and a stub
  `POST /api/echo`, serving `frontend/dist` for every other path.
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

## Owned offline browser baseline

The credential-free owned runner builds the frontend, starts an isolated
backend on a free loopback port, provisions synthetic external fixtures, and
runs only `frontend/e2e/baseline.spec.ts`. It never uses the ambient server
or the weather/provider path. Preserve the artifact root and ledger between
runs:

```sh
node scripts/e2e-runner.mjs --lane offline --scenario baseline \
  --artifact-root <retained-artifact-root> --budget-ledger <retained-artifact-root>/live-budget.json
```

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
`infrastructure-base` package:

```sh
docker build -t template-local .
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
docker build --build-arg BASE_IMAGE=infrastructure-base:local -t template-local .
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
