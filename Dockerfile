# syntax=docker/dockerfile:1

# Base toolchain (Rust + wasm32 + trunk + cargo-chef) is inherited from the
# `infrastructure` repo's published image; this repo has no toolchain-setup
# of its own. Defaults to the pinned, digest-verified GHCR image (package 06
# proved this exact digest is clean-pullable via
# `scripts/verify-digest-pull.sh`). The `infrastructure-base` package is
# private, so pulling it requires `docker login ghcr.io` with a credential
# that has `read:packages` access (e.g. `INFRA_GHCR_PAT`). Without that
# access, override with
# `--build-arg BASE_IMAGE=infrastructure-base:local` after building it
# locally in a sibling `infrastructure` checkout:
#   docker build -t infrastructure-base:local --target base .
ARG BASE_IMAGE=ghcr.io/mankinskin/infrastructure-base@sha256:d0823f9fa4b323a2abdc23058b22dfdf947489e6558a1d4e0c180dcceec61247
FROM ${BASE_IMAGE} AS chef
WORKDIR /app

# Captures the workspace's dependency graph (Cargo.toml/Cargo.lock only, no
# source), so this recipe only changes when a manifest or lockfile does —
# never on an ordinary source edit.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Compiles third-party dependencies (host target, dev profile, every
# target + clippy's own fingerprint) into a real image layer keyed on the
# recipe alone, so a source-only change never re-triggers this step.
FROM chef AS deps-dev
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --recipe-path recipe.json --workspace --all-targets && \
    cargo chef cook --clippy --recipe-path recipe.json --workspace --all-targets

# Same idea, for the frontend's wasm32 target in release profile — matching
# how `trunk build --release` actually compiles it.
FROM deps-dev AS deps-wasm
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json --target wasm32-unknown-unknown --package frontend

# Same idea again, release profile, for the production backend binary.
FROM chef AS deps-release
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json --package backend

FROM deps-release AS deps-release-wasm
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json --target wasm32-unknown-unknown --package frontend

# CI target: docker build --target lint .
FROM deps-dev AS lint
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    bash scripts/docker/lint.sh

# CI target: docker build --target test .
FROM deps-dev AS test
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo test --workspace

# Production image target: docker build -t template-local .
FROM deps-release-wasm AS builder
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    bash scripts/docker/build-release.sh

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/dist/backend ./server
COPY --from=builder /app/frontend/dist ./frontend/dist

ENV STATIC_DIR=/app/frontend/dist
EXPOSE 8080
ENTRYPOINT ["./server"]
