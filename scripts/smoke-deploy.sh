#!/usr/bin/env bash
set -euo pipefail

# Generic post-deploy validation against a Render-hosted instance: HTTPS
# enforcement plus a health-check probe. Copied in from
# infrastructure/providers/render/smoke-deploy.sh; retargeted at this
# template's own /health route.
BASE_URL="${1:?Usage: smoke-deploy.sh <base-url>}"
HEALTH_PATH="/health"

set +x

echo "smoke-deploy: starting production validation tests"

echo "TEST 1: HTTPS enforcement (reject plaintext HTTP)"
if [[ "$BASE_URL" == http://* ]]; then
  echo "  ERROR: BASE_URL must use https:// (plaintext HTTP not allowed in production)" >&2
  echo "  Deployment blocked: HTTPS is mandatory" >&2
  exit 1
fi
if [[ "$BASE_URL" != https://* ]]; then
  echo "  ERROR: BASE_URL must use https:// scheme (got: $BASE_URL)" >&2
  exit 1
fi
echo "  PASS: HTTPS scheme validated"

echo "TEST 2: Health check endpoint validation"
status="$(curl -s -o /dev/null -w '%{http_code}' "${BASE_URL%/}${HEALTH_PATH}")"
if [[ "$status" != "200" ]]; then
  echo "  ERROR: ${HEALTH_PATH} check failed (status ${status})" >&2
  exit 1
fi
echo "  PASS: ${HEALTH_PATH} returns 200"

echo "smoke-deploy: all checks passed"
