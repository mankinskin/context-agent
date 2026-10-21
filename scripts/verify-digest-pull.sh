#!/usr/bin/env bash
# Digest-based publish-to-consume pipeline check (package 06). Pulls the
# published `infrastructure-base` image by its exact digest, builds this
# repo FROM that pinned digest, and smoke-tests the result — classifying
# any failure into exactly one of three buckets so diagnosis is
# deterministic. Assumes the caller has already authenticated to GHCR
# (e.g. `docker login ghcr.io`) if the package is private; this script
# never handles or requests credentials itself.
#
# Usage: scripts/verify-digest-pull.sh <org>/infrastructure-base@sha256:<digest>
#
# Exit codes / failure buckets (each printed as FAILURE_BUCKET=<name>):
#   0  - success, no failure bucket printed
#   20 - registry failure: the digest pull itself failed (auth, visibility,
#        retention-expired, network) — printed as FAILURE_BUCKET=registry
#   21 - template/consumer failure: the pull succeeded but this repo's
#        build or smoke checks failed against that digest — printed as
#        FAILURE_BUCKET=template
#   22 - secret-leak failure: the pulled image's layers contain a
#        recognizable secret pattern — printed as FAILURE_BUCKET=secret-leak
#
# Base-image failures (package 04's own build/test steps failing before
# publish) are out of scope here by construction: this script only runs
# once a digest has already been published, so that bucket is package 04's
# own CI (portability-tests/secret-sentinel jobs), not this script.
set -uo pipefail

REF="${1:?usage: verify-digest-pull.sh <org>/infrastructure-base@sha256:<digest>}"
FULL_REF="ghcr.io/${REF}"
BUILD_TAG="template-digest-check"

echo "verify-digest-pull: pulling ${FULL_REF}..."
if ! docker pull "$FULL_REF"; then
  echo "verify-digest-pull: FATAL: digest pull failed (auth, visibility, retention, or network)." >&2
  echo "FAILURE_BUCKET=registry" >&2
  exit 20
fi

echo "verify-digest-pull: building this repo FROM the pinned digest..."
if ! docker build --ssh default --build-arg "BASE_IMAGE=${FULL_REF}" -t "$BUILD_TAG" .; then
  echo "verify-digest-pull: FATAL: build failed against the pinned digest." >&2
  echo "FAILURE_BUCKET=template" >&2
  exit 21
fi

echo "verify-digest-pull: scanning pulled image layers for known secret patterns..."
SECRET_PATTERNS=(
  'BEGIN OPENSSH PRIVATE KEY'
  'BEGIN RSA PRIVATE KEY'
  'BEGIN PRIVATE KEY'
  'ghp_'
  'github_pat_'
)
LEAK_FOUND=0
DUMP_FILE=$(mktemp)
docker save "$FULL_REF" | tar -xO > "$DUMP_FILE" 2>/dev/null || true
for pattern in "${SECRET_PATTERNS[@]}"; do
  if grep -F -q "$pattern" "$DUMP_FILE"; then
    echo "verify-digest-pull: FATAL: found secret pattern '$pattern' in pulled image layers." >&2
    LEAK_FOUND=1
  fi
done
rm -f "$DUMP_FILE"
if [ "$LEAK_FOUND" -eq 1 ]; then
  echo "FAILURE_BUCKET=secret-leak" >&2
  exit 22
fi

echo "verify-digest-pull: running container smoke checks against the digest-pinned build..."
if ! bash "$(dirname "${BASH_SOURCE[0]}")/smoke-container.sh" "$BUILD_TAG" 18081; then
  echo "verify-digest-pull: FATAL: container smoke checks failed against the pinned digest." >&2
  echo "FAILURE_BUCKET=template" >&2
  exit 21
fi

echo "verify-digest-pull: success — ${FULL_REF} pulls clean and builds/runs correctly."
