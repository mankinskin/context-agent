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
