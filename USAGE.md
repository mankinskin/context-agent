# USAGE

How to turn this starter into your own app.

## Replace the echo endpoint

1. In `backend/src/lib.rs`, replace the `echo` handler and its
   `EchoRequest`/`EchoResponse` types with your own `/api/<name>` route and
   payload types. Add the route to the `Router` in `router()`.
2. In `frontend/src/main.rs`, replace the `call_echo` function and the
   button's `onclick` handler with a call to your new endpoint. Keep the
   `fetch`-via-`web-sys` pattern (same-origin, JSON body) unless you have a
   reason to change it.
3. Add any new request/response types to both crates' `Cargo.toml` if you
   need extra `serde`-derivable fields, or share a `contract` crate/module
   the way `file-combiner` does (see below).

## Adding a private git dependency (e.g. `agent-builder`)

`file-combiner` is the reference example for wiring a private
`ssh://git@github.com/...` dependency through this same
`infrastructure`-based Docker/CI shape:

1. Add the dependency to `backend/Cargo.toml`:
   ```toml
   agent-builder = { git = "ssh://git@github.com/mankinskin/agent-builder", branch = "main" }
   ```
2. In every Dockerfile stage that runs `cargo chef cook` or otherwise
   touches that dependency, mount the forwarded SSH agent and wrap the
   command with the inherited `scripts/docker/with-git-ssh-key.sh`:
   ```dockerfile
   RUN --mount=type=ssh --mount=type=cache,target=/usr/local/cargo/registry \
       --mount=type=cache,target=/usr/local/cargo/git \
       bash scripts/docker/with-git-ssh-key.sh cargo chef cook ...
   ```
3. In CI (see `infrastructure`'s `templates/ci-template.yml`), load a deploy
   key via `webfactory/ssh-agent` and pass `ssh: default=${{ env.SSH_AUTH_SOCK }}`
   to every `docker/build-push-action` step.
4. For non-agent-forwarding hosts (e.g. a Render Secret File), see
   `with-git-ssh-key.sh`'s Secret-File path — no Dockerfile change needed,
   only an `SSH_SECRET_FILE`-mounted deploy key at deploy time.

See `file-combiner/Dockerfile` in this workspace for the full worked
example (multiple `deps-*` stages, `frontend-smoke` browser test, etc.).

## CI

`.github/workflows/ci.yml` already runs `rust-quality` (fmt/clippy),
`backend-contract` (`cargo test -p backend`), `frontend-browser`
(`scripts/smoke-browser.sh`), `container-api`
(`scripts/smoke-container.sh`), and — on push to `production` —
`deploy-render` against every push/PR. Adjust job steps to match any
stages, routes, or scripts you add or rename.

## Deploying to Render

`scripts/deploy-render.sh <api-key> <service-id>` and
`scripts/smoke-deploy.sh <base-url>` are copied in from
`infrastructure/providers/render/` (see that repo's README) and are already
wired into the `deploy-render` CI job above. Set the `RENDER_API_KEY`,
`RENDER_SERVICE_ID`, and `RENDER_EXTERNAL_URL` repository secrets to enable
it; the job only runs on a push to the `production` branch. If your app adds
authenticated routes, extend your copy of `smoke-deploy.sh` with an
app-specific check the way `file-combiner/scripts/smoke-deploy.sh` does.
