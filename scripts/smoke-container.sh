#!/usr/bin/env bash
# HTTP smoke only; the owned browser runner supplies responsive E2E evidence.
set -euo pipefail

IMAGE="${1:?usage: smoke-container.sh <image-tag> [port]}"
PORT="${2:-0}"
NAME="context-agent-smoke-$$-${RANDOM}"
CREATED=0
cleanup() {
  local status=$?
  if [[ "$CREATED" == 1 ]]; then
    if ! docker rm -f "$NAME" >/dev/null; then
      echo "smoke-container: cleanup failed for owned container $NAME" >&2
      status=15
    fi
  fi
  exit "$status"
}
trap cleanup EXIT

docker run -d -p "127.0.0.1:${PORT}:8080" --name "$NAME" "$IMAGE" >/dev/null
CREATED=1
PORT="$(docker port "$NAME" 8080/tcp)"
BASE_URL="http://${PORT}"
READY=0
for _ in $(seq 1 30); do
  if [[ "$(curl --max-time 1 -fsS "$BASE_URL/health" 2>/dev/null || true)" == "ok" ]]; then
    READY=1
    break
  fi
  sleep 0.5
done
if [[ "$READY" != 1 ]]; then
  echo "smoke-container: health readiness deadline exceeded" >&2
  docker logs "$NAME" >&2
  exit 11
fi
curl --max-time 5 -fsS "$BASE_URL/" >/dev/null
SESSIONS="$(curl --max-time 5 -fsS "$BASE_URL/api/sessions")"
printf '%s' "$SESSIONS" | node -e '
let value = "";
process.stdin.on("data", chunk => value += chunk);
process.stdin.on("end", () => {
  if (!Array.isArray(JSON.parse(value))) throw new Error("sessions response must be an array");
});'
echo "smoke-container: HTTP contract passed (/, /health, /api/sessions); no echo endpoint or provider request"
