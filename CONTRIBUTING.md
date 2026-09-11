# Contributing to template

`template` is a Rust workspace with an Axum backend and a Dioxus/Trunk
frontend. The backend serves the built frontend from the same origin.

## Before You Start

Install the project toolchain:

```bash
rustup toolchain install 1.90.0
rustup target add wasm32-unknown-unknown --toolchain 1.90.0
cargo install trunk --version 0.21.14 --locked
```

Docker builds pull the private `infrastructure-base` image. Sign in to
GitHub Container Registry with a token that has `read:packages` access when
the image is not already available locally:

```bash
docker login ghcr.io
```

If you added a private Git dependency (see `USAGE.md`'s "Adding a private
git dependency" section), add a key with read access to that dependency to
your SSH agent before running Cargo or Docker commands:

```bash
ssh-add ~/.ssh/<your-private-dep>_ed25519
ssh -T git@github.com
```

Never commit tokens, private keys, generated `frontend/dist` assets, or local
environment files.

## Development Workflow

1. Start from an up-to-date `main` branch and make a focused change.
2. Add or update a focused regression test for changed behavior.
3. Run the checks in [Verification](#verification) before opening a pull
   request.

For a local browser workflow, build the frontend and start the backend:

```bash
(cd frontend && trunk build --release)
STATIC_DIR=frontend/dist cargo run -p backend
```

Open `http://127.0.0.1:8080`.

`scripts/start-local-container.sh` builds and starts the containerized local
workflow, waiting for `/health`.

## Verification

Run the narrowest relevant check while developing. Before submitting a change,
run the full applicable set from the repository root:

```bash
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd frontend && trunk build --release)
```

For HTTP and browser changes, also run:

```bash
bash scripts/smoke-browser.sh
```

For Dockerfile, runtime configuration, or container-delivery changes, also run:

```bash
docker build -t template:local .
bash scripts/smoke-container.sh template:local
```

The CI workflow runs the same four gates: Rust quality, backend contract
tests, frontend browser smoke, and container API smoke. A pull request is
ready only when the relevant local checks pass or a concrete environment
blocker is documented in the pull request.

## Pull Requests

Keep each pull request limited to one behavior or architectural unit.
Describe the user-visible effect, tests added or updated, and commands run.
Call out configuration, migration, security, or compatibility changes
explicitly.
