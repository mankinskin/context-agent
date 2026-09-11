#!/usr/bin/env bash
# Builds (if needed) and runs a template container image, then exercises
# /, /health, and /api/echo against it. Generic across any locally-tagged
# or digest-pinned image built from this repo's Dockerfile.
#
# Usage: scripts/smoke-container.sh <image-tag> [port]
#
# Exit codes (documented so CI/callers can interpret failures precisely):
#   0  - all checks passed
#   10 - container failed to start
#   11 - container did not become ready (health check timed out)
#   12 - GET /health did not return 200
#   13 - GET / did not return 200
#   14 - POST /api/echo did not echo the submitted message
set -uo pipefail

IMAGE="${1:?usage: smoke-container.sh <image-tag> [port]}"
PORT="${2:-18080}"
NAME="template-smoke-$$"

cleanup() {
  docker rm -f "$NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

if ! docker run -d --rm -p "${PORT}:8080" --name "$NAME" "$IMAGE" >/dev/null; then
  echo "smoke-container: FATAL: container failed to start" >&2
  exit 10
fi

ready=0
for _ in $(seq 1 20); do
  if curl -fsS -o /dev/null "http://localhost:${PORT}/health" 2>/dev/null; then
    ready=1
    break
  fi
  sleep 0.5
done
if [ "$ready" -ne 1 ]; then
  echo "smoke-container: FATAL: container did not become ready within 10s" >&2
  docker logs "$NAME" >&2 || true
  exit 11
fi

health_code=$(curl -s -o /dev/null -w '%{http_code}' "http://localhost:${PORT}/health")
if [ "$health_code" != "200" ]; then
  echo "smoke-container: FATAL: GET /health returned $health_code, expected 200" >&2
  exit 12
fi

root_code=$(curl -s -o /dev/null -w '%{http_code}' "http://localhost:${PORT}/")
if [ "$root_code" != "200" ]; then
  echo "smoke-container: FATAL: GET / returned $root_code, expected 200" >&2
  exit 13
fi

echo_body=$(curl -s -X POST -H 'content-type: application/json' \
  -d '{"message":"smoke-test"}' "http://localhost:${PORT}/api/echo")
if [[ "$echo_body" != *'"message":"smoke-test"'* ]]; then
  echo "smoke-container: FATAL: POST /api/echo did not echo the submitted message (got: $echo_body)" >&2
  exit 14
fi

echo "smoke-container: all checks passed against $IMAGE (/, /health, /api/echo)"
